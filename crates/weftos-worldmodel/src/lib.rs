//! `weftos-worldmodel` — single user-facing entrypoint for the LeWM latent
//! world model (WEFT-522).
//!
//! Re-exports:
//! - **traits + latent contract** from [`weftos_worldmodel_core`] (WEFT-520)
//! - **default stubs + wiring** from [`weftos_worldmodel_impls`] (WEFT-521)
//!
//! Downstream crates (sensor pipeline, service binary, LatticeApi hosts)
//! should depend on this crate rather than pin core/impls separately.
//!
//! # Latent contract (WEFT-543)
//!
//! SIGReg manifold is isotropic Gaussian `N(0, I)` in **192** dimensions for
//! `mesh.sensor.v1`. Changing width requires a wire major-version bump.
//!
//! # No ML training in this crate
//!
//! Default builds are **weights-free** (`no_std` + `alloc`). Optional
//! `candle` feature only forwards the impls skeleton (Unavailable without
//! checkpoints). Full ViT-tiny / AdaLN training is out of scope here
//! (see WEFT-529 and `weftos-worldmodel-impls` README).
//!
//! # Quick start (stubs)
//!
//! ```rust
//! use weftos_worldmodel::{
//!     default_stub_lattice, Action, Encoder, LatticeApi, NullActionEncoder,
//!     NullEncoder, ObservationFrame, Predictor, ActionEncoder, LATENT_DIM_U16,
//! };
//!
//! let mut lattice = default_stub_lattice();
//! let z = lattice
//!     .observe(ObservationFrame {
//!         bytes: b"frame",
//!         latent_dim: LATENT_DIM_U16,
//!         timestamp_ms: 0,
//!     })
//!     .expect("observe");
//! assert_eq!(z.len(), 192);
//!
//! let enc = NullEncoder;
//! let a = NullActionEncoder.encode_bytes(b"noop").expect("action");
//! let _ = enc.encode(b"sensor").expect("encode");
//! let _ = lattice.predict(&z, &a).expect("predict");
//! let _ = lattice.plan(&z, 4).expect("plan");
//! ```
//!
//! # Feature flags
//!
//! - **default** — stub path, `no_std` + `alloc`
//! - **std** — host marker forwarded to core + impls
//! - **candle** — experimental ML skeleton (implies `std`); no trained weights

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

extern crate alloc;

// ── Underlying crates (advanced / selective imports) ───────────────────────
/// Core traits and latent contract (WEFT-520). Prefer top-level re-exports.
pub use weftos_worldmodel_core as core;
/// Concrete stubs and optional candle skeleton (WEFT-521). Prefer top-level re-exports.
pub use weftos_worldmodel_impls as impls;

// ── Core: traits, types, latent contract ───────────────────────────────────
pub use weftos_worldmodel_core::{
    latent_dim_matches_v1, zero_latent, Action, ActionPlan, Encoder, Latent, LatentPlanner,
    LatentVersion, LatticeApi, LatticeMethod, NodeId, ObservationFrame, PlanStep, PlannerKind,
    Predictor, RecallHit, SigRegHealth, SigRegMonitor, SubscriptionId, WorldModelError,
    WorldModelResult, LATENT_DIM, LATENT_DIM_U16, LATENT_SCHEMA_MAJOR_V1, LATTICE_METHOD_COUNT,
    LATTICE_METHODS, SIGREG_HEALTH_ROLLBACK_THRESHOLD, SIGREG_HEALTH_WINDOW_SECS,
};

// ── Impls: default stubs + action encoder ──────────────────────────────────
pub use weftos_worldmodel_impls::{
    ActionEncoder, HashActionEncoder, HashEncoder, IdentityPredictor, NullActionEncoder,
    NullEncoder, NullPlanner, NullPredictor, NullSigRegMonitor, StubLattice, ACTION_CODE_DIM,
};

// ── Optional candle skeleton (no weights) ──────────────────────────────────
#[cfg(feature = "candle")]
#[cfg_attr(docsrs, doc(cfg(feature = "candle")))]
pub use weftos_worldmodel_impls::{candle_cpu_device, AdaLnPredictor, CandleVitEncoder, VitTinyConfig};

/// Facade crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the default weight-free [`StubLattice`] for service scaffolding / tests.
///
/// Composes null encoder, identity predictor, and null CEM planner. No ML
/// weights and no candle dependency on the default feature set.
#[inline]
pub fn default_stub_lattice() -> StubLattice {
    StubLattice::default()
}

/// Default stub stack: encoder, action encoder, predictor, planner, lattice,
/// and SIGReg monitor in one place for hosts that want a single entry object.
///
/// This is the recommended wiring for consumers until production weights and
/// the sensor pipeline (WEFT-523) land. No training; all components are stubs.
#[derive(Debug, Clone)]
pub struct DefaultWorldModel {
    /// Sensor → latent encoder (null by default).
    pub encoder: NullEncoder,
    /// Control bytes → action code (null by default).
    pub action_encoder: NullActionEncoder,
    /// `pred_φ` stub (null latent by default).
    pub predictor: NullPredictor,
    /// CEM-shaped planner stub.
    pub planner: NullPlanner,
    /// Composed lattice API (observe / predict / plan / recall / subscribe).
    pub lattice: StubLattice,
    /// SIGReg health monitor stub (always healthy).
    pub sigreg: NullSigRegMonitor,
}

