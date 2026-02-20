# Phase E-Consumer: WhatsApp, Signal, Matrix/IRC Channels

> **Element:** 06 -- Channel Enhancements
> **Phase:** E-Consumer (E3, E4, E5)
> **Timeline:** Week 6-8
> **Priority:** P2
> **Crates:** `crates/clawft-channels/` (new submodules: `whatsapp/`, `signal/`, `matrix/`, `irc/`)
> **Dependencies IN:** C1 (`ChannelAdapter` trait from `clawft-plugin`), A4 (`SecretRef` for credential fields), A9 (feature gates pattern)
> **Blocks:** None directly
> **Status:** Planning

---

## 1. Overview

Phase E-Consumer adds four consumer messaging channel adapters -- WhatsApp (E3), Signal (E4), Matrix (E5), and IRC (E5) -- to the clawft channel plugin system. All four channels implement the `ChannelAdapter` trait from `clawft-plugin` (NOT the legacy `Channel` trait from `clawft-channels/src/traits.rs`). Each channel is feature-gated so it can be compiled independently, and all credential fields use the `SecretRef` type established in A4.

### Key Design Decisions

- **ChannelAdapter over Channel:** New channels implement `ChannelAdapter` from `clawft-plugin`. The C7 (PluginHost Unification) phase handles bridging/migration for existing channels. During the transition, the `ChannelAdapter->Channel` shim in `clawft-plugin` allows new adapters to be loaded by the existing `PluginHost`.
- **Feature gating:** Each channel is behind a Cargo feature flag so users only compile the channels they need. No new channels are included in the default feature set.
- **Subprocess isolation (E4):** Signal uses `signal-cli` as a subprocess, requiring strict argument sanitization and process lifecycle management.

---

## 2. Architecture

### Trait Implementation

All channels implement the `ChannelAdapter` trait defined in `clawft-plugin`:

```rust
// From crates/clawft-plugin (C1)
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn supports_threads(&self) -> bool;
    fn supports_media(&self) -> bool;

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError>;

    async fn send(
        &self,
        target: &str,
        payload: MessagePayload,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<String, PluginError>;
}
```

### Module Layout

```
crates/clawft-channels/src/
  whatsapp/           # E3 - WhatsApp Cloud API
    mod.rs            # Module re-exports
    channel.rs        # WhatsAppChannel (ChannelAdapter impl)
    api.rs            # WhatsApp Cloud API REST client
    types.rs          # WhatsApp-specific types (webhook payloads, message formats)
    factory.rs        # WhatsAppChannelFactory
  signal/             # E4 - signal-cli subprocess
    mod.rs            # Module re-exports
    channel.rs        # SignalChannel (ChannelAdapter impl)
    subprocess.rs     # SignalSubprocess lifecycle + sanitization
    types.rs          # Signal-specific types (JSON-RPC messages)
    factory.rs        # SignalChannelFactory
  matrix/             # E5 - Matrix SDK
    mod.rs            # Module re-exports
    channel.rs        # MatrixChannel (ChannelAdapter impl)
    api.rs            # Matrix SDK wrapper
    types.rs          # Matrix-specific types (room events, sync state)
    factory.rs        # MatrixChannelFactory
  irc/                # E5 - IRC protocol
    mod.rs            # Module re-exports
    channel.rs        # IrcChannel (ChannelAdapter impl)
    protocol.rs       # IRC client wrapper
    types.rs          # IRC-specific types (commands, modes)
    factory.rs        # IrcChannelFactory
```

### Feature Gating

In `crates/clawft-channels/Cargo.toml`:

```toml
[features]
whatsapp = ["dep:reqwest"]
signal = []
matrix = ["dep:matrix-sdk"]
irc = ["dep:irc"]
```

In `crates/clawft-channels/src/lib.rs`:

```rust
#[cfg(feature = "whatsapp")]
pub mod whatsapp;
#[cfg(feature = "signal")]
pub mod signal;
#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "irc")]
pub mod irc;
```

Note: `reqwest` is already a dependency of `clawft-channels` (used by Slack and Discord). The `whatsapp` feature reuses this existing dependency. The `signal` feature has no additional crate dependencies since it communicates via subprocess.

---

## 3. Implementation Tasks

### E3: WhatsApp Channel

**Transport:** WhatsApp Cloud API (REST)
**Auth:** App access token via `SecretRef`
**New Files:** `crates/clawft-channels/src/whatsapp/{mod.rs, channel.rs, api.rs, types.rs, factory.rs}`
**Feature Flag:** `whatsapp`

#### Task E3.1: WhatsApp Types

Define webhook payload types and outbound message format types.

```rust
// crates/clawft-channels/src/whatsapp/types.rs

use serde::{Deserialize, Serialize};

/// Inbound webhook notification from WhatsApp Cloud API.
#[derive(Debug, Deserialize)]
pub struct WebhookNotification {
    pub object: String,
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookChange {
    pub field: String,
    pub value: WebhookValue,
}

#[derive(Debug, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: String,
    pub metadata: WebhookMetadata,
    #[serde(default)]
    pub messages: Vec<WebhookMessage>,
    #[serde(default)]
    pub statuses: Vec<WebhookStatus>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookMetadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: Option<TextContent>,
}

#[derive(Debug, Deserialize)]
pub struct TextContent {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookStatus {
    pub id: String,
    pub status: String,
    pub timestamp: String,
    pub recipient_id: String,
}

/// Outbound message payload for WhatsApp Cloud API.
#[derive(Debug, Serialize)]
pub struct OutboundPayload {
    pub messaging_product: String,
    pub recipient_type: String,
    pub to: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<OutboundText>,
}

#[derive(Debug, Serialize)]
pub struct OutboundText {
    pub preview_url: bool,
    pub body: String,
}

/// WhatsApp channel configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WhatsAppConfig {
    /// WhatsApp phone number ID.
    pub phone_number_id: String,
    /// Environment variable name for the access token.
    pub access_token_env: String,
    /// Environment variable name for the webhook verify token.
    pub verify_token_env: String,
    /// Port for the inbound webhook HTTP server.
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    /// Maximum outbound messages per second.
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_second: u32,
    /// Optional allowed sender phone numbers (empty = all allowed).
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}

fn default_webhook_port() -> u16 {
    8080
}

fn default_rate_limit() -> u32 {
    80
}
```

#### Task E3.2: WhatsApp API Client

REST client for WhatsApp Cloud API with rate limiting and backoff.

