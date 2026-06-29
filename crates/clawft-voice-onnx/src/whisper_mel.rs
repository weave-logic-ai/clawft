//! Whisper-compatible log-mel front-end (the smart-turn-v3 "input prep").
//!
//! smart-turn-v3 is a Whisper-Tiny encoder + a shallow linear head, so its
//! `input_features` tensor is exactly what HuggingFace `WhisperFeatureExtractor`
//! produces — but with the conversational twist baked into the reference
//! inference (`pipecat-ai/smart-turn`): the waveform is first cropped/padded to
//! the **last 8 s** and the extractor is built with `chunk_length=8`, so the
//! feature matrix is `[80, 800]` (8 s of 10 ms frames), not Whisper's 30 s /
//! `[80, 3000]`.
//!
//! This reproduces the extractor's pipeline faithfully so the native path
//! classifies the same as the Python reference:
//! 1. crop to the last `8·16000 = 128000` samples, else left-pad with zeros;
//! 2. `zero_mean_unit_var_norm` over the whole 8 s window (`do_normalize=True`);
//! 3. reflect-pad by `n_fft/2`, periodic-Hann STFT (`n_fft=400`, `hop=160`),
//!    power spectrum, **Slaney**-scale 80-bin mel filterbank with Slaney area
//!    normalization, `log10`, drop the trailing frame (`[:, :-1]`);
//! 4. `log_spec = max(log_spec, max−8)`, then `(log_spec + 4) / 4`.
//!
//! Pure math, no model dependency — always compiled and unit-tested. Note the
//! **Slaney** mel scale/normalization here is deliberately different from the
//! HTK mel in [`crate::fbank`] (ECAPA): the two front-ends are not
//! interchangeable.

use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// Sample rate the extractor (and the model) expect.
pub const SAMPLE_RATE: u32 = 16_000;
/// STFT size (25 ms at 16 kHz).
pub const N_FFT: usize = 400;
/// Hop between frames (10 ms at 16 kHz).
pub const HOP: usize = 160;
/// Mel feature dimension (Whisper-Tiny encoder → 80).
pub const N_MELS: usize = 80;
/// Context window the smart-turn reference crops/pads to (8 s).
pub const CHUNK_SECONDS: usize = 8;
/// Window length in samples (`8 · 16000`).
pub const N_SAMPLES: usize = CHUNK_SECONDS * SAMPLE_RATE as usize;
/// Number of output frames after dropping the trailing STFT frame (`= 800`).
pub const N_FRAMES: usize = N_SAMPLES / HOP;

/// One-sided spectrum bin count (`n_fft/2 + 1`).
const N_STFT: usize = N_FFT / 2 + 1;
const MEL_FLOOR: f32 = 1e-10;

/// Whisper log-mel extractor. Build once (precomputes the Hann window and the
/// Slaney mel matrix, plans the FFT), then call [`WhisperMel::input_features`].
pub struct WhisperMel {
    /// Periodic Hann window, length [`N_FFT`].
    window: Vec<f32>,
    /// Slaney triangular mel filters, `N_MELS` rows of `N_STFT` weights each.
    mel_filters: Vec<Vec<f32>>,
    fft: Arc<dyn Fft<f32>>,
}

