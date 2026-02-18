//! Progressive search with three-tier recall strategy.
//!
//! Provides a [`ProgressiveSearch`] index that (in its final form) routes
//! through three layers:
//!
//! - **Layer A** -- Coarse routing (~70 % recall, microseconds)
//! - **Layer B** -- Hot-region refinement (~85 % recall)
//! - **Layer C** -- Full graph traversal (~95 % recall)
//!
//! The current implementation is a brute-force cosine similarity search that
//! serves as a drop-in stub. When the `rvf-index` crate becomes available the
//! internals will be swapped for a real HNSW graph while keeping this public
//! API stable.
//!
//! This module is gated behind the `rvf` feature flag.

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

// ── Types ──────────────────────────────────────────────────────────────

/// A single search result returned by [`ProgressiveSearch::search`].
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Identifier of the matched entry.
    pub id: String,
    /// Cosine similarity score (higher is better).
    pub score: f32,
    /// Arbitrary metadata attached to the entry.
    pub metadata: serde_json::Value,
}

/// Serializable representation of one entry (used for persistence).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    id: String,
    embedding: Vec<f32>,
    metadata: serde_json::Value,
}

// ── ProgressiveSearch ──────────────────────────────────────────────────

/// Three-tier progressive search index.
///
/// See the [module-level documentation](self) for tier descriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressiveSearch {
    entries: Vec<Entry>,
}

impl ProgressiveSearch {
    /// Create a new, empty index.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert an entry into the index.
    pub fn insert(&mut self, id: &str, embedding: Vec<f32>, metadata: serde_json::Value) {
        self.entries.push(Entry {
            id: id.to_owned(),
            embedding,
            metadata,
        });
    }

    /// Search for the `k` nearest entries to `query`.
    ///
    /// Returns results sorted by descending cosine similarity.
    ///
    /// # Stub behaviour
    ///
    /// The current implementation performs brute-force cosine similarity over
    /// all entries. This will be replaced by a real HNSW graph when
    /// `rvf-index` is available.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        if self.entries.is_empty() || k == 0 {
            return Vec::new();
        }

        let mut scored: Vec<SearchResult> = self
            .entries
            .iter()
            .map(|entry| SearchResult {
                id: entry.id.clone(),
                score: cosine_similarity(query, &entry.embedding),
                metadata: entry.metadata.clone(),
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored.truncate(k);
        scored
    }

    /// Return the number of entries in the index.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return `true` when the index contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Persist the index to `path` as JSON.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string(self).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("serialize failed: {e}"))
        })?;
        std::fs::write(path, json)
    }

    /// Load a previously saved index from `path`.
    pub fn load(path: &Path) -> io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize failed: {e}"),
            )
        })
    }
}

impl Default for ProgressiveSearch {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_search_returns_correct_top_k() {
        let mut idx = ProgressiveSearch::new();
        idx.insert("a", vec![1.0, 0.0, 0.0], serde_json::json!({"tag": "a"}));
        idx.insert("b", vec![0.0, 1.0, 0.0], serde_json::json!({"tag": "b"}));
        idx.insert("c", vec![0.7, 0.7, 0.0], serde_json::json!({"tag": "c"}));

        let results = idx.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!((results[0].score - 1.0).abs() < 0.01);
        // Second should be "c" (partial match), not "b" (orthogonal)
        assert_eq!(results[1].id, "c");
    }

    #[test]
    fn cosine_similarity_identical_vectors() {
        let score = cosine_similarity(&[0.6, 0.8], &[0.6, 0.8]);
        assert!(
            (score - 1.0).abs() < 0.001,
            "identical vectors should score ~1.0, got {score}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let score = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(
            score.abs() < 0.001,
            "orthogonal vectors should score ~0.0, got {score}"
        );
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let score = cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]);
        assert!(
            (score - (-1.0)).abs() < 0.001,
            "opposite vectors should score ~-1.0, got {score}"
        );
    }

    #[test]
    fn cosine_similarity_different_lengths() {
        let score = cosine_similarity(&[1.0, 0.0], &[1.0]);
        assert!(
            score.abs() < f32::EPSILON,
            "mismatched lengths should return 0.0"
        );
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let score = cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]);
        assert!(score.abs() < f32::EPSILON, "zero norm should return 0.0");
    }

    #[test]
    fn save_and_load_round_trip() {
        let mut idx = ProgressiveSearch::new();
        idx.insert("x", vec![1.0, 2.0, 3.0], serde_json::json!({"v": 42}));
        idx.insert("y", vec![4.0, 5.0, 6.0], serde_json::json!(null));

        let dir = std::env::temp_dir().join("clawft_progressive_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_index.json");

        idx.save(&path).unwrap();
        let loaded = ProgressiveSearch::load(&path).unwrap();

        assert_eq!(loaded.len(), 2);

        // Verify search still works identically.
        let orig_results = idx.search(&[1.0, 2.0, 3.0], 1);
        let loaded_results = loaded.search(&[1.0, 2.0, 3.0], 1);

        assert_eq!(orig_results[0].id, loaded_results[0].id);
        assert!((orig_results[0].score - loaded_results[0].score).abs() < f32::EPSILON);

        // Clean up.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_search_returns_empty() {
        let idx = ProgressiveSearch::new();
        assert!(idx.search(&[1.0, 0.0], 5).is_empty());
    }

    #[test]
    fn search_with_k_zero_returns_empty() {
        let mut idx = ProgressiveSearch::new();
        idx.insert("a", vec![1.0], serde_json::json!(null));
        assert!(idx.search(&[1.0], 0).is_empty());
    }

    #[test]
    fn len_and_is_empty() {
        let mut idx = ProgressiveSearch::new();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);

        idx.insert("a", vec![1.0], serde_json::json!(null));
        assert!(!idx.is_empty());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn default_creates_empty() {
        let idx = ProgressiveSearch::default();
        assert!(idx.is_empty());
    }

    #[test]
    fn search_preserves_metadata() {
        let mut idx = ProgressiveSearch::new();
        let meta = serde_json::json!({"source": "test", "priority": 5});
        idx.insert("m", vec![1.0, 0.0], meta.clone());

        let results = idx.search(&[1.0, 0.0], 1);
        assert_eq!(results[0].metadata, meta);
    }

    #[test]
    fn search_top_k_larger_than_store() {
        let mut idx = ProgressiveSearch::new();
        idx.insert("only", vec![1.0], serde_json::json!(null));
        let results = idx.search(&[1.0], 100);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let result = ProgressiveSearch::load(Path::new("/nonexistent/path/index.json"));
        assert!(result.is_err());
    }
}
