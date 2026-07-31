# Wasmtime v29 to v33 Upgrade

**Date:** 2026-04-04
**Scope:** Dependency bump to close Dependabot vulnerability alerts

## Summary

Upgraded wasmtime from v29.x to v33.x across the workspace. This closes
10 Dependabot vulnerability alerts against transitive dependencies pulled
in by wasmtime 29.

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` (root) | `wasmtime` 29 -> 33, `wasmtime-wasi` 29 -> 33 |
| `crates/clawft-wasm/Cargo.toml` | `wasmtime` 29 -> 33 (local pin) |
| `crates/clawft-kernel/src/wasm_runner/runner.rs` | Two import path migrations |

## API Migrations

wasmtime-wasi v33 reorganized its public API under the `p2` module namespace.
The following changes were required:

### 1. Pipe imports moved to `p2::pipe`

```rust
// Before (v29)
use wasmtime_wasi::pipe::{MemoryInputPipe, MemoryOutputPipe};

// After (v33)
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
```

### 2. WasiCtxBuilder moved to `p2`

```rust
// Before (v29)
wasmtime_wasi::WasiCtxBuilder::new()

// After (v33)
wasmtime_wasi::p2::WasiCtxBuilder::new()
```

### 3. No changes required

The following APIs remained stable between v29 and v33:

- `wasmtime::Config` (consume_fuel, async_support, epoch_interruption)
- `wasmtime::Engine`, `wasmtime::Store`, `wasmtime::Module`
- `wasmtime::Linker` (func_wrap, instantiate, instantiate_async)
- `wasmtime::Instance` (get_func, get_typed_func, get_export)
- `wasmtime::Func` (call, call_async)
- `wasmtime::Trap` enum (OutOfFuel variant)
- `wasmtime::ResourceLimiter` trait
- `wasmtime::StoreLimitsBuilder`
- `wasmtime::Caller`
- `wasmtime::WasmParams` / `wasmtime::WasmResults` traits
- `wasmtime_wasi::preview1::WasiP1Ctx`
- `wasmtime_wasi::preview1::add_to_linker_async`
- `WasiCtxBuilder::build_p1()` method
- `MemoryOutputPipe::contents()` method

## Affected Crates

- **clawft-kernel** (feature `wasm-sandbox`): WASM tool runner using
  wasmtime + wasmtime-wasi for sandboxed execution with WASI preview1 stdio.
- **clawft-wasm** (feature `wasm-plugins`): Plugin engine using wasmtime
  for host-function-based plugin execution (no wasmtime-wasi dependency).

## Dependabot Alerts Closed

This upgrade resolves alerts against vulnerable transitive dependencies
in the wasmtime 29.x dependency tree, including cranelift, wasmparser,
and wasm-encoder packages that had known CVEs fixed in later releases.

## Verification

- `scripts/build.sh check` passes (full workspace)
- `cargo check -p clawft-kernel --features wasm-sandbox` passes
- `cargo check -p clawft-wasm --features wasm-plugins` passes
- `scripts/build.sh test` passes (1 pre-existing failure in
  `version_output` unrelated to this change)

## Follow-up (WEFT-473)

Quarterly dependency-sweep cadence (so major bumps are not only reactive
Dependabot events) is defined in `docs/deployment/release.md` under
**Quarterly dependency sweep**. Subsequent wasmtime work: WEFT-551
(33 → 45.0.3 advisory clear).
