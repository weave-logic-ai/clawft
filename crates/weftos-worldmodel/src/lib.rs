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
//! # ExoChain attestation (WEFT-533)
//!
//! With `feature = "std"`, [`attestation`] serializes
//! [`ObservationTuple`] `(a_t, z_t, z_{t+1}, surprise)` with a
//! version-tagged SIGReg manifold reference for append onto a
//! [`ChainSink`] (in-memory or kernel ExoChain bridge).
//!
//! # Feature flags
//!
//! - **default** — stub path, `no_std` + `alloc`
//! - **std** — host marker + JSON attestation encode/decode
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

// ── Core: traits, types, latent contract, attestation ──────────────────────
pub use weftos_worldmodel_core::{
    latent_dim_matches_v1, surprise_voe, zero_latent, Action, ActionPlan, Encoder, GateCondition,
    GateVerdict, Latent, LatentPlanner, LatentVersion, LatticeApi, LatticeMethod, NodeId,
    ObservationFrame, ObservationTuple, PlanStep, PlannerKind, Predictor, RecallHit,
    RollbackGate, RollbackGateMetrics, SigRegHealth, SigRegMonitor, SubscriptionId,
    WorldModelError, WorldModelResult, ATTESTATION_SOURCE, EVENT_KIND_LEWM_FRAME_ATTESTATION,
    EVENT_KIND_ROLLBACK_GATE, GATE_HELD_OUT_PROBE_MIN, GATE_SIGREG_HEALTH_MIN,
    GATE_TEMPORAL_STRAIGHTEN_MIN, GATE_VOE_DIFF_MIN, LATENT_DIM, LATENT_DIM_U16,
    LATENT_SCHEMA_MAJOR_V1, LATTICE_METHOD_COUNT, LATTICE_METHODS,
    SIGREG_HEALTH_ROLLBACK_THRESHOLD, SIGREG_HEALTH_WINDOW_SECS,
};

// ── Host attestation path (JSON + ChainSink; requires `std`) ───────────────
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod attestation;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use attestation::{
    attest_frame, AttestationError, AttestationPayload, ChainEntry, ChainSink, MemoryChainSink,
    ATTESTATION_SCHEMA_MAJOR, ATTESTATION_SCHEMA_MINOR,
};

// ── Impls: runtime monitors/planners + stubs + action encoder ─────────────
pub use weftos_worldmodel_impls::{
    planner_for_kind, ActionEncoder, CemPlanner, FourConditionRollbackGate, GradientPlanner,
    HashActionEncoder, HashEncoder, IdentityPredictor, LinearPredPhi, MppiWarmPlanner,
    NullActionEncoder, NullEncoder, NullPlanner, NullPredictor, NullSigRegMonitor, PredPhi,
    ProbePair, RollbackGateLog, SigRegHealthEvent, SigRegHealthLog, StubLattice,
    WelfordSigRegMonitor, ACTION_CODE_DIM, DEFAULT_MIN_SAMPLES, DEFAULT_PROBE_CAPACITY,
    DEFAULT_TRAJECTORY_CAPACITY, EVENT_KIND_SIGREG_HEALTH,
};

// ── Optional candle skeleton (no weights) ──────────────────────────────────
#[cfg(feature = "candle")]
#[cfg_attr(docsrs, doc(cfg(feature = "candle")))]
pub use weftos_worldmodel_impls::{candle_cpu_device, AdaLnPredictor, CandleVitEncoder, VitTinyConfig};

// ── ServiceApi-shaped LatticeApi facade (WEFT-527) ─────────────────────────
/// LatticeApi via ServiceApi-shaped call surface (requires `service-api`).
#[cfg(feature = "service-api")]
#[cfg_attr(docsrs, doc(cfg(feature = "service-api")))]
pub mod service_api;

#[cfg(feature = "service-api")]
#[cfg_attr(docsrs, doc(cfg(feature = "service-api")))]
pub use service_api::{
    default_lattice_service, dispatch_typed, observe_json, parse_lattice_method, FacadeHealth,
    FacadeServiceInfo, LatticeServiceError, LatticeServiceFacade, LatticeServiceResult,
    LATTICE_SERVICE_NAME, LATTICE_SERVICE_TYPE,
};

