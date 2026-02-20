# Workstream I: Type Safety & Cleanup

## Overview

| Field | Value |
|-------|-------|
| Priority | P2 (Week 2-4) |
| Branch | `sprint/phase-5-5A` (shared with Workstream A) |
| Items | I1-I8 (8 tasks) |
| Dependencies | I2 should target post-B3 file paths if `config.rs` has been split |
| Parallelism | All 8 items can run in parallel except I2/B3 ordering |

These are correctness and maintainability fixes that improve type safety, remove dead code, and fix known test and serialization issues. They are lower risk than Workstreams A and B but should be completed to keep the codebase clean.

## Concurrency Plan

```
I1 (DelegationTarget serde)     ─── parallel ───┐
I3 (ChatMessage content)        ─── parallel ───┤
I4 (Job ID collision)           ─── parallel ───┤
I5 (camelCase acronym fix)      ─── parallel ───┤
I6 (Dead code removal)          ─── parallel ───┤ All independent
I7 (Always-true assertion)      ─── parallel ───┤
I8 (MockTransport sharing)      ─── parallel ───┘

I2 (String policy to enum)      ─── blocked by B3 config.rs split ───
```

---

## Task Specifications

---

### I1: `DelegationTarget` serde consistency

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-types/src/delegation.rs` |
| Lines | 87-99 |
| Risk | Low (serde attribute change with backward-compat aliases) |

#### Current Code

```rust
// crates/clawft-types/src/delegation.rs:87-99
/// Where a task should be executed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationTarget {
    /// Execute locally (built-in tool pipeline).
    Local,
    /// Delegate to Claude AI.
    Claude,
    /// Delegate to Claude Flow orchestration.
    Flow,
    /// Automatically decide based on complexity heuristics.
    #[default]
    Auto,
}
```

**Bug**: Serializes as PascalCase (`"Local"`, `"Claude"`, `"Flow"`, `"Auto"`) while other enums in the codebase use `snake_case`. Confirmed by the test at line 183-196:

```rust
// crates/clawft-types/src/delegation.rs:183-196
#[test]
fn delegation_target_variants() {
    let targets = [
        (DelegationTarget::Local, "\"Local\""),
        (DelegationTarget::Claude, "\"Claude\""),
        (DelegationTarget::Flow, "\"Flow\""),
        (DelegationTarget::Auto, "\"Auto\""),
    ];
    // ...
}
```

Compare with `TierSelectionStrategy` in `crates/clawft-types/src/routing.rs:21-22`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TierSelectionStrategy { ... }
```

#### Fix

Add `#[serde(rename_all = "snake_case")]` to `DelegationTarget`, plus backward-compatibility aliases for each variant so existing config files with PascalCase still deserialize:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationTarget {
    #[serde(alias = "Local")]
    Local,
    #[serde(alias = "Claude")]
    Claude,
    #[serde(alias = "Flow")]
    Flow,
    #[default]
    #[serde(alias = "Auto")]
    Auto,
}
```

#### Tests

1. Update `delegation_target_variants` test to expect `"local"`, `"claude"`, `"flow"`, `"auto"`.
2. Add backward-compat test: deserialize `"Local"`, `"Claude"`, `"Flow"`, `"Auto"` (PascalCase) and verify they map to the correct variants.
3. Verify `DelegationConfig` roundtrip still works (existing `delegation_config_serde_roundtrip` test).

#### Acceptance Criteria

- [ ] `DelegationTarget` serializes as `snake_case`
- [ ] PascalCase input still deserializes (backward compat)
- [ ] All existing tests in `delegation.rs` pass (updated expectations)
- [ ] `cargo clippy` clean

---

### I2: String-typed policy modes to enums

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-types/src/config.rs` (or post-B3: `config/policies.rs`) |
| Lines | 986-1015 |
| Risk | Low (enum introduction with backward compat) |
| Dependency | Should target post-B3 paths if config.rs has been split |

#### Current Code