impl Default for DefaultWorldModel {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultWorldModel {
    /// Construct the default stub stack.
    #[inline]
    pub fn new() -> Self {
        Self {
            encoder: NullEncoder,
            action_encoder: NullActionEncoder,
            predictor: NullPredictor,
            planner: NullPlanner::default(),
            lattice: StubLattice::default(),
            sigreg: NullSigRegMonitor::default(),
        }
    }

    /// Observe a frame through the composed lattice and update SIGReg health.
    pub fn observe(
        &mut self,
        frame: ObservationFrame<'_>,
    ) -> WorldModelResult<(Latent, SigRegHealth)> {
        let z = self.lattice.observe(frame)?;
        let health = self.sigreg.update(&z)?;
        Ok((z, health))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_reexports_latent_contract() {
        assert_eq!(LATENT_DIM, 192);
        assert_eq!(LATENT_DIM_U16, 192);
        assert_eq!(zero_latent().len(), 192);
        assert!(latent_dim_matches_v1(192));
        assert!(!latent_dim_matches_v1(64));
        assert_eq!(LATTICE_METHOD_COUNT, 7);
        assert_eq!(LATTICE_METHODS.len(), 7);
        assert_eq!(LatticeMethod::Observe.as_str(), "observe");
        assert!((SIGREG_HEALTH_ROLLBACK_THRESHOLD - 0.85).abs() < f32::EPSILON);
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn default_stub_lattice_seven_methods() {
        let mut api = default_stub_lattice();
        let frame = ObservationFrame {
            bytes: b"frame",
            latent_dim: LATENT_DIM_U16,
            timestamp_ms: 0,
        };
        let z = api.observe(frame).expect("observe");
        assert_eq!(z, zero_latent());
        assert!(api.predict(&z, &Action::null()).is_ok());
        assert_eq!(api.plan(&z, 3).expect("plan").steps.len(), 3);
        assert!(api.recall(&z, 2).expect("recall").is_empty());
        assert!(api.subscribe_surprise(SubscriptionId(0)).is_ok());
        assert!(api.subscribe_drift(SubscriptionId(1)).is_ok());
        let z_node = api
            .observe_node(
                NodeId(7),
                ObservationFrame {
                    bytes: b"n",
                    latent_dim: LATENT_DIM_U16,
                    timestamp_ms: 1,
                },
            )
            .expect("observe_node");
        assert_eq!(z_node.len(), LATENT_DIM);
    }

    #[test]
    fn stub_encoder_predictor_planner_via_facade() {
        let enc = NullEncoder;
        let pred = NullPredictor;
        let planner = NullPlanner::default();
        let action_enc = NullActionEncoder;

        let z = enc.encode(b"sensor").expect("encode");
        assert_eq!(z, zero_latent());
        assert_eq!(enc.latent_dim(), LATENT_DIM);

        let a = action_enc.encode_bytes(b"").expect("action");
        assert_eq!(a, Action::null());
        assert_eq!(a.code.len(), ACTION_CODE_DIM);

        let z_hat = pred.predict(&z, &a).expect("predict");
        assert_eq!(z_hat, zero_latent());

        let plan = planner.plan(&z, 4).expect("plan");
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(planner.kind(), PlannerKind::Cem);
    }

    #[test]
    fn hash_paths_are_deterministic_through_facade() {
        let enc = HashEncoder::default();
        let a = enc.encode(b"frame-a").expect("a");
        let b = enc.encode(b"frame-a").expect("a2");
        let c = enc.encode(b"frame-b").expect("b");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, zero_latent());

        let act = HashActionEncoder::default()
            .encode_bytes(b"steer")
            .expect("act");
        assert_ne!(act, Action::null());
    }

    #[test]
    fn default_world_model_observe_updates_sigreg() {
        let mut wm = DefaultWorldModel::new();
        let (z, health) = wm
            .observe(ObservationFrame {
                bytes: b"hello",
                latent_dim: LATENT_DIM_U16,
                timestamp_ms: 42,
            })
            .expect("observe");
        assert_eq!(z, zero_latent());
        assert!(health.is_healthy());
        assert!(!health.should_rollback());
    }

    #[test]
    fn observation_dim_mismatch_rejected_through_facade() {
        let mut api = default_stub_lattice();
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

    #[test]
    fn identity_predictor_via_facade() {
        let mut z = zero_latent();
        z[0] = 1.5;
        z[191] = -0.25;
        let out = IdentityPredictor
            .predict(&z, &Action::null())
            .expect("predict");
        assert_eq!(out, z);
    }

    #[test]
    fn core_and_impls_modules_reachable() {
        assert_eq!(core::LATENT_DIM, impls::LATENT_DIM);
        // Core null stub lives in the encoder module; impls re-homes it at root.
        let _ = core::encoder::NullEncoder;
        let _ = impls::NullEncoder;
    }
}
