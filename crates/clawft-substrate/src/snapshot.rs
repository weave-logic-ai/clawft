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

use parking_lot::{Mutex, RwLock};
use serde_json::Value;
use tokio::task::JoinHandle;

use crate::adapter::{AdapterError, OntologyAdapter, SubId};
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

/// An in-flight subscription tracked by the [`Substrate`].
///
/// Owns the adapter handle + the drain task's join handle so
/// [`Substrate::close_all`] can tombstone it cleanly on shutdown.
struct TrackedSub {
    id: SubId,
    adapter: Arc<dyn OntologyAdapter>,
    drain: JoinHandle<()>,
}

/// Substrate state tree — aggregates deltas from all subscribed
/// adapters.
pub struct Substrate {
    state: RwLock<BTreeMap<String, Value>>,
    /// `path → max_len` overrides applied on every `Append`. Seeded
    /// from each adapter's [`crate::TopicDecl::max_len`] at subscribe
    /// time. `None`/missing means unbounded.
    max_lens: RwLock<BTreeMap<String, usize>>,
    subscriptions: Mutex<Vec<TrackedSub>>,
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
            max_lens: RwLock::new(BTreeMap::new()),
            subscriptions: Mutex::new(Vec::new()),
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
    ///   If a `max_len` override is registered for `path` (via an
    ///   adapter topic declaration), the array is front-trimmed to at
    ///   most that many entries after the append.
    /// - [`StateDelta::Remove`] — drops the path if present; no-op
    ///   otherwise.
    pub fn apply(&self, delta: StateDelta) {
        let mut state = self.state.write();
        match delta {
            StateDelta::Replace { path, value } => {
                state.insert(path, value);
            }
            StateDelta::Append { path, value } => {
                let cap = self.max_lens.read().get(&path).copied();
                let entry = state
                    .entry(path)
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = entry {
                    arr.push(value);
                    if let Some(n) = cap {
                        if n == 0 {
                            arr.clear();
                        } else if arr.len() > n {
                            let excess = arr.len() - n;
                            arr.drain(0..excess);
                        }
                    }
                } else {
                    *entry = Value::Array(vec![value]);
                }
            }
            StateDelta::Remove { path } => {
                state.remove(&path);
            }
        }
    }

    /// Register a `max_len` override for a path. Called by
    /// [`Substrate::subscribe_adapter`] when the topic's
    /// [`crate::TopicDecl::max_len`] is `Some(_)`. Public so embedders
    /// can register caps for composer-authored deltas too.
    pub fn set_max_len(&self, path: impl Into<String>, max_len: usize) {
        self.max_lens.write().insert(path.into(), max_len);
    }

    /// Number of live subscriptions currently tracked. Decreases after
    /// [`Substrate::close_all`] or when a drain task exits on its own.
    pub fn active_subscription_count(&self) -> usize {
        // Opportunistically reap finished handles so the count reflects
        // reality without requiring the caller to poll.
        let mut subs = self.subscriptions.lock();
        subs.retain(|s| !s.drain.is_finished());
        subs.len()
    }

