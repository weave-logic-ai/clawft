//! Hybrid vector backend: hot HNSW cache + cold DiskANN store.
//!
//! Implements the "map of maps" pattern: HNSW holds the frequently-accessed
//! vectors for fast lookup, while DiskANN holds the full corpus. Vectors
//! are promoted from cold to hot based on access frequency, and evicted
//! from the hot tier using LRU when it reaches capacity.
//!
//! ## Hardening features (Cognitum Seed WS1)
//!
//! Epoch, tombstone, and capacity operations are delegated to the cold
//! (authoritative) tier. The hot tier mirrors soft-deletes to keep search
//! results consistent.
//!
//! Compiled only when the `ecc` feature is enabled.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::hnsw_service::HnswServiceConfig;
use crate::vector_backend::{SearchResult, VectorBackend, VectorError, VectorResult};
use crate::vector_diskann::{DiskAnnBackend, DiskAnnConfig};
use crate::vector_hnsw::HnswBackend;

// ── Configuration ────────────────────────────────────────────────────────

/// Eviction policy for the hot tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EvictionPolicy {
    /// Least Recently Used -- evict the vector that was accessed longest ago.
    #[default]
    Lru,
}

/// Hybrid backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    /// Maximum number of vectors in the hot (HNSW) tier.
    #[serde(default = "default_hot_capacity")]
    pub hot_capacity: usize,

    /// Access count threshold before a cold vector is promoted to hot.
    #[serde(default = "default_promotion_threshold")]
    pub promotion_threshold: u32,

    /// Eviction policy for the hot tier when it is full.
    #[serde(default)]
    pub eviction_policy: EvictionPolicy,
}

fn default_hot_capacity() -> usize {
    50_000
}
fn default_promotion_threshold() -> u32 {
    3
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            hot_capacity: default_hot_capacity(),
            promotion_threshold: default_promotion_threshold(),
            eviction_policy: EvictionPolicy::default(),
        }
    }
}

// ── LRU tracker ──────────────────────────────────────────────────────────

/// Simple LRU tracker backed by a `VecDeque` of IDs.
///
/// When an ID is "touched", it moves to the back (most recent).
/// Eviction pops from the front (least recent).
struct LruTracker {
    order: VecDeque<u64>,
    positions: HashMap<u64, usize>,
}

impl LruTracker {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            positions: HashMap::new(),
        }
    }

    /// Mark an ID as recently used. If new, push to back.
    fn touch(&mut self, id: u64) {
        if self.positions.contains_key(&id) {
            // Remove from current position and push to back.
            self.order.retain(|&x| x != id);
        }
        self.order.push_back(id);
        self.rebuild_positions();
    }

    /// Evict the least recently used ID. Returns `None` if empty.
    fn evict(&mut self) -> Option<u64> {
        let id = self.order.pop_front()?;
        self.positions.remove(&id);
        self.rebuild_positions();
        Some(id)
    }

    /// Remove a specific ID from the tracker.
    fn remove(&mut self, id: u64) {
        if self.positions.remove(&id).is_some() {
            self.order.retain(|&x| x != id);
            self.rebuild_positions();
        }
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn rebuild_positions(&mut self) {
        self.positions.clear();
        for (i, &id) in self.order.iter().enumerate() {
            self.positions.insert(id, i);
        }
    }
}

// ── Stored vector snapshot (for promotion) ───────────────────────────────

/// Snapshot of a vector for transfer between tiers.
struct VectorSnapshot {
    key: String,
    vector: Vec<f32>,
    metadata: serde_json::Value,
}

// ── Cold-tier vector cache (needed for promotion) ────────────────────────

/// Cache of raw vectors stored alongside the DiskANN backend so that
/// we can promote vectors to the hot tier without re-embedding.
struct ColdVectorCache {
    vectors: HashMap<u64, VectorSnapshot>,
}

impl ColdVectorCache {
    fn new() -> Self {
        Self {
            vectors: HashMap::new(),
        }
    }

    fn insert(&mut self, id: u64, key: String, vector: Vec<f32>, metadata: serde_json::Value) {
        self.vectors.insert(
            id,
            VectorSnapshot {
                key,
                vector,
                metadata,
            },
        );
    }

