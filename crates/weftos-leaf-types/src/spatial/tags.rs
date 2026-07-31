//! Canonical BVH leaf-tag registry (ADR-056 §3, WEFT-717).
//!
//! Every spatial leaf is `(AabbBound, identity_kind, tag: u32, payload)`.
//! The `tag` is a frozen discriminant from [`SpatialLeafTag`] so kernel,
//! mesh, dashboard, and external auditors agree without recompiling.
//!
//! # Tag bands
//!
//! | Band | Range | Purpose |
//! |------|-------|---------|
//! | Geometry | `0x4256_0001`–`0x4256_0FFF` | Narrow-phase primitives ("BV") |
//! | Splat / world-model | `0x5350_0001`–`0x5350_0FFF` | Splat + WM leaves ("SP") |
//!
//! # Deprecation (ADR-031 / ADR-056)
//!
//! Tag discriminants are a **wire-format contract**:
//!
//! - **Adding** a new tag is non-breaking (additive enum variant + const).
//! - **Renaming or repurposing** a discriminant is a wire-format break.
//! - Never reuse a retired `u32` value. Mark the old variant
//!   `#[deprecated]` and keep its discriminant reserved forever.
//! - New meaning → new tag number (never recycle).
//!
//! These rules mirror ADR-031 segment-tag stability for mesh wire
//! formats: consumers must be able to interpret historical chain /
//! mesh payloads after upgrades.

use core::convert::TryFrom;
use core::fmt;

use serde::{Deserialize, Serialize};

/// Stable u32 registry key for a BVH leaf payload shape.
///
/// Serialized as the raw `u32` discriminant (not a string name) so
/// CBOR on the chain stays compact and language-agnostic.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpatialLeafTag {
    // ── Geometry primitives (narrow-phase) ─────────────────────────
    /// Sphere: center + radius.
    Sphere = 0x4256_0001,
    /// Axis-aligned box (payload mirrors broad-phase AABB).
    Aabb = 0x4256_0002,
    /// Oriented bounding box (center, half-extents, quaternion).
    Obb = 0x4256_0003,
    /// Capsule: segment endpoints + radius.
    Capsule = 0x4256_0004,
    /// Swept AABB between two poses (motion volume).
    SweptAabb = 0x4256_0005,
    /// View / culling frustum (six planes).
    Frustum = 0x4256_0006,
    /// Time-bounded radial sphere event.
    RadialSphereEvent = 0x4256_0007,
    /// Directed beam / ray with finite thickness.
    BeamTrace = 0x4256_0008,
    /// 4D sensor observation volume (xyz + time).
    SensorRead4D = 0x4256_0009,

    // ── Splat / world-model (preserve 0x5350 band from Phase A) ────
    /// Whole reconstructed scene after a train job.
    SplatScene = 0x5350_0001,
    /// COLMAP / camera frustum observation.
    SplatCamera = 0x5350_0002,
    /// Yardstick / metric scale event.
    SplatYardstick = 0x5350_0003,
    /// Labeled or unlabeled object instance (furniture, prop, …).
    WmObject = 0x5350_0010,
    /// Planar / thin surface (floor, wall, tabletop).
    WmSurface = 0x5350_0011,
    /// Free / occupied / no-go volume.
    WmVolume = 0x5350_0012,
    /// Segment / mask-backed region linkage.
    WmSegment = 0x5350_0013,
    /// Sensor observation volume (ToF, radar, …).
    WmSensorFov = 0x5350_0014,
    /// Interaction / affordance volume (policy later).
    WmAffordance = 0x5350_0015,
}

impl SpatialLeafTag {
    /// All known tags in ascending discriminant order (snapshot / audit).
    pub const ALL: &'static [SpatialLeafTag] = &[
        Self::Sphere,
        Self::Aabb,
        Self::Obb,
        Self::Capsule,
        Self::SweptAabb,
        Self::Frustum,
        Self::RadialSphereEvent,
        Self::BeamTrace,
        Self::SensorRead4D,
        Self::SplatScene,
        Self::SplatCamera,
        Self::SplatYardstick,
        Self::WmObject,
        Self::WmSurface,
        Self::WmVolume,
        Self::WmSegment,
        Self::WmSensorFov,
        Self::WmAffordance,
    ];

    /// Stable snake_case name for diagnostics / CLI (not on the wire).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sphere => "sphere",
            Self::Aabb => "aabb",
            Self::Obb => "obb",
            Self::Capsule => "capsule",
            Self::SweptAabb => "swept_aabb",
            Self::Frustum => "frustum",
            Self::RadialSphereEvent => "radial_sphere_event",
            Self::BeamTrace => "beam_trace",
            Self::SensorRead4D => "sensor_read_4d",
            Self::SplatScene => "splat_scene",
            Self::SplatCamera => "splat_camera",
            Self::SplatYardstick => "splat_yardstick",
            Self::WmObject => "wm_object",
            Self::WmSurface => "wm_surface",
            Self::WmVolume => "wm_volume",
            Self::WmSegment => "wm_segment",
            Self::WmSensorFov => "wm_sensor_fov",
            Self::WmAffordance => "wm_affordance",
        }
    }

    /// Raw discriminant.
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

