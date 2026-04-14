# ExoChain Compliance Certification -- Non-Kernel Crates

**Date**: 2026-04-04
**Auditor**: security-auditor agent
**Scope**: `clawft-core`, `clawft-graphify`, `clawft-weave` -- state-modifying operations
**Basis**: `exochain-governance-audit.md` gap list
**Architecture**: tracing-based `chain_event` system (non-kernel crates cannot depend on `ChainManager`)

## Architecture Summary

Non-kernel crates emit chain events via one of two mechanisms:

1. **`chain_event!` macro** (`clawft-core`): Defined in `clawft-core/src/chain_event.rs`. Emits `tracing::info!(target: "chain_event", ...)` with `source`, `kind`, and identifying payload fields.
2. **Direct `tracing::info!(target: "chain_event", ...)`** (`clawft-graphify`, `clawft-weave`): Same wire format, no macro dependency.

Both produce structured tracing events on target `"chain_event"` that the daemon is expected to subscribe to and forward to `ChainManager::append()`.

## Event Kind Constants

Defined in `clawft-core/src/chain_event.rs`:

| Constant | Value |
|----------|-------|
| `EVENT_KIND_SANDBOX_EXECUTE` | `sandbox.execute` |
| `EVENT_KIND_SESSION_CREATE` | `session.create` |
| `EVENT_KIND_SESSION_DESTROY` | `session.destroy` |
| `EVENT_KIND_WORKSPACE_CREATE` | `workspace.create` |
| `EVENT_KIND_WORKSPACE_CONFIG` | `workspace.config` |
| `EVENT_KIND_TOOL_REGISTER` | `tool.register` |

Defined locally in `clawft-graphify`:

| Constant | Value | File |
|----------|-------|------|
| `EVENT_KIND_GRAPHIFY_BUILD` | `graphify.build` | `build.rs` |
| `EVENT_KIND_GRAPHIFY_INGEST` | `graphify.ingest` | `ingest.rs` |
| `EVENT_KIND_GRAPHIFY_PIPELINE` | `graphify.pipeline` | `pipeline.rs` |
| `EVENT_KIND_GRAPHIFY_HOOK` | `graphify.hook` | `hooks.rs` |

## Per-File Certification

### 1. `clawft-core/src/agent/sandbox.rs:check_command` -- PASS

- **Chain event**: YES -- `chain_event!("sandbox", EVENT_KIND_SANDBOX_EXECUTE, ...)` at line 130
- **Payload**: `agent_id`, `action`, `target` (command), `allowed` (bool)
- **Governance gate**: No (acceptable -- sandbox is the enforcement mechanism itself)
- **Notes**: Event fires on both allow and deny, which is correct for audit trail. Other check methods (`check_tool`, `check_network`, `check_file_read`, `check_file_write`) do NOT emit chain events -- they rely on the in-memory audit log only. Consider extending chain events to these in a future pass.

### 2. `clawft-core/src/session.rs:get_or_create` -- PASS

- **Chain event**: YES -- `chain_event!("session", EVENT_KIND_SESSION_CREATE, { "key": key })` at line 141
- **Payload**: session key
- **Notes**: Event fires only when a new session is created (not on cache hit or disk load). Correct behavior.

### 3. `clawft-core/src/session.rs:delete_session` -- PASS

- **Chain event**: YES -- `chain_event!("session", EVENT_KIND_SESSION_DESTROY, { "key": key })` at line 374
- **Payload**: session key
- **Notes**: Fires after disk deletion and cache invalidation.

### 4. `clawft-core/src/workspace/mod.rs:create` -- PASS

- **Chain event**: YES -- `chain_event!("workspace", EVENT_KIND_WORKSPACE_CREATE, { "name": name, "path": ws_root.display() })` at line 181
- **Payload**: workspace name and path
- **Notes**: Fires after directory creation and registry persistence.

### 5. `clawft-core/src/workspace/config.rs:load_merged_config_from` -- PASS

- **Chain event**: YES -- `chain_event!("workspace", EVENT_KIND_WORKSPACE_CONFIG, { ... })` at line 74
- **Payload**: global config path, workspace path
- **Notes**: This logs config load/merge, not a mutation per se, but captures the configuration surface. Sufficient for audit purposes.

### 6. `clawft-core/src/tools/registry.rs:register` -- PASS

- **Chain event**: YES -- `chain_event!("tools", EVENT_KIND_TOOL_REGISTER, { "tool_name": name })` at line 368
- **Payload**: tool name
- **Notes**: `register_with_metadata` (line 377) does NOT emit a chain event. This is a gap -- tools registered via MCP metadata path skip chain logging.

### 7. `clawft-graphify/src/build.rs:build` -- PASS

- **Chain event**: YES -- `tracing::info!(target: "chain_event", ...)` at line 47
- **Payload**: source="graphify", kind="graphify.build", entity_count, relationship_count, files_processed
- **Notes**: Fires after graph construction completes.

### 8. `clawft-graphify/src/ingest.rs:ingest` -- PASS