    fn get(&self, id: u64) -> Option<&VectorSnapshot> {
        self.vectors.get(&id)
    }

    fn remove(&mut self, id: u64) {
        self.vectors.remove(&id);
    }
}

// ── Hybrid backend ──────────────────────────────────────────────────────

/// Hybrid vector backend combining HNSW (hot) + DiskANN (cold).
///
/// # Data flow
///
/// - **Insert**: vector goes to DiskANN (always) and HNSW (if under hot
///   capacity).
/// - **Search**: query both tiers, fuse ranks via Reciprocal Rank Fusion
///   (RRF; WEFT-661), deduplicate by numeric ID.
/// - **Promotion**: access counts are tracked per ID. When a cold-only
///   vector exceeds `promotion_threshold` accesses, it is copied into
///   the hot tier.
/// - **Eviction**: when HNSW exceeds `hot_capacity`, the LRU entry is
///   evicted (it remains in DiskANN).
pub struct HybridBackend {
    hot: HnswBackend,
    cold: DiskAnnBackend,
    config: HybridConfig,
    /// Per-ID access counters for promotion decisions.
    access_counts: DashMap<u64, u32>,
    /// LRU tracker for the hot tier (protected by Mutex for ordered ops).
    lru: Mutex<LruTracker>,
    /// Cold vector cache for promotion (protected by Mutex).
    cold_cache: Mutex<ColdVectorCache>,
    /// Set of IDs currently in the hot tier.
    hot_ids: DashMap<u64, ()>,
}

impl HybridBackend {
    /// Create a new hybrid backend.
    pub fn new(
        hnsw_config: HnswServiceConfig,
        diskann_config: DiskAnnConfig,
        hybrid_config: HybridConfig,
    ) -> Self {
        Self {
            hot: HnswBackend::new(hnsw_config),
            cold: DiskAnnBackend::new(diskann_config),
            config: hybrid_config,
            access_counts: DashMap::new(),
            lru: Mutex::new(LruTracker::new()),
            cold_cache: Mutex::new(ColdVectorCache::new()),
            hot_ids: DashMap::new(),
        }
    }

    /// Create with all-default configurations.
    pub fn with_defaults() -> Self {
        Self::new(
            HnswServiceConfig::default(),
            DiskAnnConfig::default(),
            HybridConfig::default(),
        )
    }

    /// Borrow the hybrid configuration.
    pub fn config(&self) -> &HybridConfig {
        &self.config
    }

    /// Return the number of vectors in the hot tier.
    pub fn hot_len(&self) -> usize {
        self.hot_ids.len()
    }

    /// Return the number of vectors in the cold tier.
    pub fn cold_len(&self) -> usize {
        self.cold.len()
    }

    /// Try to promote a vector from cold to hot tier.
    fn try_promote(&self, id: u64) {
        // Already in hot tier?
        if self.hot_ids.contains_key(&id) {
            return;
        }

        // Check access count.
        let count = self.access_counts.get(&id).map(|v| *v).unwrap_or(0);

        if count < self.config.promotion_threshold {
            return;
        }

        // Get the vector from cold cache.
        let snapshot = {
            let cache = self.cold_cache.lock().expect("cold_cache lock poisoned");
            cache.get(id).map(|s| VectorSnapshot {
                key: s.key.clone(),
                vector: s.vector.clone(),
                metadata: s.metadata.clone(),
            })
        };

        let Some(snap) = snapshot else { return };

        // Evict from hot tier if at capacity.
        self.ensure_hot_capacity();

        // Insert into hot tier.
        if self
            .hot
            .insert(id, &snap.key, &snap.vector, snap.metadata)
            .is_ok()
        {
            self.hot_ids.insert(id, ());
            let mut lru = self.lru.lock().expect("LRU lock poisoned");
            lru.touch(id);
        }
    }

