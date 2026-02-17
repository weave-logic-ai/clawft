//! Slack Socket Mode and Events API types.
//!
//! These types model the Slack Socket Mode envelope format and the
//! inner event payloads used by the channel plugin.

use serde::{Deserialize, Serialize};

/// A Socket Mode envelope wrapping an event from Slack.
///
/// When Slack sends events over the Socket Mode WebSocket, each message
/// is wrapped in an envelope that includes an `envelope_id` for
/// acknowledgement.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackEnvelope {
    /// The type of envelope: `"events_api"`, `"interactive"`, `"slash_commands"`.
    #[serde(rename = "type")]
    pub envelope_type: String,

    /// Unique ID for this envelope; must be acknowledged.
    pub envelope_id: String,

    /// Whether Slack expects a response payload in the acknowledgement.
    #[serde(default)]
    pub accepts_response_payload: bool,

    /// The event payload (for `events_api` type envelopes).
    pub payload: Option<SlackEventPayload>,
}

/// The payload inside an `events_api` envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackEventPayload {
    /// Token for verification (legacy; prefer signature verification).
    pub token: Option<String>,

    /// The team/workspace ID.
    pub team_id: Option<String>,

    /// The inner event object.
    pub event: Option<SlackEvent>,

    /// The event type at the top level (e.g., `"event_callback"`).
    #[serde(rename = "type")]
    pub payload_type: Option<String>,
}

/// An inner Slack event.
///
/// Covers `message` and `app_mention` event types. Unknown event types
/// are captured with their raw `event_type` for logging.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackEvent {
    /// Event type: `"message"`, `"app_mention"`, etc.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Channel/conversation ID where the event occurred.
    pub channel: Option<String>,

    /// User ID who triggered the event.
    pub user: Option<String>,

    /// Text content of the message.
    pub text: Option<String>,

    /// Timestamp of the message (unique message ID within a channel).
    pub ts: Option<String>,

    /// Thread timestamp, if this message is in a thread.
    pub thread_ts: Option<String>,

    /// Bot ID, if the message was sent by a bot.
    pub bot_id: Option<String>,

    /// Channel type for DMs: `"im"` indicates a direct message.
    pub channel_type: Option<String>,
}

/// Acknowledgement response sent back to Slack over the WebSocket.
///
/// Every envelope must be acknowledged by sending back its `envelope_id`.
#[derive(Debug, Clone, Serialize)]
pub struct SlackAcknowledge {
    /// The envelope ID being acknowledged.
    pub envelope_id: String,

    /// Optional response payload (usually omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// A hello message sent by Slack when the WebSocket connection is established.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackHello {
    /// Always `"hello"`.
    #[serde(rename = "type")]
    pub hello_type: String,

    /// Number of active connections for this app.
    pub num_connections: Option<u32>,
}

/// Response from the `apps.connections.open` API call.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionsOpenResponse {
    /// Whether the API call succeeded.
    pub ok: bool,

    /// The WebSocket URL to connect to.
    pub url: Option<String>,

    /// Error message if `ok` is `false`.
    pub error: Option<String>,
}

/// Response from `chat.postMessage`.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatPostMessageResponse {
    /// Whether the API call succeeded.
    pub ok: bool,

    /// The posted message object.
    pub message: Option<SlackMessageResponse>,

    /// Channel where the message was posted.
    pub channel: Option<String>,

    /// Timestamp of the posted message.
    pub ts: Option<String>,

    /// Error message if `ok` is `false`.
    pub error: Option<String>,
}

/// The message object in a `chat.postMessage` response.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackMessageResponse {
    /// Text of the message.
    pub text: Option<String>,

    /// Timestamp of the message.
    pub ts: Option<String>,
}

