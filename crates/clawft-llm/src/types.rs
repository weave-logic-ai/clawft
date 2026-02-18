//! Request and response types for LLM chat completion calls.
//!
//! These types mirror the OpenAI chat completion API format, which has become
//! the de facto standard adopted by 19+ providers. They are standalone and
//! have no dependency on other clawft crates.

use serde::{Deserialize, Serialize};

/// A message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// The role of the message author (e.g. "system", "user", "assistant", "tool").
    pub role: String,

    /// The content of the message.
    pub content: String,

    /// For tool-result messages, the ID of the tool call this is a response to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Tool calls requested by the assistant in this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    /// Create a simple message with role and content.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for this tool call.
    pub id: String,

    /// The type of tool call. Currently always "function".
    #[serde(rename = "type")]
    pub call_type: String,

    /// The function to invoke.
    pub function: FunctionCall,
}

/// A function invocation within a tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,

    /// The arguments as a JSON string.
    pub arguments: String,
}

/// A chat completion request sent to an LLM provider.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// The model identifier (e.g. "gpt-4o", "claude-sonnet-4-5-20250514").
    pub model: String,

    /// The conversation messages.
    pub messages: Vec<ChatMessage>,

    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Sampling temperature (0.0 = deterministic, 2.0 = creative).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Tool definitions available to the model.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<serde_json::Value>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl ChatRequest {
    /// Create a minimal chat request with a model and messages.
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens: None,
            temperature: None,
            tools: Vec::new(),
            stream: None,
        }
    }
}

/// A chat completion response from an LLM provider (OpenAI format).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatResponse {
    /// Unique identifier for this completion.
    pub id: String,

    /// The list of completion choices.
    pub choices: Vec<Choice>,

    /// Token usage statistics for this request, if available.
    pub usage: Option<Usage>,

    /// The model that generated the response.
    pub model: String,
}

/// A single completion choice within a response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    /// The index of this choice in the list.
    pub index: i32,

    /// The assistant's response message.
    pub message: ChatMessage,

    /// Why generation stopped (e.g. "stop", "tool_calls", "length").
    pub finish_reason: Option<String>,
}

/// Token usage statistics for a completion request.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Usage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: i32,

    /// Number of tokens in the generated completion.
    pub completion_tokens: i32,

    /// Total tokens used (prompt + completion).
    pub total_tokens: i32,
}

// ── Streaming types ─────────────────────────────────────────────────────

/// A single chunk received during SSE streaming of a chat completion.
///
/// The OpenAI streaming format sends `data:` lines containing JSON objects
/// with partial content deltas, followed by a `data: [DONE]` sentinel.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamChunk {
    /// A text content delta (partial token).
    TextDelta {
        /// The partial text content.
        text: String,
    },

    /// A tool call delta (partial tool invocation).
    ToolCallDelta {
        /// Index of the tool call in the tool_calls array.
        index: usize,
        /// Tool call ID (only present on the first delta for this tool call).
        id: Option<String>,
        /// Function name (only present on the first delta for this tool call).
        name: Option<String>,
        /// Partial arguments fragment.
        arguments: Option<String>,
    },

    /// The stream is complete.
    Done {
        /// Finish reason from the last chunk (e.g. "stop", "tool_calls", "length").
        finish_reason: Option<String>,
        /// Token usage statistics (if the provider sends them in the final chunk).
        usage: Option<Usage>,
    },
}

/// A streaming delta message from an SSE chunk.
///
/// This mirrors the OpenAI `chat.completion.chunk` format. Each SSE `data:`
/// line (except `[DONE]`) deserializes into this structure.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StreamDelta {
    /// The chunk choices array (usually 1 element).
    #[serde(default)]
    pub choices: Vec<StreamDeltaChoice>,

    /// Usage statistics (some providers include this in the final chunk).
    #[serde(default)]
    pub usage: Option<StreamDeltaUsage>,
}

