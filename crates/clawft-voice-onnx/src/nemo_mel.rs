//! NeMo `AudioToMelSpectrogramPreprocessor`-compatible front-end (the parakeet
//! "input prep").
//!
//! parakeet-tdt-0.6b exports **only the transducer branch** (encoder/decoder/
//! joiner) — the mel front-end is *not* in the graph, so the caller must feed
//! the encoder a `[128, T]` log-mel feature matrix that matches NeMo's
//! `FilterbankFeatures` exactly, or the acoustic model sees out-of-distribution
//! input and decodes garbage. This reproduces that pipeline:
//! 1. pre-emphasis `x[n] − 0.97·x[n−1]` (NeMo `preemph`, `x[0]` kept);
//! 2. reflect-pad by `n_fft/2`, centered Hann STFT (`n_fft=512`, `win=400`,
//!    `hop=160`), power spectrum (`|X|²`, `mag_power=2`);
//! 3. **Slaney** (librosa) 128-bin mel filterbank, 0–8000 Hz, area-normalized;
//! 4. `log(mel + 2⁻²⁴)` (`log_zero_guard_type="add"`);
//! 5. `per_feature` normalization: per mel bin, `(x − mean) / (std + 1e-5)`
//!    over time (unbiased std, matching `torch.std`).
//!
//! Output is mel-major `[N_MELS · n_frames]` (bin `m`, frame `t` at
//! `m * n_frames + t`), i.e. the `[1, 128, T]` `audio_signal` the encoder wants.
//!
//! Pure math, no model dependency — always compiled and unit-tested.

use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// Sample rate the front-end (and the model) expect.
pub const SAMPLE_RATE: u32 = 16_000;
/// FFT size (next power of two ≥ `WIN_LENGTH`; NeMo `round_to_power_of_two`).
pub const N_FFT: usize = 512;
/// Analysis window length in samples (25 ms at 16 kHz).
pub const WIN_LENGTH: usize = 400;
/// Hop between frames (10 ms at 16 kHz).
pub const HOP: usize = 160;
/// Mel feature dimension (`feat_dim` in the model metadata).
pub const N_MELS: usize = 128;

/// One-sided spectrum bin count (`n_fft/2 + 1`).
const N_FREQ: usize = N_FFT / 2 + 1;
const PREEMPH: f32 = 0.97;
/// `log_zero_guard_value = 2⁻²⁴`.
const LOG_GUARD: f32 = 5.960_464_5e-8;
/// `per_feature` normalization epsilon added to the std.
const NORM_EPS: f32 = 1e-5;
const F_MIN: f32 = 0.0;
const F_MAX: f32 = 8_000.0;

/// NeMo log-mel extractor. Build once (precomputes the centered Hann window and
/// the Slaney mel matrix, plans the FFT), then call [`NemoMel::compute`].
pub struct NemoMel {
    /// Hann window of length [`WIN_LENGTH`], centered in an [`N_FFT`] buffer.
    window: Vec<f32>,
    /// Slaney triangular mel filters, `N_MELS` rows of `N_FREQ` weights each.
    mel_filters: Vec<Vec<f32>>,
    fft: Arc<dyn Fft<f32>>,
}

