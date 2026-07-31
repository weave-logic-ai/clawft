//! xAI voice health probe stub (ADR-074 V2 / WEFT-691).
//!
//! Pre-dial gate for [`super::xai_realtime`] and transport selection.
//! The default path is a **pure stub** (no network) so unit tests and offline
//! builds stay deterministic. A live optional probe can be enabled later
//! without changing the selection rule surface.
//!
//! # Integration
//!
//! ```ignore
//! use clawft_channels::voice::xai_health::{probe_xai_voice_health, XaiHealthProbeMode};
//! use clawft_types::config::{resolve_voice_transport_with_health, VoiceMode};
//!
//! let health = probe_xai_voice_health(XaiHealthProbeMode::Stub { healthy: true });
//! let decision = resolve_voice_transport_with_health(VoiceMode::XaiHybrid, true, &health);
//! ```

use clawft_types::config::{
    record_runtime_fallback, resolve_voice_transport_with_health, resolve_voice_transport_with_metrics,
    VoiceMode, VoiceTransportDecision, VoiceTransportMetrics, XaiHealthProbeResult,
    DEFAULT_XAI_REALTIME_ENDPOINT,
};

/// How the V2 health probe should behave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XaiHealthProbeMode {
    /// Pure stub — inject healthy/unhealthy without I/O (default for tests + CI).
    Stub { healthy: bool },
    /// Treat as healthy when an API key is present (skip network). Useful at
    /// session start before a real dial; connect failure still uses runtime fallback.
    AssumeHealthyIfKey { api_key_present: bool },
}

impl Default for XaiHealthProbeMode {
    fn default() -> Self {
        Self::Stub { healthy: true }
    }
}

/// Run the health probe (V2: stub modes only — no network I/O).
///
/// Returns [`XaiHealthProbeResult`] suitable for
/// [`resolve_voice_transport_with_health`].
pub fn probe_xai_voice_health(mode: XaiHealthProbeMode) -> XaiHealthProbeResult {
    match mode {
        XaiHealthProbeMode::Stub { healthy } => {
            if healthy {
                XaiHealthProbeResult {
                    healthy: true,
                    detail: "stub: healthy (no network probe)",
                }
            } else {
                XaiHealthProbeResult {
                    healthy: false,
                    detail: "stub: unhealthy (injected)",
                }
            }
        }
        XaiHealthProbeMode::AssumeHealthyIfKey { api_key_present } => {
            if api_key_present {
                XaiHealthProbeResult {
                    healthy: true,
                    detail: "assume-healthy: api key present (pre-dial)",
                }
            } else {
                // Without a key the selection rule will fail on key first;
                // still report unhealthy so callers that only check health degrade.
                XaiHealthProbeResult {
                    healthy: false,
                    detail: "assume-healthy: api key missing",
                }
            }
        }
    }
}

/// Select transport for Talk-Mode using key + health probe + optional metrics.
///
/// Pure w.r.t. network: `probe_mode` must be a stub / assume-key mode.
pub fn select_voice_transport(
    mode: VoiceMode,
    api_key_present: bool,
    probe_mode: XaiHealthProbeMode,
    metrics: Option<&VoiceTransportMetrics>,
) -> VoiceTransportDecision {
    let health = probe_xai_voice_health(probe_mode);
    if let Some(m) = metrics {
        resolve_voice_transport_with_metrics(mode, api_key_present, health.healthy, m)
    } else {
        resolve_voice_transport_with_health(mode, api_key_present, &health)
    }
}

/// After a connect/session error: record failure metrics and fail open to local.
pub fn graceful_local_fallback_after_failure(
    preferred_mode: VoiceMode,
    metrics: Option<&VoiceTransportMetrics>,
) -> VoiceTransportDecision {
    if let Some(m) = metrics {
        m.record_connect_fail();
        record_runtime_fallback(preferred_mode, m)
    } else {
        clawft_types::config::resolve_voice_transport_runtime_fallback(preferred_mode)
    }
}

