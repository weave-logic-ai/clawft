# Task Breakdown: WASM Browser Implementation

## Phase 1: Foundation (Week 1-2)

### P1.1 Feature-gate `dirs` in clawft-types
- **File**: `crates/clawft-types/Cargo.toml`
- **Change**: Make `dirs` optional behind `native` feature (default on)
- **File**: `crates/clawft-types/src/config/mod.rs`
- **Change**: Wrap `dirs::home_dir()` in `#[cfg(feature = "native")]`, provide fallback
- **Test**: `cargo check --target wasm32-unknown-unknown -p clawft-types --no-default-features`

### P1.2 Split clawft-platform traits from native impls
- **File**: `crates/clawft-platform/Cargo.toml`
- **Change**: Add `native`/`browser` features, make `tokio`/`reqwest`/`dirs` optional
- **Files**: `src/lib.rs`, `src/http.rs`, `src/fs.rs`, `src/env.rs`, `src/process.rs`
- **Change**: Move `Native*` impls behind `#[cfg(feature = "native")]`
- **New**: `src/browser/mod.rs`, `src/browser/http.rs`, `src/browser/fs.rs`, `src/browser/env.rs`
- **Test**: `cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features`

### P1.3 Add ?Send async_trait for browser
- **Files**: All trait definitions in `clawft-platform/src/*.rs`
- **Change**: `#[cfg_attr(feature = "browser", async_trait(?Send))]`
- **Rationale**: WASM is single-threaded; `Send` bound is unnecessary and breaks some browser types

### P1.4 Fix config_loader platform leak
- **File**: `crates/clawft-platform/src/config_loader.rs`
- **Change**: Replace `path.exists()` with async filesystem check or feature-gate
- **Browser behavior**: Config provided via init(), not filesystem discovery

### P1.5 Add WASM CI check
- **File**: `.github/workflows/pr-gates.yml`
- **Change**: Add `wasm-browser-check` job: `cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser`
- **Change**: Add `wasm-browser-size` job with 500KB gzip budget
- **Verify**: Existing jobs (`clippy`, `test`, `wasm-size`, `binary-size`, `smoke-test`) unchanged
- **Ensures**: Both native and browser targets gate every PR

### P1.6 Feature flag validation script
- **New file**: `scripts/check-features.sh`
- **Contents**: Runs `cargo check --workspace`, `cargo check --target wasm32-wasip2 -p clawft-wasm`, `cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser`
- **Used by**: Developers before pushing, CI as sanity check

### P1.7 Phase gate: native regression check
- **Run**: `cargo test --workspace` (all 823+ tests pass)
- **Run**: `cargo build --release --bin weft` (native CLI binary builds)
- **Run**: `cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm` (existing WASI build)
- **Rule**: Phase 1 cannot merge until all three pass

### P1.8 Write ADR-027: Browser WASM Support
- **New file**: `docs/architecture/adr-027-browser-wasm-support.md`
- **Contents**: Decision record: hybrid approach chosen, alternatives (full port, thin client) rejected, trade-offs, feature flag strategy rationale

### P1.9 Write feature flag development guide
- **New file**: `docs/development/feature-flags.md`
- **Contents**: How `native`/`browser` features work, rules for adding new deps (must appear in default), how to check both targets, mutual exclusivity of native/browser

---

## Phase 2: Core Engine (Week 2-3)

### P2.1 Feature-gate notify in clawft-core
- **File**: `crates/clawft-core/Cargo.toml`
- **Change**: `notify = { version = "7", optional = true }`, add to `native` feature

### P2.2 Gate skill_watcher module
- **File**: `crates/clawft-core/src/agent/mod.rs`
- **Change**: `#[cfg(feature = "native")] pub mod skill_watcher;`
- **File**: `crates/clawft-core/src/agent/mod.rs` (any re-exports)
- **Change**: Gate references to skill_watcher types

### P2.3 Create runtime abstraction module
- **New file**: `crates/clawft-core/src/runtime.rs`
- **Contents**: Feature-gated wrappers for:
  - `spawn(future)` -- tokio::spawn vs wasm_bindgen_futures::spawn_local
  - `sleep(duration)` -- tokio::time::sleep vs gloo_timers
  - `select!` macro -- tokio::select vs futures::select
  - `channel()` -- tokio::sync::mpsc vs futures::channel::mpsc
- **Impact**: `loop_core.rs` uses these wrappers instead of direct tokio calls

### P2.4 Gate CancellationToken usage
- **File**: `crates/clawft-core/src/agent/loop_core.rs`
- **Change**: Replace `tokio_util::sync::CancellationToken` with runtime-abstracted version
- **Browser impl**: `futures::channel::oneshot` based cancellation

### P2.5 Fix SystemTime in TieredRouter
- **File**: `crates/clawft-core/src/pipeline/tiered_router.rs`
- **Change**: Use `instant::Instant` crate (works in both native and WASM) or feature-gate
- **Alternative**: `#[cfg(target_arch = "wasm32")] use js_sys::Date;`

