# Browser Architecture

This document describes how clawft runs in the browser via WebAssembly and
how it differs from the native platform.

## Browser vs Native Platform

| Aspect | Native (`wasm32-wasip1` / host) | Browser (`wasm32-unknown-unknown`) |
|--------|--------------------------------|-----------------------------------|
| HTTP | `reqwest` (native TLS) | `web_sys::fetch` (browser fetch API) |
| Filesystem | `std::fs` / `tokio::fs` | In-memory `HashMap`, or OPFS with `--features browser-opfs` (WEFT-13 / WEFT-392) |
| Environment | `std::env` | In-memory `HashMap`, or OPFS snapshot at `/clawft/.clawft/env.json` with `browser-opfs` (WEFT-14) |
| Process spawning | `std::process::Command` | Not available |
| Async runtime | `tokio` (multi-threaded) | `wasm-bindgen-futures` (single-threaded) |
| Networking | Direct TCP/TLS | CORS-constrained fetch |
| Persistence | Disk files | OPFS when `browser-opfs` enabled (FS + env); else session-only memory |
| Binary format | Native ELF/Mach-O/PE | `.wasm` loaded by browser |
| Entry point | `fn main()` in `clawft-cli` | `init()` / `send_message()` via wasm-bindgen |
| Size | ~20 MB (release, stripped) | < 300 KB target (wasm-opt) |

## Data Flow

```
Browser JS                 WASM Module                  External
----------                 -----------                  --------

  User types
  message
     |
     v
  main.js
  send_message(text) -----> browser_entry::send_message()
                               |
                               v
                            AgentLoop::step()
                               |
                               v
                            Pipeline (6 stages)
                               |
                               v
                            LLM Transport
                               |
                               v
                            BrowserHttpClient::request()
                               |
                               v
                            web_sys::fetch() -----------> LLM API
                               |                         (Anthropic,
                               |                          OpenAI, etc.)
                               |<--------------------------+
                               v
                            Parse response
                               |
                               v
                            Return String
     |<------------------------+
     v
  Display in
  chat UI
```

## Feature Flag Architecture

The crate tree uses Cargo features to split native and browser code paths.
When `--features browser` is specified, only browser-compatible code is
compiled.

```
Feature: "browser"
    |
    +-- clawft-wasm (entry point)
    |     |
    |     +-- browser_entry module (#[cfg(feature = "browser")])
    |         - init(), send_message(), set_env()
    |         - wasm-bindgen exports
    |
    +-- clawft-core/browser
    |     |
    |     +-- Tool trait: async_trait(?Send)
    |     +-- AgentLoop: !Send futures
    |
    +-- clawft-llm/browser
    |     |
    |     +-- LLM transport using platform HTTP
    |
    +-- clawft-tools/browser
    |     |
    |     +-- file_tools: browser path resolution (no canonicalize)
    |     +-- Tool impls: async_trait(?Send)
    |     +-- shell_tool, spawn_tool: excluded (native-only)
    |
    +-- clawft-platform/browser
          |
          +-- BrowserPlatform
          |     +-- BrowserHttpClient (web_sys::fetch)
          |     +-- BrowserFileSystem (in-memory default)
          |     +-- BrowserEnvironment (in-memory default)
          +-- Platform trait: async_trait(?Send)

Feature: "browser-opfs" (optional sub-feature of browser; WEFT-13 / WEFT-14 / WEFT-392)
    |
    +-- clawft-platform/browser-opfs
    |     +-- BrowserFileSystem::open() → OPFS via navigator.storage.getDirectory()
    |     +-- BrowserEnvironment::open() → same OPFS, snapshot at /clawft/.clawft/env.json
    |     +-- Runtime fallback to in-memory if OPFS API missing / non-secure context
    |     +-- Virtual home: /clawft  (workspace: /clawft/workspace, config: /clawft/.clawft/…)
    +-- clawft-tools/browser-opfs  (propagates platform feature)
    +-- clawft-wasm/browser-opfs   (init() uses env open_with_seed + with_env_arc_open)

### web-sys OPFS requirements (WEFT-13 / WEFT-392)

OPFS bindings are **stable** in web-sys (no `web_sys_unstable_apis` cfg):

| Requirement | Value |
|-------------|--------|
| Minimum web-sys | **0.3.70** (FileSystemDirectoryHandle / StorageManager::getDirectory) |
| Verified in tree | **0.3.85** (Cargo.lock) |
| Feature flags | `StorageManager`, `Navigator`, `WorkerNavigator`, `FileSystemHandle`, `FileSystemHandleKind`, `FileSystemDirectoryHandle`, `FileSystemFileHandle`, `FileSystemGetDirectoryOptions`, `FileSystemGetFileOptions`, `FileSystemWritableFileStream`, `FileSystemCreateWritableOptions`, `File`, `Blob`, `WritableStream` |
| Runtime | Secure context (HTTPS or localhost); `navigator.storage.getDirectory()` |

Enable with:

```bash
cargo build -p clawft-wasm --target wasm32-unknown-unknown \
  --no-default-features --features browser-opfs
