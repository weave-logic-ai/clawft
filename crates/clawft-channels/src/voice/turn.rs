//! Semantic turn-taking / endpointing (ADR-061 §5).
//!
//! A fixed silence cutoff clips a speaker who pauses mid-thought. ADR-061
//! adopts **smart-turn**-style *semantic* endpointing: on a short pause, keep
//! listening while the turn still "reads open", and finalize only when the
//! utterance is semantically complete OR a hard max-silence ceiling is hit.
//!
//! The model is an extension seam, mirroring [`EnergyVad`](super::vad) /
//! [`SttBackend`](super::stt): the default [`HeuristicEndpoint`] needs no
//! model file (it reasons over the partial transcript's trailing token); a
//! real smart-turn ONNX model implements the same [`EndpointModel`] trait and
//! drops in without touching the controller. The controller feeds frames +
//! the running partial transcript to a [`SemanticEndpointer`] and acts on its
//! [`TurnDecision`].

use std::collections::VecDeque;

use async_trait::async_trait;

/// What the endpointer wants the capture loop to do after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDecision {
    /// Keep capturing — the turn is still active or only briefly paused.
    Continue,
    /// Finalize the turn now (commit the utterance to STT).
    Finalize,
}

/// Predicts whether the speaker has finished their turn.
#[async_trait]
pub trait EndpointModel: Send + Sync {
    /// Probability in `[0, 1]` that the turn is complete, given the recent
    /// audio window and the running partial transcript. Higher = more likely
    /// the speaker is done.
    async fn completion_prob(&self, recent_audio: &[i16], partial_text: &str) -> f32;

    /// Which endpointer produced the probability, for the `VoiceAnalysis`
    /// `endpoint.source` field. Additive with a default so existing models
    /// need no change; the smart-turn model overrides this to report
    /// `smart-turn-v3` when its ONNX runtime is live (else `heuristic`).
    fn source(&self) -> &'static str {
        "heuristic"
    }
}

/// The endpoint "related data" a finalize decision produces then normally
/// discards — captured for the `VoiceAnalysis` record (§W1.2). The controller
/// reads it after a [`TurnDecision::Finalize`] and maps it into the record's
/// `endpoint` section (adding the wall-clock latency it measures itself).
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointSnapshot {
    /// Completion probability that fired the finalize, or the last one computed
    /// before a max-silence ceiling finalize (the heuristic's empty-transcript
    /// default when no semantic check ran).
    pub completion_prob: f32,
    /// Which endpointer produced it: `smart-turn-v3` | `heuristic`.
    pub source: String,
    /// Trailing-silence length at finalize (ms).
    pub silence_tail_ms: u64,
}

/// No-model default endpointer: reasons over the partial transcript's
/// trailing token. A turn ending on a conjunction / filler / comma reads
/// "open" (the speaker is mid-thought); one ending on terminal punctuation or
/// a plain word after a pause reads "complete". This is the lowest-common-
/// denominator that works without an ONNX file; smart-turn is the upgrade.
#[derive(Debug, Default, Clone)]
pub struct HeuristicEndpoint;

/// Words that, when trailing, signal the speaker is not done.
const OPEN_WORDS: &[&str] = &[
    "and", "or", "but", "so", "because", "with", "to", "the", "a", "an", "of", "for", "if", "when",
    "while", "that", "as", "at", "in", "on", "um", "uh", "like", "i", "we", "you", "my", "is",
    "are", "was", "about", "from", "by", "into", "over", "than", "this", "these", "those", "some",
    "any",
];

#[async_trait]
impl EndpointModel for HeuristicEndpoint {
    async fn completion_prob(&self, _recent_audio: &[i16], partial_text: &str) -> f32 {
        let t = partial_text.trim();
        if t.is_empty() {
            // Nothing transcribed yet — lean "not done" so we don't clip a
            // slow starter, but the max-silence ceiling will still finalize.
            return 0.3;
        }
        let last = t.chars().last().unwrap();
        if matches!(last, '.' | '!' | '?') {
            return 0.95;
        }
        if last == ',' || last == ';' || last == ':' || last == '-' {
            return 0.1; // mid-clause
        }
        let last_word = t
            .split(|c: char| !c.is_alphanumeric())
            .next_back()
            .unwrap_or("")
            .to_ascii_lowercase();
        if OPEN_WORDS.contains(&last_word.as_str()) {
            0.15
        } else {
            // A complete-looking word after a real pause: lean done.
            0.7
        }
    }
}

/// Minimum voiced audio a turn must accumulate before a *semantic* endpoint may
/// finalize it. Below this a fragment like "From" can't finalize alone on a
/// short pause — it waits for more speech (or the hard max-silence ceiling).
const MIN_VOICED_MS: u32 = 400;

