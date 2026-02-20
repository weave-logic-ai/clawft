# Phase D-Reliability: Structured Errors, Configurable Retry & Streaming Failover

**Element**: 05 -- Pipeline & LLM Reliability
**Phase**: D-Reliability
**Timeline**: Week 3-4
**Priority**: P1 (D3, D4), P2 (D7, D2)
**Crates**: `clawft-llm`, `clawft-core`
**Dependencies**: None external (D3 -> D4 internal, D7 -> D2 recommended)
**Status**: Planning

---

## Overview

This phase fixes four correctness and resilience issues in the LLM pipeline:

1. **D3** -- Replace string-prefix matching in `is_retryable()` with a structured `ServerError` variant.
2. **D4** -- Remove the duplicate hardcoded retry loop in `llm_adapter.rs` and wire in the existing `RetryPolicy` from `clawft-llm`, making it configurable via `config.json`.
3. **D7** -- Change the `StreamCallback` type alias from `Fn` to `FnMut` to support stateful callbacks (token counters, progress bars) without interior mutability hacks.
4. **D2** -- Fix the streaming failover bug where partial output from a failed provider is concatenated with the next provider's full output.

---

## Current Code

### D3: `crates/clawft-llm/src/error.rs` (full file, 140 lines)

The `ProviderError` enum today has no structured representation for HTTP server errors. All non-special status codes fall into the catch-all `RequestFailed(String)`:

```rust
// error.rs:9-56
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("model not found: {0}")]
    ModelNotFound(String),

    #[error("provider not configured: {0}")]
    NotConfigured(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("timeout")]
    Timeout,

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("all providers exhausted: {}", attempts.join("; "))]
    AllProvidersExhausted { attempts: Vec<String> },
}
```

The retry decision in `crates/clawft-llm/src/retry.rs` (lines 43-62) relies on string-prefix matching:

```rust
// retry.rs:43-62
pub fn is_retryable(err: &ProviderError) -> bool {
    match err {
        ProviderError::RateLimited { .. } => true,
        ProviderError::Timeout => true,
        ProviderError::Http(_) => true,
        ProviderError::RequestFailed(msg) => {
            // Retry on 5xx server errors
            msg.starts_with("HTTP 500")
                || msg.starts_with("HTTP 502")
                || msg.starts_with("HTTP 503")
                || msg.starts_with("HTTP 504")
        }
        ProviderError::AuthFailed(_)
        | ProviderError::ModelNotFound(_)
        | ProviderError::NotConfigured(_)
        | ProviderError::InvalidResponse(_)
        | ProviderError::Json(_)
        | ProviderError::AllProvidersExhausted { .. } => false,
    }
}
```

**Bug**: This is fragile. If a provider changes its error message format (e.g. "HTTP 503 Service Unavailable" vs "HTTP 503: unavailable"), or if a new 5xx code (e.g. 507, 529) is returned, the retry logic silently fails. The decision is coupled to the string formatting of `openai_compat.rs`.

**Production call sites constructing `RequestFailed("HTTP ...")` strings** (in `openai_compat.rs`):

```rust
// openai_compat.rs:166-168 (non-streaming path, catch-all after 429/401/403/404 checks)
return Err(ProviderError::RequestFailed(format!(
    "HTTP {status}: {body}"
)));

// openai_compat.rs:260-262 (streaming path, same catch-all)
return Err(ProviderError::RequestFailed(format!(
    "HTTP {status}: {body}"
)));
```

---

### D4: `crates/clawft-core/src/pipeline/llm_adapter.rs` (lines 102-130)

The adapter has a **duplicate** hardcoded retry loop that only handles `RateLimited` and ignores all other retryable errors:

```rust
// llm_adapter.rs:103-129
const MAX_RETRIES: u32 = 3;
let mut last_err = String::new();

for attempt in 0..=MAX_RETRIES {
    match self.provider.complete(&request).await {
        Ok(response) => return Ok(convert_response_to_value(&response)),
        Err(ProviderError::RateLimited { retry_after_ms }) => {
            if attempt == MAX_RETRIES {
                last_err = format!("rate limited after {} retries", MAX_RETRIES);
                break;
            }
            // Use provider-suggested wait, with exponential backoff floor
            let backoff_floor = 1000u64 * 2u64.pow(attempt);
            let wait = retry_after_ms.max(backoff_floor);
            warn!(
                provider = %self.provider.name(),
                attempt = attempt + 1,
                wait_ms = wait,
                "rate limited, retrying"
            );
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        }
        Err(e) => return Err(e.to_string()),
    }
}

Err(last_err)
```

