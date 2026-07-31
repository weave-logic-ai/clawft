//! Shared observation / action / identity types for core traits.

use crate::latent::{latent_dim_matches_v1, LATENT_DIM, LATENT_DIM_U16};
use crate::error::{WorldModelError, WorldModelResult};

/// Opaque mesh / ECC node identifier (stable across LatticeApi calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeId(pub u64);

/// Subscription handle for surprise / drift streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SubscriptionId(pub u64);

/// Control / intervention action `a_t` (dense placeholder for v1).
///
/// Concrete semantics (servo targets, discrete intents) live in impls and
/// DEMOCRITUS integration. Core only needs a fixed-width code for `pred_φ`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Action {
    /// Dense action embedding (length free for now; common size 32).
    pub code: [f32; 32],
}

impl Action {
    /// Zero action (no-op / null intervention).
    pub const fn null() -> Self {
        Self { code: [0.0; 32] }
    }
}

impl Default for Action {
    fn default() -> Self {
        Self::null()
    }
}

/// Borrowed observation frame on the sensor → encoder path.
///
/// Owned CBOR framing lands in `weftos-sensor-pipeline-wire` (WEFT-523).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservationFrame<'a> {
    /// Raw payload bytes (sensor class specific; encoder interprets).
    pub bytes: &'a [u8],
    /// Declared latent width for this frame (must match local contract).
    pub latent_dim: u16,
    /// Observation timestamp (milliseconds, host clock or mesh LWW).
    pub timestamp_ms: u64,
}

impl<'a> ObservationFrame<'a> {
    /// Construct and validate against v1 dim.
    pub fn v1(bytes: &'a [u8], timestamp_ms: u64) -> WorldModelResult<Self> {
        Ok(Self {
            bytes,
            latent_dim: LATENT_DIM_U16,
            timestamp_ms,
        })
    }

    /// Reject frames whose dim ≠ local SIGReg width.
    pub fn check_dim(&self) -> WorldModelResult<()> {
        if latent_dim_matches_v1(self.latent_dim) {
            Ok(())
        } else {
            Err(WorldModelError::dim_mismatch(self.latent_dim))
        }
    }
}

/// One HNSW / latent-memory neighbour from [`super::LatticeApi::recall`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecallHit {
    /// Neighbour node (if spatial/ECC linked).
    pub node: NodeId,
    /// Distance or similarity score (impl-defined metric).
    pub score: f32,
    /// Optional latent snapshot length marker (always [`LATENT_DIM`] for v1).
    pub latent_dim: u16,
}

impl RecallHit {
    /// Construct a hit asserting v1 dim.
    pub const fn v1(node: NodeId, score: f32) -> Self {
        Self {
            node,
            score,
            latent_dim: LATENT_DIM as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_rejects_wrong_dim() {
        let bad = ObservationFrame {
            bytes: b"",
            latent_dim: 64,
            timestamp_ms: 0,
        };
        assert!(matches!(
            bad.check_dim(),
            Err(WorldModelError::LatentDimMismatch { got: 64, .. })
        ));
    }
}