    /// Tombstone every tracked subscription.
    ///
    /// Calls [`OntologyAdapter::close`] on each adapter (best-effort;
    /// errors are ignored — close is idempotent per ADR-009 tombstone
    /// discipline) and aborts the drain task. Safe to call more than
    /// once.
    pub async fn close_all(&self) {
        let drained: Vec<TrackedSub> = {
            let mut guard = self.subscriptions.lock();
            std::mem::take(&mut *guard)
        };
        for sub in drained {
            // Tell the adapter first; if the adapter has its own
            // poller task keyed on this id, it will exit on its own.
            let _ = sub.adapter.close(sub.id).await;
            // Then abort the drain task unconditionally. The sender
            // side may already be closed (making the drain naturally
            // exit), but abort covers the case where the adapter is
            // slow to tear down.
            sub.drain.abort();
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
    /// receiver into [`Substrate::apply`], and records the adapter +
    /// task handle so [`Substrate::close_all`] can tear it down. The
    /// drain task runs until the adapter closes the sender (typically
    /// on `close(id)`) or until `close_all` aborts it.
    ///
    /// If the topic's [`crate::TopicDecl::max_len`] is `Some(_)`, the
    /// cap is registered so [`Substrate::apply`] auto-trims list-typed
    /// deltas for that path.
    ///
    /// **Runtime**: the caller must be inside a tokio runtime when this
    /// is called (the background task spawns via `tokio::spawn`).
    pub async fn subscribe_adapter(
        self: &Arc<Self>,
        adapter: Arc<dyn OntologyAdapter>,
        topic: &str,
        args: Value,
    ) -> Result<SubId, AdapterError> {
        // Register max_len from the topic declaration (if any). Done
        // before `open` so the first deltas are trimmed.
        if let Some(decl) = adapter.topics().iter().find(|t| t.path == topic)
            && let Some(n) = decl.max_len
        {
            self.set_max_len(topic, n);
        }

        let sub = adapter.open(topic, args).await?;
        let id = sub.id;
        let sink = Arc::clone(self);
        let mut rx = sub.rx;
        let drain = tokio::spawn(async move {
            while let Some(delta) = rx.recv().await {
                sink.apply(delta);
            }
            // Sender closed — adapter terminated this subscription.
        });
        self.subscriptions.lock().push(TrackedSub {
            id,
            adapter,
            drain,
        });
        Ok(id)
    }
}

impl Drop for Substrate {
    /// Abort any outstanding drain tasks on shutdown.
    ///
    /// `Drop` cannot call async, so this is a synchronous best-effort:
    /// it aborts the drain handles and drops the adapter `Arc`s. The
    /// adapter's own `close()` cannot be awaited here — callers who
    /// need a clean tombstone (ADR-009) should call
    /// [`Substrate::close_all`] from async context before dropping.
    /// Aborting is safe because the drain task only reads from an
    /// mpsc receiver and writes into the substrate's own state; there
    /// is no external resource to leak.
    fn drop(&mut self) {
        let mut guard = self.subscriptions.lock();
        for sub in guard.drain(..) {
            sub.drain.abort();
        }
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

    #[test]
    fn apply_append_trims_front_when_max_len_set() {
        let sub = Substrate::new();
        sub.set_max_len("logs", 3);
        for i in 0..5 {
            sub.apply(StateDelta::Append {
                path: "logs".into(),
                value: json!(i),
            });
        }
        // After 5 appends with cap=3, we keep the last 3 (2, 3, 4).
        assert_eq!(sub.get("logs"), Some(json!([2, 3, 4])));
    }

    #[test]
    fn apply_append_without_max_len_is_unbounded() {
        let sub = Substrate::new();
        for i in 0..5 {
            sub.apply(StateDelta::Append {
                path: "logs".into(),
                value: json!(i),
            });
        }
        assert_eq!(sub.get("logs"), Some(json!([0, 1, 2, 3, 4])));
    }

    // ── close_all / subscription tracking ────────────────────────────

    use crate::adapter::{
        BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
        Subscription, TopicDecl,
    };
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    const CLOSE_MOCK_TOPICS: &[TopicDecl] = &[TopicDecl {
        path: "substrate/mock/items",
        shape: "ontology://mock",
        refresh_hint: RefreshHint::Periodic { ms: 100 },
        sensitivity: Sensitivity::Public,
        buffer_policy: BufferPolicy::BlockCapped,
        max_len: None,
    }];

    struct ForeverMock {
        closed: parking_lot::Mutex<bool>,
    }

    #[async_trait]
    impl OntologyAdapter for ForeverMock {
        fn id(&self) -> &'static str {
            "forever-mock"
        }
        fn topics(&self) -> &'static [TopicDecl] {
            CLOSE_MOCK_TOPICS
        }
        fn permissions(&self) -> &'static [PermissionReq] {
            &[]
        }
        async fn open(
            &self,
            _topic: &str,
            _args: Value,
        ) -> Result<Subscription, crate::adapter::AdapterError> {
            // Capacity 1 — we keep the sender alive forever so the
            // drain task never exits on its own. close_all must abort
            // it.
            let (tx, rx) = mpsc::channel(1);
            // Leak the tx deliberately by storing it on the heap and
            // forgetting it — the drain task will never see the
            // sender close.
            Box::leak(Box::new(tx));
            Ok(Subscription { id: SubId(7), rx })
        }
        async fn close(&self, _sub_id: SubId) -> Result<(), crate::adapter::AdapterError> {
            *self.closed.lock() = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn close_all_aborts_subscriptions() {
        let adapter = Arc::new(ForeverMock {
            closed: parking_lot::Mutex::new(false),
        });
        let substrate = Arc::new(Substrate::new());
        substrate
            .subscribe_adapter(
                adapter.clone() as Arc<dyn OntologyAdapter>,
                "substrate/mock/items",
                Value::Null,
            )
            .await
            .expect("subscribe");

        assert_eq!(substrate.active_subscription_count(), 1);

        substrate.close_all().await;

        assert_eq!(substrate.active_subscription_count(), 0);
        assert!(
            *adapter.closed.lock(),
            "adapter.close() should have been invoked"
        );
    }

    #[tokio::test]
    async fn close_all_is_idempotent() {
        let adapter = Arc::new(ForeverMock {
            closed: parking_lot::Mutex::new(false),
        });
        let substrate = Arc::new(Substrate::new());
        substrate
            .subscribe_adapter(
                adapter as Arc<dyn OntologyAdapter>,
                "substrate/mock/items",
                Value::Null,
            )
            .await
            .expect("subscribe");

        substrate.close_all().await;
        substrate.close_all().await; // second call is a no-op
        assert_eq!(substrate.active_subscription_count(), 0);
    }
}
