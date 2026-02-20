//! HNSW-backed vector store using `instant-distance`.
//!
//! Provides [`HnswStore`], a high-performance approximate nearest neighbor
//! store that wraps the `instant-distance` HNSW implementation. When the
//! entry count is small (below [`HNSW_THRESHOLD`]), queries fall back to
//! brute-force cosine similarity for correctness. Above the threshold,
//! queries route through the HNSW graph for sub-linear search.
//!
//! The store supports persistence via JSON serialization: entries are
//! stored alongside the store metadata so the HNSW graph can be rebuilt
//! on load.
//!
//! This module is gated behind the `vector-memory` feature flag.

use std::path::Path;

use instant_distance::{Builder, HnswMap, Point, Search};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Minimum number of entries before the HNSW index is built.
///
/// Below this threshold, brute-force cosine similarity is used because
/// building an HNSW graph for very few points has no benefit.
const HNSW_THRESHOLD: usize = 32;

/// Default ef_search parameter for HNSW queries.
///
/// Higher values trade latency for recall. 100 gives ~95%+ recall
/// on typical workloads.
const DEFAULT_EF_SEARCH: usize = 100;

/// Default ef_construction parameter for building the HNSW graph.
const DEFAULT_EF_CONSTRUCTION: usize = 200;

// ── Point wrapper ──────────────────────────────────────────────────────

/// Wrapper around an embedding vector that implements [`Point`].
///
/// Distance is `1.0 - cosine_similarity`, so that closer points have
/// smaller distances (as expected by the HNSW algorithm).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmbeddingPoint {
    embedding: Vec<f32>,
}

impl Point for EmbeddingPoint {
    fn distance(&self, other: &Self) -> f32 {
        1.0 - cosine_similarity(&self.embedding, &other.embedding)
    }
}

// ── Entry ──────────────────────────────────────────────────────────────

/// A single entry stored in the HNSW store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswEntry {
    /// Unique identifier for this entry.
    pub id: String,
    /// The embedding vector.
    pub embedding: Vec<f32>,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

/// A query result from the HNSW store.
#[derive(Debug, Clone)]
pub struct HnswQueryResult {
    /// The entry ID.
    pub id: String,
    /// Cosine similarity score (higher is better).
    pub score: f32,
    /// The entry metadata.
    pub metadata: serde_json::Value,
}

// ── Serializable state ─────────────────────────────────────────────────

/// Serializable snapshot of the store (entries only; the HNSW graph is
/// rebuilt on load).
#[derive(Debug, Serialize, Deserialize)]
struct StoreSnapshot {
    entries: Vec<HnswEntry>,
    ef_search: usize,
    ef_construction: usize,
}

// ── HnswStore ──────────────────────────────────────────────────────────

/// HNSW-backed vector store with automatic fallback to brute-force
/// for small datasets.
///
/// # Usage
///
/// ```rust,no_run
/// use clawft_core::embeddings::hnsw_store::HnswStore;
///
/// let mut store = HnswStore::new();
/// store.insert("doc1".into(), vec![1.0, 0.0, 0.0], serde_json::json!({"text": "hello"}));
/// let results = store.query(&[1.0, 0.0, 0.0], 5);
/// ```
pub struct HnswStore {
    /// All entries (source of truth).
    entries: Vec<HnswEntry>,
    /// The HNSW index, rebuilt when entries change above the threshold.
    index: Option<HnswMap<EmbeddingPoint, usize>>,
    /// ef_search parameter for queries.
    ef_search: usize,
    /// ef_construction parameter for building.
    ef_construction: usize,
    /// Whether the index is stale (entries changed since last build).
    dirty: bool,
}