    /// Ensure the hot tier has room for one more entry.
    fn ensure_hot_capacity(&self) {
        let mut lru = self.lru.lock().expect("LRU lock poisoned");
        while lru.len() >= self.config.hot_capacity {
            if let Some(evict_id) = lru.evict() {
                self.hot.remove(evict_id);
                self.hot_ids.remove(&evict_id);
            } else {
                break;
            }
        }
    }

    /// Increment the access counter for a given ID.
    fn record_access(&self, id: u64) {
        self.access_counts
            .entry(id)
            .and_modify(|c| {
                *c = c.saturating_add(1);
            })
            .or_insert(1);

        // Touch LRU if in hot tier.
        if self.hot_ids.contains_key(&id) {
            let mut lru = self.lru.lock().expect("LRU lock poisoned");
            lru.touch(id);
        }
    }

    /// Cormack et al. RRF smoothing constant (standard IR default).
    ///
    /// Higher `k` reduces the weight gap between rank-1 and rank-N.
    const RRF_K: f32 = 60.0;

    /// Merge and deduplicate search results from hot and cold tiers.
    ///
    /// # Ranking policy — Reciprocal Rank Fusion (WEFT-661)
    ///
    /// Hot HNSW returns **cosine** distances (~[0, 2]); real DiskANN cold
    /// returns **squared Euclidean** (L2²) on a different scale. Sorting
    /// raw distances lets cosine hits structurally dominate regardless of
    /// true proximity (bench recall@10 ≈ 0.113). RRF ranks each list
    /// independently and fuses with:
    ///
    /// ```text
    /// score(id) = Σ_lists  1 / (RRF_K + rank_list(id))   // rank is 0-based
    /// ```
    ///
    /// so metric scale is irrelevant. Higher score wins.
    ///
    /// # Dedup (WEFT-660)
    ///
    /// Dedup key is the numeric [`SearchResult::id`]. An id present in both
    /// lists accumulates RRF mass from both ranks. The retained payload
    /// prefers the **hot** entry (fresher key/metadata); cold fills only
    /// when the id is cold-only. Original tier `distance` is kept for
    /// diagnostics — callers must not assume a single metric after merge.
    fn merge_results(
        hot_results: Vec<SearchResult>,
        cold_results: Vec<SearchResult>,
        k: usize,
    ) -> Vec<SearchResult> {
        if k == 0 {
            return Vec::new();
        }

        // id → fused RRF score (higher = better)
        let mut rrf: HashMap<u64, f32> = HashMap::new();
        // id → retained SearchResult (hot wins on first insert)
        let mut by_id: HashMap<u64, SearchResult> = HashMap::new();

        for (rank, r) in hot_results.into_iter().enumerate() {
            *rrf.entry(r.id).or_insert(0.0) += 1.0 / (Self::RRF_K + rank as f32);
            by_id.entry(r.id).or_insert(r);
        }
        for (rank, r) in cold_results.into_iter().enumerate() {
            *rrf.entry(r.id).or_insert(0.0) += 1.0 / (Self::RRF_K + rank as f32);
            // Prefer hot payload when both tiers returned the same id.
            by_id.entry(r.id).or_insert(r);
        }

        let mut merged: Vec<SearchResult> = by_id.into_values().collect();
        // RRF desc, then native distance asc, then id for determinism.
        merged.sort_by(|a, b| {
            let sa = rrf.get(&a.id).copied().unwrap_or(0.0);
            let sb = rrf.get(&b.id).copied().unwrap_or(0.0);
            sb.partial_cmp(&sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.distance
                        .partial_cmp(&b.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.id.cmp(&b.id))
        });
        merged.truncate(k);
        merged
    }
}

impl VectorBackend for HybridBackend {
    fn insert(
        &self,
        id: u64,
        key: &str,
        vector: &[f32],
        metadata: serde_json::Value,
    ) -> VectorResult<()> {
        // Always insert into cold tier (authoritative for capacity).
        self.cold.insert(id, key, vector, metadata.clone())?;

        // Cache the raw vector for future promotion.
        {
            let mut cache = self.cold_cache.lock().expect("cold_cache lock poisoned");
            cache.insert(id, key.to_owned(), vector.to_vec(), metadata.clone());
        }

        // Insert into hot tier if under capacity.
        if self.hot_ids.len() < self.config.hot_capacity {
            self.hot.insert(id, key, vector, metadata)?;
            self.hot_ids.insert(id, ());
            let mut lru = self.lru.lock().expect("LRU lock poisoned");
            lru.touch(id);
        }

        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        // Search both tiers.
        let hot_results = self.hot.search(query, k);
        let cold_results = self.cold.search(query, k);

        let merged = Self::merge_results(hot_results, cold_results, k);

        // Record access for all returned IDs and potentially promote.
        for r in &merged {
            self.record_access(r.id);
            self.try_promote(r.id);
        }

        merged
    }

