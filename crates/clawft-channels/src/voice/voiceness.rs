//! Spectral voiceness gate — the "is this frame VOICE, not broadband noise?"
//! decision that the energy VAD cannot make at low SNR.
//!
//! Energy-only voice activity detection is fundamentally cornered at ~5 dB SNR:
//! when room tone is high (a hot input gain reads the floor at ~-37 dBFS) and
//! speech runs only 5–13 dB above it, the onset gate `floor + margin` sits at or
//! above speech level. Lower the margin and broadband room tone trips the gate;
//! raise it and speech is deaf. No energy threshold escapes that box — the two
//! signals are separable only by SHAPE, not level.
//!
//! Speech concentrates its energy in the 300–3400 Hz band with harmonic
//! structure; broadband room tone (fans, HVAC, hiss) spreads energy across the
//! whole spectrum. The **speech-band energy ratio** — power in 300–3400 Hz over
//! total power — is therefore ~0.4 for broadband noise (its bandwidth fraction)
//! but ~0.8–0.95 for speech, and stays separable even when speech is only a few
//! dB above the noise. Used as an AND-gate at utterance onset, it lets the
//! energy margin drop to ~4 dB (so quiet speech clears) without admitting the
//! room tone the relaxed gate would otherwise pass.
//!
//! [`Voiceness`] is the seam: this [`SpectralVoiceness`] impl is the
//! model-free fallback; a Silero-VAD ONNX backend drops in later as the
//! preferred backend (runtime-detected on model presence), reusing the same
//! trait for barge-in in Wave 3.

/// Per-frame voiceness scorer: `1.0` = confidently voice, `0.0` = confidently
/// not. Implementations must be cheap enough to run on every captured frame.
pub trait Voiceness: Send + Sync {
    /// Score `frame` (16-bit PCM at `sample_rate`) in `[0, 1]`.
    fn score(&self, frame: &[i16], sample_rate: u32) -> f32;
}

/// FFT size for the per-frame spectrum. 2048 covers up to 128 ms at 16 kHz; the
/// capture frame (usually 100 ms / 1600 samples) is Hann-windowed and zero-
/// padded to this length. Larger frames are truncated — a voiceness estimate
/// doesn't need the whole tail.
const FFT_SIZE: usize = 2048;

/// Speech band edges (Hz). The telephony band — where voiced speech energy and
/// the first few formants live. Broadband noise has only ~its bandwidth
/// fraction of energy here; speech has the great majority.
const SPEECH_LO_HZ: f32 = 300.0;
const SPEECH_HI_HZ: f32 = 3400.0;

/// Samples analysed per score — a rolling window of the most recent audio
/// (~32 ms at 16 kHz). The live capture path delivers small, variable frames
/// (~80–160 samples per 5 ms drain), FAR shorter than a usable analysis frame;
/// scoring each in isolation with a fixed window crushed them to near-silence
/// (the round-9 deafness bug). Accumulating into this rolling buffer makes the
/// score independent of how the stream is chunked.
const ANALYSIS_SAMPLES: usize = 512;

/// Below this many buffered samples the spectral estimate is meaningless; return
/// a passing score so the warm-up (≤ one frame) can't deafen the gate.
const MIN_ANALYSIS_SAMPLES: usize = 64;

/// Model-free voiceness via the speech-band energy ratio.
pub struct SpectralVoiceness {
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    /// Rolling buffer of the most recent normalized samples across `score`
    /// calls, so tiny capture frames accumulate into a real analysis window.
    recent: std::sync::Mutex<std::collections::VecDeque<f32>>,
}

impl std::fmt::Debug for SpectralVoiceness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpectralVoiceness").finish_non_exhaustive()
    }
}

impl Default for SpectralVoiceness {
    fn default() -> Self {
        Self::new()
    }
}

impl SpectralVoiceness {
    /// Build the scorer (plans the FFT once).
    pub fn new() -> Self {
        let fft = rustfft::FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        Self {
            fft,
            recent: std::sync::Mutex::new(std::collections::VecDeque::with_capacity(
                ANALYSIS_SAMPLES,
            )),
        }
    }
}

