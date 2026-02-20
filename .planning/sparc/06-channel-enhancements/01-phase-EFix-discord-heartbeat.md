# Phase E-Fix: Discord Gateway Resume & Enhanced Heartbeat

> **Element:** 06 -- Channel Enhancements
> **Phase:** E-Fix (Existing Channel Fixes)
> **Timeline:** Week 4-5
> **Priority:** P1
> **Crates:** `clawft-channels` (`src/discord/channel.rs`, `src/discord/events.rs`), `clawft-services` (`src/heartbeat/mod.rs`)
> **Dependencies IN:** None for E1; B4 (CronService unification from Element 03) for E6
> **Blocks:** E2 (Email channel depends on E6 for proactive inbox triage)
> **Status:** Planning

---

## 1. Overview

Phase E-Fix addresses two deficiencies in existing channel infrastructure before new channels are added in E-Enterprise and E-Consumer phases:

| Item | File | Problem | Fix |
|------|------|---------|-----|
| **E1** | `clawft-channels/src/discord/channel.rs` | Reconnection always sends Identify (OP 2) -- stored `session_id`, `resume_url`, and `ResumePayload` are never used | After Hello (OP 10), send Resume (OP 6) when session state exists; fall back to Identify only when session is absent or invalidated |
| **E6** | `clawft-services/src/heartbeat/mod.rs` | Single fixed-interval heartbeat with one prompt; no per-channel proactive check-in capability | Add `HeartbeatMode` enum with `Simple` (preserves current behavior) and `CheckIn` (per-channel prompts with metadata); integrate with `CronService` for scheduling |

E1 is independent and can begin immediately. E6 depends on B4 (CronService unification from Element 03) landing first for schedule management integration. E6's `Simple` mode refactor (no CronService dependency) can proceed in parallel.

**Key deliverables:**
- Discord reconnection uses Resume (OP 6) when session state is available
- Invalid Session (OP 9) handler distinguishes resumable (`d: true`) from non-resumable (`d: false`) sessions
- `resume_url` is used as the WebSocket endpoint during Resume attempts
- `HeartbeatMode::Simple` preserves exact current behavior
- `HeartbeatMode::CheckIn` supports per-channel prompts with metadata tagging
- CronService integration for schedule-driven heartbeats (gated on B4)

---

## 2. Current Code

### 2.1 Discord Channel -- Struct & Session Storage (E1)

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 39-52

The struct stores `session_id` and `resume_url` from the READY event, but never reads them during the reconnection handshake:

```rust
// crates/clawft-channels/src/discord/channel.rs:39-52
pub struct DiscordChannel {
    /// Discord REST API client (for sending messages).
    api: DiscordApiClient,
    /// Current lifecycle status.
    status: Arc<RwLock<ChannelStatus>>,
    /// Parsed configuration.
    config: DiscordConfig,
    /// Last received sequence number for heartbeats and resuming.
    sequence: AtomicU64,
    /// Session ID from the READY event (for resuming).
    session_id: RwLock<Option<String>>,
    /// Resume gateway URL from the READY event.
    resume_url: RwLock<Option<String>>,
}
```

### 2.2 Discord Channel -- READY Handler Stores Session State (E1)

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 326-337

The READY dispatch handler correctly extracts and stores `session_id` and `resume_gateway_url`:

```rust
// crates/clawft-channels/src/discord/channel.rs:326-337
"READY" => {
    if let Some(ref d) = payload.d
        && let Ok(ready) = serde_json::from_value::<ReadyEvent>(d.clone())
    {
        info!(
            bot_id = %ready.user.id,
            bot_name = %ready.user.username,
            "Discord bot authenticated"
        );
        *self.session_id.write().await = Some(ready.session_id);
        *self.resume_url.write().await = ready.resume_gateway_url;
    }
}
```

### 2.3 Discord Channel -- Unconditional Identify on Reconnect (E1, Bug)

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 245-276

After receiving Hello (OP 10), the code unconditionally sends Identify (OP 2), ignoring any stored session state. This forces a full re-authentication on every reconnect, causing the bot to miss events that occurred during the disconnect window:

