# Phase M-Foundation: Claude Flow Integration -- FlowDelegator, Runtime Detection, Feature Enable

> **Element:** 09 -- Multi-Agent Routing & Claude Flow Integration
> **Phase:** M-Foundation (M1, M2, M3)
> **Timeline:** Week 3-5
> **Priority:** P1
> **Crates:** `clawft-services`, `clawft-tools`, `clawft-types`, `clawft-cli`
> **Dependencies IN:** 03/B5 (shared tool registry), 05/D9 (MCP transport)
> **Blocks:** M4 (dynamic MCP discovery), M5 (bidirectional MCP bridge), L1-L3 (agent routing/swarming)
> **Status:** Planning
> **Orchestrator Ref:** `09-multi-agent-routing/00-orchestrator.md` Sections 4, 9
> **Dev Assignment Ref:** `02-improvements-overview/dev-assignment-09-multi-agent-routing.md` Unit 1

---

## 1. Overview

This phase establishes the Claude Flow foundation by delivering three milestones:

1. **M1 -- FlowDelegator**: A new struct that spawns the `claude` CLI as a subprocess for task delegation, plus extending `DelegationError` with three new error variants (`Timeout`, `Cancelled`, `FallbackExhausted`).

2. **M2 -- Runtime Detection**: Replace the hardcoded `let flow_available = false` at `delegate_tool.rs:105` with a cached runtime check that probes for the `claude` binary on `PATH`.

3. **M3 -- Feature Enable**: Add `"delegate"` to the default feature set in `clawft-cli/Cargo.toml` and change the `DelegationConfig::claude_enabled` default from `false` to `true` (gracefully degrades when no API key is present).

After this phase, the delegation subsystem will have three operational targets:
- **Local**: In-process tool execution (existing, always available)
- **Claude**: Anthropic Messages API delegation (existing `ClaudeDelegator`, now enabled by default)
- **Flow**: Claude Code CLI subprocess delegation (new `FlowDelegator`, available when `claude` binary is on PATH)

The fallback chain `Flow -> Claude -> Local` will function end-to-end, with the `DelegationEngine` automatically downgrading when higher-tier targets are unavailable.

---

## 2. Specification

### 2.1 M1: FlowDelegator Struct

**New file:** `crates/clawft-services/src/delegation/flow.rs`

The `FlowDelegator` spawns the `claude` CLI binary as a child process using `tokio::process::Command`. It passes tasks in non-interactive mode (`--print` flag) and parses the stdout response.

#### Core Requirements

| Requirement | Detail |
|-------------|--------|
| Binary discovery | Use `which::which("claude")` to locate the binary; return `None` from `new()` if not found |
| Subprocess mode | `claude --print "<task>"` for single-shot non-interactive execution |
| JSON mode (alternate) | `claude --json --print "<task>"` when structured output is needed |
| Environment isolation | Child process receives **only** `PATH`, `HOME`, `ANTHROPIC_API_KEY` -- not the full parent environment |
| Timeout enforcement | `tokio::time::timeout` wrapping `child.wait_with_output()`, followed by `child.kill()` on expiry |
| Delegation depth | Thread a `depth` counter; reject if `depth >= max_depth` (default: 3) |
| Agent ID context | Pass `agent_id` via `CLAWFT_AGENT_ID` env var so MCP callbacks can route back to the originating agent |
| Logging | `tracing::debug` for subprocess spawn; `tracing::warn` for failures; never log credentials |

#### Struct Definition

```rust
// crates/clawft-services/src/delegation/flow.rs

use std::path::PathBuf;
use std::time::Duration;

use clawft_types::delegation::DelegationConfig;

/// Delegator that spawns the Claude Code CLI as a subprocess.
///
/// Created via `FlowDelegator::new()`, which returns `None` if the
/// `claude` binary is not found on PATH.
pub struct FlowDelegator {
    /// Absolute path to the `claude` binary (resolved at construction time).
    claude_binary: PathBuf,
    /// Maximum wall-clock time for a single delegation call.
    timeout: Duration,
    /// Maximum delegation depth to prevent recursive loops.
    max_depth: u32,
}
```

#### Constructor

```rust
impl FlowDelegator {
    /// Create from config. Returns None if `claude` binary not found on PATH.
    ///
    /// The binary path is resolved once at construction time and cached.
    /// Timeout defaults to 120 seconds if not specified in config.
    pub fn new(config: &DelegationConfig) -> Option<Self> {
        let claude_binary = which::which("claude").ok()?;
        Some(Self {
            claude_binary,
            timeout: Duration::from_secs(120),
            max_depth: 3,
        })
    }
}
```

#### Delegate Method

