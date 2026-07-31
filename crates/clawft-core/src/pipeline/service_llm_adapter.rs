//! Bridge between the pipeline's [`LlmProvider`] trait and the narrow
//! [`LlmClient`](clawft_service_llm::LlmClient) from
//! [`clawft-service-llm`](clawft_service_llm).
//!
//! ## Why this exists alongside `llm_adapter.rs`
//!
//! There are two LLM stacks in the workspace:
//!
//! - [`clawft_llm`] — a general provider abstraction (OpenAI, Anthropic,
//!   Groq, …) with routing, failover, retry, SSE. Adapter:
//!   [`super::llm_adapter::ClawftLlmAdapter`]. Right when the agent
//!   needs cross-provider routing or remote SaaS endpoints.
//! - [`clawft_service_llm`] — the narrow daemon-resident HTTP client
//!   that talks to a single local `llama-server` OpenAI-compat
//!   endpoint. Adapter: [`ServiceLlmAdapter`] (this module). Right when
//!   the agent should share the same LLM the daemon's `llm.prompt`
//!   RPC and the chat panel already use.
//!
//! Both adapters implement the same [`LlmProvider`] trait, so the rest
//! of the pipeline is identical regardless of which one is plugged in.
//! `bootstrap.rs` picks one based on configuration.
//!
//! ## What it does on each call
//!
//! 1. **Inbound**: convert `&[serde_json::Value]` messages into
//!    [`clawft_service_llm::ChatMessage`] (extracting `role`, `content`,
//!    `tool_call_id`, and `tool_calls` when present).
//! 2. **Tool conversion**: convert pipeline-shape `&[serde_json::Value]`
//!    tools (already OpenAI `{type:"function",function:{...}}` shape)
//!    into [`clawft_service_llm::Tool`] using `serde_json::from_value`.
//! 3. **Call**: [`LlmClient::complete_with_tools`] — acquires the
//!    in-flight semaphore so concurrent agent loops serialize against
//!    the single-batch llama-server core.
//! 4. **Outbound**: serialize [`clawft_service_llm::ChatResponse`] back
//!    to OpenAI-shape `serde_json::Value` so the existing
//!    [`OpenAiCompatTransport`](super::transport::OpenAiCompatTransport)
//!    response parser can consume it unchanged.
//!
//! Streaming (`complete_stream`) is unsupported today —
//! [`LlmClient`] doesn't expose SSE yet. The default trait impl in
//! [`LlmProvider`] returns "streaming not supported", which is the
//! desired behavior until [`clawft-service-llm`] grows a streaming
//! method.

use async_trait::async_trait;
use tracing::debug;

use clawft_service_llm::{
    share_llm_client, ChatMessage, ChatResponse, ContentBlock, LlmClient, MessageContent,
    SharedLlmClient, Tool, ToolCall, ToolCallFunction, ToolChoice,
};

use super::transport::LlmProvider;

/// Adapts an [`LlmClient`] (narrow llama-server HTTP client) into the
/// pipeline's [`LlmProvider`] trait.
///
/// Cheap to clone — wraps a [`SharedLlmClient`] (`Arc<RwLock<LlmClient>>`)
/// so a daemon-side env rotation (WEFT-343) can swap the inner client
/// without rebuilding the pipeline.
#[derive(Debug, Clone)]
pub struct ServiceLlmAdapter {
    client: SharedLlmClient,
}

impl ServiceLlmAdapter {
    /// Wrap a shared, swappable client handle. Typical use:
    ///
    /// ```ignore
    /// use clawft_service_llm::{share_llm_client, LlmClient, LlmConfig};
    /// use clawft_core::pipeline::service_llm_adapter::ServiceLlmAdapter;
    ///
    /// let client = LlmClient::new(LlmConfig::from_env())?;
    /// let adapter = ServiceLlmAdapter::new(share_llm_client(client));
    /// ```
    pub fn new(client: SharedLlmClient) -> Self {
        Self { client }
    }

    /// Wrap a non-shared client (CLI / tests). Equivalent to
    /// `new(share_llm_client(client))` — the resulting handle is
    /// swappable but no other owner holds the outer `Arc`.
    pub fn from_client(client: LlmClient) -> Self {
        Self::new(share_llm_client(client))
    }