impl Voiceness for SpectralVoiceness {
    fn score(&self, frame: &[i16], sample_rate: u32) -> f32 {
        use rustfft::num_complex::Complex;
        if frame.is_empty() || sample_rate == 0 {
            return 0.0;
        }
        // Append this frame to the rolling buffer, keeping the most recent
        // ANALYSIS_SAMPLES; then snapshot into the FFT input under the lock.
        let mut buf = vec![Complex::<f32>::new(0.0, 0.0); FFT_SIZE];
        let n;
        {
            let mut recent = self.recent.lock().expect("voiceness buffer poisoned");
            for &s in frame {
                recent.push_back(f32::from(s) / 32_768.0);
            }
            while recent.len() > ANALYSIS_SAMPLES {
                recent.pop_front();
            }
            n = recent.len();
            if n < MIN_ANALYSIS_SAMPLES {
                return 1.0; // warm-up: not enough audio to judge — don't block
            }
            // Periodic Hann sized to the ACTUAL sample count (not FFT_SIZE) so a
            // short window isn't multiplied by a mismatched window's near-zero
            // head — the crux of the round-9 fix.
            let denom = n as f32;
            for (i, &s) in recent.iter().enumerate() {
                let w = (std::f32::consts::PI * i as f32 / denom).sin().powi(2);
                buf[i].re = s * w;
            }
        }
        self.fft.process(&mut buf);

        // One-sided power spectrum. Map the speech band to bin indices.
        let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
        let lo = (SPEECH_LO_HZ / bin_hz).round() as usize;
        let hi = ((SPEECH_HI_HZ / bin_hz).round() as usize).min(FFT_SIZE / 2 - 1);

        let mut speech = 0.0f32;
        let mut total = 0.0f32;
        // Skip DC (bin 0): a capture DC offset is not voice and would inflate
        // `total` toward the low end, deflating the ratio.
        for (k, c) in buf.iter().enumerate().take(FFT_SIZE / 2).skip(1) {
            let p = c.re * c.re + c.im * c.im;
            total += p;
            if k >= lo && k <= hi {
                speech += p;
            }
        }
        if total <= f32::EPSILON {
            return 0.0;
        }
        (speech / total).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    const SR: u32 = 16_000;

    /// Sum of a few in-band partials (a crude voiced vowel: fundamental + low
    /// harmonics + a formant), scaled to `amp`.
    fn speechlike(len: usize, amp: f32) -> Vec<i16> {
        let partials = [180.0f32, 360.0, 540.0, 900.0, 1400.0];
        (0..len)
            .map(|i| {
                let t = i as f32 / SR as f32;
                let s: f32 = partials.iter().map(|&f| (TAU * f * t).sin()).sum();
                (s / partials.len() as f32 * amp) as i16
            })
            .collect()
    }

    /// Deterministic white-ish noise (LCG uniform), scaled to `amp` — flat
    /// spectrum, the broadband room tone the gate must reject.
    fn broadband(len: usize, amp: f32, seed: u32) -> Vec<i16> {
        let mut state = seed.wrapping_mul(2_654_435_761).max(1);
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let u = (state >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
                ((u - 0.5) * 2.0 * amp) as i16
            })
            .collect()
    }

    fn rms(frame: &[i16]) -> f32 {
        let s: f64 = frame.iter().map(|&x| (x as f64).powi(2)).sum();
        (s / frame.len() as f64).sqrt() as f32
    }

    #[test]
    fn speech_scores_high_noise_scores_low() {
        let v = SpectralVoiceness::new();
        let speech = v.score(&speechlike(1_600, 6_000.0), SR);
        let noise = v.score(&broadband(1_600, 6_000.0, 7), SR);
        assert!(speech > 0.7, "in-band speech should score high, got {speech}");
        assert!(noise < 0.5, "broadband noise should score low, got {noise}");
        assert!(speech - noise > 0.25, "speech/noise must be separable");
    }

    #[test]
    fn speech_at_plus_5db_over_broadband_is_still_voiced() {
        // The round-8 acceptance: energy VAD provably fails this, voiceness must
        // not. Mix speech 5 dB above broadband noise and confirm the score still
        // clears the gate the caller uses (0.5).
        let v = SpectralVoiceness::new();
        let noise = broadband(1_600, 4_000.0, 11);
        let mut sp = speechlike(1_600, 100.0);
        // Scale speech so its RMS is +5 dB over the noise RMS.
        let target = rms(&noise) * 10f32.powf(5.0 / 20.0);
        let cur = rms(&sp).max(1.0);
        let g = target / cur;
        for s in &mut sp {
            *s = (*s as f32 * g) as i16;
        }
        let mix: Vec<i16> = sp
            .iter()
            .zip(&noise)
            .map(|(&a, &b)| a.saturating_add(b))
            .collect();
        let score = v.score(&mix, SR);
        assert!(
            score >= 0.5,
            "speech at +5 dB over broadband noise must stay voiced, got {score}"
        );
    }

    #[test]
    fn noise_only_stays_below_gate_across_seeds() {
        let v = SpectralVoiceness::new();
        for seed in 1..8 {
            let s = v.score(&broadband(1_600, 5_000.0, seed), SR);
            assert!(s < 0.5, "broadband seed {seed} must stay silent, got {s}");
        }
    }

    #[test]
    fn empty_and_silent_score_zero() {
        let v = SpectralVoiceness::new();
        assert_eq!(v.score(&[], SR), 0.0);
        // Silence needs enough buffered samples to be judged (not warm-up).
        for _ in 0..8 {
            v.score(&[0i16; 1_600], SR);
        }
        assert_eq!(v.score(&[0i16; 1_600], SR), 0.0);
    }

    #[test]
    fn small_frames_still_score_speech_high() {
        // The round-9 deafness bug: live capture delivers ~80-sample frames, and
        // a fixed 2048-window crushed them to ~0 so speech scored low and every
        // onset was blocked. The rolling buffer must accumulate small frames so
        // speech still scores high regardless of chunking.
        let v = SpectralVoiceness::new();
        let full = speechlike(1_600, 6_000.0);
        let mut last = 0.0;
        for chunk in full.chunks(80) {
            last = v.score(chunk, SR);
        }
        assert!(
            last > 0.7,
            "speech fed as 80-sample frames must still score high, got {last}"
        );
    }

    #[test]
    fn small_frames_noise_still_scores_low() {
        // The small-frame fix must not also start passing broadband noise.
        let v = SpectralVoiceness::new();
        let full = broadband(1_600, 6_000.0, 5);
        let mut last = 1.0;
        for chunk in full.chunks(80) {
            last = v.score(chunk, SR);
        }
        assert!(last < 0.5, "broadband in small frames stays low, got {last}");
    }
}
