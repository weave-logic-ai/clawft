# Technical Requirements Addendum: Workstreams D, E, F, H, K, L, M

> Additions to `02-technical-requirements.md` covering Pipeline & LLM Reliability,
> Channel Enhancements, Dev Tools, Memory & Workspace, Deployment, Multi-Agent,
> and Claude Flow Integration. Written at the same technical depth as the
> existing document (Rust code blocks, config examples, architecture diagrams,
> implementation notes).

---

## 12. Pipeline & LLM Reliability (Workstream D)

### 12.1 Parallel Tool Execution (D1)

**File**: `clawft-core/src/agent/loop_core.rs`

The current tool execution loop is sequential:

```rust
// BEFORE (sequential)
for tool_call in &tool_calls {
    let result = self.tools.execute(&tool_call.name, tool_call.input.clone()).await?;
    results.push(result);
}
```

Replace with `futures::future::join_all` for concurrent execution:

```rust
use futures::future::join_all;

// AFTER (parallel)
let futures: Vec<_> = tool_calls
    .iter()
    .map(|tc| {
        let tools = self.tools.clone();
        let name = tc.name.clone();
        let input = tc.input.clone();
        async move {
            let start = std::time::Instant::now();
            let result = tools.execute(&name, input).await;
            let elapsed = start.elapsed();
            tracing::debug!(tool = %name, elapsed_ms = elapsed.as_millis(), "tool executed");
            (tc.id.clone(), name, result)
        }
    })
    .collect();

let outcomes = join_all(futures).await;

// Build tool result messages preserving call order
let mut tool_messages = Vec::with_capacity(outcomes.len());
for (call_id, name, result) in outcomes {
    let content = match result {
        Ok(val) => val.to_string(),
        Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
    };
    // Enforce per-result size cap
    let content = if content.len() > MAX_TOOL_RESULT_BYTES {
        format!(
            "{{\"error\": \"Tool result truncated ({} bytes > {} limit)\", \"preview\": {:?}}}",
            content.len(),
            MAX_TOOL_RESULT_BYTES,
            &content[..256],
        )
    } else {
        content
    };
    tool_messages.push(LlmMessage {
        role: "tool".into(),
        content,
        tool_call_id: Some(call_id),
        tool_calls: None,
    });
}
```

**Safety constraint**: Tools that access shared mutable state (e.g. `write_file`
targeting the same path) must not produce data races. The `ToolRegistry` must
document thread-safety requirements on `Tool::execute`. File-writing tools should
acquire a per-path advisory lock via `tokio::sync::Mutex<HashMap<PathBuf, ()>>`.

**Dependency**: `futures` crate (add to workspace deps).

```toml
# Cargo.toml workspace addition
futures = "0.3"
```

### 12.2 Streaming Failover Correctness (D2)

**File**: `clawft-llm/src/failover.rs`

When a provider fails mid-stream, partial output has already been delivered via
`StreamCallback`. The failover controller must reset the stream state before
trying the next provider.

```rust
/// Manages stream failover state.
pub struct StreamFailoverController {
    /// Accumulated text from the current provider attempt.
    partial_buffer: String,
    /// Whether we have sent any chunks to the caller.
    chunks_delivered: bool,
}

impl StreamFailoverController {
    pub fn new() -> Self {
        Self {
            partial_buffer: String::new(),
            chunks_delivered: false,
        }
    }

    /// Record a chunk that was delivered to the caller.
    pub fn record_chunk(&mut self, chunk: &str) {
        self.partial_buffer.push_str(chunk);
        self.chunks_delivered = true;
    }

    /// Reset stream state for failover to next provider.
    ///
    /// Returns the partial text that was delivered (so the caller can
    /// optionally send a "reset" signal to downstream consumers).
    pub fn reset(&mut self) -> StreamResetInfo {
        let info = StreamResetInfo {
            partial_text: std::mem::take(&mut self.partial_buffer),
            had_output: self.chunks_delivered,
        };
        self.chunks_delivered = false;
        info
    }
}

/// Information about the partial stream that was discarded.
#[derive(Debug)]
pub struct StreamResetInfo {
    /// Text that was delivered before the failure.
    pub partial_text: String,
    /// Whether any output reached the caller.
    pub had_output: bool,
}
```

**Failover protocol for streaming**:

```
Provider A starts streaming
  -> chunks arrive, forwarded to callback
  -> Provider A fails mid-stream
  -> StreamFailoverController::reset() called
  -> If had_output: send a "[stream reset]" marker via callback
  -> Provider B starts fresh stream from the beginning
  -> All chunks from Provider B forwarded to callback
```

The `StreamCallback` type receives a special reset marker so channel adapters
can clear partial output (e.g. edit the Discord message, replace the Telegram
message). The marker format:

```rust
/// Sentinel value sent through StreamCallback when a stream resets.
pub const STREAM_RESET_MARKER: &str = "\x00__STREAM_RESET__\x00";
```

### 12.3 Structured Error Variants (D3)

**File**: `clawft-llm/src/retry.rs` (current string matching),
`clawft-types/src/error.rs` (new variants)

Replace fragile string-prefix matching with typed variants:

```rust
/// Errors from LLM provider interactions.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// HTTP transport failure (connection, DNS, TLS).
    #[error("transport error: {source}")]
    Transport {
        #[from]
        source: reqwest::Error,
    },

    /// Server returned an error status code.
    #[error("server error (HTTP {status}): {body}")]
    ServerError {
        /// HTTP status code (e.g. 500, 502, 503).
        status: u16,
        /// Response body (may be truncated).
        body: String,
        /// Whether the error is retryable based on status code.
        retryable: bool,
    },

    /// Rate limit exceeded (HTTP 429).
    #[error("rate limited: retry after {retry_after_ms:?}ms")]
    RateLimited {
        /// Suggested retry delay from `Retry-After` header.
        retry_after_ms: Option<u64>,
    },

    /// Authentication failure (HTTP 401/403).
    #[error("auth error (HTTP {status}): {message}")]
    AuthError {
        status: u16,
        message: String,
    },

    /// The response could not be parsed.
    #[error("invalid response: {reason}")]
    InvalidResponse { reason: String },

    /// Request was cancelled (e.g. by CancellationToken).
    #[error("request cancelled")]
    Cancelled,

    /// Context length exceeded (provider returned 400 with specific message).
    #[error("context length exceeded: max {max_tokens}, requested ~{requested_tokens}")]
    ContextLengthExceeded {
        max_tokens: usize,
        requested_tokens: usize,
    },
}

impl ProviderError {
    /// Whether this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Transport { .. } => true,
            Self::ServerError { retryable, .. } => *retryable,
            Self::RateLimited { .. } => true,
            Self::AuthError { .. } => false,
            Self::InvalidResponse { .. } => false,
            Self::Cancelled => false,
            Self::ContextLengthExceeded { .. } => false,
        }
    }

    /// Suggested retry delay in milliseconds.
    pub fn retry_delay_hint(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after_ms } => *retry_after_ms,
            Self::ServerError { status: 503, .. } => Some(1000),
            Self::ServerError { status: 502, .. } => Some(500),
            _ => None,
        }
    }
}
```

**HTTP status to variant mapping** (in response parser):

```rust
fn status_to_error(status: u16, body: String) -> ProviderError {
    match status {
        429 => {
            // Parse Retry-After header if available
            ProviderError::RateLimited { retry_after_ms: None }
        }
        401 | 403 => ProviderError::AuthError {
            status,
            message: body,
        },
        400 if body.contains("context_length") || body.contains("max_tokens") => {
            ProviderError::ContextLengthExceeded {
                max_tokens: 0,       // Parse from body
                requested_tokens: 0, // Parse from body
            }
        }
        s if s >= 500 => ProviderError::ServerError {
            status: s,
            body,
            retryable: matches!(s, 500 | 502 | 503 | 504),
        },
        _ => ProviderError::ServerError {
            status,
            body,
            retryable: false,
        },
    }
}
```

### 12.4 Configurable Retry Policy (D4)

**File**: `clawft-core/src/pipeline/llm_adapter.rs`

Current hardcoded values: 3 retries, fixed delay. Replace with configurable
policy:

```rust
/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial backoff delay in milliseconds.
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay in milliseconds.
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,

    /// Backoff multiplier (exponential factor).
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Add random jitter up to this percentage of the delay (0.0--1.0).
    #[serde(default = "default_jitter")]
    pub jitter: f64,

    /// HTTP status codes that are retryable (empty = use defaults).
    #[serde(default)]
    pub retryable_status_codes: Vec<u16>,
}

fn default_max_retries() -> u32 { 3 }
fn default_initial_backoff_ms() -> u64 { 500 }
fn default_max_backoff_ms() -> u64 { 30_000 }
fn default_backoff_multiplier() -> f64 { 2.0 }
fn default_jitter() -> f64 { 0.25 }

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_backoff_ms: default_initial_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            jitter: default_jitter(),
            retryable_status_codes: vec![429, 500, 502, 503, 504],
        }
    }
}

impl RetryPolicy {
    /// Compute the delay for the given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let base = self.initial_backoff_ms as f64
            * self.backoff_multiplier.powi(attempt as i32);
        let capped = base.min(self.max_backoff_ms as f64);

        // Add jitter
        let jitter_range = capped * self.jitter;
        let jitter_offset = rand::random::<f64>() * jitter_range * 2.0 - jitter_range;
        let final_ms = (capped + jitter_offset).max(0.0) as u64;

        std::time::Duration::from_millis(final_ms)
    }
}
```

**Config location** in `clawft.toml` / `config.json`:

```json
{
  "providers": {
    "retry": {
      "max_retries": 3,
      "initial_backoff_ms": 500,
      "max_backoff_ms": 30000,
      "backoff_multiplier": 2.0,
      "jitter": 0.25,
      "retryable_status_codes": [429, 500, 502, 503, 504]
    }
  }
}
```

### 12.5 Latency Recording (D5)

**Files**: `clawft-core/src/pipeline/traits.rs`, `clawft-core/src/agent/loop_core.rs`

`ResponseOutcome.latency_ms` is currently hardcoded to `0`. Record wall-clock
latency around provider calls:

```rust
// In AgentLoop::process_message (loop_core.rs)
let transport_start = std::time::Instant::now();
let llm_response = pipeline.transport.complete(&transport_request).await?;
let latency_ms = transport_start.elapsed().as_millis() as u64;

let outcome = ResponseOutcome {
    success: true,
    quality: scorer.score(&chat_request, &llm_response),
    latency_ms,
};

// Feed back to router for adaptive routing
pipeline.router.update(&routing_decision, &outcome);

// Record to metrics subsystem
tracing::info!(
    provider = %routing_decision.provider,
    model = %routing_decision.model,
    latency_ms = latency_ms,
    input_tokens = llm_response.usage.input_tokens,
    output_tokens = llm_response.usage.output_tokens,
    "llm call complete"
);
```

