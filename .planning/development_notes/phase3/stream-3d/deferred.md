# Phase 3D: Deferred to Phase 4

**Date**: 2026-02-17
**Decision**: DEFERRED

## Reason

Phase 3D (WASI HTTP/FS + Docker Multi-Arch) has been deferred to Phase 4 based on consensus review:

1. **wasi-http-client maturity**: The crate ecosystem for WASI preview2 HTTP is not stable enough for production use
2. **Rust 1.93 compatibility**: WASI preview2 HTTP bindings have not been validated against Rust 1.93
3. **WASM size impact**: Real HTTP/FS implementations may push beyond the 300KB size budget
4. **bin vs reactor component model**: The command-line one-shot model needs clarification before building real HTTP/FS

## What Remains

The following P1 items are tracked for Phase 4 implementation:
- P1 #8: Fix path traversal in `resolve()` -- reject `..` and absolute paths
- P1 #9: Define `WasmHttpError` enum -- replace `Box<dyn Error>`
- P1 #10: Pin wasmtime version in CI
- P1 #11: Add tempfile dev-dependency
- P1 #12: Clarify bin vs reactor component model

## Current State

- `clawft-wasm` compiles to `wasm32-wasip2` with stubs (57.9 KB raw / 24.3 KB gzipped)
- HTTP, FS modules are stub implementations returning "not implemented" errors
- Phase 3E optimization proceeds against this stub binary
