//! Tests for the Slack channel plugin.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use clawft_plugin::error::PluginError;
use clawft_plugin::message::MessagePayload;
use clawft_plugin::traits::{ChannelAdapter, ChannelAdapterHost};
use clawft_types::config::{SlackConfig, SlackDMConfig};

use crate::traits::{Channel, ChannelMetadata, ChannelStatus};

use super::channel::SlackChannel;
use super::events::{SlackEnvelope, SlackEvent, SlackEventPayload};

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

fn make_config() -> SlackConfig {
    SlackConfig {
        enabled: true,
        mode: "socket".into(),
        webhook_path: "/slack/events".into(),
        bot_token: "xoxb-test-token".into(),
        bot_token_env: None,
        app_token: "xapp-test-token".into(),
        app_token_env: None,
        user_token_read_only: true,
        group_policy: "mention".into(),
        group_allow_from: vec![],
        dm: SlackDMConfig {
            enabled: true,
            policy: "open".into(),
            allow_from: vec![],
        },
    }
}

fn make_envelope(event: SlackEvent) -> SlackEnvelope {
    SlackEnvelope {
        envelope_type: "events_api".into(),
        envelope_id: "test-env-id".into(),
        accepts_response_payload: false,
        payload: Some(SlackEventPayload {
            token: Some("tok".into()),
            team_id: Some("T123".into()),
            event: Some(event),
            payload_type: Some("event_callback".into()),
        }),
    }
}

fn make_message_event(
    user: &str,
    channel: &str,
    text: &str,
    channel_type: Option<&str>,
) -> SlackEvent {
    SlackEvent {
        event_type: "message".into(),
        channel: Some(channel.into()),
        user: Some(user.into()),
        text: Some(text.into()),
        ts: Some("1700000000.000100".into()),
        thread_ts: None,
        bot_id: None,
        channel_type: channel_type.map(String::from),
    }
}

// ── name ─────────────────────────────────────────────────────────────────

#[test]
fn name_is_slack() {
    let ch = SlackChannel::new(make_config());
    assert_eq!(Channel::name(&ch), "slack");
}

// ── metadata ─────────────────────────────────────────────────────────────

#[test]
fn metadata_values() {
    let ch = SlackChannel::new(make_config());
    let meta: ChannelMetadata = Channel::metadata(&ch);
    assert_eq!(meta.name, "slack");
    assert_eq!(meta.display_name, "Slack");
    assert!(meta.supports_threads);
    assert!(meta.supports_media);
}

// ── status ───────────────────────────────────────────────────────────────

#[test]
fn initial_status_is_stopped() {
    let ch = SlackChannel::new(make_config());
    assert_eq!(ch.status(), ChannelStatus::Stopped);
}

// ── is_allowed / check_allowed ──────────────────────────────────────────

#[test]
fn dm_open_policy_allows_everyone() {
    let ch = SlackChannel::new(make_config());
    assert!(ch.check_allowed("U123", Some("im")));
    assert!(ch.check_allowed("U999", Some("im")));
}

#[test]
fn dm_disabled_rejects_all() {
    let mut config = make_config();
    config.dm.enabled = false;
    let ch = SlackChannel::new(config);
    assert!(!ch.check_allowed("U123", Some("im")));
}

#[test]
fn dm_allowlist_filters() {
    let mut config = make_config();
    config.dm.policy = "allowlist".into();
    config.dm.allow_from = vec!["U100".into(), "U200".into()];
    let ch = SlackChannel::new(config);
    assert!(ch.check_allowed("U100", Some("im")));
    assert!(ch.check_allowed("U200", Some("im")));
    assert!(!ch.check_allowed("U999", Some("im")));
}

#[test]
fn dm_allowlist_empty_allows_all() {
    let mut config = make_config();
    config.dm.policy = "allowlist".into();
    config.dm.allow_from = vec![];
    let ch = SlackChannel::new(config);
    assert!(ch.check_allowed("U999", Some("im")));
}

#[test]
fn group_mention_policy_allows_all() {
    let config = make_config(); // group_policy == "mention"
    let ch = SlackChannel::new(config);
    assert!(ch.check_allowed("U123", None));
    assert!(ch.check_allowed("U999", None));
}

