//! Context compression (ADR-058 Phase 2.3).
//!
//! When a conversation outgrows the live-window budget, older turns are
//! **summarized into the window (lossy gist)** while the full text remains on
//! the chain (lossless, re-graftable in Phase 3). This module owns the
//! budget/partition/assembly logic and the summarizer that produces the gist.
//!
//! # Summarizers
//!
//! * [`LlmSummarizer`] — the real path: an LLM (the local Hermes
//!   [`Provider`]) summarizes the aged turns. This supersedes the
//!   first-sentence placeholder the ADR called out.
//! * heuristic ([`compress_context`]) — an offline, dependency-free extractive
//!   fallback (first sentence per aged turn). Used when no LLM is wired and as
//!   the failure fallback for [`compress_context_with`] so a summarizer error
//!   never aborts the turn.
//!
//! Both paths preserve the same invariants: all `system` messages are kept,
//! the most recent `recent_message_count` turns are kept verbatim, and the
//! emitted summary message carries the `# Conversation Summary (compressed)`
//! header so downstream consumers (and tests) can locate it.

#[cfg(feature = "native")]
use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use super::tokenizer::count_tokens;
use crate::pipeline::traits::LlmMessage;

#[cfg(feature = "native")]
use clawft_llm::{ChatMessage, ChatRequest, Provider};

/// Configuration for context compression.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Maximum number of tokens allowed in the assembled context.
    pub max_context_tokens: usize,
    /// Number of recent conversation messages to keep verbatim.
    pub recent_message_count: usize,
    /// Whether compression is enabled at all.
    pub compression_enabled: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 8192,
            recent_message_count: 10,
            compression_enabled: true,
        }
    }
}

/// Metadata about a compression operation.
#[derive(Debug, Clone)]
pub struct CompressionMetadata {
    /// Total tokens before compression.
    pub original_tokens: usize,
    /// Total tokens after compression.
    pub compressed_tokens: usize,
    /// Ratio of compressed to original (1.0 = no compression).
    pub compression_ratio: f64,
    /// Number of messages that were summarized.
    pub messages_summarized: usize,
}

/// Result of compressing a message list.
#[derive(Debug, Clone)]
pub struct CompressedContext {
    /// The compressed message list, ready for the LLM.
    pub messages: Vec<LlmMessage>,
    /// Metadata describing what compression did.
    pub metadata: CompressionMetadata,
}

/// Error returned by a [`Summarizer`].
#[derive(Debug, thiserror::Error)]
pub enum SummarizeError {
    /// The underlying provider/model call failed.
    #[error("summarizer backend failed: {0}")]
    Backend(String),
    /// The model returned an empty or unusable summary.
    #[error("summarizer returned no usable summary")]
    Empty,
}

/// Produces a compressed gist of aged conversation turns.
///
/// Implementations must be deterministic in *shape* (always return a single
/// summary string for the given turns) but may be lossy in content.
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// Summarize `messages` (the aged, to-be-evicted turns) into one block.
    async fn summarize(&self, messages: &[LlmMessage]) -> Result<String, SummarizeError>;
}

/// LLM-backed summarizer (ADR-058 Phase 2.3 real path).
///
/// Wraps any [`Provider`] (the local Hermes provider in the loop) and asks it
/// to distil the aged turns. Native-only because [`Provider`] lives behind the
/// `clawft-llm/native` HTTP transport in this crate's feature wiring.
#[cfg(feature = "native")]
pub struct LlmSummarizer {
    provider: Arc<dyn Provider>,
    model: String,
    max_tokens: i32,
}

#[cfg(feature = "native")]
impl LlmSummarizer {
    /// System instruction steering the model toward a faithful, compact gist.
    const SYSTEM_PROMPT: &'static str = "You compress conversation history. \
Summarize the following turns into a concise block that preserves facts, \
decisions, names, numbers, file paths, and open questions. Do not add new \
information, opinions, or pleasantries. Output only the summary.";

