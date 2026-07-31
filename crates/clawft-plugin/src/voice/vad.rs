//! Voice Activity Detection using Silero VAD (legacy scaffold).
//!
//! Product Talk Mode / energy VAD lives in `clawft-channels::voice`.
//! This module keeps the transitional Silero stub plus the shared
//! adaptive silence-timeout surface (WEFT-230) so callers can learn a
//! per-session endpoint timeout without a real microphone.

use clawft_types::config::{AdaptiveSilenceTimeout, VadConfig};

/// VAD processing result.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum VadEvent {
    /// Speech started at this sample offset.
    SpeechStart { offset: usize },
    /// Speech ended at this sample offset.
    SpeechEnd { offset: usize },
    /// No speech detected in this frame.
    Silence,
}

/// Voice Activity Detector wrapping Silero VAD model.
///
/// Currently a stub -- real sherpa-rs integration is deferred to the
/// 0.8.x in-process voice backend (see ADR-053). Silence timeout is
/// driven by a per-session [`AdaptiveSilenceTimeout`] (WEFT-230).
pub struct VoiceActivityDetector {
    threshold: f32,
    /// Baseline from config (immutable after construction unless rebaselined).
    baseline_silence_timeout_ms: u32,
    adaptive: AdaptiveSilenceTimeout,
    active: bool,
}

impl VoiceActivityDetector {
    /// Construct from threshold + baseline silence timeout with default
    /// adaptive learning (enabled, 500–3000 ms).
    pub fn new(threshold: f32, silence_timeout_ms: u32) -> Self {
        Self {
            threshold,
            baseline_silence_timeout_ms: silence_timeout_ms,
            adaptive: AdaptiveSilenceTimeout::from_baseline(silence_timeout_ms),
            active: false,
        }
    }

    /// Construct from full [`VadConfig`] (threshold, baseline, adaptive).
    pub fn from_config(cfg: &VadConfig) -> Self {
        Self {
            threshold: cfg.threshold,
            baseline_silence_timeout_ms: cfg.silence_timeout_ms,
            adaptive: cfg.silence_estimator(),
            active: false,
        }
    }

    /// Process a chunk of audio samples.
    /// Returns VAD events detected in the chunk.
    pub fn process(&mut self, _samples: &[f32]) -> Vec<VadEvent> {
        // Stub: real Silero VAD inference goes here
        vec![VadEvent::Silence]
    }

    /// Reset the VAD speech state (does **not** clear adaptive learning).
    pub fn reset(&mut self) {
        self.active = false;
    }

    /// Reset speech state **and** adaptive silence learning to baseline.
    pub fn reset_all(&mut self) {
        self.active = false;
        self.adaptive.reset();
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Config baseline silence timeout (ms), before adaptive learning.
    pub fn baseline_silence_timeout_ms(&self) -> u32 {
        self.baseline_silence_timeout_ms
    }

    /// Effective silence timeout (ms) — learned when adaptive is on.
    pub fn silence_timeout_ms(&self) -> u32 {
        self.adaptive.current_ms()
    }

    /// Access the per-session adaptive estimator.
    pub fn adaptive(&self) -> &AdaptiveSilenceTimeout {
        &self.adaptive
    }

    /// Mutable access for feeding observations from a real endpointer.
    pub fn adaptive_mut(&mut self) -> &mut AdaptiveSilenceTimeout {
        &mut self.adaptive
    }

    /// Record an utterance observation and return the updated timeout.
    ///
    /// See [`AdaptiveSilenceTimeout::record_utterance`].
    pub fn observe_utterance(&mut self, speech_ms: u32, max_intra_pause_ms: u32) -> u32 {
        self.adaptive
            .record_utterance(speech_ms, max_intra_pause_ms);
        self.silence_timeout_ms()
    }

    /// Record a false endpoint (user resumed quickly) and return the
    /// updated timeout.
    pub fn observe_false_endpoint(&mut self, bump_ms: u32) -> u32 {
        self.adaptive.record_false_endpoint(bump_ms);
        self.silence_timeout_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::config::AdaptiveSilenceConfig;

    #[test]
    fn new_starts_at_baseline() {
        let vad = VoiceActivityDetector::new(0.5, 1_500);
        assert_eq!(vad.silence_timeout_ms(), 1_500);
        assert_eq!(vad.baseline_silence_timeout_ms(), 1_500);
        assert!((vad.threshold() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn from_config_wires_adaptive() {
        let cfg = VadConfig {
            silence_timeout_ms: 1_200,
            adaptive: AdaptiveSilenceConfig {
                enabled: true,
                min_ms: 500,
                max_ms: 3_000,
                learning_rate: 1.0,
                ..AdaptiveSilenceConfig::default()
            },
            ..VadConfig::default()
        };
        let mut vad = VoiceActivityDetector::from_config(&cfg);
        assert_eq!(vad.silence_timeout_ms(), 1_200);
        let next = vad.observe_utterance(2_000, 1_000);
        // learning_rate 1.0 → snap toward pause+margin (~1250) with bias.
        assert_ne!(next, 1_200);
        assert!(next >= 500 && next <= 3_000);
    }

    #[test]
    fn observe_false_endpoint_raises() {
        let mut cfg = VadConfig::default();
        cfg.adaptive.learning_rate = 1.0;
        let mut vad = VoiceActivityDetector::from_config(&cfg);
        // Seed so current leaves pure-baseline short-circuit.
        vad.adaptive_mut().record_observation(1_500);
        let before = vad.silence_timeout_ms();
        let after = vad.observe_false_endpoint(250);
        assert!(after > before, "false endpoint should raise timeout");
    }

    #[test]
    fn reset_all_clears_learning() {
        let mut vad = VoiceActivityDetector::new(0.5, 1_500);
        for _ in 0..6 {
            vad.observe_utterance(2_500, 1_100);
        }
        assert_ne!(vad.silence_timeout_ms(), 1_500);
        vad.reset_all();
        assert_eq!(vad.silence_timeout_ms(), 1_500);
    }

    #[test]
    fn process_stub_returns_silence() {
        let mut vad = VoiceActivityDetector::new(0.5, 1_500);
        let events = vad.process(&[0.0; 320]);
        assert!(matches!(events.as_slice(), [VadEvent::Silence]));
    }
}
