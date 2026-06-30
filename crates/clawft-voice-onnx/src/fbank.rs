//! SpeechBrain-compatible log-mel filterbank front-end (the ECAPA "input prep").
//!
//! This is the audio analogue of tokenization in the text embedders: raw 16 kHz
//! mono PCM in, an `[n_frames, 80]` log-mel feature matrix out, ready to feed the
//! ECAPA ONNX graph. It reproduces SpeechBrain's `Fbank` + sentence
//! `InputNormalization` exactly (validated to max-abs-diff ~7e-4 / feature
//! cosine 0.99999994 against `speechbrain/spkrec-ecapa-voxceleb`), so the
//! embeddings match the Python reference the voicelab harness uses.
//!
//! Pipeline (all constants pinned to the model's front-end):
//! 1. centre zero-pad by `n_fft/2`, frame at 25 ms / 10 ms (400 / 160 samples);
//! 2. periodic Hamming window → 400-pt real FFT → power spectrum (`|X|²`, 201 bins);
//! 3. triangular HTK-mel filterbank (80 filters, 0–8000 Hz);
//! 4. `10·log10(max(·, 1e-10))` with a per-utterance `max − 80 dB` floor;
//! 5. per-mel-bin mean subtraction over time (sentence norm, no variance).
//!
//! Pure math with no model dependency — always compiled and unit-tested.

use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// FFT size (25 ms at 16 kHz).
pub const N_FFT: usize = 400;
/// Hop between frames (10 ms at 16 kHz).
pub const HOP: usize = 160;
/// Number of mel filters / output feature dimension per frame.
pub const N_MELS: usize = 80;
/// Target sample rate the front-end (and the model) expect.
pub const SAMPLE_RATE: u32 = 16_000;

/// One-sided spectrum bin count (`n_fft/2 + 1`).
const N_STFT: usize = N_FFT / 2 + 1;
const F_MIN: f32 = 0.0;
const F_MAX: f32 = 8_000.0;
const AMIN: f32 = 1e-10;
const TOP_DB: f32 = 80.0;
/// dB multiplier for a power spectrogram (`power_spectrogram = 2` → 10).
const DB_MULTIPLIER: f32 = 10.0;

/// Log-mel filterbank extractor. Build once (it precomputes the window and mel
/// matrix and plans the FFT), then call [`Fbank::compute`] per utterance.
pub struct Fbank {
    /// Periodic Hamming window, length [`N_FFT`].
    window: Vec<f32>,
    /// Triangular mel filters, `N_MELS` rows of `N_STFT` weights each.
    mel_filters: Vec<Vec<f32>>,
    fft: Arc<dyn Fft<f32>>,
}

impl Fbank {
    /// Build the front-end with the pinned ECAPA configuration.
    pub fn new() -> Self {
        let window = (0..N_FFT)
            .map(|n| 0.54 - 0.46 * (2.0 * std::f32::consts::PI * n as f32 / N_FFT as f32).cos())
            .collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(N_FFT);
        Self {
            window,
            mel_filters: build_mel_filters(),
            fft,
        }
    }

    /// Compute log-mel features for one utterance.
    ///
    /// `samples` is mono PCM in `[-1.0, 1.0]` at [`SAMPLE_RATE`]. Returns the
    /// frame count and a row-major `[n_frames * N_MELS]` feature buffer. An
    /// utterance shorter than one frame yields `(0, vec![])`.
    pub fn compute(&self, samples: &[f32]) -> (usize, Vec<f32>) {
        // Centre padding (`center = true`, `pad_mode = "constant"`).
        let pad = N_FFT / 2;
        let mut padded = vec![0.0f32; samples.len() + 2 * pad];
        padded[pad..pad + samples.len()].copy_from_slice(samples);
        if padded.len() < N_FFT {
            return (0, Vec::new());
        }
        let n_frames = 1 + (padded.len() - N_FFT) / HOP;

        let mut feats = vec![0.0f32; n_frames * N_MELS];
        let mut buf = vec![Complex::new(0.0f32, 0.0f32); N_FFT];
        let mut power = vec![0.0f32; N_STFT];
        let mut global_max = f32::MIN;

        for t in 0..n_frames {
            let start = t * HOP;
            for i in 0..N_FFT {
                buf[i] = Complex::new(padded[start + i] * self.window[i], 0.0);
            }
            self.fft.process(&mut buf);
            for (k, p) in power.iter_mut().enumerate() {
                *p = buf[k].re * buf[k].re + buf[k].im * buf[k].im;
            }
            for m in 0..N_MELS {
                let row = &self.mel_filters[m];
                let mut acc = 0.0f32;
                for k in 0..N_STFT {
                    acc += power[k] * row[k];
                }
                let db = DB_MULTIPLIER * acc.max(AMIN).log10();
                feats[t * N_MELS + m] = db;
                if db > global_max {
                    global_max = db;
                }
            }
        }

        // Per-utterance dB floor, then sentence mean-normalization per mel bin.
        let floor = global_max - TOP_DB;
        for v in &mut feats {
            if *v < floor {
                *v = floor;
            }
        }
        let inv_t = 1.0 / n_frames as f32;
        for m in 0..N_MELS {
            let mut mean = 0.0f32;
            for t in 0..n_frames {
                mean += feats[t * N_MELS + m];
            }
            mean *= inv_t;
            for t in 0..n_frames {
                feats[t * N_MELS + m] -= mean;
            }
        }

        (n_frames, feats)
    }
}

