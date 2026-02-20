# Element 09: Multi-Agent Routing & Claude Flow Integration - Sprint Tracker
**Workstreams**: L (Multi-Agent Routing), M (Claude Flow Integration)
**Timeline**: Weeks 3-9
**Status**: Planning

---

## Phase Tracking

| Phase | Document | Status | Assigned | Notes |
|-------|----------|--------|----------|-------|
| M-Foundation (W3-5) | 01-phase-MFoundation-flow-delegator.md | Planning | TBD | FlowDelegator, DelegationError, flow_available, delegate feature |
| L-Routing (W5-7) + L3 Swarming (W7-8) | 02-phase-LRouting-agents-swarming.md | Planning | TBD | AgentRouter, per-agent isolation, AgentBus, SwarmCoordinator |
| M-Advanced (W6-8) + L4 Planning (W8-9) | 03-phase-MAdvanced-mcp-planning.md | Planning | TBD | McpServerManager, hot-reload, MCP bridge, PlanningRouter |

---

## Key Deliverables Checklist

### M-Foundation (Weeks 3-5)
- [ ] **M1**: FlowDelegator creation (`delegation/flow.rs`)
- [ ] **M2**: Wire `flow_available` to runtime detection
- [ ] **M3**: Enable `delegate` feature by default

### L-Routing & Swarming (Weeks 5-8)
- [ ] **L1**: Agent routing table (`routing.rs`)
- [ ] **L2**: Per-agent workspace and session isolation
- [ ] **L3**: InterAgentMessage, AgentBus, SwarmCoordinator

### M-Advanced & Planning (Weeks 6-9)
- [ ] **M4**: Dynamic MCP server discovery (`discovery.rs`)
- [ ] **M5**: Bidirectional MCP bridge (`bridge.rs`)
- [ ] **L4**: ReAct and Plan-and-Execute with guard rails (`planning.rs`)
- [ ] **M6**: Delegation config documentation

---

## File Map

| File | Unit | Action |
|------|------|--------|
| `crates/clawft-services/src/delegation/flow.rs` | M1 | NEW |
| `crates/clawft-services/src/delegation/claude.rs` | M1 | Extend DelegationError |
| `crates/clawft-services/src/delegation/mod.rs` | M1 | Add `pub mod flow` |
| `crates/clawft-tools/src/delegate_tool.rs` | M2 | Wire `flow_available` |
| `crates/clawft-cli/Cargo.toml` | M3 | Add `delegate` to default |
| `crates/clawft-types/src/delegation.rs` | M3 | `claude_enabled=true` |
| `crates/clawft-core/src/routing.rs` | L1 | NEW |
| `crates/clawft-types/src/routing.rs` | L1 | NEW |
| `crates/clawft-types/src/agent_bus.rs` | L3 | NEW |
| `crates/clawft-core/src/agent_bus.rs` | L3 | NEW |
| `crates/clawft-services/src/mcp/discovery.rs` | M4 | NEW |
| `crates/clawft-services/src/mcp/bridge.rs` | M5 | NEW |
| `crates/clawft-core/src/planning.rs` | L4 | NEW |
| `docs/guides/configuration.md` | M6 | Update |
| `docs/guides/tool-calls.md` | M6 | Update |

---

## Cross-Element Dependencies

| Dependency | Element | Description |
|------------|---------|-------------|
| 03/B5 | Critical Fixes & Cleanup | Shared tool registry |
| 04/C1 | Plugin & Skill System | Plugin traits |
| 05/D6 | Pipeline Reliability | `sender_id` threading |
| 05/D9 | Pipeline Reliability | MCP transport |
| 08/H1 | Memory & Workspace | `WorkspaceManager::ensure_agent_workspace()` |

---

## Risks

| # | Risk | Impact | Mitigation |
|---|------|--------|------------|
| 1 | **Environment leakage** between agents | API keys or secrets from one agent visible to another | Per-agent workspace isolation (L2); sanitize env before delegation |
| 2 | **Bus eavesdropping** on inter-agent messages | Agents reading messages not intended for them | Channel-level ACLs on AgentBus; message encryption for sensitive payloads |
| 3 | **Recursive delegation** loops | FlowDelegator delegates to Claude Flow which delegates back infinitely | Max delegation depth counter; circuit breaker on re-entrant calls |
| 4 | **Hot-reload race conditions** in MCP discovery | Server list changes mid-request causing routing failures | Atomic swap of server registry; grace period for in-flight requests |
| 5 | **Planning loops** in ReAct/Plan-and-Execute | Agent gets stuck re-planning without making progress | Step budget guard rails; forced action after N consecutive re-plans |
| 6 | **Message delivery failures** in AgentBus | Lost messages cause silent task failures | At-least-once delivery with ack; dead-letter queue for undeliverable messages |
| 7 | **File ownership conflicts** in shared workspaces | Multiple agents writing to same file simultaneously | File-level locks via WorkspaceManager; optimistic concurrency with conflict detection |
