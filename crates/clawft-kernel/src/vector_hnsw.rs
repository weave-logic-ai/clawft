//! HNSW-backed [`VectorBackend`] implementation.
//!
//! Wraps the existing [`HnswService`] behind the unified
//! [`VectorBackend`] trait so that it can be used standalone or as the
//! hot tier inside [`HybridBackend`](super::vector_hybrid::HybridBackend).
//!
//! ## Hardening features (Cognitum Seed WS1)
//!
//! - **Epoch-based versioning**: monotonic epoch bumped on every mutation.
//! - **Optimistic concurrency**: `insert_with_epoch` rejects stale writes.
//! - **Soft-delete + compaction**: tombstone records excluded from search.
//! - **Capacity limits**: configurable `max_vectors` with `StoreFull` error.
//! - **Tombstone persistence** (WEFT-127): [`HnswBackend::save_to_file`] /
//!   [`HnswBackend::load_from_file`] round-trip id map, epoch, capacity,
//!   and soft-delete tombstones so deleted entries stay gone after restart.
//!
//! ## Soft-delete and compaction
//!
//! Soft-delete ([`VectorBackend::soft_delete`]) marks a numeric id as
//! tombstoned at the current epoch. Tombstoned ids are excluded from
//! `search`, `contains`, and `len`, but the underlying HNSW entry remains
//! until compaction.
//!
//! [`VectorBackend::compact`] with `older_than_epoch` **physically** removes
//! tombstones whose `deleted_at_epoch` is strictly less than that threshold:
//! the id-map entry is dropped and the string-keyed vector is deleted from
//! the inner [`HnswService`]. Callers that want durable soft-delete across
//! process restarts must call [`HnswBackend::save_to_file`] after deletes
//! (and after compact, if they want the purged state on disk).
//!
//! On-disk format is a JSON superset of the core `HnswStore` snapshot:
//! legacy files with only `entries` / `ef_search` / `ef_construction`
//! load with an empty tombstone section (backward compatible).
//!
//! Compiled only when the `ecc` feature is enabled.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use clawft_core::embeddings::hnsw_store::{HnswEntry, HnswStore};

use crate::hnsw_service::{HnswService, HnswServiceConfig};
use crate::vector_backend::{SearchResult, VectorBackend, VectorError, VectorResult};

// ── ID ↔ key mapping ────────────────────────────────────────────────────

/// Internal mapping between numeric IDs (used by `VectorBackend`) and
/// the string keys used by `HnswService`.
struct IdMap {
    id_to_key: HashMap<u64, String>,
    key_to_id: HashMap<String, u64>,
}

impl IdMap {
    fn new() -> Self {
        Self {
            id_to_key: HashMap::new(),
            key_to_id: HashMap::new(),
        }
    }

    fn insert(&mut self, id: u64, key: String) {
        // Remove old key mapping if this id was previously used.
        if let Some(old_key) = self.id_to_key.insert(id, key.clone())
            && old_key != key
        {
            self.key_to_id.remove(&old_key);
        }
        self.key_to_id.insert(key, id);
    }

    fn contains_id(&self, id: u64) -> bool {
        self.id_to_key.contains_key(&id)
    }

    fn key_for_id(&self, id: u64) -> Option<&str> {
        self.id_to_key.get(&id).map(|s| s.as_str())
    }

    fn remove(&mut self, id: u64) -> Option<String> {
        if let Some(key) = self.id_to_key.remove(&id) {
            self.key_to_id.remove(&key);
            Some(key)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.id_to_key.len()
    }
}

// ── Tombstone record ───────────────────────────────────────────────────

/// A soft-deleted vector. Keeps the ID and the epoch at which it was
/// deleted so that [`VectorBackend::compact`] can purge old tombstones.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tombstone {
    /// Epoch at which the vector was soft-deleted.
    deleted_at_epoch: u64,
}

// ── On-disk snapshot (WEFT-127) ─────────────────────────────────────────