impl From<SpatialLeafTag> for u32 {
    fn from(tag: SpatialLeafTag) -> u32 {
        tag as u32
    }
}

/// Unknown or reserved tag discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownSpatialTag(pub u32);

impl fmt::Display for UnknownSpatialTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown spatial leaf tag: 0x{:08X}", self.0)
    }
}

impl TryFrom<u32> for SpatialLeafTag {
    type Error = UnknownSpatialTag;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        for &tag in Self::ALL {
            if tag as u32 == value {
                return Ok(tag);
            }
        }
        Err(UnknownSpatialTag(value))
    }
}

impl Serialize for SpatialLeafTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(*self as u32)
    }
}

impl<'de> Deserialize<'de> for SpatialLeafTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = u32::deserialize(deserializer)?;
        SpatialLeafTag::try_from(v).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for SpatialLeafTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(0x{:08X})", self.as_str(), self.as_u32())
    }
}

// ── Numeric constants (compat + clawft-bvh re-exports) ─────────────

/// Flat `u32` constants mirroring [`SpatialLeafTag`] for call sites that
/// only need the discriminant (registry maps, splat pipeline JSON).
pub mod consts {
    use super::SpatialLeafTag;

    /// [`SpatialLeafTag::Sphere`]
    pub const SPHERE: u32 = SpatialLeafTag::Sphere as u32;
    /// [`SpatialLeafTag::Aabb`]
    pub const AABB: u32 = SpatialLeafTag::Aabb as u32;
    /// [`SpatialLeafTag::Obb`]
    pub const OBB: u32 = SpatialLeafTag::Obb as u32;
    /// [`SpatialLeafTag::Capsule`]
    pub const CAPSULE: u32 = SpatialLeafTag::Capsule as u32;
    /// [`SpatialLeafTag::SweptAabb`]
    pub const SWEPT_AABB: u32 = SpatialLeafTag::SweptAabb as u32;
    /// [`SpatialLeafTag::Frustum`]
    pub const FRUSTUM: u32 = SpatialLeafTag::Frustum as u32;
    /// [`SpatialLeafTag::RadialSphereEvent`]
    pub const RADIAL_SPHERE_EVENT: u32 = SpatialLeafTag::RadialSphereEvent as u32;
    /// [`SpatialLeafTag::BeamTrace`]
    pub const BEAM_TRACE: u32 = SpatialLeafTag::BeamTrace as u32;
    /// [`SpatialLeafTag::SensorRead4D`]
    pub const SENSOR_READ_4D: u32 = SpatialLeafTag::SensorRead4D as u32;

