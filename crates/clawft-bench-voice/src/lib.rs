//! `clawft-bench-voice` — Latency, WER, and CPU harness for the WeftOS voice pipeline.
//!
//! # WEFT-229
//!
//! Closes the gap called out in the 0.7.0 voice audit / VS3.3:
//! - **Latency**: speech-end → first-response-byte (mock pipeline ok)
//! - **WER**: word error rate against reference transcripts
//! - **CPU**: wake-word budget checker (hard concept: &lt; 2% of one core)
//!
//! Default builds and unit/integration tests need **no ONNX models**, no mic,
//! and no live STT/TTS. Real models can plug in later via the
//! [`latency::VoicePipeline`] trait (or by feeding measured timestamps /
//! transcripts into the helpers).
//!
//! # Quick start
//!
//! ```rust
//! use clawft_bench_voice::latency::{LatencyHarness, MockPipeline};
//! use clawft_bench_voice::thresholds::VoiceBenchThresholds;
//! use clawft_bench_voice::wer::word_error_rate;
//! use clawft_bench_voice::cpu::{CpuBudget, CpuSample};
//! use std::time::Duration;
//!
//! let thresholds = VoiceBenchThresholds::default();
//!
//! // Latency (mock pipeline returns first byte after a fixed delay)
//! let mut harness = LatencyHarness::new(MockPipeline::with_delay(Duration::from_millis(5)));
//! let sample = harness.run_once().expect("mock never fails");
//! assert!(thresholds.latency.check_sample(&sample).passed);
//!
//! // WER
//! let wer = word_error_rate("hello world", "hello world");
//! assert_eq!(wer.wer, 0.0);
//!
//! // CPU wake budget
//! let budget = CpuBudget::wake_default(); // 2% of one core
//! let sample = CpuSample::from_percent(1.0);
//! assert!(budget.check(&sample).passed);
//! ```
//!
//! # Thresholds
//!
//! Defaults (from `.planning/voice_development.md` VS2.1.8 / VS3.3):
//! - speech-end → first-response-byte p95 ≤ 500 ms
//! - WER ≤ 0.15 (15%) on the fixture English corpus
//! - wake-word CPU &lt; 2% of one core; full pipeline &lt; 10%
//!
//! Override via [`thresholds::VoiceBenchThresholds`] or JSON
//! ([`thresholds::VoiceBenchThresholds::from_json`]).

#![warn(missing_docs)]

pub mod cpu;
pub mod latency;
pub mod report;
pub mod thresholds;
pub mod wer;

pub use cpu::{CpuBudget, CpuBudgetVerdict, CpuSample};
pub use latency::{
    LatencyHarness, LatencySample, LatencyStats, MockPipeline, VoicePipeline,
};
pub use report::{MetricStatus, VoiceBenchReport};
pub use thresholds::{
    CpuThresholds, LatencyThresholds, VoiceBenchThresholds, WerThresholds,
};
pub use wer::{WerResult, word_error_rate, word_error_rate_corpus};

/// Crate version string for diagnostics / report metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ticket this crate implements.
pub const TICKET: &str = "WEFT-229";
