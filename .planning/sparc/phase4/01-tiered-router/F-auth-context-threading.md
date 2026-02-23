# SPARC Implementation Plan: Phase F - Auth Context Threading

## Agent Instructions

### Context
This is Phase F of the Tiered Router sprint. The clawft project is a Rust AI assistant
framework. The CLI binary is named `weft`. This phase threads authentication context
from channel plugins through the pipeline so the TieredRouter can enforce per-user
permissions, rate limits, tier access, and budget constraints.

### Dependencies
**Depends on Phase B (UserPermissions + PermissionResolver) and Phase C (TieredRouter core):**
- `UserPermissions` struct with `zero_trust_defaults()`, `user_defaults()`, `admin_defaults()` constructors -- defined in `clawft_types::routing` (Phase A)
- `AuthContext` struct with `sender_id`, `channel`, `permissions` fields -- defined in `clawft_types::routing` (Phase A)
- `PermissionLevelConfig` struct for partial config overrides -- defined in `clawft_types::routing` (Phase A). Note: `PermissionOverrides` was removed; use `PermissionLevelConfig` instead.
- `PermissionResolver` struct with `resolve(sender_id, channel, allow_from_match: bool) -> UserPermissions` method -- defined in `clawft-core/src/pipeline/permissions.rs` (Phase B). This is the ONLY permission resolution implementation; Phase F must NOT re-implement any resolution or merge logic.
- `PermissionResolver::from_config(routing, channels, workspace_routing)` constructor
- `TieredRouter` reads `request.auth_context` in its `route()` implementation
- `ChatRequest` already has `auth_context: Option<AuthContext>` with `#[serde(default, skip_deserializing)]` -- added by Phase C. Phase F does NOT add this field.

### Planning Documents to Reference
- `08-tiered-router.md` -- Section 3.3 (auth context threading flow), Section 9.2 (channel auth to router), Section 9.3 (allow_from compatibility)
- `B-permissions-resolution.md` -- Phase B AuthContext, UserPermissions, PermissionResolver
- `crates/clawft-core/src/pipeline/traits.rs` -- ChatRequest struct that gains `auth_context` field
- `crates/clawft-types/src/event.rs` -- InboundMessage struct (already has `sender_id` and `channel` fields)
- `crates/clawft-types/src/config.rs` -- ChannelsConfig with `allow_from` lists per channel
- `crates/clawft-core/src/agent/loop_core.rs` -- AgentLoop that calls `process_message()`
- `crates/clawft-channels/src/telegram/channel.rs` -- Telegram sender_id extraction pattern
- `crates/clawft-channels/src/discord/channel.rs` -- Discord sender_id extraction pattern
- `crates/clawft-channels/src/slack/channel.rs` -- Slack sender_id extraction pattern
- `crates/clawft-cli/src/commands/agent.rs` -- CLI InboundMessage construction

### Files to Modify (no new files)
- `crates/clawft-core/src/agent/loop_core.rs` -- resolve and attach AuthContext before pipeline call
- `crates/clawft-cli/src/commands/agent.rs` -- change CLI sender_id from `"user"` to `"local"`

**NOT modified by Phase F** (owned by Phase C):
- `crates/clawft-core/src/pipeline/traits.rs` -- Phase C already added `auth_context: Option<AuthContext>` to ChatRequest with `#[serde(default, skip_deserializing)]`. Phase F does NOT touch this file for the field addition.

### Files That Are Already Correct (no changes needed)
- Channel plugins (Telegram, Discord, Slack) already set `sender_id` and `channel` as
  top-level fields on `InboundMessage`. These are NOT in metadata -- they are dedicated
  struct fields. No channel plugin changes are required.
- `clawft-types/src/event.rs` -- `InboundMessage` already has `pub sender_id: String`
  and `pub channel: String` fields.

### Branch
Work on branch: `weft/tiered-router`

---

## 1. Specification

### 1.1 Goal

Thread `AuthContext` from channel plugins through the pipeline to the TieredRouter. Add
an optional `auth_context: Option<AuthContext>` field to `ChatRequest`. The `AgentLoop`
resolves `UserPermissions` from config + `sender_id` + `channel` using the
`PermissionResolver` (Phase B) and attaches the result as an `AuthContext` on the
`ChatRequest`. The `PipelineRegistry` passes the `ChatRequest` (including `auth_context`)
through to the `TieredRouter`, which reads it for permission-aware routing.

### 1.2 Data Flow

```
Channel Plugin (Telegram / Slack / Discord / CLI)
  |  Each plugin already identifies the sender via platform-specific auth:
  |    - Telegram: update.message.from.id (numeric, as string)
  |    - Slack:    event.user (Slack user ID, e.g. "U12345")
  |    - Discord:  message.author.id (snowflake, as string)
  |    - CLI:      "local" (constant)
  |  Each plugin already sets InboundMessage.sender_id and InboundMessage.channel
  |  Each plugin already checks allow_from and rejects disallowed users
  v
InboundMessage { channel: String, sender_id: String, chat_id, content, metadata }
  |  (InboundMessage already carries sender_id and channel as top-level fields)
  v
AgentLoop::process_message(inbound)
  |  Reads inbound.sender_id and inbound.channel directly (not from metadata)
  |  Calls PermissionResolver::resolve(sender_id, channel, allow_from_match) -> UserPermissions
  |  Constructs AuthContext { sender_id, channel, permissions }
  |  Attaches auth_context to ChatRequest
  v
ChatRequest { messages, tools, model, max_tokens, temperature, auth_context }
  |
  v
PipelineRegistry::complete(request)
  |  Passes ChatRequest through all 6 pipeline stages unchanged
  v
TieredRouter::route(request, profile)
  |  Reads request.auth_context (defaults to zero_trust if None)
  |  Uses permissions for tier filtering, escalation, rate limiting, budget checks
  v
RoutingDecision { provider, model, reason, tier, cost_estimate }
```

