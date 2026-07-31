//! Latent planner trait (CEM default @ 10 Hz; WEFT-529).

use alloc::vec::Vec;

use crate::error::WorldModelResult;
use crate::latent::Latent;
use crate::types::Action;

/// Planner algorithm selection (research stream: CEM default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum PlannerKind {
    /// Cross-entropy method (default).
    #[default]
    Cem,
    /// MPPI warm-start variant.
    MppiWarm,
    /// Gradient shooting.
    Gradient,
}

/// Single waypoint in a latent-space plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanStep {
    /// Action at this step.
    pub action: Action,
    /// Predicted latent after applying `action` (optional fill).
    pub z_hat: Latent,
}

/// Sequence of actions / predicted latents for DEMOCRITUS interpolation.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionPlan {
    /// Ordered plan steps (horizon length).
    pub steps: Vec<PlanStep>,
    /// Algorithm that produced the plan.
    pub kind: PlannerKind,
}

impl ActionPlan {
    /// Empty plan with declared kind.
    pub fn empty(kind: PlannerKind) -> Self {
        Self {
            steps: Vec::new(),
            kind,
        }
    }
}

/// Plans action sequences in latent space for the 10 Hz control loop.
///
/// Must never run inline on the 1 ms DEMOCRITUS servo path — publish via
/// `ArcSwap<ActionPlan>` at the facade / service layer.
pub trait LatentPlanner {
    /// Plan `horizon` steps from current latent `z_t`.
    fn plan(&self, z_t: &Latent, horizon: usize) -> WorldModelResult<ActionPlan>;

    /// Active planner algorithm.
    fn kind(&self) -> PlannerKind {
        PlannerKind::Cem
    }
}

/// Stub planner: emits `horizon` null actions with constant latent.
#[derive(Debug, Clone, Copy)]
pub struct NullPlanner {
    /// Reported algorithm kind.
    pub kind: PlannerKind,
}

impl Default for NullPlanner {
    fn default() -> Self {
        Self {
            kind: PlannerKind::Cem,
        }
    }
}

impl LatentPlanner for NullPlanner {
    fn plan(&self, z_t: &Latent, horizon: usize) -> WorldModelResult<ActionPlan> {
        let mut steps = Vec::with_capacity(horizon);
        for _ in 0..horizon {
            steps.push(PlanStep {
                action: Action::null(),
                z_hat: *z_t,
            });
        }
        Ok(ActionPlan {
            steps,
            kind: self.kind,
        })
    }

    fn kind(&self) -> PlannerKind {
        self.kind
    }
}
