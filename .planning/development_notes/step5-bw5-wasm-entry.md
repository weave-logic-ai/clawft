# BW5: WASM Entry Point + Tools Feature-Gating

**Date**: 2026-02-24
**Branch**: `feature/three-workstream-implementation`
**Status**: Complete

## Summary

BW5 wires all previous BW1-BW4 work together: feature-gates `clawft-tools` for browser WASM, adds the browser entry point to `clawft-wasm`, and ensures the entire stack compiles for `wasm32-unknown-unknown`.

## Changes

### 1. clawft-tools/Cargo.toml -- Feature-gating

- Added `native` feature that gates `dep:tokio`, `clawft-platform/native`, `clawft-core/native`, `clawft-types/native`
- Added `browser` feature that gates `clawft-platform/browser`, `clawft-core/browser`
- Changed `default` to `["native-exec", "native"]`
- Made `tokio` optional (only pulled in by `native`)
- Set `default-features = false` on workspace dependencies (`clawft-types`, `clawft-platform`, `clawft-core`)

### 2. Workspace Cargo.toml updates

- Set `default-features = false` on `clawft-core` and `clawft-tools` workspace deps
- Updated `clawft-cli/Cargo.toml` to explicitly request `features = ["native", "full"]` for `clawft-core` and `features = ["native-exec", "native"]` for `clawft-tools` (since workspace-level default-features is now false)

### 3. file_tools.rs -- Native/browser path resolution

- Replaced direct `std::fs::canonicalize()` calls with feature-gated `resolve_sandbox_path()`:
  - `#[cfg(feature = "native")]`: delegates to `std::fs::canonicalize()`
  - `#[cfg(not(feature = "native"))]`: normalizes path components without filesystem access (safe for OPFS which has no symlinks)
- Replaced `path.exists()` with feature-gated `path_exists()`:
  - `#[cfg(feature = "native")]`: delegates to `Path::exists()`
  - `#[cfg(not(feature = "native"))]`: returns true (browser relies on platform fs errors)
- Feature-gated `tokio::fs::metadata()` in `ListDirectoryTool::execute`:
  - `#[cfg(feature = "native")]`: calls `tokio::fs::metadata()` for is_dir/size
  - `#[cfg(not(feature = "native"))]`: returns `(false, 0u64)` placeholder

### 4. async_trait Send bounds

- Changed `Tool` trait in `clawft-core/src/tools/registry.rs` from `#[async_trait]` to conditional:
  ```rust
  #[cfg_attr(not(feature = "browser"), async_trait)]
  #[cfg_attr(feature = "browser", async_trait(?Send))]
  ```
- Applied same conditional pattern to all `impl Tool for ...` blocks in clawft-tools:
  - `file_tools.rs` (4 impls: ReadFile, WriteFile, EditFile, ListDirectory)
  - `web_fetch.rs` (WebFetchTool)
  - `web_search.rs` (WebSearchTool)
  - `memory_tool.rs` (MemoryReadTool, MemoryWriteTool)
  - `message_tool.rs` (MessageTool)
- Impls behind native-only features (shell_tool, spawn_tool, voice_*, delegate_tool, render_ui) were left unchanged since they never compile for browser.

### 5. clawft-wasm/Cargo.toml -- Browser feature

- Added `browser` feature that activates:
  - `dep:clawft-core`, `dep:clawft-llm`, `dep:clawft-tools`, `dep:clawft-platform` (all with `/browser` feature)
  - `dep:wasm-bindgen`, `dep:wasm-bindgen-futures`, `dep:web-sys`, `dep:js-sys`, `dep:console_error_panic_hook`
- All browser-related deps are optional and only pulled in when `browser` is enabled

### 6. clawft-wasm/src/lib.rs -- Browser entry module

- Added `#[cfg(feature = "browser")] mod browser_entry` with wasm-bindgen exports:
  - `init(config_json: &str)` -- parses Config, creates BrowserPlatform, logs to console
  - `send_message(text: &str)` -- placeholder response (to be wired to AgentLoop)
  - `set_env(key, value)` -- stub for BrowserPlatform env wiring

## Issues Encountered

### async_trait Send bound mismatch
The `Tool` trait used `#[async_trait]` which requires `Send` futures. Browser WASM Platform methods return `!Send` futures. Fixed by applying the same `cfg_attr` pattern already used by the `Platform` trait.

### Workspace default-features inheritance
Cargo's workspace dependency inheritance does not allow a member crate to set `default-features = false` if the workspace-level dep has `default-features = true` (the default). Fixed by setting `default-features = false` at the workspace level for `clawft-core` and `clawft-tools`, then having consumers (clawft-cli) explicitly request the features they need.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo test --workspace` | All tests pass (0 failures) |
| `cargo build --release --bin weft` | Native binary builds |
| `cargo check --target wasm32-unknown-unknown -p clawft-tools --no-default-features --features browser` | Compiles |
| `cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser` | Compiles |

## Files Modified

- `Cargo.toml` (workspace root) -- default-features for clawft-core, clawft-tools
- `crates/clawft-cli/Cargo.toml` -- explicit feature requests
- `crates/clawft-tools/Cargo.toml` -- native/browser features, optional tokio
- `crates/clawft-tools/src/file_tools.rs` -- resolve_sandbox_path, path_exists, tokio gate
- `crates/clawft-tools/src/web_fetch.rs` -- conditional async_trait
- `crates/clawft-tools/src/web_search.rs` -- conditional async_trait
- `crates/clawft-tools/src/memory_tool.rs` -- conditional async_trait
- `crates/clawft-tools/src/message_tool.rs` -- conditional async_trait
- `crates/clawft-core/src/tools/registry.rs` -- Tool trait conditional async_trait
- `crates/clawft-wasm/Cargo.toml` -- browser feature + deps
- `crates/clawft-wasm/src/lib.rs` -- browser_entry module

## What Remains for BW6

- **Integration testing**: End-to-end test of browser pipeline with real BrowserPlatform
- **Wire AgentLoop**: Connect `init()` to real AgentLoop with BrowserPlatform once all internal types are plumbed
- **ListDirectoryTool metadata**: Browser `(false, 0)` fallback could be improved with OPFS metadata API
- **Binary size audit**: Measure actual wasm32 binary size with `wasm-opt` and verify < 300 KB budget
- **wasm-bindgen test**: Run wasm-bindgen-test suite in headless browser
