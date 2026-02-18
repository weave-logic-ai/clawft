//! Configuration schema types.
//!
//! A faithful port of `nanobot/config/schema.py`. All structs support
//! both `snake_case` and `camelCase` field names in JSON via `#[serde(alias)]`.
//! Unknown fields are silently ignored for forward compatibility.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::delegation::DelegationConfig;
use crate::routing::RoutingConfig;

// ── Root config ──────────────────────────────────────────────────────────

/// Root configuration for the clawft framework.
///
/// Mirrors the Python `Config(BaseSettings)` class.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Agent defaults and per-agent overrides.
    #[serde(default)]
    pub agents: AgentsConfig,

    /// Chat channel configurations (Telegram, Slack, Discord, etc.).
    #[serde(default)]
    pub channels: ChannelsConfig,

    /// LLM provider credentials and settings.
    #[serde(default)]
    pub providers: ProvidersConfig,

    /// Gateway / HTTP server settings.
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Tool configurations (web search, exec, MCP servers).
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Task delegation routing configuration.
    #[serde(default)]
    pub delegation: DelegationConfig,

    /// Tiered routing and permission configuration.
    #[serde(default)]
    pub routing: RoutingConfig,
}

impl Config {
    /// Get the expanded workspace path.
    pub fn workspace_path(&self) -> PathBuf {
        let raw = &self.agents.defaults.workspace;
        if let Some(rest) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(rest);
        }
        PathBuf::from(raw)
    }
}

// ── Agents ───────────────────────────────────────────────────────────────

/// Agent configuration container.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    /// Default settings applied to all agents.
    #[serde(default)]
    pub defaults: AgentDefaults,
}

/// Default agent settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    /// Working directory for agent file operations.
    #[serde(default = "default_workspace")]
    pub workspace: String,

    /// Default LLM model identifier.
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens in a single LLM response.
    #[serde(default = "default_max_tokens", alias = "maxTokens")]
    pub max_tokens: i32,

    /// Sampling temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f64,

    /// Maximum tool-use iterations per turn.
    #[serde(default = "default_max_tool_iterations", alias = "maxToolIterations")]
    pub max_tool_iterations: i32,

    /// Number of recent messages to include in context.
    #[serde(default = "default_memory_window", alias = "memoryWindow")]
    pub memory_window: i32,
}

fn default_workspace() -> String {
    "~/.nanobot/workspace".into()
}
fn default_model() -> String {
    "anthropic/claude-opus-4-5".into()
}
fn default_max_tokens() -> i32 {
    8192
}
fn default_temperature() -> f64 {
    0.7
}
fn default_max_tool_iterations() -> i32 {
    20
}
fn default_memory_window() -> i32 {
    50
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            workspace: default_workspace(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            max_tool_iterations: default_max_tool_iterations(),
            memory_window: default_memory_window(),
        }
    }
}