    /// Whole reconstructed scene (`SPLAT_SCENE`).
    pub const SPLAT_SCENE: u32 = SpatialLeafTag::SplatScene as u32;
    /// COLMAP / camera frustum (`SPLAT_CAMERA`).
    pub const SPLAT_CAMERA: u32 = SpatialLeafTag::SplatCamera as u32;
    /// Yardstick / metric scale (`SPLAT_YARDSTICK`).
    pub const SPLAT_YARDSTICK: u32 = SpatialLeafTag::SplatYardstick as u32;
    /// Object instance (`WM_OBJECT`).
    pub const WM_OBJECT: u32 = SpatialLeafTag::WmObject as u32;
    /// Surface (`WM_SURFACE`).
    pub const WM_SURFACE: u32 = SpatialLeafTag::WmSurface as u32;
    /// Volume (`WM_VOLUME`).
    pub const WM_VOLUME: u32 = SpatialLeafTag::WmVolume as u32;
    /// Segment (`WM_SEGMENT`).
    pub const WM_SEGMENT: u32 = SpatialLeafTag::WmSegment as u32;
    /// Sensor FOV (`WM_SENSOR_FOV`).
    pub const WM_SENSOR_FOV: u32 = SpatialLeafTag::WmSensorFov as u32;
    /// Affordance (`WM_AFFORDANCE`).
    pub const WM_AFFORDANCE: u32 = SpatialLeafTag::WmAffordance as u32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode, encode};
    use alloc::vec::Vec;

    /// Frozen discriminant snapshot — must not change without ADR-031 deprecation.
    #[test]
    fn tag_discriminants_stable_snapshot() {
        let expected: &[(SpatialLeafTag, u32)] = &[
            (SpatialLeafTag::Sphere, 0x4256_0001),
            (SpatialLeafTag::Aabb, 0x4256_0002),
            (SpatialLeafTag::Obb, 0x4256_0003),
            (SpatialLeafTag::Capsule, 0x4256_0004),
            (SpatialLeafTag::SweptAabb, 0x4256_0005),
            (SpatialLeafTag::Frustum, 0x4256_0006),
            (SpatialLeafTag::RadialSphereEvent, 0x4256_0007),
            (SpatialLeafTag::BeamTrace, 0x4256_0008),
            (SpatialLeafTag::SensorRead4D, 0x4256_0009),
            (SpatialLeafTag::SplatScene, 0x5350_0001),
            (SpatialLeafTag::SplatCamera, 0x5350_0002),
            (SpatialLeafTag::SplatYardstick, 0x5350_0003),
            (SpatialLeafTag::WmObject, 0x5350_0010),
            (SpatialLeafTag::WmSurface, 0x5350_0011),
            (SpatialLeafTag::WmVolume, 0x5350_0012),
            (SpatialLeafTag::WmSegment, 0x5350_0013),
            (SpatialLeafTag::WmSensorFov, 0x5350_0014),
            (SpatialLeafTag::WmAffordance, 0x5350_0015),
        ];
        assert_eq!(SpatialLeafTag::ALL.len(), expected.len());
        for (i, &(tag, disc)) in expected.iter().enumerate() {
            assert_eq!(SpatialLeafTag::ALL[i], tag);
            assert_eq!(tag as u32, disc, "{tag}");
            assert_eq!(SpatialLeafTag::try_from(disc).unwrap(), tag);
            assert_eq!(consts_for(tag), disc);
        }
        // Legacy Phase A splat values preserved for splat-pipeline JSON.
        assert_eq!(consts::SPLAT_SCENE, 0x5350_0001);
        assert_eq!(consts::WM_OBJECT, 0x5350_0010);
    }

    fn consts_for(tag: SpatialLeafTag) -> u32 {
        match tag {
            SpatialLeafTag::Sphere => consts::SPHERE,
            SpatialLeafTag::Aabb => consts::AABB,
            SpatialLeafTag::Obb => consts::OBB,
            SpatialLeafTag::Capsule => consts::CAPSULE,
            SpatialLeafTag::SweptAabb => consts::SWEPT_AABB,
            SpatialLeafTag::Frustum => consts::FRUSTUM,
            SpatialLeafTag::RadialSphereEvent => consts::RADIAL_SPHERE_EVENT,
            SpatialLeafTag::BeamTrace => consts::BEAM_TRACE,
            SpatialLeafTag::SensorRead4D => consts::SENSOR_READ_4D,
            SpatialLeafTag::SplatScene => consts::SPLAT_SCENE,
            SpatialLeafTag::SplatCamera => consts::SPLAT_CAMERA,
            SpatialLeafTag::SplatYardstick => consts::SPLAT_YARDSTICK,
            SpatialLeafTag::WmObject => consts::WM_OBJECT,
            SpatialLeafTag::WmSurface => consts::WM_SURFACE,
            SpatialLeafTag::WmVolume => consts::WM_VOLUME,
            SpatialLeafTag::WmSegment => consts::WM_SEGMENT,
            SpatialLeafTag::WmSensorFov => consts::WM_SENSOR_FOV,
            SpatialLeafTag::WmAffordance => consts::WM_AFFORDANCE,
        }
    }

    #[test]
    fn unknown_tag_rejected() {
        assert!(SpatialLeafTag::try_from(0).is_err());
        assert!(SpatialLeafTag::try_from(0xDEAD_BEEF).is_err());
    }

    #[test]
    fn tag_cbor_roundtrip_as_u32() {
        for &tag in SpatialLeafTag::ALL {
            let bytes = encode(&tag).unwrap();
            let decoded: SpatialLeafTag = decode(&bytes).unwrap();
            assert_eq!(decoded, tag);
            // Also decodable as bare u32
            let as_u32: u32 = decode(&bytes).unwrap();
            assert_eq!(as_u32, tag as u32);
        }
    }

    #[test]
    fn all_tags_unique_discriminants() {
        let mut seen: Vec<u32> = SpatialLeafTag::ALL.iter().map(|t| *t as u32).collect();
        let n = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n);
    }
}
