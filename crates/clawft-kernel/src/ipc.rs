//! Kernel IPC subsystem.
//!
//! [`KernelIpc`] wraps the existing [`MessageBus`] from `clawft-core`,
//! adding typed [`KernelMessage`] envelopes and PID-based routing.
//! The underlying message bus channels are reused (no new channels).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use clawft_core::bus::MessageBus;

use crate::error::KernelError;
use crate::process::Pid;

/// Target for a kernel message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageTarget {
    /// Send to a specific process by PID.
    Process(Pid),
    /// Publish to a named topic (all subscribers receive).
    Topic(String),
    /// Broadcast to all processes.
    Broadcast,
    /// Send to a named service (routed via ServiceRegistry).
    Service(String),
    /// Send to a specific method on a named service (D19, K2.1).
    ///
    /// The router resolves the service via ServiceRegistry and wraps
    /// the payload with method metadata for the receiving agent.
    ServiceMethod {
        /// Service name to resolve.
        service: String,
        /// Method to invoke on the service.
        method: String,
    },
    /// Send to the kernel itself.
    Kernel,
}

/// Payload types for kernel messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Plain text message.
    Text(String),
    /// Structured JSON data.
    Json(serde_json::Value),
    /// Tool call delegation from one agent to another.
    ToolCall {
        /// Name of the tool to call.
        name: String,
        /// Tool arguments.
        args: serde_json::Value,
    },
    /// Result of a delegated tool call.
    ToolResult {
        /// Correlation ID linking to the original request.
        call_id: String,
        /// Tool execution result.
        result: serde_json::Value,
    },
    /// System control signal.
    Signal(KernelSignal),
    /// RVF-typed payload with segment type hint.
    ///
    /// Agents can exchange RVF-typed messages. The segment type tells
    /// the receiver what format the data is in (using rvf-types
    /// discriminants, e.g. 0x40 = ExochainEvent).
    Rvf {
        /// RVF segment type discriminant.
        segment_type: u8,
        /// Payload data (CBOR, JSON, or raw bytes).
        data: Vec<u8>,
    },
}

impl MessagePayload {
    /// Return the payload type name (for logging/chain events).
    pub fn type_name(&self) -> &'static str {
        match self {
            MessagePayload::Text(_) => "text",
            MessagePayload::Json(_) => "json",
            MessagePayload::ToolCall { .. } => "tool_call",
            MessagePayload::ToolResult { .. } => "tool_result",
            MessagePayload::Signal(_) => "signal",
            MessagePayload::Rvf { .. } => "rvf",
        }
    }
}

/// Kernel control signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelSignal {
    /// Request a process to shut down gracefully.
    Shutdown,
    /// Request a process to suspend.
    Suspend,
    /// Request a process to resume from suspension.
    Resume,
    /// Heartbeat / keep-alive ping.
    Ping,
    /// Response to a heartbeat ping.
    Pong,
}

/// A typed message envelope for kernel IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelMessage {
    /// Unique message identifier.
    pub id: String,
    /// Sender PID (0 = kernel).
    pub from: Pid,
    /// Target for delivery.
    pub target: MessageTarget,
    /// Message payload.
    pub payload: MessagePayload,
    /// Creation timestamp.
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID for request-response patterns.
    ///
    /// When set, this links a response message back to the original
    /// request that triggered it. Used by the A2A protocol's
    /// request-response tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl KernelMessage {
    /// Create a new kernel message.
    pub fn new(from: Pid, target: MessageTarget, payload: MessagePayload) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            target,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }

    /// Create a new kernel message with a correlation ID.
    pub fn with_correlation(
        from: Pid,
        target: MessageTarget,
        payload: MessagePayload,
        correlation_id: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            target,
            payload,
            timestamp: Utc::now(),
            correlation_id: Some(correlation_id),
        }
    }

    /// Create a text message.
    pub fn text(from: Pid, target: MessageTarget, text: impl Into<String>) -> Self {
        Self::new(from, target, MessagePayload::Text(text.into()))
    }

    /// Create a signal message.
    pub fn signal(from: Pid, target: MessageTarget, signal: KernelSignal) -> Self {
        Self::new(from, target, MessagePayload::Signal(signal))
    }

    /// Create a tool call message.
    pub fn tool_call(
        from: Pid,
        target: MessageTarget,
        name: impl Into<String>,
        args: serde_json::Value,
    ) -> Self {
        Self::new(
            from,
            target,
            MessagePayload::ToolCall {
                name: name.into(),
                args,
            },
        )
    }

    /// Create a tool result message (response to a tool call).
    pub fn tool_result(
        from: Pid,
        target: MessageTarget,
        call_id: impl Into<String>,
        result: serde_json::Value,
    ) -> Self {
        Self::new(
            from,
            target,
            MessagePayload::ToolResult {
                call_id: call_id.into(),
                result,
            },
        )
    }
}