### 1.3 Key Insight: InboundMessage Already Has What We Need

The `InboundMessage` struct in `clawft-types/src/event.rs` already has dedicated
`sender_id: String` and `channel: String` fields. Channel plugins already populate
these correctly:

| Channel | sender_id source | Current code |
|---------|------------------|--------------|
| Telegram | `msg.from.as_ref().map(u.id.to_string())` | `crates/clawft-channels/src/telegram/channel.rs:113-117` |
| Discord | `msg.author.id` | `crates/clawft-channels/src/discord/channel.rs:100` |
| Slack | `event.user.as_deref().unwrap_or_default()` | `crates/clawft-channels/src/slack/channel.rs:156` |
| CLI | `"user"` (currently; should change to `"local"`) | `crates/clawft-cli/src/commands/agent.rs:174,282` |

The only channel-side change needed is the CLI: rename `sender_id` from `"user"` to
`"local"` to match the convention established in the design doc (Section 3.3) and
Phase B (`PermissionResolver` checks `channel == "cli"` for admin default, and the
`AuthContext::default()` uses `sender_id = "local"`).

### 1.4 Requirements

| Requirement | Detail |
|-------------|--------|
| ChatRequest has auth_context (Phase C) | `pub auth_context: Option<AuthContext>` with `#[serde(default, skip_deserializing)]` -- already added by Phase C. Phase F does NOT add this field. |
| Backward compatibility | Existing code creating ChatRequest without auth_context must compile. `Option::None` is the default. |
| AgentLoop resolves auth | `process_message()` extracts sender_id/channel from InboundMessage, calls PermissionResolver, attaches AuthContext |
| Permission resolution | Uses Phase B's `PermissionResolver` exclusively (the ONLY resolution implementation). AgentLoop calls `resolver.resolve(sender_id, channel, allow_from_match)`. Phase F does NOT re-implement any resolution or merge logic. |
| None -> zero_trust | When auth_context is None (absent), the TieredRouter treats the request as zero_trust (Level 0) |
| CLI always admin | When channel == "cli", PermissionResolver returns Level 2 (admin) by default |
| CLI sender_id change | CLI changes sender_id from "user" to "local" for consistency with Phase B conventions |
| No new files | All changes extend existing files. AuthContext and UserPermissions types are imported from `clawft_types::routing` (Phase A). PermissionResolver comes from Phase B. Do NOT create `clawft-types/src/auth.rs`. Do NOT redefine any types. |

---

## 2. Pseudocode

### 2.1 ChatRequest Extension -- Owned by Phase C (reference only)

Phase C already added the `auth_context` field to ChatRequest. Phase F does NOT modify
`crates/clawft-core/src/pipeline/traits.rs` for this purpose. The field as defined by
Phase C is:

```rust
/// Authentication context for permission-aware routing.
/// When None, the router defaults to zero-trust behavior.
/// Populated server-side ONLY by AgentLoop from InboundMessage sender identity.
/// SECURITY: #[serde(skip_deserializing)] prevents injection via gateway JSON input.
/// AuthContext is set by channel plugins and AgentLoop -- never from user JSON input.
#[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
pub auth_context: Option<AuthContext>,
```

Key security properties (set by Phase C, documented here for Phase F implementers):
- `#[serde(skip_deserializing)]` prevents clients from injecting AuthContext via the
  gateway API JSON body. Even if a malicious client sends `"auth_context": {...}` in
  their request JSON, serde will ignore it and the field will be `None`.
- AuthContext is populated server-side ONLY by channel plugins and the AgentLoop.
- `#[serde(skip_serializing_if = "Option::is_none")]` keeps serialized JSON clean when
  auth is absent.

### 2.2 AgentLoop::process_message Enhancement (clawft-core/src/agent/loop_core.rs)

The agent loop already receives `InboundMessage` with `sender_id` and `channel` as
top-level fields. The enhancement adds PermissionResolver integration:

