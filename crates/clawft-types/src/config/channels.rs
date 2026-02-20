//! Channel configuration types.
//!
//! Configuration for all supported chat channels: Telegram, Slack, Discord,
//! WhatsApp, Feishu, DingTalk, Mochat, Email, and QQ.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::secret::SecretString;

/// Configuration for all chat channels.
///
/// The `extra` field captures unknown channel plugins via `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    /// Telegram bot configuration.
    #[serde(default)]
    pub telegram: TelegramConfig,

    /// Slack configuration.
    #[serde(default)]
    pub slack: SlackConfig,

    /// Discord bot configuration.
    #[serde(default)]
    pub discord: DiscordConfig,

    /// WhatsApp bridge configuration.
    #[serde(default)]
    pub whatsapp: WhatsAppConfig,

    /// Feishu / Lark configuration.
    #[serde(default)]
    pub feishu: FeishuConfig,

    /// DingTalk configuration.
    #[serde(default)]
    pub dingtalk: DingTalkConfig,

    /// Mochat configuration.
    #[serde(default)]
    pub mochat: MochatConfig,

    /// Email (IMAP + SMTP) configuration.
    #[serde(default)]
    pub email: EmailConfig,

    /// QQ bot configuration.
    #[serde(default)]
    pub qq: QQConfig,

    /// Unknown channel plugins (forward compatibility).
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Telegram channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Bot token from `@BotFather`.
    #[serde(default)]
    pub token: SecretString,

    /// Environment variable name that holds the bot token (e.g. `"TELEGRAM_BOT_TOKEN"`).
    /// When set, the env var is used if `token` is empty.
    #[serde(default, alias = "tokenEnv")]
    pub token_env: Option<String>,

    /// Allowed user IDs or usernames. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,

    /// HTTP/SOCKS5 proxy URL.
    #[serde(default)]
    pub proxy: Option<String>,
}

/// Slack channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Connection mode (currently only `"socket"` is supported).
    #[serde(default = "default_slack_mode")]
    pub mode: String,

    /// Webhook path for event subscriptions.
    #[serde(default = "default_webhook_path", alias = "webhookPath")]
    pub webhook_path: String,

    /// Bot token (`xoxb-...`).
    #[serde(default, alias = "botToken")]
    pub bot_token: SecretString,

    /// Environment variable name for the bot token (e.g. `"SLACK_BOT_TOKEN"`).
    /// Used when `bot_token` is empty.
    #[serde(default, alias = "botTokenEnv")]
    pub bot_token_env: Option<String>,

    /// App-level token (`xapp-...`).
    #[serde(default, alias = "appToken")]
    pub app_token: SecretString,

    /// Environment variable name for the app token (e.g. `"SLACK_APP_TOKEN"`).
    /// Used when `app_token` is empty.
    #[serde(default, alias = "appTokenEnv")]
    pub app_token_env: Option<String>,

    /// Whether the user token is read-only.
    #[serde(default = "super::default_true", alias = "userTokenReadOnly")]
    pub user_token_read_only: bool,

    /// Group message policy: `"mention"`, `"open"`, or `"allowlist"`.
    #[serde(default = "default_group_policy", alias = "groupPolicy")]
    pub group_policy: String,

    /// Allowed channel IDs when `group_policy` is `"allowlist"`.
    #[serde(default, alias = "groupAllowFrom")]
    pub group_allow_from: Vec<String>,

    /// DM-specific policy.
    #[serde(default)]
    pub dm: SlackDMConfig,
}

fn default_slack_mode() -> String {
    "socket".into()
}
fn default_webhook_path() -> String {
    "/slack/events".into()
}
fn default_group_policy() -> String {
    "mention".into()
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_slack_mode(),
            webhook_path: default_webhook_path(),
            bot_token: SecretString::default(),
            bot_token_env: None,
            app_token: SecretString::default(),
            app_token_env: None,
            user_token_read_only: true,
            group_policy: default_group_policy(),
            group_allow_from: Vec::new(),
            dm: SlackDMConfig::default(),
        }
    }
}

