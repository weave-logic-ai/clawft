//! Hermes serving glue for [`LocalProvider`](crate::local_provider::LocalProvider).
//!
//! Hermes 4.x (the ADR-060 default agent-loop model) speaks two non-standard
//! dialects on top of the OpenAI chat-completions wire format:
//!
//! 1. **Tool calls** are emitted as XML-style `<tool_call>{…}</tool_call>` blocks
//!    inside the assistant `content`, not as native `tool_calls`. With llama.cpp
//!    `--jinja` the embedded chat template usually parses these into native
//!    `tool_calls`, but not every backend (Ollama, a mis-templated server) does —
//!    so the provider must be able to recover them from `content` itself.
//! 2. **Hybrid reasoning** is emitted as `<think>…</think>` blocks. These must be
//!    stripped *before* tool-call parsing, or a `<tool_call>` the model merely
//!    *mentions* while reasoning would be parsed as a real call and its arguments
//!    corrupted (ADR-060: "`<think>` stripped/routed out of `tool_calls` parsing").
//!
//! This module holds the pure, endpoint-independent logic for both, plus the
//! request-body augmentation for the per-turn reasoning toggle and explicit
//! context window (`num_ctx`) — the knobs ADR-060 requires set on every path so a
//! default-4k server never silently truncates the loop.

use serde_json::{Value, json};

use crate::types::{ChatMessage, FunctionCall, ToolCall};

/// Per-turn hybrid-reasoning control for Hermes (`<think>`).
///
/// ADR-060 calls for a per-turn toggle: reasoning **on** for planning turns,
/// **off** for tool-execution turns. The toggle maps to the `enable_thinking`
/// chat-template kwarg understood by llama.cpp `--jinja` (and vLLM) for
/// Hermes/Qwen-style templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReasoningMode {
    /// Don't inject any control — use the server/template default.
    #[default]
    Default,
    /// Request reasoning (`enable_thinking: true`) — planning turns.
    On,
    /// Suppress reasoning (`enable_thinking: false`) — tool-execution turns.
    Off,
}

impl ReasoningMode {
    /// The `enable_thinking` value to inject, or `None` to leave it unset.
    fn enable_thinking(self) -> Option<bool> {
        match self {
            ReasoningMode::Default => None,
            ReasoningMode::On => Some(true),
            ReasoningMode::Off => Some(false),
        }
    }
}

/// Remove every `<think>…</think>` block from `content`.
///
/// Returns the cleaned content and the concatenated reasoning text (routed out,
/// never fed to tool-call parsing). An unterminated `<think>` (truncated output)
/// strips to the end of the string. Matching is case-insensitive on the tag name
/// to tolerate `<Think>`/`<THINK>` variants.
pub fn strip_think(content: &str) -> (String, Option<String>) {
    const OPEN: &str = "<think>";
    const CLOSE: &str = "</think>";

    let lower = content.to_ascii_lowercase();
    if !lower.contains(OPEN) {
        return (content.to_string(), None);
    }

    let mut cleaned = String::with_capacity(content.len());
    let mut think = String::new();
    let mut cursor = 0usize;

    while let Some(rel_open) = lower[cursor..].find(OPEN) {
        let open = cursor + rel_open;
        cleaned.push_str(&content[cursor..open]);
        let inner_start = open + OPEN.len();
        match lower[inner_start..].find(CLOSE) {
            Some(rel_close) => {
                let close = inner_start + rel_close;
                think.push_str(content[inner_start..close].trim());
                think.push('\n');
                cursor = close + CLOSE.len();
            }
            None => {
                // Unterminated reasoning block — consume to end.
                think.push_str(content[inner_start..].trim());
                cursor = content.len();
                break;
            }
        }
    }
    cleaned.push_str(&content[cursor..]);

    let think = think.trim();
    let think = if think.is_empty() {
        None
    } else {
        Some(think.to_string())
    };
    (cleaned, think)
}

