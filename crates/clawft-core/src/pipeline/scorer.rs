//! Quality scorers for the pipeline (Stage 5).
//!
//! Provides two implementations:
//! - [`NoopScorer`] -- Level 0 baseline (returns 1.0 for everything)
//! - [`FitnessScorer`] -- Level 1 GEPA-inspired multi-objective scorer
//!
//! The active scorer is selected by configuration. Both implement
//! [`QualityScorer`].
//!
//! # Error-indicator allowlist (WEFT-54)
//!
//! [`FitnessScorer`] lightly penalizes task completion when the response
//! contains a configured refusal/error phrase. Defaults are **English-only**
//! substring markers (see [`DEFAULT_ERROR_INDICATORS`]). This is a GEPA
//! fitness heuristic — **not** a safety filter, jailbreak detector, or
//! localization layer. Limits and known false positives are documented in
//! `docs/guides/fitness-scorer-error-indicators.md`.

use clawft_types::provider::{ContentBlock, LlmResponse, StopReason};

use super::traits::{ChatRequest, QualityScore, QualityScorer};

// ─── Default error indicators (WEFT-54) ───────────────────────────────

/// Default refusal / soft-failure phrases used by [`FitnessScorerConfig`].
///
/// # Language scope (English-only)
///
/// These markers are **English only**. Multilingual expansion is deliberately
/// deferred: case-insensitive substring matching is already false-positive
/// prone in English (e.g. `"I can't wait"` contains `"I can't"`). Adding other
/// languages without native-speaker review and locale-aware matching would
/// multiply false positives without improving GEPA fitness quality.
///
/// # Matching semantics
///
/// Case-insensitive substring contains. First hit applies a single −0.2
/// task-completion penalty (not stacked). Operators may replace the list via
/// [`FitnessScorerConfig::error_indicators`].
///
/// # Not a security control
///
/// Jailbreak reframes, roleplay wrappers, and non-English refusals can evade
/// these markers. Do **not** use this list for content moderation or policy
/// enforcement.
pub const DEFAULT_ERROR_INDICATORS: &[&str] = &[
    // Soft capability refusals (high signal in English assistant outputs)
    "I can't",
    "I'm unable",
    "I cannot",
    "I don't have access",
    "I'm not able",
    // Identity / policy hedging common in stock LLM refusals
    "as an AI",
    // Slight expansion (still English, higher specificity → lower FP risk)
    "I'm not allowed",
    "I must decline",
    "I won't be able",
];

// ─── NoopScorer (Level 0) ─────────────────────────────────────────────

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

// ─── FitnessScorer (Level 1 -- GEPA-inspired) ────────────────────────

/// Configurable weights for multi-objective fitness scoring.
#[derive(Debug, Clone)]
pub struct FitnessScorerWeights {
    /// Weight for task completion signal (0.0--1.0).
    pub task_completion: f32,
    /// Weight for token efficiency signal (0.0--1.0).
    pub efficiency: f32,
    /// Weight for tool accuracy signal (0.0--1.0).
    pub tool_accuracy: f32,
    /// Weight for coherence / readability signal (0.0--1.0).
    pub coherence: f32,
}

impl Default for FitnessScorerWeights {
    fn default() -> Self {
        Self {
            task_completion: 0.4,
            efficiency: 0.2,
            tool_accuracy: 0.2,
            coherence: 0.2,
        }
    }
}

/// Configuration for the fitness scorer.
#[derive(Debug, Clone)]
pub struct FitnessScorerConfig {
    /// Scoring weights.
    pub weights: FitnessScorerWeights,
    /// Token budget: responses using fewer tokens score higher on efficiency.
    /// Default: 4096.
    pub token_budget: u32,
    /// Minimum response length (in chars) below which task_completion drops.
    /// Default: 10.
    pub min_response_length: usize,
    /// Error indicator phrases that reduce task-completion scores.
    ///
    /// Defaults to [`DEFAULT_ERROR_INDICATORS`] (English-only). See that
    /// constant and `docs/guides/fitness-scorer-error-indicators.md` for
    /// localization / jailbreak limits and the known-good / known-bad catalog.
    pub error_indicators: Vec<String>,
}