/// Slack DM policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackDMConfig {
    /// Whether DMs are enabled.
    #[serde(default = "super::default_true")]
    pub enabled: bool,

    /// DM policy: `"open"` or `"allowlist"`.
    #[serde(default = "default_dm_policy")]
    pub policy: String,

    /// Allowed Slack user IDs when policy is `"allowlist"`.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}

fn default_dm_policy() -> String {
    "open".into()
}

impl Default for SlackDMConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy: default_dm_policy(),
            allow_from: Vec::new(),
        }
    }
}

/// Discord channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Bot token from Discord Developer Portal.
    #[serde(default)]
    pub token: SecretString,

    /// Environment variable name that holds the bot token (e.g. `"DISCORD_BOT_TOKEN"`).
    /// When set, the env var is used if `token` is empty.
    #[serde(default, alias = "tokenEnv")]
    pub token_env: Option<String>,

    /// Allowed user IDs. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,

    /// Gateway WebSocket URL.
    #[serde(default = "default_discord_gateway_url", alias = "gatewayUrl")]
    pub gateway_url: String,

    /// Gateway intents bitmask.
    #[serde(default = "default_discord_intents")]
    pub intents: u32,
}

fn default_discord_gateway_url() -> String {
    "wss://gateway.discord.gg/?v=10&encoding=json".into()
}
fn default_discord_intents() -> u32 {
    37377 // GUILDS + GUILD_MESSAGES + DIRECT_MESSAGES + MESSAGE_CONTENT
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: SecretString::default(),
            token_env: None,
            allow_from: Vec::new(),
            gateway_url: default_discord_gateway_url(),
            intents: default_discord_intents(),
        }
    }
}

/// WhatsApp bridge configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// WebSocket bridge URL.
    #[serde(default = "default_whatsapp_bridge_url", alias = "bridgeUrl")]
    pub bridge_url: String,

    /// Shared token for bridge authentication.
    #[serde(default, alias = "bridgeToken")]
    pub bridge_token: SecretString,

    /// Allowed phone numbers. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}

fn default_whatsapp_bridge_url() -> String {
    "ws://localhost:3001".into()
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bridge_url: default_whatsapp_bridge_url(),
            bridge_token: SecretString::default(),
            allow_from: Vec::new(),
        }
    }
}

/// Feishu / Lark channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeishuConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// App ID from Feishu Open Platform.
    #[serde(default, alias = "appId")]
    pub app_id: String,

    /// App Secret from Feishu Open Platform.
    #[serde(default, alias = "appSecret")]
    pub app_secret: SecretString,

    /// Encrypt Key for event subscription.
    #[serde(default, alias = "encryptKey")]
    pub encrypt_key: String,

    /// Verification Token for event subscription.
    #[serde(default, alias = "verificationToken")]
    pub verification_token: SecretString,

    /// Allowed user open_ids. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}

/// DingTalk channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DingTalkConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// AppKey.
    #[serde(default, alias = "clientId")]
    pub client_id: String,

    /// AppSecret.
    #[serde(default, alias = "clientSecret")]
    pub client_secret: SecretString,

    /// Allowed staff IDs. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}

/// Mochat mention behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MochatMentionConfig {
    /// Whether mentions are required in group messages.
    #[serde(default, alias = "requireInGroups")]
    pub require_in_groups: bool,
}

/// Mochat per-group mention requirement.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MochatGroupRule {
    /// Whether mentions are required in this group.
    #[serde(default, alias = "requireMention")]
    pub require_mention: bool,
}

