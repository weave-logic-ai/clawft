# SPARC Task: Element 06 -- Phase E-Enterprise

| Field | Value |
|-------|-------|
| **Element** | 06 -- Channel Enhancements |
| **Phase** | E-Enterprise (Email, Google Chat, Microsoft Teams) |
| **Timeline** | Week 5-7 |
| **Priority** | E2 = P0 (MVP), E5a = P1, E5b = P1 |
| **Crates** | `clawft-channels`, `clawft-plugin` (C1 ChannelAdapter trait) |
| **Dependencies** | 04/C1 (ChannelAdapter trait), 03/A4 (SecretRef credentials), 07/F6 (OAuth2 helper -- E5a blocked until available) |
| **Status** | Planning |

---

## 1. Overview

Phase E-Enterprise adds three enterprise-grade communication channels to clawft:

1. **E2 -- Email (IMAP + SMTP)** [P0 MVP]: Full bidirectional email channel supporting Gmail OAuth2 and generic IMAP/SMTP with password auth. Polls a mailbox for inbound messages and sends replies via SMTP.
2. **E5a -- Google Chat**: Google Workspace Chat API integration via REST. Requires OAuth2 service account or user-delegated flow. Blocked on F6 (OAuth2 helper).
3. **E5b -- Microsoft Teams**: Microsoft Bot Framework / Graph API integration. Uses Azure AD client credentials or certificate-based auth.

All three channels implement the new `ChannelAdapter` trait from `clawft-plugin` (C1), NOT the legacy `Channel` trait from `clawft-channels/src/traits.rs`. During the transition period, a `ChannelAdapter->Channel` shim in `clawft-plugin` allows new adapters to be loaded by the existing `PluginHost`.

---

## 2. Architecture

### 2.1 Trait Hierarchy

```text
                  clawft-plugin (C1)
                  ┌──────────────────────┐
                  │  trait ChannelAdapter │  <-- New channels implement THIS
                  │    fn name()         │
                  │    fn metadata()     │
                  │    fn status()       │
                  │    fn is_allowed()   │
                  │    async fn start()  │
                  │    async fn send()   │
                  └──────────┬───────────┘
                             │
              ┌──────────────┼──────────────────┐
              │              │                  │
         EmailChannel   GoogleChatChannel   TeamsChannel
              │              │                  │
              ▼              ▼                  ▼
     ChannelAdapter->Channel shim (clawft-plugin)
              │              │                  │
              ▼              ▼                  ▼
          PluginHost (clawft-channels/src/host.rs)
```

### 2.2 Message Flow (All Enterprise Channels)

```text
  External Service          Channel Impl               PluginHost
  ────────────────         ─────────────              ──────────────
  Inbound email   ──IMAP──>  EmailChannel
  GChat message   ──REST──>  GoogleChatChannel   ──deliver_inbound()──>  Agent Pipeline
  Teams activity  ──REST──>  TeamsChannel

  Agent Pipeline  ──send()──>  Channel.send()
                                    │
                              ┌─────┼──────┐
                              │     │      │
                            SMTP   REST   REST
                              │     │      │
                              ▼     ▼      ▼
                           Email  GChat  Teams
```

### 2.3 Module Layout (Per Channel)

Each new channel follows the Telegram module pattern:

```text
crates/clawft-channels/src/<channel_name>/
  ├── mod.rs          -- Module re-exports
  ├── channel.rs      -- ChannelAdapter impl, start() loop, process logic
  ├── client.rs       -- HTTP/protocol client (IMAP/SMTP, REST API)
  │   (or api.rs)
  ├── types.rs        -- Request/response serde types
  ├── factory.rs      -- ChannelFactory impl (parses JSON config)
  └── tests.rs        -- Unit tests (MockHost pattern from Telegram)
```

---

## 3. Current Code Reference: Telegram Channel Pattern

The existing Telegram implementation (`crates/clawft-channels/src/telegram/`) defines the pattern all new channels follow. Key structural points:

### 3.1 Channel Struct (channel.rs:52-63)

```rust
pub struct TelegramChannel {
    client: TelegramClient,           // Protocol-specific client
    status: Arc<RwLock<ChannelStatus>>,// Lifecycle tracking
    offset: AtomicI64,                 // Cursor for polling
    allowed_users: Vec<String>,        // Allow-list filter
    poll_interval_secs: u64,           // Configurable poll rate
}
```

