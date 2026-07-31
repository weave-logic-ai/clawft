//! Integration tests for the WEFT-229 voice bench harness.
//!
//! No ONNX models, no mic, no network — pure mock/fixture path so
//! `scripts/build.sh test` stays green in CI.

use std::time::Duration;

use clawft_bench_voice::cpu::{CpuBudget, CpuSample};
use clawft_bench_voice::latency::{LatencyHarness, LatencySample, LatencyStats, MockPipeline};
use clawft_bench_voice::report::VoiceBenchReport;
use clawft_bench_voice::thresholds::VoiceBenchThresholds;
use clawft_bench_voice::wer::{fixture_english_corpus, word_error_rate, word_error_rate_corpus};
use clawft_bench_voice::{TICKET, VERSION};

#[test]
fn end_to_end_mock_pipeline_passes_default_thresholds() {
    let thresholds = VoiceBenchThresholds::default();

    // --- Latency: 5 quick mock turns (instant first-byte) ---
    let mut harness = LatencyHarness::new(MockPipeline::instant().named("mock-ci"));
    harness.run_n(5).expect("mock turns");
    let stats = harness.stats().expect("stats");
    assert_eq!(stats.count, 5);
    assert!(
        thresholds.latency.check_stats(&stats).passed,
        "instant mock p95 must be under {}ms, got {}ms",
        thresholds.latency.max_p95_ms,
        stats.p95.as_millis()
    );

    // --- WER: perfect fixture corpus ---
    let pairs = fixture_english_corpus();
    let wer = word_error_rate_corpus(&pairs);
    assert!(wer.is_perfect());
    assert!(thresholds.wer.check(&wer).passed);

    // --- CPU: synthetic samples under wake/pipeline budgets ---
    let wake_samples = [
        CpuSample::from_percent(0.4).with_label("wake"),
        CpuSample::from_percent(1.1).with_label("wake"),
    ];
    let pipeline_samples = [
        CpuSample::from_percent(3.0).with_label("pipeline"),
        CpuSample::from_percent(7.5).with_label("pipeline"),
    ];
    assert!(thresholds.cpu.wake_budget().check_all(&wake_samples).passed);
    assert!(
        thresholds
            .cpu
            .pipeline_budget()
            .check_all(&pipeline_samples)
            .passed
    );

    let evaluation =
        thresholds.evaluate(Some(&stats), Some(&wer), &wake_samples, &pipeline_samples);
    assert!(evaluation.passed);

    let report = VoiceBenchReport::from_evaluation(
        thresholds,
        evaluation,
        Some(stats),
        Some(wer),
    );
    assert!(report.passed);
    assert_eq!(report.ticket, TICKET);
    assert_eq!(report.version, VERSION);
    assert!(report.metrics.iter().all(|m| m.passed));

    // JSON artifact shape is stable for CI collectors.
    let json = report.to_json_pretty().expect("json");
    assert!(json.contains("WEFT-229"));
    assert!(json.contains("latency_p95"));
    assert!(json.contains("cpu_wake"));
}

#[test]
fn injected_latency_samples_drive_stats_without_pipeline() {
    let mut harness = LatencyHarness::new(MockPipeline::instant());
    // Bypass run_turn — feed pre-measured fixture timings.
    for ms in [40u64, 55, 60, 70, 120] {
        harness.push_sample(LatencySample::from_duration("fixture-audio", Duration::from_millis(ms)));
    }
    let stats = harness.stats().unwrap();
    assert_eq!(stats.count, 5);
    assert_eq!(stats.min, Duration::from_millis(40));
    assert_eq!(stats.max, Duration::from_millis(120));
    let t = VoiceBenchThresholds::default();
    assert!(t.latency.check_stats(&stats).passed);
}

#[test]
fn wer_detects_corrupt_hypothesis_against_fixture() {
    let pairs = fixture_english_corpus();
    // Corrupt first utterance: replace all words.
    let mut bad: Vec<(String, String)> = pairs
        .iter()
        .map(|(r, h)| ((*r).to_string(), (*h).to_string()))
        .collect();
    bad[0].1 = "zzzz yyyy xxxx wwww vvvv uuuu tttt".into();
    let refs: Vec<(&str, &str)> = bad
        .iter()
        .map(|(r, h)| (r.as_str(), h.as_str()))
        .collect();
    let wer = word_error_rate_corpus(&refs);
    assert!(wer.wer > 0.0);
    // Still likely under 15% if only one of five utterances is fully wrong —
    // compute expected: first utt has 9 words all wrong → 9 edits; total N is sum.
    // Force a clearly failing case:
    let fail = word_error_rate(
        "one two three four five six seven eight",
        "a b c d e f g h",
    );
    let t = VoiceBenchThresholds::default();
    assert!(!t.wer.check(&fail).passed);
    assert!(fail.wer > t.wer.max_wer);
}

#[test]
fn wake_budget_enforcement_fails_loud() {
    let budget = CpuBudget::wake_default();
    let over = CpuSample::from_percent(2.5);
    let verdict = budget.check(&over);
    assert!(!verdict.passed);
    assert_eq!(verdict.budget_name, "wake");

    let summary = budget.check_all(&[
        CpuSample::from_percent(0.5),
        CpuSample::from_percent(2.5),
        CpuSample::from_percent(1.0),
    ]);
    assert!(!summary.passed);
    assert_eq!(summary.failures, 1);
}

#[test]
fn thresholds_json_is_ci_loadable() {
    let json = r#"{
        "latency": { "max_p95_ms": 300, "max_sample_ms": 1000 },
        "wer": { "max_wer": 0.10 },
        "cpu": { "wake_max_percent": 2.0, "pipeline_max_percent": 10.0 }
    }"#;
    let t = VoiceBenchThresholds::from_json(json).expect("parse");
    assert_eq!(t.latency.max_p95_ms, 300);
    assert!((t.wer.max_wer - 0.10).abs() < 1e-9);

    // Stats with p95 around 400ms should fail the 300ms gate.
    let samples: Vec<_> = (0..20)
        .map(|i| LatencySample::from_duration("x", Duration::from_millis(350 + i * 5)))
        .collect();
    let stats = LatencyStats::from_samples(&samples).unwrap();
    assert!(!t.latency.check_stats(&stats).passed);
}

/// Heavy multi-iteration bench — ignored in normal CI; run with
/// `cargo test -p clawft-bench-voice -- --ignored`.
#[test]
#[ignore = "optional multi-iteration latency soak; not required for CI gate"]
fn soak_latency_one_hundred_mock_turns() {
    let mut harness = LatencyHarness::new(MockPipeline::with_delay(Duration::from_millis(1)));
    harness.run_n(100).expect("100 turns");
    let stats = harness.stats().unwrap();
    assert_eq!(stats.count, 100);
    // With 1ms sleep, p95 should still be well under the 500ms default.
    assert!(stats.p95 < Duration::from_millis(100));
}
