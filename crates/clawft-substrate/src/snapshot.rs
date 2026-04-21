//! Substrate state tree + snapshot API.
//!
//! The substrate is a flat `BTreeMap<String, Value>` keyed by absolute
//! topic-rooted path. Every incoming [`StateDelta`] is applied in order
//! by [`Substrate::apply`]. Surface composers read the whole tree with
//! [`Substrate::snapshot`] and build their primitive tree off the
//! returned [`OntologySnapshot`].
//!
//! Future work (M1.5-B surface composer): replace the flat map with a
//! hierarchical cursor that can yield subtrees cheaply. For M1.5 a flat
//! map is sufficient — the kernel adapter emits at most ~200 log
//! entries plus a handful of paths.

use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use crate::adapter::{AdapterError, OntologyAdapter};
use crate::delta::StateDelta;

/// Read-only snapshot of the substrate state tree at a point in time.
///
/// Surface composers hold this for the duration of one frame and walk
/// it to resolve ontology bindings.
///
/// **M1.5 compatibility note**: the shape (`BTreeMap<String, Value>`)
/// is shared with sibling M1.5-B `clawft-surface`. When that crate
/// lands the type will move to a common crate; today both define
/// structurally-identical versions to avoid a pre-merge dependency
/// cycle. TODO: M1.5-D unify.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OntologySnapshot(pub BTreeMap<String, Value>);

impl OntologySnapshot {
    /// Lookup a single path. `None` if the path is not present.
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.0.get(path)
    }

    /// Iterate over every path/value pair.
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, String, Value> {
        self.0.iter()
    }

    /// Number of paths currently populated.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the snapshot holds zero paths.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Substrate state tree — aggregates deltas from all subscribed
/// adapters.
pub struct Substrate {
    state: RwLock<BTreeMap<String, Value>>,
}

impl Default for Substrate {
    fn default() -> Self {
        Self::new()
    }
}

impl Substrate {
    /// Construct an empty substrate.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(BTreeMap::new()),
        }
    }

    /// Apply a single delta.
    ///
    /// Semantics:
    /// - [`StateDelta::Replace`] — overwrites the value at `path`.
    /// - [`StateDelta::Append`] — appends to an existing array; creates
    ///   a new single-element array if the path is empty; if the path
    ///   exists with a non-array value, replaces it with a new array
    ///   containing the appended value (permissive rather than panic).
    /// - [`StateDelta::Remove`] — drops the path if present; no-op
    ///   otherwise.
    pub fn apply(&self, delta: StateDelta) {
        let mut state = self.state.write();
        match delta {
            StateDelta::Replace { path, value } => {
                state.insert(path, value);
            }
            StateDelta::Append { path, value } => {
                let entry = state
                    .entry(path)
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = entry {
                    arr.push(value);
                } else {
                    *entry = Value::Array(vec![value]);
                }
            }
            StateDelta::Remove { path } => {
                state.remove(&path);
            }
        }
    }

    /// Snapshot the current state. Clones the map; cheap enough for
    /// M1.5 (kernel adapter produces ~O(1kB) total).
    pub fn snapshot(&self) -> OntologySnapshot {
        OntologySnapshot(self.state.read().clone())
    }

    /// Direct read access for callers that only need one path and want
    /// to avoid cloning the whole map.
    pub fn get(&self, path: &str) -> Option<Value> {
        self.state.read().get(path).cloned()
    }

    /// Subscribe to `topic` on `adapter`, wiring the delta stream into
    /// this substrate.
    ///
    /// Opens the subscription, spawns a tokio task that drains the
    /// receiver into [`Substrate::apply`], and returns. The task runs
    /// until the adapter closes the sender (typically on `close(id)`).
    ///
    /// **Runtime**: the caller must be inside a tokio runtime when this
    /// is called (the background task spawns via `tokio::spawn`).
    pub async fn subscribe_adapter(
        self: &Arc<Self>,
        adapter: Arc<dyn OntologyAdapter>,
        topic: &str,
        args: Value,
    ) -> Result<crate::adapter::SubId, AdapterError> {
        let sub = adapter.open(topic, args).await?;
        let id = sub.id;
        let sink = Arc::clone(self);
        let mut rx = sub.rx;
        tokio::spawn(async move {
            while let Some(delta) = rx.recv().await {
                sink.apply(delta);
            }
            // Sender closed — adapter terminated this subscription.
        });
        Ok(id)
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn apply_replace_sets_and_overwrites() {
        let sub = Substrate::new();
        sub.apply(StateDelta::Replace {
            path: "substrate/kernel/status".into(),
            value: json!({ "state": "running", "pid": 1234 }),
        });
        assert_eq!(
            sub.get("substrate/kernel/status"),
            Some(json!({ "state": "running", "pid": 1234 }))
        );

        sub.apply(StateDelta::Replace {
            path: "substrate/kernel/status".into(),
            value: json!({ "state": "stopping" }),
        });
        assert_eq!(
            sub.get("substrate/kernel/status"),
            Some(json!({ "state": "stopping" }))
        );
    }

    #[test]
    fn apply_append_creates_and_extends_array() {
        let sub = Substrate::new();
        sub.apply(StateDelta::Append {
            path: "substrate/kernel/logs".into(),
            value: json!({ "ts": 1, "msg": "boot" }),
        });
        sub.apply(StateDelta::Append {
            path: "substrate/kernel/logs".into(),
            value: json!({ "ts": 2, "msg": "ready" }),
        });
        let logs = sub.get("substrate/kernel/logs").unwrap();
        assert_eq!(logs.as_array().map(|a| a.len()), Some(2));
        assert_eq!(logs[0]["msg"], "boot");
        assert_eq!(logs[1]["msg"], "ready");
    }

    #[test]
    fn apply_remove_drops_path() {
        let sub = Substrate::new();
        sub.apply(StateDelta::Replace {
            path: "substrate/kernel/status".into(),
            value: json!({ "state": "running" }),
        });
        sub.apply(StateDelta::Remove {
            path: "substrate/kernel/status".into(),
        });
        assert_eq!(sub.get("substrate/kernel/status"), None);
    }

    #[test]
    fn snapshot_clones_current_state() {
        let sub = Substrate::new();
        sub.apply(StateDelta::Replace {
            path: "a".into(),
            value: json!(1),
        });
        let snap = sub.snapshot();
        // Mutate after snapshot — snapshot unchanged.
        sub.apply(StateDelta::Replace {
            path: "a".into(),
            value: json!(2),
        });
        assert_eq!(snap.get("a"), Some(&json!(1)));
        assert_eq!(sub.get("a"), Some(json!(2)));
        assert_eq!(snap.len(), 1);
        assert!(!snap.is_empty());
    }

    #[test]
    fn apply_append_over_non_array_replaces_with_array() {
        let sub = Substrate::new();
        sub.apply(StateDelta::Replace {
            path: "x".into(),
            value: json!("scalar"),
        });
        sub.apply(StateDelta::Append {
            path: "x".into(),
            value: json!("added"),
        });
        assert_eq!(sub.get("x"), Some(json!(["added"])));
    }
}
