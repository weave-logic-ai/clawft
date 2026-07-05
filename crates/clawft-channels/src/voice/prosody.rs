//! Prosody DSP + capture health (Voice Phase 1, Wave 1 §W1.2).
//!
//! Greenfield signal processing over a finalized utterance — **no model**:
//!
//! - **f0 / pitch track** via a YIN-lite estimator (cumulative-mean-normalized
//!   difference), yielding `f0_mean/min/max_hz`, `f0_range_semitones`, and a
//!   normalized `f0_slope`.
//! - **Energy dynamics + capture health** — per-frame RMS envelope (reusing
//!   [`EnergyVad::rms_dbfs`]) → dynamics range, clipping %, DC offset.
//! - **Speaking rate** — tokens over voiced seconds.
//! - **Pause structure** — gaps between consecutive token spans.
//! - **The VAD (valence/arousal/dominance) emotion mapping** with the plan's
//!   honesty rules: **arousal is the always-present floor** (robust from
//!   prosody, `Medium` confidence); **valence is a coarse lean** (`Low`); and
//!   **dominance is `0.0`** (not DSP-inferable — needs a SER model).
//!
//! This is the DSP producer §W1.2 ships alongside the (degraded-off) SER seam
//! in [`crate::voice::ser`]. A present SER model overrides `valence` + `label`
//! (and `dominance` if it emits it); DSP arousal stays the floor.

use super::analysis::{Confidence, EmotionAnalysis, EmotionSource, ProsodyAnalysis};
use super::stt::TranscriptToken;
use super::vad::EnergyVad;

/// Lowest pitch the estimator will report (Hz). Below typical adult male f0.
const MIN_F0_HZ: f32 = 70.0;
/// Highest pitch the estimator will report (Hz). Above typical adult female f0.
const MAX_F0_HZ: f32 = 400.0;
/// Analysis window for the f0 estimator (samples at 16 kHz ≈ 64 ms — several
/// periods of the lowest pitch for a stable difference function).
const F0_WINDOW: usize = 1_024;
/// Hop between f0 frames (samples at 16 kHz ≈ 16 ms).
const F0_HOP: usize = 256;
/// YIN absolute threshold: the first CMND dip below this is taken as the
/// period. 0.15 is the canonical clean-speech value.
const YIN_THRESHOLD: f32 = 0.15;
/// CMND value above which a frame is treated as unvoiced (no clear period).
const YIN_UNVOICED: f32 = 0.55;
/// RMS-envelope frame length (samples at 16 kHz = 20 ms).
const ENERGY_FRAME: usize = 320;
/// A gap at least this long between two token spans counts as a pause (ms).
const PAUSE_MIN_MS: u32 = 150;
/// Fraction of full-scale at/above which a sample counts as clipped.
const CLIP_LEVEL: i16 = (i16::MAX as f32 * 0.99) as i16;

/// The pieces the controller feeds the prosody analyzer. Everything here is
/// already on hand at the observer boundary: the finalized (voiced) PCM, the
/// per-token timing from the enriched STT, and the VAD voiced-sample counter.
pub struct ProsodyInput<'a> {
    /// Finalized utterance PCM (16 kHz mono `s16le`; voiced frames only).
    pub samples: &'a [i16],
    /// Sample rate of `samples`.
    pub sample_rate: u32,
    /// Voiced audio duration (ms) from the VAD voiced-sample counter.
    pub voiced_ms: u64,
    /// Per-token spans from the native STT path (empty on the substrate path).
    pub tokens: &'a [TranscriptToken],
}

/// The pitch-track summary the f0 estimator produces.
#[derive(Debug, Clone, PartialEq)]
pub struct F0Track {
    /// Mean pitch over voiced frames (Hz; `0.0` if no voiced frame).
    pub mean_hz: f32,
    /// Minimum voiced pitch (Hz).
    pub min_hz: f32,
    /// Maximum voiced pitch (Hz).
    pub max_hz: f32,
    /// Pitch range in semitones (`12·log2(max/min)`).
    pub range_semitones: f32,
    /// Normalized pitch trend in `[-1, 1]` (rise positive, fall negative).
    pub slope: f32,
    /// Fraction of analyzed frames judged voiced.
    pub voiced_ratio: f32,
}

