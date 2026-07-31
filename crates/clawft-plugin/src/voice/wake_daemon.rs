//! Background daemon for continuous wake word detection.
//!
//! Runs the [`WakeWordDetector`] in a background loop, monitoring
//! audio input for the "Hey Weft" trigger phrase. When detected,
//! activates Talk Mode.
//!
//! ## Disposition (WEFT-671 / WEFT-216)
//!
//! Transitional home for wake: CLI `weft voice wake` is the sole live
//! external consumer. Not part of the canonical Talk Mode stack
//! (`clawft-voice-talk` / `clawft-channels::voice`). See
//! `docs/plans/wave-0a-WEFT-671-decision.md`.
//!
//! **Stub capture + stub detector** (WEFT-216): logs and waits for cancel;
//! no mic capture, no rustpotter (blocked), no ONNX KWS yet. See
//! `docs/plans/wave-0k-WEFT-216-result.md` for the engine decision and
//! revisit triggers. CPU-budget auto-throttle (<2%) is deferred until a
//! live engine + capture loop exists.

use tracing::{info, warn};

use crate::error::PluginError;
use crate::traits::CancellationToken;

use super::rate_limit::{VoiceRateDecision, VoiceRateLimiter};
use super::wake::{WakeWordBackend, WakeWordConfig, WakeWordDetector, WakeWordEvent};

/// Daemon that runs wake word detection in the background.
///
/// When the wake word is detected, the daemon can activate Talk Mode.
/// The daemon runs until cancelled via its [`CancellationToken`].
///
/// Today both capture and detection are stubs — `run` does not open a
/// microphone and never activates Talk Mode from a wake hit.
///
/// # SC-8 / WEFT-226
///
/// [`try_activate`] consults the optional [`VoiceRateLimiter`] wake
/// bucket (5/min default) before reporting an activation. Wire a shared
/// limiter from the command path so both surfaces share post-fail cooldown.
pub struct WakeDaemon {
    detector: WakeWordDetector,
    active: bool,
    /// Optional SC-8 rate limiter (wake bucket).
    rate_limiter: Option<VoiceRateLimiter>,
}

impl WakeDaemon {
    /// Create a new wake daemon with the given configuration.
    ///
    /// Uses the stub detector backend (see [`WakeWordDetector::new`]).
    pub fn new(config: WakeWordConfig) -> Result<Self, PluginError> {
        let detector = WakeWordDetector::new(config)?;
        Ok(Self {
            detector,
            active: false,
            rate_limiter: Some(VoiceRateLimiter::with_defaults()),
        })
    }

    /// Attach or replace the SC-8 rate limiter (shared with command path).
    pub fn with_rate_limiter(mut self, limiter: VoiceRateLimiter) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Disable rate limiting (tests / explicit opt-out).
    pub fn without_rate_limiter(mut self) -> Self {
        self.rate_limiter = None;
        self
    }

    /// Mutable access to the SC-8 limiter, if attached.
    pub fn rate_limiter_mut(&mut self) -> Option<&mut VoiceRateLimiter> {
        self.rate_limiter.as_mut()
    }

    /// Record a wake activation against the SC-8 wake bucket.
    ///
    /// Returns [`VoiceRateDecision::Allow`] when no limiter is attached.
    /// Call this when a real detector would open Talk Mode.
    pub fn try_activate(&mut self) -> VoiceRateDecision {
        match self.rate_limiter.as_mut() {
            Some(lim) => lim.check_wake(),
            None => VoiceRateDecision::Allow,
        }
    }

    /// Run the daemon until cancelled.
    ///
    /// **Stub:** logs backend identity, marks active, waits for cancellation.
    /// No audio capture. Real loop will feed PCM into
    /// [`WakeWordDetector::process_frame`] once an engine and cpal (or
    /// equivalent) capture path land.
    ///
    /// When the detector is a stub, emits [`WakeWordEvent::Error`] once at
    /// start (WEFT-234) so the Error variant is reached on a live path and
    /// operators see that detection is deferred — not a silent half-integration.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run(&mut self, cancel: CancellationToken) -> Result<(), PluginError> {
        info!(
            backend = %self.detector.backend(),
            stub = self.detector.is_stub(),
            "wake daemon started (no capture; engine deferred WEFT-216)"
        );
        self.active = true;
        self.detector.start();

        if self.detector.is_stub() {
            let err_event = WakeWordDetector::error_event_for_rustpotter_block();
            if let WakeWordEvent::Error { ref message } = err_event {
                warn!(
                    event = "wake_word_error",
                    backend = %self.detector.backend(),
                    "wake engine unavailable: {message}"
                );
            }
            // Keep the Error event available for callers/tests.
            let _ = err_event;
        }

        // Wait for cancellation.
        cancel.cancelled().await;

        self.detector.stop();
        self.active = false;
        info!("wake daemon stopped");
        Ok(())
    }

    /// Last engine-unavailable error event (WEFT-234 surface for tests/CLI).
    pub fn engine_unavailable_event(&self) -> Option<WakeWordEvent> {
        if self.detector.is_stub() {
            Some(WakeWordDetector::error_event_for_rustpotter_block())
        } else {
            None
        }
    }

    /// Check if the daemon is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get a reference to the underlying detector.
    pub fn detector(&self) -> &WakeWordDetector {
        &self.detector
    }

    /// Backend in use (always [`WakeWordBackend::Stub`] today).
    pub fn backend(&self) -> WakeWordBackend {
        self.detector.backend()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_daemon_create() {
        let config = WakeWordConfig::default();
        let daemon = WakeDaemon::new(config).unwrap();
        assert!(!daemon.is_active());
        assert_eq!(daemon.backend(), WakeWordBackend::Stub);
        assert!(daemon.detector().is_stub());
    }

    #[test]
    fn wake_daemon_sc8_rate_limits_activations() {
        // WEFT-226: default limiter is 5 wake activations/min.
        let mut daemon = WakeDaemon::new(WakeWordConfig::default()).unwrap();
        for i in 0..5 {
            assert!(
                daemon.try_activate().is_allowed(),
                "wake activation {i} should pass"
            );
        }
        assert!(
            !daemon.try_activate().is_allowed(),
            "6th wake activation must be rate-limited"
        );
    }

    #[test]
    fn wake_daemon_without_limiter_always_allows() {
        let mut daemon = WakeDaemon::new(WakeWordConfig::default())
            .unwrap()
            .without_rate_limiter();
        for _ in 0..20 {
            assert!(daemon.try_activate().is_allowed());
        }
    }

    #[test]
    fn wake_daemon_detector_access() {
        let config = WakeWordConfig {
            threshold: 0.3,
            ..Default::default()
        };
        let daemon = WakeDaemon::new(config).unwrap();
        assert!((daemon.detector().config().threshold - 0.3).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn wake_daemon_run_and_cancel() {
        let config = WakeWordConfig::default();
        let mut daemon = WakeDaemon::new(config).unwrap();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { daemon.run(cancel_clone).await });

        // Give the daemon a moment to start.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Cancel and verify clean shutdown.
        cancel.cancel();
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn wake_daemon_exposes_engine_unavailable_error_event() {
        let daemon = WakeDaemon::new(WakeWordConfig::default()).unwrap();
        let event = daemon.engine_unavailable_event().expect("stub => error");
        match event {
            WakeWordEvent::Error { message } => {
                assert!(message.contains("WEFT-216"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
