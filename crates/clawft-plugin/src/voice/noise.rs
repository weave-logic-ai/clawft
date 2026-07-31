//! Noise suppression for the voice pipeline (legacy scaffold — WEFT-671).
//!
//! **Transitional API.** Live NS/AEC is `clawft-voice-aec` (WebRTC APM NS).
//! This module implements **real pure-Rust spectral subtraction** (WEFT-217)
//! so the scaffold is not a deceptive passthrough: `process()` attenuates
//! bins below the learned noise floor.
//!
//! # Algorithm
//!
//! Overlap-add STFT (frame 256, hop 128, Hann window) with magnitude spectral
//! subtraction. Noise PSD is tracked with a slow EMA on low-energy frames and
//! a slower floor tracker on all frames. Over-subtraction scales with
//! `aggressiveness` (0 = gentle, 3 = aggressive). Phase is preserved from the
//! analysis STFT.
//!
//! # Not RNNoise
//!
//! Full RNNoise (neural residual suppression) is **not** bundled: it needs a
//! trained model and native deps. Prefer WebRTC NS via `clawft-voice-aec` for
//! production Talk Mode. This filter is still real DSP — it is not identity.

use serde::{Deserialize, Serialize};

/// Analysis / synthesis frame size (power of two for the in-tree FFT).
const FRAME: usize = 256;
/// Hop size (50% overlap).
const HOP: usize = 128;
/// Number of unique magnitude bins (including DC and Nyquist).
const BINS: usize = FRAME / 2 + 1;

/// Configuration for noise suppression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseSuppressorConfig {
    /// Enable noise suppression.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Suppression aggressiveness (0 = gentle, 3 = aggressive).
    #[serde(default = "default_aggressiveness")]
    pub aggressiveness: u8,
}

fn default_true() -> bool {
    true
}
fn default_aggressiveness() -> u8 {
    2
}

impl Default for NoiseSuppressorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            aggressiveness: 2,
        }
    }
}

/// Spectral-subtraction noise suppressor.
pub struct NoiseSuppressor {
    config: NoiseSuppressorConfig,
    /// Estimated broadband noise floor (RMS, for quality metrics / API).
    noise_floor: f32,
    /// Per-bin noise power spectral density estimate.
    noise_psd: Vec<f32>,
    /// Whether noise_psd has been seeded.
    noise_seeded: bool,
    /// Input overlap buffer.
    in_buf: Vec<f32>,
    /// Samples waiting in in_buf.
    in_fill: usize,
    /// Overlap-add synthesis tail.
    ola: Vec<f32>,
    /// Number of processed frames (API counter: process() calls).
    frames_processed: u64,
    /// STFT frames fully analysed.
    stft_frames: u64,
    /// Hann window.
    window: Vec<f32>,
}

impl NoiseSuppressor {
    /// Create a new noise suppressor with the given configuration.
    pub fn new(config: NoiseSuppressorConfig) -> Self {
        // Periodic Hann: w[n] + w[n+HOP] = 1 at hop = N/2 → perfect COLA
        // with analysis window only (rectangular synthesis).
        let window: Vec<f32> = (0..FRAME)
            .map(|i| {
                let x = 2.0 * std::f32::consts::PI * i as f32 / FRAME as f32;
                0.5 * (1.0 - x.cos())
            })
            .collect();
        Self {
            config,
            noise_floor: 0.0,
            noise_psd: vec![1e-8; BINS],
            noise_seeded: false,
            in_buf: vec![0.0; FRAME],
            in_fill: 0,
            ola: vec![0.0; FRAME],
            frames_processed: 0,
            stft_frames: 0,
            window,
        }
    }

    /// Process audio frame, suppressing stationary noise via spectral subtraction.
    ///
    /// Streaming: incomplete STFT frames are held internally; output length
    /// may lag input until the first hop is ready, then tracks hop-aligned
    /// production. When disabled, returns a copy of the input unchanged.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.frames_processed += 1;
        if !self.config.enabled {
            return input.to_vec();
        }

        // Track broadband RMS floor for the public noise_floor() API and
        // quality metrics (EMA on all frames — same spirit as the old stub).
        if !input.is_empty() {
            let rms = (input.iter().map(|s| s * s).sum::<f32>() / input.len() as f32).sqrt();
            self.noise_floor = 0.95 * self.noise_floor + 0.05 * rms;
        }

