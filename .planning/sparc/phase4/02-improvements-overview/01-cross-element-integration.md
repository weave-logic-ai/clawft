# Cross-Element Integration Specification

**Project**: clawft -- Improvements Sprint (Post-Phase 4)
**Created**: 2026-02-19
**Status**: Active specification
**Purpose**: Resolve integration gaps identified across SPARC elements 03-10 in iteration-1 reviews

---

## 1. Phase Numbering Standard

All sprint work is "Phase 5". The numbering schemes across documents map as follows:

| Document | Numbering Scheme | Scope |
|----------|-----------------|-------|
| Business Requirements (`01-biz-features-deploy.md`) | Sections 5d through 5o | Feature requirements in dependency order |
| Technical Requirements (`02-tech-pipeline.md`) | Sections 12-19 (core), 20-26 (pipeline) | Implementation specifications |
| Development Guide (`03-dev-phases.md`) | Phase 5 with streams 5A-5K | Weekly schedule and stream assignments |

All three documents describe the same unified sprint. References such as "Phase 1.5", "Phase 2", "Phase 3G+", or "Phase 6" from earlier planning iterations are superseded. Any remaining occurrences of non-Phase-5 numbering in sprint documents are errata and must be corrected to use the Phase 5 convention.

**Cross-reference table** (business requirement -> technical section -> development stream):

| Business Section | Tech Section(s) | Dev Stream | Description |
|-----------------|-----------------|------------|-------------|
| 5d | 12-13 | 5A | Critical fixes and security patches |
| 5e | 14 | 5B | Architecture cleanup and file splits |
| 5f | 15-16 | 5C | Plugin trait crate and WASM host |
| 5g | 17-18 | 5D | Pipeline reliability and bounded bus |
| 5h | 19 | 5E | Channel enhancements |
| 5i | 20 | 5F | Dev tools and applications |
| 5j | 21-22 | 5G | Memory and workspace |
| 5k | 23 | 5H | Multi-agent routing |
| 5l | 24 | 5I | Claude Flow integration |
| 5m | 25 | 5J | Type safety and doc sync |
| 5n-5o | 26 | 5K | Deployment and community |

---

## 2. Integration Test Plan

The following end-to-end integration tests verify cross-element interactions. Each test exercises at least two SPARC elements and validates that their interface contracts hold under realistic conditions.

| Test | Elements | Description | Week | Priority |
|------|----------|-------------|------|----------|
| Email Channel -> OAuth2 Helper | 06, 07 | Configure Gmail via E2, verify F6 OAuth2 flow completes, token stored in agent workspace | 7 | P0 |
| Plugin -> Hot-reload -> MCP | 04, 07 | Install a skill via C4 loader, modify SKILL.md, verify MCP `tools/list` response updates within hot-reload debounce window | 8 | P0 |
| FlowDelegator -> Per-Agent Isolation | 09, 08 | Delegate task from agent A via M1, verify agent B's workspace directory is not accessible from the delegation subprocess | 9 | P0 |
| Multi-Agent -> Bus Isolation | 09, 05 | Send messages to 2 agents on the same gateway, verify no cross-agent message leakage on the bus | 9 | P0 |
| Agent Routing -> Sandbox | 09, 10 | Route message to agent with restricted tool permissions, verify K3 sandbox enforces tool restrictions at runtime | 10 | P1 |
| Vector Search -> ClawHub Discovery | 08, 10 | Search ClawHub with a semantic query via H2 vector store, verify relevant skill results returned (requires H2.1 HNSW + K4 registry) | 11 | P1 |
| ClawHub Install -> Security Scan | 10, 04 | Run `weft skill install` from ClawHub, verify K3a security scan executes before skill activation, verify blocking severity halts install | 11 | P1 |

### Test Infrastructure Requirements