```rust
impl FlowDelegator {
    /// Delegate a task via Claude Code CLI subprocess.
    ///
    /// # Arguments
    /// * `task` - The task description to delegate.
    /// * `agent_id` - The originating agent ID for MCP callback routing.
    /// * `depth` - Current delegation depth (incremented per delegation hop).
    ///
    /// # Errors
    /// * `DelegationError::Timeout` if the subprocess exceeds the configured timeout.
    /// * `DelegationError::Cancelled` if the subprocess is killed externally.
    /// * `DelegationError::ToolExecFailed` if the subprocess exits with non-zero status.
    /// * `DelegationError::InvalidResponse` if stdout cannot be parsed.
    pub async fn delegate(
        &self,
        task: &str,
        agent_id: &str,
        depth: u32,
    ) -> Result<String, DelegationError> {
        if depth >= self.max_depth {
            return Err(DelegationError::FallbackExhausted {
                attempts: vec![(DelegationTarget::Flow,
                    format!("delegation depth {} exceeds max {}", depth, self.max_depth))],
            });
        }
        // ... subprocess spawn logic (see Section 3 pseudocode)
    }
}
```

### 2.2 M1: Extended DelegationError

**Modified file:** `crates/clawft-services/src/delegation/claude.rs` (lines 29-50)

Add three new variants to the existing `DelegationError` enum. The existing five variants (`Http`, `Api`, `InvalidResponse`, `MaxTurnsExceeded`, `ToolExecFailed`) remain unchanged.

#### Current State (lines 29-50)

```rust
#[derive(Debug, thiserror::Error)]
pub enum DelegationError {
    #[error("http error: {0}")]
    Http(String),
    #[error("api error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("max turns ({0}) exceeded")]
    MaxTurnsExceeded(u32),
    #[error("tool execution failed: {0}")]
    ToolExecFailed(String),
}
```

#### New Variants to Add

```rust
    /// The delegation exceeded the configured timeout.
    #[error("delegation timed out after {elapsed:?}")]
    Timeout { elapsed: std::time::Duration },

    /// The delegation was cancelled (user abort, agent shutdown).
    #[error("delegation cancelled")]
    Cancelled,

    /// All fallback targets exhausted (Flow -> Claude -> Local).
    #[error("all delegation targets exhausted")]
    FallbackExhausted {
        attempts: Vec<(clawft_types::delegation::DelegationTarget, String)>,
    },
```

**Import requirement:** `DelegationError` now depends on `clawft_types::delegation::DelegationTarget`. Since `clawft-services` already depends on `clawft-types` (see `crates/clawft-services/Cargo.toml:19`), no new dependency is needed.

**Derive requirement:** `DelegationTarget` already derives `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize` (see `crates/clawft-types/src/delegation.rs:88`). For `DelegationError` to use it in a `Vec<(DelegationTarget, String)>`, no additional derives are needed since `thiserror::Error` handles `Debug` and `Display`.

### 2.3 M2: Runtime Detection

**Modified file:** `crates/clawft-tools/src/delegate_tool.rs`

#### Current State (lines 34-41, 95-106)

```rust
// Lines 34-41: struct fields
pub struct DelegateTaskTool {
    delegator: Arc<ClaudeDelegator>,
    engine: Arc<DelegationEngine>,
    tool_schemas: Vec<Value>,
    registry: Arc<ToolRegistry>,
}

// Lines 104-106: hardcoded false
let claude_available = true; // We have a delegator.
let flow_available = false; // Flow not wired yet.
let decision = self.engine.decide(task, claude_available, flow_available);
```

#### Changes Required

1. **Add `flow_delegator` field** to `DelegateTaskTool`:

```rust
pub struct DelegateTaskTool {
    delegator: Arc<ClaudeDelegator>,
    engine: Arc<DelegationEngine>,
    tool_schemas: Vec<Value>,
    registry: Arc<ToolRegistry>,
    /// Optional Flow delegator, present when `claude` binary is on PATH.
    flow_delegator: Option<Arc<FlowDelegator>>,
    /// Cached detection result for flow availability.
    flow_available: std::sync::atomic::AtomicBool,
}
```

2. **Add `which` crate** to `clawft-tools/Cargo.toml` (optional, behind `delegate` feature).

3. **Replace hardcoded false** at line 105:

```rust
// BEFORE:
let flow_available = false; // Flow not wired yet.

// AFTER:
let flow_available = self.flow_delegator.is_some()
    && self.flow_available.load(std::sync::atomic::Ordering::Relaxed);
```

4. **Update constructor** (`DelegateTaskTool::new`) to accept the optional `FlowDelegator`:

```rust
pub fn new(
    delegator: Arc<ClaudeDelegator>,
    engine: Arc<DelegationEngine>,
    tool_schemas: Vec<Value>,
    registry: Arc<ToolRegistry>,
    flow_delegator: Option<Arc<FlowDelegator>>,
) -> Self {
    let flow_available = std::sync::atomic::AtomicBool::new(flow_delegator.is_some());
    Self {
        delegator,
        engine,
        tool_schemas,
        registry,
        flow_delegator,
        flow_available,
    }
}
```

5. **Update execute method** to use `FlowDelegator` when the engine decides `Flow`:

