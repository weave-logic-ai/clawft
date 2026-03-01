//! Cluster membership and node fabric.
//!
//! Defines types for multi-node WeftOS clusters where agents
//! can migrate between nodes. Each node is a WeftOS kernel
//! instance -- native binary on cloud/edge, or WASM in a browser.
//!
//! # Feature Gate
//!
//! All types compile unconditionally. Actual peer discovery,
//! health monitoring, and cross-node communication require the
//! `cluster` feature flag and a distributed networking layer.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Unique node identifier (UUID or DID string).
pub type NodeId = String;

/// Node platform type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodePlatform {
    /// Native binary on a cloud VM or bare metal server.
    CloudNative,
    /// Native binary on an edge device.
    Edge,
    /// WASM module running in a browser tab.
    Browser,
    /// WASI module in a container.
    Wasi,
    /// Custom platform label.
    Custom(String),
}

impl std::fmt::Display for NodePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodePlatform::CloudNative => write!(f, "cloud-native"),
            NodePlatform::Edge => write!(f, "edge"),
            NodePlatform::Browser => write!(f, "browser"),
            NodePlatform::Wasi => write!(f, "wasi"),
            NodePlatform::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

/// Node health state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is joining the cluster.
    Joining,
    /// Node is healthy and active.
    Active,
    /// Node is suspected unreachable (missed heartbeats).
    Suspect,
    /// Node has been confirmed unreachable.
    Unreachable,
    /// Node is gracefully leaving the cluster.
    Leaving,
    /// Node has left the cluster.
    Left,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Joining => write!(f, "joining"),
            NodeState::Active => write!(f, "active"),
            NodeState::Suspect => write!(f, "suspect"),
            NodeState::Unreachable => write!(f, "unreachable"),
            NodeState::Leaving => write!(f, "leaving"),
            NodeState::Left => write!(f, "left"),
        }
    }
}

/// Information about a peer node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerNode {
    /// Unique node identifier.
    pub id: NodeId,

    /// Human-readable node name.
    pub name: String,

    /// Node platform.
    pub platform: NodePlatform,

    /// Current state in the cluster.
    pub state: NodeState,

    /// Network address for direct communication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    /// When this node was first seen.
    pub first_seen: DateTime<Utc>,

    /// When the last heartbeat was received.
    pub last_heartbeat: DateTime<Utc>,

    /// Capabilities this node advertises.
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Labels for scheduling and filtering.
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Cluster configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// This node's identifier.
    pub node_id: NodeId,

    /// This node's display name.
    #[serde(default = "default_node_name")]
    pub node_name: String,

    /// This node's platform type.
    #[serde(default = "default_platform")]
    pub platform: NodePlatform,

    /// Heartbeat interval in seconds.
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,

    /// How many missed heartbeats before marking a node suspect.
    #[serde(default = "default_suspect_threshold")]
    pub suspect_threshold: u32,

    /// How many missed heartbeats before marking a node unreachable.
    #[serde(default = "default_unreachable_threshold")]
    pub unreachable_threshold: u32,

    /// Maximum cluster size (0 = unlimited).
    #[serde(default)]
    pub max_nodes: u32,
}

fn default_node_name() -> String {
    "local".into()
}

fn default_platform() -> NodePlatform {
    NodePlatform::CloudNative
}

fn default_heartbeat_interval() -> u64 {
    5
}

fn default_suspect_threshold() -> u32 {
    3
}

fn default_unreachable_threshold() -> u32 {
    10
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            node_name: default_node_name(),
            platform: default_platform(),
            heartbeat_interval_secs: default_heartbeat_interval(),
            suspect_threshold: default_suspect_threshold(),
            unreachable_threshold: default_unreachable_threshold(),
            max_nodes: 0,
        }
    }
}

/// Cluster membership errors.
#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    /// Node already in the cluster.
    #[error("node already exists: '{node_id}'")]
    NodeAlreadyExists {
        /// Node ID.
        node_id: NodeId,
    },

    /// Node not found.
    #[error("node not found: '{node_id}'")]
    NodeNotFound {
        /// Node ID.
        node_id: NodeId,
    },

    /// Cluster is at maximum capacity.
    #[error("cluster full: max {max} nodes")]
    ClusterFull {
        /// Maximum node count.
        max: u32,
    },

    /// Invalid state transition.
    #[error("invalid node state transition: {from} -> {to}")]
    InvalidTransition {
        /// Current state.
        from: String,
        /// Requested state.
        to: String,
    },
}