/// Per-utterance capture health + energy envelope — one pass over the i16
/// frames. Reuses [`EnergyVad::rms_dbfs`] so the dBFS math matches the VAD.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureHealth {
    /// Mean per-frame RMS (dBFS).
    pub rms_dbfs_mean: f32,
    /// Peak per-frame RMS (dBFS).
    pub rms_dbfs_peak: f32,
    /// Envelope range (dB) between the loudest and quietest audible frame.
    pub energy_dynamics_db: f32,
    /// Fraction of samples at/near i16 full-scale (%).
    pub clip_pct: f32,
    /// Mean sample offset (DC bias) in i16 units.
    pub dc_offset: i32,
}

/// Estimate the pitch track over an utterance via YIN-lite.
pub fn estimate_f0_track(samples: &[i16], sample_rate: u32) -> F0Track {
    let sr = sample_rate.max(1) as f32;
    let tau_min = (sr / MAX_F0_HZ).floor() as usize;
    let tau_max = (sr / MIN_F0_HZ).ceil() as usize;
    let mono: Vec<f32> = samples.iter().map(|&s| s as f32 / 32_768.0).collect();

    let mut voiced: Vec<(f32, f32)> = Vec::new(); // (time_s, f0_hz)
    let mut analyzed = 0usize;
    if mono.len() >= F0_WINDOW && tau_max < F0_WINDOW {
        let mut start = 0;
        while start + F0_WINDOW <= mono.len() {
            analyzed += 1;
            if let Some(f0) = yin_frame(&mono[start..start + F0_WINDOW], sr, tau_min, tau_max) {
                let t = (start + F0_WINDOW / 2) as f32 / sr;
                voiced.push((t, f0));
            }
            start += F0_HOP;
        }
    }

    if voiced.is_empty() {
        return F0Track {
            mean_hz: 0.0,
            min_hz: 0.0,
            max_hz: 0.0,
            range_semitones: 0.0,
            slope: 0.0,
            voiced_ratio: 0.0,
        };
    }

    // YIN is prone to octave errors (a frame reported at 2× or ½ the true
    // pitch), which min/max range grossly inflates — a flat calm sentence read
    // ~2 octaves before this. Clean the track in two passes:
    //  1. octave-fold every frame toward the track median (halve/double until
    //     within a ~1.5× band), which collapses octave doubling/halving for
    //     speech (whose real range sits well inside one octave);
    //  2. 5-point median filter to drop residual single-frame spikes.
    // Then report a ROBUST range from the p10–p90 spread, not min/max.
    let raw: Vec<f32> = voiced.iter().map(|(_, f)| *f).collect();
    let med = median(&raw);
    let mut folded: Vec<(f32, f32)> = voiced
        .iter()
        .map(|(t, f)| (*t, octave_fold(*f, med)))
        .collect();
    median_filter_f0(&mut folded, 5);

    let vals: Vec<f32> = folded.iter().map(|(_, f)| *f).collect();
    let mean = vals.iter().sum::<f32>() / vals.len() as f32;
    let p10 = percentile(&vals, 0.10);
    let p90 = percentile(&vals, 0.90);
    let range_semitones = if p10 > 0.0 {
        12.0 * (p90 / p10).log2()
    } else {
        0.0
    };
    F0Track {
        mean_hz: mean,
        min_hz: p10,
        max_hz: p90,
        range_semitones,
        slope: normalized_slope(&folded, mean),
        voiced_ratio: if analyzed > 0 {
            folded.len() as f32 / analyzed as f32
        } else {
            0.0
        },
    }
}

/// Median of a slice (returns 0.0 for empty).
fn median(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s[s.len() / 2]
}

/// Interpolation-free percentile `p ∈ [0,1]` of a slice (nearest-rank).
fn percentile(v: &[f32], p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((p * (s.len() - 1) as f32).round() as usize).min(s.len() - 1);
    s[idx]
}

/// Fold `f` toward `reference` by octaves until it sits within a ~1.5× band —
/// collapsing YIN octave errors for speech (real range < 1 octave).
fn octave_fold(f: f32, reference: f32) -> f32 {
    if reference <= 0.0 || f <= 0.0 {
        return f;
    }
    let mut v = f;
    while v > reference * 1.5 {
        v /= 2.0;
    }
    while v < reference / 1.5 {
        v *= 2.0;
    }
    v
}

/// In-place odd-window median filter over the f0 values (times untouched).
fn median_filter_f0(track: &mut [(f32, f32)], window: usize) {
    if track.len() < window || window < 3 {
        return;
    }
    let half = window / 2;
    let orig: Vec<f32> = track.iter().map(|(_, f)| *f).collect();
    for i in half..track.len() - half {
        let mut w: Vec<f32> = orig[i - half..=i + half].to_vec();
        w.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        track[i].1 = w[half];
    }
}