/// Below this much voiced audio the smart-turn completion probability is
/// discounted: the model is overconfident on tiny clips (observed "From" at
/// p=0.83). Halving keeps a genuine short answer finalizable at a very high
/// prob while stopping mid-sentence fragments from ending the turn.
const SHORT_AUDIO_MS: u32 = 500;

/// Streaming semantic endpointer. Hold one per capture stream; call
/// [`observe`](Self::observe) per frame with the current voice-activity flag
/// and the running partial transcript.
pub struct SemanticEndpointer<M: EndpointModel> {
    model: M,
    threshold: f32,
    sample_rate: u32,
    short_silence_samples: u64,
    max_silence_samples: u64,
    /// Minimum voiced samples before a semantic finalize is allowed.
    min_voiced_samples: u64,
    /// Voiced threshold below which the completion prob is discounted.
    short_audio_samples: u64,
    recent_cap: usize,
    silence_run: u64,
    /// Voiced samples accumulated across the current turn (through pauses).
    voiced_samples: u64,
    checked_this_pause: bool,
    recent_audio: VecDeque<i16>,
    active: bool,
    /// Last completion probability computed this turn (the heuristic's
    /// empty-transcript default until a semantic check runs). Captured into the
    /// finalize snapshot so a max-silence ceiling finalize still reports a prob.
    last_prob: f32,
    /// The endpoint related-data from the most recent finalize, read by the
    /// controller for the `VoiceAnalysis` record.
    last_endpoint: Option<EndpointSnapshot>,
}

impl<M: EndpointModel> SemanticEndpointer<M> {
    /// Build an endpointer.
    ///
    /// `short_silence_ms` is the pause length that triggers a semantic check
    /// (e.g. 250 ms). `max_silence_ms` is the hard ceiling that finalizes
    /// regardless of the model (e.g. 2000 ms). `threshold` is the completion
    /// probability at/above which the turn is finalized (e.g. 0.5).
    pub fn new(
        model: M,
        sample_rate: u32,
        short_silence_ms: u32,
        max_silence_ms: u32,
        threshold: f32,
    ) -> Self {
        let sr = u64::from(sample_rate);
        Self {
            model,
            threshold,
            sample_rate,
            short_silence_samples: sr * u64::from(short_silence_ms) / 1_000,
            max_silence_samples: sr * u64::from(max_silence_ms) / 1_000,
            min_voiced_samples: sr * u64::from(MIN_VOICED_MS) / 1_000,
            short_audio_samples: sr * u64::from(SHORT_AUDIO_MS) / 1_000,
            // Keep ~2 s of recent audio for the model window.
            recent_cap: (sr * 2) as usize,
            silence_run: 0,
            voiced_samples: 0,
            checked_this_pause: false,
            recent_audio: VecDeque::new(),
            active: false,
            last_prob: 0.0,
            last_endpoint: None,
        }
    }

    /// The endpoint related-data from the most recent finalize (probability,
    /// source, silence tail). `None` until the first turn finalizes; consumed
    /// by the controller when assembling the `VoiceAnalysis` record.
    pub fn last_endpoint(&self) -> Option<&EndpointSnapshot> {
        self.last_endpoint.as_ref()
    }

    /// Milliseconds represented by a trailing-silence sample count.
    fn samples_to_ms(&self, samples: u64) -> u64 {
        samples * 1_000 / u64::from(self.sample_rate.max(1))
    }

    /// Record the finalize snapshot (before the turn state is reset).
    fn snapshot(&mut self) {
        self.last_endpoint = Some(EndpointSnapshot {
            completion_prob: self.last_prob,
            source: self.model.source().to_string(),
            silence_tail_ms: self.samples_to_ms(self.silence_run),
        });
    }

