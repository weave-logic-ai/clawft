//! Bridge between the pipeline's [`LlmProvider`] trait and `clawft-llm`'s
//! [`Provider`](clawft_llm::Provider) trait.
//!
//! The pipeline system uses [`LlmProvider`] (defined in [`super::transport`]),
//! which operates on raw `serde_json::Value` messages and responses. The
//! `clawft-llm` crate uses typed [`ChatRequest`](clawft_llm::ChatRequest) and
//! [`ChatResponse`](clawft_llm::ChatResponse).
//!
//! This module provides:
//!
//! - [`ClawftLlmAdapter`] -- wraps an `Arc<dyn clawft_llm::Provider>` and
//!   implements [`LlmProvider`], converting between the two type systems.
//!
//! - [`create_adapter_from_config`] -- factory that resolves the right provider
//!   from a [`Config`] and returns it as `Arc<dyn LlmProvider>`.
//!
//! - [`build_live_pipeline`] -- constructs a full [`PipelineRegistry`] wired
//!   with a real LLM transport.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use clawft_llm::{
    ChatMessage, ChatRequest as LlmChatRequest, ChatResponse, OpenAiCompatProvider,
    ProviderConfig as LlmProviderConfig, ProviderRouter,
};
use clawft_types::config::Config;

use super::assembler::TokenBudgetAssembler;
use super::classifier::KeywordClassifier;
use super::learner::NoopLearner;
use super::router::StaticRouter;
use super::scorer::NoopScorer;
use super::traits::{Pipeline, PipelineRegistry};
use super::transport::{LlmProvider, OpenAiCompatTransport};

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Adapts a [`clawft_llm::Provider`] into the pipeline's [`LlmProvider`] trait.
///
/// The adapter handles two conversions on every call:
///
/// 1. **Inbound**: `&[serde_json::Value]` messages are converted to
///    `Vec<ChatMessage>`, and the remaining scalar parameters are packed into a
///    [`LlmChatRequest`].
///
/// 2. **Outbound**: The [`ChatResponse`] is serialized back into OpenAI-format
///    `serde_json::Value` so the existing [`OpenAiCompatTransport`] response
///    parser can consume it.
pub struct ClawftLlmAdapter {
    /// The wrapped clawft-llm provider.
    provider: Arc<dyn clawft_llm::Provider>,
}