- Integration tests live in `tests/integration/cross_element/` (one file per test).
- Tests require a running gateway instance with at least 2 configured agents.
- Tests that involve MCP require a mock MCP server (can reuse existing test fixtures from `clawft-services`).
- Tests that involve ClawHub require a local HTTP index (mock server or file-based stub).
- CI runs cross-element integration tests on the `sprint/phase-5` integration branch only (not on individual stream branches).

### Test Sequencing

Tests are ordered by the week their dependent elements complete:

```
Week 7:  Email -> OAuth2
Week 8:  Plugin -> Hot-reload -> MCP
Week 9:  FlowDelegator -> Isolation, Multi-Agent -> Bus Isolation
Week 10: Agent Routing -> Sandbox
Week 11: Vector Search -> ClawHub, ClawHub Install -> Security Scan
```

Each test becomes a CI gate on the integration branch starting from its scheduled week.

---

## 3. Cross-Element Interface Contracts

### 3.1 Tool Plugin <-> Memory (07 <-> 08)

**Problem**: Dev tools (F1-F7) need to persist state (OAuth tokens, repo locations, browser bookmarks). Direct dependency on `MemoryBackend` from plugin crates creates a circular dependency with `clawft-plugin`.

**Contract**:

- Tool plugins use `ToolContext::key_value_store` for state persistence.
- `KeyValueStore` is a simple trait: `get(key) -> Option<Vec<u8>>`, `set(key, value)`, `delete(key)`.
- `KeyValueStore` does NOT provide vector search, embedding, or RVF operations.
- `ToolContext` is provided by `AgentLoop` at tool execution time.
- The `AgentLoop` implementation backs `KeyValueStore` with the agent's workspace directory (`~/.clawft/agents/<agentId>/tool_state/<plugin_name>/`).
- Plugin crates depend on `clawft-plugin` (for the trait) only, never on `clawft-core` memory modules.

**Trait definition** (lives in `clawft-plugin`):

```rust
pub trait KeyValueStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
    fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
}
```

### 3.2 MCP Client <-> Agent Routing (07 <-> 09)

**Problem**: When Agent A has access to MCP server X but Agent B does not, the per-agent tool registry must respect this boundary. The current plan lacks per-agent MCP server configuration.

**Contract**:

- Per-agent MCP server configuration lives in the agent workspace config: `~/.clawft/agents/<agentId>/config.toml`.
- The `mcp_servers` section in agent config overrides or extends the global `clawft.toml` MCP server list.
- Override semantics: agent-level entries with the same server name replace global entries. Agent-level entries with new names are appended. To explicitly exclude a global server, set `enabled = false` at the agent level.
- The routing table entry can optionally specify `mcp_servers` to limit which servers connect for that agent.
- At agent startup (or first routing), the agent's MCP client connections are established based on the merged config.

**Config example**:

```toml
# In ~/.clawft/agents/coding-agent/config.toml
[mcp_servers.github]
url = "stdio://gh-mcp-server"
enabled = true

[mcp_servers.slack]
enabled = false  # Explicitly exclude global slack server for this agent
```

### 3.3 Workspace <-> Routing (08 <-> 09)

**Problem**: Per-agent workspace isolation (H1/L2) is described in both elements but ownership is unclear. Who creates the workspace? Who manages lifecycle?

**Contract**:

- H1 owns `WorkspaceManager` -- the sole authority for workspace CRUD operations.
- `WorkspaceManager` provides: `create_workspace(agent_id, template) -> Result<PathBuf>`, `ensure_workspace(agent_id) -> Result<PathBuf>`, `delete_workspace(agent_id) -> Result<()>`, `list_workspaces() -> Result<Vec<AgentId>>`.
- L2 (agent routing) calls `WorkspaceManager::ensure_workspace(agent_id)` when routing a message to an agent for the first time.
- `ensure_workspace` is idempotent: if the workspace exists, it returns the path. If not, it creates from the default template (`~/.clawft/agents/default/` if present, otherwise bare minimum with empty `SOUL.md` and default `config.toml`).
- Workspace deletion is an administrative operation (CLI command or API), never triggered by routing or message handling.

