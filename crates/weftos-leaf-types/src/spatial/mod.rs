//! Spatial leaf-tag registry and CBOR primitives (ADR-056 Phase B / WEFT-717).
//!
//! Cross-consumer contract for BVH leaf discriminants and narrow-phase
//! payloads. Kernel, mesh, splat pipeline, and offline tooling all bind
//! here so tag numbers never drift.
//!
//! See [`tags`] for deprecation rules (ADR-031: additive only; never
//! reuse retired discriminants).

pub mod primitives;
pub mod tags;

pub use primitives::{
    AabbPayload, AabbWire, BeamTracePayload, CapsulePayload, FrustumPayload, ObbPayload,
    RadialSphereEventPayload, SensorRead4DPayload, SpherePayload, SplatCameraPayload,
    SplatScenePayload, SplatYardstickPayload, SweptAabbPayload, Vec3Wire, WmAffordancePayload,
    WmObjectPayload, WmSegmentPayload, WmSensorFovPayload, WmSurfacePayload, WmVolumePayload,
};
pub use tags::{SpatialLeafTag, UnknownSpatialTag, consts as tag_consts};
