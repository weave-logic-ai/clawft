//! Swarm coordinator: fan-out / collect over the [`AgentBus`](super::AgentBus).
//!
//! [`SwarmCoordinator`] implements the lead-agent pattern: dispatch
//! subtasks to registered workers, optionally spawn worker loops, and
//! collect replies correlated by `reply_to`.
//!
//! # Topology (WEFT-199)
//!
//! Runtime coordination is **flat fan-out / collect**
//! ([`SwarmTopology::Flat`]). Labels such as `mesh`, `hierarchical`, and
//! `adaptive` exist as [`SwarmTopology`] variants for documentation and
//! future axes, but they do **not** change dispatch behaviour today.
//!
//! Claude-flow agent names (`mesh-coordinator`, `hierarchical-coordinator`,
//! `adaptive-coordinator`) are **prompt-only** swarm roles used by the
//! Ruflo / claude-flow harness — not separate in-tree coordinator
//! implementations. See `docs/architecture/swarm-topology.md`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::debug;
use uuid::Uuid;

use clawft_types::agent_bus::{AgentBusError, InterAgentMessage};

use super::bus::{AgentBus, AgentInbox, DEFAULT_INBOX_CAPACITY};
use super::worker::{
    spawn_worker_loop, spawn_worker_loop_with_ttl, RuntimeBackedWorker, WorkerHandler,
    DEFAULT_REPLY_TTL,
};

/// Declared swarm topology axis (WEFT-199).
///
/// Only [`Self::Flat`] is implemented by [`SwarmCoordinator`] today:
/// the lead dispatches to every registered worker and collects replies.
/// Other variants are **reserved labels** so hosts / docs can name the
/// intent without implying distinct runtime graphs. Claude-flow prompt
/// agents (`mesh-coordinator`, etc.) remain external to this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SwarmTopology {
    /// Lead fan-out to all workers, collect by `reply_to` (implemented).
    #[default]
    Flat,
    /// Reserved: peer-to-peer mesh (claude-flow prompt topology only).
    Mesh,
    /// Reserved: tree of coordinators (claude-flow prompt topology only).
    Hierarchical,
    /// Reserved: dynamic topology switching (claude-flow prompt only).
    Adaptive,
}

impl SwarmTopology {
    /// Whether this topology changes [`SwarmCoordinator`] dispatch today.
    pub fn is_runtime_implemented(self) -> bool {
        matches!(self, Self::Flat)
    }

    /// Stable string id for logs / CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Mesh => "mesh",
            Self::Hierarchical => "hierarchical",
            Self::Adaptive => "adaptive",
        }
    }
}

/// Coordinator for dispatching subtasks to worker agents and collecting
/// results via the [`AgentBus`].
///
/// Implements the coordinator pattern: a lead agent dispatches work to
/// worker agents and waits for their replies. Topology is always flat
/// fan-out at runtime (see [`SwarmTopology`]).
pub struct SwarmCoordinator {
    /// Shared agent bus for message delivery.
    bus: Arc<AgentBus>,
    /// Agent ID of the coordinator.
    coordinator_id: String,
    /// Registered worker agent IDs.
    worker_agents: Vec<String>,
    /// Declared topology (informational; only Flat is implemented).
    topology: SwarmTopology,
    /// Coordinator inbox (created when collect / spawn is used).
    inbox: Option<AgentInbox>,
    /// Live worker join handles (populated by [`Self::spawn_workers`]).
    worker_handles: Vec<JoinHandle<()>>,
}

impl SwarmCoordinator {
    /// Create a new coordinator bound to an existing bus.
    pub fn new(
        bus: Arc<AgentBus>,
        coordinator_id: impl Into<String>,
        worker_agents: Vec<String>,
    ) -> Self {
        Self {
            bus,
            coordinator_id: coordinator_id.into(),
            worker_agents,
            topology: SwarmTopology::Flat,
            inbox: None,
            worker_handles: Vec::new(),
        }
    }

    /// Create a coordinator **and** a capacity-bounded bus in one step.
    ///
    /// This is the production convenience constructor that exercises
    /// [`AgentBus::with_capacity`] (previously unused outside tests).
    ///
    /// Returns `(coordinator, bus)` so callers can share the bus with
    /// other host components (e.g. `AppContext::set_agent_bus`).
    pub fn with_capacity(
        coordinator_id: impl Into<String>,
        worker_agents: Vec<String>,
        capacity: usize,
    ) -> (Self, Arc<AgentBus>) {
        let bus = Arc::new(AgentBus::with_capacity(capacity));
        let coord = Self::new(bus.clone(), coordinator_id, worker_agents);
        (coord, bus)
    }

