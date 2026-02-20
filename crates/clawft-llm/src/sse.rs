//! SSE (Server-Sent Events) line parser for OpenAI-compatible streaming.
//!
//! Parses the `data:` lines from an SSE stream into [`StreamChunk`] values.
//! The OpenAI streaming format sends lines like:
//!
//! ```text
//! data: {"id":"...","choices":[{"delta":{"content":"Hello"},...}],...}
//!
//! data: {"id":"...","choices":[{"delta":{"content":" world"},...}],...}
//!
//! data: [DONE]
//! ```
//!
//! Each non-empty `data:` line is either:
//! - A JSON object containing a streaming delta
//! - The literal `[DONE]` sentinel marking end of stream

use crate::error::{ProviderError, Result};
use crate::types::{StreamChunk, StreamDelta, StreamDeltaUsage, Usage};

/// The sentinel value that marks the end of an SSE stream.
const DONE_SENTINEL: &str = "[DONE]";

/// Parse a single SSE `data:` line into zero or more [`StreamChunk`] values.
///
/// Returns `Ok(vec![])` for:
/// - Empty lines (SSE event boundaries)
/// - Lines that are not `data:` prefixed (comments, event types)
/// - `data:` lines with empty payloads
///
/// Returns `Ok(vec![chunk])` for normal delta lines, or `Ok(vec![Done])`
/// for the `[DONE]` sentinel.
///
/// # Errors
///
/// Returns [`ProviderError::InvalidResponse`] if a `data:` line contains
/// JSON that cannot be parsed as a streaming delta.
pub fn parse_sse_line(line: &str) -> Result<Vec<StreamChunk>> {
    let line = line.trim_end();

    // Skip empty lines (SSE event boundaries) and non-data lines
    if line.is_empty() || line.starts_with(':') {
        return Ok(vec![]);
    }

    // Must be a data: line
    let payload = if let Some(rest) = line.strip_prefix("data:") {
        rest.trim_start()
    } else {
        // event:, id:, retry: lines -- skip
        return Ok(vec![]);
    };

    // Empty data payload
    if payload.is_empty() {
        return Ok(vec![]);
    }

    // [DONE] sentinel
    if payload == DONE_SENTINEL {
        return Ok(vec![StreamChunk::Done {
            finish_reason: None,
            usage: None,
        }]);
    }

    // Parse JSON delta
    let delta: StreamDelta = serde_json::from_str(payload)
        .map_err(|e| ProviderError::InvalidResponse(format!("failed to parse SSE delta: {e}")))?;

    Ok(convert_delta_to_chunks(&delta))
}

/// Convert a parsed [`StreamDelta`] into a list of [`StreamChunk`] values.
///
/// A single delta can produce:
/// - A `TextDelta` if the delta has text content
/// - One or more `ToolCallDelta` chunks if the delta has tool calls
/// - A `Done` chunk if a finish_reason is present
fn convert_delta_to_chunks(delta: &StreamDelta) -> Vec<StreamChunk> {
    let mut chunks = Vec::new();

    if let Some(choice) = delta.choices.first() {
        // Text content delta
        if let Some(ref text) = choice.delta.content
            && !text.is_empty()
        {
            chunks.push(StreamChunk::TextDelta { text: text.clone() });
        }

        // Tool call deltas
        if let Some(ref tool_calls) = choice.delta.tool_calls {
            for tc in tool_calls {
                let (name, arguments) = match &tc.function {
                    Some(f) => (f.name.clone(), f.arguments.clone()),
                    None => (None, None),
                };
                chunks.push(StreamChunk::ToolCallDelta {
                    index: tc.index,
                    id: tc.id.clone(),
                    name,
                    arguments,
                });
            }
        }

        // Finish reason signals end of stream
        if choice.finish_reason.is_some() {
            let usage = delta.usage.as_ref().map(convert_usage);
            chunks.push(StreamChunk::Done {
                finish_reason: choice.finish_reason.clone(),
                usage,
            });
        }
    }

    chunks
}