### 3.4 Delegation <-> Routing (09 internal: L <-> M)

**Problem**: The delegation system (M) routes tasks to external agents (Claude Code). Multi-agent routing (L) routes messages to internal agents. No specification covers how these interact.

**Contract**:

- Any agent can delegate to Claude Code, not just the default agent.
- `FlowDelegator` includes the requesting `agent_id` in the MCP callback context, so that Claude Code responses route back to the correct agent.
- The delegation path threads `agent_id` through: `AgentLoop -> DelegationEngine.decide(task, agent_id) -> FlowDelegator.delegate(task, agent_id)`.
- Claude Code MCP callbacks include `x-clawft-agent-id` in the request metadata.
- The MCP server handler extracts `x-clawft-agent-id` and routes the tool call result back to the originating agent's pipeline.
- If `x-clawft-agent-id` is missing (legacy or misconfigured), the response routes to the default agent.

---

## 4. MVP Milestone Clarification

### Items IN MVP (Week 8)

| Category | Items | Description |
|----------|-------|-------------|
| Critical Fixes | A1-A9 | All critical and high severity fixes resolved |
| Architecture | B1-B9 | File splits, architecture cleanup complete |
| Plugin System | C1-C4 | Plugin trait crate, WASM host, skill loader, hot-reload all functional |
| Email Channel | E2 | Email channel operational with OAuth2 |
| Agent Routing | L1 | Agent routing table with first-match-wins semantics |
| Claude Flow | M1-M3 | FlowDelegator basic delegation, delegate feature enabled by default |
| MCP Client | F9a | Minimal MCP client: connect to single configured server, list tools, invoke tools |
| OpenClaw Skills | 3 named | `coding-agent`, `web-search`, `file-management` -- ported as native plugins |

### Items NOT in MVP (post-Week 8)

| Item | Target Week | Reason for Exclusion |
|------|------------|---------------------|
| F9b full MCP auto-discovery | Week 9-10 | Connection pooling, schema caching, health checks are post-MVP polish |
| K2-K5 deployment and community | Week 8-12 | Docker images, ClawHub, benchmarks depend on stable feature set |
| C4a autonomous skill creation | P2/stretch | Requires mature plugin ecosystem; premature for MVP |
| L3-L4 advanced multi-agent | Week 8-9 | Swarming and ReAct/Plan-and-Execute are advanced features beyond basic routing |
| H2.6 WITNESS segments | Week 7-8 | Tamper-evident audit trail is a hardening feature, not MVP-critical |
| H2.7 temperature-based quantization | Week 8+ | Performance optimization, not functional requirement |
| H2.8 WASM micro-HNSW | Week 8+ | WASM deployment is post-MVP |

### MVP Verification Checklist

1. `cargo test --workspace` passes with zero failures
2. `cargo clippy --workspace -- -D warnings` produces no warnings
3. Binary size < 10 MB (release build, default features)
4. Gateway starts, accepts email message, routes to agent, agent responds
5. `weft skill install coding-agent` loads skill, tools appear in `tools/list`
6. Hot-reload: modify `SKILL.md`, verify tool list updates within 2 seconds
7. FlowDelegator: delegate task to Claude Code, receive response, response routes to correct agent
8. MCP client (F9a): connect to external MCP server, list tools, invoke one tool

---

## 5. Merge Coordination Protocol

With 11 concurrent stream branches (5A through 5K), merge coordination is critical to avoid integration drift.

### 5.1 Merge Order

Merges into the integration branch follow the dependency graph:

```
5A (fixes) -> 5B (cleanup) -> 5C (plugin) -> 5D (pipeline)
                                    |
                                    +-> 5E (channels)
                                    +-> 5F (dev tools)
                                    +-> 5G (memory)
                                    +-> 5H (routing)
                                    +-> 5I (claude flow)
                                    +-> 5J (type safety)
                                    +-> 5K (deployment)
```

Streams that share no dependency can merge in any order relative to each other (e.g., 5E and 5F are independent).

### 5.2 Merge Requirements

