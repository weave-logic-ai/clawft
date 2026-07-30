//! Substrate-backed [`RouterDecisionLog`](clawft_core::agent::routing_log::RouterDecisionLog)
//! (WEFT-335 / agent-core-v1.1).
//!
//! Publishes each router decision under the mesh-canonical path
//! `substrate/_derived/agent/routing/recent/<ulid>` through
//! [`SubstrateService::publish_gated_with_grants`] so the daemon's
//! `agent` [`DerivedWriteGrant`] is enforced. Without the grant (or
//! with a wrong node id) the publish is rejected and the error
//! surfaces to the loop as a non-fatal log failure.
//!
//! Bounded retention: after each successful append the writer lists
//! the prefix, sorts by path (ULID / id lexicographic order ≈ time
//! order), and tombstones the oldest entries when the count exceeds
//! the configured cap (default
//! [`DEFAULT_ROUTING_LOG_RETENTION`](clawft_core::agent::routing_log::DEFAULT_ROUTING_LOG_RETENTION)).
//! Tombstones are `{ "_pruned": true }` so path keys remain but
//! downstream `weft routing trace` can skip them.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::agent::routing_log::{
    RouterDecisionLog, RouterDecisionRecord, DEFAULT_ROUTING_LOG_RETENTION,
    ROUTING_RECENT_SUBSTRATE_PREFIX,
};
use tracing::debug;
use ulid::Ulid;

use crate::substrate_sink::SubstrateClient;

/// Grant-gated router-decision writer over a [`SubstrateClient`].
pub struct SubstrateRouterDecisionLog {
    client: Arc<dyn SubstrateClient>,
    /// Daemon node id that holds the `agent` derived-write grant.
    node_id: String,
    /// Last-N retention cap (`0` = unbounded).
    retention: usize,
}

impl SubstrateRouterDecisionLog {
    /// Construct a writer bound to `node_id`'s grants with default retention.
    pub fn new(client: Arc<dyn SubstrateClient>, node_id: impl Into<String>) -> Self {
        Self::with_retention(client, node_id, DEFAULT_ROUTING_LOG_RETENTION)
    }

    /// Construct with an explicit retention cap.
    pub fn with_retention(
        client: Arc<dyn SubstrateClient>,
        node_id: impl Into<String>,
        retention: usize,
    ) -> Self {
        Self {
            client,
            node_id: node_id.into(),
            retention,
        }
    }

    /// Substrate path for one decision id.
    pub fn entry_path(decision_id: &str) -> String {
        format!("{ROUTING_RECENT_SUBSTRATE_PREFIX}/{decision_id}")
    }

    /// Tombstone oldest entries when over the retention cap.
    fn prune_if_needed(&self) -> Result<(), String> {
        if self.retention == 0 {
            return Ok(());
        }
        let mut paths = self
            .client
            .list(ROUTING_RECENT_SUBSTRATE_PREFIX, 1)?;
        // Drop the prefix itself if listed; keep only child entries.
        paths.retain(|p| p != ROUTING_RECENT_SUBSTRATE_PREFIX && !p.ends_with("/_index"));
        if paths.len() <= self.retention {
            return Ok(());
        }
        paths.sort();
        let excess = paths.len() - self.retention;
        let tombstone = serde_json::json!({ "_pruned": true });
        for path in paths.into_iter().take(excess) {
            // Best-effort: a prune failure must not fail the append that
            // already succeeded.
            if let Err(e) = self.client.publish(&self.node_id, &path, tombstone.clone()) {
                debug!(error = %e, path = %path, "routing log: prune tombstone failed");
            }
        }
        Ok(())
    }
}