/// A single choice within a streaming delta.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StreamDeltaChoice {
    /// The delta object containing partial content.
    #[serde(default)]
    pub delta: StreamDeltaContent,

    /// Finish reason (present only on the final chunk).
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// The delta content within a streaming choice.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct StreamDeltaContent {
    /// Partial text content (if the model is generating text).
    #[serde(default)]
    pub content: Option<String>,

    /// Partial tool calls (if the model is invoking tools).
    #[serde(default)]
    pub tool_calls: Option<Vec<StreamDeltaToolCall>>,
}

/// A tool call delta within a streaming choice.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StreamDeltaToolCall {
    /// Index of this tool call in the tool_calls array.
    pub index: usize,

    /// Tool call ID (only in the first delta for this tool call).
    #[serde(default)]
    pub id: Option<String>,

    /// Function info (name and/or argument fragments).
    #[serde(default)]
    pub function: Option<StreamDeltaFunction>,
}

/// Function details within a tool call delta.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct StreamDeltaFunction {
    /// Function name (only in the first delta for this tool call).
    #[serde(default)]
    pub name: Option<String>,

    /// Partial arguments fragment.
    #[serde(default)]
    pub arguments: Option<String>,
}

/// Usage statistics in streaming responses.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StreamDeltaUsage {
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_new_helpers() {
        let sys = ChatMessage::system("You are helpful.");
        assert_eq!(sys.role, "system");
        assert_eq!(sys.content, "You are helpful.");
        assert!(sys.tool_call_id.is_none());
        assert!(sys.tool_calls.is_none());

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, "user");

        let asst = ChatMessage::assistant("Hi there");
        assert_eq!(asst.role, "assistant");
    }

    #[test]
    fn chat_message_serde_roundtrip() {
        let msg = ChatMessage::user("Hello, world!");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn chat_message_skips_none_fields() {
        let msg = ChatMessage::user("Hi");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("tool_call_id"));
        assert!(!json.contains("tool_calls"));
    }

    #[test]
    fn chat_message_with_tool_calls_roundtrip() {
        let msg = ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_abc123".into(),
                call_type: "function".into(),
                function: FunctionCall {
                    name: "get_weather".into(),
                    arguments: r#"{"city":"London"}"#.into(),
                },
            }]),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("tool_calls"));
        assert!(json.contains("call_abc123"));
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn tool_call_type_field_renamed() {
        let tc = ToolCall {
            id: "tc1".into(),
            call_type: "function".into(),
            function: FunctionCall {
                name: "search".into(),
                arguments: "{}".into(),
            },
        };
        let json = serde_json::to_string(&tc).unwrap();
        // Should serialize as "type", not "call_type"
        assert!(json.contains(r#""type":"function""#));
        assert!(!json.contains("call_type"));
    }

    #[test]
    fn chat_request_serialization() {
        let req = ChatRequest::new("gpt-4o", vec![ChatMessage::user("Hi")]);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""model":"gpt-4o""#));
        assert!(json.contains(r#""role":"user""#));
        // Empty tools and None fields should be absent
        assert!(!json.contains("tools"));
        assert!(!json.contains("stream"));
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("temperature"));
    }

    #[test]
    fn chat_request_with_all_fields() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage::user("test")],
            max_tokens: Some(100),
            temperature: Some(0.7),
            tools: vec![serde_json::json!({"type": "function", "function": {"name": "test"}})],
            stream: Some(true),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("max_tokens"));
        assert!(json.contains("temperature"));
        assert!(json.contains("tools"));
        assert!(json.contains("stream"));
    }

    #[test]
    fn chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-abc123",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            },
            "model": "gpt-4o-2024-05-13"
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "chatcmpl-abc123");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
        assert_eq!(resp.model, "gpt-4o-2024-05-13");
    }

    #[test]
    fn chat_response_without_usage() {
        let json = r#"{
            "id": "resp-1",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Ok"},
                "finish_reason": null
            }],
            "usage": null,
            "model": "test-model"
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
        assert!(resp.choices[0].finish_reason.is_none());
    }

    #[test]
    fn usage_serde_roundtrip() {
        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let parsed: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage, parsed);
    }
}
