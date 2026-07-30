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
//! | `substrate/kernel/processes` | `ontology://process-list` | periodic 1s | block-capped | list-by-id; per-row `Replace`/`Remove` at `…/by-id/{row_id}` + root list when membership/content changes (WEFT-416) |
//! | `substrate/kernel/services` | `ontology://service-list` | periodic 2s | block-capped | list-by-name; per-row `Replace`/`Remove` at `…/by-name/{row_id}` + root list when membership/content changes (WEFT-416) |
//! | `substrate/kernel/logs` | `ontology://log-ring` | event-driven (`kernel.logs_stream`) | drop-oldest | append-only ring; live Append per entry |
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
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::adapter::{
    AdapterError, BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
    Subscription, TopicDecl,
};
use crate::delta::StateDelta;

use clawft_rpc::{DaemonClient, Request};

/// Default number of log entries to request per `kernel.logs` call.
const LOG_TAIL: usize = 200;

/// Maximum retained log entries in the substrate (ADR-017 §5 — the
/// drop-oldest ring). Declared on the `substrate/kernel/logs` topic's
/// [`TopicDecl::max_len`] so the [`crate::Substrate`] auto-trims
/// append-only list topics. Kept as a named constant so callers can
/// reference the contract value.
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
        max_len: None,
    },
    TopicDecl {
        path: "substrate/kernel/processes",
        shape: "ontology://process-list",
        refresh_hint: RefreshHint::Periodic { ms: 1000 },
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::BlockCapped,
        max_len: None,
    },
    TopicDecl {
        path: "substrate/kernel/services",
        shape: "ontology://service-list",
        refresh_hint: RefreshHint::Periodic { ms: 2000 },
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::BlockCapped,
        max_len: None,
    },
    TopicDecl {
        path: "substrate/kernel/logs",
        shape: "ontology://log-ring",
        // WEFT-434: genuine event-driven stream via `kernel.logs_stream`.
        refresh_hint: RefreshHint::EventDriven,
        sensitivity: Sensitivity::Workspace,
        buffer_policy: BufferPolicy::DropOldest,
        max_len: Some(LOG_RING),
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

    async fn open(&self, topic: &str, args: Value) -> Result<Subscription, AdapterError> {
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
/// status is a singleton `Replace`, processes/services emit per-row
/// `Replace`/`Remove` deltas (WEFT-416), logs emit `Append` deltas
/// for new entries only.
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
            "substrate/kernel/logs" => stream_logs(args, tx, cancel_rx).await,
            _ => { /* open() validated; unreachable */ }
        }
    });
}

// ── Per-id list deltas (WEFT-416) ───────────────────────────────────

/// Path segment under which process rows are stored:
/// `substrate/kernel/processes/by-id/{row_id}`.
const PROCESSES_BY_ID: &str = "substrate/kernel/processes/by-id";
/// Path segment under which service rows are stored:
/// `substrate/kernel/services/by-name/{row_id}`.
const SERVICES_BY_NAME: &str = "substrate/kernel/services/by-name";
/// Topic root for the process list (array — backward-compat for table UIs).
const PROCESSES_ROOT: &str = "substrate/kernel/processes";
/// Topic root for the service list (array — backward-compat for table UIs).
const SERVICES_ROOT: &str = "substrate/kernel/services";

/// Sanitize a row_id so it is safe as a single path segment
/// (no `/`, no empty). Empty ids become `"_"`; slashes become `_`.
fn sanitize_row_id(id: &str) -> String {
    if id.is_empty() {
        return "_".into();
    }
    if id.contains('/') {
        id.replace('/', "_")
    } else {
        id.to_string()
    }
}

/// Extract a stable row id from a process row (daemon row-id contract).
///
/// Preference order:
/// 1. explicit `row_id` string (WEFT-416 daemon contract)
/// 2. composite `{pid}:{agent_id}` when both present (legacy daemons;
///    services-as-process share the daemon pid so pid alone is not unique)
/// 3. `pid` alone
/// 4. `agent_id` alone
fn process_row_id(row: &Value) -> Option<String> {
    if let Some(id) = row.get("row_id").and_then(|v| v.as_str())
        && !id.is_empty()
    {
        return Some(sanitize_row_id(id));
    }
    let pid = row.get("pid").and_then(|v| v.as_u64());
    let agent = row.get("agent_id").and_then(|v| v.as_str());
    match (pid, agent) {
        (Some(p), Some(a)) if !a.is_empty() => Some(sanitize_row_id(&format!("{p}:{a}"))),
        (Some(p), _) => Some(format!("pid:{p}")),
        (None, Some(a)) if !a.is_empty() => Some(sanitize_row_id(a)),
        _ => None,
    }
}

