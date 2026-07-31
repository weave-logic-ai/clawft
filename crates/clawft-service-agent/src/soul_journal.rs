//! Substrate-backed soul journal (WEFT-330 write + WEFT-96 read).
//!
//! ## Write path (WEFT-330)
//!
//! [`SubstrateSoulJournal`] publishes each drift observation under the
//! mesh-canonical path `substrate/_derived/soul_journal/<ulid>` through
//! [`SubstrateService::publish_gated_with_grants`] so the F1
//! `soul_journal` [`DerivedWriteGrant`] is enforced. Without the
//! grant (or with a wrong node id) the publish is rejected and the
//! error surfaces to the loop as a non-fatal journal failure.
//!
//! ## Read path (WEFT-96 / WS-D1)
//!
//! [`SubstrateSoulJournalReader`] lists + reads the same prefix on every
//! identity turn via
//! [`JournalAwareIdentityProvider`](clawft_core::agent::identity::JournalAwareIdentityProvider).
//! Reads are unauthenticated mesh reads (`SubstrateClient::list` /
//! `read`) — matching `weaver soul promote` — so they do not need the
//! derived-write grant.
//!
//! Path layout matches what `weaver soul promote` lists
//! (`SOUL_JOURNAL_SUBSTRATE_PREFIX` in clawft-core). Entry payload
//! shape matches `clawft-weave::commands::soul_cmd::JournalEntry`
//! (`summary` / `content` / `ts` + optional `conv_id` / `signal`).

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::agent::soul_journal::{
    DriftObservation, PendingJournalEntry, SoulJournal, SoulJournalReader,
    SOUL_JOURNAL_SUBSTRATE_PREFIX,
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

/// Substrate-backed [`SoulJournalReader`] for the every-turn identity
/// path (WEFT-96 / WS-D1).
///
/// Lists value-bearing children under [`SOUL_JOURNAL_SUBSTRATE_PREFIX`]
/// and decodes each payload into a [`PendingJournalEntry`]. Intended
/// to be wrapped by
/// [`JournalAwareIdentityProvider`](clawft_core::agent::identity::JournalAwareIdentityProvider)
/// so identity loads consult pending journal rows without auto-promoting
/// them into `SOUL.md`.
pub struct SubstrateSoulJournalReader {
    client: Arc<dyn SubstrateClient>,
}

impl SubstrateSoulJournalReader {
    /// Construct a reader over a shared substrate client.
    pub fn new(client: Arc<dyn SubstrateClient>) -> Self {
        Self { client }
    }

    /// Extract the entry id (final path segment) from a full substrate path.
    fn entry_id_from_path(path: &str) -> &str {
        path.rsplit('/').next().unwrap_or(path)
    }
}

#[async_trait]
impl SoulJournalReader for SubstrateSoulJournalReader {
    async fn list_pending(&self) -> Result<Vec<PendingJournalEntry>, String> {
        // depth=1: immediate children under the journal prefix
        // (each child is one ULID entry path).
        let mut paths = self
            .client
            .list(SOUL_JOURNAL_SUBSTRATE_PREFIX, 1)
            .map_err(|e| format!("substrate list soul_journal: {e}"))?;
        // Stable order for deterministic prompt annotations / tests.
        paths.sort();

        let mut out = Vec::with_capacity(paths.len());
        for path in paths {
            let value = self
                .client
                .read(&path)
                .map_err(|e| format!("substrate read {path}: {e}"))?;
            let Some(value) = value else {
                continue;
            };
            let entry_id = Self::entry_id_from_path(&path).to_string();
            out.push(PendingJournalEntry::from_substrate_value(entry_id, &value));
        }
        debug!(
            count = out.len(),
            "soul journal: substrate read-on-every-turn listed pending"
        );
        Ok(out)
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
    async fn substrate_journal_reader_lists_pending_after_write() {
        // WEFT-96: write via SubstrateSoulJournal, read via
        // SubstrateSoulJournalReader (every-turn identity path).
        use clawft_core::agent::soul_journal::SoulJournalReader;

        let client = Arc::new(MapClient::new());
        let journal = SubstrateSoulJournal::new(client.clone(), "n-daemon");
        journal
            .append(DriftObservation::synthetic(
                "prefer terse",
                "user wants shorter answers",
            ))
            .await
            .unwrap();
        journal
            .append(DriftObservation::synthetic("second", "another note"))
            .await
            .unwrap();

        let reader = SubstrateSoulJournalReader::new(client);
        let pending = reader.list_pending().await.expect("list");
        assert_eq!(pending.len(), 2);
        // Sorted by full path → stable ULID order.
        assert!(pending.iter().any(|p| p.summary == "prefer terse"));
        assert!(pending.iter().any(|p| p.content == "another note"));
        assert!(!pending[0].entry_id.is_empty());
    }

    #[tokio::test]
    async fn substrate_journal_reader_returns_empty_when_none() {
        use clawft_core::agent::soul_journal::SoulJournalReader;

        let reader = SubstrateSoulJournalReader::new(Arc::new(MapClient::new()));
        let pending = reader.list_pending().await.unwrap();
        assert!(pending.is_empty());
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