```rust
// crates/clawft-channels/src/discord/channel.rs:245-276
// Send Identify (opcode 2).
let identify = GatewayPayload {
    op: super::events::OP_IDENTIFY,
    d: Some(
        serde_json::to_value(IdentifyPayload {
            token: self.config.token.clone(),
            intents: self.config.intents,
            properties: ConnectionProperties {
                os: std::env::consts::OS.to_owned(),
                browser: "clawft".into(),
                device: "clawft".into(),
            },
        })
        .unwrap_or_default(),
    ),
    s: None,
    t: None,
};

if let Ok(json) = serde_json::to_string(&identify)
    && let Err(e) = ws_write.send(WsMessage::Text(json)).await
{
    error!(error = %e, "failed to send Identify");
    self.set_status(ChannelStatus::Error(e.to_string())).await;

    tokio::select! {
        _ = cancel.cancelled() => break,
        _ = tokio::time::sleep(
            std::time::Duration::from_secs(RECONNECT_DELAY_SECS)
        ) => continue,
    }
}
```

### 2.4 Discord Channel -- Gateway URL Selection (E1, Partial)

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 186-191

The reconnection loop already uses `resume_url` to select the WebSocket endpoint. This part is correct and requires no changes:

```rust
// crates/clawft-channels/src/discord/channel.rs:186-191
let gateway_url = {
    let resume = self.resume_url.read().await;
    resume
        .clone()
        .unwrap_or_else(|| self.config.gateway_url.clone())
};
```

### 2.5 Discord Channel -- Invalid Session Handler (E1, Incomplete)

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 388-394

The current Invalid Session (OP 9) handler always clears session state and reconnects. It does not distinguish between `d: true` (resumable -- retry Resume after jitter) and `d: false` (non-resumable -- must re-Identify):

```rust
// crates/clawft-channels/src/discord/channel.rs:388-394
OP_INVALID_SESSION => {
    warn!("invalid session, reconnecting with fresh identify");
    *self.session_id.write().await = None;
    *self.resume_url.write().await = None;
    self.sequence.store(0, Ordering::SeqCst);
    break;
}
```

### 2.6 Discord Events -- ResumePayload (E1, Dead Code)

**File:** `crates/clawft-channels/src/discord/events.rs`, lines 89-100

The `ResumePayload` struct and `OP_RESUME` constant are defined and tested (serialization test at events.rs:370-386) but never constructed in the channel's connection logic:

```rust
// crates/clawft-channels/src/discord/events.rs:19-20
/// Opcode 6: Resume -- resume a previous session.
pub const OP_RESUME: u8 = 6;

// crates/clawft-channels/src/discord/events.rs:89-100
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
```

### 2.7 Discord Events -- Import List (E1)

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 25-28

The current import from `super::events` does not include `OP_RESUME` or `ResumePayload`:

```rust
// crates/clawft-channels/src/discord/channel.rs:25-28
use super::events::{
    ConnectionProperties, GatewayPayload, HelloData, IdentifyPayload, MessageCreate, OP_DISPATCH,
    OP_HEARTBEAT, OP_HEARTBEAT_ACK, OP_HELLO, OP_INVALID_SESSION, OP_RECONNECT, ReadyEvent,
};
```

### 2.8 HeartbeatService -- Struct (E6)

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, lines 18-22

The current struct supports only a single interval and a single prompt. There is no notion of modes, per-channel targeting, or metadata:

```rust
// crates/clawft-services/src/heartbeat/mod.rs:18-22
pub struct HeartbeatService {
    interval: Duration,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
}
```

### 2.9 HeartbeatService -- Start Loop (E6)

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, lines 45-79

The loop emits a fixed `InboundMessage` with hardcoded `channel: "heartbeat"`, `sender_id: "system"`, `chat_id: "heartbeat"`, and empty metadata:

```rust
// crates/clawft-services/src/heartbeat/mod.rs:45-79
pub async fn start(&self, cancel: CancellationToken) -> Result<()> {
    info!(
        interval_secs = self.interval.as_secs(),
        "heartbeat service started"
    );
    let mut interval = tokio::time::interval(self.interval);

    // The first tick fires immediately; skip it so the first heartbeat
    // happens after one full interval.
    interval.tick().await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("heartbeat service shutting down");
                return Ok(());
            }
            _ = interval.tick() => {
                let msg = InboundMessage {
                    channel: "heartbeat".to_string(),
                    sender_id: "system".to_string(),
                    chat_id: "heartbeat".to_string(),
                    content: self.prompt.clone(),
                    timestamp: Utc::now(),
                    media: vec![],
                    metadata: HashMap::new(),
                };

                if self.message_tx.send(msg).is_err() {
                    return Err(ServiceError::ChannelClosed);
                }
            }
        }
    }
}
```