### 3.2 Start Loop Pattern (channel.rs:184-266)

```rust
async fn start(&self, host: Arc<dyn ChannelHost>, cancel: CancellationToken) -> Result<(), ChannelError> {
    self.set_status(ChannelStatus::Starting).await;
    // 1. Verify credentials (get_me / auth check)
    // 2. Set status to Running
    // 3. Long-poll loop with tokio::select! on cancel
    //    - On success: process each message, advance cursor
    //    - On error: set Error status, backoff, retry
    // 4. On cancel: set Stopped, return Ok
}
```

### 3.3 Message Processing (channel.rs:95-154)

```rust
async fn process_update(&self, update: &Update, host: &Arc<dyn ChannelHost>) -> Result<(), ChannelError> {
    // 1. Extract text from update (skip non-text)
    // 2. Check is_allowed(sender_id)
    // 3. Build InboundMessage with metadata HashMap
    // 4. Call host.deliver_inbound(inbound)
}
```

### 3.4 Factory Pattern (channel.rs:307-346)

```rust
impl ChannelFactory for TelegramChannelFactory {
    fn channel_name(&self) -> &str { "telegram" }
    fn build(&self, config: &serde_json::Value) -> Result<Arc<dyn Channel>, ChannelError> {
        // 1. Parse token (direct value or env var via token_env)
        // 2. Parse allowed_users (default empty = all allowed)
        // 3. Return Arc::new(TelegramChannel::new(...))
    }
}
```

### 3.5 Test Pattern (tests.rs)

- `MockHost` struct with `tokio::sync::Mutex<Vec<InboundMessage>>` for assertions
- Unit tests for: `is_allowed`, `metadata`, `name`, `status`, `send` error cases, `factory` build/error cases, `process_update` with various message shapes

---

## 4. Implementation Tasks

### 4.1 E2: Email Channel (IMAP + SMTP) -- P0 MVP

#### 4.1.1 New Files

| File | Purpose |
|------|---------|
| `crates/clawft-channels/src/email/mod.rs` | Module declaration, re-exports |
| `crates/clawft-channels/src/email/channel.rs` | `EmailChannel` struct, `ChannelAdapter` impl |
| `crates/clawft-channels/src/email/imap_client.rs` | IMAP connection, mailbox polling, message fetch |
| `crates/clawft-channels/src/email/smtp_client.rs` | SMTP connection, message sending |
| `crates/clawft-channels/src/email/types.rs` | `EmailMessage`, `EmailEnvelope`, config serde types |
| `crates/clawft-channels/src/email/factory.rs` | `EmailChannelFactory` -- parses JSON config |
| `crates/clawft-channels/src/email/tests.rs` | Unit tests (MockHost pattern) |

#### 4.1.2 Cargo.toml Changes

In `crates/clawft-channels/Cargo.toml`, add feature-gated dependencies:

```toml
[dependencies]
# ... existing ...
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport"], optional = true }
imap = { version = "3", optional = true }
oauth2 = { version = "4", optional = true }

[features]
default = []
email = ["lettre", "imap", "oauth2"]
```

In `crates/clawft-channels/src/lib.rs`, add feature-gated module:

```rust
#[cfg(feature = "email")]
pub mod email;
```

#### 4.1.3 Config Shape

```json
{
  "imap_host": "imap.gmail.com",
  "imap_port": 993,
  "smtp_host": "smtp.gmail.com",
  "smtp_port": 587,
  "email_address": "user@gmail.com",
  "auth": {
    "type": "oauth2",
    "client_id_env": "GMAIL_CLIENT_ID",
    "client_secret_env": "GMAIL_CLIENT_SECRET",
    "refresh_token_env": "GMAIL_REFRESH_TOKEN"
  },
  "mailbox": "INBOX",
  "poll_interval_secs": 60,
  "allowed_senders": ["boss@company.com"],
  "subject_filter": "clawft:",
  "max_body_bytes": 65536
}
```

Alternative auth for non-OAuth providers:

```json
{
  "auth": {
    "type": "password",
    "password_env": "EMAIL_PASSWORD"
  }
}
```

All credential fields use `SecretRef` type from A4 -- never plaintext strings.

#### 4.1.4 Detailed Implementation