/// Mochat channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MochatConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Mochat API base URL.
    #[serde(default = "default_mochat_base_url", alias = "baseUrl")]
    pub base_url: String,

    /// WebSocket URL for real-time messaging.
    #[serde(default, alias = "socketUrl")]
    pub socket_url: String,

    /// Socket.IO path.
    #[serde(default = "default_socket_path", alias = "socketPath")]
    pub socket_path: String,

    /// Disable msgpack encoding for Socket.IO.
    #[serde(default, alias = "socketDisableMsgpack")]
    pub socket_disable_msgpack: bool,

    /// Reconnect delay in milliseconds.
    #[serde(default = "default_1000", alias = "socketReconnectDelayMs")]
    pub socket_reconnect_delay_ms: i64,

    /// Maximum reconnect delay in milliseconds.
    #[serde(default = "default_10000", alias = "socketMaxReconnectDelayMs")]
    pub socket_max_reconnect_delay_ms: i64,

    /// Connection timeout in milliseconds.
    #[serde(default = "default_10000", alias = "socketConnectTimeoutMs")]
    pub socket_connect_timeout_ms: i64,

    /// Room refresh interval in milliseconds.
    #[serde(default = "default_30000", alias = "refreshIntervalMs")]
    pub refresh_interval_ms: i64,

    /// Watch timeout in milliseconds.
    #[serde(default = "default_25000", alias = "watchTimeoutMs")]
    pub watch_timeout_ms: i64,

    /// Maximum messages per watch cycle.
    #[serde(default = "default_100", alias = "watchLimit")]
    pub watch_limit: i64,

    /// Retry delay in milliseconds.
    #[serde(default = "default_500", alias = "retryDelayMs")]
    pub retry_delay_ms: i64,

    /// Maximum retry attempts (0 = unlimited).
    #[serde(default, alias = "maxRetryAttempts")]
    pub max_retry_attempts: i64,

    /// Authentication token.
    #[serde(default, alias = "clawToken")]
    pub claw_token: SecretString,

    /// Agent user ID for the bot.
    #[serde(default, alias = "agentUserId")]
    pub agent_user_id: String,

    /// Session IDs to monitor.
    #[serde(default)]
    pub sessions: Vec<String>,

    /// Panel IDs to monitor.
    #[serde(default)]
    pub panels: Vec<String>,

    /// Allowed user IDs. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,

    /// Mention behavior configuration.
    #[serde(default)]
    pub mention: MochatMentionConfig,

    /// Per-group rules keyed by group ID.
    #[serde(default)]
    pub groups: HashMap<String, MochatGroupRule>,

    /// Reply delay mode: `"off"` or `"non-mention"`.
    #[serde(default = "default_reply_delay_mode", alias = "replyDelayMode")]
    pub reply_delay_mode: String,

    /// Reply delay in milliseconds.
    #[serde(default = "default_120000", alias = "replyDelayMs")]
    pub reply_delay_ms: i64,
}

fn default_mochat_base_url() -> String {
    "https://mochat.io".into()
}
fn default_socket_path() -> String {
    "/socket.io".into()
}
fn default_reply_delay_mode() -> String {
    "non-mention".into()
}
fn default_1000() -> i64 {
    1000
}
fn default_10000() -> i64 {
    10000
}
fn default_30000() -> i64 {
    30000
}
fn default_25000() -> i64 {
    25000
}
fn default_100() -> i64 {
    100
}
fn default_500() -> i64 {
    500
}
fn default_120000() -> i64 {
    120000
}

impl Default for MochatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_mochat_base_url(),
            socket_url: String::new(),
            socket_path: default_socket_path(),
            socket_disable_msgpack: false,
            socket_reconnect_delay_ms: default_1000(),
            socket_max_reconnect_delay_ms: default_10000(),
            socket_connect_timeout_ms: default_10000(),
            refresh_interval_ms: default_30000(),
            watch_timeout_ms: default_25000(),
            watch_limit: default_100(),
            retry_delay_ms: default_500(),
            max_retry_attempts: 0,
            claw_token: SecretString::default(),
            agent_user_id: String::new(),
            sessions: Vec::new(),
            panels: Vec::new(),
            allow_from: Vec::new(),
            mention: MochatMentionConfig::default(),
            groups: HashMap::new(),
            reply_delay_mode: default_reply_delay_mode(),
            reply_delay_ms: default_120000(),
        }
    }
}