**Metrics integration point**: Latency is recorded as a `tracing::info!` event
with structured fields. This allows any `tracing-subscriber` layer (OpenTelemetry,
Prometheus, file logger) to capture it without coupling the core to a specific
metrics system.

### 12.6 StreamCallback FnMut Change (D7)

**File**: `clawft-core/src/pipeline/traits.rs`

Current definition prevents stateful callbacks:

```rust
// BEFORE (Fn -- no mutation allowed)
pub type StreamCallback = Box<dyn Fn(&str) -> bool + Send + Sync>;
```

Change to `FnMut` to support token accumulators and progress trackers:

```rust
// AFTER (FnMut -- accumulation and state mutation allowed)
pub type StreamCallback = Box<dyn FnMut(&str) -> bool + Send>;
```

**Note**: Dropping `Sync` is intentional -- the callback is only invoked from a
single async task (the transport layer). `Send` is required because the future
crosses await points.

**Impact on callers**: The `LlmTransport::complete_stream` signature must take
`callback: StreamCallback` by value (already the case). Callers that construct
callbacks must ensure they use `move` closures when capturing external state:

```rust
// Example: accumulating tokens
let mut accumulated = String::new();
let callback: StreamCallback = Box::new(move |chunk: &str| -> bool {
    accumulated.push_str(chunk);
    // Could update a progress bar, etc.
    true
});
```

### 12.7 Bounded Message Bus (D8)

**File**: `clawft-core/src/bus.rs`

Replace unbounded channels with bounded channels and configurable buffer sizes:

```rust
/// Configuration for the message bus buffer sizes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusConfig {
    /// Maximum number of buffered inbound messages (default: 1024).
    #[serde(default = "default_inbound_buffer")]
    pub inbound_buffer_size: usize,

    /// Maximum number of buffered outbound messages (default: 1024).
    #[serde(default = "default_outbound_buffer")]
    pub outbound_buffer_size: usize,

    /// Behavior when the buffer is full.
    #[serde(default)]
    pub overflow_policy: OverflowPolicy,
}

fn default_inbound_buffer() -> usize { 1024 }
fn default_outbound_buffer() -> usize { 1024 }

/// How to handle messages when the buffer is full.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    /// Block the sender until space is available (default).
    #[default]
    Block,
    /// Drop the oldest message to make room.
    DropOldest,
    /// Drop the new message and log a warning.
    DropNew,
}

pub struct MessageBus {
    inbound_tx: mpsc::Sender<InboundMessage>,
    inbound_rx: Mutex<mpsc::Receiver<InboundMessage>>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::Receiver<OutboundMessage>>,
    config: BusConfig,
}

impl MessageBus {
    /// Create a new message bus with the given buffer configuration.
    pub fn with_config(config: BusConfig) -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel(config.inbound_buffer_size);
        let (outbound_tx, outbound_rx) = mpsc::channel(config.outbound_buffer_size);

        debug!(
            inbound_buffer = config.inbound_buffer_size,
            outbound_buffer = config.outbound_buffer_size,
            "MessageBus created (bounded)"
        );

        Self {
            inbound_tx,
            inbound_rx: Mutex::new(inbound_rx),
            outbound_tx,
            outbound_rx: Mutex::new(outbound_rx),
            config,
        }
    }

    /// Create with default buffer sizes (backward-compatible).
    pub fn new() -> Self {
        Self::with_config(BusConfig::default())
    }
}
```

**Config JSON**:

```json
{
  "bus": {
    "inbound_buffer_size": 1024,
    "outbound_buffer_size": 1024,
    "overflow_policy": "block"
  }
}
```

---

## 13. Channel Enhancements (Workstream E)

### 13.1 Discord Resume (OP 6) Implementation (E1)

**File**: `clawft-channels/src/discord/channel.rs`

The `DiscordChannel` already stores `session_id` and `resume_url` from the
READY event but always re-identifies on reconnect. Implement Gateway Resume
per [Discord docs](https://discord.com/developers/docs/events/gateway#resume).

**Resume flow**:

```
Connection drops
  |
  v
Has session_id AND resume_url?
  |                    |
  YES                  NO
  |                    |
  v                    v
Connect to resume_url  Connect to default gateway URL
Send OP 6 (Resume)     Send OP 2 (Identify)
  |
  v
Receive response
  |         |
  OP 0      OP 9 (Invalid Session)
  (Dispatch)    |
  |             v
  v          session_id = None
  RESUMED    Fall back to Identify
```

**Resume payload** (OP 6):

```rust
/// OP 6 Resume payload for reconnecting to the Discord Gateway.
#[derive(Debug, Serialize)]
pub struct ResumePayload {
    /// The bot token.
    pub token: String,
    /// The session ID from the last READY event.
    pub session_id: String,
    /// The last received sequence number.
    pub seq: u64,
}

impl DiscordChannel {
    /// Attempt to resume an existing session.
    ///
    /// Returns `true` if resume was sent, `false` if no session to resume.
    async fn try_resume(
        &self,
        ws_sink: &mut SplitSink<WebSocketStream, WsMessage>,
    ) -> Result<bool, ChannelError> {
        let session_id = self.session_id.read().await;
        let resume_url = self.resume_url.read().await;

        if let (Some(sid), Some(_url)) = (session_id.as_ref(), resume_url.as_ref()) {
            let seq = self.sequence.load(Ordering::SeqCst);
            let payload = GatewayPayload {
                op: OP_RESUME,
                d: Some(serde_json::to_value(ResumePayload {
                    token: self.config.token.clone(),
                    session_id: sid.clone(),
                    seq,
                })?),
                s: None,
                t: None,
            };
            ws_sink.send(WsMessage::Text(serde_json::to_string(&payload)?)).await?;
            debug!(session_id = %sid, seq = seq, "sent Gateway Resume (OP 6)");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Handle OP 9 (Invalid Session) by clearing state and re-identifying.
    async fn handle_invalid_session(&self, resumable: bool) {
        if !resumable {
            *self.session_id.write().await = None;
            *self.resume_url.write().await = None;
            self.sequence.store(0, Ordering::SeqCst);
            debug!("session invalidated, will re-identify");
        }
    }
}
```

**Connection logic change** in the reconnect loop:

```rust
// In the reconnect method:
let gateway_url = {
    let resume_url = self.resume_url.read().await;
    resume_url.clone().unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string())
};

let (mut ws_stream, _) = connect_async(&gateway_url).await?;
let (mut sink, mut stream) = ws_stream.split();

// Wait for OP 10 (Hello) first
// ... receive hello, start heartbeat ...

// Try resume before identify
if !self.try_resume(&mut sink).await? {
    self.identify(&mut sink).await?;
}
```

### 13.2 Email Channel Plugin Architecture (E2)

**New files**: `clawft-channels/src/email/` (feature-gated behind `email`)

```
clawft-channels/src/email/
  mod.rs          -- EmailChannel + EmailChannelFactory
  imap_client.rs  -- IMAP polling client
  smtp_client.rs  -- SMTP sending client
  oauth2.rs       -- Gmail/Outlook OAuth2 flow
  parser.rs       -- Email -> InboundMessage conversion
  formatter.rs    -- OutboundMessage -> MIME email
```

**Architecture diagram**:

```
                      +------------------+
                      |  EmailChannel    |
                      |  (Channel trait) |
                      +--------+---------+
                               |
            +------------------+------------------+
            |                                     |
    +-------v--------+                   +--------v-------+
    | ImapPoller     |                   | SmtpSender     |
    | (background    |                   | (lettre crate) |
    | task, polls    |                   +--------+-------+
    | inbox on cron) |                            |
    +-------+--------+                   +--------v-------+
            |                            | EmailFormatter |
    +-------v--------+                   | (markdown ->   |
    | EmailParser    |                   |  MIME/HTML)    |
    | (MIME -> text, |                   +----------------+
    |  attachments)  |
    +-------+--------+
            |
    +-------v---------+
    | OAuth2Provider  |
    | (token refresh, |
    |  XOAUTH2 SASL) |
    +------------------+
```

**Key types**:

```rust
/// Email channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// IMAP server hostname.
    pub imap_host: String,
    /// IMAP server port (default: 993 for TLS).
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,

    /// SMTP server hostname.
    pub smtp_host: String,
    /// SMTP server port (default: 587 for STARTTLS).
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,

    /// Email address (used as sender and IMAP login).
    pub email_address: String,

    /// Authentication method.
    pub auth: EmailAuth,

    /// Polling interval in seconds (default: 60).
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    /// IMAP folder to monitor (default: "INBOX").
    #[serde(default = "default_folder")]
    pub folder: String,

    /// Only process emails from these addresses (empty = all).
    #[serde(default)]
    pub allowed_senders: Vec<String>,
}

fn default_imap_port() -> u16 { 993 }
fn default_smtp_port() -> u16 { 587 }
fn default_poll_interval() -> u64 { 60 }
fn default_folder() -> String { "INBOX".into() }

/// Email authentication method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmailAuth {
    /// Username + password (env var name, not plaintext -- see A4).
    Password {
        password_env: String,
    },
    /// OAuth2 (Gmail, Outlook).
    OAuth2 {
        /// Client ID env var name.
        client_id_env: String,
        /// Client secret env var name.
        client_secret_env: String,
        /// OAuth2 token endpoint.
        token_url: String,
        /// Refresh token env var name.
        refresh_token_env: String,
        /// OAuth2 scopes.
        #[serde(default)]
        scopes: Vec<String>,
    },
}
```

**Channel trait implementation**:

```rust
#[async_trait]
impl Channel for EmailChannel {
    fn name(&self) -> &str { "email" }

    async fn start(
        &self,
        host: Arc<dyn ChannelHost>,
        cancel: CancellationToken,
    ) -> Result<()> {
        // 1. Authenticate (password or OAuth2 token refresh)
        // 2. Connect IMAP client
        // 3. Spawn polling loop:
        //    - IMAP IDLE or poll every N seconds
        //    - For each new email: parse -> InboundMessage -> host.deliver_inbound()
        //    - Mark processed emails as seen
        // 4. Wait for cancellation
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<()> {
        // 1. Format OutboundMessage as MIME (text/plain + text/html multipart)
        // 2. Resolve reply-to from msg.metadata (In-Reply-To, References headers)
        // 3. Send via SMTP (lettre transport)
    }
}
```

**Dependencies**:

```toml
# clawft-channels/Cargo.toml additions
[features]
email = ["dep:lettre", "dep:imap", "dep:oauth2", "dep:mailparse"]

[dependencies]
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport"], optional = true }
imap = { version = "3.0", optional = true }
oauth2 = { version = "4.4", optional = true }
mailparse = { version = "0.15", optional = true }
```

### 13.3 WhatsApp Cloud API Plugin Architecture (E3)

**New files**: `clawft-channels/src/whatsapp/` (feature-gated behind `whatsapp`)

```
clawft-channels/src/whatsapp/
  mod.rs          -- WhatsAppChannel + factory
  api_client.rs   -- Cloud API REST client
  webhook.rs      -- Webhook receiver (HTTP server)
  types.rs        -- WhatsApp-specific message types
```

**Architecture**:

```
Inbound:
  WhatsApp Cloud -> Webhook (HTTP POST) -> WhatsAppChannel -> MessageBus

Outbound:
  MessageBus -> WhatsAppChannel -> Cloud API (HTTP POST) -> WhatsApp

Webhook verification:
  GET /webhook?hub.mode=subscribe&hub.verify_token=...&hub.challenge=...
  -> Return hub.challenge
```

**Key types**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// WhatsApp Business Account phone number ID.
    pub phone_number_id: String,
    /// Access token env var name.
    pub access_token_env: String,
    /// Webhook verify token (for registration).
    pub verify_token: String,
    /// API version (default: "v21.0").
    #[serde(default = "default_api_version")]
    pub api_version: String,
    /// Webhook listen port (default: 8443).
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
}

