//! Token budget assembler (Level 0 implementation).
//!
//! Truncates conversation history to fit within a token budget.
//! Uses a simple `chars / 4` heuristic for token estimation.
//!
//! The assembler always preserves the first (system) message and
//! the most recent messages, dropping older messages from the middle
//! when the budget is exceeded.

use async_trait::async_trait;

use super::traits::{
    AssembledContext, ChatRequest, ContextAssembler, LlmMessage, TaskProfile,
};

/// Level 0 token-budget context assembler.
///
/// Estimates token count as `character_count / 4` and truncates
/// messages from the beginning of the conversation (keeping the
/// first system message and most recent messages) to fit within
/// the configured budget.
pub struct TokenBudgetAssembler {
    max_tokens: usize,
}

impl TokenBudgetAssembler {
    /// Create a new assembler with the given token budget.
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Returns the configured maximum token budget.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }
}

#[async_trait]
impl ContextAssembler for TokenBudgetAssembler {
    async fn assemble(&self, request: &ChatRequest, _profile: &TaskProfile) -> AssembledContext {
        let messages = &request.messages;

        if messages.is_empty() {
            return AssembledContext {
                messages: vec![],
                token_estimate: 0,
                truncated: false,
            };
        }

        let total_tokens = estimate_tokens_for_messages(messages);

        // If everything fits, return as-is.
        if total_tokens <= self.max_tokens {
            return AssembledContext {
                messages: messages.clone(),
                token_estimate: total_tokens,
                truncated: false,
            };
        }

        // Truncation strategy:
        //   1. Always keep the first message (typically a system prompt).
        //   2. Add messages from the end until we hit the budget.
        //   3. Drop messages from the middle.
        let first = &messages[0];
        let first_tokens = estimate_tokens(first);

        if messages.len() == 1 {
            // Single message that exceeds budget -- include it anyway.
            return AssembledContext {
                messages: vec![first.clone()],
                token_estimate: first_tokens,
                truncated: true,
            };
        }

        let mut remaining_budget = self.max_tokens.saturating_sub(first_tokens);
        let mut tail_messages: Vec<LlmMessage> = Vec::new();

        // Walk backwards from the end, adding messages until budget is exhausted.
        for msg in messages[1..].iter().rev() {
            let msg_tokens = estimate_tokens(msg);
            if msg_tokens > remaining_budget {
                break;
            }
            remaining_budget -= msg_tokens;
            tail_messages.push(msg.clone());
        }

        // Reverse so messages are in chronological order.
        tail_messages.reverse();

        let mut result = Vec::with_capacity(1 + tail_messages.len());
        result.push(first.clone());
        result.extend(tail_messages);

        let final_tokens = estimate_tokens_for_messages(&result);
        let truncated = result.len() < messages.len();

        AssembledContext {
            messages: result,
            token_estimate: final_tokens,
            truncated,
        }
    }
}

/// Estimate token count for a single message (chars / 4 heuristic).
///
/// Also accounts for the role field and a small overhead per message
/// (the role name + structural tokens like `<|im_start|>` / JSON keys).
fn estimate_tokens(msg: &LlmMessage) -> usize {
    let content_tokens = msg.content.len() / 4;
    let role_overhead = 4; // ~4 tokens for role + message structure
    content_tokens + role_overhead
}