```rust
// AgentLoop gains a PermissionResolver field:
pub struct AgentLoop<P: Platform> {
    config: AgentsConfig,
    platform: Arc<P>,
    bus: Arc<MessageBus>,
    pipeline: PipelineRegistry,
    tools: ToolRegistry,
    context: ContextBuilder<P>,
    sessions: SessionManager<P>,
    permission_resolver: PermissionResolver,  // NEW
}

// In AgentLoop::new(), construct PermissionResolver from config:
// permission_resolver: PermissionResolver::from_config(
//     &full_config.routing,
//     &full_config.channels,
//     workspace_routing.as_ref(),
// )

// In AgentLoop::process_message():
async fn process_message(&self, msg: InboundMessage) -> clawft_types::Result<()> {
    let session_key = msg.session_key();
    let mut session = self.sessions.get_or_create(&session_key).await?;
    session.add_message("user", &msg.content, None);

    let context_messages = self.context.build_messages(&session, &[]).await;
    let mut messages: Vec<LlmMessage> = context_messages
        .iter()
        .map(|m| LlmMessage {
            role: m.role.clone(),
            content: m.content.clone(),
            tool_call_id: None,
            tool_calls: None,
        })
        .collect();
    messages.push(LlmMessage {
        role: "user".into(),
        content: msg.content.clone(),
        tool_call_id: None,
        tool_calls: None,
    });

    // --- NEW: Resolve auth context from InboundMessage identity ---
    let auth_context = self.resolve_auth_context(&msg);

    let request = ChatRequest {
        messages,
        tools: self.tools.schemas(),
        model: Some(self.config.defaults.model.clone()),
        max_tokens: Some(self.config.defaults.max_tokens),
        temperature: Some(self.config.defaults.temperature),
        auth_context: Some(auth_context),  // NEW
    };

    let response_text = self.run_tool_loop(request).await?;
    // ... rest unchanged ...
}

/// Resolve AuthContext from the InboundMessage's sender identity.
///
/// Uses Phase B's PermissionResolver exclusively (the ONLY resolution
/// implementation). Performs the 5-layer permission resolution:
/// built-in defaults -> global config -> workspace config ->
/// user overrides -> channel overrides (channel is highest priority,
/// per design doc Section 3.2).
///
/// Phase F does NOT re-implement any resolution or merge logic.
/// The allow_from_match parameter is determined by checking whether
/// the sender_id appears in the channel's allow_from list.
fn resolve_auth_context(&self, msg: &InboundMessage) -> AuthContext {
    let allow_from_match = self.permission_resolver
        .is_in_allow_from(&msg.sender_id, &msg.channel);
    let permissions = self.permission_resolver.resolve(
        &msg.sender_id,
        &msg.channel,
        allow_from_match,
    );
    AuthContext {
        sender_id: msg.sender_id.clone(),
        channel: msg.channel.clone(),
        permissions,
    }
}
```

### 2.3 Permission Level Resolution Logic (Phase B -- reference only)

Phase B's `PermissionResolver` is the ONLY permission resolution implementation.
Phase F calls `resolver.resolve(sender_id, channel, allow_from_match)` and does NOT
re-implement any of the logic below. This section is included for reference only.

The `PermissionResolver::resolve(sender_id, channel, allow_from_match)` method performs:

```
1. Check routing.permissions.users[sender_id] -> use that entry's level
2. Check routing.permissions.channels[channel] -> use that entry's level
3. If allow_from_match == true -> Level 1 (user)
4. If channel == "cli" -> Level 2 (admin)
5. Otherwise -> Level 0 (zero_trust)

IMPORTANT: Unknown levels in defaults_for_level() MUST return zero_trust (Level 0),
NOT admin. This is a security invariant.

Then layer overrides on top of the level defaults:
  built-in defaults for level (unknown -> zero_trust)
  -> merge global routing.permissions.<level_name>
  -> merge workspace routing.permissions.<level_name>
  -> merge routing.permissions.users.<sender_id>
  -> merge routing.permissions.channels.<channel>  (highest priority, per design doc Section 3.2)
```

The `allow_from_match` boolean is determined by the caller (Phase F's
`resolve_auth_context()`) by checking whether the sender_id appears in the
channel's `allow_from` list via `resolver.is_in_allow_from(sender_id, channel)`.

### 2.4 Channel Plugin Sender ID Extraction (already implemented)

Each channel plugin already sets `sender_id` on `InboundMessage`. No changes needed
to channel plugins except CLI.

```rust
// Telegram (crates/clawft-channels/src/telegram/channel.rs:113-117):
// Already implemented:
let sender_id = msg.from.as_ref().map(|u| u.id.to_string()).unwrap_or_default();
// Sets InboundMessage { channel: "telegram", sender_id, ... }

// Discord (crates/clawft-channels/src/discord/channel.rs:100):
// Already implemented:
let sender_id = &msg.author.id;
// Sets InboundMessage { channel: "discord", sender_id: sender_id.clone(), ... }

// Slack (crates/clawft-channels/src/slack/channel.rs:156):
// Already implemented:
let sender_id = event.user.as_deref().unwrap_or_default();
// Sets InboundMessage { channel: "slack", sender_id: sender_id.to_owned(), ... }

// CLI (crates/clawft-cli/src/commands/agent.rs:174,282):
// CHANGE NEEDED: "user" -> "local"
let inbound = InboundMessage {
    channel: "cli".into(),
    sender_id: "local".into(),  // Changed from "user" to "local"
    chat_id: "cli-session".into(),
    content: message.to_owned(),
    timestamp: chrono::Utc::now(),
    media: vec![],
    metadata: HashMap::new(),
};
```

### 2.5 Backward Compatibility: None -> Zero Trust Default

When `auth_context` is `None` on a `ChatRequest`, the TieredRouter (Phase C) applies
the zero-trust default:

```rust
// In TieredRouter::route() (Phase C code, not modified by Phase F):
let auth = request.auth_context.as_ref()
    .cloned()
    .unwrap_or_default();  // AuthContext::default() -> zero_trust permissions
let permissions = &auth.permissions;
```