### P2.6 Gate dirs usage in clawft-core
- **File**: `crates/clawft-core/Cargo.toml`
- **Change**: Make `dirs` optional behind `native` feature
- **File**: All `dirs::home_dir()` call sites
- **Change**: Accept home dir as parameter or use Platform::fs()::home_dir()

### P2.7 Verify pipeline modules compile unchanged
- **Test**: `cargo check --target wasm32-unknown-unknown -p clawft-core --no-default-features --features browser`
- **Expected**: classifier.rs, router.rs, tiered_router.rs, traits.rs need zero changes

### P2.8 Phase gate: native regression check
- **Run**: `cargo test --workspace` -- zero regressions
- **Run**: `cargo build --release --bin weft` -- native CLI still builds
- **Run**: `cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm` -- WASI target still works
- **Run**: `scripts/check-features.sh` -- all targets pass

---

## Phase 3: LLM Transport (Week 3-4)

### P3.1 Add browser feature to clawft-llm
- **File**: `crates/clawft-llm/Cargo.toml`
- **Change**: `browser = ["reqwest/wasm"]` or alternative HTTP transport
- **Note**: reqwest natively supports `wasm32-unknown-unknown` with `wasm` feature

### P3.2 Handle streaming in browser
- **File**: `crates/clawft-llm/src/` (streaming module)
- **Change**: Use `ReadableStream` API via web-sys for SSE parsing in browser
- **Fallback**: Non-streaming mode (full response) for initial implementation

### P3.3 CORS proxy configuration
- **File**: `crates/clawft-types/src/config/mod.rs` (provider config)
- **Change**: Add `cors_proxy_url` field to provider config
- **Behavior**: If set, prepend proxy URL to API calls; if unset, call directly

### P3.4 Multi-provider CORS strategy
- **File**: `crates/clawft-types/src/config/mod.rs`
- **Change**: Add `browser_direct: bool` and `cors_proxy: Option<String>` to `ProviderConfig`
- **File**: `crates/clawft-llm/src/` (transport layer)
- **Change**: In browser mode, resolve URL through CORS proxy if configured, add provider-specific headers (e.g., `anthropic-dangerous-direct-browser-access: true` for Anthropic)
- **Supports**: Anthropic (direct), OpenAI (proxied), Ollama/LM Studio (localhost), OpenRouter (proxied), any OpenAI-compatible endpoint
- **Test**: Verify tiered routing works with mixed direct/proxied providers
- **Note**: New config fields use `#[serde(default)]` so existing configs are unaffected

### P3.5 Phase gate: native regression check
- **Run**: `cargo test --workspace` -- new `ProviderConfig` fields don't break existing tests (serde default)
- **Run**: `cargo build --release --bin weft`
- **Verify**: Existing `.clawft/config.json` files parse without errors (new fields default to `false`/`None`)

### P3.6 Write provider CORS documentation
- **New file**: `docs/browser/cors-provider-setup.md`
- **Contents**: Per-provider setup instructions (Anthropic direct, OpenAI proxy, Ollama local, custom proxy), config examples, troubleshooting

### P3.7 Write config schema reference
- **New file**: `docs/browser/config-schema.md`
- **Contents**: Full config.json schema with new browser fields, annotated examples for browser vs native

---

## Phase 4: BrowserPlatform (Week 4-5)

### P4.1 Implement BrowserHttpClient
- **New file**: `crates/clawft-platform/src/browser/http.rs`
- **Uses**: `web-sys` fetch API or `gloo-net`
- **Tests**: Integration tests with mock server (or live test in browser)

### P4.2 Implement BrowserFileSystem
- **New file**: `crates/clawft-platform/src/browser/fs.rs`
- **Uses**: OPFS via `web-sys` (`FileSystemDirectoryHandle`, `FileSystemFileHandle`)
- **Fallback**: In-memory HashMap filesystem
- **Tests**: WASM integration tests

### P4.3 Implement BrowserEnvironment
- **New file**: `crates/clawft-platform/src/browser/env.rs`
- **Uses**: `RefCell<HashMap<String, String>>`
- **Pattern**: Reuse `WasiEnvironment` from `clawft-wasm/src/env.rs`

### P4.4 Wire BrowserPlatform struct
- **New file**: `crates/clawft-platform/src/browser/mod.rs`
- **Implements**: `Platform` trait with `?Send` async_trait
- **Returns**: `process() -> None`

### P4.5 Config from JavaScript
- **New file**: `crates/clawft-platform/src/browser/config.rs`
- **Accepts**: JSON string from JS `init()` call
- **Writes**: Parsed config to BrowserFileSystem at `/.clawft/config.json`
- **Pattern**: Similar to openbrowserclaw's IndexedDB config store

---