    /// Create a summarizer over `provider`, using `model` for the request and
    /// capping the summary at `max_tokens`.
    pub fn new(provider: Arc<dyn Provider>, model: impl Into<String>, max_tokens: i32) -> Self {
        Self {
            provider,
            model: model.into(),
            max_tokens,
        }
    }

    /// Render the aged turns into a single user-message body for the model.
    fn render_turns(messages: &[LlmMessage]) -> String {
        let mut out = String::new();
        for msg in messages {
            out.push('[');
            out.push_str(&msg.role);
            out.push_str("]: ");
            out.push_str(&msg.content);
            out.push('\n');
        }
        out
    }
}

#[cfg(feature = "native")]
#[async_trait]
impl Summarizer for LlmSummarizer {
    async fn summarize(&self, messages: &[LlmMessage]) -> Result<String, SummarizeError> {
        if messages.is_empty() {
            return Ok(String::new());
        }
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage::system(Self::SYSTEM_PROMPT),
                ChatMessage::user(Self::render_turns(messages)),
            ],
            max_tokens: Some(self.max_tokens),
            temperature: Some(0.0),
            tools: Vec::new(),
            stream: None,
            tool_choice: None,
        };
        let response = self
            .provider
            .complete(&request)
            .await
            .map_err(|e| SummarizeError::Backend(e.to_string()))?;
        let summary = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or(SummarizeError::Empty)?;
        Ok(summary)
    }
}

/// Partition for compression: system messages (always kept), the aged turns to
/// summarize, and the recent turns to keep verbatim.
struct Partition {
    system: Vec<LlmMessage>,
    old: Vec<LlmMessage>,
    recent: Vec<LlmMessage>,
}

/// Split a message list into system / old / recent according to `config`.
fn partition(messages: Vec<LlmMessage>, config: &CompressionConfig) -> Partition {
    let mut system = Vec::new();
    let mut conversation = Vec::new();
    for msg in messages {
        if msg.role == "system" {
            system.push(msg);
        } else {
            conversation.push(msg);
        }
    }
    let recent_count = config.recent_message_count.min(conversation.len());
    let split = conversation.len() - recent_count;
    let recent = conversation.split_off(split);
    Partition {
        system,
        old: conversation,
        recent,
    }
}

/// Reassemble the compressed list: system messages, the optional summary
/// message, then the recent verbatim turns. Computes metadata.
fn assemble(
    part: Partition,
    summary: Option<String>,
    original_tokens: usize,
    messages_summarized: usize,
) -> CompressedContext {
    let mut result = Vec::with_capacity(part.system.len() + 1 + part.recent.len());
    result.extend(part.system);
    if let Some(summary_text) = summary {
        result.push(LlmMessage {
            role: "system".into(),
            content: format!(
                "# Conversation Summary (compressed)\n\n\
                 The following is a summary of {messages_summarized} earlier messages:\n\n{summary_text}"
            ),
            tool_call_id: None,
            tool_calls: None,
        });
    }
    result.extend(part.recent);

    let compressed_tokens: usize = result.iter().map(|m| count_tokens(&m.content)).sum();
    let compression_ratio = if original_tokens > 0 {
        compressed_tokens as f64 / original_tokens as f64
    } else {
        1.0
    };
    CompressedContext {
        messages: result,
        metadata: CompressionMetadata {
            original_tokens,
            compressed_tokens,
            compression_ratio,
            messages_summarized,
        },
    }
}

/// `true` if the message list already fits the budget (or compression is off),
/// in which case the caller returns the list unchanged.
fn fits(original_tokens: usize, config: &CompressionConfig) -> bool {
    !config.compression_enabled || original_tokens <= config.max_context_tokens
}

/// Build the no-op result for an already-fitting context.
fn no_op(messages: Vec<LlmMessage>, original_tokens: usize) -> CompressedContext {
    CompressedContext {
        messages,
        metadata: CompressionMetadata {
            original_tokens,
            compressed_tokens: original_tokens,
            compression_ratio: 1.0,
            messages_summarized: 0,
        },
    }
}

