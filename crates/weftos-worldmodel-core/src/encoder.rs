//! Sensor → latent encoder trait (ViT-tiny target in WEFT-521).

use crate::error::WorldModelResult;
use crate::latent::{Latent, LATENT_DIM};

/// Encodes raw sensor observations into the SIGReg latent manifold.
///
/// Production path: candle ViT-tiny in `weftos-worldmodel-impls` (WEFT-521).
pub trait Encoder {
    /// Map observation bytes to `z_t ∈ ℝ^{LATENT_DIM}`.
    fn encode(&self, observation: &[u8]) -> WorldModelResult<Latent>;

    /// Latent width this encoder produces (must equal [`LATENT_DIM`] for v1).
    fn latent_dim(&self) -> usize {
        LATENT_DIM
    }
}

/// Stub encoder: always returns the zero latent (tests / no-op hosts).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEncoder;

impl Encoder for NullEncoder {
    fn encode(&self, _observation: &[u8]) -> WorldModelResult<Latent> {
        Ok(crate::latent::zero_latent())
    }
}
