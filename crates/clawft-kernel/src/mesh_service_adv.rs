//! Service advertisement for mesh networking (K6.5).
//!
//! Enables cluster-wide service discovery by advertising services
//! to the mesh and resolving services across nodes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ipc::GlobalPid;

/// Service advertised to the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAdvertisement {
    /// Service name (must be unique within the cluster).
    pub name: String,
    /// Available methods.
    pub methods: Vec<String>,
    /// Node hosting this service.
    pub node_id: String,
    /// Process hosting this service.
    pub global_pid: GlobalPid,
    /// Service version.
    pub version: String,
    /// Contract hash (if chain-anchored).
    pub contract_hash: Option<String>,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
    /// When this advertisement was last updated.
    pub last_updated: u64,
}

/// Cluster-wide service registry built from advertisements.
pub struct ClusterServiceRegistry {
    /// Services keyed by name. Multiple nodes may advertise the same service.
    services: HashMap<String, Vec<ServiceAdvertisement>>,
}

impl ClusterServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Merge a service advertisement (add or update).
    pub fn merge(&mut self, advert: ServiceAdvertisement) {
        let entries = self.services.entry(advert.name.clone()).or_default();
        // Update existing entry for same node, or add new.
        if let Some(existing) = entries.iter_mut().find(|e| e.node_id == advert.node_id) {
            if advert.last_updated > existing.last_updated {
                *existing = advert;
            }
        } else {
            entries.push(advert);
        }
    }

    /// Resolve a service by name. Returns all nodes hosting it.
    pub fn resolve(&self, name: &str) -> Vec<&ServiceAdvertisement> {
        self.services
            .get(name)
            .map(|entries| entries.iter().collect())
            .unwrap_or_default()
    }

    /// Resolve a service, preferring a specific node.
    pub fn resolve_preferred(
        &self,
        name: &str,
        preferred_node: &str,
    ) -> Option<&ServiceAdvertisement> {
        let entries = self.services.get(name)?;
        entries
            .iter()
            .find(|e| e.node_id == preferred_node)
            .or_else(|| entries.first())
    }

    /// List all known service names.
    pub fn service_names(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }

    /// Remove all services from a node.
    pub fn remove_node(&mut self, node_id: &str) {
        for entries in self.services.values_mut() {
            entries.retain(|e| e.node_id != node_id);
        }
        self.services.retain(|_, entries| !entries.is_empty());
    }

    /// Get all advertisements (for gossip).
    pub fn all_advertisements(&self) -> Vec<&ServiceAdvertisement> {
        self.services.values().flat_map(|v| v.iter()).collect()
    }

    /// Number of unique services.
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Total number of advertisements (across all replicas).
    pub fn total_advertisements(&self) -> usize {
        self.services.values().map(|v| v.len()).sum()
    }

    /// Resolve a service using round-robin across replicas.
    ///
    /// The caller provides a shared atomic counter that is incremented on
    /// each call, distributing requests evenly across all nodes hosting
    /// the named service.
    pub fn resolve_round_robin(
        &self,
        name: &str,
        counter: &std::sync::atomic::AtomicU64,
    ) -> Option<&ServiceAdvertisement> {
        let entries = self.services.get(name)?;
        if entries.is_empty() {
            return None;
        }
        let idx =
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize % entries.len();
        entries.get(idx)
    }
}

