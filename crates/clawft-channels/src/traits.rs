//! Channel plugin trait definitions.
//!
//! Defines the three core traits for the channel plugin system:
//!
//! - [`Channel`] -- implemented by each channel plugin (Telegram, Slack, etc.)
//! - [`ChannelHost`] -- implemented by the host, consumed by plugins for
//!   delivering inbound messages and accessing configuration
//! - [`ChannelFactory`] -- implemented by plugins, consumed by the host to
//!   instantiate channels from JSON configuration

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use clawft_types::error::ChannelError;
use clawft_types::event::{InboundMessage, OutboundMessage};

/// Metadata describing a channel plugin's capabilities.
#[derive(Debug, Clone)]
pub struct ChannelMetadata {
    /// Channel identifier (e.g., `"telegram"`, `"slack"`).
    pub name: String,
    /// Human-readable display name (e.g., `"Telegram Bot"`).
    pub display_name: String,
    /// Whether this channel supports threaded conversations.
    pub supports_threads: bool,
    /// Whether this channel supports media attachments.
    pub supports_media: bool,
}

/// A command that can be registered with the host (e.g., slash commands).
#[derive(Debug, Clone)]
pub struct Command {
    /// Command name (e.g., `"/help"`).
    pub name: String,
    /// Human-readable description of what the command does.
    pub description: String,
    /// Positional / named parameters the command accepts.
    pub parameters: Vec<CommandParameter>,
}

/// A single parameter accepted by a [`Command`].
#[derive(Debug, Clone)]
pub struct CommandParameter {
    /// Parameter name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether this parameter must be supplied.
    pub required: bool,
}

/// Status of a channel plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    /// Not yet started.
    Stopped,
    /// Currently connecting / initializing.
    Starting,
    /// Running and processing messages.
    Running,
    /// Encountered an error.
    Error(String),
    /// Shutting down.
    Stopping,
}

/// Unique identifier for a sent message, returned by [`Channel::send`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId(pub String);

/// The trait every channel plugin must implement.
///
/// A channel represents a bidirectional connection to a chat platform
/// (Telegram, Slack, Discord, etc.). The host manages the lifecycle:
///
/// 1. A [`ChannelFactory`] creates the `Channel` from config.
/// 2. The host calls [`start`](Channel::start) with an `Arc<dyn ChannelHost>`
///    and a [`CancellationToken`].
/// 3. `start` is long-lived -- it runs until the token is cancelled.
/// 4. The host calls [`send`](Channel::send) to push outbound messages.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Unique channel identifier (e.g., `"telegram"`, `"slack"`).
    fn name(&self) -> &str;

    /// Get channel metadata describing capabilities.
    fn metadata(&self) -> ChannelMetadata;

    /// Get the current lifecycle status.
    fn status(&self) -> ChannelStatus;

    /// Check if a sender is allowed to interact with this channel.
    ///
    /// Returns `true` when the allow-list is empty (everyone allowed)
    /// or when `sender_id` appears in the allow-list.
    fn is_allowed(&self, sender_id: &str) -> bool;

    /// Start receiving messages.
    ///
    /// This method is long-lived: it should run until `cancel` is triggered.
    /// Inbound messages are delivered to the pipeline via
    /// [`ChannelHost::deliver_inbound`].
    async fn start(
        &self,
        host: Arc<dyn ChannelHost>,
        cancel: CancellationToken,
    ) -> Result<(), ChannelError>;

    /// Send an outbound message through this channel.
    async fn send(&self, msg: &OutboundMessage) -> Result<MessageId, ChannelError>;
}

/// Services the host exposes to channel plugins.
///
/// Passed into [`Channel::start`] so that plugins can deliver inbound
/// messages and register commands without holding references to the
/// full application state.
#[async_trait]
pub trait ChannelHost: Send + Sync {
    /// Deliver an inbound message from a channel to the agent pipeline.
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError>;

    /// Register a command (e.g., a slash command) with the host.
    async fn register_command(&self, cmd: Command) -> Result<(), ChannelError>;

    /// Build an [`InboundMessage`] from raw parts and deliver it.
    ///
    /// This is a convenience wrapper over [`deliver_inbound`](ChannelHost::deliver_inbound)
    /// that constructs the message and performs the allow-list check.
    async fn publish_inbound(
        &self,
        channel: &str,
        sender_id: &str,
        chat_id: &str,
        content: &str,
        media: Vec<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<(), ChannelError>;
}

/// Factory for creating [`Channel`] instances from JSON configuration.
///
/// Each channel plugin provides a factory so that the host can
/// instantiate channels dynamically based on the loaded config.
pub trait ChannelFactory: Send + Sync {
    /// The channel name this factory creates (e.g., `"telegram"`).
    fn channel_name(&self) -> &str;

    /// Create a channel instance from its JSON config section.
    fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn Channel>, ChannelError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_metadata_creation() {
        let meta = ChannelMetadata {
            name: "telegram".into(),
            display_name: "Telegram Bot".into(),
            supports_threads: false,
            supports_media: true,
        };
        assert_eq!(meta.name, "telegram");
        assert_eq!(meta.display_name, "Telegram Bot");
        assert!(!meta.supports_threads);
        assert!(meta.supports_media);
    }

    #[test]
    fn channel_metadata_clone() {
        let meta = ChannelMetadata {
            name: "slack".into(),
            display_name: "Slack".into(),
            supports_threads: true,
            supports_media: true,
        };
        let cloned = meta.clone();
        assert_eq!(cloned.name, meta.name);
        assert_eq!(cloned.supports_threads, meta.supports_threads);
    }

    #[test]
    fn channel_status_equality() {
        assert_eq!(ChannelStatus::Stopped, ChannelStatus::Stopped);
        assert_eq!(ChannelStatus::Running, ChannelStatus::Running);
        assert_eq!(ChannelStatus::Starting, ChannelStatus::Starting);
        assert_eq!(ChannelStatus::Stopping, ChannelStatus::Stopping);
        assert_eq!(
            ChannelStatus::Error("timeout".into()),
            ChannelStatus::Error("timeout".into()),
        );
        assert_ne!(ChannelStatus::Stopped, ChannelStatus::Running);
        assert_ne!(
            ChannelStatus::Error("a".into()),
            ChannelStatus::Error("b".into()),
        );
    }

    #[test]
    fn message_id_equality_and_hash() {
        use std::collections::HashSet;

        let id1 = MessageId("msg-001".into());
        let id2 = MessageId("msg-001".into());
        let id3 = MessageId("msg-002".into());

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        let mut set = HashSet::new();
        set.insert(id1.clone());
        assert!(set.contains(&id2));
        assert!(!set.contains(&id3));
    }

    #[test]
    fn command_creation() {
        let cmd = Command {
            name: "/help".into(),
            description: "Show help text".into(),
            parameters: vec![
                CommandParameter {
                    name: "topic".into(),
                    description: "Help topic".into(),
                    required: false,
                },
            ],
        };
        assert_eq!(cmd.name, "/help");
        assert_eq!(cmd.parameters.len(), 1);
        assert!(!cmd.parameters[0].required);
    }

    #[test]
    fn command_no_parameters() {
        let cmd = Command {
            name: "/ping".into(),
            description: "Health check".into(),
            parameters: vec![],
        };
        assert!(cmd.parameters.is_empty());
    }
}