fn default_api_version() -> String { "v21.0".into() }
fn default_webhook_port() -> u16 { 8443 }
```

**Dependencies**: `axum` for webhook HTTP server (or reuse existing gateway HTTP
if available).

### 13.4 Signal / iMessage Bridge Architecture (E4)

**Approach**: Signal via `signal-cli` subprocess (Java-based CLI tool).
iMessage via macOS-only `Messages.app` AppleScript bridge.

```rust
/// Signal bridge using signal-cli as a subprocess.
pub struct SignalChannel {
    /// Path to signal-cli binary.
    signal_cli_path: PathBuf,
    /// Registered phone number.
    phone_number: String,
    /// Spawner for process execution.
    spawner: Arc<dyn ProcessSpawner>,
}

#[async_trait]
impl Channel for SignalChannel {
    async fn start(&self, host: Arc<dyn ChannelHost>, cancel: CancellationToken) -> Result<()> {
        // Run: signal-cli -u <number> daemon --json
        // Parse JSON-line output for incoming messages
        // Deliver to host.deliver_inbound()
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<()> {
        // Run: signal-cli -u <number> send -m <text> <recipient>
    }
}
```

### 13.5 Matrix / IRC Adapter Architecture (E5)

**Matrix**: Native Rust client via `matrix-sdk` crate.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// Homeserver URL (e.g. "https://matrix.org").
    pub homeserver_url: String,
    /// Username.
    pub user_id: String,
    /// Password env var name.
    pub password_env: String,
    /// Rooms to join (empty = accept all invites).
    #[serde(default)]
    pub rooms: Vec<String>,
}
```

**IRC**: Minimal `irc` crate integration.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrcConfig {
    /// IRC server hostname.
    pub server: String,
    /// Server port (default: 6697 for TLS).
    #[serde(default = "default_irc_port")]
    pub port: u16,
    /// Nickname.
    pub nickname: String,
    /// Channels to join.
    pub channels: Vec<String>,
    /// Use TLS (default: true).
    #[serde(default = "default_true")]
    pub tls: bool,
}
```

### 13.6 Google Chat Plugin Architecture (E5a)

**New files**: `clawft-channels/src/google_chat/`

Uses Google Workspace Chat API. Requires a Google Cloud project with
Chat API enabled and a service account or OAuth2 credentials.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatConfig {
    /// Google Cloud project ID.
    pub project_id: String,
    /// Service account key file path (env var name for the path).
    pub service_account_key_env: String,
    /// Webhook URL for receiving messages (pub/sub or HTTP push).
    pub subscription_endpoint: String,
    /// Spaces to monitor (empty = all).
    #[serde(default)]
    pub spaces: Vec<String>,
}
```

**Message flow**:

```
Inbound:
  Google Chat -> Cloud Pub/Sub -> HTTP push to webhook -> GoogleChatChannel -> MessageBus

Outbound:
  MessageBus -> GoogleChatChannel -> spaces.messages.create API -> Google Chat
```

### 13.7 Microsoft Teams Plugin Architecture (E5b)

**New files**: `clawft-channels/src/teams/`

Uses Microsoft Bot Framework with Azure AD authentication.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Azure AD App (client) ID.
    pub app_id: String,
    /// App secret env var name.
    pub app_secret_env: String,
    /// Bot endpoint URL (where Teams sends activities).
    pub messaging_endpoint: String,
    /// Tenant ID (default: "common" for multi-tenant).
    #[serde(default = "default_tenant")]
    pub tenant_id: String,
}

fn default_tenant() -> String { "common".into() }
```

**Authentication flow**: OAuth2 client_credentials grant against
`https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token` with
scope `https://api.botframework.com/.default`.

### 13.8 Updated Channel Feature Matrix

| Channel | Feature Flag | Transport | Auth | Status |
|---------|-------------|-----------|------|--------|
| Telegram | `telegram` | HTTP long-poll + REST | Bot token | Implemented |
| Slack | `slack` | Socket Mode WS + REST | Bot token + signing | Implemented |
| Discord | `discord` | Gateway WS + REST | Bot token | Implemented (resume: E1) |
| Email | `email` | IMAP + SMTP | Password / OAuth2 | Planned (E2) |
| WhatsApp | `whatsapp` | Cloud API + Webhook | Access token | Planned (E3) |
| Signal | `signal` | signal-cli subprocess | Phone registration | Planned (E4) |
| iMessage | `imessage` | macOS AppleScript | System auth | Planned (E4) |
| Matrix | `matrix` | matrix-sdk | User/pass | Planned (E5) |
| IRC | `irc` | irc crate | Nick/pass | Planned (E5) |
| Google Chat | `google-chat` | Pub/Sub + REST | Service account | Planned (E5a) |
| Teams | `teams` | Bot Framework | Azure AD OAuth2 | Planned (E5b) |

---

## 14. Software Dev & App Tooling (Workstream F)

### 14.1 Git Tool Plugin (F1)

**New files**: `clawft-tools/src/git_tool.rs` (feature-gated behind `git`)

Uses the `git2` crate for native Git operations without shelling out.

```rust
/// Git operations tool providing repository management capabilities.
pub struct GitTool {
    /// Base directory for resolving relative repo paths.
    workspace_root: PathBuf,
}

impl GitTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str { "git" }

    fn description(&self) -> &str {
        "Perform Git operations: clone, status, diff, commit, branch, log, blame"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "log", "blame", "commit",
                             "branch", "checkout", "clone", "pull", "push"],
                    "description": "The Git operation to perform"
                },
                "repo_path": {
                    "type": "string",
                    "description": "Path to the repository (default: workspace root)"
                },
                "args": {
                    "type": "object",
                    "description": "Operation-specific arguments",
                    "properties": {
                        "message": { "type": "string" },
                        "files": { "type": "array", "items": { "type": "string" } },
                        "branch": { "type": "string" },
                        "url": { "type": "string" },
                        "path": { "type": "string" },
                        "n": { "type": "integer" }
                    }
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing operation".into()))?;

        let repo_path = args.get("repo_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.workspace_root.clone());

        // Validate repo_path is within allowed directories
        // (reuse existing path sanitization from file tools)

        match operation {
            "status" => self.git_status(&repo_path).await,
            "diff" => self.git_diff(&repo_path, &args).await,
            "log" => self.git_log(&repo_path, &args).await,
            "blame" => self.git_blame(&repo_path, &args).await,
            "commit" => self.git_commit(&repo_path, &args).await,
            "branch" => self.git_branch(&repo_path, &args).await,
            "checkout" => self.git_checkout(&repo_path, &args).await,
            "clone" => self.git_clone(&args).await,
            _ => Err(ToolError::InvalidArgs(format!("unknown operation: {operation}"))),
        }
    }
}
```

**Implementation notes**:

- `git2::Repository::open()` for all local operations
- `git2::Repository::clone()` for clone (with progress callback)
- Read-only operations (`status`, `diff`, `log`, `blame`) use only libgit2
- Write operations (`commit`, `push`) require explicit user confirmation via the
  agent loop (the tool returns a "confirm" response that the LLM must acknowledge)
- Push/pull operations may require SSH key or token auth -- use
  `git2::Cred::ssh_key_from_agent()` or credential helper callbacks

**Dependencies**:

```toml
# clawft-tools/Cargo.toml
[features]
git = ["dep:git2"]

[dependencies]
git2 = { version = "0.19", optional = true }
```

### 14.2 Code Analysis via tree-sitter (F3)

**New files**: `clawft-tools/src/code_analysis.rs` (feature-gated behind `code-analysis`)

Provides AST-level code parsing for intelligent code operations.

```rust
/// Code analysis tool using tree-sitter for AST parsing.
pub struct CodeAnalysisTool {
    /// Loaded language parsers, keyed by file extension.
    parsers: HashMap<String, tree_sitter::Language>,
}

/// Supported analysis operations.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisOperation {
    /// Extract function/method signatures.
    ListFunctions,
    /// Extract struct/class definitions.
    ListTypes,
    /// Find all imports/use statements.
    ListImports,
    /// Get the AST structure summary.
    AstSummary,
    /// Find symbol definition.
    FindDefinition { symbol: String },
    /// Find all references to a symbol.
    FindReferences { symbol: String },
    /// Calculate complexity metrics (cyclomatic, cognitive).
    Complexity,
}
```

**Architecture**:

```
CodeAnalysisTool
  |
  +-- tree_sitter::Parser (per-language)
  |     |-- tree_sitter_rust
  |     |-- tree_sitter_python
  |     |-- tree_sitter_javascript
  |     |-- tree_sitter_typescript
  |     +-- (more languages via dynamic loading)
  |
  +-- QueryEngine
  |     |-- Pre-compiled S-expression queries per language
  |     +-- Custom query support
  |
  +-- MetricsCalculator
        |-- Cyclomatic complexity
        +-- Cognitive complexity
```