```rust
// crates/clawft-channels/src/whatsapp/api.rs

use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::Client;
use tokio::sync::Semaphore;
use tracing::{debug, warn};

use clawft_plugin::PluginError;

use super::types::{OutboundPayload, OutboundText};

const WHATSAPP_API_BASE: &str = "https://graph.facebook.com/v21.0";
const MAX_RETRY_ATTEMPTS: u32 = 5;

/// REST client for WhatsApp Cloud API with built-in rate limiting.
pub struct WhatsAppApiClient {
    http: Client,
    phone_number_id: String,
    access_token: String,
    rate_limiter: Arc<Semaphore>,
    rate_limit_per_second: u32,
}

impl WhatsAppApiClient {
    pub fn new(
        phone_number_id: String,
        access_token: String,
        rate_limit_per_second: u32,
    ) -> Self {
        Self {
            http: Client::new(),
            phone_number_id,
            access_token,
            rate_limiter: Arc::new(Semaphore::new(rate_limit_per_second as usize)),
            rate_limit_per_second,
        }
    }

    /// Send a text message to the given recipient phone number.
    /// Implements exponential backoff on 429 responses.
    pub async fn send_text(
        &self,
        to: &str,
        body: &str,
    ) -> Result<String, PluginError> {
        let payload = OutboundPayload {
            messaging_product: "whatsapp".into(),
            recipient_type: "individual".into(),
            to: to.into(),
            message_type: "text".into(),
            text: Some(OutboundText {
                preview_url: false,
                body: body.into(),
            }),
        };

        let url = format!(
            "{}/{}/messages",
            WHATSAPP_API_BASE, self.phone_number_id
        );

        let mut attempt = 0u32;
        loop {
            // Acquire rate limit permit.
            let _permit = self.rate_limiter.acquire().await
                .map_err(|e| PluginError::Channel(format!("rate limiter closed: {e}")))?;

            // Release permit after 1/rate_limit_per_second duration.
            let limiter = self.rate_limiter.clone();
            let interval_ms = 1000 / self.rate_limit_per_second.max(1) as u64;
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                drop(_permit);
            });

            let response = self
                .http
                .post(&url)
                .bearer_auth(&self.access_token)
                .json(&payload)
                .send()
                .await
                .map_err(|e| PluginError::Channel(format!("WhatsApp API request failed: {e}")))?;

            if response.status().is_success() {
                let body: serde_json::Value = response.json().await
                    .map_err(|e| PluginError::Channel(format!("WhatsApp API response parse error: {e}")))?;
                let message_id = body["messages"][0]["id"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                return Ok(message_id);
            }

            if response.status().as_u16() == 429 {
                attempt += 1;
                if attempt >= MAX_RETRY_ATTEMPTS {
                    return Err(PluginError::Channel(
                        "WhatsApp API rate limit exceeded after max retries".into(),
                    ));
                }
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt));
                warn!(
                    attempt,
                    backoff_ms = backoff.as_millis(),
                    "WhatsApp API 429 rate limited, backing off"
                );
                tokio::time::sleep(backoff).await;
                continue;
            }

            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(PluginError::Channel(format!(
                "WhatsApp API error {status}: {error_body}"
            )));
        }
    }
}
```

#### Task E3.3: WhatsApp Channel (ChannelAdapter impl)

```rust
// crates/clawft-channels/src/whatsapp/channel.rs

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use clawft_plugin::{ChannelAdapter, ChannelAdapterHost, MessagePayload, PluginError};

use super::api::WhatsAppApiClient;
use super::types::{WebhookNotification, WhatsAppConfig};

/// WhatsApp channel adapter using the WhatsApp Cloud API.
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
    api: WhatsAppApiClient,
    status: Arc<RwLock<ChannelStatus>>,
}

/// Internal status tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ChannelStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

impl WhatsAppChannel {
    pub fn new(config: WhatsAppConfig, access_token: String) -> Self {
        let api = WhatsAppApiClient::new(
            config.phone_number_id.clone(),
            access_token,
            config.rate_limit_per_second,
        );
        Self {
            config,
            api,
            status: Arc::new(RwLock::new(ChannelStatus::Stopped)),
        }
    }

    /// Check if a sender phone number is allowed.
    fn is_sender_allowed(&self, sender: &str) -> bool {
        self.config.allowed_senders.is_empty()
            || self.config.allowed_senders.iter().any(|s| s == sender)
    }

    /// Process an inbound webhook notification.
    async fn process_webhook(
        &self,
        notification: WebhookNotification,
        host: &Arc<dyn ChannelAdapterHost>,
    ) -> Result<(), PluginError> {
        for entry in &notification.entry {
            for change in &entry.changes {
                if change.field != "messages" {
                    continue;
                }
                for message in &change.value.messages {
                    if !self.is_sender_allowed(&message.from) {
                        warn!(
                            sender = %message.from,
                            "message from disallowed sender, ignoring"
                        );
                        continue;
                    }

                    let content = match &message.text {
                        Some(text) => text.body.clone(),
                        None => {
                            debug!(
                                message_type = %message.message_type,
                                "skipping non-text message"
                            );
                            continue;
                        }
                    };

                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "whatsapp_message_id".into(),
                        serde_json::Value::String(message.id.clone()),
                    );
                    metadata.insert(
                        "timestamp".into(),
                        serde_json::Value::String(message.timestamp.clone()),
                    );
                    metadata.insert(
                        "phone_number_id".into(),
                        serde_json::Value::String(
                            change.value.metadata.phone_number_id.clone(),
                        ),
                    );

                    host.deliver_inbound(
                        "whatsapp",
                        &message.from,
                        &message.from, // chat_id = sender phone for WhatsApp
                        MessagePayload::Text(content),
                        metadata,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for WhatsAppChannel {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn display_name(&self) -> &str {
        "WhatsApp"
    }

    fn supports_threads(&self) -> bool {
        false
    }

    fn supports_media(&self) -> bool {
        true // WhatsApp supports images, audio, video, documents
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        *self.status.write().await = ChannelStatus::Starting;
        info!(
            port = self.config.webhook_port,
            "WhatsApp channel starting webhook server"
        );

        // Bind the webhook HTTP server.
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.config.webhook_port));
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            PluginError::Channel(format!("failed to bind webhook port {}: {e}", self.config.webhook_port))
        })?;

        *self.status.write().await = ChannelStatus::Running;
        info!("WhatsApp webhook server listening on {}", addr);

        // Accept connections until cancellation.
        // NOTE: Full HTTP server implementation (using hyper or axum) will parse
        // GET requests for webhook verification and POST requests for message delivery.
        // The webhook verification handler must:
        //   1. Check hub.mode == "subscribe"
        //   2. Compare hub.verify_token against the configured verify_token (from SecretRef)
        //   3. Return hub.challenge as the response body
        //
        // The message handler must:
        //   1. Parse the JSON body as WebhookNotification
        //   2. Call self.process_webhook(notification, &host)
        //   3. Return 200 OK

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("WhatsApp channel received cancellation");
                    *self.status.write().await = ChannelStatus::Stopped;
                    return Ok(());
                }
                result = listener.accept() => {
                    match result {
                        Ok((_stream, peer)) => {
                            debug!(peer = %peer, "accepted webhook connection");
                            // TODO: Dispatch to HTTP handler (hyper/axum)
                        }
                        Err(e) => {
                            error!(error = %e, "webhook accept error");
                        }
                    }
                }
            }
        }
    }

    async fn send(
        &self,
        target: &str,
        payload: MessagePayload,
        _metadata: HashMap<String, serde_json::Value>,
    ) -> Result<String, PluginError> {
        match payload {
            MessagePayload::Text(text) => self.api.send_text(target, &text).await,
            _ => Err(PluginError::Channel(
                "WhatsApp channel currently supports text messages only".into(),
            )),
        }
    }
}
```

#### Task E3.4: WhatsApp Factory