### 2.10 HeartbeatService -- Existing Tests (E6)

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, lines 82-166

Four existing tests that must continue passing unchanged after E6 changes:

| Test | Lines | Description |
|------|-------|-------------|
| `heartbeat_sends_messages` | 88-114 | Creates service with 50ms interval, waits 150ms, verifies at least one message with correct fields |
| `graceful_shutdown_on_cancel` | 117-136 | Creates service with 3600s interval, cancels after 10ms, verifies clean exit |
| `channel_closed_returns_error` | 138-158 | Drops receiver, verifies `ServiceError::ChannelClosed` is returned |
| `new_sets_interval_from_minutes` | 160-165 | Verifies `HeartbeatService::new(5, ...)` produces a 300-second interval |

**Note:** These tests construct `HeartbeatService` directly via struct literal (not `::new()`), so adding fields to the struct requires updating these tests to include the new fields (with defaults that preserve Simple mode behavior).

---

## 3. Implementation Tasks

### Task E1.1: Add Resume/ResumePayload imports

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 25-28

Add `OP_RESUME` and `ResumePayload` to the import list:

```rust
// BEFORE (channel.rs:25-28):
use super::events::{
    ConnectionProperties, GatewayPayload, HelloData, IdentifyPayload, MessageCreate, OP_DISPATCH,
    OP_HEARTBEAT, OP_HEARTBEAT_ACK, OP_HELLO, OP_INVALID_SESSION, OP_RECONNECT, ReadyEvent,
};

// AFTER:
use super::events::{
    ConnectionProperties, GatewayPayload, HelloData, IdentifyPayload, MessageCreate, OP_DISPATCH,
    OP_HEARTBEAT, OP_HEARTBEAT_ACK, OP_HELLO, OP_INVALID_SESSION, OP_RECONNECT, OP_RESUME,
    ReadyEvent, ResumePayload,
};
```

### Task E1.2: Replace unconditional Identify with Resume/Identify decision

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 245-276

Replace the unconditional Identify block with a conditional check: if `session_id` is `Some`, send Resume (OP 6); otherwise, send Identify (OP 2) as before.

```rust
// REPLACE channel.rs:245-276 WITH:

// Decide: Resume (OP 6) if we have session state, else Identify (OP 2).
let handshake_payload = {
    let session = self.session_id.read().await;
    if let Some(ref sid) = *session {
        let seq = self.sequence.load(Ordering::SeqCst);
        info!(session_id = %sid, seq = seq, "attempting Resume (OP 6)");
        GatewayPayload {
            op: OP_RESUME,
            d: Some(
                serde_json::to_value(ResumePayload {
                    token: self.config.token.clone(),
                    session_id: sid.clone(),
                    seq,
                })
                .unwrap_or_default(),
            ),
            s: None,
            t: None,
        }
    } else {
        info!("no session state, sending Identify (OP 2)");
        GatewayPayload {
            op: super::events::OP_IDENTIFY,
            d: Some(
                serde_json::to_value(IdentifyPayload {
                    token: self.config.token.clone(),
                    intents: self.config.intents,
                    properties: ConnectionProperties {
                        os: std::env::consts::OS.to_owned(),
                        browser: "clawft".into(),
                        device: "clawft".into(),
                    },
                })
                .unwrap_or_default(),
            ),
            s: None,
            t: None,
        }
    }
};

if let Ok(json) = serde_json::to_string(&handshake_payload)
    && let Err(e) = ws_write.send(WsMessage::Text(json)).await
{
    error!(error = %e, "failed to send handshake");
    self.set_status(ChannelStatus::Error(e.to_string())).await;

    tokio::select! {
        _ = cancel.cancelled() => break,
        _ = tokio::time::sleep(
            std::time::Duration::from_secs(RECONNECT_DELAY_SECS)
        ) => continue,
    }
}
```

