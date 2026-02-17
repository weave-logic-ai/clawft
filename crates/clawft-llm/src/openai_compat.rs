//! OpenAI-compatible provider implementation.
//!
//! [`OpenAiCompatProvider`] works with any API that follows the OpenAI chat
//! completion format. This covers OpenAI, Anthropic (via their OpenAI-compat
//! endpoint), Groq, DeepSeek, Mistral, Together AI, OpenRouter, and many more.

use async_trait::async_trait;
use tracing::{debug, warn};

use crate::config::ProviderConfig;
use crate::error::{ProviderError, Result};
use crate::provider::Provider;
use crate::types::{ChatRequest, ChatResponse};

/// An LLM provider that uses the OpenAI-compatible chat completion API.
///
/// This is the primary provider implementation in clawft-llm. It can be
/// configured to talk to any endpoint that accepts the OpenAI request format
/// by changing the `base_url` in the [`ProviderConfig`].
///
/// # Construction
///
/// ```rust,ignore
/// use clawft_llm::{OpenAiCompatProvider, ProviderConfig};
///
/// let config = ProviderConfig {
///     name: "openai".into(),
///     base_url: "https://api.openai.com/v1".into(),
///     api_key_env: "OPENAI_API_KEY".into(),
///     model_prefix: Some("openai/".into()),
///     default_model: Some("gpt-4o".into()),
///     headers: Default::default(),
/// };
/// let provider = OpenAiCompatProvider::new(config);
/// ```
pub struct OpenAiCompatProvider {
    config: ProviderConfig,
    http: reqwest::Client,
    api_key: Option<String>,
}

impl OpenAiCompatProvider {
    /// Create a new provider from configuration.
    ///
    /// The API key will be resolved from the environment variable specified
    /// in `config.api_key_env` at request time.
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            api_key: None,
        }
    }

    /// Create a new provider with an explicit API key.
    ///
    /// This bypasses environment variable lookup and uses the provided key
    /// directly.
    pub fn with_api_key(config: ProviderConfig, api_key: String) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            api_key: Some(api_key),
        }
    }

    /// Returns the provider configuration.
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    /// Returns the chat completions endpoint URL.
    fn completions_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{base}/chat/completions")
    }

    /// Resolve the API key: explicit key > environment variable.
    fn resolve_api_key(&self) -> Result<String> {
        if let Some(ref key) = self.api_key {
            return Ok(key.clone());
        }
        std::env::var(&self.config.api_key_env).map_err(|_| {
            ProviderError::NotConfigured(format!("set {} env var", self.config.api_key_env))
        })
    }
}

#[async_trait]
impl Provider for OpenAiCompatProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let api_key = self.resolve_api_key()?;
        let url = self.completions_url();

        debug!(
            provider = %self.config.name,
            model = %request.model,
            messages = request.messages.len(),
            "sending chat completion request"
        );

        let mut req = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json");

        for (k, v) in &self.config.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let response = req.json(request).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                // Try to parse retry-after from the response body
                let retry_ms = parse_retry_after_ms(&body).unwrap_or(1000);
                warn!(
                    provider = %self.config.name,
                    retry_after_ms = retry_ms,
                    "rate limited"
                );
                return Err(ProviderError::RateLimited {
                    retry_after_ms: retry_ms,
                });
            }

            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ProviderError::AuthFailed(body));
            }

            if status.as_u16() == 404 {
                return Err(ProviderError::ModelNotFound(format!(
                    "model '{}': {}",
                    request.model, body
                )));
            }

            return Err(ProviderError::RequestFailed(format!(
                "HTTP {status}: {body}"
            )));
        }

        let chat_response: ChatResponse = response.json().await.map_err(|e| {
            ProviderError::InvalidResponse(format!("failed to parse response: {e}"))
        })?;

        debug!(
            provider = %self.config.name,
            model = %chat_response.model,
            choices = chat_response.choices.len(),
            "chat completion response received"
        );

        Ok(chat_response)
    }
}

/// Try to extract a retry-after value from a JSON error response body.
fn parse_retry_after_ms(body: &str) -> Option<u64> {
    // Some providers include retry_after or retry_after_ms in the error JSON
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("retry_after_ms")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            value
                .get("retry_after")
                .and_then(|v| v.as_f64())
                .map(|secs| (secs * 1000.0) as u64)
        })
}