/// JSON format written by [`HnswBackend::save_to_file`].
///
/// Super-set of the core `HnswStore` snapshot so:
/// - new files remain loadable by `HnswStore::load` (extra keys ignored);
/// - old files (no `tombstones` / `id_map` / `epoch`) load with empty
///   tombstone state via `#[serde(default)]`.
#[derive(Debug, Serialize, Deserialize)]
struct HnswBackendSnapshot {
    entries: Vec<HnswEntry>,
    ef_search: usize,
    ef_construction: usize,
    /// Numeric id → string key map. Missing on legacy snapshots → empty.
    #[serde(default)]
    id_map: Vec<(u64, String)>,
    /// Soft-deleted ids with deletion epoch. Missing → empty (compat).
    #[serde(default)]
    tombstones: Vec<(u64, Tombstone)>,
    /// Monotonic backend epoch. Missing → 0.
    #[serde(default)]
    epoch: u64,
    /// Optional capacity limit. Missing → `None` (unbounded).
    #[serde(default)]
    max_vectors: Option<usize>,
}

// ── Backend ─────────────────────────────────────────────────────────────

/// HNSW vector backend wrapping [`HnswService`].
pub struct HnswBackend {
    inner: HnswService,
    id_map: Mutex<IdMap>,
    /// Monotonic epoch counter -- bumped on every mutation.
    epoch: AtomicU64,
    /// Soft-deleted entries keyed by vector ID.
    tombstones: Mutex<HashMap<u64, Tombstone>>,
    /// Optional upper bound on stored vectors.
    max_vectors: Mutex<Option<usize>>,
}

impl HnswBackend {
    /// Create a new HNSW backend with the given configuration.
    pub fn new(config: HnswServiceConfig) -> Self {
        Self {
            inner: HnswService::new(config),
            id_map: Mutex::new(IdMap::new()),
            epoch: AtomicU64::new(0),
            tombstones: Mutex::new(HashMap::new()),
            max_vectors: Mutex::new(None),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(HnswServiceConfig::default())
    }

    /// Create with a capacity limit.
    pub fn with_max_vectors(config: HnswServiceConfig, limit: usize) -> Self {
        Self {
            inner: HnswService::new(config),
            id_map: Mutex::new(IdMap::new()),
            epoch: AtomicU64::new(0),
            tombstones: Mutex::new(HashMap::new()),
            max_vectors: Mutex::new(Some(limit)),
        }
    }

    /// Access the underlying [`HnswService`] for legacy code paths.
    pub fn inner(&self) -> &HnswService {
        &self.inner
    }

    /// Persist backend state (vectors, id map, tombstones, epoch, capacity).
    ///
    /// Soft-deleted entries remain in the vector list but are recorded in
    /// the `tombstones` section so they stay excluded after
    /// [`load_from_file`]. See module docs for the compaction story.
    pub fn save_to_file(&self, path: &Path) -> VectorResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let entries = self.inner.snapshot_entries();
        let (ef_search, ef_construction) = self.inner.ef_params();

        let map = self.id_map.lock().expect("IdMap lock poisoned");
        let id_map: Vec<(u64, String)> = map
            .id_to_key
            .iter()
            .map(|(&id, key)| (id, key.clone()))
            .collect();

        let ts = self.tombstones.lock().expect("tombstones lock poisoned");
        let tombstones: Vec<(u64, Tombstone)> = ts
            .iter()
            .map(|(&id, t)| (id, t.clone()))
            .collect();

        let snapshot = HnswBackendSnapshot {
            entries,
            ef_search,
            ef_construction,
            id_map,
            tombstones,
            epoch: self.epoch.load(Ordering::SeqCst),
            max_vectors: *self.max_vectors.lock().expect("max_vectors lock poisoned"),
        };

        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| VectorError::Other(format!("serialize hnsw backend: {e}")))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a backend from a snapshot written by [`save_to_file`].
    ///
    /// **Backward compatible**: files produced by plain `HnswStore` /
    /// `HnswService` save (entries + ef params only) load successfully
    /// with empty `id_map`, empty tombstones, epoch 0, and no capacity
    /// limit.
    pub fn load_from_file(path: &Path) -> VectorResult<Self> {
        if !path.exists() {
            return Ok(Self::with_defaults());
        }

        let data = std::fs::read_to_string(path)?;
        let snapshot: HnswBackendSnapshot = serde_json::from_str(&data)
            .map_err(|e| VectorError::Other(format!("deserialize hnsw backend: {e}")))?;

        let store = HnswStore::from_entries(
            snapshot.entries,
            snapshot.ef_search,
            snapshot.ef_construction,
        );
        let config = HnswServiceConfig {
            ef_search: snapshot.ef_search,
            ef_construction: snapshot.ef_construction,
            default_dimensions: 384,
        };
        let inner = HnswService::from_store(config, store);

        let mut id_map = IdMap::new();
        for (id, key) in snapshot.id_map {
            id_map.insert(id, key);
        }

        let mut tombstones = HashMap::new();
        for (id, t) in snapshot.tombstones {
            tombstones.insert(id, t);
        }

        Ok(Self {
            inner,
            id_map: Mutex::new(id_map),
            epoch: AtomicU64::new(snapshot.epoch),
            tombstones: Mutex::new(tombstones),
            max_vectors: Mutex::new(snapshot.max_vectors),
        })
    }

