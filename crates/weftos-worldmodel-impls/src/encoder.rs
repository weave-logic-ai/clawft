//! Sensor → latent encoder implementations (null / hash; candle ViT in `candle`).

use weftos_worldmodel_core::{
    zero_latent, Encoder, Latent, WorldModelResult, LATENT_DIM,
};

use crate::action_encoder::hash_to_f32_array;

/// Stub encoder: always returns the zero latent (tests / no-op hosts).
///
/// Same contract as `weftos_worldmodel_core::NullEncoder`; re-homed here so
/// the impls crate is the default import path for applications.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEncoder;

impl Encoder for NullEncoder {
    fn encode(&self, _observation: &[u8]) -> WorldModelResult<Latent> {
        Ok(zero_latent())
    }

    fn latent_dim(&self) -> usize {
        LATENT_DIM
    }
}

/// Deterministic hash encoder for tests and weight-free demos.
///
/// Produces a stable non-zero latent from observation bytes. **Not** a
/// learned ViT — production path is candle ViT-tiny under `feature = "candle"`.
#[derive(Debug, Default, Clone, Copy)]
pub struct HashEncoder;

impl Encoder for HashEncoder {
    fn encode(&self, observation: &[u8]) -> WorldModelResult<Latent> {
        Ok(hash_to_f32_array::<LATENT_DIM>(observation))
    }

    fn latent_dim(&self) -> usize {
        LATENT_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_zero() {
        assert_eq!(NullEncoder.encode(b"x").unwrap(), zero_latent());
    }

    #[test]
    fn hash_differs_by_input() {
        let e = HashEncoder;
        assert_ne!(e.encode(b"a").unwrap(), e.encode(b"b").unwrap());
    }
}