/// Extract `<tool_call>{…}</tool_call>` blocks from `content`.
///
/// Returns the content with the blocks removed and the parsed [`ToolCall`]s. Each
/// block's inner JSON is `{"name": "...", "arguments": {…}|"..."}` (the Hermes
/// shape). Object `arguments` are re-serialized to the JSON *string* the OpenAI
/// [`FunctionCall`] type expects; string `arguments` pass through. Blocks whose
/// inner text is not valid JSON, or lack a `name`, are skipped (left in content).
///
/// IDs are assigned positionally (`call_0`, `call_1`, …) so parsing is
/// deterministic and testable without a live endpoint.
///
/// Callers MUST run [`strip_think`] first so a `<tool_call>` appearing inside a
/// `<think>` block is never extracted.
pub fn extract_tool_calls(content: &str) -> (String, Vec<ToolCall>) {
    const OPEN: &str = "<tool_call>";
    const CLOSE: &str = "</tool_call>";

    if !content.contains(OPEN) {
        return (content.to_string(), Vec::new());
    }

    let mut remaining = String::with_capacity(content.len());
    let mut calls = Vec::new();
    let mut cursor = 0usize;

    while let Some(rel_open) = content[cursor..].find(OPEN) {
        let open = cursor + rel_open;
        let inner_start = open + OPEN.len();
        let Some(rel_close) = content[inner_start..].find(CLOSE) else {
            // No closing tag — keep the rest verbatim and stop.
            break;
        };
        let close = inner_start + rel_close;
        let inner = content[inner_start..close].trim();

        match parse_hermes_call(inner, calls.len()) {
            Some(call) => {
                // Drop the block (and surrounding whitespace it leaves behind).
                remaining.push_str(&content[cursor..open]);
                cursor = close + CLOSE.len();
                calls.push(call);
            }
            None => {
                // Not a parseable call — keep the block verbatim, advance past it.
                remaining.push_str(&content[cursor..close + CLOSE.len()]);
                cursor = close + CLOSE.len();
            }
        }
    }
    remaining.push_str(&content[cursor..]);
    (remaining, calls)
}

/// Parse one Hermes tool-call body into a [`ToolCall`], or `None` if malformed.
fn parse_hermes_call(inner: &str, index: usize) -> Option<ToolCall> {
    let value: Value = serde_json::from_str(inner).ok()?;
    let name = value.get("name")?.as_str()?.to_string();
    let arguments = match value.get("arguments") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "{}".to_string(),
    };
    Some(ToolCall {
        id: format!("call_{index}"),
        call_type: "function".to_string(),
        function: FunctionCall { name, arguments },
    })
}

/// Normalize a Hermes assistant message to the OpenAI shape.
///
/// 1. Strips `<think>` reasoning out of the content.
/// 2. If the message has no native `tool_calls`, recovers any
///    `<tool_call>` blocks from the (think-stripped) content.
///
/// Returns `true` if tool calls were recovered from content (so the caller can
/// fix up `finish_reason`). The message's content is set to `None` when it is
/// empty after stripping/extraction.
pub fn normalize_message(message: &mut ChatMessage) -> bool {
    let Some(raw) = message.content.take() else {
        return false;
    };

    let (cleaned, _think) = strip_think(&raw);

    let has_native_calls = message.tool_calls.as_ref().is_some_and(|c| !c.is_empty());

    let (text, recovered) = if has_native_calls {
        (cleaned, Vec::new())
    } else {
        extract_tool_calls(&cleaned)
    };

    let recovered_any = !recovered.is_empty();
    if recovered_any {
        message.tool_calls = Some(recovered);
    }

    let text = text.trim();
    message.content = if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    };

    recovered_any
}