**Dependencies**:

```toml
[features]
code-analysis = ["dep:tree-sitter", "dep:tree-sitter-rust", "dep:tree-sitter-python"]

[dependencies]
tree-sitter = { version = "0.24", optional = true }
tree-sitter-rust = { version = "0.23", optional = true }
tree-sitter-python = { version = "0.23", optional = true }
```

### 14.3 Browser CDP Plugin (F4)

**New files**: `clawft-tools/src/browser_tool.rs` (feature-gated behind `browser`)

Uses `chromiumoxide` for Chrome DevTools Protocol automation.

```rust
/// Browser automation tool using Chrome DevTools Protocol.
pub struct BrowserTool {
    /// Shared browser instance (lazy-initialized).
    browser: Arc<Mutex<Option<Browser>>>,
    /// Configuration.
    config: BrowserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Path to Chrome/Chromium binary (auto-detected if not set).
    #[serde(default)]
    pub chrome_path: Option<PathBuf>,
    /// Run headless (default: true).
    #[serde(default = "default_true")]
    pub headless: bool,
    /// Navigation timeout in seconds (default: 30).
    #[serde(default = "default_nav_timeout")]
    pub timeout_secs: u64,
    /// Maximum concurrent pages (default: 3).
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    /// Sandbox mode: restrict to allowed domains.
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

fn default_nav_timeout() -> u64 { 30 }
fn default_max_pages() -> usize { 3 }
```

**Supported operations**:

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserOperation {
    /// Navigate to a URL.
    Navigate { url: String },
    /// Take a screenshot (returns base64 PNG).
    Screenshot,
    /// Get the page HTML content.
    GetHtml,
    /// Get visible text content.
    GetText,
    /// Click an element by CSS selector.
    Click { selector: String },
    /// Fill a form field.
    Fill { selector: String, value: String },
    /// Execute JavaScript and return the result.
    Evaluate { script: String },
    /// Wait for a selector to appear.
    WaitFor { selector: String, timeout_ms: Option<u64> },
    /// Close the current page.
    Close,
}
```

**Security**: The browser runs in a separate process. `allowed_domains` restricts
navigation. JavaScript evaluation is sandboxed by CDP. No access to local
filesystem from the browser context.

**Dependencies**:

```toml
[features]
browser = ["dep:chromiumoxide"]

[dependencies]
chromiumoxide = { version = "0.7", features = ["tokio-runtime"], optional = true }
```

### 14.4 Generic REST + OAuth2 Helper (F6)

**New files**: `clawft-services/src/oauth2_helper.rs`

Reusable OAuth2 flow used by email, calendar, Google Chat, Teams, and future
integrations.

```rust
/// Generic OAuth2 token manager with automatic refresh.
pub struct OAuth2TokenManager {
    /// OAuth2 client configuration.
    client: oauth2::basic::BasicClient,
    /// Cached access token.
    current_token: RwLock<Option<CachedToken>>,
    /// Refresh token (for refreshing expired access tokens).
    refresh_token: Option<oauth2::RefreshToken>,
}

#[derive(Debug)]
struct CachedToken {
    access_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl OAuth2TokenManager {
    /// Create a new token manager from environment variable names.
    pub fn from_env(
        client_id_env: &str,
        client_secret_env: &str,
        token_url: &str,
        refresh_token_env: Option<&str>,
        scopes: &[String],
    ) -> Result<Self> {
        // Resolve env vars (follows A4 pattern: store env var names, not values)
        // ...
    }

    /// Get a valid access token, refreshing if necessary.
    pub async fn get_token(&self) -> Result<String> {
        let token = self.current_token.read().await;
        if let Some(cached) = token.as_ref() {
            if cached.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
                return Ok(cached.access_token.clone());
            }
        }
        drop(token);

        // Refresh
        self.refresh().await
    }

    /// Force a token refresh.
    async fn refresh(&self) -> Result<String> {
        // Use oauth2 crate to refresh token
        // Cache the new token
        // ...
    }
}
```

### 14.5 MCP Client for External Servers (F9)

**Files**: `clawft-services/src/mcp/client.rs` (new), `clawft-cli/src/commands/mcp.rs` (new)

Connects to external MCP servers as a client, making their tools available to the
agent. This is the counterpart to the existing MCP server mode.

**Architecture**:

```
+-------------------+     +------------------+     +--------------------+
| MCP Server A      |     | MCP Server B     |     | MCP Server C       |
| (stdio)           |     | (HTTP/SSE)       |     | (stdio)            |
+--------+----------+     +--------+---------+     +--------+-----------+
         |                         |                         |
         v                         v                         v
+--------+---------+     +---------+--------+     +----------+---------+
| StdioTransport   |     | HttpTransport    |     | StdioTransport     |
| (child process)  |     | (reqwest Client) |     | (child process)    |
+--------+---------+     +---------+--------+     +----------+---------+
         |                         |                         |
         +------------+------------+------------+------------+
                      |
              +-------v--------+
              | McpClientPool  |
              | - connection   |
              |   management   |
              | - health check |
              | - reconnect    |
              +-------+--------+
                      |
              +-------v--------+
              | McpToolBridge  |
              | - schema cache |
              | - tool proxy   |
              +-------+--------+
                      |
              +-------v--------+
              | ToolRegistry   |
              | (existing)     |
              +----------------+
```

**Key types**:

```rust
/// Configuration for an external MCP server connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Display name for this server.
    pub name: String,

    /// Transport type and configuration.
    pub transport: McpTransportConfig,

    /// Whether to auto-connect on startup (default: true).
    #[serde(default = "default_true")]
    pub auto_connect: bool,

    /// Health check interval in seconds (0 = disabled).
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,

    /// Tool name prefix (to avoid collisions with built-in tools).
    /// e.g. "github_" -> "github_create_issue".
    #[serde(default)]
    pub tool_prefix: Option<String>,
}

fn default_health_interval() -> u64 { 30 }

/// Transport configuration for MCP client connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransportConfig {
    /// Stdio transport (child process).
    Stdio {
        /// Command to run.
        command: String,
        /// Command arguments.
        #[serde(default)]
        args: Vec<String>,
        /// Working directory.
        #[serde(default)]
        cwd: Option<PathBuf>,
        /// Environment variables to set.
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// HTTP/SSE transport.
    Http {
        /// Server URL.
        url: String,
        /// Extra headers.
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Pool managing multiple MCP client connections.
pub struct McpClientPool {
    /// Active connections keyed by server name.
    connections: RwLock<HashMap<String, McpClientConnection>>,
    /// Shared HTTP client for HTTP transports.
    http_client: Arc<reqwest::Client>,
}

impl McpClientPool {
    /// Initialize connections from config.
    pub async fn from_configs(configs: &HashMap<String, McpClientConfig>) -> Result<Self> {
        // ...
    }

    /// Add a new server connection at runtime (for `weft mcp add`).
    pub async fn add_server(&self, name: &str, config: McpClientConfig) -> Result<()> {
        // Connect, discover tools, register in pool
    }

    /// Remove a server connection.
    pub async fn remove_server(&self, name: &str) -> Result<()> {
        // Disconnect, unregister tools
    }

    /// List all connected servers and their tools.
    pub async fn list_servers(&self) -> Vec<McpServerInfo> {
        // ...
    }

    /// Discover and cache tool schemas from a connected server.
    async fn discover_tools(&self, conn: &mut McpClientConnection) -> Result<Vec<McpToolSchema>> {
        // Send tools/list request
        // Cache response
        // Return tool schemas
    }
}

/// Bridge that exposes MCP server tools as native clawft tools.
pub struct McpToolBridge {
    /// The client pool for dispatching calls.
    pool: Arc<McpClientPool>,
    /// Server name this bridge connects to.
    server_name: String,
    /// Cached tool schema from the server.
    schema: McpToolSchema,
}

#[async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str { &self.schema.prefixed_name }
    fn description(&self) -> &str { &self.schema.description }
    fn parameters(&self) -> Value { self.schema.input_schema.clone() }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        // Dispatch to MCP server via pool
        self.pool.call_tool(&self.server_name, &self.schema.name, args).await
            .map_err(|e| ToolError::Execution(e.to_string()))
    }
}
```

**CLI commands**:

```
weft mcp add <name> --stdio <command> [--args <args>...]
weft mcp add <name> --http <url> [--header <key=value>...]
weft mcp list
weft mcp remove <name>
weft mcp tools <name>
```

**Config location** in `clawft.toml`:

```toml
[tools.mcp_servers.github]
transport.type = "stdio"
transport.command = "npx"
transport.args = ["-y", "@modelcontextprotocol/server-github"]
transport.env = { GITHUB_PERSONAL_ACCESS_TOKEN = "" }
auto_connect = true
tool_prefix = "github_"

[tools.mcp_servers.filesystem]
transport.type = "stdio"
transport.command = "npx"
transport.args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
```

---

## 15. Memory & Workspace (Workstream H)

### 15.1 Per-Agent Workspace Isolation (H1)

**Directory structure per agent**:

```
~/.clawft/
  agents/
    work-agent/
      config.toml       -- Agent-specific config overrides
      SOUL.md           -- Agent personality
      AGENTS.md         -- Agent capabilities description
      USER.md           -- User preferences for this agent
      workspace/
        MEMORY.md       -- Agent-scoped memory
        HISTORY.md      -- Agent-scoped history
      sessions/
        *.jsonl         -- Agent-scoped sessions
      skills/           -- Agent-scoped skill overrides
    personal-agent/
      config.toml
      SOUL.md
      ...
```

**Implementation**:

```rust
/// Per-agent workspace manager.
pub struct AgentWorkspace {
    /// Unique agent identifier.
    agent_id: String,
    /// Root directory for this agent (e.g. ~/.clawft/agents/work-agent/).
    root: PathBuf,
    /// Whether cross-agent memory access is enabled.
    cross_agent_access: bool,
}

impl AgentWorkspace {
    /// Resolve the workspace for a given agent ID.
    pub fn for_agent(agent_id: &str, global_root: &Path) -> Self {
        let root = global_root.join("agents").join(agent_id);
        Self {
            agent_id: agent_id.to_string(),
            root,
            cross_agent_access: false,
        }
    }

    /// Initialize the workspace directory structure.
    pub fn initialize(&self, fs: &dyn FileSystem) -> Result<()> {
        fs.create_dir_all(&self.root)?;
        fs.create_dir_all(&self.root.join("workspace"))?;
        fs.create_dir_all(&self.root.join("sessions"))?;
        fs.create_dir_all(&self.root.join("skills"))?;
        Ok(())
    }