```rust
// After the engine decides Flow:
if decision == DelegationTarget::Flow {
    if let Some(ref flow) = self.flow_delegator {
        match flow.delegate(task, /* agent_id */ "default", /* depth */ 0).await {
            Ok(response) => return Ok(json!({
                "status": "delegated_flow",
                "response": response,
                "task": task,
            })),
            Err(e) => {
                // Fall through to Claude API delegation
                tracing::warn!(error = %e, "Flow delegation failed, falling back to Claude API");
            }
        }
    }
}
```

### 2.4 M3: Enable delegate Feature by Default

#### 2.4.1 Cargo.toml Changes

**Modified file:** `crates/clawft-cli/Cargo.toml` (line 18)

```toml
# BEFORE (line 18):
default = ["channels", "services"]

# AFTER:
default = ["channels", "services", "delegate"]
```

This enables the full delegation chain by default. Users who do not have an Anthropic API key or the `claude` binary will gracefully degrade (both `ClaudeDelegator::new()` and `FlowDelegator::new()` return `None` when their prerequisites are missing).

#### 2.4.2 DelegationConfig Default Change

**Modified file:** `crates/clawft-types/src/delegation.rs` (line 62)

```rust
// BEFORE (line 62):
claude_enabled: false,

// AFTER:
claude_enabled: true,
```

**Important:** `claude_flow_enabled` stays `false` at line 66. This is intentional -- Claude Flow requires the `FlowDelegator` to be fully wired and tested before enabling by default. Users can opt-in via config.

#### 2.4.3 New Dependency: `which` crate

**Modified file:** `crates/clawft-services/Cargo.toml`

Add `which` as an optional dependency behind the `delegate` feature:

```toml
[features]
default = []
delegate = ["regex", "which"]

[dependencies]
# ... existing deps ...
which = { version = "7", optional = true }
```

**Modified file:** `crates/clawft-tools/Cargo.toml`

No change needed -- `clawft-tools` delegates to `clawft-services/delegate` which already pulls in what it needs.

---

## 3. Pseudocode

### 3.1 M1: FlowDelegator Implementation

```
FILE: crates/clawft-services/src/delegation/flow.rs

STRUCT FlowDelegator:
    claude_binary: PathBuf
    timeout: Duration
    max_depth: u32

FUNCTION FlowDelegator::new(config: &DelegationConfig) -> Option<Self>:
    // Step 1: Locate claude binary on PATH
    binary_path = which::which("claude")
    IF binary_path is Err:
        RETURN None

    // Step 2: Configure timeout (default 120s)
    timeout = Duration::from_secs(120)

    // Step 3: Configure max depth (default 3)
    max_depth = 3

    RETURN Some(FlowDelegator { binary_path, timeout, max_depth })

FUNCTION FlowDelegator::delegate(task, agent_id, depth) -> Result<String, DelegationError>:
    // Step 1: Check depth limit
    IF depth >= self.max_depth:
        RETURN Err(FallbackExhausted with depth exceeded message)

    // Step 2: Construct minimal child environment
    env = HashMap::new()
    env.insert("PATH", std::env::var("PATH").unwrap_or_default())
    env.insert("HOME", std::env::var("HOME").unwrap_or_default())
    IF let Ok(key) = std::env::var("ANTHROPIC_API_KEY"):
        env.insert("ANTHROPIC_API_KEY", key)
    env.insert("CLAWFT_AGENT_ID", agent_id)
    env.insert("CLAWFT_DELEGATION_DEPTH", (depth + 1).to_string())

    // Step 3: Build subprocess command
    cmd = tokio::process::Command::new(&self.claude_binary)
    cmd.arg("--print")
    cmd.arg(task)
    cmd.env_clear()           // CRITICAL: clear parent environment
    cmd.envs(env)             // apply minimal environment
    cmd.stdout(Stdio::piped())
    cmd.stderr(Stdio::piped())

    // Step 4: Spawn child process
    child = cmd.spawn()
    IF spawn fails:
        RETURN Err(ToolExecFailed("failed to spawn claude process"))

    // Step 5: Wait with timeout
    result = tokio::time::timeout(self.timeout, child.wait_with_output()).await
    MATCH result:
        Err(_timeout):
            // Timeout elapsed -- kill the child
            child.kill().await  // best-effort
            RETURN Err(Timeout { elapsed: self.timeout })

        Ok(Err(io_error)):
            RETURN Err(ToolExecFailed(io_error.to_string()))

        Ok(Ok(output)):
            // Step 6: Check exit status
            IF !output.status.success():
                stderr_text = String::from_utf8_lossy(&output.stderr)
                RETURN Err(ToolExecFailed(format!(
                    "claude exited with {}: {}", output.status, stderr_text
                )))

            // Step 7: Parse stdout
            stdout_text = String::from_utf8(output.stdout)
            IF stdout_text is Err:
                RETURN Err(InvalidResponse("non-UTF-8 output from claude"))

            response = stdout_text.unwrap().trim().to_string()
            IF response.is_empty():
                RETURN Err(InvalidResponse("empty output from claude"))

            RETURN Ok(response)
```

### 3.2 M1: DelegationError Extension

