//! Kernel configuration types.
//!
//! These types are defined in `clawft-types` so they can be embedded
//! in the root [`Config`](super::Config) without creating a circular
//! dependency with `clawft-kernel`.

use serde::{Deserialize, Serialize};

/// Default maximum number of concurrent processes.
fn default_max_processes() -> u32 {
    64
}

/// Default health check interval in seconds.
fn default_health_check_interval_secs() -> u64 {
    30
}

/// Kernel is enabled by default.
fn default_enabled() -> bool {
    true
}

/// Cluster networking configuration for distributed WeftOS nodes.
///
/// Controls the ruvector-powered clustering layer that coordinates
/// native nodes. Browser/edge nodes join via WebSocket to a
/// coordinator and do not need this configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNetworkConfig {
    /// Number of replica copies for each shard (default: 3).
    #[serde(default = "default_replication_factor", alias = "replicationFactor")]
    pub replication_factor: usize,

    /// Total number of shards in the cluster (default: 64).
    #[serde(default = "default_shard_count", alias = "shardCount")]
    pub shard_count: u32,

    /// Interval between heartbeat checks in seconds (default: 5).
    #[serde(
        default = "default_cluster_heartbeat",
        alias = "heartbeatIntervalSecs"
    )]
    pub heartbeat_interval_secs: u64,

    /// Timeout before marking a node offline in seconds (default: 30).
    #[serde(default = "default_node_timeout", alias = "nodeTimeoutSecs")]
    pub node_timeout_secs: u64,

    /// Whether to enable DAG-based consensus (default: true).
    #[serde(default = "default_enable_consensus", alias = "enableConsensus")]
    pub enable_consensus: bool,

    /// Minimum nodes required for quorum (default: 2).
    #[serde(default = "default_min_quorum", alias = "minQuorumSize")]
    pub min_quorum_size: usize,

    /// Seed node addresses for discovery (coordinator addresses).
    #[serde(default, alias = "seedNodes")]
    pub seed_nodes: Vec<String>,

    /// Human-readable display name for this node.
    #[serde(default, alias = "nodeName")]
    pub node_name: Option<String>,
}

fn default_replication_factor() -> usize {
    3
}
fn default_shard_count() -> u32 {
    64
}
fn default_cluster_heartbeat() -> u64 {
    5
}
fn default_node_timeout() -> u64 {
    30
}
fn default_enable_consensus() -> bool {
    true
}
fn default_min_quorum() -> usize {
    2
}

impl Default for ClusterNetworkConfig {
    fn default() -> Self {
        Self {
            replication_factor: default_replication_factor(),
            shard_count: default_shard_count(),
            heartbeat_interval_secs: default_cluster_heartbeat(),
            node_timeout_secs: default_node_timeout(),
            enable_consensus: default_enable_consensus(),
            min_quorum_size: default_min_quorum(),
            seed_nodes: Vec::new(),
            node_name: None,
        }
    }
}

/// Kernel subsystem configuration.
///
/// Embedded in the root `Config` under the `kernel` key. All fields
/// have sensible defaults so that existing configuration files parse
/// without errors.
///
/// # Example JSON
///
/// ```json
/// {
///   "kernel": {
///     "enabled": false,
///     "max_processes": 128,
///     "health_check_interval_secs": 15
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    /// Whether the kernel subsystem is enabled.
    ///
    /// When `false`, kernel subsystems do not activate unless explicitly
    /// invoked via `weave kernel` CLI commands. Defaults to `true`.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Maximum number of concurrent processes in the process table.
    #[serde(default = "default_max_processes", alias = "maxProcesses")]
    pub max_processes: u32,

    /// Interval (in seconds) between periodic health checks.
    #[serde(
        default = "default_health_check_interval_secs",
        alias = "healthCheckIntervalSecs"
    )]
    pub health_check_interval_secs: u64,

    /// Cluster networking configuration (native coordinator nodes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster: Option<ClusterNetworkConfig>,

    /// Local chain configuration (exochain feature).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<ChainConfig>,

    /// Resource tree configuration (exochain feature).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "resourceTree")]
    pub resource_tree: Option<ResourceTreeConfig>,

    /// Vector search backend configuration (ECC feature).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorConfig>,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_processes: default_max_processes(),
            health_check_interval_secs: default_health_check_interval_secs(),
            cluster: None,
            chain: None,
            resource_tree: None,
            vector: None,
        }
    }
}

