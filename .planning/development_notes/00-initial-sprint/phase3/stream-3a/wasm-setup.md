# Stream 3A: WASM Core -- Development Notes

**Status**: In Progress (Foundation)
**Started**: 2026-02-17

## Objective

Enable the clawft agent to run as a WebAssembly module targeting `wasm32-wasip2`.

## Decisions

### Foundation First

- Set up clawft-wasm crate with proper Cargo.toml, feature flags, and basic exports
- Add conditional compilation markers for native-only modules
- Do NOT attempt full WASM build yet (requires WASI preview2 runtime setup)

### Excluded Components (WASM)

- `shell_tool` -- no shell in WASM
- `spawn_tool` -- no process spawning in WASM
- `message_tool` -- requires bus which uses channels (tokio mpsc, not available in WASM)
- `clawft-channels` -- WebSocket/HTTP long-polling not available in WASI
- `clawft-cli` -- native terminal I/O

### Included Components (WASM)

- `file_tools` -- maps to WASI filesystem
- `memory_tool` -- maps to WASI filesystem
- `web_search` / `web_fetch` -- maps to WASI HTTP outbound
- `clawft-core` pipeline -- pure Rust, no OS deps
- `clawft-types` -- pure data types

## Size Budget

| Component | Estimated Size |
|-----------|---------------|
| clawft-core pipeline | ~100 KB |
| clawft-types | ~30 KB |
| Tool implementations | ~50 KB |
| serde_json | ~80 KB |
| **Total** | ~260 KB |
| **Target** | < 300 KB |

## Blockers

- WASI preview2 HTTP client implementation needed for LLM calls
- Need `wasm32-wasip2` target installed (`rustup target add wasm32-wasip2`)
- Full WASM testing requires wasmtime or similar runtime

## Next Steps

1. Implement WasiHttpClient for platform trait
2. Implement WasiFileSystem for platform trait
3. Wire pipeline into WASM exports
4. Size profiling with twiggy
