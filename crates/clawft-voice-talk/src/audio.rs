//! [`AudioControl`] barge-in binding over `clawft_voice_aec::AecProcessor`.

use std::sync::{Arc, Mutex};

use clawft_channels::voice::talkmode::AudioControl;
use clawft_voice_aec::AecProcessor;

/// Barge-in control backed by the in-process AEC.
///
/// Holds the **same** `Arc<Mutex<AecProcessor>>` the full-duplex audio loop uses
/// for `push_render` (the [`AecTtsSink`](clawft_voice_aec::AecTtsSink) playback
/// reference) and `process_capture` ([`run_capture`](clawft_voice_aec::run_capture)),
/// so a barge-in [`flush`](AudioControl::flush) drops the *same* queued render
/// reference that is cancelling the user's onset (the live path shares one AEC
/// across sink + capture + this control — ADR-062 Phase 5/6, [`from_shared`]).
///
/// [`from_shared`]: AecAudioControl::from_shared
pub struct AecAudioControl {
    proc: Arc<Mutex<AecProcessor>>,
}

impl AecAudioControl {
    /// Wrap a freshly-owned processor (no sharing — tests / no-device paths).
    pub fn new(proc: AecProcessor) -> Self {
        Self {
            proc: Arc::new(Mutex::new(proc)),
        }
    }

    /// Wrap the **shared** processor handle the live audio loop uses for
    /// playback (render reference) and capture, so barge-in flushes the one AEC
    /// the echo is cancelled by (ADR-062 Phase 6 live composition).
    pub fn from_shared(proc: Arc<Mutex<AecProcessor>>) -> Self {
        Self { proc }
    }

    /// Build over a fresh default processor (passthrough unless built with
    /// `--features webrtc-aec`).
    pub fn from_default() -> Self {
        Self::new(AecProcessor::new())
    }

    /// Whether the underlying AEC is the real echo canceller (vs passthrough).
    pub fn is_active(&self) -> bool {
        self.proc.lock().map(|p| p.is_active()).unwrap_or(false)
    }
}

impl AudioControl for AecAudioControl {
    fn flush(&self) {
        if let Ok(mut p) = self.proc.lock() {
            p.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_is_safe_and_idempotent() {
        let ctl = AecAudioControl::from_default();
        // Queue a render reference, then flush twice — must not panic.
        ctl.flush();
        ctl.flush();
        // is_active reflects the build feature (passthrough in the default test).
        assert_eq!(ctl.is_active(), cfg!(feature = "webrtc-aec"));
    }
}
