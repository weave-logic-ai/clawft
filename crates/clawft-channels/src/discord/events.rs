//! Discord Gateway event types and opcodes.
//!
//! These types model the Discord Gateway v10 WebSocket protocol.
//!
//! # Gateway intents (WEFT-173)
//!
//! Identify sends an `intents` bitmask. The clawft default is
//! [`DEFAULT_GATEWAY_INTENTS`] (`37377` =
//! [`INTENT_GUILDS`] | [`INTENT_GUILD_MESSAGES`] | [`INTENT_DIRECT_MESSAGES`] |
//! [`INTENT_MESSAGE_CONTENT`]).
//!
//! Privileged intents ([`INTENT_GUILD_MEMBERS`], [`INTENT_GUILD_PRESENCES`],
//! [`INTENT_MESSAGE_CONTENT`]) must also be enabled in the Discord Developer
//! Portal. If they are requested but not enabled, Discord closes the gateway
//! with code [`CLOSE_CODE_DISALLOWED_INTENTS`] (4014).

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Gateway intents (v10) ────────────────────────────────────────────────

/// Guild create/update/delete and basic guild lifecycle.
pub const INTENT_GUILDS: u32 = 1 << 0;
/// Member join/update/leave — **privileged** (Developer Portal + bot approval).
pub const INTENT_GUILD_MEMBERS: u32 = 1 << 1;
/// Presence updates — **privileged**.
pub const INTENT_GUILD_PRESENCES: u32 = 1 << 8;
/// Message create/update/delete in guild channels.
pub const INTENT_GUILD_MESSAGES: u32 = 1 << 9;
/// Message create/update/delete in DM channels.
pub const INTENT_DIRECT_MESSAGES: u32 = 1 << 12;
/// Access to message text/embeds/attachments content — **privileged**.
pub const INTENT_MESSAGE_CONTENT: u32 = 1 << 15;

/// Privileged intent bits that Discord rejects (close 4014) unless enabled
/// for the application in the Developer Portal.
pub const PRIVILEGED_INTENTS: u32 =
    INTENT_GUILD_MEMBERS | INTENT_GUILD_PRESENCES | INTENT_MESSAGE_CONTENT;

/// Default Identify intents for clawft Discord bots.
///
/// `37377` = GUILDS (1) | GUILD_MESSAGES (512) | DIRECT_MESSAGES (4096) |
/// MESSAGE_CONTENT (32768). MESSAGE_CONTENT is privileged and must be toggled
/// on under Bot → Privileged Gateway Intents.
pub const DEFAULT_GATEWAY_INTENTS: u32 =
    INTENT_GUILDS | INTENT_GUILD_MESSAGES | INTENT_DIRECT_MESSAGES | INTENT_MESSAGE_CONTENT;

/// Discord gateway close code: Disallowed Intents (privileged intent not
/// enabled for the application).
pub const CLOSE_CODE_DISALLOWED_INTENTS: u16 = 4014;

/// Validate gateway intents before Identify.
///
/// Returns `Err` when `intents == 0` (no events would ever be received).
/// Does **not** reject privileged intents: those are allowed when the bot
/// has them enabled in the Developer Portal (default config includes
/// MESSAGE_CONTENT).
pub fn validate_gateway_intents(intents: u32) -> Result<(), String> {
    if intents == 0 {
        return Err(
            "discord intents bitmask is 0: no Gateway events will be received. \
             Use the default 37377 (GUILDS | GUILD_MESSAGES | DIRECT_MESSAGES | \
             MESSAGE_CONTENT) or OR the intent bits you need"
                .into(),
        );
    }
    Ok(())
}

/// Human-readable warnings for privileged intent bits that are set.
///
/// Callers should log these at factory/build or Identify time so operators
/// know the Developer Portal must match. If Discord has not negotiated the
/// privileged intent, the gateway closes with
/// [`CLOSE_CODE_DISALLOWED_INTENTS`].
pub fn privileged_intent_warnings(intents: u32) -> Vec<&'static str> {
    let mut out = Vec::new();
    if intents & INTENT_GUILD_MEMBERS != 0 {
        out.push(
            "privileged intent GUILD_MEMBERS (bit 1) is set: enable Server Members Intent \
             in the Discord Developer Portal, or Discord will reject Identify (close 4014)",
        );
    }
    if intents & INTENT_GUILD_PRESENCES != 0 {
        out.push(
            "privileged intent GUILD_PRESENCES (bit 8) is set: enable Presence Intent \
             in the Discord Developer Portal, or Discord will reject Identify (close 4014)",
        );
    }
    if intents & INTENT_MESSAGE_CONTENT != 0 {
        out.push(
            "privileged intent MESSAGE_CONTENT (bit 15) is set: enable Message Content Intent \
             in the Discord Developer Portal, or Discord will reject Identify (close 4014)",
        );
    }
    out
}

