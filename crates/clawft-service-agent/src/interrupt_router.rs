//! Interrupt router (Wave 2 §W2.2) — the cognitive interrupt/steering brain.
//!
//! Text output has no playback, so the mic stays HOT while the agent works
//! (no AEC/ERL confounds — §W2.1). Every finalized utterance that lands *while
//! the agent is busy* is routed here into an [`InterruptAction`]; the executor
//! (Wave 2 §W2.3, owned by the forest half) turns that action into the actual
//! cancel / prune / Contradicts / resubmit / Backchannel work.
//!
//! **The timing axis is load-bearing.** The SAME words are a normal turn when
//! idle and an interrupt when busy — the router keys on `busy` first. "Busy" =
//! the conversation has an in-flight (Frontier, uncommitted) turn
//! (`talk_loop.rs::current_turn`); the caller reads it and passes it in, so the
//! router stays a pure, deterministic function of its [`InterruptSignals`].
//!
//! Routing is `busy × intent × paralinguistics`, in strict priority order:
//! STOP (explicit cancel) → Backchannel (a "mm-hmm" is not a turn) → Refine
//! (steer the in-flight work) → Queue (default; supersede ONLY on explicit
//! STOP — never guess abandonment).

use crate::dialogue_act::Intent;

/// What to do with a finalized utterance. The executor (§W2.3) maps each to the
/// landed floor machinery; the router only decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterruptAction {
    /// Idle path — a normal turn (`agent.chat`). The mic wasn't busy.
    Turn,
    /// STOP lexicon while busy, with no steering continuation — cancel the
    /// in-flight turn (`agent.chat.cancel` → prune to tombstone + witness).
    Stop,
    /// A correction / steering ask while busy (incl. a STOP that continues into
    /// one, e.g. "hold on, actually do X") — cancel-and-resubmit-with-amendment
    /// (§W2.4). `amendment` is the steering content; the executor combines it
    /// with the original goal and draws the `Contradicts` edge to the pruned
    /// attempt. Injection upgrades behind this same variant later.
    Refine {
        /// The steering content (correction / continuation) the executor combines
        /// with the original goal on resubmit.
        amendment: String,
    },
    /// A backchannel while busy ("mm-hmm", "ok") — NO turn: emit `Backchannel
    /// 0x60` → a `Continuer` cross-ref; the agent keeps working.
    Backchannel,
    /// A fresh, topically-discontinuous request while busy — queue behind the
    /// in-flight turn; it becomes the next turn on commit.
    Queue,
}

/// The signals the router decides on — all projected by the caller from Wave 1 /
/// classification outputs so the router itself has no upstream deps and is
/// deterministically testable.
#[derive(Debug, Clone)]
pub struct InterruptSignals {
    /// The finalized transcript (for the STOP lexicon + as the amendment).
    pub text: String,
    /// Classified intent (`dialogue_act::Intent`, incl. `Correction`).
    pub intent: Intent,
    /// Wave-1 `paralinguistics.class == BackchannelCandidate` (projected to a
    /// bool so the router doesn't depend on clawft-channels).
    pub is_backchannel: bool,
    /// Whether the utterance is very short (few words) — a short `Social` while
    /// busy is a backchannel-grade acknowledgment.
    pub is_short: bool,
    /// Whether the agent has an in-flight (Frontier, uncommitted) turn. THE
    /// load-bearing timing axis.
    pub busy: bool,
    /// Whether the utterance is topically continuous with the in-flight work —
    /// a continuous Request steers (Refine); a discontinuous one queues.
    pub topically_continuous: bool,
}

/// Routes finalized utterances to [`InterruptAction`]s. Holds the STOP lexicon;
/// otherwise stateless.
#[derive(Debug, Clone)]
pub struct InterruptRouter {
    stop_phrases: Vec<&'static str>,
}

/// The STOP lexicon (~15 phrases, the `OPEN_WORDS`/`PATTERNS` idiom). Matched on
/// the normalized (lowercased, punctuation-stripped) transcript as a whole
/// utterance OR as a leading phrase (so "hold on, actually …" still trips it).
/// These are only interrupts *while busy* — idle, "stop" is just a word.
const STOP_PHRASES: &[&str] = &[
    "stop",
    "hold on",
    "hold up",
    "wait",
    "wait wait",
    "cancel",
    "cancel that",
    "never mind",
    "nevermind",
    "forget it",
    "scratch that",
    "abort",
    "halt",
    "belay that",
    "quit",
    "no wait",
    "stop stop",
];

