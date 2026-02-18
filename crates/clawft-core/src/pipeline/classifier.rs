//! Keyword classifier (Level 0 implementation).
//!
//! Classifies incoming chat requests by scanning the last user message
//! for keyword patterns. This is the simplest possible classifier -- no
//! ML, no embeddings, just case-insensitive substring matching.
//!
//! Keyword groups are checked in priority order; the first match wins.
//! If no keywords match, the task defaults to [`TaskType::Chat`].

use super::traits::{ChatRequest, TaskClassifier, TaskProfile, TaskType};

/// A keyword pattern entry: a list of keywords and the task type they map to.
struct KeywordPattern {
    keywords: &'static [&'static str],
    task_type: TaskType,
}

/// Static keyword patterns checked in priority order (first match wins).
///
/// Code-related patterns are checked before analysis because keywords like
/// "fix" and "debug" are more specific than "explain" or "summarize".
const PATTERNS: &[KeywordPattern] = &[
    // Code generation (most specific -- check first)
    KeywordPattern {
        keywords: &[
            "code",
            "function",
            "implement",
            "debug",
            "fix",
            "program",
            "script",
            "compile",
            "refactor",
            "class",
            "struct",
            "module",
        ],
        task_type: TaskType::CodeGeneration,
    },
    // Code review
    KeywordPattern {
        keywords: &["review", "check", "audit", "lint", "inspect"],
        task_type: TaskType::CodeReview,
    },
    // Research
    KeywordPattern {
        keywords: &[
            "search", "find", "research", "look up", "lookup", "discover",
        ],
        task_type: TaskType::Research,
    },
    // Creative
    KeywordPattern {
        keywords: &[
            "write",
            "story",
            "poem",
            "creative",
            "compose",
            "draft",
            "narrative",
        ],
        task_type: TaskType::Creative,
    },
    // Analysis
    KeywordPattern {
        keywords: &[
            "analyze",
            "explain",
            "summarize",
            "compare",
            "evaluate",
            "assess",
        ],
        task_type: TaskType::Analysis,
    },
    // Tool use
    KeywordPattern {
        keywords: &["use tool", "run tool", "execute", "call function"],
        task_type: TaskType::ToolUse,
    },
];

/// Level 0 keyword-based task classifier.
///
/// Scans the last user message for known keyword patterns and returns
/// a [`TaskProfile`] with the detected type, estimated complexity, and
/// matched keywords.
///
/// # Complexity heuristic
///
/// Complexity is estimated as the ratio of matched keywords to total words
/// in the message, clamped to \[0.1, 0.9\]. More keyword hits = higher
/// estimated complexity (the request is touching multiple concerns).
pub struct KeywordClassifier;