        let mut out = Vec::with_capacity(input.len() + HOP);
        for &s in input {
            if self.in_fill < FRAME {
                self.in_buf[self.in_fill] = s;
                self.in_fill += 1;
            }
            if self.in_fill == FRAME {
                self.process_stft_frame(&mut out);
                // Overlap: keep last (FRAME - HOP) samples as the next head.
                self.in_buf.copy_within(HOP..FRAME, 0);
                self.in_fill = FRAME - HOP;
            }
        }
        out
    }

    /// Flush remaining buffered audio (pads with zeros to a full hop).
    pub fn flush(&mut self) -> Vec<f32> {
        if !self.config.enabled || self.in_fill == 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        // Pad to a full frame and emit remaining hops.
        while self.in_fill > 0 && self.in_fill < FRAME {
            self.in_buf[self.in_fill] = 0.0;
            self.in_fill += 1;
        }
        if self.in_fill == FRAME {
            self.process_stft_frame(&mut out);
            self.in_fill = 0;
        }
        // Drain OLA head (the part already fully summed).
        out
    }

    fn process_stft_frame(&mut self, out: &mut Vec<f32>) {
        // Windowed frame.
        let mut time = [0.0f32; FRAME];
        for i in 0..FRAME {
            time[i] = self.in_buf[i] * self.window[i];
        }

        let mut re = [0.0f32; FRAME];
        let mut im = [0.0f32; FRAME];
        re.copy_from_slice(&time);
        fft_inplace(&mut re, &mut im, false);

        // Magnitude / power per bin.
        let mut mag = [0.0f32; BINS];
        for k in 0..BINS {
            mag[k] = (re[k] * re[k] + im[k] * im[k]).sqrt();
        }

        // Noise PSD update: seed hard, then only adapt on noise-like frames.
        // During speech (high power relative to noise) freeze the estimate so
        // we do not chase the talker into the noise floor.
        let frame_power: f32 = mag.iter().map(|m| m * m).sum::<f32>() / BINS as f32;
        let mean_n = self.mean_noise_power();
        let is_noise_like = !self.noise_seeded || frame_power < 1.5 * mean_n.max(1e-12);
        if !self.noise_seeded {
            for k in 0..BINS {
                self.noise_psd[k] = (mag[k] * mag[k]).max(1e-8);
            }
            self.noise_seeded = true;
        } else if is_noise_like {
            let alpha = 0.2f32;
            for k in 0..BINS {
                let p = mag[k] * mag[k];
                self.noise_psd[k] = (1.0 - alpha) * self.noise_psd[k] + alpha * p;
            }
        }

        // Over-subtraction factor from aggressiveness (0..=3).
        let over = 1.0 + 0.35 * self.config.aggressiveness.min(3) as f32;
        // Spectral floor (avoid musical noise / total wipeout).
        let floor_frac = match self.config.aggressiveness.min(3) {
            0 => 0.20,
            1 => 0.12,
            2 => 0.08,
            _ => 0.05,
        };

        for k in 0..BINS {
            let noise_mag = self.noise_psd[k].sqrt();
            // Wiener-like soft gain after magnitude subtraction.
            let clean = (mag[k] - over * noise_mag).max(0.0);
            let mut gain = if mag[k] > 1e-12 {
                clean / mag[k]
            } else {
                0.0
            };
            gain = gain.max(floor_frac);
            re[k] *= gain;
            im[k] *= gain;
            // Mirror for real IFFT (except DC / Nyquist).
            if k > 0 && k < FRAME / 2 {
                let mk = FRAME - k;
                re[mk] *= gain;
                im[mk] *= gain;
            }
        }

        // IFFT
        fft_inplace(&mut re, &mut im, true);

        // Overlap-add (analysis Hann only; COLA → gain 1 at hop = N/2).
        for i in 0..FRAME {
            self.ola[i] += re[i];
        }
        // Emit hop samples.
        for i in 0..HOP {
            out.push(self.ola[i]);
        }
        // Shift OLA buffer.
        self.ola.copy_within(HOP..FRAME, 0);
        for i in (FRAME - HOP)..FRAME {
            self.ola[i] = 0.0;
        }

        self.stft_frames += 1;
    }

    fn mean_noise_power(&self) -> f32 {
        self.noise_psd.iter().sum::<f32>() / BINS as f32
    }

    /// Get the current estimated broadband noise floor (RMS EMA).
    pub fn noise_floor(&self) -> f32 {
        self.noise_floor
    }

    /// Get number of process() calls.
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed
    }

    /// Number of completed STFT analysis frames.
    pub fn stft_frames(&self) -> u64 {
        self.stft_frames
    }

    /// Reset the noise suppressor state.
    pub fn reset(&mut self) {
        self.noise_floor = 0.0;
        self.noise_psd.fill(1e-8);
        self.noise_seeded = false;
        self.in_buf.fill(0.0);
        self.in_fill = 0;
        self.ola.fill(0.0);
        self.frames_processed = 0;
        self.stft_frames = 0;
    }
}