impl WhisperMel {
    /// Build the front-end with the pinned Whisper configuration.
    pub fn new() -> Self {
        let window = (0..N_FFT)
            .map(|n| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / N_FFT as f32).cos())
            .collect();
        let fft = FftPlanner::<f32>::new().plan_fft_forward(N_FFT);
        Self {
            window,
            mel_filters: build_slaney_mel_filters(),
            fft,
        }
    }

    /// Full reference pipeline: arbitrary-length 16 kHz mono PCM in `[-1, 1]` →
    /// row-major `[N_MELS · N_FRAMES]` `input_features` (mel-major: bin `m`,
    /// frame `t` at index `m * N_FRAMES + t`), ready for the ONNX graph.
    pub fn input_features(&self, samples: &[f32]) -> Vec<f32> {
        let mut win = crop_or_pad_8s(samples);
        zero_mean_unit_var(&mut win);
        self.log_mel(&win)
    }

    /// Steps 3–4 on an already 8 s-windowed, normalized waveform.
    fn log_mel(&self, samples: &[f32]) -> Vec<f32> {
        debug_assert_eq!(samples.len(), N_SAMPLES);
        let padded = reflect_pad(samples, N_FFT / 2);
        // `1 + (len - n_fft)/hop` STFT frames; we keep the first `N_FRAMES`
        // (the reference drops the trailing frame via `[:, :-1]`).
        let total_frames = 1 + (padded.len() - N_FFT) / HOP;
        let keep = total_frames.min(N_FRAMES);

        let mut feats = vec![MEL_FLOOR.log10(); N_MELS * N_FRAMES];
        let mut buf = vec![Complex::new(0.0f32, 0.0f32); N_FFT];
        let mut power = vec![0.0f32; N_STFT];
        let mut global_max = f32::MIN;

        for t in 0..keep {
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
                let db = acc.max(MEL_FLOOR).log10();
                feats[m * N_FRAMES + t] = db;
            }
        }
        for &v in &feats {
            if v > global_max {
                global_max = v;
            }
        }

        // `max(log_spec, max − 8)`, then `(log_spec + 4) / 4`.
        let floor = global_max - 8.0;
        for v in &mut feats {
            if *v < floor {
                *v = floor;
            }
            *v = (*v + 4.0) / 4.0;
        }
        feats
    }
}

impl Default for WhisperMel {
    fn default() -> Self {
        Self::new()
    }
}

/// Crop to the last [`N_SAMPLES`] samples, or left-pad with zeros to reach it
/// (mirrors `truncate_audio_to_last_n_seconds` in the smart-turn reference).
pub fn crop_or_pad_8s(samples: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0f32; N_SAMPLES];
    if samples.len() >= N_SAMPLES {
        out.copy_from_slice(&samples[samples.len() - N_SAMPLES..]);
    } else {
        // Left-pad: zeros first, audio flush to the end.
        out[N_SAMPLES - samples.len()..].copy_from_slice(samples);
    }
    out
}

/// `(x − mean) / sqrt(var + 1e-7)` over the whole slice (numpy population
/// variance, `ddof=0`) — Whisper's `zero_mean_unit_var_norm` with a full mask.
pub fn zero_mean_unit_var(x: &mut [f32]) {
    if x.is_empty() {
        return;
    }
    let n = x.len() as f64;
    let mean = x.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = x.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
    let inv = 1.0 / (var + 1e-7).sqrt();
    for v in x.iter_mut() {
        *v = ((*v as f64 - mean) * inv) as f32;
    }
}

/// numpy `np.pad(x, p, mode="reflect")` (edge sample not repeated). Requires
/// `x.len() > p`.
fn reflect_pad(x: &[f32], p: usize) -> Vec<f32> {
    let n = x.len();
    let mut out = vec![0.0f32; n + 2 * p];
    for k in 0..p {
        out[k] = x[p - k]; // x[p], x[p-1], …, x[1]
    }
    out[p..p + n].copy_from_slice(x);
    for k in 0..p {
        out[p + n + k] = x[n - 2 - k]; // x[n-2], x[n-3], …
    }
    out
}