```rust
// crates/clawft-types/src/config.rs:986-1001
/// Command execution security policy configuration.
pub struct CommandPolicyConfig {
    /// Policy mode: "allowlist" (default, recommended) or "denylist".
    #[serde(default = "default_policy_mode")]
    pub mode: String,

    /// Permitted command basenames when in allowlist mode.
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Blocked command patterns when in denylist mode.
    #[serde(default)]
    pub denylist: Vec<String>,
}

fn default_policy_mode() -> String {
    "allowlist".to_string()
}
```

**Bug**: `mode` is `String` accepting `"allowlist"` or `"denylist"`. This allows any arbitrary string, and comparisons elsewhere must use string literals rather than pattern matching. No compile-time validation.

#### Fix

Define a proper enum and replace the `String` field:

```rust
/// Policy mode for command execution security.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    /// Only explicitly permitted commands can run.
    #[default]
    Allowlist,
    /// Any command not matching a blocked pattern is allowed.
    Denylist,
}

pub struct CommandPolicyConfig {
    #[serde(default)]
    pub mode: PolicyMode,

    #[serde(default)]
    pub allowlist: Vec<String>,

    #[serde(default)]
    pub denylist: Vec<String>,
}
```

Remove `default_policy_mode()` function. Update all call sites that compare `mode` as a string (grep for `mode == "allowlist"` / `mode == "denylist"`).

#### Migration Note

If B3 splits `config.rs`, target the new `config/policies.rs` file. The `PolicyMode` enum can live alongside `CommandPolicyConfig`.

#### Tests

1. Deserialize `{"mode": "allowlist"}` and `{"mode": "denylist"}` -- both succeed.
2. Deserialize `{"mode": "invalid"}` -- returns an error.
3. Default `CommandPolicyConfig` has `mode == PolicyMode::Allowlist`.
4. Roundtrip serialization preserves `snake_case` values.

#### Acceptance Criteria

- [ ] `CommandPolicyConfig::mode` is `PolicyMode` enum, not `String`
- [ ] All string comparisons updated to pattern match on enum
- [ ] Serde roundtrip works with `snake_case`
- [ ] Invalid mode values rejected at deserialization

---

### I3: `ChatMessage::content` serialization

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-llm/src/types.rs` |
| Lines | 10-27 |
| Risk | Low (single attribute addition) |

#### Current Code

```rust
// crates/clawft-llm/src/types.rs:10-27
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,

    /// The content of the message. `None` for assistant messages that only
    /// contain tool calls (serializes as `"content": null`).
    #[serde(default)]
    pub content: Option<String>,

    /// For tool-result messages, the ID of the tool call this is a response to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Tool calls requested by the assistant in this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

**Bug**: `content: Option<String>` at line 18 serializes `None` as `"content": null`. Some LLM providers reject `null` content fields. The `tool_call_id` field at line 22 already has `#[serde(skip_serializing_if = "Option::is_none")]` -- the `content` field is missing it.

#### Fix

Add the skip annotation to `content`:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
```

#### Tests

1. Serialize a `ChatMessage` with `content: None` -- verify the JSON output does NOT contain a `"content"` key.
2. Serialize a `ChatMessage` with `content: Some("hello")` -- verify `"content": "hello"` is present.
3. Deserialize JSON without a `content` field -- verify `content` is `None` (existing `#[serde(default)]` handles this).
4. Verify existing tests still pass.

#### Acceptance Criteria

- [ ] `content: None` is omitted from serialized JSON
- [ ] `content: Some(...)` is included normally
- [ ] Deserialization still works for JSON with and without `content` field

---

