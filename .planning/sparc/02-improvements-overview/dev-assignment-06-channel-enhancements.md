# Development Assignment: Element 06 -- Channel Enhancements

**Workstream**: E (Channel Enhancements)
**Timeline**: Weeks 4-8
**Element Orchestrator**: `.planning/sparc/06-channel-enhancements/00-orchestrator.md`
**Dev Stream Branch**: `sprint/phase-5-E`

---

## Prerequisites

Before starting any work in this element, the following must be merged to the integration branch:

| Prerequisite | Element | Description | Blocks |
|-------------|---------|-------------|--------|
| C1 Plugin Traits | 04 | `ChannelAdapter` trait in `clawft-plugin` | E2-E5b (new channels) |
| A4 SecretRef | 03 | Credential fields use `SecretRef` type | E2 (email OAuth2), E5a, E5b |
| B4 CronService Unification | 03 | Unified JSONL cron storage | E6 (heartbeat enhancement) |
| F6 OAuth2 Helper | 07 | Reusable OAuth2 flow plugin | E5a (Google Chat) |

---

## Unit 1: E1 Discord Resume + E6 Heartbeat (Week 4-5)

### E1: Discord Gateway Resume (OP 6)

**Problem**: On reconnect, the Discord channel always re-identifies (OP 2) instead of resuming (OP 6). The `session_id`, `resume_url`, and `ResumePayload` are stored but never used.

**Affected Files**:
- `crates/clawft-channels/src/discord/channel.rs` (main fix)
- `crates/clawft-channels/src/discord/events.rs` (already has `ResumePayload`)

**Current Code** (`crates/clawft-channels/src/discord/channel.rs:39-52`):

```rust
pub struct DiscordChannel {
    api: DiscordApiClient,
    status: Arc<RwLock<ChannelStatus>>,
    config: DiscordConfig,
    sequence: AtomicU64,
    session_id: RwLock<Option<String>>,    // stored but unused on reconnect
    resume_url: RwLock<Option<String>>,     // stored but unused on reconnect
}
```

The READY handler correctly stores session state (`channel.rs:327-336`):

```rust
"READY" => {
    if let Some(ref d) = payload.d
        && let Ok(ready) = serde_json::from_value::<ReadyEvent>(d.clone())
    {
        info!(bot_id = %ready.user.id, bot_name = %ready.user.username, "Discord bot authenticated");
        *self.session_id.write().await = Some(ready.session_id);
        *self.resume_url.write().await = ready.resume_gateway_url;
    }
}
```

But the reconnection path (`channel.rs:245-262`) always sends Identify:

```rust
// Send Identify (opcode 2).
let identify = GatewayPayload {
    op: super::events::OP_IDENTIFY,
    d: Some(serde_json::to_value(IdentifyPayload { ... }).unwrap_or_default()),
    s: None,
    t: None,
};
```

The `ResumePayload` in `events.rs:90-100` is defined but dead code:

```rust
pub struct ResumePayload {
    pub token: String,
    pub session_id: String,
    pub seq: u64,
}
```

**Implementation Plan**:

1. After receiving Hello (OP 10), check if `session_id` is `Some`.
2. If yes: send Resume (OP 6) using `ResumePayload` with `token`, `session_id`, and last `sequence`.
3. If no: send Identify (OP 2) as currently done.
4. Handle OP 9 (Invalid Session) with `d: false` -- clear session state and fall back to Identify.
5. Handle OP 9 with `d: true` -- wait 1-5 seconds (random jitter) then attempt Resume again.

**Key change location**: Replace the unconditional Identify block at `channel.rs:245-262` with a conditional Resume/Identify decision.

