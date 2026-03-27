//! Message tool for sending messages through the message bus.
//!
//! Allows the agent to proactively send messages to specific channels
//! and chat IDs, enabling multi-channel communication.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::bus::MessageBus;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_types::event::OutboundMessage;
use serde_json::json;
use tracing::debug;

/// Tool for sending messages through the message bus.
///
/// Enables the agent to dispatch messages to specific channels and chat IDs
/// without going through the normal response path. Useful for:
/// - Sending notifications to other channels
/// - Broadcasting updates
/// - Cross-channel communication
pub struct MessageTool {
    bus: Arc<MessageBus>,
}

impl MessageTool {
    /// Create a new message tool backed by the given message bus.
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self { bus }
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl Tool for MessageTool {
    fn name(&self) -> &str {
        "message"
    }

    fn description(&self) -> &str {
        "Send a message to a specific channel and chat. Useful for cross-channel notifications."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Target channel (e.g., 'telegram', 'slack', 'discord')"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Target chat/conversation ID"
                },
                "content": {
                    "type": "string",
                    "description": "Message content to send"
                }
            },
            "required": ["channel", "chat_id", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let channel = args
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: channel".into()))?;

        let chat_id = args
            .get("chat_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: chat_id".into()))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: content".into()))?;

        debug!(
            channel = %channel,
            chat_id = %chat_id,
            content_len = content.len(),
            "sending message via tool"
        );

        let outbound = OutboundMessage {
            channel: channel.to_owned(),
            chat_id: chat_id.to_owned(),
            content: content.to_owned(),
            reply_to: None,
            media: vec![],
            metadata: Default::default(),
        };

        self.bus
            .dispatch_outbound(outbound)
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to dispatch message: {e}")))?;

        Ok(json!({
            "status": "sent",
            "channel": channel,
            "chat_id": chat_id,
            "content_length": content.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool() -> MessageTool {
        MessageTool::new(Arc::new(MessageBus::new()))
    }

    #[test]
    fn name_is_message() {
        assert_eq!(make_tool().name(), "message");
    }

    #[test]
    fn description_not_empty() {
        assert!(!make_tool().description().is_empty());
    }

    #[test]
    fn parameters_has_required_fields() {
        let params = make_tool().parameters();
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&json!("channel")));
        assert!(required.contains(&json!("chat_id")));
        assert!(required.contains(&json!("content")));
    }

    #[tokio::test]
    async fn missing_channel_returns_error() {
        let err = make_tool()
            .execute(json!({"chat_id": "123", "content": "hi"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn missing_chat_id_returns_error() {
        let err = make_tool()
            .execute(json!({"channel": "test", "content": "hi"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn missing_content_returns_error() {
        let err = make_tool()
            .execute(json!({"channel": "test", "chat_id": "123"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn successful_dispatch() {
        let bus = Arc::new(MessageBus::new());
        let tool = MessageTool::new(bus.clone());

        let result = tool
            .execute(json!({
                "channel": "telegram",
                "chat_id": "12345",
                "content": "hello from tool"
            }))
            .await
            .unwrap();

        assert_eq!(result["status"], "sent");
        assert_eq!(result["channel"], "telegram");
        assert_eq!(result["chat_id"], "12345");

        // Verify message was dispatched to the bus.
        let outbound = bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.channel, "telegram");
        assert_eq!(outbound.chat_id, "12345");
        assert_eq!(outbound.content, "hello from tool");
    }

    #[test]
    fn tool_is_object_safe() {
        fn accepts_tool(_t: &dyn Tool) {}
        accepts_tool(&make_tool());
    }
}
