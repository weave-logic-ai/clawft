# Phase BW3: LLM Transport

**Phase ID**: BW3
**Workstream**: W-BROWSER
**Duration**: Week 3-4
**Depends On**: BW1 (Foundation -- feature flags in clawft-types)
**Goal**: Make `clawft-llm` work in browser with CORS support and streaming

---

## S -- Specification

### What Changes

This phase adds a `browser` feature to `clawft-llm` that switches `reqwest` from `rustls-tls` to its `wasm` feature, and adds provider-level CORS configuration to `ProviderConfig` in `clawft-types`. The LLM transport layer itself needs minimal code changes because `reqwest` natively supports `wasm32-unknown-unknown` with the `wasm` feature.

### Files to Change

| File | Change | Task |
|---|---|---|
| `crates/clawft-llm/Cargo.toml` | Add `native`/`browser` features; make `tokio` optional | P3.1 |
| `crates/clawft-llm/src/failover.rs` | Feature-gate `tokio::time::sleep` for retry delays | P3.1 |
| `crates/clawft-types/src/config/mod.rs` | Add `browser_direct` and `cors_proxy` to `ProviderConfig` | P3.4 |
| `crates/clawft-llm/src/transport.rs` (or wherever HTTP calls are made) | Add CORS proxy URL resolution and browser-direct headers | P3.4 |
| `docs/browser/cors-provider-setup.md` | New: per-provider CORS setup guide | P3.6 |
| `docs/browser/config-schema.md` | New: config schema reference with browser fields | P3.7 |

### Exact Cargo.toml Changes

#### `crates/clawft-llm/Cargo.toml`

Current:
```toml
[dependencies]
async-trait = { workspace = true }
clawft-types = { workspace = true }
futures-util = { workspace = true }
reqwest = { workspace = true, features = ["stream"] }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
```

New:
```toml
[features]
default = ["native"]
native = ["reqwest/rustls-tls", "dep:tokio", "clawft-types/native"]
browser = ["reqwest/wasm"]

[dependencies]
async-trait = { workspace = true }
clawft-types = { workspace = true, default-features = false }
futures-util = { workspace = true }
reqwest = { workspace = true, default-features = false, features = ["json", "stream"] }
tracing = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }

# Native only
tokio = { workspace = true, optional = true }
```

Key points:
- `reqwest` base dependency is always included (no `rustls-tls` or `wasm` in base)
- `native` feature adds `reqwest/rustls-tls` and `dep:tokio`
- `browser` feature adds `reqwest/wasm`
- `reqwest` with `wasm` feature uses browser's fetch API internally
- `futures-util` is always available (WASM-safe)

### Provider Config Schema Additions

Add to `crates/clawft-types/src/config/mod.rs`, in the `ProviderConfig` struct:

```rust
/// Provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub api_key: Option<SecretString>,
    #[serde(alias = "baseUrl")]
    pub base_url: Option<String>,
    pub model: Option<String>,

    // NEW: Browser CORS support fields
    /// When true, use direct browser fetch with provider-specific headers.
    /// For Anthropic: adds `anthropic-dangerous-direct-browser-access: true`.
    /// For local models (Ollama, LM Studio): no special headers needed.
    #[serde(default, alias = "browserDirect")]
    pub browser_direct: bool,

    /// CORS proxy URL to prepend to API calls.
    /// When set, the actual request URL becomes `{cors_proxy}{base_url}{path}`.
    /// Example: "https://corsproxy.io/?" or "https://your-proxy.example.com/"
    #[serde(default, alias = "corsProxy")]
    pub cors_proxy: Option<String>,
}
```

These fields use `#[serde(default)]` so existing config files parse without errors.

### Multi-Provider CORS Strategy

| Provider | `browser_direct` | `cors_proxy` | Headers Added |
|---|---|---|---|
| Anthropic | `true` | `None` | `anthropic-dangerous-direct-browser-access: true` |
| OpenAI | `false` | `"https://your-proxy.example.com/"` | None (proxy handles CORS) |
| OpenRouter | `false` | `"https://your-proxy.example.com/"` | None |
| Groq | `false` | `"https://your-proxy.example.com/"` | None |
| Ollama (local) | `true` | `None` | None (localhost CORS allowed) |
| LM Studio (local) | `true` | `None` | None (built-in CORS support) |
| vLLM (local) | `true` | `None` | None (`--allowed-origins '*'` flag) |

### Streaming in Browser

`reqwest` with `wasm` feature supports streaming responses via the browser's `ReadableStream` API. The `stream` feature on reqwest enables `.bytes_stream()` which works on both native and WASM. No additional code is needed for SSE parsing if the existing streaming implementation uses `reqwest`'s streaming API.