```
FILE: crates/clawft-services/src/delegation/claude.rs

MODIFY DelegationError enum (after line 49, before closing brace at line 50):

    ADD variant:
        Timeout { elapsed: Duration }
        Display: "delegation timed out after {elapsed:?}"

    ADD variant:
        Cancelled
        Display: "delegation cancelled"

    ADD variant:
        FallbackExhausted { attempts: Vec<(DelegationTarget, String)> }
        Display: "all delegation targets exhausted"

ADD import at top of file:
    use std::time::Duration;
    use clawft_types::delegation::DelegationTarget;

MODIFY existing tests:
    ADD error_display tests for the 3 new variants
```

### 3.3 M1: Module Registration

```
FILE: crates/clawft-services/src/delegation/mod.rs

ADD after line 9 (after `pub mod schema;`):
    pub mod flow;

This exposes FlowDelegator as:
    clawft_services::delegation::flow::FlowDelegator
```

### 3.4 M2: Runtime Detection Wiring

```
FILE: crates/clawft-tools/src/delegate_tool.rs

STEP 1: Add import
    use clawft_services::delegation::flow::FlowDelegator;
    use std::sync::atomic::{AtomicBool, Ordering};

STEP 2: Modify struct (lines 34-41)
    ADD field: flow_delegator: Option<Arc<FlowDelegator>>
    ADD field: flow_available: AtomicBool

STEP 3: Modify constructor (lines 52-64)
    ADD parameter: flow_delegator: Option<Arc<FlowDelegator>>
    SET flow_available = AtomicBool::new(flow_delegator.is_some())
    STORE both new fields in Self

STEP 4: Modify execute() (lines 104-106)
    REPLACE:
        let flow_available = false; // Flow not wired yet.
    WITH:
        let flow_available = self.flow_delegator.is_some()
            && self.flow_available.load(Ordering::Relaxed);

STEP 5: Add Flow delegation path in execute() (after line 115)
    IF decision == DelegationTarget::Flow:
        IF let Some(ref flow) = self.flow_delegator:
            MATCH flow.delegate(task, "default", 0).await:
                Ok(response) => RETURN Ok(json status: "delegated_flow")
                Err(e) => LOG warn, fall through to Claude API path
```

### 3.5 M3: Feature and Config Changes

```
STEP 1: crates/clawft-cli/Cargo.toml line 18
    CHANGE: default = ["channels", "services"]
    TO:     default = ["channels", "services", "delegate"]

STEP 2: crates/clawft-types/src/delegation.rs line 62
    CHANGE: claude_enabled: false,
    TO:     claude_enabled: true,

STEP 3: crates/clawft-services/Cargo.toml line 15
    CHANGE: delegate = ["regex"]
    TO:     delegate = ["regex", "which"]

STEP 4: crates/clawft-services/Cargo.toml dependencies
    ADD: which = { version = "7", optional = true }
```

---

## 4. Architecture

### 4.1 File Map

| File | Action | Milestone | Description |
|------|--------|-----------|-------------|
| `crates/clawft-services/src/delegation/flow.rs` | NEW | M1 | FlowDelegator struct and delegate method |
| `crates/clawft-services/src/delegation/claude.rs` | MODIFY | M1 | Add 3 new DelegationError variants (lines 29-50) |
| `crates/clawft-services/src/delegation/mod.rs` | MODIFY | M1 | Add `pub mod flow;` after line 9 |
| `crates/clawft-services/Cargo.toml` | MODIFY | M1/M3 | Add `which` dep, update delegate feature |
| `crates/clawft-tools/src/delegate_tool.rs` | MODIFY | M2 | Add flow_delegator field, wire runtime detection |
| `crates/clawft-types/src/delegation.rs` | MODIFY | M3 | Change claude_enabled default to true |
| `crates/clawft-cli/Cargo.toml` | MODIFY | M3 | Add "delegate" to default features |

### 4.2 Dependency Graph

```
clawft-cli (binary)
    |
    +-- [default: channels, services, delegate]  <-- M3: add "delegate"
    |
    +-- clawft-tools [delegate]
    |       |
    |       +-- DelegateTaskTool
    |       |       |
    |       |       +-- ClaudeDelegator (Arc)     <-- existing
    |       |       +-- FlowDelegator (Option<Arc>) <-- M2: new field
    |       |       +-- DelegationEngine (Arc)    <-- existing
    |       |       +-- ToolRegistry (Arc)        <-- existing
    |       |
    |       +-- clawft-services [delegate]
    |               |
    |               +-- delegation/claude.rs      <-- M1: extend DelegationError
    |               +-- delegation/flow.rs        <-- M1: NEW FlowDelegator
    |               +-- delegation/mod.rs         <-- M1: add pub mod flow
    |               |
    |               +-- [delegate feature = regex + which]  <-- M3: add which
    |
    +-- clawft-types
            |
            +-- delegation.rs
                    |
                    +-- DelegationConfig { claude_enabled: true }  <-- M3
                    +-- DelegationTarget { Local, Claude, Flow, Auto }
```

### 4.3 Data Flow: Task Delegation Decision

