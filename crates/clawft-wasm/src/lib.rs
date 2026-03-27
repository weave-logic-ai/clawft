//! WASM entrypoint for clawft.
//!
//! This crate provides a WebAssembly build of the clawft agent core.
//! It excludes components that require native OS features:
//! - Shell execution tools (exec_shell, spawn)
//! - Channel plugins (Telegram, Slack, Discord)
//! - Native CLI terminal I/O
//! - Process spawning
//!
//! # Platform Support
//!
//! The WASM build targets `wasm32-wasip1` and uses WASI preview 1 for:
//! - HTTP outbound (LLM API calls)
//! - Filesystem (config, sessions)
//! - Environment variables
//!
//! # Dependencies
//!
//! This crate is intentionally decoupled from `clawft-core` and `clawft-platform`
//! to avoid pulling in tokio["full"] and reqwest, neither of which compiles for
//! WASM targets. It depends only on `clawft-types`, `serde`, and `serde_json`.
//!
//! # Size Budget
//!
//! Target: < 300 KB uncompressed, < 120 KB gzipped.

#[cfg(feature = "alloc-tracing")]
pub mod alloc_trace;
pub mod allocator;
pub mod env;
pub mod fs;
pub mod http;
pub mod platform;

/// Plugin sandbox enforcement for WASM plugins.
///
/// This module is only available when the `wasm-plugins` feature is enabled.
/// It provides [`sandbox::PluginSandbox`], validation functions for HTTP,
/// filesystem, and environment access, plus rate limiting and size enforcement.
#[cfg(feature = "wasm-plugins")]
pub mod sandbox;

/// Audit logging for WASM plugin host function calls.
///
/// Every host function invocation is recorded in a per-plugin audit log.
/// This provides a tamper-evident record of all side-effecting operations.
#[cfg(feature = "wasm-plugins")]
pub mod audit;

/// WASM plugin engine with fuel metering and memory limits.
///
/// Provides [`engine::WasmPluginEngine`] -- the host-side runtime for loading
/// and executing WASM plugin modules with configurable resource limits.
#[cfg(feature = "wasm-plugins")]
pub mod engine;

/// Permission persistence and approval for WASM plugin upgrades.
///
/// Provides [`permission_store::PermissionStore`] for saving/loading approved
/// permissions, and [`permission_store::PermissionApprover`] for requesting
/// user consent when a plugin upgrade introduces new permissions.
#[cfg(feature = "wasm-plugins")]
pub mod permission_store;

pub use platform::WasmPlatform;

/// Version information for the WASM build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the WASM agent.
///
/// Called once when the WASM module is instantiated. Sets up the agent
/// configuration and pipeline from WASI-accessible config files.
///
/// # Returns
///
/// Returns 0 on success, non-zero on failure.
pub fn init() -> i32 {
    // Phase 3A Week 11: Will load config from WASI filesystem
    // and set up the agent pipeline.
    0
}

/// Process a single message through the agent pipeline.
///
/// # Arguments
///
/// * `input` - The user message as a UTF-8 string.
///
/// # Returns
///
/// The agent's response as a string, or an error message.
pub fn process_message(input: &str) -> String {
    // Phase 3A Week 12: Will run the full 6-stage pipeline.
    format!(
        "clawft-wasm v{}: received '{}' (pipeline not yet wired)",
        VERSION, input
    )
}