/// Response from `chat.update`.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatUpdateResponse {
    /// Whether the API call succeeded.
    pub ok: bool,

    /// Channel where the message was updated.
    pub channel: Option<String>,

    /// Timestamp of the updated message.
    pub ts: Option<String>,

    /// Error message if `ok` is `false`.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_events_api_envelope() {
        let json = r#"{
            "type": "events_api",
            "envelope_id": "env-123",
            "accepts_response_payload": false,
            "payload": {
                "token": "verification-token",
                "team_id": "T12345",
                "type": "event_callback",
                "event": {
                    "type": "message",
                    "channel": "C01234",
                    "user": "U99999",
                    "text": "hello bot",
                    "ts": "1700000000.000100",
                    "thread_ts": null,
                    "bot_id": null
                }
            }
        }"#;
        let envelope: SlackEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.envelope_type, "events_api");
        assert_eq!(envelope.envelope_id, "env-123");
        assert!(!envelope.accepts_response_payload);

        let payload = envelope.payload.unwrap();
        assert_eq!(payload.token.as_deref(), Some("verification-token"));
        assert_eq!(payload.team_id.as_deref(), Some("T12345"));

        let event = payload.event.unwrap();
        assert_eq!(event.event_type, "message");
        assert_eq!(event.channel.as_deref(), Some("C01234"));
        assert_eq!(event.user.as_deref(), Some("U99999"));
        assert_eq!(event.text.as_deref(), Some("hello bot"));
        assert_eq!(event.ts.as_deref(), Some("1700000000.000100"));
        assert!(event.thread_ts.is_none());
        assert!(event.bot_id.is_none());
    }

    #[test]
    fn deserialize_app_mention_event() {
        let json = r#"{
            "type": "events_api",
            "envelope_id": "env-456",
            "accepts_response_payload": true,
            "payload": {
                "type": "event_callback",
                "event": {
                    "type": "app_mention",
                    "channel": "C56789",
                    "user": "U11111",
                    "text": "<@U00BOT> help me",
                    "ts": "1700000001.000200"
                }
            }
        }"#;
        let envelope: SlackEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.envelope_id, "env-456");
        assert!(envelope.accepts_response_payload);

        let event = envelope.payload.unwrap().event.unwrap();
        assert_eq!(event.event_type, "app_mention");
        assert_eq!(event.text.as_deref(), Some("<@U00BOT> help me"));
    }

    #[test]
    fn deserialize_message_with_thread() {
        let json = r#"{
            "type": "message",
            "channel": "C01234",
            "user": "U55555",
            "text": "threaded reply",
            "ts": "1700000002.000300",
            "thread_ts": "1700000000.000100"
        }"#;
        let event: SlackEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "message");
        assert_eq!(event.thread_ts.as_deref(), Some("1700000000.000100"));
    }

    #[test]
    fn deserialize_bot_message() {
        let json = r#"{
            "type": "message",
            "channel": "C01234",
            "text": "bot says hi",
            "ts": "1700000003.000400",
            "bot_id": "B12345"
        }"#;
        let event: SlackEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.bot_id.as_deref(), Some("B12345"));
        assert!(event.user.is_none());
    }

    #[test]
    fn deserialize_dm_event() {
        let json = r#"{
            "type": "message",
            "channel": "D01234",
            "user": "U77777",
            "text": "private message",
            "ts": "1700000004.000500",
            "channel_type": "im"
        }"#;
        let event: SlackEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.channel_type.as_deref(), Some("im"));
    }

    #[test]
    fn deserialize_envelope_without_payload() {
        let json = r#"{
            "type": "disconnect",
            "envelope_id": "env-789"
        }"#;
        let envelope: SlackEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.envelope_type, "disconnect");
        assert!(envelope.payload.is_none());
    }

    #[test]
    fn serialize_acknowledge() {
        let ack = SlackAcknowledge {
            envelope_id: "env-123".into(),
            payload: None,
        };
        let json = serde_json::to_value(&ack).unwrap();
        assert_eq!(json["envelope_id"], "env-123");
        // payload should be absent, not null
        assert!(json.get("payload").is_none());
    }

    #[test]
    fn serialize_acknowledge_with_payload() {
        let ack = SlackAcknowledge {
            envelope_id: "env-456".into(),
            payload: Some(serde_json::json!({"text": "ok"})),
        };
        let json = serde_json::to_value(&ack).unwrap();
        assert_eq!(json["envelope_id"], "env-456");
        assert_eq!(json["payload"]["text"], "ok");
    }

    #[test]
    fn deserialize_hello() {
        let json = r#"{"type": "hello", "num_connections": 2}"#;
        let hello: SlackHello = serde_json::from_str(json).unwrap();
        assert_eq!(hello.hello_type, "hello");
        assert_eq!(hello.num_connections, Some(2));
    }

    #[test]
    fn deserialize_connections_open_success() {
        let json = r#"{
            "ok": true,
            "url": "wss://wss-primary.slack.com/link?ticket=xxx"
        }"#;
        let resp: ConnectionsOpenResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert!(resp.url.unwrap().starts_with("wss://"));
        assert!(resp.error.is_none());
    }

    #[test]
    fn deserialize_connections_open_error() {
        let json = r#"{
            "ok": false,
            "error": "invalid_auth"
        }"#;
        let resp: ConnectionsOpenResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert!(resp.url.is_none());
        assert_eq!(resp.error.as_deref(), Some("invalid_auth"));
    }

    #[test]
    fn deserialize_chat_post_message_success() {
        let json = r#"{
            "ok": true,
            "channel": "C01234",
            "ts": "1700000005.000600",
            "message": {
                "text": "hello",
                "ts": "1700000005.000600"
            }
        }"#;
        let resp: ChatPostMessageResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.channel.as_deref(), Some("C01234"));
        assert_eq!(resp.ts.as_deref(), Some("1700000005.000600"));
        let msg = resp.message.unwrap();
        assert_eq!(msg.ts.as_deref(), Some("1700000005.000600"));
    }

    #[test]
    fn deserialize_chat_post_message_error() {
        let json = r#"{
            "ok": false,
            "error": "channel_not_found"
        }"#;
        let resp: ChatPostMessageResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error.as_deref(), Some("channel_not_found"));
    }

    #[test]
    fn deserialize_chat_update_success() {
        let json = r#"{
            "ok": true,
            "channel": "C01234",
            "ts": "1700000005.000600"
        }"#;
        let resp: ChatUpdateResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.channel.as_deref(), Some("C01234"));
    }

    #[test]
    fn deserialize_chat_update_error() {
        let json = r#"{
            "ok": false,
            "error": "message_not_found"
        }"#;
        let resp: ChatUpdateResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error.as_deref(), Some("message_not_found"));
    }
}
