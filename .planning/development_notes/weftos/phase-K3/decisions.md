# Phase K3: WASM Tool Sandboxing -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. Types without Wasmtime dependency
**Problem**: Adding `wasmtime` to the workspace would significantly increase compile times and binary size. The `wasm-sandbox` feature gate should control this.

**Decision**: All types (WasmSandboxConfig, WasmToolResult, WasmValidation, WasmError, WasmTool, ToolState) are compiled unconditionally. The actual Wasmtime engine integration is behind `#[cfg(feature = "wasm-sandbox")]`. Without the feature, `load_tool()` returns `WasmError::RuntimeUnavailable`.

**Rationale**: Types are useful for configuration and error handling even without the runtime. The feature gate only controls the heavyweight Wasmtime dependency. This pattern is common in Rust crates (e.g., `reqwest` with/without `native-tls`).

### 2. Basic WASM validation without Wasmtime
**Decision**: `validate_wasm()` performs magic byte checking (\0asm header), version validation, and module size checking without requiring Wasmtime. Full structural validation (exports/imports parsing) is deferred to when the `wasm-sandbox` feature is enabled.

**Rationale**: Basic validation catches the most common errors (not a WASM file, file too large) and is useful even without the runtime. The WASM binary format's magic bytes are well-defined and trivial to check.

### 3. Duration serialization as milliseconds
**Decision**: `WasmToolResult.execution_time` uses a custom serde module (`duration_millis`) that serializes Duration as u128 milliseconds. This avoids the need for a duration serde crate.

**Rationale**: JSON has no native Duration type. Milliseconds are the most useful unit for tool execution timing. The custom module is 8 lines of code vs pulling in a dependency.

### 4. Module size limit pre-check
**Decision**: Module size is checked before any parsing or compilation. Default limit is 10 MiB, configurable via `max_module_size_bytes`.

**Rationale**: Prevents denial-of-service via oversized modules. The check is O(1) and happens before any expensive operations.

### 5. No Wasmtime dependency added to Cargo.toml yet
**Decision**: Did NOT add `wasmtime` to workspace Cargo.toml. The feature gate `wasm-sandbox` exists but only controls `#[cfg]` blocks in Rust code. Adding wasmtime will happen when the actual runtime integration is built.

**Rationale**: Adding wasmtime would change Cargo.lock and increase CI times for all builds, not just those using the feature. The types and validation are fully testable without it.

## What Was Skipped

1. **Actual Wasmtime integration** -- Engine, Linker, Store, fuel metering, memory tracking. Requires adding wasmtime dependency.
2. **ToolRegistry WASM integration** -- register_wasm_tool() not added (needs Wasmtime).
3. **SandboxEnforcer WASM variant** -- Deferred to when runtime is available.
4. **WASI filesystem stubs** -- Deferred to when runtime is available.
5. **Host function bindings** -- Deferred to when runtime is available.
6. **Ruvector integration** (rvf-wasm micro-runtime) -- Feature-gated, not implemented.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/wasm_runner.rs` | ~350 | WasmToolRunner, config, validation, types, errors |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/lib.rs` | Added wasm_runner module and re-exports |

## Test Summary

- 13 new tests in wasm_runner.rs (config, validation, error display, serde, feature-gate behavior)
- All 152 kernel tests pass
- All workspace tests pass