/// One-pass capture health + energy envelope over the i16 frames.
pub fn capture_health(samples: &[i16], _sample_rate: u32) -> CaptureHealth {
    if samples.is_empty() {
        return CaptureHealth {
            rms_dbfs_mean: -100.0,
            rms_dbfs_peak: -100.0,
            energy_dynamics_db: 0.0,
            clip_pct: 0.0,
            dc_offset: 0,
        };
    }
    // Clipping % + DC offset in one pass over samples.
    let mut clipped = 0u64;
    let mut sum = 0i64;
    for &s in samples {
        if s.unsigned_abs() >= CLIP_LEVEL as u16 {
            clipped += 1;
        }
        sum += s as i64;
    }
    let clip_pct = clipped as f32 / samples.len() as f32 * 100.0;
    let dc_offset = (sum / samples.len() as i64) as i32;

    // Per-frame RMS envelope (dBFS), reusing the VAD's math.
    let mut frame_dbfs: Vec<f32> = Vec::new();
    let mut start = 0;
    while start < samples.len() {
        let end = (start + ENERGY_FRAME).min(samples.len());
        frame_dbfs.push(EnergyVad::rms_dbfs(&samples[start..end]));
        start = end;
    }
    let mean = frame_dbfs.iter().sum::<f32>() / frame_dbfs.len() as f32;
    let peak = frame_dbfs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    // Dynamics = span between the loudest and the quietest *audible* frame
    // (ignore true-silence frames pinned at the -100 floor).
    let quietest_audible = frame_dbfs
        .iter()
        .copied()
        .filter(|&d| d > -99.0)
        .fold(f32::INFINITY, f32::min);
    let energy_dynamics_db = if quietest_audible.is_finite() && peak.is_finite() {
        (peak - quietest_audible).max(0.0)
    } else {
        0.0
    };

    CaptureHealth {
        rms_dbfs_mean: mean,
        rms_dbfs_peak: peak,
        energy_dynamics_db,
        clip_pct,
        dc_offset,
    }
}

/// Produce the complete [`ProsodyAnalysis`] from the utterance signal + timing.
pub fn analyze_prosody(input: &ProsodyInput) -> ProsodyAnalysis {
    let f0 = estimate_f0_track(input.samples, input.sample_rate);
    let health = capture_health(input.samples, input.sample_rate);
    let (pause_count, pause_mean_ms) = pause_structure(input.tokens);
    let voiced_s = input.voiced_ms as f32 / 1_000.0;
    let rate = if voiced_s > 0.0 {
        input.tokens.len() as f32 / voiced_s
    } else {
        0.0
    };
    // Prosody is honest at Medium when there is a real voiced pitch track to
    // stand on; without one (whispered / noise / too short) it degrades to Low.
    let confidence = if f0.voiced_ratio >= 0.15 && f0.mean_hz > 0.0 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    ProsodyAnalysis {
        f0_mean_hz: f0.mean_hz,
        f0_min_hz: f0.min_hz,
        f0_max_hz: f0.max_hz,
        f0_range_semitones: f0.range_semitones,
        f0_slope: f0.slope,
        rate_tokens_per_s: rate,
        pause_count,
        pause_mean_ms,
        energy_dynamics_db: health.energy_dynamics_db,
        confidence,
    }
}