### I4: Job ID collision fix

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-cli/src/commands/cron.rs` |
| Lines | 76-85 |
| Risk | Low (ID format change, no external consumers) |

#### Current Code

```rust
// crates/clawft-cli/src/commands/cron.rs:76-85
/// Generate a short random job ID.
fn generate_job_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    // Use lower 32 bits of timestamp + process id for uniqueness.
    format!("job-{:08x}{:04x}", ts as u32, std::process::id() as u16)
}
```

**Bug**: Uses millisecond timestamp + process ID. Same-millisecond collisions are possible when creating multiple jobs rapidly (e.g., in tests or scripts). The `ts as u32` truncation also means the timestamp wraps every ~49 days.

#### Fix

Use `uuid::Uuid::new_v4()` which is already a workspace dependency (`uuid = { version = "1", features = ["v4", "serde"] }` in root `Cargo.toml`):

```rust
/// Generate a unique job ID using UUID v4.
fn generate_job_id() -> String {
    format!("job-{}", uuid::Uuid::new_v4())
}
```

Ensure `uuid` is added to `crates/clawft-cli/Cargo.toml` dependencies if not already present (it is a workspace dep, so use `uuid.workspace = true`).

#### Tests

1. Generate 1000 IDs in a loop, verify all are unique.
2. Verify the format starts with `"job-"`.
3. Verify the UUID portion is valid (36 chars, correct hyphen positions).

#### Acceptance Criteria

- [ ] Job IDs use UUID v4 -- no collision risk
- [ ] Format is `job-{uuid}` (e.g., `job-550e8400-e29b-41d4-a716-446655440000`)
- [ ] `uuid` crate added to clawft-cli Cargo.toml

---

### I5: `camel_to_snake()` acronym handling

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-platform/src/config_loader.rs` |
| Lines | 123-132 |
| Risk | Medium (algorithm change affects all config key normalization) |

#### Current Code

```rust
// crates/clawft-platform/src/config_loader.rs:123-132
pub fn camel_to_snake(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}
```

**Bug**: Inserts `_` before every uppercase letter. `"HTMLParser"` becomes `"h_t_m_l_parser"` instead of `"html_parser"`. The bug is confirmed by the test at line 169:

```rust
// crates/clawft-platform/src/config_loader.rs:169
assert_eq!(camel_to_snake("HTML"), "h_t_m_l");
```

This test documents the current (buggy) behavior rather than the desired behavior.

#### Fix

Add consecutive-uppercase tracking. Insert `_` only when:
- Transitioning from lowercase to uppercase (standard camelCase boundary)
- Transitioning from an uppercase run to uppercase+lowercase (acronym end, e.g., the `P` in `HTMLParser`)

```rust
pub fn camel_to_snake(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    let chars: Vec<char> = name.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() && i > 0 {
            let prev_upper = chars[i - 1].is_uppercase();
            let next_lower = chars.get(i + 1).map_or(false, |c| c.is_lowercase());
            // Insert underscore when:
            // 1. Previous char was lowercase (standard camelCase boundary)
            // 2. Previous char was uppercase AND next char is lowercase
            //    (end of acronym, e.g., the 'P' in "HTMLParser")
            if !prev_upper || next_lower {
                result.push('_');
            }
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}
```

#### Expected Results

| Input | Current (buggy) | Fixed |
|-------|----------------|-------|
| `"camelCase"` | `"camel_case"` | `"camel_case"` |
| `"systemPrompt"` | `"system_prompt"` | `"system_prompt"` |
| `"HTMLParser"` | `"h_t_m_l_parser"` | `"html_parser"` |
| `"HTML"` | `"h_t_m_l"` | `"html"` |
| `"getHTMLParser"` | `"get_h_t_m_l_parser"` | `"get_html_parser"` |
| `"already_snake"` | `"already_snake"` | `"already_snake"` |
| `"Config"` | `"config"` | `"config"` |
| `"XMLHTTPRequest"` | `"x_m_l_h_t_t_p_request"` | `"xmlhttp_request"` |
| `""` | `""` | `""` |

#### Tests

1. Update `test_camel_to_snake_all_upper` (line 167): change expected from `"h_t_m_l"` to `"html"`.
2. Add tests for `"HTMLParser"` -> `"html_parser"`, `"getHTMLParser"` -> `"get_html_parser"`, `"XMLHTTPRequest"` -> `"xmlhttp_request"`.
3. Verify existing basic tests (`"camelCase"`, `"systemPrompt"`, `"already_snake"`) still pass unchanged.
4. Update the doc comment example at line 121 from `"h_t_m_l_parser"` to `"html_parser"`.

#### Acceptance Criteria

- [ ] Consecutive uppercase letters treated as a single acronym
- [ ] Standard camelCase boundaries still work
- [ ] Doc example updated
- [ ] All test expectations updated

---

