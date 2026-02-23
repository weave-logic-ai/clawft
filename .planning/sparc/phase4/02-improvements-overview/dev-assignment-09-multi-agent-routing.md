# Development Assignment: Element 09 -- Multi-Agent Routing & Claude Flow Integration

**Element**: 09
**Workstreams**: L (Multi-Agent Routing & Orchestration), M (Claude Flow / Claude Code Integration)
**Timeline**: Weeks 3-9
**Dependencies**: 03/B5 (shared tool registry), 04/C1 (plugin traits), 05/D6 (sender_id), 05/D9 (MCP transport)
**Blocks**: None directly
**Branch**: `sprint/phase-5` (streams 5H + 5I)

---

## Overview

Multi-agent routing (different users -> different agents), per-agent isolation, multi-agent swarming, and full Claude Flow / Claude Code integration with bidirectional MCP bridge. This element spans two workstreams: L (routing/orchestration) and M (Claude Flow/Claude Code integration).

---

## Unit 1: M1-M3 Claude Flow Foundation (Week 3-5, MVP)

### Objective

Extend the existing `ClaudeDelegator` to add `FlowDelegator` support, wire `flow_available` to runtime detection, and enable the `delegate` feature by default.

### Current State

**`ClaudeDelegator` exists** at `crates/clawft-services/src/delegation/claude.rs`. It sends multi-turn requests to the Anthropic Messages API with tool-use loops. Current code:

```rust
// crates/clawft-services/src/delegation/claude.rs:29-50
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

**`DelegationEngine` exists** at `crates/clawft-services/src/delegation/mod.rs`. It routes tasks via regex rules and complexity heuristics to `Local`, `Claude`, or `Flow` targets. The `Flow` path falls back to `Claude` or `Local` when unavailable.

**`flow_available` is hardcoded to `false`** at `crates/clawft-tools/src/delegate_tool.rs:105`:

```rust
// crates/clawft-tools/src/delegate_tool.rs:104-106
let claude_available = true; // We have a delegator.
let flow_available = false; // Flow not wired yet.
let decision = self.engine.decide(task, claude_available, flow_available);
```

**`delegate` feature is NOT in default features** for any crate:

```toml
# crates/clawft-cli/Cargo.toml:17-22
[features]
default = ["channels", "services"]
# delegate is NOT listed in default

# crates/clawft-services/Cargo.toml:13-16
[features]
default = []
delegate = ["regex"]

# crates/clawft-tools/Cargo.toml:13-17
[features]
default = ["native-exec"]
delegate = ["clawft-services/delegate"]
```

**`DelegationConfig` defaults to disabled** (`crates/clawft-types/src/delegation.rs:59-70`):

```rust
impl Default for DelegationConfig {
    fn default() -> Self {
        Self {
            claude_enabled: false,      // M3: change to true
            claude_model: default_delegation_model(),
            max_turns: default_max_turns(),
            max_tokens: default_max_tokens(),
            claude_flow_enabled: false,  // stays false until Flow is wired
            rules: Vec::new(),
            excluded_tools: Vec::new(),
        }
    }
}
```

**`DelegationTarget` types** (`crates/clawft-types/src/delegation.rs:88-99`):

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationTarget {
    Local,
    Claude,
    Flow,
    #[default]
    Auto,
}
```

### Deliverables

#### 1.1 M1: Create FlowDelegator

Create `crates/clawft-services/src/delegation/flow.rs`:

```rust
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, warn};

use clawft_types::delegation::DelegationConfig;
use super::claude::DelegationError;

/// Delegator that spawns Claude Code CLI as a subprocess.
pub struct FlowDelegator {
    claude_binary: String,
    timeout: Duration,
    max_depth: u32,
}

impl FlowDelegator {
    /// Create from config. Returns None if Claude binary not found on PATH.
    pub fn new(config: &DelegationConfig) -> Option<Self> { ... }

    /// Delegate a task via Claude Code CLI.
    ///
    /// Spawns `claude --print` (non-interactive mode) with the task as input.
    /// Includes agent_id in MCP callback context for response routing.
    pub async fn delegate(
        &self,
        task: &str,
        agent_id: &str,
    ) -> Result<String, DelegationError> { ... }
}
```