/// Map prosody → the VAD emotion axis with the plan's honesty rules.
///
/// **Arousal** is the always-present floor, robustly derivable from pitch
/// variability + speaking rate + energy dynamics (`Medium`). **Valence** is a
/// coarse lean off pitch height/slope (`Low`). **Dominance** is `0.0` — not
/// DSP-inferable. `source` is `prosody-dsp`; a SER model overrides valence +
/// label (and dominance) via [`crate::voice::ser`] while DSP arousal stays.
pub fn emotion_from_prosody(p: &ProsodyAnalysis) -> EmotionAnalysis {
    // Normalize each cue into [0,1] against a plausible speech span, then
    // blend. These weights favour pitch variability + rate (the strongest
    // arousal correlates) with energy dynamics as support.
    let pitch_var = norm(p.f0_range_semitones, 2.0, 14.0);
    let rate = norm(p.rate_tokens_per_s, 2.0, 9.0);
    let dyn_db = norm(p.energy_dynamics_db, 6.0, 28.0);
    let arousal = (0.45 * pitch_var + 0.35 * rate + 0.20 * dyn_db).clamp(0.0, 1.0);

    // Valence: only a lean. Higher/rising pitch skews mildly positive; a
    // strong downward slope skews mildly negative. Kept small — it's Low conf.
    let height = if p.f0_mean_hz > 0.0 {
        norm(p.f0_mean_hz, 90.0, 260.0) - 0.5
    } else {
        0.0
    };
    let valence = (0.4 * height + 0.3 * p.f0_slope.clamp(-1.0, 1.0)).clamp(-1.0, 1.0);

    EmotionAnalysis {
        valence,
        arousal,
        dominance: 0.0,
        label: coarse_label(arousal, valence),
        arousal_conf: if p.confidence == Confidence::Medium {
            Confidence::Medium
        } else {
            Confidence::Low
        },
        valence_conf: Confidence::Low,
        source: EmotionSource::ProsodyDsp,
    }
}

/// YIN-lite for one window: cumulative-mean-normalized difference, first dip
/// below [`YIN_THRESHOLD`], parabolic-interpolated period → f0. `None` if the
/// window reads unvoiced.
fn yin_frame(frame: &[f32], sr: f32, tau_min: usize, tau_max: usize) -> Option<f32> {
    let w = frame.len();
    let integ = w - tau_max; // samples summed per lag (kept constant across tau)
    if integ == 0 {
        return None;
    }
    // Difference function d(tau).
    let mut d = vec![0.0f32; tau_max + 1];
    for (tau, dt) in d.iter_mut().enumerate().take(tau_max + 1).skip(1) {
        let mut sum = 0.0f32;
        for j in 0..integ {
            let diff = frame[j] - frame[j + tau];
            sum += diff * diff;
        }
        *dt = sum;
    }
    // Cumulative-mean-normalized difference d'(tau).
    let mut dp = vec![1.0f32; tau_max + 1];
    let mut running = 0.0f32;
    for tau in 1..=tau_max {
        running += d[tau];
        dp[tau] = if running > 0.0 {
            d[tau] * tau as f32 / running
        } else {
            1.0
        };
    }
    // First dip below threshold within the plausible pitch band, stepped to
    // the local minimum; else the global minimum in-band.
    let mut best_tau = 0usize;
    let mut best_val = f32::MAX;
    let mut tau = tau_min.max(1);
    while tau <= tau_max {
        if dp[tau] < YIN_THRESHOLD {
            let mut k = tau;
            while k < tau_max && dp[k + 1] < dp[k] {
                k += 1;
            }
            best_tau = k;
            best_val = dp[k];
            break;
        }
        if dp[tau] < best_val {
            best_val = dp[tau];
            best_tau = tau;
        }
        tau += 1;
    }
    if best_tau == 0 || best_val > YIN_UNVOICED {
        return None; // no clear period → unvoiced
    }
    let refined = parabolic_min(&dp, best_tau);
    let f0 = sr / refined;
    if (MIN_F0_HZ..=MAX_F0_HZ).contains(&f0) {
        Some(f0)
    } else {
        None
    }
}

/// Parabolic interpolation of the period around the CMND minimum at `tau`.
fn parabolic_min(dp: &[f32], tau: usize) -> f32 {
    if tau == 0 || tau + 1 >= dp.len() {
        return tau as f32;
    }
    let a = dp[tau - 1];
    let b = dp[tau];
    let c = dp[tau + 1];
    let denom = a - 2.0 * b + c;
    if denom.abs() < 1e-9 {
        return tau as f32;
    }
    tau as f32 + 0.5 * (a - c) / denom
}

/// Normalized linear-regression slope of f0 over time, squashed into `[-1,1]`.
/// The raw slope (Hz/s) is scaled by the mean so a proportional rise/fall of
/// one mean-pitch-per-second maps near the extremes.
fn normalized_slope(track: &[(f32, f32)], mean: f32) -> f32 {
    if track.len() < 2 || mean <= 0.0 {
        return 0.0;
    }
    let n = track.len() as f32;
    let t_mean = track.iter().map(|(t, _)| *t).sum::<f32>() / n;
    let f_mean = track.iter().map(|(_, f)| *f).sum::<f32>() / n;
    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for (t, f) in track {
        num += (t - t_mean) * (f - f_mean);
        den += (t - t_mean) * (t - t_mean);
    }
    if den.abs() < 1e-9 {
        return 0.0;
    }
    let slope_hz_per_s = num / den;
    (slope_hz_per_s / mean).clamp(-1.0, 1.0)
}

