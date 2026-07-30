//! Worker message loops for inter-agent tasks (WEFT-185).
//!
//! Production multi-agent hosts register each worker on the
//! [`AgentBus`](super::AgentBus), then spawn a
//! [`worker_message_loop`] that drains the worker's inbox, invokes a
//! [`WorkerHandler`], and (optionally) replies to the coordinator.
//!
//! Workers can be lightweight pure handlers (demo / unit tests) or
//! bound to an [`AgentRuntime`](crate::agent::runtime::AgentRuntime)
//! so each loop inherits per-agent session / tool isolation.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use clawft_types::agent_bus::InterAgentMessage;

use super::bus::{AgentBus, AgentInbox};

/// Default TTL for worker reply messages.
pub const DEFAULT_REPLY_TTL: Duration = Duration::from_secs(60);

/// Outcome of handling one inter-agent message.
#[derive(Debug, Clone)]
pub enum WorkerOutcome {
    /// No reply (fire-and-forget or side-effect only).
    Silent,
    /// Reply to the sender with the given JSON payload.
    Reply(Value),
    /// Stop the worker loop after this message.
    Shutdown,
}

/// Processes inter-agent tasks for a single worker agent.
///
/// Implementors are invoked once per non-expired inbox message.
/// The default demo path uses [`EchoWorker`]; production hosts can
/// wrap an [`AgentRuntime`](crate::agent::runtime::AgentRuntime) and
/// route tasks into a full agent turn.
#[async_trait]
pub trait WorkerHandler: Send + Sync + 'static {
    /// Stable agent ID (must match the bus registration).
    fn agent_id(&self) -> &str;

    /// Handle one inbound message; return how (if at all) to reply.
    async fn handle(&self, msg: &InterAgentMessage) -> WorkerOutcome;
}

/// Lightweight demo/production helper: echoes the task + payload back
/// as a structured reply. Useful for smoke demos without an LLM.
pub struct EchoWorker {
    agent_id: String,
}

impl EchoWorker {
    /// Create an echo worker for `agent_id`.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
        }
    }
}

#[async_trait]
impl WorkerHandler for EchoWorker {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    async fn handle(&self, msg: &InterAgentMessage) -> WorkerOutcome {
        if msg.task == "shutdown" {
            return WorkerOutcome::Shutdown;
        }
        WorkerOutcome::Reply(serde_json::json!({
            "worker": self.agent_id,
            "task": msg.task,
            "echo": msg.payload,
            "status": "ok",
        }))
    }
}

/// Worker bound to a named agent runtime identity.
///
/// Holds the runtime's `agent_id` so multi-agent dispatch can
/// attribute work to the correct isolated session directory. The
/// handler itself is pluggable (echo by default, or a custom
/// closure-like adapter).
pub struct RuntimeBackedWorker {
    agent_id: String,
    inner: Arc<dyn WorkerHandler>,
}

impl RuntimeBackedWorker {
    /// Bind a handler to an agent runtime's id.
    ///
    /// The runtime is the isolation boundary (sessions / tools);
    /// the handler decides how to process each bus message. Callers
    /// that already built an [`AgentRuntime`](crate::agent::runtime::AgentRuntime)
    /// pass `runtime.agent_id()` here.
    pub fn from_runtime_id(
        agent_id: impl Into<String>,
        handler: Arc<dyn WorkerHandler>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            inner: handler,
        }
    }

    /// Convenience: bind an [`EchoWorker`] under the runtime's id.
    pub fn echo(agent_id: impl Into<String>) -> Self {
        let agent_id = agent_id.into();
        Self {
            inner: Arc::new(EchoWorker::new(agent_id.clone())),
            agent_id,
        }
    }
}

#[async_trait]
impl WorkerHandler for RuntimeBackedWorker {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    async fn handle(&self, msg: &InterAgentMessage) -> WorkerOutcome {
        self.inner.handle(msg).await
    }
}