```
                    Inbound Task
                         |
                         v
              +-------------------+
              | DelegateTaskTool  |
              | .execute(args)    |
              +-------------------+
                         |
          +--------------+--------------+
          |                             |
    claude_available            flow_available
    = self.delegator            = self.flow_delegator.is_some()
      exists (always            && flow_available.load()
      true if construct-        (M2: runtime detection)
      ed with delegator)
          |                             |
          +-------- both flags ---------+
                         |
                         v
              +-------------------+
              | DelegationEngine  |
              | .decide(task,     |
              |   claude, flow)   |
              +-------------------+
                         |
              +----------+----------+
              |          |          |
              v          v          v
           Local      Claude      Flow
              |          |          |
              v          v          v
         Return       ClaudeDele   FlowDelegator
         "handle      gator        .delegate()
         locally"     .delegate()       |
                         |              v
                         v         claude --print
                    Anthropic      subprocess
                    Messages API        |
                         |              v
                         v         stdout parsed
                    JSON response       |
                         |              v
                         +----+---------+
                              |
                              v
                    Return delegated response
                    (with fallback on failure)
```

### 4.4 Fallback Chain

```
Decision = Flow
    |
    +-- FlowDelegator exists?
    |       |
    |       +-- YES: delegate via subprocess
    |       |       |
    |       |       +-- Success --> return "delegated_flow"
    |       |       +-- Failure --> log warn, fall through
    |       |
    |       +-- NO: fall through
    |
    +-- Fall through to Claude
    |       |
    |       +-- ClaudeDelegator.delegate()
    |       |       |
    |       |       +-- Success --> return "delegated"
    |       |       +-- Failure --> return ToolError
    |
    (If decision was Local from the start, skip both)
```

### 4.5 Subprocess Environment Isolation

```
Parent Process (weft)
    |
    | Full environment:
    |   PATH, HOME, USER, SHELL, TERM, LANG,
    |   ANTHROPIC_API_KEY, OPENAI_API_KEY,
    |   AWS_SECRET_ACCESS_KEY, DATABASE_URL,
    |   XDG_*, DISPLAY, WAYLAND_*, ...
    |
    +-- env_clear()  <-- CRITICAL: wipe all
    |
    +-- Construct minimal env:
    |       PATH              (from parent)
    |       HOME              (from parent)
    |       ANTHROPIC_API_KEY (from parent, if set)
    |       CLAWFT_AGENT_ID   (set by FlowDelegator)
    |       CLAWFT_DELEGATION_DEPTH  (incremented)
    |
    +-- Spawn child: claude --print "<task>"
            |
            Child process sees ONLY the 5 vars above.
            No DATABASE_URL, no AWS keys, no OPENAI_API_KEY.
```

---

## 5. Refinement

### 5.1 Edge Cases

#### 5.1.1 `claude` Binary Not on PATH

- `FlowDelegator::new()` returns `None`.
- `DelegateTaskTool.flow_delegator` is `None`.
- `flow_available` evaluates to `false`.
- `DelegationEngine.decide()` never returns `Flow` when `flow_available` is `false` (see `delegation/mod.rs:197-203`).
- The existing fallback logic in `resolve_availability()` handles this: `Flow` target with `flow_available=false` resolves to `Claude` (if available) or `Local`.
- No error surfaced to the user -- graceful degradation.

#### 5.1.2 `claude` Binary Exists but API Key Missing

- `FlowDelegator::new()` succeeds (binary found).
- Subprocess spawns but `claude` CLI itself fails due to missing API key.
- Child process exits with non-zero status.
- `FlowDelegator::delegate()` returns `Err(ToolExecFailed(...))`.
- `DelegateTaskTool.execute()` catches the error, logs a warning, and falls through to `ClaudeDelegator`.
- If `ClaudeDelegator` also fails (same missing key), the error propagates as `ToolError`.

#### 5.1.3 Subprocess Timeout

- `tokio::time::timeout` fires after the configured duration (default: 120s).
- The child process is killed via `child.kill().await`.
- `Err(DelegationError::Timeout { elapsed })` is returned.
- The caller decides whether to retry or fall back.

#### 5.1.4 Recursive Delegation

A recursive delegation loop can occur if:
1. User sends task to clawft.
2. Clawft delegates to Claude Code via `FlowDelegator`.
3. Claude Code has clawft registered as an MCP server.
4. Claude Code calls back into clawft's `delegate_task` tool.
5. Clawft delegates again to Claude Code.
6. Infinite loop.

**Mitigation:** The `depth` parameter is threaded through via `CLAWFT_DELEGATION_DEPTH` environment variable. Each hop increments the depth. When `depth >= max_depth` (default: 3), the delegation is rejected with `FallbackExhausted`.

#### 5.1.5 `claude_enabled: true` but No API Key

