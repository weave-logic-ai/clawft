# clawft-platform Implementation Notes

**Date**: 2026-02-16
**Stream**: 1A (Types + Platform + Plugin API)
**Status**: Complete -- all quality gates pass

## Summary

Implemented the `clawft-platform` crate with 6 source files containing platform abstraction traits and native implementations. All 45 unit tests pass, 2 doc-tests pass, and clippy reports zero warnings.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib.rs` | ~143 | `Platform` trait bundle, `NativePlatform` struct, re-exports |
| `src/http.rs` | ~183 | `HttpClient` trait + `NativeHttpClient` (reqwest) |
| `src/fs.rs` | ~192 | `FileSystem` trait + `NativeFileSystem` (tokio::fs) |
| `src/env.rs` | ~87 | `Environment` trait + `NativeEnvironment` (std::env) |
| `src/process.rs` | ~133 | `ProcessSpawner` trait + `NativeProcessSpawner` (tokio::process) |
| `src/config_loader.rs` | ~249 | Config discovery algorithm + camelCase/snake_case normalization |

**Total**: ~987 lines (including tests and doc comments)

## Architecture Decisions

### Composition over Supertrait for Platform

The SPARC plan (section 1.7) proposed a supertrait-based `Platform` where `Platform: HttpClient + FileSystem + Environment`. The task spec instead uses a composition approach where `Platform` has accessor methods (`fn http() -> &dyn HttpClient`). I followed the task spec because:

1. Composition allows each sub-capability to be a separate struct with independent state (e.g., `NativeHttpClient` owns a `reqwest::Client` with connection pool).
2. It makes `ProcessSpawner` naturally optional via `fn process() -> Option<&dyn ProcessSpawner>`.
3. It avoids the need for a single struct to implement all four traits simultaneously.

### Async FileSystem

The SPARC plan (section 1.7) specified sync filesystem operations (`fn read_to_string`). The task spec specified async (`async fn read_to_string`). I followed the task spec, using `tokio::fs` for the native implementation. This is more consistent with the async-first design and avoids blocking the tokio runtime on filesystem I/O.

### Error Types

Since `clawft-types` is being implemented concurrently and does not yet export `ClawftError`, all trait methods use either `std::io::Result`, `Box<dyn Error + Send + Sync>`, or specific error types from the standard library. This avoids a circular dependency and is marked for later integration when clawft-types stabilizes.

### Environment set_var/remove_var Safety

`std::env::set_var` and `std::env::remove_var` are `unsafe` in Rust 1.85+ (edition 2024) due to potential UB in multi-threaded programs. The native implementations wrap these in `unsafe` blocks with documented safety rationale: env var mutation is expected only during initialization, matching typical usage patterns. The `Environment` trait's API makes it possible to use safe in-memory implementations for production where needed.

### Config Discovery

The discovery algorithm faithfully ports the Python `nanobot/config/loader.py` logic with one enhancement: the `~/.clawft/` path takes priority over `~/.nanobot/`, supporting the rename from nanobot to clawft while maintaining backward compatibility. The `normalize_keys` function recursively converts camelCase JSON keys to snake_case, matching the Python `convert_keys()` behavior.

## Test Coverage

- **45 unit tests** across all modules
- **2 doc-tests** (lib.rs example, camel_to_snake example)
- Filesystem tests use unique temp paths (atomic counter + PID) for test isolation
- Process tests verify stdout/stderr capture, exit codes, timeouts, and error handling
- Config loader tests cover camelCase conversion, key normalization, and discovery path logic

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo build -p clawft-platform` | PASS |
| `cargo test -p clawft-platform` | PASS (45 unit + 2 doc) |
| `cargo clippy -p clawft-platform -- -D warnings` | PASS (zero warnings) |
| All public items have doc comments | PASS |
| No unsafe blocks (except env set/remove) | PASS (documented) |

## Integration Points for Other Crates

### clawft-types (when ready)
- `config_loader::load_config_raw` returns `serde_json::Value`; the types crate will deserialize this into a typed `Config` struct.
- Error types should be unified: replace `Box<dyn Error>` with `ClawftError` once available.

### clawft-core
- Will accept `&dyn Platform` for all I/O operations.
- `config_loader::load_config_raw` is the entry point for configuration loading.

### clawft-wasm
- Will implement `Platform` trait with fetch API, WASI filesystem, in-memory env.
- `ProcessSpawner` returns `None` in WASM.

## Dependencies Used

| Crate | Version | Purpose |
|-------|---------|---------|
| async-trait | 0.1 | Async trait methods |
| tokio | 1.x | Async filesystem and process spawning |
| reqwest | 0.12 | HTTP client (rustls-tls, no default features) |
| dirs | 6.0 | Home directory discovery |
| serde | 1.x | Serialization framework |
| serde_json | 1.x | JSON parsing for config |
| tracing | 0.1 | Structured logging |
| thiserror | 2.x | Listed in Cargo.toml (available for future error types) |