**`email/types.rs`**:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub email_address: String,
    pub auth: EmailAuth,
    #[serde(default = "default_mailbox")]
    pub mailbox: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default)]
    pub allowed_senders: Vec<String>,
    pub subject_filter: Option<String>,
    #[serde(default = "default_max_body")]
    pub max_body_bytes: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EmailAuth {
    OAuth2 {
        client_id_env: String,     // SecretRef -- env var name, not value
        client_secret_env: String, // SecretRef
        refresh_token_env: String, // SecretRef
    },
    Password {
        password_env: String,      // SecretRef -- env var name, not value
    },
}

#[derive(Debug, Clone)]
pub struct EmailEnvelope {
    pub message_id: String,
    pub from: String,
    pub subject: String,
    pub body: String,
    pub date: chrono::DateTime<chrono::Utc>,
    pub in_reply_to: Option<String>,
}

fn default_mailbox() -> String { "INBOX".into() }
fn default_poll_interval() -> u64 { 60 }
fn default_max_body() -> usize { 65536 }
```

**`email/imap_client.rs`**:
```rust
pub struct ImapClient {
    config: EmailConfig,
    // IMAP session state
}

impl ImapClient {
    pub fn new(config: EmailConfig) -> Self { ... }

    /// Connect to IMAP server using TLS.
    /// Authenticates with password or OAuth2 XOAUTH2 SASL.
    pub async fn connect(&mut self) -> Result<(), ChannelError> { ... }

    /// Fetch unseen messages from configured mailbox.
    /// Returns Vec<EmailEnvelope> for new messages since last check.
    /// Marks fetched messages as \Seen.
    pub async fn fetch_unseen(&mut self) -> Result<Vec<EmailEnvelope>, ChannelError> { ... }

    /// IDLE wait for new messages (if server supports IDLE).
    /// Falls back to poll_interval_secs sleep if not.
    pub async fn idle_or_sleep(&mut self, cancel: &CancellationToken) -> Result<(), ChannelError> { ... }
}
```

**`email/smtp_client.rs`**:
```rust
pub struct SmtpClient {
    config: EmailConfig,
}

impl SmtpClient {
    pub fn new(config: EmailConfig) -> Self { ... }

    /// Send a reply email.
    /// Sets In-Reply-To and References headers for threading.
    pub async fn send_reply(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<&str>,
    ) -> Result<String, ChannelError> { ... }
}
```

**`email/channel.rs`** -- Core implementation:
```rust
pub struct EmailChannel {
    imap: tokio::sync::Mutex<ImapClient>,
    smtp: SmtpClient,
    config: EmailConfig,
    status: Arc<RwLock<ChannelStatus>>,
}

