//! LeWM ↔ ECC decoupling invariants (WEFT-519 / ADR-090).
//!
//! The latent world model (LeWM) is an **additive consumer** of sensor and
//! ECC state. It must never become authoritative for causal reasoning, nor
//! short-circuit causal edges. This module codifies the five formal rules
//! as testable predicates and provides debug-time runtime checks for a
//! thin world-model facade.
//!
//! Full LeWM crates land later; until then these checks guard API seams
//! that accept world-model contributions into the kernel (impulses,
//! observations). See `docs/adr/adr-090-lewm-ecc-decoupling-invariant.md`.

use serde::{Deserialize, Serialize};

/// The five formal decoupling rules (ADR-090).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecouplingRule {
    /// R1 — ECC is the sole authoritative causal reasoner per node.
    EccAuthoritative = 1,
    /// R2 — World model is optional; sensor+ECC paths run without it.
    WorldModelOptional = 2,
    /// R3 — WM may publish impulses / observations only; never rewrite edges.
    ImpulseOnlyWritePath = 3,
    /// R4 — WM must not short-circuit a causal edge (no bypass of link/unlink).
    NoCausalShortCircuit = 4,
    /// R5 — Absent cluster/WM: local-ECC-only behaviour is unchanged.
    GracefulDegradation = 5,
}

impl DecouplingRule {
    pub const ALL: &'static [DecouplingRule] = &[
        Self::EccAuthoritative,
        Self::WorldModelOptional,
        Self::ImpulseOnlyWritePath,
        Self::NoCausalShortCircuit,
        Self::GracefulDegradation,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EccAuthoritative => "ecc_authoritative",
            Self::WorldModelOptional => "world_model_optional",
            Self::ImpulseOnlyWritePath => "impulse_only_write_path",
            Self::NoCausalShortCircuit => "no_causal_short_circuit",
            Self::GracefulDegradation => "graceful_degradation",
        }
    }
}

/// Classification of a proposed world-model write into the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WmWriteKind {
    /// Publish an impulse / observation (allowed).
    Impulse,
    /// Inject a sensor observation packet (allowed).
    Observation,
    /// Attempt to insert/rewrite a causal edge directly (forbidden).
    CausalEdgeMutate,
    /// Attempt to replace ECC reasoning result (forbidden).
    ReasoningOverride,
    /// Attempt to skip / short-circuit an existing edge evaluation (forbidden).
    ShortCircuit,
}

/// Result of a decoupling check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantCheck {
    pub rule: DecouplingRule,
    pub ok: bool,
    pub detail: String,
}

/// Evaluate whether a world-model write is allowed under the five rules.
pub fn check_wm_write(kind: WmWriteKind) -> Result<(), InvariantCheck> {
    match kind {
        WmWriteKind::Impulse | WmWriteKind::Observation => Ok(()),
        WmWriteKind::CausalEdgeMutate => Err(InvariantCheck {
            rule: DecouplingRule::ImpulseOnlyWritePath,
            ok: false,
            detail: "world model must not mutate causal edges directly".into(),
        }),
        WmWriteKind::ReasoningOverride => Err(InvariantCheck {
            rule: DecouplingRule::EccAuthoritative,
            ok: false,
            detail: "world model must not override ECC reasoning".into(),
        }),
        WmWriteKind::ShortCircuit => Err(InvariantCheck {
            rule: DecouplingRule::NoCausalShortCircuit,
            ok: false,
            detail: "world model must not short-circuit causal edges".into(),
        }),
    }
}

/// Predicate R2/R5: kernel without WM remains valid (always true for stub).
pub fn local_ecc_sufficient_without_wm() -> bool {
    true
}

/// Thin facade for future world-model service hooks (runtime assertions).
#[derive(Debug, Default)]
pub struct WorldModelFacade {
    /// When true, `debug_assert` / explicit checks reject forbidden writes.
    pub enforce: bool,
    /// Count of rejected forbidden writes (for tests / metrics).
    pub rejected: u64,
    /// Count of accepted impulse/observation writes.
    pub accepted: u64,
}

impl WorldModelFacade {
    pub fn new(enforce: bool) -> Self {
        Self {
            enforce,
            rejected: 0,
            accepted: 0,
        }
    }

    /// Submit a write; returns `Err` when the decoupling invariant rejects it.
    pub fn submit(&mut self, kind: WmWriteKind) -> Result<(), InvariantCheck> {
        match check_wm_write(kind) {
            Ok(()) => {
                self.accepted += 1;
                Ok(())
            }
            Err(e) => {
                self.rejected += 1;
                // Debug builds outside of unit tests: hard-fail so invariant
                // violations surface immediately during local development.
                // Under `cfg(test)` we only count + return Err so property
                // tests can exercise the reject path without panicking.
                #[cfg(all(debug_assertions, not(test)))]
                if self.enforce {
                    panic!(
                        "LeWM decoupling invariant violated: {:?} — {}",
                        e.rule, e.detail
                    );
                }
                let _ = self.enforce;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_rules_enumerated() {
        assert_eq!(DecouplingRule::ALL.len(), 5);
        assert_eq!(DecouplingRule::EccAuthoritative as u8, 1);
        assert_eq!(DecouplingRule::GracefulDegradation as u8, 5);
    }

    #[test]
    fn allows_impulse_and_observation() {
        assert!(check_wm_write(WmWriteKind::Impulse).is_ok());
        assert!(check_wm_write(WmWriteKind::Observation).is_ok());
    }

    #[test]
    fn rejects_short_circuit() {
        let err = check_wm_write(WmWriteKind::ShortCircuit).unwrap_err();
        assert_eq!(err.rule, DecouplingRule::NoCausalShortCircuit);
        assert!(!err.ok);
    }

    #[test]
    fn rejects_edge_mutate_and_override() {
        assert_eq!(
            check_wm_write(WmWriteKind::CausalEdgeMutate)
                .unwrap_err()
                .rule,
            DecouplingRule::ImpulseOnlyWritePath
        );
        assert_eq!(
            check_wm_write(WmWriteKind::ReasoningOverride)
                .unwrap_err()
                .rule,
            DecouplingRule::EccAuthoritative
        );
    }

    #[test]
    fn facade_counts_rejections() {
        let mut f = WorldModelFacade::new(true);
        assert!(f.submit(WmWriteKind::Impulse).is_ok());
        assert!(f.submit(WmWriteKind::ShortCircuit).is_err());
        assert_eq!(f.accepted, 1);
        assert_eq!(f.rejected, 1);
    }

    #[test]
    fn property_forbidden_short_circuit_never_accepted() {
        // Property-style: for any facade, short-circuit is always rejected.
        for enforce in [false, true] {
            let mut f = WorldModelFacade::new(enforce);
            assert!(f.submit(WmWriteKind::ShortCircuit).is_err());
        }
    }

    #[test]
    fn graceful_degradation_predicate() {
        assert!(local_ecc_sufficient_without_wm());
    }
}
