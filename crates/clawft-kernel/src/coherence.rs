//! Conversation coherence as a **soft dampener**, not a hard gate (ADR-062 D6).
//!
//! The RSTE coherence read (`engines/04-RSTE.md §5.2`) scores how well the
//! conversation graph hangs together. Below a threshold the Talk-Mode loop
//! (Phase 2) does **not** block — it nudges, multiplying positive urgency
//! scores by [`DAMPEN_FACTOR`] (×0.8) until coherence recovers. The dampener is
//! hysteretic ("until recovery"): once tripped it stays engaged through the
//! drift band and only clears when coherence climbs back to the recovery line.
//!
//! [`ImpulseType::CoherenceAlert`](crate::impulse::ImpulseType::CoherenceAlert)
//! already exists; emitting it on a [`CoherenceBand::Warning`]/`Break` is Phase
//! 2 work. This module is the read + the dampener only.

// ── Coherence weights (ADR-062 D6 / RSTE §5.2 — sum to 1.0) ───────────────────

/// Weight on relation coverage (fraction of expected relations present).
pub const W_RELATION_COVERAGE: f32 = 0.30;
/// Weight on mean relation confidence.
pub const W_RELATION_CONFIDENCE: f32 = 0.25;
/// Weight on structural connectivity of the graph.
pub const W_STRUCTURAL_CONNECTIVITY: f32 = 0.20;
/// Weight on QAP (question–answer pair) closure.
pub const W_QAP_CLOSURE: f32 = 0.15;
/// Weight on topic continuity across turns.
pub const W_TOPIC_CONTINUITY: f32 = 0.10;

/// The soft-dampening multiplier applied to positive scores while drifting.
pub const DAMPEN_FACTOR: f32 = 0.8;

/// Default coherence below which the dampener trips (the Strong/MildDrift
/// boundary's lower neighbour — entering Warning territory).
pub const DEFAULT_DAMPEN_BELOW: f32 = 0.65;
/// Default coherence at/above which the dampener clears (back to Strong).
pub const DEFAULT_RECOVER_AT: f32 = 0.85;

// ── Coherence signals & read ─────────────────────────────────────────────────

/// The five RSTE inputs feeding [`compute_coherence`]. Each `f32` is expected in
/// `[0, 1]`; values are clamped.
#[derive(Debug, Clone, Copy)]
pub struct CoherenceSignals {
    /// Fraction of expected relations actually present.
    pub relation_coverage: f32,
    /// Mean confidence of the present relations.
    pub relation_confidence: f32,
    /// Structural connectivity of the conversation graph.
    pub structural_connectivity: f32,
    /// Question–answer pair closure.
    pub qap_closure: f32,
    /// Topic continuity across turns.
    pub topic_continuity: f32,
}

impl CoherenceSignals {
    /// Coherence in `[0, 1]` (see [`compute_coherence`]).
    pub fn score(&self) -> f32 {
        compute_coherence(self)
    }
}

/// Weighted RSTE coherence read in `[0, 1]` (ADR-062 D6).
///
/// ```text
/// coherence = 0.30·coverage + 0.25·confidence + 0.20·connectivity
///           + 0.15·qap_closure + 0.10·topic_continuity
/// ```
pub fn compute_coherence(s: &CoherenceSignals) -> f32 {
    let c = |v: f32| v.clamp(0.0, 1.0);
    W_RELATION_COVERAGE * c(s.relation_coverage)
        + W_RELATION_CONFIDENCE * c(s.relation_confidence)
        + W_STRUCTURAL_CONNECTIVITY * c(s.structural_connectivity)
        + W_QAP_CLOSURE * c(s.qap_closure)
        + W_TOPIC_CONTINUITY * c(s.topic_continuity)
}

/// Coherence band (ADR-062 D6 bands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoherenceBand {
    /// ≥ 0.85 — strong, no action.
    Strong,
    /// 0.65–0.85 — mild drift; floor weights toward coherence-restorers.
    MildDrift,
    /// 0.40–0.65 — warning; refocus prompt (would emit `CoherenceAlert`).
    Warning,
    /// < 0.40 — break (would emit `CoherenceAlert`).
    Break,
}

impl CoherenceBand {
    /// Classify a coherence score into its band.
    pub fn from_score(coherence: f32) -> Self {
        if coherence >= 0.85 {
            CoherenceBand::Strong
        } else if coherence >= 0.65 {
            CoherenceBand::MildDrift
        } else if coherence >= 0.40 {
            CoherenceBand::Warning
        } else {
            CoherenceBand::Break
        }
    }

    /// Whether this band warrants a `CoherenceAlert` (Warning or Break).
    pub fn is_alerting(self) -> bool {
        matches!(self, CoherenceBand::Warning | CoherenceBand::Break)
    }
}

// ── Soft dampener ────────────────────────────────────────────────────────────

/// A hysteretic soft dampener (ADR-062 D6): below `dampen_below` it engages and
/// multiplies positive scores by `factor` (×0.8); it only disengages once
/// coherence reaches `recover_at`. Between the two it holds its prior state —
/// this is "×0.8 until recovery", never a hard gate.
#[derive(Debug, Clone)]
pub struct CoherenceDampener {
    dampen_below: f32,
    recover_at: f32,
    factor: f32,
    dampened: bool,
}

impl Default for CoherenceDampener {
    fn default() -> Self {
        Self::new()
    }
}