### I6: Dead code removal

| Field | Value |
|-------|-------|
| Priority | P2 |
| Files | Multiple (see below) |
| Risk | Low (removing unused code / adding TODO markers) |

#### I6a: `evict_if_needed` in rate_limiter.rs

**File**: `crates/clawft-core/src/pipeline/rate_limiter.rs:282-286`

```rust
// crates/clawft-core/src/pipeline/rate_limiter.rs:282-286
    /// Evict oldest entries when max_tracked_users is exceeded.
    #[allow(dead_code)]
    fn evict_if_needed(&self) {
        let mut windows = self.windows.write().unwrap();
        self.evict_oldest(&mut windows);
    }
```

**Analysis**: This is a thin wrapper around `evict_oldest()` which is already called inline at line 163. The `#[allow(dead_code)]` confirms it is unused. The method `evict_oldest` (line 258) is the actual implementation and IS used.

**Fix**: Remove `evict_if_needed` entirely. It has no callers and `evict_oldest` is the real implementation.

#### I6b: `ResumePayload` in discord events

**File**: `crates/clawft-channels/src/discord/events.rs:90-100`

```rust
// crates/clawft-channels/src/discord/events.rs:90-100
/// The `d` field of an opcode 6 (Resume) payload.
#[derive(Debug, Clone, Serialize)]
pub struct ResumePayload {
    pub token: String,
    pub session_id: String,
    pub seq: u64,
}
```

**Analysis**: This is part of the Discord Gateway resume protocol. It is currently used in tests (line 370-386 `serialize_resume` test) but NOT used in production code yet -- the Discord channel does not implement session resumption. It will be needed for workstream E1 (Discord enhancements).

**Fix**: Add `// TODO(E1): Used when Discord gateway resume is implemented` comment. Do NOT remove -- it is a correct protocol type that E1 will need.

#### I6c: Interactive slash-command framework

**Files**: `crates/clawft-cli/src/interactive/` (3 files: `mod.rs`, `builtins.rs`, `registry.rs`)

```rust
// crates/clawft-cli/src/interactive/mod.rs:1-8
//! Interactive slash-command framework for the `weft agent` REPL.
pub mod builtins;
pub mod registry;
```

**Analysis**: The interactive framework (slash commands `/help`, `/skills`, `/use`, `/agent`, `/tools`, `/clear`, `/status`, `/quit`) is fully implemented with tests but is NOT wired into the agent REPL yet. It will be used when workstream C5 (interactive REPL) is implemented.

**Fix**: Add `// TODO(C5): Wire into agent REPL when interactive mode is implemented` to `mod.rs`. Do NOT remove -- the code is correct and well-tested.

#### I6d: `--trust-project-skills` and `--intelligent-routing` CLI flags

**Files**:
- `crates/clawft-cli/src/commands/agent.rs:57-67` (both flags)
- `crates/clawft-cli/src/commands/gateway.rs:62-64` (`--intelligent-routing` only)

```rust
// crates/clawft-cli/src/commands/agent.rs:57-67
    /// Enable intelligent routing (requires vector-memory feature).
    #[arg(long)]
    pub intelligent_routing: bool,

    /// Trust workspace-level (project) skills.
    #[arg(long)]
    pub trust_project_skills: bool,
```

**Analysis**:
- `--trust-project-skills` IS used: passed to `SkillLoader::discover_with_trust()` at `crates/clawft-core/src/agent/skills_v2.rs:360-361`. This is NOT dead -- do not remove.
- `--intelligent-routing` IS partially functional: when the `vector-memory` feature is enabled, it logs a message but does not actually wire up the `IntelligentRouter`. Without the feature, it bails with an error. The flag itself is NOT a no-op (it gates a code path), but the enabled path is incomplete.

**Fix**: For `--intelligent-routing`, add `// TODO(D3): Wire IntelligentRouter when vector-memory routing is fully implemented` where the stub info message is logged. Do NOT remove the flag -- it correctly feature-gates.

#### Tests

1. Verify `evict_if_needed` removal causes no test failures.
2. Verify all TODO comments reference the correct workstream IDs.
3. Run `cargo build` to confirm no dead-code warnings after changes.