/// Message for gateway close code 4014 (privileged intents not negotiated).
///
/// Returns `None` for other close codes.
pub fn disallowed_intents_close_message(close_code: u16) -> Option<&'static str> {
    if close_code == CLOSE_CODE_DISALLOWED_INTENTS {
        Some(
            "Discord rejected privileged intents (close 4014 Disallowed Intents): \
             enable Message Content / Server Members / Presence under Bot → Privileged \
             Gateway Intents, or remove those bits from channels.discord.intents",
        )
    } else {
        None
    }
}

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

    /// Gateway intents bitmask (see module-level intent constants; default
    /// [`DEFAULT_GATEWAY_INTENTS`] = 37377).
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
        // WEFT-176: browser/device come from the product brand accessor.
        let brand = clawft_types::config::brand();
        let identify = IdentifyPayload {
            token: "my-token".into(),
            intents: DEFAULT_GATEWAY_INTENTS,
            properties: ConnectionProperties {
                os: "linux".into(),
                browser: brand.clone(),
                device: brand.clone(),
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
        assert_eq!(json["d"]["intents"], DEFAULT_GATEWAY_INTENTS);
        assert_eq!(json["d"]["properties"]["os"], "linux");
        assert_eq!(json["d"]["properties"]["browser"], brand);
        assert_eq!(json["d"]["properties"]["device"], brand);
    }

    #[test]
    fn identify_brand_is_configurable() {
        use clawft_types::config::{install_brand, reset_brand_for_test};
        install_brand("Valtech Agentic Mesh");
        assert_eq!(clawft_types::config::brand(), "Valtech Agentic Mesh");
        let identify = IdentifyPayload {
            token: "t".into(),
            intents: DEFAULT_GATEWAY_INTENTS,
            properties: ConnectionProperties {
                os: "linux".into(),
                browser: clawft_types::config::brand(),
                device: clawft_types::config::brand(),
            },
        };
        let v = serde_json::to_value(&identify).unwrap();
        assert_eq!(v["properties"]["browser"], "Valtech Agentic Mesh");
        assert_eq!(v["properties"]["device"], "Valtech Agentic Mesh");
        reset_brand_for_test();
        assert_eq!(
            clawft_types::config::brand(),
            clawft_types::config::DEFAULT_BRAND
        );
    }

    #[test]
    fn default_gateway_intents_bitmask() {
        assert_eq!(DEFAULT_GATEWAY_INTENTS, 37377);
        assert_eq!(
            DEFAULT_GATEWAY_INTENTS,
            INTENT_GUILDS | INTENT_GUILD_MESSAGES | INTENT_DIRECT_MESSAGES | INTENT_MESSAGE_CONTENT
        );
        // Default includes privileged MESSAGE_CONTENT only (not members/presences).
        assert_eq!(
            DEFAULT_GATEWAY_INTENTS & PRIVILEGED_INTENTS,
            INTENT_MESSAGE_CONTENT
        );
    }

    #[test]
    fn validate_gateway_intents_rejects_zero() {
        let err = validate_gateway_intents(0).unwrap_err();
        assert!(
            err.contains("intents bitmask is 0"),
            "expected clear zero-intents error, got: {err}"
        );
        assert!(validate_gateway_intents(DEFAULT_GATEWAY_INTENTS).is_ok());
        assert!(validate_gateway_intents(INTENT_GUILDS).is_ok());
    }

    #[test]
    fn privileged_intent_warnings_for_each_flag() {
        assert!(privileged_intent_warnings(INTENT_GUILDS).is_empty());
        assert!(privileged_intent_warnings(INTENT_GUILD_MESSAGES | INTENT_DIRECT_MESSAGES).is_empty());

        let msg_content = privileged_intent_warnings(INTENT_MESSAGE_CONTENT);
        assert_eq!(msg_content.len(), 1);
        assert!(msg_content[0].contains("MESSAGE_CONTENT"));
        assert!(msg_content[0].contains("4014"));

        let members = privileged_intent_warnings(INTENT_GUILD_MEMBERS);
        assert_eq!(members.len(), 1);
        assert!(members[0].contains("GUILD_MEMBERS"));

        let all = privileged_intent_warnings(PRIVILEGED_INTENTS);
        assert_eq!(all.len(), 3);

        // Default config includes MESSAGE_CONTENT → one warning.
        let default_warns = privileged_intent_warnings(DEFAULT_GATEWAY_INTENTS);
        assert_eq!(default_warns.len(), 1);
        assert!(default_warns[0].contains("MESSAGE_CONTENT"));
    }

    #[test]
    fn disallowed_intents_close_message_only_for_4014() {
        let msg = disallowed_intents_close_message(CLOSE_CODE_DISALLOWED_INTENTS).unwrap();
        assert!(msg.contains("4014"));
        assert!(msg.contains("privileged"));
        assert!(disallowed_intents_close_message(1000).is_none());
        assert!(disallowed_intents_close_message(4000).is_none());
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
