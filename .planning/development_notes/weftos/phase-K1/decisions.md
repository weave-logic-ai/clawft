# Phase K1: Supervisor + RBAC -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. AgentSupervisor does not own AgentLoop execution
**Problem**: The spec suggests the supervisor spawns `AgentLoop`, but the kernel crate cannot depend on the full AgentLoop setup (LLM transport, pipeline, etc.) without pulling in massive dependency chains.

**Decision**: AgentSupervisor creates the process table entry, allocates PID, assigns capabilities, and provides the cancellation token. The actual AgentLoop execution remains the caller's responsibility (CLI or gateway code). The supervisor manages lifecycle (stop/restart) via the cancellation token.

**Rationale**: Clean separation of concerns. The supervisor manages process metadata and lifecycle signaling; the caller manages actual execution. This avoids circular dependencies and keeps the kernel crate focused.

### 2. CapabilityChecker takes tool_permissions and sandbox as parameters
**Problem**: The spec shows CapabilityChecker reading permissions from the process entry, but ToolPermissions and SandboxPolicy are richer types than what fits in AgentCapabilities (which is designed for serializable config).

**Decision**: CapabilityChecker methods accept `Option<&ToolPermissions>` and `Option<&SandboxPolicy>` as parameters rather than reading them from the process entry. The process entry's `AgentCapabilities` provides the coarse-grained checks (can_exec_tools, can_ipc, ipc_scope, resource_limits), while fine-grained tool/service filtering comes from the caller-provided ToolPermissions.

**Rationale**: Keeps AgentCapabilities backward-compatible (it's already serialized to JSON in configs). Richer permission structures can be loaded from separate capability files without modifying the process table schema.

### 3. ToolPermissions and SandboxPolicy are separate from AgentCapabilities
**Decision**: Added three new types in capability.rs: `SandboxPolicy`, `ToolPermissions`, `ResourceType`. These are not embedded in `AgentCapabilities` to maintain serialization simplicity. They are used by `CapabilityChecker` as call parameters.

**Rationale**: AgentCapabilities was designed in K0 for process table storage with simple bool/enum fields. The richer permission model (allow/deny lists, service access, sandbox paths) is used at the checker level, not stored per-process. This follows the principle from the spec: "capabilities add tool-level and resource-level access control on top."

### 4. filtered_tools() on ToolRegistry
**Decision**: Added `filtered_tools(allow: &[String], deny: &[String]) -> ToolRegistry` to `clawft-core::tools::registry::ToolRegistry`. It creates a new registry snapshot containing only the permitted tools.

**Rationale**: The existing `schemas_for_tools()` only filters schemas (JSON output). `filtered_tools()` returns a full ToolRegistry, useful for creating per-agent tool environments where even the `execute()` method respects the filter.

### 5. Collapsible-if patterns
**Decision**: Used Rust 2024 edition `let && let` chains for capability checker code where clippy required collapsing nested if-let-if blocks.

**Rationale**: Clippy in the workspace runs as errors (-D warnings). The `if let Some(x) = y && condition` pattern is stable in edition 2024.

### 6. Error variants for RBAC
**Decision**: Added three new error variants to KernelError: `CapabilityDenied { pid, action, reason }`, `ResourceLimitExceeded { pid, resource, current, limit }`, `SpawnFailed { agent_id, reason }`.

**Rationale**: Typed errors with context (PID, resource name, limits) are essential for debugging capability denials. The error messages include enough information to diagnose issues without looking at logs.

## What Was Skipped

1. **Actual AgentLoop integration** -- supervisor creates process entries but does not spawn AgentLoop. This is an integration point for later.
2. **watch() method with tokio::sync::watch** -- deferred to K2 where we need proper async state change notifications.
3. **Capability hot-update** -- as noted in the spec, requires agent restart. Not implemented.
4. **CLI integration** (weft agent spawn --capabilities) -- deferred to after K2 when the full IPC system is available.
5. **Ruvector integration** -- feature-gated, not implemented (crates not published).

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/supervisor.rs` | ~300 | AgentSupervisor, SpawnRequest, SpawnResult |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/capability.rs` | Added CapabilityChecker, SandboxPolicy, ToolPermissions, ResourceType |
| `crates/clawft-kernel/src/process.rs` | Added set_capabilities(), count_by_state() |
| `crates/clawft-kernel/src/error.rs` | Added CapabilityDenied, ResourceLimitExceeded, SpawnFailed variants |
| `crates/clawft-kernel/src/boot.rs` | Added supervisor field, supervisor() accessor |
| `crates/clawft-kernel/src/lib.rs` | Added supervisor module, new re-exports |
| `crates/clawft-core/src/tools/registry.rs` | Added filtered_tools() method |

## Test Summary

- 20 new tests in supervisor.rs (spawn, stop, restart, inspect, list, serde)
- 21 new tests in capability.rs (CapabilityChecker: tool access, IPC, service, resources, sandbox)
- 3 new tests in process.rs (set_capabilities, count_by_state)
- 5 new tests in registry.rs (filtered_tools: allow, deny, preserve metadata)
- All 108+ pre-existing kernel tests continue to pass
- All workspace tests pass
