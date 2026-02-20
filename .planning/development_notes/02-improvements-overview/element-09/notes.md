# Development Notes: Element 09 - Multi-Agent Routing & Claude Flow

**Workstreams**: L, M
**Weeks**: 3-9
**Status**: All phases complete except M6 (docs deferred)
**Completed**: 2026-02-20
**Agent**: Agent-09 (a32bd90)

---

## Implementation Log

### M1: FlowDelegator -- DONE
- `crates/clawft-services/src/delegation/flow.rs` (new)
- FlowDelegator with subprocess spawning, timeout enforcement, depth limit
- DelegationError extended: SubprocessFailed, OutputParseFailed, Timeout, Cancelled, FallbackExhausted
- Minimal env construction (PATH, HOME, ANTHROPIC_API_KEY only)
- `which` crate for binary detection with OnceLock caching

### M2: flow_available Runtime Detection -- DONE
- DelegateTaskTool updated with flow_delegator field and detect_flow_available()
- Flow -> Claude fallback chain implemented

### M3: Enable delegate Feature by Default -- DONE
- `delegate` added to clawft-cli default features
- `claude_enabled` defaults to `true` (graceful degradation)

### L1: Agent Routing Table -- DONE
- `crates/clawft-types/src/agent_routing.rs` (new): AgentRoute, MatchCriteria, AgentRoutingConfig
- `crates/clawft-core/src/agent_routing.rs` (new): AgentRouter with first-match-wins
- Catch-all routing, anonymous message routing
- No-match rejection with warn logging (not silent drop)

### L3: AgentBus & SwarmCoordinator -- DONE
- `crates/clawft-types/src/agent_bus.rs` (new): InterAgentMessage, MessagePayload
- `crates/clawft-core/src/agent_bus.rs` (new): AgentBus, AgentInbox, SwarmCoordinator
- Per-agent inboxes with bounded channels and TTL enforcement
- Agent-scoped delivery (security: no cross-agent reads)
- dispatch_subtask and broadcast_task coordination

### M4: Dynamic MCP Server Discovery -- DONE
- `crates/clawft-services/src/mcp/discovery.rs` (new)
- McpServerManager with add/remove/list/get operations
- ServerStatus enum (Connected, Connecting, Draining, Disconnected, Error)
- Hot-reload via apply_config_diff, 500ms debounce, 30s drain timeout

### M5: Bidirectional MCP Bridge -- DONE
- `crates/clawft-services/src/mcp/bridge.rs` (new)
- McpBridge with BridgeConfig and BridgeStatus
- Inbound (Claude Code -> clawft) and outbound (clawft -> Claude Code)
- Tool namespacing: mcp:<namespace>:<tool-name>

### L4: Planning Strategies -- DONE
- `crates/clawft-core/src/planning.rs` (new)
- PlanningStrategy enum (React, PlanAndExecute)
- PlanningConfig: max_depth=10, max_cost=$1.0, step_timeout=60s
- Circuit breaker: 3 no-op steps -> abort with partial results
- explain_termination() for human-readable output

### Ancillary Fix
- Added `use chrono::TimeZone` to `cron_service/storage.rs`

### M6: Delegation Config Documentation -- DEFERRED to docs sprint
