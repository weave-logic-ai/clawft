//! Exponential backoff retry logic for LLM provider calls.
//!
//! [`RetryPolicy`] wraps any [`Provider`] and automatically retries failed
//! requests with configurable exponential backoff. Retries are applied to
//! transient errors (HTTP 429, 500, 502, 503, 504) and network failures.

use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, warn};

use tokio::sync::mpsc;

use crate::error::{ProviderError, Result};
use crate::provider::Provider;
use crate::types::{ChatRequest, ChatResponse, StreamChunk};

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3).
    pub max_retries: u32,
    /// Base delay between retries (default: 1 second).
    pub base_delay: Duration,
    /// Maximum delay between retries (default: 30 seconds).
    pub max_delay: Duration,
    /// Jitter factor: random 0..jitter_fraction of the delay is added (default: 0.25).
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

/// Determines whether a [`ProviderError`] should be retried.
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

/// Calculate delay for attempt `n` (0-indexed) with exponential backoff + jitter.
///
/// The delay is `min(base_delay * 2^n, max_delay)` plus a random jitter of
/// `0..jitter_fraction * delay`.
pub fn compute_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let exp = 2u64.saturating_pow(attempt);
    let base_ms = config.base_delay.as_millis() as u64;
    let raw_ms = base_ms.saturating_mul(exp);
    let capped_ms = raw_ms.min(config.max_delay.as_millis() as u64);

    // Add deterministic-ish jitter: use attempt number to vary
    // For real randomness we use a simple LCG seeded from attempt + current time nanos.
    let jitter_max_ms = (capped_ms as f64 * config.jitter_fraction) as u64;
    let jitter_ms = if jitter_max_ms > 0 {
        // Simple pseudo-random using system time nanoseconds
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        seed % (jitter_max_ms + 1)
    } else {
        0
    };

    Duration::from_millis(capped_ms + jitter_ms)
}

/// A provider wrapper that retries transient failures with exponential backoff.
///
/// # Example
///
/// ```rust,ignore
/// use clawft_llm::{OpenAiCompatProvider, ProviderConfig};
/// use clawft_llm::retry::{RetryPolicy, RetryConfig};
///
/// let inner = OpenAiCompatProvider::new(config);
/// let provider = RetryPolicy::new(inner, RetryConfig::default());
/// // Calls to provider.complete() will now automatically retry on transient errors.
/// ```
pub struct RetryPolicy<P> {
    inner: P,
    config: RetryConfig,
}

impl<P: Provider> RetryPolicy<P> {
    /// Wrap a provider with retry logic.
    pub fn new(inner: P, config: RetryConfig) -> Self {
        Self { inner, config }
    }

    /// Returns a reference to the retry configuration.
    pub fn retry_config(&self) -> &RetryConfig {
        &self.config
    }

    /// Returns a reference to the inner provider.
    pub fn inner(&self) -> &P {
        &self.inner
    }
}