// ChannelAdapter impl (from clawft-plugin C1)
// Pattern mirrors Telegram but with IMAP polling instead of HTTP long-poll:
//
// start():
//   1. imap.connect() -- verify credentials
//   2. set Running
//   3. Loop:
//      a. imap.fetch_unseen()
//      b. For each email: check allowed_senders, build InboundMessage, host.deliver_inbound()
//      c. imap.idle_or_sleep(cancel) -- wait for new mail or poll interval
//      d. On cancel: break
//   4. set Stopped
//
// send():
//   1. Parse chat_id as email address
//   2. smtp.send_reply(to, subject, content, reply_to)
//   3. Return MessageId(<smtp-message-id>)
```

**InboundMessage mapping** (email -> pipeline):
| Email Field | InboundMessage Field |
|-------------|---------------------|
| `from` address | `sender_id` |
| `from` address | `chat_id` (1:1 conversation = sender address) |
| Body text (truncated to `max_body_bytes`) | `content` |
| `chrono::Utc::now()` | `timestamp` |
| `message_id`, `subject`, `in_reply_to`, `date` | `metadata` HashMap entries |

#### 4.1.5 Security Requirements (E2)

- All config credential fields (`client_id_env`, `client_secret_env`, `refresh_token_env`, `password_env`) store env var names via `SecretRef` (A4), not plaintext values
- OAuth2 flows MUST include `state` parameter for CSRF protection
- OAuth2 token refresh persists rotated tokens to `~/.clawft/tokens/` with file permissions `0600`
- Custom `Debug` impl on `EmailConfig` MUST redact all auth fields
- Email body content truncated at `max_body_bytes` to prevent memory exhaustion
- IMAP connections use TLS (port 993); SMTP uses STARTTLS (port 587) -- no cleartext fallback
- Subject-based filtering (`subject_filter`) to limit processing scope

#### 4.1.6 Cross-Element Dependencies (E2)

| Dependency | Element | Status | Impact |
|-----------|---------|--------|--------|
| A4 (SecretRef) | 03 | Required | All credential fields must use SecretRef type |
| E6 (Heartbeat triage) | 06/E-Fix | Week 4-5 | Email triage uses heartbeat for proactive check-ins |
| F6 (OAuth2 helper) | 07 | Week 7-9 | Gmail OAuth2 flow; E2 can ship with password auth first |

---

### 4.2 E5a: Google Chat Channel

#### 4.2.1 New Files

| File | Purpose |
|------|---------|
| `crates/clawft-channels/src/google_chat/mod.rs` | Module declaration, re-exports |
| `crates/clawft-channels/src/google_chat/channel.rs` | `GoogleChatChannel` struct, `ChannelAdapter` impl |
| `crates/clawft-channels/src/google_chat/api.rs` | Google Workspace Chat API REST client |
| `crates/clawft-channels/src/google_chat/types.rs` | API request/response serde types |
| `crates/clawft-channels/src/google_chat/factory.rs` | `GoogleChatChannelFactory` -- parses JSON config |
| `crates/clawft-channels/src/google_chat/tests.rs` | Unit tests |

#### 4.2.2 Cargo.toml Changes

```toml
[dependencies]
# ... existing ...
# google-chat shares reqwest (already a workspace dep) + oauth2
# oauth2 is already added under email feature

[features]
google-chat = ["oauth2"]
```

In `crates/clawft-channels/src/lib.rs`:

```rust
#[cfg(feature = "google-chat")]
pub mod google_chat;
```

#### 4.2.3 Config Shape

```json
{
  "project_id": "my-gcp-project",
  "space_name": "spaces/AAAA-BBBB",
  "auth": {
    "type": "service_account",
    "credentials_file_env": "GOOGLE_APPLICATION_CREDENTIALS"
  },
  "notification_mode": "pubsub",
  "pubsub_subscription": "projects/my-project/subscriptions/gchat-sub",
  "poll_interval_secs": 10,
  "allowed_senders": ["user@company.com"]
}
```

Alternative auth (user-delegated):

```json
{
  "auth": {
    "type": "oauth2_user",
    "client_id_env": "GCHAT_CLIENT_ID",
    "client_secret_env": "GCHAT_CLIENT_SECRET",
    "refresh_token_env": "GCHAT_REFRESH_TOKEN"
  }
}
```

#### 4.2.4 Detailed Implementation

**Transport**: Google Workspace Chat API v1 (REST)

**Inbound message reception** (two modes):
1. **Pub/Sub push** (preferred): Google Chat publishes events to a Cloud Pub/Sub topic. The channel subscribes to the topic and processes incoming messages.
2. **Polling fallback**: `spaces.messages.list` with a timestamp cursor, polled at `poll_interval_secs`.

**Outbound**: `POST https://chat.googleapis.com/v1/{space}/messages` with `Bearer` token.

**`google_chat/api.rs`**:
```rust
pub struct GoogleChatApi {
    http: reqwest::Client,
    base_url: String,  // https://chat.googleapis.com/v1
    // OAuth2 token provider (from F6 helper)
}

impl GoogleChatApi {
    /// List messages in a space since a given timestamp.
    pub async fn list_messages(
        &self, space: &str, since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ChatMessage>, ChannelError> { ... }

    /// Send a message to a space.
    pub async fn send_message(
        &self, space: &str, text: &str, thread_key: Option<&str>,
    ) -> Result<ChatMessage, ChannelError> { ... }
}
```

**`google_chat/channel.rs`** -- Start loop:
```text
start():
  1. Authenticate via F6 OAuth2 helper (service account or user flow)
  2. Set Running
  3. Loop (poll or Pub/Sub):
     a. Receive new messages
     b. For each: check allowed_senders, build InboundMessage, deliver_inbound()
     c. Sleep or await next Pub/Sub message
     d. On cancel: break
  4. Set Stopped
```