/// Local chain configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Whether the local chain is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum events before auto-checkpoint.
    #[serde(default = "default_checkpoint_interval", alias = "checkpointInterval")]
    pub checkpoint_interval: u64,

    /// Chain ID (0 = local node chain).
    #[serde(default)]
    pub chain_id: u32,

    /// Path to the chain checkpoint file for persistence across restarts.
    /// If `None`, defaults to `~/.clawft/chain/local.json`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "checkpointPath"
    )]
    pub checkpoint_path: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_checkpoint_interval() -> u64 {
    1000
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            checkpoint_interval: default_checkpoint_interval(),
            chain_id: 0,
            checkpoint_path: None,
        }
    }
}

impl ChainConfig {
    /// Returns the effective checkpoint path.
    ///
    /// If `checkpoint_path` is set, returns it. Otherwise falls back to
    /// `~/.clawft/chain.json` (requires the `native` feature for `dirs`).
    pub fn effective_checkpoint_path(&self) -> Option<String> {
        if self.checkpoint_path.is_some() {
            return self.checkpoint_path.clone();
        }
        #[cfg(feature = "native")]
        {
            dirs::home_dir().map(|h| {
                h.join(".clawft")
                    .join("chain.json")
                    .to_string_lossy()
                    .into_owned()
            })
        }
        #[cfg(not(feature = "native"))]
        {
            None
        }
    }
}

/// Resource tree configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTreeConfig {
    /// Whether the resource tree is enabled.
    #[serde(default = "default_true_rt")]
    pub enabled: bool,

    /// Path to checkpoint file (None = in-memory only).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "checkpointPath"
    )]
    pub checkpoint_path: Option<String>,
}

fn default_true_rt() -> bool {
    true
}

impl Default for ResourceTreeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            checkpoint_path: None,
        }
    }
}

// ── Vector search backend configuration ──────────────────────────────────

/// Which vector search backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VectorBackendKind {
    /// In-memory HNSW (default, fast, suitable for <1M vectors).
    #[default]
    Hnsw,
    /// SSD-backed DiskANN (large scale, 1M+ vectors).
    DiskAnn,
    /// Hot HNSW cache + cold DiskANN store.
    Hybrid,
}

/// HNSW-specific vector configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHnswConfig {
    /// ef_construction parameter for index building.
    #[serde(default = "default_ef_construction")]
    pub ef_construction: usize,

    /// Number of bi-directional links per node (M parameter).
    #[serde(default = "default_m")]
    pub m: usize,

    /// Maximum number of elements the index can hold.
    #[serde(default = "default_max_elements")]
    pub max_elements: usize,
}

fn default_ef_construction() -> usize {
    200
}
fn default_m() -> usize {
    16
}
fn default_max_elements() -> usize {
    100_000
}

impl Default for VectorHnswConfig {
    fn default() -> Self {
        Self {
            ef_construction: default_ef_construction(),
            m: default_m(),
            max_elements: default_max_elements(),
        }
    }
}

/// DiskANN-specific vector configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDiskAnnConfig {
    /// Maximum number of points the index can hold.
    #[serde(default = "default_diskann_max_points")]
    pub max_points: usize,

    /// Vector dimensionality.
    #[serde(default = "default_diskann_dimensions")]
    pub dimensions: usize,

    /// Number of neighbors per node in the DiskANN graph.
    #[serde(default = "default_diskann_num_neighbors")]
    pub num_neighbors: usize,

    /// Size of the search candidate list.
    #[serde(default = "default_diskann_search_list_size")]
    pub search_list_size: usize,

    /// Directory path for SSD-backed data files.
    #[serde(default = "default_diskann_data_path")]
    pub data_path: String,

    /// Whether to use product quantization for compression.
    #[serde(default = "default_diskann_use_pq")]
    pub use_pq: bool,

    /// Number of PQ sub-quantizer chunks.
    #[serde(default = "default_diskann_pq_num_chunks")]
    pub pq_num_chunks: usize,
}

fn default_diskann_max_points() -> usize {
    10_000_000
}
fn default_diskann_dimensions() -> usize {
    384
}
fn default_diskann_num_neighbors() -> usize {
    64
}
fn default_diskann_search_list_size() -> usize {
    100
}
fn default_diskann_data_path() -> String {
    ".weftos/diskann".to_owned()
}
fn default_diskann_use_pq() -> bool {
    true
}
fn default_diskann_pq_num_chunks() -> usize {
    48
}

impl Default for VectorDiskAnnConfig {
    fn default() -> Self {
        Self {
            max_points: default_diskann_max_points(),
            dimensions: default_diskann_dimensions(),
            num_neighbors: default_diskann_num_neighbors(),
            search_list_size: default_diskann_search_list_size(),
            data_path: default_diskann_data_path(),
            use_pq: default_diskann_use_pq(),
            pq_num_chunks: default_diskann_pq_num_chunks(),
        }
    }
}

