//! CBOR payload structs for each [`super::SpatialLeafTag`] (WEFT-717).
//!
//! These are the **narrow-phase** shapes stored in `Leaf.payload`.
//! Broad-phase always uses AABB (ADR-056 §2); payloads refine hits.
//!
//! Wire format: CBOR via crate-level [`crate::encode`] / [`crate::decode`].
//! Coordinates are reconstruction / scene-local f32 unless noted.
//!
//! ## Optional feature handles (ADR-088)
//!
//! Most payloads carry an optional [`VectorRef`] (`vector` field). It is
//! **not** an inline embedding — only a handle into an external index
//! (typically ECC HNSW). Default / absent = pure geometry. See
//! `docs/design/bvh_schema_updates.md` and ADR-088.

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use super::vector_ref::VectorRef;

// ── Shared geometry ────────────────────────────────────────────────

/// 3D point / vector on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3Wire {
    /// X
    pub x: f32,
    /// Y
    pub y: f32,
    /// Z
    pub z: f32,
}

impl Vec3Wire {
    /// Construct.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Origin.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
}

/// Axis-aligned box on the wire (`min`/`max` corners).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AabbWire {
    /// Minimum corner.
    pub min: Vec3Wire,
    /// Maximum corner.
    pub max: Vec3Wire,
}

impl AabbWire {
    /// From min/max (caller ensures min ≤ max per axis).
    pub const fn from_min_max(min: Vec3Wire, max: Vec3Wire) -> Self {
        Self { min, max }
    }
}

// ── Geometry primitive payloads ────────────────────────────────────

