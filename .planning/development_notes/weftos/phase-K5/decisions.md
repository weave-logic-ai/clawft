# Phase K5: Application Framework -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. AppManager without Platform generic
**Problem**: The spec shows `AppManager<P: Platform>` to hold an `Arc<AgentSupervisor<P>>`. However, the app module's types and state machine can be tested and compiled without the Platform generic, keeping the module simpler.

**Decision**: `AppManager` is not generic over Platform. It manages manifest validation, lifecycle state, and installed app tracking. Integration with the AgentSupervisor and ContainerManager is done externally (the boot sequence or CLI wires them together).

**Rationale**: The types and state machine are the core value. Making AppManager generic would force all consumers to carry the Platform type parameter. The supervisor and container manager integrations are injected via method calls (add_agent_pid, add_service_name) rather than struct fields.

### 2. JSON for manifest testing, TOML deferred
**Problem**: The spec specifies `weftapp.toml` but the workspace does not have a `toml` crate dependency. Adding it would change Cargo.lock for all builds.

**Decision**: AppManifest derives Serialize/Deserialize and is format-agnostic. Tests use serde_json. TOML support (via the `toml` crate) will be added when the CLI `weft app install` command is implemented, as that's where file-based manifest loading happens.

**Rationale**: The type system doesn't care about serialization format. JSON testing validates the serde derives. Adding the toml dependency only when needed keeps the kernel crate lightweight.

### 3. validate_manifest() as free function
**Decision**: `validate_manifest()` is a standalone function, not a method on AppManifest. It returns `Result<(), AppError>`.

**Rationale**: Validation is a stateless operation that doesn't need `&self`. As a free function it can be called before constructing an InstalledApp. It's also easier to test in isolation.

### 4. State machine via transition_to()
**Decision**: `AppManager::transition_to()` is the single entry point for all state changes. It validates transitions against the state machine (Installed->Starting->Running->Stopping->Stopped, with Failed as an absorbing state from Starting/Stopping). Invalid transitions return `AppError::InvalidState`.

**Rationale**: Centralizing state transitions prevents invalid states. The manager doesn't perform side effects (agent spawning, service starting); those are done by the caller and the manager records the result.

### 5. Clippy: matches! macro for state transitions
**Problem**: Clippy flagged the match expression for state validation as `match_like_matches_macro`.

**Decision**: Converted to `matches!(...)` with `|` pattern alternatives.

**Rationale**: Clippy compliance. The `matches!` macro is more idiomatic for boolean pattern matching.

### 6. Namespaced agent and tool IDs
**Decision**: Helper methods `namespaced_agent_ids()` and `namespaced_tool_names()` produce IDs in the form `app-name/agent-id` and `app-name/tool-name`.

**Rationale**: Prevents naming conflicts between apps and with built-in tools. The `/` separator is a natural namespace delimiter used in many package systems.

## What Was Skipped

1. **CLI integration** (`weft app install/start/stop/list/inspect/remove`) -- Requires modifying CLI files with many other pending changes.
2. **Filesystem-based install** (copying app directory, reading weftapp.toml from disk) -- Requires tokio filesystem and `toml` crate.
3. **Hook script execution** (running on_install, on_start, etc.) -- Requires async runtime and process spawning.
4. **AgentSupervisor wiring** (spawning agents from manifest) -- Requires Platform generic and boot integration.
5. **ContainerManager wiring** (starting container services) -- Requires boot integration.
6. **WasmToolRunner wiring** (loading WASM tools from manifest) -- Requires boot integration.
7. **Ruvector learning integration** (SonaEngine, FastGRNN router) -- Feature-gated, not implemented.
8. **ClawHub remote app discovery** -- Future work.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/app.rs` | ~495 | AppManifest, AppManager, lifecycle state machine, validation, 28 tests |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/lib.rs` | Added app module, doc comment, re-exports |

## Test Summary

- 28 new tests in app.rs (manifest serde, validation, install, state transitions, remove, namespacing, error display, capabilities, hooks)
- All 195 kernel tests pass
- All workspace tests pass
