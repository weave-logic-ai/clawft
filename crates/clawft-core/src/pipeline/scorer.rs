//! No-op scorer (Level 0 implementation).
//!
//! Returns perfect quality scores for every response. Real scoring
//! will be implemented in Level 1+ with heuristics (response length,
//! format compliance) or Level 2+ with neural quality models.

use clawft_types::provider::LlmResponse;

use super::traits::{ChatRequest, QualityScore, QualityScorer};

/// Level 0 no-op quality scorer.
///
/// Always returns perfect scores (1.0 for all dimensions).
/// This serves as a baseline for the pipeline and will be replaced
/// by heuristic or ML-based scorers at higher levels.
pub struct NoopScorer;

impl NoopScorer {
    /// Create a new no-op scorer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl QualityScorer for NoopScorer {
    fn score(&self, _request: &ChatRequest, _response: &LlmResponse) -> QualityScore {
        QualityScore {
            overall: 1.0,
            relevance: 1.0,
            coherence: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::traits::LlmMessage;
    use clawft_types::provider::{ContentBlock, StopReason, Usage};
    use std::collections::HashMap;

    fn make_request() -> ChatRequest {
        ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "hello".into(),
                tool_call_id: None,
                tool_calls: None,
            }],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        }
    }

    fn make_response() -> LlmResponse {
        LlmResponse {
            id: "resp-1".into(),
            content: vec![ContentBlock::Text {
                text: "Hi there!".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 3,
            },
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn noop_scorer_returns_perfect_scores() {
        let scorer = NoopScorer::new();
        let score = scorer.score(&make_request(), &make_response());
        assert!((score.overall - 1.0).abs() < f32::EPSILON);
        assert!((score.relevance - 1.0).abs() < f32::EPSILON);
        assert!((score.coherence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn noop_scorer_ignores_request_content() {
        let scorer = NoopScorer::new();
        let req = ChatRequest {
            messages: vec![
                LlmMessage {
                    role: "system".into(),
                    content: "You are a code reviewer.".into(),
                    tool_call_id: None,
                    tool_calls: None,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Review my code: fn main() {}".into(),
                    tool_call_id: None,
                    tool_calls: None,
                },
            ],
            tools: vec![serde_json::json!({"type": "function"})],
            model: Some("gpt-4o".into()),
            max_tokens: Some(4096),
            temperature: Some(0.0),
        };
        let score = scorer.score(&req, &make_response());
        assert!((score.overall - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn noop_scorer_ignores_response_content() {
        let scorer = NoopScorer::new();
        let resp = LlmResponse {
            id: "bad-response".into(),
            content: vec![],
            stop_reason: StopReason::MaxTokens,
            usage: Usage {
                input_tokens: 1000,
                output_tokens: 0,
            },
            metadata: HashMap::new(),
        };
        // Even an empty response with MaxTokens stop gets perfect scores.
        let score = scorer.score(&make_request(), &resp);
        assert!((score.overall - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn noop_scorer_consistent_across_calls() {
        let scorer = NoopScorer::new();
        let score1 = scorer.score(&make_request(), &make_response());
        let score2 = scorer.score(&make_request(), &make_response());
        assert!((score1.overall - score2.overall).abs() < f32::EPSILON);
        assert!((score1.relevance - score2.relevance).abs() < f32::EPSILON);
        assert!((score1.coherence - score2.coherence).abs() < f32::EPSILON);
    }

    #[test]
    fn default_trait_impl() {
        let scorer = NoopScorer;
        let score = scorer.score(&make_request(), &make_response());
        assert!((score.overall - 1.0).abs() < f32::EPSILON);
    }
}