```rust
// crates/clawft-channels/src/whatsapp/factory.rs

use std::sync::Arc;

use clawft_plugin::{ChannelAdapter, PluginError};

use super::channel::WhatsAppChannel;
use super::types::WhatsAppConfig;

/// Factory for creating WhatsApp channel adapters from JSON configuration.
pub struct WhatsAppChannelFactory;

impl WhatsAppChannelFactory {
    pub fn channel_name(&self) -> &str {
        "whatsapp"
    }

    pub fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn ChannelAdapter>, PluginError> {
        let wa_config: WhatsAppConfig = serde_json::from_value(config.clone())
            .map_err(|e| PluginError::Channel(format!("invalid whatsapp config: {e}")))?;

        // Resolve access token from environment (SecretRef pattern).
        let access_token = std::env::var(&wa_config.access_token_env).map_err(|_| {
            PluginError::Channel(format!(
                "WhatsApp access_token_env '{}' is not set",
                wa_config.access_token_env
            ))
        })?;

        // Validate verify token is also resolvable (fail fast).
        std::env::var(&wa_config.verify_token_env).map_err(|_| {
            PluginError::Channel(format!(
                "WhatsApp verify_token_env '{}' is not set",
                wa_config.verify_token_env
            ))
        })?;

        Ok(Arc::new(WhatsAppChannel::new(wa_config, access_token)))
    }
}
```

#### Task E3.5: WhatsApp Module

```rust
// crates/clawft-channels/src/whatsapp/mod.rs

pub mod api;
pub mod channel;
pub mod factory;
pub mod types;

pub use channel::WhatsAppChannel;
pub use factory::WhatsAppChannelFactory;
```

---

### E4: Signal Channel

**Transport:** `signal-cli` subprocess (JSON-RPC mode)
**Auth:** Local (linked device via `signal-cli link`)
**New Files:** `crates/clawft-channels/src/signal/{mod.rs, channel.rs, subprocess.rs, types.rs, factory.rs}`
**Feature Flag:** `signal`

#### Task E4.1: Signal Types

```rust
// crates/clawft-channels/src/signal/types.rs

use serde::{Deserialize, Serialize};

/// JSON-RPC request sent to signal-cli.
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

impl JsonRpcRequest {
    pub fn new(method: &str, params: serde_json::Value, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id,
        }
    }
}

/// JSON-RPC response from signal-cli.
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    /// For notifications (no id), the method name.
    pub method: Option<String>,
    /// For notifications, the params.
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Inbound message extracted from a signal-cli notification.
#[derive(Debug)]
pub struct SignalInboundMessage {
    pub sender: String,
    pub group_id: Option<String>,
    pub body: String,
    pub timestamp: u64,
}

/// Signal channel configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SignalConfig {
    /// Path to the signal-cli binary.
    #[serde(default = "default_signal_cli_path")]
    pub signal_cli_path: String,
    /// The phone number this Signal account is registered to.
    pub account: String,
    /// Health check interval in seconds.
    #[serde(default = "default_health_interval")]
    pub health_interval_secs: u64,
    /// Kill timeout for hung subprocesses in seconds.
    #[serde(default = "default_kill_timeout")]
    pub kill_timeout_secs: u64,
    /// Maximum number of automatic restarts before giving up.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    /// Optional allowed sender numbers (empty = all allowed).
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}

fn default_signal_cli_path() -> String {
    "signal-cli".into()
}

fn default_health_interval() -> u64 {
    30
}

fn default_kill_timeout() -> u64 {
    30
}

fn default_max_restarts() -> u32 {
    5
}
```

#### Task E4.2: Signal Subprocess Management (SECURITY-CRITICAL)

```rust
// crates/clawft-channels/src/signal/subprocess.rs

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, error, info, warn};

use clawft_plugin::PluginError;

use super::types::{JsonRpcRequest, JsonRpcResponse, SignalConfig};

/// Manages the `signal-cli` subprocess lifecycle.
///
/// SECURITY: All subprocess arguments are sanitized to prevent command injection.
/// Never format user input directly into command strings. Use the builder pattern
/// through the public API methods exclusively.
pub struct SignalSubprocess {
    child: Child,
    pid: u32,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    health_interval: Duration,
    kill_timeout: Duration,
    next_request_id: u64,
}

/// Sanitize a string argument for use with signal-cli.
///
/// Rejects any string containing shell metacharacters that could enable
/// command injection. This is a defense-in-depth measure -- arguments are
/// passed via the `Command` API (not shell) but we still validate.
fn sanitize_signal_arg(arg: &str) -> Result<String, PluginError> {
    if arg.chars().any(|c| matches!(c, '|' | '&' | ';' | '$' | '`' | '\'' | '"' | '\\' | '\n' | '\r' | '\0')) {
        return Err(PluginError::Channel(format!(
            "unsafe characters in signal-cli argument: {:?}",
            arg
        )));
    }
    if arg.is_empty() {
        return Err(PluginError::Channel(
            "empty signal-cli argument".into(),
        ));
    }
    Ok(arg.to_string())
}

impl SignalSubprocess {
    /// Spawn a new `signal-cli jsonRpc` subprocess.
    ///
    /// SECURITY: The `config.signal_cli_path` and `config.account` values are
    /// sanitized before being passed to `Command::new()`. Arguments are passed
    /// via the array API (NOT shell expansion).
    pub async fn spawn(config: &SignalConfig) -> Result<Self, PluginError> {
        let cli_path = sanitize_signal_arg(&config.signal_cli_path)?;
        let account = sanitize_signal_arg(&config.account)?;

        info!(
            cli_path = %cli_path,
            account = %account,
            "spawning signal-cli subprocess"
        );

        let mut child = Command::new(&cli_path)
            .arg("--account")
            .arg(&account)
            .arg("jsonRpc")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // Do NOT use shell -- pass args directly to avoid injection.
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| PluginError::Channel(format!(
                "failed to spawn signal-cli at '{}': {e}", cli_path
            )))?;

        let pid = child.id().unwrap_or(0);
        let stdin = child.stdin.take().ok_or_else(|| {
            PluginError::Channel("failed to capture signal-cli stdin".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            PluginError::Channel("failed to capture signal-cli stdout".into())
        })?;

        info!(pid, "signal-cli subprocess started");

        Ok(Self {
            child,
            pid,
            stdin,
            stdout: BufReader::new(stdout),
            health_interval: Duration::from_secs(config.health_interval_secs),
            kill_timeout: Duration::from_secs(config.kill_timeout_secs),
            next_request_id: 1,
        })
    }

    /// Send a JSON-RPC request to the subprocess.
    pub async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<u64, PluginError> {
        let id = self.next_request_id;
        self.next_request_id += 1;

        let request = JsonRpcRequest::new(method, params, id);
        let mut json = serde_json::to_string(&request)
            .map_err(|e| PluginError::Channel(format!("JSON-RPC serialize error: {e}")))?;
        json.push('\n');

        self.stdin.write_all(json.as_bytes()).await.map_err(|e| {
            PluginError::Channel(format!("failed to write to signal-cli stdin: {e}"))
        })?;
        self.stdin.flush().await.map_err(|e| {
            PluginError::Channel(format!("failed to flush signal-cli stdin: {e}"))
        })?;

        debug!(id, method, "sent JSON-RPC request to signal-cli");
        Ok(id)
    }

    /// Read the next JSON-RPC response or notification from stdout.
    /// Returns `None` if stdout is closed (subprocess exited).
    pub async fn read_response(&mut self) -> Result<Option<JsonRpcResponse>, PluginError> {
        let mut line = String::new();
        let bytes_read = self.stdout.read_line(&mut line).await.map_err(|e| {
            PluginError::Channel(format!("failed to read from signal-cli stdout: {e}"))
        })?;

        if bytes_read == 0 {
            return Ok(None); // EOF -- subprocess exited
        }

        let response: JsonRpcResponse = serde_json::from_str(line.trim()).map_err(|e| {
            PluginError::Channel(format!(
                "failed to parse signal-cli JSON-RPC response: {e}, raw: {}",
                line.trim()
            ))
        })?;

        Ok(Some(response))
    }