// ── Channels ─────────────────────────────────────────────────────────────

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
    pub token: String,

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
    pub bot_token: String,

    /// App-level token (`xapp-...`).
    #[serde(default, alias = "appToken")]
    pub app_token: String,

    /// Whether the user token is read-only.
    #[serde(default = "default_true", alias = "userTokenReadOnly")]
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
            bot_token: String::new(),
            app_token: String::new(),
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
    #[serde(default = "default_true")]
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
    pub token: String,

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
            token: String::new(),
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
    pub bridge_token: String,

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
            bridge_token: String::new(),
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
    pub app_secret: String,

    /// Encrypt Key for event subscription.
    #[serde(default, alias = "encryptKey")]
    pub encrypt_key: String,

    /// Verification Token for event subscription.
    #[serde(default, alias = "verificationToken")]
    pub verification_token: String,

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
    pub client_secret: String,

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
    pub claw_token: String,

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
            claw_token: String::new(),
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

    // ── IMAP (receive) ──
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
    pub imap_password: String,

    /// IMAP mailbox to watch.
    #[serde(default = "default_imap_mailbox", alias = "imapMailbox")]
    pub imap_mailbox: String,

    /// Use SSL for IMAP.
    #[serde(default = "default_true", alias = "imapUseSsl")]
    pub imap_use_ssl: bool,

    // ── SMTP (send) ──
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
    pub smtp_password: String,

    /// Use STARTTLS for SMTP.
    #[serde(default = "default_true", alias = "smtpUseTls")]
    pub smtp_use_tls: bool,

    /// Use implicit SSL for SMTP.
    #[serde(default, alias = "smtpUseSsl")]
    pub smtp_use_ssl: bool,

    /// From address for outgoing emails.
    #[serde(default, alias = "fromAddress")]
    pub from_address: String,

    // ── Behavior ──
    /// If false, inbound email is read but no automatic reply is sent.
    #[serde(default = "default_true", alias = "autoReplyEnabled")]
    pub auto_reply_enabled: bool,

    /// Polling interval in seconds.
    #[serde(default = "default_poll_interval", alias = "pollIntervalSeconds")]
    pub poll_interval_seconds: u32,

    /// Mark messages as seen after processing.
    #[serde(default = "default_true", alias = "markSeen")]
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
            imap_password: String::new(),
            imap_mailbox: default_imap_mailbox(),
            imap_use_ssl: true,
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_username: String::new(),
            smtp_password: String::new(),
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
    pub secret: String,

    /// Allowed user openids. Empty = allow all.
    #[serde(default, alias = "allowFrom")]
    pub allow_from: Vec<String>,
}

// ── Providers ────────────────────────────────────────────────────────────

/// LLM provider credentials.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    /// API key for authentication.
    #[serde(default, alias = "apiKey")]
    pub api_key: String,

    /// Base URL override (e.g. for proxies).
    #[serde(default, alias = "apiBase")]
    pub api_base: Option<String>,

    /// Custom HTTP headers (e.g. `APP-Code` for AiHubMix).
    #[serde(default, alias = "extraHeaders")]
    pub extra_headers: Option<HashMap<String, String>>,
}

/// Configuration for all LLM providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    /// Custom OpenAI-compatible endpoint.
    #[serde(default)]
    pub custom: ProviderConfig,

    /// Anthropic.
    #[serde(default)]
    pub anthropic: ProviderConfig,

    /// OpenAI.
    #[serde(default)]
    pub openai: ProviderConfig,

    /// OpenRouter gateway.
    #[serde(default)]
    pub openrouter: ProviderConfig,

    /// DeepSeek.
    #[serde(default)]
    pub deepseek: ProviderConfig,

    /// Groq.
    #[serde(default)]
    pub groq: ProviderConfig,

    /// Zhipu AI.
    #[serde(default)]
    pub zhipu: ProviderConfig,

    /// DashScope (Alibaba Cloud Qwen).
    #[serde(default)]
    pub dashscope: ProviderConfig,

    /// vLLM / local server.
    #[serde(default)]
    pub vllm: ProviderConfig,

    /// Google Gemini.
    #[serde(default)]
    pub gemini: ProviderConfig,

    /// Moonshot (Kimi).
    #[serde(default)]
    pub moonshot: ProviderConfig,

    /// MiniMax.
    #[serde(default)]
    pub minimax: ProviderConfig,

    /// AiHubMix gateway.
    #[serde(default)]
    pub aihubmix: ProviderConfig,

    /// OpenAI Codex (OAuth-based).
    #[serde(default)]
    pub openai_codex: ProviderConfig,
}

// ── Gateway ──────────────────────────────────────────────────────────────