/// Augment a serialized chat-completions request body with the Hermes serving
/// knobs ADR-060 requires explicit on every path.
///
/// - `num_ctx` → `options.num_ctx` (the Ollama context-window field; a default-4k
///   server silently truncates the loop without it). llama.cpp ignores it
///   harmlessly — its window is fixed by `--ctx-size` at launch.
/// - `reasoning` → `chat_template_kwargs.enable_thinking` (the per-turn `<think>`
///   toggle for `--jinja` templates).
///
/// `body` must be a JSON object (a serialized [`ChatRequest`](crate::types::ChatRequest)).
pub fn augment_request_body(body: &mut Value, num_ctx: Option<u32>, reasoning: ReasoningMode) {
    let Some(obj) = body.as_object_mut() else {
        return;
    };

    if let Some(ctx) = num_ctx {
        let options = obj.entry("options").or_insert_with(|| json!({}));
        if let Some(options) = options.as_object_mut() {
            options.insert("num_ctx".to_string(), json!(ctx));
        }
    }

    if let Some(enable) = reasoning.enable_thinking() {
        let kwargs = obj
            .entry("chat_template_kwargs")
            .or_insert_with(|| json!({}));
        if let Some(kwargs) = kwargs.as_object_mut() {
            kwargs.insert("enable_thinking".to_string(), json!(enable));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_think ─────────────────────────────────────────────────

    #[test]
    fn strip_think_removes_block_and_routes_reasoning() {
        let (cleaned, think) = strip_think("<think>I should greet them.</think>Hello there!");
        assert_eq!(cleaned, "Hello there!");
        assert_eq!(think.as_deref(), Some("I should greet them."));
    }

    #[test]
    fn strip_think_no_block_is_identity() {
        let (cleaned, think) = strip_think("Just a plain answer.");
        assert_eq!(cleaned, "Just a plain answer.");
        assert!(think.is_none());
    }

    #[test]
    fn strip_think_multiple_blocks() {
        let (cleaned, think) = strip_think("<think>a</think>X<think>b</think>Y");
        assert_eq!(cleaned, "XY");
        assert_eq!(think.as_deref(), Some("a\nb"));
    }

    #[test]
    fn strip_think_unterminated_consumes_to_end() {
        let (cleaned, think) = strip_think("done.<think>truncated reasoning...");
        assert_eq!(cleaned, "done.");
        assert_eq!(think.as_deref(), Some("truncated reasoning..."));
    }

    #[test]
    fn strip_think_case_insensitive() {
        let (cleaned, _) = strip_think("<THINK>x</THINK>y");
        assert_eq!(cleaned, "y");
    }

    // ── extract_tool_calls ──────────────────────────────────────────

    #[test]
    fn extract_single_tool_call_object_arguments() {
        let content =
            r#"<tool_call>{"name": "get_weather", "arguments": {"city": "SF"}}</tool_call>"#;
        let (remaining, calls) = extract_tool_calls(content);
        assert_eq!(remaining.trim(), "");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_0");
        assert_eq!(calls[0].call_type, "function");
        assert_eq!(calls[0].function.name, "get_weather");
        // Object arguments are re-serialized to a JSON string.
        let parsed: Value = serde_json::from_str(&calls[0].function.arguments).unwrap();
        assert_eq!(parsed["city"], "SF");
    }

    #[test]
    fn extract_tool_call_string_arguments_pass_through() {
        let content = r#"<tool_call>{"name": "echo", "arguments": "{\"x\":1}"}</tool_call>"#;
        let (_, calls) = extract_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.arguments, r#"{"x":1}"#);
    }

    #[test]
    fn extract_multiple_tool_calls_keep_text() {
        let content = concat!(
            "Let me check both.\n",
            r#"<tool_call>{"name": "a", "arguments": {}}</tool_call>"#,
            "\n",
            r#"<tool_call>{"name": "b", "arguments": {}}</tool_call>"#,
        );
        let (remaining, calls) = extract_tool_calls(content);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "call_0");
        assert_eq!(calls[1].id, "call_1");
        assert!(remaining.contains("Let me check both."));
        assert!(!remaining.contains("<tool_call>"));
    }

    #[test]
    fn extract_no_tool_call_is_identity() {
        let (remaining, calls) = extract_tool_calls("No tools here.");
        assert_eq!(remaining, "No tools here.");
        assert!(calls.is_empty());
    }

    #[test]
    fn extract_malformed_block_left_verbatim() {
        let content = r#"<tool_call>not json</tool_call>"#;
        let (remaining, calls) = extract_tool_calls(content);
        assert!(calls.is_empty());
        assert_eq!(remaining, content);
    }

    #[test]
    fn extract_unterminated_block_left_verbatim() {
        let content = r#"prefix <tool_call>{"name":"a"}"#;
        let (remaining, calls) = extract_tool_calls(content);
        assert!(calls.is_empty());
        assert_eq!(remaining, content);
    }

    // ── the load-bearing ordering guarantee ─────────────────────────

    #[test]
    fn think_mentioned_tool_call_is_not_extracted() {
        // The model *reasons about* calling a tool inside <think>, then makes the
        // real call afterward. Only the real one must be parsed; the reasoning
        // mention must never reach tool_calls.
        let content = concat!(
            r#"<think>I will call <tool_call>{"name":"WRONG","arguments":{}}</tool_call></think>"#,
            r#"<tool_call>{"name":"right","arguments":{"q":"x"}}</tool_call>"#,
        );
        let (cleaned, _) = strip_think(content);
        let (_, calls) = extract_tool_calls(&cleaned);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "right");
    }

    // ── normalize_message ───────────────────────────────────────────

    #[test]
    fn normalize_recovers_tool_calls_from_content() {
        let mut msg = ChatMessage {
            role: "assistant".into(),
            content: Some(
                r#"<think>plan</think><tool_call>{"name":"search","arguments":{"q":"rust"}}</tool_call>"#
                    .into(),
            ),
            tool_call_id: None,
            tool_calls: None,
        };
        let recovered = normalize_message(&mut msg);
        assert!(recovered);
        assert!(msg.content.is_none());
        let calls = msg.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "search");
        // <think> must never leak into a parsed call.
        assert!(!calls[0].function.arguments.contains("plan"));
    }

    #[test]
    fn normalize_keeps_native_tool_calls_and_strips_think() {
        let native = ToolCall {
            id: "call_native".into(),
            call_type: "function".into(),
            function: FunctionCall {
                name: "already_parsed".into(),
                arguments: "{}".into(),
            },
        };
        let mut msg = ChatMessage {
            role: "assistant".into(),
            content: Some("<think>reasoning</think>".into()),
            tool_call_id: None,
            tool_calls: Some(vec![native.clone()]),
        };
        let recovered = normalize_message(&mut msg);
        assert!(!recovered);
        // Native calls preserved untouched; content was only reasoning -> None.
        assert_eq!(msg.tool_calls.unwrap()[0].id, "call_native");
        assert!(msg.content.is_none());
    }

    #[test]
    fn normalize_plain_text_passes_through() {
        let mut msg = ChatMessage::assistant("Hello!");
        let recovered = normalize_message(&mut msg);
        assert!(!recovered);
        assert_eq!(msg.content.as_deref(), Some("Hello!"));
        assert!(msg.tool_calls.is_none());
    }

    // ── augment_request_body ────────────────────────────────────────

    #[test]
    fn augment_injects_num_ctx_and_reasoning() {
        let mut body = json!({"model": "hermes", "messages": []});
        augment_request_body(&mut body, Some(32768), ReasoningMode::Off);
        assert_eq!(body["options"]["num_ctx"], 32768);
        assert_eq!(body["chat_template_kwargs"]["enable_thinking"], false);
    }

    #[test]
    fn augment_reasoning_on() {
        let mut body = json!({"model": "hermes"});
        augment_request_body(&mut body, None, ReasoningMode::On);
        assert_eq!(body["chat_template_kwargs"]["enable_thinking"], true);
        assert!(body.get("options").is_none());
    }

    #[test]
    fn augment_default_injects_nothing() {
        let mut body = json!({"model": "hermes"});
        augment_request_body(&mut body, None, ReasoningMode::Default);
        assert!(body.get("options").is_none());
        assert!(body.get("chat_template_kwargs").is_none());
    }

    #[test]
    fn augment_preserves_existing_options() {
        let mut body = json!({"model": "hermes", "options": {"temperature": 0.5}});
        augment_request_body(&mut body, Some(8192), ReasoningMode::Default);
        assert_eq!(body["options"]["temperature"], 0.5);
        assert_eq!(body["options"]["num_ctx"], 8192);
    }
}