    /// Check if the subprocess is still alive.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Get the subprocess PID.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Kill the subprocess with a timeout.
    /// First sends SIGTERM, waits up to `kill_timeout`, then sends SIGKILL.
    pub async fn kill(&mut self) -> Result<(), PluginError> {
        info!(pid = self.pid, "killing signal-cli subprocess");

        // Try graceful shutdown first.
        if let Err(e) = self.child.kill().await {
            warn!(
                pid = self.pid,
                error = %e,
                "failed to kill signal-cli subprocess (may already be dead)"
            );
        }

        // Wait for process to exit with timeout.
        match tokio::time::timeout(self.kill_timeout, self.child.wait()).await {
            Ok(Ok(status)) => {
                info!(pid = self.pid, status = %status, "signal-cli subprocess exited");
            }
            Ok(Err(e)) => {
                error!(pid = self.pid, error = %e, "error waiting for signal-cli exit");
            }
            Err(_) => {
                error!(
                    pid = self.pid,
                    "signal-cli subprocess did not exit within kill timeout"
                );
                // Process will be killed on drop via kill_on_drop(true).
            }
        }

        Ok(())
    }
}
```

#### Task E4.3: Signal Channel (ChannelAdapter impl)

```rust
// crates/clawft-channels/src/signal/channel.rs

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use clawft_plugin::{ChannelAdapter, ChannelAdapterHost, MessagePayload, PluginError};

use super::subprocess::SignalSubprocess;
use super::types::{SignalConfig, SignalInboundMessage};

/// Signal channel adapter using `signal-cli` subprocess.
pub struct SignalChannel {
    config: SignalConfig,
    subprocess: Mutex<Option<SignalSubprocess>>,
    status: Arc<RwLock<ChannelStatus>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChannelStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

impl SignalChannel {
    pub fn new(config: SignalConfig) -> Self {
        Self {
            config,
            subprocess: Mutex::new(None),
            status: Arc::new(RwLock::new(ChannelStatus::Stopped)),
        }
    }

    /// Check if a sender is allowed.
    fn is_sender_allowed(&self, sender: &str) -> bool {
        self.config.allowed_senders.is_empty()
            || self.config.allowed_senders.iter().any(|s| s == sender)
    }

    /// Extract an inbound message from a JSON-RPC notification.
    fn extract_inbound(
        &self,
        params: &serde_json::Value,
    ) -> Option<SignalInboundMessage> {
        let envelope = params.get("envelope")?;
        let sender = envelope.get("source")?.as_str()?;
        let data_message = envelope.get("dataMessage")?;
        let body = data_message.get("message")?.as_str()?;
        let timestamp = data_message.get("timestamp")?.as_u64()?;
        let group_id = data_message
            .get("groupInfo")
            .and_then(|g| g.get("groupId"))
            .and_then(|id| id.as_str())
            .map(String::from);

        Some(SignalInboundMessage {
            sender: sender.to_string(),
            group_id,
            body: body.to_string(),
            timestamp,
        })
    }
}

#[async_trait]
impl ChannelAdapter for SignalChannel {
    fn name(&self) -> &str {
        "signal"
    }

    fn display_name(&self) -> &str {
        "Signal"
    }

    fn supports_threads(&self) -> bool {
        false
    }

