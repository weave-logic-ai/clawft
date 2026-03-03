//! Agent-to-agent IPC protocol.
//!
//! The [`A2ARouter`] provides direct PID-to-PID messaging with
//! capability-checked routing, per-agent inboxes, and request-response
//! patterns with timeout support. It integrates with the
//! [`TopicRouter`] for pub/sub delivery.
//!
//! # Message Flow
//!
//! ```text
//! Agent A (PID 1)       A2ARouter          Agent B (PID 7)
//!      |                    |                    |
//!      |-- send(msg) ------>|                    |
//!      |                    |-- check_scope ---->|
//!      |                    |<-- Ok -------------|
//!      |                    |-- inbox.send(7) -->|
//!      |                    |                    |-- recv msg
//! ```

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::capability::CapabilityChecker;
use crate::error::{KernelError, KernelResult};
use crate::ipc::{KernelMessage, MessageTarget};
use crate::process::{Pid, ProcessState, ProcessTable};
use crate::topic::TopicRouter;

#[cfg(feature = "exochain")]
use crate::chain::ChainManager;

/// Default inbox channel capacity per agent.
const DEFAULT_INBOX_CAPACITY: usize = 1024;

/// Agent-to-agent message router.
///
/// Manages per-agent inboxes (bounded `mpsc` channels), validates
/// IPC scope through the capability checker, and routes messages
/// to their targets (direct PID, topic, broadcast, service).
pub struct A2ARouter {
    /// Process table for state validation.
    process_table: Arc<ProcessTable>,

    /// Capability checker for IPC scope enforcement.
    capability_checker: Arc<CapabilityChecker>,

    /// Topic router for pub/sub delivery.
    topic_router: Arc<TopicRouter>,

    /// Per-agent inboxes: PID -> sender half of inbox channel.
    inboxes: DashMap<Pid, mpsc::Sender<KernelMessage>>,
}

impl A2ARouter {
    /// Create a new A2A router.
    pub fn new(
        process_table: Arc<ProcessTable>,
        capability_checker: Arc<CapabilityChecker>,
        topic_router: Arc<TopicRouter>,
    ) -> Self {
        Self {
            process_table,
            capability_checker,
            topic_router,
            inboxes: DashMap::new(),
        }
    }

    /// Create an inbox for a process.
    ///
    /// Returns the receiver half that the agent should poll for
    /// incoming messages. The sender half is stored internally
    /// for routing.
    ///
    /// If an inbox already exists for this PID, the old one is
    /// replaced (existing messages are lost).
    pub fn create_inbox(&self, pid: Pid) -> mpsc::Receiver<KernelMessage> {
        let (tx, rx) = mpsc::channel(DEFAULT_INBOX_CAPACITY);
        self.inboxes.insert(pid, tx);
        debug!(pid, "created inbox");
        rx
    }

    /// Remove an inbox (used during process cleanup).
    pub fn remove_inbox(&self, pid: Pid) {
        self.inboxes.remove(&pid);
        debug!(pid, "removed inbox");
    }

