//! Provider configuration types and built-in provider definitions.
//!
//! Each [`ProviderConfig`] describes how to connect to an LLM provider:
//! the base URL, API key environment variable, model prefix for routing,
//! and any extra headers needed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single LLM provider endpoint.
///
/// Renamed from `ProviderConfig` to avoid collision with
/// `clawft_types::config::ProviderConfig` (the user-facing config type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Human-readable provider name (e.g. "openai", "anthropic").
    pub name: String,

    /// Base URL for the OpenAI-compatible API (e.g. "https://api.openai.com/v1").
    pub base_url: String,

    /// Environment variable that holds the API key (e.g. "OPENAI_API_KEY").
    pub api_key_env: String,

    /// Prefix used for routing model names to this provider (e.g. "openai/").
    /// A model string like "openai/gpt-4o" is routed to this provider and the
    /// prefix is stripped before sending the request.
    #[serde(default)]
    pub model_prefix: Option<String>,

    /// Default model to use when none is specified.
    #[serde(default)]
    pub default_model: Option<String>,

    /// Extra HTTP headers to include in every request to this provider.
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request timeout in seconds. Defaults to 120.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Returns the built-in provider configurations.
///
/// These cover the most common OpenAI-compatible providers. Users can
/// override or extend these via their clawft configuration file.
pub fn builtin_providers() -> Vec<LlmProviderConfig> {
    vec![
        LlmProviderConfig {
            name: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key_env: "OPENAI_API_KEY".into(),
            model_prefix: Some("openai/".into()),
            default_model: Some("gpt-4o".into()),
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "anthropic".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            api_key_env: "ANTHROPIC_API_KEY".into(),
            model_prefix: Some("anthropic/".into()),
            default_model: Some("claude-sonnet-4-5-20250514".into()),
            headers: HashMap::from([("anthropic-version".into(), "2023-06-01".into())]),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            api_key_env: "GROQ_API_KEY".into(),
            model_prefix: Some("groq/".into()),
            default_model: Some("llama-3.1-70b-versatile".into()),
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "deepseek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            api_key_env: "DEEPSEEK_API_KEY".into(),
            model_prefix: Some("deepseek/".into()),
            default_model: Some("deepseek-chat".into()),
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "mistral".into(),
            base_url: "https://api.mistral.ai/v1".into(),
            api_key_env: "MISTRAL_API_KEY".into(),
            model_prefix: Some("mistral/".into()),
            default_model: Some("mistral-large-latest".into()),
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "together".into(),
            base_url: "https://api.together.xyz/v1".into(),
            api_key_env: "TOGETHER_API_KEY".into(),
            model_prefix: Some("together/".into()),
            default_model: None,
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            api_key_env: "OPENROUTER_API_KEY".into(),
            model_prefix: Some("openrouter/".into()),
            default_model: None,
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "gemini".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            api_key_env: "GOOGLE_GEMINI_API_KEY".into(),
            model_prefix: Some("gemini/".into()),
            default_model: Some("gemini-2.5-flash".into()),
            headers: HashMap::new(),
            timeout_secs: None,
        },
        LlmProviderConfig {
            name: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            api_key_env: "XAI_API_KEY".into(),
            model_prefix: Some("xai/".into()),
            default_model: Some("grok-3-mini".into()),
            headers: HashMap::new(),
            timeout_secs: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_providers_count() {
        let providers = builtin_providers();
        assert_eq!(providers.len(), 9);
    }

    #[test]
    fn builtin_providers_names() {
        let providers = builtin_providers();
        let names: Vec<&str> = providers.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"openai"));
        assert!(names.contains(&"anthropic"));
        assert!(names.contains(&"groq"));
        assert!(names.contains(&"deepseek"));
        assert!(names.contains(&"mistral"));
        assert!(names.contains(&"together"));
        assert!(names.contains(&"openrouter"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"xai"));
    }

    #[test]
    fn builtin_openai_config() {
        let providers = builtin_providers();
        let openai = providers.iter().find(|p| p.name == "openai").unwrap();
        assert_eq!(openai.base_url, "https://api.openai.com/v1");
        assert_eq!(openai.api_key_env, "OPENAI_API_KEY");
        assert_eq!(openai.model_prefix.as_deref(), Some("openai/"));
        assert_eq!(openai.default_model.as_deref(), Some("gpt-4o"));
        assert!(openai.headers.is_empty());
    }

    #[test]
    fn builtin_anthropic_has_version_header() {
        let providers = builtin_providers();
        let anthropic = providers.iter().find(|p| p.name == "anthropic").unwrap();
        assert_eq!(
            anthropic.headers.get("anthropic-version"),
            Some(&"2023-06-01".to_string())
        );
    }

    #[test]
    fn all_builtins_have_prefixes() {
        let providers = builtin_providers();
        for p in &providers {
            assert!(
                p.model_prefix.is_some(),
                "Provider {} should have a model_prefix",
                p.name
            );
            let prefix = p.model_prefix.as_ref().unwrap();
            assert!(
                prefix.ends_with('/'),
                "Prefix for {} should end with /",
                p.name
            );
        }
    }

    #[test]
    fn all_builtins_have_api_key_env() {
        let providers = builtin_providers();
        for p in &providers {
            assert!(
                !p.api_key_env.is_empty(),
                "Provider {} must have api_key_env",
                p.name
            );
            assert!(
                p.api_key_env.ends_with("_KEY") || p.api_key_env.ends_with("_API_KEY"),
                "api_key_env for {} should follow naming convention",
                p.name
            );
        }
    }

    #[test]
    fn provider_config_serde_roundtrip() {
        let config = LlmProviderConfig {
            name: "test-provider".into(),
            base_url: "https://example.com/v1".into(),
            api_key_env: "TEST_API_KEY".into(),
            model_prefix: Some("test/".into()),
            default_model: Some("test-model".into()),
            headers: HashMap::from([("X-Custom".into(), "value".into())]),
            timeout_secs: Some(60),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: LlmProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, config.name);
        assert_eq!(parsed.base_url, config.base_url);
        assert_eq!(parsed.api_key_env, config.api_key_env);
        assert_eq!(parsed.model_prefix, config.model_prefix);
        assert_eq!(parsed.default_model, config.default_model);
        assert_eq!(parsed.headers, config.headers);
    }

    #[test]
    fn provider_config_deserialize_minimal() {
        let json = r#"{
            "name": "minimal",
            "base_url": "https://example.com",
            "api_key_env": "MINIMAL_KEY"
        }"#;
        let config: LlmProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "minimal");
        assert!(config.model_prefix.is_none());
        assert!(config.default_model.is_none());
        assert!(config.headers.is_empty());
    }
}
