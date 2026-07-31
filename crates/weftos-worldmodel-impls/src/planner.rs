//! Latent planner stubs (CEM-shaped; real CEM samples land in WEFT-529).

use alloc::vec::Vec;

use weftos_worldmodel_core::{
    Action, ActionPlan, Latent, LatentPlanner, PlanStep, PlannerKind, WorldModelResult,
};

use crate::predictor::{IdentityPredictor, NullPredictor};
use weftos_worldmodel_core::Predictor;

/// Stub planner: emits `horizon` null actions.
///
/// When `echo_latent` is true, predicted latents copy `z_t` (identity
/// dynamics); otherwise they are the zero prior.
#[derive(Debug, Clone, Copy)]
pub struct NullPlanner {
    /// Reported algorithm kind (default CEM).
    pub kind: PlannerKind,
    /// If true, fill `z_hat` with `z_t`; else zero latent.
    pub echo_latent: bool,
}

impl Default for NullPlanner {
    fn default() -> Self {
        Self {
            kind: PlannerKind::Cem,
            echo_latent: true,
        }
    }
}

impl LatentPlanner for NullPlanner {
    fn plan(&self, z_t: &Latent, horizon: usize) -> WorldModelResult<ActionPlan> {
        let mut steps = Vec::with_capacity(horizon);
        let action = Action::null();
        for _ in 0..horizon {
            let z_hat = if self.echo_latent {
                IdentityPredictor.predict(z_t, &action)?
            } else {
                NullPredictor.predict(z_t, &action)?
            };
            steps.push(PlanStep { action, z_hat });
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

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_worldmodel_core::zero_latent;

    #[test]
    fn horizon_length() {
        let p = NullPlanner::default();
        let plan = p.plan(&zero_latent(), 5).unwrap();
        assert_eq!(plan.steps.len(), 5);
        assert_eq!(plan.kind, PlannerKind::Cem);
    }

    #[test]
    fn zero_latent_mode() {
        let p = NullPlanner {
            kind: PlannerKind::Cem,
            echo_latent: false,
        };
        let mut z = zero_latent();
        z[0] = 1.0;
        let plan = p.plan(&z, 1).unwrap();
        assert_eq!(plan.steps[0].z_hat, zero_latent());
    }
}