impl Default for FitnessScorerConfig {
    fn default() -> Self {
        Self {
            weights: FitnessScorerWeights::default(),
            token_budget: 4096,
            min_response_length: 10,
            // English-only defaults; see DEFAULT_ERROR_INDICATORS (WEFT-54).
            error_indicators: DEFAULT_ERROR_INDICATORS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }
}

/// Level 1 multi-objective fitness scorer (GEPA-inspired).
///
/// Evaluates responses across four dimensions:
/// - **Task completion**: Did the response address the request?
/// - **Efficiency**: Token usage relative to budget
/// - **Tool accuracy**: Correct tool call patterns
/// - **Coherence**: Readability and structure quality
///
/// Produces a weighted sum as the overall score. Each dimension
/// feeds into the relevant [`QualityScore`] field.
pub struct FitnessScorer {
    config: FitnessScorerConfig,
}

impl FitnessScorer {
    /// Create a new fitness scorer with default configuration.
    pub fn new() -> Self {
        Self {
            config: FitnessScorerConfig::default(),
        }
    }

    /// Create a new fitness scorer with custom configuration.
    pub fn with_config(config: FitnessScorerConfig) -> Self {
        Self { config }
    }

    /// Return true if `text` matches any configured error indicator.
    ///
    /// Matching is case-insensitive substring contains (first hit wins for
    /// scoring). Exposed for tests and operator diagnostics (WEFT-54).
    pub fn matches_error_indicator(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.config
            .error_indicators
            .iter()
            .any(|indicator| lower.contains(&indicator.to_lowercase()))
    }

    /// Score task completion (0.0--1.0).
    ///
    /// Heuristics:
    /// - Penalize empty or very short responses
    /// - Penalize error indicator phrases ([`Self::matches_error_indicator`])
    /// - Penalize MaxTokens stop reason (likely truncated)
    /// - Reward responses that contain content relevant to the request
    fn score_task_completion(&self, request: &ChatRequest, response: &LlmResponse) -> f32 {
        let response_text = extract_text(response);
        let mut score: f32 = 1.0;

        // Penalize empty response
        if response_text.is_empty() {
            return 0.0;
        }

        // Penalize very short responses
        if response_text.len() < self.config.min_response_length {
            score -= 0.3;
        }

        // Penalize error indicator phrases (once)
        let lower = response_text.to_lowercase();
        if self.matches_error_indicator(&response_text) {
            score -= 0.2;
        }

        // Penalize truncated responses (MaxTokens stop)
        if response.stop_reason == StopReason::MaxTokens {
            score -= 0.15;
        }

        // Reward keyword overlap between request and response
        let request_words = extract_request_keywords(request);
        if !request_words.is_empty() {
            let overlap = request_words
                .iter()
                .filter(|w| lower.contains(&w.to_lowercase()))
                .count();
            let overlap_ratio = overlap as f32 / request_words.len() as f32;
            // Slight boost for relevance (0..0.15)
            score += overlap_ratio * 0.15;
        }

        score.clamp(0.0, 1.0)
    }

    /// Score token efficiency (0.0--1.0).
    ///
    /// Responses that use fewer tokens relative to the budget score
    /// higher, but extremely short responses (which may indicate
    /// failure) are penalized.
    fn score_efficiency(&self, response: &LlmResponse) -> f32 {
        let output_tokens = response.usage.output_tokens as f32;
        let budget = self.config.token_budget as f32;

        if budget <= 0.0 {
            return 1.0;
        }

        let ratio = output_tokens / budget;

        if output_tokens < 2.0 {
            // Suspiciously short -- likely a failure
            return 0.2;
        }

        // Sweet spot: 5-50% of budget. Over 80% may indicate truncation.
        if ratio <= 0.5 {
            1.0
        } else if ratio <= 0.8 {
            1.0 - (ratio - 0.5) * 0.5 // Linear decay 1.0 -> 0.85
        } else {
            0.7 - (ratio - 0.8) * 1.5 // Steeper decay after 80%
        }
        .clamp(0.0, 1.0)
    }