impl HnswStore {
    /// Create a new, empty HNSW store with default parameters.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: None,
            ef_search: DEFAULT_EF_SEARCH,
            ef_construction: DEFAULT_EF_CONSTRUCTION,
            dirty: false,
        }
    }

    /// Create a store with custom HNSW parameters.
    pub fn with_params(ef_search: usize, ef_construction: usize) -> Self {
        Self {
            entries: Vec::new(),
            index: None,
            ef_search,
            ef_construction,
            dirty: false,
        }
    }

    /// Insert an entry into the store.
    ///
    /// Uses upsert semantics: if an entry with the same ID already exists,
    /// it is replaced.
    pub fn insert(
        &mut self,
        id: String,
        embedding: Vec<f32>,
        metadata: serde_json::Value,
    ) {
        // Remove existing entry with the same ID (upsert).
        self.entries.retain(|e| e.id != id);
        self.entries.push(HnswEntry {
            id,
            embedding,
            metadata,
        });
        self.dirty = true;
    }

    /// Query the store for the top-k most similar entries.
    ///
    /// Returns results sorted by descending cosine similarity.
    pub fn query(
        &mut self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Vec<HnswQueryResult> {
        if self.entries.is_empty() || top_k == 0 {
            return Vec::new();
        }

        // Use brute-force for small datasets.
        if self.entries.len() < HNSW_THRESHOLD {
            return self.brute_force_query(query_embedding, top_k);
        }

        // Rebuild the HNSW index if stale.
        if self.dirty || self.index.is_none() {
            self.rebuild_index();
        }

        // If index build failed (shouldn't happen), fall back.
        let Some(ref index) = self.index else {
            return self.brute_force_query(query_embedding, top_k);
        };

        let query_point = EmbeddingPoint {
            embedding: query_embedding.to_vec(),
        };
        let mut search = Search::default();

        let mut results: Vec<HnswQueryResult> = index
            .search(&query_point, &mut search)
            .take(top_k)
            .map(|item| {
                let idx = *item.value;
                let entry = &self.entries[idx];
                HnswQueryResult {
                    id: entry.id.clone(),
                    score: cosine_similarity(query_embedding, &entry.embedding),
                    metadata: entry.metadata.clone(),
                }
            })
            .collect();

        // The HNSW iterator returns by ascending distance, so results
        // are already roughly ordered. Re-sort by descending score for
        // consistency with brute-force.
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Delete an entry by ID. Returns `true` if removed.
    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        let removed = self.entries.len() < before;
        if removed {
            self.dirty = true;
        }
        removed
    }

    /// Return the number of entries in the store.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return `true` if the store has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get an entry by ID.
    pub fn get(&self, id: &str) -> Option<&HnswEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Persist the store to a JSON file.
    ///
    /// Only entries and parameters are saved. The HNSW graph is rebuilt
    /// on load.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let snapshot = StoreSnapshot {
            entries: self.entries.clone(),
            ef_search: self.ef_search,
            ef_construction: self.ef_construction,
        };

        let json = serde_json::to_string_pretty(&snapshot).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        std::fs::write(path, json)
    }

    /// Load a store from a JSON file.
    ///
    /// If the file does not exist, returns a new empty store.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let data = std::fs::read_to_string(path)?;
        let snapshot: StoreSnapshot =
            serde_json::from_str(&data).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
            })?;

        debug!(
            entries = snapshot.entries.len(),
            ef_search = snapshot.ef_search,
            "loaded HnswStore from disk"
        );

        let mut store = Self {
            entries: snapshot.entries,
            index: None,
            ef_search: snapshot.ef_search,
            ef_construction: snapshot.ef_construction,
            dirty: true,
        };

        // Pre-build the index if above threshold.
        if store.entries.len() >= HNSW_THRESHOLD {
            store.rebuild_index();
        }

        Ok(store)
    }

    /// Force a rebuild of the HNSW index.
    ///
    /// Called automatically on query when the index is stale.
    pub fn rebuild_index(&mut self) {
        if self.entries.is_empty() {
            self.index = None;
            self.dirty = false;
            return;
        }

        let points: Vec<EmbeddingPoint> = self
            .entries
            .iter()
            .map(|e| EmbeddingPoint {
                embedding: e.embedding.clone(),
            })
            .collect();

        let values: Vec<usize> = (0..self.entries.len()).collect();

        debug!(
            entries = self.entries.len(),
            ef_construction = self.ef_construction,
            ef_search = self.ef_search,
            "rebuilding HNSW index"
        );

        let map = Builder::default()
            .ef_search(self.ef_search)
            .ef_construction(self.ef_construction)
            .build(points, values);

        self.index = Some(map);
        self.dirty = false;
    }

    /// Brute-force cosine similarity search (fallback for small datasets).
    fn brute_force_query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Vec<HnswQueryResult> {
        let mut scored: Vec<HnswQueryResult> = self
            .entries
            .iter()
            .map(|entry| HnswQueryResult {
                id: entry.id.clone(),
                score: cosine_similarity(query_embedding, &entry.embedding),
                metadata: entry.metadata.clone(),
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }
}

impl Default for HnswStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute cosine similarity between two vectors.
///
/// Returns `0.0` when either vector has zero norm or when lengths differ.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!(
            "clawft_hnsw_test_{label}_{pid}_{n}.json"
        ))
    }

    #[test]
    fn create_empty_store() {
        let store = HnswStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn insert_and_brute_force_query() {
        let mut store = HnswStore::new();
        store.insert(
            "doc1".into(),
            vec![1.0, 0.0, 0.0],
            serde_json::json!({"text": "hello"}),
        );
        store.insert(
            "doc2".into(),
            vec![0.0, 1.0, 0.0],
            serde_json::json!({"text": "world"}),
        );

        let results = store.query(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!((results[0].score - 1.0).abs() < 0.01);
    }

    #[test]
    fn query_ordering() {
        let mut store = HnswStore::new();
        store.insert("a".into(), vec![1.0, 0.0], serde_json::json!({}));
        store.insert("b".into(), vec![0.7, 0.7], serde_json::json!({}));
        store.insert("c".into(), vec![0.0, 1.0], serde_json::json!({}));

        let results = store.query(&[1.0, 0.0], 3);
        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "b");
        assert_eq!(results[2].id, "c");
    }

    #[test]
    fn upsert_semantics() {
        let mut store = HnswStore::new();
        store.insert(
            "doc1".into(),
            vec![1.0, 0.0],
            serde_json::json!({"v": 1}),
        );
        store.insert(
            "doc1".into(),
            vec![0.0, 1.0],
            serde_json::json!({"v": 2}),
        );

        assert_eq!(store.len(), 1);
        let entry = store.get("doc1").unwrap();
        assert_eq!(entry.metadata["v"], 2);
        assert_eq!(entry.embedding, vec![0.0, 1.0]);
    }

    #[test]
    fn delete_entry() {
        let mut store = HnswStore::new();
        store.insert("doc1".into(), vec![1.0], serde_json::json!({}));
        store.insert("doc2".into(), vec![0.0], serde_json::json!({}));

        assert!(store.delete("doc1"));
        assert_eq!(store.len(), 1);
        assert!(!store.delete("doc1"));
    }

    #[test]
    fn query_empty_store() {
        let mut store = HnswStore::new();
        let results = store.query(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn query_top_k_zero() {
        let mut store = HnswStore::new();
        store.insert("x".into(), vec![1.0], serde_json::json!({}));
        let results = store.query(&[1.0], 0);
        assert!(results.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_path("roundtrip");

        {
            let mut store = HnswStore::new();
            store.insert(
                "doc1".into(),
                vec![1.0, 0.0, 0.0],
                serde_json::json!({"text": "hello"}),
            );
            store.insert(
                "doc2".into(),
                vec![0.0, 1.0, 0.0],
                serde_json::json!({"text": "world"}),
            );
            store.save(&path).unwrap();
        }

        let mut store = HnswStore::load(&path).unwrap();
        assert_eq!(store.len(), 2);

        let results = store.query(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results[0].id, "doc1");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let path = temp_path("nonexist");
        let _ = std::fs::remove_file(&path);

        let store = HnswStore::load(&path).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn hnsw_index_triggers_above_threshold() {
        let mut store = HnswStore::new();
        // Insert enough entries to trigger HNSW indexing.
        for i in 0..HNSW_THRESHOLD + 5 {
            let dim = 16;
            let mut emb = vec![0.0f32; dim];
            emb[i % dim] = 1.0;
            store.insert(
                format!("doc{i}"),
                emb,
                serde_json::json!({"idx": i}),
            );
        }

        // Query should use HNSW path and return correct results.
        let mut query = vec![0.0f32; 16];
        query[0] = 1.0;

        let results = store.query(&query, 3);
        assert!(!results.is_empty());
        // The exact top match depends on how many entries share
        // dimension 0, but we should get results.
        assert!(results.len() <= 3);
    }

    #[test]
    fn get_entry_by_id() {
        let mut store = HnswStore::new();
        store.insert(
            "doc1".into(),
            vec![1.0, 2.0],
            serde_json::json!({"key": "value"}),
        );

        let entry = store.get("doc1");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().id, "doc1");

        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn default_creates_empty() {
        let store = HnswStore::default();
        assert!(store.is_empty());
    }

    #[test]
    fn cosine_similarity_identical() {
        let score = cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let score = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(score.abs() < 0.01);
    }

    #[test]
    fn cosine_similarity_different_lengths() {
        let score = cosine_similarity(&[1.0, 0.0], &[1.0]);
        assert!((score - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn with_params_custom_values() {
        let store = HnswStore::with_params(50, 100);
        assert_eq!(store.ef_search, 50);
        assert_eq!(store.ef_construction, 100);
        assert!(store.is_empty());
    }

    #[test]
    fn hnsw_query_consistency_with_brute_force() {
        // Verify that the HNSW index returns the same top-1 as brute-force
        // for a simple dataset above threshold.
        let dim = 8;
        let n = HNSW_THRESHOLD + 10;
        let mut store = HnswStore::new();

        // Create entries with known embeddings.
        for i in 0..n {
            let mut emb = vec![0.0f32; dim];
            emb[i % dim] = 1.0;
            // Add a small perturbation so entries aren't identical.
            for (j, val) in emb.iter_mut().enumerate() {
                *val += (i as f32 * 0.001) * ((j + 1) as f32);
            }
            store.insert(format!("d{i}"), emb, serde_json::json!({}));
        }

        // Query for the first entry's embedding.
        let target = store.get("d0").unwrap().embedding.clone();
        let results = store.query(&target, 1);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "d0");
        assert!(results[0].score > 0.9);
    }
}