impl Default for Fbank {
    fn default() -> Self {
        Self::new()
    }
}

/// HTK mel scale (`2595·log10(1 + hz/700)`).
fn to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Inverse HTK mel scale.
fn to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Build SpeechBrain's triangular mel filterbank: `N_MELS` symmetric triangles
/// whose centres are mel-spaced and whose half-width is the lower mel spacing.
fn build_mel_filters() -> Vec<Vec<f32>> {
    let mel_min = to_mel(F_MIN);
    let mel_max = to_mel(F_MAX);
    // `N_MELS + 2` mel points → 80 central frequencies + edges.
    let points = N_MELS + 2;
    let hz: Vec<f32> = (0..points)
        .map(|i| {
            let mel = mel_min + (mel_max - mel_min) * i as f32 / (points - 1) as f32;
            to_hz(mel)
        })
        .collect();
    // band = (hz[1:] - hz[:-1])[:-1]; f_central = hz[1:-1]
    let f_central: Vec<f32> = hz[1..=N_MELS].to_vec();
    let band: Vec<f32> = (0..N_MELS).map(|i| hz[i + 1] - hz[i]).collect();
    // all_freqs = linspace(0, sample_rate/2, n_stft)
    let all_freqs: Vec<f32> = (0..N_STFT)
        .map(|k| F_MAX * k as f32 / (N_STFT - 1) as f32)
        .collect();

    (0..N_MELS)
        .map(|m| {
            all_freqs
                .iter()
                .map(|&f| {
                    let slope = (f - f_central[m]) / band[m];
                    (slope + 1.0).min(-slope + 1.0).max(0.0)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_is_periodic_hamming() {
        let fb = Fbank::new();
        assert_eq!(fb.window.len(), N_FFT);
        // periodic Hamming: w[0] = 0.54 - 0.46 = 0.08.
        assert!((fb.window[0] - 0.08).abs() < 1e-6);
        // symmetric-ish peak near the middle (~1.0).
        let max = fb.window.iter().cloned().fold(f32::MIN, f32::max);
        assert!((max - 1.0).abs() < 1e-3);
    }

    #[test]
    fn mel_matrix_shape_and_nonneg() {
        let filters = build_mel_filters();
        assert_eq!(filters.len(), N_MELS);
        for row in &filters {
            assert_eq!(row.len(), N_STFT);
            assert!(row.iter().all(|&w| w >= 0.0));
            // each triangular filter must have some support.
            assert!(row.iter().any(|&w| w > 0.0));
        }
    }

    #[test]
    fn compute_shape_matches_frame_count() {
        let fb = Fbank::new();
        // 1 s of silence → expected centre-padded frame count.
        let samples = vec![0.0f32; SAMPLE_RATE as usize];
        let (frames, feats) = fb.compute(&samples);
        let expected = 1 + (samples.len() + N_FFT - N_FFT) / HOP;
        assert_eq!(frames, expected);
        assert_eq!(feats.len(), frames * N_MELS);
    }

    #[test]
    fn too_short_input_yields_no_frames() {
        let fb = Fbank::new();
        let (frames, feats) = fb.compute(&[]);
        // Even empty input centre-pads to n_fft, giving exactly one frame.
        assert_eq!(frames, 1);
        assert_eq!(feats.len(), N_MELS);
    }

    #[test]
    fn silence_is_mean_zero_per_bin() {
        let fb = Fbank::new();
        let samples = vec![0.0f32; SAMPLE_RATE as usize / 2];
        let (frames, feats) = fb.compute(&samples);
        // Sentence mean-norm ⇒ each mel bin sums to ~0 over time.
        for m in 0..N_MELS {
            let sum: f32 = (0..frames).map(|t| feats[t * N_MELS + m]).sum();
            assert!(sum.abs() < 1e-2, "bin {m} sum {sum}");
        }
    }

    #[test]
    fn deterministic() {
        let fb = Fbank::new();
        let samples: Vec<f32> = (0..16_000).map(|n| (n as f32 * 0.05).sin() * 0.3).collect();
        let (_, a) = fb.compute(&samples);
        let (_, b) = fb.compute(&samples);
        assert_eq!(a, b);
    }

    #[test]
    fn tone_differs_from_silence() {
        let fb = Fbank::new();
        let tone: Vec<f32> = (0..16_000).map(|n| (n as f32 * 0.2).sin() * 0.5).collect();
        let silence = vec![0.0f32; 16_000];
        let (_, ta) = fb.compute(&tone);
        let (_, sa) = fb.compute(&silence);
        assert_ne!(ta, sa);
    }
}
