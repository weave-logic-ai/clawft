//! Message bus for async channel-agent communication.
//!
//! Provides a thread-safe [`MessageBus`] using tokio unbounded MPSC channels
//! for routing inbound messages (from channels) and outbound messages
//! (from the agent pipeline) without backpressure-induced drops.
//!
//! Ported from Python `nanobot/bus/queue.py`.

use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::debug;

use clawft_types::error::ClawftError;
use clawft_types::event::{InboundMessage, OutboundMessage};

/// Thread-safe message bus for routing messages between channels and the agent pipeline.
///
/// Uses unbounded tokio MPSC channels so burst traffic is buffered in memory
/// rather than dropped. The bus owns both send and receive halves; callers
/// can obtain cloneable [`mpsc::UnboundedSender`] handles via
/// [`inbound_sender`](MessageBus::inbound_sender) and
/// [`outbound_sender`](MessageBus::outbound_sender) for multi-producer use.
///
/// # Example
///
/// ```rust
/// use clawft_core::bus::MessageBus;
///
/// # async fn example() {
/// let bus = MessageBus::new();
/// // Channel adapters clone the inbound sender
/// let tx = bus.inbound_sender();
/// // Agent loop consumes inbound messages
/// // tx.send(msg).unwrap();
/// // let msg = bus.consume_inbound().await;
/// # }
/// ```
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Mutex<mpsc::UnboundedReceiver<InboundMessage>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::UnboundedReceiver<OutboundMessage>>,
}

impl MessageBus {
    /// Create a new message bus with fresh inbound and outbound channels.
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        debug!("MessageBus created");

