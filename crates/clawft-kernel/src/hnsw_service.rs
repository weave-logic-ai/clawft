//! HNSW vector search as a kernel `SystemService`.
//!
//! Wraps `clawft_core::embeddings::hnsw_store::HnswStore` behind a
//! `Mutex` so that the service satisfies `Send + Sync` and can be
//! registered in the `ServiceRegistry`.
//!
//! This module is compiled only when the `ecc` feature is enabled.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use clawft_core::embeddings::hnsw_store::HnswStore;

use crate::health::HealthStatus;
use crate::service::{ServiceType, SystemService};

// ── Configuration ────────────────────────────────────────────────────────

/// Configuration for the [`HnswService`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswServiceConfig {
    /// ef_search parameter forwarded to `HnswStore`.
    pub ef_search: usize,
    /// ef_construction parameter forwarded to `HnswStore`.
    pub ef_construction: usize,
    /// Default embedding dimensionality (informational; not enforced by the store).
    pub default_dimensions: usize,
}

impl Default for HnswServiceConfig {
    fn default() -> Self {
        Self {
            ef_search: 100,
            ef_construction: 200,
            default_dimensions: 384,
        }
    }
}

// ── Search result ────────────────────────────────────────────────────────

/// A single search result returned by [`HnswService::search`].
#[derive(Debug, Clone)]
pub struct HnswSearchResult {
    /// Entry identifier.
    pub id: String,
    /// Cosine similarity score (higher is better).
    pub score: f32,
    /// Arbitrary metadata stored alongside the embedding.
    pub metadata: serde_json::Value,
}

// ── Service ──────────────────────────────────────────────────────────────

/// Kernel service wrapping the HNSW vector store.
///
/// All mutable access to the inner `HnswStore` is serialized through a
/// [`Mutex`]. Atomic counters track insert and search operations for
/// observability without requiring the lock.
pub struct HnswService {
    store: Mutex<HnswStore>,
    config: HnswServiceConfig,
    insert_count: AtomicU64,
    search_count: AtomicU64,
}

impl HnswService {
    /// Create a new service with the given configuration.
    pub fn new(config: HnswServiceConfig) -> Self {
        let store = HnswStore::with_params(config.ef_search, config.ef_construction);
        Self {
            store: Mutex::new(store),
            config,
            insert_count: AtomicU64::new(0),
            search_count: AtomicU64::new(0),
        }
    }