If the current implementation uses `tokio::io::AsyncBufRead` or `tokio::io::Lines` for SSE parsing, those must be replaced with `futures_util` equivalents:
- `futures_util::io::AsyncBufReadExt::lines()` works on both
- Or parse SSE manually from the `bytes_stream()`

---

## P -- Pseudocode

### CORS URL Resolution

```rust
// In the LLM transport layer (wherever HTTP requests are constructed)

/// Resolve the final URL for an LLM API call, applying CORS proxy if configured.
fn resolve_url(provider: &ProviderConfig, path: &str) -> String {
    let base = provider.base_url.as_deref().unwrap_or("https://api.anthropic.com");
    let full_url = format!("{}{}", base.trim_end_matches('/'), path);

    match &provider.cors_proxy {
        Some(proxy) => format!("{}{}", proxy, full_url),
        None => full_url,
    }
}

/// Add browser-specific headers for direct browser access.
fn add_browser_headers(
    headers: &mut HashMap<String, String>,
    provider: &ProviderConfig,
) {
    if !provider.browser_direct {
        return;
    }

    // Anthropic requires an explicit opt-in header for direct browser access
    if let Some(ref base_url) = provider.base_url {
        if base_url.contains("anthropic.com") {
            headers.insert(
                "anthropic-dangerous-direct-browser-access".to_string(),
                "true".to_string(),
            );
        }
    }
}
```

### Feature-Gating tokio::time::sleep in Failover

```rust
// crates/clawft-llm/src/failover.rs

// Replace tokio::time::sleep with a feature-gated version:

#[cfg(feature = "native")]
async fn delay(duration: std::time::Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(feature = "browser")]
async fn delay(duration: std::time::Duration) {
    // Use gloo_timers or a JS Promise-based sleep
    let millis = duration.as_millis() as i32;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .expect("no window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
            .expect("setTimeout failed");
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.ok();
}

// Or more simply, if clawft-core's runtime module is available:
// use clawft_core::runtime::sleep;
// But clawft-llm should not depend on clawft-core (circular dep risk).
// Better: add a small sleep utility directly in clawft-llm or use gloo-timers.
```

### Streaming SSE Parsing (Browser-Compatible)

```rust
// If current SSE parsing uses tokio-specific types, replace with:

use futures_util::StreamExt;

async fn stream_response(
    response: reqwest::Response,
) -> Result<Vec<String>, LlmError> {
    let mut stream = response.bytes_stream();
    let mut chunks = Vec::new();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| LlmError::Transport(e.to_string()))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Parse SSE events from buffer
        while let Some(pos) = buffer.find("\n\n") {
            let event = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            if let Some(data) = event.strip_prefix("data: ") {
                if data != "[DONE]" {
                    chunks.push(data.to_string());
                }
            }
        }
    }

    Ok(chunks)
}
```

---

## A -- Architecture

### How BrowserHttpClient Resolves URLs Per Provider

```
User configures config.json:
    providers:
        anthropic:
            api_key: "sk-ant-..."
            base_url: "https://api.anthropic.com"
            browser_direct: true           <-- direct access
        openai:
            api_key: "sk-..."
            base_url: "https://api.openai.com/v1"
            cors_proxy: "https://proxy.example.com/"   <-- via proxy

LLM Transport makes a call:
    provider = "anthropic", path = "/v1/messages"
        |
        v
    resolve_url() -> "https://api.anthropic.com/v1/messages"
    add_browser_headers() -> { "anthropic-dangerous-direct-browser-access": "true" }
        |
        v
    fetch("https://api.anthropic.com/v1/messages", { headers: {...} })

    provider = "openai", path = "/chat/completions"
        |
        v
    resolve_url() -> "https://proxy.example.com/https://api.openai.com/v1/chat/completions"
    add_browser_headers() -> {} (no special headers)
        |
        v
    fetch("https://proxy.example.com/https://api.openai.com/v1/chat/completions")
```

### Tiered Routing in Browser

The `TieredRouter` selects providers based on complexity tiers. This works unchanged in browser:

```
free tier  (0.0-0.3) -> local Ollama/LM Studio (browser_direct: true, no CORS issue)
standard   (0.0-0.7) -> OpenRouter or proxied OpenAI (cors_proxy set)
premium    (0.5-1.0) -> Direct Anthropic (browser_direct: true)
```

No changes to `TieredRouter` logic needed. Only the HTTP transport layer adapts per provider using `browser_direct` and `cors_proxy` config fields.

### reqwest WASM Architecture