- `ClaudeDelegator::new()` returns `None` when the API key is empty (see `claude.rs:80-83`).
- Callers that construct `DelegateTaskTool` will have `delegator` as `None`.
- The `DelegateTaskTool` struct currently requires `Arc<ClaudeDelegator>` (not `Option`). This must be adjusted:
  - Either make `delegator` an `Option<Arc<ClaudeDelegator>>` and handle the `None` case.
  - Or keep the current contract where `DelegateTaskTool` is only constructed when a delegator is available, and the `claude_enabled: true` default simply means the config *allows* Claude delegation when the API key is present.
- **Recommended approach:** Keep the existing construction pattern. The `claude_enabled: true` default means "don't require explicit opt-in in config." The actual tool registration code should check `ClaudeDelegator::new()` and skip registration if `None`. This matches the existing pattern at `delegate_tool.rs` where the tool is only registered when a `ClaudeDelegator` is successfully constructed.

#### 5.1.6 Platform Differences

- `which::which()` works on Linux, macOS, and Windows.
- `claude` CLI may be installed as `claude.exe` on Windows -- `which` handles this.
- `env_clear()` on Windows clears `SYSTEMROOT`, `COMSPEC`, etc. The minimal environment must include `SYSTEMROOT` on Windows for subprocess health. Add a platform-conditional env var:

```rust
#[cfg(windows)]
if let Ok(sr) = std::env::var("SYSTEMROOT") {
    env.insert("SYSTEMROOT".to_string(), sr);
}
```

### 5.2 Error Handling Strategy

| Error Source | DelegationError Variant | Fallback Action |
|-------------|------------------------|-----------------|
| Binary not found (construction) | N/A (returns `None`) | Skip FlowDelegator entirely |
| Spawn failure | `ToolExecFailed` | Fall through to Claude API |
| Non-zero exit | `ToolExecFailed` | Fall through to Claude API |
| Timeout | `Timeout` | Kill child, fall through to Claude API |
| Non-UTF-8 output | `InvalidResponse` | Fall through to Claude API |
| Empty output | `InvalidResponse` | Fall through to Claude API |
| Depth exceeded | `FallbackExhausted` | Return error to caller (no further fallback) |
| User cancellation | `Cancelled` | Propagate immediately |

### 5.3 Security Considerations

#### 5.3.1 Environment Variable Leakage (Risk Score: 8)

The `env_clear()` call is **critical**. Without it, the child process inherits all parent environment variables, including `DATABASE_URL`, `AWS_SECRET_ACCESS_KEY`, `OPENAI_API_KEY`, and any other secrets in the parent environment.

**Implementation rule:** Always use `env_clear()` followed by explicit `envs()` with the minimal set. Never use `env()` (which adds to existing environment without clearing).

**Verification:** Unit test must assert that the constructed `Command` has `env_clear` set. Since `tokio::process::Command` does not expose its environment configuration, the test should verify the child process output (e.g., spawn a test script that prints all env vars and verify only the expected ones are present).

#### 5.3.2 No Credentials in Error Messages

All error messages and log lines must be reviewed to ensure they do not contain:
- API keys
- Tokens
- Passwords
- File contents that might contain secrets

The `stderr` output from the subprocess is included in `ToolExecFailed` error messages. This is acceptable because the subprocess stderr is generated by the `claude` CLI itself, which already sanitizes its output. However, the stderr should be truncated to a maximum of 1024 bytes to prevent excessive error messages.

```rust
let stderr_text = String::from_utf8_lossy(&output.stderr);
let truncated = if stderr_text.len() > 1024 {
    format!("{}... (truncated)", &stderr_text[..1024])
} else {
    stderr_text.to_string()
};
```

#### 5.3.3 Delegation Depth Limit

The depth counter is threaded via `CLAWFT_DELEGATION_DEPTH` environment variable. When the `DelegateTaskTool` starts an execution, it should read this variable to determine the current depth:

```rust
let current_depth: u32 = std::env::var("CLAWFT_DELEGATION_DEPTH")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(0);
```

This allows the depth to survive across process boundaries (clawft -> Claude Code -> clawft callback).

### 5.4 Backward Compatibility

#### 5.4.1 Existing Test Impact

Enabling `delegate` by default means the existing test suite must compile with the delegation modules included. Key risks:

- Tests in `clawft-services/src/delegation/claude.rs` (lines 284-570) already pass and should not be affected.
- Tests in `clawft-services/src/delegation/mod.rs` (lines 218-411) already pass and should not be affected.
- The new `DelegationError` variants must have corresponding `error_display` tests.
- The `DelegationConfig` default test at `clawft-types/src/delegation.rs:106-115` must be updated to expect `claude_enabled: true`.

#### 5.4.2 Existing Config Files

Users with existing `clawft.toml` files that explicitly set `claude_enabled: false` will not be affected by the default change. Serde deserialization of explicit values takes priority over defaults.

Users with no `clawft.toml` or no delegation section will get `claude_enabled: true` by default. Since `ClaudeDelegator::new()` returns `None` without an API key, this change has no effect for users without credentials.

---

## 6. Completion

### 6.1 Acceptance Criteria Checklist

#### M1: FlowDelegator

