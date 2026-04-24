//! Node identity registry.
//!
//! A **Node** is a physical thing in the mesh — an ESP32, the daemon
//! host, a Pi. It signs *emissions* (sensor data, heartbeats, anything
//! it merely reports). Sensing is not acting; nodes are deliberately
//! distinct from actors ([`crate::agent_registry::AgentRegistry`]),
//! which sign *Actions* (Foundry-style mutations).
//!
//! Each node holds an Ed25519 keypair. The node-id is the Ed25519
//! pubkey's short fingerprint (the first 16 bytes of its SHA-256,
//! hex-encoded). A friendly human label lives as a property at
//! `substrate/<node-id>/meta/label`, not as the identity itself —
//! labels can collide, keys cannot.
//!
//! # Substrate write gate
//!
//! Under the node-identity contract every substrate write belongs to
//! exactly one node and must land under that node's namespace:
//!
//! ```text
//! substrate/<node-id>/...
//! ```
//!
//! This module provides the registry (node-id → pubkey) and the
//! canonical signing payload
//! ([`node_publish_payload`]). Enforcement of the prefix rule lives
//! on [`SubstrateService`][crate::substrate_service::SubstrateService];
//! signature verification lives at the RPC boundary in
//! `clawft-weave`. Both consult this registry.
//!
//! The registry is in-memory. Node keys are provisioned on the node
//! side (firmware-burned for the ESP32, persisted on disk for the
//! daemon) and registered at kernel boot. Restart-resets by design —
//! the on-disk provisioning is the source of truth.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;

/// A registered node: node-id, public key, optional friendly label,
/// and when it was added to the registry.
#[derive(Debug, Clone)]
pub struct RegisteredNode {
    /// Stable identifier derived from the pubkey. Hex-encoded prefix
    /// of SHA-256(pubkey). See [`node_id_from_pubkey`].
    pub node_id: String,
    /// Ed25519 public key (32 bytes).
    pub pubkey: [u8; 32],
    /// Optional human-readable label, e.g. `"esp32-workbench"` or
    /// `"daemon-wsl"`. Authoritative label lives under
    /// `substrate/<node-id>/meta/label`; this field is a convenience
    /// copy so code paths that don't touch substrate can still render
    /// a friendly name.
    pub label: Option<String>,
    /// When the node was registered with the kernel.
    pub registered_at: DateTime<Utc>,
}

/// In-memory node-id → public-key map.
///
/// Cheap to clone — the inner map is wrapped in `Arc`/`DashMap`.
#[derive(Debug, Default, Clone)]
pub struct NodeRegistry {
    inner: Arc<DashMap<String, RegisteredNode>>,
}

impl NodeRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node. The node-id is derived deterministically from
    /// the pubkey — re-registering the same key updates the label but
    /// keeps the id stable. Returns the (possibly-updated) entry.
    pub fn register(&self, pubkey: [u8; 32], label: Option<String>) -> RegisteredNode {
        let node_id = node_id_from_pubkey(&pubkey);
        let entry = RegisteredNode {
            node_id: node_id.clone(),
            pubkey,
            label,
            registered_at: Utc::now(),
        };
        self.inner.insert(node_id, entry.clone());
        entry
    }

    /// Look up a node by its id.
    pub fn get(&self, node_id: &str) -> Option<RegisteredNode> {
        self.inner.get(node_id).map(|e| e.clone())
    }

    /// Whether `node_id` is known.
    pub fn contains(&self, node_id: &str) -> bool {
        self.inner.contains_key(node_id)
    }

    /// Number of registered nodes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// List all registered nodes.
    pub fn list(&self) -> Vec<RegisteredNode> {
        self.inner.iter().map(|e| e.value().clone()).collect()
    }
}