/// Get the agent's capabilities as a JSON string.
///
/// Returns a JSON object describing available tools, providers,
/// and configuration for this WASM instance.
pub fn capabilities() -> String {
    serde_json::json!({
        "version": VERSION,
        "platform": "wasm32-wasip1",
        "tools": ["read_file", "write_file", "edit_file", "list_directory", "memory_read", "memory_write", "web_fetch", "web_search"],
        "excluded_tools": ["exec_shell", "spawn", "message"],
        "channels": [],
        "status": "initializing"
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Browser WASM entry point (feature = "browser")
// ---------------------------------------------------------------------------

/// Browser entry point module providing wasm-bindgen exports for
/// running clawft in a web browser via the BrowserPlatform.
#[cfg(feature = "browser")]
mod browser_entry {
    use std::sync::OnceLock;

    use clawft_llm::browser_transport::BrowserLlmClient;
    use clawft_llm::config::LlmProviderConfig;
    use clawft_llm::types::{ChatMessage, ChatRequest};
    use clawft_types::config::Config;
    use wasm_bindgen::prelude::*;

    /// Persistent runtime state initialized by `init()`.
    struct BrowserRuntime {
        config: Config,
        client: BrowserLlmClient,
        /// The model name to send in requests (prefix stripped).
        model_name: String,
        /// Conversation history.
        messages: std::sync::Mutex<Vec<ChatMessage>>,
    }

    static RUNTIME: OnceLock<BrowserRuntime> = OnceLock::new();

    /// Look up the user's `ProviderConfig` by provider name.
    fn user_provider_config(
        config: &Config,
        name: &str,
    ) -> clawft_types::config::ProviderConfig {
        match name {
            "anthropic" => config.providers.anthropic.clone(),
            "openai" => config.providers.openai.clone(),
            "openrouter" => config.providers.openrouter.clone(),
            "deepseek" => config.providers.deepseek.clone(),
            "groq" => config.providers.groq.clone(),
            "gemini" => config.providers.gemini.clone(),
            "xai" => config.providers.xai.clone(),
            _ => config.providers.custom.clone(),
        }
    }

    /// Route a model string like "anthropic/claude-sonnet-4-20250514" to the
    /// correct provider config and return (LlmProviderConfig, stripped_model, ProviderConfig).
    ///
    /// Resolution order:
    /// 1. Exact prefix match against builtin providers (e.g. `openrouter/` → OpenRouter).
    /// 2. Fallback: if no prefix matches, route to OpenRouter (or the first provider
    ///    with an API key). The full model string is sent as-is since there's no
    ///    prefix to strip. This handles models like `arcee-ai/trinity-large-preview:free`
    ///    that are hosted on OpenRouter but don't use the `openrouter/` prefix.
    fn resolve_provider(
        config: &Config,
        model: &str,
    ) -> Result<(LlmProviderConfig, String, clawft_types::config::ProviderConfig), String> {
        let builtins = clawft_llm::config::builtin_providers();

        // 1. Find the builtin whose model_prefix matches.
        for builtin in &builtins {
            if let Some(ref prefix) = builtin.model_prefix {
                if model.starts_with(prefix) {
                    let stripped = model[prefix.len()..].to_string();
                    let user_cfg = user_provider_config(config, &builtin.name);

                    // Merge user overrides into the builtin config.
                    let mut llm_cfg = builtin.clone();
                    if let Some(ref base) = user_cfg.api_base {
                        llm_cfg.base_url = base.clone();
                    }
                    if let Some(ref extra) = user_cfg.extra_headers {
                        llm_cfg.headers.extend(extra.clone());
                    }

                    return Ok((llm_cfg, stripped, user_cfg));
                }
            }
        }

        // 2. No prefix matched — fall back to OpenRouter if it has an API key,
        //    since OpenRouter aggregates third-party models with vendor/ prefixes
        //    (e.g. arcee-ai/, meta-llama/, mistralai/).
        let fallback_order = ["openrouter", "openai", "anthropic", "groq", "deepseek", "gemini", "xai"];

        for name in &fallback_order {
            let user_cfg = user_provider_config(config, name);
            if !user_cfg.api_key.expose().is_empty() {
                let builtin = builtins.iter().find(|b| b.name == *name).cloned();
                if let Some(mut llm_cfg) = builtin {
                    if let Some(ref base) = user_cfg.api_base {
                        llm_cfg.base_url = base.clone();
                    }
                    if let Some(ref extra) = user_cfg.extra_headers {
                        llm_cfg.headers.extend(extra.clone());
                    }

                    web_sys::console::log_1(
                        &format!(
                            "[clawft] no prefix match for '{}', falling back to {} provider",
                            model, name
                        )
                        .into(),
                    );

                    // Send the full model string as-is (no prefix to strip).
                    return Ok((llm_cfg, model.to_string(), user_cfg));
                }
            }
        }

        Err(format!(
            "no provider found for model '{}'. Either use a prefixed model like \
             'openrouter/arcee-ai/trinity-large-preview:free', or configure an \
             API key for a provider.",
            model
        ))
    }

    /// Initialize the clawft-wasm browser runtime.
    ///
    /// Parses the provided JSON config and sets up a BrowserLlmClient.
    /// Must be called once before `send_message`.
    #[wasm_bindgen]
    pub async fn init(config_json: &str) -> Result<(), JsValue> {
        console_error_panic_hook::set_once();

        let config: Config = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("config parse error: {e}")))?;

        let _platform = clawft_platform::BrowserPlatform::new();

        let model = &config.agents.defaults.model;
        let (llm_cfg, stripped_model, user_cfg) = resolve_provider(&config, model)
            .map_err(|e| JsValue::from_str(&e))?;

        let api_key = user_cfg.api_key.expose();
        if api_key.is_empty() {
            return Err(JsValue::from_str(&format!(
                "no API key configured for provider matching model '{}'. Set apiKey in the provider config.",
                model
            )));
        }

        web_sys::console::log_1(
            &format!(
                "[clawft] provider={}, cors_proxy={:?}, browser_direct={}, base_url={}",
                llm_cfg.name,
                user_cfg.cors_proxy,
                user_cfg.browser_direct,
                llm_cfg.base_url,
            )
            .into(),
        );

        let client = BrowserLlmClient::with_api_key(
            llm_cfg,
            api_key.to_string(),
            user_cfg.browser_direct,
            user_cfg.cors_proxy.clone(),
        );

        let system_prompt = "You are a helpful AI assistant running in the browser via clawft-wasm.";

        RUNTIME
            .set(BrowserRuntime {
                config,
                client,
                model_name: stripped_model,
                messages: std::sync::Mutex::new(vec![ChatMessage::system(system_prompt)]),
            })
            .map_err(|_| JsValue::from_str("already initialized"))?;

        web_sys::console::log_1(&"clawft-wasm initialized".into());
        Ok(())
    }

    /// Send a message through the clawft LLM pipeline.
    ///
    /// Appends the user message to the conversation history, sends it
    /// to the configured LLM provider via BrowserLlmClient, and returns
    /// the assistant's response.
    #[wasm_bindgen]
    pub async fn send_message(text: &str) -> Result<String, JsValue> {
        let rt = RUNTIME
            .get()
            .ok_or_else(|| JsValue::from_str("not initialized — call init() first"))?;

        // Add user message to history.
        let messages = {
            let mut msgs = rt
                .messages
                .lock()
                .map_err(|e| JsValue::from_str(&format!("lock error: {e}")))?;
            msgs.push(ChatMessage::user(text));
            msgs.clone()
        };

        let max_tokens = rt.config.agents.defaults.max_tokens;

        let request = ChatRequest {
            model: rt.model_name.clone(),
            messages,
            max_tokens: Some(max_tokens),
            temperature: Some(rt.config.agents.defaults.temperature),
            stream: None,
            tools: vec![],
        };

        let response = rt
            .client
            .complete(&request)
            .await
            .map_err(|e| JsValue::from_str(&format!("LLM error: {e}")))?;

        let reply = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "No response from model.".to_string());

        // Add assistant reply to history.
        {
            let mut msgs = rt
                .messages
                .lock()
                .map_err(|e| JsValue::from_str(&format!("lock error: {e}")))?;
            msgs.push(ChatMessage::assistant(&reply));
        }

        Ok(reply)
    }

    /// Set an environment variable on the BrowserPlatform.
    #[wasm_bindgen]
    pub fn set_env(_key: &str, _value: &str) {
        // Browser env vars are managed by BrowserPlatform.env()
    }
}

#[cfg(feature = "browser")]
pub use browser_entry::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_returns_zero() {
        assert_eq!(init(), 0);
    }

    #[test]
    fn process_message_returns_response() {
        let response = process_message("hello");
        assert!(response.contains("hello"));
        assert!(response.contains("clawft-wasm"));
    }

    #[test]
    fn capabilities_is_valid_json() {
        let caps = capabilities();
        let parsed: serde_json::Value = serde_json::from_str(&caps).unwrap();
        assert_eq!(parsed["platform"], "wasm32-wasip1");
        assert!(parsed["tools"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn excluded_tools_listed() {
        let caps = capabilities();
        let parsed: serde_json::Value = serde_json::from_str(&caps).unwrap();
        let excluded = parsed["excluded_tools"].as_array().unwrap();
        assert!(excluded.contains(&serde_json::json!("exec_shell")));
        assert!(excluded.contains(&serde_json::json!("spawn")));
    }
}
