//! Light audio cues for the talk loop.
//!
//! Replaces the WEFT-658 spoken acks/fillers ("One sec.", "Okay, let me
//! think.") — live feedback: the canned phrases read as parroting and broke
//! the conversational illusion badly enough to be worse than silence. A
//! short, quiet synthesized tone carries the same information ("heard you"
//! / "still working") with no fake speech: zero TTS latency, no voice
//! mismatch, nothing for STT to self-echo into a user turn.
//!
//! Cues are generated as plain [`TtsChunk`]s at the playback pipeline's
//! native 16 kHz, so they enter the existing sink → AEC-render-reference →
//! drain path exactly like rendered speech (echo discipline preserved).

use std::f32::consts::TAU;

use super::tts::TtsChunk;

/// Which cue to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueKind {
    /// Turn accepted — spoken reply is on its way ("heard you"). A soft
    /// rising two-note blip.
    Ack,
    /// The brain is still generating ("still working") — repeated at a slow
    /// period while the answer wait drags on. A single softer tick, quieter
    /// and lower than [`CueKind::Ack`] so repetition doesn't grate.
    Working,
}

impl CueKind {
    /// Stable lowercase label for observers/logs ("ack" / "working").
    pub fn label(self) -> &'static str {
        match self {
            CueKind::Ack => "ack",
            CueKind::Working => "working",
        }
    }
}

/// Sample rate cues are generated at — the playback pipeline's native rate
/// (`clawft-voice-aec`'s `TARGET_SR`), so the sink's resampler is a no-op.
pub const CUE_SAMPLE_RATE: u32 = 16_000;

/// Attack/release ramp applied to each tone (ms) — a raised-cosine edge so
/// the cue never clicks.
const RAMP_MS: usize = 8;

/// Append `ms` of a `freq` Hz sine at `amp` (0..=1 full-scale) to `out`,
/// with raised-cosine attack/release ramps.
fn tone(freq: f32, ms: usize, amp: f32, out: &mut Vec<i16>) {
    let n = CUE_SAMPLE_RATE as usize * ms / 1000;
    let ramp = (CUE_SAMPLE_RATE as usize * RAMP_MS / 1000).min(n / 2);
    for i in 0..n {
        let env = if i < ramp {
            let x = i as f32 / ramp as f32;
            (1.0 - (x * std::f32::consts::PI).cos()) / 2.0
        } else if i >= n - ramp {
            let x = (n - i) as f32 / ramp as f32;
            (1.0 - (x * std::f32::consts::PI).cos()) / 2.0
        } else {
            1.0
        };
        let s = (TAU * freq * i as f32 / CUE_SAMPLE_RATE as f32).sin();
        out.push((s * env * amp * f32::from(i16::MAX)) as i16);
    }
}

/// Append `ms` of silence to `out`.
fn rest(ms: usize, out: &mut Vec<i16>) {
    out.extend(std::iter::repeat_n(0i16, CUE_SAMPLE_RATE as usize * ms / 1000));
}

/// Synthesize `kind` as one playback-ready chunk. Deliberately quiet
/// (≤ −16 dBFS peak) — a presence cue, not an alert.
pub fn cue_chunk(kind: CueKind) -> TtsChunk {
    let mut samples = Vec::new();
    match kind {
        CueKind::Ack => {
            // E5 → A5, a soft rising "got it".
            tone(659.25, 70, 0.15, &mut samples);
            rest(20, &mut samples);
            tone(880.0, 90, 0.12, &mut samples);
        }
        CueKind::Working => {
            // One low, quiet tick.
            tone(523.25, 80, 0.08, &mut samples);
        }
    }
    TtsChunk {
        samples,
        sample_rate: CUE_SAMPLE_RATE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cues_are_short_and_quiet() {
        for kind in [CueKind::Ack, CueKind::Working] {
            let chunk = cue_chunk(kind);
            assert_eq!(chunk.sample_rate, CUE_SAMPLE_RATE);
            let ms = chunk.samples.len() * 1000 / CUE_SAMPLE_RATE as usize;
            assert!(ms <= 250, "{kind:?} runs {ms}ms — a cue, not a jingle");
            let peak = chunk.samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
            // ≤ −16 dBFS: light presence, never startling.
            assert!(
                f32::from(peak) <= 0.16 * f32::from(i16::MAX),
                "{kind:?} peak {peak} too loud"
            );
            assert!(peak > 0, "{kind:?} produced silence");
        }
    }

    #[test]
    fn tones_ramp_to_silence_at_edges() {
        // No clicks: first and last samples are (near) zero.
        for kind in [CueKind::Ack, CueKind::Working] {
            let chunk = cue_chunk(kind);
            assert!(chunk.samples.first().unwrap().unsigned_abs() < 200);
            assert!(chunk.samples.last().unwrap().unsigned_abs() < 200);
        }
    }

    #[test]
    fn working_cue_is_quieter_than_ack() {
        let ack_peak = cue_chunk(CueKind::Ack)
            .samples
            .iter()
            .map(|s| s.unsigned_abs())
            .max()
            .unwrap();
        let working_peak = cue_chunk(CueKind::Working)
            .samples
            .iter()
            .map(|s| s.unsigned_abs())
            .max()
            .unwrap();
        assert!(working_peak < ack_peak, "repetition must not grate");
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(CueKind::Ack.label(), "ack");
        assert_eq!(CueKind::Working.label(), "working");
    }
}