    fn len(&self) -> usize {
        // Total unique vectors = cold tier (which holds everything).
        self.cold.len()
    }

    fn contains(&self, id: u64) -> bool {
        self.cold.contains(id)
    }

    fn remove(&self, id: u64) -> bool {
        // Remove from both tiers.
        let hot_removed = self.hot.remove(id);
        if hot_removed {
            self.hot_ids.remove(&id);
            let mut lru = self.lru.lock().expect("LRU lock poisoned");
            lru.remove(id);
        }
        let cold_removed = self.cold.remove(id);
        self.access_counts.remove(&id);
        {
            let mut cache = self.cold_cache.lock().expect("cold_cache lock poisoned");
            cache.remove(id);
        }
        cold_removed
    }

    fn flush(&self) -> VectorResult<()> {
        self.hot.flush()?;
        self.cold.flush()?;
        Ok(())
    }

    fn backend_name(&self) -> &str {
        "hybrid"
    }

    // ── Epoch (delegated to cold tier as source of truth) ───────────────

    fn current_epoch(&self) -> u64 {
        self.cold.current_epoch()
    }

    fn insert_with_epoch(
        &self,
        id: u64,
        key: &str,
        vector: &[f32],
        metadata: serde_json::Value,
        parent_epoch: u64,
    ) -> VectorResult<()> {
        let current = self.cold.current_epoch();
        if parent_epoch < current {
            return Err(VectorError::EpochConflict {
                expected: parent_epoch,
                actual: current,
            });
        }
        self.insert(id, key, vector, metadata)
    }

    // ── Soft-delete + compaction (mirrored to both tiers) ───────────────

    fn soft_delete(&self, id: u64) -> bool {
        // Soft-delete in cold (authoritative).
        let deleted = self.cold.soft_delete(id);
        if deleted {
            // Also soft-delete in hot tier so searches exclude it.
            self.hot.soft_delete(id);
            if self.hot_ids.contains_key(&id) {
                self.hot_ids.remove(&id);
                let mut lru = self.lru.lock().expect("LRU lock poisoned");
                lru.remove(id);
            }
        }
        deleted
    }

    fn compact(&self, older_than_epoch: u64) -> usize {
        // Compact cold tier (authoritative).
        let purged = self.cold.compact(older_than_epoch);
        // Also compact hot tier to stay in sync.
        self.hot.compact(older_than_epoch);
        purged
    }

    fn tombstone_count(&self) -> usize {
        self.cold.tombstone_count()
    }

    // ── Capacity limits (delegated to cold tier) ────────────────────────

    fn max_vectors(&self) -> Option<usize> {
        self.cold.max_vectors()
    }

    fn set_max_vectors(&self, limit: Option<usize>) {
        self.cold.set_max_vectors(limit);
    }
}

impl std::fmt::Debug for HybridBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridBackend")
            .field("hot_len", &self.hot_len())
            .field("cold_len", &self.cold_len())
            .field("epoch", &self.current_epoch())
            .field("tombstones", &self.tombstone_count())
            .field("config", &self.config)
            .finish()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn small_hybrid() -> HybridBackend {
        HybridBackend::new(
            HnswServiceConfig::default(),
            DiskAnnConfig {
                max_points: 1000,
                ..DiskAnnConfig::default()
            },
            HybridConfig {
                hot_capacity: 3,
                promotion_threshold: 2,
                eviction_policy: EvictionPolicy::Lru,
            },
        )
    }