/// [`super::SpatialLeafTag::Sphere`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpherePayload {
    /// Center.
    pub center: Vec3Wire,
    /// Radius (≥ 0).
    pub radius: f32,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::Aabb`] payload (mirrors broad-phase bound).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AabbPayload {
    /// Box extent.
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::Obb`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ObbPayload {
    /// Center.
    pub center: Vec3Wire,
    /// Half-extents along local axes.
    pub half_extents: Vec3Wire,
    /// Unit quaternion orientation as `[x, y, z, w]`.
    pub orientation: [f32; 4],
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::Capsule`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CapsulePayload {
    /// Segment start.
    pub a: Vec3Wire,
    /// Segment end.
    pub b: Vec3Wire,
    /// Radius (≥ 0).
    pub radius: f32,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::SweptAabb`] payload (motion volume).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SweptAabbPayload {
    /// AABB at motion start.
    pub start: AabbWire,
    /// AABB at motion end.
    pub end: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::Frustum`] payload (six half-spaces).
///
/// Each plane is `(nx, ny, nz, d)` with equation `n·p + d ≤ 0` inside.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrustumPayload {
    /// Six frustum planes.
    pub planes: [[f32; 4]; 6],
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::RadialSphereEvent`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RadialSphereEventPayload {
    /// Center.
    pub center: Vec3Wire,
    /// Radius at peak.
    pub radius: f32,
    /// Inclusive start time (seconds, scene clock or unix).
    pub t_start: f64,
    /// Inclusive end time.
    pub t_end: f64,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::BeamTrace`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeamTracePayload {
    /// Origin.
    pub origin: Vec3Wire,
    /// Direction (need not be unit; length scales with `max_t`).
    pub direction: Vec3Wire,
    /// Maximum ray parameter.
    pub max_t: f32,
    /// Beam thickness radius (≥ 0; 0 = ideal ray).
    pub radius: f32,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::SensorRead4D`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensorRead4DPayload {
    /// Volume center.
    pub center: Vec3Wire,
    /// Half-extents of the observation volume.
    pub half_extents: Vec3Wire,
    /// Observation time.
    pub t: f64,
    /// Opaque sensor identifier.
    pub sensor_id: u64,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

// ── Splat / world-model payloads ───────────────────────────────────

/// [`super::SpatialLeafTag::SplatScene`] payload (W0-compatible shell).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplatScenePayload {
    /// Job / train run id.
    pub job_id: String,
    /// Optional scene AABB in `scene_local` frame.
    pub bound: Option<AabbWire>,
    /// Relative artifact names (sog, ply, …).
    pub artifacts: Vec<(String, String)>,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::SplatCamera`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SplatCameraPayload {
    /// Camera position.
    pub position: Vec3Wire,
    /// Look direction (unit preferred).
    pub forward: Vec3Wire,
    /// Up vector.
    pub up: Vec3Wire,
    /// Horizontal FOV radians.
    pub fov_y: f32,
    /// Near / far clip.
    pub near: f32,
    /// Far clip.
    pub far: f32,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::SplatYardstick`] payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SplatYardstickPayload {
    /// Segment in scene units.
    pub a: Vec3Wire,
    /// Segment end.
    pub b: Vec3Wire,
    /// Real-world length in meters.
    pub length_m: f32,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::WmObject`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmObjectPayload {
    /// Optional human / model label.
    pub label: Option<String>,
    /// Confidence 0.0–1.0 when labeled by a model.
    pub confidence: Option<f32>,
    /// Local AABB in scene frame.
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::WmSurface`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmSurfacePayload {
    /// Optional label (floor, wall, …).
    pub label: Option<String>,
    /// Plane normal (unit preferred).
    pub normal: Vec3Wire,
    /// Point on the plane.
    pub point: Vec3Wire,
    /// Approximate planar extent AABB.
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::WmVolume`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmVolumePayload {
    /// Free / occupied / no-go kind as a short string.
    pub kind: String,
    /// Volume AABB.
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::WmSegment`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmSegmentPayload {
    /// Link to a mask / segment id in an external store.
    pub segment_id: String,
    /// Covering AABB.
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::WmSensorFov`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmSensorFovPayload {
    /// Sensor kind (tof, radar, …).
    pub kind: String,
    /// Origin.
    pub origin: Vec3Wire,
    /// Approximate FOV AABB (or use [`FrustumPayload`] with Frustum tag).
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

/// [`super::SpatialLeafTag::WmAffordance`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WmAffordancePayload {
    /// Affordance kind (sit, grasp, …).
    pub kind: String,
    /// Interaction volume.
    pub bound: AabbWire,
    /// Optional external feature handle (ADR-088).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<VectorRef>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode, encode};
    use alloc::string::String;
    use alloc::vec;

    fn unit_aabb() -> AabbWire {
        AabbWire::from_min_max(Vec3Wire::new(-0.5, -0.5, -0.5), Vec3Wire::new(0.5, 0.5, 0.5))
    }

    #[test]
    fn roundtrip_sphere() {
        let p = SpherePayload {
            center: Vec3Wire::new(1.0, 2.0, 3.0),
            radius: 0.5,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<SpherePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_aabb() {
        let p = AabbPayload {
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<AabbPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_obb() {
        let p = ObbPayload {
            center: Vec3Wire::ZERO,
            half_extents: Vec3Wire::new(1.0, 2.0, 3.0),
            orientation: [0.0, 0.0, 0.0, 1.0],
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<ObbPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_capsule() {
        let p = CapsulePayload {
            a: Vec3Wire::new(0.0, 0.0, 0.0),
            b: Vec3Wire::new(0.0, 1.0, 0.0),
            radius: 0.1,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<CapsulePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_swept_aabb() {
        let p = SweptAabbPayload {
            start: unit_aabb(),
            end: AabbWire::from_min_max(Vec3Wire::new(0.5, 0.5, 0.5), Vec3Wire::new(1.5, 1.5, 1.5)),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<SweptAabbPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_frustum() {
        let p = FrustumPayload {
            planes: [
                [1.0, 0.0, 0.0, -1.0],
                [-1.0, 0.0, 0.0, -1.0],
                [0.0, 1.0, 0.0, -1.0],
                [0.0, -1.0, 0.0, -1.0],
                [0.0, 0.0, 1.0, -1.0],
                [0.0, 0.0, -1.0, -1.0],
            ],
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<FrustumPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_radial_sphere_event() {
        let p = RadialSphereEventPayload {
            center: Vec3Wire::new(0.0, 0.0, 1.0),
            radius: 2.0,
            t_start: 100.0,
            t_end: 200.0,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<RadialSphereEventPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_beam_trace() {
        let p = BeamTracePayload {
            origin: Vec3Wire::ZERO,
            direction: Vec3Wire::new(0.0, 0.0, 1.0),
            max_t: 10.0,
            radius: 0.05,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<BeamTracePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_sensor_read_4d() {
        let p = SensorRead4DPayload {
            center: Vec3Wire::new(1.0, 0.0, 0.0),
            half_extents: Vec3Wire::new(0.2, 0.2, 0.2),
            t: 1_700_000_000.0,
            sensor_id: 42,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<SensorRead4DPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_splat_scene() {
        let p = SplatScenePayload {
            job_id: String::from("job-abc"),
            bound: Some(unit_aabb()),
            artifacts: vec![
                (String::from("sog"), String::from("scene.sog")),
                (String::from("ply"), String::from("splat.ply")),
            ],
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<SplatScenePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_splat_camera() {
        let p = SplatCameraPayload {
            position: Vec3Wire::new(0.0, 1.5, 3.0),
            forward: Vec3Wire::new(0.0, 0.0, -1.0),
            up: Vec3Wire::new(0.0, 1.0, 0.0),
            fov_y: 1.0,
            near: 0.1,
            far: 100.0,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<SplatCameraPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_splat_yardstick() {
        let p = SplatYardstickPayload {
            a: Vec3Wire::ZERO,
            b: Vec3Wire::new(1.0, 0.0, 0.0),
            length_m: 1.0,
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<SplatYardstickPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_wm_object() {
        let p = WmObjectPayload {
            label: Some(String::from("chair")),
            confidence: Some(0.91),
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<WmObjectPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_wm_surface() {
        let p = WmSurfacePayload {
            label: Some(String::from("floor")),
            normal: Vec3Wire::new(0.0, 1.0, 0.0),
            point: Vec3Wire::ZERO,
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<WmSurfacePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_wm_volume() {
        let p = WmVolumePayload {
            kind: String::from("occupied"),
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<WmVolumePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_wm_segment() {
        let p = WmSegmentPayload {
            segment_id: String::from("seg-7"),
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<WmSegmentPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_wm_sensor_fov() {
        let p = WmSensorFovPayload {
            kind: String::from("tof"),
            origin: Vec3Wire::ZERO,
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<WmSensorFovPayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn roundtrip_wm_affordance() {
        let p = WmAffordancePayload {
            kind: String::from("sit"),
            bound: unit_aabb(),
            vector: None,
        };
        let bytes = encode(&p).unwrap();
        assert_eq!(decode::<WmAffordancePayload>(&bytes).unwrap(), p);
    }

    #[test]
    fn wm_object_vector_ref_roundtrip() {
        let p = WmObjectPayload {
            label: Some(String::from("chair")),
            confidence: Some(0.9),
            bound: unit_aabb(),
            vector: Some(VectorRef::ecc_hnsw(99)),
        };
        let bytes = encode(&p).unwrap();
        let q = decode::<WmObjectPayload>(&bytes).unwrap();
        assert_eq!(q.vector, Some(VectorRef::ecc_hnsw(99)));
    }

    #[test]
    fn legacy_sphere_without_vector_field_decodes() {
        // Encode a map without `vector` by using CBOR of partial-compatible shape:
        // Sphere with only center+radius via a slim struct then decode into full.
        #[derive(Serialize)]
        struct Legacy {
            center: Vec3Wire,
            radius: f32,
        }
        let legacy = Legacy {
            center: Vec3Wire::new(1.0, 0.0, 0.0),
            radius: 2.0,
        };
        let bytes = encode(&legacy).unwrap();
        let p = decode::<SpherePayload>(&bytes).unwrap();
        assert_eq!(p.radius, 2.0);
        assert_eq!(p.vector, None);
    }
}