Each merge into the integration branch requires ALL of the following:

1. **All stream tests pass**: `cargo test -p <affected-crates>` green
2. **Binary size check**: `cargo build --release` binary < 10 MB (asserted by CI)
3. **No new clippy warnings**: `cargo clippy --workspace -- -D warnings` clean
4. **Cross-element tests pass** (if the merge touches elements with integration tests scheduled for that week or earlier)
5. **PR review**: At minimum one review from a developer not on the merging stream

### 5.3 Integration Branch

- **Branch name**: `sprint/phase-5`
- **Merge target**: All stream branches merge here
- **CI gates on `sprint/phase-5`**:
  - Full workspace test (`cargo test --workspace`)
  - Binary size assertion (< 10 MB release)
  - Clippy lint check
  - Cross-element integration tests (cumulative, based on current week)
- **Merge to `master`**: Only at MVP milestone (Week 8) and sprint completion (Week 12), after full regression pass

### 5.4 Merge Cadence

- **Minimum**: Weekly integration merges from each active stream
- **Recommended**: Merge after each completed sub-deliverable (e.g., after C1 lands, after E2 lands)
- **Conflict resolution**: When two streams modify the same file, the stream that is earlier in the dependency graph merges first. The later stream rebases.

### 5.5 Conflict Zones

Known conflict zones requiring extra coordination:

| Files | Streams | Resolution |
|-------|---------|------------|
| `clawft-types/src/config.rs` | 5A, 5B, 5C | 5A merges first (fixes), 5B rebases, 5C rebases |
| `clawft-core/src/pipeline/` | 5D, 5C | 5C merges first (plugin traits), 5D rebases for tiered_router changes |
| `clawft-services/src/mcp/` | 5C, 5F, 5I | Per-file ownership (see Section 3.2 of arch review). C6 owns `server.rs`, F9 owns `client.rs`, M4 owns `discovery.rs`, M5 owns `bridge.rs`, F8 owns `ide.rs`. Shared files (`transport.rs`, `types.rs`, `middleware.rs`) require cross-stream PR review. |

---

## 6. Forward-Compat Verification

The following verification tests ensure that forward-compatibility hooks for Voice and UI are not broken during sprint execution.

| Hook | Verification Test | Element | Type | Week Added |
|------|------------------|---------|------|------------|
| `VoiceHandler` trait | Compile-time: instantiate `VoiceHandler` placeholder struct, verify it implements required trait bounds | 04 | Compile | 5 |
| Binary `ChannelAdapter` | Runtime: send a binary payload (`mime_type: "audio/wav"`, data: 1KB zero bytes) through `ChannelAdapter`, verify it round-trips without corruption | 04, 06 | Runtime | 6 |
| `MessagePayload` enum | Compile-time: construct `Text`, `Structured`, and `Binary` variants of `MessagePayload`, verify exhaustive match compiles | 05, 09 | Compile | 5 |
| Voice feature flag | Build: `cargo build --features voice` compiles without errors (feature is a no-op but must not break) | 04 | Build | 4 |
| Config read-access API | Runtime: read agent config and session state without holding an `AgentLoop` reference, verify values are accessible | 08, 09 | Runtime | 7 |

### Verification Placement

- Compile-time tests: `tests/forward_compat/compile_checks.rs` (runs as part of `cargo test`)
- Runtime tests: `tests/forward_compat/runtime_checks.rs` (runs as part of integration test suite)
- Build tests: CI job that runs `cargo build --features voice` on every merge to `sprint/phase-5`

### Regression Policy

If any forward-compat verification test fails on a merge to `sprint/phase-5`:

1. The merge is blocked
2. The stream author must fix the regression before re-merging
3. Forward-compat tests are never deleted or `#[ignore]`d without explicit sign-off from the sprint lead

---

## 7. F9a/F9b Split Decision Record

### Decision

F9 (MCP Client for External Servers) is split into two deliverables to resolve the M4/F9 timeline conflict identified in both the deployment-integration review and the arch-07-08-09 review.