impl Default for InterruptRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl InterruptRouter {
    /// Build a router with the default STOP lexicon.
    pub fn new() -> Self {
        Self {
            stop_phrases: STOP_PHRASES.to_vec(),
        }
    }

    /// Decide the [`InterruptAction`] for a finalized utterance.
    pub fn route(&self, s: &InterruptSignals) -> InterruptAction {
        // Idle: every utterance is a normal turn. The timing axis is the whole
        // point — the same words only interrupt while the agent is busy.
        if !s.busy {
            return InterruptAction::Turn;
        }

        // Busy — priority order matters.

        // 1. Explicit STOP. A bare stop cancels; a stop that CONTINUES into a
        //    correction/request steers (cancel-and-resubmit — the executor
        //    cancels either way), so "hold on, actually do X" is a Refine.
        if let Some(remainder) = self.stop_remainder(&s.text) {
            let steers = s.intent == Intent::Correction
                || (self.is_actionable(s.intent) && !remainder.is_empty());
            if steers {
                let amendment = if remainder.is_empty() {
                    s.text.clone()
                } else {
                    remainder
                };
                return InterruptAction::Refine { amendment };
            }
            return InterruptAction::Stop;
        }

        // 2. Backchannel — a "mm-hmm"/"ok" (non-lexical) or a short social
        //    pleasantry is an acknowledgment, never a turn or a steer.
        if s.is_backchannel || (s.intent == Intent::Social && s.is_short) {
            return InterruptAction::Backchannel;
        }

        // 3. Refinement — a correction, or a topically-continuous ask, steers
        //    the in-flight work.
        if s.intent == Intent::Correction
            || (self.is_actionable(s.intent) && s.topically_continuous)
        {
            return InterruptAction::Refine {
                amendment: s.text.clone(),
            };
        }

        // 4. Default — a fresh, discontinuous request queues behind. Supersede
        //    ONLY on an explicit STOP (handled above); never guess abandonment.
        InterruptAction::Queue
    }

    /// Intents that carry an actionable ask (steer when continuous, queue when
    /// not). A `Statement`/`Feedback`/`Meta` is neither a steer nor a queue-worthy
    /// request.
    fn is_actionable(&self, intent: Intent) -> bool {
        matches!(intent, Intent::Request | Intent::Question)
    }

    /// If `text` opens with a STOP phrase, returns the trimmed remainder after
    /// it (empty string for a bare stop); `None` if no STOP phrase matches. The
    /// remainder is sliced from the ORIGINAL text so the amendment keeps its
    /// casing for the LLM.
    fn stop_remainder(&self, text: &str) -> Option<String> {
        let trimmed = text.trim();
        // Lowercase for matching; STOP phrases are ASCII so byte offsets into the
        // lowercased string map 1:1 onto the (ASCII-prefixed) original.
        let lower = trimmed.to_lowercase();
        let bare = normalize(text);
        // Longest phrase first so "hold on" wins over a hypothetical "hold".
        let mut phrases = self.stop_phrases.clone();
        phrases.sort_by_key(|p| std::cmp::Reverse(p.len()));
        for p in phrases {
            if bare == p {
                return Some(String::new());
            }
            if let Some(rest) = lower.strip_prefix(p) {
                // Require a word boundary so "stopwatch" doesn't match "stop".
                if rest.starts_with(' ') || rest.starts_with(',') {
                    let orig_rest = &trimmed[p.len()..];
                    return Some(orig_rest.trim_start_matches([' ', ',']).trim().to_string());
                }
            }
        }
        None
    }
}

