//! Tests for the Discord channel plugin.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use clawft_plugin::error::PluginError;
use clawft_plugin::message::MessagePayload;
use clawft_plugin::traits::{ChannelAdapter, ChannelAdapterHost};
use clawft_types::config::DiscordConfig;

use crate::traits::{Channel, ChannelMetadata, ChannelStatus};

use super::channel::DiscordChannel;
use super::events::{MessageCreate, MessageReference, User};

// ── Mock adapter host ────────────────────────────────────────────────────

struct MockAdapterHost {
    deliveries: tokio::sync::Mutex<Vec<RecordedDelivery>>,
}

struct RecordedDelivery {
    channel: String,
    sender_id: String,
    chat_id: String,
    content: String,
    metadata: HashMap<String, serde_json::Value>,
}

impl MockAdapterHost {
    fn new() -> Self {
        Self {
            deliveries: tokio::sync::Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl ChannelAdapterHost for MockAdapterHost {
    async fn deliver_inbound(
        &self,
        channel: &str,
        sender_id: &str,
        chat_id: &str,
        payload: MessagePayload,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<(), PluginError> {
        let content = match payload {
            MessagePayload::Text { content } => content,
            other => format!("{other:?}"),
        };
        self.deliveries.lock().await.push(RecordedDelivery {
            channel: channel.to_owned(),
            sender_id: sender_id.to_owned(),
            chat_id: chat_id.to_owned(),
            content,
            metadata,
        });
        Ok(())
    }
}

// ── Helper ───────────────────────────────────────────────────────────────

fn make_config() -> DiscordConfig {
    DiscordConfig {
        enabled: true,
        token: "test-bot-token".into(),
        ..Default::default()
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
    assert_eq!(Channel::name(&ch), "discord");
    assert_eq!(ChannelAdapter::name(&ch), "discord");
}

// ── metadata ─────────────────────────────────────────────────────────────

#[test]
fn metadata_values() {
    let ch = DiscordChannel::new(make_config());
    let meta: ChannelMetadata = Channel::metadata(&ch);
    assert_eq!(meta.name, "discord");
    assert_eq!(meta.display_name, "Discord");
    assert!(meta.supports_threads);
    assert!(meta.supports_media);
}

#[test]
fn adapter_capabilities() {
    let ch = DiscordChannel::new(make_config());
    assert_eq!(ChannelAdapter::display_name(&ch), "Discord");
    assert!(ChannelAdapter::supports_threads(&ch));
    assert!(ChannelAdapter::supports_media(&ch));
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
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let msg = make_message_create("U99999", "testuser", "C01234", "hello bot", false);

    ch.process_message_create(&msg, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
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
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let msg = make_message_create("B12345", "botuser", "C01234", "bot msg", true);

    ch.process_message_create(&msg, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_message_create_rejects_disallowed_user() {
    let mut config = make_config();
    config.allow_from = vec!["100".into()];
    let ch = DiscordChannel::new(config);
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let msg = make_message_create("999", "sneaky", "C01234", "disallowed", false);

    ch.process_message_create(&msg, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_message_create_dm_no_guild() {
    let ch = DiscordChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

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

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].chat_id, "DM-001");
    assert!(!msgs[0].metadata.contains_key("guild_id"));
}

#[tokio::test]
async fn process_message_create_with_reply_reference() {
    let ch = DiscordChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

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

    let msgs = mock_host.deliveries.lock().await;
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

// ── ChannelAdapter send / cancel contract (WEFT-170) ─────────────────────

#[tokio::test]
async fn adapter_send_rejects_non_text_payload() {
    let ch = DiscordChannel::new(make_config());
    let payload = MessagePayload::structured(serde_json::json!({"k": 1}));
    let err = ChannelAdapter::send(&ch, "C01234", &payload)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("text"));
}

#[tokio::test]
async fn adapter_cancellation_token_is_tokio_util() {
    // On native (default), ChannelAdapter::start takes tokio_util::CancellationToken
    // — same type as the legacy Channel path (no poll bridge needed).
    let cancel = CancellationToken::new();
    cancel.cancel();
    tokio::select! {
        _ = cancel.cancelled() => {}
        _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
            panic!("pre-cancelled token did not resolve");
        }
    }
}