The `FlowDelegator` must:
- Spawn `claude` as subprocess via `tokio::process::Command`
- Pass tasks via `claude --print` (non-interactive) or `claude --json`
- Construct minimal child environment: only `PATH`, `HOME`, `ANTHROPIC_API_KEY` (security: no full parent env)
- Enforce timeout with `child.kill()` on expiry
- Include `agent_id` in MCP callback context for correct routing

#### 1.2 M1: Extended DelegationError

Add missing error variants per orchestrator Section 4:

```rust
// Extend existing DelegationError in crates/clawft-services/src/delegation/claude.rs
pub enum DelegationError {
    // ... existing variants ...

    /// The delegation exceeded the configured timeout.
    #[error("delegation timed out after {elapsed:?}")]
    Timeout { elapsed: Duration },

    /// The delegation was cancelled (user abort, agent shutdown).
    #[error("delegation cancelled")]
    Cancelled,

    /// All fallback targets exhausted (Flow -> Claude -> Local).
    #[error("all delegation targets exhausted")]
    FallbackExhausted {
        attempts: Vec<(DelegationTarget, String)>,
    },
}
```

Note: `SubprocessFailed` and `OutputParseFailed` from the orchestrator spec map to the existing `Http`/`Api`/`InvalidResponse` variants plus the new subprocess-specific errors from `FlowDelegator`.

#### 1.3 M2: Wire flow_available to Runtime Detection

Replace the hardcoded `false` in `crates/clawft-tools/src/delegate_tool.rs:105`:

```rust
// BEFORE (current code):
let flow_available = false; // Flow not wired yet.

// AFTER:
let flow_available = self.detect_flow_available();
```

Add detection method to `DelegateTaskTool`:

```rust
impl DelegateTaskTool {
    /// Detect whether Claude Code CLI is available.
    /// Checks:
    /// 1. DelegationConfig.claude_flow_enabled is true
    /// 2. `claude` binary is on PATH (cached, not re-probed per call)
    fn detect_flow_available(&self) -> bool {
        // Cache the result using OnceCell or AtomicBool
        // Use `which::which("claude")` or `Command::new("which").arg("claude")`
    }
}
```

Add a `flow_delegator: Option<Arc<FlowDelegator>>` field to `DelegateTaskTool` and update the constructor.

#### 1.4 M3: Enable delegate Feature by Default

In `crates/clawft-cli/Cargo.toml`:

```toml
[features]
default = ["channels", "services", "delegate"]  # add delegate
```

In `crates/clawft-types/src/delegation.rs`, change default:

```rust
claude_enabled: true,  // was false; gracefully degrades if no API key
```

`claude_flow_enabled` stays `false` by default until Flow is fully wired.

### Cross-Element Dependencies

- **Contract 3.4 (Delegation <-> Routing)**: Any agent can delegate to Claude Code. `FlowDelegator` includes `agent_id` in MCP callback context via `x-clawft-agent-id` header. MCP server extracts this header and routes results back to the originating agent.
- **D9 (Element 05)**: MCP transport concurrency needed for bidirectional bridge (M5).

### Security Criteria

- [x] `FlowDelegator` child process receives minimal environment (PATH, HOME, ANTHROPIC_API_KEY only)
- [x] Delegation depth limit enforced (default: 3, configurable)
- [x] Timeout enforcement with `child.kill()` on expiry
- [x] No credentials in error messages or logs

### Acceptance Criteria

- [x] `FlowDelegator` spawns `claude` CLI and returns results
- [x] `FlowDelegator::new()` returns `None` if `claude` not on PATH
- [x] `DelegationError` has all 5 required variants
- [x] `flow_available` detects `claude` binary at runtime (cached)
- [x] `delegate` feature is in `default` features for `clawft-cli`
- [x] `claude_enabled` defaults to `true`
- [x] Fallback chain works: Flow -> Claude -> Local
- [x] All existing delegation tests pass

### Test Requirements

- Unit test: `FlowDelegator::new()` returns None when binary not found
- Unit test: `DelegationError` display for all variants
- Integration test: delegate task to Claude CLI (mock subprocess)
- Test fallback: Flow unavailable -> falls back to Claude API
- Test depth limit enforcement
- Test timeout enforcement

---

## Unit 2: L1-L2 Agent Routing (Week 5-7)

### Objective

Implement agent routing table that maps channel/user combinations to specific agent instances, with per-agent workspace isolation.

### Current State

The `MessageBus` in `crates/clawft-core/src/bus.rs` routes all messages through a single inbound/outbound channel pair with no agent routing:

```rust
// crates/clawft-core/src/bus.rs:38-43
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Mutex<mpsc::UnboundedReceiver<InboundMessage>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::UnboundedReceiver<OutboundMessage>>,
}
```

`InboundMessage` already has `sender_id` and `channel` fields (`crates/clawft-types/src/event.rs:17-41`):

```rust
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub media: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Deliverables

#### 2.1 L1: Agent Routing Table

Create `crates/clawft-core/src/agent_routing.rs`:

```rust
use clawft_types::event::InboundMessage;

/// A single routing rule that maps channel + match criteria to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoute {
    pub channel: String,
    pub match_criteria: MatchCriteria,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCriteria {
    pub user_id: Option<String>,
    pub phone: Option<String>,
    pub chat_id: Option<String>,
}

pub struct AgentRouter {
    routes: Vec<AgentRoute>,
    catch_all: Option<String>,
}

impl AgentRouter {
    /// Route a message to the appropriate agent ID.
    /// First match wins.
    pub fn route(&self, msg: &InboundMessage) -> RoutingResult { ... }
}

pub enum RoutingResult {
    Agent(String),
    CatchAll(String),
    NoMatch,
}
```

Config format (from improvements.md L1):

```toml
[[agent_routes]]
channel = "telegram"
match = { user_id = "12345" }
agent = "work-agent"

[[agent_routes]]
channel = "whatsapp"
match = { phone = "+1..." }
agent = "personal-agent"
```

#### 2.2 L1: Routing Fallback Behavior

Per orchestrator Section 7:

- **No matching route + no catch-all**: Reject with "no agent configured" error, logged at `warn`
- **Matched agent workspace does not exist**: Auto-create from default template (calls `WorkspaceManager::ensure_agent_workspace` from Element 08/H1)
- **Anonymous messages** (no `sender_id`): Route to catch-all or dedicated "anonymous" agent with reduced permissions

#### 2.3 L2: Per-Agent Workspace and Session Isolation

Each routed agent gets:
- Dedicated workspace via `WorkspaceManager::ensure_agent_workspace(agent_id)` (from Element 08, Unit 1)
- Own session store under `~/.clawft/agents/<agentId>/sessions/`
- Own `SOUL.md`/`AGENTS.md`/`USER.md`
- Own skill overrides
- No cross-talk unless explicitly enabled via shared memory namespace

### Cross-Element Dependencies

- **Contract 3.3 (Workspace <-> Routing)**: L2 calls `WorkspaceManager::ensure_agent_workspace(agent_id)` from Element 08/H1 when routing to a new agent. H1 owns workspace CRUD; L2 is a consumer.
- **Contract 3.2 (MCP Client <-> Agent Routing)**: Per-agent MCP server configuration in agent workspace config overrides/extends global MCP config.
- **B5 (Element 03)**: Shared tool registry builder needed so each agent can have its own tool registry instance.
- **D6 (Element 05)**: `sender_id` threaded through pipeline for cost recording and routing.

### Security Criteria

- [x] Agents cannot access other agents' workspaces without explicit sharing
- [x] Anonymous agents have reduced permissions (no shell access, read-only file ops)
- [x] Routing table changes require restart (no runtime modification without auth)

### Acceptance Criteria

- [x] Routing table routes messages to correct agent based on channel + user match
- [x] First-match-wins semantics work correctly
- [x] No-match rejection works with clear error message
- [x] Auto-create workspace on first routing to new agent
- [x] Anonymous message routing to catch-all agent works
- [x] Per-agent session isolation (messages to agent A don't appear in agent B's sessions)
- [x] Configuration loaded from `clawft.toml` `[[agent_routes]]` section

### Test Requirements

- Route matching: multiple rules, verify first-match-wins
- Route fallback: no match -> rejection
- Route fallback: no match + catch-all -> catch-all agent
- Anonymous routing test
- Workspace auto-creation on first route

---

## Unit 3: L3 Multi-Agent Swarming (Week 7-8)

### Objective

Implement inter-agent communication via `AgentBus` with per-agent inboxes and coordinator pattern for subtask dispatch.

### Current State

The existing `MessageBus` (`crates/clawft-core/src/bus.rs`) handles channel-to-agent communication. There is no agent-to-agent communication layer.

### Deliverables

#### 3.1 InterAgentMessage Type

Per orchestrator Section 3:

```rust
// crates/clawft-types/src/agent_bus.rs (NEW)
use std::time::Duration;
use uuid::Uuid;
use serde_json::Value;

