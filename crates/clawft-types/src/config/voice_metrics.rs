//! Voice transport metrics (ADR-074 V2 / WEFT-691).
//!
//! Process-local counters for transport selection, fail-open fallback, and
//! basic session health (connects, disconnects, TTFA samples). Safe to use
//! from async tasks; never stores secrets or audio.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::voice::{
    resolve_voice_transport, resolve_voice_transport_runtime_fallback, ResolvedVoiceTransport,
    VoiceFallbackReason, VoiceMode, VoiceTransportDecision,
};

/// Snapshot of [`VoiceTransportMetrics`] for tests / diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoiceTransportMetricsSnapshot {
    pub select_local: u64,
    pub select_xai_s2s: u64,
    pub select_xai_hybrid: u64,
    pub fallback_no_key: u64,
    pub fallback_unhealthy: u64,
    pub fallback_runtime: u64,
    pub connect_ok: u64,
    pub connect_fail: u64,
    pub disconnects: u64,
    pub ttfa_ms_sum: u64,
    pub ttfa_samples: u64,
}

impl VoiceTransportMetricsSnapshot {
    /// Mean TTFA in milliseconds, if any samples were recorded.
    pub fn ttfa_ms_mean(self) -> Option<f64> {
        if self.ttfa_samples == 0 {
            None
        } else {
            Some(self.ttfa_ms_sum as f64 / self.ttfa_samples as f64)
        }
    }

    /// Total transport selections recorded.
    pub fn total_selections(self) -> u64 {
        self.select_local + self.select_xai_s2s + self.select_xai_hybrid
    }

    /// Total fail-open / runtime fallback events.
    pub fn total_fallbacks(self) -> u64 {
        self.fallback_no_key + self.fallback_unhealthy + self.fallback_runtime
    }
}

/// Atomic counters for ADR-074 voice transport selection and failures.
///
/// Thread-safe; cheap to clone via [`Arc`]. Not a Prometheus exporter — just
/// in-process hooks the daemon / Talk-Mode can read or log.
#[derive(Debug, Default)]
pub struct VoiceTransportMetrics {
    select_local: AtomicU64,
    select_xai_s2s: AtomicU64,
    select_xai_hybrid: AtomicU64,
    fallback_no_key: AtomicU64,
    fallback_unhealthy: AtomicU64,
    fallback_runtime: AtomicU64,
    connect_ok: AtomicU64,
    connect_fail: AtomicU64,
    disconnects: AtomicU64,
    ttfa_ms_sum: AtomicU64,
    ttfa_samples: AtomicU64,
}