    /// Bump the epoch and return the new value.
    fn bump_epoch(&self) -> u64 {
        self.epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Number of live (non-tombstoned) vectors.
    fn live_count(&self) -> usize {
        let map = self.id_map.lock().expect("IdMap lock poisoned");
        let ts = self.tombstones.lock().expect("tombstones lock poisoned");
        map.len().saturating_sub(ts.len())
    }

    /// Check capacity and return an error if full.
    fn check_capacity(
        &self,
        id_map: &IdMap,
        tombstones: &HashMap<u64, Tombstone>,
        id: u64,
    ) -> VectorResult<()> {
        let limit = self.max_vectors.lock().expect("max_vectors lock poisoned");
        if let Some(max) = *limit {
            let live = id_map.len().saturating_sub(tombstones.len());
            // Allow upsert of existing id.
            if live >= max && !id_map.contains_id(id) {
                return Err(VectorError::StoreFull { max, current: live });
            }
        }
        Ok(())
    }
}

impl VectorBackend for HnswBackend {
    fn insert(
        &self,
        id: u64,
        key: &str,
        vector: &[f32],
        metadata: serde_json::Value,
    ) -> VectorResult<()> {
        let mut map = self.id_map.lock().expect("IdMap lock poisoned");
        let mut ts = self.tombstones.lock().expect("tombstones lock poisoned");

        // If this id was tombstoned, un-tombstone it (re-insert).
        ts.remove(&id);

        self.check_capacity(&map, &ts, id)?;

        map.insert(id, key.to_owned());
        drop(ts);
        drop(map);

        self.inner.insert(key.to_owned(), vector.to_vec(), metadata);
        self.bump_epoch();
        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        let results = self.inner.search(query, k);
        let map = self.id_map.lock().expect("IdMap lock poisoned");
        let ts = self.tombstones.lock().expect("tombstones lock poisoned");

        results
            .into_iter()
            .filter_map(|r| {
                // Reverse-lookup the numeric id from the string key.
                map.key_to_id.get(&r.id).and_then(|&numeric_id| {
                    // Exclude tombstoned entries.
                    if ts.contains_key(&numeric_id) {
                        return None;
                    }
                    // HnswService returns cosine similarity (1.0 = identical).
                    // Convert to distance: distance = 1.0 - similarity.
                    let distance = 1.0 - r.score;
                    Some(SearchResult::new(numeric_id, r.id, distance, r.metadata))
                })
            })
            .collect()
    }

    fn len(&self) -> usize {
        self.live_count()
    }

    fn contains(&self, id: u64) -> bool {
        let map = self.id_map.lock().expect("IdMap lock poisoned");
        let ts = self.tombstones.lock().expect("tombstones lock poisoned");
        map.contains_id(id) && !ts.contains_key(&id)
    }

    fn remove(&self, id: u64) -> bool {
        let mut map = self.id_map.lock().expect("IdMap lock poisoned");
        let mut ts = self.tombstones.lock().expect("tombstones lock poisoned");
        ts.remove(&id);
        let key = map.remove(id);
        let removed = key.is_some();
        if removed {
            drop(ts);
            drop(map);
            if let Some(key) = key {
                self.inner.delete(&key);
            }
            self.bump_epoch();
        }
        removed
    }

    fn flush(&self) -> VectorResult<()> {
        // Pure in-memory HNSW has no automatic durable path. Callers that
        // need durability (including tombstones) use [`Self::save_to_file`].
        Ok(())
    }

    fn backend_name(&self) -> &str {
        "hnsw"
    }

    // ── Epoch ───────────────────────────────────────────────────────────

    fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    fn insert_with_epoch(
        &self,
        id: u64,
        key: &str,
        vector: &[f32],
        metadata: serde_json::Value,
        parent_epoch: u64,
    ) -> VectorResult<()> {
        let current = self.epoch.load(Ordering::SeqCst);
        if parent_epoch < current {
            return Err(VectorError::EpochConflict {
                expected: parent_epoch,
                actual: current,
            });
        }
        self.insert(id, key, vector, metadata)
    }

    // ── Soft-delete + compaction ────────────────────────────────────────

    fn soft_delete(&self, id: u64) -> bool {
        let map = self.id_map.lock().expect("IdMap lock poisoned");
        let mut ts = self.tombstones.lock().expect("tombstones lock poisoned");

        if !map.contains_id(id) || ts.contains_key(&id) {
            return false;
        }

        let epoch = self.bump_epoch();
        ts.insert(
            id,
            Tombstone {
                deleted_at_epoch: epoch,
            },
        );
        true
    }

    fn compact(&self, older_than_epoch: u64) -> usize {
        let mut map = self.id_map.lock().expect("IdMap lock poisoned");
        let mut ts = self.tombstones.lock().expect("tombstones lock poisoned");

        let to_purge: Vec<(u64, String)> = ts
            .iter()
            .filter(|(_, t)| t.deleted_at_epoch < older_than_epoch)
            .filter_map(|(&id, _)| map.key_for_id(id).map(|k| (id, k.to_owned())))
            .collect();

        // Also drop orphan tombstones with no id_map entry.
        let orphan_ids: Vec<u64> = ts
            .iter()
            .filter(|(_, t)| t.deleted_at_epoch < older_than_epoch)
            .map(|(&id, _)| id)
            .filter(|id| !to_purge.iter().any(|(p, _)| p == id))
            .collect();

        let count = to_purge.len() + orphan_ids.len();
        let mut keys_to_delete = Vec::with_capacity(to_purge.len());
        for (id, key) in to_purge {
            ts.remove(&id);
            map.remove(id);
            keys_to_delete.push(key);
        }
        for id in orphan_ids {
            ts.remove(&id);
        }

        if count > 0 {
            drop(ts);
            drop(map);
            for key in keys_to_delete {
                self.inner.delete(&key);
            }
            self.bump_epoch();
        }

        count
    }

    fn tombstone_count(&self) -> usize {
        let ts = self.tombstones.lock().expect("tombstones lock poisoned");
        ts.len()
    }

    // ── Capacity limits ────────────────────────────────────────────────

    fn max_vectors(&self) -> Option<usize> {
        *self.max_vectors.lock().expect("max_vectors lock poisoned")
    }

    fn set_max_vectors(&self, limit: Option<usize>) {
        *self.max_vectors.lock().expect("max_vectors lock poisoned") = limit;
    }
}

impl std::fmt::Debug for HnswBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswBackend")
            .field("len", &self.len())
            .field("epoch", &self.current_epoch())
            .field("tombstones", &self.tombstone_count())
            .field("max_vectors", &self.max_vectors())
            .finish()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_backend() -> HnswBackend {
        HnswBackend::with_defaults()
    }

    #[test]
    fn insert_and_search() {
        let b = make_backend();
        b.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        b.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({}))
            .unwrap();
        b.insert(3, "c", &[0.0, 0.0, 1.0], serde_json::json!({}))
            .unwrap();

        assert_eq!(b.len(), 3);
        assert!(!b.is_empty());