    /// Create a coordinator with the default inbox capacity.
    pub fn with_default_capacity(
        coordinator_id: impl Into<String>,
        worker_agents: Vec<String>,
    ) -> (Self, Arc<AgentBus>) {
        Self::with_capacity(coordinator_id, worker_agents, DEFAULT_INBOX_CAPACITY)
    }

    /// Record a declared topology label (WEFT-199).
    ///
    /// Non-flat values are accepted for telemetry / docs alignment but
    /// **do not** change fan-out behaviour. Prefer leaving the default
    /// [`SwarmTopology::Flat`] unless you are intentionally tagging a
    /// claude-flow prompt topology for observability.
    pub fn with_topology(mut self, topology: SwarmTopology) -> Self {
        if !topology.is_runtime_implemented() {
            debug!(
                topology = topology.as_str(),
                "SwarmTopology is prompt/docs-only; runtime remains flat fan-out"
            );
        }
        self.topology = topology;
        self
    }

    /// Declared topology (always flat fan-out at runtime today).
    pub fn topology(&self) -> SwarmTopology {
        self.topology
    }

    /// Shared bus handle.
    pub fn bus(&self) -> &Arc<AgentBus> {
        &self.bus
    }

    /// Coordinator agent ID.
    pub fn coordinator_id(&self) -> &str {
        &self.coordinator_id
    }

    /// List of registered worker agent IDs.
    pub fn workers(&self) -> &[String] {
        &self.worker_agents
    }

    /// Ensure the coordinator is registered on the bus and owns an inbox.
    pub async fn ensure_registered(&mut self) {
        if self.inbox.is_none() {
            let inbox = self.bus.register_agent(&self.coordinator_id).await;
            self.inbox = Some(inbox);
        }
    }

    /// Dispatch a subtask to a specific worker agent.
    ///
    /// Sends an [`InterAgentMessage`] to the worker and returns the
    /// message ID for later correlation.
    pub async fn dispatch_subtask(
        &self,
        task: &str,
        worker: &str,
        payload: serde_json::Value,
        ttl: Duration,
    ) -> Result<Uuid, AgentBusError> {
        let msg = InterAgentMessage::new(&self.coordinator_id, worker, task, payload, ttl);
        let msg_id = msg.id;
        self.bus.send(msg).await?;
        debug!(
            coordinator = %self.coordinator_id,
            worker = %worker,
            task = %task,
            msg_id = %msg_id,
            "dispatched subtask"
        );
        Ok(msg_id)
    }

    /// Dispatch the same task to all registered workers.
    ///
    /// Returns a list of (worker_id, message_id) pairs.
    pub async fn broadcast_task(
        &self,
        task: &str,
        payload: serde_json::Value,
        ttl: Duration,
    ) -> Vec<(String, Result<Uuid, AgentBusError>)> {
        let mut results = Vec::new();
        for worker in &self.worker_agents {
            let result = self
                .dispatch_subtask(task, worker, payload.clone(), ttl)
                .await;
            results.push((worker.clone(), result));
        }
        results
    }

    /// Spawn echo worker loops for every configured worker id.
    ///
    /// Each worker is registered on the bus and runs
    /// [`worker_message_loop`](super::worker::worker_message_loop) in a
    /// background task. The coordinator is also registered so replies
    /// can be collected.
    pub async fn spawn_workers(&mut self) {
        self.ensure_registered().await;
        for worker_id in self.worker_agents.clone() {
            let handler: Arc<dyn WorkerHandler> =
                Arc::new(RuntimeBackedWorker::echo(worker_id));
            let handle = spawn_worker_loop(self.bus.clone(), handler).await;
            self.worker_handles.push(handle);
        }
    }

