//! Discord Gateway event types and opcodes.
//!
//! These types model the Discord Gateway v10 WebSocket protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Gateway opcodes ─────────────────────────────────────────────────────

/// Opcode 0: Dispatch -- an event was dispatched.
pub const OP_DISPATCH: u8 = 0;

/// Opcode 1: Heartbeat -- fired periodically to keep the connection alive.
pub const OP_HEARTBEAT: u8 = 1;

/// Opcode 2: Identify -- start a new session.
pub const OP_IDENTIFY: u8 = 2;

/// Opcode 6: Resume -- resume a previous session.
pub const OP_RESUME: u8 = 6;

/// Opcode 7: Reconnect -- server is going away, client should reconnect.
pub const OP_RECONNECT: u8 = 7;

/// Opcode 9: Invalid Session -- the session has been invalidated.
pub const OP_INVALID_SESSION: u8 = 9;

/// Opcode 10: Hello -- sent on connection, contains heartbeat_interval.
pub const OP_HELLO: u8 = 10;

/// Opcode 11: Heartbeat ACK -- sent in response to receiving a heartbeat.
pub const OP_HEARTBEAT_ACK: u8 = 11;

// ── Payload types ───────────────────────────────────────────────────────

/// A Gateway payload (incoming or outgoing).
///
/// All Gateway communication uses this envelope format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPayload {
    /// The opcode for this payload.
    pub op: u8,

    /// Event data (the `d` field). May be `null` for heartbeats.
    pub d: Option<Value>,

    /// Sequence number, used for resuming sessions and heartbeats.
    /// Only present for opcode 0 (Dispatch).
    pub s: Option<u64>,

    /// Event name (e.g., `"MESSAGE_CREATE"`, `"READY"`).
    /// Only present for opcode 0 (Dispatch).
    pub t: Option<String>,
}

/// The `d` field of an opcode 10 (Hello) payload.
#[derive(Debug, Clone, Deserialize)]
pub struct HelloData {
    /// Interval (in milliseconds) between sending heartbeats.
    pub heartbeat_interval: u64,
}

/// The `d` field of an opcode 2 (Identify) payload.
#[derive(Debug, Clone, Serialize)]
pub struct IdentifyPayload {
    /// Authentication token.
    pub token: String,

    /// Gateway intents bitmask.
    pub intents: u32,

    /// Connection properties (OS, browser, device).
    pub properties: ConnectionProperties,
}

/// Connection properties sent in the Identify payload.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionProperties {
    /// Operating system.
    pub os: String,

    /// Library / browser name.
    pub browser: String,

    /// Device name.
    pub device: String,
}

/// The `d` field of an opcode 6 (Resume) payload.
#[derive(Debug, Clone, Serialize)]
pub struct ResumePayload {
    /// Authentication token.
    pub token: String,

    /// Session ID from the READY event.
    pub session_id: String,

    /// Last sequence number received.
    pub seq: u64,
}

/// A `MESSAGE_CREATE` event payload.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageCreate {
    /// Unique message ID.
    pub id: String,

    /// Channel ID where the message was sent.
    pub channel_id: String,

    /// Text content of the message.
    pub content: String,

    /// The author of the message.
    pub author: User,

    /// Guild (server) ID, if applicable.
    pub guild_id: Option<String>,

    /// Reference to the message being replied to.
    pub message_reference: Option<MessageReference>,
}

/// A Discord user.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    /// Unique user ID (snowflake).
    pub id: String,

    /// The user's display name.
    pub username: String,

    /// Whether this user is a bot.
    #[serde(default)]
    pub bot: bool,
}

/// A message reference (for replies).
#[derive(Debug, Clone, Deserialize)]
pub struct MessageReference {
    /// The ID of the message being referenced.
    pub message_id: Option<String>,

    /// The channel ID of the referenced message.
    pub channel_id: Option<String>,
}

/// The `d` field of a READY event.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyEvent {
    /// Gateway version.
    pub v: u32,

    /// The bot user object.
    pub user: User,

    /// Session ID for resuming.
    pub session_id: String,

    /// The gateway URL for resuming.
    pub resume_gateway_url: Option<String>,
}

/// Rate limit information parsed from Discord REST API response headers.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Number of remaining requests in the current window.
    pub remaining: Option<u32>,

    /// When the rate limit resets (Unix timestamp in seconds, as float).
    pub reset: Option<f64>,

    /// Time in seconds until the rate limit resets.
    pub reset_after: Option<f64>,

    /// The rate limit bucket identifier.
    pub bucket: Option<String>,
}

