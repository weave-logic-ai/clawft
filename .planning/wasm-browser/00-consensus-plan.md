# WASM Browser Plan: clawft in the Browser

**Date**: 2026-02-24
**Status**: Consensus Plan (3-agent synthesis)
**Target**: `.planning/wasm-browser/`

---

## Expert Consensus Summary

Three expert agents independently analyzed:

1. **Researcher** -- Deep analysis of [sachaa/openbrowserclaw](https://github.com/sachaa/openbrowserclaw) (TypeScript browser-native agent)
2. **Explorer** -- Audit of clawft WASM crate, platform layer, dependencies, and existing browser support
3. **System Architect** -- Full architectural analysis, dependency graph, portability assessment, migration roadmap

### Unanimous Findings

- **Hybrid approach**: Run the real `AgentLoop<BrowserPlatform>` and pipeline in browser WASM; exclude channels/services/CLI
- **Feature flags over new crates**: Gate native deps (`tokio["full"]`, `reqwest["rustls-tls"]`, `dirs`, `notify`, `tokio-util`) behind `native` feature; add `browser` feature
- **Five root-cause dependencies** block WASM compilation for the entire core chain
- **Three code leaks** bypass the Platform trait and must be fixed
- **The Platform trait design is correct** -- it already models the right abstraction seams
- **60% of the codebase is already portable** -- pipeline, types, security are pure computation
- **`clawft-wasm` is decoupled** from `clawft-core`/`clawft-platform` and must be reconnected

### Key Lesson from openbrowserclaw

openbrowserclaw is NOT a WASM port -- it's a pure TypeScript SPA. But it demonstrates:

| Browser Concern | openbrowserclaw Approach | clawft Browser Approach |
|---|---|---|
| Config storage | IndexedDB key-value (`config` object store) | IndexedDB via `BrowserFileSystem` trait impl |
| File workspace | OPFS (Origin Private File System) | OPFS via `BrowserFileSystem` trait impl |
| LLM API calls | Direct `fetch()` with `anthropic-dangerous-direct-browser-access` header | `BrowserHttpClient` impl using `web-sys` fetch or `reqwest` wasm feature |
| Shell commands | Pure JS shell emulator (747 lines) | Browser-safe tool subset (no shell/spawn) |
| Concurrency | Web Worker for agent loop | `wasm-bindgen-futures` executor in main thread or Web Worker |
| Encryption | Web Crypto API (AES-256-GCM, non-extractable keys) | Same pattern for API key storage |
| Persistent memory | `CLAUDE.md` file per group in OPFS | Session metadata via `BrowserFileSystem` |

---

## Architecture

```
+----------------------------------------------------------+
|  Browser Tab (PWA-capable)                                |
|                                                           |
|  +-----------------------------------------------------+ |
|  | JavaScript UI Layer                                  | |
|  |  - Chat interface                                    | |
|  |  - Settings (config.json editor, API key input)      | |
|  |  - File browser (OPFS workspace view)                | |
|  |  - Tool result display                               | |
|  +-----------------------------------------------------+ |
|           |  wasm-bindgen bridge                          |
|  +-----------------------------------------------------+ |
|  | WASM Module (clawft-core + clawft-llm + clawft-tools)| |
|  |                                                       | |
|  |  BrowserPlatform implements Platform trait             | |
|  |    +-- BrowserHttpClient (web-sys fetch API)          | |
|  |    +-- BrowserFileSystem (OPFS + IndexedDB)           | |
|  |    +-- BrowserEnvironment (in-memory HashMap)         | |
|  |    +-- process() -> None                              | |
|  |                                                       | |
|  |  AgentLoop<BrowserPlatform>                           | |
|  |    +-- KeywordClassifier (unchanged)                  | |
|  |    +-- TieredRouter (unchanged)                       | |
|  |    +-- ContextAssembler (unchanged)                   | |
|  |    +-- LlmTransport (via BrowserHttpClient)           | |
|  |    +-- QualityScorer (unchanged)                      | |
|  |    +-- Verification (unchanged)                       | |
|  |                                                       | |
|  |  ToolRegistry (browser-safe subset)                   | |
|  |    +-- ReadFile, WriteFile, EditFile, ListDirectory   | |
|  |    +-- WebSearch, WebFetch                            | |
|  |    +-- MemoryRead, MemoryWrite                        | |
|  |    +-- Message                                        | |
|  +-----------------------------------------------------+ |
+----------------------------------------------------------+
           |
           | HTTPS (direct or via CORS proxy)
           v
    +------------------+
    | LLM Provider API |
    +------------------+
```

### Browser Config Flow (`.clawft/config.json` equivalent)

```
User opens app
    |
    v
[Check IndexedDB for stored config]
    |
    +-- Found: parse JSON, pass to WASM init()
    |
    +-- Not found: show settings UI
         |
         v
    [User enters API key, model prefs, etc.]
         |
         v
    [Encrypt API key with Web Crypto (AES-256-GCM)]
         |
         v
    [Store config in IndexedDB]
         |
         v
    [Pass config JSON to WASM init()]
         |
         v
    [BrowserPlatform::new(config)]
         |
         v
    [BrowserFileSystem writes config to OPFS: /.clawft/config.json]
         |
         v
    [AgentLoop<BrowserPlatform> starts with real pipeline]
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Goal**: `clawft-types` + `clawft-platform` (traits) + `clawft-security` compile for `wasm32-unknown-unknown`

| Task | File(s) | Detail |
|------|---------|--------|
| Feature-gate `dirs` in clawft-types | `crates/clawft-types/Cargo.toml` | `native = ["dep:dirs"]`, default on. Replace `dirs::home_dir()` with `Option` param |
| Split platform traits from native impls | `crates/clawft-platform/Cargo.toml`, `src/lib.rs` | `native` feature gates `NativePlatform`, `NativeHttpClient`, `NativeFileSystem`, etc. |
| Add `?Send` for browser async_trait | `crates/clawft-platform/src/*.rs` | `#[cfg_attr(feature = "browser", async_trait(?Send))]` |
| Fix config_loader `path.exists()` leak | `crates/clawft-platform/src/config_loader.rs` | Use `Platform::fs().exists()` or make async |
| Add CI WASM check | `.github/workflows/` | `cargo check --target wasm32-unknown-unknown -p clawft-types -p clawft-platform --no-default-features` |

### Phase 2: Core Engine (Week 2-3)

**Goal**: `clawft-core` compiles for WASM with `--features browser`

| Task | File(s) | Detail |
|------|---------|--------|
| Feature-gate `notify` dependency | `crates/clawft-core/Cargo.toml` | `native = ["dep:notify", "dep:dirs"]` |
| Gate `skill_watcher` module | `crates/clawft-core/src/agent/mod.rs` | `#[cfg(feature = "native")] pub mod skill_watcher;` |
| Abstract async runtime | New: `crates/clawft-core/src/runtime.rs` | Provide `select!`, `spawn`, `sleep`, `channel` backed by tokio (native) or futures+wasm-bindgen-futures (browser) |
| Gate `tokio-util` CancellationToken | `crates/clawft-core/src/agent/loop_core.rs` | Replace with `futures::channel::oneshot` on browser |
| Fix `SystemTime` in TieredRouter | `crates/clawft-core/src/pipeline/tiered_router.rs` | Use `js_sys::Date::now()` on browser target |
| Verify pipeline compiles unchanged | `crates/clawft-core/src/pipeline/*.rs` | classifier, router, tiered_router, traits -- should need zero changes |

### Phase 3: LLM Transport (Week 3-4)

**Goal**: `clawft-llm` works in browser

| Task | File(s) | Detail |
|------|---------|--------|
| Add `browser` feature to clawft-llm | `crates/clawft-llm/Cargo.toml` | `browser = ["reqwest/wasm"]` or impl `BrowserLlmTransport` using Platform HttpClient |
| Handle streaming (SSE) in browser | `crates/clawft-llm/src/` | `ReadableStream` via web-sys or reqwest wasm streaming |
| CORS proxy support | Config option | `base_url` override for proxied LLM endpoints |
| Direct browser access header | `crates/clawft-llm/src/` | Add `anthropic-dangerous-direct-browser-access: true` header for Anthropic API (learned from openbrowserclaw) |

### Phase 4: BrowserPlatform (Week 4-5)

**Goal**: Full `Platform` trait implementation for browser

| Task | File(s) | Detail |
|------|---------|--------|
| `BrowserHttpClient` | `crates/clawft-platform/src/browser/http.rs` | `web-sys` fetch API or `gloo-net` |
| `BrowserFileSystem` | `crates/clawft-platform/src/browser/fs.rs` | OPFS for file ops, IndexedDB for config. Virtual path resolution (no `canonicalize`) |
| `BrowserEnvironment` | `crates/clawft-platform/src/browser/env.rs` | In-memory HashMap (reuse WasiEnvironment pattern) |
| `BrowserPlatform` struct | `crates/clawft-platform/src/browser/mod.rs` | Bundles all three, `process() -> None` |
| Config from JS | `crates/clawft-platform/src/browser/config.rs` | Accept config JSON string from JS init, write to OPFS `/.clawft/config.json` |

### Phase 5: WASM Entry Point + Tools (Week 5-6)

**Goal**: Real agent loop exposed via `wasm-bindgen`

| Task | File(s) | Detail |
|------|---------|--------|
| Fix `tokio::fs::metadata` leak | `crates/clawft-tools/src/file_tools.rs:374` | Use Platform `fs` trait method |
| Fix `canonicalize()` for browser | `crates/clawft-tools/src/file_tools.rs` | Virtual path normalization (no symlinks in OPFS) |
| Browser tool registry | `crates/clawft-wasm/src/tools.rs` | Register browser-safe subset: ReadFile, WriteFile, EditFile, ListDir, WebSearch, WebFetch, Memory* |
| Wire real AgentLoop | `crates/clawft-wasm/src/lib.rs` | `AgentLoop<BrowserPlatform>` with real pipeline, replacing stubs |
| wasm-bindgen entry points | `crates/clawft-wasm/src/lib.rs` | `init(config_json)`, `send_message(text) -> Promise`, `on_response(callback)` |
| Binary size audit | Build output | Target <500KB gzipped (relaxed from 120KB due to full pipeline) |

### Phase 6: Integration + Testing (Week 6+)

| Task | Detail |
|------|--------|
| Minimal HTML/JS test harness | Static page that loads WASM, provides chat UI |
| End-to-end pipeline test | user msg -> classify -> route -> LLM call -> tool use -> response |
| OPFS file operation tests | Read/write/list via BrowserFileSystem |
| Config persistence test | Store config, reload page, verify config survives |
| Performance profiling | WASM load time, message-to-response latency |
| Web Worker variant | Move WASM agent loop to Web Worker for UI responsiveness |

---

## What Does NOT Change

- **Pipeline** (classifier, router, tiered_router, scorer, learner) -- pure computation, zero changes
- **Verification module** -- uses Platform `fs` trait, works unchanged
- **Security crate** -- already WASM-compatible
- **Session/context management** -- uses Platform `fs`, works with BrowserFileSystem
- **Tool dispatch logic** -- ToolRegistry is generic, only tool implementations differ
- **Config parsing** -- `serde_json` deserialization is platform-independent

## What Gets Excluded (Browser)

- `clawft-channels` -- server-side channel adapters (Discord, Slack, Telegram)
- `clawft-services` -- cron, heartbeat, MCP server, delegation
- `clawft-cli` -- native binary entry point
- `clawft-plugin-*` -- all plugin crates (git, cargo, browser CDP, containers, etc.)
- `skill_watcher` -- OS filesystem watcher (skills bundled at build time in browser)
- `ShellExecTool`, `SpawnTool` -- already feature-gated behind `native-exec`

---

## Multi-Provider Browser Support

The existing tiered routing system (`TieredRouter`) supports multiple LLM providers. This works
unchanged in browser -- the pipeline selects the provider/model, then `LlmTransport` makes the
HTTP call via `BrowserHttpClient`. The challenge is **CORS**: most providers block browser-origin
requests.

### Provider CORS Compatibility

| Provider | Direct Browser Access | CORS Header | Proxy Needed? |
|----------|----------------------|-------------|---------------|
| **Anthropic** | Yes (opt-in) | `anthropic-dangerous-direct-browser-access: true` | No |
| **OpenAI** | No | None | Yes |
| **OpenRouter** | Partial (depends on plan) | Check `Access-Control-Allow-Origin` | Usually yes |
| **Groq** | No | None | Yes |
| **Ollama** (local) | Yes (localhost CORS allowed) | `--cors-allowed-origins '*'` flag | No (if configured) |
| **LM Studio** (local) | Yes (localhost) | Built-in CORS support | No |
| **vLLM** (local) | Yes (localhost) | `--allowed-origins '*'` flag | No (if configured) |
| **Any OpenAI-compatible** | Varies | Varies | Usually yes |

### CORS Proxy Strategy

Three options, configurable per-provider in `config.json`:

**Option 1: Lightweight self-hosted proxy** (recommended for production)
```json
{
  "providers": {
    "openai": {
      "base_url": "https://your-proxy.example.com/v1",
      "api_key": "sk-...",
      "proxy_mode": "passthrough"
    }
  }
}
```
A minimal proxy (Cloudflare Worker, Vercel Edge Function, or Nginx) that:
- Adds CORS headers to responses
- Optionally injects API key server-side (so key never touches browser)
- Passes through request/response unchanged otherwise

**Option 2: Direct browser access** (Anthropic, local models)
```json
{
  "providers": {
    "anthropic": {
      "base_url": "https://api.anthropic.com",
      "api_key": "sk-ant-...",
      "browser_direct": true
    }
  }
}
```
When `browser_direct: true`, the `BrowserHttpClient` adds provider-specific headers
(e.g., `anthropic-dangerous-direct-browser-access: true`).

**Option 3: Bundled CORS proxy** (development/demo)
```json
{
  "providers": {
    "openai": {
      "base_url": "https://api.openai.com/v1",
      "api_key": "sk-...",
      "cors_proxy": "https://corsproxy.io/?"
    }
  }
}
```
Prepend a public CORS proxy URL. Not recommended for production (API key transits
third-party server).

### Provider Config Schema Addition

```rust
// In clawft-types/src/config/mod.rs -- ProviderConfig additions
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    // New fields for browser support:
    #[serde(default)]
    pub browser_direct: bool,        // Use direct browser fetch (no proxy)
    #[serde(default)]
    pub cors_proxy: Option<String>,  // Prepend this URL for CORS
}
```

### Implementation in BrowserHttpClient

```rust
impl BrowserHttpClient {
    fn resolve_url(&self, provider: &ProviderConfig, path: &str) -> String {
        let base = provider.base_url.as_deref().unwrap_or(DEFAULT_BASE);
        let full_url = format!("{}{}", base, path);

        if let Some(ref proxy) = provider.cors_proxy {
            format!("{}{}", proxy, full_url)
        } else {
            full_url
        }
    }

    fn add_browser_headers(&self, headers: &mut HashMap<String, String>, provider: &ProviderConfig) {
        if provider.browser_direct {
            // Provider-specific direct-access headers
            if provider.base_url.as_deref().unwrap_or("").contains("anthropic") {
                headers.insert(
                    "anthropic-dangerous-direct-browser-access".into(),
                    "true".into(),
                );
            }
        }
    }
}
```

### Tiered Routing in Browser

The `TieredRouter` already selects providers based on complexity tiers:

```
free tier  (0.0-0.3) -> local Ollama/LM Studio (no CORS issue)
standard   (0.0-0.7) -> OpenRouter or proxied OpenAI
premium    (0.5-1.0) -> Direct Anthropic (browser_direct: true)
```

This means for typical browser usage:
- Simple queries hit local models (no CORS issues)
- Complex queries hit Anthropic directly (browser_direct header)
- Middle-tier queries go through CORS proxy to OpenAI/OpenRouter

No changes to `TieredRouter` logic needed -- only the HTTP transport layer adapts per provider.

---

## Native/CLI Regression Protection

### Principle: Zero Impact on Default Build

Every change uses `default = ["native"]` features. The following commands MUST continue to work
identically before and after each phase:

```bash
cargo build                          # Native binary (default features)
cargo build --release --bin weft     # Release CLI binary
cargo test --workspace               # All tests (823+ pass)
cargo clippy --workspace             # No warnings
cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm  # Existing WASI WASM
```

### Specific Regression Risks

| # | Risk | How It Happens | Prevention |
|---|------|----------------|------------|
| N1 | Bare `cargo build` breaks | `default` features misconfigured; required dep becomes optional without being in default | **Rule**: Every `dep:X` made optional MUST appear in `default = ["native"]` feature |
| N2 | Tests fail with default features | Test code uses type that moved behind `#[cfg(feature = "native")]` | **Rule**: All test modules get `#[cfg(test)]` (already true), and tests compile under default features |
| N3 | Downstream crate import breaks | `clawft-core` uses `clawft_platform::NativePlatform` which moved behind feature | **Rule**: When gating a type, add re-export under default feature. `clawft-platform` always re-exports `NativePlatform` when `native` is on |
| N4 | `Send` bounds lost on native | `#[async_trait(?Send)]` accidentally applied to native build | **Rule**: Use `#[cfg_attr(feature = "browser", async_trait(?Send))]` + `#[cfg_attr(not(feature = "browser"), async_trait)]` -- never just `?Send` |
| N5 | WASI WASM build breaks | Changes to `clawft-wasm` for browser support break existing `wasm32-wasip2` target | **Rule**: Browser support goes in new `browser` feature; existing default features unchanged. CI runs both targets |
| N6 | Clippy warnings from dead code | `#[cfg(feature = "native")]` makes code unreachable under `--features browser` | **Rule**: Use `#[cfg_attr(not(feature = "native"), allow(dead_code))]` sparingly; prefer clean feature separation |
| N7 | Cargo feature unification | A dependency enables `native` transitively when only `browser` is intended | **Rule**: Never put `native` and `browser` in same dependency's features. They are mutually exclusive |

### CI Matrix (Updated)

The existing CI pipeline (`pr-gates.yml`) must be extended, not replaced:

```yaml
jobs:
  # EXISTING -- unchanged
  clippy:        cargo clippy --workspace
  test:          cargo test --workspace
  wasm-size:     cargo build --target wasm32-wasip2 -p clawft-wasm  # Existing WASI target
  binary-size:   cargo build --release --bin weft
  smoke-test:    Docker build + gateway health check

  # NEW -- added for browser WASM
  wasm-browser-check:
    name: Browser WASM compilation check
    steps:
      - cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser
      # Verify native still works after feature changes
      - cargo check --workspace  # Redundant with clippy but fast sanity check

  wasm-browser-size:
    name: Browser WASM size gate
    steps:
      - wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser
      # Gate: < 500KB gzipped (different budget from WASI target)
```

### Feature Flag Validation Script

Add `scripts/check-features.sh` that verifies both targets compile:

```bash
#!/bin/bash
set -euo pipefail
echo "=== Native (default) ==="
cargo check --workspace
cargo test --workspace --no-run

echo "=== WASI WASM (existing) ==="
cargo check --target wasm32-wasip2 -p clawft-wasm

echo "=== Browser WASM (new) ==="
cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser

echo "All targets OK"
```

### Phase Gate Rule

Each implementation phase MUST end with:
1. `cargo test --workspace` -- all existing tests pass (zero regressions)
2. `cargo build --release --bin weft` -- native CLI binary builds
3. `cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm` -- existing WASI build works
4. If browser features added in that phase: `cargo check --target wasm32-unknown-unknown -p <crate> --no-default-features --features browser`

---

## Documentation Plan

Documentation is created alongside each implementation phase, not as an afterthought.

### Phase 1 Docs
| Doc | Location | Contents |
|-----|----------|----------|
| ADR: Browser WASM Support | `docs/architecture/adr-027-browser-wasm-support.md` | Decision record: why hybrid approach, alternatives considered, trade-offs |
| Feature flag guide | `docs/development/feature-flags.md` | How `native`/`browser` features work, rules for adding new deps, how to check both targets |

### Phase 3 Docs
| Doc | Location | Contents |
|-----|----------|----------|
| Provider CORS guide | `docs/browser/cors-provider-setup.md` | Per-provider setup: Anthropic direct, OpenAI proxy, Ollama local, custom proxy deployment |
| Config schema reference | `docs/browser/config-schema.md` | New fields: `browser_direct`, `cors_proxy`, browser-specific provider config examples |

### Phase 5 Docs
| Doc | Location | Contents |
|-----|----------|----------|
| Browser build guide | `docs/browser/building.md` | How to build WASM module, wasm-pack commands, output files, JS integration |
| Browser quick start | `docs/browser/quickstart.md` | Minimal HTML/JS example, config setup, first message |
| API reference | `docs/browser/api-reference.md` | `init()`, `send_message()`, `set_env()` -- wasm-bindgen exported functions |

### Phase 6 Docs
| Doc | Location | Contents |
|-----|----------|----------|
| Browser deployment guide | `docs/browser/deployment.md` | Static hosting (Vercel, Netlify, S3), CORS proxy deployment, PWA setup |
| Architecture overview | `docs/browser/architecture.md` | Updated diagram showing browser vs native paths, crate dependency graph with features |

### Existing Docs Updates
| Doc | Change |
|-----|--------|
| `README.md` | Add "Browser" section with build instructions and link to browser docs |
| `docs/architecture/wasm-browser-portability-analysis.md` | Already written by architect agent -- keep as reference |
| `CLAUDE.md` | Add browser build commands to Build & Test section |

---

## Risk Register

| # | Risk | Severity | Mitigation |
|---|------|----------|------------|
| R1 | CORS blocks direct LLM API calls | High | Lightweight proxy; or use Anthropic's `dangerous-direct-browser-access` header |
| R2 | WASM binary size exceeds budget | Medium | Tree-shake via `wasm-opt`, disable `serde_yaml` in browser, audit dependencies |
| R3 | `async_trait` Send bounds incompatible with WASM | High | `#[async_trait(?Send)]` for browser feature builds |
| R4 | OPFS browser support gaps | Low | OPFS is supported in Chrome 102+, Firefox 111+, Safari 15.2+; fallback to IndexedDB |
| R5 | API key exposure in browser | Medium | Web Crypto API encryption (AES-256-GCM, non-extractable key); warn users |
| R6 | `getrandom` crate needs `js` feature for WASM | Low | Add `getrandom = { version = "0.2", features = ["js"] }` to workspace deps |
| R7 | `chrono` needs `js-sys` feature for `Local::now()` | Low | Add feature flag; or use `js_sys::Date::now()` directly |
| R8 | Web Worker message passing overhead | Low | Structured clone is fast; batch tool results |

---

## Success Criteria

1. `cargo build --target wasm32-unknown-unknown -p clawft-wasm --features browser` succeeds
2. WASM module loads in browser, accepts config JSON, initializes `AgentLoop<BrowserPlatform>`
3. Full pipeline executes: classify -> route -> assemble -> LLM call -> score -> response
4. File tools (read/write/edit) work via OPFS
5. Config persists across page reloads via IndexedDB
6. All existing native tests pass with `--features native` (no regressions)
7. WASM binary < 500KB gzipped
