//! Echo cancellation for voice pipeline.
//!
//! Prevents the system from hearing its own TTS output during Talk Mode.
//! Uses a reference signal (the TTS audio) to cancel echoes from the mic input.
//!
//! Current implementation: stub (passthrough). Real AEC will use webrtc-audio-processing
//! or a custom delay-line canceller in a future phase.

use serde::{Deserialize, Serialize};

/// Configuration for echo cancellation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoCancellerConfig {
    /// Enable echo cancellation.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Tail length in milliseconds (how far back to look for echoes).
    /// Typical values: 50-200ms.
    #[serde(default = "default_tail_ms")]
    pub tail_length_ms: u32,
    /// Suppression level (0.0 = no suppression, 1.0 = maximum suppression).
    #[serde(default = "default_suppression")]
    pub suppression_level: f32,
}

fn default_true() -> bool {
    true
}
fn default_tail_ms() -> u32 {
    128
}
fn default_suppression() -> f32 {
    0.8
}

impl Default for EchoCancellerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tail_length_ms: 128,
            suppression_level: 0.8,
        }
    }
}

/// Echo canceller state.
pub struct EchoCanceller {
    config: EchoCancellerConfig,
    /// Circular buffer for reference signal (TTS output).
    reference_buffer: Vec<f32>,
    /// Write position in reference buffer.
    write_pos: usize,
    /// Number of processed frames.
    frames_processed: u64,
}

impl EchoCanceller {
    /// Create a new echo canceller with the given configuration.
    pub fn new(config: EchoCancellerConfig) -> Self {
        // Buffer size: tail_length_ms * sample_rate / 1000
        let buffer_size = (config.tail_length_ms as usize * 16000) / 1000;
        Self {
            config,
            reference_buffer: vec![0.0; buffer_size],
            write_pos: 0,
            frames_processed: 0,
        }
    }

    /// Feed reference audio (TTS output) to the canceller.
    pub fn feed_reference(&mut self, samples: &[f32]) {
        if !self.config.enabled {
            return;
        }
        for &sample in samples {
            self.reference_buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.reference_buffer.len();
        }
    }

    /// Process mic input, cancelling echo from reference.
    /// Currently a passthrough stub -- returns input unchanged.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.frames_processed += 1;
        if !self.config.enabled {
            return input.to_vec();
        }
        // STUB: Real AEC would use NLMS or frequency-domain adaptive filter here.
        // For now, just pass through.
        input.to_vec()
    }

    /// Reset the echo canceller state.
    pub fn reset(&mut self) {
        self.reference_buffer.fill(0.0);
        self.write_pos = 0;
        self.frames_processed = 0;
    }

    /// Get number of frames processed.
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_with_defaults() {
        let ec = EchoCanceller::new(EchoCancellerConfig::default());
        assert_eq!(ec.frames_processed(), 0);
        // Buffer size = 128ms * 16000Hz / 1000 = 2048 samples
        assert_eq!(ec.reference_buffer.len(), 2048);
    }

    #[test]
    fn process_passthrough_preserves_input() {
        let mut ec = EchoCanceller::new(EchoCancellerConfig::default());
        let input = vec![0.1, 0.2, -0.3, 0.4, -0.5];
        let output = ec.process(&input);
        assert_eq!(input, output);
        assert_eq!(ec.frames_processed(), 1);
    }

    #[test]
    fn feed_reference_does_not_panic() {
        let mut ec = EchoCanceller::new(EchoCancellerConfig::default());
        let reference = vec![0.5; 4096]; // larger than buffer to test wrap-around
        ec.feed_reference(&reference);
        // Should not panic, write_pos wraps around
        assert!(ec.write_pos < ec.reference_buffer.len());
    }

    #[test]
    fn reset_clears_state() {
        let mut ec = EchoCanceller::new(EchoCancellerConfig::default());
        ec.feed_reference(&[1.0; 100]);
        ec.process(&[0.5; 10]);
        ec.process(&[0.5; 10]);
        assert_eq!(ec.frames_processed(), 2);

        ec.reset();
        assert_eq!(ec.frames_processed(), 0);
        assert_eq!(ec.write_pos, 0);
        assert!(ec.reference_buffer.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn disabled_skips_reference_and_processing() {
        let config = EchoCancellerConfig {
            enabled: false,
            ..Default::default()
        };
        let mut ec = EchoCanceller::new(config);

        // Feed reference should be a no-op when disabled
        ec.feed_reference(&[1.0; 100]);
        assert!(ec.reference_buffer.iter().all(|&s| s == 0.0));

        // Process still returns input (passthrough) and increments counter
        let input = vec![0.1, 0.2, 0.3];
        let output = ec.process(&input);
        assert_eq!(input, output);
        assert_eq!(ec.frames_processed(), 1);
    }

    #[test]
    fn config_defaults_are_correct() {
        let config = EchoCancellerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.tail_length_ms, 128);
        assert!((config.suppression_level - 0.8).abs() < f32::EPSILON);
    }
}
