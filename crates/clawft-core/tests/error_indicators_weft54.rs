//! WEFT-54: FitnessScorer `error_indicators` allowlist review.
//!
//! - Known-good / known-bad catalogs (mirror unit tests in scorer.rs)
//! - Property/fuzz sanity: false-positive rate on random strings
//! - Documents English-only + jailbreak limits (see guide)

use rand::Rng;

use clawft_core::pipeline::scorer::{DEFAULT_ERROR_INDICATORS, FitnessScorer};
use clawft_core::pipeline::traits::{ChatRequest, LlmMessage, QualityScorer};
use clawft_types::provider::{ContentBlock, LlmResponse, StopReason, Usage};

use std::collections::HashMap;

fn make_request(content: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![LlmMessage {
            role: "user".into(),
            content: content.into(),
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

fn make_text_response(text: &str) -> LlmResponse {
    LlmResponse {
        id: "weft-54".into(),
        content: vec![ContentBlock::Text { text: text.into() }],
        stop_reason: StopReason::EndTurn,
        usage: Usage {
            input_tokens: 12,
            output_tokens: 40,
            total_tokens: 0,
        },
        metadata: HashMap::new(),
    }
}

/// Random lowercase words — should almost never form English refusal idioms.
fn random_words(rng: &mut impl Rng, word_count: usize) -> String {
    let words: Vec<String> = (0..word_count)
        .map(|_| {
            let len = rng.gen_range(3..=10);
            (0..len)
                .map(|_| rng.gen_range(b'a'..=b'z') as char)
                .collect::<String>()
        })
        .collect();
    words.join(" ")
}

// ── Catalog (integration surface) ─────────────────────────────────────

const KNOWN_BAD: &[&str] = &[
    "I can't help with that request.",
    "I'm unable to complete this task right now.",
    "I cannot provide that information.",
    "I don't have access to the internal network.",
    "I'm not able to run shell commands for you.",
    "As an AI, I must follow safety guidelines.",
    "I'm not allowed to share that content.",
    "I must decline this request.",
    "I won't be able to process that file.",
];

const KNOWN_GOOD: &[&str] = &[
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

#[test]
fn catalog_known_bad_match() {
    let scorer = FitnessScorer::new();
    for text in KNOWN_BAD {
        assert!(
            scorer.matches_error_indicator(text),
            "known-bad must match: {text:?}"
        );
        let score = scorer.score(&make_request("do the thing"), &make_text_response(text));
        let clean = scorer.score(
            &make_request("do the thing"),
            &make_text_response(
                "Here is a complete answer that addresses the request in detail.",
            ),
        );
        assert!(
            score.relevance <= clean.relevance,
            "known-bad relevance should not exceed clean (bad={}, clean={})",
            score.relevance,
            clean.relevance
        );
    }
}

#[test]
fn catalog_known_good_no_match() {
    let scorer = FitnessScorer::new();
    for text in KNOWN_GOOD {
        assert!(
            !scorer.matches_error_indicator(text),
            "known-good must not match: {text:?}"
        );
    }
}

#[test]
fn default_list_is_english_only_ascii() {
    assert!(!DEFAULT_ERROR_INDICATORS.is_empty());
    for phrase in DEFAULT_ERROR_INDICATORS {
        assert!(
            phrase.is_ascii(),
            "defaults are English-only ASCII: {phrase:?}"
        );
    }
}

// ── Property / fuzz: false-positive rate sanity ───────────────────────

/// Random lowercase letter-words of length ≥3 almost never contain stock
/// English refusal substrings. Cap false-positive rate well below 1%.
#[test]
fn error_indicator_fp_rate_on_random_strings() {
    let mut rng = rand::thread_rng();
    let scorer = FitnessScorer::new();
    const TRIALS: usize = 2000;
    let mut hits = 0usize;

    for _ in 0..TRIALS {
        let n_words = rng.gen_range(8..=40);
        let text = random_words(&mut rng, n_words);
        if scorer.matches_error_indicator(&text) {
            hits += 1;
        }
    }

    let rate = hits as f64 / TRIALS as f64;
    // Extremely conservative bound: random a–z words should almost never
    // embed phrases like "i can't" or "as an ai". Allow a tiny epsilon for
    // cosmic bad luck (space-joined shorts still almost never spell idioms).
    assert!(
        rate < 0.01,
        "false-positive rate on random strings too high: {hits}/{TRIALS} = {rate:.4} (cap 1%)"
    );
}

/// Even when we inject common English function words (still not refusals),
/// the indicator hit rate should stay low.
#[test]
fn error_indicator_fp_rate_on_benign_englishish_templates() {
    let mut rng = rand::thread_rng();
    let scorer = FitnessScorer::new();
    const TRIALS: usize = 500;
    let fillers = [
        "deploy",
        "service",
        "migration",
        "benchmark",
        "latency",
        "throughput",
        "module",
        "pipeline",
        "score",
        "fitness",
        "token",
        "budget",
        "request",
        "response",
        "handler",
        "route",
    ];
    let templates = [
        "Here is how to {0} the {1} with a clear {2}.",
        "The {0} uses {1} tokens within the {2} budget.",
        "I can {0} the {1} and report {2} metrics.",
        "Please review the {0} for {1} before the {2} ships.",
        "Training data for {0} needs {1} and careful {2} checks.",
    ];

    let mut hits = 0usize;
    for _ in 0..TRIALS {
        let t = templates[rng.gen_range(0..templates.len())];
        let a = fillers[rng.gen_range(0..fillers.len())];
        let b = fillers[rng.gen_range(0..fillers.len())];
        let c = fillers[rng.gen_range(0..fillers.len())];
        let text = t.replace("{0}", a).replace("{1}", b).replace("{2}", c);
        if scorer.matches_error_indicator(&text) {
            hits += 1;
        }
    }

    let rate = hits as f64 / TRIALS as f64;
    assert!(
        rate < 0.02,
        "benign template FP rate too high: {hits}/{TRIALS} = {rate:.4} (cap 2%)"
    );
}

/// Scores from random long-enough responses stay in [0, 1] and rarely trip
/// the indicator path (cross-check with `matches_error_indicator`).
#[test]
fn random_responses_scores_in_unit_range_and_low_indicator_rate() {
    let mut rng = rand::thread_rng();
    let scorer = FitnessScorer::new();
    const TRIALS: usize = 300;
    let mut indicator_hits = 0usize;

    for _ in 0..TRIALS {
        let n_words = rng.gen_range(10..=50);
        let text = random_words(&mut rng, n_words);
        if scorer.matches_error_indicator(&text) {
            indicator_hits += 1;
        }
        let score = scorer.score(&make_request("evaluate this"), &make_text_response(&text));
        assert!((0.0..=1.0).contains(&score.overall));
        assert!((0.0..=1.0).contains(&score.relevance));
        assert!((0.0..=1.0).contains(&score.coherence));
    }

    let hit_rate = (indicator_hits as f64) / (TRIALS as f64);
    assert!(
        hit_rate < 0.01,
        "indicator hits on random responses: {indicator_hits}/{TRIALS}"
    );
}