/// Drain `inbox` until closed (unregister / bus drop) or a handler
/// returns [`WorkerOutcome::Shutdown`].
///
/// When the handler returns [`WorkerOutcome::Reply`], a reply
/// message is posted back through `bus` to the original sender.
pub async fn worker_message_loop(
    bus: Arc<AgentBus>,
    mut inbox: AgentInbox,
    handler: Arc<dyn WorkerHandler>,
    reply_ttl: Duration,
) {
    let agent_id = handler.agent_id().to_string();
    info!(agent_id = %agent_id, "worker message loop started");

    while let Some(msg) = inbox.recv().await {
        debug!(
            agent_id = %agent_id,
            msg_id = %msg.id,
            from = %msg.from_agent,
            task = %msg.task,
            "worker received message"
        );

        let outcome = handler.handle(&msg).await;
        match outcome {
            WorkerOutcome::Silent => {}
            WorkerOutcome::Reply(payload) => {
                let reply = InterAgentMessage::reply(&msg, payload, reply_ttl);
                if let Err(err) = bus.send(reply).await {
                    warn!(
                        agent_id = %agent_id,
                        error = %err,
                        "failed to send worker reply"
                    );
                }
            }
            WorkerOutcome::Shutdown => {
                info!(agent_id = %agent_id, "worker shutting down by handler request");
                break;
            }
        }
    }

    // Ensure the agent is no longer addressable after the loop ends.
    bus.unregister_agent(&agent_id).await;
    info!(agent_id = %agent_id, "worker message loop exited");
}

/// Register `handler.agent_id()` on the bus and spawn a background
/// [`worker_message_loop`].
///
/// Returns the tokio join handle. Dropping / aborting the handle stops
/// the loop; the bus entry is cleaned up when the loop exits.
pub async fn spawn_worker_loop(
    bus: Arc<AgentBus>,
    handler: Arc<dyn WorkerHandler>,
) -> JoinHandle<()> {
    spawn_worker_loop_with_ttl(bus, handler, DEFAULT_REPLY_TTL).await
}

/// Like [`spawn_worker_loop`] with an explicit reply TTL.
pub async fn spawn_worker_loop_with_ttl(
    bus: Arc<AgentBus>,
    handler: Arc<dyn WorkerHandler>,
    reply_ttl: Duration,
) -> JoinHandle<()> {
    let agent_id = handler.agent_id().to_string();
    let inbox = bus.register_agent(&agent_id).await;
    tokio::spawn(worker_message_loop(bus, inbox, handler, reply_ttl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn echo_worker_loop_replies() {
        let bus = Arc::new(AgentBus::new());
        let mut coord_inbox = bus.register_agent("coord").await;

        let handle = spawn_worker_loop(bus.clone(), Arc::new(EchoWorker::new("w1"))).await;

        let msg = InterAgentMessage::new(
            "coord",
            "w1",
            "ping",
            serde_json::json!({"n": 1}),
            Duration::from_secs(30),
        );
        let msg_id = msg.id;
        bus.send(msg).await.unwrap();

        let reply = tokio::time::timeout(Duration::from_secs(2), coord_inbox.recv())
            .await
            .expect("timeout waiting for reply")
            .expect("channel closed");
        assert_eq!(reply.reply_to, Some(msg_id));
        assert_eq!(reply.from_agent, "w1");
        assert_eq!(reply.payload["status"], "ok");
        assert_eq!(reply.payload["echo"]["n"], 1);

        // Ask worker to shut down so the join handle can complete.
        let shutdown = InterAgentMessage::new(
            "coord",
            "w1",
            "shutdown",
            Value::Null,
            Duration::from_secs(30),
        );
        bus.send(shutdown).await.unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("worker did not exit");
    }

    #[tokio::test]
    async fn runtime_backed_worker_uses_agent_id() {
        let w = RuntimeBackedWorker::echo("runtime-agent");
        assert_eq!(w.agent_id(), "runtime-agent");
        let msg = InterAgentMessage::new(
            "c",
            "runtime-agent",
            "t",
            Value::Null,
            Duration::from_secs(5),
        );
        match w.handle(&msg).await {
            WorkerOutcome::Reply(v) => assert_eq!(v["worker"], "runtime-agent"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