    /// Insert an embedding with associated metadata (upsert semantics).
    pub fn insert(&self, id: String, embedding: Vec<f32>, metadata: serde_json::Value) {
        let mut store = self.store.lock().expect("HnswStore lock poisoned");
        store.insert(id, embedding, metadata);
        self.insert_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Search for the `top_k` nearest embeddings to `query`.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<HnswSearchResult> {
        let mut store = self.store.lock().expect("HnswStore lock poisoned");
        self.search_count.fetch_add(1, Ordering::Relaxed);
        store
            .query(query, top_k)
            .into_iter()
            .map(|r| HnswSearchResult {
                id: r.id,
                score: r.score,
                metadata: r.metadata,
            })
            .collect()
    }

    /// Return the number of entries currently in the store.
    pub fn len(&self) -> usize {
        let store = self.store.lock().expect("HnswStore lock poisoned");
        store.len()
    }

    /// Return `true` if the store contains no entries.
    pub fn is_empty(&self) -> bool {
        let store = self.store.lock().expect("HnswStore lock poisoned");
        store.is_empty()
    }

    /// Total number of insert operations since service creation.
    pub fn insert_count(&self) -> u64 {
        self.insert_count.load(Ordering::Relaxed)
    }

    /// Total number of search operations since service creation.
    pub fn search_count(&self) -> u64 {
        self.search_count.load(Ordering::Relaxed)
    }

    /// Replace the inner store with a fresh, empty instance.
    ///
    /// Useful for calibration cleanup. Counters are **not** reset.
    pub fn clear(&self) {
        let mut store = self.store.lock().expect("HnswStore lock poisoned");
        *store = HnswStore::with_params(self.config.ef_search, self.config.ef_construction);
    }

    /// Borrow the service configuration.
    pub fn config(&self) -> &HnswServiceConfig {
        &self.config
    }
}

// ── SystemService impl ──────────────────────────────────────────────────

#[async_trait]
impl SystemService for HnswService {
    fn name(&self) -> &str {
        "ecc.hnsw"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> HnswService {
        HnswService::new(HnswServiceConfig::default())
    }

    #[test]
    fn new_service_empty() {
        let svc = make_service();
        assert!(svc.is_empty());
        assert_eq!(svc.len(), 0);
    }

    #[test]
    fn insert_and_len() {
        let svc = make_service();
        svc.insert("a".into(), vec![1.0, 0.0], serde_json::json!({}));
        svc.insert("b".into(), vec![0.0, 1.0], serde_json::json!({}));
        assert_eq!(svc.len(), 2);
        assert!(!svc.is_empty());
    }

    #[test]
    fn insert_upsert() {
        let svc = make_service();
        svc.insert("a".into(), vec![1.0, 0.0], serde_json::json!({"v": 1}));
        svc.insert("a".into(), vec![0.0, 1.0], serde_json::json!({"v": 2}));
        assert_eq!(svc.len(), 1);
    }

    #[test]
    fn search_empty_returns_empty() {
        let svc = make_service();
        let results = svc.search(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_results() {
        let svc = make_service();
        svc.insert("a".into(), vec![1.0, 0.0, 0.0], serde_json::json!({}));
        svc.insert("b".into(), vec![0.0, 1.0, 0.0], serde_json::json!({}));
        svc.insert("c".into(), vec![0.0, 0.0, 1.0], serde_json::json!({}));

        let results = svc.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!((results[0].score - 1.0).abs() < 0.01);
    }

    #[test]
    fn search_count_incremented() {
        let svc = make_service();
        assert_eq!(svc.search_count(), 0);
        svc.search(&[1.0], 1);
        svc.search(&[1.0], 1);
        assert_eq!(svc.search_count(), 2);
    }

    #[test]
    fn insert_count_incremented() {
        let svc = make_service();
        assert_eq!(svc.insert_count(), 0);
        svc.insert("a".into(), vec![1.0], serde_json::json!({}));
        svc.insert("b".into(), vec![0.0], serde_json::json!({}));
        assert_eq!(svc.insert_count(), 2);
    }

    #[test]
    fn clear_resets() {
        let svc = make_service();
        svc.insert("a".into(), vec![1.0], serde_json::json!({}));
        svc.insert("b".into(), vec![0.0], serde_json::json!({}));
        assert_eq!(svc.len(), 2);

        svc.clear();
        assert!(svc.is_empty());
        assert_eq!(svc.len(), 0);
        // Counters are preserved after clear.
        assert_eq!(svc.insert_count(), 2);
    }

    #[test]
    fn config_default() {
        let cfg = HnswServiceConfig::default();
        assert_eq!(cfg.ef_search, 100);
        assert_eq!(cfg.ef_construction, 200);
        assert_eq!(cfg.default_dimensions, 384);

        let svc = HnswService::new(cfg);
        let c = svc.config();
        assert_eq!(c.ef_search, 100);
        assert_eq!(c.ef_construction, 200);
        assert_eq!(c.default_dimensions, 384);
    }

    #[test]
    fn service_name_is_ecc_hnsw() {
        let svc = make_service();
        assert_eq!(svc.name(), "ecc.hnsw");
        assert_eq!(svc.service_type(), ServiceType::Core);
    }

    #[tokio::test]
    async fn service_lifecycle() {
        let svc = make_service();
        svc.start().await.unwrap();
        let health = svc.health_check().await;
        assert_eq!(health, HealthStatus::Healthy);
        svc.stop().await.unwrap();
    }
}
