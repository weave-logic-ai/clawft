//! Browser-side bridge between the pipeline's [`LlmProvider`] trait and
//! [`clawft_llm::browser_transport::BrowserLlmClient`].
//!
//! On native, [`super::llm_adapter::ClawftLlmAdapter`] wraps an
//! `Arc<dyn clawft_llm::Provider>` and forwards through reqwest's tokio
//! runtime. On browser WASM the analogous adapter wraps a
//! [`BrowserLlmClient`] directly because the native `Provider` trait is
//! built around tokio mpsc streaming which is not available in the
//! single-threaded WASM event loop.
//!
//! # Send/Sync soundness
//!
//! [`LlmProvider`] requires `Send + Sync`, but `BrowserLlmClient` holds a
//! `reqwest::Client` whose wasm32 build is `!Send` (the underlying
//! `JsValue` / `Promise` chain cannot cross threads). On
//! `wasm32-unknown-unknown` the runtime is single-threaded by
//! construction (no `std::thread::spawn`, no `tokio::spawn` runtime), so
//! a value can never actually traverse a thread boundary. Accordingly we
//! `unsafe impl Send + Sync` for the adapter — sound on this target,
//! and gated behind `#[cfg(feature = "browser")]` so the bound is never
//! exercised on native.
//!
//! # Pipeline wiring
//!
//! The adapter slots into [`OpenAiCompatTransport::with_provider`]
//! exactly like its native sibling, so the rest of the pipeline
//! (classifier → router → assembler → scorer → learner) is unchanged
//! between targets.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use clawft_llm::browser_transport::BrowserLlmClient;
use clawft_llm::types::{ChatMessage, ChatRequest as LlmChatRequest, StreamChunk};

use super::transport::LlmProvider;

/// Adapts a [`BrowserLlmClient`] into the pipeline's [`LlmProvider`]
/// trait. See module docs for the [`Send`]/[`Sync`] story.
pub struct BrowserLlmAdapter {
    /// The wrapped browser LLM client.
    client: Arc<BrowserLlmClient>,
}

impl BrowserLlmAdapter {
    /// Wrap a [`BrowserLlmClient`] in the adapter.
    pub fn new(client: Arc<BrowserLlmClient>) -> Self {
        Self { client }
    }
}

// SAFETY: `wasm32-unknown-unknown` is single-threaded; nothing can
// actually traverse a thread boundary. The pipeline's `LlmProvider`
// `Send + Sync` bound is satisfied by construction at runtime even if
// the underlying reqwest wasm client is `!Send` at the type level.
unsafe impl Send for BrowserLlmAdapter {}
unsafe impl Sync for BrowserLlmAdapter {}

