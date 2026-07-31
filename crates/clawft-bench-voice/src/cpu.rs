//! CPU budget checker for wake-word and full voice pipeline.
//!
//! From `.planning/voice_development.md`:
//! - wake word uses **&lt; 2%** of one core (VS2.1.8 / VS3.3.7)
//! - full pipeline **&lt; 10%** of one core (VS3.3.7)
//!
//! This module does **not** attach a profiler. Callers (or a future OS sampler)
//! supply [`CpuSample`]s; the budget checker applies thresholds and records a
//! pass/fail verdict. Unit tests inject synthetic samples so CI never needs
//! root, `perf`, or platform-specific CPU counters.

use serde::{Deserialize, Serialize};

/// A single CPU utilization observation, expressed as **percent of one core**.
///
/// `1.5` means 1.5% of one logical CPU. Values may exceed 100 when a workload
/// spans multiple cores (full pipeline multi-thread).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuSample {
    /// Percent of one core (0.0 … ∞).
    pub percent_of_one_core: f64,
    /// Optional wall duration the sample covers, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<u64>,
    /// Optional label (`"wake"`, `"pipeline"`, host name, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl CpuSample {
    /// Construct from a percent-of-one-core reading.
    pub fn from_percent(percent_of_one_core: f64) -> Self {
        Self {
            percent_of_one_core,
            window_ms: None,
            label: None,
        }
    }

    /// Attach a sampling window length.
    pub fn with_window_ms(mut self, ms: u64) -> Self {
        self.window_ms = Some(ms);
        self
    }

    /// Attach a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Estimate CPU percent from process CPU time vs wall time.
    ///
    /// `cpu_time` / `wall_time` × 100 → percent of one core. Returns `None` if
    /// `wall_time` is zero. Useful when a harness has already measured both
    /// durations (e.g. via `std::time` around a busy section, or platform
    /// `getrusage` wrappers outside this crate).
    pub fn from_times(cpu_time: std::time::Duration, wall_time: std::time::Duration) -> Option<Self> {
        let wall_ns = wall_time.as_nanos();
        if wall_ns == 0 {
            return None;
        }
        let pct = (cpu_time.as_nanos() as f64 / wall_ns as f64) * 100.0;
        Some(Self {
            percent_of_one_core: pct,
            window_ms: Some(wall_time.as_millis() as u64),
            label: None,
        })
    }
}

/// Named CPU budget (wake or full pipeline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuBudget {
    /// Budget name (`"wake"`, `"pipeline"`).
    pub name: String,
    /// Maximum allowed percent of one core (exclusive upper bound for "under").
    pub max_percent_of_one_core: f64,
}

impl CpuBudget {
    /// Default wake-word budget: **2%** of one core.
    pub fn wake_default() -> Self {
        Self {
            name: "wake".into(),
            max_percent_of_one_core: 2.0,
        }
    }

    /// Default full-pipeline budget: **10%** of one core.
    pub fn pipeline_default() -> Self {
        Self {
            name: "pipeline".into(),
            max_percent_of_one_core: 10.0,
        }
    }

    /// Custom budget.
    pub fn new(name: impl Into<String>, max_percent_of_one_core: f64) -> Self {
        Self {
            name: name.into(),
            max_percent_of_one_core,
        }
    }

    /// Check a single sample against this budget.
    ///
    /// Passes when `sample.percent_of_one_core < max` (strict under-budget).
    pub fn check(&self, sample: &CpuSample) -> CpuBudgetVerdict {
        let passed = sample.percent_of_one_core < self.max_percent_of_one_core;
        CpuBudgetVerdict {
            budget_name: self.name.clone(),
            max_percent_of_one_core: self.max_percent_of_one_core,
            observed_percent: sample.percent_of_one_core,
            passed,
            sample: sample_without_owned(sample),
        }
    }

