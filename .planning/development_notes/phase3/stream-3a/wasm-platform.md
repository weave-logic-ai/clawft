# Stream 3A: WASM Platform Implementation -- Development Notes

**Status**: In Progress (Round 4 -- dep decoupling DONE; wasip2 unblocked by Rust 1.93 upgrade)
**Started**: 2026-02-17
**Depends On**: wasm-setup.md (Round 1 skeleton)
**Last Updated**: 2026-02-17 (Phase 3C -- Rust 1.93.1 upgrade unblocks wasip2)

## Objective

Implement WASI platform stubs and feature flags so that `clawft-wasm` can compile for
WASM targets with graceful degradation for unavailable native features.

**Primary target**: `wasm32-wasip2` (available after Rust 1.93.1 upgrade in Phase 3C)
**Fallback target**: `wasm32-wasip1` (backward compatibility)

## Round 2 Work Items (Complete)

| Item | Crate | Status |
|------|-------|--------|
| WasiHttpClient stub | clawft-wasm | DONE |
| WasiFileSystem stub | clawft-wasm | DONE |
| WasiEnvironment (in-memory) | clawft-wasm | DONE |
| WasmPlatform bundle struct | clawft-wasm | DONE |
| `native-exec` feature flag | clawft-tools | DONE |
| Feature flags in clawft-core | clawft-core | DONE |
| Feature flags in clawft-cli | clawft-cli | DONE |
| WASM build CI workflow | .github/workflows | DONE |

## Round 4 Work Items (In Progress)

| Item | Crate | Status |
|------|-------|--------|
| Switch target from wasip2 to wasip1 | clawft-wasm | DONE |
| Decouple from clawft-core/clawft-platform deps | clawft-wasm | DONE |
| Add dlmalloc allocator | clawft-wasm | DONE |
| WASM size profiling script | scripts | In Progress |
| Fix wasm-build.yml for wasip1 | .github/workflows | In Progress |
| `cargo check --target wasm32-wasip1` passes | clawft-wasm | DONE |

## Decisions

### WASM Target: wasip1 instead of wasip2 (Round 4) -- SUPERSEDED by Phase 3C

- **Original Decision (Round 4)**: Switch from `wasm32-wasip2` to `wasm32-wasip1`
- **Original Rationale**: `wasm32-wasip2` required Rust 1.87+ for std lib compilation. At the time, we were on Rust 1.85.1.
- **Update (Phase 3C)**: Rust has been upgraded to 1.93.1. `wasm32-wasip2` is now available and is the **primary target**. `wasm32-wasip1` is retained as a fallback for backward compatibility. The CI workflow (`wasm-build.yml`) has been updated to check both targets, with `wasip2` as primary.

### Dependency Decoupling (Round 4)

- **Decision**: Remove `clawft-core` and `clawft-platform` from `clawft-wasm` dependencies. Keep only `clawft-types`.
- **Rationale**: `clawft-platform` pulls `tokio["full"]` which does not compile for WASM targets. `clawft-core` pulls `clawft-llm` which depends on `reqwest`, also problematic for WASM. By depending only on `clawft-types`, `clawft-wasm` can compile cleanly for `wasm32-wasip1`. The WASM platform implementations are self-contained stubs that don't need the full runtime.
- **Future**: Will bridge back to the `Platform` trait when real WASI implementations are ready.

### Allocator: dlmalloc (Round 4)

- **Decision**: Use `dlmalloc` as the WASM global allocator instead of `talc`
- **Rationale**: `dlmalloc` is the standard choice for WASM targets, well-tested, and has minimal overhead. Previously planned `talc` is less mature for WASM.

### Stub Implementations

- **Decision**: WASI platform traits return explicit errors (not panics) for unimplemented operations
- **Rationale**: Allows the agent runtime to handle missing capabilities gracefully rather than crashing. The LLM can report "this tool is unavailable in WASM mode" back to the user.

### WasmPlatform Struct

- **Decision**: Bundle all WASI adapters into a single `WasmPlatform` struct
- **Components**:
  - `WasiHttpClient` -- outbound HTTP via WASI HTTP preview2
  - `WasiFileSystem` -- filesystem via WASI filesystem preview2
  - `WasiEnvironment` -- environment variables via in-memory `HashMap`
