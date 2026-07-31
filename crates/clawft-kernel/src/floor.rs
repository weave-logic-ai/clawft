//! Conversational floor as a **scored read**, not an engine (ADR-062 D4).
//!
//! Each tick the Talk-Mode loop (Phase 2) reads who should hold the floor by
//! scoring the contending candidates with [`compute_urgency`] and folding the
//! result into a [`FloorState`]. Nothing here owns state-machine orchestration:
//! it is a pure function of the [`ImpulseQueue`](crate::impulse::ImpulseQueue)
//! entries (the turn signals) crossed with the live frontier
//! ([`SessionView::live_seqs`](crate::context_graft::SessionView::live_seqs)).
//!
//! The weights are **constants** in v0.1 (hand-tuned, ADR-062 §D4); the Weaver
//! learns them later (deferred backlog). A hard interrupt short-circuits the
//! score to [`f32::MAX`] so it preempts everything and cancels generation.

use crate::impulse::{Impulse, ImpulseType};

// ── Urgency weights (ADR-062 D4 — sum to 1.0) ────────────────────────────────

/// Weight on semantic relevance to the frontier head.
pub const W_SEMANTIC: f32 = 0.30;
/// Weight on the speaker's emotional arousal (VAD arousal).
pub const W_AROUSAL: f32 = 0.15;
/// Weight on fairness pressure (how long this candidate has waited).
pub const W_WAIT: f32 = 0.20;
/// Weight on crowd density (number of contending actors).
pub const W_CROWD: f32 = 0.10;
/// Weight on content readiness (generated > partial > not-started).
pub const W_READINESS: f32 = 0.25;

/// Crowd-density saturation point: at/above this many contenders the crowd
/// signal reads `1.0`.
pub const MAX_CROWD: usize = 4;

/// Default ERL confidence floor for admitting a barge (ADR-068 D1 / WEFT-628).
///
/// A `TurnClaim` whose edge-measured echo-return-loss confidence is **below**
/// this value is treated as self-echo (AEC leakage of the bot's own reply) and
/// must not win the floor — the principled self-cancel fix. Exposed as a knob
/// so Phase-1 sim data and live loud-room verification can retune it.
pub const ERL_BARGE_FLOOR: f32 = 0.50;

/// Whether a measured ERL confidence clears the barge floor (WEFT-628).
///
/// Used by the duplex / Talk-Mode loop when translating a provisional `Overlap`
/// into an ECC [`FloorVerdict`](crate::duplex::FloorVerdict): high ERL →
/// `GrantUser`, low ERL → `DenyBarge`.
#[inline]
pub fn erl_admits_barge(erl: f32) -> bool {
    erl.is_finite() && erl >= ERL_BARGE_FLOOR
}

// ── Content readiness ────────────────────────────────────────────────────────

/// How far along a candidate's response content is (ADR-062 D4). This is the
/// term that ties floor-winning to speculative-generation progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentReadiness {
    /// Nothing generated yet.
    NotStarted,
    /// Partially generated (e.g. a streaming speculative draft in flight).
    Partial,
    /// Fully generated and ready to render.
    Generated,
}

impl ContentReadiness {
    /// Normalized score in `[0, 1]`.
    pub fn score(self) -> f32 {
        match self {
            ContentReadiness::NotStarted => 0.0,
            ContentReadiness::Partial => 0.5,
            ContentReadiness::Generated => 1.0,
        }
    }
}

// ── Urgency signals ──────────────────────────────────────────────────────────