        let results = b.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
        assert_eq!(results[0].key, "a");
        assert!(results[0].distance < 0.01);
    }

    #[test]
    fn contains_and_remove() {
        let b = make_backend();
        b.insert(10, "x", &[1.0, 0.0], serde_json::json!({}))
            .unwrap();
        assert!(b.contains(10));
        assert!(!b.contains(99));

        assert!(b.remove(10));
        assert!(!b.contains(10));
        assert!(!b.remove(10));
    }

    #[test]
    fn flush_is_noop() {
        let b = make_backend();
        b.flush().unwrap();
    }

    #[test]
    fn backend_name() {
        let b = make_backend();
        assert_eq!(b.backend_name(), "hnsw");
    }

    #[test]
    fn empty_search() {
        let b = make_backend();
        let results = b.search(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    // ── Epoch tests ─────────────────────────────────────────────────────

    #[test]
    fn epoch_starts_at_zero() {
        let b = make_backend();
        assert_eq!(b.current_epoch(), 0);
    }

    #[test]
    fn epoch_increments_on_insert() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        assert_eq!(b.current_epoch(), 1);
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();
        assert_eq!(b.current_epoch(), 2);
    }

    #[test]
    fn epoch_increments_on_remove() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        assert_eq!(b.current_epoch(), 1);
        b.remove(1);
        assert_eq!(b.current_epoch(), 2);
    }

    // ── Optimistic concurrency tests ────────────────────────────────────

    #[test]
    fn insert_with_epoch_succeeds_when_current() {
        let b = make_backend();
        let epoch = b.current_epoch();
        b.insert_with_epoch(1, "a", &[1.0], serde_json::json!({}), epoch)
            .unwrap();
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn insert_with_epoch_rejects_stale() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        // Epoch is now 1. Trying with parent_epoch=0 should fail.
        let result = b.insert_with_epoch(2, "b", &[0.0], serde_json::json!({}), 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            VectorError::EpochConflict { expected, actual } => {
                assert_eq!(expected, 0);
                assert_eq!(actual, 1);
            }
            other => panic!("expected EpochConflict, got: {other}"),
        }
    }

    #[test]
    fn insert_with_epoch_accepts_current_epoch() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        let epoch = b.current_epoch(); // 1
        b.insert_with_epoch(2, "b", &[0.0], serde_json::json!({}), epoch)
            .unwrap();
        assert_eq!(b.len(), 2);
    }

    // ── Soft-delete tests ───────────────────────────────────────────────

    #[test]
    fn soft_delete_hides_from_search() {
        let b = make_backend();
        b.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        b.insert(2, "b", &[0.9, 0.1, 0.0], serde_json::json!({}))
            .unwrap();

        assert!(b.soft_delete(1));
        assert_eq!(b.tombstone_count(), 1);
        assert!(!b.contains(1));

        // Search should not return tombstoned vector.
        let results = b.search(&[1.0, 0.0, 0.0], 5);
        for r in &results {
            assert_ne!(r.id, 1, "tombstoned vector should not appear in search");
        }
    }

    #[test]
    fn soft_delete_nonexistent_returns_false() {
        let b = make_backend();
        assert!(!b.soft_delete(999));
    }

    #[test]
    fn soft_delete_already_tombstoned_returns_false() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        assert!(b.soft_delete(1));
        assert!(!b.soft_delete(1));
    }

    #[test]
    fn soft_delete_reduces_len() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();
        assert_eq!(b.len(), 2);
        b.soft_delete(1);
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn reinsert_after_soft_delete() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.soft_delete(1);
        assert_eq!(b.len(), 0);
        assert_eq!(b.tombstone_count(), 1);

        // Re-insert same id clears the tombstone.
        b.insert(1, "a", &[0.5], serde_json::json!({})).unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b.tombstone_count(), 0);
    }

    // ── Compaction tests ────────────────────────────────────────────────

    #[test]
    fn compact_purges_old_tombstones() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();

        b.soft_delete(1);
        let delete_epoch = b.current_epoch();

        b.soft_delete(2);

        // Compact tombstones older than the second soft-delete epoch.
        // Only tombstone for id=1 should be purged.
        let purged = b.compact(delete_epoch + 1);
        assert_eq!(purged, 1);
        assert_eq!(b.tombstone_count(), 1);
    }

    #[test]
    fn compact_returns_zero_when_nothing_to_purge() {
        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        assert_eq!(b.compact(100), 0);
    }

    // ── Capacity limit tests ────────────────────────────────────────────

    #[test]
    fn capacity_limit_enforced() {
        let b = HnswBackend::with_max_vectors(HnswServiceConfig::default(), 2);
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();

        let result = b.insert(3, "c", &[0.5], serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            VectorError::StoreFull { max, current } => {
                assert_eq!(max, 2);
                assert_eq!(current, 2);
            }
            other => panic!("expected StoreFull, got: {other}"),
        }
    }

    #[test]
    fn capacity_upsert_does_not_count_double() {
        let b = HnswBackend::with_max_vectors(HnswServiceConfig::default(), 2);
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();
        // Upsert existing id=1 should succeed.
        b.insert(1, "a-v2", &[0.5], serde_json::json!({})).unwrap();
    }

    #[test]
    fn capacity_default_is_none() {
        let b = make_backend();
        assert_eq!(b.max_vectors(), None);
    }

    #[test]
    fn set_max_vectors_at_runtime() {
        let b = make_backend();
        assert_eq!(b.max_vectors(), None);
        b.set_max_vectors(Some(10));
        assert_eq!(b.max_vectors(), Some(10));
        b.set_max_vectors(None);
        assert_eq!(b.max_vectors(), None);
    }

    #[test]
    fn soft_deleted_vectors_do_not_count_against_capacity() {
        let b = HnswBackend::with_max_vectors(HnswServiceConfig::default(), 2);
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();

        // Soft-delete one -- frees a slot.
        b.soft_delete(1);

        // Should now accept a new insert.
        b.insert(3, "c", &[0.5], serde_json::json!({})).unwrap();
        assert_eq!(b.len(), 2);
    }

    // ── Persistence / tombstone round-trip (WEFT-127) ───────────────────

    fn temp_backend_path(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, AtomicOrdering::Relaxed);
        std::env::temp_dir().join(format!(
            "weft127_hnsw_backend_{label}_{}_{n}.json",
            std::process::id()
        ))
    }

    #[test]
    fn soft_delete_save_load_keeps_tombstones() {
        let path = temp_backend_path("tombstone_roundtrip");
        let _ = std::fs::remove_file(&path);

        {
            let b = make_backend();
            b.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({"k": "a"}))
                .unwrap();
            b.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({"k": "b"}))
                .unwrap();
            b.insert(3, "c", &[0.0, 0.0, 1.0], serde_json::json!({"k": "c"}))
                .unwrap();

            assert!(b.soft_delete(1));
            assert_eq!(b.tombstone_count(), 1);
            assert!(!b.contains(1));
            assert_eq!(b.len(), 2);

            let epoch_before = b.current_epoch();
            b.save_to_file(&path).unwrap();

            // Sanity: epoch captured in snapshot should match.
            assert!(epoch_before > 0);
        }

        let loaded = HnswBackend::load_from_file(&path).unwrap();
        assert_eq!(loaded.tombstone_count(), 1, "tombstone must survive load");
        assert!(!loaded.contains(1), "soft-deleted id must stay gone");
        assert!(loaded.contains(2));
        assert!(loaded.contains(3));
        assert_eq!(loaded.len(), 2);

        // Search must not resurface the soft-deleted vector.
        let results = loaded.search(&[1.0, 0.0, 0.0], 5);
        for r in &results {
            assert_ne!(r.id, 1, "tombstoned vector must not appear after load");
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_legacy_store_snapshot_has_empty_tombstones() {
        // Write a plain HnswStore snapshot (no id_map / tombstones section).
        let path = temp_backend_path("legacy_compat");
        let _ = std::fs::remove_file(&path);

        {
            let mut store = HnswStore::new();
            store.insert("legacy-a".into(), vec![1.0, 0.0], serde_json::json!({}));
            store.insert("legacy-b".into(), vec![0.0, 1.0], serde_json::json!({}));
            store.save(&path).unwrap();
        }

        let loaded = HnswBackend::load_from_file(&path).unwrap();
        assert_eq!(
            loaded.tombstone_count(),
            0,
            "missing tombstone section must default to empty"
        );
        assert_eq!(loaded.current_epoch(), 0);
        assert_eq!(loaded.max_vectors(), None);
        // id_map is empty on legacy files — vectors live only in the store
        // until re-inserted through the backend API.
        assert_eq!(loaded.len(), 0);
        assert_eq!(loaded.inner().len(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn compact_physically_removes_and_survives_save() {
        let path = temp_backend_path("compact_save");
        let _ = std::fs::remove_file(&path);

        let b = make_backend();
        b.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        b.insert(2, "b", &[0.0], serde_json::json!({})).unwrap();
        b.soft_delete(1);
        let delete_epoch = b.current_epoch();

        let purged = b.compact(delete_epoch + 1);
        assert_eq!(purged, 1);
        assert_eq!(b.tombstone_count(), 0);
        assert!(!b.contains(1));
        // Physical delete from inner store.
        assert_eq!(b.inner().len(), 1);

        b.save_to_file(&path).unwrap();
        let loaded = HnswBackend::load_from_file(&path).unwrap();
        assert_eq!(loaded.tombstone_count(), 0);
        assert!(!loaded.contains(1));
        assert!(loaded.contains(2));
        assert_eq!(loaded.inner().len(), 1);

        let _ = std::fs::remove_file(&path);
    }
}