**InboundMessage mapping** (Google Chat -> pipeline):
| Chat API Field | InboundMessage Field |
|---------------|---------------------|
| `sender.name` (users/USER_ID) | `sender_id` |
| `space.name` (spaces/SPACE_ID) | `chat_id` |
| `text` | `content` |
| `createTime` | `timestamp` |
| `thread.name`, `name` (message ID) | `metadata` |

#### 4.2.5 TIMELINE RISK (E5a)

**F6 (OAuth2 helper) dependency conflict:**

F6 is scheduled for Element 07, Week 7-9. E5a is scheduled Week 5-7. This creates a direct conflict.

**Resolution options (choose ONE):**

| Option | Pro | Con | Recommendation |
|--------|-----|-----|----------------|
| 1. Coordinate with Element 07 to accelerate F6 to Week 5 | E5a stays on schedule | Pressure on Element 07 | Preferred if feasible |
| 2. Defer E5a to Week 8+ | No cross-element pressure | Delays enterprise delivery | Acceptable fallback |
| 3. Implement standalone OAuth2 in E5a | No external dependency | Duplicates F6, wastes effort, technical debt | **DO NOT DO THIS** |

**Decision required before Week 5 sprint planning.**

---

### 4.3 E5b: Microsoft Teams Channel

#### 4.3.1 New Files

| File | Purpose |
|------|---------|
| `crates/clawft-channels/src/teams/mod.rs` | Module declaration, re-exports |
| `crates/clawft-channels/src/teams/channel.rs` | `TeamsChannel` struct, `ChannelAdapter` impl |
| `crates/clawft-channels/src/teams/api.rs` | Microsoft Graph API / Bot Framework REST client |
| `crates/clawft-channels/src/teams/types.rs` | Activity, Conversation, Message serde types |
| `crates/clawft-channels/src/teams/factory.rs` | `TeamsChannelFactory` -- parses JSON config |
| `crates/clawft-channels/src/teams/tests.rs` | Unit tests |

#### 4.3.2 Cargo.toml Changes

```toml
[features]
teams = ["oauth2"]
```

In `crates/clawft-channels/src/lib.rs`:

```rust
#[cfg(feature = "teams")]
pub mod teams;
```

#### 4.3.3 Config Shape

```json
{
  "app_id_env": "TEAMS_APP_ID",
  "app_secret_env": "TEAMS_APP_SECRET",
  "tenant_id": "your-tenant-id",
  "bot_endpoint": "https://your-domain.com/api/messages",
  "allowed_senders": ["user@company.com"],
  "notification_url": "https://your-domain.com/webhooks/teams"
}
```

#### 4.3.4 Detailed Implementation

**Transport**: Microsoft Bot Framework v3 + Microsoft Graph API

**Inbound message reception**:
- Bot Framework webhook: Azure Bot Service forwards activities (messages) to `bot_endpoint`
- The channel starts an HTTP listener (or registers with the gateway HTTP server) at the configured endpoint
- Incoming POST requests contain Bot Framework Activity JSON

**Outbound**: Graph API `POST /chats/{id}/messages` or Bot Framework `POST` to service URL with `Bearer` token.

**Authentication**: Azure AD client credentials flow:
```text
POST https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token
  grant_type=client_credentials
  client_id={app_id}
  client_secret={app_secret}
  scope=https://graph.microsoft.com/.default
```

**`teams/api.rs`**:
```rust
pub struct TeamsApi {
    http: reqwest::Client,
    graph_base: String,  // https://graph.microsoft.com/v1.0
    tenant_id: String,
    app_id: String,
    // Token cache with auto-refresh
}

impl TeamsApi {
    /// Acquire Azure AD token via client credentials flow.
    pub async fn authenticate(&mut self) -> Result<(), ChannelError> { ... }

    /// Send a message to a Teams chat/channel.
    pub async fn send_message(
        &self, chat_id: &str, text: &str, reply_to: Option<&str>,
    ) -> Result<TeamsMessage, ChannelError> { ... }

    /// Process incoming Bot Framework activity.
    pub fn parse_activity(&self, body: &[u8]) -> Result<Activity, ChannelError> { ... }
}
```

