//! Leaf identity and payload shell (ADR-056 §3–4).

use crate::aabb::Aabb;
use serde::{Deserialize, Serialize};

/// Stable leaf handle after insertion into a tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeafId(pub u64);

/// Object leaves keep stable IDs across branches; event leaves are one-shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityKind {
    /// Persistent entity (scene, part, sensor).
    Object,
    /// Immutable observation (train run, pose sample).
    Event,
}

/// BVH leaf: broad-phase AABB + registry tag + opaque payload bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Leaf {
    /// Broad-phase bound.
    pub bound: Aabb,
    /// Object vs event semantics.
    pub identity_kind: IdentityKind,
    /// Registry tag (`weftos-leaf-types` spatial tags later; free u32 for now).
    pub tag: u32,
    /// Opaque payload (CBOR/JSON bytes; chain-safe refs only at higher layers).
    pub payload: Vec<u8>,
}

impl Leaf {
    /// Construct a leaf.
    pub fn new(bound: Aabb, identity_kind: IdentityKind, tag: u32, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            bound,
            identity_kind,
            tag,
            payload: payload.into(),
        }
    }

    /// Empty payload helper.
    pub fn empty_payload(bound: Aabb, identity_kind: IdentityKind, tag: u32) -> Self {
        Self::new(bound, identity_kind, tag, Vec::new())
    }
}

/// Reserved tag band for splat + world-model leaves (formal registry later).
/// Product contract: `docs/weftos/splat-to-world-model.md`, ADR-078.
pub mod tags {
    #![allow(dead_code)] // consumed when clawft-service-splat / BVH Phase B wire

    /// Whole reconstructed scene after a train job.
    pub const SPLAT_SCENE: u32 = 0x5350_0001; // "SP" + 1
    /// COLMAP / camera frustum observation.
    pub const SPLAT_CAMERA: u32 = 0x5350_0002;
    /// Yardstick / metric scale event.
    pub const SPLAT_YARDSTICK: u32 = 0x5350_0003;

    /// Labeled or unlabeled object instance (furniture, prop, …).
    pub const WM_OBJECT: u32 = 0x5350_0010;
    /// Planar / thin surface (floor, wall, tabletop).
    pub const WM_SURFACE: u32 = 0x5350_0011;
    /// Free / occupied / no-go volume.
    pub const WM_VOLUME: u32 = 0x5350_0012;
    /// Segment / mask-backed region linkage.
    pub const WM_SEGMENT: u32 = 0x5350_0013;
    /// Sensor observation volume (ToF, radar, …).
    pub const WM_SENSOR_FOV: u32 = 0x5350_0014;
    /// Interaction / affordance volume (policy later).
    pub const WM_AFFORDANCE: u32 = 0x5350_0015;
}