#[async_trait(?Send)]
impl LlmProvider for BrowserLlmAdapter {
    async fn complete(
        &self,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        max_tokens: Option<i32>,
        temperature: Option<f64>,
        tool_choice: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let chat_messages: Vec<ChatMessage> =
            messages.iter().map(convert_value_to_message).collect();

        let request = LlmChatRequest {
            model: model.to_string(),
            messages: chat_messages,
            max_tokens,
            temperature,
            tools: tools.to_vec(),
            stream: None,
            tool_choice: tool_choice.cloned(),
        };

        debug!(
            provider = %self.client.name(),
            model = %model,
            messages = request.messages.len(),
            tools = request.tools.len(),
            "browser adapter forwarding request to BrowserLlmClient"
        );

        match self.client.complete(&request).await {
            Ok(response) => Ok(convert_response_to_value(&response)),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Stream text deltas via [`BrowserLlmClient::complete_stream_callback`].
    ///
    /// Under the hood reqwest's wasm client consumes the Fetch
    /// `ReadableStream` (wasm-streams). CORS proxy routing from
    /// `BrowserLlmClient` applies unchanged. WEFT-390.
    #[allow(clippy::too_many_arguments)]
    async fn complete_stream(
        &self,
        model: &str,
        messages: &[serde_json::Value],
        tools: &[serde_json::Value],
        max_tokens: Option<i32>,
        temperature: Option<f64>,
        tool_choice: Option<&serde_json::Value>,
        mut on_delta: Box<dyn for<'a> FnMut(&'a str) -> bool>,
    ) -> Result<serde_json::Value, String> {
        let chat_messages: Vec<ChatMessage> =
            messages.iter().map(convert_value_to_message).collect();

        let request = LlmChatRequest {
            model: model.to_string(),
            messages: chat_messages,
            max_tokens,
            temperature,
            tools: tools.to_vec(),
            stream: Some(true),
            tool_choice: tool_choice.cloned(),
        };

        debug!(
            provider = %self.client.name(),
            model = %model,
            messages = request.messages.len(),
            tools = request.tools.len(),
            "browser adapter streaming request to BrowserLlmClient"
        );

        let mut full_text = String::new();
        let mut finish_reason: Option<String> = None;
        let mut usage: Option<clawft_llm::Usage> = None;

        self.client
            .complete_stream_callback(&request, |chunk| match chunk {
                StreamChunk::TextDelta { text } => {
                    full_text.push_str(&text);
                    on_delta(&text)
                }
                StreamChunk::Done {
                    finish_reason: fr,
                    usage: u,
                } => {
                    if fr.is_some() {
                        finish_reason = fr;
                    }
                    if u.is_some() {
                        usage = u;
                    }
                    true
                }
                StreamChunk::ToolCallDelta { .. } => {
                    // Tool-call deltas are not forwarded as text; a future
                    // path can accumulate them the way the native adapter does.
                    true
                }
            })
            .await
            .map_err(|e| e.to_string())?;

        let fr = finish_reason.unwrap_or_else(|| "stop".into());
        let response = clawft_llm::types::ChatResponse {
            id: "stream-response".into(),
            model: model.to_string(),
            choices: vec![clawft_llm::types::Choice {
                index: 0,
                message: ChatMessage::assistant(full_text),
                finish_reason: Some(fr),
            }],
            usage,
        };

        Ok(convert_response_to_value(&response))
    }
}

/// Convert a JSON message (as the pipeline emits) into a
/// [`ChatMessage`] understood by the browser transport.
fn convert_value_to_message(value: &serde_json::Value) -> ChatMessage {
    let role = value
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("user")
        .to_string();
    let content = value
        .get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let tool_call_id = value
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    ChatMessage {
        role,
        content,
        tool_call_id,
        tool_calls: None,
    }
}

/// Convert a [`clawft_llm::ChatResponse`] back into the OpenAI-format
/// JSON shape that [`super::transport::OpenAiCompatTransport`] expects.
fn convert_response_to_value(response: &clawft_llm::types::ChatResponse) -> serde_json::Value {
    let mut choices = Vec::with_capacity(response.choices.len());
    for choice in &response.choices {
        let mut message = serde_json::Map::new();
        message.insert(
            "role".into(),
            serde_json::Value::String(choice.message.role.clone()),
        );
        message.insert(
            "content".into(),
            choice
                .message
                .content
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
        // Tool calls — the native adapter pipes them through; do the
        // same here so a future browser tool-call path inherits the
        // wiring without extra work.
        if let Some(ref tcs) = choice.message.tool_calls {
            message.insert(
                "tool_calls".into(),
                serde_json::to_value(tcs).unwrap_or(serde_json::Value::Null),
            );
        }
        let mut entry = serde_json::Map::new();
        entry.insert(
            "index".into(),
            serde_json::Value::Number(serde_json::Number::from(choice.index)),
        );
        entry.insert("message".into(), serde_json::Value::Object(message));
        if let Some(ref reason) = choice.finish_reason {
            entry.insert(
                "finish_reason".into(),
                serde_json::Value::String(reason.clone()),
            );
        }
        choices.push(serde_json::Value::Object(entry));
    }

    let mut out = serde_json::Map::new();
    out.insert("id".into(), serde_json::Value::String(response.id.clone()));
    out.insert(
        "model".into(),
        serde_json::Value::String(response.model.clone()),
    );
    out.insert("choices".into(), serde_json::Value::Array(choices));

    if let Some(ref usage_obj) = response.usage {
        let mut usage = serde_json::Map::new();
        usage.insert(
            "prompt_tokens".into(),
            serde_json::Value::Number(serde_json::Number::from(usage_obj.input_tokens)),
        );
        usage.insert(
            "completion_tokens".into(),
            serde_json::Value::Number(serde_json::Number::from(usage_obj.output_tokens)),
        );
        usage.insert(
            "total_tokens".into(),
            serde_json::Value::Number(serde_json::Number::from(usage_obj.total_tokens)),
        );
        out.insert("usage".into(), serde_json::Value::Object(usage));
    }

    let _: HashMap<String, serde_json::Value> = HashMap::new();
    serde_json::Value::Object(out)
}
