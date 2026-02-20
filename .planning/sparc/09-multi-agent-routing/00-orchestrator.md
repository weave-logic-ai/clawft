# SPARC Feature Element 09: Multi-Agent Routing & Claude Flow Integration

**Workstreams**: L (Multi-Agent Routing & Orchestration), M (Claude Flow / Claude Code Integration)
**Timeline**: Weeks 3-9
**Status**: Planning
**Dependencies**: 03/B5 (shared tool registry), 04/C1 (plugin traits), 05/D6 (sender_id), 05/D9 (MCP transport)
**Blocks**: None directly

---

## 1. Summary

Multi-agent routing (different users -> different agents), per-agent isolation, multi-agent swarming, and full Claude Flow / Claude Code integration with bidirectional MCP bridge.

---

## 2. Phases

### Phase M-Foundation: Claude Flow Integration (Week 3-5)

| Item | Description |
|------|-------------|
| M1 | Extend `FlowDelegator` (already exists in `clawft-services/src/delegation/claude.rs`; M1 extends it, not creates from scratch) with full error handling contract (see Section 4) |
| M2 | Wire `flow_available` to runtime detection (`which` + config) |
| M3 | Enable `delegate` feature by default |

### Phase L-Routing: Agent Routing (Week 5-7)

| Item | Description |
|------|-------------|
| L1 | Agent routing table (channel/user -> agent mapping) with fallback behavior (see Section 7) |
| L2 | Per-agent workspace and session isolation |
| L3 | Multi-agent swarming (coordinator pattern via AgentBus, see Section 3) |

### Phase M-Advanced: MCP & Discovery (Week 6-8)

| Item | Description |
|------|-------------|
| M4 | Dynamic MCP server discovery (`weft mcp add/list/remove`). Depends on F9a (core MCP client). |
| M5 | Bidirectional MCP bridge (clawft <-> Claude Code) with hot-reload (see Section 5) |
| M6 | Delegation config documentation |

### Phase L-Advanced: Planning Strategies (Week 8-9)

| Item | Description |
|------|-------------|
| L4 | ReAct (Reason+Act) and Plan-and-Execute in Router with guard rails (see Section 8) |

---

## 3. Cross-Agent Communication Protocol

### InterAgentMessage Type

```rust
pub struct InterAgentMessage {
    /// Unique message identifier.
    pub id: Uuid,
    /// Agent ID of the sender.
    pub from_agent: String,
    /// Agent ID of the recipient.
    pub to_agent: String,
    /// Task description or intent.
    pub task: String,
    /// Arbitrary JSON payload.
    pub payload: Value,
    /// If this is a reply, the ID of the original message.
    pub reply_to: Option<Uuid>,
    /// Time-to-live: message expires if undelivered within this duration.
    pub ttl: Duration,
}
```

### AgentBus

The `AgentBus` is a dedicated inter-agent communication layer, separate from the channel message bus (`MessageBus`).

- **Per-agent inboxes**: Each agent has its own inbox. Messages are delivered to the inbox of `to_agent`.
- **Agent-scoped delivery**: Agents can only read messages from their own inbox. An agent cannot access another agent's inbox.
- **Coordinator pattern**: The L3 `SwarmCoordinator` uses the `AgentBus` to dispatch subtasks to worker agents and collect results.
- **TTL enforcement**: Expired messages are dropped and logged at `warn` level.
- **Acknowledgment**: The recipient sends a reply message (with `reply_to` set) to acknowledge receipt or return results.

### UI Forward-Compat: MessagePayload

Bus message types support structured and binary payloads for future canvas rendering and voice:

```rust
pub enum MessagePayload {
    /// Plain text content.
    Text(String),
    /// Structured JSON content.
    Structured(Value),
    /// Binary content with MIME type (e.g., image/png, audio/wav).
    Binary { mime_type: String, data: Vec<u8> },
}
```

This enum should be implemented in stream 5D (D8 bounded bus) or 5B (B1/B2 type unification) since it is a shared type change.

---

## 4. FlowDelegator Error Handling Contract

```rust
pub enum DelegationError {
    /// The claude subprocess exited with a non-zero status.
    SubprocessFailed { exit_code: i32, stderr: String },
    /// stdout produced output that could not be parsed as expected JSON.
    OutputParseFailed { raw_output: String, parse_error: String },
    /// The delegation exceeded the configured timeout.
    Timeout { elapsed: Duration },
    /// The delegation was cancelled (e.g., user abort, agent shutdown).
    Cancelled,
    /// All fallback targets exhausted (Flow -> Claude -> Local).
    FallbackExhausted { attempts: Vec<(DelegationTarget, String)> },
}
```

The `FlowDelegator` must handle all variants and surface actionable error messages to the agent loop. `FallbackExhausted` triggers a user-facing error explaining which delegation targets were tried and why each failed.

---

## 5. MCP Hot-Reload Protocol

Dynamic MCP server management (M4) uses a **drain-and-swap** protocol when `clawft.toml` changes:

1. File watcher detects change to `[mcp_servers]` section; **debounce 500ms**.
2. Diff old and new server lists.
3. **New servers**: Connect immediately; add tools to `ToolRegistry` with `mcp:<server-name>:<tool-name>` namespace.
4. **Removed servers**: Mark as "draining"; complete in-flight tool calls (up to **30s timeout**); then disconnect and remove tools from `ToolRegistry`.
5. **Changed servers** (same name, different config): Treat as remove + add.

Tool calls arriving during the drain period for a removed server are completed normally. New tool calls targeting a draining server are rejected with an error.

---

## 6. MCP File Ownership

Formalized file ownership within `clawft-services/src/mcp/`:

| File | Owner | Purpose |
|------|-------|---------|
| `server.rs` | C6 | Existing MCP server; skill exposure via MCP |
| `client.rs` | F9 | NEW: MCP client connections to external servers |
| `discovery.rs` | M4 | NEW: Dynamic server management (`weft mcp add/list/remove`) |
| `bridge.rs` | M5 | NEW: Bidirectional bridge orchestration (clawft <-> Claude Code) |
| `ide.rs` | F8 | NEW: IDE-specific MCP extensions (VS Code backend) |
| `transport.rs` | Shared | Existing transport layer (changes require cross-stream review) |
| `types.rs` | Shared | Existing types (changes require cross-stream review) |
| `middleware.rs` | Shared | Existing middleware (changes require cross-stream review) |

---

## 7. Routing Fallback Behavior

Explicit behavior for edge cases in the L1 agent routing table:

- **No matching route + no catch-all**: Reject message with "no agent configured for this channel/user" error, logged at `warn` level. The message is not silently dropped.
- **Matched agent workspace does not exist**: Auto-create from default template (`~/.clawft/agents/default/` if it exists, otherwise bare minimum with empty SOUL.md). Auto-created agents inherit a minimal permission profile.
- **Anonymous messages** (no sender_id): Route to the catch-all agent or a dedicated "anonymous" agent with reduced permissions.

---

## 8. L4 Planning Guard Rails

ReAct and Plan-and-Execute strategies (L4) must enforce the following limits to prevent infinite loops and runaway costs:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_planning_depth` | 10 | Maximum number of planning steps before forced termination |
| `max_planning_cost_usd` | 1.0 | Hard budget cap for LLM calls during planning |
| `planning_step_timeout` | 60s | Maximum duration for a single planning step |
| Circuit breaker | 3 no-op steps | If 3 consecutive planning steps produce no actionable output, abort and return partial results with explanation |

All limits are configurable in `clawft.toml` under `[router.planning]`. When any limit is hit, the planner returns partial results to the user with a clear explanation of which limit was triggered.

---

## 9. M1 Note: Existing FlowDelegator

`ClaudeDelegator` already exists in `clawft-services/src/delegation/claude.rs`. M1 **extends** this existing implementation rather than creating it from scratch. The extension adds:
- Full `DelegationError` handling (Section 4)
- MCP callback registration verification
- Minimal environment construction for the child process
- Timeout enforcement with `child.kill()` on expiry

---

## 10. Exit Criteria

### Core Exit Criteria

- [ ] `FlowDelegator` delegates tasks to Claude Code CLI with full error handling
- [ ] `flow_available` detects `claude` binary at runtime
- [ ] `delegate` feature enabled by default
- [ ] Agent routing table maps users to isolated agents
- [ ] Routing fallback behavior works (no-match rejection, auto-create workspace)
- [ ] `weft mcp add <name> <command>` registers MCP servers at runtime
- [ ] Bidirectional MCP bridge tested (clawft -> Claude Code and reverse)
- [ ] Delegation config documented in `docs/guides/configuration.md`
- [ ] All existing tests pass

### Cross-Agent Communication Exit Criteria

- [ ] `InterAgentMessage` type defined and used by L3 swarming
- [ ] `AgentBus` provides per-agent inboxes with agent-scoped delivery
- [ ] Coordinator pattern uses `AgentBus` for subtask dispatch and result collection

### FlowDelegator Exit Criteria

- [ ] `DelegationError` enum covers all error variants (subprocess, parse, timeout, cancel, fallback)
- [ ] Fallback chain (Flow -> Claude -> Local) handles each error variant gracefully

### MCP Exit Criteria

- [ ] Hot-reload drain-and-swap protocol works (add/remove/change servers without downtime)
- [ ] File ownership respected (each stream owns its designated file in `clawft-services/src/mcp/`)

### L4 Planning Exit Criteria

- [ ] `max_planning_depth`, `max_planning_cost_usd`, `planning_step_timeout` enforced
- [ ] Circuit breaker aborts after 3 consecutive no-op steps
- [ ] Partial results returned with explanation when limits hit

### Security Exit Criteria

- [ ] Bus messages tagged with agent IDs; agents cannot read other agents' messages
- [ ] `FlowDelegator` child process receives a minimal, explicitly-constructed environment (not full parent env)
- [ ] MCP temp files use `tempfile` crate with `0600` permissions
- [ ] Delegation depth limit enforced (default: 3, configurable)

---

## 11. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Cross-agent bus message eavesdropping | Medium | High | **6** | Per-agent inboxes with agent-scoped delivery; agents cannot read other agents' messages |
| FlowDelegator environment variable leakage (secrets to child process) | Medium | Critical | **8** | Construct minimal child env explicitly; only pass PATH, HOME, ANTHROPIC_API_KEY |
| Recursive delegation loop (clawft -> Claude -> clawft -> Claude -> ...) | Low | Medium | **3** | Delegation depth limit (default: 3); depth counter threaded through delegation path |
| MCP hot-reload race condition (tool call during drain) | Low | Medium | **3** | Drain-and-swap protocol; 30s timeout for in-flight calls; new calls to draining server rejected |
| L4 planning infinite loop / cost runaway | Medium | High | **6** | max_planning_depth=10, max_planning_cost_usd=1.0, circuit breaker after 3 no-op steps |
| Inter-agent message delivery failure under load | Low | Medium | **3** | TTL enforcement; warn-level logging for expired messages; bounded inbox size |
| MCP file ownership conflicts across 5 streams | Medium | Medium | **4** | Formalized per-file ownership table; shared files require cross-stream review |