/// Kernel IPC subsystem wrapping the core MessageBus.
///
/// Adds kernel-level message envelope (type, routing, timestamps)
/// on top of the existing broadcast channel infrastructure.
pub struct KernelIpc {
    bus: Arc<MessageBus>,
}

impl KernelIpc {
    /// Create a new KernelIpc wrapping the given MessageBus.
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self { bus }
    }

    /// Get a reference to the underlying MessageBus.
    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    /// Send a kernel message with RBAC enforcement and chain logging.
    ///
    /// 1. If the target is `Process(to_pid)`, checks IPC capability
    ///    via the `CapabilityChecker`.
    /// 2. Logs the send event to the chain (if provided).
    /// 3. Publishes via the bus.
    #[cfg(feature = "exochain")]
    pub fn send_checked(
        &self,
        msg: &KernelMessage,
        checker: &crate::capability::CapabilityChecker,
        chain: Option<&crate::chain::ChainManager>,
    ) -> Result<(), KernelError> {
        // 1. Check IPC capability
        if let MessageTarget::Process(to_pid) = &msg.target {
            checker.check_ipc_target(msg.from, *to_pid)?;
        }

        // 2. Log to chain
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

        // 3. Send via bus
        self.send(msg)
    }

    /// Send a kernel message.
    ///
    /// Currently serializes the message to JSON and publishes it
    /// as an inbound message on the bus. Future versions (K2) will
    /// implement PID-based routing and topic subscriptions.
    pub fn send(&self, msg: &KernelMessage) -> Result<(), KernelError> {
        debug!(
            id = %msg.id,
            from = msg.from,
            "sending kernel message"
        );

        let json = serde_json::to_string(msg)
            .map_err(|e| KernelError::Ipc(format!("failed to serialize message: {e}")))?;

        // For now, publish as an inbound message. The A2A routing (K2)
        // will replace this with proper PID-based delivery.
        let inbound = clawft_types::event::InboundMessage {
            channel: "kernel-ipc".to_owned(),
            sender_id: format!("pid-{}", msg.from),
            chat_id: msg.id.clone(),
            content: json,
            timestamp: msg.timestamp,
            media: vec![],
            metadata: std::collections::HashMap::new(),
        };

        self.bus
            .publish_inbound(inbound)
            .map_err(|e| KernelError::Ipc(format!("bus publish failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_message_text() {
        let msg = KernelMessage::text(0, MessageTarget::Process(1), "hello");
        assert_eq!(msg.from, 0);
        assert!(matches!(msg.target, MessageTarget::Process(1)));
        assert!(matches!(msg.payload, MessagePayload::Text(ref t) if t == "hello"));
    }

    #[test]
    fn kernel_message_signal() {
        let msg = KernelMessage::signal(0, MessageTarget::Broadcast, KernelSignal::Shutdown);
        assert!(matches!(msg.target, MessageTarget::Broadcast));
        assert!(matches!(
            msg.payload,
            MessagePayload::Signal(KernelSignal::Shutdown)
        ));
    }

    #[test]
    fn kernel_message_json_payload() {
        let payload = MessagePayload::Json(serde_json::json!({"key": "value"}));
        let msg = KernelMessage::new(1, MessageTarget::Kernel, payload);
        assert!(matches!(msg.payload, MessagePayload::Json(_)));
    }

    #[test]
    fn message_serde_roundtrip() {
        let msg = KernelMessage::text(5, MessageTarget::Service("health".into()), "check");
        let json = serde_json::to_string(&msg).unwrap();
        let restored: KernelMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, msg.id);
        assert_eq!(restored.from, 5);
    }

    #[tokio::test]
    async fn ipc_send() {
        let bus = Arc::new(MessageBus::new());
        let ipc = KernelIpc::new(bus.clone());

        let msg = KernelMessage::text(0, MessageTarget::Process(1), "test");
        ipc.send(&msg).unwrap();

        // Should be consumable from the bus
        let received = bus.consume_inbound().await.unwrap();
        assert_eq!(received.channel, "kernel-ipc");
        assert_eq!(received.sender_id, "pid-0");
    }

    #[test]
    fn ipc_bus_ref() {
        let bus = Arc::new(MessageBus::new());
        let ipc = KernelIpc::new(bus.clone());
        assert!(Arc::ptr_eq(ipc.bus(), &bus));
    }

    #[test]
    fn message_target_variants() {
        let targets = vec![
            MessageTarget::Process(1),
            MessageTarget::Broadcast,
            MessageTarget::Service("test".into()),
            MessageTarget::ServiceMethod {
                service: "auth".into(),
                method: "validate_token".into(),
            },
            MessageTarget::Kernel,
        ];
        for target in targets {
            let json = serde_json::to_string(&target).unwrap();
            let _: MessageTarget = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn kernel_signal_variants() {
        let signals = vec![
            KernelSignal::Shutdown,
            KernelSignal::Suspend,
            KernelSignal::Resume,
            KernelSignal::Ping,
            KernelSignal::Pong,
        ];
        for signal in signals {
            let json = serde_json::to_string(&signal).unwrap();
            let _: KernelSignal = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn message_with_correlation_id() {
        let msg = KernelMessage::with_correlation(
            1,
            MessageTarget::Process(2),
            MessagePayload::Text("request".into()),
            "req-123".into(),
        );
        assert_eq!(msg.correlation_id, Some("req-123".into()));
        assert_eq!(msg.from, 1);
    }

    #[test]
    fn message_without_correlation_id() {
        let msg = KernelMessage::text(1, MessageTarget::Process(2), "hello");
        assert!(msg.correlation_id.is_none());
    }

    #[test]
    fn tool_call_message() {
        let msg = KernelMessage::tool_call(
            1,
            MessageTarget::Process(2),
            "read_file",
            serde_json::json!({"path": "/src/main.rs"}),
        );
        match &msg.payload {
            MessagePayload::ToolCall { name, args } => {
                assert_eq!(name, "read_file");
                assert_eq!(args["path"], "/src/main.rs");
            }
            other => panic!("expected ToolCall, got: {other:?}"),
        }
    }

    #[test]
    fn tool_result_message() {
        let msg = KernelMessage::tool_result(
            2,
            MessageTarget::Process(1),
            "call-123",
            serde_json::json!({"content": "file contents"}),
        );
        match &msg.payload {
            MessagePayload::ToolResult { call_id, result } => {
                assert_eq!(call_id, "call-123");
                assert_eq!(result["content"], "file contents");
            }
            other => panic!("expected ToolResult, got: {other:?}"),
        }
    }

    #[test]
    fn topic_target() {
        let msg = KernelMessage::text(1, MessageTarget::Topic("build-status".into()), "done");
        assert!(matches!(msg.target, MessageTarget::Topic(ref t) if t == "build-status"));
    }

    #[test]
    fn tool_call_serde_roundtrip() {
        let msg = KernelMessage::tool_call(
            1,
            MessageTarget::Process(2),
            "search",
            serde_json::json!({"query": "test"}),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let restored: KernelMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            restored.payload,
            MessagePayload::ToolCall { ref name, .. } if name == "search"
        ));
    }

    #[test]
    fn correlation_id_serde_roundtrip() {
        let msg = KernelMessage::with_correlation(
            1,
            MessageTarget::Process(2),
            MessagePayload::Text("req".into()),
            "corr-456".into(),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let restored: KernelMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.correlation_id, Some("corr-456".into()));
    }

    #[test]
    fn rvf_payload_variant() {
        let payload = MessagePayload::Rvf {
            segment_type: 0x40,
            data: vec![0xCA, 0xFE],
        };
        let msg = KernelMessage::new(1, MessageTarget::Process(2), payload);
        assert_eq!(msg.payload.type_name(), "rvf");

        // serde roundtrip
        let json = serde_json::to_string(&msg).unwrap();
        let restored: KernelMessage = serde_json::from_str(&json).unwrap();
        match &restored.payload {
            MessagePayload::Rvf { segment_type, data } => {
                assert_eq!(*segment_type, 0x40);
                assert_eq!(data, &[0xCA, 0xFE]);
            }
            other => panic!("expected Rvf, got: {other:?}"),
        }
    }

    #[test]
    fn payload_type_names() {
        assert_eq!(MessagePayload::Text("hi".into()).type_name(), "text");
        assert_eq!(
            MessagePayload::Json(serde_json::json!(1)).type_name(),
            "json"
        );
        assert_eq!(
            MessagePayload::Signal(KernelSignal::Ping).type_name(),
            "signal"
        );
    }

    #[test]
    fn correlation_id_absent_in_json_when_none() {
        let msg = KernelMessage::text(1, MessageTarget::Process(2), "hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("correlation_id"));
    }
}
