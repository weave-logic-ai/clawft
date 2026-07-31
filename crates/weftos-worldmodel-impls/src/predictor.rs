//! Action-conditioned latent predictors (`pred_φ`: z_t, a_t → ẑ_{t+1}`).
//!
//! # Runtime path (WEFT-529)
//!
//! [`LinearPredPhi`] is the default weights-free production predictor: a
//! linear residual map from the 32-d action code into the 192-d latent,
//! `ẑ = z + scale · P a`. No trained network weights are required; residual
//! AdaLN / candle paths remain under [`crate::candle`] when enabled.
//!
//! Full neural training (ViT-tiny encoder + AdaLN bake-off) is still residual
//! work — see crate README.

use weftos_worldmodel_core::{
    zero_latent, Action, Latent, Predictor, WorldModelResult, LATENT_DIM,
};

/// Stub predictor: returns the prior mean (zero latent), ignoring inputs.
///
/// Useful when a host wants a pure no-op that does not echo `z_t` (contrast
/// [`IdentityPredictor`] / core `NullPredictor`).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullPredictor;

impl Predictor for NullPredictor {
    fn predict(&self, _z_t: &Latent, _action: &Action) -> WorldModelResult<Latent> {
        Ok(zero_latent())
    }
}

/// Identity predictor: `ẑ_{t+1} = z_t` (action ignored).
///
/// Matches `weftos_worldmodel_core::NullPredictor` semantics; preferred name
/// in the impls crate so "null" means zero prior and "identity" means hold.
#[derive(Debug, Default, Clone, Copy)]
pub struct IdentityPredictor;

impl Predictor for IdentityPredictor {
    fn predict(&self, z_t: &Latent, _action: &Action) -> WorldModelResult<Latent> {
        Ok(*z_t)
    }
}

/// Linear residual action-conditioned predictor `pred_φ`.
///
/// ```text
/// ẑ_i = z_i + scale * a_{i mod 32}
/// ```
///
/// This is the default runtime dynamics model used by [`crate::planner::CemPlanner`]
/// until trained AdaLN weights are available. Deterministic, `no_std`, and
/// unit-testable; action magnitude moves the latent along repeating action
/// channels so CEM can optimise toward a goal.
#[derive(Debug, Clone, Copy)]
pub struct LinearPredPhi {
    /// Multiplier on the action residual (`ẑ = z + scale · broadcast(a)`).
    pub scale: f32,
}

impl Default for LinearPredPhi {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl LinearPredPhi {
    /// Construct with an explicit residual scale.
    pub const fn new(scale: f32) -> Self {
        Self { scale }
    }
}

impl Predictor for LinearPredPhi {
    fn predict(&self, z_t: &Latent, action: &Action) -> WorldModelResult<Latent> {
        let mut out = *z_t;
        let code = &action.code;
        let n_a = code.len(); // 32
        for i in 0..LATENT_DIM {
            out[i] += self.scale * code[i % n_a];
        }
        Ok(out)
    }
}

/// Alias used in docs / service wiring for the production `pred_φ` path.
pub type PredPhi = LinearPredPhi;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_is_zero() {
        let mut z = zero_latent();
        z[0] = 3.0;
        assert_eq!(
            NullPredictor.predict(&z, &Action::null()).unwrap(),
            zero_latent()
        );
    }

    #[test]
    fn identity_holds() {
        let mut z = zero_latent();
        z[10] = 2.0;
        assert_eq!(IdentityPredictor.predict(&z, &Action::null()).unwrap(), z);
    }

    #[test]
    fn linear_pred_phi_applies_action_residual() {
        let mut z = zero_latent();
        z[0] = 1.0;
        z[32] = 2.0;
        let mut a = Action::null();
        a.code[0] = 0.5;
        let pred = LinearPredPhi::new(2.0);
        let out = pred.predict(&z, &a).unwrap();
        // dim 0 and 32 both get + scale * a[0]
        assert!((out[0] - (1.0 + 2.0 * 0.5)).abs() < 1e-5);
        assert!((out[32] - (2.0 + 2.0 * 0.5)).abs() < 1e-5);
        // untouched channel stays put
        assert!((out[1] - 0.0).abs() < 1e-5);
    }

    #[test]
    fn null_action_is_identity_under_linear_pred() {
        let mut z = zero_latent();
        z[5] = -1.25;
        let out = LinearPredPhi::default()
            .predict(&z, &Action::null())
            .unwrap();
        assert_eq!(out, z);
    }
}