/// Eviction policy for the hybrid backend's hot tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VectorEvictionPolicy {
    /// Least Recently Used.
    #[default]
    Lru,
}

/// Hybrid backend-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHybridConfig {
    /// Maximum number of vectors in the hot (HNSW) tier.
    #[serde(default = "default_hybrid_hot_capacity")]
    pub hot_capacity: usize,

    /// Access count threshold before a cold vector is promoted to hot.
    #[serde(default = "default_hybrid_promotion_threshold")]
    pub promotion_threshold: u32,

    /// Eviction policy when the hot tier is full.
    #[serde(default)]
    pub eviction_policy: VectorEvictionPolicy,
}

fn default_hybrid_hot_capacity() -> usize {
    50_000
}
fn default_hybrid_promotion_threshold() -> u32 {
    3
}

impl Default for VectorHybridConfig {
    fn default() -> Self {
        Self {
            hot_capacity: default_hybrid_hot_capacity(),
            promotion_threshold: default_hybrid_promotion_threshold(),
            eviction_policy: VectorEvictionPolicy::default(),
        }
    }
}

/// Unified vector search backend configuration.
///
/// Controls which backend is used for the ECC cognitive substrate's
/// vector search layer.
///
/// # Example TOML
///
/// ```toml
/// [kernel.vector]
/// backend = "hybrid"
///
/// [kernel.vector.hnsw]
/// ef_construction = 200
/// max_elements = 100000
///
/// [kernel.vector.diskann]
/// max_points = 10000000
/// data_path = ".weftos/diskann"
///
/// [kernel.vector.hybrid]
/// hot_capacity = 50000
/// promotion_threshold = 3
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    /// Which backend to use.
    #[serde(default)]
    pub backend: VectorBackendKind,

    /// HNSW-specific settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hnsw: Option<VectorHnswConfig>,

    /// DiskANN-specific settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diskann: Option<VectorDiskAnnConfig>,

    /// Hybrid-specific settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hybrid: Option<VectorHybridConfig>,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            backend: VectorBackendKind::default(),
            hnsw: None,
            diskann: None,
            hybrid: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_kernel_config() {
        let cfg = KernelConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_processes, 64);
        assert_eq!(cfg.health_check_interval_secs, 30);
    }

    #[test]
    fn deserialize_empty() {
        let cfg: KernelConfig = serde_json::from_str("{}").unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_processes, 64);
    }

    #[test]
    fn deserialize_camel_case() {
        let json = r#"{"maxProcesses": 128, "healthCheckIntervalSecs": 15}"#;
        let cfg: KernelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.max_processes, 128);
        assert_eq!(cfg.health_check_interval_secs, 15);
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = KernelConfig {
            enabled: true,
            max_processes: 256,
            health_check_interval_secs: 10,
            cluster: None,
            chain: None,
            resource_tree: None,
            vector: None,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: KernelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.enabled, cfg.enabled);
        assert_eq!(restored.max_processes, cfg.max_processes);
    }

    #[test]
    fn vector_config_defaults() {
        let cfg = VectorConfig::default();
        assert_eq!(cfg.backend, VectorBackendKind::Hnsw);
        assert!(cfg.hnsw.is_none());
        assert!(cfg.diskann.is_none());
        assert!(cfg.hybrid.is_none());
    }

    #[test]
    fn vector_config_deserialize_hybrid() {
        let json = r#"{"backend": "hybrid", "hybrid": {"hot_capacity": 1000, "promotion_threshold": 5}}"#;
        let cfg: VectorConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.backend, VectorBackendKind::Hybrid);
        let h = cfg.hybrid.unwrap();
        assert_eq!(h.hot_capacity, 1000);
        assert_eq!(h.promotion_threshold, 5);
    }

    #[test]
    fn vector_config_deserialize_diskann() {
        let json = r#"{"backend": "diskann", "diskann": {"max_points": 5000000}}"#;
        let cfg: VectorConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.backend, VectorBackendKind::DiskAnn);
        let d = cfg.diskann.unwrap();
        assert_eq!(d.max_points, 5_000_000);
    }

    #[test]
    fn kernel_config_with_vector() {
        let json = r#"{"vector": {"backend": "hnsw"}}"#;
        let cfg: KernelConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.vector.is_some());
        assert_eq!(cfg.vector.unwrap().backend, VectorBackendKind::Hnsw);
    }
}