/// The scored inputs (plus the hard-interrupt flag) feeding
/// [`compute_urgency`]. Each `f32` is expected in `[0, 1]`; values are clamped.
///
/// ## ERL term (ADR-068 D1 / WEFT-628)
///
/// Acoustic barge claims carry an optional [`erl_confidence`](Self::erl_confidence)
/// scalar streamed up by the thin edge's AEC. When present, it **multiplies** the
/// base ADR-062 D4 score so low-ERL (self-echo) claims cannot win the floor —
/// the principled self-cancel fix. Text / non-acoustic contenders leave this
/// `None` and keep the unscaled base score.
#[derive(Debug, Clone, Copy)]
pub struct UrgencySignals {
    /// `cosine(candidate, frontier head)` — relevance to what is being said.
    pub semantic_relevance: f32,
    /// VAD arousal of this actor.
    pub emotional_arousal: f32,
    /// Normalized fairness pressure (longer wait ⇒ higher).
    pub wait_time: f32,
    /// Normalized contending-actor count (see [`crowd_density`]).
    pub crowd_density: f32,
    /// Readiness of this candidate's content.
    pub content_readiness: ContentReadiness,
    /// Hard interrupt — preempt everything, cancel in-flight generation.
    pub hard_interrupt: bool,
    /// Optional echo-return-loss confidence from the edge AEC (ADR-068 D1).
    ///
    /// `Some(v)` scales the base urgency by `v.clamp(0,1)` so a low-ERL
    /// `TurnClaim` (bot self-echo past AEC) is weighted down. `None` means
    /// "not an acoustic barge claim" and leaves the base score unchanged.
    pub erl_confidence: Option<f32>,
}

impl Default for UrgencySignals {
    fn default() -> Self {
        Self {
            semantic_relevance: 0.0,
            emotional_arousal: 0.0,
            wait_time: 0.0,
            crowd_density: 0.0,
            content_readiness: ContentReadiness::NotStarted,
            hard_interrupt: false,
            erl_confidence: None,
        }
    }
}

/// Score a candidate's urgency for the floor (ADR-062 D4 + ADR-068 ERL term).
///
/// ```text
/// base    = 0.30·semantic + 0.15·arousal + 0.20·wait + 0.10·crowd + 0.25·readiness
/// urgency = base · erl_scale            ; erl_scale = erl.clamp(0,1) or 1.0 if None
///         ; hard interrupt ⇒ f32::MAX
/// ```
pub fn compute_urgency(s: &UrgencySignals) -> f32 {
    if s.hard_interrupt {
        return f32::MAX;
    }
    let c = |v: f32| v.clamp(0.0, 1.0);
    let base = W_SEMANTIC * c(s.semantic_relevance)
        + W_AROUSAL * c(s.emotional_arousal)
        + W_WAIT * c(s.wait_time)
        + W_CROWD * c(s.crowd_density)
        + W_READINESS * s.content_readiness.score();
    // ADR-068 D1: low ERL confidence weights a TurnClaim down so self-echo
    // cannot win the floor. Non-acoustic candidates leave erl_confidence None.
    let erl_scale = match s.erl_confidence {
        Some(erl) if erl.is_finite() => erl.clamp(0.0, 1.0),
        _ => 1.0,
    };
    base * erl_scale
}

/// Map a duplex `TurnClaim` ERL into a floor verdict scaffold (WEFT-628).
///
/// The full Talk-Mode loop still owns when to *ask* this; the helper is the
/// pure ERL branch of ADR-068 D1 so unit tests and the thin-edge loopback can
/// prove self-cancel without the full daemon.
#[inline]
pub fn floor_verdict_from_erl(erl: Option<f32>) -> crate::duplex::FloorVerdict {
    match erl {
        Some(v) if erl_admits_barge(v) => crate::duplex::FloorVerdict::GrantUser,
        Some(_) => crate::duplex::FloorVerdict::DenyBarge,
        // No ERL yet — treat as unknown/low and deny (safe default for self-cancel).
        None => crate::duplex::FloorVerdict::DenyBarge,
    }
}

/// Normalized crowd-density signal in `[0, 1]` from the number of contending
/// actors, saturating at [`MAX_CROWD`]. Derived from the
/// [`ImpulseQueue`](crate::impulse::ImpulseQueue) (see [`contending_count`]).
pub fn crowd_density(contenders: usize) -> f32 {
    if MAX_CROWD == 0 {
        return 0.0;
    }
    (contenders as f32 / MAX_CROWD as f32).clamp(0.0, 1.0)
}

/// Count the floor-claiming impulses in a drained set — every
/// [`ImpulseType::TurnClaim`] is a contender for the floor (ADR-062 D5). Feeds
/// [`crowd_density`].
pub fn contending_count(impulses: &[Impulse]) -> usize {
    impulses
        .iter()
        .filter(|i| i.impulse_type == ImpulseType::TurnClaim)
        .count()
}