    /// Observe one frame.
    ///
    /// `voiced` is whether the VAD considers this frame speech. `partial_text`
    /// is the best transcript so far (may be empty if streaming STT is off).
    /// Returns [`TurnDecision::Finalize`] when the turn should be committed.
    pub async fn observe(
        &mut self,
        frame: &[i16],
        voiced: bool,
        partial_text: &str,
    ) -> TurnDecision {
        // Maintain the recent-audio ring for the model window.
        self.recent_audio.extend(frame.iter().copied());
        while self.recent_audio.len() > self.recent_cap {
            self.recent_audio.pop_front();
        }

        if voiced {
            if !self.active {
                // New turn onset — clear the prior turn's probability so a
                // max-silence finalize can't report a stale value.
                self.last_prob = 0.0;
            }
            self.active = true;
            self.silence_run = 0;
            self.checked_this_pause = false;
            self.voiced_samples = self.voiced_samples.saturating_add(frame.len() as u64);
            return TurnDecision::Continue;
        }

        // Silence frame.
        if !self.active {
            return TurnDecision::Continue; // no turn in progress
        }
        self.silence_run = self.silence_run.saturating_add(frame.len() as u64);

        // Hard ceiling: finalize no matter what the model thinks.
        if self.silence_run >= self.max_silence_samples {
            self.snapshot();
            self.reset_turn();
            return TurnDecision::Finalize;
        }

        // Cross the short-silence threshold once per pause → semantic check.
        if self.silence_run >= self.short_silence_samples && !self.checked_this_pause {
            self.checked_this_pause = true;

            // Min-voiced gate: a fragment shorter than MIN_VOICED_MS can't end
            // the turn on a short pause — it waits for more speech (mid-sentence
            // "From" must not finalize alone). Skip the model call entirely in
            // that case: the smart-turn inference is the heaviest thing on the
            // capture loop (§round-5), so we never run it when it can't finalize.
            if self.voiced_samples >= self.min_voiced_samples {
                let audio: Vec<i16> = self.recent_audio.iter().copied().collect();
                let prob = self.model.completion_prob(&audio, partial_text).await;
                self.last_prob = prob; // report the RAW prob in the record
                // Short-audio discount: smart-turn is overconfident on tiny
                // clips, so halve its prob below SHORT_AUDIO_MS voiced.
                let effective = if self.voiced_samples < self.short_audio_samples {
                    prob * 0.5
                } else {
                    prob
                };
                if effective >= self.threshold {
                    self.snapshot();
                    self.reset_turn();
                    return TurnDecision::Finalize;
                }
            }
            // Still open (too short, or prob below threshold) → keep listening.
        }
        TurnDecision::Continue
    }

    /// Whether a turn is currently in progress.
    pub fn turn_active(&self) -> bool {
        self.active
    }

    fn reset_turn(&mut self) {
        self.active = false;
        self.silence_run = 0;
        self.voiced_samples = 0;
        self.checked_this_pause = false;
        self.recent_audio.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_ms(sr: u32, ms: u32) -> usize {
        (sr as usize) * (ms as usize) / 1_000
    }

    #[tokio::test]
    async fn open_word_pause_does_not_finalize() {
        // 250 ms short, 2000 ms max, threshold 0.5.
        let mut ep = SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 2_000, 0.5);
        let voiced = vec![3_000i16; frame_ms(16_000, 100)];
        let silent = vec![0i16; frame_ms(16_000, 100)];

        // Speak.
        assert_eq!(
            ep.observe(&voiced, true, "tell me about").await,
            TurnDecision::Continue
        );
        // 300 ms pause but the partial ends on an open word ("about") → keep
        // listening (the user is mid-thought).
        ep.observe(&silent, false, "tell me about").await;
        ep.observe(&silent, false, "tell me about").await;
        let d = ep.observe(&silent, false, "tell me about").await;
        assert_eq!(d, TurnDecision::Continue, "must not clip mid-thought");
        assert!(ep.turn_active());
    }