    fn supports_media(&self) -> bool {
        true // Signal supports attachments
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        *self.status.write().await = ChannelStatus::Starting;
        info!("Signal channel starting");

        let mut restart_count = 0u32;

        loop {
            // Spawn subprocess.
            let mut proc = match SignalSubprocess::spawn(&self.config).await {
                Ok(p) => p,
                Err(e) => {
                    error!(error = %e, "failed to spawn signal-cli");
                    *self.status.write().await = ChannelStatus::Error(e.to_string());

                    restart_count += 1;
                    if restart_count > self.config.max_restarts {
                        return Err(PluginError::Channel(format!(
                            "signal-cli exceeded max restarts ({})", self.config.max_restarts
                        )));
                    }

                    tokio::select! {
                        _ = cancel.cancelled() => {
                            *self.status.write().await = ChannelStatus::Stopped;
                            return Ok(());
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => continue,
                    }
                }
            };

            *self.status.write().await = ChannelStatus::Running;
            info!(pid = proc.pid(), "signal-cli subprocess running");

            // Health check interval.
            let mut health_timer = tokio::time::interval(
                std::time::Duration::from_secs(self.config.health_interval_secs)
            );

            // Message receive loop.
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        info!("Signal channel received cancellation");
                        proc.kill().await?;
                        *self.subprocess.lock().await = None;
                        *self.status.write().await = ChannelStatus::Stopped;
                        return Ok(());
                    }
                    response = proc.read_response() => {
                        match response {
                            Ok(Some(resp)) => {
                                // Handle notification (inbound message).
                                if resp.method.as_deref() == Some("receive") {
                                    if let Some(ref params) = resp.params {
                                        if let Some(msg) = self.extract_inbound(params) {
                                            if !self.is_sender_allowed(&msg.sender) {
                                                warn!(
                                                    sender = %msg.sender,
                                                    "message from disallowed sender"
                                                );
                                                continue;
                                            }

                                            let chat_id = msg.group_id
                                                .as_deref()
                                                .unwrap_or(&msg.sender);

                                            let mut metadata = HashMap::new();
                                            metadata.insert(
                                                "timestamp".into(),
                                                serde_json::Value::Number(msg.timestamp.into()),
                                            );
                                            if let Some(ref gid) = msg.group_id {
                                                metadata.insert(
                                                    "group_id".into(),
                                                    serde_json::Value::String(gid.clone()),
                                                );
                                            }

                                            if let Err(e) = host.deliver_inbound(
                                                "signal",
                                                &msg.sender,
                                                chat_id,
                                                MessagePayload::Text(msg.body),
                                                metadata,
                                            ).await {
                                                error!(
                                                    error = %e,
                                                    "failed to deliver Signal inbound message"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                // Subprocess exited.
                                warn!(pid = proc.pid(), "signal-cli subprocess exited");
                                break;
                            }
                            Err(e) => {
                                error!(error = %e, "signal-cli read error");
                                break;
                            }
                        }
                    }
                    _ = health_timer.tick() => {
                        if !proc.is_alive() {
                            warn!(pid = proc.pid(), "signal-cli health check failed");
                            break;
                        }
                        debug!(pid = proc.pid(), "signal-cli health check passed");
                    }
                }
            }

            // Subprocess exited -- attempt restart.
            *self.status.write().await = ChannelStatus::Error("subprocess exited".into());
            restart_count += 1;
            if restart_count > self.config.max_restarts {
                return Err(PluginError::Channel(format!(
                    "signal-cli exceeded max restarts ({})", self.config.max_restarts
                )));
            }

            warn!(
                restart_count,
                max_restarts = self.config.max_restarts,
                "restarting signal-cli subprocess"
            );

            tokio::select! {
                _ = cancel.cancelled() => {
                    *self.status.write().await = ChannelStatus::Stopped;
                    return Ok(());
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {}
            }
        }
    }

    async fn send(
        &self,
        target: &str,
        payload: MessagePayload,
        _metadata: HashMap<String, serde_json::Value>,
    ) -> Result<String, PluginError> {
        let text = match payload {
            MessagePayload::Text(t) => t,
            _ => {
                return Err(PluginError::Channel(
                    "Signal channel currently supports text messages only".into(),
                ));
            }
        };

        let mut proc_guard = self.subprocess.lock().await;
        let proc = proc_guard.as_mut().ok_or_else(|| {
            PluginError::Channel("signal-cli subprocess not running".into())
        })?;

        // SECURITY: target is used as a JSON value, not as a shell argument.
        // The subprocess.send_request method writes JSON to stdin, not shell commands.
        let params = serde_json::json!({
            "recipient": [target],
            "message": text,
        });

        let request_id = proc.send_request("send", params).await?;
        Ok(format!("signal-rpc-{}", request_id))
    }
}
```

#### Task E4.4: Signal Factory

```rust
// crates/clawft-channels/src/signal/factory.rs

use std::sync::Arc;

use clawft_plugin::{ChannelAdapter, PluginError};

use super::channel::SignalChannel;
use super::types::SignalConfig;

/// Factory for creating Signal channel adapters from JSON configuration.
pub struct SignalChannelFactory;

impl SignalChannelFactory {
    pub fn channel_name(&self) -> &str {
        "signal"
    }

    pub fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn ChannelAdapter>, PluginError> {
        let signal_config: SignalConfig = serde_json::from_value(config.clone())
            .map_err(|e| PluginError::Channel(format!("invalid signal config: {e}")))?;

        // Validate that the signal-cli path does not contain shell metacharacters.
        if signal_config.signal_cli_path.contains(|c: char| {
            matches!(c, '|' | '&' | ';' | '$' | '`' | '\'' | '"' | '\\' | '\n' | '\r')
        }) {
            return Err(PluginError::Channel(
                "signal_cli_path contains unsafe characters".into(),
            ));
        }

        Ok(Arc::new(SignalChannel::new(signal_config)))
    }
}
```

#### Task E4.5: Signal Module

```rust
// crates/clawft-channels/src/signal/mod.rs

pub mod channel;
pub mod factory;
pub mod subprocess;
pub mod types;

pub use channel::SignalChannel;
pub use factory::SignalChannelFactory;
```

---

### E5: Matrix Channel

**Transport:** Matrix Client-Server API via `matrix-sdk`
**Auth:** Access token or SSO
**New Files:** `crates/clawft-channels/src/matrix/{mod.rs, channel.rs, api.rs, types.rs, factory.rs}`
**Feature Flag:** `matrix`
**Key Dependency:** `matrix-sdk` crate

#### Task E5.1: Matrix Types

```rust
// crates/clawft-channels/src/matrix/types.rs

use serde::Deserialize;

/// Matrix channel configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct MatrixConfig {
    /// Matrix homeserver URL (e.g., "https://matrix.org").
    pub homeserver_url: String,
    /// Environment variable name for the access token.
    pub access_token_env: String,
    /// Matrix user ID (e.g., "@bot:matrix.org").
    pub user_id: String,
    /// Rooms to auto-join on startup.
    #[serde(default)]
    pub auto_join_rooms: Vec<String>,
    /// Whether to auto-join rooms when invited.
    #[serde(default)]
    pub auto_join_on_invite: bool,
    /// Optional allowed sender user IDs (empty = all allowed).
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}
```

#### Task E5.2: Matrix API Wrapper

```rust
// crates/clawft-channels/src/matrix/api.rs

use tracing::{debug, info};

use clawft_plugin::PluginError;

/// Wrapper around `matrix-sdk::Client` for simplified room and message operations.
///
/// NOTE: Actual implementation depends on the `matrix-sdk` crate API.
/// This wrapper isolates SDK-specific details from the channel adapter.
pub struct MatrixApiClient {
    // client: matrix_sdk::Client,  // Uncomment when matrix-sdk is added
    homeserver_url: String,
    access_token: String,
    user_id: String,
}

impl MatrixApiClient {
    pub async fn new(
        homeserver_url: &str,
        access_token: &str,
        user_id: &str,
    ) -> Result<Self, PluginError> {
        // TODO: Initialize matrix_sdk::Client with homeserver URL and access token.
        // let client = matrix_sdk::Client::builder()
        //     .homeserver_url(homeserver_url)
        //     .build()
        //     .await
        //     .map_err(|e| PluginError::Channel(format!("Matrix SDK init error: {e}")))?;
        //
        // client.restore_login(matrix_sdk::Session { ... }).await?;

        info!(homeserver = %homeserver_url, user = %user_id, "Matrix API client created");

        Ok(Self {
            homeserver_url: homeserver_url.into(),
            access_token: access_token.into(),
            user_id: user_id.into(),
        })
    }

    /// Join a room by room ID or alias.
    pub async fn join_room(&self, room_id: &str) -> Result<(), PluginError> {
        debug!(room = %room_id, "joining Matrix room");
        // TODO: self.client.join_room_by_id_or_alias(room_id.try_into()?, &[]).await?;
        Ok(())
    }

    /// Send a text message to a room.
    pub async fn send_text(
        &self,
        room_id: &str,
        body: &str,
    ) -> Result<String, PluginError> {
        debug!(room = %room_id, "sending text message to Matrix room");
        // TODO: Use matrix_sdk to send message.
        // let room = self.client.get_joined_room(room_id.try_into()?)?;
        // let response = room.send(RoomMessageEventContent::text_plain(body), None).await?;
        // Ok(response.event_id.to_string())
        Ok(format!("$matrix-msg-{}", uuid::Uuid::new_v4()))
    }

    /// Start sync loop, returning events via callback.
    pub async fn sync_loop<F>(&self, _on_message: F) -> Result<(), PluginError>
    where
        F: Fn(String, String, String, String) + Send + Sync + 'static,
    {
        // TODO: Use matrix_sdk sync with event handler.
        // self.client.add_event_handler(|event: SyncRoomMessageEvent, room: Room| async {
        //     on_message(room_id, sender, body, event_id);
        // });
        // self.client.sync(SyncSettings::default()).await?;
        Ok(())
    }
}
```

#### Task E5.3: Matrix Channel (ChannelAdapter impl)

```rust
// crates/clawft-channels/src/matrix/channel.rs

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use clawft_plugin::{ChannelAdapter, ChannelAdapterHost, MessagePayload, PluginError};

use super::api::MatrixApiClient;
use super::types::MatrixConfig;

/// Matrix channel adapter using the Matrix Client-Server API.
pub struct MatrixChannel {
    config: MatrixConfig,
    api: RwLock<Option<MatrixApiClient>>,
    status: Arc<RwLock<ChannelStatus>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChannelStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

impl MatrixChannel {
    pub fn new(config: MatrixConfig) -> Self {
        Self {
            config,
            api: RwLock::new(None),
            status: Arc::new(RwLock::new(ChannelStatus::Stopped)),
        }
    }

    fn is_sender_allowed(&self, sender: &str) -> bool {
        self.config.allowed_senders.is_empty()
            || self.config.allowed_senders.iter().any(|s| s == sender)
    }
}

#[async_trait]
impl ChannelAdapter for MatrixChannel {
    fn name(&self) -> &str {
        "matrix"
    }

    fn display_name(&self) -> &str {
        "Matrix"
    }

    fn supports_threads(&self) -> bool {
        true // Matrix supports threads via reply chains
    }

    fn supports_media(&self) -> bool {
        true
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        *self.status.write().await = ChannelStatus::Starting;
        info!("Matrix channel starting");

        // Resolve access token from environment (SecretRef pattern).
        let access_token = std::env::var(&self.config.access_token_env).map_err(|_| {
            PluginError::Channel(format!(
                "Matrix access_token_env '{}' is not set",
                self.config.access_token_env
            ))
        })?;

        // Initialize the Matrix API client.
        let api = MatrixApiClient::new(
            &self.config.homeserver_url,
            &access_token,
            &self.config.user_id,
        )
        .await?;

        // Auto-join configured rooms.
        for room in &self.config.auto_join_rooms {
            if let Err(e) = api.join_room(room).await {
                warn!(room = %room, error = %e, "failed to auto-join Matrix room");
            }
        }

        *self.api.write().await = Some(api);
        *self.status.write().await = ChannelStatus::Running;
        info!("Matrix channel running");

        // Run sync loop until cancellation.
        // NOTE: The actual implementation will use the matrix-sdk sync mechanism
        // with an event handler that calls host.deliver_inbound for each message.
        cancel.cancelled().await;

        *self.status.write().await = ChannelStatus::Stopped;
        info!("Matrix channel stopped");
        Ok(())
    }

    async fn send(
        &self,
        target: &str,
        payload: MessagePayload,
        _metadata: HashMap<String, serde_json::Value>,
    ) -> Result<String, PluginError> {
        let text = match payload {
            MessagePayload::Text(t) => t,
            _ => {
                return Err(PluginError::Channel(
                    "Matrix channel currently supports text messages only".into(),
                ));
            }
        };

        let api_guard = self.api.read().await;
        let api = api_guard.as_ref().ok_or_else(|| {
            PluginError::Channel("Matrix API client not initialized".into())
        })?;

        api.send_text(target, &text).await
    }
}
```

#### Task E5.4: Matrix Factory

```rust
// crates/clawft-channels/src/matrix/factory.rs

use std::sync::Arc;

use clawft_plugin::{ChannelAdapter, PluginError};

use super::channel::MatrixChannel;
use super::types::MatrixConfig;

/// Factory for creating Matrix channel adapters from JSON configuration.
pub struct MatrixChannelFactory;

impl MatrixChannelFactory {
    pub fn channel_name(&self) -> &str {
        "matrix"
    }

    pub fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn ChannelAdapter>, PluginError> {
        let matrix_config: MatrixConfig = serde_json::from_value(config.clone())
            .map_err(|e| PluginError::Channel(format!("invalid matrix config: {e}")))?;

        // Validate homeserver URL is well-formed.
        if !matrix_config.homeserver_url.starts_with("https://")
            && !matrix_config.homeserver_url.starts_with("http://")
        {
            return Err(PluginError::Channel(
                "Matrix homeserver_url must start with https:// or http://".into(),
            ));
        }

        // Validate access_token_env is set (fail fast).
        std::env::var(&matrix_config.access_token_env).map_err(|_| {
            PluginError::Channel(format!(
                "Matrix access_token_env '{}' is not set",
                matrix_config.access_token_env
            ))
        })?;

        Ok(Arc::new(MatrixChannel::new(matrix_config)))
    }
}
```

#### Task E5.5: Matrix Module

```rust
// crates/clawft-channels/src/matrix/mod.rs

pub mod api;
pub mod channel;
pub mod factory;
pub mod types;

pub use channel::MatrixChannel;
pub use factory::MatrixChannelFactory;
```

---

### E5 (cont): IRC Channel

**Transport:** IRC protocol via `irc` crate
**Auth:** NickServ/SASL
**New Files:** `crates/clawft-channels/src/irc/{mod.rs, channel.rs, protocol.rs, types.rs, factory.rs}`
**Feature Flag:** `irc`
**Key Dependency:** `irc` crate

#### Task E5.6: IRC Types

```rust
// crates/clawft-channels/src/irc/types.rs

use serde::Deserialize;

/// IRC channel configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct IrcConfig {
    /// IRC server hostname.
    pub server: String,
    /// IRC server port.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Whether to use TLS.
    #[serde(default = "default_use_tls")]
    pub use_tls: bool,
    /// Bot nickname.
    pub nickname: String,
    /// IRC channels to join (e.g., ["#general", "#dev"]).
    #[serde(default)]
    pub channels: Vec<String>,
    /// Authentication method: "none", "nickserv", or "sasl".
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    /// Environment variable name for the auth password.
    #[serde(default)]
    pub password_env: Option<String>,
    /// Optional SASL username (defaults to nickname).
    #[serde(default)]
    pub sasl_username: Option<String>,
    /// Optional allowed sender nicknames (empty = all allowed).
    #[serde(default)]
    pub allowed_senders: Vec<String>,
    /// Reconnect delay in seconds.
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,
}

fn default_port() -> u16 {
    6697 // TLS default
}

fn default_use_tls() -> bool {
    true
}

fn default_auth_method() -> String {
    "none".into()
}

fn default_reconnect_delay() -> u64 {
    5
}
```

#### Task E5.7: IRC Protocol Wrapper

```rust
// crates/clawft-channels/src/irc/protocol.rs

use tracing::{debug, info};

use clawft_plugin::PluginError;

use super::types::IrcConfig;

/// IRC client wrapper around the `irc` crate.
///
/// NOTE: Actual implementation depends on the `irc` crate API.
/// This wrapper isolates IRC protocol details from the channel adapter.
pub struct IrcClient {
    // client: irc::client::Client,  // Uncomment when irc crate is added
    config: IrcConfig,
}

impl IrcClient {
    pub async fn connect(config: &IrcConfig) -> Result<Self, PluginError> {
        info!(
            server = %config.server,
            port = config.port,
            nickname = %config.nickname,
            "connecting to IRC server"
        );

        // TODO: Initialize irc::client::Client.
        // let irc_config = irc::client::data::Config {
        //     nickname: Some(config.nickname.clone()),
        //     server: Some(config.server.clone()),
        //     port: Some(config.port),
        //     use_tls: Some(config.use_tls),
        //     channels: config.channels.clone(),
        //     ..Default::default()
        // };
        // let client = irc::client::Client::from_config(irc_config).await
        //     .map_err(|e| PluginError::Channel(format!("IRC connect error: {e}")))?;
        // client.identify()
        //     .map_err(|e| PluginError::Channel(format!("IRC identify error: {e}")))?;

        Ok(Self {
            config: config.clone(),
        })
    }

    /// Authenticate with NickServ or SASL.
    pub async fn authenticate(&self) -> Result<(), PluginError> {
        match self.config.auth_method.as_str() {
            "nickserv" => {
                let password_env = self.config.password_env.as_deref().ok_or_else(|| {
                    PluginError::Channel("NickServ auth requires password_env".into())
                })?;
                let _password = std::env::var(password_env).map_err(|_| {
                    PluginError::Channel(format!(
                        "IRC password_env '{}' is not set", password_env
                    ))
                })?;
                // TODO: client.send_privmsg("NickServ", &format!("IDENTIFY {}", password))?;
                debug!("NickServ authentication sent");
            }
            "sasl" => {
                // SASL PLAIN authentication is handled during connection setup.
                debug!("SASL authentication configured");
            }
            "none" => {}
            other => {
                return Err(PluginError::Channel(format!(
                    "unknown IRC auth method: {}", other
                )));
            }
        }
        Ok(())
    }

    /// Send a PRIVMSG to a channel or user.
    pub async fn send_privmsg(
        &self,
        target: &str,
        message: &str,
    ) -> Result<(), PluginError> {
        debug!(target = %target, "sending IRC PRIVMSG");
        // TODO: self.client.send_privmsg(target, message)?;
        Ok(())
    }

    /// Start the message receive stream.
    pub async fn message_stream(&self) -> Result<(), PluginError> {
        // TODO: Use client.stream()? to get incoming messages.
        Ok(())
    }
}
```

#### Task E5.8: IRC Channel (ChannelAdapter impl)

```rust
// crates/clawft-channels/src/irc/channel.rs

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use clawft_plugin::{ChannelAdapter, ChannelAdapterHost, MessagePayload, PluginError};

use super::protocol::IrcClient;
use super::types::IrcConfig;

/// IRC channel adapter.
pub struct IrcChannel {
    config: IrcConfig,
    client: RwLock<Option<IrcClient>>,
    status: Arc<RwLock<ChannelStatus>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChannelStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

impl IrcChannel {
    pub fn new(config: IrcConfig) -> Self {
        Self {
            config,
            client: RwLock::new(None),
            status: Arc::new(RwLock::new(ChannelStatus::Stopped)),
        }
    }

    fn is_sender_allowed(&self, sender: &str) -> bool {
        self.config.allowed_senders.is_empty()
            || self.config.allowed_senders.iter().any(|s| s == sender)
    }
}

#[async_trait]
impl ChannelAdapter for IrcChannel {
    fn name(&self) -> &str {
        "irc"
    }

    fn display_name(&self) -> &str {
        "IRC"
    }

    fn supports_threads(&self) -> bool {
        false
    }

    fn supports_media(&self) -> bool {
        false // IRC is text-only
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        *self.status.write().await = ChannelStatus::Starting;
        info!("IRC channel starting");

        let mut reconnect_count = 0u32;

        loop {
            // Connect to IRC server.
            let irc_client = match IrcClient::connect(&self.config).await {
                Ok(c) => c,
                Err(e) => {
                    error!(error = %e, "failed to connect to IRC server");
                    *self.status.write().await = ChannelStatus::Error(e.to_string());

                    tokio::select! {
                        _ = cancel.cancelled() => {
                            *self.status.write().await = ChannelStatus::Stopped;
                            return Ok(());
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(
                            self.config.reconnect_delay_secs
                        )) => continue,
                    }
                }
            };

            // Authenticate.
            if let Err(e) = irc_client.authenticate().await {
                warn!(error = %e, "IRC authentication failed");
            }

            *self.client.write().await = Some(irc_client);
            *self.status.write().await = ChannelStatus::Running;
            info!("IRC channel connected and running");

            // NOTE: The actual message receive loop will use the irc crate's
            // Stream to process incoming PRIVMSG events, filter by allowed senders,
            // and deliver to host.deliver_inbound.

            // Wait for cancellation or disconnect.
            cancel.cancelled().await;

            *self.status.write().await = ChannelStatus::Stopped;
            info!("IRC channel stopped");
            return Ok(());
        }
    }

    async fn send(
        &self,
        target: &str,
        payload: MessagePayload,
        _metadata: HashMap<String, serde_json::Value>,
    ) -> Result<String, PluginError> {
        let text = match payload {
            MessagePayload::Text(t) => t,
            _ => {
                return Err(PluginError::Channel(
                    "IRC channel supports text messages only".into(),
                ));
            }
        };

        let client_guard = self.client.read().await;
        let client = client_guard.as_ref().ok_or_else(|| {
            PluginError::Channel("IRC client not connected".into())
        })?;

        client.send_privmsg(target, &text).await?;

        // IRC does not return message IDs.
        Ok(format!("irc-{}-{}", target, chrono::Utc::now().timestamp_millis()))
    }
}
```

#### Task E5.9: IRC Factory

```rust
// crates/clawft-channels/src/irc/factory.rs

use std::sync::Arc;

use clawft_plugin::{ChannelAdapter, PluginError};

use super::channel::IrcChannel;
use super::types::IrcConfig;

/// Factory for creating IRC channel adapters from JSON configuration.
pub struct IrcChannelFactory;

impl IrcChannelFactory {
    pub fn channel_name(&self) -> &str {
        "irc"
    }

    pub fn build(
        &self,
        config: &serde_json::Value,
    ) -> Result<Arc<dyn ChannelAdapter>, PluginError> {
        let irc_config: IrcConfig = serde_json::from_value(config.clone())
            .map_err(|e| PluginError::Channel(format!("invalid irc config: {e}")))?;

        // Validate auth configuration.
        if irc_config.auth_method == "nickserv" || irc_config.auth_method == "sasl" {
            let password_env = irc_config.password_env.as_deref().ok_or_else(|| {
                PluginError::Channel(format!(
                    "IRC auth method '{}' requires password_env", irc_config.auth_method
                ))
            })?;
            std::env::var(password_env).map_err(|_| {
                PluginError::Channel(format!(
                    "IRC password_env '{}' is not set", password_env
                ))
            })?;
        }

        Ok(Arc::new(IrcChannel::new(irc_config)))
    }
}
```

#### Task E5.10: IRC Module

```rust
// crates/clawft-channels/src/irc/mod.rs

pub mod channel;
pub mod factory;
pub mod protocol;
pub mod types;

pub use channel::IrcChannel;
pub use factory::IrcChannelFactory;
```

---

## 4. Security Requirements

### Critical: Subprocess Sanitization (E4)

Signal channel uses `signal-cli` as a subprocess. All arguments MUST be sanitized to prevent command injection.

| Requirement | Implementation |
|-------------|---------------|
| No shell metacharacters in subprocess arguments | `sanitize_signal_arg()` rejects `\| & ; $ \` ' " \ \n \r \0` |
| Arguments passed via array API, not shell | `Command::new(path).arg(...)` -- never `Command::new("sh").arg("-c")` |
| Kill on drop | `Command::kill_on_drop(true)` prevents orphan processes |
| Config path validation | Factory validates `signal_cli_path` at construction time |
| Empty argument rejection | `sanitize_signal_arg()` rejects empty strings |

### Credential Security (All Channels)

| Requirement | Implementation |
|-------------|---------------|
| No plaintext secrets in config structs | All credential fields use `*_env` pattern (SecretRef from A4) |
| WhatsApp `access_token` via env var | `access_token_env` field, resolved in factory |
| WhatsApp `verify_token` via env var | `verify_token_env` field, resolved in factory |
| Matrix `access_token` via env var | `access_token_env` field, resolved in factory |
| IRC password via env var | `password_env` field, resolved in factory |
| Fail-fast on missing secrets | Factory validates env vars exist at construction time |

### Input Validation

| Channel | Validation |
|---------|-----------|
| WhatsApp | Sender phone number checked against `allowed_senders` |
| Signal | Sender checked against `allowed_senders`; subprocess args sanitized |
| Matrix | Sender checked against `allowed_senders`; homeserver URL validated |
| IRC | Sender nickname checked against `allowed_senders`; auth method validated |

---

## 5. Tests Required

| Test ID | Channel | Description | Type |
|---------|---------|-------------|------|
| E3-T1 | WhatsApp | Factory builds channel from valid config | Unit |
| E3-T2 | WhatsApp | Factory rejects config with missing `access_token_env` | Unit |
| E3-T3 | WhatsApp | Factory rejects config with missing `verify_token_env` | Unit |
| E3-T4 | WhatsApp | `process_webhook` delivers text message to host | Unit |
| E3-T5 | WhatsApp | `process_webhook` skips non-text messages | Unit |
| E3-T6 | WhatsApp | `is_sender_allowed` respects `allowed_senders` list | Unit |
| E3-T7 | WhatsApp | API client retries on 429 with exponential backoff | Unit |
| E3-T8 | WhatsApp | API client fails after `MAX_RETRY_ATTEMPTS` | Unit |
| E3-T9 | WhatsApp | `send` returns message ID on success | Unit |
| E4-T1 | Signal | `sanitize_signal_arg` rejects shell metacharacters | Unit |
| E4-T2 | Signal | `sanitize_signal_arg` rejects empty strings | Unit |
| E4-T3 | Signal | `sanitize_signal_arg` accepts valid phone numbers | Unit |
| E4-T4 | Signal | `sanitize_signal_arg` accepts valid file paths (no metacharacters) | Unit |
| E4-T5 | Signal | Factory rejects config with unsafe `signal_cli_path` | Unit |
| E4-T6 | Signal | Factory builds channel from valid config | Unit |
| E4-T7 | Signal | `extract_inbound` parses valid notification | Unit |
| E4-T8 | Signal | `extract_inbound` returns None for malformed notification | Unit |
| E4-T9 | Signal | `is_sender_allowed` respects `allowed_senders` list | Unit |
| E4-T10 | Signal | Subprocess `is_alive` returns false after process exit | Integration |
| E4-T11 | Signal | Channel restarts subprocess on crash (up to `max_restarts`) | Integration |
| E5-T1 | Matrix | Factory builds channel from valid config | Unit |
| E5-T2 | Matrix | Factory rejects config with invalid `homeserver_url` | Unit |
| E5-T3 | Matrix | Factory rejects config with missing `access_token_env` | Unit |
| E5-T4 | Matrix | `is_sender_allowed` respects `allowed_senders` list | Unit |
| E5-T5 | Matrix | `send` returns error when API client not initialized | Unit |
| E5-T6 | IRC | Factory builds channel from valid config | Unit |
| E5-T7 | IRC | Factory rejects config with auth but missing `password_env` | Unit |
| E5-T8 | IRC | `is_sender_allowed` respects `allowed_senders` list | Unit |
| E5-T9 | IRC | `send` returns error when client not connected | Unit |
| E5-T10 | IRC | `send` returns error for non-text payload | Unit |
| REG-1 | All | All existing channel tests (Slack, Discord, Telegram) still pass | Regression |
| REG-2 | All | `cargo build -p clawft-channels` (no features) compiles without new channels | Regression |
| REG-3 | All | `cargo build -p clawft-channels --features whatsapp` compiles cleanly | Regression |
| REG-4 | All | `cargo build -p clawft-channels --features signal` compiles cleanly | Regression |
| REG-5 | All | `cargo build -p clawft-channels --features matrix` compiles cleanly | Regression |
| REG-6 | All | `cargo build -p clawft-channels --features irc` compiles cleanly | Regression |

---

## 6. Acceptance Criteria

- [ ] E3: WhatsApp channel sends and receives text messages via Cloud API
- [ ] E3: WhatsApp webhook verification uses verify_token from `SecretRef` (env var)
- [ ] E3: WhatsApp API client implements rate limiting with exponential backoff on 429
- [ ] E3: WhatsApp access token resolved from env var, never in plaintext config
- [ ] E4: Signal channel sends and receives messages via `signal-cli` subprocess
- [ ] E4: All subprocess arguments sanitized by `sanitize_signal_arg()`
- [ ] E4: `signal-cli` spawned via `Command` array API (no shell expansion)
- [ ] E4: Subprocess PID tracked with periodic health checks
- [ ] E4: Subprocess auto-restarts on crash (up to `max_restarts`)
- [ ] E4: Hung subprocesses killed after `kill_timeout_secs`
- [ ] E4: `kill_on_drop(true)` set to prevent orphan processes
- [ ] E5: Matrix channel joins rooms and sends/receives messages via Matrix SDK
- [ ] E5: Matrix access token resolved from env var
- [ ] E5: Matrix homeserver URL validated at factory construction
- [ ] E5: IRC channel connects, authenticates (NickServ/SASL), and sends/receives messages
- [ ] E5: IRC password resolved from env var
- [ ] All channels implement `ChannelAdapter` trait (NOT legacy `Channel`)
- [ ] All channels feature-gated in `Cargo.toml` and conditionally compiled
- [ ] All config credential fields use env var pattern (SecretRef from A4)
- [ ] All channels have sender allow-list support
- [ ] All existing channel tests pass (regression)
- [ ] `cargo build -p clawft-channels` compiles without new channel features

---

## 7. Risk Notes

| Risk | L | I | Score | Mitigation |
|------|---|---|-------|------------|
| WhatsApp Cloud API rate limits throttle message delivery | M | M | 4 | Rate limiter with `Semaphore` + exponential backoff on 429 + max retry limit + outbound message queue (future) |
| `signal-cli` subprocess management (zombie processes, crash recovery) | M | M | 4 | PID tracking + health checks at configurable interval + auto-restart with backoff + `kill_on_drop(true)` + `kill_timeout` for hung processes |
| `signal-cli` command injection via user-controlled arguments | L | H | 4 | `sanitize_signal_arg()` rejects shell metacharacters + `Command` array API (no shell) + config validation in factory |
| `matrix-sdk` API instability across versions | M | L | 3 | Pin `matrix-sdk` version in workspace `Cargo.toml` + wrap SDK calls in `MatrixApiClient` abstraction layer |
| IRC reconnection loops on persistent server failures | L | M | 3 | Configurable `reconnect_delay_secs` + backoff + circuit breaker (future enhancement) |
| Feature gate combinations cause conditional compilation errors | L | M | 3 | CI matrix testing: build with each feature individually and all features combined |

---

## 8. Dependencies Summary

### Upstream (this phase depends on)

| Dependency | Element/Phase | Required For | Status |
|------------|--------------|--------------|--------|
| `ChannelAdapter` trait | 04/C1 | All new channels implement this trait | Planning |
| `SecretRef` type | 03/A4 | Credential fields in config structs | Planning |
| Feature gate pattern | 03/A9 | Conditional compilation of new channels | Planning |

### Downstream (depends on this phase)

| Dependent | Element/Phase | Dependency |
|-----------|--------------|------------|
| C7 PluginHost Unification | 04/C7 | New channels must be loadable via unified PluginHost |

### New Crate Dependencies

| Crate | Feature Flag | Used By | Purpose |
|-------|-------------|---------|---------|
| `reqwest` | `whatsapp` | E3 | WhatsApp Cloud API REST calls (already a workspace dependency) |
| `matrix-sdk` | `matrix` | E5 | Matrix Client-Server API |
| `irc` | `irc` | E5 | IRC protocol client |

### Workspace `Cargo.toml` Changes

```toml
# Add to [workspace.dependencies]:
matrix-sdk = { version = "0.7", default-features = false, features = ["e2e-encryption", "native-tls"] }
irc = "1.0"
```