#### Acceptance Criteria

- [ ] `evict_if_needed` removed from `rate_limiter.rs`
- [ ] `#[allow(dead_code)]` on `window_seconds` field (line 58) evaluated -- it IS used in construction, keep as-is
- [ ] TODO comments added for E1, C5, D3 placeholders
- [ ] `cargo clippy` clean

---

### I7: Fix always-true test assertion

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-core/src/pipeline/transport.rs` |
| Lines | 1232-1266 |
| Risk | Low (test-only change) |

#### Current Code

```rust
// crates/clawft-core/src/pipeline/transport.rs:1232-1266
#[tokio::test]
async fn streaming_fallback_for_non_streaming_provider() {
    // MockProvider does not implement complete_stream (uses default),
    // but the LlmTransport::complete_stream default impl falls back
    // to complete() and sends the full text via callback.
    let provider = Arc::new(MockProvider::text_response("Full response text"));
    let transport = OpenAiCompatTransport::with_provider(provider);

    let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected_clone = collected.clone();

    let callback: StreamCallback = Box::new(move |text| {
        collected_clone.lock().unwrap().push(text.to_string());
        true
    });

    // The default complete_stream on LlmTransport calls complete()
    // then fires the callback once with the full text. Since MockProvider
    // does not implement LlmProvider::complete_stream, the
    // OpenAiCompatTransport::complete_stream will use the spawned task
    // which will fail, but we have collected text as fallback.
    // Actually, the MockProvider's default complete_stream returns an error,
    // so the transport should use the fallback path.
    let result = transport
        .complete_stream(&make_transport_request(), callback)
        .await;

    // The result should still succeed because we fall back to non-streaming
    // when the stream provider returns an error. Actually, the default
    // LlmProvider::complete_stream returns an error, so the transport
    // spawned task fails. Since no text was collected, it propagates the error.
    // This is correct behavior: use complete() directly for providers
    // that do not support streaming.
    assert!(result.is_err() || result.is_ok());
}
```

**Bug**: Line 1265: `assert!(result.is_err() || result.is_ok())` is always true for any `Result`. The assertion tests nothing. The comments show the author was unsure whether the result should be `Ok` or `Err`.

**Analysis of expected behavior**: The comments indicate:
1. MockProvider does not implement `complete_stream` (returns error by default).
2. The transport's spawned streaming task fails.
3. "Since no text was collected, it propagates the error."
4. The next test `streaming_stub_returns_error` (line 1268-1280) tests a similar scenario and asserts `result.is_err()` with error message `"transport not configured"`.

The difference is that `streaming_fallback_for_non_streaming_provider` uses `OpenAiCompatTransport::with_provider(provider)` (configured with a provider), while `streaming_stub_returns_error` uses `OpenAiCompatTransport::new()` (no provider). With a provider, the fallback path should attempt `complete()` and may succeed. The expected behavior based on the comments is that the error propagates, so `result.is_err()` is the correct assertion.

#### Fix

Replace the tautological assertion with the correct specific assertion:

```rust
    // MockProvider's default complete_stream returns an error, and the
    // transport propagates it since no text was collected via callback.
    assert!(result.is_err());
```

However, if the fallback path actually succeeds (calls `complete()` via the MockProvider which CAN respond), then the assertion should be:

```rust
    assert!(result.is_ok());
    let chunks = collected.lock().unwrap().clone();
    assert!(!chunks.is_empty());
```

**Recommendation**: Run the test locally to determine actual behavior before committing. The test should assert whichever outcome the transport actually produces, and the comment should explain why.

#### Tests

1. Replace the always-true assertion with the correct specific outcome.
2. Add a comment explaining why that outcome is expected.
3. Verify the test passes.

#### Acceptance Criteria

- [ ] `assert!(result.is_err() || result.is_ok())` replaced with a meaningful assertion
- [ ] Comment updated to explain expected behavior
- [ ] Test still passes

---

### I8: Share `MockTransport` across crates

| Field | Value |
|-------|-------|
| Priority | P2 |
| File | `crates/clawft-services/src/mcp/transport.rs` |
| Lines | 218-265 |
| Risk | Low (feature flag addition) |

#### Current Code

```rust
// crates/clawft-services/src/mcp/transport.rs:218-265
/// A mock transport for testing.
///
/// Allows pre-programming responses that will be returned in order.
/// Also records all sent notifications for verification.
#[cfg(test)]
pub struct MockTransport {
    responses: Arc<Mutex<Vec<JsonRpcResponse>>>,
    requests: Arc<Mutex<Vec<JsonRpcRequest>>>,
    notifications: Arc<Mutex<Vec<JsonRpcNotification>>>,
}