    /// Get the path to this agent's SOUL.md.
    pub fn soul_path(&self) -> PathBuf { self.root.join("SOUL.md") }

    /// Get the path to this agent's session store.
    pub fn sessions_dir(&self) -> PathBuf { self.root.join("sessions") }

    /// Get the path to this agent's memory file.
    pub fn memory_path(&self) -> PathBuf { self.root.join("workspace").join("MEMORY.md") }

    /// Get the path to this agent's skill overrides directory.
    pub fn skills_dir(&self) -> PathBuf { self.root.join("skills") }

    /// Get agent-specific config overrides.
    pub fn config_path(&self) -> PathBuf { self.root.join("config.toml") }
}
```

**Skill discovery with agent isolation** (extends Section 10 of existing doc):

```
Discovery order (highest precedence wins):
  1. Agent skills:   ~/.clawft/agents/<id>/skills/      (agent-scoped)
  2. Project skills: <project>/.clawft/skills/           (project-scoped)
  3. User skills:    ~/.clawft/skills/                   (personal)
  4. Global skills:  ~/.clawft/workspace/skills/         (global)
```

### 15.2 RVF Phase 3 Completion (H2)

Completion of the remaining 8/9 RVF Phase 3 items (only crate dependency
integration is done).

#### 15.2.1 HNSW-backed VectorStore

**File**: `clawft-core/src/vector_store.rs` (replace brute-force stub)

```rust
use instant_distance::{Builder, HnswMap, Search};

/// HNSW-backed vector store for efficient approximate nearest neighbor search.
pub struct HnswVectorStore {
    /// The HNSW graph.
    map: RwLock<HnswMap<MemoryPoint, String>>,
    /// Dimension of stored vectors.
    dimension: usize,
    /// Number of neighbors to consider during search (ef_search).
    ef_search: usize,
    /// Number of neighbors to consider during construction (ef_construction).
    ef_construction: usize,
    /// Maximum number of connections per node.
    m: usize,
}

/// A point in the HNSW graph with its embedding vector.
#[derive(Clone)]
struct MemoryPoint {
    /// The embedding vector.
    vector: Vec<f32>,
}

impl instant_distance::Point for MemoryPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Cosine distance = 1 - cosine_similarity
        let dot: f32 = self.vector.iter().zip(&other.vector).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let similarity = if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        };
        1.0 - similarity
    }
}

impl HnswVectorStore {
    /// Build a new HNSW index from existing vectors.
    pub fn build(vectors: Vec<(Vec<f32>, String)>, config: HnswConfig) -> Self {
        let points: Vec<MemoryPoint> = vectors.iter()
            .map(|(v, _)| MemoryPoint { vector: v.clone() })
            .collect();
        let values: Vec<String> = vectors.into_iter().map(|(_, id)| id).collect();

        let map = Builder::default()
            .ef_construction(config.ef_construction)
            .build(points, values);

        Self {
            map: RwLock::new(map),
            dimension: config.dimension,
            ef_search: config.ef_search,
            ef_construction: config.ef_construction,
            m: config.m,
        }
    }

    /// Search for the k nearest neighbors to the query vector.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        let map = self.map.read().unwrap();
        let point = MemoryPoint { vector: query.to_vec() };
        let mut search = Search::default();

        let results = map.search(&point, &mut search);
        results
            .take(k)
            .map(|item| SearchResult {
                id: item.value.clone(),
                distance: item.distance,
                similarity: 1.0 - item.distance,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Vector dimension.
    pub dimension: usize,
    /// ef parameter for search (higher = better recall, slower).
    #[serde(default = "default_ef_search")]
    pub ef_search: usize,
    /// ef parameter for construction.
    #[serde(default = "default_ef_construction")]
    pub ef_construction: usize,
    /// Maximum connections per node.
    #[serde(default = "default_m")]
    pub m: usize,
}

fn default_ef_search() -> usize { 100 }
fn default_ef_construction() -> usize { 200 }
fn default_m() -> usize { 16 }
```

**Dependencies**:

```toml
instant-distance = { version = "0.6", optional = true }
```

#### 15.2.2 Production Embedder

Wire `api_embedder.rs` (exists but unconnected) to replace `HashEmbedder`:

```rust
/// LLM-backed embedder using OpenAI-compatible embedding API.
pub struct ApiEmbedder {
    /// HTTP client for API calls.
    client: reqwest::Client,
    /// API base URL (e.g. "https://api.openai.com/v1").
    base_url: String,
    /// API key (resolved from env var).
    api_key: String,
    /// Model to use (e.g. "text-embedding-3-small").
    model: String,
    /// Expected output dimension.
    dimension: usize,
    /// Local cache to avoid redundant API calls.
    cache: RwLock<HashMap<u64, Vec<f32>>>,
}

impl ApiEmbedder {
    /// Embed a batch of texts, returning one vector per input.
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // POST to /embeddings with { model, input: texts }
        // Parse response and return vectors
        // Cache results keyed by content hash
    }
}
```

#### 15.2.3 RVF File I/O

Implement real RVF segment read/write:

```rust
/// Read/write RVF format segments for persistent vector storage.
pub struct RvfStorage {
    /// Directory for RVF segment files.
    data_dir: PathBuf,
}

impl RvfStorage {
    /// Write vectors and metadata to an RVF segment file.
    pub fn write_segment(&self, segment: &RvfSegment) -> Result<PathBuf> {
        // Binary format: header + vectors + metadata + index
    }

    /// Read an RVF segment file.
    pub fn read_segment(&self, path: &Path) -> Result<RvfSegment> {
        // Parse binary format
    }

    /// Export all segments to a portable archive.
    pub fn export(&self, output: &Path) -> Result<()> {
        // Tar + gzip all segment files
    }

    /// Import segments from a portable archive.
    pub fn import(&self, archive: &Path) -> Result<()> {
        // Extract and validate segments
    }
}
```

#### 15.2.4 CLI Commands

```
weft memory export --output backup.rvf.tar.gz [--namespace <ns>]
weft memory import --input backup.rvf.tar.gz [--merge | --replace]
```

### 15.3 Timestamp Standardization (H3)

**Files**: Various across `clawft-types`

Standardize all timestamp representations to `DateTime<Utc>`:

| Type | Field | Current | Target |
|------|-------|---------|--------|
| `InboundMessage` | `timestamp` | `DateTime<Utc>` | `DateTime<Utc>` (no change) |
| `CronJob` | `created_at_ms` | `i64` (ms) | `DateTime<Utc>` |
| `CronJob` | `last_run_ms` | `Option<i64>` | `Option<DateTime<Utc>>` |
| `WorkspaceEntry` | `last_accessed` | `Option<String>` | `Option<DateTime<Utc>>` |
| `Session` | `created_at` | `String` (ISO) | `DateTime<Utc>` |
| `Session` | `updated_at` | `String` (ISO) | `DateTime<Utc>` |

**Migration helper**:

```rust
/// Deserialize either a DateTime<Utc> or a millisecond timestamp.
fn deserialize_timestamp<'de, D>(de: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct TimestampVisitor;
    impl<'de> de::Visitor<'de> for TimestampVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a DateTime string or millisecond timestamp")
        }

        fn visit_i64<E: de::Error>(self, ms: i64) -> Result<DateTime<Utc>, E> {
            DateTime::from_timestamp_millis(ms)
                .ok_or_else(|| E::custom("invalid timestamp"))
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<DateTime<Utc>, E> {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(de::Error::custom)
        }
    }

    de.deserialize_any(TimestampVisitor)
}
```

---

## 16. Deployment (Workstream K)

### 16.1 Docker Multi-Arch Image (K2)

**New file**: `Dockerfile` (project root)

```dockerfile
# syntax=docker/dockerfile:1
# Multi-stage build for clawft (weft binary)

# ── Build stage ────────────────────────────────────────────────────────
FROM --platform=$BUILDPLATFORM rust:1.85-bookworm AS builder

WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/

# Build release binary with default features
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release -p clawft-cli \
    && cp target/release/weft /usr/local/bin/weft

# ── Runtime stage ──────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/weft /usr/local/bin/weft

# Non-root user
RUN useradd -m -s /bin/bash clawft
USER clawft
WORKDIR /home/clawft

# Default config and workspace
RUN mkdir -p .clawft/workspace .clawft/sessions

ENTRYPOINT ["weft"]
CMD ["gateway"]
```

**Multi-arch build** via GitHub Actions:

```yaml
# .github/workflows/docker.yml
name: Docker Multi-Arch

