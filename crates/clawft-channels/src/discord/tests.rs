//! Tests for the Discord channel plugin.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use clawft_types::config::DiscordConfig;
use clawft_types::error::ChannelError;
use clawft_types::event::InboundMessage;

use crate::traits::{Channel, ChannelHost, ChannelMetadata, ChannelStatus, Command};

use super::channel::DiscordChannel;
use super::events::{MessageCreate, MessageReference, User};

// ── Mock host ────────────────────────────────────────────────────────────

/// Mock host that collects delivered inbound messages.
struct MockHost {
    messages: tokio::sync::Mutex<Vec<InboundMessage>>,
}

impl MockHost {
    fn new() -> Self {
        Self {
            messages: tokio::sync::Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl ChannelHost for MockHost {
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError> {
        self.messages.lock().await.push(msg);
        Ok(())
    }

    async fn register_command(&self, _cmd: Command) -> Result<(), ChannelError> {
        Ok(())
    }

    async fn publish_inbound(
        &self,
        _channel: &str,
        _sender_id: &str,
        _chat_id: &str,
        _content: &str,
        _media: Vec<String>,
        _metadata: HashMap<String, serde_json::Value>,
    ) -> Result<(), ChannelError> {
        Ok(())
    }
}

// ── Helper ───────────────────────────────────────────────────────────────

fn make_config() -> DiscordConfig {
    DiscordConfig {
        enabled: true,
        token: "test-bot-token".into(),
        allow_from: vec![],
        gateway_url: "wss://gateway.discord.gg/?v=10&encoding=json".into(),
        intents: 37377,
    }
}

fn make_message_create(
    user_id: &str,
    username: &str,
    channel_id: &str,
    content: &str,
    is_bot: bool,
) -> MessageCreate {
    MessageCreate {
        id: "msg-123".into(),
        channel_id: channel_id.into(),
        content: content.into(),
        author: User {
            id: user_id.into(),
            username: username.into(),
            bot: is_bot,
        },
        guild_id: Some("guild-456".into()),
        message_reference: None,
    }
}

// ── name ─────────────────────────────────────────────────────────────────

#[test]
fn name_is_discord() {
    let ch = DiscordChannel::new(make_config());
    assert_eq!(ch.name(), "discord");
}

// ── metadata ─────────────────────────────────────────────────────────────

#[test]
fn metadata_values() {
    let ch = DiscordChannel::new(make_config());
    let meta: ChannelMetadata = ch.metadata();
    assert_eq!(meta.name, "discord");
    assert_eq!(meta.display_name, "Discord");
    assert!(meta.supports_threads);
    assert!(meta.supports_media);
}

// ── status ───────────────────────────────────────────────────────────────

#[test]
fn initial_status_is_stopped() {
    let ch = DiscordChannel::new(make_config());
    assert_eq!(ch.status(), ChannelStatus::Stopped);
}

// ── is_allowed ──────────────────────────────────────────────────────────

#[test]
fn is_allowed_empty_list_allows_everyone() {
    let ch = DiscordChannel::new(make_config());
    assert!(ch.is_allowed("123"));
    assert!(ch.is_allowed("anyone"));
    assert!(ch.is_allowed(""));
}

#[test]
fn is_allowed_with_list_allows_only_listed() {
    let mut config = make_config();
    config.allow_from = vec!["100".into(), "200".into()];
    let ch = DiscordChannel::new(config);
    assert!(ch.is_allowed("100"));
    assert!(ch.is_allowed("200"));
    assert!(!ch.is_allowed("300"));
    assert!(!ch.is_allowed(""));
}

// ── process_message_create ──────────────────────────────────────────────

#[tokio::test]
async fn process_message_create_delivers_message() {
    let ch = DiscordChannel::new(make_config());
    let mock_host = Arc::new(MockHost::new());
    let host: Arc<dyn ChannelHost> = mock_host.clone();

    let msg = make_message_create("U99999", "testuser", "C01234", "hello bot", false);

    ch.process_message_create(&msg, &host).await.unwrap();

    let msgs = mock_host.messages.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].channel, "discord");
    assert_eq!(msgs[0].sender_id, "U99999");
    assert_eq!(msgs[0].chat_id, "C01234");
    assert_eq!(msgs[0].content, "hello bot");
    assert_eq!(
        msgs[0].metadata.get("username"),
        Some(&serde_json::Value::String("testuser".into()))
    );
    assert_eq!(
        msgs[0].metadata.get("message_id"),
        Some(&serde_json::Value::String("msg-123".into()))
    );
    assert_eq!(
        msgs[0].metadata.get("guild_id"),
        Some(&serde_json::Value::String("guild-456".into()))
    );
}

#[tokio::test]
async fn process_message_create_skips_bot_messages() {
    let ch = DiscordChannel::new(make_config());
    let mock_host = Arc::new(MockHost::new());
    let host: Arc<dyn ChannelHost> = mock_host.clone();

    let msg = make_message_create("B12345", "botuser", "C01234", "bot msg", true);

    ch.process_message_create(&msg, &host).await.unwrap();
    assert!(mock_host.messages.lock().await.is_empty());
}

#[tokio::test]
async fn process_message_create_rejects_disallowed_user() {
    let mut config = make_config();
    config.allow_from = vec!["100".into()];
    let ch = DiscordChannel::new(config);
    let mock_host = Arc::new(MockHost::new());
    let host: Arc<dyn ChannelHost> = mock_host.clone();

    let msg = make_message_create("999", "sneaky", "C01234", "disallowed", false);

    ch.process_message_create(&msg, &host).await.unwrap();
    assert!(mock_host.messages.lock().await.is_empty());
}

#[tokio::test]
async fn process_message_create_dm_no_guild() {
    let ch = DiscordChannel::new(make_config());
    let mock_host = Arc::new(MockHost::new());
    let host: Arc<dyn ChannelHost> = mock_host.clone();

    let msg = MessageCreate {
        id: "msg-456".into(),
        channel_id: "DM-001".into(),
        content: "private msg".into(),
        author: User {
            id: "U77777".into(),
            username: "dmuser".into(),
            bot: false,
        },
        guild_id: None,
        message_reference: None,
    };

    ch.process_message_create(&msg, &host).await.unwrap();

    let msgs = mock_host.messages.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].chat_id, "DM-001");
    assert!(!msgs[0].metadata.contains_key("guild_id"));
}

