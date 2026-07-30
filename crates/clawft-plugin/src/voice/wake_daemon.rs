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

use tracing::info;

use crate::error::PluginError;
use crate::traits::CancellationToken;

use super::wake::{WakeWordBackend, WakeWordConfig, WakeWordDetector};

/// Daemon that runs wake word detection in the background.
///
/// When the wake word is detected, the daemon can activate Talk Mode.
/// The daemon runs until cancelled via its [`CancellationToken`].
///
/// Today both capture and detection are stubs — `run` does not open a
/// microphone and never activates Talk Mode from a wake hit.
pub struct WakeDaemon {
    detector: WakeWordDetector,
    active: bool,
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
        })
    }

    /// Run the daemon until cancelled.
    ///
    /// **Stub:** logs backend identity, marks active, waits for cancellation.
    /// No audio capture. Real loop will feed PCM into
    /// [`WakeWordDetector::process_frame`] once an engine and cpal (or
    /// equivalent) capture path land.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run(&mut self, cancel: CancellationToken) -> Result<(), PluginError> {
        info!(
            backend = %self.detector.backend(),
            stub = self.detector.is_stub(),
            "wake daemon started (no capture; engine deferred WEFT-216)"
        );
        self.active = true;
        self.detector.start();

        // Wait for cancellation.
        cancel.cancelled().await;

        self.detector.stop();
        self.active = false;
        info!("wake daemon stopped");
        Ok(())
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
}