#[test]
fn group_open_policy_allows_all() {
    let mut config = make_config();
    config.group_policy = "open".into();
    let ch = SlackChannel::new(config);
    assert!(ch.check_allowed("U123", None));
}

#[test]
fn group_allowlist_policy_filters() {
    let mut config = make_config();
    config.group_policy = "allowlist".into();
    config.group_allow_from = vec!["U100".into()];
    let ch = SlackChannel::new(config);
    assert!(ch.check_allowed("U100", None));
    assert!(!ch.check_allowed("U999", None));
}

#[test]
fn group_allowlist_empty_allows_all() {
    let mut config = make_config();
    config.group_policy = "allowlist".into();
    config.group_allow_from = vec![];
    let ch = SlackChannel::new(config);
    assert!(ch.check_allowed("U999", None));
}

#[test]
fn is_allowed_delegates_to_check_allowed() {
    let ch = SlackChannel::new(make_config());
    assert!(ch.is_allowed("anyone"));
}

// ── process_envelope ────────────────────────────────────────────────────

#[tokio::test]
async fn process_envelope_delivers_message() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = make_message_event("U99999", "C01234", "hello bot", None);
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].channel, "slack");
    assert_eq!(msgs[0].sender_id, "U99999");
    assert_eq!(msgs[0].chat_id, "C01234");
    assert_eq!(msgs[0].content, "hello bot");
    assert_eq!(
        msgs[0].metadata.get("event_type"),
        Some(&serde_json::Value::String("message".into()))
    );
}

#[tokio::test]
async fn process_envelope_delivers_app_mention() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = SlackEvent {
        event_type: "app_mention".into(),
        channel: Some("C56789".into()),
        user: Some("U11111".into()),
        text: Some("<@U00BOT> help".into()),
        ts: Some("1700000001.000200".into()),
        thread_ts: None,
        bot_id: None,
        channel_type: None,
    };
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "<@U00BOT> help");
    assert_eq!(
        msgs[0].metadata.get("event_type"),
        Some(&serde_json::Value::String("app_mention".into()))
    );
}

#[tokio::test]
async fn process_envelope_skips_bot_messages() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = SlackEvent {
        event_type: "message".into(),
        channel: Some("C01234".into()),
        user: None,
        text: Some("bot message".into()),
        ts: Some("1700000002.000300".into()),
        thread_ts: None,
        bot_id: Some("B12345".into()),
        channel_type: None,
    };
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_envelope_skips_non_events_api() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let envelope = SlackEnvelope {
        envelope_type: "interactive".into(),
        envelope_id: "env-1".into(),
        accepts_response_payload: false,
        payload: None,
    };

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_envelope_skips_no_text() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = SlackEvent {
        event_type: "message".into(),
        channel: Some("C01234".into()),
        user: Some("U99999".into()),
        text: None,
        ts: Some("1700000003.000400".into()),
        thread_ts: None,
        bot_id: None,
        channel_type: None,
    };
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_envelope_skips_unknown_event_type() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = SlackEvent {
        event_type: "reaction_added".into(),
        channel: Some("C01234".into()),
        user: Some("U99999".into()),
        text: None,
        ts: Some("1700000004.000500".into()),
        thread_ts: None,
        bot_id: None,
        channel_type: None,
    };
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_envelope_rejects_disallowed_dm() {
    let mut config = make_config();
    config.dm.policy = "allowlist".into();
    config.dm.allow_from = vec!["U100".into()];
    let ch = SlackChannel::new(config);
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = make_message_event("U999", "D01234", "sneaky dm", Some("im"));
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_envelope_includes_thread_ts_in_metadata() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = SlackEvent {
        event_type: "message".into(),
        channel: Some("C01234".into()),
        user: Some("U99999".into()),
        text: Some("threaded reply".into()),
        ts: Some("1700000005.000600".into()),
        thread_ts: Some("1700000000.000100".into()),
        bot_id: None,
        channel_type: None,
    };
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].metadata.get("thread_ts"),
        Some(&serde_json::Value::String("1700000000.000100".into()))
    );
}