impl Default for ClusterServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_svc_advert(name: &str, node: &str, pid: u64, ts: u64) -> ServiceAdvertisement {
        ServiceAdvertisement {
            name: name.to_string(),
            methods: vec!["get".to_string(), "set".to_string()],
            node_id: node.to_string(),
            global_pid: GlobalPid::local(pid, node),
            version: "1.0.0".to_string(),
            contract_hash: None,
            metadata: HashMap::new(),
            last_updated: ts,
        }
    }

    #[test]
    fn service_advertisement_serde_roundtrip() {
        let advert = make_svc_advert("cache", "node-a", 1, 100);
        let json = serde_json::to_string(&advert).unwrap();
        let restored: ServiceAdvertisement = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "cache");
        assert_eq!(restored.node_id, "node-a");
        assert_eq!(restored.version, "1.0.0");
    }

    #[test]
    fn merge_and_resolve() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));

        let results = registry.resolve("cache");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, "node-a");
    }

    #[test]
    fn resolve_preferred_finds_preferred() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));
        registry.merge(make_svc_advert("cache", "node-b", 2, 100));

        let result = registry.resolve_preferred("cache", "node-b").unwrap();
        assert_eq!(result.node_id, "node-b");
    }

    #[test]
    fn resolve_preferred_falls_back() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));

        // Preferred node not found, falls back to first
        let result = registry.resolve_preferred("cache", "node-z").unwrap();
        assert_eq!(result.node_id, "node-a");
    }

    #[test]
    fn resolve_preferred_missing_service() {
        let registry = ClusterServiceRegistry::new();
        assert!(registry.resolve_preferred("missing", "node-a").is_none());
    }

    #[test]
    fn remove_node_removes_all_services() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));
        registry.merge(make_svc_advert("auth", "node-a", 2, 100));
        registry.merge(make_svc_advert("cache", "node-b", 3, 100));

        registry.remove_node("node-a");

        assert_eq!(registry.service_count(), 1); // only cache on node-b remains
        assert!(registry.resolve("auth").is_empty());
        assert_eq!(registry.resolve("cache").len(), 1);
    }

    #[test]
    fn multiple_replicas_same_service() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));
        registry.merge(make_svc_advert("cache", "node-b", 2, 100));
        registry.merge(make_svc_advert("cache", "node-c", 3, 100));

        assert_eq!(registry.service_count(), 1);
        assert_eq!(registry.total_advertisements(), 3);
        assert_eq!(registry.resolve("cache").len(), 3);
    }

    #[test]
    fn service_names_lists_all() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));
        registry.merge(make_svc_advert("auth", "node-a", 2, 100));
        registry.merge(make_svc_advert("search", "node-b", 3, 100));

        let mut names = registry.service_names();
        names.sort();
        assert_eq!(names, vec!["auth", "cache", "search"]);
    }

    #[test]
    fn merge_updates_existing_node_entry() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));

        // Update with newer timestamp
        let mut updated = make_svc_advert("cache", "node-a", 1, 200);
        updated.version = "2.0.0".to_string();
        registry.merge(updated);

        // Should still be one entry, but updated
        assert_eq!(registry.total_advertisements(), 1);
        let results = registry.resolve("cache");
        assert_eq!(results[0].version, "2.0.0");
    }

    #[test]
    fn resolve_round_robin_distributes() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 100));
        registry.merge(make_svc_advert("cache", "node-b", 2, 100));
        registry.merge(make_svc_advert("cache", "node-c", 3, 100));

        let counter = std::sync::atomic::AtomicU64::new(0);

        // Three calls should hit all three nodes in order
        let r0 = registry.resolve_round_robin("cache", &counter).unwrap();
        let r1 = registry.resolve_round_robin("cache", &counter).unwrap();
        let r2 = registry.resolve_round_robin("cache", &counter).unwrap();

        // All three nodes must be visited
        let mut nodes: Vec<&str> = vec![&r0.node_id, &r1.node_id, &r2.node_id];
        nodes.sort();
        assert_eq!(nodes, vec!["node-a", "node-b", "node-c"]);

        // Fourth call wraps around to first node
        let r3 = registry.resolve_round_robin("cache", &counter).unwrap();
        assert_eq!(r3.node_id, r0.node_id);
    }

    #[test]
    fn resolve_round_robin_missing_service() {
        let registry = ClusterServiceRegistry::new();
        let counter = std::sync::atomic::AtomicU64::new(0);
        assert!(registry.resolve_round_robin("missing", &counter).is_none());
    }

    #[test]
    fn merge_ignores_older_update() {
        let mut registry = ClusterServiceRegistry::new();
        registry.merge(make_svc_advert("cache", "node-a", 1, 200));

        let mut older = make_svc_advert("cache", "node-a", 1, 100);
        older.version = "0.1.0".to_string();
        registry.merge(older);

        let results = registry.resolve("cache");
        assert_eq!(results[0].version, "1.0.0"); // unchanged
    }
}
