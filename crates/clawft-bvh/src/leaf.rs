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
    /// Registry tag — use [`crate::tags`] / [`weftos_leaf_types::spatial::SpatialLeafTag`].
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

/// Canonical spatial tags re-exported from `weftos-leaf-types` (WEFT-717).
///
/// Prefer [`crate::SpatialLeafTag`] or these `u32` constants — do not
/// redefine discriminants in other crates.
///
/// Product contract: `docs/weftos/splat-to-world-model.md`, ADR-056 §3, ADR-078.
pub mod tags {
    pub use weftos_leaf_types::spatial::tag_consts::*;
}

#[cfg(test)]
mod tests {
    use super::tags;
    use weftos_leaf_types::spatial::SpatialLeafTag;

    #[test]
    fn reexported_tags_match_canonical_registry() {
        assert_eq!(tags::SPLAT_SCENE, SpatialLeafTag::SplatScene as u32);
        assert_eq!(tags::SPLAT_SCENE, 0x5350_0001);
        assert_eq!(tags::WM_OBJECT, SpatialLeafTag::WmObject as u32);
        assert_eq!(tags::SPHERE, SpatialLeafTag::Sphere as u32);
        assert_eq!(tags::AABB, SpatialLeafTag::Aabb as u32);
    }
}