/// Facade crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build the default weight-free [`StubLattice`] for service scaffolding / tests.
///
/// Composes null encoder, linear `pred_φ`, and CEM planner. No ML weights and
/// no candle dependency on the default feature set.
#[inline]
pub fn default_stub_lattice() -> StubLattice {
    StubLattice::default()
}

/// Default runtime stack: encoder, action encoder, `pred_φ`, CEM planner,
/// lattice, Welford SIGReg monitor, and four-condition AND rollback gate
/// (WEFT-528 / WEFT-529 / WEFT-530).
///
/// Weights-free but not a pure no-op: linear residual prediction, CEM planning,
/// online SIGReg health with auto-rollback, and streaming-merge promotion
/// gating. Trained AdaLN / ViT weights remain residual (see impls README).
#[derive(Debug, Clone)]
pub struct DefaultWorldModel {
    /// Sensor → latent encoder (null by default).
    pub encoder: NullEncoder,
    /// Control bytes → action code (null by default).
    pub action_encoder: NullActionEncoder,
    /// Action-conditioned `pred_φ` (linear residual).
    pub predictor: LinearPredPhi,
    /// CEM planner (default algorithm).
    pub planner: CemPlanner,
    /// Composed lattice API (observe / predict / plan / recall / subscribe).
    pub lattice: StubLattice,
    /// Welford SIGReg health monitor (auto-rollback at 0.85 / 30s).
    pub sigreg: WelfordSigRegMonitor,
    /// Four-condition AND rollback / promotion gate (WEFT-530).
    pub rollback_gate: FourConditionRollbackGate,
}

impl Default for DefaultWorldModel {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultWorldModel {
    /// Construct the default runtime stack.
    #[inline]
    pub fn new() -> Self {
        Self {
            encoder: NullEncoder,
            action_encoder: NullActionEncoder,
            predictor: LinearPredPhi::default(),
            planner: CemPlanner::default(),
            lattice: StubLattice::default(),
            sigreg: WelfordSigRegMonitor::new(1),
            rollback_gate: FourConditionRollbackGate::new(),
        }
    }

    /// Observe a frame through the composed lattice and update SIGReg health.
    pub fn observe(
        &mut self,
        frame: ObservationFrame<'_>,
    ) -> WorldModelResult<(Latent, SigRegHealth)> {
        let z = self.lattice.observe(frame)?;
        let health = self.sigreg.update_at(&z, frame.timestamp_ms)?;
        self.rollback_gate.set_sigreg_health(health.score);
        Ok((z, health))
    }

    /// Ingest a full transition into the four-condition gate and return the
    /// AND verdict (blocks streaming-merge promotion when any condition fails).
    pub fn evaluate_transition(
        &mut self,
        z_t: &Latent,
        z_hat: &Latent,
        z_tp1: &Latent,
        sigreg_health: Option<f32>,
    ) -> GateVerdict {
        self.rollback_gate
            .observe_transition(z_t, z_hat, z_tp1, sigreg_health)
    }

    /// Whether a streaming-merge checkpoint may be promoted right now.
    pub fn may_promote_checkpoint(&self) -> bool {
        self.rollback_gate.may_promote()
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
        let pred = LinearPredPhi::default();
        let planner = CemPlanner::with_seed(7);
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
    fn default_world_model_four_condition_gate() {
        let mut wm = DefaultWorldModel::new();
        let z = zero_latent();
        // Cold start promotes.
        let v = wm.evaluate_transition(&z, &z, &z, Some(0.95));
        assert!(v.promote);
        assert!(wm.may_promote_checkpoint());

        // SIGReg-only veto.
        let v = wm.evaluate_transition(&z, &z, &z, Some(0.10));
        assert!(!v.promote);
        assert_eq!(v.first_veto(), Some(GateCondition::ClusterSigRegHealth));
        assert!(v.blocks_promotion());
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