- [ ] `FlowDelegator` struct defined in `crates/clawft-services/src/delegation/flow.rs`
- [ ] `FlowDelegator::new()` returns `None` if `claude` binary not on PATH
- [ ] `FlowDelegator::new()` returns `Some` if `claude` binary is found
- [ ] `FlowDelegator::delegate()` spawns `claude --print` subprocess
- [ ] Child process environment is cleared and rebuilt with only `PATH`, `HOME`, `ANTHROPIC_API_KEY`, `CLAWFT_AGENT_ID`, `CLAWFT_DELEGATION_DEPTH`
- [ ] Timeout enforcement: subprocess killed after configured timeout
- [ ] Delegation depth limit enforced (default: 3)
- [ ] `DelegationError` extended with `Timeout`, `Cancelled`, `FallbackExhausted` variants
- [ ] All existing `DelegationError` tests still pass
- [ ] New `DelegationError` display tests for all 3 new variants
- [ ] `pub mod flow;` added to `delegation/mod.rs`

#### M2: Runtime Detection

- [ ] `DelegateTaskTool` has `flow_delegator: Option<Arc<FlowDelegator>>` field
- [ ] `DelegateTaskTool` has `flow_available: AtomicBool` field
- [ ] Hardcoded `let flow_available = false;` at `delegate_tool.rs:105` replaced
- [ ] `DelegateTaskTool::new()` constructor updated with `flow_delegator` parameter
- [ ] `execute()` method routes to `FlowDelegator` when decision is `Flow`
- [ ] Fallback from Flow to Claude API works when Flow delegation fails
- [ ] `flow_available` detection is cached (not re-probed per call)

#### M3: Feature Enable

- [ ] `"delegate"` added to `default` features in `crates/clawft-cli/Cargo.toml`
- [ ] `claude_enabled` default changed to `true` in `crates/clawft-types/src/delegation.rs`
- [ ] `claude_flow_enabled` default remains `false`
- [ ] `which` crate added as optional dependency to `crates/clawft-services/Cargo.toml`
- [ ] `delegate` feature updated to include `"which"` in `crates/clawft-services/Cargo.toml`
- [ ] `cargo build -p clawft-cli` compiles with delegate feature enabled
- [ ] `cargo build -p clawft-cli --no-default-features --features channels,services` compiles without delegate
- [ ] Existing `DelegationConfig` test updated for `claude_enabled: true` default
- [ ] All existing tests pass with `cargo test --workspace`

### 6.2 Test Plan

#### Unit Tests: FlowDelegator

| Test | Location | Description |
|------|----------|-------------|
| `new_returns_none_without_binary` | `flow.rs` | Mock or ensure `claude` not on PATH; assert `new()` returns `None` |
| `new_returns_some_with_binary` | `flow.rs` | Requires `claude` on PATH (skip if not available); assert `Some` |
| `depth_limit_exceeded` | `flow.rs` | Call `delegate()` with `depth = 3`, `max_depth = 3`; expect `FallbackExhausted` |
| `timeout_enforcement` | `flow.rs` | Spawn a subprocess that sleeps forever (mock); expect `Timeout` |
| `minimal_environment` | `flow.rs` | Spawn a test script that prints env vars; verify only expected vars present |
| `stderr_truncation` | `flow.rs` | Spawn process with long stderr; verify error message truncated to 1024 bytes |

#### Unit Tests: DelegationError Extension

| Test | Location | Description |
|------|----------|-------------|
| `error_display_timeout` | `claude.rs` | Verify `Timeout` display format |
| `error_display_cancelled` | `claude.rs` | Verify `Cancelled` display format |
| `error_display_fallback_exhausted` | `claude.rs` | Verify `FallbackExhausted` display format |

#### Unit Tests: DelegateTaskTool Changes

| Test | Location | Description |
|------|----------|-------------|
| `flow_available_when_delegator_present` | `delegate_tool.rs` | Construct with `Some(FlowDelegator)`; verify `flow_available` is true |
| `flow_unavailable_when_delegator_absent` | `delegate_tool.rs` | Construct with `None`; verify `flow_available` is false |

#### Unit Tests: Config Default Change

| Test | Location | Description |
|------|----------|-------------|
| `delegation_config_defaults_updated` | `delegation.rs` | Assert `claude_enabled` is `true`, `claude_flow_enabled` is `false` |

#### Integration Tests

| Test | Description |
|------|-------------|
| `fallback_flow_to_claude` | Configure both delegators; Flow fails -> Claude succeeds |
| `fallback_flow_to_local` | Flow and Claude both unavailable -> Local |
| `delegate_feature_compile` | `cargo build -p clawft-cli` succeeds with default features |
| `no_delegate_feature_compile` | `cargo build -p clawft-cli --no-default-features --features channels,services` succeeds |

### 6.3 Exit Criteria

This phase is complete when:

1. **All acceptance criteria checked** (Section 6.1)
2. **All unit tests pass:** `cargo test -p clawft-services --features delegate` and `cargo test -p clawft-tools --features delegate`
3. **Full workspace build:** `cargo build --workspace` succeeds
4. **Full workspace test:** `cargo test --workspace` succeeds
5. **Clippy clean:** `cargo clippy --workspace -- -D warnings` passes
6. **No credential leakage:** Manual review confirms no API keys in error messages or logs
7. **Backward compat verified:** Existing `clawft.toml` files with `claude_enabled: false` still disable delegation