impl std::fmt::Debug for OpenAiCompatProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatProvider")
            .field("name", &self.config.name)
            .field("base_url", &self.config.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProviderConfig;
    use std::collections::HashMap;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            name: "test-provider".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key_env: "TEST_PROVIDER_API_KEY".into(),
            model_prefix: Some("test/".into()),
            default_model: Some("test-model".into()),
            headers: HashMap::new(),
        }
    }

    fn config_with_headers() -> ProviderConfig {
        ProviderConfig {
            name: "anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            api_key_env: "ANTHROPIC_API_KEY".into(),
            model_prefix: Some("anthropic/".into()),
            default_model: None,
            headers: HashMap::from([("anthropic-version".into(), "2023-06-01".into())]),
        }
    }

    #[test]
    fn new_provider() {
        let provider = OpenAiCompatProvider::new(test_config());
        assert_eq!(provider.name(), "test-provider");
        assert!(provider.api_key.is_none());
    }

    #[test]
    fn with_api_key_provider() {
        let provider = OpenAiCompatProvider::with_api_key(test_config(), "sk-test123".into());
        assert_eq!(provider.name(), "test-provider");
        assert_eq!(provider.api_key.as_deref(), Some("sk-test123"));
    }

    #[test]
    fn completions_url_construction() {
        let provider = OpenAiCompatProvider::new(test_config());
        assert_eq!(
            provider.completions_url(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn completions_url_strips_trailing_slash() {
        let mut config = test_config();
        config.base_url = "https://api.example.com/v1/".into();
        let provider = OpenAiCompatProvider::new(config);
        assert_eq!(
            provider.completions_url(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn resolve_api_key_explicit() {
        let provider = OpenAiCompatProvider::with_api_key(test_config(), "sk-explicit".into());
        let key = provider.resolve_api_key().unwrap();
        assert_eq!(key, "sk-explicit");
    }

    #[test]
    fn resolve_api_key_from_env() {
        // Use a unique env var name to avoid conflicts with real env
        let env_var = "CLAWFT_TEST_RESOLVE_KEY_12345";
        let mut config = test_config();
        config.api_key_env = env_var.into();

        // Set env var for this test
        unsafe { std::env::set_var(env_var, "sk-from-env") };
        let provider = OpenAiCompatProvider::new(config);
        let key = provider.resolve_api_key().unwrap();
        assert_eq!(key, "sk-from-env");

        // Clean up
        unsafe { std::env::remove_var(env_var) };
    }

    #[test]
    fn resolve_api_key_missing() {
        let mut config = test_config();
        config.api_key_env = "CLAWFT_NONEXISTENT_KEY_98765".into();
        let provider = OpenAiCompatProvider::new(config);
        let err = provider.resolve_api_key().unwrap_err();
        assert!(matches!(err, ProviderError::NotConfigured(_)));
        assert!(err.to_string().contains("CLAWFT_NONEXISTENT_KEY_98765"));
    }

    #[test]
    fn config_accessor() {
        let config = test_config();
        let provider = OpenAiCompatProvider::new(config.clone());
        assert_eq!(provider.config().name, "test-provider");
        assert_eq!(provider.config().base_url, "https://api.example.com/v1");
    }

    #[test]
    fn provider_with_headers_config() {
        let provider = OpenAiCompatProvider::new(config_with_headers());
        assert_eq!(provider.config().headers.len(), 1);
        assert_eq!(
            provider.config().headers.get("anthropic-version"),
            Some(&"2023-06-01".to_string())
        );
    }

    #[test]
    fn debug_hides_api_key() {
        let provider = OpenAiCompatProvider::with_api_key(test_config(), "sk-secret-key".into());
        let debug_str = format!("{:?}", provider);
        assert!(!debug_str.contains("sk-secret-key"));
        assert!(debug_str.contains("***"));
    }

    #[test]
    fn debug_shows_none_for_missing_key() {
        let provider = OpenAiCompatProvider::new(test_config());
        let debug_str = format!("{:?}", provider);
        assert!(debug_str.contains("None"));
    }

    #[test]
    fn parse_retry_after_ms_from_ms_field() {
        let body = r#"{"retry_after_ms": 2500}"#;
        assert_eq!(parse_retry_after_ms(body), Some(2500));
    }

    #[test]
    fn parse_retry_after_ms_from_seconds_field() {
        let body = r#"{"retry_after": 3.5}"#;
        assert_eq!(parse_retry_after_ms(body), Some(3500));
    }

    #[test]
    fn parse_retry_after_ms_missing() {
        let body = r#"{"error": "rate limited"}"#;
        assert_eq!(parse_retry_after_ms(body), None);
    }

    #[test]
    fn parse_retry_after_ms_invalid_json() {
        assert_eq!(parse_retry_after_ms("not json"), None);
    }
}
