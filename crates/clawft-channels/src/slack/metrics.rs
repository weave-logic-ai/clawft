//! Slack Socket Mode observability counters (WEFT-174).
//!
//! Process-local atomics for API-drift detection. The primary signal is
//! [`METRIC_UNKNOWN_ENVELOPE`]: a regression where Slack changes the
//! envelope shape would drive this counter continuously; today the
//! channel only logs `received non-envelope message` and moves on.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// Metric name for non-envelope Socket Mode frames (parse failures).
///
/// Stable string for metrics exporters / health snapshots.
pub const METRIC_UNKNOWN_ENVELOPE: &str = "slack.unknown_envelope";

/// Snapshot of Slack Socket Mode counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SlackMetricsSnapshot {
    /// Count of WebSocket text frames that failed to parse as
    /// [`super::events::SlackEnvelope`].
    pub unknown_envelope: u64,
}

/// Process-local Slack Socket Mode counters.
///
/// Thread-safe; cheap to share via [`shared`]. Not a Prometheus exporter —
/// operators / tests read via [`snapshot`].
#[derive(Debug, Default)]
pub struct SlackMetrics {
    unknown_envelope: AtomicU64,
}

impl SlackMetrics {
    /// Fresh zeroed counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shared handle for multi-task sessions.
    pub fn shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }

    /// Record a non-envelope / parse-failed Socket Mode text frame.
    pub fn record_unknown_envelope(&self) {
        self.unknown_envelope.fetch_add(1, Ordering::Relaxed);
    }

    /// Current `slack.unknown_envelope` count.
    pub fn unknown_envelope(&self) -> u64 {
        self.unknown_envelope.load(Ordering::Relaxed)
    }

    /// Point-in-time snapshot for logging or tests.
    pub fn snapshot(&self) -> SlackMetricsSnapshot {
        SlackMetricsSnapshot {
            unknown_envelope: self.unknown_envelope(),
        }
    }

    /// Reset counters (tests).
    pub fn reset(&self) {
        self.unknown_envelope.store(0, Ordering::Relaxed);
    }
}

fn global_metrics() -> &'static SlackMetrics {
    static METRICS: OnceLock<SlackMetrics> = OnceLock::new();
    METRICS.get_or_init(SlackMetrics::new)
}

/// Process-wide Slack metrics (default surface for the Socket Mode loop).
pub fn slack_metrics() -> &'static SlackMetrics {
    global_metrics()
}

/// Increment the process-wide `slack.unknown_envelope` counter.
pub fn record_unknown_envelope() {
    global_metrics().record_unknown_envelope();
}

/// Read the process-wide `slack.unknown_envelope` counter.
pub fn unknown_envelope_count() -> u64 {
    global_metrics().unknown_envelope()
}

/// Classify a Socket Mode text frame as an envelope or unknown.
///
/// On parse failure, increments the process-wide
/// [`METRIC_UNKNOWN_ENVELOPE`] counter (API drift signal).
pub fn parse_socket_envelope(
    text: &str,
) -> Result<super::events::SlackEnvelope, serde_json::Error> {
    match serde_json::from_str::<super::events::SlackEnvelope>(text) {
        Ok(envelope) => Ok(envelope),
        Err(e) => {
            record_unknown_envelope();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_name_is_stable() {
        assert_eq!(METRIC_UNKNOWN_ENVELOPE, "slack.unknown_envelope");
    }

    #[test]
    fn local_metrics_increment() {
        let m = SlackMetrics::new();
        assert_eq!(m.unknown_envelope(), 0);
        m.record_unknown_envelope();
        m.record_unknown_envelope();
        assert_eq!(m.unknown_envelope(), 2);
        assert_eq!(
            m.snapshot(),
            SlackMetricsSnapshot {
                unknown_envelope: 2
            }
        );
        m.reset();
        assert_eq!(m.unknown_envelope(), 0);
    }

    #[test]
    fn known_bad_payload_increments_global_counter() {
        // Isolate from other tests by reading delta.
        let before = unknown_envelope_count();
        let bad = r#"{"not":"an envelope"}"#;
        let err = parse_socket_envelope(bad);
        assert!(err.is_err(), "known-bad payload must fail to parse");
        assert_eq!(
            unknown_envelope_count(),
            before + 1,
            "slack.unknown_envelope must increment on parse failure"
        );
    }

    #[test]
    fn valid_envelope_does_not_increment() {
        let before = unknown_envelope_count();
        let good = r#"{
            "type": "events_api",
            "envelope_id": "env-1",
            "accepts_response_payload": false,
            "payload": null
        }"#;
        let env = parse_socket_envelope(good).expect("valid envelope");
        assert_eq!(env.envelope_id, "env-1");
        assert_eq!(
            unknown_envelope_count(),
            before,
            "valid envelope must not bump unknown_envelope"
        );
    }
}