### Task E1.3: Handle RESUMED dispatch event

**File:** `crates/clawft-channels/src/discord/channel.rs`, inside the `OP_DISPATCH` match at lines 323-367

Add a `"RESUMED"` arm after the `"READY"` arm. When a Resume is successful, Discord sends a `RESUMED` dispatch event. Log it and do not clear session state:

```rust
// Add after the "READY" => { ... } arm (channel.rs ~338):
"RESUMED" => {
    info!("Discord session resumed successfully");
}
```

### Task E1.4: Distinguish OP 9 (Invalid Session) resumable vs non-resumable

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 388-394

Replace the current OP_INVALID_SESSION handler with logic that inspects `d`:
- `d: false` -- session is non-resumable. Clear all state and fall back to Identify on next reconnect.
- `d: true` -- session may be resumable. Wait 1-5 seconds (random jitter per Discord docs), then attempt Resume again (do NOT clear session state).

```rust
// REPLACE channel.rs:388-394 WITH:
OP_INVALID_SESSION => {
    let resumable = payload.d
        .as_ref()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if resumable {
        // Session may still be valid. Wait with random jitter
        // (1-5 seconds per Discord docs), then retry Resume.
        let jitter_ms = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::time::SystemTime::now().hash(&mut hasher);
            1000 + (hasher.finish() % 4000)
        };
        warn!(
            jitter_ms = jitter_ms,
            "invalid session (resumable), retrying after jitter"
        );
        tokio::time::sleep(std::time::Duration::from_millis(jitter_ms)).await;
        break; // reconnect loop will attempt Resume since session_id is still set
    } else {
        warn!("invalid session (non-resumable), clearing state for fresh Identify");
        *self.session_id.write().await = None;
        *self.resume_url.write().await = None;
        self.sequence.store(0, Ordering::SeqCst);
        break;
    }
}
```

**Note on jitter:** Discord requires a random wait of 1-5 seconds before reconnecting after a resumable Invalid Session. We use a hash-based pseudo-random value instead of pulling in `rand` as a dependency. This is adequate for jitter purposes (not security-critical). If a `rand` dependency is already in the tree, prefer `rand::thread_rng().gen_range(1000..=5000)` instead.

### Task E1.5: Handle OP 7 (Reconnect) -- preserve session state

**File:** `crates/clawft-channels/src/discord/channel.rs`, lines 384-387

The existing OP_RECONNECT handler is correct -- it breaks out of the message loop into the reconnection loop. Session state is preserved, so the next iteration will attempt Resume. No change needed, but verify it does not clear `session_id` or `resume_url`.

```rust
// channel.rs:384-387 (EXISTING -- NO CHANGE NEEDED)
OP_RECONNECT => {
    info!("server requested reconnect");
    break;
}
```

### Task E6.1: Define HeartbeatMode enum

**File:** `crates/clawft-services/src/heartbeat/mod.rs`

Add a `HeartbeatMode` enum above the `HeartbeatService` struct:

```rust
/// Heartbeat operation mode.
#[derive(Debug, Clone)]
pub enum HeartbeatMode {
    /// Simple fixed-interval heartbeat with a single prompt (current behavior).
    Simple,
    /// Proactive check-in mode: sends per-channel prompts with metadata.
    CheckIn {
        /// Map of target channel name to the prompt to send for that channel.
        /// Example: `{ "email": "Check inbox for new messages", "slack": "Review new Slack messages" }`
        channel_prompts: HashMap<String, String>,
    },
}
```

### Task E6.2: Add mode field to HeartbeatService

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, lines 18-22

Add a `mode` field to `HeartbeatService`:

```rust
// BEFORE (mod.rs:18-22):
pub struct HeartbeatService {
    interval: Duration,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
}

// AFTER:
pub struct HeartbeatService {
    interval: Duration,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
    mode: HeartbeatMode,
}
```

### Task E6.3: Update constructor to default to Simple mode

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, lines 29-39

Update the `new()` constructor to set `mode: HeartbeatMode::Simple`, preserving the existing API:

