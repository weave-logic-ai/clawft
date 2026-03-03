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
    /// When `false` (the default), kernel subsystems do not activate
    /// unless explicitly invoked via `weave kernel` CLI commands.
    #[serde(default)]
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
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_processes: default_max_processes(),
            health_check_interval_secs: default_health_check_interval_secs(),
            cluster: None,
            chain: None,
            resource_tree: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_kernel_config() {
        let cfg = KernelConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.max_processes, 64);
        assert_eq!(cfg.health_check_interval_secs, 30);
    }

    #[test]
    fn deserialize_empty() {
        let cfg: KernelConfig = serde_json::from_str("{}").unwrap();
        assert!(!cfg.enabled);
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
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: KernelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.enabled, cfg.enabled);
        assert_eq!(restored.max_processes, cfg.max_processes);
    }
}