- **Chain event**: YES -- `tracing::info!(target: "chain_event", ...)` at line 376
- **Payload**: source="graphify", kind="graphify.ingest", url, url_type, filename
- **Notes**: Fires after successful ingestion. Failed ingestions return `Err` before reaching the event.

### 9. `clawft-graphify/src/pipeline.rs:run_from_extractions` -- PASS

- **Chain event**: YES -- `tracing::info!(target: "chain_event", ...)` at line 175
- **Payload**: source="graphify", kind="graphify.pipeline", entity_count, relationship_count, files_processed, has_analysis
- **Notes**: The `run()` method (file-based) returns `Err` before any chain event, which is acceptable since it's a stub.

### 10. `clawft-graphify/src/hooks.rs:install_hooks` -- PASS

- **Chain event**: YES -- `tracing::info!(target: "chain_event", ...)` at line 154
- **Payload**: source="graphify", kind="graphify.hook", repo_root, action="install"

### 11. `clawft-graphify/src/hooks.rs:uninstall_hooks` -- PASS

- **Chain event**: YES -- `tracing::info!(target: "chain_event", ...)` at line 178
- **Payload**: source="graphify", kind="graphify.hook", repo_root, action="uninstall"

### 12. `clawft-weave/src/commands/init_cmd.rs:run` -- PASS

- **Chain event**: YES -- `tracing::info!(target: "chain_event", ...)` at line 32
- **Payload**: source="weave", kind="project.init", force, skills_only, analyze
- **Notes**: Fires before delegation to shell script.

## Summary

| # | File:Method | Chain Event | Status |
|---|-------------|-------------|--------|
| 1 | `sandbox.rs:check_command` | `sandbox.execute` | **PASS** |
| 2 | `session.rs:get_or_create` | `session.create` | **PASS** |
| 3 | `session.rs:delete_session` | `session.destroy` | **PASS** |
| 4 | `workspace/mod.rs:create` | `workspace.create` | **PASS** |
| 5 | `workspace/config.rs:load_merged_config_from` | `workspace.config` | **PASS** |
| 6 | `tools/registry.rs:register` | `tool.register` | **PASS** |
| 7 | `build.rs:build` | `graphify.build` | **PASS** |
| 8 | `ingest.rs:ingest` | `graphify.ingest` | **PASS** |
| 9 | `pipeline.rs:run_from_extractions` | `graphify.pipeline` | **PASS** |
| 10 | `hooks.rs:install_hooks` | `graphify.hook` | **PASS** |
| 11 | `hooks.rs:uninstall_hooks` | `graphify.hook` | **PASS** |
| 12 | `init_cmd.rs:run` | `project.init` | **PASS** |

**Result: 12/12 PASS -- all audited operations emit chain events.**

## Tracing-to-ChainManager Bridge Status

**STATUS: MISSING**

The `chain_event.rs` module documents the intended architecture:

> The daemon layer (`clawft-weave`) subscribes to the `chain_event` target and forwards matching spans to the real ExoChain via `ChainManager::append()`.

However, inspection of `clawft-weave/src/main.rs` (lines 106-111) shows that the daemon uses a standard `tracing_subscriber::fmt()` subscriber with no custom layer that filters on `target: "chain_event"` and forwards to `ChainManager`.

This means:

1. All 12 chain events are **emitted** correctly.
2. None of them are **captured and forwarded** to the ExoChain.
3. The events currently appear only in stdout/stderr log output (if the log level includes INFO).
4. No persistent, tamper-evident chain record is created from non-kernel crate operations.

### What is needed

A custom `tracing::Layer` implementation that:

1. Filters events where `target == "chain_event"`.
2. Extracts `source`, `kind`, and payload fields from the event metadata.
3. Calls `ChainManager::append(source, kind, payload)` for each matching event.
4. Is composed into the subscriber stack in `main.rs` alongside the existing `fmt` layer.

Example skeleton:

```rust
struct ChainEventLayer {
    chain: Arc<ChainManager>,
}

impl<S: Subscriber> Layer<S> for ChainEventLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() == "chain_event" {
            // Extract fields and forward to self.chain.append(...)
        }
    }
}
```

This bridge is the final link required to make non-kernel chain events actually persist in the ExoChain.

## Minor Gaps (Non-Blocking)

1. **`ToolRegistry::register_with_metadata`** does not emit a chain event, unlike `register`. MCP-registered tools skip chain logging.
2. **`sandbox.rs` check methods** other than `check_command` (`check_tool`, `check_network`, `check_file_read`, `check_file_write`) do not emit chain events. They use the in-memory audit log only.
3. **`save_query_result`** in `ingest.rs` (feedback loop storage) does not emit a chain event.

These are enhancement opportunities, not certification failures, since the original audit did not identify them as required coverage points.

## Certification

All 10 operations identified in the audit request are certified as having correct chain event emission. The tracing-based mechanism is architecturally sound and consistently implemented across all three non-kernel crates. The blocking gap is the missing tracing-to-ChainManager bridge layer in the daemon, without which these events are logged but not persisted to the ExoChain.