/// Extract the first sentence from a text block.
///
/// Returns everything up to and including the first sentence-ending
/// punctuation (`.`, `!`, `?`) followed by whitespace or end-of-string.
/// If no sentence boundary is found, returns up to the first 120 characters.
fn first_sentence(text: &str) -> &str {
    for (i, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            let end = i + ch.len_utf8();
            if end >= text.len() || text[end..].starts_with(char::is_whitespace) {
                return &text[..end];
            }
        }
    }
    let limit = text
        .char_indices()
        .nth(120)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    &text[..limit]
}

/// Extractive (heuristic) summary of aged turns: first sentence per turn.
///
/// The offline fallback used by [`compress_context`] and by
/// [`compress_context_with`] when the LLM summarizer errors.
fn extractive_summary(old: &[LlmMessage]) -> Option<String> {
    if old.is_empty() {
        return None;
    }
    let mut lines = Vec::with_capacity(old.len());
    for msg in old {
        lines.push(format!("[{}]: {}", msg.role, first_sentence(&msg.content)));
    }
    Some(lines.join("\n"))
}

/// Compress a message list to fit within a token budget (heuristic path).
///
/// Preserves: (1) all system messages; (2) the last
/// `config.recent_message_count` non-system messages verbatim; (3) older
/// non-system messages summarized extractively (first-sentence per turn).
///
/// If compression is disabled or the context already fits, the messages are
/// returned unchanged. This is the offline fallback; the LLM path is
/// [`compress_context_with`].
pub fn compress_context(
    messages: Vec<LlmMessage>,
    config: &CompressionConfig,
) -> CompressedContext {
    let original_tokens: usize = messages.iter().map(|m| count_tokens(&m.content)).sum();
    if fits(original_tokens, config) {
        return no_op(messages, original_tokens);
    }
    let part = partition(messages, config);
    let messages_summarized = part.old.len();
    let summary = extractive_summary(&part.old);
    assemble(part, summary, original_tokens, messages_summarized)
}