#[async_trait]
impl RouterDecisionLog for SubstrateRouterDecisionLog {
    async fn append(&self, record: RouterDecisionRecord) -> Result<(), String> {
        let mut rec = record.finalized();
        // Prefer a true ULID for trace/replay sorting.
        if rec.decision_id.is_empty() || rec.decision_id.starts_with("rd-") {
            rec.decision_id = Ulid::new().to_string();
        }
        let path = Self::entry_path(&rec.decision_id);
        let value = rec.to_substrate_value();
        let tick = self.client.publish(&self.node_id, &path, value)?;
        debug!(
            path = %path,
            decision_id = %rec.decision_id,
            latency_ms = rec.latency_ms,
            tick,
            "routing log: substrate decision published"
        );
        // Retention is best-effort; never fail the append after publish.
        let _ = self.prune_if_needed();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_core::agent::context_router::{ContextDecision, ContextRequest};
    use clawft_core::agent::routing_log::RouterDecisionRecord;
    use clawft_kernel::{GrantScope, NodeRegistry, SubstrateService};
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::substrate_sink::KernelSubstrateClient;

    /// In-memory substrate client for isolated unit tests.
    struct MapClient {
        store: Mutex<HashMap<String, serde_json::Value>>,
        deny: bool,
    }

    impl MapClient {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
                deny: false,
            }
        }
    }

    impl SubstrateClient for MapClient {
        fn publish(
            &self,
            _node_id: &str,
            path: &str,
            value: serde_json::Value,
        ) -> Result<u64, String> {
            if self.deny {
                return Err("MissingDerivedGrant".into());
            }
            self.store.lock().unwrap().insert(path.to_string(), value);
            Ok(1)
        }
        fn list(&self, prefix: &str, _depth: u32) -> Result<Vec<String>, String> {
            Ok(self
                .store
                .lock()
                .unwrap()
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect())
        }
        fn read(&self, path: &str) -> Result<Option<serde_json::Value>, String> {
            Ok(self.store.lock().unwrap().get(path).cloned())
        }
    }

    fn sample_record(n: usize) -> RouterDecisionRecord {
        let req = ContextRequest {
            content: format!("query-{n}"),
            channel: "panel".into(),
            chat_id: format!("c{n}"),
            metadata: Default::default(),
        };
        let decision = ContextDecision::default();
        RouterDecisionRecord::from_route(&req, &decision, n as u64, vec![], None, false)
    }

    #[tokio::test]
    async fn substrate_log_publishes_under_derived_prefix() {
        let client = Arc::new(MapClient::new());
        let log = SubstrateRouterDecisionLog::new(client.clone(), "n-daemon");
        log.append(sample_record(0)).await.unwrap();

        let paths = client.list(ROUTING_RECENT_SUBSTRATE_PREFIX, 1).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(
            paths[0].starts_with("substrate/_derived/agent/routing/recent/"),
            "path must be mesh-canonical: {}",
            paths[0]
        );
        let value = client.read(&paths[0]).unwrap().unwrap();
        assert_eq!(value["query"], "query-0");
        assert_eq!(value["latency_ms"], 0);
        assert!(!value["decision_id"].as_str().unwrap_or("").is_empty());
        assert!(!value["ts"].as_str().unwrap_or("").is_empty());
    }

    #[tokio::test]
    async fn one_hundred_routes_produce_one_hundred_substrate_entries() {
        let client = Arc::new(MapClient::new());
        let log = SubstrateRouterDecisionLog::with_retention(client.clone(), "n-daemon", 1_000);
        for i in 0..100 {
            log.append(sample_record(i)).await.unwrap();
        }
        let paths = client.list(ROUTING_RECENT_SUBSTRATE_PREFIX, 1).unwrap();
        assert_eq!(
            paths.len(),
            100,
            "100 routes must produce 100 substrate entries"
        );
    }

    #[tokio::test]
    async fn retention_tombstones_oldest() {
        let client = Arc::new(MapClient::new());
        let log = SubstrateRouterDecisionLog::with_retention(client.clone(), "n-daemon", 3);
        for i in 0..5 {
            log.append(sample_record(i)).await.unwrap();
        }
        let paths = client.list(ROUTING_RECENT_SUBSTRATE_PREFIX, 1).unwrap();
        assert_eq!(paths.len(), 5, "tombstones keep paths");
        let pruned = paths
            .iter()
            .filter(|p| {
                client
                    .read(p)
                    .ok()
                    .flatten()
                    .and_then(|v| v.get("_pruned").and_then(|x| x.as_bool()))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(pruned, 2, "excess 2 entries should be tombstoned");
        let live = paths
            .iter()
            .filter(|p| {
                !client
                    .read(p)
                    .ok()
                    .flatten()
                    .and_then(|v| v.get("_pruned").and_then(|x| x.as_bool()))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(live, 3);
    }

    #[tokio::test]
    async fn substrate_log_surfaces_grant_denial() {
        let mut client = MapClient::new();
        client.deny = true;
        let log = SubstrateRouterDecisionLog::new(Arc::new(client), "n-daemon");
        let err = log.append(sample_record(0)).await.unwrap_err();
        assert!(
            err.contains("MissingDerivedGrant") || err.contains("grant"),
            "expected grant denial, got: {err}"
        );
    }

    #[tokio::test]
    async fn kernel_client_requires_agent_grant() {
        let substrate = SubstrateService::new();
        let registry = NodeRegistry::default();
        let node = registry.register([7u8; 32], Some("test".into()));
        let node_id = node.node_id.clone();

        let client = Arc::new(KernelSubstrateClient::new(
            substrate.clone(),
            registry.clone(),
        ));
        let log = SubstrateRouterDecisionLog::new(client.clone(), node_id.clone());
        let denied = log.append(sample_record(0)).await;
        assert!(denied.is_err(), "publish without grant must fail");

        // Topic `agent` covers `agent/routing/recent/<ulid>`.
        registry
            .issue_derived_grant(&node_id, "agent", GrantScope::TopicPrefix)
            .unwrap();
        log.append(sample_record(1))
            .await
            .expect("publish with agent grant must succeed");

        let paths = client.list(ROUTING_RECENT_SUBSTRATE_PREFIX, 2).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].starts_with(ROUTING_RECENT_SUBSTRATE_PREFIX));
    }
}