#[tokio::test]
async fn process_message_create_with_reply_reference() {
    let ch = DiscordChannel::new(make_config());
    let mock_host = Arc::new(MockHost::new());
    let host: Arc<dyn ChannelHost> = mock_host.clone();

    let msg = MessageCreate {
        id: "msg-789".into(),
        channel_id: "C01234".into(),
        content: "reply text".into(),
        author: User {
            id: "U55555".into(),
            username: "replier".into(),
            bot: false,
        },
        guild_id: Some("G001".into()),
        message_reference: Some(MessageReference {
            message_id: Some("msg-100".into()),
            channel_id: Some("C01234".into()),
        }),
    };

    ch.process_message_create(&msg, &host).await.unwrap();

    let msgs = mock_host.messages.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].metadata.get("reply_to_message_id"),
        Some(&serde_json::Value::String("msg-100".into()))
    );
}

// ── status transitions ──────────────────────────────────────────────────

#[tokio::test]
async fn set_status_transitions() {
    let ch = DiscordChannel::new(make_config());
    assert_eq!(ch.status(), ChannelStatus::Stopped);

    ch.set_status(ChannelStatus::Starting).await;
    assert_eq!(ch.status(), ChannelStatus::Starting);

    ch.set_status(ChannelStatus::Running).await;
    assert_eq!(ch.status(), ChannelStatus::Running);

    ch.set_status(ChannelStatus::Error("test error".into()))
        .await;
    assert_eq!(ch.status(), ChannelStatus::Error("test error".into()));

    ch.set_status(ChannelStatus::Stopped).await;
    assert_eq!(ch.status(), ChannelStatus::Stopped);
}