/// Convert streaming usage stats to the public [`Usage`] type.
fn convert_usage(u: &StreamDeltaUsage) -> Usage {
    Usage {
        input_tokens: u.prompt_tokens.unwrap_or(0) as u32,
        output_tokens: u.completion_tokens.unwrap_or(0) as u32,
        total_tokens: u.total_tokens.unwrap_or(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Empty / skip lines ──────────────────────────────────────────

    #[test]
    fn empty_line_returns_empty() {
        assert!(parse_sse_line("").unwrap().is_empty());
    }

    #[test]
    fn whitespace_line_returns_empty() {
        assert!(parse_sse_line("   ").unwrap().is_empty());
    }

    #[test]
    fn comment_line_returns_empty() {
        assert!(parse_sse_line(": this is a comment").unwrap().is_empty());
    }

    #[test]
    fn event_line_returns_empty() {
        assert!(parse_sse_line("event: message").unwrap().is_empty());
    }

    #[test]
    fn id_line_returns_empty() {
        assert!(parse_sse_line("id: 123").unwrap().is_empty());
    }

    #[test]
    fn retry_line_returns_empty() {
        assert!(parse_sse_line("retry: 1000").unwrap().is_empty());
    }

    #[test]
    fn data_empty_payload_returns_empty() {
        assert!(parse_sse_line("data:").unwrap().is_empty());
        assert!(parse_sse_line("data: ").unwrap().is_empty());
    }

    // ── [DONE] sentinel ─────────────────────────────────────────────

    #[test]
    fn done_sentinel() {
        let chunks = parse_sse_line("data: [DONE]").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::Done {
                finish_reason: None,
                usage: None,
            }
        );
    }

    #[test]
    fn done_sentinel_no_space() {
        let chunks = parse_sse_line("data:[DONE]").unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], StreamChunk::Done { .. }));
    }

    // ── Text delta ──────────────────────────────────────────────────

    #[test]
    fn text_delta() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::TextDelta {
                text: "Hello".into()
            }
        );
    }

    #[test]
    fn text_delta_with_whitespace() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::TextDelta {
                text: " world".into()
            }
        );
    }

    #[test]
    fn empty_content_delta_skipped() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"content":""},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn role_only_delta_no_content() {
        // First chunk often has role but no content
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert!(chunks.is_empty());
    }

    // ── Tool call delta ─────────────────────────────────────────────

    #[test]
    fn tool_call_delta_first_chunk() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"get_weather","arguments":""}}]},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::ToolCallDelta {
                index: 0,
                id: Some("call_abc".into()),
                name: Some("get_weather".into()),
                arguments: Some(String::new()),
            }
        );
    }

    #[test]
    fn tool_call_delta_argument_fragment() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"city\""}}]},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::ToolCallDelta {
                index: 0,
                id: None,
                name: None,
                arguments: Some("{\"city\"".into()),
            }
        );
    }

    // ── Finish reason ───────────────────────────────────────────────

    #[test]
    fn finish_reason_stop() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::Done {
                finish_reason: Some("stop".into()),
                usage: None,
            }
        );
    }

    #[test]
    fn finish_reason_tool_calls() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::Done {
                finish_reason: Some("tool_calls".into()),
                usage: None,
            }
        );
    }

    #[test]
    fn finish_reason_with_usage() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            StreamChunk::Done {
                finish_reason: Some("stop".into()),
                usage: Some(Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                }),
            }
        );
    }

    // ── Error cases ─────────────────────────────────────────────────

    #[test]
    fn invalid_json_returns_error() {
        let line = "data: {not valid json}";
        let result = parse_sse_line(line);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProviderError::InvalidResponse(_)));
    }

    // ── Multiple chunks from one delta ──────────────────────────────

    #[test]
    fn text_and_finish_in_same_delta() {
        // Some providers send the last text token with finish_reason in the same chunk
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":"stop"}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], StreamChunk::TextDelta { text: "!".into() });
        assert_eq!(
            chunks[1],
            StreamChunk::Done {
                finish_reason: Some("stop".into()),
                usage: None,
            }
        );
    }

    // ── Realistic multi-line SSE stream ─────────────────────────────

    #[test]
    fn parse_full_stream() {
        let stream = [
            "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}",
            "",
            "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}",
            "",
            "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}",
            "",
            "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}",
            "",
            "data: [DONE]",
        ];

        let mut all_chunks: Vec<StreamChunk> = Vec::new();
        for line in &stream {
            all_chunks.extend(parse_sse_line(line).unwrap());
        }

        assert_eq!(all_chunks.len(), 4); // "Hello", " world", Done(stop), Done(sentinel)
        assert_eq!(
            all_chunks[0],
            StreamChunk::TextDelta {
                text: "Hello".into()
            }
        );
        assert_eq!(
            all_chunks[1],
            StreamChunk::TextDelta {
                text: " world".into()
            }
        );
        assert_eq!(
            all_chunks[2],
            StreamChunk::Done {
                finish_reason: Some("stop".into()),
                usage: None,
            }
        );
        assert_eq!(
            all_chunks[3],
            StreamChunk::Done {
                finish_reason: None,
                usage: None,
            }
        );
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn data_with_no_choices() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn data_with_trailing_newline() {
        let line = "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n";
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], StreamChunk::TextDelta { text: "Hi".into() });
    }

    #[test]
    fn multiple_tool_calls_in_one_delta() {
        let line = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"search","arguments":""}},{"index":1,"id":"call_2","type":"function","function":{"name":"fetch","arguments":""}}]},"finish_reason":null}]}"#;
        let chunks = parse_sse_line(line).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(matches!(
            chunks[0],
            StreamChunk::ToolCallDelta { index: 0, .. }
        ));
        assert!(matches!(
            chunks[1],
            StreamChunk::ToolCallDelta { index: 1, .. }
        ));
    }
}