```rust
// BEFORE (mod.rs:29-39):
pub fn new(
    interval_minutes: u64,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
) -> Self {
    Self {
        interval: Duration::from_secs(interval_minutes * 60),
        prompt,
        message_tx,
    }
}

// AFTER:
pub fn new(
    interval_minutes: u64,
    prompt: String,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
) -> Self {
    Self {
        interval: Duration::from_secs(interval_minutes * 60),
        prompt,
        message_tx,
        mode: HeartbeatMode::Simple,
    }
}
```

### Task E6.4: Add check-in constructor

**File:** `crates/clawft-services/src/heartbeat/mod.rs`

Add a new constructor for CheckIn mode:

```rust
/// Create a heartbeat service in check-in mode.
///
/// Each tick sends a separate `InboundMessage` per channel in `channel_prompts`,
/// with metadata tagging the heartbeat type and target channel.
pub fn new_check_in(
    interval_minutes: u64,
    default_prompt: String,
    channel_prompts: HashMap<String, String>,
    message_tx: mpsc::UnboundedSender<InboundMessage>,
) -> Self {
    Self {
        interval: Duration::from_secs(interval_minutes * 60),
        prompt: default_prompt,
        message_tx,
        mode: HeartbeatMode::CheckIn { channel_prompts },
    }
}
```

### Task E6.5: Update start() loop for mode-aware message generation

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, lines 45-79

Modify the `start()` method's tick handler to dispatch based on `self.mode`:

```rust
// REPLACE the interval.tick() arm in start() WITH:
_ = interval.tick() => {
    match &self.mode {
        HeartbeatMode::Simple => {
            let msg = InboundMessage {
                channel: "heartbeat".to_string(),
                sender_id: "system".to_string(),
                chat_id: "heartbeat".to_string(),
                content: self.prompt.clone(),
                timestamp: Utc::now(),
                media: vec![],
                metadata: HashMap::new(),
            };
            if self.message_tx.send(msg).is_err() {
                return Err(ServiceError::ChannelClosed);
            }
        }
        HeartbeatMode::CheckIn { channel_prompts } => {
            for (target_channel, prompt) in channel_prompts {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "heartbeat_type".to_string(),
                    serde_json::Value::String("check_in".to_string()),
                );
                metadata.insert(
                    "target_channel".to_string(),
                    serde_json::Value::String(target_channel.clone()),
                );

                let msg = InboundMessage {
                    channel: "heartbeat".to_string(),
                    sender_id: "system".to_string(),
                    chat_id: format!("heartbeat:{}", target_channel),
                    content: prompt.clone(),
                    timestamp: Utc::now(),
                    media: vec![],
                    metadata,
                };
                if self.message_tx.send(msg).is_err() {
                    return Err(ServiceError::ChannelClosed);
                }
            }
        }
    }
}
```

### Task E6.6: Update existing tests to include mode field

**File:** `crates/clawft-services/src/heartbeat/mod.rs`, test module

The existing tests construct `HeartbeatService` via struct literal. Each must include the new `mode` field set to `HeartbeatMode::Simple`:

```rust
// EXAMPLE (test heartbeat_sends_messages):
let svc = HeartbeatService {
    interval: Duration::from_millis(50),
    prompt: "heartbeat check".into(),
    message_tx: tx,
    mode: HeartbeatMode::Simple, // <-- ADD THIS LINE
};
```

Apply the same change to `graceful_shutdown_on_cancel` and `channel_closed_returns_error`. The `new_sets_interval_from_minutes` test uses `::new()` which already defaults to `Simple` mode -- no change needed.

### Task E6.7: CronService integration (GATED ON B4)

**Depends on:** Element 03, Task B4 (CronService unification)

This task cannot be implemented until B4 lands. When B4 is available:

1. Add an optional `CronService` reference to `HeartbeatService`
2. In `CheckIn` mode, register heartbeat schedules with `CronService` instead of using `tokio::time::interval`
3. Support cron expressions (e.g., `"0 */15 * * * *"` for every 15 minutes)
4. `Simple` mode continues to use `tokio::time::interval` (no CronService dependency)

**Placeholder comment in code:**

```rust
// TODO(E6.7): When B4 (CronService unification) lands, replace
// `tokio::time::interval` in CheckIn mode with CronService schedules.
// Simple mode will continue to use tokio::time::interval.
```

---

## 4. Concurrency Plan