impl CoherenceDampener {
    /// A dampener with the ADR-062 default thresholds (engage < 0.65, clear ≥
    /// 0.85, factor 0.8).
    pub fn new() -> Self {
        Self {
            dampen_below: DEFAULT_DAMPEN_BELOW,
            recover_at: DEFAULT_RECOVER_AT,
            factor: DAMPEN_FACTOR,
            dampened: false,
        }
    }

    /// A dampener with explicit thresholds. `recover_at` should be ≥
    /// `dampen_below` for the hysteresis band to make sense.
    pub fn with_thresholds(dampen_below: f32, recover_at: f32, factor: f32) -> Self {
        Self {
            dampen_below,
            recover_at,
            factor,
            dampened: false,
        }
    }

    /// Feed the latest coherence read; updates the engaged/cleared state with
    /// hysteresis and returns whether the dampener is now engaged.
    pub fn observe(&mut self, coherence: f32) -> bool {
        if coherence < self.dampen_below {
            self.dampened = true;
        } else if coherence >= self.recover_at {
            self.dampened = false;
        }
        // Otherwise hold prior state (hysteresis band).
        self.dampened
    }

    /// Whether the dampener is currently engaged.
    pub fn is_dampened(&self) -> bool {
        self.dampened
    }

    /// Apply the dampener to a score: while engaged, **positive** scores are
    /// scaled by `factor`; zero/negative scores pass through unchanged (a
    /// dampener weakens pressure, it does not invent it).
    pub fn apply(&self, score: f32) -> f32 {
        if self.dampened && score > 0.0 {
            score * self.factor
        } else {
            score
        }
    }

    /// Force the dampener back to the cleared state (e.g. on calibration).
    pub fn reset(&mut self) {
        self.dampened = false;
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(cov: f32, conf: f32, conn: f32, qap: f32, topic: f32) -> CoherenceSignals {
        CoherenceSignals {
            relation_coverage: cov,
            relation_confidence: conf,
            structural_connectivity: conn,
            qap_closure: qap,
            topic_continuity: topic,
        }
    }

    #[test]
    fn coherence_matches_hand_computed() {
        // 0.30*0.8 + 0.25*0.9 + 0.20*0.7 + 0.15*0.6 + 0.10*1.0
        // = 0.24 + 0.225 + 0.14 + 0.09 + 0.10 = 0.795
        let s = signals(0.8, 0.9, 0.7, 0.6, 1.0);
        assert!((compute_coherence(&s) - 0.795).abs() < 1e-6);
    }

    #[test]
    fn perfect_coherence_is_one() {
        let s = signals(1.0, 1.0, 1.0, 1.0, 1.0);
        assert!((compute_coherence(&s) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bands_classify_correctly() {
        assert_eq!(CoherenceBand::from_score(0.95), CoherenceBand::Strong);
        assert_eq!(CoherenceBand::from_score(0.85), CoherenceBand::Strong);
        assert_eq!(CoherenceBand::from_score(0.70), CoherenceBand::MildDrift);
        assert_eq!(CoherenceBand::from_score(0.50), CoherenceBand::Warning);
        assert_eq!(CoherenceBand::from_score(0.20), CoherenceBand::Break);
        assert!(!CoherenceBand::Strong.is_alerting());
        assert!(!CoherenceBand::MildDrift.is_alerting());
        assert!(CoherenceBand::Warning.is_alerting());
        assert!(CoherenceBand::Break.is_alerting());
    }

    /// The dampener applies below threshold and clears on recovery, with
    /// hysteresis in the drift band (ADR-062 D6 Done-when).
    #[test]
    fn dampener_applies_below_threshold_and_clears_on_recovery() {
        let mut d = CoherenceDampener::new();
        // Starts cleared: scores pass through.
        assert!(!d.is_dampened());
        assert!((d.apply(1.0) - 1.0).abs() < 1e-6);

        // Drop below 0.65 → engaged; positive scores ×0.8.
        assert!(d.observe(0.50));
        assert!(d.is_dampened());
        assert!((d.apply(1.0) - 0.8).abs() < 1e-6);
        assert!((d.apply(0.5) - 0.4).abs() < 1e-6);
        // Zero/negative scores are untouched.
        assert_eq!(d.apply(0.0), 0.0);
        assert!((d.apply(-0.3) - (-0.3)).abs() < 1e-6);

        // In the hysteresis band (0.65–0.85) the dampener holds.
        assert!(d.observe(0.70));
        assert!(d.is_dampened());
        assert!((d.apply(1.0) - 0.8).abs() < 1e-6);

        // Recover to ≥ 0.85 → cleared; full pressure restored.
        assert!(!d.observe(0.90));
        assert!(!d.is_dampened());
        assert!((d.apply(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn dampener_reset_clears_state() {
        let mut d = CoherenceDampener::new();
        d.observe(0.1);
        assert!(d.is_dampened());
        d.reset();
        assert!(!d.is_dampened());
    }

    #[test]
    fn custom_thresholds_respected() {
        let mut d = CoherenceDampener::with_thresholds(0.4, 0.6, 0.5);
        assert!(d.observe(0.3));
        assert!((d.apply(1.0) - 0.5).abs() < 1e-6);
        // 0.5 is in the hysteresis band → still dampened.
        assert!(d.observe(0.5));
        // 0.6 clears.
        assert!(!d.observe(0.6));
    }
}