    #[tokio::test]
    async fn complete_sentence_pause_finalizes() {
        let mut ep = SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 2_000, 0.5);
        let voiced = vec![3_000i16; frame_ms(16_000, 100)];
        let silent = vec![0i16; frame_ms(16_000, 100)];
        // 500 ms voiced clears the min-voiced gate (a real sentence, not a blip).
        for _ in 0..5 {
            ep.observe(&voiced, true, "what time is it?").await;
        }
        ep.observe(&silent, false, "what time is it?").await;
        ep.observe(&silent, false, "what time is it?").await;
        let d = ep.observe(&silent, false, "what time is it?").await;
        assert_eq!(d, TurnDecision::Finalize);
        assert!(!ep.turn_active());
    }

    #[tokio::test]
    async fn max_silence_ceiling_finalizes_open_turn() {
        // Even with an open-word partial, exceeding max-silence must finalize.
        let mut ep = SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 500, 0.5);
        let voiced = vec![3_000i16; frame_ms(16_000, 100)];
        let silent = vec![0i16; frame_ms(16_000, 100)];
        ep.observe(&voiced, true, "and").await;
        let mut decisions = Vec::new();
        for _ in 0..8 {
            decisions.push(ep.observe(&silent, false, "and").await);
        }
        assert!(
            decisions.contains(&TurnDecision::Finalize),
            "max-silence ceiling must finalize even an open turn"
        );
    }

    #[tokio::test]
    async fn heuristic_probabilities() {
        let m = HeuristicEndpoint;
        assert!(m.completion_prob(&[], "done.").await > 0.9);
        assert!(m.completion_prob(&[], "wait and").await < 0.3);
        assert!(m.completion_prob(&[], "the quick brown fox").await > 0.5);
        assert!(m.completion_prob(&[], "hold on,").await < 0.2);
    }

    #[tokio::test]
    async fn finalize_captures_endpoint_snapshot() {
        let mut ep = SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 2_000, 0.5);
        let voiced = vec![3_000i16; frame_ms(16_000, 100)];
        let silent = vec![0i16; frame_ms(16_000, 100)];
        assert!(ep.last_endpoint().is_none());
        for _ in 0..5 {
            ep.observe(&voiced, true, "what time is it?").await; // ≥400ms voiced
        }
        ep.observe(&silent, false, "what time is it?").await;
        ep.observe(&silent, false, "what time is it?").await;
        let d = ep.observe(&silent, false, "what time is it?").await;
        assert_eq!(d, TurnDecision::Finalize);
        let snap = ep.last_endpoint().expect("finalize records a snapshot");
        // Terminal punctuation → high completion prob from the heuristic.
        assert!(snap.completion_prob > 0.9, "prob {}", snap.completion_prob);
        assert_eq!(snap.source, "heuristic");
        assert!(snap.silence_tail_ms >= 250, "silence tail {}", snap.silence_tail_ms);
    }

    #[tokio::test]
    async fn max_silence_snapshot_reports_last_prob_and_tail() {
        // Open-word partial never crosses threshold; the max-silence ceiling
        // finalizes and the snapshot still carries the last computed prob.
        let mut ep = SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 500, 0.5);
        let voiced = vec![3_000i16; frame_ms(16_000, 100)];
        let silent = vec![0i16; frame_ms(16_000, 100)];
        ep.observe(&voiced, true, "and").await;
        for _ in 0..8 {
            ep.observe(&silent, false, "and").await;
        }
        let snap = ep.last_endpoint().expect("ceiling finalize records a snapshot");
        assert!(snap.completion_prob < 0.3, "open word prob {}", snap.completion_prob);
        assert!(snap.silence_tail_ms >= 500, "tail {}", snap.silence_tail_ms);
    }

    #[tokio::test]
    async fn silence_without_turn_is_noop() {
        let mut ep = SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 2_000, 0.5);
        let silent = vec![0i16; frame_ms(16_000, 100)];
        for _ in 0..30 {
            assert_eq!(ep.observe(&silent, false, "").await, TurnDecision::Continue);
        }
        assert!(!ep.turn_active());
    }

    /// An endpoint model that always returns a fixed probability — stands in
    /// for smart-turn's overconfidence on tiny clips.
    struct FixedProb(f32);
    #[async_trait]
    impl EndpointModel for FixedProb {
        async fn completion_prob(&self, _recent: &[i16], _partial: &str) -> f32 {
            self.0
        }
    }

    #[tokio::test]
    async fn short_fragment_gate_then_discount_then_finalize() {
        // Round-3 "From" bug: smart-turn returns p=0.83 on a fragment. The
        // min-voiced gate + short-audio discount must stop it finalizing until
        // enough voiced audio has accumulated. max-silence huge so the ceiling
        // never intervenes.
        let mut ep = SemanticEndpointer::new(FixedProb(0.83), 16_000, 250, 10_000, 0.5);
        let v100 = vec![3_000i16; frame_ms(16_000, 100)];
        let v50 = vec![3_000i16; frame_ms(16_000, 50)];
        let silent = vec![0i16; frame_ms(16_000, 100)];

        // (1) 300 ms voiced → below the 400 ms min-voiced gate → must not end.
        for _ in 0..3 {
            ep.observe(&v100, true, "").await;
        }
        ep.observe(&silent, false, "").await;
        ep.observe(&silent, false, "").await;
        let d1 = ep.observe(&silent, false, "").await; // crosses 250ms short-silence
        assert_eq!(
            d1,
            TurnDecision::Continue,
            "300ms fragment must not finalize (min-voiced gate)"
        );

        // (2) accumulate to 450 ms → gate passes, but the short-audio discount
        // (0.83·0.5 = 0.415 < 0.5) still holds the turn open.
        ep.observe(&v100, true, "").await; // → 400ms, resets the pause
        ep.observe(&v50, true, "").await; // → 450ms
        ep.observe(&silent, false, "").await;
        ep.observe(&silent, false, "").await;
        let d2 = ep.observe(&silent, false, "").await;
        assert_eq!(
            d2,
            TurnDecision::Continue,
            "450ms fragment: discounted prob stays below threshold"
        );

        // (3) past 500 ms voiced → no discount → 0.83 ≥ 0.5 → finalize.
        ep.observe(&v100, true, "").await; // → 550ms
        ep.observe(&silent, false, "").await;
        ep.observe(&silent, false, "").await;
        let d3 = ep.observe(&silent, false, "").await;
        assert_eq!(
            d3,
            TurnDecision::Finalize,
            "past 500ms voiced the full prob finalizes"
        );
    }
}