**`teams/channel.rs`** -- Start loop:
```text
start():
  1. Authenticate with Azure AD (client credentials)
  2. Set Running
  3. Start webhook listener (or register route with gateway HTTP server)
  4. Loop:
     a. Await incoming Activity from webhook
     b. Validate Activity (check HMAC signature from Bot Framework)
     c. For message activities: check allowed_senders, build InboundMessage, deliver_inbound()
     d. On cancel: stop listener, break
  5. Set Stopped
```

**InboundMessage mapping** (Teams -> pipeline):
| Teams Field | InboundMessage Field |
|------------|---------------------|
| `from.id` (AAD object ID) | `sender_id` |
| `conversation.id` | `chat_id` |
| `text` (stripped of `<at>` mentions) | `content` |
| `timestamp` | `timestamp` |
| `id` (activity ID), `conversation.conversationType`, `channelId` | `metadata` |

#### 4.3.5 Security Requirements (E5b)

- Azure AD app credentials (`app_id_env`, `app_secret_env`) use `SecretRef` (A4)
- Bot Framework webhook validates HMAC signature on incoming activities (prevents spoofing)
- Token cache handles expiry and refresh automatically
- `<at>` mention tags stripped from message content before delivery (prevents injection)

---

## 5. Security Checklist (All Enterprise Channels)

| Requirement | E2 Email | E5a Google Chat | E5b Teams |
|------------|----------|-----------------|-----------|
| All credential fields use `SecretRef` (A4) | Yes | Yes | Yes |
| No plaintext secrets in config structs | Yes | Yes | Yes |
| OAuth2 `state` parameter for CSRF | Yes (Gmail OAuth2) | Yes (user-delegated flow) | N/A (client credentials) |
| Token refresh persists encrypted (0600 perms) | Yes (`~/.clawft/tokens/`) | Yes (via F6) | Yes (in-memory cache, short-lived) |
| Custom `Debug` redacts sensitive fields | Yes | Yes | Yes |
| TLS/STARTTLS for all connections | Yes (IMAP TLS, SMTP STARTTLS) | Yes (HTTPS) | Yes (HTTPS) |
| Input size limits | Yes (`max_body_bytes`) | Yes (API response limits) | Yes (Activity size limits) |
| Webhook signature validation | N/A | N/A (Pub/Sub auth) | Yes (Bot Framework HMAC) |
| Existing channel tests pass (regression) | Yes | Yes | Yes |

---

## 6. Tests Required

### 6.1 E2: Email Channel Tests

| Test | Type | Description |
|------|------|-------------|
| `factory_build_oauth2_config` | Unit | Factory parses OAuth2 config correctly |
| `factory_build_password_config` | Unit | Factory parses password auth config correctly |
| `factory_build_missing_imap_host_errors` | Unit | Missing required fields produce ChannelError |
| `factory_build_invalid_auth_type_errors` | Unit | Unknown auth type produces ChannelError |
| `is_allowed_empty_senders_allows_all` | Unit | Empty allowed_senders permits all senders |
| `is_allowed_filters_unlisted_sender` | Unit | Sender not in list is rejected |
| `process_email_delivers_inbound` | Unit | Email envelope converted to InboundMessage correctly |
| `process_email_skips_disallowed_sender` | Unit | Disallowed sender email is dropped |
| `process_email_truncates_large_body` | Unit | Body exceeding max_body_bytes is truncated |
| `process_email_respects_subject_filter` | Unit | Non-matching subjects are skipped |
| `send_constructs_reply_email` | Unit | OutboundMessage mapped to SMTP message correctly |
| `send_sets_reply_headers` | Unit | In-Reply-To and References headers set |
| `metadata_values` | Unit | name="email", display_name, supports_threads=true |
| `initial_status_is_stopped` | Unit | Channel starts in Stopped state |
| `imap_connect_tls_required` | Integration | Connection fails without TLS |
| `oauth2_token_refresh_persists` | Integration | Rotated token saved to disk |

### 6.2 E5a: Google Chat Channel Tests

| Test | Type | Description |
|------|------|-------------|
| `factory_build_service_account_config` | Unit | Factory parses service account config |
| `factory_build_oauth2_user_config` | Unit | Factory parses user-delegated config |
| `factory_build_missing_space_errors` | Unit | Missing space_name produces error |
| `is_allowed_filters_senders` | Unit | allowed_senders check works |
| `process_chat_message_delivers_inbound` | Unit | Chat message -> InboundMessage mapping |
| `send_constructs_api_request` | Unit | OutboundMessage -> API request mapping |
| `send_with_thread_key` | Unit | Thread key passed to API call |
| `metadata_values` | Unit | name="google-chat", supports_threads=true |
| `api_list_messages_pagination` | Unit | Handles paginated responses |
| `api_auth_token_refresh` | Unit | Expired token triggers refresh |