**Acceptance Criteria**:
- [ ] When `session_id` is available, reconnect sends OP 6 (Resume) instead of OP 2 (Identify)
- [ ] When `session_id` is `None`, reconnect sends OP 2 (Identify) as before
- [ ] OP 9 (Invalid Session, `d: false`) clears session state and triggers fresh Identify
- [ ] OP 9 (Invalid Session, `d: true`) retries Resume with random jitter delay
- [ ] `resume_url` is used as the WebSocket endpoint during Resume (already partially done at `channel.rs:186-191`)
- [ ] All existing Discord tests pass
- [ ] New unit test: verify `ResumePayload` serialization matches Discord Gateway spec

**Test Requirements**:
- Unit test: `ResumePayload` serialization (token, session_id, seq fields present)
- Unit test: reconnect logic chooses Resume when `session_id` is `Some`
- Unit test: reconnect logic chooses Identify when `session_id` is `None`
- Unit test: Invalid Session (false) clears state
- Existing tests: `crates/clawft-channels/src/discord/tests.rs` must continue passing

---

### E6: Enhanced Heartbeat / Proactive Check-in

**Problem**: The current `HeartbeatService` is a simple timer that emits a single fixed prompt at a fixed interval. It needs enhancement to support proactive check-in modes across multiple channels with configurable behaviors.

**Affected Files**:
- `crates/clawft-services/src/heartbeat/mod.rs` (enhance existing)

**Current Code** (`crates/clawft-services/src/heartbeat/mod.rs:18-22`):

```rust
pub struct HeartbeatService {
    interval: Duration,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
}
```

**Dependencies**:
- B4 (CronService unification) must land first -- heartbeat schedules should use the unified CronService rather than maintaining a separate timer.

**Implementation Plan**:

1. Add `HeartbeatMode` enum: `Simple` (current behavior), `CheckIn` (proactive triage per channel).
2. `CheckIn` mode supports configurable prompts per channel (e.g., "Check inbox for new emails" for email, "Check Slack channels for updates" for Slack).
3. Integrate with `CronService` for schedule management instead of `tokio::time::interval`.
4. Each check-in sends an `InboundMessage` with `channel: "heartbeat"` and metadata indicating the target channel and check-in type.

**Acceptance Criteria**:
- [ ] `HeartbeatMode::Simple` preserves current behavior exactly
- [ ] `HeartbeatMode::CheckIn` supports per-channel prompts
- [ ] Heartbeat triggers use `CronService` scheduling (depends on B4)
- [ ] Check-in messages include metadata: `{ "heartbeat_type": "check_in", "target_channel": "email" }`
- [ ] Existing heartbeat tests pass unchanged
- [ ] New tests for `CheckIn` mode with mock channel targets

**Test Requirements**:
- Existing tests in `crates/clawft-services/src/heartbeat/mod.rs` pass unchanged
- New test: `CheckIn` mode generates messages with correct metadata
- New test: multiple channel targets produce separate messages

---

## Unit 2: E2 Email Channel (Week 5-7, P0 for MVP)

### E2: Email Channel (IMAP + SMTP)

**Priority**: P0 -- required for MVP milestone (Week 8)

**New Files**:
- `crates/clawft-channels/src/email/mod.rs`
- `crates/clawft-channels/src/email/channel.rs`
- `crates/clawft-channels/src/email/imap_client.rs`
- `crates/clawft-channels/src/email/smtp_client.rs`
- `crates/clawft-channels/src/email/types.rs`
- `crates/clawft-channels/src/email/factory.rs`
- `crates/clawft-channels/src/email/tests.rs`

**Crate**: `clawft-channels` (feature-gated: `email`)

**New Dependencies** (add to `crates/clawft-channels/Cargo.toml`):
```toml
[dependencies]
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport"], optional = true }
imap = { version = "3", optional = true }
oauth2 = { version = "4", optional = true }

[features]
email = ["lettre", "imap", "oauth2"]
```

**Architecture**: Implements `ChannelAdapter` trait from `clawft-plugin` (C1), NOT the legacy `Channel` trait. Uses the `ChannelAdapter->Channel` shim from `clawft-plugin` so the existing `PluginHost` can load it.

