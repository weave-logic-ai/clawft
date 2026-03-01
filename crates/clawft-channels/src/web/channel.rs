//! Web channel implementation.

use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::traits::{
    Channel, ChannelFactory, ChannelHost, ChannelMetadata, ChannelStatus, MessageId,
};
use clawft_types::error::ChannelError;
use clawft_types::event::OutboundMessage;

/// Callback trait for publishing outbound messages to browser clients.
///
/// The gateway wires this to the `TopicBroadcaster` at construction time.
#[async_trait]
pub trait WebPublisher: Send + Sync {
    /// Publish a JSON message to a named topic.
    async fn publish(&self, topic: &str, message: serde_json::Value);
}

/// A channel that delivers outbound messages to browser clients via
/// a [`WebPublisher`] (backed by the WebSocket/SSE broadcaster).
///
/// Inbound messages arrive via the REST API's `/api/sessions/{key}/messages`
/// endpoint, which publishes directly to the message bus. The web channel's
/// `start()` method is therefore a no-op that waits for cancellation.
pub struct WebChannel {
    publisher: Arc<dyn WebPublisher>,
}

impl WebChannel {
    /// Create a new web channel with the given publisher.
    pub fn new(publisher: Arc<dyn WebPublisher>) -> Self {
        Self { publisher }
    }
}

#[async_trait]
impl Channel for WebChannel {
    fn name(&self) -> &str {
        "web"
    }

    fn metadata(&self) -> ChannelMetadata {
        ChannelMetadata {
            name: "web".into(),
            display_name: "Web Dashboard".into(),
            supports_threads: true,
            supports_media: false,
        }
    }

    fn status(&self) -> ChannelStatus {
        // The web channel is always "running" when registered.
        ChannelStatus::Running
    }

    fn is_allowed(&self, _sender_id: &str) -> bool {
        // Auth is handled by the API middleware, not here.
        true
    }

    async fn start(
        &self,
        _host: Arc<dyn ChannelHost>,
        cancel: CancellationToken,
    ) -> Result<(), ChannelError> {
        // No polling loop needed — inbound arrives via REST API.
        // Just wait for shutdown.
        cancel.cancelled().await;
        Ok(())
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<MessageId, ChannelError> {
        let topic = format!("sessions:{}", msg.chat_id);
        let payload = serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": &msg.content,
            "session_key": &msg.chat_id,
            "channel": "web",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        debug!(
            topic = %topic,
            chat_id = %msg.chat_id,
            "web channel publishing to broadcaster"
        );

        self.publisher.publish(&topic, payload).await;

        // Also publish to the general sessions topic.
        self.publisher
            .publish(
                "sessions",
                serde_json::json!({
                    "type": "message_added",
                    "session_key": &msg.chat_id,
                }),
            )
            .await;

        let msg_id = format!("web-{}", uuid::Uuid::new_v4());
        Ok(MessageId(msg_id))
    }
}

/// Factory that creates [`WebChannel`] instances.
///
/// Since the web channel requires a pre-built `WebPublisher` (not something
/// derivable from JSON config), the factory holds the publisher and passes
/// it into each built channel.
pub struct WebChannelFactory {
    publisher: Arc<dyn WebPublisher>,
}

impl WebChannelFactory {
    /// Create a new factory with the given publisher.
    pub fn new(publisher: Arc<dyn WebPublisher>) -> Self {
        Self { publisher }
    }
}

impl ChannelFactory for WebChannelFactory {
    fn channel_name(&self) -> &str {
        "web"
    }

    fn build(&self, _config: &serde_json::Value) -> Result<Arc<dyn Channel>, ChannelError> {
        Ok(Arc::new(WebChannel::new(self.publisher.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockPublisher {
        messages: Mutex<Vec<(String, serde_json::Value)>>,
    }

    impl MockPublisher {
        fn new() -> Self {
            Self {
                messages: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl WebPublisher for MockPublisher {
        async fn publish(&self, topic: &str, message: serde_json::Value) {
            self.messages
                .lock()
                .unwrap()
                .push((topic.to_string(), message));
        }
    }

    #[test]
    fn web_channel_name() {
        let pub_ = Arc::new(MockPublisher::new());
        let ch = WebChannel::new(pub_);
        assert_eq!(ch.name(), "web");
    }

    #[test]
    fn web_channel_metadata() {
        let pub_ = Arc::new(MockPublisher::new());
        let ch = WebChannel::new(pub_);
        let meta = ch.metadata();
        assert_eq!(meta.name, "web");
        assert_eq!(meta.display_name, "Web Dashboard");
        assert!(meta.supports_threads);
    }

    #[test]
    fn web_channel_always_running() {
        let pub_ = Arc::new(MockPublisher::new());
        let ch = WebChannel::new(pub_);
        assert_eq!(ch.status(), ChannelStatus::Running);
    }

    #[test]
    fn web_channel_allows_all() {
        let pub_ = Arc::new(MockPublisher::new());
        let ch = WebChannel::new(pub_);
        assert!(ch.is_allowed("anyone"));
    }

    #[tokio::test]
    async fn web_channel_send_publishes() {
        let pub_ = Arc::new(MockPublisher::new());
        let ch = WebChannel::new(pub_.clone());

        let msg = OutboundMessage {
            channel: "web".into(),
            chat_id: "voice".into(),
            content: "Hello!".into(),
            reply_to: None,
            media: vec![],
            metadata: HashMap::new(),
        };

        let result = ch.send(&msg).await;
        assert!(result.is_ok());

        let messages = pub_.messages.lock().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].0, "sessions:voice");
        assert_eq!(messages[0].1["role"], "assistant");
        assert_eq!(messages[0].1["content"], "Hello!");
        assert_eq!(messages[1].0, "sessions");
        assert_eq!(messages[1].1["type"], "message_added");
    }

    #[test]
    fn factory_builds_web_channel() {
        let pub_ = Arc::new(MockPublisher::new());
        let factory = WebChannelFactory::new(pub_);
        assert_eq!(factory.channel_name(), "web");

        let ch = factory.build(&serde_json::json!({})).unwrap();
        assert_eq!(ch.name(), "web");
    }
}