#[cfg(test)]
impl MockTransport {
    pub fn new(responses: Vec<JsonRpcResponse>) -> Self { ... }
    pub async fn requests(&self) -> Vec<JsonRpcRequest> { ... }
    pub async fn notifications(&self) -> Vec<JsonRpcNotification> { ... }
}

#[cfg(test)]
#[async_trait]
impl McpTransport for MockTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> { ... }
    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> { ... }
}
```

**Bug**: `#[cfg(test)]` makes `MockTransport` only available within the `clawft-services` crate's own tests. Downstream crates that need to test MCP transport behavior must duplicate the mock.

#### Fix

1. Add a `test-utils` feature to `crates/clawft-services/Cargo.toml`:

```toml
[features]
default = []
test-utils = []
```

2. Change `#[cfg(test)]` to `#[cfg(any(test, feature = "test-utils"))]` on all three items (struct, impl block, trait impl):

```rust
#[cfg(any(test, feature = "test-utils"))]
pub struct MockTransport { ... }

#[cfg(any(test, feature = "test-utils"))]
impl MockTransport { ... }

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl McpTransport for MockTransport { ... }
```

3. Downstream crates can add the dependency:

```toml
[dev-dependencies]
clawft-services = { path = "../clawft-services", features = ["test-utils"] }
```

#### Tests

1. `cargo test -p clawft-services` still passes (existing tests use `#[cfg(test)]` which is captured by `any(test, ...)`).
2. Create a smoke test in a downstream crate that imports `MockTransport` via the `test-utils` feature.
3. `cargo build` without `test-utils` feature does NOT compile `MockTransport` into the binary.

#### Acceptance Criteria

- [ ] `test-utils` feature added to `clawft-services`
- [ ] `MockTransport` available when either `test` or `test-utils` is active
- [ ] No `MockTransport` in release binary
- [ ] Downstream crates can import via `features = ["test-utils"]`

---

## Exit Criteria

- [ ] All 8 items (I1-I8) resolved or documented as deferred
- [ ] Zero clippy warnings across affected crates
- [ ] All existing tests pass (with updated expectations where documented)
- [ ] No new `#[allow(dead_code)]` attributes added
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes for all affected crates:
  - `clawft-types`
  - `clawft-llm`
  - `clawft-cli`
  - `clawft-platform`
  - `clawft-core`
  - `clawft-services`

## File Index

| Item | Primary File | Line Range |
|------|-------------|-----------|
| I1 | `crates/clawft-types/src/delegation.rs` | 87-99, 183-196 |
| I2 | `crates/clawft-types/src/config.rs` | 986-1015 |
| I3 | `crates/clawft-llm/src/types.rs` | 10-27 |
| I4 | `crates/clawft-cli/src/commands/cron.rs` | 76-85 |
| I5 | `crates/clawft-platform/src/config_loader.rs` | 123-132, 167-170 |
| I6a | `crates/clawft-core/src/pipeline/rate_limiter.rs` | 282-286 |
| I6b | `crates/clawft-channels/src/discord/events.rs` | 90-100 |
| I6c | `crates/clawft-cli/src/interactive/` | mod.rs, builtins.rs, registry.rs |
| I6d | `crates/clawft-cli/src/commands/agent.rs` | 57-67, 130-143 |
| I6d | `crates/clawft-cli/src/commands/gateway.rs` | 62-64, 156-167 |
| I7 | `crates/clawft-core/src/pipeline/transport.rs` | 1232-1266 |
| I8 | `crates/clawft-services/src/mcp/transport.rs` | 218-265 |