/// Extract a stable row id from a service row.
///
/// Preference: explicit `row_id`, then `name`.
fn service_row_id(row: &Value) -> Option<String> {
    if let Some(id) = row.get("row_id").and_then(|v| v.as_str())
        && !id.is_empty()
    {
        return Some(sanitize_row_id(id));
    }
    row.get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(sanitize_row_id)
}

/// Diff a keyed list against the previous snapshot and produce
/// per-row [`StateDelta`]s.
///
/// # Semantics
///
/// - New or changed row → `Replace` at `{row_prefix}/{row_id}`
/// - Missing row (was present, now gone) → `Remove` at `{row_prefix}/{row_id}`
/// - When any per-row delta is produced **or** the root array order /
///   content changed, also emit a root-list `Replace` at `list_root`
///   so table UIs that bind the whole array keep working.
/// - Steady state (identical map of row_id → value) → empty vec.
///
/// `prev` is updated in place to match `next` so the caller can reuse
/// it as the watermark for the next tick.
///
/// # Arguments
///
/// * `prev` — prior tick's `row_id → row` map (mutated to match next)
/// * `next_rows` — projected array from the daemon this tick
/// * `row_id_of` — extracts a stable key from one row (`None` skips it)
/// * `row_prefix` — absolute path prefix for per-row cells
/// * `list_root` — absolute path of the whole-list array (compat)
fn diff_keyed_list(
    prev: &mut std::collections::BTreeMap<String, Value>,
    next_rows: &[Value],
    row_id_of: impl Fn(&Value) -> Option<String>,
    row_prefix: &str,
    list_root: &str,
) -> Vec<StateDelta> {
    let mut next_map: std::collections::BTreeMap<String, Value> =
        std::collections::BTreeMap::new();
    let mut ordered: Vec<Value> = Vec::with_capacity(next_rows.len());
    for row in next_rows {
        let Some(id) = row_id_of(row) else {
            // Unkeyed row: still include it in the root list so we
            // don't silently drop data, but skip per-id tracking.
            ordered.push(row.clone());
            continue;
        };
        ordered.push(row.clone());
        next_map.insert(id, row.clone());
    }

    let mut deltas = Vec::new();

    // Removals first (ids present last tick, gone now).
    for id in prev.keys() {
        if !next_map.contains_key(id) {
            deltas.push(StateDelta::Remove {
                path: format!("{row_prefix}/{id}"),
            });
        }
    }

    // Inserts / updates.
    for (id, value) in &next_map {
        match prev.get(id) {
            Some(old) if old == value => { /* unchanged */ }
            _ => {
                deltas.push(StateDelta::Replace {
                    path: format!("{row_prefix}/{id}"),
                    value: value.clone(),
                });
            }
        }
    }

    // Root list for table consumers — only when anything changed.
    // On the first tick `prev` is empty so any non-empty next yields
    // per-row deltas and a root replace; steady-state emits nothing.
    // An empty→empty transition (never had rows) also emits nothing;
    // UIs treat a missing root path as "not yet polled / no data".
    if !deltas.is_empty() {
        deltas.push(StateDelta::Replace {
            path: list_root.to_string(),
            value: Value::Array(ordered),
        });
    }

    *prev = next_map;
    deltas
}

/// Loop helper — call `rpc` on each tick, emit one `Replace` per success.
async fn poll_replace_loop(
    topic_path: &str,
    rpc_method: &'static str,
    period: Duration,
    tx: mpsc::Sender<StateDelta>,
    cancel_rx: oneshot::Receiver<()>,
) {
    poll_replace_loop_with_projection(topic_path, rpc_method, period, |v| v, tx, cancel_rx).await;
}