    /// Send a message, routing it to the appropriate target.
    ///
    /// Validates that the sender exists and is running, checks
    /// IPC scope via the capability checker, then delivers the
    /// message to the target.
    ///
    /// # Routing
    ///
    /// - `Process(pid)`: delivers directly to the target's inbox
    /// - `Topic(name)`: publishes to all topic subscribers
    /// - `Broadcast`: delivers to all inboxes except the sender
    /// - `Service(name)`: logs a warning (service routing is a
    ///   future extension)
    /// - `Kernel`: logs a warning (kernel messages are internal)
    ///
    /// # Errors
    ///
    /// Returns `KernelError::ProcessNotFound` if the sender PID
    /// is not in the process table, or `KernelError::CapabilityDenied`
    /// if the sender's IPC scope does not permit the target.
    pub async fn send(&self, msg: KernelMessage) -> KernelResult<()> {
        let from = msg.from;

        // Validate sender exists and is running
        let sender = self
            .process_table
            .get(from)
            .ok_or(KernelError::ProcessNotFound { pid: from })?;

        if !matches!(sender.state, ProcessState::Running) {
            return Err(KernelError::Ipc(format!(
                "sender PID {from} is not running (state: {})",
                sender.state
            )));
        }

        // Route based on target
        match &msg.target {
            MessageTarget::Process(target_pid) => {
                // Check IPC scope
                self.capability_checker
                    .check_ipc_target(from, *target_pid)?;

                self.deliver_to_inbox(*target_pid, msg).await
            }
            MessageTarget::Topic(topic) => {
                let subscribers = self.topic_router.live_subscribers(topic);
                let mut delivered = 0u32;
                for &sub_pid in &subscribers {
                    if sub_pid != from {
                        let msg_clone = msg.clone();
                        if self.deliver_to_inbox(sub_pid, msg_clone).await.is_ok() {
                            delivered += 1;
                        }
                    }
                }
                debug!(from, topic, delivered, "published to topic");
                Ok(())
            }
            MessageTarget::Broadcast => {
                let mut delivered = 0u32;
                let pids: Vec<Pid> = self.inboxes.iter().map(|entry| *entry.key()).collect();

                for pid in pids {
                    if pid != from {
                        // Check IPC scope for each target
                        if self.capability_checker.check_ipc_target(from, pid).is_ok() {
                            let msg_clone = msg.clone();
                            if self.deliver_to_inbox(pid, msg_clone).await.is_ok() {
                                delivered += 1;
                            }
                        }
                    }
                }
                debug!(from, delivered, "broadcast sent");
                Ok(())
            }
            MessageTarget::Service(name) => {
                debug!(from, service = %name, "service routing not yet implemented");
                Ok(())
            }
            MessageTarget::Kernel => {
                debug!(from, "kernel message routing not yet implemented");
                Ok(())
            }
        }
    }

    /// Deliver a message to a specific PID's inbox.
    ///
    /// If the inbox does not exist or is full, the message is dropped
    /// with a warning.
    async fn deliver_to_inbox(&self, pid: Pid, msg: KernelMessage) -> KernelResult<()> {
        // Clone the sender so we release the DashMap read lock before
        // any potential remove() call (which needs a write lock on the
        // same shard — holding both would deadlock).
        let tx = self
            .inboxes
            .get(&pid)
            .ok_or(KernelError::Ipc(format!("no inbox for PID {pid}")))?
            .clone();

        match tx.try_send(msg) {
            Ok(()) => {
                debug!(pid, "message delivered to inbox");
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(pid, "inbox full, message dropped");
                Err(KernelError::Ipc(format!("inbox full for PID {pid}")))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!(pid, "inbox closed, removing");
                self.inboxes.remove(&pid);
                Err(KernelError::Ipc(format!("inbox closed for PID {pid}")))
            }
        }
    }

    /// Send a message with chain-event logging.
    ///
    /// This mirrors [`KernelIpc::send_checked`] but for the A2ARouter:
    /// every routed message is logged as an `ipc.send` chain event with
    /// sender, target, payload type, and message ID — forming a
    /// tamper-evident IPC audit trail in the exochain.
    ///
    /// When the `exochain` feature is disabled this is equivalent to
    /// a plain `send()`.
    #[cfg(feature = "exochain")]
    pub async fn send_checked(
        &self,
        msg: KernelMessage,
        chain: Option<&ChainManager>,
    ) -> KernelResult<()> {
        // Log the IPC event before delivery so the chain records intent
        // even if the inbox is full or closed.
        if let Some(cm) = chain {
            cm.append(
                "ipc",
                "ipc.send",
                Some(serde_json::json!({
                    "from": msg.from,
                    "target": format!("{:?}", msg.target),
                    "payload_type": msg.payload.type_name(),
                    "msg_id": msg.id,
                })),
            );
        }
        self.send(msg).await
    }