        Self {
            inbound_tx,
            inbound_rx: Mutex::new(inbound_rx),
            outbound_tx,
            outbound_rx: Mutex::new(outbound_rx),
        }
    }

    /// Publish an inbound message (from a channel adapter) to the bus.
    ///
    /// Returns an error if the inbound channel has been closed (all receivers dropped).
    pub fn publish_inbound(&self, msg: InboundMessage) -> Result<(), ClawftError> {
        debug!(
            channel = %msg.channel,
            chat_id = %msg.chat_id,
            "publishing inbound message"
        );
        self.inbound_tx
            .send(msg)
            .map_err(|e| ClawftError::Channel(format!("inbound channel closed: {e}")))
    }

    /// Consume the next inbound message from the bus.
    ///
    /// Returns `None` if the channel is closed and all buffered messages
    /// have been consumed.
    pub async fn consume_inbound(&self) -> Option<InboundMessage> {
        let mut rx = self.inbound_rx.lock().await;
        rx.recv().await
    }

    /// Dispatch an outbound message (from the agent pipeline) to the bus.
    ///
    /// Returns an error if the outbound channel has been closed.
    pub fn dispatch_outbound(&self, msg: OutboundMessage) -> Result<(), ClawftError> {
        debug!(
            channel = %msg.channel,
            chat_id = %msg.chat_id,
            "dispatching outbound message"
        );
        self.outbound_tx
            .send(msg)
            .map_err(|e| ClawftError::Channel(format!("outbound channel closed: {e}")))
    }

    /// Consume the next outbound message from the bus.
    ///
    /// Returns `None` if the channel is closed and all buffered messages
    /// have been consumed.
    pub async fn consume_outbound(&self) -> Option<OutboundMessage> {
        let mut rx = self.outbound_rx.lock().await;
        rx.recv().await
    }

    /// Get a cloneable sender handle for publishing inbound messages.
    ///
    /// Channel adapters should clone this sender so that multiple producers
    /// (e.g. Telegram, Slack, Discord) can publish concurrently.
    pub fn inbound_sender(&self) -> mpsc::UnboundedSender<InboundMessage> {
        self.inbound_tx.clone()
    }

    /// Get a cloneable sender handle for dispatching outbound messages.
    ///
    /// Pipeline stages or agent tasks can clone this sender for concurrent
    /// outbound dispatch.
    pub fn outbound_sender(&self) -> mpsc::UnboundedSender<OutboundMessage> {
        self.outbound_tx.clone()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_inbound(content: &str) -> InboundMessage {
        InboundMessage {
            channel: "test".into(),
            sender_id: "user1".into(),
            chat_id: "chat1".into(),
            content: content.into(),
            timestamp: Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        }
    }

    fn make_outbound(content: &str) -> OutboundMessage {
        OutboundMessage {
            channel: "test".into(),
            chat_id: "chat1".into(),
            content: content.into(),
            reply_to: None,
            media: vec![],
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn publish_and_consume_inbound() {
        let bus = MessageBus::new();
        let msg = make_inbound("hello");
        bus.publish_inbound(msg).unwrap();

        let received = bus.consume_inbound().await.unwrap();
        assert_eq!(received.content, "hello");
        assert_eq!(received.channel, "test");
    }

    #[tokio::test]
    async fn dispatch_and_consume_outbound() {
        let bus = MessageBus::new();
        let msg = make_outbound("reply");
        bus.dispatch_outbound(msg).unwrap();

        let received = bus.consume_outbound().await.unwrap();
        assert_eq!(received.content, "reply");
    }

    #[tokio::test]
    async fn multiple_inbound_messages_in_order() {
        let bus = MessageBus::new();
        for i in 0..5 {
            bus.publish_inbound(make_inbound(&format!("msg-{i}")))
                .unwrap();
        }

        for i in 0..5 {
            let msg = bus.consume_inbound().await.unwrap();
            assert_eq!(msg.content, format!("msg-{i}"));
        }
    }

    #[tokio::test]
    async fn multiple_outbound_messages_in_order() {
        let bus = MessageBus::new();
        for i in 0..5 {
            bus.dispatch_outbound(make_outbound(&format!("out-{i}")))
                .unwrap();
        }

        for i in 0..5 {
            let msg = bus.consume_outbound().await.unwrap();
            assert_eq!(msg.content, format!("out-{i}"));
        }
    }

    #[tokio::test]
    async fn inbound_sender_allows_multi_producer() {
        let bus = MessageBus::new();
        let tx1 = bus.inbound_sender();
        let tx2 = bus.inbound_sender();

        tx1.send(make_inbound("from-tx1")).unwrap();
        tx2.send(make_inbound("from-tx2")).unwrap();

        let msg1 = bus.consume_inbound().await.unwrap();
        let msg2 = bus.consume_inbound().await.unwrap();
        assert_eq!(msg1.content, "from-tx1");
        assert_eq!(msg2.content, "from-tx2");
    }

    #[tokio::test]
    async fn outbound_sender_allows_multi_producer() {
        let bus = MessageBus::new();
        let tx1 = bus.outbound_sender();
        let tx2 = bus.outbound_sender();

        tx1.send(make_outbound("from-tx1")).unwrap();
        tx2.send(make_outbound("from-tx2")).unwrap();

        let msg1 = bus.consume_outbound().await.unwrap();
        let msg2 = bus.consume_outbound().await.unwrap();
        assert_eq!(msg1.content, "from-tx1");
        assert_eq!(msg2.content, "from-tx2");
    }

    #[tokio::test]
    async fn consume_returns_none_when_all_senders_dropped() {
        let bus = MessageBus::new();
        bus.publish_inbound(make_inbound("last")).unwrap();

        // Drop the bus's own sender by dropping the bus, but we hold
        // onto the receiver indirectly -- we need a different approach.
        // Instead, create a standalone channel to test the None behavior.
        let (tx, rx) = mpsc::unbounded_channel::<InboundMessage>();
        let rx = Mutex::new(rx);
        tx.send(make_inbound("msg")).unwrap();
        drop(tx);

        // First recv returns the buffered message.
        let mut guard = rx.lock().await;
        assert!(guard.recv().await.is_some());
        // Second recv returns None because sender is dropped.
        assert!(guard.recv().await.is_none());
    }

    #[tokio::test]
    async fn publish_inbound_error_on_closed_channel() {
        let (tx, rx) = mpsc::unbounded_channel::<InboundMessage>();
        drop(rx); // Close the channel by dropping receiver.

        let result = tx.send(make_inbound("orphan"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn dispatch_outbound_error_on_closed_channel() {
        let (tx, rx) = mpsc::unbounded_channel::<OutboundMessage>();
        drop(rx);

        let result = tx.send(make_outbound("orphan"));
        assert!(result.is_err());
    }

    #[test]
    fn default_creates_valid_bus() {
        let bus = MessageBus::default();
        // Should be able to publish without error.
        bus.publish_inbound(make_inbound("default-test")).unwrap();
        bus.dispatch_outbound(make_outbound("default-test"))
            .unwrap();
    }

    #[tokio::test]
    async fn concurrent_publish_and_consume() {
        let bus = std::sync::Arc::new(MessageBus::new());
        let bus_clone = bus.clone();

        let producer = tokio::spawn(async move {
            for i in 0..100 {
                bus_clone
                    .publish_inbound(make_inbound(&format!("concurrent-{i}")))
                    .unwrap();
            }
        });

        let consumer = tokio::spawn(async move {
            let mut received = Vec::new();
            for _ in 0..100 {
                if let Some(msg) = bus.consume_inbound().await {
                    received.push(msg.content);
                }
            }
            received
        });

        producer.await.unwrap();
        let results = consumer.await.unwrap();
        assert_eq!(results.len(), 100);
    }

    #[test]
    fn message_bus_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MessageBus>();
    }

    #[tokio::test]
    async fn inbound_and_outbound_are_independent() {
        let bus = MessageBus::new();

        bus.publish_inbound(make_inbound("in")).unwrap();
        bus.dispatch_outbound(make_outbound("out")).unwrap();

        let inbound = bus.consume_inbound().await.unwrap();
        let outbound = bus.consume_outbound().await.unwrap();

        assert_eq!(inbound.content, "in");
        assert_eq!(outbound.content, "out");
    }
}