impl ClawftLlmAdapter {
    /// Wrap a provider in the adapter.
    pub fn new(provider: Arc<dyn clawft_llm::Provider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl LlmProvider for ClawftLlmAdapter {
    async fn complete(
        &self,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        max_tokens: Option<i32>,
        temperature: Option<f64>,
    ) -> Result<serde_json::Value, String> {
        // -- Inbound conversion: Value messages -> ChatMessage ---------------
        let chat_messages: Vec<ChatMessage> = messages.iter().map(convert_value_to_message).collect();

        let request = LlmChatRequest {
            model: model.to_string(),
            messages: chat_messages,
            max_tokens,
            temperature,
            tools: tools.to_vec(),
            stream: None,
        };

        debug!(
            provider = %self.provider.name(),
            model = %model,
            messages = request.messages.len(),
            tools = request.tools.len(),
            "adapter forwarding request to clawft-llm provider"
        );

        // -- Call the underlying provider ------------------------------------
        let response = self
            .provider
            .complete(&request)
            .await
            .map_err(|e| e.to_string())?;

        // -- Outbound conversion: ChatResponse -> Value ----------------------
        Ok(convert_response_to_value(&response))
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value` message into a [`ChatMessage`].
///
/// Extracts `role`, `content`, `tool_call_id`, and `tool_calls` fields.
fn convert_value_to_message(value: &serde_json::Value) -> ChatMessage {
    let role = value["role"]
        .as_str()
        .unwrap_or("user")
        .to_string();
    let content = value["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let tool_call_id = value
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let tool_calls = value
        .get("tool_calls")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    ChatMessage {
        role,
        content,
        tool_call_id,
        tool_calls,
    }
}

/// Convert a [`ChatResponse`] into an OpenAI-format `serde_json::Value`.
fn convert_response_to_value(response: &ChatResponse) -> serde_json::Value {
    let choices: Vec<serde_json::Value> = response
        .choices
        .iter()
        .map(|c| {
            let mut msg = serde_json::json!({
                "role": c.message.role,
                "content": c.message.content,
            });
            if let Some(ref tcs) = c.message.tool_calls {
                msg["tool_calls"] = serde_json::to_value(tcs).unwrap_or_default();
            }
            serde_json::json!({
                "index": c.index,
                "message": msg,
                "finish_reason": c.finish_reason,
            })
        })
        .collect();

    let usage = response.usage.as_ref().map(|u| {
        serde_json::json!({
            "prompt_tokens": u.prompt_tokens,
            "completion_tokens": u.completion_tokens,
            "total_tokens": u.total_tokens,
        })
    });

    serde_json::json!({
        "id": response.id,
        "model": response.model,
        "choices": choices,
        "usage": usage,
    })
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create an [`LlmProvider`] adapter from application configuration.
///
/// Resolution strategy:
///
/// 1. Parse the model prefix from `config.agents.defaults.model` (e.g.
///    `"anthropic/"` from `"anthropic/claude-opus-4-5"`).
/// 2. Find the matching built-in [`LlmProviderConfig`](LlmProviderConfig)
///    from [`clawft_llm::config::builtin_providers()`].
/// 3. If the application config (`config.providers`) has an API key or
///    base URL override for that provider, apply it.
/// 4. Create an [`OpenAiCompatProvider`] and wrap it in a [`ClawftLlmAdapter`].
///
/// Falls back to the first built-in provider (OpenAI) when no prefix matches.
pub fn create_adapter_from_config(config: &Config) -> Arc<dyn LlmProvider> {
    let model = &config.agents.defaults.model;
    let (prefix, _bare_model) = ProviderRouter::strip_prefix(model);

    let builtins = clawft_llm::config::builtin_providers();

    // Find the matching built-in config by provider name prefix.
    let mut provider_config = match &prefix {
        Some(name) => builtins
            .iter()
            .find(|c| c.name == *name)
            .cloned()
            .unwrap_or_else(|| builtins[0].clone()),
        None => builtins[0].clone(),
    };

    // Apply overrides from the application config's providers section.
    apply_config_overrides(&mut provider_config, config, prefix.as_deref());

    debug!(
        provider = %provider_config.name,
        base_url = %provider_config.base_url,
        model = %model,
        "creating LLM adapter from config"
    );

    let provider = OpenAiCompatProvider::new(provider_config);
    Arc::new(ClawftLlmAdapter::new(Arc::new(provider)))
}

/// Apply API key and base URL overrides from the application config to a
/// built-in provider config.
///
/// The application config stores provider credentials in
/// `config.providers.<name>`, where each entry has `api_key` and optionally
/// `api_base`. If present, these override the built-in defaults.
fn apply_config_overrides(
    llm_config: &mut LlmProviderConfig,
    app_config: &Config,
    provider_name: Option<&str>,
) {
    let name = provider_name.unwrap_or(&llm_config.name);

    // Look up the matching provider in the app config.
    let app_provider = match name {
        "openai" => &app_config.providers.openai,
        "anthropic" => &app_config.providers.anthropic,
        "groq" => &app_config.providers.groq,
        "deepseek" => &app_config.providers.deepseek,
        "openrouter" => &app_config.providers.openrouter,
        _ => return,
    };

    // Override base URL if provided.
    if let Some(ref base) = app_provider.api_base
        && !base.is_empty()
    {
        llm_config.base_url = base.clone();
    }

    // Merge extra headers if provided.
    if let Some(ref headers) = app_provider.extra_headers {
        for (k, v) in headers {
            llm_config.headers.insert(k.clone(), v.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Pipeline construction
// ---------------------------------------------------------------------------

/// Build a pipeline registry backed by a real LLM provider.
///
/// This is the production counterpart of
/// [`build_default_pipeline`](crate::bootstrap::build_default_pipeline).
/// Instead of a stub transport, it uses [`OpenAiCompatTransport::with_provider`]
/// with an adapter created from the application config.
pub fn build_live_pipeline(config: &Config) -> PipelineRegistry {
    let classifier = Arc::new(KeywordClassifier::new());
    let router = Arc::new(StaticRouter::from_config(&config.agents));
    let assembler = Arc::new(TokenBudgetAssembler::new(
        config.agents.defaults.max_tokens.max(1) as usize,
    ));
    let adapter = create_adapter_from_config(config);
    let transport = Arc::new(OpenAiCompatTransport::with_provider(adapter));
    let scorer = Arc::new(NoopScorer::new());
    let learner = Arc::new(NoopLearner::new());

    let pipeline = Pipeline {
        classifier,
        router,
        assembler,
        transport,
        scorer,
        learner,
    };

    PipelineRegistry::new(pipeline)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_llm::types::{Choice, FunctionCall, ToolCall, Usage as LlmUsage};
    use clawft_types::config::{AgentDefaults, AgentsConfig};

    fn test_config() -> Config {
        Config {
            agents: AgentsConfig {
                defaults: AgentDefaults {
                    workspace: "~/.clawft/workspace".into(),
                    model: "anthropic/claude-opus-4-5".into(),
                    max_tokens: 4096,
                    temperature: 0.7,
                    max_tool_iterations: 10,
                    memory_window: 50,
                },
            },
            ..Config::default()
        }
    }

    // -- convert_value_to_message -------------------------------------------

    #[test]
    fn adapter_converts_messages() {
        let value = serde_json::json!({
            "role": "user",
            "content": "Hello, world!",
        });
        let msg = convert_value_to_message(&value);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.tool_call_id.is_none());
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn adapter_converts_message_with_tool_call_id() {
        let value = serde_json::json!({
            "role": "tool",
            "content": "result data",
            "tool_call_id": "call-123",
        });
        let msg = convert_value_to_message(&value);
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.content, "result data");
        assert_eq!(msg.tool_call_id.as_deref(), Some("call-123"));
    }

    #[test]
    fn adapter_converts_message_defaults() {
        // Missing role and content should fall back to defaults.
        let value = serde_json::json!({});
        let msg = convert_value_to_message(&value);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "");
    }

    // -- convert_response_to_value ------------------------------------------

    fn make_text_response() -> ChatResponse {
        ChatResponse {
            id: "resp-1".into(),
            model: "claude-opus-4-5".into(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage::assistant("Hello from the LLM!"),
                finish_reason: Some("stop".into()),
            }],
            usage: Some(LlmUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        }
    }

    #[test]
    fn adapter_converts_response() {
        let response = make_text_response();
        let value = convert_response_to_value(&response);

        assert_eq!(value["id"], "resp-1");
        assert_eq!(value["model"], "claude-opus-4-5");

        let choice = &value["choices"][0];
        assert_eq!(choice["index"], 0);
        assert_eq!(choice["message"]["role"], "assistant");
        assert_eq!(choice["message"]["content"], "Hello from the LLM!");
        assert_eq!(choice["finish_reason"], "stop");

        let usage = &value["usage"];
        assert_eq!(usage["prompt_tokens"], 10);
        assert_eq!(usage["completion_tokens"], 5);
        assert_eq!(usage["total_tokens"], 15);
    }

    #[test]
    fn adapter_converts_response_no_usage() {
        let response = ChatResponse {
            id: "resp-no-usage".into(),
            model: "test-model".into(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage::assistant("ok"),
                finish_reason: Some("stop".into()),
            }],
            usage: None,
        };
        let value = convert_response_to_value(&response);
        assert!(value["usage"].is_null());
    }

    // -- tool call round-trip -----------------------------------------------

    #[test]
    fn adapter_handles_tool_calls() {
        // Build a response with tool calls.
        let response = ChatResponse {
            id: "resp-tc".into(),
            model: "test-model".into(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_call_id: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "call_abc".into(),
                        call_type: "function".into(),
                        function: FunctionCall {
                            name: "get_weather".into(),
                            arguments: r#"{"city":"London"}"#.into(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".into()),
            }],
            usage: Some(LlmUsage {
                prompt_tokens: 15,
                completion_tokens: 8,
                total_tokens: 23,
            }),
        };

        let value = convert_response_to_value(&response);

        // Verify tool calls are present in the JSON output.
        let tool_calls = value["choices"][0]["message"]["tool_calls"]
            .as_array()
            .expect("tool_calls should be an array");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "call_abc");
        assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
        assert_eq!(
            tool_calls[0]["function"]["arguments"],
            r#"{"city":"London"}"#
        );

        // Now convert the tool_calls back to a message Value and re-parse.
        let msg_value = &value["choices"][0]["message"];
        let round_tripped = convert_value_to_message(msg_value);
        assert_eq!(round_tripped.role, "assistant");
        let tcs = round_tripped.tool_calls.expect("should have tool_calls");
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].id, "call_abc");
        assert_eq!(tcs[0].function.name, "get_weather");
    }

    // -- error propagation --------------------------------------------------

    struct FailingProvider;

    #[async_trait]
    impl clawft_llm::Provider for FailingProvider {
        fn name(&self) -> &str {
            "failing"
        }
        async fn complete(
            &self,
            _request: &LlmChatRequest,
        ) -> clawft_llm::Result<ChatResponse> {
            Err(clawft_llm::ProviderError::RequestFailed(
                "simulated network failure".into(),
            ))
        }
    }

    #[tokio::test]
    async fn adapter_maps_errors() {
        let adapter = ClawftLlmAdapter::new(Arc::new(FailingProvider));
        let result = adapter
            .complete("test-model", &[], &[], None, None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("simulated network failure"),
            "error should propagate: {err}"
        );
    }

    // -- successful round-trip through adapter ------------------------------

    struct EchoProvider;

    #[async_trait]
    impl clawft_llm::Provider for EchoProvider {
        fn name(&self) -> &str {
            "echo"
        }
        async fn complete(
            &self,
            request: &LlmChatRequest,
        ) -> clawft_llm::Result<ChatResponse> {
            let content = request
                .messages
                .last()
                .map(|m| format!("echo: {}", m.content))
                .unwrap_or_else(|| "echo: (empty)".into());
            Ok(ChatResponse {
                id: "echo-resp".into(),
                model: request.model.clone(),
                choices: vec![Choice {
                    index: 0,
                    message: ChatMessage::assistant(content),
                    finish_reason: Some("stop".into()),
                }],
                usage: Some(LlmUsage {
                    prompt_tokens: 5,
                    completion_tokens: 3,
                    total_tokens: 8,
                }),
            })
        }
    }

    #[tokio::test]
    async fn adapter_complete_round_trip() {
        let adapter = ClawftLlmAdapter::new(Arc::new(EchoProvider));

        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "ping",
        })];

        let result = adapter
            .complete("test-model", &messages, &[], Some(100), Some(0.5))
            .await
            .unwrap();

        assert_eq!(result["id"], "echo-resp");
        assert_eq!(result["model"], "test-model");
        assert_eq!(
            result["choices"][0]["message"]["content"],
            "echo: ping"
        );
        assert_eq!(result["usage"]["total_tokens"], 8);
    }

    // -- factory tests ------------------------------------------------------

    #[test]
    fn create_adapter_returns_provider() {
        let config = test_config();
        let adapter = create_adapter_from_config(&config);
        // The adapter should be created without panicking.
        // We cannot call complete() without a real API key, but we can
        // verify the Arc is valid by checking it exists.
        let _ = Arc::strong_count(&adapter);
    }

    #[test]
    fn create_adapter_with_default_config() {
        let config = Config::default();
        let adapter = create_adapter_from_config(&config);
        let _ = Arc::strong_count(&adapter);
    }

    #[test]
    fn create_adapter_unknown_prefix() {
        let mut config = test_config();
        config.agents.defaults.model = "unknown-provider/some-model".into();
        // Should fall back to the first built-in (openai) without panicking.
        let adapter = create_adapter_from_config(&config);
        let _ = Arc::strong_count(&adapter);
    }

    #[test]
    fn create_adapter_no_prefix() {
        let mut config = test_config();
        config.agents.defaults.model = "gpt-4o".into();
        // No prefix -- should use the default (openai) provider.
        let adapter = create_adapter_from_config(&config);
        let _ = Arc::strong_count(&adapter);
    }

    // -- build_live_pipeline ------------------------------------------------

    #[test]
    fn build_live_pipeline_is_configured() {
        let config = test_config();
        let _registry = build_live_pipeline(&config);
        // Should not panic; the pipeline is fully wired.
    }

    #[test]
    fn build_live_pipeline_with_defaults() {
        let config = Config::default();
        let _registry = build_live_pipeline(&config);
    }

    // -- apply_config_overrides ---------------------------------------------

    #[test]
    fn overrides_apply_base_url() {
        let mut config = test_config();
        config.providers.anthropic.api_base = Some("https://custom.proxy.com/v1".into());

        let builtins = clawft_llm::config::builtin_providers();
        let mut llm_config = builtins
            .iter()
            .find(|c| c.name == "anthropic")
            .cloned()
            .unwrap();

        apply_config_overrides(&mut llm_config, &config, Some("anthropic"));
        assert_eq!(llm_config.base_url, "https://custom.proxy.com/v1");
    }

    #[test]
    fn overrides_skip_empty_base_url() {
        let config = test_config();

        let builtins = clawft_llm::config::builtin_providers();
        let mut llm_config = builtins
            .iter()
            .find(|c| c.name == "openai")
            .cloned()
            .unwrap();

        let original_url = llm_config.base_url.clone();
        apply_config_overrides(&mut llm_config, &config, Some("openai"));
        // No override was set, so the URL should remain unchanged.
        assert_eq!(llm_config.base_url, original_url);
    }

    #[test]
    fn overrides_merge_headers() {
        let mut config = test_config();
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Custom".into(), "value".into());
        config.providers.anthropic.extra_headers = Some(headers);

        let builtins = clawft_llm::config::builtin_providers();
        let mut llm_config = builtins
            .iter()
            .find(|c| c.name == "anthropic")
            .cloned()
            .unwrap();

        apply_config_overrides(&mut llm_config, &config, Some("anthropic"));
        // Should have both the original anthropic-version header and the custom one.
        assert_eq!(llm_config.headers.get("X-Custom").unwrap(), "value");
        assert!(llm_config.headers.contains_key("anthropic-version"));
    }

    // -- end-to-end: MockProvider -> ClawftLlmAdapter -> OpenAiCompatTransport -

    /// End-to-end round-trip test that exercises the full adapter-to-transport path.
    ///
    /// 1. Creates a mock `clawft_llm::Provider` (EchoProvider).
    /// 2. Wraps it in `ClawftLlmAdapter`.
    /// 3. Creates `OpenAiCompatTransport::with_provider(adapter)`.
    /// 4. Constructs a `TransportRequest`.
    /// 5. Calls `transport.complete()` and verifies the `LlmResponse`.
    #[tokio::test]
    async fn transport_adapter_round_trip() {
        use crate::pipeline::traits::{LlmMessage, LlmTransport, TransportRequest};
        use clawft_types::provider::{ContentBlock, StopReason};

        // 1. Create mock provider
        let echo_provider: Arc<dyn clawft_llm::Provider> = Arc::new(EchoProvider);

        // 2. Wrap in adapter
        let adapter: Arc<dyn LlmProvider> = Arc::new(ClawftLlmAdapter::new(echo_provider));

        // 3. Create transport with the adapter
        let transport = OpenAiCompatTransport::with_provider(adapter);
        assert!(transport.is_configured());

        // 4. Build a TransportRequest
        let request = TransportRequest {
            provider: "echo".into(),
            model: "echo-model".into(),
            messages: vec![
                LlmMessage {
                    role: "system".into(),
                    content: "You are a test assistant.".into(),
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "integration test".into(),
                    tool_call_id: None,
                },
            ],
            tools: vec![],
            max_tokens: Some(100),
            temperature: Some(0.5),
        };

        // 5. Call complete and verify
        let response = transport.complete(&request).await
            .expect("transport round-trip should succeed with mock provider");

        assert_eq!(response.id, "echo-resp");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
        assert_eq!(response.usage.input_tokens, 5);
        assert_eq!(response.usage.output_tokens, 3);

        // Verify the content block contains the echoed message.
        assert!(!response.content.is_empty());
        match &response.content[0] {
            ContentBlock::Text { text } => {
                assert_eq!(text, "echo: integration test");
            }
            other => panic!("expected Text block, got: {other:?}"),
        }
    }

    /// End-to-end test with a failing provider through the transport layer.
    #[tokio::test]
    async fn transport_adapter_error_propagation() {
        use crate::pipeline::traits::{LlmMessage, LlmTransport, TransportRequest};

        let failing_provider: Arc<dyn clawft_llm::Provider> = Arc::new(FailingProvider);
        let adapter: Arc<dyn LlmProvider> = Arc::new(ClawftLlmAdapter::new(failing_provider));
        let transport = OpenAiCompatTransport::with_provider(adapter);

        let request = TransportRequest {
            provider: "failing".into(),
            model: "test".into(),
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "this should fail".into(),
                tool_call_id: None,
            }],
            tools: vec![],
            max_tokens: None,
            temperature: None,
        };

        let result = transport.complete(&request).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("simulated network failure"),
            "error should propagate through adapter+transport: {err_msg}"
        );
    }

    /// End-to-end test: adapter wraps a provider with tool call responses,
    /// verifying the full chain: Provider -> Adapter -> Transport -> LlmResponse.
    #[tokio::test]
    async fn transport_adapter_tool_call_round_trip() {
        use crate::pipeline::traits::{LlmMessage, LlmTransport, TransportRequest};
        use clawft_types::provider::{ContentBlock, StopReason};

        // Provider that returns tool calls.
        struct ToolCallProvider;

        #[async_trait]
        impl clawft_llm::Provider for ToolCallProvider {
            fn name(&self) -> &str { "tool-call-provider" }
            async fn complete(
                &self,
                _request: &LlmChatRequest,
            ) -> clawft_llm::Result<ChatResponse> {
                Ok(ChatResponse {
                    id: "tc-resp".into(),
                    model: "test-model".into(),
                    choices: vec![Choice {
                        index: 0,
                        message: ChatMessage {
                            role: "assistant".into(),
                            content: String::new(),
                            tool_call_id: None,
                            tool_calls: Some(vec![ToolCall {
                                id: "call_xyz".into(),
                                call_type: "function".into(),
                                function: FunctionCall {
                                    name: "web_search".into(),
                                    arguments: r#"{"query":"rust lang"}"#.into(),
                                },
                            }]),
                        },
                        finish_reason: Some("tool_calls".into()),
                    }],
                    usage: Some(LlmUsage {
                        prompt_tokens: 20,
                        completion_tokens: 10,
                        total_tokens: 30,
                    }),
                })
            }
        }

        let provider: Arc<dyn clawft_llm::Provider> = Arc::new(ToolCallProvider);
        let adapter: Arc<dyn LlmProvider> = Arc::new(ClawftLlmAdapter::new(provider));
        let transport = OpenAiCompatTransport::with_provider(adapter);

        let request = TransportRequest {
            provider: "tool-call-provider".into(),
            model: "test-model".into(),
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "search for rust".into(),
                tool_call_id: None,
            }],
            tools: vec![serde_json::json!({"type": "function", "name": "web_search"})],
            max_tokens: Some(100),
            temperature: None,
        };

        let response = transport.complete(&request).await
            .expect("tool call round-trip should succeed");

        assert_eq!(response.id, "tc-resp");
        assert_eq!(response.stop_reason, StopReason::ToolUse);
        assert_eq!(response.usage.input_tokens, 20);
        assert_eq!(response.usage.output_tokens, 10);

        // Should have a ToolUse content block (empty text is skipped).
        assert_eq!(response.content.len(), 1);
        match &response.content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_xyz");
                assert_eq!(name, "web_search");
                assert_eq!(input["query"], "rust lang");
            }
            other => panic!("expected ToolUse block, got: {other:?}"),
        }
    }
}