/// Safe-to-log default endpoint string for diagnostics (no credentials).
pub fn health_probe_endpoint_label() -> &'static str {
    DEFAULT_XAI_REALTIME_ENDPOINT
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::config::{
        HybridVoiceComposition, HybridVoiceLeg, ResolvedVoiceTransport, VoiceFallbackReason,
        VOICE_NOTICE_XAI_RUNTIME_FALLBACK, VOICE_NOTICE_XAI_UNHEALTHY,
    };

    #[test]
    fn stub_unhealthy_forces_local_for_hybrid() {
        let d = select_voice_transport(
            VoiceMode::XaiHybrid,
            true,
            XaiHealthProbeMode::Stub { healthy: false },
            None,
        );
        assert_eq!(d.transport, ResolvedVoiceTransport::Local);
        assert_eq!(d.notice, Some(VOICE_NOTICE_XAI_UNHEALTHY));
        assert_eq!(d.fallback_reason, Some(VoiceFallbackReason::HealthFailed));
        assert_eq!(d.hybrid, HybridVoiceComposition::LOCAL_ALL);
        assert!(!d.hybrid.uses_xai());
    }

    #[test]
    fn stub_healthy_keeps_hybrid_composition() {
        let d = select_voice_transport(
            VoiceMode::XaiHybrid,
            true,
            XaiHealthProbeMode::Stub { healthy: true },
            None,
        );
        assert_eq!(d.transport, ResolvedVoiceTransport::XaiHybrid);
        assert_eq!(d.hybrid.stt, HybridVoiceLeg::Local);
        assert_eq!(d.hybrid.tts, HybridVoiceLeg::Xai);
        assert!(d.notice.is_none());
    }

    #[test]
    fn assume_healthy_if_key_gates_on_key() {
        let no_key = probe_xai_voice_health(XaiHealthProbeMode::AssumeHealthyIfKey {
            api_key_present: false,
        });
        assert!(!no_key.healthy);

        let with_key = probe_xai_voice_health(XaiHealthProbeMode::AssumeHealthyIfKey {
            api_key_present: true,
        });
        assert!(with_key.healthy);
    }

    #[test]
    fn graceful_fallback_records_metrics() {
        let metrics = VoiceTransportMetrics::new();
        let d = graceful_local_fallback_after_failure(VoiceMode::XaiS2s, Some(&metrics));
        assert_eq!(d.transport, ResolvedVoiceTransport::Local);
        assert_eq!(d.notice, Some(VOICE_NOTICE_XAI_RUNTIME_FALLBACK));
        assert_eq!(d.fallback_reason, Some(VoiceFallbackReason::RuntimeFailure));

        let snap = metrics.snapshot();
        assert_eq!(snap.connect_fail, 1);
        assert_eq!(snap.fallback_runtime, 1);
        assert_eq!(snap.select_local, 1);
    }

    #[test]
    fn select_with_metrics_counts_hybrid_and_fallback() {
        let metrics = VoiceTransportMetrics::new();
        let _ = select_voice_transport(
            VoiceMode::XaiHybrid,
            true,
            XaiHealthProbeMode::Stub { healthy: true },
            Some(&metrics),
        );
        let _ = select_voice_transport(
            VoiceMode::XaiHybrid,
            true,
            XaiHealthProbeMode::Stub { healthy: false },
            Some(&metrics),
        );
        let snap = metrics.snapshot();
        assert_eq!(snap.select_xai_hybrid, 1);
        assert_eq!(snap.select_local, 1);
        assert_eq!(snap.fallback_unhealthy, 1);
    }

    #[test]
    fn endpoint_label_is_safe() {
        let e = health_probe_endpoint_label();
        assert!(e.starts_with("wss://"));
        assert!(!e.contains("Bearer"));
    }
}