E1 and E6 are independent -- they modify different crates (`clawft-channels` vs `clawft-services`) with no shared code. They can be implemented in parallel by separate developers or agents.

Within E1, tasks E1.1 through E1.5 are sequential (each builds on imports/changes from the previous task).

Within E6, tasks E6.1 through E6.6 are sequential. E6.7 is blocked on B4 and should be deferred.

---

## 5. Tests Required

### E1: Discord Resume Tests

| Test | Location | Description |
|------|----------|-------------|
| `resume_payload_serialization` | `discord/events.rs` | **Already exists** (events.rs:370-386). Verify `ResumePayload` serializes to JSON with `op: 6`, `session_id`, `seq` fields. No new test needed. |
| `resume_chosen_when_session_exists` | `discord/channel.rs` (new) | Set `session_id` to `Some("test-session")` and `sequence` to 42. Invoke the handshake decision logic. Assert the resulting `GatewayPayload` has `op == OP_RESUME` and the `d` field contains `session_id: "test-session"` and `seq: 42`. |
| `identify_chosen_when_no_session` | `discord/channel.rs` (new) | Leave `session_id` as `None`. Invoke the handshake decision logic. Assert the resulting `GatewayPayload` has `op == OP_IDENTIFY`. |
| `invalid_session_false_clears_state` | `discord/channel.rs` (new) | Simulate receiving OP 9 with `d: false`. Assert that `session_id` is set to `None`, `resume_url` is set to `None`, and `sequence` is reset to 0. |
| `invalid_session_true_preserves_state` | `discord/channel.rs` (new) | Simulate receiving OP 9 with `d: true`. Assert that `session_id`, `resume_url`, and `sequence` are NOT cleared. |
| `reconnect_preserves_session_state` | `discord/channel.rs` (new) | Simulate receiving OP 7 (Reconnect). Assert that `session_id` and `resume_url` are preserved (not cleared). |
| `existing_discord_tests_pass` | `discord/tests.rs`, `discord/events.rs` | **Regression gate.** All existing tests in the discord module must continue passing without modification. |

### E6: Heartbeat Tests

| Test | Location | Description |
|------|----------|-------------|
| `heartbeat_sends_messages` | `heartbeat/mod.rs` | **Existing** (mod.rs:88-114). Updated to include `mode: HeartbeatMode::Simple`. Same assertions. |
| `graceful_shutdown_on_cancel` | `heartbeat/mod.rs` | **Existing** (mod.rs:117-136). Updated to include `mode: HeartbeatMode::Simple`. Same assertions. |
| `channel_closed_returns_error` | `heartbeat/mod.rs` | **Existing** (mod.rs:138-158). Updated to include `mode: HeartbeatMode::Simple`. Same assertions. |
| `new_sets_interval_from_minutes` | `heartbeat/mod.rs` | **Existing** (mod.rs:160-165). No change needed (uses `::new()` which defaults to Simple). |
| `checkin_mode_sends_per_channel_messages` | `heartbeat/mod.rs` (new) | Create a `HeartbeatService` with `HeartbeatMode::CheckIn` and two channel prompts (`"email"` -> `"Check inbox"`, `"slack"` -> `"Review Slack"`). Run for one tick. Assert two messages were sent with correct `chat_id` (`"heartbeat:email"`, `"heartbeat:slack"`) and correct metadata (`heartbeat_type: "check_in"`, `target_channel` matching each key). |
| `checkin_mode_metadata_format` | `heartbeat/mod.rs` (new) | Verify that each check-in message's `metadata` map contains exactly two keys: `"heartbeat_type"` with value `"check_in"` and `"target_channel"` with the correct channel name string. |
| `simple_mode_preserves_empty_metadata` | `heartbeat/mod.rs` (new) | Create a `HeartbeatService` in `Simple` mode. Run for one tick. Assert the emitted message has an empty `metadata` map (exact current behavior). |
| `new_check_in_constructor` | `heartbeat/mod.rs` (new) | Call `HeartbeatService::new_check_in(5, ...)`. Assert `interval` is 300 seconds and `mode` matches `HeartbeatMode::CheckIn`. |

---

## 6. Acceptance Criteria

### E1: Discord Gateway Resume