on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-qemu-action@v3
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v6
        with:
          platforms: linux/amd64,linux/arm64
          push: true
          tags: |
            ghcr.io/${{ github.repository }}:latest
            ghcr.io/${{ github.repository }}:${{ github.ref_name }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

**Image size targets**:

| Variant | Target Size | Includes |
|---------|------------|----------|
| `slim` (default features) | < 25 MB | weft binary + TLS certs |
| `full` (all channels + tools) | < 50 MB | Above + browser deps excluded |
| `dev` | < 200 MB | Above + git, tree-sitter, debug symbols |

### 16.2 Per-Agent Sandbox Architecture (K3)

**Security layers** (defense-in-depth):

```
Layer 1: WASM sandbox (wasmtime)
  - Memory isolation per plugin
  - No filesystem access except through WASI capabilities
  - CPU time limits via fuel metering

Layer 2: seccomp-bpf (Linux)
  - System call filtering per agent
  - Block: execve, ptrace, mount, etc.
  - Allow: read, write, futex, clock_gettime, etc.

Layer 3: Landlock (Linux 5.13+)
  - Filesystem access restrictions per agent
  - Agent A: read-write to ~/.clawft/agents/a/
  - Agent B: read-only to project root

Layer 4: Network policy
  - Per-agent allowed domains
  - Block local network access (SSRF protection)
```

**Implementation**:

```rust
/// Sandbox configuration for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox type to use.
    #[serde(default)]
    pub sandbox_type: SandboxType,

    /// Filesystem access rules.
    #[serde(default)]
    pub filesystem: FilesystemPolicy,

    /// Network access rules.
    #[serde(default)]
    pub network: NetworkPolicy,

    /// Process execution rules.
    #[serde(default)]
    pub process: ProcessPolicy,

    /// Resource limits.
    #[serde(default)]
    pub resources: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxType {
    /// No sandbox (trust the agent).
    #[default]
    None,
    /// WASM sandbox (plugins only).
    Wasm,
    /// OS-level sandbox (seccomp + landlock).
    OsSandbox,
    /// Both WASM and OS-level.
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilesystemPolicy {
    /// Directories with read access.
    #[serde(default)]
    pub read_paths: Vec<PathBuf>,
    /// Directories with read-write access.
    #[serde(default)]
    pub write_paths: Vec<PathBuf>,
    /// Directories explicitly denied (overrides above).
    #[serde(default)]
    pub deny_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkPolicy {
    /// Allowed outbound domains (empty = all allowed).
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Whether local network access is allowed.
    #[serde(default)]
    pub allow_local_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in MB (0 = unlimited).
    #[serde(default)]
    pub max_memory_mb: u64,
    /// Maximum CPU time in seconds per tool execution.
    #[serde(default = "default_cpu_limit")]
    pub max_cpu_secs: u64,
    /// Maximum concurrent tool executions.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_tools: usize,
}

fn default_cpu_limit() -> u64 { 30 }
fn default_max_concurrent() -> usize { 5 }
```

**Per-agent config** (`~/.clawft/agents/<id>/config.toml`):

```toml
[sandbox]
sandbox_type = "os_sandbox"

[sandbox.filesystem]
read_paths = ["/home/user/projects/myapp"]
write_paths = ["/home/user/.clawft/agents/work-agent"]
deny_paths = ["/etc", "/root"]

[sandbox.network]
allowed_domains = ["api.openai.com", "api.anthropic.com", "github.com"]
allow_local_network = false

[sandbox.resources]
max_memory_mb = 512
max_cpu_secs = 60
max_concurrent_tools = 3
```

### 16.3 Security Plugin System (K3a)

**Architecture**:

```
+-------------------+
| SecurityPlugin    |
| (trait)           |
+--------+----------+
         |
    +----+----+----+----+
    |         |         |
+---v---+ +---v---+ +---v---+
| Audit | | Harden| | Monitor|
| Checks| | Modules| | Service|
+---+---+ +---+---+ +---+----+
    |         |          |
    v         v          v
50+ scan   seccomp/    anomaly
rules    landlock    detection
         profiles
```

**Key trait**:

```rust
/// Security plugin interface.
#[async_trait]
pub trait SecurityPlugin: Send + Sync {
    /// Run audit checks against a skill or tool definition.
    async fn audit_skill(&self, skill: &SkillDefinition) -> Vec<AuditFinding>;

    /// Generate a hardening profile for an agent.
    async fn harden_agent(&self, agent_id: &str, config: &SandboxConfig) -> HardeningProfile;

    /// Check a tool invocation against security policy.
    async fn check_tool_call(
        &self,
        agent_id: &str,
        tool_name: &str,
        args: &Value,
    ) -> SecurityDecision;
}

#[derive(Debug)]
pub struct AuditFinding {
    pub severity: Severity,
    pub category: AuditCategory,
    pub message: String,
    pub location: Option<String>,
    pub remediation: Option<String>,
}

#[derive(Debug)]
pub enum Severity { Low, Medium, High, Critical }

#[derive(Debug)]
pub enum AuditCategory {
    PromptInjection,
    DataExfiltration,
    UnsafeShellCommand,
    CredentialLeak,
    ExcessivePermission,
    UnvalidatedInput,
}

#[derive(Debug)]
pub enum SecurityDecision {
    Allow,
    Deny { reason: String },
    AllowWithWarning { warning: String },
}
```

**CLI integration**:

```
weft security scan [--path <dir>]        -- Scan skills for vulnerabilities
weft security audit [--agent <id>]       -- Audit an agent's configuration
weft security harden [--agent <id>]      -- Apply hardening profile
weft security monitor [--agent <id>]     -- Start real-time monitoring
```

### 16.4 ClawHub Registry with Vector Search (K4)

**Architecture**:

```
+------------------+       +-------------------+
| weft skill       |       | ClawHub API       |
| install/publish  | ----> | (HTTPS REST)      |
+------------------+       +--------+----------+
                                    |
                            +-------v--------+
                            | Registry Index |
                            | - metadata DB  |
                            | - HNSW vector  |
                            |   search       |
                            | - git-backed   |
                            |   storage      |
                            +----------------+
```

**Search flow**:

```rust
/// ClawHub client for skill discovery and installation.
pub struct ClawHubClient {
    /// HTTP client.
    http: reqwest::Client,
    /// Registry URL.
    base_url: String,
    /// Local cache of installed skills.
    cache_dir: PathBuf,
}

impl ClawHubClient {
    /// Search for skills using semantic vector search.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SkillListing>> {
        // POST /api/v1/skills/search
        // Body: { "query": "...", "limit": 10 }
        // Server embeds query and runs HNSW search against skill descriptions
    }

    /// Install a skill by name and version.
    pub async fn install(&self, name: &str, version: Option<&str>) -> Result<InstalledSkill> {
        // GET /api/v1/skills/{name}/versions/{version}
        // Download skill archive
        // Verify signature (if present)
        // Extract to managed skills directory
    }

    /// Publish a skill to the registry.
    pub async fn publish(&self, skill_dir: &Path) -> Result<PublishResult> {
        // Read SKILL.md and manifest
        // Package as tar.gz
        // POST /api/v1/skills
    }
}
```

**CLI commands**:

```
weft skill search "git automation"       -- Semantic search
weft skill install <name> [--version v]  -- Install from ClawHub
weft skill publish <dir>                 -- Publish to ClawHub
weft skill update [--all]                -- Check for updates
weft skill list [--installed | --remote] -- List skills
```

---

## 17. Multi-Agent Routing & Orchestration (Workstream L)

### 17.1 Agent Routing Table (L1)

**File**: `clawft-types/src/config.rs` (new `AgentRoute` type),
`clawft-core/src/agent/router.rs` (new module)

Maps inbound channel messages to isolated agent instances.

**Configuration** (TOML):

```toml
# Agent routing table in clawft.toml
# Each route maps a channel + match criteria to an agent ID

[[agent_routes]]
channel = "telegram"
match = { user_id = "12345" }
agent = "work-agent"

[[agent_routes]]
channel = "telegram"
match = { user_id = "67890" }
agent = "personal-agent"

[[agent_routes]]
channel = "whatsapp"
match = { phone = "+15551234567" }
agent = "personal-agent"

[[agent_routes]]
channel = "slack"
match = { workspace_id = "T12345", user_id = "U67890" }
agent = "office-agent"

[[agent_routes]]
channel = "discord"
match = { guild_id = "123", channel_id = "456" }
agent = "gaming-agent"

# Default agent for unmatched messages
[agent_routing]
default_agent = "general-agent"
# Whether to create new agents on-the-fly for unrecognized senders
auto_create = false
```

**Types**:

```rust
/// A single agent routing rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoute {
    /// Channel name to match (e.g. "telegram", "discord").
    pub channel: String,

    /// Match criteria (channel-specific key-value pairs).
    /// Keys: "user_id", "chat_id", "phone", "workspace_id", "guild_id", etc.
    #[serde(rename = "match")]
    pub match_criteria: HashMap<String, String>,

    /// Target agent ID.
    pub agent: String,
}

/// Agent routing configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentRoutingConfig {
    /// Ordered list of routing rules (first match wins).
    #[serde(default)]
    pub routes: Vec<AgentRoute>,

    /// Default agent for unmatched messages.
    #[serde(default = "default_agent_id")]
    pub default_agent: String,

    /// Whether to auto-create agent workspaces for new senders.
    #[serde(default)]
    pub auto_create: bool,
}

fn default_agent_id() -> String { "default".into() }

/// Resolves inbound messages to agent IDs.
pub struct AgentRouter {
    config: AgentRoutingConfig,
}

impl AgentRouter {
    /// Resolve the target agent for an inbound message.
    pub fn resolve(&self, msg: &InboundMessage) -> &str {
        for route in &self.config.routes {
            if route.channel != msg.channel {
                continue;
            }

            let all_match = route.match_criteria.iter().all(|(key, value)| {
                match key.as_str() {
                    "user_id" => msg.sender_id.as_deref() == Some(value),
                    "chat_id" => &msg.chat_id == value,
                    "phone" => msg.metadata.get("phone").and_then(|v| v.as_str()) == Some(value),
                    "workspace_id" => msg.metadata.get("workspace_id").and_then(|v| v.as_str()) == Some(value),
                    "guild_id" => msg.metadata.get("guild_id").and_then(|v| v.as_str()) == Some(value),
                    "channel_id" => msg.metadata.get("channel_id").and_then(|v| v.as_str()) == Some(value),
                    _ => false,
                }
            });

            if all_match {
                return &route.agent;
            }
        }

        &self.config.default_agent
    }
}
```

**Integration into AgentLoop**:

```
InboundMessage arrives on MessageBus
  |
  v
AgentRouter::resolve(&msg) -> agent_id
  |
  v
AgentRegistry::get_or_create(agent_id)
  |
  v
AgentInstance processes message with its own:
  - SessionManager (isolated sessions dir)
  - ContextBuilder (agent-specific SOUL.md, AGENTS.md)
  - SkillsLoader (agent-specific skill overrides)
  - PermissionResolver (agent-specific sandbox)
```

### 17.2 Per-Agent Session Isolation (L2)

Each routed agent gets a dedicated session store:

```rust
/// Agent-aware session manager.
pub struct IsolatedSessionManager<P: Platform> {
    /// Per-agent session managers.
    agents: RwLock<HashMap<String, SessionManager<P>>>,
    /// Platform abstraction.
    platform: Arc<P>,
}

impl<P: Platform> IsolatedSessionManager<P> {
    /// Get or create a session manager for the given agent.
    pub async fn for_agent(&self, agent_id: &str) -> &SessionManager<P> {
        let agents = self.agents.read().await;
        if agents.contains_key(agent_id) {
            return &agents[agent_id];
        }
        drop(agents);

        let mut agents = self.agents.write().await;
        agents.entry(agent_id.to_string()).or_insert_with(|| {
            let workspace = AgentWorkspace::for_agent(agent_id, &self.global_root);
            SessionManager::new(
                self.platform.clone(),
                workspace.sessions_dir(),
            )
        });
        &agents[agent_id]
    }
}
```

### 17.3 Multi-Agent Swarming Architecture (L3)

**Coordinator pattern**: One "lead" agent decomposes tasks and dispatches to
worker agents via the MessageBus.

```
                    +-------------------+
                    | Coordinator Agent |
                    | (lead)            |
                    +--------+----------+
                             |
              +--------------+--------------+
              |              |              |
      +-------v------+ +----v-------+ +----v-------+
      | Worker Agent  | | Worker     | | Worker     |
      | (research)    | | (code)     | | (test)     |
      +--------------+ +------------+ +------------+
              |              |              |
              +--------------+--------------+
                             |
                    +--------v----------+
                    | Shared Result Bus |
                    | (memory bus)      |
                    +-------------------+
```

**Swarm types**:

```rust
/// A task that can be dispatched to a worker agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTask {
    /// Unique task ID.
    pub id: String,
    /// The task description / prompt.
    pub prompt: String,
    /// Target agent type (e.g. "researcher", "coder", "tester").
    pub agent_type: String,
    /// Dependencies: task IDs that must complete before this one.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Maximum execution time in seconds.
    #[serde(default = "default_task_timeout")]
    pub timeout_secs: u64,
}

fn default_task_timeout() -> u64 { 300 }

/// Result from a completed swarm task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTaskResult {
    /// Task ID.
    pub task_id: String,
    /// Agent that executed the task.
    pub agent_id: String,
    /// Whether the task succeeded.
    pub success: bool,
    /// The result text/output.
    pub output: String,
    /// Artifacts produced (file paths, etc.).
    #[serde(default)]
    pub artifacts: Vec<String>,
}

/// Orchestrates a swarm of agents for complex multi-step tasks.
pub struct SwarmCoordinator {
    /// Routing table for agent resolution.
    router: AgentRouter,
    /// Active swarm tasks.
    tasks: RwLock<HashMap<String, SwarmTask>>,
    /// Completed task results.
    results: RwLock<HashMap<String, SwarmTaskResult>>,
    /// Message bus for inter-agent communication.
    bus: Arc<MessageBus>,
}

impl SwarmCoordinator {
    /// Decompose a complex task into swarm tasks and dispatch.
    pub async fn execute_plan(&self, plan: Vec<SwarmTask>) -> Result<Vec<SwarmTaskResult>> {
        // 1. Topological sort tasks by dependencies
        // 2. Dispatch tasks with no dependencies first
        // 3. As tasks complete, dispatch dependent tasks
        // 4. Collect all results
        // 5. Return aggregated results
    }
}
```

### 17.4 Planning Strategies (L4)

**File**: `clawft-core/src/pipeline/planner.rs` (new)

Two planning strategies integrated into the pipeline Router:

```rust
/// Planning strategy for complex multi-step tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    /// No planning -- direct execution (default).
    None,
    /// ReAct: Reason + Act interleaved.
    /// The agent reasons about the next step, acts, observes, repeats.
    React,
    /// Plan-and-Execute: Generate full plan, then execute steps.
    /// The agent generates a complete plan first, then executes each step.
    PlanAndExecute,
}

/// ReAct loop state.
pub struct ReactLoop {
    /// Maximum reasoning iterations.
    max_iterations: usize,
    /// Thought-Action-Observation trace.
    trace: Vec<ReactStep>,
}

#[derive(Debug, Clone)]
pub struct ReactStep {
    /// The agent's reasoning.
    pub thought: String,
    /// The action taken (tool call or response).
    pub action: ReactAction,
    /// The observation from the action.
    pub observation: String,
}

#[derive(Debug, Clone)]
pub enum ReactAction {
    /// Invoke a tool.
    ToolCall { name: String, args: Value },
    /// Produce the final answer.
    FinalAnswer { text: String },
}

/// Plan-and-Execute strategy.
pub struct PlanExecutor {
    /// The generated plan steps.
    steps: Vec<PlanStep>,
    /// Execution state.
    current_step: usize,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Step description.
    pub description: String,
    /// Expected tool to use (if any).
    pub expected_tool: Option<String>,
    /// Whether this step has been completed.
    pub completed: bool,
    /// Result from execution.
    pub result: Option<String>,
}
```

**Integration point**: The `TaskClassifier` detects when a request requires
multi-step planning (complexity > 0.7 + multiple sub-tasks detected). The
`ModelRouter` selects the planning strategy based on the task profile. The
`AgentLoop` wraps tool execution in the chosen planning loop.

---

## 18. Claude Flow / Claude Code Integration (Workstream M)

### 18.1 FlowDelegator Implementation (M1)

**New file**: `clawft-services/src/delegation/flow.rs`

Implements delegation via the `claude` CLI (Claude Code) as a subprocess.

```rust
/// Delegator that spawns Claude Code CLI for task execution.
pub struct FlowDelegator {
    /// Path to the `claude` binary.
    claude_path: PathBuf,
    /// Whether to use JSON output mode.
    json_mode: bool,
    /// Maximum execution time.
    timeout: Duration,
    /// MCP server config for callback tools.
    mcp_callback_config: Option<McpCallbackConfig>,
}

/// Configuration for MCP callback (Claude Code calling back into clawft tools).
#[derive(Debug, Clone)]
struct McpCallbackConfig {
    /// Path to the weft binary (for `claude mcp add weft -- weft mcp-server`).
    weft_path: PathBuf,
}

impl FlowDelegator {
    /// Create a new FlowDelegator.
    ///
    /// Returns `None` if the `claude` binary is not found on PATH.
    pub fn detect() -> Option<Self> {
        let claude_path = which::which("claude").ok()?;
        Some(Self {
            claude_path,
            json_mode: true,
            timeout: Duration::from_secs(300),
            mcp_callback_config: None,
        })
    }

    /// Create with explicit path to claude binary.
    pub fn with_path(claude_path: PathBuf) -> Option<Self> {
        if claude_path.exists() {
            Some(Self {
                claude_path,
                json_mode: true,
                timeout: Duration::from_secs(300),
                mcp_callback_config: None,
            })
        } else {
            None
        }
    }

    /// Enable MCP callback so Claude Code can use clawft tools.
    pub fn with_mcp_callback(mut self, weft_path: PathBuf) -> Self {
        self.mcp_callback_config = Some(McpCallbackConfig { weft_path });
        self
    }

    /// Delegate a task to Claude Code.
    pub async fn delegate(
        &self,
        task: &str,
        tool_schemas: &[Value],
        tool_executor: impl Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync,
    ) -> Result<FlowDelegationResult> {
        let mut cmd = tokio::process::Command::new(&self.claude_path);

        if self.json_mode {
            cmd.arg("--output-format").arg("json");
        }

        cmd.arg("--print");
        cmd.arg("--max-turns").arg("10");

        // If MCP callback is configured, add clawft as an MCP server
        if let Some(mcp_config) = &self.mcp_callback_config {
            cmd.arg("--mcp-config")
                .arg(self.build_mcp_config_json(mcp_config)?);
        }

        // Pass the task via stdin
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| FlowError::SpawnFailed(e.to_string()))?;

        // Write task to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(task.as_bytes()).await?;
            stdin.shutdown().await?;
        }

        // Wait for completion with timeout
        let output = tokio::time::timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| FlowError::Timeout(self.timeout))?
            .map_err(|e| FlowError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FlowError::NonZeroExit {
                code: output.status.code(),
                stderr: stderr.to_string(),
            }.into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        if self.json_mode {
            let parsed: Value = serde_json::from_str(&stdout)
                .map_err(|e| FlowError::InvalidOutput(e.to_string()))?;
            Ok(FlowDelegationResult::from_json(parsed))
        } else {
            Ok(FlowDelegationResult {
                text: stdout.to_string(),
                tool_calls_made: 0,
                cost_usd: None,
            })
        }
    }

    /// Build a temporary MCP config JSON for Claude Code.
    fn build_mcp_config_json(&self, config: &McpCallbackConfig) -> Result<PathBuf> {
        let mcp_json = serde_json::json!({
            "mcpServers": {
                "clawft": {
                    "command": config.weft_path.to_str().unwrap(),
                    "args": ["mcp-server"]
                }
            }
        });

        let tmp_path = std::env::temp_dir().join("clawft-mcp-config.json");
        std::fs::write(&tmp_path, serde_json::to_string_pretty(&mcp_json)?)?;
        Ok(tmp_path)
    }
}

/// Result from a Flow delegation.
#[derive(Debug)]
pub struct FlowDelegationResult {
    /// The final text output.
    pub text: String,
    /// Number of tool calls Claude Code made.
    pub tool_calls_made: usize,
    /// Estimated cost in USD (if available from JSON output).
    pub cost_usd: Option<f64>,
}

/// Errors specific to Flow delegation.
#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("failed to spawn claude CLI: {0}")]
    SpawnFailed(String),

    #[error("execution timed out after {0:?}")]
    Timeout(Duration),

    #[error("claude CLI exited with code {code:?}: {stderr}")]
    NonZeroExit { code: Option<i32>, stderr: String },

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("invalid output: {0}")]
    InvalidOutput(String),
}
```

**Fallback chain**:

```
DelegationEngine::decide() returns DelegationTarget::Flow
  |
  v
FlowDelegator available?
  |           |
  YES         NO
  |           |
  v           v
Execute via   ClaudeDelegator available?
claude CLI      |           |
  |           YES         NO
  v           |           |
Return        v           v
result      Execute via   DelegationTarget::Local
            Anthropic     (handle in agent loop)
            API
```

### 18.2 Runtime flow_available Detection (M2)

**File**: `clawft-tools/src/delegate_tool.rs`

Replace hardcoded `false` with runtime detection:

```rust
use std::sync::OnceLock;

/// Cached result of claude CLI availability check.
static CLAUDE_CLI_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if the `claude` CLI is available on PATH.
fn detect_claude_cli() -> bool {
    *CLAUDE_CLI_AVAILABLE.get_or_init(|| {
        match which::which("claude") {
            Ok(path) => {
                tracing::info!(path = %path.display(), "claude CLI detected");
                true
            }
            Err(_) => {
                tracing::debug!("claude CLI not found on PATH");
                false
            }
        }
    })
}

/// Determine if Flow delegation is available.
fn check_flow_available(config: &DelegationConfig) -> bool {
    if !config.claude_flow_enabled {
        return false;
    }
    detect_claude_cli()
}
```

**Updated usage in delegate_tool.rs**:

```rust
// BEFORE:
let flow_available = false; // Flow not wired yet.

// AFTER:
let flow_available = check_flow_available(self.engine.config());
```

### 18.3 Dynamic MCP Server Discovery (M4)

**File**: `clawft-cli/src/commands/mcp.rs` (new), `clawft-services/src/mcp/discovery.rs` (new)

Runtime management of MCP server connections:

```rust
/// MCP server discovery and lifecycle manager.
pub struct McpDiscoveryService {
    /// Active MCP client pool.
    pool: Arc<McpClientPool>,
    /// Config file watcher (for hot-reload).
    watcher: Option<notify::RecommendedWatcher>,
    /// Health check interval.
    health_interval: Duration,
}

impl McpDiscoveryService {
    /// Start the discovery service.
    pub async fn start(&mut self, cancel: CancellationToken) -> Result<()> {
        // 1. Load initial MCP server configs from config.json
        // 2. Connect to all auto_connect servers
        // 3. Start health check loop
        // 4. Start config file watcher for hot-reload
        // 5. Wait for cancellation
    }

    /// Handle config file change (hot-reload).
    async fn on_config_change(&self, new_config: &HashMap<String, McpClientConfig>) -> Result<()> {
        let current = self.pool.list_servers().await;
        let current_names: HashSet<_> = current.iter().map(|s| &s.name).collect();
        let new_names: HashSet<_> = new_config.keys().collect();

        // Add new servers
        for name in new_names.difference(&current_names) {
            if let Some(config) = new_config.get(*name) {
                self.pool.add_server(name, config.clone()).await?;
            }
        }

        // Remove deleted servers
        for name in current_names.difference(&new_names) {
            self.pool.remove_server(name).await?;
        }

        Ok(())
    }

    /// Health check loop: ping servers and reconnect failed ones.
    async fn health_check_loop(&self, cancel: CancellationToken) {
        let mut interval = tokio::time::interval(self.health_interval);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = interval.tick() => {
                    for server in self.pool.list_servers().await {
                        if let Err(e) = self.pool.ping(&server.name).await {
                            tracing::warn!(
                                server = %server.name,
                                error = %e,
                                "MCP server health check failed, attempting reconnect"
                            );
                            let _ = self.pool.reconnect(&server.name).await;
                        }
                    }
                }
            }
        }
    }
}
```

### 18.4 Bidirectional MCP Bridge (M5)

**Outbound** (clawft as MCP server for Claude Code): Already partially working
via `McpServerShell`. Needs documentation and testing.

**Inbound** (clawft as MCP client of Claude Code): Uses the MCP client from F9.

```
+----------------------------------------------+
|              Bidirectional MCP Bridge         |
|                                               |
|  +---------+          +---------+             |
|  | clawft  |  <-----> | Claude  |             |
|  | MCP     |  tools   | Code    |             |
|  | Server  |  ------>  | (MCP    |             |
|  |         |          | client) |             |
|  +---------+          +---------+             |
|       ^                    |                  |
|       |                    v                  |
|  +---------+          +---------+             |
|  | clawft  |  <-----> | Claude  |             |
|  | MCP     |  tools   | Code    |             |
|  | Client  |  <------  | (MCP    |             |
|  |         |          | server) |             |
|  +---------+          +---------+             |
|                                               |
+----------------------------------------------+
```

**Configuration**:

```toml
# clawft.toml -- bidirectional MCP bridge
[tools.mcp_servers.claude-code]
transport.type = "stdio"
transport.command = "claude"
transport.args = ["mcp", "serve"]
auto_connect = true
tool_prefix = "cc_"

# Claude Code side (claude_desktop_config.json or .claude/settings.json):
# {
#   "mcpServers": {
#     "clawft": {
#       "command": "weft",
#       "args": ["mcp-server"]
#     }
#   }
# }
```

### 18.5 Delegation Config Documentation Requirements (M6)

The delegation system must be documented in `docs/guides/configuration.md` and
`docs/guides/tool-calls.md`. Required documentation sections:

1. **Enabling delegation**: Feature flags, config toggles (`claude_enabled`,
   `claude_flow_enabled`)
2. **Writing routing rules**: Regex patterns, target types (`local`, `claude`,
   `flow`, `auto`)
3. **Excluded tools**: How to prevent specific tools from being available to
   delegates
4. **Claude Code integration**: PATH setup, API key configuration, MCP bridge
   setup
5. **Troubleshooting**: Common failure modes and diagnostic commands

---

## Additions to Existing Sections

### Feature Flags (Addition to Section 4)

```toml
# clawft-cli/Cargo.toml -- new feature flags
[features]
default = ["channels", "services", "delegate"]  # delegate now default (M3)

# New channel plugins
channel-email = ["clawft-channels/email"]
channel-whatsapp = ["clawft-channels/whatsapp"]
channel-signal = ["clawft-channels/signal"]
channel-matrix = ["clawft-channels/matrix"]
channel-irc = ["clawft-channels/irc"]
channel-google-chat = ["clawft-channels/google-chat"]
channel-teams = ["clawft-channels/teams"]
all-channels = [
    "channels",
    "channel-email", "channel-whatsapp", "channel-signal",
    "channel-matrix", "channel-irc", "channel-google-chat", "channel-teams",
]

# New tool plugins
tool-git = ["clawft-tools/git"]
tool-browser = ["clawft-tools/browser"]
tool-code-analysis = ["clawft-tools/code-analysis"]
all-tools = [
    "tool-exec", "tool-web", "tool-spawn", "tool-cron",
    "tool-git", "tool-browser", "tool-code-analysis",
]

# Delegation (now default)
delegate = ["clawft-services/delegate", "clawft-tools/delegate"]

# clawft-channels/Cargo.toml -- new feature flags
[features]
email = ["dep:lettre", "dep:imap", "dep:oauth2", "dep:mailparse"]
whatsapp = ["dep:axum"]
signal = []   # No extra deps (subprocess-based)
matrix = ["dep:matrix-sdk"]
irc = ["dep:irc"]
google-chat = ["dep:oauth2"]
teams = ["dep:oauth2"]

# clawft-tools/Cargo.toml -- new feature flags
[features]
git = ["dep:git2"]
browser = ["dep:chromiumoxide"]
code-analysis = ["dep:tree-sitter", "dep:tree-sitter-rust"]
delegate = ["dep:clawft-services"]
```

### Testing Strategy (Addition to Section 9)

| Level | Tool | Scope |
|-------|------|-------|
| Plugin system | Feature flag test | Each plugin compiles independently behind its flag |
| Channel plugin | Mock ChannelHost | Email IMAP/SMTP mock, WhatsApp webhook mock |
| Channel resume | Mock WebSocket | Discord OP 6 resume + OP 9 invalid session |
| Tool plugin | Unit test | Git, browser, code-analysis operations |
| Multi-agent routing | Integration test | Agent routing table resolves correctly |
| Multi-agent isolation | Integration test | Agents have separate session stores |
| Swarm orchestration | Integration test | Coordinator dispatches and collects results |
| Delegation - Claude | Mock HTTP | ClaudeDelegator multi-turn loop |
| Delegation - Flow | Mock subprocess | FlowDelegator spawns and parses output |
| MCP client | Mock MCP server | Tool discovery, invocation, reconnection |
| MCP bidirectional | Integration test | clawft <-> Claude Code tool exchange |
| Security plugin | Audit test | 50+ rules detect known vulnerability patterns |
| Sandbox | seccomp test | Blocked syscalls produce clean errors |
| Docker | CI image build | Multi-arch build, image size assertion |
| Planning strategy | Unit test | ReAct loop, Plan-and-Execute step tracking |
| Bounded bus | Load test | Backpressure behavior under burst traffic |
| Stream failover | Integration test | Mid-stream failure produces clean reset |
| Retry policy | Unit test | Exponential backoff, jitter, status code filtering |

### New Workspace Dependencies (Addition to Section 4)

```toml
# Cargo.toml workspace additions for new workstreams
[workspace.dependencies]
# Parallel tool execution (D1)
futures = "0.3"

# Email channel (E2)
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport"], optional = true }
imap = { version = "3.0", optional = true }
mailparse = { version = "0.15", optional = true }

# OAuth2 helper (F6, E2, E5a, E5b)
oauth2 = { version = "4.4", optional = true }

# Git tool (F1)
git2 = { version = "0.19", optional = true }

# Code analysis (F3)
tree-sitter = { version = "0.24", optional = true }
tree-sitter-rust = { version = "0.23", optional = true }
tree-sitter-python = { version = "0.23", optional = true }

# Browser automation (F4)
chromiumoxide = { version = "0.7", features = ["tokio-runtime"], optional = true }

# Matrix channel (E5)
matrix-sdk = { version = "0.7", optional = true }

# HNSW vector search (H2)
instant-distance = { version = "0.6", optional = true }

# Claude CLI detection (M2)
which = "7.0"

# File watcher for hot-reload (M4, C4)
notify = { version = "7.0", features = ["macos_kqueue"] }

# HTTP server for webhooks (E3 WhatsApp, E5a Google Chat, E5b Teams)
axum = { version = "0.7", features = ["tokio"], optional = true }
```

---

## Cross-Cutting Implementation Notes

### Binary Size Impact Projections

| Addition | Estimated Size Impact | Feature Flag |
|----------|----------------------|-------------|
| `futures` (D1) | ~50 KB | always (core) |
| `lettre` + `imap` (E2) | ~400 KB | `email` |
| `git2` + `libgit2` (F1) | ~2 MB | `git` |
| `tree-sitter` (F3) | ~800 KB | `code-analysis` |
| `chromiumoxide` (F4) | ~300 KB | `browser` |
| `oauth2` (F6) | ~150 KB | shared by email, chat, teams |
| `instant-distance` (H2) | ~100 KB | `vector-memory` |
| `matrix-sdk` (E5) | ~1 MB | `matrix` |
| `axum` (E3, E5a, E5b) | ~400 KB | `whatsapp`, `google-chat`, `teams` |
| `which` (M2) | ~20 KB | `delegate` |

**Default binary impact** (only `futures` + `which` added to defaults): ~70 KB.
All heavy dependencies behind feature flags, preserving the <10 MB base binary target.

### Migration Path for Bounded Bus (D8)

The change from `mpsc::unbounded_channel` to `mpsc::channel` is a breaking
change for callers using `UnboundedSender`. Migration:

1. `MessageBus::new()` continues to work (uses default buffer sizes)
2. `MessageBus::inbound_sender()` returns `mpsc::Sender` instead of `mpsc::UnboundedSender`
3. All `.send()` calls become `.send().await` (bounded send is async)
4. Channel plugins must handle `SendError` (buffer full) gracefully

### Error Type Migration (D3)

The `ProviderError` enum replaces string-based error checking. Migration:

1. `clawft-llm` response parsers construct typed `ProviderError` variants
2. `retry.rs` matches on `ProviderError::is_retryable()` instead of string prefixes
3. `ClawftLlmAdapter` maps `ProviderError` to `ClawftError::Provider(ProviderError)`
4. Pipeline stages pattern-match on error variants for specific handling