/// Gateway / HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Bind address.
    #[serde(default = "default_gateway_host")]
    pub host: String,

    /// Listen port.
    #[serde(default = "default_gateway_port")]
    pub port: u16,

    /// Heartbeat interval in minutes (0 = disabled).
    #[serde(default, alias = "heartbeatIntervalMinutes")]
    pub heartbeat_interval_minutes: u64,

    /// Heartbeat prompt text.
    #[serde(default = "default_heartbeat_prompt", alias = "heartbeatPrompt")]
    pub heartbeat_prompt: String,
}

fn default_gateway_host() -> String {
    "0.0.0.0".into()
}
fn default_gateway_port() -> u16 {
    18790
}
fn default_heartbeat_prompt() -> String {
    "heartbeat".into()
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_gateway_host(),
            port: default_gateway_port(),
            heartbeat_interval_minutes: 0,
            heartbeat_prompt: default_heartbeat_prompt(),
        }
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

/// Tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// Web tools (search, etc.).
    #[serde(default)]
    pub web: WebToolsConfig,

    /// Shell exec tool settings.
    #[serde(default, rename = "exec")]
    pub exec_tool: ExecToolConfig,

    /// Whether to restrict all tool access to the workspace directory.
    #[serde(default, alias = "restrictToWorkspace")]
    pub restrict_to_workspace: bool,

    /// MCP server connections.
    #[serde(default, alias = "mcpServers")]
    pub mcp_servers: HashMap<String, MCPServerConfig>,

    /// Command execution policy (allowlist/denylist).
    #[serde(default, alias = "commandPolicy")]
    pub command_policy: CommandPolicyConfig,

    /// URL safety policy (SSRF protection).
    #[serde(default, alias = "urlPolicy")]
    pub url_policy: UrlPolicyConfig,
}

/// Web tools configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebToolsConfig {
    /// Search engine settings.
    #[serde(default)]
    pub search: WebSearchConfig,
}

/// Web search tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    /// Search API key (e.g. Brave Search).
    #[serde(default, alias = "apiKey")]
    pub api_key: String,

    /// Maximum number of search results.
    #[serde(default = "default_max_results", alias = "maxResults")]
    pub max_results: u32,
}

fn default_max_results() -> u32 {
    5
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            max_results: default_max_results(),
        }
    }
}

/// Shell exec tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecToolConfig {
    /// Command timeout in seconds.
    #[serde(default = "default_exec_timeout")]
    pub timeout: u32,
}

fn default_exec_timeout() -> u32 {
    60
}

impl Default for ExecToolConfig {
    fn default() -> Self {
        Self {
            timeout: default_exec_timeout(),
        }
    }
}

/// Command execution security policy configuration.
///
/// Controls which commands the shell and spawn tools are allowed to execute.
/// In allowlist mode (default), only explicitly permitted commands can run.
/// In denylist mode, any command not matching a blocked pattern is allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicyConfig {
    /// Policy mode: "allowlist" (default, recommended) or "denylist".
    #[serde(default = "default_policy_mode")]
    pub mode: String,

    /// Permitted command basenames when in allowlist mode.
    /// Overrides defaults when non-empty.
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Blocked command patterns when in denylist mode.
    /// Overrides defaults when non-empty.
    #[serde(default)]
    pub denylist: Vec<String>,
}

fn default_policy_mode() -> String {
    "allowlist".to_string()
}

impl Default for CommandPolicyConfig {
    fn default() -> Self {
        Self {
            mode: default_policy_mode(),
            allowlist: Vec::new(),
            denylist: Vec::new(),
        }
    }
}

/// URL safety policy configuration for SSRF protection.
///
/// Controls which URLs the web fetch tool is allowed to access.
/// When enabled (default), requests to private networks, loopback
/// addresses, and cloud metadata endpoints are blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlPolicyConfig {
    /// Whether URL safety validation is enabled.
    #[serde(default = "default_url_policy_enabled")]
    pub enabled: bool,

    /// Allow requests to private/internal IP ranges.
    #[serde(default, alias = "allowPrivate")]
    pub allow_private: bool,

    /// Domains that bypass all safety checks.
    #[serde(default, alias = "allowedDomains")]
    pub allowed_domains: Vec<String>,

    /// Domains that are always blocked.
    #[serde(default, alias = "blockedDomains")]
    pub blocked_domains: Vec<String>,
}

