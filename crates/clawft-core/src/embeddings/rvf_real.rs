//! Real `rvf-runtime`-backed vector store with the same public interface as
//! [`crate::embeddings::rvf_stub`].
//!
//! This is the Phase 0 prerequisite from
//! `.planning/ruv/integration/agenticow-integration-plan.md` §5/§8: the brain
//! moves off the brute-force, JSON-backed stub onto the real, on-disk
//! `rvf_runtime::store::RvfStore` (append-only segments, HNSW-ready,
//! witness-chained). This module is gated behind the `rvf` feature; the
//! stub remains the backend for builds that can't link `rvf-runtime`
//! (wasm/browser/no-default). See [`crate::embeddings::rvf_backend`] for
//! the selector.
//!
//! # On-disk layout
//!
//! A store created at `path` (e.g. `memory.rvf`) is actually **two** files:
//!
//! - `path` itself — the real `rvf-runtime` binary segment file (vector
//!   segments, deletion bitmap, manifest, witness chain) written and read by
//!   `rvf_runtime::store::RvfStore`. This is the durable, append-only,
//!   witnessed store.
//! - `path` + `.sidecar.json` — a JSON side-index this module owns. It
//!   holds the `String -> u64` id assignment (RVF vector ids are `u64`;
//!   callers here use `String`) and a `u64 -> RvfEntry` cache (original,
//!   non-normalized embedding + arbitrary JSON metadata). RVF 0.2 has no
//!   public API to read a vector's raw components or arbitrary metadata
//!   back out by id (`query()` returns only `{id, distance}`, and its
//!   `MetadataEntry`/`MetadataValue` are typed, filter-oriented fields, not
//!   a JSON blob) — the sidecar bridges both gaps. `compact()` rewrites
//!   both files together; deleting one without the other desynchronizes
//!   the store.
//!
//! # Semantic gaps bridged vs. `rvf_stub`
//!
//! - **Dimension.** RVF requires a fixed `dimension` at creation time; the
//!   stub infers it from the first ingested vector. This module defers the
//!   real `RvfStore::create` until the first [`RvfStore::ingest`] call so
//!   the public `create()` can stay dimension-agnostic, matching the stub.
//! - **Cosine metric survives reopen.** Verified against the vendored
//!   `rvf-runtime` 0.2.0 source: `RvfStore::open()` does NOT restore the
//!   `metric` option from the manifest (it resets to `RvfOptions::default()`,
//!   i.e. `DistanceMetric::L2`) — see `store.rs::open()`. Per plan §3/§6,
//!   this module copies agenticow's discipline verbatim: vectors are
//!   L2-normalized before `ingest`/`query`, and the metric is always `L2`
//!   (the default, so it's inert to the reopen bug either way). On unit
//!   vectors, `||a-b||^2 = 2 - 2*cos(a,b)`, so L2 order == cosine order and
//!   `score = 1 - distance/2` recovers the cosine similarity the stub's
//!   `cosine_similarity` would have returned.
//! - **String ids.** Bridged via the sidecar's `String -> u64` map (see
//!   above). Re-ingesting an existing `String` id reuses its `u64` id,
//!   which RVF treats as an in-place upsert (its in-memory vector index is
//!   keyed by id), preserving the stub's upsert semantics.
//! - **No backing file (`create(None)`).** The stub allows a pure in-memory
//!   store; RVF always requires a real file. `create(None)` allocates a
//!   private temp file (removed in `Drop`) and `path()` still reports
//!   `None`, preserving the stub's caller-visible "no persistent location
//!   requested" contract even though this backend touches disk internally.
//! - **Metadata is not RVF-native.** This module does not use RVF's typed
//!   `MetadataEntry`/`MetadataValue` segments (metadata-based filtering is
//!   out of scope for Phase 0's parity contract) — arbitrary JSON metadata
//!   lives entirely in the sidecar cache.
//! - **Zero-length / oversized vectors.** RVF rejects a zero dimension and
//!   caps dimension at `u16::MAX`; the stub has no such limits. The first
//!   `ingest` call that establishes the store's dimension surfaces
//!   [`RvfError::InvalidDimension`] for either case instead of silently
//!   accepting it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tracing::debug;