- [ ] Import list in `channel.rs` includes `OP_RESUME` and `ResumePayload`
- [ ] After Hello (OP 10), Resume (OP 6) is sent when `session_id` is `Some`
- [ ] After Hello (OP 10), Identify (OP 2) is sent when `session_id` is `None`
- [ ] Resume payload includes `token`, `session_id`, and last `sequence` number
- [ ] `resume_url` is used as the WebSocket endpoint during Resume (already works via existing gateway URL selection at channel.rs:186-191)
- [ ] `RESUMED` dispatch event is handled (logged, session state preserved)
- [ ] OP 9 (Invalid Session, `d: false`) clears `session_id`, `resume_url`, and `sequence`
- [ ] OP 9 (Invalid Session, `d: true`) waits 1-5 seconds (random jitter) then reconnects without clearing session state
- [ ] OP 7 (Reconnect) preserves session state (existing behavior, verified by test)
- [ ] All existing Discord tests pass unchanged
- [ ] New unit tests: Resume chosen with session, Identify chosen without session, Invalid Session (true/false) behavior, Reconnect preserves state
- [ ] `cargo clippy -p clawft-channels -- -D warnings` is clean
- [ ] `cargo test -p clawft-channels` -- all tests pass

### E6: Enhanced Heartbeat

- [ ] `HeartbeatMode` enum defined with `Simple` and `CheckIn` variants
- [ ] `HeartbeatService` struct includes `mode` field
- [ ] `HeartbeatService::new()` defaults to `HeartbeatMode::Simple`
- [ ] `HeartbeatService::new_check_in()` constructor creates `CheckIn` mode service
- [ ] `Simple` mode produces messages with exact same fields as current implementation (empty metadata, `chat_id: "heartbeat"`)
- [ ] `CheckIn` mode produces one message per channel with `chat_id: "heartbeat:{channel_name}"`
- [ ] `CheckIn` messages include `metadata: { "heartbeat_type": "check_in", "target_channel": "{name}" }`
- [ ] All 4 existing heartbeat tests pass (with `mode: HeartbeatMode::Simple` added to struct literals)
- [ ] New tests: check-in per-channel messages, metadata format, simple mode empty metadata, constructor
- [ ] TODO comment for E6.7 CronService integration placed in code
- [ ] `cargo clippy -p clawft-services -- -D warnings` is clean
- [ ] `cargo test -p clawft-services` -- all tests pass

---

## 7. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Discord changes Resume semantics in Gateway v11+ | Low | Medium | Pin to Gateway v10 protocol. The `v` field in READY events confirms the version. If v11 breaks Resume, the Identify fallback path still works. |
| Resume fails silently if `sequence` was missed or stale | Medium | Low | Discord sends Invalid Session (OP 9) if Resume cannot be honored. The handler (E1.4) falls back to Identify correctly. Missed events between disconnect and Identify are inherent to the Identify path -- Resume actually reduces this window. |
| Random jitter implementation using `DefaultHasher` is low-entropy | Low | Low | Jitter is not security-critical -- it only prevents thundering herd on mass reconnects. `SystemTime::now()` provides sufficient entropy for this purpose. If `rand` is in the dependency tree, prefer `thread_rng().gen_range()`. |
| Adding `mode` field to `HeartbeatService` is a breaking change for code constructing via struct literal | Medium | Low | Only test code uses struct literal construction (production code uses `::new()`). The fix is mechanical: add `mode: HeartbeatMode::Simple` to each test's struct literal. |
| E6.7 CronService integration blocked on B4 | High | Medium | E6 is designed so that `Simple` and `CheckIn` modes both work without CronService (using `tokio::time::interval`). CronService integration is an enhancement, not a blocker for E6's core functionality. The TODO comment ensures this is not forgotten. |
| `HeartbeatMode::CheckIn` with many channels could flood the message bus | Low | Medium | In practice, channel count is small (typically 3-5). If needed, add a configurable rate limit or stagger per-channel sends with a small delay between them. Not needed for initial implementation. |
| `InboundMessage.metadata` type mismatch between heartbeat and Discord | Low | Low | Both use `HashMap<String, serde_json::Value>`. The heartbeat metadata uses `Value::String` variants which are consistent with how Discord channel metadata is already populated (see channel.rs:112-133). |