- **Rationale**: Single construction point makes it easy to pass the platform context through the pipeline

### WasiEnvironment: In-Memory HashMap

- **Decision**: Use `HashMap<String, String>` for environment variables instead of real env access
- **Rationale**: WASM has no real environment. An in-memory map is immediately usable and can be pre-populated by the host at instantiation time. This is the one component that works out of the box without any WASI host support.

### Feature Flag: `native-exec`

- **Decision**: Gate `shell_tool` and `spawn_tool` behind a `native-exec` feature flag in clawft-tools
- **Rationale**: These tools require OS process spawning which is fundamentally unavailable in WASM. Conditional compilation avoids dead code in the WASM binary and keeps the size budget tight.
- **Default**: Enabled on native targets, disabled for `wasm32-wasip1`

### Conditional Compilation Strategy

| Crate | Flag | Controls |
|-------|------|----------|
| clawft-tools | `native-exec` | shell_tool, spawn_tool registration |
| clawft-core | `native-channels` | tokio mpsc bus, channel transport |
| clawft-cli | (excluded from WASM) | entire crate is native-only |

## Architecture

```
WasmPlatform
  +-- WasiHttpClient      (stub: returns Err for now, WASI HTTP preview2 later)
  +-- WasiFileSystem       (stub: returns Err for now, WASI FS preview2 later)
  +-- WasiEnvironment      (working: in-memory HashMap)

clawft-tools (native-exec enabled)
  +-- shell_tool.rs        (compiled)
  +-- spawn_tool.rs        (compiled)

clawft-tools (native-exec disabled / WASM)
  +-- shell_tool.rs        (excluded)
  +-- spawn_tool.rs        (excluded)
```

## Blockers

- ~~`wasm32-wasip2` target availability on stable Rust (nightly may be needed)~~ **Resolved**: switched to wasip1 in R4, then fully unblocked by Rust 1.93.1 upgrade in Phase 3C
- ~~tokio/reqwest dependency chain preventing WASM compilation~~ **Resolved**: decoupled in R4 -- clawft-wasm now depends only on clawft-types, serde, serde_json
- WASI HTTP preview2 host support in wasmtime still maturing
- Full end-to-end WASM testing requires a WASI-compatible runtime in CI

## Current Compilation Status (Post Phase 3C)

- **Primary target**: `wasm32-wasip2` (available after Rust 1.93.1 upgrade)
- **Fallback target**: `wasm32-wasip1` (backward compatibility)
- **Status**: PASSING for wasip1; wasip2 verification pending (agents running)
- **clawft-types**: Compiles cleanly for both targets (no platform-specific deps)
- **clawft-wasm**: Depends only on clawft-types, serde, serde_json (+ dlmalloc on wasm32)
- **clawft-core/clawft-platform**: Excluded from WASM dependency tree
- **Native check**: `cargo check -p clawft-wasm` passes
- **Tests**: All 41 clawft-wasm tests pass on native; workspace tests unaffected

### What Changed

- Removed `clawft-core`, `clawft-platform`, `async-trait` from `clawft-wasm` dependencies
- Removed `tokio` from `clawft-wasm` dev-dependencies
- All WASI stubs (http, fs, env, platform) are now self-contained with local types
- Methods converted from async (requiring async-trait + tokio) to synchronous stubs
- Tests converted from `#[tokio::test]` to plain `#[test]`

## Next Steps

1. ~~**R4**: Complete dependency decoupling and verify `cargo check --target wasm32-wasip1 -p clawft-wasm`~~ DONE
2. ~~**R4**: Add dlmalloc allocator~~ DONE -- measure binary size pending
3. ~~**R4 (remaining)**: Fix wasm-build.yml to use wasip1 target~~ DONE
4. ~~**Phase 3C**: Upgrade to `wasm32-wasip2` when Rust 1.87+ is available~~ DONE -- Rust 1.93.1 upgrade complete, wasip2 now primary target
5. Wire `WasmPlatform` into clawft-wasm exported entry point
6. Size profiling with `twiggy` to ensure < 300 KB budget
7. Implement real `WasiHttpClient` using WASI HTTP preview2 (now unblocked by wasip2)
8. Implement real `WasiFileSystem` using WASI FS preview2 (now unblocked by wasip2)
9. Add WASM integration tests using wasmtime as test harness