pub struct InterAgentMessage {
    pub id: Uuid,
    pub from_agent: String,
    pub to_agent: String,
    pub task: String,
    pub payload: Value,
    pub reply_to: Option<Uuid>,
    pub ttl: Duration,
}
```

#### 3.2 MessagePayload Enum

Per orchestrator Section 3 (UI forward-compat):

```rust
pub enum MessagePayload {
    Text(String),
    Structured(Value),
    Binary { mime_type: String, data: Vec<u8> },
}
```

This is a shared type change. Coordinate with Element 05 (D8 bounded bus or B1/B2 type unification).

#### 3.3 AgentBus

Create `crates/clawft-core/src/agent_bus.rs`:

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::InterAgentMessage;

pub struct AgentBus {
    /// Per-agent inboxes: agent_id -> sender
    inboxes: HashMap<String, mpsc::UnboundedSender<InterAgentMessage>>,
}

impl AgentBus {
    pub fn new() -> Self { ... }
    pub fn register_agent(&mut self, agent_id: &str) -> mpsc::UnboundedReceiver<InterAgentMessage> { ... }
    pub fn unregister_agent(&mut self, agent_id: &str) { ... }
    pub fn send(&self, msg: InterAgentMessage) -> Result<(), AgentBusError> { ... }
}
```

Key properties:
- **Per-agent inboxes**: Each agent has its own inbox. Messages delivered to `to_agent`'s inbox.
- **Agent-scoped delivery**: Agents can only read from their own inbox.
- **TTL enforcement**: Expired messages dropped and logged at `warn`.
- **Acknowledgment**: Recipient sends reply message with `reply_to` set.

#### 3.4 Coordinator Pattern

The `SwarmCoordinator` uses `AgentBus` for subtask dispatch:

```rust
pub struct SwarmCoordinator {
    agent_bus: Arc<AgentBus>,
    worker_agents: Vec<String>,
}

impl SwarmCoordinator {
    pub async fn dispatch_subtask(&self, task: &str, worker: &str) -> Result<Value> { ... }
    pub async fn collect_results(&self, timeout: Duration) -> Vec<(String, Value)> { ... }
}
```

### Cross-Element Dependencies

- **D8 (Element 05)**: Bounded bus channels -- `AgentBus` should use bounded channels to prevent memory exhaustion
- **L1-L2 (Unit 2)**: Agent routing creates agents that can communicate via `AgentBus`

### Security Criteria

- [x] Bus messages tagged with agent IDs
- [x] Agents cannot read other agents' messages (inbox scoping)
- [x] Bounded inbox size to prevent memory exhaustion

### Acceptance Criteria

- [x] `InterAgentMessage` type defined and serializable
- [x] `AgentBus` provides per-agent inboxes
- [x] Agent can send message to another agent and receive reply
- [x] TTL enforcement: expired messages dropped
- [x] Coordinator dispatches subtasks and collects results
- [x] `MessagePayload` enum supports Text, Structured, Binary variants

### Test Requirements

- Send/receive between two agents
- TTL expiry test
- Agent isolation test: agent A cannot read agent B's inbox
- Coordinator dispatch and collect pattern

---

## Unit 4: M4-M5 Dynamic MCP + Bridge (Week 6-8)

### Objective

Add dynamic MCP server management (`weft mcp add/list/remove`) and bidirectional MCP bridge between clawft and Claude Code.

### Current State

`McpClient` and `McpSession` exist in `crates/clawft-services/src/mcp/mod.rs`:

```rust
// crates/clawft-services/src/mcp/mod.rs:56-59
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    request_id: AtomicU64,
}
```

`McpSession` performs the initialize handshake and wraps `McpClient`. Supports `list_tools()` and `call_tool()`.

MCP server exists at `crates/clawft-services/src/mcp/server.rs` but there is no `client.rs`, `discovery.rs`, or `bridge.rs`.

MCP file ownership table per orchestrator Section 6:

| File | Owner | Purpose |
|------|-------|---------|
| `server.rs` | C6 | Existing MCP server |
| `client.rs` | F9 | NEW: MCP client connections |
| `discovery.rs` | M4 | NEW: Dynamic server management |
| `bridge.rs` | M5 | NEW: Bidirectional bridge |
| `ide.rs` | F8 | NEW: IDE-specific extensions |
| `transport.rs` | Shared | Existing (cross-stream review required) |
| `types.rs` | Shared | Existing (cross-stream review required) |
| `middleware.rs` | Shared | Existing (cross-stream review required) |