Meanwhile, `clawft-llm/src/retry.rs` already has a proper `RetryConfig` + `RetryPolicy<P>` wrapper (lines 18-234) that handles all retryable error types, exponential backoff with jitter, rate-limit suggested delays, and streaming retry. This adapter code is a **redundant, inferior copy**.

**Existing `RetryConfig`** (retry.rs:18-39):

```rust
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_fraction: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.25,
        }
    }
}
```

---

### D7: `crates/clawft-core/src/pipeline/traits.rs` (line 264)

```rust
// traits.rs:264
pub type StreamCallback = Box<dyn Fn(&str) -> bool + Send + Sync>;
```

**Issue**: `Fn` requires immutable self, so stateful callbacks (token counters, progress bars) must use `Arc<Mutex<...>>` or `AtomicUsize` interior mutability workarounds. `Sync` is unnecessary since the callback is invoked from a single async task.

**Call sites using `StreamCallback`** (all in `clawft-core`):

| File | Line | Usage |
|------|------|-------|
| `pipeline/traits.rs` | 264 | Type alias definition |
| `pipeline/traits.rs` | 279 | `complete_stream()` default implementation parameter |
| `pipeline/traits.rs` | 466 | `PipelineRegistry::complete_stream()` parameter |
| `pipeline/transport.rs` | 25 | Import |
| `pipeline/transport.rs` | 206 | `OpenAiCompatTransport::complete_stream()` parameter |
| `pipeline/transport.rs` | 1189, 1215, 1243, 1272 | Test callback constructions |

---

### D2: `crates/clawft-llm/src/failover.rs` (lines 113-149)

```rust
// failover.rs:113-149
async fn complete_stream(
    &self,
    request: &ChatRequest,
    tx: mpsc::Sender<StreamChunk>,
) -> Result<()> {
    let mut errors: Vec<(String, ProviderError)> = Vec::new();

    for (idx, provider) in self.providers.iter().enumerate() {
        match provider.complete_stream(request, tx.clone()).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                let provider_name = provider.name().to_owned();

                if !is_retryable(&err) && !is_failover_eligible(&err) {
                    return Err(err);
                }

                warn!(
                    provider = %provider_name,
                    provider_index = idx,
                    total_providers = self.providers.len(),
                    error = %err,
                    "provider streaming failed, trying next in failover chain"
                );

                errors.push((provider_name, err));
            }
        }
    }

    let summary: Vec<String> = errors
        .iter()
        .map(|(name, err)| format!("{name}: {err}"))
        .collect();

    Err(ProviderError::AllProvidersExhausted { attempts: summary })
}
```

**Bug**: Each provider attempt receives `tx.clone()`. If provider A fails **mid-stream** (after sending partial chunks), those chunks have already been forwarded to the consumer via `tx`. When provider B starts, it sends its complete output to the **same** `tx`. The consumer sees `partial_A + full_B` concatenated. This produces garbled output.

The same bug exists in `retry.rs:191-192` where `complete_stream` is retried with `tx.clone()`:

```rust
// retry.rs:192
match self.inner.complete_stream(request, tx.clone()).await {
```

---

## Implementation Tasks

### Task D3: Structured Error Variants for Retry

**Goal**: Add a `ServerError { status: u16, body: String }` variant to `ProviderError`. Replace string-prefix matching in `is_retryable()` with numeric range check.

#### Step 1: Add the new variant to `ProviderError`

**File**: `crates/clawft-llm/src/error.rs`

Add after the `RequestFailed` variant (after line 13):

```rust
/// The provider returned a server error (HTTP 5xx).
#[error("server error {status}: {body}")]
ServerError {
    /// The HTTP status code (e.g. 500, 502, 503, 504).
    status: u16,
    /// The response body text.
    body: String,
},
```