use rvf_runtime::options::{DistanceMetric, QueryOptions, RvfOptions};
use rvf_runtime::store::RvfStore as RvfRuntimeStore;

/// Errors from the real RVF-backed store.
#[non_exhaustive]
#[derive(Debug)]
pub enum RvfError {
    /// An I/O error occurred (reading/writing the store or sidecar file).
    Io(std::io::Error),
    /// A serialization/deserialization error occurred (sidecar JSON).
    Serde(serde_json::Error),
    /// The entry was not found (also used when a store's sidecar index is
    /// missing while its `.rvf` file exists — the two must travel together).
    NotFound(String),
    /// A vector's length didn't match the store's fixed dimension and was
    /// rejected by `ingest_batch`.
    DimensionMismatch {
        /// The store's fixed dimension.
        expected: usize,
        /// The length of the vector that was rejected.
        actual: usize,
    },
    /// A vector's length can't become the store's dimension: either zero,
    /// or larger than RVF's `u16` dimension limit.
    InvalidDimension(usize),
    /// An error surfaced directly from the underlying `rvf-runtime` store.
    Rvf(rvf_types::RvfError),
}

impl std::fmt::Display for RvfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RvfError::Io(e) => write!(f, "rvf I/O error: {e}"),
            RvfError::Serde(e) => write!(f, "rvf serde error: {e}"),
            RvfError::NotFound(id) => write!(f, "rvf entry not found: {id}"),
            RvfError::DimensionMismatch { expected, actual } => {
                write!(f, "rvf dimension mismatch: expected {expected}, got {actual}")
            }
            RvfError::InvalidDimension(d) => {
                write!(f, "rvf invalid dimension: {d} (must be 1..=65535)")
            }
            RvfError::Rvf(e) => write!(f, "rvf-runtime error: {e}"),
        }
    }
}

impl std::error::Error for RvfError {}

impl From<std::io::Error> for RvfError {
    fn from(e: std::io::Error) -> Self {
        RvfError::Io(e)
    }
}

impl From<serde_json::Error> for RvfError {
    fn from(e: serde_json::Error) -> Self {
        RvfError::Serde(e)
    }
}

impl From<rvf_types::RvfError> for RvfError {
    fn from(e: rvf_types::RvfError) -> Self {
        RvfError::Rvf(e)
    }
}

/// A single entry in the store, as returned by [`RvfStore::get`].
///
/// Same shape as [`crate::embeddings::rvf_stub::RvfEntry`]: `embedding` is
/// the caller's original vector (not the L2-normalized copy this module
/// stores internally for search).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RvfEntry {
    /// Unique identifier.
    pub id: String,
    /// The embedding vector, as originally ingested.
    pub embedding: Vec<f32>,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

/// A query result from the store.
#[derive(Debug, Clone)]
pub struct RvfQueryResult {
    /// The entry ID.
    pub id: String,
    /// Cosine similarity score (recovered from L2 distance over normalized
    /// vectors -- see module docs).
    pub score: f32,
    /// The entry metadata.
    pub metadata: serde_json::Value,
}

/// The persisted sidecar index: id assignment + entry cache.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SidecarFile {
    next_id: u64,
    entries: HashMap<u64, RvfEntry>,
}

/// In-memory view of the sidecar, plus the derived `String -> u64` map.
#[derive(Debug, Default)]
struct SidecarCache {
    next_id: u64,
    entries: HashMap<u64, RvfEntry>,
    by_string: HashMap<String, u64>,
}

impl SidecarCache {
    fn from_file(file: SidecarFile) -> Self {
        let by_string = file
            .entries
            .iter()
            .map(|(&rvf_id, entry)| (entry.id.clone(), rvf_id))
            .collect();
        Self {
            next_id: file.next_id,
            entries: file.entries,
            by_string,
        }
    }

    fn to_file(&self) -> SidecarFile {
        SidecarFile {
            next_id: self.next_id,
            entries: self.entries.clone(),
        }
    }
}

fn sidecar_path_for(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".sidecar.json");
    PathBuf::from(s)
}

fn write_sidecar(path: &Path, cache: &SidecarCache) -> Result<(), RvfError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&cache.to_file())?;
    std::fs::write(path, json)?;
    Ok(())
}