    /// Spawn custom handlers (one per configured worker, by agent_id match).
    ///
    /// Handlers whose `agent_id` is not in `worker_agents` are ignored.
    /// Workers without a matching handler are not spawned.
    pub async fn spawn_handlers(&mut self, handlers: Vec<Arc<dyn WorkerHandler>>) {
        self.ensure_registered().await;
        let wanted: HashSet<String> = self.worker_agents.iter().cloned().collect();
        for handler in handlers {
            if !wanted.contains(handler.agent_id()) {
                continue;
            }
            let handle =
                spawn_worker_loop_with_ttl(self.bus.clone(), handler, DEFAULT_REPLY_TTL).await;
            self.worker_handles.push(handle);
        }
    }

    /// Collect replies that correlate to the given outbound message IDs.
    ///
    /// Blocks until every id has a reply or `timeout` elapses. Returns
    /// a map of original-msg-id → reply message for those that arrived.
    pub async fn collect_replies(
        &mut self,
        expected: &[Uuid],
        timeout: Duration,
    ) -> HashMap<Uuid, InterAgentMessage> {
        self.ensure_registered().await;
        let inbox = self
            .inbox
            .as_mut()
            .expect("ensure_registered always sets inbox");

        let mut remaining: HashSet<Uuid> = expected.iter().copied().collect();
        let mut collected: HashMap<Uuid, InterAgentMessage> = HashMap::new();
        let deadline = tokio::time::Instant::now() + timeout;

        while !remaining.is_empty() {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match tokio::time::timeout(left, inbox.recv()).await {
                Ok(Some(msg)) => {
                    if let Some(parent) = msg.reply_to
                        && remaining.remove(&parent)
                    {
                        collected.insert(parent, msg);
                    }
                }
                Ok(None) => break, // bus closed
                Err(_) => break,   // timeout
            }
        }
        collected
    }

    /// Dispatch one task per worker, then collect replies.
    ///
    /// End-to-end convenience used by demos and e2e tests.
    pub async fn dispatch_and_collect(
        &mut self,
        task: &str,
        payload: serde_json::Value,
        dispatch_ttl: Duration,
        collect_timeout: Duration,
    ) -> Vec<(String, Result<InterAgentMessage, AgentBusError>)> {
        let mut ids: Vec<(String, Result<Uuid, AgentBusError>)> = Vec::new();
        for worker in self.worker_agents.clone() {
            let r = self
                .dispatch_subtask(task, &worker, payload.clone(), dispatch_ttl)
                .await;
            ids.push((worker, r));
        }

        let expected: Vec<Uuid> = ids
            .iter()
            .filter_map(|(_, r)| r.as_ref().ok().copied())
            .collect();
        let replies = self.collect_replies(&expected, collect_timeout).await;

        ids.into_iter()
            .map(|(worker, result)| {
                let mapped = match result {
                    Ok(id) => replies
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| AgentBusError::AgentNotFound(worker.clone())),
                    Err(e) => Err(e),
                };
                (worker, mapped)
            })
            .collect()
    }

    /// Ask every live worker to shut down (`task = "shutdown"`) and
    /// await their join handles (best-effort, with timeout).
    pub async fn shutdown_workers(&mut self, timeout: Duration) {
        for worker in &self.worker_agents {
            let msg = InterAgentMessage::new(
                &self.coordinator_id,
                worker,
                "shutdown",
                serde_json::Value::Null,
                Duration::from_secs(30),
            );
            let _ = self.bus.send(msg).await;
        }
        let handles = std::mem::take(&mut self.worker_handles);
        let join_all = async {
            for h in handles {
                let _ = h.await;
            }
        };
        let _ = tokio::time::timeout(timeout, join_all).await;
    }

    /// Number of spawned worker tasks still tracked.
    pub fn live_worker_count(&self) -> usize {
        self.worker_handles
            .iter()
            .filter(|h| !h.is_finished())
            .count()
    }
}