#### Step 2: Update `is_retryable()` in retry.rs

**File**: `crates/clawft-llm/src/retry.rs`

Replace the `RequestFailed(msg)` arm (lines 48-54) with the new `ServerError` arm:

```rust
pub fn is_retryable(err: &ProviderError) -> bool {
    match err {
        ProviderError::RateLimited { .. } => true,
        ProviderError::Timeout => true,
        ProviderError::Http(_) => true,
        ProviderError::ServerError { status, .. } => (500..=599).contains(status),
        ProviderError::RequestFailed(_)
        | ProviderError::AuthFailed(_)
        | ProviderError::ModelNotFound(_)
        | ProviderError::NotConfigured(_)
        | ProviderError::InvalidResponse(_)
        | ProviderError::Json(_)
        | ProviderError::AllProvidersExhausted { .. } => false,
    }
}
```

**Key change**: `RequestFailed` is now **not retryable** by default. All HTTP 5xx errors use `ServerError`. The `(500..=599).contains()` range check covers all current and future 5xx codes without maintenance.

#### Step 3: Update provider implementations

**File**: `crates/clawft-llm/src/openai_compat.rs`

Replace the catch-all `RequestFailed` at line 166-168 (non-streaming):

```rust
// Before (line 166-168):
return Err(ProviderError::RequestFailed(format!(
    "HTTP {status}: {body}"
)));

// After:
let status_code = status.as_u16();
return Err(if (500..=599).contains(&status_code) {
    ProviderError::ServerError { status: status_code, body }
} else {
    ProviderError::RequestFailed(format!("HTTP {status}: {body}"))
});
```

Apply the same change at line 260-262 (streaming path).

#### Step 4: Update tests

**File**: `crates/clawft-llm/src/retry.rs` -- test `is_retryable_server_errors` (line 337)

Migrate from `RequestFailed("HTTP 503: ...")` to `ServerError { status: 503, body: "..." }`:

```rust
#[test]
fn is_retryable_server_errors() {
    assert!(is_retryable(&ProviderError::ServerError {
        status: 500, body: "internal".into(),
    }));
    assert!(is_retryable(&ProviderError::ServerError {
        status: 502, body: "bad gateway".into(),
    }));
    assert!(is_retryable(&ProviderError::ServerError {
        status: 503, body: "unavailable".into(),
    }));
    assert!(is_retryable(&ProviderError::ServerError {
        status: 504, body: "timeout".into(),
    }));
}
```

Add new test for non-retryable 4xx through `RequestFailed`:

```rust
#[test]
fn is_not_retryable_request_failed() {
    // RequestFailed is now never retryable (all server errors use ServerError)
    assert!(!is_retryable(&ProviderError::RequestFailed(
        "some generic failure".into()
    )));
}
```

Add test for edge-case server codes:

```rust
#[test]
fn is_retryable_all_5xx_range() {
    // 507 Insufficient Storage, 529 Site Overloaded -- future-proof
    for status in [500, 501, 502, 503, 504, 507, 529, 599] {
        assert!(is_retryable(&ProviderError::ServerError {
            status,
            body: "test".into(),
        }), "status {status} should be retryable");
    }
}
```

Add `ServerError` display test in `crates/clawft-llm/src/error.rs`:

```rust
#[test]
fn display_server_error() {
    let err = ProviderError::ServerError {
        status: 503,
        body: "service unavailable".into(),
    };
    assert_eq!(err.to_string(), "server error 503: service unavailable");
}
```

**Files also needing `ServerError` migration in tests**:

| File | Lines | Current pattern | New pattern |
|------|-------|-----------------|-------------|
| `retry.rs` | 447 | `RequestFailed("HTTP 503: unavailable")` | `ServerError { status: 503, body: "unavailable" }` |
| `retry.rs` | 458 | `RequestFailed("HTTP 500: error")` | `ServerError { status: 500, body: "error" }` |
| `failover.rs` | 281 | `RequestFailed("HTTP 503: unavailable")` | `ServerError { status: 503, body: "unavailable" }` |
| `failover.rs` | 349 | `RequestFailed("HTTP 500: error")` | `ServerError { status: 500, body: "error" }` |
| `failover.rs` | 374 | `RequestFailed("HTTP 502: bad gateway")` | `ServerError { status: 502, body: "bad gateway" }` |

