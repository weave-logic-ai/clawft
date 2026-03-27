//! DAG-backed mutation events for audit trail.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{ResourceId, ResourceKind};
use crate::scoring::NodeScoring;

/// A mutation event in the resource tree DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationEvent {
    /// A new resource node was created.
    Create {
        id: ResourceId,
        kind: ResourceKind,
        parent: ResourceId,
        timestamp: DateTime<Utc>,
        /// Ed25519 signature of the mutation (optional in K0).
        signature: Option<Vec<u8>>,
    },
    /// A resource node was removed.
    Remove {
        id: ResourceId,
        timestamp: DateTime<Utc>,
        signature: Option<Vec<u8>>,
    },
    /// Metadata was updated on a resource.
    UpdateMeta {
        id: ResourceId,
        key: String,
        value: Option<serde_json::Value>,
        timestamp: DateTime<Utc>,
        signature: Option<Vec<u8>>,
    },
    /// A resource was moved to a new parent.
    Move {
        id: ResourceId,
        old_parent: ResourceId,
        new_parent: ResourceId,
        timestamp: DateTime<Utc>,
        signature: Option<Vec<u8>>,
    },
    /// Scoring vector was updated on a resource.
    UpdateScoring {
        id: ResourceId,
        old: NodeScoring,
        new: NodeScoring,
        timestamp: DateTime<Utc>,
        signature: Option<Vec<u8>>,
    },
}

/// Append-only mutation log.
pub struct MutationLog {
    events: Vec<MutationEvent>,
}

impl MutationLog {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn append(&mut self, event: MutationEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[MutationEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for MutationLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_log_append_and_query() {
        let mut log = MutationLog::new();
        assert!(log.is_empty());

        log.append(MutationEvent::Create {
            id: ResourceId::new("/kernel"),
            kind: ResourceKind::Namespace,
            parent: ResourceId::root(),
            timestamp: Utc::now(),
            signature: None,
        });

        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());

        log.append(MutationEvent::Remove {
            id: ResourceId::new("/kernel"),
            timestamp: Utc::now(),
            signature: None,
        });

        assert_eq!(log.len(), 2);
        assert_eq!(log.events().len(), 2);
    }

    #[test]
    fn mutation_event_serde_roundtrip() {
        let event = MutationEvent::UpdateMeta {
            id: ResourceId::new("/apps/myapp"),
            key: "version".to_string(),
            value: Some(serde_json::json!("1.2.3")),
            timestamp: Utc::now(),
            signature: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: MutationEvent = serde_json::from_str(&json).unwrap();
        // Verify the deserialized variant
        match back {
            MutationEvent::UpdateMeta { id, key, .. } => {
                assert_eq!(id, ResourceId::new("/apps/myapp"));
                assert_eq!(key, "version");
            }
            _ => panic!("expected UpdateMeta variant"),
        }
    }

    #[test]
    fn mutation_log_default() {
        let log = MutationLog::default();
        assert!(log.is_empty());
    }
}