/// Run the built-in swarm demo workflow (WEFT-185).
///
/// Spawns `worker_count` echo workers under a coordinator, dispatches
/// a `"map"` subtask to each, collects replies, and shuts down.
///
/// Returns JSON summarizing the run (worker ids + reply payloads).
pub async fn run_swarm_demo(worker_count: usize) -> serde_json::Value {
    let workers: Vec<String> = (0..worker_count).map(|i| format!("worker-{i}")).collect();
    let (mut coord, _bus) = SwarmCoordinator::with_capacity("demo-coord", workers, 64);
    coord.spawn_workers().await;

    let results = coord
        .dispatch_and_collect(
            "map",
            serde_json::json!({"demo": true, "item": "hello-swarm"}),
            Duration::from_secs(30),
            Duration::from_secs(5),
        )
        .await;

    let replies: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(worker, result)| match result {
            Ok(msg) => serde_json::json!({
                "worker": worker,
                "ok": true,
                "payload": msg.payload,
            }),
            Err(e) => serde_json::json!({
                "worker": worker,
                "ok": false,
                "error": e.to_string(),
            }),
        })
        .collect();

    coord.shutdown_workers(Duration::from_secs(2)).await;

    serde_json::json!({
        "coordinator": "demo-coord",
        "worker_count": worker_count,
        "replies": replies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn topology_flat_is_runtime_default() {
        assert!(SwarmTopology::Flat.is_runtime_implemented());
        assert!(!SwarmTopology::Mesh.is_runtime_implemented());
        assert!(!SwarmTopology::Hierarchical.is_runtime_implemented());
        assert!(!SwarmTopology::Adaptive.is_runtime_implemented());
        assert_eq!(SwarmTopology::default(), SwarmTopology::Flat);
        assert_eq!(SwarmTopology::Hierarchical.as_str(), "hierarchical");
    }

    #[test]
    fn coordinator_defaults_to_flat_topology() {
        let bus = Arc::new(AgentBus::new());
        let coord = SwarmCoordinator::new(bus, "c", vec!["w".into()]);
        assert_eq!(coord.topology(), SwarmTopology::Flat);
        let tagged = coord.with_topology(SwarmTopology::Mesh);
        assert_eq!(tagged.topology(), SwarmTopology::Mesh);
        // Label only — workers list unchanged.
        assert_eq!(tagged.workers(), &["w".to_string()]);
    }

    #[tokio::test]
    async fn coordinator_dispatch() {
        let bus = Arc::new(AgentBus::new());
        let _inbox = bus.register_agent("worker-1").await;

        let coord = SwarmCoordinator::new(bus.clone(), "coordinator", vec!["worker-1".into()]);

        let msg_id = coord
            .dispatch_subtask(
                "subtask-1",
                "worker-1",
                json!({"data": 42}),
                Duration::from_secs(60),
            )
            .await
            .unwrap();

        assert!(!msg_id.is_nil());
    }

    #[tokio::test]
    async fn coordinator_broadcast() {
        let bus = Arc::new(AgentBus::new());
        let _inbox1 = bus.register_agent("w1").await;
        let _inbox2 = bus.register_agent("w2").await;

        let coord = SwarmCoordinator::new(bus.clone(), "coord", vec!["w1".into(), "w2".into()]);

        let results = coord
            .broadcast_task("broadcast-task", json!({}), Duration::from_secs(60))
            .await;

        assert_eq!(results.len(), 2);
        for (_, result) in &results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn with_capacity_constructs_bounded_bus() {
        let (coord, bus) =
            SwarmCoordinator::with_capacity("c", vec!["w".into()], 4);
        assert_eq!(bus.inbox_capacity(), 4);
        assert_eq!(coord.workers(), &["w".to_string()]);
    }

    #[tokio::test]
    async fn spawn_workers_dispatch_and_collect_e2e() {
        let (mut coord, _bus) =
            SwarmCoordinator::with_capacity("lead", vec!["a".into(), "b".into()], 32);
        coord.spawn_workers().await;
        assert_eq!(coord.live_worker_count(), 2);

        let results = coord
            .dispatch_and_collect(
                "work",
                json!({"x": 7}),
                Duration::from_secs(30),
                Duration::from_secs(3),
            )
            .await;

        assert_eq!(results.len(), 2);
        for (worker, result) in &results {
            let msg = result.as_ref().unwrap_or_else(|e| panic!("{worker}: {e}"));
            assert_eq!(msg.payload["status"], "ok");
            assert_eq!(msg.payload["echo"]["x"], 7);
        }

        coord.shutdown_workers(Duration::from_secs(2)).await;
        assert_eq!(coord.live_worker_count(), 0);
    }

    #[tokio::test]
    async fn run_swarm_demo_e2e() {
        let summary = run_swarm_demo(3).await;
        assert_eq!(summary["worker_count"], 3);
        let replies = summary["replies"].as_array().unwrap();
        assert_eq!(replies.len(), 3);
        assert!(replies.iter().all(|r| r["ok"] == true));
    }
}
