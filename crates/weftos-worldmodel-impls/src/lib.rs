//! `weftos-worldmodel-impls` — concrete LeWM trait implementations (WEFT-521).
//!
//! Default path is **weights-free**: null/stub types that satisfy
//! [`weftos_worldmodel_core`] traits and unit-test without candle or model
//! files. Optional `candle` feature exposes a ViT-tiny / AdaLN **skeleton**
//! (layout + `Unavailable` fallbacks); trained weights and full training
//! loops remain follow-up work — see crate `README.md`.
//!
//! # Modules
//!
//! - [`action_encoder`] — control intent → fixed-width [`Action`] code
//! - [`encoder`] — sensor → latent (null / hash stubs; candle ViT skeleton)
//! - [`predictor`] — `pred_φ` stubs + AdaLN skeleton under `candle`
//! - [`planner`] — CEM-shaped stub planner
//! - [`lattice`] — composed [`StubLattice`] implementing [`LatticeApi`]
//! - [`sigreg`] — stub SIGReg monitor (Welford lands in WEFT-528)
//! - [`candle`] — optional ML skeleton (`feature = "candle"`)
//!
//! Designed for `no_std` + `alloc` on the default feature set. The `candle`
//! feature implies `std`.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

extern crate alloc;

pub mod action_encoder;
pub mod encoder;
pub mod lattice;
pub mod planner;
pub mod predictor;
pub mod sigreg;

#[cfg(feature = "candle")]
#[cfg_attr(docsrs, doc(cfg(feature = "candle")))]
pub mod candle;

// ── Re-exports from core (facade-friendly surface) ─────────────────────────
pub use weftos_worldmodel_core::{
    zero_latent, Action, ActionPlan, Encoder, Latent, LatentPlanner, LatticeApi, NodeId,
    ObservationFrame, PlanStep, PlannerKind, Predictor, RecallHit, SigRegHealth, SigRegMonitor,
    SubscriptionId, WorldModelError, WorldModelResult, LATENT_DIM, LATENT_DIM_U16,
};

// ── Crate-local stubs ──────────────────────────────────────────────────────
pub use action_encoder::{ActionEncoder, HashActionEncoder, NullActionEncoder, ACTION_CODE_DIM};
pub use encoder::{HashEncoder, NullEncoder};
pub use lattice::StubLattice;
pub use planner::NullPlanner;
pub use predictor::{IdentityPredictor, NullPredictor};
pub use sigreg::NullSigRegMonitor;

#[cfg(feature = "candle")]
pub use candle::{candle_cpu_device, AdaLnPredictor, CandleVitEncoder, VitTinyConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_worldmodel_core::LATTICE_METHOD_COUNT;

    #[test]
    fn null_encoder_predictor_planner_satisfy_traits() {
        let enc = NullEncoder;
        let pred = NullPredictor;
        let planner = NullPlanner::default();
        let action_enc = NullActionEncoder;

        let z = enc.encode(b"sensor").expect("encode");
        assert_eq!(z, zero_latent());
        assert_eq!(enc.latent_dim(), LATENT_DIM);

        let action = action_enc.encode_bytes(b"").expect("action");
        assert_eq!(action, Action::null());

        let z_hat = pred.predict(&z, &action).expect("predict");
        assert_eq!(z_hat, zero_latent());

        let plan = planner.plan(&z, 4).expect("plan");
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(planner.kind(), PlannerKind::Cem);
    }

    #[test]
    fn hash_encoder_is_deterministic_and_non_zero() {
        let enc = HashEncoder::default();
        let a = enc.encode(b"frame-a").expect("a");
        let b = enc.encode(b"frame-a").expect("a2");
        let c = enc.encode(b"frame-b").expect("b");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, zero_latent());
        assert_eq!(enc.latent_dim(), LATENT_DIM);
    }

    #[test]
    fn hash_action_encoder_fills_code() {
        let enc = HashActionEncoder::default();
        let a = enc.encode_bytes(b"steer-left").expect("act");
        assert_ne!(a, Action::null());
        assert_eq!(a.code.len(), ACTION_CODE_DIM);
    }

    #[test]
    fn stub_lattice_seven_methods_smoke() {
        assert_eq!(LATTICE_METHOD_COUNT, 7);
        let mut api = StubLattice::default();
        let frame = ObservationFrame {
            bytes: b"frame",
            latent_dim: LATENT_DIM_U16,
            timestamp_ms: 0,
        };
        let z = api.observe(frame).expect("observe");
        assert_eq!(z.len(), LATENT_DIM);
        let z2 = api
            .observe_node(
                NodeId(1),
                ObservationFrame {
                    bytes: b"n",
                    latent_dim: LATENT_DIM_U16,
                    timestamp_ms: 1,
                },
            )
            .expect("observe_node");
        assert_eq!(z2.len(), LATENT_DIM);
        assert!(api.predict(&z, &Action::null()).is_ok());
        assert!(api.plan(&z, 2).is_ok());
        assert!(api.recall(&z, 3).expect("recall").is_empty());
        assert!(api.subscribe_surprise(SubscriptionId(0)).is_ok());
        assert!(api.subscribe_drift(SubscriptionId(1)).is_ok());
    }

    #[test]
    fn sigreg_stub_healthy() {
        let mut mon = NullSigRegMonitor::default();
        let h = mon.update(&zero_latent()).expect("update");
        assert!(h.is_healthy());
        assert!(!h.should_rollback());
    }

    #[test]
    fn identity_predictor_returns_input() {
        let mut z = zero_latent();
        z[0] = 1.5;
        z[191] = -0.25;
        let out = IdentityPredictor
            .predict(&z, &Action::null())
            .expect("predict");
        assert_eq!(out, z);
    }

    #[test]
    fn observation_dim_mismatch_rejected() {
        let mut api = StubLattice::default();
        let bad = ObservationFrame {
            bytes: b"",
            latent_dim: 64,
            timestamp_ms: 0,
        };
        assert!(matches!(
            api.observe(bad),
            Err(WorldModelError::LatentDimMismatch { got: 64, .. })
        ));
    }
}