    /// Check many samples; fails if **any** sample exceeds the budget.
    ///
    /// Also reports mean / max observed.
    pub fn check_all(&self, samples: &[CpuSample]) -> CpuBudgetSummary {
        if samples.is_empty() {
            return CpuBudgetSummary {
                budget_name: self.name.clone(),
                max_percent_of_one_core: self.max_percent_of_one_core,
                count: 0,
                mean_percent: 0.0,
                max_percent: 0.0,
                passed: true,
                failures: 0,
            };
        }
        let mut failures = 0usize;
        let mut sum = 0.0;
        let mut max = f64::NEG_INFINITY;
        for s in samples {
            sum += s.percent_of_one_core;
            if s.percent_of_one_core > max {
                max = s.percent_of_one_core;
            }
            if s.percent_of_one_core >= self.max_percent_of_one_core {
                failures += 1;
            }
        }
        let mean = sum / samples.len() as f64;
        CpuBudgetSummary {
            budget_name: self.name.clone(),
            max_percent_of_one_core: self.max_percent_of_one_core,
            count: samples.len(),
            mean_percent: mean,
            max_percent: max,
            passed: failures == 0,
            failures,
        }
    }
}

/// Outcome of checking one sample against a budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuBudgetVerdict {
    /// Budget name.
    pub budget_name: String,
    /// Configured max percent.
    pub max_percent_of_one_core: f64,
    /// Observed percent.
    pub observed_percent: f64,
    /// Whether the sample was under budget.
    pub passed: bool,
    /// Copy of the sample (label omitted to keep verdict `Copy`-friendly JSON).
    pub sample: CpuSampleBare,
}

/// Sample fields needed in a verdict without owned strings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CpuSampleBare {
    /// Percent of one core.
    pub percent_of_one_core: f64,
    /// Optional window ms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<u64>,
}

fn sample_without_owned(s: &CpuSample) -> CpuSampleBare {
    CpuSampleBare {
        percent_of_one_core: s.percent_of_one_core,
        window_ms: s.window_ms,
    }
}

// Re-export bare as part of sample API convenience — keep CpuSample label.

/// Aggregate check over multiple samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuBudgetSummary {
    /// Budget name.
    pub budget_name: String,
    /// Configured max percent.
    pub max_percent_of_one_core: f64,
    /// Number of samples considered.
    pub count: usize,
    /// Mean observed percent.
    pub mean_percent: f64,
    /// Max observed percent.
    pub max_percent: f64,
    /// True when every sample was under budget.
    pub passed: bool,
    /// Number of samples that met or exceeded the budget.
    pub failures: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn wake_budget_passes_under_two_percent() {
        let budget = CpuBudget::wake_default();
        let v = budget.check(&CpuSample::from_percent(1.5));
        assert!(v.passed);
        assert_eq!(v.max_percent_of_one_core, 2.0);
    }

    #[test]
    fn wake_budget_fails_at_or_above_two_percent() {
        let budget = CpuBudget::wake_default();
        assert!(!budget.check(&CpuSample::from_percent(2.0)).passed);
        assert!(!budget.check(&CpuSample::from_percent(5.0)).passed);
    }

    #[test]
    fn pipeline_budget_default_ten() {
        let b = CpuBudget::pipeline_default();
        assert_eq!(b.max_percent_of_one_core, 10.0);
        assert!(b.check(&CpuSample::from_percent(9.9)).passed);
        assert!(!b.check(&CpuSample::from_percent(10.0)).passed);
    }

    #[test]
    fn check_all_reports_failures() {
        let b = CpuBudget::wake_default();
        let samples = [
            CpuSample::from_percent(0.5),
            CpuSample::from_percent(1.0),
            CpuSample::from_percent(3.0),
        ];
        let summary = b.check_all(&samples);
        assert!(!summary.passed);
        assert_eq!(summary.failures, 1);
        assert_eq!(summary.count, 3);
        assert!((summary.max_percent - 3.0).abs() < 1e-9);
    }

    #[test]
    fn from_times_half_core() {
        let s = CpuSample::from_times(Duration::from_millis(50), Duration::from_millis(100))
            .expect("non-zero wall");
        assert!((s.percent_of_one_core - 50.0).abs() < 1e-6);
        assert_eq!(s.window_ms, Some(100));
    }

    #[test]
    fn from_times_zero_wall() {
        assert!(CpuSample::from_times(Duration::from_millis(1), Duration::ZERO).is_none());
    }
}