#[tokio::test]
async fn process_envelope_no_payload() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let envelope = SlackEnvelope {
        envelope_type: "events_api".into(),
        envelope_id: "env-2".into(),
        accepts_response_payload: false,
        payload: None,
    };

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

#[tokio::test]
async fn process_envelope_no_event_in_payload() {
    let ch = SlackChannel::new(make_config());
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let envelope = SlackEnvelope {
        envelope_type: "events_api".into(),
        envelope_id: "env-3".into(),
        accepts_response_payload: false,
        payload: Some(SlackEventPayload {
            token: None,
            team_id: None,
            event: None,
            payload_type: None,
        }),
    };

    ch.process_envelope(&envelope, &host).await.unwrap();
    assert!(mock_host.deliveries.lock().await.is_empty());
}

// ── allow_from_match metadata (WEFT-162) ────────────────────────────────

/// DM with allowlist policy: matched sender → metadata has
/// `allow_from_match: true`.
#[tokio::test]
async fn process_envelope_emits_allow_from_match_for_dm_match() {
    let mut config = make_config();
    config.dm.policy = "allowlist".into();
    config.dm.allow_from = vec!["U100".into(), "U200".into()];
    let ch = SlackChannel::new(config);
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = make_message_event("U100", "D01234", "hello", Some("im"));
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].metadata.get("allow_from_match"),
        Some(&serde_json::Value::Bool(true)),
        "DM allowlist match must emit allow_from_match=true"
    );
}

/// Open DM policy with empty allowlist: sender is allowed but the
/// metadata flag must NOT be set (no explicit allow_from to match).
#[tokio::test]
async fn process_envelope_no_allow_from_match_when_dm_open() {
    let ch = SlackChannel::new(make_config()); // dm.policy == "open"
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = make_message_event("U999", "D01234", "hi", Some("im"));
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert!(
        !msgs[0].metadata.contains_key("allow_from_match"),
        "open DM policy must not emit allow_from_match"
    );
}

/// Group with allowlist policy: matched sender → metadata has
/// `allow_from_match: true`.
#[tokio::test]
async fn process_envelope_emits_allow_from_match_for_group_match() {
    let mut config = make_config();
    config.group_policy = "allowlist".into();
    config.group_allow_from = vec!["U100".into()];
    let ch = SlackChannel::new(config);
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = make_message_event("U100", "C01234", "hello team", None);
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].metadata.get("allow_from_match"),
        Some(&serde_json::Value::Bool(true)),
        "group allowlist match must emit allow_from_match=true"
    );
}

/// Group with mention policy and empty allowlist: sender is allowed,
/// but no allow_from_match because there is no explicit allow list.
#[tokio::test]
async fn process_envelope_no_allow_from_match_when_group_mention_policy() {
    let ch = SlackChannel::new(make_config()); // group_policy == "mention"
    let mock_host = Arc::new(MockAdapterHost::new());
    let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

    let event = make_message_event("U999", "C01234", "hello", None);
    let envelope = make_envelope(event);

    ch.process_envelope(&envelope, &host).await.unwrap();

    let msgs = mock_host.deliveries.lock().await;
    assert_eq!(msgs.len(), 1);
    assert!(
        !msgs[0].metadata.contains_key("allow_from_match"),
        "mention policy with empty allowlist must not emit allow_from_match"
    );
}

// ── ChannelAdapter (WEFT-170) ────────────────────────────────────────────

#[test]
fn adapter_capabilities() {
    let ch = SlackChannel::new(make_config());
    assert_eq!(ChannelAdapter::name(&ch), "slack");
    assert_eq!(ChannelAdapter::display_name(&ch), "Slack");
    assert!(ChannelAdapter::supports_threads(&ch));
    assert!(ChannelAdapter::supports_media(&ch));
}

#[tokio::test]
async fn adapter_send_rejects_non_text_payload() {
    let ch = SlackChannel::new(make_config());
    let payload = MessagePayload::binary("audio/wav", vec![0u8; 2]);
    let err = ChannelAdapter::send(&ch, "C01234", &payload)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("text"));
}

#[tokio::test]
async fn adapter_cancellation_token_is_tokio_util() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    tokio::select! {
        _ = cancel.cancelled() => {}
        _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
            panic!("pre-cancelled token did not resolve");
        }
    }
}