/// Cluster membership tracker.
///
/// Tracks which nodes are part of the cluster, their health state,
/// and capabilities. Actual peer discovery and heartbeat monitoring
/// require a networking layer not included here.
pub struct ClusterMembership {
    config: ClusterConfig,
    peers: DashMap<NodeId, PeerNode>,
}

impl ClusterMembership {
    /// Create a new cluster membership tracker.
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            config,
            peers: DashMap::new(),
        }
    }

    /// Get the cluster configuration.
    pub fn config(&self) -> &ClusterConfig {
        &self.config
    }

    /// Get this node's ID.
    pub fn local_node_id(&self) -> &str {
        &self.config.node_id
    }

    /// Register a peer node.
    pub fn add_peer(&self, peer: PeerNode) -> Result<(), ClusterError> {
        if self.peers.contains_key(&peer.id) {
            return Err(ClusterError::NodeAlreadyExists {
                node_id: peer.id,
            });
        }

        if self.config.max_nodes > 0
            && self.peers.len() as u32 >= self.config.max_nodes
        {
            return Err(ClusterError::ClusterFull {
                max: self.config.max_nodes,
            });
        }

        debug!(node_id = %peer.id, name = %peer.name, "adding peer to cluster");
        self.peers.insert(peer.id.clone(), peer);
        Ok(())
    }

    /// Remove a peer node.
    pub fn remove_peer(&self, node_id: &str) -> Result<PeerNode, ClusterError> {
        self.peers
            .remove(node_id)
            .map(|(_, peer)| peer)
            .ok_or_else(|| ClusterError::NodeNotFound {
                node_id: node_id.to_owned(),
            })
    }

    /// Update a peer's state.
    pub fn update_state(
        &self,
        node_id: &str,
        new_state: NodeState,
    ) -> Result<(), ClusterError> {
        let mut entry = self
            .peers
            .get_mut(node_id)
            .ok_or_else(|| ClusterError::NodeNotFound {
                node_id: node_id.to_owned(),
            })?;
        entry.state = new_state;
        Ok(())
    }

    /// Record a heartbeat from a peer.
    pub fn heartbeat(&self, node_id: &str) -> Result<(), ClusterError> {
        let mut entry = self
            .peers
            .get_mut(node_id)
            .ok_or_else(|| ClusterError::NodeNotFound {
                node_id: node_id.to_owned(),
            })?;
        entry.last_heartbeat = Utc::now();
        if entry.state == NodeState::Suspect {
            entry.state = NodeState::Active;
        }
        Ok(())
    }

    /// Get a snapshot of a peer's state.
    pub fn get_peer(&self, node_id: &str) -> Option<PeerNode> {
        self.peers.get(node_id).map(|e| e.value().clone())
    }

    /// List all peers with their states.
    pub fn list_peers(&self) -> Vec<(NodeId, NodeState, NodePlatform)> {
        self.peers
            .iter()
            .map(|e| (e.key().clone(), e.state.clone(), e.platform.clone()))
            .collect()
    }

    /// Count peers by state.
    pub fn count_by_state(&self, state: &NodeState) -> usize {
        self.peers.iter().filter(|e| &e.state == state).count()
    }

    /// Count total peers.
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Check if cluster has no peers.
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Get all active peer node IDs.
    pub fn active_peers(&self) -> Vec<NodeId> {
        self.peers
            .iter()
            .filter(|e| e.state == NodeState::Active)
            .map(|e| e.key().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: &str, name: &str) -> PeerNode {
        PeerNode {
            id: id.into(),
            name: name.into(),
            platform: NodePlatform::CloudNative,
            state: NodeState::Active,
            address: Some("10.0.0.1:8080".into()),
            first_seen: Utc::now(),
            last_heartbeat: Utc::now(),
            capabilities: vec!["compute".into()],
            labels: HashMap::from([("region".into(), "us-east".into())]),
        }
    }

    #[test]
    fn default_config() {
        let config = ClusterConfig::default();
        assert_eq!(config.node_name, "local");
        assert_eq!(config.heartbeat_interval_secs, 5);
        assert_eq!(config.suspect_threshold, 3);
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = ClusterConfig {
            node_id: "node-1".into(),
            node_name: "primary".into(),
            platform: NodePlatform::Edge,
            heartbeat_interval_secs: 10,
            suspect_threshold: 5,
            unreachable_threshold: 15,
            max_nodes: 100,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: ClusterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.node_name, "primary");
        assert_eq!(restored.max_nodes, 100);
    }

    #[test]
    fn node_platform_display() {
        assert_eq!(NodePlatform::CloudNative.to_string(), "cloud-native");
        assert_eq!(NodePlatform::Browser.to_string(), "browser");
        assert_eq!(
            NodePlatform::Custom("k8s".into()).to_string(),
            "custom(k8s)"
        );
    }

    #[test]
    fn node_state_display() {
        assert_eq!(NodeState::Active.to_string(), "active");
        assert_eq!(NodeState::Suspect.to_string(), "suspect");
        assert_eq!(NodeState::Unreachable.to_string(), "unreachable");
    }

    #[test]
    fn add_and_list_peers() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        cluster.add_peer(make_peer("node-2", "beta")).unwrap();

        let peers = cluster.list_peers();
        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn add_duplicate_fails() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        assert!(matches!(
            cluster.add_peer(make_peer("node-1", "alpha-dup")),
            Err(ClusterError::NodeAlreadyExists { .. })
        ));
    }

    #[test]
    fn cluster_full() {
        let config = ClusterConfig {
            max_nodes: 1,
            ..Default::default()
        };
        let cluster = ClusterMembership::new(config);
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        assert!(matches!(
            cluster.add_peer(make_peer("node-2", "beta")),
            Err(ClusterError::ClusterFull { .. })
        ));
    }

    #[test]
    fn remove_peer() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        let removed = cluster.remove_peer("node-1").unwrap();
        assert_eq!(removed.name, "alpha");
        assert!(cluster.is_empty());
    }

    #[test]
    fn remove_nonexistent_fails() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        assert!(matches!(
            cluster.remove_peer("nope"),
            Err(ClusterError::NodeNotFound { .. })
        ));
    }

    #[test]
    fn update_state() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        cluster
            .update_state("node-1", NodeState::Suspect)
            .unwrap();
        let peer = cluster.get_peer("node-1").unwrap();
        assert_eq!(peer.state, NodeState::Suspect);
    }

    #[test]
    fn heartbeat_clears_suspect() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        cluster
            .update_state("node-1", NodeState::Suspect)
            .unwrap();
        cluster.heartbeat("node-1").unwrap();
        let peer = cluster.get_peer("node-1").unwrap();
        assert_eq!(peer.state, NodeState::Active);
    }

    #[test]
    fn count_by_state() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        cluster.add_peer(make_peer("node-2", "beta")).unwrap();
        cluster
            .update_state("node-2", NodeState::Suspect)
            .unwrap();
        assert_eq!(cluster.count_by_state(&NodeState::Active), 1);
        assert_eq!(cluster.count_by_state(&NodeState::Suspect), 1);
    }

    #[test]
    fn active_peers() {
        let cluster = ClusterMembership::new(ClusterConfig::default());
        cluster.add_peer(make_peer("node-1", "alpha")).unwrap();
        cluster.add_peer(make_peer("node-2", "beta")).unwrap();
        cluster
            .update_state("node-2", NodeState::Leaving)
            .unwrap();
        let active = cluster.active_peers();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], "node-1");
    }

    #[test]
    fn peer_serde_roundtrip() {
        let peer = make_peer("node-1", "alpha");
        let json = serde_json::to_string(&peer).unwrap();
        let restored: PeerNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "node-1");
        assert_eq!(restored.capabilities, vec!["compute"]);
    }

    #[test]
    fn cluster_error_display() {
        let err = ClusterError::NodeNotFound {
            node_id: "node-1".into(),
        };
        assert!(err.to_string().contains("node-1"));

        let err = ClusterError::ClusterFull { max: 10 };
        assert!(err.to_string().contains("10"));
    }
}