/// Derive a node-id from an Ed25519 public key.
///
/// Layout: lowercase hex of the first 16 bytes of `SHA-256(pubkey)`.
/// 128 bits of collision resistance — plenty for a mesh, and short
/// enough to fit comfortably in a substrate path segment.
pub fn node_id_from_pubkey(pubkey: &[u8; 32]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(pubkey);
    let full = h.finalize();
    let short: &[u8] = &full[..16];
    let mut out = String::with_capacity(32);
    for b in short {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Compose the canonical byte payload a node must sign for a
/// `substrate.publish`.
///
/// Layout:
/// `b"substrate.publish.node\0" || path || b"\0" || message || b"\0" || ts_le || b"\0" || node_id`.
///
/// Separate from [`crate::agent_registry::publish_payload`] so an
/// actor signature over an ipc.publish payload cannot be replayed as
/// a node signature over a substrate.publish — the domain-separation
/// prefix differs.
pub fn node_publish_payload(path: &str, message: &str, ts: u64, node_id: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        22 + path.len() + 1 + message.len() + 1 + 8 + 1 + node_id.len(),
    );
    buf.extend_from_slice(b"substrate.publish.node\0");
    buf.extend_from_slice(path.as_bytes());
    buf.push(0);
    buf.extend_from_slice(message.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&ts.to_le_bytes());
    buf.push(0);
    buf.extend_from_slice(node_id.as_bytes());
    buf
}

/// Required path prefix for any substrate write originated by `node_id`.
///
/// Returns `substrate/<node-id>/` (with the trailing slash so a
/// `starts_with` check correctly rejects e.g.
/// `substrate/<node-id-prefix-collision>/…`).
pub fn required_path_prefix(node_id: &str) -> String {
    format!("substrate/{node_id}/")
}

/// Whether `path` is a legal write target for `node_id`. Convenience
/// wrapper over [`required_path_prefix`].
pub fn path_belongs_to(path: &str, node_id: &str) -> bool {
    path.starts_with(&required_path_prefix(node_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_is_stable_per_pubkey() {
        let pk = [7u8; 32];
        let a = node_id_from_pubkey(&pk);
        let b = node_id_from_pubkey(&pk);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32, "16 bytes hex = 32 chars");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn node_id_differs_per_pubkey() {
        let a = node_id_from_pubkey(&[1u8; 32]);
        let b = node_id_from_pubkey(&[2u8; 32]);
        assert_ne!(a, b);
    }

    #[test]
    fn register_and_lookup_round_trips() {
        let reg = NodeRegistry::new();
        let entry = reg.register([7u8; 32], Some("esp32-workbench".into()));
        assert_eq!(reg.len(), 1);
        let fetched = reg.get(&entry.node_id).unwrap();
        assert_eq!(fetched.pubkey, [7u8; 32]);
        assert_eq!(fetched.label.as_deref(), Some("esp32-workbench"));
    }

    #[test]
    fn reregister_same_key_updates_label_keeps_id() {
        let reg = NodeRegistry::new();
        let a = reg.register([9u8; 32], Some("old".into()));
        let b = reg.register([9u8; 32], Some("new".into()));
        assert_eq!(a.node_id, b.node_id);
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get(&a.node_id).unwrap().label.as_deref(), Some("new"));
    }

    #[test]
    fn contains_reports_membership() {
        let reg = NodeRegistry::new();
        let e = reg.register([3u8; 32], None);
        assert!(reg.contains(&e.node_id));
        assert!(!reg.contains("deadbeef"));
    }

    #[test]
    fn list_returns_every_entry() {
        let reg = NodeRegistry::new();
        reg.register([1u8; 32], None);
        reg.register([2u8; 32], None);
        reg.register([3u8; 32], None);
        let all = reg.list();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn publish_payload_is_deterministic_per_inputs() {
        let p1 = node_publish_payload("substrate/n1/sensor/mic", "{}", 42, "n1");
        let p2 = node_publish_payload("substrate/n1/sensor/mic", "{}", 42, "n1");
        assert_eq!(p1, p2);
    }

    #[test]
    fn publish_payload_changes_with_ts() {
        let p1 = node_publish_payload("substrate/n1/x", "v", 1, "n1");
        let p2 = node_publish_payload("substrate/n1/x", "v", 2, "n1");
        assert_ne!(p1, p2);
    }

    #[test]
    fn publish_payload_domain_separated_from_actor() {
        // The actor-level `publish_payload` uses `b"ipc.publish\0"`
        // as its prefix. Ours uses `b"substrate.publish.node\0"`.
        // The two byte streams must not collide on any input.
        let node_p = node_publish_payload("t", "v", 1, "n1");
        let actor_p = crate::agent_registry::publish_payload("t", "v", 1, "n1");
        assert_ne!(node_p, actor_p);
    }

    #[test]
    fn required_prefix_includes_trailing_slash() {
        assert_eq!(required_path_prefix("n1"), "substrate/n1/");
    }

    #[test]
    fn path_belongs_to_accepts_exact_subtree() {
        assert!(path_belongs_to("substrate/n1/sensor/mic", "n1"));
        assert!(path_belongs_to("substrate/n1/meta/label", "n1"));
    }

    #[test]
    fn path_belongs_to_rejects_cross_node() {
        assert!(!path_belongs_to("substrate/n2/sensor/mic", "n1"));
    }

    #[test]
    fn path_belongs_to_rejects_prefix_collision() {
        // A node named `n1` must not be able to write under `n11`'s
        // namespace just because `starts_with("substrate/n1")` is true.
        // The trailing-slash guard blocks this.
        assert!(!path_belongs_to("substrate/n11/anything", "n1"));
    }

    #[test]
    fn path_belongs_to_rejects_top_level() {
        assert!(!path_belongs_to("substrate/sensor/mic", "n1"));
        assert!(!path_belongs_to("", "n1"));
        assert!(!path_belongs_to("something-else", "n1"));
    }
}
