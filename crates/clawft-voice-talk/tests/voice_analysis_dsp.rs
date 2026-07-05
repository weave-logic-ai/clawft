//! Wave 1 §W1.5 — the acoustics→arousal chain, device-free and CI-runnable.
//!
//! Coder-A's `prosody.rs` unit tests prove the emotion mapping on *hand-set*
//! `ProsodyAnalysis` fields (`emotion_arousal_rises_with_prosodic_activation`).
//! This closes the gap above them: it drives **synthetic acted vs. calm PCM**
//! through the whole public DSP pipeline — `estimate_f0_track` → `analyze_prosody`
//! → `emotion_from_prosody` — and asserts the acted clip reads a materially
//! higher arousal. That is the automated form of the exit-test's "act out an
//! emotion and watch the arousal bar move": the DSP responds to the *acoustics*
//! (pitch height + pitch variability + energy dynamics), not to any words, so a
//! model-free synthetic clip is a valid stimulus (the STT gate that would drop a
//! wordless clip lives in the full session, not here).
//!
//! No models, no devices, no daemon — pure signal in, VAD-axis out.

use std::f32::consts::PI;

use clawft_channels::voice::{
    analyze_prosody, emotion_from_prosody, estimate_f0_track, Confidence, EmotionSource,
    ProsodyInput,
};

const SR: u32 = 16_000;

/// Synthesize a voiced, speech-like utterance: six harmonics (1/k rolloff) over
/// a pitch contour that sweeps `span_st` semitones around `f0_base` with a 5 Hz
/// vibrato, shaped by a syllabic energy envelope whose depth grows with
/// `energy_mod`. Deterministic (no RNG) so the arousal comparison never flakes.
///
/// - **calm**: low base pitch, a narrow sweep, a nearly-flat envelope.
/// - **acted**: high base pitch, a wide sweep, a punchy envelope — exactly the
///   prosodic activation the DSP arousal estimator keys on.
fn voiced(f0_base: f32, span_st: f32, energy_mod: f32, secs: f32) -> Vec<i16> {
    let sr = SR as f32;
    let n = (sr * secs) as usize;
    let mut phase = 0.0f32;
    let mut raw = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;
        let frac = i as f32 / n as f32; // 0..1 across the utterance
        let vib = (2.0 * PI * 5.0 * t).sin();
        // A rise/fall sweep of `span_st` peak-to-peak plus vibrato detail.
        let semis = (frac - 0.5) * span_st + 0.5 * vib * (span_st / 12.0);
        let f0 = f0_base * 2f32.powf(semis / 12.0);
        phase += 2.0 * PI * f0 / sr;
        let mut s = 0.0f32;
        for k in 1..=6u32 {
            s += (1.0 / k as f32) * (k as f32 * phase).sin();
        }
        // Syllabic envelope: deeper loud/soft swings for higher `energy_mod`
        // → wider energy dynamics (the third arousal cue).
        let burst = (0.5 + 0.5 * (2.0 * PI * 3.5 * t).sin()).powi(3);
        let env = 1.0 - energy_mod * 0.9 * burst;
        raw.push(s * env);
    }
    let peak = raw.iter().cloned().fold(1e-6f32, |a, b| a.max(b.abs()));
    raw.iter().map(|v| (v / peak * 0.9 * 32_767.0) as i16).collect()
}

fn prosody_of(samples: &[i16], secs: f32) -> clawft_channels::voice::ProsodyAnalysis {
    analyze_prosody(&ProsodyInput {
        samples,
        sample_rate: SR,
        voiced_ms: (secs * 1_000.0) as u64,
        tokens: &[], // no lexical tokens: isolate the acoustic arousal cues
    })
}

/// The headline exit assertion: an acted (high, swinging pitch + punchy energy)
/// utterance reads a materially higher DSP arousal than a calm one, through the
/// entire public pipeline from raw PCM. Rate is held equal (no tokens either
/// side), so the separation is purely acoustic — pitch + energy, as designed.
#[test]
fn acted_clip_reads_higher_arousal_than_calm() {
    let calm_pcm = voiced(130.0, 3.0, 0.10, 1.5);
    let acted_pcm = voiced(210.0, 12.0, 0.90, 1.5);

    let calm = emotion_from_prosody(&prosody_of(&calm_pcm, 1.5));
    let acted = emotion_from_prosody(&prosody_of(&acted_pcm, 1.5));

    assert!(
        acted.arousal > calm.arousal + 0.15,
        "acted arousal {:.3} should clear calm {:.3} by >0.15 (pitch+energy activation)",
        acted.arousal,
        calm.arousal
    );
    assert!(
        (0.0..=1.0).contains(&acted.arousal) && (0.0..=1.0).contains(&calm.arousal),
        "arousal stays in [0,1]"
    );

    // Honesty rules hold on the real pipeline output, not just hand-set fields:
    // arousal is the Medium-confidence floor, dominance is never DSP-inferred,
    // valence stays a Low-confidence lean, and the source badge reads prosody.
    assert_eq!(acted.arousal_conf, Confidence::Medium, "voiced pitch → Medium arousal conf");
    assert_eq!(acted.dominance, 0.0, "dominance is 0.0 without a SER model");
    assert_eq!(acted.valence_conf, Confidence::Low, "valence is only ever a Low-conf lean");
    assert_eq!(acted.source, EmotionSource::ProsodyDsp, "DSP floor, no SER staged");
}

/// The synthetic stimuli are what the arousal claim rests on, so pin their
/// acoustics: both clips must actually read as voiced pitch (else arousal would
/// be a Low-confidence guess), the acted clip must sit higher and swing wider,
/// and `analyze_prosody` must return a complete Medium-confidence record.
#[test]
fn synthetic_stimuli_are_voiced_and_separated_in_pitch() {
    let calm_pcm = voiced(130.0, 3.0, 0.10, 1.5);
    let acted_pcm = voiced(210.0, 12.0, 0.90, 1.5);

    let calm_f0 = estimate_f0_track(&calm_pcm, SR);
    let acted_f0 = estimate_f0_track(&acted_pcm, SR);

    assert!(calm_f0.voiced_ratio > 0.5 && acted_f0.voiced_ratio > 0.5, "both read voiced");
    assert!(
        acted_f0.mean_hz > calm_f0.mean_hz + 30.0,
        "acted pitch {:.0}Hz should sit well above calm {:.0}Hz",
        acted_f0.mean_hz,
        calm_f0.mean_hz
    );
    assert!(
        acted_f0.range_semitones > calm_f0.range_semitones,
        "acted sweep {:.1}st wider than calm {:.1}st",
        acted_f0.range_semitones,
        calm_f0.range_semitones
    );

    // The record the arousal came from is complete + honestly Medium.
    let p = prosody_of(&acted_pcm, 1.5);
    assert!(p.f0_mean_hz > 0.0, "f0 populated");
    assert!(p.energy_dynamics_db > 0.0, "energy envelope populated");
    assert_eq!(p.confidence, Confidence::Medium, "voiced pitch track → Medium prosody conf");
}