### Deliverables

#### 4.1 M4: Dynamic MCP Server Discovery

Create `crates/clawft-services/src/mcp/discovery.rs`:

```rust
pub struct McpServerManager {
    servers: HashMap<String, ManagedMcpServer>,
    tool_registry: Arc<ToolRegistry>,
}

pub struct ManagedMcpServer {
    name: String,
    session: McpSession,
    status: ServerStatus,
}

impl McpServerManager {
    pub async fn add_server(&mut self, name: &str, config: McpServerConfig) -> Result<()> { ... }
    pub async fn remove_server(&mut self, name: &str) -> Result<()> { ... }
    pub fn list_servers(&self) -> Vec<&ManagedMcpServer> { ... }
}
```

CLI commands:
```
weft mcp add <name> <command|url>
weft mcp list
weft mcp remove <name>
```

**Depends on F9a** (minimal MCP client from Element 07). M4 uses the single-server `McpClient::connect()` and `list_tools()` to register external server tools.

#### 4.2 M4: Hot-Reload Drain-and-Swap

Per orchestrator Section 5:

1. File watcher detects `[mcp_servers]` change in `clawft.toml` (debounce 500ms)
2. Diff old and new server lists
3. New servers: connect immediately, add tools as `mcp:<server-name>:<tool-name>`
4. Removed servers: mark "draining", complete in-flight calls (30s timeout), disconnect
5. Changed servers (same name, different config): remove + add

#### 4.3 M5: Bidirectional MCP Bridge

Create `crates/clawft-services/src/mcp/bridge.rs`:

**Outbound (clawft -> Claude Code)**: clawft already has `server.rs` for MCP server exposure. Document the `claude mcp add` workflow for registering clawft as an MCP server in Claude Code.

**Inbound (Claude Code -> clawft)**: clawft connects to Claude Code's MCP server as a client:

```toml
# In clawft.toml
[tools.mcp_servers.claude-code]
command = "claude"
args = ["mcp", "serve"]
```

The bridge orchestrates both directions, handling tool namespace conflicts.

### Cross-Element Dependencies

- **F9a (Element 07)**: Minimal MCP client needed for M4. F9a provides `MpcClient::connect()`, `list_tools()`, `invoke()`.
- **Contract 3.2 (MCP Client <-> Agent Routing)**: Per-agent MCP server configuration overrides global config.
- **MCP file ownership**: M4 owns `discovery.rs`, M5 owns `bridge.rs`. Shared files (`transport.rs`, `types.rs`, `middleware.rs`) require cross-stream review.

### Security Criteria

- [x] MCP temp files use `tempfile` crate with `0600` permissions
- [x] MCP server connections use validated URLs (SSRF protection)
- [x] Drain-and-swap protocol: no tool calls to draining servers after drain starts

### Acceptance Criteria

- [x] `weft mcp add <name> <command>` registers MCP server at runtime
- [x] `weft mcp list` shows all registered servers and their status
- [x] `weft mcp remove <name>` cleanly disconnects and removes server
- [x] Hot-reload: modify `clawft.toml` MCP section -> tools update within debounce window
- [x] Drain-and-swap: in-flight calls complete before disconnect
- [x] Bidirectional bridge: clawft tools accessible from Claude Code, Claude Code tools from clawft
- [x] File ownership respected (M4 = discovery.rs, M5 = bridge.rs)

### Test Requirements

- Add/remove MCP server lifecycle test
- Hot-reload: modify config, verify tool list updates
- Drain protocol: start tool call, remove server, verify call completes
- Bridge: verify tools visible in both directions (mock MCP)

---

## Unit 5: L4 Planning Strategies (Week 8-9, advanced)

### Objective

Add ReAct (Reason+Act) and Plan-and-Execute strategies to the Router with guard rails to prevent infinite loops and cost runaway.

### Deliverables

#### 5.1 Planning Strategies in Router

```rust
pub enum PlanningStrategy {
    ReAct,         // Reason+Act loop
    PlanAndExecute, // Generate plan, then execute steps
}

pub struct PlanningRouter {
    strategy: PlanningStrategy,
    config: PlanningConfig,
}
```

#### 5.2 Guard Rails