fn default_url_policy_enabled() -> bool {
    true
}

impl Default for UrlPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: default_url_policy_enabled(),
            allow_private: false,
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }
}

/// MCP server connection configuration (stdio or HTTP).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MCPServerConfig {
    /// Command to run (for stdio transport, e.g. `"npx"`).
    #[serde(default)]
    pub command: String,

    /// Command arguments (for stdio transport).
    #[serde(default)]
    pub args: Vec<String>,

    /// Extra environment variables (for stdio transport).
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Streamable HTTP endpoint URL (for HTTP transport).
    #[serde(default)]
    pub url: String,
}

// ── Shared helpers ───────────────────────────────────────────────────────

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Path to the test fixture config.
    const FIXTURE_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/config.json"
    );

    fn load_fixture() -> Config {
        let content =
            std::fs::read_to_string(FIXTURE_PATH).expect("fixture config.json should exist");
        serde_json::from_str(&content).expect("fixture should deserialize")
    }

    #[test]
    fn deserialize_fixture() {
        let cfg = load_fixture();

        // Agents
        assert_eq!(cfg.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert_eq!(cfg.agents.defaults.max_tokens, 8192);
        assert_eq!(cfg.agents.defaults.temperature, 0.7);
        assert_eq!(cfg.agents.defaults.max_tool_iterations, 20);
        assert_eq!(cfg.agents.defaults.memory_window, 50);

        // Channels
        assert!(cfg.channels.telegram.enabled);
        assert_eq!(cfg.channels.telegram.token, "test-bot-token-123");
        assert_eq!(cfg.channels.telegram.allow_from, vec!["user1", "user2"]);
        assert!(!cfg.channels.slack.enabled);
        assert!(!cfg.channels.discord.enabled);

        // Providers
        assert_eq!(cfg.providers.anthropic.api_key, "sk-ant-test-key");
        assert_eq!(cfg.providers.openrouter.api_key, "sk-or-test-key");
        assert_eq!(
            cfg.providers.openrouter.api_base.as_deref(),
            Some("https://openrouter.ai/api/v1")
        );
        assert!(cfg.providers.deepseek.api_key.is_empty());

        // Gateway
        assert_eq!(cfg.gateway.host, "0.0.0.0");
        assert_eq!(cfg.gateway.port, 18790);

        // Tools
        assert_eq!(cfg.tools.web.search.max_results, 5);
        assert_eq!(cfg.tools.exec_tool.timeout, 60);
        assert!(!cfg.tools.restrict_to_workspace);
        assert!(cfg.tools.mcp_servers.contains_key("test-server"));
        let mcp = &cfg.tools.mcp_servers["test-server"];
        assert_eq!(mcp.command, "npx");
        assert_eq!(mcp.args, vec!["-y", "test-mcp-server"]);
    }

    #[test]
    fn camel_case_aliases() {
        // The fixture uses camelCase (maxTokens, allowFrom, etc.)
        // This test is essentially the same as deserialize_fixture
        // but focuses on alias correctness.
        let cfg = load_fixture();
        assert_eq!(cfg.agents.defaults.max_tokens, 8192); // maxTokens
        assert_eq!(cfg.agents.defaults.max_tool_iterations, 20); // maxToolIterations
        assert_eq!(cfg.agents.defaults.memory_window, 50); // memoryWindow
        assert_eq!(cfg.channels.telegram.allow_from, vec!["user1", "user2"]); // allowFrom
    }

    #[test]
    fn default_values_for_missing_fields() {
        let json = r#"{}"#;
        let cfg: Config = serde_json::from_str(json).unwrap();

        // Agent defaults
        assert_eq!(cfg.agents.defaults.workspace, "~/.nanobot/workspace");
        assert_eq!(cfg.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert_eq!(cfg.agents.defaults.max_tokens, 8192);
        assert!((cfg.agents.defaults.temperature - 0.7).abs() < f64::EPSILON);
        assert_eq!(cfg.agents.defaults.max_tool_iterations, 20);
        assert_eq!(cfg.agents.defaults.memory_window, 50);

        // Channel defaults
        assert!(!cfg.channels.telegram.enabled);
        assert!(cfg.channels.telegram.token.is_empty());
        assert!(!cfg.channels.slack.enabled);
        assert_eq!(cfg.channels.slack.mode, "socket");
        assert!(!cfg.channels.discord.enabled);
        assert_eq!(cfg.channels.discord.intents, 37377);

        // Gateway defaults
        assert_eq!(cfg.gateway.host, "0.0.0.0");
        assert_eq!(cfg.gateway.port, 18790);

        // Tool defaults
        assert_eq!(cfg.tools.exec_tool.timeout, 60);
        assert_eq!(cfg.tools.web.search.max_results, 5);
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = load_fixture();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agents.defaults.model, cfg.agents.defaults.model);
        assert_eq!(restored.gateway.port, cfg.gateway.port);
        assert_eq!(
            restored.providers.anthropic.api_key,
            cfg.providers.anthropic.api_key
        );
    }

    #[test]
    fn unknown_fields_ignored() {
        let json = r#"{
            "agents": { "defaults": { "model": "test" } },
            "unknown_top_level": true,
            "channels": {
                "telegram": { "enabled": false, "some_future_field": 42 }
            },
            "providers": {
                "anthropic": { "apiKey": "k", "newField": "x" }
            }
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.agents.defaults.model, "test");
        assert!(!cfg.channels.telegram.enabled);
        assert_eq!(cfg.providers.anthropic.api_key, "k");
    }

    #[test]
    fn unknown_channel_plugins_in_extra() {
        let json = r#"{
            "channels": {
                "telegram": { "enabled": true },
                "my_custom_channel": { "url": "wss://custom.io" }
            }
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert!(cfg.channels.telegram.enabled);
        assert!(cfg.channels.extra.contains_key("my_custom_channel"));
    }

    #[test]
    fn workspace_path_expansion() {
        let mut cfg = Config::default();
        cfg.agents.defaults.workspace = "~/.clawft/workspace".into();
        let path = cfg.workspace_path();
        // Should not start with "~" after expansion
        assert!(!path.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn provider_config_with_extra_headers() {
        let json = r#"{
            "apiKey": "test",
            "extraHeaders": { "X-Custom": "value" }
        }"#;
        let cfg: ProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.api_key, "test");
        let headers = cfg.extra_headers.unwrap();
        assert_eq!(headers["X-Custom"], "value");
    }

    #[test]
    fn email_config_defaults() {
        let cfg = EmailConfig::default();
        assert_eq!(cfg.imap_port, 993);
        assert!(cfg.imap_use_ssl);
        assert_eq!(cfg.smtp_port, 587);
        assert!(cfg.smtp_use_tls);
        assert!(!cfg.smtp_use_ssl);
        assert!(cfg.auto_reply_enabled);
        assert_eq!(cfg.poll_interval_seconds, 30);
        assert!(cfg.mark_seen);
        assert_eq!(cfg.max_body_chars, 12000);
        assert_eq!(cfg.subject_prefix, "Re: ");
    }

    #[test]
    fn mochat_config_defaults() {
        let cfg = MochatConfig::default();
        assert_eq!(cfg.base_url, "https://mochat.io");
        assert_eq!(cfg.socket_path, "/socket.io");
        assert_eq!(cfg.socket_reconnect_delay_ms, 1000);
        assert_eq!(cfg.socket_max_reconnect_delay_ms, 10000);
        assert_eq!(cfg.socket_connect_timeout_ms, 10000);
        assert_eq!(cfg.refresh_interval_ms, 30000);
        assert_eq!(cfg.watch_timeout_ms, 25000);
        assert_eq!(cfg.watch_limit, 100);
        assert_eq!(cfg.retry_delay_ms, 500);
        assert_eq!(cfg.max_retry_attempts, 0);
        assert_eq!(cfg.reply_delay_mode, "non-mention");
        assert_eq!(cfg.reply_delay_ms, 120000);
    }

    #[test]
    fn slack_dm_config_defaults() {
        let cfg = SlackDMConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.policy, "open");
    }

    #[test]
    fn gateway_heartbeat_defaults() {
        let cfg = GatewayConfig::default();
        assert_eq!(cfg.heartbeat_interval_minutes, 0);
        assert_eq!(cfg.heartbeat_prompt, "heartbeat");
    }

    #[test]
    fn gateway_heartbeat_from_json() {
        let json = r#"{
            "host": "0.0.0.0",
            "port": 8080,
            "heartbeatIntervalMinutes": 15,
            "heartbeatPrompt": "status check"
        }"#;
        let cfg: GatewayConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.heartbeat_interval_minutes, 15);
        assert_eq!(cfg.heartbeat_prompt, "status check");
    }

    #[test]
    fn gateway_heartbeat_disabled_by_default() {
        let json = r#"{"host": "0.0.0.0"}"#;
        let cfg: GatewayConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.heartbeat_interval_minutes, 0);
        assert_eq!(cfg.heartbeat_prompt, "heartbeat");
    }

    #[test]
    fn mcp_server_config_roundtrip() {
        let cfg = MCPServerConfig {
            command: "npx".into(),
            args: vec!["-y".into(), "test-server".into()],
            env: {
                let mut m = HashMap::new();
                m.insert("API_KEY".into(), "secret".into());
                m
            },
            url: String::new(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: MCPServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.command, "npx");
        assert_eq!(restored.args.len(), 2);
        assert_eq!(restored.env["API_KEY"], "secret");
    }

    #[test]
    fn command_policy_config_defaults() {
        let config: CommandPolicyConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.mode, "allowlist");
        assert!(config.allowlist.is_empty());
        assert!(config.denylist.is_empty());
    }

    #[test]
    fn command_policy_config_custom() {
        let json = r#"{"mode": "denylist", "allowlist": ["echo", "ls"], "denylist": ["rm -rf /"]}"#;
        let config: CommandPolicyConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.mode, "denylist");
        assert_eq!(config.allowlist, vec!["echo", "ls"]);
        assert_eq!(config.denylist, vec!["rm -rf /"]);
    }

    #[test]
    fn url_policy_config_defaults() {
        let config: UrlPolicyConfig = serde_json::from_str("{}").unwrap();
        assert!(config.enabled);
        assert!(!config.allow_private);
        assert!(config.allowed_domains.is_empty());
        assert!(config.blocked_domains.is_empty());
    }

    #[test]
    fn url_policy_config_custom() {
        let json = r#"{"enabled": false, "allowPrivate": true, "allowedDomains": ["internal.corp"], "blockedDomains": ["evil.com"]}"#;
        let config: UrlPolicyConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert!(config.allow_private);
        assert_eq!(config.allowed_domains, vec!["internal.corp"]);
        assert_eq!(config.blocked_domains, vec!["evil.com"]);
    }

    #[test]
    fn tools_config_includes_policies() {
        let json = r#"{"commandPolicy": {"mode": "denylist"}, "urlPolicy": {"enabled": false}}"#;
        let config: ToolsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.command_policy.mode, "denylist");
        assert!(!config.url_policy.enabled);
    }

    #[test]
    fn tools_config_policies_default_when_absent() {
        let config: ToolsConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.command_policy.mode, "allowlist");
        assert!(config.url_policy.enabled);
    }
}
