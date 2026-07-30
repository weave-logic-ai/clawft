//! Substrate-backed [`SoulJournal`](clawft_core::agent::soul_journal::SoulJournal)
//! (WEFT-330 / agent-core-v1.1).
//!
//! Publishes each drift observation under the mesh-canonical path
//! `substrate/_derived/soul_journal/<ulid>` through
//! [`SubstrateService::publish_gated_with_grants`] so the F1
//! `soul_journal` [`DerivedWriteGrant`] is enforced. Without the
//! grant (or with a wrong node id) the publish is rejected and the
//! error surfaces to the loop as a non-fatal journal failure.
//!
//! Path layout matches what `weaver soul promote` lists
//! (`SOUL_JOURNAL_SUBSTRATE_PREFIX` in clawft-core). Entry payload
//! shape matches `clawft-weave::commands::soul_cmd::JournalEntry`
//! (`summary` / `content` / `ts` + optional `conv_id` / `signal`).

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::agent::soul_journal::{
    DriftObservation, SoulJournal, SOUL_JOURNAL_SUBSTRATE_PREFIX,
};
use tracing::debug;
use ulid::Ulid;

use crate::substrate_sink::SubstrateClient;

/// Grant-gated soul-journal writer over a [`SubstrateClient`].
pub struct SubstrateSoulJournal {
    client: Arc<dyn SubstrateClient>,
    /// Daemon node id that holds the `soul_journal` derived-write grant.
    node_id: String,
}

impl SubstrateSoulJournal {
    /// Construct a writer bound to `node_id`'s grants.
    pub fn new(client: Arc<dyn SubstrateClient>, node_id: impl Into<String>) -> Self {
        Self {
            client,
            node_id: node_id.into(),
        }
    }

    /// Substrate path for one entry id.
    pub fn entry_path(entry_id: &str) -> String {
        format!("{SOUL_JOURNAL_SUBSTRATE_PREFIX}/{entry_id}")
    }
}

#[async_trait]
impl SoulJournal for SubstrateSoulJournal {
    async fn append(&self, observation: DriftObservation) -> Result<(), String> {
        let mut obs = observation.finalized();
        // Prefer a true ULID for promote-side sorting / stable ids.
        // finalized() may have minted an `obs-…` fallback when the
        // caller left entry_id empty.
        if obs.entry_id.is_empty() || obs.entry_id.starts_with("obs-") {
            obs.entry_id = Ulid::new().to_string();
        }
        let path = Self::entry_path(&obs.entry_id);
        let value = obs.to_substrate_value();
        let tick = self.client.publish(&self.node_id, &path, value)?;
        debug!(
            path = %path,
            entry_id = %obs.entry_id,
            tick,
            "soul journal: substrate entry published"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_core::agent::soul_journal::DriftObservation;
    use clawft_kernel::{GrantScope, NodeRegistry, SubstrateService};
    use std::sync::Mutex;
    use std::collections::HashMap;

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

    #[tokio::test]
    async fn substrate_journal_publishes_under_derived_prefix() {
        let client = Arc::new(MapClient::new());
        let journal = SubstrateSoulJournal::new(client.clone(), "n-daemon");
        journal
            .append(DriftObservation::synthetic(
                "noticed bias toward verbose answers",
                "prefer narrative paragraphs over bullet lists",
            ))
            .await
            .unwrap();

        let paths = client.list(SOUL_JOURNAL_SUBSTRATE_PREFIX, 1).unwrap();
        assert_eq!(paths.len(), 1);
        assert!(
            paths[0].starts_with("substrate/_derived/soul_journal/"),
            "path must be mesh-canonical: {}",
            paths[0]
        );
        let value = client.read(&paths[0]).unwrap().unwrap();
        assert_eq!(
            value["summary"],
            "noticed bias toward verbose answers"
        );
        assert_eq!(
            value["content"],
            "prefer narrative paragraphs over bullet lists"
        );
        assert!(!value["ts"].as_str().unwrap_or("").is_empty());
    }

    #[tokio::test]
    async fn substrate_journal_surfaces_grant_denial() {
        let mut client = MapClient::new();
        client.deny = true;
        let journal = SubstrateSoulJournal::new(Arc::new(client), "n-daemon");
        let err = journal
            .append(DriftObservation::synthetic("s", "c"))
            .await
            .unwrap_err();
        assert!(
            err.contains("MissingDerivedGrant") || err.contains("grant"),
            "expected grant denial, got: {err}"
        );
    }

    #[tokio::test]
    async fn kernel_client_requires_soul_journal_grant() {
        let substrate = SubstrateService::new();
        let registry = NodeRegistry::default();
        // Register a node so identity is valid, but do NOT issue the
        // soul_journal grant yet — publish must fail.
        let node = registry.register([42u8; 32], Some("test".into()));
        let node_id = node.node_id.clone();

        let client = Arc::new(KernelSubstrateClient::new(
            substrate.clone(),
            registry.clone(),
        ));
        let journal = SubstrateSoulJournal::new(client.clone(), node_id.clone());
        let denied = journal
            .append(DriftObservation::synthetic("s", "c"))
            .await;
        assert!(denied.is_err(), "publish without grant must fail");

        // Issue the grant (mirrors daemon boot F1) and retry.
        registry
            .issue_derived_grant(&node_id, "soul_journal", GrantScope::TopicPrefix)
            .unwrap();
        journal
            .append(DriftObservation::synthetic(
                "promote-visible",
                "body for weaver soul promote",
            ))
            .await
            .expect("publish with grant must succeed");

        let paths = client
            .list(SOUL_JOURNAL_SUBSTRATE_PREFIX, 1)
            .expect("list");
        assert_eq!(paths.len(), 1);
        let value = client.read(&paths[0]).unwrap().unwrap();
        assert_eq!(value["summary"], "promote-visible");
        assert_eq!(value["content"], "body for weaver soul promote");
    }
}