/// Same as [`poll_replace_loop`] but applies `project` to each RPC
/// response before emitting the `Replace` delta. Used to align the
/// daemon's wire shape with the user-facing admin ontology without
/// changing the RPC contract itself.
async fn poll_replace_loop_with_projection<F>(
    topic_path: &str,
    rpc_method: &'static str,
    period: Duration,
    project: F,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) where
    F: Fn(Value) -> Value + Send + 'static,
{
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
                            value: project(value),
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

/// Processes poller — WEFT-416 per-id deltas.
///
/// Diffs the projected `kernel.ps` array against the previous tick
/// and emits:
/// - `Replace` at `substrate/kernel/processes/by-id/{row_id}` for
///   new/changed rows
/// - `Remove` for rows that disappeared
/// - a root-list `Replace` at `substrate/kernel/processes` when any
///   per-row delta was produced (table-UI backward compat)
///
/// Steady-state ticks (identical row map) emit **zero** deltas.
async fn poll_processes(tx: mpsc::Sender<StateDelta>, cancel_rx: oneshot::Receiver<()>) {
    poll_keyed_list_loop(
        "kernel.ps",
        Duration::from_millis(1000),
        crate::projection::project_process_rows,
        process_row_id,
        PROCESSES_BY_ID,
        PROCESSES_ROOT,
        tx,
        cancel_rx,
    )
    .await;
}

/// Services poller — WEFT-416 per-id deltas (see [`poll_processes`]).
async fn poll_services(tx: mpsc::Sender<StateDelta>, cancel_rx: oneshot::Receiver<()>) {
    poll_keyed_list_loop(
        "kernel.services",
        Duration::from_millis(2000),
        crate::projection::project_service_rows,
        service_row_id,
        SERVICES_BY_NAME,
        SERVICES_ROOT,
        tx,
        cancel_rx,
    )
    .await;
}

/// Shared poll loop for list-by-id topics (processes / services).
#[allow(clippy::too_many_arguments)]
async fn poll_keyed_list_loop<F, K>(
    rpc_method: &'static str,
    period: Duration,
    project: F,
    row_id_of: K,
    row_prefix: &'static str,
    list_root: &'static str,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) where
    F: Fn(Value) -> Value + Send + 'static,
    K: Fn(&Value) -> Option<String> + Send + 'static,
{
    let mut client: Option<DaemonClient> = None;
    let mut ticker = tokio::time::interval(period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut prev: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();

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
                        let projected = project(value);
                        let rows = match projected.as_array() {
                            Some(arr) => arr.as_slice(),
                            // Non-array response: fall back to whole-path Replace
                            // so we never silently drop the payload.
                            None => {
                                let delta = StateDelta::Replace {
                                    path: list_root.to_string(),
                                    value: projected,
                                };
                                if tx.send(delta).await.is_err() {
                                    return;
                                }
                                prev.clear();
                                continue;
                            }
                        };
                        let deltas = diff_keyed_list(
                            &mut prev,
                            rows,
                            &row_id_of,
                            row_prefix,
                            list_root,
                        );
                        for delta in deltas {
                            if tx.send(delta).await.is_err() {
                                return; // subscriber dropped
                            }
                        }
                    }
                    Err(_e) => {
                        client = None;
                    }
                }
            }
        }
    }
}