The `AuthContext::default()` implementation (from Phase A/B) returns:
- `sender_id: ""` (empty)
- `channel: ""` (empty)
- `permissions: UserPermissions::default()` (zero_trust-level defaults)

This means existing code paths that create `ChatRequest` without setting `auth_context`
(e.g., internal tool-use loop re-invocations, tests) automatically get zero-trust
behavior, which is the most restrictive and therefore safest default.

### 2.6 PipelineRegistry Pass-Through

The `PipelineRegistry::complete()` method already passes the entire `ChatRequest` through
to the router:

```rust
// In PipelineRegistry::complete() (already implemented, no changes needed):
let routing = pipeline.router.route(request, &profile).await;
```

The `ChatRequest` reference carries `auth_context` through all 6 pipeline stages. The
router reads it; other stages ignore it. No changes needed to `PipelineRegistry`.

---

## 3. Architecture

### 3.1 File Changes

| File | Change Type | Lines Changed | Description |
|------|-------------|---------------|-------------|
| `crates/clawft-core/src/pipeline/traits.rs` | **No change by Phase F** | 0 | Phase C already added `auth_context: Option<AuthContext>` with `#[serde(default, skip_deserializing)]`. Phase F does NOT modify this file for the field addition. Phase F may update test construction sites if needed (adding `auth_context: None`), but the struct change is Phase C's responsibility. |
| `crates/clawft-core/src/agent/loop_core.rs` | Edit | ~30 | Add `PermissionResolver` field to `AgentLoop`. Add `resolve_auth_context()` method that calls `resolver.resolve(sender_id, channel, allow_from_match)` -- delegating entirely to Phase B's implementation. Wire `auth_context` into `ChatRequest` in `process_message()`. Update `AgentLoop::new()` signature to accept `PermissionResolver`. |
| `crates/clawft-cli/src/commands/agent.rs` | Edit | ~4 | Change `sender_id: "user"` to `sender_id: "local"` at two construction sites (single-message mode line 174 and REPL mode line 282). |

### 3.2 No New Files -- Import Only

All types needed by this phase are defined in earlier phases. Phase F does NOT create
any new type definition files. In particular, do NOT create `clawft-types/src/auth.rs`.
Do NOT redefine `UserPermissions`, `AuthContext`, or any other types. Import only.

- `AuthContext` -- Phase A (`clawft-types/src/routing.rs`), import as `clawft_types::routing::AuthContext`
- `UserPermissions` -- Phase A (`clawft-types/src/routing.rs`), import as `clawft_types::routing::UserPermissions`
- `PermissionLevelConfig` -- Phase A (`clawft-types/src/routing.rs`), for partial config overrides (replaces the removed `PermissionOverrides`)
- `PermissionResolver` -- Phase B (`clawft-core/src/pipeline/permissions.rs`), the ONLY permission resolution implementation

This phase only **wires** those types into the existing pipeline flow. It does NOT
implement any resolution or merge logic.

### 3.3 Dependency Chain

```
Phase A: clawft-types/src/routing.rs
  |  AuthContext, UserPermissions, PermissionLevelConfig (type definitions)
  |  All types imported from clawft_types::routing -- NOT redefined
  v
Phase B: clawft-core/src/pipeline/permissions.rs
  |  PermissionResolver (the ONLY resolution implementation)
  |  resolve(sender_id, channel, allow_from_match) -> UserPermissions
  v
Phase C: clawft-core/src/pipeline/traits.rs
  |  ChatRequest gains auth_context: Option<AuthContext>
  |  with #[serde(default, skip_deserializing)] -- prevents JSON injection
  v
Phase F (this phase): wiring only
  |
  +-- clawft-core/src/agent/loop_core.rs
  |     AgentLoop gains PermissionResolver field
  |     process_message() calls resolve_auth_context()
  |     resolve_auth_context() delegates to Phase B's resolver.resolve()
  |     ChatRequest construction includes auth_context
  |
  +-- clawft-cli/src/commands/agent.rs
  |     CLI sender_id "user" -> "local"
  |
  v
Phase C: TieredRouter reads request.auth_context in route()
  (already handles None -> zero_trust default)
```

### 3.4 Crate Dependencies

The `clawft-core` crate already depends on `clawft-types`. Adding the `AuthContext`
import to `pipeline/traits.rs` does not introduce any new crate dependencies. The
`PermissionResolver` is in `clawft-core` itself (added by Phase B), so no cross-crate
dependency changes are needed.

### 3.5 Integration with allow_from

The existing `allow_from` mechanism remains the **first gate** -- a binary access
control enforced by channel plugins before messages ever reach the `AgentLoop`. The
permission system is the **second gate** -- a granular capability control applied
inside the `AgentLoop`.

```
Channel Plugin (Telegram/Slack/Discord)
  |  Checks allow_from: is this user permitted to talk to the bot?
  |  If NOT allowed -> reject message (never reaches pipeline)
  |  If allowed (or allow_from is empty) -> forward to bus
  v
MessageBus
  v
AgentLoop::process_message()
  |  Reads sender_id and channel from InboundMessage
  |  Calls PermissionResolver::resolve() for capability-level permissions
  |  allow_from membership is one input to the resolver:
  |    - User in allow_from -> at least Level 1 (user)
  |    - User NOT in allow_from but list is non-empty -> never reaches here (rejected above)
  |    - allow_from empty (allow all) -> channel/user config determines level
  v
ChatRequest with AuthContext
```

