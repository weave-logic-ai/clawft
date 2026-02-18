//! Heartbeat service.
//!
//! Periodically posts a prompt as an [`InboundMessage`] at a fixed interval,
//! useful for health checks or periodic agent nudges.

use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::error::{Result, ServiceError};
use clawft_types::event::InboundMessage;

/// A service that emits heartbeat messages at a regular interval.
pub struct HeartbeatService {
    interval: Duration,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
}

impl HeartbeatService {
    /// Create a new heartbeat service.
    ///
    /// `interval_minutes` sets the delay between heartbeats.
    /// `prompt` is the message content delivered each heartbeat.
    pub fn new(
        interval_minutes: u64,
        prompt: String,
        message_tx: mpsc::UnboundedSender<InboundMessage>,
    ) -> Self {
        Self {
            interval: Duration::from_secs(interval_minutes * 60),
            prompt,
            message_tx,
        }
    }

    /// Start the heartbeat loop.
    ///
    /// Posts an [`InboundMessage`] with `channel: "heartbeat"` at each tick.
    /// Exits gracefully when the cancellation token is triggered.
    pub async fn start(&self, cancel: CancellationToken) -> Result<()> {
        info!(
            interval_secs = self.interval.as_secs(),
            "heartbeat service started"
        );
        let mut interval = tokio::time::interval(self.interval);

        // The first tick fires immediately; skip it so the first heartbeat
        // happens after one full interval.
        interval.tick().await;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("heartbeat service shutting down");
                    return Ok(());
                }
                _ = interval.tick() => {
                    let msg = InboundMessage {
                        channel: "heartbeat".to_string(),
                        sender_id: "system".to_string(),
                        chat_id: "heartbeat".to_string(),
                        content: self.prompt.clone(),
                        timestamp: Utc::now(),
                        media: vec![],
                        metadata: HashMap::new(),
                    };

                    if self.message_tx.send(msg).is_err() {
                        return Err(ServiceError::ChannelClosed);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn heartbeat_sends_messages() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let svc = HeartbeatService {
            interval: Duration::from_millis(50),
            prompt: "heartbeat check".into(),
            message_tx: tx,
        };

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { svc.start(cancel_clone).await });

        // Wait for at least one heartbeat.
        tokio::time::sleep(Duration::from_millis(150)).await;
        cancel.cancel();

        let result = handle.await.unwrap();
        assert!(result.is_ok());

        // We should have received at least one message.
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.channel, "heartbeat");
        assert_eq!(msg.sender_id, "system");
        assert_eq!(msg.chat_id, "heartbeat");
        assert_eq!(msg.content, "heartbeat check");
    }

    #[tokio::test]
    async fn graceful_shutdown_on_cancel() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let svc = HeartbeatService {
            interval: Duration::from_secs(3600), // long interval
            prompt: "test".into(),
            message_tx: tx,
        };

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { svc.start(cancel_clone).await });

        // Cancel immediately.
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancel.cancel();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn channel_closed_returns_error() {
        let (tx, rx) = mpsc::unbounded_channel();
        let svc = HeartbeatService {
            interval: Duration::from_millis(10),
            prompt: "test".into(),
            message_tx: tx,
        };

        // Drop the receiver so the channel is closed.
        drop(rx);

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { svc.start(cancel_clone).await });

        let result = handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::ChannelClosed));
    }

    #[test]
    fn new_sets_interval_from_minutes() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let svc = HeartbeatService::new(5, "test".into(), tx);
        assert_eq!(svc.interval, Duration::from_secs(300));
    }
}