/// Logs streamer (WEFT-434) — opens `kernel.logs_stream` and emits one
/// `Append` delta per entry. Event-driven: no periodic poll of
/// `kernel.logs`. On disconnect, reconnects with exponential backoff
/// (connection resilience, not a poll fallback).
///
/// Entries carry a monotonic `seq` from the daemon. Across reconnects
/// we skip frames with `seq <= last_seq` so the initial tail replay
/// does not re-emit already-seen lines.
async fn stream_logs(
    args: Value,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let tail = args
        .get("tail")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(LOG_TAIL);

    let mut last_seq: u64 = 0;
    let mut backoff = Duration::from_millis(200);

    loop {
        // Wait for a daemon connection (or cancel).
        let client = loop {
            tokio::select! {
                _ = &mut cancel_rx => return,
                c = DaemonClient::connect() => {
                    if let Some(c) = c {
                        break c;
                    }
                }
            }
            tokio::select! {
                _ = &mut cancel_rx => return,
                _ = tokio::time::sleep(backoff) => {}
            }
            backoff = (backoff.saturating_mul(2)).min(Duration::from_secs(5));
        };

        let params = json!({ "count": tail });
        let req = Request::with_params("kernel.logs_stream", params);
        let open = client.open_stream(req).await;
        let (ack, mut session) = match open {
            Ok(pair) if pair.0.ok => {
                backoff = Duration::from_millis(200);
                pair
            }
            _ => {
                // Stream open failed — back off and retry.
                tokio::select! {
                    _ = &mut cancel_rx => return,
                    _ = tokio::time::sleep(backoff) => {}
                }
                backoff = (backoff.saturating_mul(2)).min(Duration::from_secs(5));
                continue;
            }
        };
        let _ = ack; // streaming:true ack; frames follow on the session

        // Pump frames until cancel or stream EOF/error.
        let pump_result = pump_log_stream(&mut session, &tx, &mut last_seq, &mut cancel_rx).await;
        if pump_result == PumpOutcome::Cancelled || pump_result == PumpOutcome::SubscriberGone {
            return;
        }
        // Disconnected — reconnect after backoff.
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = tokio::time::sleep(backoff) => {}
        }
        backoff = (backoff.saturating_mul(2)).min(Duration::from_secs(5));
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PumpOutcome {
    /// Subscription cancel fired.
    Cancelled,
    /// Downstream substrate consumer dropped.
    SubscriberGone,
    /// Stream ended or errored; caller should reconnect.
    Disconnected,
}

/// Read log frames from an open stream session and Append them.
async fn pump_log_stream(
    session: &mut clawft_rpc::StreamSession,
    tx: &mpsc::Sender<StateDelta>,
    last_seq: &mut u64,
    cancel_rx: &mut oneshot::Receiver<()>,
) -> PumpOutcome {
    loop {
        tokio::select! {
            _ = &mut *cancel_rx => return PumpOutcome::Cancelled,
            line = session.next_line() => {
                match line {
                    Ok(Some(raw)) => {
                        let trimmed = raw.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let Ok(entry) = serde_json::from_str::<Value>(trimmed) else {
                            continue;
                        };
                        if let Some(seq) = entry.get("seq").and_then(|v| v.as_u64()) {
                            if seq > 0 && seq <= *last_seq {
                                continue;
                            }
                            if seq > *last_seq {
                                *last_seq = seq;
                            }
                        }
                        let delta = StateDelta::Append {
                            path: "substrate/kernel/logs".into(),
                            value: entry,
                        };
                        if tx.send(delta).await.is_err() {
                            return PumpOutcome::SubscriberGone;
                        }
                    }
                    Ok(None) | Err(_) => return PumpOutcome::Disconnected,
                }
            }
        }
    }
}

/// Result of a `diff_tail` — either a batch of new entries or a
/// synthetic overflow notice. Kept for unit tests of the historical
/// option-2 poll-window logic; production path uses streaming + seq
/// (WEFT-434 option-1).
#[cfg(test)]
#[derive(Debug, PartialEq)]
enum DiffTailOutcome<'a> {
    /// Zero-or-more new entries strictly newer than the watermark.
    New(Vec<&'a Value>),
    /// The previous watermark was not found in the current window and
    /// the window is at the poll-buffer limit — entries were lost
    /// between ticks.
    Overflow {
        /// Best-effort count of entries that fell off the window.
        lost_estimate: usize,
    },
}

/// Diff a tail window against the previous watermark.
///
/// Historical option-2 (capped-tail) helper. Production log path now
/// uses `kernel.logs_stream` + monotonic `seq` (WEFT-434); this remains
/// for unit coverage of the prior algorithm.
#[cfg(test)]
fn diff_tail<'a>(
    entries: &'a [Value],
    watermark: Option<&Value>,
    window_cap: usize,
) -> DiffTailOutcome<'a> {
    let Some(mark) = watermark else {
        return DiffTailOutcome::New(entries.iter().collect());
    };
    match entries.iter().rposition(|e| e == mark) {
        Some(idx) => DiffTailOutcome::New(entries.iter().skip(idx + 1).collect()),
        None => {
            if entries.len() >= window_cap {
                DiffTailOutcome::Overflow {
                    lost_estimate: entries.len(),
                }
            } else {
                DiffTailOutcome::New(entries.iter().collect())
            }
        }
    }
}

// ── RPC helpers ─────────────────────────────────────────────────────