### 6.3 E5b: Microsoft Teams Channel Tests

| Test | Type | Description |
|------|------|-------------|
| `factory_build_config` | Unit | Factory parses Teams config correctly |
| `factory_build_missing_app_id_errors` | Unit | Missing app_id_env produces error |
| `factory_build_missing_tenant_errors` | Unit | Missing tenant_id produces error |
| `is_allowed_filters_senders` | Unit | allowed_senders check works |
| `parse_activity_message` | Unit | Bot Framework Activity parsed correctly |
| `parse_activity_non_message_skipped` | Unit | Non-message activities ignored |
| `parse_activity_invalid_json_errors` | Unit | Malformed JSON produces error |
| `process_activity_delivers_inbound` | Unit | Activity -> InboundMessage mapping |
| `process_activity_strips_at_mentions` | Unit | `<at>` tags removed from content |
| `send_constructs_graph_request` | Unit | OutboundMessage -> Graph API request |
| `metadata_values` | Unit | name="teams", supports_threads=true |
| `azure_ad_auth_token_request` | Unit | Client credentials token request format |
| `webhook_hmac_validation` | Unit | Valid HMAC passes, invalid rejected |

### 6.4 Regression Tests

| Test | Type | Description |
|------|------|-------------|
| `telegram_tests_still_pass` | Regression | All existing Telegram tests pass |
| `discord_tests_still_pass` | Regression | All existing Discord tests pass |
| `slack_tests_still_pass` | Regression | All existing Slack tests pass |
| `plugin_host_loads_new_channels` | Integration | PluginHost can load new ChannelAdapter impls via shim |

---

## 7. Acceptance Criteria

### E2: Email Channel (P0 MVP)
- [ ] `EmailChannel` implements `ChannelAdapter` trait (not legacy `Channel`)
- [ ] IMAP polling receives emails from configured mailbox
- [ ] SMTP sends reply emails with proper threading headers (In-Reply-To, References)
- [ ] Gmail OAuth2 flow completes without plaintext passwords in config
- [ ] Password auth mode works for generic IMAP/SMTP servers
- [ ] `allowed_senders` filter works correctly (empty = all allowed)
- [ ] `subject_filter` limits which emails are processed
- [ ] Body truncation at `max_body_bytes` prevents memory exhaustion
- [ ] All config credential fields use `SecretRef` type (A4)
- [ ] Custom `Debug` impl redacts all auth-related fields
- [ ] Token refresh persists rotated tokens to `~/.clawft/tokens/` (0600 permissions)
- [ ] Feature-gated behind `email` feature flag
- [ ] All unit tests pass
- [ ] Existing channel tests pass (regression gate)

### E5a: Google Chat Channel
- [ ] `GoogleChatChannel` implements `ChannelAdapter` trait
- [ ] Receives messages via Pub/Sub push or polling fallback
- [ ] Sends messages via `spaces.messages.create` REST endpoint
- [ ] OAuth2 authentication via F6 helper (service account or user-delegated)
- [ ] `allowed_senders` filter works correctly
- [ ] Thread support via `thread.name` / thread key
- [ ] All config credential fields use `SecretRef` type (A4)
- [ ] Feature-gated behind `google-chat` feature flag
- [ ] All unit tests pass
- [ ] Existing channel tests pass (regression gate)

### E5b: Microsoft Teams Channel
- [ ] `TeamsChannel` implements `ChannelAdapter` trait
- [ ] Receives messages via Bot Framework webhook (Activity handler)
- [ ] Sends messages via Graph API `chats/{id}/messages`
- [ ] Azure AD client credentials auth works correctly
- [ ] Bot Framework webhook validates HMAC signature
- [ ] `<at>` mention tags stripped from incoming message content
- [ ] `allowed_senders` filter works correctly
- [ ] All config credential fields use `SecretRef` type (A4)
- [ ] Feature-gated behind `teams` feature flag
- [ ] All unit tests pass
- [ ] Existing channel tests pass (regression gate)