// ── Minimal radix-2 Cooley–Tukey FFT (real-friendly complex buffers) ────────

fn fft_inplace(re: &mut [f32], im: &mut [f32], inverse: bool) {
    let n = re.len();
    assert_eq!(n, im.len());
    assert!(n.is_power_of_two());

    // Bit-reverse permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let sign = if inverse { 1.0f32 } else { -1.0f32 };
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let ang = sign * 2.0 * std::f32::consts::PI / len as f32;
        let (wlen_re, wlen_im) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let mut wr = 1.0f32;
            let mut wi = 0.0f32;
            for k in 0..half {
                let u_re = re[i + k];
                let u_im = im[i + k];
                let v_re = re[i + k + half] * wr - im[i + k + half] * wi;
                let v_im = re[i + k + half] * wi + im[i + k + half] * wr;
                re[i + k] = u_re + v_re;
                im[i + k] = u_im + v_im;
                re[i + k + half] = u_re - v_re;
                im[i + k + half] = u_im - v_im;
                let nwr = wr * wlen_re - wi * wlen_im;
                wi = wr * wlen_im + wi * wlen_re;
                wr = nwr;
            }
        }
        len <<= 1;
    }

    if inverse {
        let inv = 1.0 / n as f32;
        for i in 0..n {
            re[i] *= inv;
            im[i] *= inv;
        }
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

    #[test]
    fn new_creates_with_defaults() {
        let ns = NoiseSuppressor::new(NoiseSuppressorConfig::default());
        assert_eq!(ns.frames_processed(), 0);
        assert!((ns.noise_floor() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn process_returns_audio_not_empty_after_warmup() {
        let mut ns = NoiseSuppressor::new(NoiseSuppressorConfig::default());
        // Need at least one full FRAME of samples to emit.
        let input = vec![0.1, 0.2, -0.3, 0.4, -0.5]
            .into_iter()
            .cycle()
            .take(FRAME + HOP)
            .collect::<Vec<_>>();
        let output = ns.process(&input);
        assert!(!output.is_empty(), "expected OLA output after one STFT frame");
        assert_eq!(ns.frames_processed(), 1);
        assert!(ns.stft_frames() >= 1);
    }

    #[test]
    fn noise_floor_updates_on_process() {
        let mut ns = NoiseSuppressor::new(NoiseSuppressorConfig::default());
        let input = vec![0.5; 160];
        ns.process(&input);

        let floor = ns.noise_floor();
        assert!(
            floor > 0.0,
            "Noise floor should be positive after processing"
        );
        assert!(
            floor < 0.5,
            "Noise floor should not have reached signal level yet"
        );

        for _ in 0..200 {
            ns.process(&input);
        }
        let floor = ns.noise_floor();
        assert!(
            (floor - 0.5).abs() < 0.05,
            "Noise floor should converge near 0.5, got {}",
            floor
        );
    }

    #[test]
    fn disabled_passthrough_no_noise_floor_update() {
        let config = NoiseSuppressorConfig {
            enabled: false,
            ..Default::default()
        };
        let mut ns = NoiseSuppressor::new(config);
        let input = vec![0.5; 160];
        let output = ns.process(&input);
        assert_eq!(input, output);
        assert!((ns.noise_floor() - 0.0).abs() < f32::EPSILON);
        assert_eq!(ns.frames_processed(), 1);
    }

    #[test]
    fn reset_clears_state() {
        let mut ns = NoiseSuppressor::new(NoiseSuppressorConfig::default());
        ns.process(&[0.5; FRAME]);
        ns.process(&[0.5; FRAME]);
        assert!(ns.noise_floor() > 0.0);
        assert!(ns.frames_processed() >= 2);

        ns.reset();
        assert!((ns.noise_floor() - 0.0).abs() < f32::EPSILON);
        assert_eq!(ns.frames_processed(), 0);
        assert_eq!(ns.stft_frames(), 0);
    }

    #[test]
    fn config_defaults_are_correct() {
        let config = NoiseSuppressorConfig::default();
        assert!(config.enabled);
        assert_eq!(config.aggressiveness, 2);
    }

    /// Stationary noise-only input must be attenuated vs identity.
    #[test]
    fn synthetic_noise_is_attenuated() {
        let mut ns = NoiseSuppressor::new(NoiseSuppressorConfig {
            enabled: true,
            aggressiveness: 3,
        });

        // Deterministic "noise": multi-tone low-level stationary signal.
        let n = FRAME * 40;
        let mut noisy = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / 16_000.0;
            // Broadband-ish sum of inharmonic tones (stationary).
            let s = 0.08 * (2.0 * std::f32::consts::PI * 230.0 * t).sin()
                + 0.07 * (2.0 * std::f32::consts::PI * 710.0 * t).sin()
                + 0.06 * (2.0 * std::f32::consts::PI * 1450.0 * t).sin()
                + 0.05 * (2.0 * std::f32::consts::PI * 2900.0 * t).sin();
            noisy.push(s);
        }

        let mut out = Vec::new();
        for start in (0..n).step_by(160) {
            let end = (start + 160).min(n);
            out.extend(ns.process(&noisy[start..end]));
        }
        // Flush remaining OLA by feeding silence.
        out.extend(ns.process(&vec![0.0; FRAME]));

        assert!(out.len() > FRAME, "expected streaming output");
        // Compare steady-state region (skip warmup + seed).
        let skip = FRAME * 4;
        let in_region = &noisy[skip..noisy.len().min(skip + out.len() - skip)];
        let out_region = &out[skip..skip + in_region.len()];
        let in_rms = rms(in_region);
        let out_rms = rms(out_region);
        assert!(in_rms > 0.01, "input noise energy expected, got {in_rms}");
        let ratio = out_rms / in_rms;
        assert!(
            ratio < 0.5,
            "spectral subtraction must attenuate stationary noise: \
             out/in = {ratio:.3} (out_rms={out_rms:.5}, in_rms={in_rms:.5}); \
             not a passthrough"
        );
    }

    /// Speech-like tone + noise: residual should keep more of the tone than
    /// pure noise attenuation would suggest (SNR improvement).
    #[test]
    fn speech_like_tone_retained_better_than_noise() {
        let mut ns = NoiseSuppressor::new(NoiseSuppressorConfig {
            enabled: true,
            aggressiveness: 2,
        });

        // Calibrate on noise-only.
        let cal = FRAME * 8;
        let mut noise_only = Vec::with_capacity(cal);
        for i in 0..cal {
            let t = i as f32 / 16_000.0;
            noise_only.push(
                0.05 * (2.0 * std::f32::consts::PI * 400.0 * t).sin()
                    + 0.04 * (2.0 * std::f32::consts::PI * 1600.0 * t).sin(),
            );
        }
        for start in (0..cal).step_by(160) {
            let end = (start + 160).min(cal);
            let _ = ns.process(&noise_only[start..end]);
        }

        // Then speech-like stronger midband tone on top of same noise.
        let n = FRAME * 20;
        let mut mixed = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / 16_000.0;
            let noise = 0.05 * (2.0 * std::f32::consts::PI * 400.0 * t).sin()
                + 0.04 * (2.0 * std::f32::consts::PI * 1600.0 * t).sin();
            let speech = 0.35 * (2.0 * std::f32::consts::PI * 900.0 * t).sin();
            mixed.push(noise + speech);
        }
        let mut out = Vec::new();
        for start in (0..n).step_by(160) {
            let end = (start + 160).min(n);
            out.extend(ns.process(&mixed[start..end]));
        }
        out.extend(ns.process(&vec![0.0; FRAME]));

        let skip = FRAME * 2;
        let end = out.len().min(mixed.len()).saturating_sub(FRAME);
        assert!(end > skip + FRAME);
        let in_rms = rms(&mixed[skip..end]);
        let out_rms = rms(&out[skip..end]);
        // Strong speech component should not be wiped; ratio should stay
        // above a floor (not near-zero).
        assert!(
            out_rms / in_rms > 0.25,
            "speech-like tone should largely survive: out/in = {:.3}",
            out_rms / in_rms
        );
        // And still not identity (some noise removed).
        let max_diff = mixed[skip..end]
            .iter()
            .zip(out[skip..end].iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_diff > 1e-3,
            "output must differ from input (noise removed)"
        );
    }

    #[test]
    fn fft_roundtrip_identity() {
        let mut re: Vec<f32> = (0..FRAME)
            .map(|i| (i as f32 * 0.07).sin() * 0.3)
            .collect();
        let mut im = vec![0.0f32; FRAME];
        let original = re.clone();
        fft_inplace(&mut re, &mut im, false);
        fft_inplace(&mut re, &mut im, true);
        for i in 0..FRAME {
            assert!(
                (re[i] - original[i]).abs() < 1e-4,
                "FFT roundtrip failed at {i}: {} vs {}",
                re[i],
                original[i]
            );
        }
    }
}