/// Email channel configuration (IMAP inbound + SMTP outbound).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Explicit owner permission to access mailbox data.
    #[serde(default, alias = "consentGranted")]
    pub consent_granted: bool,

    // -- IMAP (receive) --
    /// IMAP server hostname.
    #[serde(default, alias = "imapHost")]
    pub imap_host: String,

    /// IMAP server port.
    #[serde(default = "default_imap_port", alias = "imapPort")]
    pub imap_port: u16,

    /// IMAP username.
    #[serde(default, alias = "imapUsername")]
    pub imap_username: String,

    /// IMAP password.
    #[serde(default, alias = "imapPassword")]
    pub imap_password: SecretString,

    /// IMAP mailbox to watch.
    #[serde(default = "default_imap_mailbox", alias = "imapMailbox")]
    pub imap_mailbox: String,

    /// Use SSL for IMAP.
    #[serde(default = "super::default_true", alias = "imapUseSsl")]
    pub imap_use_ssl: bool,

    // -- SMTP (send) --
    /// SMTP server hostname.
    #[serde(default, alias = "smtpHost")]
    pub smtp_host: String,

    /// SMTP server port.
    #[serde(default = "default_smtp_port", alias = "smtpPort")]
    pub smtp_port: u16,

    /// SMTP username.
    #[serde(default, alias = "smtpUsername")]
    pub smtp_username: String,

    /// SMTP password.
    #[serde(default, alias = "smtpPassword")]
    pub smtp_password: SecretString,

    /// Use STARTTLS for SMTP.
    #[serde(default = "super::default_true", alias = "smtpUseTls")]
    pub smtp_use_tls: bool,

    /// Use implicit SSL for SMTP.
    #[serde(default, alias = "smtpUseSsl")]
    pub smtp_use_ssl: bool,

    /// From address for outgoing emails.
    #[serde(default, alias = "fromAddress")]
    pub from_address: String,

    // -- Behavior --
    /// If false, inbound email is read but no automatic reply is sent.
    #[serde(default = "super::default_true", alias = "autoReplyEnabled")]
    pub auto_reply_enabled: bool,

    /// Polling interval in seconds.
    #[serde(default = "default_poll_interval", alias = "pollIntervalSeconds")]
    pub poll_interval_seconds: u32,

    /// Mark messages as seen after processing.
    #[serde(default = "super::default_true", alias = "markSeen")]
    pub mark_seen: bool,

    /// Maximum email body characters to process.
    #[serde(default = "default_max_body_chars", alias = "maxBodyChars")]
    pub max_body_chars: u32,

    /// Prefix for reply subjects.
    #[serde(default = "default_subject_prefix", alias = "subjectPrefix")]
    pub subject_prefix: String,

    /// Allowed sender email addresses. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}

fn default_imap_port() -> u16 {
    993
}
fn default_imap_mailbox() -> String {
    "INBOX".into()
}
fn default_smtp_port() -> u16 {
    587
}
fn default_poll_interval() -> u32 {
    30
}
fn default_max_body_chars() -> u32 {
    12000
}
fn default_subject_prefix() -> String {
    "Re: ".into()
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            consent_granted: false,
            imap_host: String::new(),
            imap_port: default_imap_port(),
            imap_username: String::new(),
            imap_password: SecretString::default(),
            imap_mailbox: default_imap_mailbox(),
            imap_use_ssl: true,
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_username: String::new(),
            smtp_password: SecretString::default(),
            smtp_use_tls: true,
            smtp_use_ssl: false,
            from_address: String::new(),
            auto_reply_enabled: true,
            poll_interval_seconds: default_poll_interval(),
            mark_seen: true,
            max_body_chars: default_max_body_chars(),
            subject_prefix: default_subject_prefix(),
            allow_from: Vec::new(),
        }
    }
}

/// QQ bot configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QQConfig {
    /// Whether this channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Bot AppID.
    #[serde(default, alias = "appId")]
    pub app_id: String,

    /// Bot AppSecret.
    #[serde(default)]
    pub secret: SecretString,

    /// Allowed user openids. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}