impl NemoMel {
    /// Build the front-end with the pinned NeMo configuration.
    pub fn new() -> Self {
        // torch.stft centers a `win_length` window inside an `n_fft` buffer.
        let pad = (N_FFT - WIN_LENGTH) / 2;
        let mut window = vec![0.0f32; N_FFT];
        for n in 0..WIN_LENGTH {
            window[pad + n] =
                0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / WIN_LENGTH as f32).cos();
        }
        let fft = FftPlanner::<f32>::new().plan_fft_forward(N_FFT);
        Self {
            window,
            mel_filters: build_slaney_mel_filters(),
            fft,
        }
    }

    /// Compute NeMo log-mel features for 16 kHz mono PCM in `[-1, 1]`.
    ///
    /// Returns the frame count and a mel-major `[N_MELS · n_frames]` buffer. An
    /// utterance too short to frame yields `(0, vec![])`.
    pub fn compute(&self, samples: &[f32]) -> (usize, Vec<f32>) {
        if samples.len() <= N_FFT / 2 {
            return (0, Vec::new());
        }
        // 1. pre-emphasis (x[0] preserved).
        let mut pre = vec![0.0f32; samples.len()];
        pre[0] = samples[0];
        for n in 1..samples.len() {
            pre[n] = samples[n] - PREEMPH * samples[n - 1];
        }
        // 2. center reflect-pad by n_fft/2.
        let padded = reflect_pad(&pre, N_FFT / 2);
        let n_frames = 1 + (padded.len() - N_FFT) / HOP;

        let mut feats = vec![0.0f32; N_MELS * n_frames];
        let mut buf = vec![Complex::new(0.0f32, 0.0f32); N_FFT];
        let mut power = vec![0.0f32; N_FREQ];

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
                for k in 0..N_FREQ {
                    acc += power[k] * row[k];
                }
                feats[m * n_frames + t] = (acc + LOG_GUARD).ln();
            }
        }

        // 5. per_feature normalization (per mel bin, over time, unbiased std).
        normalize_per_feature(&mut feats, N_MELS, n_frames);
        (n_frames, feats)
    }
}

impl Default for NemoMel {
    fn default() -> Self {
        Self::new()
    }
}

/// `(x − mean) / (std + 1e-5)` per row (mel bin) over `n_frames` columns, with
/// unbiased (`ddof=1`) variance to match `torch.std`.
fn normalize_per_feature(feats: &mut [f32], n_mels: usize, n_frames: usize) {
    if n_frames < 2 {
        return;
    }
    let inv_n = 1.0 / n_frames as f64;
    let inv_n1 = 1.0 / (n_frames - 1) as f64;
    for m in 0..n_mels {
        let row = &mut feats[m * n_frames..(m + 1) * n_frames];
        let mean = row.iter().map(|&v| v as f64).sum::<f64>() * inv_n;
        let var = row.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() * inv_n1;
        let inv_std = 1.0 / (var.sqrt() + NORM_EPS as f64);
        for v in row.iter_mut() {
            *v = ((*v as f64 - mean) * inv_std) as f32;
        }
    }
}

/// numpy/torch `reflect` pad (edge sample not repeated). Requires `x.len() > p`.
fn reflect_pad(x: &[f32], p: usize) -> Vec<f32> {
    let n = x.len();
    let mut out = vec![0.0f32; n + 2 * p];
    for k in 0..p {
        out[k] = x[p - k];
    }
    out[p..p + n].copy_from_slice(x);
    for k in 0..p {
        out[p + n + k] = x[n - 2 - k];
    }
    out
}

/// Slaney mel scale: linear below 1 kHz, log above (librosa `htk=False`).
fn hertz_to_mel(hz: f32) -> f32 {
    const MIN_LOG_HZ: f32 = 1000.0;
    const MIN_LOG_MEL: f32 = 15.0;
    let logstep = 6.4f32.ln() / 27.0;
    if hz >= MIN_LOG_HZ {
        MIN_LOG_MEL + (hz / MIN_LOG_HZ).ln() / logstep
    } else {
        3.0 * hz / 200.0
    }
}

/// Inverse Slaney mel scale.
fn mel_to_hertz(mel: f32) -> f32 {
    const MIN_LOG_HZ: f32 = 1000.0;
    const MIN_LOG_MEL: f32 = 15.0;
    let logstep = 6.4f32.ln() / 27.0;
    if mel >= MIN_LOG_MEL {
        MIN_LOG_HZ * (logstep * (mel - MIN_LOG_MEL)).exp()
    } else {
        200.0 * mel / 3.0
    }
}

