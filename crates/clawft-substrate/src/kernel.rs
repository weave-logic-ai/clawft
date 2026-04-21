//! `kernel` reference adapter — ADR-017 §5 entry.
//!
//! Refactored from `crates/clawft-gui-egui/src/live/*`. Instead of a
//! single shared `RwLock<Snapshot>` that every poll tick overwrites, the
//! adapter opens per-topic subscriptions and emits targeted
//! [`StateDelta`]s.
//!
//! # Topics
//!
//! | Topic | Shape | Refresh | Buffer | Semantics |
//! |-------|-------|---------|--------|-----------|
//! | `substrate/kernel/status` | `ontology://kernel-status` | periodic 2s | refuse | singleton; one `Replace` per tick |
//! | `substrate/kernel/processes` | `ontology://process-list` | periodic 1s | block-capped | list-by-pid; `Replace` of the whole list per tick (M1.6+ will emit per-pid deltas) |
//! | `substrate/kernel/services` | `ontology://service-list` | periodic 2s | block-capped | list-by-name; `Replace` of the whole list per tick |
//! | `substrate/kernel/logs` | `ontology://log-ring` | event-driven (fallback periodic 1s) | drop-oldest | append-only ring; new lines only per tick |
//!
//! All topics are [`Sensitivity::Workspace`]; none require
//! [`PermissionReq`].
//!
//! # Lifecycle
//!
//! `open` allocates a subscription id, creates a bounded mpsc, and
//! spawns a polling task. `close` looks up the `cancel` handle keyed by
//! id and drops the sender — the poller task sees the closed channel
//! on its next send attempt and exits.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

use crate::adapter::{
    AdapterError, BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
    Subscription, TopicDecl,
};
use crate::delta::StateDelta;

use clawft_rpc::{DaemonClient, Request};

/// Default number of log entries to request per `kernel.logs` call.
const LOG_TAIL: usize = 200;

/// Maximum retained log entries in the substrate (ADR-017 §4 — the
/// drop-oldest ring). The adapter doesn't currently trim; the composer
/// trims in the future. TODO.
#[allow(dead_code)]
const LOG_RING: usize = 1000;

/// Channel depth for singleton topics (refuse policy).
const CHAN_SINGLETON: usize = 1;
/// Channel depth for list topics (block-capped).
const CHAN_LIST: usize = 128;
/// Channel depth for log topics (drop-oldest ring).
const CHAN_LOG: usize = 1000;

/// Declared topics — static so callers can introspect without
/// instantiating the adapter. Order matches §5 of ADR-017 (kernel
/// entry).
pub const TOPICS: &[TopicDecl] = &[
    TopicDecl {
        path: "substrate/kernel/status",
        shape: "ontology://kernel-status",
        refresh_hint: RefreshHint::Periodic { ms: 2000 },
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::Refuse,
    },
    TopicDecl {
        path: "substrate/kernel/processes",
        shape: "ontology://process-list",
        refresh_hint: RefreshHint::Periodic { ms: 1000 },
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::BlockCapped,
    },
    TopicDecl {
        path: "substrate/kernel/services",
        shape: "ontology://service-list",
        refresh_hint: RefreshHint::Periodic { ms: 2000 },
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::BlockCapped,
    },
    TopicDecl {
        path: "substrate/kernel/logs",
        shape: "ontology://log-ring",
        // Event-driven when the daemon supports a tail stream; falls
        // back to periodic poll (1s) in the current implementation.
        refresh_hint: RefreshHint::EventDriven,
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::DropOldest,
    },
];

/// Permissions — kernel adapter requires nothing beyond daemon reach.
pub const PERMISSIONS: &[PermissionReq] = &[];

/// Per-subscription cancel channel. Dropping the sender closes the
/// poller task's receiver, which it watches alongside the poll ticker.
type CancelTx = oneshot::Sender<()>;

/// Internal registry of live subscriptions.
struct Registry {
    next_id: u64,
    live: HashMap<SubId, CancelTx>,
}

impl Registry {
    fn new() -> Self {
        Self {
            next_id: 1,
            live: HashMap::new(),
        }
    }

    fn allocate(&mut self) -> SubId {
        let id = SubId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

/// The kernel adapter.
///
/// Holds a [`Mutex`] over the subscription registry only; the daemon
/// RPC client is re-acquired on demand per poll task (the connection
/// may drop between ticks, and isolating per-task is simpler than
/// sharing a single reconnecting client across many pollers for M1.5).
pub struct KernelAdapter {
    reg: Mutex<Registry>,
}

impl Default for KernelAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelAdapter {
    /// Build a new adapter. Does not open any subscriptions; adapters
    /// are lazy — `open` is the constructor of each data stream.
    pub fn new() -> Self {
        Self {
            reg: Mutex::new(Registry::new()),
        }
    }
}

#[async_trait]
impl OntologyAdapter for KernelAdapter {
    fn id(&self) -> &'static str {
        "kernel"
    }

    fn topics(&self) -> &'static [TopicDecl] {
        TOPICS
    }