### F9a: Core MCP Client (Week 5-6, MVP scope)

- **Scope**: Connect to a single configured MCP server, list available tools, invoke tools with JSON-RPC, return results.
- **Location**: `clawft-services/src/mcp/client.rs`
- **Interface**: `MpcClient::connect(config) -> Result<Self>`, `list_tools() -> Result<Vec<ToolSchema>>`, `invoke(tool_name, params) -> Result<Value>`
- **Dependencies**: D9 (MCP transport concurrency) must land first
- **Consumers**: M4 depends on F9a for dynamic MCP discovery (single-server case)

### F9b: Full MCP Client Features (Week 9-10, post-MVP)

- **Scope**: Auto-discovery of MCP servers from config/environment, connection pooling (reuse connections across tool calls), schema caching (avoid re-listing on every call), health checks (detect and reconnect failed servers), namespace management for 1000+ external tools.
- **Location**: Extends `clawft-services/src/mcp/client.rs` and adds `clawft-services/src/mcp/discovery.rs`
- **Dependencies**: F9a must be stable
- **Consumers**: K4 ClawHub MCP integration, full multi-server agent configurations

### Rationale

- M4 is scheduled for Week 5. F9 was originally scheduled for Week 10. M4 depends on F9.
- The full F9 feature set (auto-discovery, pooling, caching) is not needed for M4's core use case.
- Splitting allows M4 to proceed on schedule using F9a's minimal client.
- F9b adds production hardening that benefits from the stability of later sprint weeks.

### Impact on Other Elements

- M4: Unblocked. Depends on F9a only.
- K4 (ClawHub): No impact. K4 starts Week 10-11, by which time F9b is available.
- F8 (MCP IDE): No impact. F8 is server-side, not client-side.
- M5 (Bidirectional bridge): Can use F9a for initial implementation, upgraded to F9b when available.

---

## 8. Phase 4/5 Transition

Phase 5 assumes Phase 4 (Tiered Router) is complete and its file ownership claims are released. This section defines the transition protocol for the overlap period.

### 8.1 Shared Crate Coordination

| Crate | Phase 4 Owner | Phase 5 Streams | Coordination Required |
|-------|--------------|-----------------|----------------------|
| `clawft-types` | 4A (provider types) | 5A, 5B (config cleanup) | PR review from both phase leads |
| `clawft-core/src/pipeline/` | 4C (tiered_router.rs) | 5D (pipeline reliability) | Merge 4C first, 5D rebases |
| `clawft-llm` | 4B (router logic) | 5D (streaming failover) | Distinct files, low conflict risk |
| `clawft-cli` | 4D (CLI integration) | 5A (fix commands) | Sequential: 4D completes before 5A touches CLI |

### 8.2 Transition Gate

Phase 5 stream branches must not be created until:

1. Phase 4 PR is merged to `master` (or its integration branch)
2. All Phase 4 CI gates pass
3. Phase 4 file ownership claims are explicitly released (documented in PR description)

If Phase 4 and Phase 5 overlap due to schedule pressure:

- `clawft-types`: Coordinated changes between 4A and 5A/5B require cross-phase PR review. Both phase leads must approve.
- `clawft-core/src/pipeline/tiered_router.rs`: 4C changes merge first. 5D changes to the same file are rebased onto 4C's final state. No parallel edits to `tiered_router.rs`.
- Other crates: Phase 5 streams may begin work in crates not touched by Phase 4 (e.g., `clawft-channels`, `clawft-services`, `clawft-tools`).

### 8.3 Rollback Protocol

If Phase 4 is reverted after Phase 5 has started:

- Phase 5 streams that depend on Phase 4 types (`ProviderConfig`, `TieredRouterConfig`) must be rebased or reverted.
- Phase 5 streams with no Phase 4 dependency (06, 07, 08) continue unaffected.
- The integration branch `sprint/phase-5` must pass `cargo test --workspace` after any Phase 4 revert.