| allow_from status | Permission behavior |
|-------------------|--------------------|
| allow_from is empty (allow all) | All users get channel-default permission level |
| User is in allow_from | User gets Level 1 (user) minimum, unless user/channel config overrides higher |
| User is NOT in allow_from (non-empty list) | Rejected by channel plugin; never reaches AgentLoop |
| CLI channel (no allow_from concept) | Always Level 2 (admin) unless routing.permissions.channels.cli overrides |

### 3.6 The PermissionResolver and allow_from Integration

The `PermissionResolver` (Phase B) needs access to the channel `allow_from` lists to
determine if a sender is an authenticated user for a given channel. This is passed at
construction time via `PermissionResolver::from_config()`, which extracts `allow_from`
lists from the `ChannelsConfig`.

In the `AgentLoop`, the resolver is constructed once at startup. When `resolve()` is
called with `(sender_id, channel, allow_from_match)`, it checks:
1. Is this sender_id in `routing.permissions.users`? -> use that level
2. Is this channel in `routing.permissions.channels`? -> use that level
3. Is `allow_from_match == true`? -> Level 1
4. Is this the CLI channel? -> Level 2
5. Otherwise -> Level 0 (zero_trust)

Unknown levels in `defaults_for_level()` MUST return zero_trust (not admin).

---

## 4. Refinement

### 4.1 Backward Compatibility (Critical)

This is the single most important concern for Phase F. Existing code creates `ChatRequest`
in multiple places, and all of those sites must continue to compile after adding the
new field.

**Strategy:** Phase C already added the `auth_context` field with `Option<AuthContext>`
and `#[serde(default, skip_deserializing)]`. Phase C also updated all existing
`ChatRequest` construction sites to include `auth_context: None`. By the time Phase F
runs, this is already done.

| Scenario | Behavior |
|----------|----------|
| Existing Rust code | Already updated by Phase C to include `auth_context: None` |
| Existing JSON: `{"messages": [...]}` | Deserializes with `auth_context: None` (no error) |
| Malicious JSON: `{"auth_context": {...}, "messages": [...]}` | `#[serde(skip_deserializing)]` ignores injected auth_context; field remains `None` |
| New code from AgentLoop (Phase F) | Sets `auth_context: Some(auth)` |
| Tool loop re-invocation | The `ChatRequest` is reused with its existing `auth_context` |

Phase F's only construction-site change is in `AgentLoop::process_message()` where
it sets `auth_context: Some(auth_context)` using the result of `resolve_auth_context()`.

### 4.2 allow_from Interaction

The existing `allow_from` fields on channel configs (`TelegramConfig.allow_from`,
`DiscordConfig.allow_from`, `SlackConfig.dm.allow_from`, etc.) continue to function
exactly as before for binary access control. The permission system adds a second layer:

**Binary access control (allow_from):** Can this user talk to the bot at all?
- Enforced by channel plugins before messages reach the pipeline.
- Empty list = allow everyone. Non-empty list = only listed users.
- Unchanged by this phase.

**Capability control (permissions):** What can this user do?
- Enforced by the TieredRouter and ToolRegistry after the message reaches the pipeline.
- Determined by the 5-layer permission resolution algorithm.
- `allow_from` membership is one input: if the sender is in `allow_from`, they get
  at least Level 1 (user) permissions.

This two-gate design means no existing configs break. An empty `allow_from` list still
means "allow everyone" at the channel access level. The permission level of those
"everyone" users depends on the routing config.

### 4.3 Channel Plugin Changes Are Minimal

The channel plugins already populate `sender_id` and `channel` on `InboundMessage`.
The only change is to the CLI:

**Before (current code in agent.rs:174):**
```rust
sender_id: "user".into(),
```

**After:**
```rust
sender_id: "local".into(),
```

This change is necessary because the `PermissionResolver` (Phase B) uses `sender_id`
as a key for per-user permission lookups. The design doc consistently uses `"local"`
as the CLI sender_id, and `AuthContext::default()` (the fallback for missing auth)
uses `sender_id: ""` (empty string), not `"local"`. The CLI should be identifiable
as a specific known sender.

### 4.4 Edge Cases

| Edge Case | Handling |
|-----------|----------|
| InboundMessage with empty sender_id | PermissionResolver treats empty sender_id as anonymous. Falls through to channel default or zero_trust. |
| InboundMessage from unknown channel name | No channel override applies. Falls through to zero_trust unless user has per-user config. |
| AgentLoop process_message called with CLI InboundMessage | sender_id = "local", channel = "cli". Resolver returns Level 2 (admin) by default. |
| Tool loop re-invocations | The `ChatRequest` object is reused across tool loop iterations. The `auth_context` persists -- the same user's permissions apply to all tool calls in the loop. |
| Streaming pipeline (complete_stream) | Same as non-streaming. The `ChatRequest` carries auth_context through both code paths. |
| Config has no routing section | `RoutingConfig::default()` has mode = "static". StaticRouter ignores auth_context entirely. PermissionResolver still works (returns built-in defaults). |
| Config has routing but no permissions section | `PermissionsConfig::default()` has no overrides. All resolution uses built-in defaults. CLI gets admin; channels get zero_trust. |
| Deserialization of ChatRequest from old JSON | `auth_context` field is absent in JSON. `#[serde(default, skip_deserializing)]` means it deserializes as `None` regardless of whether the field is present or absent in the input. |
| Serialization of ChatRequest with auth_context: None | `skip_serializing_if = "Option::is_none"` omits the field from JSON output. |
| Admin user on a channel with rate_limit config | The channel-level rate limit takes priority over the user-level permissions (admin, no rate limit) because channel overrides are highest priority in the merge (design doc Section 3.2: per-channel > per-user). To bypass a channel restriction for a specific user, modify the channel config directly. |
| Per-user budget override | A user with explicit `cost_budget_daily_usd` in `routing.permissions.users` gets that budget, overriding the level default. |