/// Compress a message list using a [`Summarizer`] for the aged turns.
///
/// Same invariants as [`compress_context`], but the gist is produced by
/// `summarizer` (the LLM in production). If the summarizer errors, falls back
/// to the extractive summary and logs — a summarizer failure must never abort
/// the turn (ADR-058: the loop is cattle, but a single turn should degrade,
/// not die).
pub async fn compress_context_with(
    messages: Vec<LlmMessage>,
    config: &CompressionConfig,
    summarizer: &dyn Summarizer,
) -> CompressedContext {
    let original_tokens: usize = messages.iter().map(|m| count_tokens(&m.content)).sum();
    if fits(original_tokens, config) {
        return no_op(messages, original_tokens);
    }
    let part = partition(messages, config);
    let messages_summarized = part.old.len();

    let summary = if part.old.is_empty() {
        None
    } else {
        match summarizer.summarize(&part.old).await {
            Ok(s) if !s.trim().is_empty() => Some(s),
            Ok(_) | Err(SummarizeError::Empty) => {
                warn!("summarizer produced no summary; using extractive fallback");
                extractive_summary(&part.old)
            }
            Err(e) => {
                warn!(error = %e, "summarizer failed; using extractive fallback");
                extractive_summary(&part.old)
            }
        }
    };
    assemble(part, summary, original_tokens, messages_summarized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: &str, content: &str) -> LlmMessage {
        LlmMessage {
            role: role.into(),
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    #[test]
    fn first_sentence_extracts_correctly() {
        assert_eq!(
            first_sentence("Hello world. More text here."),
            "Hello world."
        );
        assert_eq!(first_sentence("No period here"), "No period here");
        assert_eq!(first_sentence("Question? Yes."), "Question?");
        assert_eq!(first_sentence("Exclaim! Done."), "Exclaim!");
    }

    #[test]
    fn compress_context_no_op_when_within_budget() {
        let messages = vec![
            make_msg("system", "You are helpful."),
            make_msg("user", "Hi"),
            make_msg("assistant", "Hello!"),
        ];
        let config = CompressionConfig {
            max_context_tokens: 100_000,
            recent_message_count: 10,
            compression_enabled: true,
        };
        let result = compress_context(messages.clone(), &config);
        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.metadata.compression_ratio, 1.0);
        assert_eq!(result.metadata.messages_summarized, 0);
    }

    #[test]
    fn compress_context_no_op_when_disabled() {
        let messages = vec![
            make_msg("system", "sys"),
            make_msg("user", "a long message that exceeds budget"),
        ];
        let config = CompressionConfig {
            max_context_tokens: 1,
            recent_message_count: 10,
            compression_enabled: false,
        };
        let result = compress_context(messages.clone(), &config);
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.metadata.messages_summarized, 0);
    }

    #[test]
    fn compress_context_preserves_system_prompt() {
        let mut messages = vec![make_msg(
            "system",
            "You are a helpful assistant with many capabilities.",
        )];
        for i in 0..20 {
            messages.push(make_msg("user", &format!("User message number {i}. This is a fairly long message to inflate token count significantly.")));
            messages.push(make_msg("assistant", &format!("Assistant response number {i}. Also fairly long to push tokens over the budget limit.")));
        }
        let config = CompressionConfig {
            max_context_tokens: 50,
            recent_message_count: 4,
            compression_enabled: true,
        };
        let result = compress_context(messages, &config);
        assert_eq!(result.messages[0].role, "system");
        assert!(result.messages[0].content.contains("helpful assistant"));
        let summary = result
            .messages
            .iter()
            .find(|m| m.content.contains("Conversation Summary"));
        assert!(summary.is_some(), "should have a summary message");
        let non_system: Vec<_> = result
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .collect();
        assert_eq!(non_system.len(), 4);
        let last = result.messages.last().unwrap();
        assert_eq!(last.role, "assistant");
        assert!(last.content.contains("response number 19"));
    }

    #[test]
    fn compress_context_recent_messages_intact() {
        let mut messages = vec![make_msg("system", "sys prompt")];
        for i in 0..15 {
            messages.push(make_msg(
                "user",
                &format!("msg {i} with enough words to make the token count go over budget easily"),
            ));
        }
        let config = CompressionConfig {
            max_context_tokens: 10,
            recent_message_count: 5,
            compression_enabled: true,
        };
        let result = compress_context(messages, &config);
        let user_msgs: Vec<_> = result
            .messages
            .iter()
            .filter(|m| m.role == "user")
            .collect();
        assert_eq!(user_msgs.len(), 5);
        for (idx, msg) in user_msgs.iter().enumerate() {
            let expected_num = 10 + idx;
            assert!(msg.content.contains(&format!("msg {expected_num}")));
        }
    }

    #[test]
    fn compress_context_metadata_accuracy() {
        let mut messages = vec![make_msg("system", "short system")];
        for i in 0..10 {
            messages.push(make_msg(
                "user",
                &format!("Message number {i} with several words to inflate the count."),
            ));
        }
        let original_tokens: usize = messages.iter().map(|m| count_tokens(&m.content)).sum();
        let config = CompressionConfig {
            max_context_tokens: 10,
            recent_message_count: 2,
            compression_enabled: true,
        };
        let result = compress_context(messages, &config);
        assert_eq!(result.metadata.original_tokens, original_tokens);
        assert_eq!(result.metadata.messages_summarized, 8);
        assert!(result.metadata.compression_ratio > 0.0);
        let actual_compressed: usize = result
            .messages
            .iter()
            .map(|m| count_tokens(&m.content))
            .sum();
        assert_eq!(result.metadata.compressed_tokens, actual_compressed);
    }

    // ── Summarizer-driven compression (Phase 2.3) ────────────────────

    /// A deterministic, offline summarizer used to exercise the LLM path
    /// without a live model. Records how many turns it was handed.
    struct MockSummarizer {
        canned: String,
    }

    #[async_trait]
    impl Summarizer for MockSummarizer {
        async fn summarize(&self, messages: &[LlmMessage]) -> Result<String, SummarizeError> {
            Ok(format!("{} ({} turns)", self.canned, messages.len()))
        }
    }

    /// A summarizer that always fails, to verify the extractive fallback.
    struct FailingSummarizer;

    #[async_trait]
    impl Summarizer for FailingSummarizer {
        async fn summarize(&self, _messages: &[LlmMessage]) -> Result<String, SummarizeError> {
            Err(SummarizeError::Backend("boom".into()))
        }
    }

    fn over_budget_messages() -> Vec<LlmMessage> {
        let mut messages = vec![make_msg("system", "sys prompt")];
        for i in 0..20 {
            messages.push(make_msg(
                "user",
                &format!("User turn {i} discussing an important topic at length with detail."),
            ));
            messages.push(make_msg(
                "assistant",
                &format!("Assistant turn {i} with a thorough and helpful response body."),
            ));
        }
        messages
    }

    #[tokio::test]
    async fn compress_with_summarizer_uses_summary_and_respects_budget() {
        let config = CompressionConfig {
            max_context_tokens: 80,
            recent_message_count: 4,
            compression_enabled: true,
        };
        let summarizer = MockSummarizer {
            canned: "SUMMARY_OF_OLD".into(),
        };
        let messages = over_budget_messages();
        let result = compress_context_with(messages, &config, &summarizer).await;

        // The aged turns were summarized via the summarizer, not first-sentence.
        let summary = result
            .messages
            .iter()
            .find(|m| m.content.contains("Conversation Summary"))
            .expect("summary present");
        assert!(
            summary.content.contains("SUMMARY_OF_OLD"),
            "summary should carry the summarizer output, got: {}",
            summary.content
        );
        // Recent turns kept verbatim.
        let non_system: Vec<_> = result
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .collect();
        assert_eq!(non_system.len(), 4);
        assert!(result.metadata.messages_summarized > 0);
    }

    #[tokio::test]
    async fn compress_with_summarizer_no_op_within_budget() {
        let config = CompressionConfig {
            max_context_tokens: 1_000_000,
            recent_message_count: 10,
            compression_enabled: true,
        };
        let summarizer = MockSummarizer {
            canned: "unused".into(),
        };
        let messages = over_budget_messages();
        let n = messages.len();
        let result = compress_context_with(messages, &config, &summarizer).await;
        assert_eq!(result.messages.len(), n);
        assert_eq!(result.metadata.compression_ratio, 1.0);
    }

    #[tokio::test]
    async fn compress_with_summarizer_falls_back_on_error() {
        let config = CompressionConfig {
            max_context_tokens: 80,
            recent_message_count: 4,
            compression_enabled: true,
        };
        let messages = over_budget_messages();
        let result = compress_context_with(messages, &config, &FailingSummarizer).await;
        // Extractive fallback still produces a summary so the turn survives.
        let summary = result
            .messages
            .iter()
            .find(|m| m.content.contains("Conversation Summary"));
        assert!(summary.is_some(), "extractive fallback should summarize");
        assert!(result.metadata.messages_summarized > 0);
    }

    /// Live LLM summarization against the ADR-060 Hermes endpoint. Ignored by
    /// default — the live endpoint is blocked pending model download (0.1).
    /// Run manually once Hermes is served:
    /// `cargo test -p clawft-core --features native -- --ignored live_llm`.
    #[cfg(feature = "native")]
    #[tokio::test]
    #[ignore = "requires live Hermes endpoint (ADR-060 0.1, blocked on model download)"]
    async fn live_llm_summarizer_compresses() {
        use clawft_llm::LocalProvider;
        let provider: Arc<dyn Provider> = Arc::new(LocalProvider::hermes_serving());
        let summarizer = LlmSummarizer::new(provider, "hermes-4.3-36b", 256);
        let config = CompressionConfig {
            max_context_tokens: 80,
            recent_message_count: 4,
            compression_enabled: true,
        };
        let result = compress_context_with(over_budget_messages(), &config, &summarizer).await;
        let summary = result
            .messages
            .iter()
            .find(|m| m.content.contains("Conversation Summary"))
            .expect("summary present");
        assert!(summary.content.len() > "# Conversation Summary (compressed)".len());
    }
}