    fn permissions(&self) -> &'static [PermissionReq] {
        PERMISSIONS
    }

    async fn open(
        &self,
        topic: &str,
        args: Value,
    ) -> Result<Subscription, AdapterError> {
        let depth = match topic {
            "substrate/kernel/status" => CHAN_SINGLETON,
            "substrate/kernel/logs" => CHAN_LOG,
            "substrate/kernel/processes" | "substrate/kernel/services" => CHAN_LIST,
            other => return Err(AdapterError::UnknownTopic(other.into())),
        };
        let id = {
            let mut reg = self.reg.lock();
            reg.allocate()
        };
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let (tx, rx) = mpsc::channel::<StateDelta>(depth);
        self.reg.lock().live.insert(id, cancel_tx);
        spawn_poller(topic.to_string(), args, tx, cancel_rx);
        Ok(Subscription { id, rx })
    }

    async fn close(&self, sub_id: SubId) -> Result<(), AdapterError> {
        // Dropping the stored cancel sender triggers `cancel_rx` on the
        // poller, which exits cleanly. Idempotent: unknown id is a
        // no-op per ADR-009 tombstone discipline.
        let _ = self.reg.lock().live.remove(&sub_id);
        Ok(())
    }
}

/// Spawn the per-topic poller task. Each topic has its own shape —
/// status is a singleton `Replace`, processes/services are whole-list
/// `Replace`s, logs emit `Append` deltas for new entries only.
fn spawn_poller(
    topic: String,
    args: Value,
    tx: mpsc::Sender<StateDelta>,
    cancel_rx: oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        match topic.as_str() {
            "substrate/kernel/status" => poll_status(tx, cancel_rx).await,
            "substrate/kernel/processes" => poll_processes(tx, cancel_rx).await,
            "substrate/kernel/services" => poll_services(tx, cancel_rx).await,
            "substrate/kernel/logs" => poll_logs(args, tx, cancel_rx).await,
            _ => { /* open() validated; unreachable */ }
        }
    });
}

/// Loop helper — call `rpc` on each tick, emit one `Replace` per success.
async fn poll_replace_loop(
    topic_path: &str,
    rpc_method: &'static str,
    period: Duration,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let mut client: Option<DaemonClient> = None;
    let mut ticker = tokio::time::interval(period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = ticker.tick() => {
                if client.is_none() {
                    client = DaemonClient::connect().await;
                }
                let Some(c) = client.as_mut() else { continue; };
                match simple_call(c, rpc_method).await {
                    Ok(value) => {
                        let delta = StateDelta::Replace {
                            path: topic_path.to_string(),
                            value,
                        };
                        if tx.send(delta).await.is_err() {
                            return; // subscriber dropped
                        }
                    }
                    Err(_e) => {
                        // Reset client; the next tick will reconnect.
                        client = None;
                    }
                }
            }
        }
    }
}

async fn poll_status(tx: mpsc::Sender<StateDelta>, cancel_rx: oneshot::Receiver<()>) {
    poll_replace_loop(
        "substrate/kernel/status",
        "kernel.status",
        Duration::from_millis(2000),
        tx,
        cancel_rx,
    )
    .await;
}

async fn poll_processes(tx: mpsc::Sender<StateDelta>, cancel_rx: oneshot::Receiver<()>) {
    poll_replace_loop(
        "substrate/kernel/processes",
        "kernel.ps",
        Duration::from_millis(1000),
        tx,
        cancel_rx,
    )
    .await;
}

async fn poll_services(tx: mpsc::Sender<StateDelta>, cancel_rx: oneshot::Receiver<()>) {
    poll_replace_loop(
        "substrate/kernel/services",
        "kernel.services",
        Duration::from_millis(2000),
        tx,
        cancel_rx,
    )
    .await;
}

