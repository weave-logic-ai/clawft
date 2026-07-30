# WEFT-390 result — browser streaming chat via ReadableStream / wasm-streams

**Ticket:** WEFT-390  
**Branch:** `wave0l/weft-390-stream-chat`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4ea-5b24-73d2-a8c3-9e8c1ed1da1d`  
**Date:** 2026-07-30

## Problem

`StreamCallback = Box<dyn FnMut(&str) -> bool + Send>` is incompatible with the
browser’s single-threaded `!Send` model. `PipelineRegistry::complete_stream` and
`LlmTransport::complete_stream` were gated `#[cfg(not(feature = "browser"))]`.
`browser_delay()` was a no-op yield (retry/backoff never waited). No WASM or UI
entry exposed pipeline streaming.

## Design choice

**Keep callback-based streaming** (not a full rewrite onto raw
`web-sys` ReadableStream). Under the hood, `BrowserLlmClient::complete_stream_callback`
already consumes SSE via reqwest’s `bytes_stream()`, which on wasm32 uses
**wasm-streams / Fetch `ReadableStream`**. CORS proxy routing on
`resolve_url` applies unchanged to the streaming path.

Browser path uses a **non-`Send` `StreamCallback`** and a callback-shaped
`LlmProvider::complete_stream` (no tokio mpsc).

## Acceptance criteria

| AC | Status |
|----|--------|
| Browser-flavoured stream callback without `Send` | Done — `StreamCallback` is `Box<dyn FnMut(&str) -> bool>` under `feature = "browser"` |
| Browser streaming path through the pipeline | Done — `LlmTransport` / `PipelineRegistry` / `OpenAiCompatTransport` / `BrowserLlmAdapter` |
| `browser_delay()` → `gloo-timers` | Done on wasm32; host tests use tokio sleep when `native` is on |
| CORS-proxy compatibility for SSE | Unchanged path: `resolve_url` + same headers on `complete_stream_callback` |
| Test exercising streamed chunks via the pipeline | Done — `crates/clawft-core/tests/browser_stream.rs` (3 tests) |

## What shipped

### 1. Non-`Send` stream callback (clawft-core)

- `StreamCallback` is feature-conditional: `+ Send` on native, no `Send` on browser.
- `LlmTransport::complete_stream` and `PipelineRegistry::complete_stream` always available.

### 2. Browser transport stream path

- `LlmProvider::complete_stream` browser signature:
  `Box<dyn for<'a> FnMut(&'a str) -> bool>` (no mpsc).
- `OpenAiCompatTransport::complete_stream` under browser: sequential
  callback fan-out via `Rc<RefCell<_>>` (no `tokio::spawn`).
- `BrowserLlmAdapter::complete_stream` → `BrowserLlmClient::complete_stream_callback`
  (SSE text deltas + synthetic OpenAI-shaped final JSON).

### 3. `browser_delay` / timers

- wasm32 + browser: `gloo_timers::future::TimeoutFuture`
- host: `tokio::time::sleep` when `native` is enabled
- Cargo: optional `gloo-timers` on `clawft-llm` `browser` feature

### 4. WASM + UI surface

- `stream_chat(text, on_chunk: Function) -> Promise<string>` wasm-bindgen export
- `AgentLoop::pipeline()` accessor for the streaming entry
- `WasmAdapter.sendMessageStream()` (falls back to `send_message` if export missing)
- browser regression: `stream_chat` pre-init guard

### 5. Host clock under browser feature

- `runtime::now_millis()` uses `js_sys` only on `wasm32`; host browser-feature
  tests use `SystemTime` so `PipelineRegistry::complete_stream` can run natively.

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/src/pipeline/traits.rs` | Non-Send `StreamCallback`; ungated `complete_stream` |
| `crates/clawft-core/src/pipeline/transport.rs` | Browser `LlmProvider` + transport stream path |
| `crates/clawft-core/src/pipeline/browser_llm_adapter.rs` | `complete_stream` via client callback |
| `crates/clawft-core/src/agent/loop_core.rs` | `pipeline()` accessor |
| `crates/clawft-core/src/runtime.rs` | Host-safe `now_millis` under browser feature |
| `crates/clawft-core/tests/browser_stream.rs` | **new** — pipeline stream tests |
| `crates/clawft-core/Cargo.toml` | `[[test]] browser_stream` required-features |
| `crates/clawft-llm/src/browser_transport.rs` | `browser_delay` → gloo-timers |
| `crates/clawft-llm/Cargo.toml` | `gloo-timers` optional dep |
| `crates/clawft-wasm/src/lib.rs` | `stream_chat` export |
| `crates/clawft-wasm/tests/browser_pipeline.rs` | pre-init stream_chat test |
| `clawft-ui/src/lib/adapters/wasm-adapter.ts` | `sendMessageStream` |
| `docs/plans/wave-0l-WEFT-390-result.md` | this file |

## How to test

```bash
# Native regression (stream transport still works)
cargo test -p clawft-core --lib pipeline::transport

# Browser stream path (host)
cargo test -p clawft-core --no-default-features --features browser --test browser_stream

# Browser LLM helpers + delay
cargo test -p clawft-llm --features browser --lib browser_transport

# WASM compile
cargo check -p clawft-wasm --target wasm32-unknown-unknown --no-default-features --features browser

# Optional headless browser suite (needs chrome + wasm-pack)
scripts/build.sh test-browser
```

## Verification (this branch)

- `scripts/build.sh check` — PASS  
- `cargo test -p clawft-core --lib pipeline::transport` — 30 passed  
- `cargo test -p clawft-core --no-default-features --features browser --test browser_stream` — 3 passed  
- `cargo test -p clawft-llm --features browser --lib browser_transport` — 28 passed  
- `cargo check -p clawft-wasm --target wasm32-unknown-unknown --no-default-features --features browser` — PASS  

## Out of scope / follow-ups

- Full agent-loop tool-turn streaming (pairs with WEFT-350 voice/streaming chat)
- Raw `web-sys` ReadableStream JS return type (callback export is the JS surface)
- Live network SSE e2e in wasm-bindgen-test (still harness / CORS proxy)
