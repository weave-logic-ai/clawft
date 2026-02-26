# BW4: Browser Platform Implementation

**Date**: 2026-02-24
**Branch**: `feature/three-workstream-implementation`
**Status**: Complete

## Summary

Implemented the full `Platform` trait for browser/WASM targets via `BrowserPlatform`, which bundles three sub-implementations:

- `BrowserHttpClient` -- HTTP via the web-sys fetch API
- `BrowserFileSystem` -- In-memory filesystem (OPFS deferred to future iteration)
- `BrowserEnvironment` -- In-memory key-value environment variable store

Process spawning returns `None` since WASM cannot spawn OS processes.

## Files Created

| File | Purpose |
|------|---------|
| `crates/clawft-platform/src/browser/mod.rs` | `BrowserPlatform` struct + `Platform` impl |
| `crates/clawft-platform/src/browser/http.rs` | `BrowserHttpClient` using web-sys fetch |
| `crates/clawft-platform/src/browser/fs.rs` | `BrowserFileSystem` using in-memory HashMap |
| `crates/clawft-platform/src/browser/env.rs` | `BrowserEnvironment` using in-memory HashMap |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-platform/src/lib.rs` | Added `#[cfg(feature = "browser")] pub mod browser` and re-export |

## Design Decisions

### 1. `Mutex<HashMap>` over `RefCell<HashMap>`

The `Environment` and `FileSystem` traits have unconditional `Send + Sync` bounds. `RefCell` is not `Sync`, so we use `std::sync::Mutex` instead. In WASM's single-threaded runtime this has zero contention overhead and avoids `unsafe impl Sync`.

### 2. In-Memory FileSystem over OPFS

The Origin Private File System (OPFS) web-sys bindings require additional feature flags (`FileSystemDirectoryHandle`, `FileSystemFileHandle`, etc.) that are marked as unstable in web-sys. The in-memory implementation is sufficient for the MVP/stub phase and can be swapped for OPFS later without changing the public API.

### 3. Fetch Dispatch Strategy

`BrowserHttpClient` tries `WorkerGlobalScope::fetch` first (for use in web workers), then falls back to `Window::fetch` (main thread). This covers both execution contexts without requiring the caller to specify.

### 4. `async_trait(?Send)` for Browser

All async traits use `#[async_trait(?Send)]` when the `browser` feature is active. This is critical because WASM futures are not `Send` (single-threaded runtime, JS interop values are `!Send`).

## Verification

- `cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser` -- PASS (0 warnings)
- `cargo test -p clawft-platform` -- 46 passed, 0 failed, 2 doc-tests passed
- `cargo test --workspace` -- all tests pass, zero regressions
- `cargo build --release --bin weft` -- native binary builds successfully