/// Logs poller — diffs tail responses and only emits `Append` deltas
/// for entries we haven't seen. Falls back to periodic poll because the
/// daemon does not yet expose a tail-stream RPC; M1.6+ can replace this
/// with a genuine event-driven subscription without changing the topic
/// contract.
async fn poll_logs(
    args: Value,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let tail = args
        .get("tail")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(LOG_TAIL);

    let mut client: Option<DaemonClient> = None;
    let mut ticker = tokio::time::interval(Duration::from_millis(1000));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Track the last seen log entry to diff against. For M1.5 the
    // kernel log entries have `{ ts, ... }` and we compare the top of
    // the tail window as a water-mark. Good enough until the daemon
    // exposes a monotonic log offset.
    let mut watermark: Option<Value> = None;

    loop {
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = ticker.tick() => {
                if client.is_none() {
                    client = DaemonClient::connect().await;
                }
                let Some(c) = client.as_mut() else { continue; };
                let params = json!({ "count": tail });
                match call(c, "kernel.logs", params).await {
                    Ok(value) => {
                        let Some(entries) = value.as_array() else { continue; };
                        let new_entries = diff_tail(entries, watermark.as_ref());
                        for entry in &new_entries {
                            let delta = StateDelta::Append {
                                path: "substrate/kernel/logs".into(),
                                value: (*entry).clone(),
                            };
                            if tx.send(delta).await.is_err() {
                                return;
                            }
                        }
                        if let Some(last) = entries.last() {
                            watermark = Some(last.clone());
                        }
                    }
                    Err(_) => {
                        client = None;
                    }
                }
            }
        }
    }
}

/// Diff a tail window against the previous watermark. Returns the
/// entries strictly after the watermark. If the watermark is unknown
/// (first call), returns the entire window so the consumer sees the
/// current tail on subscription.
fn diff_tail<'a>(entries: &'a [Value], watermark: Option<&Value>) -> Vec<&'a Value> {
    let Some(mark) = watermark else {
        return entries.iter().collect();
    };
    match entries.iter().rposition(|e| e == mark) {
        Some(idx) => entries.iter().skip(idx + 1).collect(),
        None => entries.iter().collect(),
    }
}

// ── RPC helpers ─────────────────────────────────────────────────────

async fn simple_call(
    client: &mut DaemonClient,
    method: &str,
) -> Result<Value, String> {
    let resp = client
        .simple_call(method)
        .await
        .map_err(|e| format!("{method}: {e}"))?;
    if !resp.ok {
        return Err(format!(
            "{method}: {}",
            resp.error.unwrap_or_else(|| "unknown error".into())
        ));
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

async fn call(
    client: &mut DaemonClient,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let resp = client
        .call(Request::with_params(method, params))
        .await
        .map_err(|e| format!("{method}: {e}"))?;
    if !resp.ok {
        return Err(format!(
            "{method}: {}",
            resp.error.unwrap_or_else(|| "unknown error".into())
        ));
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topics_declares_all_four_kernel_entries() {
        let paths: Vec<&str> = TOPICS.iter().map(|t| t.path).collect();
        assert_eq!(
            paths,
            vec![
                "substrate/kernel/status",
                "substrate/kernel/processes",
                "substrate/kernel/services",
                "substrate/kernel/logs",
            ]
        );
    }

    #[test]
    fn topics_sensitivities_are_all_workspace() {
        for t in TOPICS {
            assert_eq!(t.sensitivity, Sensitivity::Workspace, "topic {}", t.path);
        }
    }

    #[test]
    fn status_is_refuse_buffer_logs_are_drop_oldest() {
        let status = TOPICS
            .iter()
            .find(|t| t.path == "substrate/kernel/status")
            .unwrap();
        assert_eq!(status.buffer_policy, BufferPolicy::Refuse);
        let logs = TOPICS
            .iter()
            .find(|t| t.path == "substrate/kernel/logs")
            .unwrap();
        assert_eq!(logs.buffer_policy, BufferPolicy::DropOldest);
    }

    #[test]
    fn permissions_are_empty() {
        let adapter = KernelAdapter::new();
        assert_eq!(adapter.permissions(), PERMISSIONS);
        assert!(PERMISSIONS.is_empty());
    }

    #[test]
    fn id_is_kernel() {
        let adapter = KernelAdapter::new();
        assert_eq!(adapter.id(), "kernel");
    }

    #[test]
    fn diff_tail_returns_everything_on_first_call() {
        let e = vec![json!(1), json!(2), json!(3)];
        let out = diff_tail(&e, None);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn diff_tail_returns_new_after_watermark() {
        let e = vec![json!(1), json!(2), json!(3), json!(4)];
        let mark = json!(2);
        let out = diff_tail(&e, Some(&mark));
        assert_eq!(out.len(), 2);
        assert_eq!(*out[0], json!(3));
    }

    #[test]
    fn diff_tail_with_unknown_watermark_returns_all() {
        let e = vec![json!(5), json!(6)];
        let mark = json!(99);
        let out = diff_tail(&e, Some(&mark));
        assert_eq!(out.len(), 2);
    }
}