---

### Task D4: Configurable Retry Policy (depends on D3)

**Goal**: Remove the duplicate retry loop from `llm_adapter.rs`. Make the adapter use `RetryPolicy` from `clawft-llm`. Add `retryable_status_codes` to `RetryConfig`. Make retry settings configurable from `config.json`.

#### Step 1: Add `retryable_status_codes` to `RetryConfig`

**File**: `crates/clawft-llm/src/retry.rs`

```rust
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_fraction: f64,
    /// HTTP status codes that are considered retryable.
    /// Defaults to 500..=599 (all 5xx). Set to a specific list
    /// (e.g. `vec![429, 500, 502, 503, 504]`) to restrict retries.
    pub retryable_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.25,
            retryable_status_codes: (500..=599).collect(),
        }
    }
}
```

Update `is_retryable()` to accept the config (or make it a method on `RetryConfig`):

```rust
/// Determines whether a [`ProviderError`] should be retried, given the
/// retry configuration.
pub fn is_retryable_with_config(err: &ProviderError, config: &RetryConfig) -> bool {
    match err {
        ProviderError::RateLimited { .. } => true,
        ProviderError::Timeout => true,
        ProviderError::Http(_) => true,
        ProviderError::ServerError { status, .. } => {
            config.retryable_status_codes.contains(status)
        }
        _ => false,
    }
}

/// Convenience function using default 5xx range.
pub fn is_retryable(err: &ProviderError) -> bool {
    is_retryable_with_config(err, &RetryConfig::default())
}
```

#### Step 2: Add retry config to `config.json` schema

**File**: `crates/clawft-types/src/config.rs` (or wherever `Config` is defined)

Add a `retry` section to the config:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySettings {
    /// Maximum number of retries (default: 3).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Base delay in milliseconds (default: 1000).
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,

    /// Maximum delay in milliseconds (default: 30000).
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Jitter fraction 0.0-1.0 (default: 0.25).
    #[serde(default = "default_jitter")]
    pub jitter_fraction: f64,

    /// HTTP status codes to retry (default: all 5xx).
    #[serde(default)]
    pub retryable_status_codes: Vec<u16>,
}
```

Example `config.json`:

```json
{
  "retry": {
    "max_retries": 5,
    "base_delay_ms": 500,
    "max_delay_ms": 60000,
    "jitter_fraction": 0.3,
    "retryable_status_codes": [500, 502, 503, 504, 529]
  }
}
```

#### Step 3: Remove the duplicate retry loop from `llm_adapter.rs`

**File**: `crates/clawft-core/src/pipeline/llm_adapter.rs`

Replace the hardcoded retry loop (lines 102-129) with a direct provider call. The retry is handled by wrapping the provider in `RetryPolicy` at construction time.

**Before** (lines 102-129):

```rust
// -- Call the underlying provider with retry on rate-limit -----------
const MAX_RETRIES: u32 = 3;
let mut last_err = String::new();

for attempt in 0..=MAX_RETRIES {
    match self.provider.complete(&request).await {
        Ok(response) => return Ok(convert_response_to_value(&response)),
        Err(ProviderError::RateLimited { retry_after_ms }) => {
            // ... 12 lines of retry logic ...
        }
        Err(e) => return Err(e.to_string()),
    }
}

Err(last_err)
```

**After**:

```rust
// RetryPolicy wrapping is applied at construction time (create_adapter_from_config).
match self.provider.complete(&request).await {
    Ok(response) => Ok(convert_response_to_value(&response)),
    Err(e) => Err(e.to_string()),
}
```

#### Step 4: Wrap the provider in `RetryPolicy` at construction

**File**: `crates/clawft-core/src/pipeline/llm_adapter.rs`

In `create_adapter_from_config()` and `create_adapter_for_provider()`, wrap the `OpenAiCompatProvider` in `RetryPolicy` before wrapping in `ClawftLlmAdapter`:

```rust
use clawft_llm::retry::{RetryPolicy, RetryConfig};

