//! Per-frame observational tuples for ExoChain attestation (WEFT-533).
//!
//! Every world-model step should record `(a_t, z_t, z_{t+1}, surprise)` with a
//! version-tagged SIGReg manifold reference so standby / peer services can
//! replay and audit without trusting the live bus.

use crate::latent::{Latent, LatentVersion, LATENT_DIM, LATENT_DIM_U16};
use crate::types::Action;

/// ExoChain / audit event kind for a single LeWM frame attestation.
///
/// Mirrors the constant used by the service binary and (when wired) the
/// kernel chain vocabulary so producers share one string without a compile
/// dependency on `clawft-kernel`.
pub const EVENT_KIND_LEWM_FRAME_ATTESTATION: &str = "lewm.frame.attestation";

/// Source subsystem label for chain logging of world-model frames.
pub const ATTESTATION_SOURCE: &str = "lewm.worldmodel";

/// Observational transition tuple attested every frame.
///
/// Fields match the H-O-E-A cycle: action `a_t`, prior latent `z_t`, next
/// observed latent `z_{t+1}`, and VoE surprise `‖ẑ_{t+1} − z_{t+1}‖²`.
/// The manifold reference is the version-tagged SIGReg contract (ADR-050 /
/// WEFT-543), not an implementation-specific weight hash.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservationTuple {
    /// Control / intervention applied at step `t`.
    pub a_t: Action,
    /// Latent state at step `t`.
    pub z_t: Latent,
    /// Observed latent at step `t+1`.
    pub z_tp1: Latent,
    /// Predicted latent `ẑ_{t+1}` from `pred_φ(z_t, a_t)` (audit trail).
    pub z_hat: Latent,
    /// VoE surprise: squared L2 `‖ẑ_{t+1} − z_{t+1}‖²`.
    pub surprise: f32,
    /// Version-tagged SIGReg manifold reference.
    pub manifold: LatentVersion,
    /// Declared latent width (must match [`LATENT_DIM`] for v1).
    pub latent_dim: u16,
    /// Observation timestamp (milliseconds).
    pub timestamp_ms: u64,
    /// Monotonic frame sequence within the service process.
    pub frame_seq: u64,
}

impl ObservationTuple {
    /// Build a tuple, computing surprise from `z_hat` and `z_tp1`.
    ///
    /// Uses [`LatentVersion::V1`] and [`LATENT_DIM_U16`] by default.
    pub fn new(
        a_t: Action,
        z_t: Latent,
        z_hat: Latent,
        z_tp1: Latent,
        timestamp_ms: u64,
        frame_seq: u64,
    ) -> Self {
        Self {
            a_t,
            z_t,
            z_tp1,
            z_hat,
            surprise: surprise_voe(&z_hat, &z_tp1),
            manifold: LatentVersion::V1,
            latent_dim: LATENT_DIM_U16,
            timestamp_ms,
            frame_seq,
        }
    }

    /// True when manifold major expects the local latent width.
    pub fn manifold_matches_local(&self) -> bool {
        self.manifold.expects_v1_dim() && self.latent_dim == LATENT_DIM_U16
    }
}

/// VoE surprise `‖ẑ − z‖²` over the full latent (diagram ② EVALUATE).
#[inline]
pub fn surprise_voe(z_hat: &Latent, z_tp1: &Latent) -> f32 {
    let mut acc = 0.0f32;
    let mut i = 0;
    while i < LATENT_DIM {
        let d = z_hat[i] - z_tp1[i];
        acc += d * d;
        i += 1;
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latent::zero_latent;

    #[test]
    fn zero_prediction_zero_surprise() {
        let z = zero_latent();
        assert_eq!(surprise_voe(&z, &z), 0.0);
    }

    #[test]
    fn surprise_is_squared_l2() {
        let mut z_hat = zero_latent();
        let mut z = zero_latent();
        z_hat[0] = 3.0;
        z[0] = 0.0;
        z_hat[1] = 4.0;
        z[1] = 0.0;
        assert!((surprise_voe(&z_hat, &z) - 25.0).abs() < 1e-5);
    }

    #[test]
    fn tuple_defaults_to_v1_manifold() {
        let t = ObservationTuple::new(
            Action::null(),
            zero_latent(),
            zero_latent(),
            zero_latent(),
            0,
            1,
        );
        assert_eq!(t.manifold, LatentVersion::V1);
        assert_eq!(t.latent_dim, 192);
        assert!(t.manifold_matches_local());
        assert_eq!(t.surprise, 0.0);
        assert_eq!(EVENT_KIND_LEWM_FRAME_ATTESTATION, "lewm.frame.attestation");
    }
}