/// Pause structure from consecutive token spans: count and mean of the gaps
/// (silence between one token's end and the next's onset) ≥ [`PAUSE_MIN_MS`].
fn pause_structure(tokens: &[TranscriptToken]) -> (u32, u32) {
    let mut count = 0u32;
    let mut total = 0u64;
    for pair in tokens.windows(2) {
        let prev_end = pair[0].start_ms + pair[0].duration_ms;
        let gap = pair[1].start_ms.saturating_sub(prev_end);
        if gap >= PAUSE_MIN_MS {
            count += 1;
            total += gap as u64;
        }
    }
    let mean = if count > 0 {
        (total / count as u64) as u32
    } else {
        0
    };
    (count, mean)
}

/// Coarse categorical label from the VAD quadrant — deliberately a small,
/// prosody-honest set. A SER model replaces this with a real label.
fn coarse_label(arousal: f32, valence: f32) -> String {
    let label = if arousal >= 0.6 {
        if valence <= -0.15 {
            "agitated"
        } else if valence >= 0.15 {
            "excited"
        } else {
            "activated"
        }
    } else if arousal < 0.35 {
        if valence <= -0.15 {
            "subdued"
        } else {
            "calm"
        }
    } else {
        "neutral"
    };
    label.to_string()
}

/// Clamp-normalize `x` from `[lo, hi]` into `[0, 1]`.
fn norm(x: f32, lo: f32, hi: f32) -> f32 {
    if hi <= lo {
        return 0.0;
    }
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pure sine at `hz` for `secs` seconds at 16 kHz, amplitude `amp`.
    fn tone(hz: f32, secs: f32, amp: i16) -> Vec<i16> {
        let sr = 16_000.0;
        let n = (sr * secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr;
                (amp as f32 * (2.0 * std::f32::consts::PI * hz * t).sin()) as i16
            })
            .collect()
    }

    #[test]
    fn f0_on_synthetic_tone_120hz() {
        let s = tone(120.0, 0.5, 8_000);
        let track = estimate_f0_track(&s, 16_000);
        assert!(track.voiced_ratio > 0.5, "tone should read voiced");
        assert!(
            (track.mean_hz - 120.0).abs() < 6.0,
            "f0 {} should be ~120 Hz",
            track.mean_hz
        );
    }

    #[test]
    fn f0_on_synthetic_tone_220hz() {
        let s = tone(220.0, 0.5, 8_000);
        let track = estimate_f0_track(&s, 16_000);
        assert!(
            (track.mean_hz - 220.0).abs() < 8.0,
            "f0 {} should be ~220 Hz",
            track.mean_hz
        );
    }

    #[test]
    fn silence_reads_unvoiced() {
        let track = estimate_f0_track(&vec![0i16; 8_000], 16_000);
        assert_eq!(track.voiced_ratio, 0.0);
        assert_eq!(track.mean_hz, 0.0);
    }

    #[test]
    fn octave_jumps_are_folded_to_low_range() {
        // A 150↔300 Hz octave alternation is the classic YIN octave-error shape.
        // The estimator folds toward the median, so the reported range stays
        // small instead of the ~12 st a raw min/max would show.
        let mut s = tone(150.0, 0.3, 8_000);
        s.extend(tone(300.0, 0.3, 8_000));
        let track = estimate_f0_track(&s, 16_000);
        assert!(
            track.range_semitones < 4.0,
            "octave error must fold to a low range, got {}",
            track.range_semitones
        );
    }

    #[test]
    fn octave_fold_collapses_doubling_and_halving() {
        assert!((octave_fold(240.0, 120.0) - 120.0).abs() < 1e-3);
        assert!((octave_fold(60.0, 120.0) - 120.0).abs() < 1e-3);
        // A real sub-octave excursion is preserved (a fifth ≈ 180 Hz).
        assert!((octave_fold(180.0, 120.0) - 180.0).abs() < 1e-3);
    }

    #[test]
    fn median_filter_drops_single_spike() {
        let mut track: Vec<(f32, f32)> = vec![
            (0.0, 120.0),
            (0.1, 121.0),
            (0.2, 480.0), // octave-error spike
            (0.3, 119.0),
            (0.4, 120.0),
        ];
        median_filter_f0(&mut track, 3);
        assert!(track[2].1 < 130.0, "spike should be median-smoothed, got {}", track[2].1);
    }

    #[test]
    fn percentile_nearest_rank() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&v, 0.0), 1.0);
        assert_eq!(percentile(&v, 1.0), 5.0);
        assert!((percentile(&v, 0.5) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn calm_monotone_reads_low_range_and_arousal() {
        // Acceptance: a calm ~monotone sentence reads range < 4 st and the DSP
        // emotion floor stays calm (arousal < 0.35).
        let s = tone(140.0, 1.0, 6_000);
        let input = ProsodyInput {
            samples: &s,
            sample_rate: 16_000,
            voiced_ms: 1_000,
            tokens: &[],
        };
        let p = analyze_prosody(&input);
        assert!(p.f0_range_semitones < 4.0, "calm range {} must be <4st", p.f0_range_semitones);
        let e = emotion_from_prosody(&p);
        assert!(e.arousal < 0.35, "calm arousal {} must be <0.35", e.arousal);
    }

    #[test]
    fn capture_health_detects_clipping_and_dc() {
        // Half full-scale sine biased by +1000 DC, plus a clipped stretch.
        let mut s: Vec<i16> = tone(150.0, 0.2, 16_000);
        for v in s.iter_mut() {
            *v = v.saturating_add(1_000);
        }
        s.extend(vec![i16::MAX; 400]);
        let h = capture_health(&s, 16_000);
        assert!(h.clip_pct > 0.0, "clipped stretch should register");
        assert!(h.dc_offset > 500, "DC bias should be positive, got {}", h.dc_offset);
        assert!(h.rms_dbfs_peak > -20.0);
    }

    #[test]
    fn pause_structure_counts_token_gaps() {
        let tok = |t: u32, d: u32| TranscriptToken {
            text: "x".into(),
            start_ms: t,
            duration_ms: d,
            confidence: 0.9,
        };
        // 0..80, 80..160 (no gap), then 400..480 (240 ms gap).
        let tokens = vec![tok(0, 80), tok(80, 80), tok(400, 80)];
        let (count, mean) = pause_structure(&tokens);
        assert_eq!(count, 1);
        assert!((200..=260).contains(&mean), "gap mean {mean}");
    }

    #[test]
    fn rate_from_tokens_over_voiced_time() {
        let tokens: Vec<TranscriptToken> = (0..8)
            .map(|i| TranscriptToken {
                text: "x".into(),
                start_ms: i * 80,
                duration_ms: 80,
                confidence: 0.9,
            })
            .collect();
        let input = ProsodyInput {
            samples: &tone(150.0, 1.0, 6_000),
            sample_rate: 16_000,
            voiced_ms: 2_000,
            tokens: &tokens,
        };
        let p = analyze_prosody(&input);
        // 8 tokens over 2 s = 4 tokens/s.
        assert!((p.rate_tokens_per_s - 4.0).abs() < 1e-3);
        assert_eq!(p.confidence, Confidence::Medium);
    }

    #[test]
    fn emotion_arousal_rises_with_prosodic_activation() {
        let calm = ProsodyAnalysis {
            f0_mean_hz: 120.0,
            f0_range_semitones: 2.0,
            rate_tokens_per_s: 2.5,
            energy_dynamics_db: 6.0,
            confidence: Confidence::Medium,
            ..ProsodyAnalysis::default()
        };
        let hot = ProsodyAnalysis {
            f0_mean_hz: 220.0,
            f0_range_semitones: 13.0,
            rate_tokens_per_s: 8.0,
            energy_dynamics_db: 26.0,
            confidence: Confidence::Medium,
            ..ProsodyAnalysis::default()
        };
        let ce = emotion_from_prosody(&calm);
        let he = emotion_from_prosody(&hot);
        assert!(he.arousal > ce.arousal + 0.3, "activation should raise arousal");
        assert_eq!(he.arousal_conf, Confidence::Medium);
        // Honesty rules: dominance is always 0, valence is only ever Low conf.
        assert_eq!(he.dominance, 0.0);
        assert_eq!(he.valence_conf, Confidence::Low);
        assert_eq!(he.source, EmotionSource::ProsodyDsp);
    }

    #[test]
    fn emotion_dominance_is_zero_and_source_dsp() {
        let e = emotion_from_prosody(&ProsodyAnalysis::default());
        assert_eq!(e.dominance, 0.0);
        assert_eq!(e.source, EmotionSource::ProsodyDsp);
    }
}
