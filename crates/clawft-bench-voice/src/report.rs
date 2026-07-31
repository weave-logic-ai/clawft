//! Aggregate voice-bench report (JSON-serializable for CI artifacts).

use serde::{Deserialize, Serialize};

use crate::cpu::CpuBudgetSummary;
use crate::latency::LatencyStats;
use crate::thresholds::{ThresholdEvaluation, VoiceBenchThresholds};
use crate::wer::WerResult;
use crate::{TICKET, VERSION};

/// Pass/fail status for one named metric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricStatus {
    /// Metric name (`latency_p95`, `wer`, …).
    pub name: String,
    /// Whether the metric passed its threshold.
    pub passed: bool,
    /// Observed value (human-readable).
    pub observed: String,
    /// Limit (human-readable).
    pub limit: String,
    /// Optional extra detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Full report written by the harness for a bench run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceBenchReport {
    /// Ticket id.
    pub ticket: String,
    /// Crate version.
    pub version: String,
    /// Thresholds used for this run.
    pub thresholds: VoiceBenchThresholds,
    /// Latency stats if measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<LatencyStats>,
    /// WER result if measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wer: Option<WerResult>,
    /// Wake CPU summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_cpu: Option<CpuBudgetSummary>,
    /// Pipeline CPU summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_cpu: Option<CpuBudgetSummary>,
    /// Per-metric statuses (flat list for CI grepping).
    pub metrics: Vec<MetricStatus>,
    /// Overall pass.
    pub passed: bool,
    /// Free-form notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl VoiceBenchReport {
    /// Build a report from an evaluation + optional raw metrics.
    pub fn from_evaluation(
        thresholds: VoiceBenchThresholds,
        evaluation: ThresholdEvaluation,
        latency: Option<LatencyStats>,
        wer: Option<WerResult>,
    ) -> Self {
        let mut metrics = Vec::new();
        if let Some(m) = evaluation.latency.clone() {
            metrics.push(m);
        }
        if let Some(m) = evaluation.wer.clone() {
            metrics.push(m);
        }
        metrics.push(MetricStatus {
            name: "cpu_wake".into(),
            passed: evaluation.wake_cpu.passed,
            observed: format!("{:.3}%", evaluation.wake_cpu.max_percent),
            limit: format!("<{}%", evaluation.wake_cpu.max_percent_of_one_core),
            detail: Some(format!(
                "mean={:.3}% failures={}/{}",
                evaluation.wake_cpu.mean_percent,
                evaluation.wake_cpu.failures,
                evaluation.wake_cpu.count
            )),
        });
        metrics.push(MetricStatus {
            name: "cpu_pipeline".into(),
            passed: evaluation.pipeline_cpu.passed,
            observed: format!("{:.3}%", evaluation.pipeline_cpu.max_percent),
            limit: format!("<{}%", evaluation.pipeline_cpu.max_percent_of_one_core),
            detail: Some(format!(
                "mean={:.3}% failures={}/{}",
                evaluation.pipeline_cpu.mean_percent,
                evaluation.pipeline_cpu.failures,
                evaluation.pipeline_cpu.count
            )),
        });

        Self {
            ticket: TICKET.into(),
            version: VERSION.into(),
            thresholds,
            latency,
            wer,
            wake_cpu: Some(evaluation.wake_cpu),
            pipeline_cpu: Some(evaluation.pipeline_cpu),
            metrics,
            passed: evaluation.passed,
            notes: None,
        }
    }

    /// Pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Compact JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::CpuSample;
    use crate::latency::LatencySample;
    use crate::wer::word_error_rate;
    use std::time::Duration;

    #[test]
    fn report_serializes() {
        let thresholds = VoiceBenchThresholds::default();
        let samples = vec![LatencySample::from_duration("mock", Duration::from_millis(12))];
        let stats = LatencyStats::from_samples(&samples).unwrap();
        let wer = word_error_rate("hi there", "hi there");
        let wake = [CpuSample::from_percent(0.8)];
        let pipe = [CpuSample::from_percent(4.0)];
        let ev = thresholds.evaluate(Some(&stats), Some(&wer), &wake, &pipe);
        let report = VoiceBenchReport::from_evaluation(
            thresholds,
            ev,
            Some(stats),
            Some(wer),
        );
        assert!(report.passed);
        assert_eq!(report.ticket, "WEFT-229");
        let json = report.to_json_pretty().unwrap();
        assert!(json.contains("latency_p95"));
        assert!(json.contains("cpu_wake"));
        let back: VoiceBenchReport = serde_json::from_str(&json).unwrap();
        assert!(back.passed);
    }
}
