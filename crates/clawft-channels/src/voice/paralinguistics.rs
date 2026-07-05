//! Coarse paralinguistic classification (Voice Phase 1, Wave 1 §W1.2).
//!
//! When the STT transcript is empty/near-empty but there is voiced energy
//! (laughter, "mm-hmm", a sigh, a filled pause), the utterance is a
//! **non-lexical vocalization**, not speech. This rule-based classifier reads
//! signals already in the record — short `voiced_ms`, the energy-burst
//! envelope, f0 presence, and the (near-)empty transcript — into a coarse
//! `class`. It is the groundwork Wave 3's DuplexChannel `Backchannel` verdict
//! (ADR-062 D5 / ADR-068 D1) consumes; designing the field + a coarse
//! classifier now means storage/surface never reshape when the real
//! classifier lands.
//!
//! Coarse by construction → `Confidence::Low`. No model, no audio dependency.

use super::analysis::{Confidence, ParalinguisticClass, ParalinguisticsAnalysis};

/// A short standalone lexical token that is really a filled pause / backchannel
/// even though STT emitted text for it.
const FILLER_WORDS: &[&str] = &["um", "uh", "erm", "hmm", "mm", "mhm", "mmhm", "uhhuh"];

/// Backchannel-ish acknowledgement tokens ("mm-hmm", "yeah", "uh-huh").
const BACKCHANNEL_WORDS: &[&str] = &["mhm", "mmhm", "uhhuh", "yeah", "yep", "yup", "okay", "ok"];

/// Below this voiced duration a non-lexical burst is a backchannel/filler
/// rather than an utterance (ms).
const SHORT_MS: u64 = 700;

/// Energy-dynamics (dB) at/above which a non-lexical, near-empty burst looks
/// like laughter (rhythmic loud/soft) rather than a flat "mm-hmm".
const LAUGHTER_DYNAMICS_DB: f32 = 14.0;

/// The signals the classifier reads — all already computed for the record.
pub struct ParalinguisticInput<'a> {
    /// The finalized transcript (may be empty on a non-lexical turn).
    pub transcript: &'a str,
    /// Voiced audio duration (ms).
    pub voiced_ms: u64,
    /// Energy envelope range (dB) from prosody / capture health.
    pub energy_dynamics_db: f32,
    /// Whether the pitch track found a voiced f0.
    pub has_f0: bool,
}

/// Classify an utterance's paralinguistic class from its coarse signals.
pub fn classify_paralinguistics(input: &ParalinguisticInput) -> ParalinguisticsAnalysis {
    let words: Vec<String> = input
        .transcript
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase())
        .collect();
    let near_empty = words.is_empty();
    let single = words.len() == 1;
    let short = input.voiced_ms <= SHORT_MS;

    // Non-lexical: no words at all but there was voiced energy → a vocalization.
    if near_empty {
        // Only treat it as a genuine non-lexical vocalization if there was
        // real voiced content; otherwise it is unknown (noise blip).
        let non_lexical = input.has_f0 || input.voiced_ms > 0;
        let class = if !non_lexical {
            ParalinguisticClass::Unknown
        } else if input.energy_dynamics_db >= LAUGHTER_DYNAMICS_DB && short {
            ParalinguisticClass::LaughterCandidate
        } else if short {
            ParalinguisticClass::BackchannelCandidate
        } else {
            ParalinguisticClass::Unknown
        };
        return ParalinguisticsAnalysis {
            non_lexical,
            class,
            confidence: Confidence::Low,
        };
    }

    // A single short filler/backchannel word standing alone is paralinguistic
    // even though STT lexicalized it.
    if single && short {
        let w = words[0].as_str();
        if BACKCHANNEL_WORDS.contains(&w) {
            return ParalinguisticsAnalysis {
                non_lexical: true,
                class: ParalinguisticClass::BackchannelCandidate,
                confidence: Confidence::Low,
            };
        }
        if FILLER_WORDS.contains(&w) {
            return ParalinguisticsAnalysis {
                non_lexical: true,
                class: ParalinguisticClass::Filler,
                confidence: Confidence::Low,
            };
        }
    }

    // Otherwise: ordinary lexical speech.
    ParalinguisticsAnalysis {
        non_lexical: false,
        class: ParalinguisticClass::Speech,
        confidence: Confidence::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_speech_is_speech() {
        let r = classify_paralinguistics(&ParalinguisticInput {
            transcript: "what time is the meeting",
            voiced_ms: 1_500,
            energy_dynamics_db: 10.0,
            has_f0: true,
        });
        assert_eq!(r.class, ParalinguisticClass::Speech);
        assert!(!r.non_lexical);
    }

    #[test]
    fn empty_short_voiced_is_backchannel_candidate() {
        let r = classify_paralinguistics(&ParalinguisticInput {
            transcript: "",
            voiced_ms: 300,
            energy_dynamics_db: 6.0,
            has_f0: true,
        });
        assert!(r.non_lexical);
        assert_eq!(r.class, ParalinguisticClass::BackchannelCandidate);
    }

    #[test]
    fn empty_short_dynamic_burst_is_laughter_candidate() {
        let r = classify_paralinguistics(&ParalinguisticInput {
            transcript: "  ",
            voiced_ms: 500,
            energy_dynamics_db: 20.0,
            has_f0: true,
        });
        assert!(r.non_lexical);
        assert_eq!(r.class, ParalinguisticClass::LaughterCandidate);
    }

    #[test]
    fn standalone_filler_word_is_filler() {
        let r = classify_paralinguistics(&ParalinguisticInput {
            transcript: "um",
            voiced_ms: 400,
            energy_dynamics_db: 4.0,
            has_f0: true,
        });
        assert!(r.non_lexical);
        assert_eq!(r.class, ParalinguisticClass::Filler);
    }

    #[test]
    fn standalone_yeah_is_backchannel() {
        let r = classify_paralinguistics(&ParalinguisticInput {
            transcript: "yeah",
            voiced_ms: 350,
            energy_dynamics_db: 5.0,
            has_f0: true,
        });
        assert_eq!(r.class, ParalinguisticClass::BackchannelCandidate);
    }

    #[test]
    fn empty_silent_blip_is_unknown() {
        let r = classify_paralinguistics(&ParalinguisticInput {
            transcript: "",
            voiced_ms: 0,
            energy_dynamics_db: 0.0,
            has_f0: false,
        });
        assert!(!r.non_lexical);
        assert_eq!(r.class, ParalinguisticClass::Unknown);
    }
}
