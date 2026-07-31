//! Echo cancellation for the voice pipeline (legacy scaffold — WEFT-671).
//!
//! **Transitional API.** Product AEC for Talk Mode lives in
//! `clawft-voice-aec` (WebRTC AEC3 + NS). This module still implements a
//! **real pure-Rust NLMS adaptive filter** (WEFT-217) so the scaffold is not
//! a deceptive passthrough: `process()` subtracts an adaptive estimate of the
//! far-end reference from the mic.
//!
//! Limitations vs WebRTC AEC3: fixed filter length, no delay estimation,
//! mono 16 kHz assumption, weaker nonlinear residual suppression. Prefer
//! `clawft-voice-aec` on the live path. RNNoise is not used here.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Sample rate assumed by the adaptive filter (matches voice wire format).
const SAMPLE_RATE: u32 = 16_000;

/// Default NLMS step size (stable for speech-band energy).
const DEFAULT_MU: f32 = 0.4;

/// Regularization for the NLMS denominator.
const EPS: f32 = 1e-6;

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
    /// Suppression level (0.0 = no suppression, 1.0 = maximum subtraction).
    /// Scales the adaptive estimate before subtraction.
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

/// Normalized LMS (NLMS) adaptive echo canceller.
///
/// Far-end (TTS / playback) samples are queued via [`feed_reference`]; near-end
/// (mic) frames are cleaned by [`process`]. Each mic sample consumes one
/// queued reference sample (or zero if the queue is empty), updates the FIR
/// delay line, and adapts filter weights to minimise residual echo energy.
pub struct EchoCanceller {
    config: EchoCancellerConfig,
    /// Pending far-end samples waiting to align with mic samples.
    pending_ref: VecDeque<f32>,
    /// FIR delay line for the reference signal (index 0 = newest).
    delay: Vec<f32>,
    /// Adaptive FIR coefficients (same length as delay).
    weights: Vec<f32>,
    /// Number of processed frames.
    frames_processed: u64,
    /// Samples that drove an NLMS update.
    samples_adapted: u64,
}

impl EchoCanceller {
    /// Create a new echo canceller with the given configuration.
    pub fn new(config: EchoCancellerConfig) -> Self {
        let filter_len = ((config.tail_length_ms as usize * SAMPLE_RATE as usize) / 1000).max(1);
        Self {
            config,
            pending_ref: VecDeque::new(),
            delay: vec![0.0; filter_len],
            weights: vec![0.0; filter_len],
            frames_processed: 0,
            samples_adapted: 0,
        }
    }

    /// Feed reference audio (TTS / far-end playback) to the canceller.
    ///
    /// Samples are queued and consumed one-for-one with mic samples in
    /// [`process`]. Call this with the audio that was (or is about to be)
    /// played, ideally before the corresponding mic chunk.
    pub fn feed_reference(&mut self, samples: &[f32]) {
        if !self.config.enabled {
            return;
        }
        self.pending_ref.extend(samples.iter().copied());
    }

    /// Process mic input, cancelling echo correlated with the reference.
    ///
    /// Returns residual (mic − suppression_level × adaptive_estimate). When
    /// disabled, returns a copy of the input unchanged.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.frames_processed += 1;
        if !self.config.enabled {
            return input.to_vec();
        }

        let n = self.delay.len();
        let mu = DEFAULT_MU;
        let gain = self.config.suppression_level.clamp(0.0, 1.0);
        let mut out = Vec::with_capacity(input.len());

        for &d in input {
            // Align: one reference sample per mic sample (0 if underrun).
            let x0 = self.pending_ref.pop_front().unwrap_or(0.0);
            // Shift delay line (newest at index 0).
            self.delay.copy_within(0..n.saturating_sub(1), 1);
            if n > 0 {
                self.delay[0] = x0;
            }

            // y = w · x
            let mut y = 0.0f32;
            let mut power = 0.0f32;
            for i in 0..n {
                let x = self.delay[i];
                y += self.weights[i] * x;
                power += x * x;
            }

            let residual = d - gain * y;
            out.push(residual);

            // NLMS: w += μ · e · x / (‖x‖² + ε)
            let e = d - y;
            let step = mu * e / (power + EPS);
            for i in 0..n {
                self.weights[i] += step * self.delay[i];
            }
            self.samples_adapted += 1;
        }