/// Slaney mel scale: linear below 1 kHz, log above (librosa / HF `slaney`).
fn hertz_to_mel(hz: f32) -> f32 {
    const MIN_LOG_HZ: f32 = 1000.0;
    const MIN_LOG_MEL: f32 = 15.0; // (1000 − 0) / (200/3)
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

/// HF `mel_filter_bank(201, 80, 0, 8000, 16000, norm="slaney", mel_scale="slaney")`
/// → `N_MELS` rows of `N_STFT` triangular, area-normalized weights.
fn build_slaney_mel_filters() -> Vec<Vec<f32>> {
    let mel_min = hertz_to_mel(0.0);
    let mel_max = hertz_to_mel(SAMPLE_RATE as f32 / 2.0);
    let points = N_MELS + 2;
    let filter_freqs: Vec<f32> = (0..points)
        .map(|i| {
            let mel = mel_min + (mel_max - mel_min) * i as f32 / (points - 1) as f32;
            mel_to_hertz(mel)
        })
        .collect();
    let fft_freqs: Vec<f32> = (0..N_STFT)
        .map(|k| (SAMPLE_RATE as f32 / 2.0) * k as f32 / (N_STFT - 1) as f32)
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
    fn window_is_periodic_hann() {
        let w = WhisperMel::new();
        assert_eq!(w.window.len(), N_FFT);
        // periodic Hann: w[0] = 0.
        assert!(w.window[0].abs() < 1e-6);
        let max = w.window.iter().cloned().fold(f32::MIN, f32::max);
        assert!((max - 1.0).abs() < 1e-3);
    }

    #[test]
    fn slaney_mel_roundtrips_and_anchors() {
        // Slaney: linear region passes through, 1 kHz is the 15-mel knee.
        assert!((hertz_to_mel(0.0)).abs() < 1e-6);
        assert!((hertz_to_mel(1000.0) - 15.0).abs() < 1e-4);
        for hz in [0.0f32, 250.0, 999.0, 1000.0, 4000.0, 8000.0] {
            assert!(
                (mel_to_hertz(hertz_to_mel(hz)) - hz).abs() < 1e-1,
                "hz {hz}"
            );
        }
    }

    #[test]
    fn mel_matrix_shape_and_support() {
        let f = build_slaney_mel_filters();
        assert_eq!(f.len(), N_MELS);
        for row in &f {
            assert_eq!(row.len(), N_STFT);
            assert!(row.iter().all(|&w| w >= 0.0));
            assert!(row.iter().any(|&w| w > 0.0));
        }
    }

    #[test]
    fn crop_or_pad_is_exactly_8s() {
        assert_eq!(crop_or_pad_8s(&[]).len(), N_SAMPLES);
        // Short input lands flush at the end (left-padded with zeros).
        let short = vec![0.7f32; 1000];
        let p = crop_or_pad_8s(&short);
        assert_eq!(p.len(), N_SAMPLES);
        assert_eq!(p[N_SAMPLES - 1], 0.7);
        assert_eq!(p[N_SAMPLES - 1000], 0.7);
        assert_eq!(p[N_SAMPLES - 1001], 0.0);
        // Long input keeps the tail.
        let long: Vec<f32> = (0..N_SAMPLES + 500).map(|i| i as f32).collect();
        let c = crop_or_pad_8s(&long);
        assert_eq!(c[N_SAMPLES - 1], (N_SAMPLES + 499) as f32);
        assert_eq!(c[0], 500.0);
    }

    #[test]
    fn zero_mean_unit_var_normalizes() {
        let mut x: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin()).collect();
        zero_mean_unit_var(&mut x);
        let n = x.len() as f64;
        let mean = x.iter().map(|&v| v as f64).sum::<f64>() / n;
        let var = x.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
        assert!(mean.abs() < 1e-5, "mean {mean}");
        assert!((var - 1.0).abs() < 1e-3, "var {var}");
    }

    #[test]
    fn reflect_pad_matches_numpy() {
        // np.pad([0,1,2,3,4], 2, "reflect") == [2,1,0,1,2,3,4,3,2]
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(
            reflect_pad(&x, 2),
            vec![2.0, 1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0]
        );
    }

    #[test]
    fn input_features_shape_and_finite() {
        let mel = WhisperMel::new();
        let tone: Vec<f32> = (0..16_000).map(|n| (n as f32 * 0.1).sin() * 0.3).collect();
        let feats = mel.input_features(&tone);
        assert_eq!(feats.len(), N_MELS * N_FRAMES);
        assert!(feats.iter().all(|v| v.is_finite()));
        // Post-scale range is roughly [-1, 1.x]; silence floor maps to ~ -1.
        assert!(feats.iter().cloned().fold(f32::MAX, f32::min) >= -1.5);
    }

    #[test]
    fn tone_differs_from_silence() {
        let mel = WhisperMel::new();
        let tone: Vec<f32> = (0..128_000).map(|n| (n as f32 * 0.2).sin() * 0.5).collect();
        let silence = vec![0.0f32; 128_000];
        assert_ne!(mel.input_features(&tone), mel.input_features(&silence));
    }

    #[test]
    fn deterministic() {
        let mel = WhisperMel::new();
        let s: Vec<f32> = (0..40_000).map(|n| (n as f32 * 0.05).sin() * 0.3).collect();
        assert_eq!(mel.input_features(&s), mel.input_features(&s));
    }
}