// ── Floor state & decision ───────────────────────────────────────────────────

/// A candidate contending for the floor: an identity (actor/speaker) plus its
/// scored signals and the HLC time it began contending.
#[derive(Debug, Clone)]
pub struct FloorCandidate {
    /// Actor/speaker identity (or frontier seq tag).
    pub id: String,
    /// HLC timestamp this candidate began contending (for `Held.since`).
    pub since: u64,
    /// Scored urgency inputs.
    pub signals: UrgencySignals,
}

impl FloorCandidate {
    /// Convenience constructor.
    pub fn new(id: impl Into<String>, since: u64, signals: UrgencySignals) -> Self {
        Self {
            id: id.into(),
            since,
            signals,
        }
    }

    /// This candidate's urgency score.
    pub fn urgency(&self) -> f32 {
        compute_urgency(&self.signals)
    }
}

/// The status of the conversational floor (ADR-062 D4 §8.3).
#[derive(Debug, Clone, PartialEq)]
pub enum FloorState {
    /// No one is contending — the floor is free.
    Open,
    /// One clear holder, since the given HLC time.
    Held {
        /// The holder's identity.
        holder: String,
        /// HLC time the hold began.
        since: u64,
    },
    /// Several actors contending (ordered by descending urgency).
    Contended {
        /// Contending identities, highest urgency first.
        candidates: Vec<String>,
    },
    /// Preempted/locked (e.g. a hard interrupt cancelled generation).
    Locked {
        /// Why the floor is locked.
        reason: String,
    },
}

/// The outcome of a floor read: the resulting [`FloorState`], the scored winner
/// (if any), and the winning urgency score.
#[derive(Debug, Clone)]
pub struct FloorDecision {
    /// Resulting floor status.
    pub state: FloorState,
    /// The winning candidate's id, if the floor is taken/contended/locked.
    pub winner: Option<String>,
    /// The winning urgency score (`0.0` when the floor is Open).
    pub score: f32,
}