/// librosa `mel(16000, 512, 128, fmin=0, fmax=8000, norm="slaney")` →
/// `N_MELS` rows of `N_FREQ` triangular, area-normalized weights.
fn build_slaney_mel_filters() -> Vec<Vec<f32>> {
    let mel_min = hertz_to_mel(F_MIN);
    let mel_max = hertz_to_mel(F_MAX);
    let points = N_MELS + 2;
    let filter_freqs: Vec<f32> = (0..points)
        .map(|i| mel_to_hertz(mel_min + (mel_max - mel_min) * i as f32 / (points - 1) as f32))
        .collect();
    let fft_freqs: Vec<f32> = (0..N_FREQ)
        .map(|k| (SAMPLE_RATE as f32 / 2.0) * k as f32 / (N_FREQ - 1) as f32)
        .collect();

    (0..N_MELS)
        .map(|m| {
            let lo = filter_freqs[m];
            let ctr = filter_freqs[m + 1];
            let hi = filter_freqs[m + 2];
            let enorm = 2.0 / (hi - lo);
            fft_freqs
                .iter()
                .map(|&f| {
                    let down = (f - lo) / (ctr - lo);
                    let up = (hi - f) / (hi - ctr);
                    down.min(up).max(0.0) * enorm
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_is_centered_hann() {
        let m = NemoMel::new();
        assert_eq!(m.window.len(), N_FFT);
        let pad = (N_FFT - WIN_LENGTH) / 2;
        // Zero-padding flanks, periodic Hann inside (w[0 of win] = 0).
        assert!(m.window[pad - 1].abs() < 1e-9);
        assert!(m.window[pad].abs() < 1e-6);
        let max = m.window.iter().cloned().fold(f32::MIN, f32::max);
        assert!((max - 1.0).abs() < 1e-3);
    }

    #[test]
    fn mel_matrix_shape_and_support() {
        let f = build_slaney_mel_filters();
        assert_eq!(f.len(), N_MELS);
        for row in &f {
            assert_eq!(row.len(), N_FREQ);
            assert!(row.iter().all(|&w| w >= 0.0));
            assert!(row.iter().any(|&w| w > 0.0));
        }
    }

    #[test]
    fn slaney_anchors() {
        assert!(hertz_to_mel(0.0).abs() < 1e-6);
        assert!((hertz_to_mel(1000.0) - 15.0).abs() < 1e-4);
        assert!((mel_to_hertz(hertz_to_mel(4000.0)) - 4000.0).abs() < 1e-1);
    }

    #[test]
    fn compute_shape_and_finite() {
        let m = NemoMel::new();
        let s: Vec<f32> = (0..16_000).map(|n| (n as f32 * 0.1).sin() * 0.3).collect();
        let (frames, feats) = m.compute(&s);
        assert_eq!(frames, 1 + s.len() / HOP);
        assert_eq!(feats.len(), N_MELS * frames);
        assert!(feats.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn per_feature_norm_zero_mean() {
        let m = NemoMel::new();
        let s: Vec<f32> = (0..16_000).map(|n| (n as f32 * 0.05).sin() * 0.4).collect();
        let (frames, feats) = m.compute(&s);
        // Each mel bin is normalized to ~zero mean over time.
        for bin in 0..N_MELS {
            let row = &feats[bin * frames..(bin + 1) * frames];
            let mean: f32 = row.iter().sum::<f32>() / frames as f32;
            assert!(mean.abs() < 1e-2, "bin {bin} mean {mean}");
        }
    }

    #[test]
    fn too_short_yields_no_frames() {
        let m = NemoMel::new();
        assert_eq!(m.compute(&[0.0; 10]), (0, Vec::new()));
    }

    #[test]
    fn deterministic() {
        let m = NemoMel::new();
        let s: Vec<f32> = (0..20_000).map(|n| (n as f32 * 0.03).sin() * 0.3).collect();
        assert_eq!(m.compute(&s), m.compute(&s));
    }

    #[test]
    fn reflect_pad_matches_numpy() {
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(
            reflect_pad(&x, 2),
            vec![2.0, 1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0]
        );
    }
}