    /// Get the topic router.
    pub fn topic_router(&self) -> &Arc<TopicRouter> {
        &self.topic_router
    }

    /// Get the number of active inboxes.
    pub fn inbox_count(&self) -> usize {
        self.inboxes.len()
    }

    /// Check whether a PID has an inbox.
    pub fn has_inbox(&self, pid: Pid) -> bool {
        self.inboxes.contains_key(&pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::AgentCapabilities;
    use crate::ipc::MessagePayload;
    use crate::process::{ProcessEntry, ResourceUsage};
    use tokio_util::sync::CancellationToken;

    fn setup_router(
        agent_count: usize,
    ) -> (A2ARouter, Vec<Pid>, Vec<mpsc::Receiver<KernelMessage>>) {
        let table = Arc::new(ProcessTable::new(64));
        let mut pids = Vec::new();

        for i in 0..agent_count {
            let entry = ProcessEntry {
                pid: 0,
                agent_id: format!("agent-{i}"),
                state: ProcessState::Running,
                capabilities: AgentCapabilities::default(),
                resource_usage: ResourceUsage::default(),
                cancel_token: CancellationToken::new(),
                parent_pid: None,
            };
            let pid = table.insert(entry).unwrap();
            pids.push(pid);
        }

        let checker = Arc::new(CapabilityChecker::new(table.clone()));
        let topic_router = Arc::new(TopicRouter::new(table.clone()));
        let router = A2ARouter::new(table, checker, topic_router);

        let mut receivers = Vec::new();
        for &pid in &pids {
            let rx = router.create_inbox(pid);
            receivers.push(rx);
        }

        (router, pids, receivers)
    }

    #[tokio::test]
    async fn direct_message_delivery() {
        let (router, pids, mut receivers) = setup_router(2);

        let msg = KernelMessage::text(pids[0], MessageTarget::Process(pids[1]), "hello");
        router.send(msg).await.unwrap();

        let received = receivers[1].try_recv().unwrap();
        assert_eq!(received.from, pids[0]);
        assert!(matches!(
            received.payload,
            MessagePayload::Text(ref t) if t == "hello"
        ));
    }

    #[tokio::test]
    async fn message_to_self_works() {
        let (router, pids, mut receivers) = setup_router(1);

        let msg = KernelMessage::text(pids[0], MessageTarget::Process(pids[0]), "self-msg");
        router.send(msg).await.unwrap();

        let received = receivers[0].try_recv().unwrap();
        assert!(matches!(
            received.payload,
            MessagePayload::Text(ref t) if t == "self-msg"
        ));
    }

    #[tokio::test]
    async fn broadcast_delivers_to_all_except_sender() {
        let (router, pids, mut receivers) = setup_router(3);

        let msg = KernelMessage::text(pids[0], MessageTarget::Broadcast, "broadcast");
        router.send(msg).await.unwrap();

        // Sender should not receive
        assert!(receivers[0].try_recv().is_err());

        // Others should receive
        let r1 = receivers[1].try_recv().unwrap();
        assert!(matches!(
            r1.payload,
            MessagePayload::Text(ref t) if t == "broadcast"
        ));
        let r2 = receivers[2].try_recv().unwrap();
        assert!(matches!(
            r2.payload,
            MessagePayload::Text(ref t) if t == "broadcast"
        ));
    }

    #[tokio::test]
    async fn topic_publish_delivers_to_subscribers() {
        let (router, pids, mut receivers) = setup_router(3);

        // Subscribe pids[1] and pids[2] to "build"
        router.topic_router().subscribe(pids[1], "build");
        router.topic_router().subscribe(pids[2], "build");

        let msg = KernelMessage::text(pids[0], MessageTarget::Topic("build".into()), "build done");
        router.send(msg).await.unwrap();

        // Sender not subscribed, should not receive
        assert!(receivers[0].try_recv().is_err());

        // Subscribers should receive
        assert!(receivers[1].try_recv().is_ok());
        assert!(receivers[2].try_recv().is_ok());
    }

    #[tokio::test]
    async fn topic_publish_excludes_sender_if_subscribed() {
        let (router, pids, mut receivers) = setup_router(2);

        // Both subscribe
        router.topic_router().subscribe(pids[0], "build");
        router.topic_router().subscribe(pids[1], "build");

        let msg = KernelMessage::text(pids[0], MessageTarget::Topic("build".into()), "done");
        router.send(msg).await.unwrap();

        // Sender should not receive their own publish
        assert!(receivers[0].try_recv().is_err());
        // Other subscriber should receive
        assert!(receivers[1].try_recv().is_ok());
    }

    #[tokio::test]
    async fn send_from_nonexistent_pid_fails() {
        let (router, _pids, _receivers) = setup_router(1);

        let msg = KernelMessage::text(999, MessageTarget::Process(1), "hello");
        let result = router.send(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_to_pid_without_inbox_fails() {
        let table = Arc::new(ProcessTable::new(64));

        // Create sender (running)
        let sender_entry = ProcessEntry {
            pid: 0,
            agent_id: "sender".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let sender_pid = table.insert(sender_entry).unwrap();

        // Create target (running) but don't create inbox
        let target_entry = ProcessEntry {
            pid: 0,
            agent_id: "target".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let target_pid = table.insert(target_entry).unwrap();

        let checker = Arc::new(CapabilityChecker::new(table.clone()));
        let topic_router = Arc::new(TopicRouter::new(table.clone()));
        let router = A2ARouter::new(table, checker, topic_router);

        // Create inbox only for sender
        let _rx = router.create_inbox(sender_pid);

        let msg = KernelMessage::text(sender_pid, MessageTarget::Process(target_pid), "hello");
        let result = router.send(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ipc_scope_restricts_messaging() {
        let table = Arc::new(ProcessTable::new(64));

        // Create sender with restricted IPC scope
        use crate::capability::IpcScope;
        let sender_entry = ProcessEntry {
            pid: 0,
            agent_id: "restricted".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities {
                ipc_scope: IpcScope::Restricted(vec![]), // No allowed PIDs
                ..Default::default()
            },
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let sender_pid = table.insert(sender_entry).unwrap();

        // Create target
        let target_entry = ProcessEntry {
            pid: 0,
            agent_id: "target".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let target_pid = table.insert(target_entry).unwrap();

        let checker = Arc::new(CapabilityChecker::new(table.clone()));
        let topic_router = Arc::new(TopicRouter::new(table.clone()));
        let router = A2ARouter::new(table, checker, topic_router);
        let _rx1 = router.create_inbox(sender_pid);
        let _rx2 = router.create_inbox(target_pid);

        let msg = KernelMessage::text(sender_pid, MessageTarget::Process(target_pid), "blocked");
        let result = router.send(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn ipc_scope_none_blocks_all() {
        let table = Arc::new(ProcessTable::new(64));

        use crate::capability::IpcScope;
        let sender_entry = ProcessEntry {
            pid: 0,
            agent_id: "no-ipc".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities {
                can_ipc: false,
                ipc_scope: IpcScope::None,
                ..Default::default()
            },
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let sender_pid = table.insert(sender_entry).unwrap();

        let target_entry = ProcessEntry {
            pid: 0,
            agent_id: "target".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let target_pid = table.insert(target_entry).unwrap();

        let checker = Arc::new(CapabilityChecker::new(table.clone()));
        let topic_router = Arc::new(TopicRouter::new(table.clone()));
        let router = A2ARouter::new(table, checker, topic_router);
        let _rx1 = router.create_inbox(sender_pid);
        let _rx2 = router.create_inbox(target_pid);

        let msg = KernelMessage::text(sender_pid, MessageTarget::Process(target_pid), "blocked");
        let result = router.send(msg).await;
        assert!(result.is_err());
    }

    #[test]
    fn create_and_remove_inbox() {
        let table = Arc::new(ProcessTable::new(64));
        let checker = Arc::new(CapabilityChecker::new(table.clone()));
        let topic_router = Arc::new(TopicRouter::new(table.clone()));
        let router = A2ARouter::new(table, checker, topic_router);

        let _rx = router.create_inbox(42);
        assert!(router.has_inbox(42));
        assert_eq!(router.inbox_count(), 1);

        router.remove_inbox(42);
        assert!(!router.has_inbox(42));
        assert_eq!(router.inbox_count(), 0);
    }

    #[tokio::test]
    async fn tool_call_message_routes() {
        let (router, pids, mut receivers) = setup_router(2);

        let msg = KernelMessage::tool_call(
            pids[0],
            MessageTarget::Process(pids[1]),
            "read_file",
            serde_json::json!({"path": "/test"}),
        );
        router.send(msg).await.unwrap();

        let received = receivers[1].try_recv().unwrap();
        assert!(matches!(
            received.payload,
            MessagePayload::ToolCall { ref name, .. } if name == "read_file"
        ));
    }

    #[tokio::test]
    async fn tool_result_message_routes() {
        let (router, pids, mut receivers) = setup_router(2);

        let msg = KernelMessage::tool_result(
            pids[1],
            MessageTarget::Process(pids[0]),
            "call-1",
            serde_json::json!({"content": "data"}),
        );
        router.send(msg).await.unwrap();

        let received = receivers[0].try_recv().unwrap();
        assert!(matches!(
            received.payload,
            MessagePayload::ToolResult { ref call_id, .. } if call_id == "call-1"
        ));
    }

    #[cfg(feature = "exochain")]
    #[tokio::test]
    async fn send_checked_logs_chain_event() {
        let (router, pids, mut receivers) = setup_router(2);

        let chain = crate::chain::ChainManager::new(0, 1000);
        let initial_seq = chain.sequence();

        let msg = KernelMessage::text(pids[0], MessageTarget::Process(pids[1]), "audited");
        router.send_checked(msg, Some(&chain)).await.unwrap();

        // Message should still be delivered
        let received = receivers[1].try_recv().unwrap();
        assert!(matches!(
            received.payload,
            MessagePayload::Text(ref t) if t == "audited"
        ));

        // Chain should have a new ipc.send event
        assert_eq!(chain.sequence(), initial_seq + 1);
        let events = chain.tail(1);
        assert_eq!(events[0].kind, "ipc.send");
        assert_eq!(events[0].source, "ipc");
        let payload = events[0].payload.as_ref().unwrap();
        assert_eq!(payload["from"], pids[0]);
        assert_eq!(payload["payload_type"], "text");
    }

    #[cfg(feature = "exochain")]
    #[tokio::test]
    async fn send_checked_without_chain_still_delivers() {
        let (router, pids, mut receivers) = setup_router(2);

        let msg = KernelMessage::text(pids[0], MessageTarget::Process(pids[1]), "no-chain");
        router.send_checked(msg, None).await.unwrap();

        let received = receivers[1].try_recv().unwrap();
        assert!(matches!(
            received.payload,
            MessagePayload::Text(ref t) if t == "no-chain"
        ));
    }

    #[tokio::test]
    async fn rvf_payload_routes() {
        let (router, pids, mut receivers) = setup_router(2);

        let msg = KernelMessage::new(
            pids[0],
            MessageTarget::Process(pids[1]),
            MessagePayload::Rvf {
                segment_type: 0x40,
                data: vec![0xCA, 0xFE],
            },
        );
        router.send(msg).await.unwrap();

        let received = receivers[1].try_recv().unwrap();
        assert!(matches!(
            received.payload,
            MessagePayload::Rvf { segment_type: 0x40, .. }
        ));
    }
}