#[async_trait]
impl<P: Provider> Provider for RetryPolicy<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let mut last_err = None;

        for attempt in 0..=self.config.max_retries {
            match self.inner.complete(request).await {
                Ok(response) => {
                    if attempt > 0 {
                        debug!(
                            provider = %self.inner.name(),
                            attempt,
                            "request succeeded after retry"
                        );
                    }
                    return Ok(response);
                }
                Err(err) => {
                    if !is_retryable(&err) || attempt == self.config.max_retries {
                        return Err(err);
                    }

                    // For rate limiting, use the provider's suggested delay if larger
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
                        "retrying after transient error"
                    );

                    tokio::time::sleep(delay).await;
                    last_err = Some(err);
                }
            }
        }

        // This should not be reachable, but handle it defensively
        Err(last_err.unwrap_or(ProviderError::RequestFailed(
            "retry loop exhausted without error".into(),
        )))
    }

    async fn complete_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        // Streaming retry: we can only retry before the stream starts sending
        // chunks. Once the inner provider starts streaming successfully, we
        // cannot replay chunks that were already sent.  Therefore we use the
        // same retry loop structure but delegate to complete_stream.
        let mut last_err = None;

        for attempt in 0..=self.config.max_retries {
            match self.inner.complete_stream(request, tx.clone()).await {
                Ok(()) => {
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
}

impl<P: std::fmt::Debug> std::fmt::Debug for RetryPolicy<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetryPolicy")
            .field("inner", &self.inner)
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatMessage, ChatResponse, Choice, Usage};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A mock provider that fails a configurable number of times before succeeding.
    struct MockProvider {
        name: String,
        fail_count: AtomicU32,
        fail_with: fn(u32) -> ProviderError,
    }

    impl MockProvider {
        fn new(name: &str, failures: u32, fail_with: fn(u32) -> ProviderError) -> Self {
            Self {
                name: name.into(),
                fail_count: AtomicU32::new(failures),
                fail_with,
            }
        }

        fn success_response() -> ChatResponse {
            ChatResponse {
                id: "resp-1".into(),
                choices: vec![Choice {
                    index: 0,
                    message: ChatMessage::assistant("Hello!"),
                    finish_reason: Some("stop".into()),
                }],
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
                model: "test-model".into(),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn complete(&self, _request: &ChatRequest) -> Result<ChatResponse> {
            let remaining = self.fail_count.load(Ordering::SeqCst);
            if remaining > 0 {
                self.fail_count.fetch_sub(1, Ordering::SeqCst);
                return Err((self.fail_with)(remaining));
            }
            Ok(Self::success_response())
        }
    }

    fn test_request() -> ChatRequest {
        ChatRequest::new("test-model", vec![ChatMessage::user("Hi")])
    }

    fn fast_retry_config() -> RetryConfig {
        RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter_fraction: 0.0,
        }
    }

    #[test]
    fn default_retry_config() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.base_delay, Duration::from_secs(1));
        assert_eq!(cfg.max_delay, Duration::from_secs(30));
        assert!((cfg.jitter_fraction - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn is_retryable_rate_limited() {
        assert!(is_retryable(&ProviderError::RateLimited {
            retry_after_ms: 1000,
        }));
    }

    #[test]
    fn is_retryable_timeout() {
        assert!(is_retryable(&ProviderError::Timeout));
    }

    #[test]
    fn is_retryable_server_errors() {
        assert!(is_retryable(&ProviderError::RequestFailed(
            "HTTP 500: internal".into()
        )));
        assert!(is_retryable(&ProviderError::RequestFailed(
            "HTTP 502: bad gateway".into()
        )));
        assert!(is_retryable(&ProviderError::RequestFailed(
            "HTTP 503: unavailable".into()
        )));
        assert!(is_retryable(&ProviderError::RequestFailed(
            "HTTP 504: timeout".into()
        )));
    }

    #[test]
    fn is_not_retryable_auth() {
        assert!(!is_retryable(&ProviderError::AuthFailed("bad key".into())));
    }

    #[test]
    fn is_not_retryable_model_not_found() {
        assert!(!is_retryable(&ProviderError::ModelNotFound("gpt-5".into())));
    }

    #[test]
    fn is_not_retryable_not_configured() {
        assert!(!is_retryable(&ProviderError::NotConfigured(
            "missing key".into()
        )));
    }

    #[test]
    fn is_not_retryable_invalid_response() {
        assert!(!is_retryable(&ProviderError::InvalidResponse(
            "bad json".into()
        )));
    }

    #[test]
    fn is_not_retryable_client_error() {
        assert!(!is_retryable(&ProviderError::RequestFailed(
            "HTTP 400: bad request".into()
        )));
    }

    #[test]
    fn compute_delay_exponential() {
        let config = RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            jitter_fraction: 0.0,
        };
        // attempt 0: 100ms * 2^0 = 100ms
        let d0 = compute_delay(&config, 0);
        assert_eq!(d0.as_millis(), 100);

        // attempt 1: 100ms * 2^1 = 200ms
        let d1 = compute_delay(&config, 1);
        assert_eq!(d1.as_millis(), 200);

        // attempt 2: 100ms * 2^2 = 400ms
        let d2 = compute_delay(&config, 2);
        assert_eq!(d2.as_millis(), 400);
    }

    #[test]
    fn compute_delay_capped() {
        let config = RetryConfig {
            max_retries: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            jitter_fraction: 0.0,
        };
        // attempt 5: 1s * 32 = 32s, but capped at 5s
        let d = compute_delay(&config, 5);
        assert_eq!(d.as_millis(), 5000);
    }

    #[test]
    fn compute_delay_with_jitter_bounded() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.25,
        };
        // attempt 0: base = 1000ms, jitter max = 250ms
        // total should be in [1000, 1250]
        for _ in 0..20 {
            let d = compute_delay(&config, 0);
            let ms = d.as_millis();
            assert!(ms >= 1000, "delay {ms} < 1000");
            assert!(ms <= 1250, "delay {ms} > 1250");
        }
    }

    #[tokio::test]
    async fn retry_succeeds_first_try() {
        let mock = MockProvider::new("test", 0, |_| ProviderError::Timeout);
        let provider = RetryPolicy::new(mock, fast_retry_config());

        let resp = provider.complete(&test_request()).await.unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello!");
    }

    #[tokio::test]
    async fn retry_succeeds_after_transient_failures() {
        let mock = MockProvider::new("test", 2, |_| {
            ProviderError::RequestFailed("HTTP 503: unavailable".into())
        });
        let provider = RetryPolicy::new(mock, fast_retry_config());

        let resp = provider.complete(&test_request()).await.unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello!");
    }

    #[tokio::test]
    async fn retry_exhausted_returns_last_error() {
        let mock = MockProvider::new("test", 10, |_| {
            ProviderError::RequestFailed("HTTP 500: error".into())
        });
        let config = RetryConfig {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            jitter_fraction: 0.0,
        };
        let provider = RetryPolicy::new(mock, config);

        let err = provider.complete(&test_request()).await.unwrap_err();
        assert!(matches!(err, ProviderError::RequestFailed(_)));
    }

    #[tokio::test]
    async fn retry_does_not_retry_auth_errors() {
        let call_count = Arc::new(AtomicU32::new(0));

        struct CountingProvider {
            count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl Provider for CountingProvider {
            fn name(&self) -> &str {
                "counting"
            }
            async fn complete(&self, _req: &ChatRequest) -> Result<ChatResponse> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Err(ProviderError::AuthFailed("invalid key".into()))
            }
        }

        let provider = RetryPolicy::new(
            CountingProvider {
                count: call_count.clone(),
            },
            fast_retry_config(),
        );

        let err = provider.complete(&test_request()).await.unwrap_err();
        assert!(matches!(err, ProviderError::AuthFailed(_)));
        // Should only be called once -- no retries for auth errors
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_rate_limited_uses_suggested_delay() {
        let mock = MockProvider::new("test", 1, |_| ProviderError::RateLimited {
            retry_after_ms: 5,
        });
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(100),
            jitter_fraction: 0.0,
        };
        let provider = RetryPolicy::new(mock, config);

        let resp = provider.complete(&test_request()).await.unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello!");
    }

    #[test]
    fn retry_policy_name_delegates() {
        let mock = MockProvider::new("my-provider", 0, |_| ProviderError::Timeout);
        let provider = RetryPolicy::new(mock, RetryConfig::default());
        assert_eq!(provider.name(), "my-provider");
    }

    #[test]
    fn retry_config_accessor() {
        let mock = MockProvider::new("test", 0, |_| ProviderError::Timeout);
        let config = RetryConfig {
            max_retries: 5,
            ..RetryConfig::default()
        };
        let provider = RetryPolicy::new(mock, config);
        assert_eq!(provider.retry_config().max_retries, 5);
    }
}