    #[test]
    fn insert_populates_both_tiers_under_capacity() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({}))
            .unwrap();

        assert_eq!(h.len(), 2);
        assert_eq!(h.hot_len(), 2);
        assert_eq!(h.cold_len(), 2);
    }

    #[test]
    fn insert_beyond_hot_capacity_stays_cold_only() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(3, "c", &[0.0, 0.0, 1.0], serde_json::json!({}))
            .unwrap();

        // 4th vector exceeds hot capacity.
        h.insert(4, "d", &[0.5, 0.5, 0.0], serde_json::json!({}))
            .unwrap();

        assert_eq!(h.len(), 4);
        assert_eq!(h.hot_len(), 3);
        assert_eq!(h.cold_len(), 4);
    }

    #[test]
    fn search_returns_merged_results() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({}))
            .unwrap();

        let results = h.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
        assert!(results[0].distance < 0.01);
    }

    #[test]
    fn promotion_after_threshold() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(3, "c", &[0.0, 0.0, 1.0], serde_json::json!({}))
            .unwrap();

        // Insert cold-only vector.
        h.insert(4, "d", &[0.5, 0.5, 0.0], serde_json::json!({}))
            .unwrap();
        assert!(!h.hot_ids.contains_key(&4));

        h.record_access(4);
        h.record_access(4);
        h.try_promote(4);

        assert!(h.hot_ids.contains_key(&4));
        assert_eq!(h.hot_len(), 3);
    }

    #[test]
    fn remove_cleans_both_tiers() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0, 0.0], serde_json::json!({}))
            .unwrap();
        assert!(h.contains(1));
        assert!(h.remove(1));
        assert!(!h.contains(1));
        assert_eq!(h.hot_len(), 0);
        assert_eq!(h.cold_len(), 0);
    }

    #[test]
    fn merge_results_deduplicates() {
        let hot = vec![
            SearchResult::new(1, "a".into(), 0.1, serde_json::json!({})),
            SearchResult::new(2, "b".into(), 0.3, serde_json::json!({})),
        ];
        let cold = vec![
            SearchResult::new(1, "a".into(), 0.1, serde_json::json!({})),
            SearchResult::new(3, "c".into(), 0.2, serde_json::json!({})),
        ];

        let merged = HybridBackend::merge_results(hot, cold, 3);
        assert_eq!(merged.len(), 3);
        // id=1 is in both lists → highest RRF (1/60 + 1/60).
        assert_eq!(merged[0].id, 1);
        // id=2 and id=3 both rank-1 in one list (score 1/61); distance
        // tie-break: 0.2 < 0.3 → cold id=3 before hot id=2.
        assert_eq!(merged[1].id, 3);
        assert_eq!(merged[2].id, 2);
    }

    /// WEFT-661: cosine-scale hot distances must not drown L2² cold ranks.
    #[test]
    fn merge_results_rrf_not_raw_distance() {
        // Hot: tiny cosine-like distances. Cold: large L2²-like distances.
        // Raw-distance sort would return only hot ids for k=2.
        let hot = vec![
            SearchResult::new(10, "hot0".into(), 0.001, serde_json::Value::Null),
            SearchResult::new(11, "hot1".into(), 0.002, serde_json::Value::Null),
        ];
        let cold = vec![
            SearchResult::new(20, "cold0".into(), 100.0, serde_json::Value::Null),
            SearchResult::new(21, "cold1".into(), 200.0, serde_json::Value::Null),
        ];

        let merged = HybridBackend::merge_results(hot, cold, 2);
        assert_eq!(merged.len(), 2);
        let ids: std::collections::HashSet<u64> = merged.iter().map(|r| r.id).collect();
        // Rank-0 from each list share the top RRF band → both tiers in top-2.
        assert!(ids.contains(&10), "hot rank-0 must survive RRF: {ids:?}");
        assert!(ids.contains(&20), "cold rank-0 must survive RRF: {ids:?}");
    }

    /// WEFT-660/661: same numeric id from both tiers is one hit; hot payload wins.
    #[test]
    fn merge_results_dedup_prefers_hot_payload() {
        let hot = vec![SearchResult::new(
            42,
            "hot-key".into(),
            0.05,
            serde_json::json!({"tier": "hot"}),
        )];
        let cold = vec![SearchResult::new(
            42,
            "cold-key".into(),
            50.0,
            serde_json::json!({"tier": "cold"}),
        )];

        let merged = HybridBackend::merge_results(hot, cold, 1);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, 42);
        assert_eq!(merged[0].key, "hot-key");
        assert_eq!(merged[0].metadata, serde_json::json!({"tier": "hot"}));
        // Native hot distance retained (not rewritten to RRF).
        assert!((merged[0].distance - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn merge_results_empty_and_k_zero() {
        assert!(HybridBackend::merge_results(vec![], vec![], 5).is_empty());
        let hot = vec![SearchResult::new(1, "a".into(), 0.1, serde_json::Value::Null)];
        assert!(HybridBackend::merge_results(hot, vec![], 0).is_empty());
    }

    #[test]
    fn merge_results_cold_only_ids_survive() {
        let hot = vec![SearchResult::new(1, "a".into(), 0.01, serde_json::Value::Null)];
        let cold = vec![
            SearchResult::new(2, "b".into(), 99.0, serde_json::Value::Null),
            SearchResult::new(3, "c".into(), 100.0, serde_json::Value::Null),
        ];
        let merged = HybridBackend::merge_results(hot, cold, 3);
        assert_eq!(merged.len(), 3);
        let ids: std::collections::HashSet<u64> = merged.iter().map(|r| r.id).collect();
        assert_eq!(ids, [1, 2, 3].into_iter().collect());
    }

    #[test]
    fn backend_name() {
        let h = small_hybrid();
        assert_eq!(h.backend_name(), "hybrid");
    }

    #[test]
    fn flush_both_tiers() {
        let h = small_hybrid();
        h.flush().unwrap();
    }

    #[test]
    fn eviction_policy_default() {
        assert_eq!(EvictionPolicy::default(), EvictionPolicy::Lru);
    }

    #[test]
    fn lru_tracker_basics() {
        let mut lru = LruTracker::new();
        lru.touch(1);
        lru.touch(2);
        lru.touch(3);
        assert_eq!(lru.len(), 3);

        lru.touch(1);
        assert_eq!(lru.evict(), Some(2));
        assert_eq!(lru.len(), 2);
    }

    // ── Epoch tests ─────────────────────────────────────────────────────

    #[test]
    fn hybrid_epoch_tracks_cold_tier() {
        let h = small_hybrid();
        assert_eq!(h.current_epoch(), 0);
        h.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        assert!(h.current_epoch() > 0);
    }

    #[test]
    fn hybrid_insert_with_epoch_rejects_stale() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        let result = h.insert_with_epoch(2, "b", &[0.0], serde_json::json!({}), 0);
        assert!(matches!(result, Err(VectorError::EpochConflict { .. })));
    }

    // ── Soft-delete tests ───────────────────────────────────────────────

    #[test]
    fn hybrid_soft_delete_hides_from_both_tiers() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0, 0.0, 0.0], serde_json::json!({}))
            .unwrap();
        h.insert(2, "b", &[0.0, 1.0, 0.0], serde_json::json!({}))
            .unwrap();

        assert!(h.soft_delete(1));
        assert_eq!(h.tombstone_count(), 1);
        assert_eq!(h.len(), 1);
        assert!(!h.contains(1));
    }

    #[test]
    fn hybrid_compact() {
        let h = small_hybrid();
        h.insert(1, "a", &[1.0], serde_json::json!({})).unwrap();
        h.soft_delete(1);
        let epoch = h.current_epoch();
        let purged = h.compact(epoch + 1);
        assert_eq!(purged, 1);
        assert_eq!(h.tombstone_count(), 0);
    }

    // ── Capacity tests ──────────────────────────────────────────────────

    #[test]
    fn hybrid_capacity_delegates_to_cold() {
        let h = small_hybrid();
        assert_eq!(h.max_vectors(), Some(1000));
        h.set_max_vectors(Some(5));
        assert_eq!(h.max_vectors(), Some(5));
    }
}