        out
    }

    /// Reset the echo canceller state (queue, delay line, weights, counters).
    pub fn reset(&mut self) {
        self.pending_ref.clear();
        self.delay.fill(0.0);
        self.weights.fill(0.0);
        self.frames_processed = 0;
        self.samples_adapted = 0;
    }

    /// Get number of frames processed.
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed
    }

    /// Number of samples that drove an NLMS update.
    pub fn samples_adapted(&self) -> u64 {
        self.samples_adapted
    }

    /// L2 norm of the adaptive weights (diagnostics / tests).
    pub fn weight_energy(&self) -> f32 {
        self.weights.iter().map(|w| w * w).sum()
    }

    /// Length of the adaptive FIR (samples).
    pub fn filter_len(&self) -> usize {
        self.delay.len()
    }

    /// Queued reference samples not yet aligned with mic.
    pub fn pending_ref_len(&self) -> usize {
        self.pending_ref.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let ss: f32 = samples.iter().map(|s| s * s).sum();
        (ss / samples.len() as f32).sqrt()
    }

    fn tone(freq_hz: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                (2.0 * std::f32::consts::PI * freq_hz * t).sin() * amp
            })
            .collect()
    }

    #[test]
    fn new_creates_with_defaults() {
        let ec = EchoCanceller::new(EchoCancellerConfig::default());
        assert_eq!(ec.frames_processed(), 0);
        // Buffer size = 128ms * 16000Hz / 1000 = 2048 samples
        assert_eq!(ec.filter_len(), 2048);
        assert_eq!(ec.weights.len(), 2048);
    }

    #[test]
    fn process_without_reference_preserves_energy() {
        // No far-end → residual ≈ mic (weights stay near zero).
        let mut ec = EchoCanceller::new(EchoCancellerConfig::default());
        let input = vec![0.1, 0.2, -0.3, 0.4, -0.5];
        let output = ec.process(&input);
        assert_eq!(output.len(), input.len());
        assert_eq!(ec.frames_processed(), 1);
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-4, "expected near-identity without ref");
        }
    }

    #[test]
    fn feed_reference_queues_samples() {
        let mut ec = EchoCanceller::new(EchoCancellerConfig::default());
        let reference = vec![0.5; 4096];
        ec.feed_reference(&reference);
        assert_eq!(ec.pending_ref_len(), 4096);
        // Processing drains the queue.
        let _ = ec.process(&vec![0.0; 100]);
        assert_eq!(ec.pending_ref_len(), 4096 - 100);
    }

    #[test]
    fn reset_clears_state() {
        let mut ec = EchoCanceller::new(EchoCancellerConfig {
            tail_length_ms: 16,
            ..Default::default()
        });
        ec.feed_reference(&[1.0; 100]);
        ec.process(&[0.5; 10]);
        ec.process(&[0.5; 10]);
        assert_eq!(ec.frames_processed(), 2);
        assert!(ec.samples_adapted() > 0);

        ec.reset();
        assert_eq!(ec.frames_processed(), 0);
        assert_eq!(ec.samples_adapted(), 0);
        assert_eq!(ec.pending_ref_len(), 0);
        assert!(ec.delay.iter().all(|&s| s == 0.0));
        assert!(ec.weights.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn disabled_skips_reference_and_processing() {
        let config = EchoCancellerConfig {
            enabled: false,
            ..Default::default()
        };
        let mut ec = EchoCanceller::new(config);

        ec.feed_reference(&[1.0; 100]);
        assert_eq!(ec.pending_ref_len(), 0);

        let input = vec![0.1, 0.2, 0.3];
        let output = ec.process(&input);
        assert_eq!(input, output);
        assert_eq!(ec.frames_processed(), 1);
        assert_eq!(ec.samples_adapted(), 0);
    }

    #[test]
    fn config_defaults_are_correct() {
        let config = EchoCancellerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.tail_length_ms, 128);
        assert!((config.suppression_level - 0.8).abs() < f32::EPSILON);
    }

    /// Synthetic linear echo: mic = delayed scaled reference.
    /// After adaptation the residual energy must be well below the mic energy.
    #[test]
    fn synthetic_echo_is_attenuated() {
        let tail_ms = 32u32;
        let filter_len = (tail_ms as usize * SAMPLE_RATE as usize) / 1000;
        let mut ec = EchoCanceller::new(EchoCancellerConfig {
            enabled: true,
            tail_length_ms: tail_ms,
            suppression_level: 1.0,
        });

        let n_train = filter_len * 40;
        let mut far = Vec::with_capacity(n_train);
        for i in 0..n_train {
            let t = i as f32 / SAMPLE_RATE as f32;
            let s = 0.4 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
                + 0.3 * (2.0 * std::f32::consts::PI * 880.0 * t).sin()
                + 0.2 * (2.0 * std::f32::consts::PI * 1320.0 * t).sin();
            far.push(s);
        }

        // Echo path: gain 0.6 at lag 8 samples (single-tap linear).
        let delay = 8usize;
        let path_gain = 0.6f32;
        let mut near = vec![0.0f32; n_train];
        for i in delay..n_train {
            near[i] = path_gain * far[i - delay];
        }

        let chunk = 80; // 5 ms
        let mut residual = Vec::new();
        for start in (0..n_train).step_by(chunk) {
            let end = (start + chunk).min(n_train);
            ec.feed_reference(&far[start..end]);
            residual.extend(ec.process(&near[start..end]));
        }

        let half = residual.len() / 2;
        let mic_rms = rms(&near[half..]);
        let res_rms = rms(&residual[half..]);
        assert!(
            mic_rms > 0.01,
            "synthetic echo should have energy, mic_rms={mic_rms}"
        );
        let ratio = res_rms / mic_rms;
        assert!(
            ratio < 0.25,
            "NLMS must attenuate synthetic echo: residual/mic = {ratio:.3} \
             (res_rms={res_rms:.5}, mic_rms={mic_rms:.5}); not a passthrough"
        );
        assert!(
            ec.weight_energy() > 1e-4,
            "adaptive weights should be non-zero after echo training"
        );
    }

    #[test]
    fn process_is_not_identity_with_correlated_echo() {
        let mut ec = EchoCanceller::new(EchoCancellerConfig {
            tail_length_ms: 16,
            suppression_level: 1.0,
            ..Default::default()
        });
        let far = tone(500.0, 2000, 0.5);
        let mut saw_diff = false;
        for start in (0..far.len()).step_by(160) {
            let end = (start + 160).min(far.len());
            ec.feed_reference(&far[start..end]);
            let out = ec.process(&far[start..end]);
            if start > 800 {
                let same = out
                    .iter()
                    .zip(far[start..end].iter())
                    .all(|(a, b)| (a - b).abs() < 1e-6);
                if !same {
                    saw_diff = true;
                }
            }
        }
        assert!(
            saw_diff,
            "output must not be bit-identical to input once adapted"
        );
    }
}