impl RateLimitInfo {
    /// Parse rate limit information from HTTP response headers.
    pub fn from_headers(headers: &reqwest::header::HeaderMap) -> Self {
        Self {
            remaining: headers
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok()),
            reset: headers
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok()),
            reset_after: headers
                .get("x-ratelimit-reset-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok()),
            bucket: headers
                .get("x-ratelimit-bucket")
                .and_then(|v| v.to_str().ok())
                .map(String::from),
        }
    }

    /// Check if we are rate limited (remaining == 0).
    pub fn is_limited(&self) -> bool {
        self.remaining == Some(0)
    }

    /// Get the number of milliseconds to wait before retrying.
    pub fn retry_after_ms(&self) -> Option<u64> {
        self.reset_after.map(|s| (s * 1000.0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_hello() {
        let json = r#"{
            "op": 10,
            "d": {"heartbeat_interval": 41250},
            "s": null,
            "t": null
        }"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OP_HELLO);
        assert!(payload.s.is_none());
        assert!(payload.t.is_none());

        let hello: HelloData = serde_json::from_value(payload.d.unwrap()).unwrap();
        assert_eq!(hello.heartbeat_interval, 41250);
    }

    #[test]
    fn deserialize_heartbeat_ack() {
        let json = r#"{"op": 11, "d": null, "s": null, "t": null}"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OP_HEARTBEAT_ACK);
        assert!(payload.d.is_none());
    }

    #[test]
    fn deserialize_dispatch_message_create() {
        let json = r#"{
            "op": 0,
            "d": {
                "id": "123456789",
                "channel_id": "987654321",
                "content": "Hello, world!",
                "author": {
                    "id": "111222333",
                    "username": "testuser",
                    "bot": false
                },
                "guild_id": "444555666"
            },
            "s": 42,
            "t": "MESSAGE_CREATE"
        }"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OP_DISPATCH);
        assert_eq!(payload.s, Some(42));
        assert_eq!(payload.t.as_deref(), Some("MESSAGE_CREATE"));

        let msg: MessageCreate = serde_json::from_value(payload.d.unwrap()).unwrap();
        assert_eq!(msg.id, "123456789");
        assert_eq!(msg.channel_id, "987654321");
        assert_eq!(msg.content, "Hello, world!");
        assert_eq!(msg.author.id, "111222333");
        assert_eq!(msg.author.username, "testuser");
        assert!(!msg.author.bot);
        assert_eq!(msg.guild_id.as_deref(), Some("444555666"));
    }

    #[test]
    fn deserialize_message_create_with_reply() {
        let json = r#"{
            "id": "123",
            "channel_id": "456",
            "content": "reply text",
            "author": {"id": "789", "username": "user", "bot": false},
            "message_reference": {
                "message_id": "100",
                "channel_id": "456"
            }
        }"#;
        let msg: MessageCreate = serde_json::from_str(json).unwrap();
        let reference = msg.message_reference.unwrap();
        assert_eq!(reference.message_id.as_deref(), Some("100"));
        assert_eq!(reference.channel_id.as_deref(), Some("456"));
    }

    #[test]
    fn deserialize_dm_message_no_guild() {
        let json = r#"{
            "id": "123",
            "channel_id": "456",
            "content": "dm text",
            "author": {"id": "789", "username": "user"}
        }"#;
        let msg: MessageCreate = serde_json::from_str(json).unwrap();
        assert!(msg.guild_id.is_none());
        assert!(!msg.author.bot);
    }

    #[test]
    fn deserialize_bot_author() {
        let json = r#"{
            "id": "123",
            "channel_id": "456",
            "content": "bot message",
            "author": {"id": "789", "username": "botuser", "bot": true}
        }"#;
        let msg: MessageCreate = serde_json::from_str(json).unwrap();
        assert!(msg.author.bot);
    }

    #[test]
    fn serialize_identify() {
        let identify = IdentifyPayload {
            token: "my-token".into(),
            intents: 37377,
            properties: ConnectionProperties {
                os: "linux".into(),
                browser: "clawft".into(),
                device: "clawft".into(),
            },
        };
        let payload = GatewayPayload {
            op: OP_IDENTIFY,
            d: Some(serde_json::to_value(&identify).unwrap()),
            s: None,
            t: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["op"], 2);
        assert_eq!(json["d"]["token"], "my-token");
        assert_eq!(json["d"]["intents"], 37377);
        assert_eq!(json["d"]["properties"]["os"], "linux");
    }

    #[test]
    fn serialize_heartbeat() {
        let payload = GatewayPayload {
            op: OP_HEARTBEAT,
            d: Some(serde_json::json!(42)),
            s: None,
            t: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["op"], 1);
        assert_eq!(json["d"], 42);
    }

    #[test]
    fn serialize_heartbeat_null_sequence() {
        let payload = GatewayPayload {
            op: OP_HEARTBEAT,
            d: None,
            s: None,
            t: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["op"], 1);
        assert!(json["d"].is_null());
    }

    #[test]
    fn serialize_resume() {
        let resume = ResumePayload {
            token: "my-token".into(),
            session_id: "session-123".into(),
            seq: 42,
        };
        let payload = GatewayPayload {
            op: OP_RESUME,
            d: Some(serde_json::to_value(&resume).unwrap()),
            s: None,
            t: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["op"], 6);
        assert_eq!(json["d"]["session_id"], "session-123");
        assert_eq!(json["d"]["seq"], 42);
    }

    #[test]
    fn deserialize_ready_event() {
        let json = r#"{
            "v": 10,
            "user": {"id": "123", "username": "bot", "bot": true},
            "session_id": "abc-def",
            "resume_gateway_url": "wss://gateway-resume.discord.gg"
        }"#;
        let ready: ReadyEvent = serde_json::from_str(json).unwrap();
        assert_eq!(ready.v, 10);
        assert_eq!(ready.user.id, "123");
        assert_eq!(ready.session_id, "abc-def");
        assert_eq!(
            ready.resume_gateway_url.as_deref(),
            Some("wss://gateway-resume.discord.gg")
        );
    }

    #[test]
    fn deserialize_ready_event_no_resume_url() {
        let json = r#"{
            "v": 10,
            "user": {"id": "123", "username": "bot", "bot": true},
            "session_id": "abc-def"
        }"#;
        let ready: ReadyEvent = serde_json::from_str(json).unwrap();
        assert!(ready.resume_gateway_url.is_none());
    }

    #[test]
    fn opcode_constants() {
        assert_eq!(OP_DISPATCH, 0);
        assert_eq!(OP_HEARTBEAT, 1);
        assert_eq!(OP_IDENTIFY, 2);
        assert_eq!(OP_RESUME, 6);
        assert_eq!(OP_RECONNECT, 7);
        assert_eq!(OP_INVALID_SESSION, 9);
        assert_eq!(OP_HELLO, 10);
        assert_eq!(OP_HEARTBEAT_ACK, 11);
    }

    #[test]
    fn rate_limit_info_from_headers() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-ratelimit-remaining", "5".parse().unwrap());
        headers.insert("x-ratelimit-reset", "1700000000.123".parse().unwrap());
        headers.insert("x-ratelimit-reset-after", "1.5".parse().unwrap());
        headers.insert("x-ratelimit-bucket", "abc123".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);
        assert_eq!(info.remaining, Some(5));
        assert!((info.reset.unwrap() - 1700000000.123).abs() < 0.001);
        assert!((info.reset_after.unwrap() - 1.5).abs() < 0.001);
        assert_eq!(info.bucket.as_deref(), Some("abc123"));
        assert!(!info.is_limited());
    }

    #[test]
    fn rate_limit_info_is_limited() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-ratelimit-remaining", "0".parse().unwrap());
        headers.insert("x-ratelimit-reset-after", "2.0".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);
        assert!(info.is_limited());
        assert_eq!(info.retry_after_ms(), Some(2000));
    }

    #[test]
    fn rate_limit_info_empty_headers() {
        let headers = reqwest::header::HeaderMap::new();
        let info = RateLimitInfo::from_headers(&headers);
        assert!(info.remaining.is_none());
        assert!(info.reset.is_none());
        assert!(info.reset_after.is_none());
        assert!(info.bucket.is_none());
        assert!(!info.is_limited());
        assert!(info.retry_after_ms().is_none());
    }

    #[test]
    fn rate_limit_info_malformed_headers() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-ratelimit-remaining", "not-a-number".parse().unwrap());

        let info = RateLimitInfo::from_headers(&headers);
        assert!(info.remaining.is_none());
    }

    #[test]
    fn deserialize_reconnect() {
        let json = r#"{"op": 7, "d": null, "s": null, "t": null}"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OP_RECONNECT);
    }

    #[test]
    fn deserialize_invalid_session() {
        let json = r#"{"op": 9, "d": false, "s": null, "t": null}"#;
        let payload: GatewayPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.op, OP_INVALID_SESSION);
        // d is false (not resumable).
        assert_eq!(payload.d.unwrap(), serde_json::json!(false));
    }
}
