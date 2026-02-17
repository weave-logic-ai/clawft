//! No-op learner (Level 0 implementation).
//!
//! Silently discards all trajectory records and learning signals.
//! Real learning backends will be implemented at Level 1+ (e.g.
//! EMA statistics) or Level 2+ (neural trajectory learning).

use super::traits::{LearningBackend, LearningSignal, Trajectory};

/// Level 0 no-op learning backend.
///
/// Both [`record`](LearningBackend::record) and
/// [`adapt`](LearningBackend::adapt) are no-ops. This serves as
/// the baseline backend until adaptive learning is implemented.
pub struct NoopLearner;

impl NoopLearner {
    /// Create a new no-op learner.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningBackend for NoopLearner {
    fn record(&self, _trajectory: &Trajectory) {
        // No-op: Level 0 does not learn from interactions.
    }

    fn adapt(&self, _signal: &LearningSignal) {
        // No-op: Level 0 does not adapt from signals.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::traits::{
        ChatRequest, LlmMessage, QualityScore, RoutingDecision,
    };
    use clawft_types::provider::{ContentBlock, LlmResponse, StopReason, Usage};
    use std::collections::HashMap;

    fn make_trajectory() -> Trajectory {
        Trajectory {
            request: ChatRequest {
                messages: vec![LlmMessage {
                    role: "user".into(),
                    content: "hello".into(),
                    tool_call_id: None,
                }],
                tools: vec![],
                model: None,
                max_tokens: None,
                temperature: None,
            },
            routing: RoutingDecision {
                provider: "openai".into(),
                model: "gpt-4o".into(),
                reason: "test".into(),
            },
            response: LlmResponse {
                id: "resp-1".into(),
                content: vec![ContentBlock::Text {
                    text: "Hi!".into(),
                }],
                stop_reason: StopReason::EndTurn,
                usage: Usage {
                    input_tokens: 5,
                    output_tokens: 2,
                },
                metadata: HashMap::new(),
            },
            quality: QualityScore {
                overall: 0.9,
                relevance: 0.95,
                coherence: 0.85,
            },
        }
    }

    #[test]
    fn record_does_not_panic() {
        let learner = NoopLearner::new();
        learner.record(&make_trajectory());
    }

    #[test]
    fn adapt_does_not_panic() {
        let learner = NoopLearner::new();
        learner.adapt(&LearningSignal {
            feedback_type: "thumbs_up".into(),
            value: 1.0,
        });
    }

    #[test]
    fn adapt_negative_signal_does_not_panic() {
        let learner = NoopLearner::new();
        learner.adapt(&LearningSignal {
            feedback_type: "thumbs_down".into(),
            value: -1.0,
        });
    }

    #[test]
    fn multiple_records_do_not_panic() {
        let learner = NoopLearner::new();
        for _ in 0..100 {
            learner.record(&make_trajectory());
        }
    }

    #[test]
    fn multiple_adapts_do_not_panic() {
        let learner = NoopLearner::new();
        for i in 0..50 {
            learner.adapt(&LearningSignal {
                feedback_type: format!("signal_{i}"),
                value: i as f32 / 50.0,
            });
        }
    }

    #[test]
    fn default_trait_impl() {
        let learner = NoopLearner;
        learner.record(&make_trajectory());
        learner.adapt(&LearningSignal {
            feedback_type: "test".into(),
            value: 0.0,
        });
    }
}
