//! Action-conditioned latent predictors (`pred_φ` stubs).
//!
//! Production AdaLN network is scaffolded under [`crate::candle`] when the
//! `candle` feature is enabled (weights / training = follow-up; WEFT-529).

use weftos_worldmodel_core::{Action, Latent, Predictor, WorldModelResult};

/// Stub predictor: returns the prior mean (zero latent), ignoring inputs.
///
/// Useful when a host wants a pure no-op that does not echo `z_t` (contrast
/// [`IdentityPredictor`] / core `NullPredictor`).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullPredictor;

impl Predictor for NullPredictor {
    fn predict(&self, _z_t: &Latent, _action: &Action) -> WorldModelResult<Latent> {
        Ok(weftos_worldmodel_core::zero_latent())
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

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_worldmodel_core::zero_latent;

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
}