/// Lowercase, collapse whitespace, drop trailing sentence punctuation — so
/// "Hold on!" and "hold on" match the same lexicon entry.
fn normalize(text: &str) -> String {
    let lowered = text.trim().to_lowercase();
    let collapsed = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(text: &str, intent: Intent, busy: bool) -> InterruptSignals {
        InterruptSignals {
            text: text.to_string(),
            intent,
            is_backchannel: false,
            is_short: false,
            busy,
            topically_continuous: false,
        }
    }

    #[test]
    fn timing_axis_same_words_idle_is_a_turn_busy_is_an_interrupt() {
        let r = InterruptRouter::new();
        // Idle: "stop" is just a normal turn.
        assert_eq!(r.route(&sig("stop", Intent::Request, false)), InterruptAction::Turn);
        // Busy: the same word cancels.
        assert_eq!(r.route(&sig("stop", Intent::Request, true)), InterruptAction::Stop);
    }

    #[test]
    fn idle_never_interrupts_regardless_of_content() {
        let r = InterruptRouter::new();
        for (text, intent) in [
            ("hold on", Intent::Meta),
            ("actually make it blue", Intent::Correction),
            ("mm-hmm", Intent::Social),
            ("what time is it", Intent::Question),
        ] {
            assert_eq!(r.route(&sig(text, intent, false)), InterruptAction::Turn, "{text}");
        }
    }

    #[test]
    fn bare_stop_while_busy_cancels() {
        let r = InterruptRouter::new();
        for text in ["stop", "hold on", "wait", "cancel", "never mind", "nevermind", "forget it"] {
            assert_eq!(r.route(&sig(text, Intent::Meta, true)), InterruptAction::Stop, "{text}");
        }
    }

    #[test]
    fn stop_lexicon_is_punctuation_and_case_insensitive() {
        let r = InterruptRouter::new();
        assert_eq!(r.route(&sig("Hold on!", Intent::Meta, true)), InterruptAction::Stop);
        assert_eq!(r.route(&sig("  STOP.  ", Intent::Meta, true)), InterruptAction::Stop);
    }

    #[test]
    fn stop_does_not_match_a_word_that_merely_contains_it() {
        let r = InterruptRouter::new();
        // "stopwatch" must NOT trip the "stop" lexicon → falls through to Queue
        // (a fresh discontinuous request while busy).
        let s = sig("stopwatch feature please", Intent::Request, true);
        assert_eq!(r.route(&s), InterruptAction::Queue);
    }

    #[test]
    fn stop_that_continues_into_a_correction_is_a_refine() {
        let r = InterruptRouter::new();
        // The exit-test utterance: cancel AND resubmit with the amendment.
        let s = sig("hold on, actually do X instead", Intent::Correction, true);
        assert_eq!(
            r.route(&s),
            InterruptAction::Refine { amendment: "actually do X instead".to_string() }
        );
    }

    #[test]
    fn stop_that_continues_into_a_request_is_a_refine() {
        let r = InterruptRouter::new();
        let s = sig("wait, use the other file", Intent::Request, true);
        assert_eq!(
            r.route(&s),
            InterruptAction::Refine { amendment: "use the other file".to_string() }
        );
    }

    #[test]
    fn correction_while_busy_refines() {
        let r = InterruptRouter::new();
        let s = sig("actually make it blue", Intent::Correction, true);
        assert_eq!(
            r.route(&s),
            InterruptAction::Refine { amendment: "actually make it blue".to_string() }
        );
    }

    #[test]
    fn continuous_request_refines_discontinuous_queues() {
        let r = InterruptRouter::new();
        let mut cont = sig("also sort it descending", Intent::Request, true);
        cont.topically_continuous = true;
        assert!(matches!(r.route(&cont), InterruptAction::Refine { .. }));

        let mut disc = sig("what's the weather", Intent::Question, true);
        disc.topically_continuous = false;
        assert_eq!(r.route(&disc), InterruptAction::Queue);
    }

    #[test]
    fn backchannel_while_busy_is_a_continuer_not_a_turn() {
        let r = InterruptRouter::new();
        let mut bc = sig("mm-hmm", Intent::Social, true);
        bc.is_backchannel = true;
        assert_eq!(r.route(&bc), InterruptAction::Backchannel);

        // Short social pleasantry counts even without the paralinguistic flag.
        let mut short_social = sig("ok", Intent::Social, true);
        short_social.is_short = true;
        assert_eq!(r.route(&short_social), InterruptAction::Backchannel);
    }

    #[test]
    fn stop_takes_priority_over_backchannel_flag() {
        let r = InterruptRouter::new();
        // Even if paralinguistics mislabels a "stop" as backchannel-ish, the
        // explicit lexicon wins — cancelling must never be swallowed.
        let mut s = sig("stop", Intent::Social, true);
        s.is_backchannel = true;
        assert_eq!(r.route(&s), InterruptAction::Stop);
    }
}
