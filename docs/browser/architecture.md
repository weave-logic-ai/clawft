# Browser Architecture

This document describes how clawft runs in the browser via WebAssembly and
how it differs from the native platform.

## Browser vs Native Platform

| Aspect | Native (`wasm32-wasip1` / host) | Browser (`wasm32-unknown-unknown`) |
|--------|--------------------------------|-----------------------------------|
| HTTP | `reqwest` (native TLS) | `web_sys::fetch` (browser fetch API) |
| Filesystem | `std::fs` / `tokio::fs` | In-memory `HashMap` (OPFS planned) |
| Environment | `std::env` | In-memory `HashMap` |
| Process spawning | `std::process::Command` | Not available |
| Async runtime | `tokio` (multi-threaded) | `wasm-bindgen-futures` (single-threaded) |
| Networking | Direct TCP/TLS | CORS-constrained fetch |
| Persistence | Disk files | None (OPFS / IndexedDB planned) |
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
          |     +-- BrowserFileSystem (in-memory)
          |     +-- BrowserEnvironment (in-memory)
          +-- Platform trait: async_trait(?Send)


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
                    clawft-wasm
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
```

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