### Cross-Cutting
- [ ] All three channels follow the Telegram module structure pattern
- [ ] All three channels use `ChannelAdapter` trait, NOT legacy `Channel`
- [ ] `ChannelAdapter->Channel` shim successfully loads all three in `PluginHost`
- [ ] No plaintext secrets anywhere in config structs, debug output, or logs
- [ ] `cargo test -p clawft-channels` passes with all features enabled
- [ ] `cargo test -p clawft-channels` passes with no optional features (existing tests only)

---

## 8. Risk Notes

### 8.1 F6/E5a Timeline Conflict (HIGH)

**Risk**: F6 (OAuth2 helper) from Element 07 is scheduled Week 7-9, but E5a needs OAuth2 at Week 5-7.

**Impact**: E5a cannot be implemented without F6. Standalone OAuth2 implementation would duplicate effort and create technical debt.

**Mitigation**:
1. Coordinate with Element 07 to accelerate F6 delivery to Week 5
2. If F6 cannot be accelerated, defer E5a to Week 8+ (after F6 ships)
3. DO NOT implement standalone OAuth2 -- this is explicitly prohibited to avoid duplication

**Decision gate**: Week 5 sprint planning must resolve this.

### 8.2 IMAP Library Maturity (MEDIUM)

**Risk**: The `imap` crate (v3) is less mature than `lettre` for SMTP. Some IMAP servers have quirky behavior with IDLE, encoding, and authentication extensions.

**Impact**: Potential connection reliability issues with certain email providers.

**Mitigation**:
- Test against multiple IMAP servers (Gmail, Outlook, generic)
- Implement fallback from IDLE to polling if IDLE fails
- Set connection timeouts and implement reconnection logic
- Log IMAP protocol exchanges at TRACE level for debugging

### 8.3 Bot Framework Webhook Complexity (MEDIUM)

**Risk**: Microsoft Teams Bot Framework requires a publicly reachable HTTPS endpoint for webhook delivery. This adds deployment complexity.

**Impact**: Teams channel may not work in development environments without a tunnel (e.g., ngrok).

**Mitigation**:
- Document tunnel setup for development
- Consider optional polling mode via Graph API `delta` queries as fallback
- Gateway HTTP server (if available) can host the webhook endpoint

### 8.4 OAuth2 Token Rotation on Process Restart (MEDIUM)

**Risk**: If the process crashes after receiving a rotated refresh token but before persisting it, the old token becomes invalid.

**Impact**: Channel stops working until manual re-authentication.

**Mitigation**:
- Persist tokens immediately after refresh, before using the new access token
- Write to temp file + atomic rename to prevent partial writes
- Document manual recovery procedure
- Consider token encryption at rest (AES-256-GCM)

### 8.5 ChannelAdapter Shim Behavioral Differences (LOW)

**Risk**: The `ChannelAdapter->Channel` shim in `clawft-plugin` may introduce subtle behavioral differences from the native `Channel` trait implementations.

**Impact**: Edge cases where new channels behave differently from Telegram/Discord/Slack in the `PluginHost` lifecycle.

**Mitigation**:
- Integration tests that exercise the shim with at least one new channel
- Document known differences between `Channel` and `ChannelAdapter` traits
- Plan for C7 (PluginHost unification) to eliminate the shim entirely

---

## 9. Implementation Order

```text
Week 5:
  ├── E2 Email: types.rs, config parsing, factory
  ├── E2 Email: imap_client.rs (connect, fetch_unseen)
  └── E2 Email: smtp_client.rs (send_reply)

Week 6:
  ├── E2 Email: channel.rs (ChannelAdapter impl, start loop)
  ├── E2 Email: tests.rs (full unit test suite)
  ├── E2 Email: OAuth2 flow (if F6 available) or password-only
  └── E5b Teams: types.rs, api.rs (Azure AD auth, Graph API client)

Week 7:
  ├── E5b Teams: channel.rs (webhook handler, ChannelAdapter impl)
  ├── E5b Teams: tests.rs (full unit test suite)
  ├── E5a Google Chat: Begin if F6 available, otherwise defer
  └── Integration: PluginHost loading all new channels via shim
```

**Critical path**: E2 (Email) is P0 and has no blockers beyond A4 (SecretRef). Start here.
