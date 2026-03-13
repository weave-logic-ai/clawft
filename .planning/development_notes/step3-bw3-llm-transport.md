# Step 3 - BW3: LLM Transport (Browser WASM)

## Summary

Made `clawft-llm` compile and work for browser WASM targets with CORS proxy support and SSE streaming.

## Changes

### 1. `crates/clawft-llm/Cargo.toml` -- Dependency restructuring

- Made `reqwest` a **non-optional** dependency with `default-features = false` and features `["json", "stream"]`.
- The `native` feature now activates `reqwest/rustls-tls` (TLS backend for server/desktop).
- The `browser` feature no longer needs any reqwest feature flags; reqwest auto-detects `wasm32-unknown-unknown` and uses the browser Fetch API.

**Before:**
```toml
native = ["dep:reqwest", "dep:tokio", "clawft-types/native"]
browser = ["clawft-types/browser"]
reqwest = { workspace = true, optional = true, features = ["stream"] }
```

**After:**
```toml
native = ["reqwest/rustls-tls", "dep:tokio", "clawft-types/native"]
browser = ["clawft-types/browser"]
reqwest = { workspace = true, default-features = false, features = ["json", "stream"] }
```

### 2. `crates/clawft-llm/src/error.rs` -- Un-gated Http variant

- Removed `#[cfg(feature = "native")]` from `ProviderError::Http(reqwest::Error)` since reqwest is now always available.
- This allows both native and browser code paths to convert reqwest errors.

### 3. `crates/clawft-llm/src/browser_transport.rs` -- New module (browser-gated)

Created a complete browser transport module with:

- **`resolve_url(base_url, path, cors_proxy)`** -- Constructs the final URL, optionally routing through a CORS proxy. The proxy URL format is `{proxy}/{original_url}` (standard CORS-anywhere pattern).

- **`add_browser_headers(headers, browser_direct, provider_name)`** -- Adds the `anthropic-dangerous-direct-browser-access: true` header when `browser_direct` is enabled for Anthropic/Claude providers.

- **`BrowserLlmClient`** -- Browser-compatible LLM client wrapping `reqwest::Client`:
  - `new()` / `with_api_key()` constructors
  - `complete()` for non-streaming requests
  - `complete_stream_callback()` for SSE streaming with a callback (no tokio mpsc dependency)
  - API key resolution requires explicit key in browser (no env vars)
  - Debug impl masks API keys

- **`browser_delay()`** -- Async sleep utility that yields to the browser event loop. Currently a no-op yield; notes for `gloo-timers` upgrade path.

- **`classify_error()`** -- Maps HTTP status codes to `ProviderError` variants.

### 4. `crates/clawft-llm/src/lib.rs` -- Module registration

Added:
```rust
#[cfg(feature = "browser")]
pub mod browser_transport;

#[cfg(feature = "browser")]
pub use browser_transport::BrowserLlmClient;
```

## Design Decisions

1. **reqwest always present**: Rather than maintaining separate HTTP clients for native/browser, reqwest handles both targets. On native it uses rustls-tls; on wasm32 it uses the Fetch API automatically. This means the `error::Http` variant and reqwest types compile on both platforms.

2. **Callback streaming instead of channels**: Browser WASM does not have `tokio::sync::mpsc`. The `BrowserLlmClient::complete_stream_callback` method uses an `FnMut(StreamChunk) -> bool` callback. This keeps the browser code tokio-free.

3. **Native modules stay gated**: `openai_compat.rs`, `provider.rs`, `retry.rs`, `failover.rs`, and `router.rs` remain gated behind `#[cfg(feature = "native")]` since they use `tokio::sync::mpsc` and `tokio::time::sleep`. The browser transport is a parallel implementation, not a replacement.

4. **CORS proxy pattern**: Uses the CORS-anywhere `{proxy}/{target_url}` convention, which is the most common pattern for browser CORS proxies.

## Test Coverage

28 new tests in `browser_transport::tests`:
- `resolve_url` -- 5 tests (no proxy, with proxy, trailing slashes, leading slashes)
- `add_browser_headers` -- 5 tests (anthropic direct, name contains, claude, non-anthropic, not direct)
- `BrowserLlmClient` -- 7 tests (construction, url generation, API key resolution, debug masking)
- `classify_error` -- 7 tests (429, 401, 403, 404, 500, 503, other)
- Backward compatibility -- 3 tests (config parsing, browser fields defaults, roundtrip)
- `browser_delay` -- 1 test

## Verification

| Check | Result |
|-------|--------|
| `cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser` | PASS (0 warnings) |
| `cargo test --workspace` | PASS (all existing tests) |
| `cargo test -p clawft-llm --features browser` | PASS (152 tests: 124 existing + 28 new) |
| `cargo build --release --bin weft` | PASS |
