//! Action-conditioned latent predictor `pred_φ` (ẑ_{t+1} = f(z_t, a_t)).

use crate::error::WorldModelResult;
use crate::latent::Latent;
use crate::types::Action;

/// Predicts next latent from current latent and action.
///
/// Production path: AdaLN-modulated network in `weftos-worldmodel-impls`
/// (WEFT-521 / WEFT-529).
pub trait Predictor {
    /// One-step prediction `ẑ_{t+1}`.
    fn predict(&self, z_t: &Latent, action: &Action) -> WorldModelResult<Latent>;
}

/// Stub predictor: identity (returns `z_t` unchanged).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullPredictor;

impl Predictor for NullPredictor {
    fn predict(&self, z_t: &Latent, _action: &Action) -> WorldModelResult<Latent> {
        Ok(*z_t)
    }
}