async fn simple_call(client: &mut DaemonClient, method: &str) -> Result<Value, String> {
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
        match diff_tail(&e, None, LOG_TAIL) {
            DiffTailOutcome::New(out) => assert_eq!(out.len(), 3),
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn diff_tail_returns_new_after_watermark() {
        let e = vec![json!(1), json!(2), json!(3), json!(4)];
        let mark = json!(2);
        match diff_tail(&e, Some(&mark), LOG_TAIL) {
            DiffTailOutcome::New(out) => {
                assert_eq!(out.len(), 2);
                assert_eq!(*out[0], json!(3));
                assert_eq!(*out[1], json!(4));
            }
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn diff_tail_missing_watermark_undercapped_returns_all() {
        // Window is smaller than the poll cap → daemon likely reset.
        // Safe to treat as a fresh first-call.
        let e = vec![json!(5), json!(6)];
        let mark = json!(99);
        match diff_tail(&e, Some(&mark), LOG_TAIL) {
            DiffTailOutcome::New(out) => assert_eq!(out.len(), 2),
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn diff_tail_missing_watermark_at_cap_reports_overflow() {
        // Window == cap, watermark missing → overflow: we can't tell
        // if these are all new or partially duplicated. Return a
        // synthetic overflow rather than re-emit the whole window.
        let cap = 4;
        let e = vec![json!(10), json!(11), json!(12), json!(13)];
        let mark = json!(1);
        match diff_tail(&e, Some(&mark), cap) {
            DiffTailOutcome::Overflow { lost_estimate } => {
                assert_eq!(lost_estimate, cap);
            }
            other => panic!("expected Overflow, got {other:?}"),
        }
    }

    #[test]
    fn diff_tail_empty_window_returns_empty_new() {
        let e: Vec<Value> = vec![];
        match diff_tail(&e, None, LOG_TAIL) {
            DiffTailOutcome::New(out) => assert!(out.is_empty()),
            other => panic!("expected New, got {other:?}"),
        }
        let mark = json!(1);
        match diff_tail(&e, Some(&mark), LOG_TAIL) {
            // Empty window with a watermark: cannot be overflow (len
            // == 0, below any sensible cap) — treated as "no new".
            DiffTailOutcome::New(out) => assert!(out.is_empty()),
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn diff_tail_watermark_is_last_entry_returns_empty() {
        let e = vec![json!(1), json!(2), json!(3)];
        let mark = json!(3);
        match diff_tail(&e, Some(&mark), LOG_TAIL) {
            DiffTailOutcome::New(out) => assert!(out.is_empty()),
            other => panic!("expected New, got {other:?}"),
        }
    }

    #[test]
    fn logs_topic_is_event_driven() {
        let logs = TOPICS
            .iter()
            .find(|t| t.path == "substrate/kernel/logs")
            .unwrap();
        assert_eq!(logs.refresh_hint, RefreshHint::EventDriven);
    }

    #[test]
    fn seq_skip_logic_drops_stale_reconnect_replay() {
        // Mirrors pump_log_stream reconnect de-dupe: frames with
        // seq <= last_seq are ignored; higher seq advances watermark.
        let mut last_seq = 5u64;
        let frames = [
            json!({"seq": 3, "message": "stale"}),
            json!({"seq": 5, "message": "boundary"}),
            json!({"seq": 6, "message": "fresh"}),
            json!({"seq": 0, "message": "legacy-no-seq"}),
        ];
        let mut kept = Vec::new();
        for entry in &frames {
            if let Some(seq) = entry.get("seq").and_then(|v| v.as_u64()) {
                if seq > 0 && seq <= last_seq {
                    continue;
                }
                if seq > last_seq {
                    last_seq = seq;
                }
            }
            kept.push(entry.get("message").and_then(|v| v.as_str()).unwrap());
        }
        assert_eq!(kept, vec!["fresh", "legacy-no-seq"]);
        assert_eq!(last_seq, 6);
    }

    // ── WEFT-416: per-id list deltas ───────────────────────────────

    #[test]
    fn process_row_id_prefers_explicit_row_id() {
        let row = json!({
            "row_id": "pid:42",
            "pid": 42,
            "agent_id": "kernel"
        });
        assert_eq!(process_row_id(&row).as_deref(), Some("pid:42"));
    }

    #[test]
    fn process_row_id_falls_back_to_pid_agent_composite() {
        // Legacy daemon without row_id: composite keeps service-virtual
        // rows (shared daemon pid) unique.
        let row = json!({"pid": 7, "agent_id": "mesh"});
        assert_eq!(process_row_id(&row).as_deref(), Some("7:mesh"));
    }

    #[test]
    fn service_row_id_uses_name() {
        let row = json!({"name": "cron", "state": "running"});
        assert_eq!(service_row_id(&row).as_deref(), Some("cron"));
        let with_id = json!({"row_id": "cron", "name": "cron"});
        assert_eq!(service_row_id(&with_id).as_deref(), Some("cron"));
    }

    #[test]
    fn sanitize_row_id_strips_slashes() {
        assert_eq!(sanitize_row_id("a/b"), "a_b");
        assert_eq!(sanitize_row_id(""), "_");
        assert_eq!(sanitize_row_id("pid:1"), "pid:1");
    }

    #[test]
    fn diff_keyed_list_first_tick_emits_per_row_and_root() {
        let mut prev = std::collections::BTreeMap::new();
        let rows = vec![
            json!({"row_id": "pid:1", "pid": 1, "state": "running"}),
            json!({"row_id": "pid:2", "pid": 2, "state": "running"}),
        ];
        let deltas = diff_keyed_list(
            &mut prev,
            &rows,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        // 2 per-row Replace + 1 root list Replace
        assert_eq!(deltas.len(), 3);
        assert!(
            deltas.iter().any(|d| {
                matches!(d, StateDelta::Replace { path, .. }
                    if path == "substrate/kernel/processes/by-id/pid:1")
            }),
            "missing by-id Replace for pid:1: {deltas:?}"
        );
        assert!(
            deltas.iter().any(|d| {
                matches!(d, StateDelta::Replace { path, .. }
                    if path == "substrate/kernel/processes/by-id/pid:2")
            }),
            "missing by-id Replace for pid:2: {deltas:?}"
        );
        let root = deltas.iter().find(|d| {
            matches!(d, StateDelta::Replace { path, .. } if path == PROCESSES_ROOT)
        });
        match root {
            Some(StateDelta::Replace { value, .. }) => {
                assert_eq!(value.as_array().map(|a| a.len()), Some(2));
            }
            other => panic!("expected root Replace, got {other:?}"),
        }
        assert_eq!(prev.len(), 2);
    }

    #[test]
    fn diff_keyed_list_steady_state_emits_zero_deltas() {
        let mut prev = std::collections::BTreeMap::new();
        let rows = vec![
            json!({"row_id": "pid:1", "pid": 1, "cpu_time_ms": 100}),
            json!({"row_id": "svc:mesh", "pid": 99, "agent_id": "mesh"}),
        ];
        let first = diff_keyed_list(
            &mut prev,
            &rows,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        assert!(!first.is_empty(), "first tick must seed state");

        // Identical payload on the next tick — zero deltas (the whole
        // point of WEFT-416: reduced steady-state traffic).
        let second = diff_keyed_list(
            &mut prev,
            &rows,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        assert!(
            second.is_empty(),
            "steady-state must emit no deltas, got {second:?}"
        );
    }

    #[test]
    fn diff_keyed_list_one_row_change_emits_single_replace_plus_root() {
        let mut prev = std::collections::BTreeMap::new();
        let rows_v1 = vec![
            json!({"row_id": "pid:1", "pid": 1, "cpu_time_ms": 100}),
            json!({"row_id": "pid:2", "pid": 2, "cpu_time_ms": 50}),
        ];
        let _ = diff_keyed_list(
            &mut prev,
            &rows_v1,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );

        let rows_v2 = vec![
            json!({"row_id": "pid:1", "pid": 1, "cpu_time_ms": 100}), // unchanged
            json!({"row_id": "pid:2", "pid": 2, "cpu_time_ms": 75}),  // changed
        ];
        let deltas = diff_keyed_list(
            &mut prev,
            &rows_v2,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        // 1 per-row Replace + 1 root list
        assert_eq!(deltas.len(), 2, "expected single-row update + root: {deltas:?}");
        assert!(
            deltas.iter().any(|d| {
                matches!(d, StateDelta::Replace { path, value, .. }
                    if path == "substrate/kernel/processes/by-id/pid:2"
                        && value["cpu_time_ms"] == 75)
            }),
            "missing updated pid:2: {deltas:?}"
        );
        assert!(
            !deltas.iter().any(|d| {
                matches!(d, StateDelta::Replace { path, .. }
                    if path == "substrate/kernel/processes/by-id/pid:1")
            }),
            "unchanged pid:1 must not re-emit"
        );
    }

    #[test]
    fn diff_keyed_list_removal_emits_remove_plus_root() {
        let mut prev = std::collections::BTreeMap::new();
        let rows_v1 = vec![
            json!({"row_id": "cron", "name": "cron", "state": "running"}),
            json!({"row_id": "mesh", "name": "mesh", "state": "running"}),
        ];
        let _ = diff_keyed_list(
            &mut prev,
            &rows_v1,
            service_row_id,
            SERVICES_BY_NAME,
            SERVICES_ROOT,
        );

        let rows_v2 = vec![json!({"row_id": "cron", "name": "cron", "state": "running"})];
        let deltas = diff_keyed_list(
            &mut prev,
            &rows_v2,
            service_row_id,
            SERVICES_BY_NAME,
            SERVICES_ROOT,
        );
        assert!(
            deltas.iter().any(|d| {
                matches!(d, StateDelta::Remove { path }
                    if path == "substrate/kernel/services/by-name/mesh")
            }),
            "expected Remove for mesh: {deltas:?}"
        );
        // Root list should now have only cron.
        let root = deltas.iter().find(|d| {
            matches!(d, StateDelta::Replace { path, .. } if path == SERVICES_ROOT)
        });
        match root {
            Some(StateDelta::Replace { value, .. }) => {
                assert_eq!(value.as_array().map(|a| a.len()), Some(1));
                assert_eq!(value[0]["name"], "cron");
            }
            other => panic!("expected root Replace, got {other:?}"),
        }
        assert!(!prev.contains_key("mesh"));
        assert!(prev.contains_key("cron"));
    }

    #[test]
    fn diff_keyed_list_new_row_emits_replace_plus_root() {
        let mut prev = std::collections::BTreeMap::new();
        let rows_v1 = vec![json!({"row_id": "pid:1", "pid": 1})];
        let _ = diff_keyed_list(
            &mut prev,
            &rows_v1,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );

        let rows_v2 = vec![
            json!({"row_id": "pid:1", "pid": 1}),
            json!({"row_id": "svc:cron", "pid": 99, "agent_id": "cron"}),
        ];
        let deltas = diff_keyed_list(
            &mut prev,
            &rows_v2,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        assert!(
            deltas.iter().any(|d| {
                matches!(d, StateDelta::Replace { path, .. }
                    if path == "substrate/kernel/processes/by-id/svc:cron")
            }),
            "expected Replace for new svc:cron: {deltas:?}"
        );
        // Unchanged pid:1 must not re-emit.
        assert!(
            !deltas.iter().any(|d| {
                matches!(d, StateDelta::Replace { path, .. }
                    if path == "substrate/kernel/processes/by-id/pid:1")
            }),
            "unchanged pid:1 must not re-emit"
        );
    }

    #[test]
    fn diff_keyed_list_steady_state_delta_size_strictly_less_than_full_replace() {
        // AC: "Tests confirm reduced delta size on steady-state ticks."
        // Old behaviour: 1 whole-list Replace every tick (payload = full
        // array). New behaviour: 0 deltas on steady state.
        let mut prev = std::collections::BTreeMap::new();
        let mut rows = Vec::new();
        for i in 0..20 {
            rows.push(json!({
                "row_id": format!("pid:{i}"),
                "pid": i,
                "agent_id": format!("agent-{i}"),
                "cpu_time_ms": i * 10,
                "memory_bytes": i * 1024,
                "state": "running",
            }));
        }
        let seed = diff_keyed_list(
            &mut prev,
            &rows,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        // First tick: 20 by-id + 1 root.
        assert_eq!(seed.len(), 21);

        let steady = diff_keyed_list(
            &mut prev,
            &rows,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        assert_eq!(
            steady.len(),
            0,
            "steady-state delta count must be 0 (was whole-list Replace of 20 rows)"
        );

        // Single-row mutation: only 1 by-id + root, not 20 by-id.
        rows[5] = json!({
            "row_id": "pid:5",
            "pid": 5,
            "agent_id": "agent-5",
            "cpu_time_ms": 999,
            "memory_bytes": 5 * 1024,
            "state": "running",
        });
        let partial = diff_keyed_list(
            &mut prev,
            &rows,
            process_row_id,
            PROCESSES_BY_ID,
            PROCESSES_ROOT,
        );
        assert_eq!(
            partial.len(),
            2,
            "one-row change → 1 Replace + 1 root, got {partial:?}"
        );
    }
}