### 6.4 Suggested Implementation Order

```
M1a: Extend DelegationError (claude.rs)
    |-- Add 3 new variants
    |-- Add display tests
    |-- No other file changes needed
    |
M1b: Create flow.rs module  [depends on M1a for error types]
    |-- FlowDelegator struct
    |-- new() with which::which
    |-- delegate() with subprocess spawn
    |-- Unit tests
    |
M1c: Register module (mod.rs)  [depends on M1b]
    |-- Add pub mod flow;
    |
M3a: Add which dependency (Cargo.toml)  [can parallel with M1b]
    |-- Update clawft-services/Cargo.toml
    |
M2: Wire runtime detection (delegate_tool.rs)  [depends on M1b + M1c]
    |-- Add fields to DelegateTaskTool
    |-- Update constructor
    |-- Replace hardcoded false
    |-- Add Flow delegation path
    |-- Update tests
    |
M3b: Enable feature + config defaults  [depends on M2]
    |-- Update clawft-cli/Cargo.toml default features
    |-- Update DelegationConfig default
    |-- Update existing tests for new default
    |
Final: Full workspace build + test validation
```

---

## 7. Cross-Element Dependency Contracts

### 7.1 Contract 3.4: Delegation <-> Routing

When agent routing (L1-L2, from Phase L-Routing) is implemented, any routed agent can delegate to Claude Code. The `FlowDelegator` includes `agent_id` in the MCP callback context via the `CLAWFT_AGENT_ID` environment variable. When Claude Code calls back into clawft's MCP server, the server extracts this agent ID and routes results back to the originating agent.

**This phase's responsibility:** Include `CLAWFT_AGENT_ID` in the subprocess environment.
**L-Routing's responsibility:** Read `CLAWFT_AGENT_ID` in the MCP server and route accordingly.

### 7.2 D9 (Element 05): MCP Transport Concurrency

The bidirectional MCP bridge (M5, Phase M-Advanced) requires concurrent MCP transport. This phase does not depend on D9, but the `FlowDelegator` design anticipates it by separating subprocess delegation (M1-M3) from MCP bridge delegation (M5).

### 7.3 Existing Delegation Tests

The following existing tests must continue to pass after all changes:

- `crates/clawft-services/src/delegation/claude.rs`: 11 tests (lines 284-570)
- `crates/clawft-services/src/delegation/mod.rs`: 10 tests (lines 218-411)
- `crates/clawft-tools/src/delegate_tool.rs`: 2 tests (lines 152-192)
- `crates/clawft-types/src/delegation.rs`: 6 tests (lines 101-202)

The test at `delegation.rs:106-115` (`delegation_config_defaults`) must be updated to expect `claude_enabled: true` instead of `false`.

---

## 8. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Environment variable leakage to child process | Medium | Critical | **8** | `env_clear()` + explicit minimal env construction. Unit test verifies env isolation. |
| Recursive delegation loop (clawft -> Claude Code -> clawft -> ...) | Low | Medium | **3** | `CLAWFT_DELEGATION_DEPTH` env var threaded across boundaries; max depth = 3. |
| `which` crate version incompatibility | Low | Low | **1** | Pin to `which = "7"`. No transitive dependency conflicts expected. |
| Subprocess resource exhaustion (zombie processes) | Low | Medium | **3** | `child.kill()` on timeout. `wait_with_output()` reaps the child. Tokio runtime handles cleanup. |
| `claude` binary version mismatch (missing `--print` flag) | Low | Medium | **3** | The `--print` flag has been stable since Claude Code v1.0. If missing, the subprocess fails fast with a clear error. |
| Breaking change in DelegateTaskTool constructor signature | Medium | Low | **2** | The added `flow_delegator` parameter changes the constructor. All call sites must be updated. Search for `DelegateTaskTool::new(` across the workspace. |
| Windows platform env isolation | Low | Medium | **3** | Include `SYSTEMROOT` in minimal env on Windows. Add `#[cfg(windows)]` conditional. |

---

## 9. References

- Orchestrator: `09-multi-agent-routing/00-orchestrator.md` Sections 4, 9, 10
- Dev Assignment: `02-improvements-overview/dev-assignment-09-multi-agent-routing.md` Unit 1
- Existing ClaudeDelegator: `crates/clawft-services/src/delegation/claude.rs`
- Existing DelegationEngine: `crates/clawft-services/src/delegation/mod.rs`
- Existing DelegateTaskTool: `crates/clawft-tools/src/delegate_tool.rs`
- Existing DelegationConfig: `crates/clawft-types/src/delegation.rs`
- Feature flags: `crates/clawft-cli/Cargo.toml`, `crates/clawft-services/Cargo.toml`, `crates/clawft-tools/Cargo.toml`
- `which` crate: https://crates.io/crates/which