fn read_sidecar(path: &Path) -> Result<SidecarCache, RvfError> {
    let data = std::fs::read_to_string(path)?;
    let file: SidecarFile = serde_json::from_str(&data)?;
    Ok(SidecarCache::from_file(file))
}

static EPHEMERAL_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_store_path() -> PathBuf {
    let n = EPHEMERAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("clawft_rvf_real_ephemeral_{pid}_{n}.rvf"))
}

/// L2-normalize a vector; degenerate (near-zero-norm) vectors pass through
/// unchanged, matching the stub's "treat as dissimilar to everything"
/// behavior for that edge case.
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < f32::EPSILON {
        v.to_vec()
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

enum StoreState {
    /// No real store materialized yet -- dimension isn't known until the
    /// first `ingest`.
    Uninit { path: Option<PathBuf> },
    /// Real store open and ready. Boxed: `RvfRuntimeStore` is large (~650
    /// bytes) relative to `Uninit`, and this enum is stored inline in
    /// `RvfStore`.
    Ready {
        store: Box<RvfRuntimeStore>,
        path: PathBuf,
        /// `true` if `path` is a private temp file standing in for a
        /// `create(None)` in-memory request -- `path()` reports `None` for
        /// these regardless of the real backing file.
        ephemeral: bool,
    },
}

/// Real `rvf-runtime`-backed vector store with an RVF-compatible interface.
///
/// See the module docs for the on-disk layout and the semantic gaps bridged
/// relative to [`crate::embeddings::rvf_stub::RvfStore`].
///
/// # Thread Safety
///
/// Not thread-safe, same as the stub -- wrap in a `Mutex`/`RwLock` for
/// concurrent access.
pub struct RvfStore {
    inner: StoreState,
    cache: SidecarCache,
}

impl RvfStore {
    /// Create a new store with an optional file path.
    ///
    /// Real `rvf-runtime` initialization is deferred to the first
    /// [`ingest`](RvfStore::ingest) call, since RVF needs to know the
    /// vector dimension up front and the stub-compatible contract doesn't
    /// require callers to state it here.
    pub fn create(path: Option<&Path>) -> Self {
        debug!(path = ?path, "creating new rvf_real RvfStore (deferred)");
        Self {
            inner: StoreState::Uninit {
                path: path.map(|p| p.to_path_buf()),
            },
            cache: SidecarCache::default(),
        }
    }

    /// Open an existing store.
    ///
    /// If no `.rvf` file exists at `path` yet, behaves like `create` (matches
    /// the stub's "no existing store, creating new" semantics). If the
    /// `.rvf` file exists but its `.sidecar.json` companion doesn't, that's
    /// a desynchronized store and returns `RvfError::NotFound`.
    pub fn open(path: &Path) -> Result<Self, RvfError> {
        if !path.exists() {
            debug!(path = %path.display(), "no existing store, creating new");
            return Ok(Self::create(Some(path)));
        }

        debug!(path = %path.display(), "opening existing rvf_real RvfStore");
        let store = Box::new(RvfRuntimeStore::open(path)?);

        let sidecar_path = sidecar_path_for(path);
        if !sidecar_path.exists() {
            return Err(RvfError::NotFound(format!(
                "sidecar index missing for {} (expected {})",
                path.display(),
                sidecar_path.display()
            )));
        }
        let cache = read_sidecar(&sidecar_path)?;

        Ok(Self {
            inner: StoreState::Ready {
                store,
                path: path.to_path_buf(),
                ephemeral: false,
            },
            cache,
        })
    }

    /// Materialize the real `rvf-runtime` store if it hasn't been yet,
    /// using `dim` as the store's fixed dimension.
    fn ensure_ready(&mut self, dim: usize) -> Result<(), RvfError> {
        if let StoreState::Uninit { path } = &self.inner {
            let dim_u16: u16 = dim
                .try_into()
                .map_err(|_| RvfError::InvalidDimension(dim))?;
            if dim_u16 == 0 {
                return Err(RvfError::InvalidDimension(dim));
            }

            let (real_path, ephemeral) = match path {
                Some(p) => (p.clone(), false),
                None => (temp_store_path(), true),
            };

            // Mirror the stub's `create()`: it never reads prior content at
            // `path`, it always starts fresh. The real crate's `create`
            // fails if the file already exists, so clear any stale file
            // (and its sidecar) first.
            let _ = std::fs::remove_file(&real_path);
            let _ = std::fs::remove_file(sidecar_path_for(&real_path));
            if let Some(parent) = real_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let opts = RvfOptions {
                dimension: dim_u16,
                // L2 over L2-normalized vectors -- see module docs on why
                // Cosine is not used (doesn't survive reopen in 0.2.0).
                metric: DistanceMetric::L2,
                ..Default::default()
            };
            let store = Box::new(RvfRuntimeStore::create(&real_path, opts)?);

            self.inner = StoreState::Ready {
                store,
                path: real_path,
                ephemeral,
            };
        }
        Ok(())
    }

    /// Ingest (add) an entry into the store. Upserts by `id`.
    pub fn ingest(
        &mut self,
        id: String,
        embedding: Vec<f32>,
        metadata: serde_json::Value,
    ) -> Result<(), RvfError> {
        self.ensure_ready(embedding.len())?;

        let rvf_id = match self.cache.by_string.get(&id) {
            Some(&existing) => existing,
            None => {
                let new_id = self.cache.next_id;
                self.cache.next_id += 1;
                new_id
            }
        };

        let normalized = l2_normalize(&embedding);

        let store = match &mut self.inner {
            StoreState::Ready { store, .. } => store,
            StoreState::Uninit { .. } => unreachable!("ensure_ready just materialized the store"),
        };

        let result = store.ingest_batch(&[normalized.as_slice()], &[rvf_id], None)?;
        if result.rejected > 0 {
            return Err(RvfError::DimensionMismatch {
                expected: store.dimension() as usize,
                actual: embedding.len(),
            });
        }

        self.cache.by_string.insert(id.clone(), rvf_id);
        self.cache.entries.insert(
            rvf_id,
            RvfEntry {
                id,
                embedding,
                metadata,
            },
        );
        Ok(())
    }

    /// Query the store for the top-k most similar entries.
    ///
    /// Returns results sorted by descending cosine similarity, recovered
    /// from L2 distance over normalized vectors (see module docs). Returns
    /// an empty vec if the store has never been ingested into, matching the
    /// stub's empty-store behavior.
    pub fn query(&self, query_embedding: &[f32], top_k: usize) -> Vec<RvfQueryResult> {
        let (store, _) = match &self.inner {
            StoreState::Ready { store, path, .. } => (store, path),
            StoreState::Uninit { .. } => return Vec::new(),
        };
        if top_k == 0 {
            return Vec::new();
        }

        let normalized = l2_normalize(query_embedding);
        if normalized.len() != store.dimension() as usize {
            return Vec::new();
        }

        let results = match store.query(&normalized, top_k, &QueryOptions::default()) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        results
            .into_iter()
            .filter_map(|r| {
                let entry = self.cache.entries.get(&r.id)?;
                // Unit vectors: ||a-b||^2 = 2 - 2*cos(a,b)  =>  cos = 1 - dist/2.
                let score = 1.0 - r.distance / 2.0;
                Some(RvfQueryResult {
                    id: entry.id.clone(),
                    score,
                    metadata: entry.metadata.clone(),
                })
            })
            .collect()
    }

    /// Delete an entry by ID. Returns `true` if an entry was removed.
    pub fn delete(&mut self, id: &str) -> Result<bool, RvfError> {
        let Some(rvf_id) = self.cache.by_string.get(id).copied() else {
            return Ok(false);
        };

        if let StoreState::Ready { store, .. } = &mut self.inner {
            store.delete(&[rvf_id])?;
        }

        self.cache.by_string.remove(id);
        self.cache.entries.remove(&rvf_id);
        Ok(true)
    }

    /// Persist the store: compacts the real RVF file and rewrites the
    /// sidecar. No-op if nothing has been ingested yet, or if this is an
    /// ephemeral (`create(None)`) store.
    pub fn compact(&mut self) -> Result<(), RvfError> {
        let (store, path, ephemeral) = match &mut self.inner {
            StoreState::Ready {
                store,
                path,
                ephemeral,
            } => (store, path, *ephemeral),
            StoreState::Uninit { .. } => return Ok(()),
        };

        store.compact()?;

        if !ephemeral {
            write_sidecar(&sidecar_path_for(path), &self.cache)?;
        }
        Ok(())
    }

    /// Return the number of entries in the store.
    pub fn len(&self) -> usize {
        self.cache.entries.len()
    }

    /// Return `true` if the store has no entries.
    pub fn is_empty(&self) -> bool {
        self.cache.entries.is_empty()
    }

    /// Get the file path (if set). Ephemeral (`create(None)`) stores
    /// always report `None` here, even though they use a private temp file
    /// internally -- see module docs.
    pub fn path(&self) -> Option<&Path> {
        match &self.inner {
            StoreState::Uninit { path } => path.as_deref(),
            StoreState::Ready { path, ephemeral, .. } => {
                if *ephemeral {
                    None
                } else {
                    Some(path.as_path())
                }
            }
        }
    }

    /// Get an entry by ID.
    pub fn get(&self, id: &str) -> Option<&RvfEntry> {
        let rvf_id = self.cache.by_string.get(id)?;
        self.cache.entries.get(rvf_id)
    }
}

impl Drop for RvfStore {
    fn drop(&mut self) {
        // Ephemeral (`create(None)`) stores use a private temp file that
        // was never exposed via `path()` -- clean it up so we don't leak
        // files into the temp dir on every in-memory-style use.
        if let StoreState::Ready {
            path,
            ephemeral: true,
            ..
        } = &self.inner
        {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_file(sidecar_path_for(path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64 as TestAtomicU64, Ordering as TestOrdering};

    static TEST_COUNTER: TestAtomicU64 = TestAtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, TestOrdering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_rvf_real_test_{label}_{pid}_{n}.rvf"))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(sidecar_path_for(path));
        let _ = std::fs::remove_file(path.with_extension("rvf.lock"));
    }

    #[test]
    fn create_empty_store() {
        let store = RvfStore::create(None);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.path().is_none());
    }

    #[test]
    fn ephemeral_store_ingest_and_query_no_backing_path() {
        let mut store = RvfStore::create(None);
        store
            .ingest(
                "doc1".into(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"text": "hello"}),
            )
            .unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.path().is_none(), "ephemeral store must not expose a path");

        let results = store.query(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!((results[0].score - 1.0).abs() < 0.01);
    }

    #[test]
    fn ingest_and_query_round_trip() {
        let path = temp_path("ingest_query");
        cleanup(&path);
        let mut store = RvfStore::create(Some(&path));

        store
            .ingest(
                "doc1".into(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"text": "hello"}),
            )
            .unwrap();
        store
            .ingest(
                "doc2".into(),
                vec![0.0, 1.0, 0.0],
                serde_json::json!({"text": "world"}),
            )
            .unwrap();

        let results = store.query(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!((results[0].score - 1.0).abs() < 0.01);

        cleanup(&path);
    }

    #[test]
    fn query_ordering() {
        let path = temp_path("query_ordering");
        cleanup(&path);
        let mut store = RvfStore::create(Some(&path));

        store.ingest("a".into(), vec![1.0, 0.0], serde_json::json!({})).unwrap();
        store.ingest("b".into(), vec![0.7, 0.7], serde_json::json!({})).unwrap();
        store.ingest("c".into(), vec![0.0, 1.0], serde_json::json!({})).unwrap();

        let results = store.query(&[1.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "b");
        assert_eq!(results[2].id, "c");

        cleanup(&path);
    }

    #[test]
    fn delete_entry() {
        let path = temp_path("delete");
        cleanup(&path);
        let mut store = RvfStore::create(Some(&path));

        store.ingest("doc1".into(), vec![1.0], serde_json::json!({})).unwrap();
        store.ingest("doc2".into(), vec![0.0], serde_json::json!({})).unwrap();

        assert!(store.delete("doc1").unwrap());
        assert_eq!(store.len(), 1);
        assert!(!store.delete("doc1").unwrap()); // already gone

        cleanup(&path);
    }

    #[test]
    fn upsert_semantics() {
        let path = temp_path("upsert");
        cleanup(&path);
        let mut store = RvfStore::create(Some(&path));

        store
            .ingest("doc1".into(), vec![1.0, 0.0], serde_json::json!({"v": 1}))
            .unwrap();
        store
            .ingest("doc1".into(), vec![0.0, 1.0], serde_json::json!({"v": 2}))
            .unwrap();

        assert_eq!(store.len(), 1);
        let entry = store.get("doc1").unwrap();
        assert_eq!(entry.metadata["v"], 2);
        assert_eq!(entry.embedding, vec![0.0, 1.0]);

        cleanup(&path);
    }

    #[test]
    fn compact_and_reopen_round_trip() {
        let path = temp_path("reopen");
        cleanup(&path);

        {
            let mut store = RvfStore::create(Some(&path));
            store
                .ingest(
                    "doc1".into(),
                    vec![1.0, 0.0, 0.0],
                    serde_json::json!({"text": "hello"}),
                )
                .unwrap();
            store
                .ingest(
                    "doc2".into(),
                    vec![0.0, 1.0, 0.0],
                    serde_json::json!({"text": "world"}),
                )
                .unwrap();
            store.compact().unwrap();
        }

        let store = RvfStore::open(&path).unwrap();
        assert_eq!(store.len(), 2);

        // Top-K identical to pre-close -- guards the cosine-via-normalized-L2
        // reopen discipline documented in the module docs.
        let results = store.query(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!((results[0].score - 1.0).abs() < 0.01);

        let entry = store.get("doc2").unwrap();
        assert_eq!(entry.metadata["text"], "world");

        cleanup(&path);
    }

    #[test]
    fn open_nonexistent_creates_empty() {
        let path = temp_path("nonexist");
        cleanup(&path);

        let store = RvfStore::open(&path).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.path(), Some(path.as_path()));

        cleanup(&path);
    }

    #[test]
    fn open_missing_sidecar_is_desync_error() {
        let path = temp_path("desync");
        cleanup(&path);

        {
            let mut store = RvfStore::create(Some(&path));
            store.ingest("doc1".into(), vec![1.0], serde_json::json!({})).unwrap();
            store.compact().unwrap();
        }
        std::fs::remove_file(sidecar_path_for(&path)).unwrap();

        let result = RvfStore::open(&path);
        assert!(matches!(result, Err(RvfError::NotFound(_))));

        cleanup(&path);
    }

    #[test]
    fn query_empty_store() {
        let store = RvfStore::create(None);
        let results = store.query(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn query_top_k_zero() {
        let mut store = RvfStore::create(None);
        store.ingest("x".into(), vec![1.0], serde_json::json!({})).unwrap();
        let results = store.query(&[1.0], 0);
        assert!(results.is_empty());
    }

    #[test]
    fn get_entry_by_id() {
        let mut store = RvfStore::create(None);
        store
            .ingest(
                "doc1".into(),
                vec![1.0, 2.0],
                serde_json::json!({"key": "value"}),
            )
            .unwrap();

        let entry = store.get("doc1");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().id, "doc1");
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn dimension_mismatch_is_rejected() {
        let mut store = RvfStore::create(None);
        store.ingest("a".into(), vec![1.0, 0.0], serde_json::json!({})).unwrap();

        let result = store.ingest("b".into(), vec![1.0, 0.0, 0.0], serde_json::json!({}));
        assert!(matches!(result, Err(RvfError::DimensionMismatch { .. })));
    }

    #[test]
    fn zero_dimension_is_rejected() {
        let mut store = RvfStore::create(None);
        let result = store.ingest("a".into(), vec![], serde_json::json!({}));
        assert!(matches!(result, Err(RvfError::InvalidDimension(0))));
    }

    #[test]
    fn compact_without_ingest_is_noop() {
        let path = temp_path("noop_compact");
        cleanup(&path);
        let mut store = RvfStore::create(Some(&path));
        assert!(store.compact().is_ok());
        assert!(!path.exists(), "no real store should have been materialized");
        cleanup(&path);
    }

    #[test]
    fn rvf_error_display() {
        let err = RvfError::NotFound("xyz".into());
        assert!(format!("{err}").contains("xyz"));

        let err = RvfError::DimensionMismatch { expected: 3, actual: 4 };
        assert!(format!("{err}").contains('3'));
        assert!(format!("{err}").contains('4'));
    }
}