    /// Score tool accuracy (0.0--1.0).
    ///
    /// Checks whether tool calls were made when tools were available
    /// and whether the response contains tool results.
    fn score_tool_accuracy(&self, request: &ChatRequest, response: &LlmResponse) -> f32 {
        let has_tools = !request.tools.is_empty();
        let has_tool_use = response
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));

        if !has_tools {
            // No tools defined -- full marks (not applicable)
            return 1.0;
        }

        if has_tool_use {
            // Tools were available and used -- good
            1.0
        } else {
            // Tools available but not used -- mild penalty
            // (the model may have correctly decided not to use tools)
            0.7
        }
    }

    /// Score coherence (0.0--1.0).
    ///
    /// Heuristic analysis of response structure and readability.
    fn score_coherence(&self, response: &LlmResponse) -> f32 {
        let text = extract_text(response);
        if text.is_empty() {
            return 0.0;
        }

        let mut score: f32 = 1.0;

        // Penalize extremely long single-line responses (wall of text)
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() == 1 && text.len() > 500 {
            score -= 0.15;
        }

        // Reward structured responses (markdown headers, bullet points, code blocks)
        let has_structure = text.contains('#')
            || text.contains("- ")
            || text.contains("* ")
            || text.contains("```")
            || text.contains("1.");
        if has_structure && text.len() > 100 {
            score += 0.1;
        }

        // Penalize excessive repetition (naive check: repeated sentences)
        let sentences: Vec<&str> = text.split(". ").collect();
        if sentences.len() > 3 {
            let unique_count = {
                let mut seen = std::collections::HashSet::new();
                sentences
                    .iter()
                    .filter(|s| seen.insert(s.to_lowercase()))
                    .count()
            };
            let uniqueness = unique_count as f32 / sentences.len() as f32;
            if uniqueness < 0.5 {
                score -= 0.3; // Heavy repetition
            }
        }

        score.clamp(0.0, 1.0)
    }
}

impl Default for FitnessScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl QualityScorer for FitnessScorer {
    fn score(&self, request: &ChatRequest, response: &LlmResponse) -> QualityScore {
        let task_completion = self.score_task_completion(request, response);
        let efficiency = self.score_efficiency(response);
        let tool_accuracy = self.score_tool_accuracy(request, response);
        let coherence = self.score_coherence(response);

        let w = &self.config.weights;
        let overall = (task_completion * w.task_completion
            + efficiency * w.efficiency
            + tool_accuracy * w.tool_accuracy
            + coherence * w.coherence)
            .clamp(0.0, 1.0);

        QualityScore {
            overall,
            relevance: task_completion,
            coherence,
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// Extract all text content from an LLM response.
fn extract_text(response: &LlmResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| {
            if let ContentBlock::Text { text } = block {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract meaningful keywords from the request messages for
/// relevance checking.
fn extract_request_keywords(request: &ChatRequest) -> Vec<String> {
    let stop_words: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "can", "shall",
        "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into", "through",
        "during", "before", "after", "above", "below", "and", "but", "or", "nor", "not", "so",
        "yet", "both", "either", "neither", "each", "every", "all", "any", "few", "more", "most",
        "other", "some", "such", "no", "only", "own", "same", "than", "too", "very", "just",
        "because", "if", "when", "i", "me", "my", "you", "your", "it", "its", "we", "us", "they",
        "them", "this", "that", "these", "those", "what", "which", "who", "how", "please", "help",
    ];

    request
        .messages
        .iter()
        .filter(|m| m.role == "user")
        .flat_map(|m| {
            m.content
                .split_whitespace()
                .map(|w| {
                    w.trim_matches(|c: char| !c.is_alphanumeric())
                        .to_lowercase()
                })
                .filter(|w| w.len() > 2 && !stop_words.contains(&w.as_str()))
                .collect::<Vec<_>>()
        })
        .collect()
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
            auth_context: None,
            complexity_boost: 0.0,
            tool_choice: None,
        }
    }

    fn make_request_with_tools() -> ChatRequest {
        ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "Search for rust documentation".into(),
                tool_call_id: None,
                tool_calls: None,
            }],
            tools: vec![serde_json::json!({"type": "function", "name": "web_search"})],
            model: None,
            max_tokens: None,
            temperature: None,
            auth_context: None,
            complexity_boost: 0.0,
            tool_choice: None,
        }
    }