**Existing Channel Pattern** (follow Telegram's structure at `crates/clawft-channels/src/telegram/channel.rs`):

The Telegram channel demonstrates the pattern:
- Long-polling loop in `start()` with `CancellationToken` support
- `process_update()` method that constructs `InboundMessage` and calls `host.deliver_inbound()`
- `ChannelFactory` that parses JSON config and creates the channel

For the email channel, translate this pattern to:
- IMAP IDLE or periodic poll for new messages
- `process_email()` extracts sender, subject, body, and constructs `InboundMessage`
- SMTP sends outbound replies via `send()`

**Config Shape**:
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
  "allowed_senders": ["boss@company.com"]
}
```

**Security Requirements**:
- All credential fields MUST use `SecretRef` type from A4 (no plaintext passwords in config structs)
- OAuth2 flows MUST include `state` parameter for CSRF protection
- Token refresh must persist rotated refresh tokens to `~/.clawft/tokens/` with 0600 permissions

**Cross-Element Dependencies**:
- **A4 (SecretRef)**: Email credentials must use `SecretRef`, not plain `String`
- **E6 (Heartbeat)**: Proactive inbox triage requires both E2 and enhanced E6
- **F6 (OAuth2 helper)**: Gmail OAuth2 should use F6's reusable flow when available; implement minimal inline OAuth2 only if F6 is not ready

**Cross-Element Integration Test** (from `01-cross-element-integration.md`):
> "Email Channel -> OAuth2 Helper" (Week 7, P0): Configure Gmail via E2, verify F6 OAuth2 flow completes, token stored in agent workspace.

**Acceptance Criteria**:
- [ ] Email channel connects to IMAP server and receives messages
- [ ] Email channel sends replies via SMTP
- [ ] Gmail OAuth2 flow completes without plaintext passwords in config
- [ ] Proactive inbox triage via E6 heartbeat integration
- [ ] Implements `ChannelAdapter` trait (NOT legacy `Channel`)
- [ ] Feature-gated behind `email` feature flag
- [ ] Allow-list filtering by sender address
- [ ] Credentials stored via `SecretRef` (env var names, not raw secrets)
- [ ] Token refresh persists to encrypted file with 0600 permissions

**Test Requirements**:
- Unit test: IMAP message parsing to `InboundMessage`
- Unit test: SMTP message construction from `OutboundMessage`
- Unit test: OAuth2 token refresh flow (mock HTTP)
- Unit test: Config parsing with `SecretRef` fields
- Unit test: allow-list filtering by sender email
- Integration test: Email -> OAuth2 cross-element test (Week 7)

---

## Unit 3: E5a Google Chat + E5b Microsoft Teams (Week 5-7)

### E5a: Google Chat Channel

**New Files**:
- `crates/clawft-channels/src/google_chat/mod.rs`
- `crates/clawft-channels/src/google_chat/channel.rs`
- `crates/clawft-channels/src/google_chat/api.rs`
- `crates/clawft-channels/src/google_chat/types.rs`
- `crates/clawft-channels/src/google_chat/factory.rs`

**Feature Flag**: `google-chat` in `crates/clawft-channels/Cargo.toml`

**Transport**: Google Workspace Chat API (REST)

**Auth**: OAuth2 via F6 helper (required dependency)

**TIMELINE RISK**: F6 (OAuth2 helper) is scheduled for Week 7-9 in Element 07, but E5a needs it at Week 5-7. This is flagged as a high-likelihood risk in the orchestrator (score 6). Options:
1. Coordinate with Element 07 to accelerate F6 delivery to Week 5-6
2. Defer E5a to Week 8+ (E-Consumer phase)
3. DO NOT implement standalone OAuth2 in E5a -- this would duplicate F6 effort

**Implementation Pattern**: Same `ChannelAdapter` trait implementation as E2 (email). Google Chat uses:
- Pub/Sub push notifications or webhook for inbound messages
- REST API `spaces.messages.create` for outbound
- OAuth2 service account or user flow for authentication

**Acceptance Criteria**:
- [ ] Google Chat channel sends and receives messages via Workspace API
- [ ] Uses F6 OAuth2 helper for authentication (no standalone OAuth2 impl)
- [ ] Supports both DMs and Spaces
- [ ] Implements `ChannelAdapter` trait
- [ ] Feature-gated behind `google-chat`
- [ ] Credentials via `SecretRef`
- [ ] OAuth2 flow includes `state` parameter for CSRF protection

### E5b: Microsoft Teams Channel

**New Files**:
- `crates/clawft-channels/src/teams/mod.rs`
- `crates/clawft-channels/src/teams/channel.rs`
- `crates/clawft-channels/src/teams/api.rs`
- `crates/clawft-channels/src/teams/types.rs`
- `crates/clawft-channels/src/teams/factory.rs`

**Feature Flag**: `teams` in `crates/clawft-channels/Cargo.toml`

**Transport**: Microsoft Bot Framework / Graph API (REST)

**Auth**: Azure AD (client credentials flow or certificate-based)

**Implementation**: Teams uses:
- Bot Framework webhook for inbound messages (activity handler pattern)
- Graph API `chats/{id}/messages` for outbound
- Azure AD app registration with Bot Framework channel configuration

**Acceptance Criteria**:
- [ ] Teams channel sends and receives messages via Bot Framework
- [ ] Azure AD authentication completes (client credentials flow)
- [ ] Supports both 1:1 chats and channel messages
- [ ] Implements `ChannelAdapter` trait
- [ ] Feature-gated behind `teams`
- [ ] Credentials via `SecretRef`

**Test Requirements for Unit 3**:
- Unit test: Google Chat message parsing
- Unit test: Teams activity handler message parsing
- Unit test: Azure AD token acquisition (mock HTTP)
- Integration test: at least one enterprise channel operational (exit criteria)

---

## Unit 4: E3 WhatsApp + E4 Signal + E5 Matrix/IRC (Week 6-8)

### E3: WhatsApp Channel

**New Files**: `crates/clawft-channels/src/whatsapp/` (channel, api, types, factory)
**Feature Flag**: `whatsapp`
**Transport**: WhatsApp Cloud API (REST)
**Auth**: App token (via `SecretRef`)

**Implementation Notes**:
- Webhook endpoint for inbound message notifications
- REST `POST /v21.0/{phone_number_id}/messages` for outbound
- Webhook verification via `verify_token` (MUST use `SecretRef`, not plaintext)
- Rate limiter with exponential backoff for 429 responses (risk item, score 4)

**Acceptance Criteria**:
- [ ] WhatsApp channel sends and receives text messages via Cloud API
- [ ] Webhook verification token uses `SecretRef`
- [ ] Rate limiter handles 429 responses with exponential backoff
- [ ] Implements `ChannelAdapter` trait
- [ ] Feature-gated behind `whatsapp`

### E4: Signal Channel

**New Files**: `crates/clawft-channels/src/signal/` (channel, subprocess, types, factory)
**Feature Flag**: `signal`
**Transport**: `signal-cli` subprocess
**Auth**: Local (linked device)

**Implementation Notes**:
- Spawns `signal-cli` as subprocess via `tokio::process::Command`
- JSON-RPC mode (`signal-cli jsonRpc`) for message send/receive
- PID tracking with health checks for crash recovery
- Timeout kill for hung subprocesses

**Security**: All subprocess arguments MUST be sanitized against command injection. Use a builder pattern for constructing `signal-cli` commands -- never format user input directly into command strings.

**Acceptance Criteria**:
- [ ] Signal channel sends and receives messages via `signal-cli` subprocess
- [ ] Subprocess management: PID tracking, health checks, auto-restart on crash
- [ ] All arguments sanitized against command injection
- [ ] Timeout kill for hung subprocesses (configurable, default 30s)
- [ ] Implements `ChannelAdapter` trait
- [ ] Feature-gated behind `signal`

### E5: Matrix / IRC Channels

**New Files**:
- `crates/clawft-channels/src/matrix/` (channel, api, types, factory)
- `crates/clawft-channels/src/irc/` (channel, protocol, types, factory)

**Feature Flags**: `matrix`, `irc`

**Matrix**: Uses `matrix-sdk` crate for room joins, message send/receive.
**IRC**: Direct protocol implementation or use `irc` crate.

**Acceptance Criteria**:
- [ ] Matrix channel joins rooms and sends/receives messages
- [ ] IRC channel connects to server and handles messages
- [ ] Both implement `ChannelAdapter` trait
- [ ] Feature-gated behind respective flags

**Test Requirements for Unit 4**:
- Unit test per channel: message parsing, outbound construction
- Unit test: Signal subprocess argument sanitization
- Unit test: WhatsApp rate limiter backoff behavior
- Unit test: Matrix room join and message flow

---

## ChannelAdapter Shim Pattern

All new channels (E2-E5b) implement `ChannelAdapter` from `clawft-plugin`, NOT the legacy `Channel` trait in `crates/clawft-channels/src/traits.rs`. During the transition period, the shim in `clawft-plugin` converts `ChannelAdapter` implementations so they can be loaded by the existing `PluginHost` in `crates/clawft-channels/src/host.rs`.

E1 (Discord Resume) modifies the existing `Channel` impl directly since Discord is an existing channel.

After C7 (PluginHost unification), the legacy `Channel` trait and shim will be removed and all channels will use `ChannelAdapter` natively.

---

## Completion Status

**Per tracker**: 8/9 items complete (89%). IRC channel deferred.

- [x] E1 -- Discord Gateway Resume (OP 6) -- DONE 2026-02-20
- [x] E2 -- Email channel (IMAP + SMTP + OAuth2) -- DONE 2026-02-20
- [x] E3 -- WhatsApp Cloud API -- DONE 2026-02-20
- [x] E4 -- Signal subprocess bridge -- DONE 2026-02-20
- [x] E5 -- Matrix channel -- DONE 2026-02-20
- [ ] E5 -- IRC channel -- deferred (low priority)
- [x] E5a -- Google Chat Workspace API -- DONE 2026-02-20
- [x] E5b -- Microsoft Teams Bot Framework -- DONE 2026-02-20
- [x] E6 -- Enhanced heartbeat / check-in -- DONE 2026-02-20

## Security Checklist (All Units)

These security criteria are mandatory exit gates for the element:

- [x] All channel config credential fields use `SecretRef` type (A4 dependency) -- DONE 2026-02-20
- [x] No plaintext secrets in config structs (including WhatsApp `verify_token`) -- DONE 2026-02-20
- [x] OAuth2 flows include `state` parameter for CSRF protection (E2, E5a) -- DONE 2026-02-20
- [x] Subprocess-based channels (E4 Signal) sanitize all arguments against command injection -- DONE 2026-02-20
- [x] OAuth2 token refresh persists rotated tokens to encrypted file (`~/.clawft/tokens/`, 0600 permissions) -- DONE 2026-02-20
- [x] All existing channel tests pass (regression gate) -- DONE 2026-02-20

---

## Merge Coordination

**Conflict Zones**: E-stream modifies `clawft-channels` crate exclusively. No conflict with other streams except:
- `clawft-types/src/config.rs` if adding new channel config structs (coordinate with streams 5A, 5B)
- The email channel's OAuth2 integration touches F6 territory (coordinate with 5F stream)

**Merge Order**: E1 + E6 first (fixes existing code), then E2 (MVP critical), then E5a/E5b, then E3/E4/E5.