pub fn create_adapter_from_config(config: &Config) -> Arc<dyn LlmProvider> {
    // ... existing provider resolution ...

    let provider = OpenAiCompatProvider::new(provider_config);

    // Wrap in RetryPolicy with config from settings
    let retry_config = build_retry_config(config);
    let retried = RetryPolicy::new(provider, retry_config);

    Arc::new(ClawftLlmAdapter::new(Arc::new(retried)))
}

fn build_retry_config(config: &Config) -> RetryConfig {
    // Convert from config.json settings to RetryConfig
    // Falls back to RetryConfig::default() if no retry section
    match &config.retry {
        Some(settings) => RetryConfig {
            max_retries: settings.max_retries,
            base_delay: Duration::from_millis(settings.base_delay_ms),
            max_delay: Duration::from_millis(settings.max_delay_ms),
            jitter_fraction: settings.jitter_fraction,
            retryable_status_codes: if settings.retryable_status_codes.is_empty() {
                (500..=599).collect()
            } else {
                settings.retryable_status_codes.clone()
            },
        },
        None => RetryConfig::default(),
    }
}
```

---

### Task D7: StreamCallback to FnMut

**Goal**: Change the `StreamCallback` type alias from `Fn` to `FnMut` and drop `Sync`.

#### Step 1: Update the type alias

**File**: `crates/clawft-core/src/pipeline/traits.rs` (line 264)

```rust
// Before:
pub type StreamCallback = Box<dyn Fn(&str) -> bool + Send + Sync>;

// After:
pub type StreamCallback = Box<dyn FnMut(&str) -> bool + Send>;
```

#### Step 2: Update call sites to use `&mut callback`

The `LlmTransport::complete_stream()` default implementation (traits.rs:276-293) takes callback by value but must call it mutably:

```rust
// traits.rs -- default implementation
async fn complete_stream(
    &self,
    request: &TransportRequest,
    mut callback: StreamCallback,  // add `mut`
) -> clawft_types::Result<LlmResponse> {
    let response = self.complete(request).await?;
    for block in &response.content {
        if let clawft_types::provider::ContentBlock::Text { text } = block
            && !callback(text)  // FnMut: called via `&mut self`, Rust handles this for `mut callback`
        {
            break;
        }
    }
    Ok(response)
}
```

**File**: `crates/clawft-core/src/pipeline/transport.rs` (line 206)

The `OpenAiCompatTransport::complete_stream()` implementation parameter needs `mut`:

```rust
async fn complete_stream(
    &self,
    request: &TransportRequest,
    mut callback: StreamCallback,  // add `mut`
) -> clawft_types::Result<LlmResponse> {
    // ... body unchanged, callback is invoked via `callback(text)` which works with FnMut
}
```

**File**: `crates/clawft-core/src/pipeline/traits.rs` (line 463)

The `PipelineRegistry::complete_stream()` takes `StreamCallback` by value and passes it down -- no change needed as it moves ownership.

#### Step 3: Verify test callbacks compile

All existing test callbacks in `transport.rs` use `Box::new(move |text| { ... })` with `Fn` closures. These are a subset of `FnMut` and will continue to compile. The `move |text|` closures that capture `Arc<Mutex<Vec<String>>>` already work because `Fn: FnMut`.

New test to verify stateful callbacks work:

```rust
#[tokio::test]
async fn streaming_fnmut_callback_accumulates_state() {
    let transport = TestTransport;  // uses the default complete_stream fallback
    let request = /* ... */;

    let mut token_count: usize = 0;
    let callback: StreamCallback = Box::new(move |text: &str| -> bool {
        token_count += text.split_whitespace().count();
        true
    });

    let _response = transport.complete_stream(&request, callback).await.unwrap();
    // token_count is captured by move; we verify it compiled and ran without Sync
}
```

---

### Task D2: Streaming Failover Correctness

**Goal**: Prevent partial output concatenation when a provider fails mid-stream during failover.

#### Step 1: Implement `StreamFailoverController`

**File**: `crates/clawft-llm/src/failover.rs`

The controller creates a fresh intermediate channel for each provider attempt. On success, it forwards all buffered chunks to the real `tx`. On failure, the intermediate channel is dropped.

```rust
/// Controls stream failover by buffering chunks per-attempt and only
/// committing to the consumer channel on success.
struct StreamFailoverController {
    /// The real consumer-facing sender.
    real_tx: mpsc::Sender<StreamChunk>,
}