```


Feature: "native" (default for clawft-cli)
    |
    +-- clawft-cli (entry point)
    |     - fn main(), tokio runtime
    |
    +-- clawft-core/native
    |     +-- Tool trait: async_trait (Send)
    |
    +-- clawft-tools/native
    |     +-- All tools including shell, spawn
    |     +-- tokio::fs for file operations
    |
    +-- clawft-platform (native)
          +-- NativePlatform
                +-- reqwest HTTP
                +-- tokio::fs filesystem
                +-- std::env environment
                +-- std::process spawner
```

## Crate Dependency Graph

```
                    clawft-wasm          (browser / wasip2 entry)
                   /     |     \
                  /      |      \
     clawft-core    clawft-llm   clawft-tools
          |              |            |
          +---------+----+-----+------+
                    |          |
              clawft-types   clawft-platform
                    |          |
                    +----+-----+
                         |
                  [serde, serde_json]


   clawft-wasm-host   (WEFT-398 — native wasmtime plugin host ONLY)
          |
     clawft-plugin
          |
   [wasmtime, sandbox, engine, audit, permission_store]
```

`clawft-wasm-host` is **not** on the browser graph. Historical
`--features wasm-plugins` on `clawft-wasm` re-exports its modules for
API compatibility; new host code should depend on `clawft-wasm-host`
directly. See [`docs/browser/crate-split.md`](crate-split.md).

When the `browser` feature is enabled:

- `clawft-wasm` depends on all four mid-layer crates with their `/browser` features.
- `clawft-platform` compiles `BrowserPlatform` instead of `NativePlatform`.
- `clawft-tools` excludes `shell_tool`, `spawn_tool`, and other native-only tools.
- `clawft-core` uses `async_trait(?Send)` for `!Send` futures.
- `wasm-bindgen`, `web-sys`, `js-sys`, and `console_error_panic_hook` are pulled in.

When the `browser` feature is disabled (default):

- `clawft-wasm` only exposes the WASI-oriented `WasmPlatform` and stubs.
- No `wasm-bindgen` or browser dependencies are compiled.

## Send Bound Differences

Browser WASM is single-threaded. Futures do not need to be `Send`. The codebase
uses conditional compilation to relax `Send` bounds:

```rust
// In clawft-core's Tool trait:
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
pub trait Tool: Send + Sync { ... }
```

This same pattern is applied to the `Platform` trait and all `impl Tool for ...`
blocks in `clawft-tools`.

## Security: API keys in JS-readable memory

Provider keys injected via `init(config_json)` live in WASM linear memory that
is readable from same-origin JavaScript. `SecretString` redacts logs only; it
does not isolate secrets from XSS. For the full threat model, mitigations
(backend proxy, short-lived tokens, CSP, never embed secrets), and a production
checklist, see **[security.md](./security.md)** (WEFT-406).