    /// Borrow the shared handle. Useful for tests and for the
    /// daemon's wiring where the same client backs both the
    /// `llm.prompt` RPC and the agent loop's transport.
    pub fn client(&self) -> &SharedLlmClient {
        &self.client
    }
}

#[async_trait]
impl LlmProvider for ServiceLlmAdapter {
    async fn complete(
        &self,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        max_tokens: Option<i32>,
        temperature: Option<f64>,
        tool_choice: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let chat_messages: Vec<ChatMessage> = messages.iter().map(value_to_message).collect();
        let choice = tool_choice.and_then(value_to_tool_choice);

        // When the caller pins a specific tool (`tool_choice = {function:
        // name}`), narrow the advertised tool set to just that tool. This
        // maximizes the odds the pin is honored and rules out the model
        // firing a *different* tool. Note: enforcement is ultimately
        // server-side — llama-server + Hermes treats `tool_choice` as
        // advisory once a system prompt is present and may still answer in
        // text, so this improves but does not guarantee forcing on that
        // stack. `Auto` / `None` / `Required` leave the tool set untouched.
        let forced_name = match &choice {
            Some(ToolChoice::Function(n)) => Some(n.as_str()),
            _ => None,
        };
        let parsed_tools: Vec<Tool> = tools
            .iter()
            .filter(|t| match forced_name {
                Some(name) => {
                    t.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        == Some(name)
                }
                None => true,
            })
            .filter_map(|t| match serde_json::from_value::<Tool>(t.clone()) {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    debug!(error = %e, "service-llm adapter: skipping malformed tool");
                    None
                }
            })
            .collect();

        // Hold a read lock for the duration of the HTTP call so a
        // concurrent WEFT-343 swap waits until in-flight work finishes
        // (and never observes a half-swapped client).
        let client = self.client.read().await;

        debug!(
            base_url = %client.config().base_url,
            model = %model,
            messages = chat_messages.len(),
            tools = parsed_tools.len(),
            "service-llm adapter forwarding request"
        );

        // The narrow client takes f32/u32 — clamp from the trait's
        // f64/i32. `max_tokens < 0` is meaningless here so we ignore
        // it; `None` falls back to the client config's default.
        let temp = temperature.map(|t| t as f32);
        let max_tok = max_tokens.and_then(|n| u32::try_from(n).ok());

        // WEFT-256: forward the routed model id as a per-call override
        // so a panel chip-strip selection (or tiered-router decision)
        // reaches the upstream body instead of only the daemon default.
        let model_override = if model.is_empty() { None } else { Some(model) };

