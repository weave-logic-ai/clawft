# Architectural Analysis: Browser/WASM Portability for clawft

**Date**: 2026-02-24
**Status**: Analysis Complete
**Scope**: Full 18-crate workspace at `/home/aepod/dev/clawft`

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Crate Dependency Graph](#2-crate-dependency-graph)
3. [WASM Compilation Assessment](#3-wasm-compilation-assessment)
4. [Platform Trait Analysis](#4-platform-trait-analysis)
5. [Component Portability Deep-Dive](#5-component-portability-deep-dive)
6. [Required Browser Platform Implementations](#6-required-browser-platform-implementations)
7. [Recommended Crate Split and Feature-Flag Strategy](#7-recommended-crate-split-and-feature-flag-strategy)
8. [Risk Areas and Blockers](#8-risk-areas-and-blockers)
9. [Recommended Approach](#9-recommended-approach)
10. [Migration Roadmap](#10-migration-roadmap)
11. [Appendix: Per-Crate Dependency Audit](#appendix-per-crate-dependency-audit)

---

## 1. Executive Summary

The clawft workspace has strong architectural foundations for WASM portability. The `Platform` trait abstraction in `clawft-platform` already defines the right seams, and the pipeline system is almost entirely pure computation. However, there is a critical structural gap: the existing `clawft-wasm` crate is **fully decoupled** from `clawft-core` and `clawft-platform`, meaning it cannot run the real agent loop or pipeline. It duplicates concepts instead of implementing the shared trait interfaces.

The recommended approach is a **hybrid architecture** that makes `clawft-platform` compilable for `wasm32-unknown-unknown` via feature flags, provides browser-native implementations of the platform traits (fetch API, IndexedDB/OPFS, in-memory env), and gates native-only subsystems (skill watcher, channels, CLI) behind features. This unlocks running the real `clawft-core` agent loop and pipeline in-browser without maintaining a parallel WASM-only codebase.

**Estimated effort**: 4-6 weeks for a functional browser prototype with the real pipeline running, based on the fact that approximately 60% of the codebase is already portable or nearly portable.

---

## 2. Crate Dependency Graph

### 2.1 Workspace Members (18 crates)

```
clawft-types          (foundation -- no platform deps)
clawft-platform       (trait definitions + native impls)
clawft-plugin         (trait definitions)
clawft-security       (pure computation)
clawft-core           (agent loop, pipeline, tools, sessions)
clawft-llm            (LLM provider abstraction)
clawft-tools          (tool implementations)
clawft-channels       (Discord, Slack, Telegram, etc.)
clawft-services       (cron, heartbeat, MCP, delegation)
clawft-cli            (binary entry point)
clawft-wasm           (WASM entry point -- currently decoupled)
clawft-plugin-git
clawft-plugin-cargo
clawft-plugin-oauth2
clawft-plugin-treesitter
clawft-plugin-browser (CDP automation -- not in-browser)
clawft-plugin-calendar
clawft-plugin-containers
```

### 2.2 Core Dependency Chain

```
clawft-types
    |
    v
clawft-platform  <-- clawft-plugin
    |                    |
    v                    v
clawft-llm         clawft-core
    |               /    |    \
    +------->------+     |     \
                         v      v
                  clawft-tools  clawft-services
                         |           |
                         v           v
                     clawft-channels
                         |
                         v
                      clawft-cli
```

### 2.3 Minimal Browser Subset

The minimum crate set needed for a functional browser agent:

```
clawft-types       -- shared types (WASM-ready with one fix)
clawft-platform    -- trait definitions (needs feature-gating of native impls)
clawft-plugin      -- plugin traits (nearly WASM-ready)
clawft-security    -- already WASM-compatible
clawft-core        -- agent loop + pipeline (needs 3 deps gated)
clawft-llm         -- LLM transport (needs reqwest -> fetch swap)
clawft-tools       -- browser-safe subset (file tools via OPFS, web tools via fetch)
```

Crates that are **excluded** from the browser target:

```
clawft-channels         -- WebSocket/REST channel adapters (server-side)
clawft-services         -- cron, heartbeat, MCP server (server-side)
clawft-cli              -- native binary
clawft-plugin-*         -- all plugin crates (native tooling)
```

---

## 3. WASM Compilation Assessment

### 3.1 Compilation Target

The existing `clawft-wasm` targets `wasm32-wasip1` (WASI Preview 1). For browser deployment, the target should be `wasm32-unknown-unknown` with `wasm-bindgen` for JavaScript interop. These are fundamentally different targets:

| Aspect | wasm32-wasip1 | wasm32-unknown-unknown |
|--------|--------------|----------------------|
| Runtime | WASI host (wasmtime, etc.) | Browser/JS engine |
| I/O | WASI syscalls | JavaScript FFI via wasm-bindgen |
| Async | Needs WASI async proposals | wasm-bindgen-futures + JS promises |
| HTTP | WASI HTTP proposal | fetch API via web-sys/gloo |
| FS | WASI filesystem | IndexedDB / OPFS via web-sys |
| Use case | Edge/server WASM | Browser UI |

Both targets should be supported but require different platform implementations.

### 3.2 Per-Crate WASM Compilation Status

| Crate | Compiles to WASM? | Blocker(s) | Effort to Fix |
|-------|-------------------|------------|---------------|
| `clawft-types` | Almost | `dirs` crate (home dir) | Low -- feature-gate `dirs` |
| `clawft-platform` | No | `tokio`, `reqwest`, `dirs` | Medium -- split traits from impls |
| `clawft-plugin` | Almost | `tokio-util` (CancellationToken) | Low -- feature-gate or replace |
| `clawft-security` | Yes | None | None |
| `clawft-core` | No | `tokio`, `notify`, `dirs`, `futures-util` | High -- feature-gate native deps |
| `clawft-llm` | No | `reqwest`, `tokio` | Medium -- swap HTTP transport |
| `clawft-tools` | No | `tokio`, via `clawft-platform`/`clawft-core` | Medium -- follows platform fixes |
| `clawft-channels` | No | `tokio-tungstenite`, `reqwest`, `hmac` | Not needed for browser |
| `clawft-services` | No | `tokio`, `reqwest`, `cron`, `dirs` | Not needed for browser |
| `clawft-cli` | No | `clap`, terminal I/O | Not applicable |
| `clawft-wasm` | Yes | N/A (already compiles) | N/A |

---

## 4. Platform Trait Analysis

### 4.1 Current Architecture

`clawft-platform/src/lib.rs` defines the core `Platform` trait:

```rust
#[async_trait]
pub trait Platform: Send + Sync {
    fn http(&self) -> &dyn http::HttpClient;
    fn fs(&self) -> &dyn fs::FileSystem;
    fn env(&self) -> &dyn env::Environment;
    fn process(&self) -> Option<&dyn process::ProcessSpawner>;
}
```

Sub-traits:
- **`HttpClient`** -- async `request`, `get`, `post` methods
- **`FileSystem`** -- async `read_to_string`, `write_string`, `exists`, `list_dir`, `create_dir_all`, `remove_file`, `home_dir`
- **`Environment`** -- sync `get_var`, `set_var`, `remove_var`
- **`ProcessSpawner`** -- async `run` (already `Option` in Platform)

### 4.2 The Disconnection Problem

The `clawft-wasm` crate defines its own `WasmPlatform`:

```rust
// clawft-wasm/src/platform.rs
pub struct WasmPlatform {
    http: WasiHttpClient,
    fs: WasiFileSystem,
    env: WasiEnvironment,
}
```

This struct has methods with the **same names** as the Platform trait (`http()`, `fs()`, `env()`, `process()`) but does **not** implement the `Platform` trait. Key differences:

1. **No `async_trait`**: The WASM implementations are synchronous stubs.
2. **Different return types**: Returns concrete types, not `&dyn` trait objects.
3. **No trait bound satisfaction**: `clawft-core::AgentLoop<P: Platform>` cannot accept `WasmPlatform`.

This means the entire `clawft-core` pipeline (classifier, router, assembler, transport, scorer, learner) and agent loop cannot be used from `clawft-wasm`. The WASM crate currently returns stub responses instead of running the real pipeline.

### 4.3 What Needs to Change

The `clawft-platform` crate needs to be split so that:
- **Trait definitions** compile to WASM (no native dependencies).
- **Native implementations** (`NativePlatform`) are behind a feature flag.
- **Browser implementations** (`BrowserPlatform`) live either in `clawft-platform` behind a `browser` feature or in a new `clawft-platform-browser` crate.

---

## 5. Component Portability Deep-Dive

### 5.1 Agent Loop (`clawft-core/src/agent/loop_core.rs`)

**Status**: Not portable as-is. Contains the most critical native dependencies.

**Hard native dependencies**:
- `tokio::select!` macro -- requires tokio runtime
- `tokio_util::sync::CancellationToken` -- requires tokio-util
- `tokio::sync::mpsc` channels -- requires tokio
- `futures_util::future::join_all` -- portable in principle but currently pulled via tokio ecosystem

**Portable aspects**:
- Generic over `P: Platform` -- correct abstraction boundary
- Tool execution via `ToolRegistry` -- uses Platform trait
- Session persistence via `SessionManager<P>` -- uses Platform fs
- Post-write verification -- uses `platform.fs()`
- All business logic (permission checks, tool dispatch, context building) is pure

**Browser replacement strategy**:
- Replace `tokio::select!` with `futures::select!` from the `futures` crate (works with any executor)
- Replace `CancellationToken` with `futures::channel::oneshot` or a custom `AbortHandle`
- Replace tokio channels with `futures::channel::mpsc`
- Use `wasm-bindgen-futures` as the executor in browser
- Alternatively: use `tokio` with the `wasm32` target feature (tokio partially supports WASM but only with `rt` feature, not `full`)

### 5.2 Pipeline System

**Status**: Almost entirely portable. This is the strongest part of the architecture.

| Component | File | Portable? | Notes |
|-----------|------|-----------|-------|
| `TaskClassifier` (KeywordClassifier) | `pipeline/classifier.rs` | Yes | Pure computation, no I/O |
| `ModelRouter` (StaticRouter) | `pipeline/router.rs` | Yes | Pure computation, uses async_trait |
| `TieredRouter` | `pipeline/tiered_router.rs` | Yes | Uses `AtomicUsize`, `SystemTime` -- both WASM-safe |
| `PipelineRegistry` | `pipeline/traits.rs` | Yes | Uses `std::time::Instant` -- WASM-safe |
| `ContextAssembler` | trait only | Yes | Trait definition, impl depends on caller |
| `LlmTransport` | trait only | Depends | Trait is portable; impl needs browser HTTP |
| `QualityScorer` | trait only | Yes | Synchronous scoring |
| `LearningBackend` | trait only | Yes | Synchronous learning |

The pipeline is the crown jewel for portability. Every stage is either pure computation or behind a trait that can be implemented for browser.

### 5.3 Config and Workspace (`clawft-platform/src/config_loader.rs`)

**Status**: Partially portable with known leaks.

**Issues**:
1. `discover_config_path()` uses `std::path::Path::exists()` directly instead of the Platform `fs` trait. This is a portability leak.
2. Config discovery checks `~/.clawft/config.json` and `~/.nanobot/config.json` -- these paths are meaningless in browser.
3. `dirs::home_dir()` is used for default paths -- `dirs` does not compile for WASM.

**Browser strategy**:
- Config comes from JavaScript (passed via `wasm-bindgen` at initialization)
- Or stored in IndexedDB/localStorage and loaded via the browser `FileSystem` implementation
- `discover_config_path` needs a browser-aware override

### 5.4 Tool System (`clawft-tools`)

**Status**: Well-structured with existing feature gates, but has a native dependency leak.

| Tool | Portable? | Notes |
|------|-----------|-------|
| `ReadFileTool` | Yes (via Platform) | Uses `platform.fs()` |
| `WriteFileTool` | Yes (via Platform) | Uses `platform.fs()` |
| `EditFileTool` | Yes (via Platform) | Uses `platform.fs()` |
| `ListDirectoryTool` | **Leak** | Line 374: `tokio::fs::metadata()` bypasses Platform |
| `ShellExecTool` | No (gated) | Correctly behind `native-exec` feature |
| `SpawnTool` | No (gated) | Correctly behind `native-exec` feature, checks `platform.process()` |
| `WebSearchTool` | Yes (via Platform) | Uses `platform.http()` |
| `WebFetchTool` | Yes (via Platform) | Uses `platform.http()` |
| `MemoryReadTool` | Yes (via Platform) | Uses `platform.fs()` |
| `MemoryWriteTool` | Yes (via Platform) | Uses `platform.fs()` |
| `MessageTool` | Yes | Uses `MessageBus` (in-memory) |
| `DelegateTaskTool` | No (gated) | Behind `delegate` feature |

**Known leak in `file_tools.rs` line 374**:
```rust
let metadata = tokio::fs::metadata(entry_path).await;
```
This call bypasses the Platform `fs` trait and directly uses tokio. It must be replaced with a platform-aware metadata call or the `FileSystem` trait must gain a `metadata()` method.

### 5.5 Channel System (`clawft-channels`)

**Status**: Not portable and not needed for browser.

All channel implementations (Discord, Slack, Telegram) are server-side adapters that:
- Open persistent WebSocket connections via `tokio-tungstenite`
- Make REST API calls via `reqwest`
- Verify webhook signatures via `hmac`/`sha2`
- Run background polling loops via `tokio::spawn`

A "browser channel" would be conceptually different -- it would be the browser UI itself acting as the channel, passing messages to the agent loop via JavaScript/WASM bridge. This does not fit the existing `Channel`/`ChannelHost` trait model and should be a separate, simpler interface:

```rust
// Conceptual browser channel interface (via wasm-bindgen)
#[wasm_bindgen]
pub fn send_message(input: &str) -> Promise;

#[wasm_bindgen]
pub fn on_response(callback: &js_sys::Function);
```

### 5.6 Skill Watcher (`clawft-core/src/agent/skill_watcher.rs`)

**Status**: Not portable. Uses `notify` crate for OS filesystem events.

This is a hard native dependency in `clawft-core/Cargo.toml` (not feature-gated). It uses:
- `notify::RecommendedWatcher` -- OS-level inotify/FSEvents/ReadDirectoryChanges
- `tokio::spawn` for the event loop
- `tokio::sync::mpsc` for event passing

**Browser alternative**: Skills would be loaded at initialization from bundled data or fetched via HTTP. Hot-reload could be replaced with a manual refresh or a WebSocket push from a development server.

---

## 6. Required Browser Platform Implementations

### 6.1 BrowserHttpClient

Implements `HttpClient` trait using the browser `fetch()` API.

```
Technology: web-sys fetch API or gloo-net
Async model: Returns JS Promise wrapped via wasm-bindgen-futures
Headers: Mapped from HashMap to web_sys::Headers
Body: Response read as text via .text() promise
CORS: Must be handled (may need proxy for some LLM APIs)
```

Existing crate options: `gloo-net`, `reqwest` with `wasm` feature (reqwest does support `wasm32-unknown-unknown` via `web-sys` -- however the workspace currently pins `rustls-tls` which is incompatible).

### 6.2 BrowserFileSystem

Implements `FileSystem` trait using browser storage.

```
Technology: Origin Private File System (OPFS) via web-sys, or IndexedDB
Async model: All operations return JS Promises wrapped via wasm-bindgen-futures
Paths: Virtual filesystem rooted at OPFS root
home_dir(): Returns a virtual root like "/user" or similar convention
Limitations: No symlinks, no permissions, size quotas apply
```

For a minimal viable implementation, an in-memory filesystem (HashMap-based) may be sufficient for the agent's working state, with IndexedDB for persistence across sessions.

### 6.3 BrowserEnvironment

Implements `Environment` trait. This is already nearly solved:

```
Technology: In-memory HashMap (same as WasiEnvironment)
Initialization: Populated from JavaScript at startup
Persistence: Optional -- could store to localStorage
```

The existing `WasiEnvironment` from `clawft-wasm/src/env.rs` is a correct implementation. It just needs to implement the `Environment` trait from `clawft-platform`.

### 6.4 ProcessSpawner

Not applicable in browser. The Platform trait already returns `Option<&dyn ProcessSpawner>`, and browser implementations return `None`. The agent loop and tools already handle this case.

---

## 7. Recommended Crate Split and Feature-Flag Strategy

### 7.1 Feature Flag Design

Add a `native` feature to crates that bundle native implementations, defaulting to enabled:

**`clawft-platform/Cargo.toml`** (proposed):
```toml
[features]
default = ["native"]
native = ["dep:tokio", "dep:reqwest", "dep:dirs"]
browser = ["dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:web-sys", "dep:js-sys"]

[dependencies]
# Always available (trait definitions)
async-trait = { workspace = true }
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# Native only
tokio = { workspace = true, optional = true }
reqwest = { workspace = true, optional = true }
dirs = { workspace = true, optional = true }

# Browser only
wasm-bindgen = { version = "0.2", optional = true }
wasm-bindgen-futures = { version = "0.4", optional = true }
web-sys = { version = "0.3", optional = true, features = [...] }
js-sys = { version = "0.3", optional = true }
```

**`clawft-core/Cargo.toml`** (proposed):
```toml
[features]
default = ["full", "native"]
full = []
native = ["dep:notify", "dep:dirs", "tokio/full"]
browser = ["clawft-platform/browser"]
vector-memory = ["dep:rand", "dep:instant-distance"]
# ... existing features
```

**`clawft-llm/Cargo.toml`** (proposed):
```toml
[features]
default = ["native"]
native = ["reqwest/rustls-tls"]
browser = ["reqwest/wasm"]  # reqwest supports wasm32 natively
```

### 7.2 Conditional Compilation Pattern

For modules that have native-only behavior:

```rust
// clawft-core/src/agent/mod.rs
pub mod loop_core;
pub mod skills;
pub mod skills_v2;

#[cfg(feature = "native")]
pub mod skill_watcher;  // Only compiles on native
```

For the agent loop, the async runtime abstraction:

```rust
// Abstract over tokio vs wasm-bindgen-futures
#[cfg(feature = "native")]
use tokio::select;

#[cfg(feature = "browser")]
use futures::select;
```

### 7.3 Crate Organization Options

**Option A: Feature flags in existing crates** (recommended)

Keep the current crate structure. Add `native`/`browser` features to `clawft-platform`, `clawft-core`, `clawft-llm`, and `clawft-tools`. Gate native implementations behind `#[cfg(feature = "native")]`.

Pros: No new crates, no duplication, single source of truth for trait definitions and business logic.
Cons: More complex Cargo.toml files, must be careful with feature unification.

**Option B: Split platform into traits + impl crates**

```
clawft-platform-traits   (trait definitions only -- always compiles)
clawft-platform-native   (NativePlatform impl)
clawft-platform-browser  (BrowserPlatform impl)
```

Pros: Clean separation, impossible to accidentally pull native deps into browser build.
Cons: More crates to manage, transitive dependency chains become longer, existing code must update import paths.

**Recommendation**: Option A for the initial port. The existing crate structure is well-designed and the Platform trait is already properly abstracted. Feature flags achieve the same compilation isolation with less churn. Option B can be pursued later if feature flag complexity becomes unmanageable.

---

## 8. Risk Areas and Blockers

### 8.1 Critical Blockers

| # | Blocker | Location | Impact | Mitigation |
|---|---------|----------|--------|------------|
| B1 | `tokio` with `features=["full"]` as workspace dep | `Cargo.toml:37` | All async code assumes tokio runtime | Split workspace dep: tokio-native vs tokio-wasm (rt only) or use futures-based abstractions |
| B2 | `reqwest` with `rustls-tls` as workspace dep | `Cargo.toml:40` | HTTP client cannot compile for browser | Use `reqwest` with `wasm` feature for browser target (reqwest supports this) |
| B3 | `notify` as hard dep in `clawft-core` | `clawft-core/Cargo.toml:43` | Pulls OS filesystem watcher into core | Feature-gate behind `native` |
| B4 | `clawft-wasm` is decoupled from `Platform` trait | `clawft-wasm/src/platform.rs` | Cannot use real agent loop or pipeline | Implement `Platform` trait in WASM platform |
| B5 | `async_trait` requires `Send + Sync` bounds | `clawft-platform/src/lib.rs:50` | WASM is single-threaded, `!Send` | Use `async_trait(?Send)` for browser builds |

### 8.2 High-Risk Areas

| # | Risk | Location | Impact | Mitigation |
|---|------|----------|--------|------------|
| R1 | `dirs` crate used in `clawft-types` | `clawft-types/Cargo.toml:19` | Foundation crate cannot compile for WASM | Feature-gate or replace with configurable path |
| R2 | `tokio::fs::metadata` leak in `ListDirectoryTool` | `clawft-tools/src/file_tools.rs:374` | Bypasses platform abstraction | Replace with Platform fs trait method |
| R3 | `path.exists()` leak in `config_loader` | `clawft-platform/src/config_loader.rs` | Bypasses Platform fs trait | Use `platform.fs().exists()` |
| R4 | `canonicalize()` in file tool sandboxing | `clawft-tools/src/file_tools.rs` | OS-specific path resolution | Provide browser-compatible path normalization |
| R5 | `std::env` usage in `NativeEnvironment` | `clawft-platform/src/env.rs` | `set_var`/`remove_var` are unsafe since Rust 1.66 | Already isolated behind trait, not a WASM concern |
| R6 | `tokio_util::sync::CancellationToken` | `clawft-channels/src/traits.rs`, `clawft-core` | Requires tokio-util | Replace with `futures::channel::oneshot` or custom |
| R7 | `SystemTime` for pseudo-random in `TieredRouter` | `clawft-core/src/pipeline/tiered_router.rs` | `SystemTime::now()` may panic on some WASM runtimes | Use `js_sys::Date::now()` on browser, feature-gate |

### 8.3 Medium-Risk Areas

| # | Risk | Location | Notes |
|---|------|----------|-------|
| M1 | `chrono` used throughout for timestamps | Multiple crates | chrono compiles for WASM but `Local::now()` requires `js-sys` feature |
| M2 | `uuid` with v4 feature requires random source | Multiple crates | Needs `getrandom` with `js` feature for browser |
| M3 | `sha2` in clawft-security | `clawft-security/Cargo.toml` | Pure Rust, WASM-safe; no issue |
| M4 | `serde_yaml` in clawft-core | `clawft-core/Cargo.toml:36` | Compiles for WASM but is 200KB+ in binary size |
| M5 | Binary size budget | `clawft-wasm/Cargo.toml` | Target <300KB uncompressed; adding core+pipeline may exceed this |

---

## 9. Recommended Approach

### 9.1 Evaluation of Approaches

| Approach | Pros | Cons | Recommended? |
|----------|------|------|-------------|
| **Full Port** | Complete feature parity in browser | Enormous effort; channels/services make no sense in browser | No |
| **Thin Client** | Minimal WASM; all logic server-side | Requires server; latency; no offline capability; defeats purpose | No |
| **Hybrid** (recommended) | Real pipeline in browser; server for channels/services | Moderate effort; must maintain feature flag discipline | **Yes** |

### 9.2 Hybrid Architecture

```
+----------------------------------------------------------+
|  Browser Tab                                              |
|                                                           |
|  +-----------------------------------------------------+ |
|  | JavaScript UI Layer (React/Svelte/etc.)              | |
|  |  - Chat interface                                    | |
|  |  - File editor (Monaco/CodeMirror)                   | |
|  |  - Tool result display                               | |
|  +-----------------------------------------------------+ |
|           |  wasm-bindgen bridge                          |
|  +-----------------------------------------------------+ |
|  | WASM Module (clawft-core + clawft-llm)               | |
|  |                                                       | |
|  |  BrowserPlatform                                      | |
|  |    +-- BrowserHttpClient (fetch API)                  | |
|  |    +-- BrowserFileSystem (OPFS / in-memory)           | |
|  |    +-- BrowserEnvironment (HashMap)                   | |
|  |    +-- process() -> None                              | |
|  |                                                       | |
|  |  AgentLoop<BrowserPlatform>                           | |
|  |    +-- KeywordClassifier                              | |
|  |    +-- TieredRouter                                   | |
|  |    +-- ContextAssembler                               | |
|  |    +-- LlmTransport (via BrowserHttpClient)           | |
|  |    +-- QualityScorer                                  | |
|  |                                                       | |
|  |  ToolRegistry (browser-safe subset)                   | |
|  |    +-- ReadFileTool, WriteFileTool, EditFileTool      | |
|  |    +-- WebSearchTool, WebFetchTool                    | |
|  |    +-- MemoryReadTool, MemoryWriteTool                | |
|  |    +-- MessageTool                                    | |
|  +-----------------------------------------------------+ |
+----------------------------------------------------------+
           |
           | HTTPS (LLM API calls via fetch)
           v
    +------------------+
    | LLM Provider API |
    | (OpenAI, etc.)   |
    +------------------+
```

Key properties of this architecture:
- The **real** `AgentLoop<P>` runs in browser, not a stub
- The **real** pipeline (classify -> route -> assemble -> transport -> score) runs in browser
- LLM API calls go directly from browser to provider (requires CORS or proxy)
- File operations use browser storage (OPFS or in-memory)
- Shell/spawn tools are excluded (already feature-gated)
- Channels are excluded (server-side concern)
- Skills are bundled at build time or fetched on demand (no fs watcher)

### 9.3 CORS and API Key Considerations

Browser-direct LLM API calls face two challenges:
1. **CORS**: Most LLM providers (OpenAI, Anthropic) do not allow browser-origin requests. A lightweight proxy or BFF (Backend-For-Frontend) is needed.
2. **API Keys**: Cannot embed API keys in browser WASM module. Keys must be provided at runtime (user input or session token) or routed through a proxy that injects keys server-side.

This does not affect the architecture of the WASM module itself -- the `BrowserHttpClient` just points at a different URL.

---

## 10. Migration Roadmap

### Phase 1: Foundation (Week 1-2)

**Goal**: Make `clawft-types`, `clawft-platform` (traits only), and `clawft-security` compile for `wasm32-unknown-unknown`.

Tasks:
1. Feature-gate `dirs` in `clawft-types` behind a `native` feature (default on).
2. Split `clawft-platform` into trait definitions (always compiled) and native impls (behind `native` feature). Move `NativePlatform`, `NativeHttpClient`, `NativeFileSystem`, `NativeEnvironment`, `NativeProcessSpawner` behind `#[cfg(feature = "native")]`.
3. Add `?Send` variant of `async_trait` for browser builds where `Send` is not required.
4. Add CI check: `cargo check --target wasm32-unknown-unknown -p clawft-types -p clawft-platform --no-default-features`.
5. Fix `config_loader::discover_config_path` to use Platform fs trait instead of `path.exists()`.

### Phase 2: Core Engine (Week 2-3)

**Goal**: Make `clawft-core` compile for WASM with `--no-default-features --features browser`.

Tasks:
1. Feature-gate `notify` dependency behind `native` feature in `clawft-core`.
2. Gate `skill_watcher.rs` module behind `#[cfg(feature = "native")]`.
3. Abstract async runtime primitives: create a small `runtime` module that provides `select!`, `spawn`, `sleep`, `channel` backed by either tokio (native) or futures + wasm-bindgen-futures (browser).
4. Gate `dirs` usage in `clawft-core` behind `native` feature.
5. Ensure pipeline modules (`classifier.rs`, `router.rs`, `tiered_router.rs`, `traits.rs`) compile without any changes (they should).
6. Fix `SystemTime` usage in `tiered_router.rs` to use a platform-abstracted time source for browser.

### Phase 3: LLM Transport (Week 3-4)

**Goal**: Make `clawft-llm` work in browser.

Tasks:
1. Add `browser` feature to `clawft-llm` that switches `reqwest` to its `wasm` feature.
2. Alternatively, implement a `BrowserLlmTransport` that uses the Platform `HttpClient` trait instead of reqwest directly.
3. Handle streaming responses (SSE) in browser context.
4. Test with at least one LLM provider (OpenAI-compatible) through a CORS proxy.

### Phase 4: Browser Platform Implementation (Week 4-5)

**Goal**: Implement `BrowserPlatform` that satisfies the `Platform` trait.

Tasks:
1. Implement `BrowserHttpClient` using `web-sys` fetch API or `gloo-net`.
2. Implement `BrowserFileSystem` using OPFS (with in-memory fallback).
3. Reuse `WasiEnvironment` pattern for `BrowserEnvironment`.
4. Wire `BrowserPlatform` into `clawft-wasm` so it implements the real `Platform` trait.
5. Remove the decoupled stubs in `clawft-wasm/src/{http,fs,env}.rs` (or keep as WASI target).

### Phase 5: Tool Integration and Entry Point (Week 5-6)

**Goal**: Wire the real agent loop through WASM entry point.

Tasks:
1. Fix `ListDirectoryTool` line 374 to use Platform fs trait instead of `tokio::fs::metadata`.
2. Fix `canonicalize()` in file tool sandboxing to work with browser virtual paths.
3. Register browser-safe tool subset in a `register_browser_tools` function.
4. Update `clawft-wasm/src/lib.rs` to construct `AppContext<BrowserPlatform>` and run the real `AgentLoop`.
5. Expose `wasm-bindgen` entry points for `init`, `send_message`, `on_response`.
6. Verify binary size budget (<300KB target, may need to raise to ~500KB with full pipeline).

### Phase 6: Integration Testing (Week 6+)

Tasks:
1. Create a minimal HTML/JS test harness.
2. Test the full pipeline: user message -> classify -> route -> assemble -> LLM call -> score -> response.
3. Test file operations (read/write via OPFS).
4. Test memory operations.
5. Performance profiling (initial load time, message latency).

---

## Appendix: Per-Crate Dependency Audit

### clawft-types

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| serde 1 | Yes | |
| serde_json 1 | Yes | |
| chrono 0.4 | Yes | Needs `js-sys` feature for `Local::now()` on wasm32 |
| thiserror 2 | Yes | |
| uuid 1 (v4, serde) | Yes | Needs `getrandom/js` feature for wasm32 |
| **dirs 6** | **No** | Home directory detection, no WASM support |

### clawft-platform

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| clawft-types | Conditional | See above |
| async-trait 0.1 | Yes | Needs `?Send` for browser |
| **tokio 1 (full)** | **No** | Full feature set requires threads, I/O |
| **reqwest 0.12** | **No** | `rustls-tls` feature incompatible; `wasm` feature exists |
| thiserror 2 | Yes | |
| tracing 0.1 | Yes | |
| **dirs 6** | **No** | |
| serde 1 | Yes | |
| serde_json 1 | Yes | |

### clawft-core

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| clawft-types | Conditional | |
| clawft-platform | Conditional | |
| clawft-llm | Conditional | |
| clawft-plugin | Nearly | tokio-util |
| async-trait 0.1 | Yes | |
| **tokio 1 (full)** | **No** | |
| tracing 0.1 | Yes | |
| thiserror 2 | Yes | |
| serde 1 | Yes | |
| serde_json 1 | Yes | |
| chrono 0.4 | Yes | With js-sys |
| uuid 1 | Yes | With getrandom/js |
| **dirs 6** | **No** | |
| serde_yaml 0.9 | Yes | Large binary size impact |
| **tokio-util 0.7** | **No** | CancellationToken |
| **futures-util 0.3** | Yes | Works without tokio |
| **notify 7** | **No** | OS filesystem watcher |
| percent-encoding 2 | Yes | |
| fnv 1 | Yes | |

### clawft-plugin

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| async-trait 0.1 | Yes | |
| serde 1 | Yes | |
| serde_json 1 | Yes | |
| thiserror 2 | Yes | |
| **tokio-util 0.7** | **Partial** | Only CancellationToken is used; could be replaced |
| chrono 0.4 | Yes | |
| tracing 0.1 | Yes | |
| semver 1 | Yes | |

### clawft-security

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| serde 1 | Yes | |
| serde_json 1 | Yes | |
| thiserror 2 | Yes | |
| tracing 0.1 | Yes | |
| regex 1 | Yes | |
| chrono 0.4 | Yes | |
| sha2 0.10 | Yes | Pure Rust |

### clawft-llm

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| async-trait 0.1 | Yes | |
| clawft-types | Conditional | |
| futures-util 0.3 | Yes | |
| **reqwest 0.12 (stream, rustls-tls)** | **No** | Switch to `wasm` feature for browser |
| **tokio 1 (full)** | **No** | |
| tracing 0.1 | Yes | |
| thiserror 2 | Yes | |
| serde 1 | Yes | |
| serde_json 1 | Yes | |
| uuid 1 | Yes | |

### clawft-tools

| Dependency | WASM-safe? | Notes |
|------------|-----------|-------|
| clawft-types | Conditional | |
| clawft-platform | Conditional | |
| clawft-core | Conditional | |
| async-trait 0.1 | Yes | |
| **tokio 1 (full)** | **No** | Direct `tokio::fs::metadata` leak |
| tracing 0.1 | Yes | |
| thiserror 2 | Yes | |
| serde 1 | Yes | |
| serde_json 1 | Yes | |
| url 2 | Yes | |
| ipnet 2 | Yes | |

---

## Summary of Findings

1. **60% of the codebase is already portable or nearly portable**. The pipeline system, security crate, types system, and plugin traits need minimal changes.

2. **The biggest structural problem is the `clawft-wasm` decoupling**. It reimplements concepts instead of implementing shared traits, meaning none of the real engine runs in WASM today.

3. **Five workspace dependencies are the root cause of most compilation failures**: `tokio["full"]`, `reqwest["rustls-tls"]`, `dirs`, `notify`, and `tokio-util`. Feature-gating these five dependencies unblocks WASM compilation for the entire core chain.

4. **The Platform trait design is correct**. It already models the right seams. The implementation just needs to be split from the trait definitions.

5. **Three specific code leaks** must be fixed: `tokio::fs::metadata` in `ListDirectoryTool`, `path.exists()` in `config_loader`, and the `notify` hard dependency in `clawft-core`.

6. **The hybrid approach is the clear winner**: run the real pipeline in browser via WASM, exclude server-side concerns (channels, services, CLI), and use a thin CORS proxy for LLM API calls.