Per orchestrator Section 8:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_planning_depth` | 10 | Max planning steps before forced termination |
| `max_planning_cost_usd` | 1.0 | Hard budget cap for LLM calls during planning |
| `planning_step_timeout` | 60s | Max duration for a single planning step |
| Circuit breaker | 3 no-op steps | If 3 consecutive no-op steps, abort with partial results |

Config in `clawft.toml`:

```toml
[router.planning]
max_planning_depth = 10
max_planning_cost_usd = 1.0
planning_step_timeout = "60s"
circuit_breaker_no_op_limit = 3
```

### Cross-Element Dependencies

- **D6 (Element 05)**: `sender_id` threaded through pipeline for cost tracking

### Note on MVP Scope

L4 is **NOT in MVP** (post-Week 8). This is an advanced feature.

### Acceptance Criteria

- [x] `max_planning_depth` enforced
- [x] `max_planning_cost_usd` enforced
- [x] `planning_step_timeout` enforced
- [x] Circuit breaker aborts after 3 consecutive no-op steps
- [x] Partial results returned with explanation when limits hit

### Test Requirements

- Test each guard rail independently
- Test circuit breaker: 3 no-op steps -> abort
- Test partial results include explanation of which limit triggered

---

## Unit 6: M6 Documentation (Week 8-9)

### Objective

Document the delegation configuration and MCP bridge setup for end users.

### Deliverables

Add to `docs/guides/configuration.md`:
- How to enable delegation (`claude_enabled`, `claude_flow_enabled`)
- How to write routing rules (regex patterns -> targets)
- How to configure excluded tools
- How to set up Claude Code integration (PATH, API key, MCP bridge)
- Troubleshooting: common failures and diagnostics

Add to `docs/guides/tool-calls.md`:
- MCP bridge setup guide
- Dynamic MCP server management (`weft mcp add/list/remove`)

### Acceptance Criteria

- [ ] Delegation config documented in `docs/guides/configuration.md`
- [ ] MCP bridge setup guide in `docs/guides/tool-calls.md`
- [ ] Troubleshooting section covers: binary not found, API key missing, timeout, depth limit

---

## File Map

| File | Unit | Action |
|------|------|--------|
| `crates/clawft-services/src/delegation/flow.rs` | 1 | NEW: FlowDelegator |
| `crates/clawft-services/src/delegation/claude.rs` | 1 | Extend DelegationError |
| `crates/clawft-services/src/delegation/mod.rs` | 1 | Add `pub mod flow;` |
| `crates/clawft-tools/src/delegate_tool.rs` | 1 | Wire flow_available, add FlowDelegator |
| `crates/clawft-cli/Cargo.toml` | 1 | Add `delegate` to default features |
| `crates/clawft-types/src/delegation.rs` | 1 | `claude_enabled` default to `true` |
| `crates/clawft-core/src/agent_routing.rs` | 2 | NEW: AgentRouter |
| `crates/clawft-types/src/agent_routing.rs` | 2 | Add AgentRoute, MatchCriteria types |
| `crates/clawft-types/src/agent_bus.rs` | 3 | NEW: InterAgentMessage, MessagePayload |
| `crates/clawft-core/src/agent_bus.rs` | 3 | NEW: AgentBus |
| `crates/clawft-services/src/mcp/discovery.rs` | 4 | NEW: McpServerManager |
| `crates/clawft-services/src/mcp/bridge.rs` | 4 | NEW: Bidirectional MCP bridge |
| `crates/clawft-core/src/planning.rs` | 5 | NEW: PlanningRouter |
| `docs/guides/configuration.md` | 6 | Update with delegation docs |
| `docs/guides/tool-calls.md` | 6 | Update with MCP bridge guide |

---

## Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| FlowDelegator env variable leakage | Medium | Critical | **8** | Construct minimal child env explicitly |
| Cross-agent bus message eavesdropping | Medium | High | **6** | Per-agent inboxes with agent-scoped delivery |
| Recursive delegation loop | Low | Medium | **3** | Depth limit (default: 3), depth counter |
| MCP hot-reload race condition | Low | Medium | **3** | Drain-and-swap protocol, 30s timeout |
| L4 planning infinite loop / cost runaway | Medium | High | **6** | Guard rails: max_depth, max_cost, circuit breaker |
| MCP file ownership conflicts | Medium | Medium | **4** | Per-file ownership table, cross-stream review |