impl StreamFailoverController {
    fn new(tx: mpsc::Sender<StreamChunk>) -> Self {
        Self { real_tx: tx }
    }

    /// Try streaming from `provider`. If successful, forward all chunks
    /// to the real consumer. If the provider fails, discard partial output.
    async fn try_provider(
        &self,
        provider: &dyn Provider,
        request: &ChatRequest,
    ) -> Result<()> {
        // Create an intermediate channel for this attempt
        let (attempt_tx, mut attempt_rx) = mpsc::channel::<StreamChunk>(64);

        // Spawn the provider's streaming call
        // We need to collect chunks as they arrive, but only forward on success
        let mut buffer: Vec<StreamChunk> = Vec::new();

        // Run the provider stream
        let stream_result = provider.complete_stream(request, attempt_tx).await;

        // Drain any remaining chunks from the intermediate channel
        while let Ok(chunk) = attempt_rx.try_recv() {
            buffer.push(chunk);
        }

        // If the provider succeeded, forward all chunks to the real consumer
        match stream_result {
            Ok(()) => {
                for chunk in buffer {
                    if self.real_tx.send(chunk).await.is_err() {
                        // Consumer dropped, stop
                        break;
                    }
                }
                Ok(())
            }
            Err(err) => {
                // Discard buffer (partial output) and return the error
                Err(err)
            }
        }
    }
}
```

**Note on buffering approach**: For very long streams, buffering all chunks in memory may be costly. An alternative is to use a "committed" flag where chunks are forwarded in real-time but the consumer is notified to discard on failure. However, the buffer approach is simpler and correct. For production streams that may be very large, a configurable buffer limit can be added later.

**Alternative (real-time forwarding with reset signal)**: If D7 is completed first, the `FnMut` `StreamCallback` at the pipeline level can receive a `\0` reset sentinel. However, at the `clawft-llm` level we use `mpsc::Sender<StreamChunk>`, so the buffer-and-commit approach is cleaner.

#### Step 2: Rewrite `FailoverChain::complete_stream()`

**File**: `crates/clawft-llm/src/failover.rs`

Replace the current `complete_stream()` (lines 113-149):

```rust
async fn complete_stream(
    &self,
    request: &ChatRequest,
    tx: mpsc::Sender<StreamChunk>,
) -> Result<()> {
    let controller = StreamFailoverController::new(tx);
    let mut errors: Vec<(String, ProviderError)> = Vec::new();

    for (idx, provider) in self.providers.iter().enumerate() {
        match controller.try_provider(provider.as_ref(), request).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                let provider_name = provider.name().to_owned();

                if !is_retryable(&err) && !is_failover_eligible(&err) {
                    return Err(err);
                }

                warn!(
                    provider = %provider_name,
                    provider_index = idx,
                    total_providers = self.providers.len(),
                    error = %err,
                    "provider streaming failed, trying next in failover chain"
                );

                errors.push((provider_name, err));
            }
        }
    }

    let summary: Vec<String> = errors
        .iter()
        .map(|(name, err)| format!("{name}: {err}"))
        .collect();

    Err(ProviderError::AllProvidersExhausted { attempts: summary })
}
```

#### Step 3: Fix the same issue in `RetryPolicy::complete_stream()`

**File**: `crates/clawft-llm/src/retry.rs` (lines 180-233)

The retry loop for streaming has the same `tx.clone()` partial-output bug. Apply the same buffering pattern:

```rust
async fn complete_stream(
    &self,
    request: &ChatRequest,
    tx: mpsc::Sender<StreamChunk>,
) -> Result<()> {
    let mut last_err = None;

    for attempt in 0..=self.config.max_retries {
        // Create intermediate channel for this attempt
        let (attempt_tx, mut attempt_rx) = mpsc::channel::<StreamChunk>(64);
        let mut buffer: Vec<StreamChunk> = Vec::new();

        let result = self.inner.complete_stream(request, attempt_tx).await;

        // Drain remaining chunks
        while let Ok(chunk) = attempt_rx.try_recv() {
            buffer.push(chunk);
        }

        match result {
            Ok(()) => {
                // Forward buffered chunks to real consumer
                for chunk in buffer {
                    let _ = tx.send(chunk).await;
                }
                if attempt > 0 {
                    debug!(
                        provider = %self.inner.name(),
                        attempt,
                        "streaming request succeeded after retry"
                    );
                }
                return Ok(());
            }
            Err(err) => {
                // Discard buffer (partial output)
                if !is_retryable(&err) || attempt == self.config.max_retries {
                    return Err(err);
                }

                let delay = if let ProviderError::RateLimited { retry_after_ms } = &err {
                    let computed = compute_delay(&self.config, attempt);
                    let suggested = Duration::from_millis(*retry_after_ms);
                    computed.max(suggested)
                } else {
                    compute_delay(&self.config, attempt)
                };

                warn!(
                    provider = %self.inner.name(),
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "retrying streaming request after transient error"
                );

                tokio::time::sleep(delay).await;
                last_err = Some(err);
            }
        }
    }

    Err(last_err.unwrap_or(ProviderError::RequestFailed(
        "streaming retry loop exhausted without error".into(),
    )))
}
```

---

## Concurrency Plan

```
Week 3                                    Week 4
  |                                         |
  D7 --------> D2 (recommended order)
  |              |
  D3 --------> D4 (hard dependency)
  |
  (D3 and D7 can start in parallel)