/// Read the floor from the current set of contending candidates (ADR-062 D4).
///
/// - **No candidates** → [`FloorState::Open`].
/// - **A hard interrupt present** → [`FloorState::Locked`], the interrupting
///   candidate wins with [`f32::MAX`] (preempt + cancel generation).
/// - **One candidate** → [`FloorState::Held`].
/// - **Several candidates** → [`FloorState::Contended`] (ordered by urgency),
///   the top scorer is the winner.
///
/// Ties are broken by input order (earliest-listed wins), so callers should
/// supply candidates oldest-wait-first for deterministic fairness.
pub fn evaluate_floor(candidates: &[FloorCandidate]) -> FloorDecision {
    if candidates.is_empty() {
        return FloorDecision {
            state: FloorState::Open,
            winner: None,
            score: 0.0,
        };
    }

    // Hard interrupt preempts everything.
    if let Some(c) = candidates.iter().find(|c| c.signals.hard_interrupt) {
        return FloorDecision {
            state: FloorState::Locked {
                reason: format!("hard-interrupt: {}", c.id),
            },
            winner: Some(c.id.clone()),
            score: f32::MAX,
        };
    }

    // Rank by urgency (stable: first-listed wins ties).
    let mut ranked: Vec<&FloorCandidate> = candidates.iter().collect();
    ranked.sort_by(|a, b| {
        b.urgency()
            .partial_cmp(&a.urgency())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let top = ranked[0];

    if candidates.len() == 1 {
        return FloorDecision {
            state: FloorState::Held {
                holder: top.id.clone(),
                since: top.since,
            },
            winner: Some(top.id.clone()),
            score: top.urgency(),
        };
    }

    FloorDecision {
        state: FloorState::Contended {
            candidates: ranked.iter().map(|c| c.id.clone()).collect(),
        },
        winner: Some(top.id.clone()),
        score: top.urgency(),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impulse::ImpulseQueue;

    fn signals(sem: f32, aro: f32, wait: f32, crowd: f32, r: ContentReadiness) -> UrgencySignals {
        UrgencySignals {
            semantic_relevance: sem,
            emotional_arousal: aro,
            wait_time: wait,
            crowd_density: crowd,
            content_readiness: r,
            hard_interrupt: false,
            erl_confidence: None,
        }
    }

    #[test]
    fn urgency_matches_hand_computed() {
        // 0.30*1 + 0.15*0.8 + 0.20*0.5 + 0.10*0.25 + 0.25*0.5
        // = 0.30 + 0.12 + 0.10 + 0.025 + 0.125 = 0.67
        let s = signals(1.0, 0.8, 0.5, 0.25, ContentReadiness::Partial);
        assert!((compute_urgency(&s) - 0.67).abs() < 1e-6);
    }

    #[test]
    fn urgency_clamps_out_of_range_inputs() {
        // Over-range inputs clamp to 1.0; readiness Generated = 1.0.
        let s = signals(2.0, 5.0, 9.0, 9.0, ContentReadiness::Generated);
        // All terms saturate ⇒ sum of weights = 1.0.
        assert!((compute_urgency(&s) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn hard_interrupt_is_max() {
        let mut s = signals(0.0, 0.0, 0.0, 0.0, ContentReadiness::NotStarted);
        s.hard_interrupt = true;
        assert_eq!(compute_urgency(&s), f32::MAX);
    }

    /// WEFT-628 / ADR-068 D1: low ERL confidence scales a barge claim toward zero
    /// so self-echo cannot win the floor.
    #[test]
    fn low_erl_weights_urgency_down() {
        let mut s = signals(1.0, 1.0, 1.0, 1.0, ContentReadiness::Generated);
        // Base without ERL saturates to 1.0.
        assert!((compute_urgency(&s) - 1.0).abs() < 1e-6);

        s.erl_confidence = Some(0.1); // self-echo territory
        assert!((compute_urgency(&s) - 0.1).abs() < 1e-6);

        s.erl_confidence = Some(0.0);
        assert!((compute_urgency(&s) - 0.0).abs() < 1e-6);

        s.erl_confidence = Some(0.9); // genuine barge
        assert!((compute_urgency(&s) - 0.9).abs() < 1e-6);
    }

    #[test]
    fn erl_barge_floor_gate() {
        assert!(!erl_admits_barge(0.0));
        assert!(!erl_admits_barge(ERL_BARGE_FLOOR - 0.01));
        assert!(erl_admits_barge(ERL_BARGE_FLOOR));
        assert!(erl_admits_barge(0.99));
        assert!(!erl_admits_barge(f32::NAN));
    }

    #[test]
    fn floor_verdict_from_erl_self_cancel() {
        use crate::duplex::FloorVerdict;
        assert_eq!(
            floor_verdict_from_erl(Some(0.1)),
            FloorVerdict::DenyBarge,
            "low ERL → DenyBarge (self-cancel fix)"
        );
        assert_eq!(
            floor_verdict_from_erl(Some(0.9)),
            FloorVerdict::GrantUser,
            "high ERL → GrantUser"
        );
        assert_eq!(
            floor_verdict_from_erl(None),
            FloorVerdict::DenyBarge,
            "missing ERL defaults safe (deny)"
        );
    }

    #[test]
    fn crowd_density_saturates() {
        assert_eq!(crowd_density(0), 0.0);
        assert!((crowd_density(2) - 0.5).abs() < 1e-6);
        assert_eq!(crowd_density(MAX_CROWD), 1.0);
        assert_eq!(crowd_density(MAX_CROWD + 10), 1.0);
    }

    #[test]
    fn empty_floor_is_open() {
        let d = evaluate_floor(&[]);
        assert_eq!(d.state, FloorState::Open);
        assert!(d.winner.is_none());
        assert_eq!(d.score, 0.0);
    }

    #[test]
    fn single_candidate_holds_floor() {
        let c = FloorCandidate::new(
            "alice",
            42,
            signals(0.5, 0.0, 0.0, 0.0, ContentReadiness::NotStarted),
        );
        let d = evaluate_floor(std::slice::from_ref(&c));
        assert_eq!(
            d.state,
            FloorState::Held {
                holder: "alice".into(),
                since: 42
            }
        );
        assert_eq!(d.winner.as_deref(), Some("alice"));
        assert!((d.score - 0.15).abs() < 1e-6); // 0.30 * 0.5
    }

    /// The scored winner among contenders matches the hand-computed ranking.
    #[test]
    fn contended_floor_picks_highest_urgency() {
        // bob: 0.30*0.9 = 0.27 ; carol: 0.25*1.0 = 0.25 ; alice: 0.20*0.5 = 0.10
        let alice = FloorCandidate::new(
            "alice",
            1,
            signals(0.0, 0.0, 0.5, 0.0, ContentReadiness::NotStarted),
        );
        let bob = FloorCandidate::new(
            "bob",
            2,
            signals(0.9, 0.0, 0.0, 0.0, ContentReadiness::NotStarted),
        );
        let carol = FloorCandidate::new(
            "carol",
            3,
            signals(0.0, 0.0, 0.0, 0.0, ContentReadiness::Generated),
        );
        let d = evaluate_floor(&[alice, bob, carol]);
        assert_eq!(d.winner.as_deref(), Some("bob"));
        assert!((d.score - 0.27).abs() < 1e-6);
        match d.state {
            FloorState::Contended { candidates } => {
                assert_eq!(candidates, vec!["bob", "carol", "alice"]);
            }
            other => panic!("expected Contended, got {other:?}"),
        }
    }

    /// A hard interrupt preempts a higher-scoring ordinary candidate.
    #[test]
    fn hard_interrupt_locks_and_preempts() {
        let speaker = FloorCandidate::new(
            "speaker",
            1,
            signals(1.0, 1.0, 1.0, 1.0, ContentReadiness::Generated),
        );
        let mut barge = signals(0.0, 0.0, 0.0, 0.0, ContentReadiness::NotStarted);
        barge.hard_interrupt = true;
        let interrupter = FloorCandidate::new("interrupter", 2, barge);

        let d = evaluate_floor(&[speaker, interrupter]);
        assert_eq!(d.winner.as_deref(), Some("interrupter"));
        assert_eq!(d.score, f32::MAX);
        assert_eq!(
            d.state,
            FloorState::Locked {
                reason: "hard-interrupt: interrupter".into()
            }
        );
    }

    /// End-to-end: the floor read operates over ImpulseQueue entries crossed
    /// with the live frontier seqs (ADR-062 D4 inputs).
    #[test]
    fn floor_read_over_queue_and_frontier() {
        // Two speakers claim the floor; queue drives crowd density.
        let q = ImpulseQueue::new();
        let a = [1u8; 32];
        let b = [2u8; 32];
        q.emit(0, a, 0, ImpulseType::TurnClaim, serde_json::json!({}), 10);
        q.emit(0, b, 0, ImpulseType::TurnClaim, serde_json::json!({}), 20);
        // A backchannel is NOT a floor claim.
        q.emit(0, a, 0, ImpulseType::Backchannel, serde_json::json!({}), 15);

        let drained = q.drain_ready();
        let contenders = contending_count(&drained);
        assert_eq!(contenders, 2);
        let crowd = crowd_density(contenders); // 2/4 = 0.5

        // Frontier seqs are the live candidate content (here: two turns).
        let live_seqs = [101u64, 202u64];
        let candidates: Vec<FloorCandidate> = live_seqs
            .iter()
            .enumerate()
            .map(|(i, seq)| {
                let readiness = if i == 0 {
                    ContentReadiness::Generated
                } else {
                    ContentReadiness::NotStarted
                };
                FloorCandidate::new(
                    format!("seq-{seq}"),
                    *seq,
                    signals(0.2, 0.0, 0.0, crowd, readiness),
                )
            })
            .collect();

        let d = evaluate_floor(&candidates);
        // seq-101: 0.30*0.2 + 0.10*0.5 + 0.25*1.0 = 0.06 + 0.05 + 0.25 = 0.36
        assert_eq!(d.winner.as_deref(), Some("seq-101"));
        assert!((d.score - 0.36).abs() < 1e-6);
    }
}