When `reqwest` is compiled with `features = ["wasm"]`:
- It uses `web_sys::window().fetch()` internally
- `reqwest::Client` works but ignores timeouts (browser manages them)
- `.bytes_stream()` returns chunks from the browser's `ReadableStream`
- CORS preflight is handled by the browser automatically
- The API surface is identical to native reqwest

This means the existing LLM transport code that uses `reqwest::Client` compiles for WASM with minimal changes. The main code changes are:
1. Feature-gating `tokio::time::sleep` (used in retry logic)
2. Adding CORS config fields to `ProviderConfig`
3. URL resolution logic for CORS proxy

---

## R -- Refinement

### Risks

| Risk | Severity | Mitigation |
|---|---|---|
| `reqwest` WASM streaming drops events | Medium | Test with Anthropic streaming API; fall back to non-streaming |
| CORS proxy adds latency | Low | Direct access for Anthropic; proxy only for OpenAI/Groq |
| API key exposure in browser | Medium | Document security implications; recommend server-side proxy for production |
| `reqwest` WASM does not support all features | Low | Features used: `json`, `stream`. Both supported on WASM |

### Edge Cases

1. **No CORS proxy configured for provider that needs one**: Browser will get a CORS error. The error should be caught and a helpful message returned: "CORS error: configure `cors_proxy` in provider config or use `browser_direct: true` if supported."

2. **Anthropic rejects `browser_direct` header**: Anthropic requires an Anthropic-Version header. Ensure the existing transport code already sends it.

3. **Local Ollama with CORS disabled**: User must start Ollama with `OLLAMA_ORIGINS="*" ollama serve`. Document this in CORS setup guide.

4. **Empty `cors_proxy` string**: Treat as `None` (no proxy). Add validation.

### Testing Strategy

1. **Compilation**: `cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser`
2. **Config backward compatibility**: Deserialize existing `config.json` files and verify new fields default to `false`/`None`
3. **Unit tests**: Test `resolve_url()` and `add_browser_headers()` with various provider configs
4. **Native regression**: `cargo test --workspace` -- existing LLM tests pass

### Backward Compatibility

The new `ProviderConfig` fields use `#[serde(default)]`:
- `browser_direct: bool` defaults to `false`
- `cors_proxy: Option<String>` defaults to `None`

Existing config files parse identically. The new fields are ignored by native builds.

---

## C -- Completion

### Exit Criteria

- [ ] `cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser` passes
- [ ] `ProviderConfig` has `browser_direct: bool` and `cors_proxy: Option<String>` fields
- [ ] Both fields use `#[serde(default)]` -- existing configs parse without errors
- [ ] `resolve_url()` correctly applies CORS proxy
- [ ] `add_browser_headers()` adds `anthropic-dangerous-direct-browser-access` when appropriate
- [ ] `tokio::time::sleep` in failover.rs is feature-gated
- [ ] `cargo test --workspace` -- zero regressions
- [ ] `cargo build --release --bin weft` -- native CLI builds
- [ ] Existing `.clawft/config.json` files parse without errors (backward compatible)

### Test Commands

```bash
# Browser WASM check
cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser

# Native regression
cargo test --workspace
cargo test -p clawft-llm
cargo test -p clawft-types

# Config backward compatibility
# Run existing config parsing tests -- they should pass with new fields defaulting
cargo test -p clawft-types -- config

# Dependency tree
cargo tree -p clawft-llm --no-default-features --features browser
# Should show reqwest with wasm feature, NO tokio, NO rustls
```

### Documentation Deliverables

1. **`docs/browser/cors-provider-setup.md`**: Per-provider setup instructions
   - Anthropic: enable `browser_direct`, no proxy needed
   - OpenAI: deploy CORS proxy, configure `cors_proxy` URL
   - Ollama: start with `OLLAMA_ORIGINS="*"`, set `browser_direct: true`
   - Custom proxy: Cloudflare Worker example, Vercel Edge Function example
   - Troubleshooting: common CORS errors and fixes

2. **`docs/browser/config-schema.md`**: Full config.json schema with browser fields
   - Annotated example for browser deployment
   - Annotated example for native deployment (unchanged)
   - Field reference: `browser_direct`, `cors_proxy`
   - Security considerations for API keys in browser

### Phase Gate

```bash
#!/bin/bash
set -euo pipefail

echo "=== Gate 1: Native tests ==="
cargo test --workspace

echo "=== Gate 2: Native CLI build ==="
cargo build --release --bin weft

echo "=== Gate 3: WASI WASM build ==="
cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm

echo "=== Gate 4: Browser WASM check ==="
cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser

echo "BW3 phase gate PASSED"
```