        let resp = client
            .complete_with_tools(
                chat_messages,
                parsed_tools,
                choice,
                temp,
                max_tok,
                model_override,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(response_to_value(&resp, model))
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value` message into a typed [`ChatMessage`].
///
/// Tolerant of partial input — missing `role` defaults to `"user"` and
/// missing / null / unrecognized `content` defaults to empty text (the
/// OpenAI-compat schema accepts both). Array content is preserved as
/// [`MessageContent::Blocks`] (vision / structured); strings stay
/// [`MessageContent::Text`].
fn value_to_message(value: &serde_json::Value) -> ChatMessage {
    let role = value
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("user")
        .to_string();
    let content = value_to_content(value.get("content"));
    let tool_call_id = value
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let tool_calls: Option<Vec<ToolCall>> = value
        .get("tool_calls")
        .filter(|v| !v.is_null())
        .and_then(|v| {
            serde_json::from_value::<Vec<ToolCall>>(v.clone())
                .map_err(|e| {
                    debug!(error = %e, "service-llm adapter: failed to parse tool_calls");
                    e
                })
                .ok()
        });

    ChatMessage {
        role,
        content,
        tool_calls,
        tool_call_id,
    }
}

/// Map a JSON `content` value into [`MessageContent`].
///
/// - missing / null → empty text
/// - string → [`MessageContent::Text`]
/// - array → [`MessageContent::Blocks`] (best-effort; malformed arrays
///   fall back to empty text so the adapter never panics)
/// - other → empty text (tolerant; log at debug)
fn value_to_content(value: Option<&serde_json::Value>) -> MessageContent {
    match value {
        None | Some(serde_json::Value::Null) => MessageContent::Text(String::new()),
        Some(serde_json::Value::String(s)) => MessageContent::Text(s.clone()),
        Some(serde_json::Value::Array(_)) => match serde_json::from_value::<Vec<ContentBlock>>(
            value.cloned().unwrap_or(serde_json::Value::Null),
        ) {
            Ok(blocks) => MessageContent::Blocks(blocks),
            Err(e) => {
                debug!(error = %e, "service-llm adapter: content blocks unparseable; empty text");
                MessageContent::Text(String::new())
            }
        },
        Some(other) => {
            debug!(value = %other, "service-llm adapter: unexpected content shape; empty text");
            MessageContent::Text(String::new())
        }
    }
}

/// Parse a pipeline `tool_choice` [`serde_json::Value`] into the narrow
/// client's [`ToolChoice`].
///
/// Accepts the OpenAI-standard forms carried through
/// `AgentChatParams.metadata["tool_choice"]`:
/// - the string `"auto"` / `"none"` / `"required"`;
/// - an object `{"type":"function","function":{"name":"…"}}` (or the
///   shorthand `{"name":"…"}`) pinning a specific tool.
///
/// Returns `None` for anything unrecognized so a malformed override
/// degrades to the server default rather than failing the turn.
fn value_to_tool_choice(value: &serde_json::Value) -> Option<ToolChoice> {
    if let Some(s) = value.as_str() {
        return match s {
            "auto" => Some(ToolChoice::Auto),
            "none" => Some(ToolChoice::None),
            "required" | "any" => Some(ToolChoice::Required),
            _ => None,
        };
    }
    if let Some(obj) = value.as_object() {
        // {"type":"function","function":{"name":"X"}} or {"name":"X"}.
        let name = obj
            .get("function")
            .and_then(|f| f.get("name"))
            .or_else(|| obj.get("name"))
            .and_then(|n| n.as_str());
        if let Some(name) = name {
            return Some(ToolChoice::Function(name.to_string()));
        }
    }
    None
}

/// Convert a typed [`ChatResponse`] into the OpenAI-shape JSON the
/// pipeline's transport expects.
///
/// `model_hint` is the model name the *caller* requested; if the
/// server didn't echo it back we surface the hint so downstream code
/// has a non-null model field for logging.
fn response_to_value(response: &ChatResponse, model_hint: &str) -> serde_json::Value {
    let choices: Vec<serde_json::Value> = response
        .choices
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut msg = serde_json::json!({
                "role": c.message.role,
                "content": c.message.content,
            });
            if let Some(ref tcs) = c.message.tool_calls {
                msg["tool_calls"] = tool_calls_to_value(tcs);
            }
            if let Some(ref id) = c.message.tool_call_id {
                msg["tool_call_id"] = serde_json::Value::String(id.clone());
            }
            serde_json::json!({
                "index": i,
                "message": msg,
                "finish_reason": c.finish_reason,
            })
        })
        .collect();

    let usage = serde_json::json!({
        "prompt_tokens": response.usage.prompt_tokens,
        "completion_tokens": response.usage.completion_tokens,
        "total_tokens": response.usage.total_tokens,
    });

    let model = response
        .model
        .clone()
        .unwrap_or_else(|| model_hint.to_string());

    serde_json::json!({
        "id": "service-llm-response",
        "model": model,
        "choices": choices,
        "usage": usage,
    })
}

fn tool_calls_to_value(calls: &[ToolCall]) -> serde_json::Value {
    let arr: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "type": c.kind,
                "function": tool_call_function_to_value(&c.function),
            })
        })
        .collect();
    serde_json::Value::Array(arr)
}

fn tool_call_function_to_value(f: &ToolCallFunction) -> serde_json::Value {
    serde_json::json!({
        "name": f.name,
        "arguments": f.arguments,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_service_llm::{ChatChoice, ChatUsage};

    #[test]
    fn value_to_message_extracts_basic_fields() {
        let v = serde_json::json!({"role": "user", "content": "hi"});
        let m = value_to_message(&v);
        assert_eq!(m.role, "user");
        assert_eq!(m.content, "hi");
        assert!(m.tool_calls.is_none());
        assert!(m.tool_call_id.is_none());
    }

    #[test]
    fn value_to_message_defaults_missing_role_and_content() {
        let v = serde_json::json!({});
        let m = value_to_message(&v);
        assert_eq!(m.role, "user");
        assert_eq!(m.content, "");
    }

    #[test]
    fn value_to_message_preserves_vision_content_blocks() {
        let v = serde_json::json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "what is this?"},
                {"type": "image_url", "image_url": {"url": "https://ex/a.png"}}
            ]
        });
        let m = value_to_message(&v);
        let blocks = m.content.blocks().expect("blocks preserved");
        assert_eq!(blocks.len(), 2);
        assert_eq!(m.content.as_text(), "what is this?");
    }

    #[test]
    fn value_to_message_null_content_is_empty_text() {
        let v = serde_json::json!({"role": "assistant", "content": null});
        let m = value_to_message(&v);
        assert!(m.content.is_empty());
        assert_eq!(m.content, MessageContent::Text(String::new()));
    }

    #[test]
    fn value_to_message_extracts_tool_call_id() {
        let v = serde_json::json!({
            "role": "tool",
            "content": "[]",
            "tool_call_id": "call_42"
        });
        let m = value_to_message(&v);
        assert_eq!(m.role, "tool");
        assert_eq!(m.tool_call_id.as_deref(), Some("call_42"));
    }

    #[test]
    fn value_to_message_extracts_tool_calls() {
        let v = serde_json::json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "id": "call_x",
                "type": "function",
                "function": {"name": "ls", "arguments": "{}"}
            }]
        });
        let m = value_to_message(&v);
        let tcs = m.tool_calls.expect("tool_calls parsed");
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].id, "call_x");
        assert_eq!(tcs[0].function.name, "ls");
    }

    #[test]
    fn value_to_message_skips_malformed_tool_calls() {
        // Garbage in tool_calls — should fall back to None, not panic.
        let v = serde_json::json!({
            "role": "assistant",
            "content": "",
            "tool_calls": "not a list"
        });
        let m = value_to_message(&v);
        assert!(m.tool_calls.is_none());
    }

    fn mock_response(content: &str) -> ChatResponse {
        ChatResponse {
            choices: vec![ChatChoice {
                message: ChatMessage::assistant(content),
                finish_reason: Some("stop".into()),
            }],
            usage: ChatUsage {
                prompt_tokens: 7,
                completion_tokens: 3,
                total_tokens: 10,
                ..ChatUsage::default()
            },
            model: Some("test-model".into()),
            timings: None,
        }
    }

    #[test]
    fn response_to_value_basic_text() {
        let resp = mock_response("hello back");
        let v = response_to_value(&resp, "requested-model");
        assert_eq!(v["model"], "test-model");
        assert_eq!(v["choices"][0]["message"]["role"], "assistant");
        assert_eq!(v["choices"][0]["message"]["content"], "hello back");
        assert_eq!(v["choices"][0]["finish_reason"], "stop");
        assert_eq!(v["usage"]["prompt_tokens"], 7);
        assert_eq!(v["usage"]["completion_tokens"], 3);
        assert_eq!(v["usage"]["total_tokens"], 10);
    }

    #[test]
    fn response_to_value_falls_back_to_model_hint_when_server_omits() {
        let mut resp = mock_response("ok");
        resp.model = None;
        let v = response_to_value(&resp, "hint-model");
        assert_eq!(v["model"], "hint-model");
    }

    #[test]
    fn response_to_value_emits_tool_calls_when_present() {
        let resp = ChatResponse {
            choices: vec![ChatChoice {
                message: ChatMessage {
                    role: "assistant".into(),
                    content: MessageContent::Text(String::new()),
                    tool_calls: Some(vec![ToolCall {
                        id: "call_abc".into(),
                        kind: "function".into(),
                        function: ToolCallFunction {
                            name: "ls".into(),
                            arguments: "{}".into(),
                        },
                    }]),
                    tool_call_id: None,
                },
                finish_reason: Some("tool_calls".into()),
            }],
            usage: ChatUsage::default(),
            model: Some("m".into()),
            timings: None,
        };
        let v = response_to_value(&resp, "m");
        let tcs = v["choices"][0]["message"]["tool_calls"]
            .as_array()
            .expect("tool_calls is array");
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0]["id"], "call_abc");
        assert_eq!(tcs[0]["type"], "function");
        assert_eq!(tcs[0]["function"]["name"], "ls");
        assert_eq!(v["choices"][0]["finish_reason"], "tool_calls");
    }

    /// End-to-end: spin up a wiremock server, build the adapter, call
    /// `complete()` on the trait, and verify the OpenAI-shape JSON
    /// makes a clean round-trip through the conversions.
    #[tokio::test]
    async fn adapter_round_trip_against_mock_server() {
        use clawft_service_llm::LlmConfig;
        use std::time::Duration;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let body = r#"{
            "id": "chatcmpl-1",
            "model": "Qwen3.6-35B",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "hi back"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}
        }"#;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let client = LlmClient::new(LlmConfig {
            base_url: server.uri(),
            request_timeout: Duration::from_secs(5),
            health_deadline: Duration::from_secs(2),
            ..Default::default()
        })
        .unwrap();
        let adapter = ServiceLlmAdapter::from_client(client);

        let messages = vec![serde_json::json!({"role":"user","content":"hi"})];
        let v = adapter
            .complete("Qwen3.6-35B", &messages, &[], None, None, None)
            .await
            .expect("adapter call should succeed");

        assert_eq!(v["choices"][0]["message"]["content"], "hi back");
        assert_eq!(v["choices"][0]["finish_reason"], "stop");
        assert_eq!(v["usage"]["total_tokens"], 6);
    }

    /// End-to-end with tool-call response: the adapter must surface
    /// the `tool_calls` array intact in the OpenAI-shape JSON the
    /// pipeline transport will parse.
    #[tokio::test]
    async fn adapter_round_trip_tool_calls() {
        use clawft_service_llm::LlmConfig;
        use std::time::Duration;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let body = r#"{
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{
                        "id": "call_42",
                        "type": "function",
                        "function": {"name":"ls","arguments":"{\"path\":\"/\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }"#;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;

        let client = LlmClient::new(LlmConfig {
            base_url: server.uri(),
            request_timeout: Duration::from_secs(5),
            health_deadline: Duration::from_secs(2),
            ..Default::default()
        })
        .unwrap();
        let adapter = ServiceLlmAdapter::from_client(client);

        let tools = vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "ls",
                "description": "List files",
                "parameters": {"type":"object"}
            }
        })];
        let v = adapter
            .complete(
                "Qwen3.6-35B",
                &[serde_json::json!({"role":"user","content":"list /"})],
                &tools,
                None,
                None,
                None,
            )
            .await
            .expect("adapter tool-call round-trip should succeed");

        let tcs = v["choices"][0]["message"]["tool_calls"]
            .as_array()
            .expect("tool_calls present");
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0]["id"], "call_42");
        assert_eq!(tcs[0]["function"]["name"], "ls");
        assert_eq!(v["choices"][0]["finish_reason"], "tool_calls");
    }

    #[test]
    fn value_to_tool_choice_parses_openai_forms() {
        // ToolChoice has no PartialEq; assert via its wire serialization
        // (the exact bytes the llama-server body will carry).
        let ser = |v: serde_json::Value| {
            value_to_tool_choice(&v).map(|tc| serde_json::to_value(&tc).unwrap())
        };

        assert_eq!(ser(serde_json::json!("auto")), Some(serde_json::json!("auto")));
        assert_eq!(ser(serde_json::json!("none")), Some(serde_json::json!("none")));
        assert_eq!(
            ser(serde_json::json!("required")),
            Some(serde_json::json!("required"))
        );
        // OpenAI-standard named form round-trips to the same object shape.
        assert_eq!(
            ser(serde_json::json!({"type":"function","function":{"name":"agent_spawn"}})),
            Some(serde_json::json!({"type":"function","function":{"name":"agent_spawn"}}))
        );
        // Shorthand {"name":…} is accepted and normalizes to the full form.
        assert_eq!(
            ser(serde_json::json!({"name":"agent_spawn"})),
            Some(serde_json::json!({"type":"function","function":{"name":"agent_spawn"}}))
        );
        // Unrecognized values degrade to None (server default), not an error.
        assert_eq!(ser(serde_json::json!("bogus")), None);
        assert_eq!(ser(serde_json::json!(42)), None);
        assert_eq!(ser(serde_json::json!({"foo":"bar"})), None);
    }
}