impl VoiceTransportMetrics {
    /// Fresh zeroed counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shared handle for multi-task sessions.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Record a resolved transport decision (selection + optional fallback).
    pub fn record_decision(&self, decision: &VoiceTransportDecision) {
        match decision.transport {
            ResolvedVoiceTransport::Local => {
                self.select_local.fetch_add(1, Ordering::Relaxed);
            }
            ResolvedVoiceTransport::XaiS2s => {
                self.select_xai_s2s.fetch_add(1, Ordering::Relaxed);
            }
            ResolvedVoiceTransport::XaiHybrid => {
                self.select_xai_hybrid.fetch_add(1, Ordering::Relaxed);
            }
        }
        if let Some(reason) = decision.fallback_reason {
            match reason {
                VoiceFallbackReason::NoApiKey => {
                    self.fallback_no_key.fetch_add(1, Ordering::Relaxed);
                }
                VoiceFallbackReason::HealthFailed => {
                    self.fallback_unhealthy.fetch_add(1, Ordering::Relaxed);
                }
                VoiceFallbackReason::RuntimeFailure => {
                    self.fallback_runtime.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Successful xAI Realtime connect / hello.
    pub fn record_connect_ok(&self) {
        self.connect_ok.fetch_add(1, Ordering::Relaxed);
    }

    /// Failed connect (counts as connect_fail; pair with runtime fallback separately).
    pub fn record_connect_fail(&self) {
        self.connect_fail.fetch_add(1, Ordering::Relaxed);
    }

    /// Session disconnect (clean or dirty).
    pub fn record_disconnect(&self) {
        self.disconnects.fetch_add(1, Ordering::Relaxed);
    }

    /// Record one time-to-first-audio sample in milliseconds.
    pub fn record_ttfa_ms(&self, ms: u64) {
        self.ttfa_ms_sum.fetch_add(ms, Ordering::Relaxed);
        self.ttfa_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Point-in-time snapshot for logging or tests.
    pub fn snapshot(&self) -> VoiceTransportMetricsSnapshot {
        VoiceTransportMetricsSnapshot {
            select_local: self.select_local.load(Ordering::Relaxed),
            select_xai_s2s: self.select_xai_s2s.load(Ordering::Relaxed),
            select_xai_hybrid: self.select_xai_hybrid.load(Ordering::Relaxed),
            fallback_no_key: self.fallback_no_key.load(Ordering::Relaxed),
            fallback_unhealthy: self.fallback_unhealthy.load(Ordering::Relaxed),
            fallback_runtime: self.fallback_runtime.load(Ordering::Relaxed),
            connect_ok: self.connect_ok.load(Ordering::Relaxed),
            connect_fail: self.connect_fail.load(Ordering::Relaxed),
            disconnects: self.disconnects.load(Ordering::Relaxed),
            ttfa_ms_sum: self.ttfa_ms_sum.load(Ordering::Relaxed),
            ttfa_samples: self.ttfa_samples.load(Ordering::Relaxed),
        }
    }
}

/// Resolve transport and record selection/fallback metrics in one step.
pub fn resolve_voice_transport_with_metrics(
    mode: VoiceMode,
    api_key_present: bool,
    xai_healthy: bool,
    metrics: &VoiceTransportMetrics,
) -> VoiceTransportDecision {
    let decision = resolve_voice_transport(mode, api_key_present, xai_healthy);
    metrics.record_decision(&decision);
    decision
}

/// Record a runtime fail-open to local (connect/session error path).
pub fn record_runtime_fallback(
    mode: VoiceMode,
    metrics: &VoiceTransportMetrics,
) -> VoiceTransportDecision {
    let decision = resolve_voice_transport_runtime_fallback(mode);
    metrics.record_decision(&decision);
    decision
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::voice::{VOICE_NOTICE_XAI_NO_KEY, VOICE_NOTICE_XAI_UNHEALTHY};

    #[test]
    fn metrics_record_selections_and_fallbacks() {
        let m = VoiceTransportMetrics::new();

        let local = resolve_voice_transport_with_metrics(VoiceMode::Local, false, false, &m);
        assert_eq!(local.transport, ResolvedVoiceTransport::Local);
        assert!(!local.is_fallback());

        let no_key =
            resolve_voice_transport_with_metrics(VoiceMode::XaiS2s, false, true, &m);
        assert_eq!(no_key.notice, Some(VOICE_NOTICE_XAI_NO_KEY));

        let unhealthy =
            resolve_voice_transport_with_metrics(VoiceMode::XaiHybrid, true, false, &m);
        assert_eq!(unhealthy.notice, Some(VOICE_NOTICE_XAI_UNHEALTHY));
        assert_eq!(unhealthy.transport, ResolvedVoiceTransport::Local);

        let hybrid_ok =
            resolve_voice_transport_with_metrics(VoiceMode::XaiHybrid, true, true, &m);
        assert_eq!(hybrid_ok.transport, ResolvedVoiceTransport::XaiHybrid);

        let s2s_ok = resolve_voice_transport_with_metrics(VoiceMode::XaiS2s, true, true, &m);
        assert_eq!(s2s_ok.transport, ResolvedVoiceTransport::XaiS2s);

        let snap = m.snapshot();
        // local ok + no_key fallback local + unhealthy fallback local = 3 local selections
        assert_eq!(snap.select_local, 3);
        assert_eq!(snap.select_xai_s2s, 1);
        assert_eq!(snap.select_xai_hybrid, 1);
        assert_eq!(snap.fallback_no_key, 1);
        assert_eq!(snap.fallback_unhealthy, 1);
        assert_eq!(snap.fallback_runtime, 0);
        assert_eq!(snap.total_selections(), 5);
        assert_eq!(snap.total_fallbacks(), 2);
    }

    #[test]
    fn runtime_fallback_increments_runtime_counter() {
        let m = VoiceTransportMetrics::new();
        let d = record_runtime_fallback(VoiceMode::XaiS2s, &m);
        assert!(d.is_fallback());
        assert_eq!(d.fallback_reason, Some(VoiceFallbackReason::RuntimeFailure));

        m.record_connect_fail();
        m.record_disconnect();
        m.record_ttfa_ms(120);
        m.record_ttfa_ms(80);

        let snap = m.snapshot();
        assert_eq!(snap.select_local, 1);
        assert_eq!(snap.fallback_runtime, 1);
        assert_eq!(snap.connect_fail, 1);
        assert_eq!(snap.disconnects, 1);
        assert_eq!(snap.ttfa_samples, 2);
        assert_eq!(snap.ttfa_ms_mean(), Some(100.0));
    }

    #[test]
    fn connect_ok_does_not_count_as_fallback() {
        let m = VoiceTransportMetrics::shared();
        let d = resolve_voice_transport_with_metrics(VoiceMode::XaiS2s, true, true, &m);
        m.record_connect_ok();
        assert!(!d.is_fallback());
        let snap = m.snapshot();
        assert_eq!(snap.connect_ok, 1);
        assert_eq!(snap.total_fallbacks(), 0);
    }
}