## Phase 5: WASM Entry Point + Tools (Week 5-6)

### P5.1 Fix tokio::fs::metadata leak
- **File**: `crates/clawft-tools/src/file_tools.rs:374`
- **Change**: Replace `tokio::fs::metadata(entry_path)` with Platform fs method
- **Option**: Add `metadata()` to FileSystem trait or use `exists()` + file read

### P5.2 Fix canonicalize for browser
- **File**: `crates/clawft-tools/src/file_tools.rs` (sandbox validation)
- **Change**: Virtual path normalization (resolve `.`, `..`, no symlinks)
- **Browser**: OPFS has no symlinks, so simple string-based path resolution works

### P5.3 Browser tool registry
- **New file**: `crates/clawft-wasm/src/tools.rs`
- **Registers**: ReadFile, WriteFile, EditFile, ListDirectory, WebSearch, WebFetch, MemoryRead, MemoryWrite, Message
- **Excludes**: ShellExec, Spawn, DelegateTask (already feature-gated)

### P5.4 Wire real AgentLoop in clawft-wasm
- **File**: `crates/clawft-wasm/src/lib.rs`
- **Change**: Replace stub `process_message()` with `AgentLoop<BrowserPlatform>::process_message()`
- **Requires**: All phases 1-4 complete

### P5.5 wasm-bindgen entry points
- **File**: `crates/clawft-wasm/src/lib.rs`
- **Exports**: `init(config_json)`, `send_message(text) -> Promise<String>`, `set_env(key, value)`
- **Uses**: `wasm-bindgen`, `wasm-bindgen-futures`

### P5.6 Binary size audit
- **Command**: `wasm-pack build crates/clawft-wasm --target web --release --features browser`
- **Target**: < 500KB gzipped
- **If over**: Apply `wasm-opt -Oz`, disable `serde_yaml`, audit for heavy deps

### P5.7 Phase gate: full regression check
- **Run**: `cargo test --workspace` -- zero regressions across all 823+ tests
- **Run**: `cargo build --release --bin weft` -- native CLI binary
- **Run**: `cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm` -- existing WASI
- **Run**: `wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser` -- browser WASM
- **Run**: `scripts/check-features.sh` -- all targets
- **This is the most critical gate**: real AgentLoop now runs in WASM; must verify native is untouched

### P5.8 Write browser build guide
- **New file**: `docs/browser/building.md`
- **Contents**: Prerequisites (wasm-pack, wasm32-unknown-unknown target), build commands, output files, JS integration

### P5.9 Write browser quick start
- **New file**: `docs/browser/quickstart.md`
- **Contents**: Minimal HTML/JS example, config setup, sending first message, expected output

### P5.10 Write wasm-bindgen API reference
- **New file**: `docs/browser/api-reference.md`
- **Contents**: `init(config_json)`, `send_message(text)`, `set_env(key, value)` -- params, return types, error handling

---

## Phase 6: Integration Testing (Week 6+)

### P6.1 HTML/JS test harness
- **New dir**: `crates/clawft-wasm/www/`
- **Files**: `index.html`, `main.js` -- minimal chat UI that loads WASM

### P6.2 End-to-end pipeline test
- **Test**: User message -> classify -> route -> LLM call -> tool use -> response
- **Requires**: CORS proxy or direct Anthropic access

### P6.3 OPFS file operations test
- **Test**: Write file, read file, list directory, delete file
- **Verify**: Persistence across page reloads

### P6.4 Config persistence test
- **Test**: Store config via init(), reload page, verify config survives

### P6.5 Performance profiling
- **Measure**: WASM load time, init time, message-to-response latency
- **Tools**: Chrome DevTools Performance tab, `console.time()`

### P6.6 Web Worker variant
- **Stretch goal**: Move WASM module to Web Worker for UI responsiveness
- **Pattern**: Same as openbrowserclaw's `agent-worker.ts` but with WASM instead of JS

### P6.7 Final regression suite
- **Run**: Full `scripts/check-features.sh`
- **Run**: `cargo test --workspace` with timing comparison to pre-browser baseline
- **Run**: Docker smoke test (existing `pr-gates.yml` smoke-test job)
- **Verify**: No test duration regressions > 10%

### P6.8 Write deployment guide
- **New file**: `docs/browser/deployment.md`
- **Contents**: Static hosting (Vercel, Netlify, S3), CORS proxy deployment (Cloudflare Worker example), PWA configuration

### P6.9 Write architecture overview
- **New file**: `docs/browser/architecture.md`
- **Contents**: Updated architecture diagram (browser vs native paths), crate dependency graph with feature flags, data flow

### P6.10 Update existing docs
- **File**: `README.md` -- Add "Browser" section with build instructions, link to `docs/browser/`
- **File**: `CLAUDE.md` -- Add browser build commands to Build & Test section
- **File**: `docs/architecture/` -- Link to ADR-027 from index