### 4.5 Security Considerations

**Sender ID authenticity:** The `sender_id` values come from platform-authenticated
sources (Telegram Bot API, Discord Gateway, Slack Socket Mode). Channel plugins extract
sender identity from platform events, NOT from user-supplied message content. A malicious
user cannot spoof their sender_id through the message text.

**AuthContext injection prevention:** The `ChatRequest.auth_context` field has
`#[serde(skip_deserializing)]` (set by Phase C). This means AuthContext is set
server-side ONLY by channel plugins and the AgentLoop -- never from user JSON input.
Even if a client sends `"auth_context": {"sender_id": "admin", ...}` in gateway
request JSON, the deserializer ignores it and the field remains `None` (zero_trust).

**CLI admin access:** Local CLI users get Level 2 (admin) by default. This is a
deliberate design choice: the local user owns the machine and has filesystem access
already. For remote access, the gateway API should be used instead, which does NOT
get automatic admin.

**CLI admin safety warning for network-exposed gateways:** When the gateway is
configured with `gateway.host == "0.0.0.0"` (network-exposed) and
`cli_default_level == 2` (admin), the system MUST emit a warning at startup:

```
WARNING: Gateway is network-exposed (host=0.0.0.0) with CLI default level 2 (admin).
Remote CLI connections will receive admin permissions. Consider setting
routing.permissions.channels.cli.level to 1 or lower, or restrict gateway.host
to 127.0.0.1 for local-only access.
```

This warning is implemented in the AgentLoop or gateway startup path. It is a
non-blocking warning (does not prevent startup) but ensures operators are aware of
the security implications of the combination of network exposure and admin defaults.

**Zero-trust as default:** When `auth_context` is `None` (no auth information available),
the system defaults to zero_trust (Level 0). This is the most restrictive level:
free-tier models only, no tools, tight rate limits, minimal budget. This ensures that
any failure to thread auth context results in the safest possible behavior, not the
most permissive.

**Deterministic resolution:** Permission resolution is a pure function of config data
and sender identity. There are no side effects, no external calls, and no mutable state.
This makes it easy to test exhaustively and reason about.

### 4.6 Performance Considerations

Permission resolution adds negligible overhead to request processing:
- `PermissionResolver::resolve()` performs O(1) HashMap lookups per layer (5 layers max)
- Each merge is O(16) field assignments (16 permission dimensions)
- Total: ~O(80) operations per request, all in-memory, no allocation beyond String clones
- The resolver is constructed once at `AgentLoop` startup, not per-request

---

## 5. Completion

### 5.1 Exit Criteria

