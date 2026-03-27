//! Unified persistence coordinator for kernel state.
//!
//! Provides a single entry point to save and restore all kernel
//! subsystems (CausalGraph, HNSW index, ExoChain) to a data directory.
//! Uses file-based JSON persistence — no external database required.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::causal::CausalGraph;
use crate::hnsw_service::{HnswService, HnswServiceConfig};

/// Configuration for the persistence coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Root directory for all persisted state.
    pub data_dir: PathBuf,
    /// If set, auto-save interval in seconds (for future use with a
    /// background timer).
    pub auto_save_interval_secs: Option<u64>,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".weftos/state"),
            auto_save_interval_secs: None,
        }
    }
}

impl PersistenceConfig {
    /// Path for the causal graph snapshot.
    pub fn causal_graph_path(&self) -> PathBuf {
        self.data_dir.join("causal_graph.json")
    }

    /// Path for the HNSW index snapshot.
    pub fn hnsw_index_path(&self) -> PathBuf {
        self.data_dir.join("hnsw_index.json")
    }

    /// Path for the ExoChain snapshot.
    pub fn chain_path(&self) -> PathBuf {
        self.data_dir.join("exochain.jsonl")
    }
}

/// Save the causal graph to the configured data directory.
pub fn save_causal_graph(
    config: &PersistenceConfig,
    graph: &CausalGraph,
) -> Result<(), std::io::Error> {
    graph.save_to_file(&config.causal_graph_path())
}

/// Load a causal graph from the configured data directory.
///
/// Returns a new empty graph if the file does not exist.
pub fn load_causal_graph(config: &PersistenceConfig) -> Result<CausalGraph, std::io::Error> {
    let path = config.causal_graph_path();
    if !path.exists() {
        return Ok(CausalGraph::new());
    }
    CausalGraph::load_from_file(&path)
}

/// Save the HNSW service state to the configured data directory.
pub fn save_hnsw(
    config: &PersistenceConfig,
    service: &HnswService,
) -> Result<(), std::io::Error> {
    service.save_to_file(&config.hnsw_index_path())
}

/// Load an HNSW service from the configured data directory.
///
/// Returns a new empty service if the file does not exist.
pub fn load_hnsw(config: &PersistenceConfig) -> Result<HnswService, std::io::Error> {
    let path = config.hnsw_index_path();
    if !path.exists() {
        return Ok(HnswService::new(HnswServiceConfig::default()));
    }
    HnswService::load_from_file(&path)
}

/// Save all kernel state to the configured data directory.
pub fn save_all(
    config: &PersistenceConfig,
    graph: &CausalGraph,
    hnsw: &HnswService,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&config.data_dir)?;
    save_causal_graph(config, graph)?;
    save_hnsw(config, hnsw)?;
    Ok(())
}

/// Restore all kernel state from the configured data directory.
///
/// Components that have no saved state are returned as fresh instances.
pub fn load_all(
    config: &PersistenceConfig,
) -> Result<(CausalGraph, HnswService), std::io::Error> {
    let graph = load_causal_graph(config)?;
    let hnsw = load_hnsw(config)?;
    Ok((graph, hnsw))
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_config() -> PersistenceConfig {
        let dir = std::env::temp_dir().join(format!(
            "weftos_persist_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        PersistenceConfig {
            data_dir: dir,
            auto_save_interval_secs: None,
        }
    }

    #[test]
    fn config_paths() {
        let cfg = PersistenceConfig {
            data_dir: PathBuf::from("/tmp/test"),
            auto_save_interval_secs: None,
        };
        assert_eq!(cfg.causal_graph_path(), PathBuf::from("/tmp/test/causal_graph.json"));
        assert_eq!(cfg.hnsw_index_path(), PathBuf::from("/tmp/test/hnsw_index.json"));
        assert_eq!(cfg.chain_path(), PathBuf::from("/tmp/test/exochain.jsonl"));
    }

    #[test]
    fn load_missing_returns_defaults() {
        let cfg = tmp_config();
        let graph = load_causal_graph(&cfg).unwrap();
        assert_eq!(graph.node_count(), 0);
        let hnsw = load_hnsw(&cfg).unwrap();
        assert!(hnsw.is_empty());
    }

    #[test]
    fn save_and_load_all_roundtrip() {
        let cfg = tmp_config();

        let graph = CausalGraph::new();
        let a = graph.add_node("A".into(), serde_json::json!({"x": 1}));
        let b = graph.add_node("B".into(), serde_json::json!({}));
        graph.link(a, b, crate::causal::CausalEdgeType::Causes, 0.9, 100, 1);

        let hnsw = HnswService::new(HnswServiceConfig::default());
        hnsw.insert("v1".into(), vec![1.0, 0.0, 0.0], serde_json::json!({"tag": "first"}));

        save_all(&cfg, &graph, &hnsw).unwrap();

        let (loaded_graph, loaded_hnsw) = load_all(&cfg).unwrap();
        assert_eq!(loaded_graph.node_count(), 2);
        assert_eq!(loaded_graph.edge_count(), 1);
        assert_eq!(loaded_hnsw.len(), 1);

        // Cleanup.
        let _ = std::fs::remove_dir_all(&cfg.data_dir);
    }
}