    fn make_response() -> LlmResponse {
        LlmResponse {
            id: "resp-1".into(),
            content: vec![ContentBlock::Text {
                text: "Hi there! I'd be happy to help you with that.".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 15,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        }
    }

    fn make_empty_response() -> LlmResponse {
        LlmResponse {
            id: "resp-empty".into(),
            content: vec![],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 0,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        }
    }

    fn make_truncated_response() -> LlmResponse {
        LlmResponse {
            id: "resp-trunc".into(),
            content: vec![ContentBlock::Text {
                text: "This response was cut off because it reached the maximum".into(),
            }],
            stop_reason: StopReason::MaxTokens,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 4000,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        }
    }

    fn make_error_response() -> LlmResponse {
        LlmResponse {
            id: "resp-err".into(),
            content: vec![ContentBlock::Text {
                text: "I'm unable to help with that request.".into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 10,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        }
    }

    // ── NoopScorer tests ──────────────────────────────────────────────

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
            auth_context: None,
            complexity_boost: 0.0,
            tool_choice: None,
        };
        let score = scorer.score(&req, &make_response());
        assert!((score.overall - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn noop_scorer_ignores_response_content() {
        let scorer = NoopScorer::new();
        let score = scorer.score(&make_request(), &make_empty_response());
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
    fn noop_default_trait_impl() {
        let scorer = NoopScorer;
        let score = scorer.score(&make_request(), &make_response());
        assert!((score.overall - 1.0).abs() < f32::EPSILON);
    }

    // ── FitnessScorer tests ───────────────────────────────────────────

    #[test]
    fn fitness_scorer_scores_good_response() {
        let scorer = FitnessScorer::new();
        let score = scorer.score(&make_request(), &make_response());

        assert!(
            score.overall > 0.5,
            "good response should score > 0.5, got {}",
            score.overall
        );
        assert!(score.relevance > 0.5);
        assert!(score.coherence > 0.5);
    }

    #[test]
    fn fitness_scorer_penalizes_empty_response() {
        let scorer = FitnessScorer::new();
        let score = scorer.score(&make_request(), &make_empty_response());

        // Empty response: task_completion=0, coherence=0
        assert!(
            score.overall < 0.5,
            "empty response should score < 0.5, got {}",
            score.overall
        );
    }

    #[test]
    fn fitness_scorer_penalizes_error_response() {
        let scorer = FitnessScorer::new();
        let score = scorer.score(&make_request(), &make_error_response());

        let good_score = scorer.score(&make_request(), &make_response());
        assert!(
            score.overall < good_score.overall,
            "error response ({}) should score lower than good response ({})",
            score.overall,
            good_score.overall
        );
    }

    #[test]
    fn fitness_scorer_penalizes_truncated_response() {
        let scorer = FitnessScorer::new();
        let score = scorer.score(&make_request(), &make_truncated_response());

        let good_score = scorer.score(&make_request(), &make_response());
        assert!(
            score.overall < good_score.overall,
            "truncated response ({}) should score lower than good response ({})",
            score.overall,
            good_score.overall
        );
    }

    #[test]
    fn fitness_scorer_tool_accuracy_with_no_tools() {
        let scorer = FitnessScorer::new();
        // No tools in request -> tool accuracy should be 1.0
        let tool_score = scorer.score_tool_accuracy(&make_request(), &make_response());
        assert!((tool_score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fitness_scorer_tool_accuracy_without_tool_use() {
        let scorer = FitnessScorer::new();
        // Tools defined but not used -> mild penalty
        let tool_score = scorer.score_tool_accuracy(&make_request_with_tools(), &make_response());
        assert!(tool_score < 1.0);
        assert!(tool_score > 0.5);
    }

    #[test]
    fn fitness_scorer_efficiency_moderate_usage() {
        let scorer = FitnessScorer::new();
        let efficiency = scorer.score_efficiency(&make_response());
        // 15 tokens out of 4096 budget -- very efficient
        assert!(
            efficiency > 0.9,
            "low token usage should be efficient, got {}",
            efficiency
        );
    }

    #[test]
    fn fitness_scorer_efficiency_suspiciously_short() {
        let scorer = FitnessScorer::new();
        let resp = LlmResponse {
            id: "resp-tiny".into(),
            content: vec![ContentBlock::Text { text: "Ok".into() }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 1,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        };
        let efficiency = scorer.score_efficiency(&resp);
        assert!(
            efficiency < 0.5,
            "suspiciously short should score low, got {}",
            efficiency
        );
    }

    #[test]
    fn fitness_scorer_custom_weights() {
        let config = FitnessScorerConfig {
            weights: FitnessScorerWeights {
                task_completion: 1.0,
                efficiency: 0.0,
                tool_accuracy: 0.0,
                coherence: 0.0,
            },
            ..Default::default()
        };
        let scorer = FitnessScorer::with_config(config);
        let score = scorer.score(&make_request(), &make_response());

        // With only task_completion weighted, overall should equal relevance
        assert!(
            (score.overall - score.relevance).abs() < 0.01,
            "overall ({}) should match relevance ({}) with task_completion-only weights",
            score.overall,
            score.relevance
        );
    }

    #[test]
    fn fitness_scorer_scores_between_zero_and_one() {
        let scorer = FitnessScorer::new();

        // Test with various response types
        let responses = vec![
            make_response(),
            make_empty_response(),
            make_truncated_response(),
            make_error_response(),
        ];

        for resp in &responses {
            let score = scorer.score(&make_request(), resp);
            assert!(
                score.overall >= 0.0 && score.overall <= 1.0,
                "overall must be in [0,1], got {}",
                score.overall
            );
            assert!(
                score.relevance >= 0.0 && score.relevance <= 1.0,
                "relevance must be in [0,1], got {}",
                score.relevance
            );
            assert!(
                score.coherence >= 0.0 && score.coherence <= 1.0,
                "coherence must be in [0,1], got {}",
                score.coherence
            );
        }
    }

    #[test]
    fn fitness_scorer_coherence_penalizes_wall_of_text() {
        let scorer = FitnessScorer::new();
        let wall = "a ".repeat(300);
        let resp = LlmResponse {
            id: "resp-wall".into(),
            content: vec![ContentBlock::Text { text: wall }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 300,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        };
        let coherence = scorer.score_coherence(&resp);
        assert!(
            coherence < 1.0,
            "wall of text should reduce coherence, got {}",
            coherence
        );
    }

    #[test]
    fn fitness_scorer_coherence_rewards_structure() {
        let scorer = FitnessScorer::new();
        let structured = "# Title\n\n- Point 1\n- Point 2\n\nSome explanation that is long enough to qualify for the structure bonus and demonstrate good formatting.";
        let resp = LlmResponse {
            id: "resp-struct".into(),
            content: vec![ContentBlock::Text {
                text: structured.into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 30,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        };
        let coherence = scorer.score_coherence(&resp);
        assert!(
            coherence >= 1.0,
            "structured response should score high, got {}",
            coherence
        );
    }

    // ── WEFT-54: error_indicators catalog (known-good / known-bad) ────

    /// Known-bad: English refusal / soft-failure phrases that MUST match.
    const KNOWN_BAD_ERROR_RESPONSES: &[&str] = &[
        "I can't help with that request.",
        "I'm unable to complete this task right now.",
        "I cannot provide that information.",
        "I don't have access to the internal network.",
        "I'm not able to run shell commands for you.",
        "As an AI, I must follow safety guidelines.",
        "I'm not allowed to share that content.",
        "I must decline this request.",
        "I won't be able to process that file.",
        // Case variation
        "i CAN'T assist with illegal activity.",
    ];

    /// Known-good: legitimate responses that should NOT match indicators.
    ///
    /// Note: idioms that embed a marker as a substring (e.g. "I can't wait")
    /// are documented false positives — see
    /// `docs/guides/fitness-scorer-error-indicators.md` — and are listed
    /// separately under KNOWN_FALSE_POSITIVES.
    const KNOWN_GOOD_RESPONSES: &[&str] = &[
        "Here is a step-by-step plan to deploy the service.",
        "I can help you write the migration script.",
        "I'll wait for your confirmation before applying changes.",
        "Training an AI model requires careful data hygiene.",
        "The API returns 403 when credentials are missing.",
        "Use cargo test to verify the change locally.",
        "Certainly — the function returns Result<(), Error>.",
        "Voici la réponse en français sans marqueur anglais.",
        "これは日本語の応答で英語の拒否フレーズを含みません。",
    ];

    /// Documented English false positives (substring collisions).
    /// These currently match; they are catalogued honestly, not hidden.
    const KNOWN_FALSE_POSITIVES: &[&str] = &[
        "I can't wait to see the benchmark results!",
        "As an AI engineer, prefer typed APIs over stringly config.",
    ];

    #[test]
    fn default_error_indicators_non_empty_english_only() {
        assert!(!DEFAULT_ERROR_INDICATORS.is_empty());
        // Sanity: every default is ASCII-ish English (no CJK/Arabic markers).
        for phrase in DEFAULT_ERROR_INDICATORS {
            assert!(
                phrase.is_ascii(),
                "default indicators are English-only (ASCII); got {phrase:?}"
            );
            assert!(!phrase.is_empty());
        }
        let scorer = FitnessScorer::new();
        assert_eq!(
            scorer.config.error_indicators.len(),
            DEFAULT_ERROR_INDICATORS.len()
        );
    }

    #[test]
    fn known_bad_responses_match_error_indicators() {
        let scorer = FitnessScorer::new();
        for text in KNOWN_BAD_ERROR_RESPONSES {
            assert!(
                scorer.matches_error_indicator(text),
                "known-bad should match an indicator: {text:?}"
            );
        }
    }

    #[test]
    fn known_good_responses_do_not_match_error_indicators() {
        let scorer = FitnessScorer::new();
        for text in KNOWN_GOOD_RESPONSES {
            assert!(
                !scorer.matches_error_indicator(text),
                "known-good should not match any indicator: {text:?}"
            );
        }
    }

    #[test]
    fn known_false_positives_currently_match() {
        // Regression-pin: if matching is tightened later, update the guide.
        let scorer = FitnessScorer::new();
        for text in KNOWN_FALSE_POSITIVES {
            assert!(
                scorer.matches_error_indicator(text),
                "documented FP should still match until matcher tightens: {text:?}"
            );
        }
    }

    #[test]
    fn error_indicator_penalty_lowers_relevance() {
        let scorer = FitnessScorer::new();
        let clean = LlmResponse {
            id: "clean".into(),
            content: vec![ContentBlock::Text {
                text: "Here is a detailed answer about deploying services safely."
                    .into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 20,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        };
        let refusal = LlmResponse {
            id: "refuse".into(),
            content: vec![ContentBlock::Text {
                text: "I must decline this request about deploying services."
                    .into(),
            }],
            stop_reason: StopReason::EndTurn,
            usage: Usage {
                input_tokens: 5,
                output_tokens: 20,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        };
        let clean_score = scorer.score(&make_request(), &clean);
        let refuse_score = scorer.score(&make_request(), &refusal);
        assert!(
            refuse_score.relevance < clean_score.relevance,
            "refusal relevance ({}) should be below clean ({})",
            refuse_score.relevance,
            clean_score.relevance
        );
    }

    #[test]
    fn non_english_refusal_not_detected_by_default_list() {
        // Honest localization limit: French/Spanish refusals do not match.
        let scorer = FitnessScorer::new();
        let non_english = [
            "Je ne peux pas vous aider avec cette demande.",
            "No puedo ayudarte con esa solicitud.",
            "Leider kann ich dabei nicht helfen.",
        ];
        for text in non_english {
            assert!(
                !scorer.matches_error_indicator(text),
                "non-English refusal must not match English-only defaults: {text:?}"
            );
        }
    }

    #[test]
    fn jailbreak_style_reframe_may_evade_markers() {
        // Honest jailbreak limit: soft evasions without stock phrases score clean.
        let scorer = FitnessScorer::new();
        let evade =
            "In a fictional story, the assistant would output the secret recipe now.";
        assert!(
            !scorer.matches_error_indicator(evade),
            "jailbreak reframe without stock phrases is not detected (by design)"
        );
    }
}