```

**Execution order**:

1. **D3 + D7** -- Start in parallel (no dependency between them)
   - D3: ~2 days (error variant + is_retryable + provider updates + test migration)
   - D7: ~0.5 day (type alias change + call site updates)
2. **D4** -- Starts after D3 completes (uses structured `ServerError` + `retryable_status_codes`)
   - D4: ~2 days (config schema + adapter rewrite + integration tests)
3. **D2** -- Starts after D7 completes (benefits from FnMut for reset callbacks)
   - D2: ~2 days (StreamFailoverController + retry.rs fix + integration tests)

**Total critical path**: D3 (2d) -> D4 (2d) = 4 days, D7 (0.5d) -> D2 (2d) = 2.5 days. **4 days on critical path** with D7+D2 running in parallel with D3+D4.

---

## Tests Required

### D3 Tests

| Test | File | Description |
|------|------|-------------|
| `display_server_error` | `error.rs` | Verify `ServerError { status: 503, body: "unavailable" }.to_string()` |
| `is_retryable_server_errors` | `retry.rs` | Migrate from `RequestFailed` to `ServerError` for 500/502/503/504 |
| `is_retryable_all_5xx_range` | `retry.rs` | New: verify 500-599 range including 507, 529 |
| `is_not_retryable_request_failed` | `retry.rs` | New: verify `RequestFailed` is no longer retryable |
| `is_not_retryable_client_error` | `retry.rs` | Update: now uses `RequestFailed("HTTP 400: ...")` which is not retryable by default |
| `retry_succeeds_after_transient_failures` | `retry.rs` | Migrate mock to emit `ServerError` |
| `retry_exhausted_returns_last_error` | `retry.rs` | Migrate mock to emit `ServerError` |
| `failover_to_second_on_retryable_error` | `failover.rs` | Migrate to `ServerError` |
| `all_providers_exhausted` | `failover.rs` | Migrate to `ServerError` |
| `failover_through_three_providers` | `failover.rs` | Migrate to `ServerError` |
| All 13 existing retry.rs tests | `retry.rs` | Must continue passing |
| All 10 existing failover.rs tests | `failover.rs` | Must continue passing |

### D4 Tests

| Test | File | Description |
|------|------|-------------|
| `adapter_retry_uses_retry_policy` | `llm_adapter.rs` | Verify adapter no longer has its own retry loop |
| `adapter_retry_config_from_settings` | `llm_adapter.rs` | Verify config.json retry settings are read |
| `build_retry_config_defaults` | `llm_adapter.rs` | Verify defaults when no retry section |
| `retryable_status_codes_custom` | `retry.rs` | Verify custom status code list is respected |
| `is_retryable_with_config` | `retry.rs` | Test `is_retryable_with_config()` with custom config |
| Existing adapter tests (12 tests) | `llm_adapter.rs` | Must continue passing |

### D7 Tests

| Test | File | Description |
|------|------|-------------|
| `streaming_fnmut_callback_accumulates_state` | `traits.rs` or `transport.rs` | New: verify FnMut callback with mutable state compiles and runs |
| Existing streaming tests (4 tests) | `transport.rs` | Must continue passing (Fn closures are FnMut) |
| `pipeline_registry_complete_stream` | `traits.rs` | Must continue passing |

### D2 Tests

| Test | File | Description |
|------|------|-------------|
| `stream_failover_discards_partial_output` | `failover.rs` | New: provider A sends 3 chunks then fails; provider B sends 5 chunks; consumer receives only B's 5 chunks |
| `stream_failover_success_first_provider` | `failover.rs` | New: primary succeeds; all chunks forwarded |
| `stream_failover_all_fail` | `failover.rs` | New: all providers fail streaming; AllProvidersExhausted returned, consumer receives no chunks |
| `stream_retry_discards_partial_output` | `retry.rs` | New: first attempt sends partial chunks then fails; second attempt succeeds; consumer receives only second attempt's chunks |
| Existing failover streaming (implicit) | `failover.rs` | The non-streaming failover tests should still pass unchanged |

---

## Acceptance Criteria

- [ ] **D3**: `is_retryable()` uses `ProviderError::ServerError { status, .. }` variant with `(500..=599).contains()` range check. Zero string-prefix matching in retry logic.
- [ ] **D3**: `openai_compat.rs` emits `ServerError` for 5xx responses and `RequestFailed` for non-5xx non-special codes.
- [ ] **D3**: All 13 existing retry tests pass. All 10 existing failover tests pass.
- [ ] **D4**: No retry loop in `llm_adapter.rs`. Retry is handled by `RetryPolicy` wrapping at construction.
- [ ] **D4**: `RetryConfig` is configurable from `config.json` with `max_retries`, `base_delay_ms`, `max_delay_ms`, `jitter_fraction`, and `retryable_status_codes`.
- [ ] **D4**: `retryable_status_codes` defaults to 500-599 when not specified.
- [ ] **D7**: `StreamCallback = Box<dyn FnMut(&str) -> bool + Send>`. No `Sync` bound.
- [ ] **D7**: A stateful token-counting callback compiles and runs without `Arc<Mutex<...>>`.
- [ ] **D2**: When provider A fails mid-stream (after sending partial chunks), the consumer sees only provider B's complete output. No concatenation of partial A + full B.
- [ ] **D2**: `RetryPolicy::complete_stream()` also buffers per-attempt, preventing partial output on retry.
- [ ] **Regression**: `cargo test -p clawft-llm` passes. `cargo test -p clawft-core` passes. Full `cargo test` passes.

---

## Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| D3: Downstream code outside clawft-llm pattern-matches on `RequestFailed` with HTTP prefix strings | Low | Medium | Grep for `RequestFailed.*HTTP` across entire repo; update all matches |
| D3: Third-party test fixtures hardcode `RequestFailed("HTTP 5xx: ...")` | Medium | Low | Search and migrate systematically (5 known locations listed above) |
| D4: Removing the retry loop from llm_adapter.rs breaks callers who expect rate-limit retries | Low | High | The `RetryPolicy` wrapper handles `RateLimited` identically; verify with the existing rate-limit retry test |
| D2: Buffering all stream chunks in memory before forwarding increases latency and memory usage | Medium | Medium | For typical LLM responses (~4K tokens), buffer is small. Add a configurable buffer limit for safety. Document that real-time streaming is deferred until a commit-then-forward approach is validated |
| D2: `try_recv()` may miss chunks still in flight between provider task completion and channel drain | Low | Medium | Use `recv()` in a loop until `None` (channel closed) instead of `try_recv()`. The provider task should drop `attempt_tx` when done, closing the channel |
| D7: Code outside clawft-core holds `StreamCallback` across `.await` points expecting `Sync` | Low | Medium | Grep for `StreamCallback` stored in `Arc` or `Mutex`; verify none require `Sync`. The callback is always passed by value and invoked from a single task |