/// Estimate total token count for a slice of messages.
fn estimate_tokens_for_messages(messages: &[LlmMessage]) -> usize {
    messages.iter().map(estimate_tokens).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::traits::TaskType;

    fn make_message(role: &str, content: &str) -> LlmMessage {
        LlmMessage {
            role: role.into(),
            content: content.into(),
            tool_call_id: None,
        }
    }

    fn make_profile() -> TaskProfile {
        TaskProfile {
            task_type: TaskType::Chat,
            complexity: 0.3,
            keywords: vec![],
        }
    }

    fn make_request(messages: Vec<LlmMessage>) -> ChatRequest {
        ChatRequest {
            messages,
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        }
    }

    #[tokio::test]
    async fn empty_messages() {
        let assembler = TokenBudgetAssembler::new(1000);
        let req = make_request(vec![]);
        let ctx = assembler.assemble(&req, &make_profile()).await;
        assert!(ctx.messages.is_empty());
        assert_eq!(ctx.token_estimate, 0);
        assert!(!ctx.truncated);
    }

    #[tokio::test]
    async fn single_message_fits() {
        let assembler = TokenBudgetAssembler::new(1000);
        let req = make_request(vec![make_message("system", "You are helpful.")]);
        let ctx = assembler.assemble(&req, &make_profile()).await;
        assert_eq!(ctx.messages.len(), 1);
        assert!(!ctx.truncated);
        assert!(ctx.token_estimate > 0);
    }

    #[tokio::test]
    async fn all_messages_fit_no_truncation() {
        let assembler = TokenBudgetAssembler::new(10000);
        let req = make_request(vec![
            make_message("system", "You are helpful."),
            make_message("user", "Hello"),
            make_message("assistant", "Hi there!"),
            make_message("user", "How are you?"),
        ]);
        let ctx = assembler.assemble(&req, &make_profile()).await;
        assert_eq!(ctx.messages.len(), 4);
        assert!(!ctx.truncated);
    }

    #[tokio::test]
    async fn truncation_preserves_system_message() {
        // Create a very tight budget: system message + 1 more message only.
        let assembler = TokenBudgetAssembler::new(20);
        let req = make_request(vec![
            make_message("system", "Be helpful."),         // ~4 + 4 = 7 tokens
            make_message("user", "First question here"),   // ~5 + 4 = 9 tokens
            make_message("assistant", "First answer here"),// ~5 + 4 = 9 tokens
            make_message("user", "Recent msg"),            // ~3 + 4 = 6 tokens
        ]);
        let ctx = assembler.assemble(&req, &make_profile()).await;

        // System message is always first.
        assert_eq!(ctx.messages[0].role, "system");
        assert_eq!(ctx.messages[0].content, "Be helpful.");

        // Should be truncated since not all messages fit.
        assert!(ctx.truncated);
        assert!(ctx.messages.len() < 4);

        // Last message should be the most recent one that fits.
        let last = ctx.messages.last().unwrap();
        assert_eq!(last.content, "Recent msg");
    }

    #[tokio::test]
    async fn truncation_keeps_most_recent_messages() {
        // Budget that can hold system + ~2 short messages.
        let assembler = TokenBudgetAssembler::new(25);
        let req = make_request(vec![
            make_message("system", "System."),        // ~2 + 4 = 5 tokens
            make_message("user", "old msg one"),       // ~3 + 4 = 7 tokens
            make_message("assistant", "old reply"),    // ~3 + 4 = 6 tokens
            make_message("user", "recent question"),   // ~4 + 4 = 7 tokens
            make_message("assistant", "recent reply"), // ~3 + 4 = 7 tokens
        ]);
        let ctx = assembler.assemble(&req, &make_profile()).await;

        assert!(ctx.truncated);
        // System message is first.
        assert_eq!(ctx.messages[0].role, "system");
        // Most recent messages are preserved.
        let last = ctx.messages.last().unwrap();
        assert_eq!(last.content, "recent reply");
    }

    #[tokio::test]
    async fn single_oversized_message_included() {
        // Budget of 5, but system message alone is larger.
        let assembler = TokenBudgetAssembler::new(5);
        let req = make_request(vec![make_message(
            "system",
            "This is a very long system prompt that exceeds the budget",
        )]);
        let ctx = assembler.assemble(&req, &make_profile()).await;
        // Single message is always included even if it exceeds budget.
        assert_eq!(ctx.messages.len(), 1);
        assert!(ctx.truncated);
    }

    #[test]
    fn estimate_tokens_basic() {
        let msg = make_message("user", "Hello world"); // 11 chars -> 2 + 4 = 6
        let tokens = estimate_tokens(&msg);
        assert_eq!(tokens, 6);
    }

    #[test]
    fn estimate_tokens_empty_content() {
        let msg = make_message("user", ""); // 0 chars -> 0 + 4 = 4
        let tokens = estimate_tokens(&msg);
        assert_eq!(tokens, 4); // role overhead only
    }

    #[test]
    fn estimate_tokens_long_content() {
        let content = "a".repeat(400); // 400 chars -> 100 + 4 = 104
        let msg = make_message("user", &content);
        let tokens = estimate_tokens(&msg);
        assert_eq!(tokens, 104);
    }

    #[test]
    fn estimate_tokens_for_multiple_messages() {
        let messages = vec![
            make_message("system", "You are helpful."), // 16/4 + 4 = 8
            make_message("user", "Hello"),               // 5/4 + 4 = 5
        ];
        let total = estimate_tokens_for_messages(&messages);
        assert_eq!(total, 13);
    }

    #[test]
    fn max_tokens_accessor() {
        let assembler = TokenBudgetAssembler::new(4096);
        assert_eq!(assembler.max_tokens(), 4096);
    }

    #[tokio::test]
    async fn exact_budget_fit_no_truncation() {
        // Build messages that exactly fit the budget.
        let msg1 = make_message("system", "ok"); // 0 + 4 = 4
        let msg2 = make_message("user", "hi");   // 0 + 4 = 4
        let budget = estimate_tokens(&msg1) + estimate_tokens(&msg2);

        let assembler = TokenBudgetAssembler::new(budget);
        let req = make_request(vec![msg1, msg2]);
        let ctx = assembler.assemble(&req, &make_profile()).await;
        assert_eq!(ctx.messages.len(), 2);
        assert!(!ctx.truncated);
    }

    #[tokio::test]
    async fn profile_is_ignored() {
        // The Level 0 assembler ignores the task profile entirely.
        let assembler = TokenBudgetAssembler::new(10000);
        let req = make_request(vec![make_message("user", "test")]);

        let profile_a = TaskProfile {
            task_type: TaskType::CodeGeneration,
            complexity: 0.9,
            keywords: vec!["code".into()],
        };
        let profile_b = TaskProfile {
            task_type: TaskType::Chat,
            complexity: 0.1,
            keywords: vec![],
        };

        let ctx_a = assembler.assemble(&req, &profile_a).await;
        let ctx_b = assembler.assemble(&req, &profile_b).await;
        assert_eq!(ctx_a.messages.len(), ctx_b.messages.len());
        assert_eq!(ctx_a.token_estimate, ctx_b.token_estimate);
    }
}