impl KeywordClassifier {
    /// Create a new keyword classifier.
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeywordClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskClassifier for KeywordClassifier {
    fn classify(&self, request: &ChatRequest) -> TaskProfile {
        // Extract the last user message content, lowercased.
        let text = last_user_message(request);
        let text_lower = text.to_lowercase();

        // Scan patterns in priority order.
        let mut matched_type = TaskType::Chat;
        let mut matched_keywords: Vec<String> = Vec::new();

        for pattern in PATTERNS {
            let hits: Vec<String> = pattern
                .keywords
                .iter()
                .filter(|kw| text_lower.contains(**kw))
                .map(|kw| (*kw).to_string())
                .collect();

            if !hits.is_empty() && matched_keywords.is_empty() {
                // First matching pattern determines the type.
                matched_type = pattern.task_type.clone();
                matched_keywords = hits;
            } else if !hits.is_empty() {
                // Additional matches from later patterns still count for complexity.
                matched_keywords.extend(hits);
            }
        }

        // Estimate complexity from keyword density.
        let word_count = text_lower.split_whitespace().count().max(1) as f32;
        let keyword_density = matched_keywords.len() as f32 / word_count;
        let complexity = keyword_density.clamp(0.1, 0.9);

        TaskProfile {
            task_type: matched_type,
            complexity,
            keywords: matched_keywords,
        }
    }
}

/// Extract the content of the last user message, or empty string if none.
fn last_user_message(request: &ChatRequest) -> String {
    request
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::traits::LlmMessage;

    fn make_request(user_content: &str) -> ChatRequest {
        ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: user_content.into(),
                tool_call_id: None,
            }],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        }
    }

    fn make_request_with_system(system: &str, user: &str) -> ChatRequest {
        ChatRequest {
            messages: vec![
                LlmMessage {
                    role: "system".into(),
                    content: system.into(),
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "user".into(),
                    content: user.into(),
                    tool_call_id: None,
                },
            ],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        }
    }

    #[test]
    fn classify_code_generation() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Please implement a sorting function"));
        assert_eq!(profile.task_type, TaskType::CodeGeneration);
        assert!(profile.keywords.contains(&"implement".to_string()));
        assert!(profile.keywords.contains(&"function".to_string()));
    }

    #[test]
    fn classify_code_generation_debug() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Can you debug this code for me?"));
        assert_eq!(profile.task_type, TaskType::CodeGeneration);
        assert!(profile.keywords.contains(&"debug".to_string()));
        assert!(profile.keywords.contains(&"code".to_string()));
    }

    #[test]
    fn classify_code_review() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Please review my pull request"));
        assert_eq!(profile.task_type, TaskType::CodeReview);
        assert!(profile.keywords.contains(&"review".to_string()));
    }

    #[test]
    fn classify_research() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Research the best database options"));
        assert_eq!(profile.task_type, TaskType::Research);
        assert!(profile.keywords.contains(&"research".to_string()));
    }

    #[test]
    fn classify_research_lookup() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Can you look up the API docs?"));
        assert_eq!(profile.task_type, TaskType::Research);
        assert!(profile.keywords.contains(&"look up".to_string()));
    }

    #[test]
    fn classify_creative() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Write me a short poem about Rust"));
        assert_eq!(profile.task_type, TaskType::Creative);
        assert!(profile.keywords.contains(&"poem".to_string()));
    }

    #[test]
    fn classify_analysis() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Explain how tokio works"));
        assert_eq!(profile.task_type, TaskType::Analysis);
        assert!(profile.keywords.contains(&"explain".to_string()));
    }

    #[test]
    fn classify_analysis_summarize() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Summarize this document for me"));
        assert_eq!(profile.task_type, TaskType::Analysis);
        assert!(profile.keywords.contains(&"summarize".to_string()));
    }

    #[test]
    fn classify_tool_use() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Use tool to fetch the data"));
        assert_eq!(profile.task_type, TaskType::ToolUse);
        assert!(profile.keywords.contains(&"use tool".to_string()));
    }

    #[test]
    fn classify_defaults_to_chat() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("Hello, how are you today?"));
        assert_eq!(profile.task_type, TaskType::Chat);
        assert!(profile.keywords.is_empty());
    }

    #[test]
    fn classify_empty_message() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request(""));
        assert_eq!(profile.task_type, TaskType::Chat);
        assert!(profile.keywords.is_empty());
    }

    #[test]
    fn classify_no_user_messages() {
        let classifier = KeywordClassifier::new();
        let req = ChatRequest {
            messages: vec![LlmMessage {
                role: "system".into(),
                content: "You are helpful. Implement code.".into(),
                tool_call_id: None,
            }],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        };
        // Only the last user message is scanned; system messages are ignored.
        let profile = classifier.classify(&req);
        assert_eq!(profile.task_type, TaskType::Chat);
    }

    #[test]
    fn classify_case_insensitive() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request("IMPLEMENT a FUNCTION in Rust"));
        assert_eq!(profile.task_type, TaskType::CodeGeneration);
    }

    #[test]
    fn classify_uses_last_user_message() {
        let classifier = KeywordClassifier::new();
        let req = ChatRequest {
            messages: vec![
                LlmMessage {
                    role: "user".into(),
                    content: "Research Rust crates".into(),
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "assistant".into(),
                    content: "Here are some crates...".into(),
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Now implement the function".into(),
                    tool_call_id: None,
                },
            ],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        };
        let profile = classifier.classify(&req);
        assert_eq!(profile.task_type, TaskType::CodeGeneration);
    }

    #[test]
    fn classify_complexity_increases_with_keywords() {
        let classifier = KeywordClassifier::new();

        // Few keywords relative to message length
        let short = classifier.classify(&make_request(
            "Can you implement something for me in my big project today please?",
        ));

        // Many keywords relative to message length
        let dense = classifier.classify(&make_request("implement debug fix code"));

        assert!(
            dense.complexity > short.complexity,
            "dense={} should be > short={}",
            dense.complexity,
            short.complexity
        );
    }

    #[test]
    fn classify_complexity_clamped_min() {
        let classifier = KeywordClassifier::new();
        // Very long message with one keyword -- complexity should be at least 0.1
        let profile = classifier.classify(&make_request(
            "This is a very long message with lots of words but only one keyword: code. \
             I am padding this out with many many more words to make the density very low.",
        ));
        assert!(
            profile.complexity >= 0.1,
            "complexity {} should be >= 0.1",
            profile.complexity
        );
    }

    #[test]
    fn classify_complexity_clamped_max() {
        let classifier = KeywordClassifier::new();
        // All keywords, very dense
        let profile = classifier.classify(&make_request("code function implement debug fix"));
        assert!(
            profile.complexity <= 0.9,
            "complexity {} should be <= 0.9",
            profile.complexity
        );
    }

    #[test]
    fn default_trait_impl() {
        let classifier = KeywordClassifier;
        let profile = classifier.classify(&make_request("hello"));
        assert_eq!(profile.task_type, TaskType::Chat);
    }

    #[test]
    fn classify_with_system_message_uses_user_only() {
        let classifier = KeywordClassifier::new();
        let profile = classifier.classify(&make_request_with_system(
            "You are a code reviewer.",
            "Hello, how is your day?",
        ));
        // "code" and "review" are in the system message, not the user message.
        assert_eq!(profile.task_type, TaskType::Chat);
    }

    #[test]
    fn classify_priority_code_over_analysis() {
        let classifier = KeywordClassifier::new();
        // "code" and "explain" both match, but code patterns are checked first.
        let profile = classifier.classify(&make_request("Explain this code to me"));
        assert_eq!(profile.task_type, TaskType::CodeGeneration);
    }
}