- [ ] `ChatRequest` struct has `auth_context: Option<AuthContext>` field with `#[serde(default, skip_deserializing)]` -- verified present (added by Phase C, not Phase F)
- [ ] Phase F does NOT modify `ChatRequest` struct definition (Phase C owns this)
- [ ] No types are redefined -- all imported from `clawft_types::routing`
- [ ] No `clawft-types/src/auth.rs` file exists (types live in `routing.rs`)
- [ ] JSON deserialization of ChatRequest ignores any `auth_context` in input JSON (skip_deserializing)
- [ ] JSON serialization of ChatRequest with `auth_context: None` omits the field
- [ ] `AgentLoop` has a `PermissionResolver` field
- [ ] `AgentLoop::process_message()` calls `resolve_auth_context()` and attaches result
- [ ] `AgentLoop::resolve_auth_context()` reads `sender_id` and `channel` from `InboundMessage` and delegates to Phase B's `resolver.resolve(sender_id, channel, allow_from_match)`
- [ ] No resolution or merge logic is re-implemented in Phase F (Phase B's `PermissionResolver` is the ONLY implementation)
- [ ] `defaults_for_level()` for unknown levels returns zero_trust (not admin)
- [ ] CLI `sender_id` is `"local"` (not `"user"`)
- [ ] CLI channel resolves to Level 2 (admin) via PermissionResolver
- [ ] Telegram channel sender resolves correctly (Level 1 if in allow_from)
- [ ] Unknown sender on any channel resolves to Level 0 (zero_trust)
- [ ] `PipelineRegistry::complete()` passes auth_context through without modification
- [ ] All existing tests pass after adding `auth_context: None` to ChatRequest construction
- [ ] All new tests pass
- [ ] CLI admin safety warning emitted when `gateway.host == "0.0.0.0"` with `cli_default_level == 2`
- [ ] Code compiles without warnings (`cargo clippy --workspace`)
- [ ] File changes stay under 500 lines per file

### 5.2 Test Plan

Minimum 17 tests across unit tests (in modified files) and integration tests (auth flow
end-to-end, deferred to Phase I).

#### ChatRequest Serde Tests (4 tests, in pipeline/traits.rs)

Note: Because `auth_context` has `#[serde(skip_deserializing)]` (set by Phase C),
deserialization always produces `None` for this field, even if the JSON contains it.
This is the intended security behavior. Roundtrip tests must account for this.

1. **`test_chat_request_skip_deserializing_auth_context`**: Create JSON with
   `"auth_context": {"sender_id": "injected", "channel": "evil", ...}` and `"messages": [...]`.
   Deserialize into ChatRequest. Verify `auth_context.is_none()` -- proving that
   `#[serde(skip_deserializing)]` prevents injection via JSON input.

2. **`test_chat_request_without_auth_context_deserializes`**: Deserialize `{"messages": [{"role": "user", "content": "hi"}]}` into ChatRequest. Verify `auth_context.is_none()`.

3. **`test_chat_request_none_auth_context_omitted_in_json`**: Create ChatRequest with `auth_context: None`. Serialize to JSON. Verify JSON string does not contain `"auth_context"`.

4. **`test_chat_request_with_auth_context_serializes_but_not_roundtrip`**: Create ChatRequest with `auth_context: Some(AuthContext { sender_id: "user_123", channel: "telegram", permissions: UserPermissions::user_defaults() })`. Serialize to JSON. Verify JSON contains `"auth_context"` and `"user_123"`. Deserialize the JSON back. Verify `auth_context.is_none()` -- confirming that `skip_deserializing` drops the field on re-read. This asymmetry is intentional: auth_context can be serialized for logging/debugging, but never accepted from external input.

#### AgentLoop Auth Resolution Tests (6 tests, in agent/loop_core.rs or a test module)

5. **`test_resolve_auth_context_cli`**: Create an InboundMessage with `channel: "cli"`, `sender_id: "local"`. Call `resolve_auth_context()`. Verify returned AuthContext has `sender_id: "local"`, `channel: "cli"`, `permissions.level: 2`.

6. **`test_resolve_auth_context_telegram_allowed_user`**: Create an InboundMessage with `channel: "telegram"`, `sender_id: "12345"`. Configure allow_from to include `"12345"`. Call `resolve_auth_context()`. Verify `permissions.level >= 1`.

7. **`test_resolve_auth_context_telegram_unknown_user`**: Create an InboundMessage with `channel: "telegram"`, `sender_id: "99999"`. Configure allow_from as `["12345"]` (does not include 99999). `is_in_allow_from()` returns false, so `resolve()` is called with `allow_from_match: false`. Verify `permissions.level == 0`.

8. **`test_resolve_auth_context_per_user_override`**: Configure `routing.permissions.users["alice_123"]` with level 2. Create InboundMessage with `sender_id: "alice_123"`. Verify `permissions.level == 2` regardless of channel.

9. **`test_resolve_auth_context_empty_sender`**: Create an InboundMessage with empty `sender_id`. Verify `permissions.level == 0` (zero_trust).

10. **`test_resolve_auth_context_gateway_channel`**: Create an InboundMessage with `channel: "gateway"`, `sender_id: "api_key_user"`. With no channel or user overrides configured, verify `permissions.level == 0` (zero_trust -- gateway users must be explicitly configured).

#### Pipeline Pass-Through Tests (2 tests, in pipeline/traits.rs)

11. **`test_pipeline_registry_passes_auth_context`**: Create a PipelineRegistry with test components. Create a ChatRequest with a specific auth_context. Call `complete()`. Verify the request passed to the router (via a capturing mock) has the same auth_context.

12. **`test_pipeline_registry_works_without_auth_context`**: Create a ChatRequest with `auth_context: None`. Call `complete()`. Verify no panic and response is returned.

#### CLI Sender ID Tests (2 tests, in clawft-cli or integration tests)

13. **`test_cli_inbound_message_sender_id_is_local`**: Verify that the CLI constructs InboundMessage with `sender_id: "local"` (not `"user"`).

14. **`test_cli_inbound_message_channel_is_cli`**: Verify that the CLI constructs InboundMessage with `channel: "cli"`.

#### CLI Admin Safety Warning Tests (2 tests)

15. **`test_cli_admin_warning_on_network_exposed_gateway`**: Configure `gateway.host = "0.0.0.0"` and `cli_default_level = 2`. Verify that startup emits a warning log message containing "network-exposed" and "admin".

16. **`test_no_cli_admin_warning_on_localhost_gateway`**: Configure `gateway.host = "127.0.0.1"` and `cli_default_level = 2`. Verify that no admin safety warning is emitted.

#### Unknown Level Safety Tests (1 test)

17. **`test_defaults_for_level_unknown_returns_zero_trust`**: Call `defaults_for_level()` with an unknown/invalid level value. Verify it returns zero_trust permissions (Level 0), NOT admin.

### 5.3 Branch

Work on branch: `weft/tiered-router`

### 5.4 Effort Estimate

**1 day** total:
- 1 hour: Verify Phase C's `auth_context` field on `ChatRequest` is in place with `#[serde(skip_deserializing)]`; update any remaining construction sites in tests
- 2 hours: Wire `PermissionResolver` into `AgentLoop`, implement `resolve_auth_context()` using Phase B's `resolver.resolve(sender_id, channel, allow_from_match)` -- no re-implementation of resolution logic
- 1 hour: Update CLI sender_id, update AgentLoop constructor signature, add CLI admin safety warning for network-exposed gateways
- 4 hours: Write and verify all 17 tests (including injection prevention, unknown level -> zero_trust, CLI admin safety warning), fix compilation issues, run full test suite

### 5.5 Dependencies

| Dependency | Status | What This Phase Needs |
|------------|--------|----------------------|
| Phase A (RoutingConfig types) | Must be complete | `AuthContext` type definition in `clawft-types/src/routing.rs` |
| Phase B (PermissionResolver) | Must be complete | `PermissionResolver` struct, `resolve(sender_id, channel, allow_from_match)` method, `is_in_allow_from(sender_id, channel)` method, `from_config()` constructor |
| Phase C (TieredRouter core) | Must be complete | `TieredRouter::route()` reads `request.auth_context` |

### 5.6 Validation Commands

```bash
# Build entire workspace
cargo build --workspace

# Lint
cargo clippy --workspace

# Run all tests
cargo test --workspace

# Run only pipeline tests
cargo test --package clawft-core pipeline::traits::tests

# Run only agent loop tests
cargo test --package clawft-core agent

# Run CLI tests
cargo test --package clawft-cli
```

### 5.7 What This Phase Does NOT Include

- No new types -- all types imported from `clawft_types::routing` (Phase A)
- No `clawft-types/src/auth.rs` -- do NOT create this file
- No redefinition of `UserPermissions` or `AuthContext` structs
- No re-implementation of resolution or merge logic -- use Phase B's `PermissionResolver` exclusively
- No modification of `ChatRequest` struct definition -- Phase C owns the `auth_context` field
- No PermissionResolver implementation (Phase B)
- No TieredRouter implementation (Phase C)
- No CostTracker (Phase D)
- No RateLimiter (Phase E)
- No tool permission enforcement (Phase G)
- No config validation (Phase H)
- No integration test fixtures (Phase I)
- No new files

This phase is purely wiring: connecting existing types and logic into the message
processing pipeline. All resolution logic is delegated to Phase B's `PermissionResolver`
via `resolver.resolve(sender_id, channel, allow_from_match)`.

---

**End of SPARC Plan**

---

## Remediation Applied

**Date**: 2026-02-18
**Source**: remediation-plan.md

The following fixes from the remediation plan have been applied to this Phase F document:

### FIX-01: Canonical Type Ownership (CRITICAL)

- Removed all references to creating `clawft-types/src/auth.rs`. All types are imported from `clawft_types::routing`.
- Removed references to redefining `UserPermissions` or `AuthContext` structs. Phase F imports only.
- Removed all re-implementation of merge logic. Phase F uses Phase B's `PermissionResolver` exclusively.
- Replaced `PermissionOverrides` references with `PermissionLevelConfig` (Phase A type).
- Documented that `defaults_for_level()` for unknown levels MUST return zero_trust (not admin).
- Added exit criterion and test (test 17) for unknown level -> zero_trust behavior.

### FIX-02: Single Permission Resolution (CRITICAL)

- Updated `PermissionResolver.resolve()` signature throughout to 3-parameter form: `resolve(sender_id, channel, allow_from_match: bool)`.
- Phase F's `resolve_auth_context()` now determines `allow_from_match` via `resolver.is_in_allow_from()` and passes it to `resolve()`.
- Documented explicitly that Phase B's `PermissionResolver` is the ONLY resolution implementation. Phase F does NOT re-implement any resolution logic.
- Section 2.3 marked as "reference only" to clarify Phase F does not own this logic.

### FIX-05: AuthContext Injection Prevention (HIGH SECURITY)

- Updated all serde annotations from `#[serde(default)]` to `#[serde(default, skip_deserializing)]` to reflect Phase C's change.
- Documented that `ChatRequest.auth_context` has `#[serde(skip_deserializing)]` set by Phase C.
- Documented that AuthContext is set server-side ONLY by channel plugins and AgentLoop, never from user JSON input.
- Updated serde test 1 to verify injection prevention (deserializing JSON with auth_context produces None).
- Updated serde test 4 to verify asymmetric serialize/deserialize behavior (serializes for logging, but never accepted from input).
- Added security considerations section on injection prevention.

### FIX-09: ChatRequest Extension Ownership (HIGH)

- Phase C owns the `ChatRequest` extension (adding `auth_context` field). Phase F does NOT add this field.
- Section 2.1 rewritten as "Owned by Phase C (reference only)".
- Section 3.1 file changes table updated: `traits.rs` marked as "No change by Phase F".
- Section 3.3 dependency chain updated to show Phase C between Phase B and Phase F.
- Section 4.1 backward compatibility updated to note Phase C already handled all construction site updates.
- Exit criteria updated to verify Phase F does NOT modify `ChatRequest` struct definition.

### FIX-12: CLI Admin Safety Warning (MEDIUM)

- Added CLI admin safety warning requirement: when `gateway.host == "0.0.0.0"` (network-exposed) and `cli_default_level == 2` (admin), emit a startup warning.
- Added warning text template to Section 4.5 (Security Considerations).
- Added exit criterion for the warning.
- Added 2 tests (tests 15-16) for warning presence on network-exposed gateway and absence on localhost.
- Total test count increased from 14 to 17.
