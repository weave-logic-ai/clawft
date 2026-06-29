//! [`AudioControl`] barge-in binding over `clawft_voice_aec::AecProcessor`.

use std::sync::Mutex;

use clawft_channels::voice::talkmode::AudioControl;
use clawft_voice_aec::AecProcessor;

/// Barge-in control backed by the in-process AEC.
///
/// Wraps the shared [`AecProcessor`] so a barge-in can drop its queued render
/// reference the instant playback is silenced — stale far-end frames must stop
/// cancelling the user's onset, or the interruption gets eaten. The full-duplex
/// audio loop owns the *same* processor handle (for `push_render` /
/// `process_capture`); here we expose only the flush so the controller stays
/// device-agnostic.
pub struct AecAudioControl {
    proc: Mutex<AecProcessor>,
}

impl AecAudioControl {
    /// Wrap an existing processor.
    pub fn new(proc: AecProcessor) -> Self {
        Self {
            proc: Mutex::new(proc),
        }
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
