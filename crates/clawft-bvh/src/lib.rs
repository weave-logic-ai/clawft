//! `clawft-bvh` — BVH broad-phase spatial index (ADR-056).
//!
//! Standalone, no `clawft-kernel` dependency. Phase A ships:
//! - AABB / Vec3 / Ray primitives
//! - Leaf + LeafId + IdentityKind
//! - Top-down median-split BVH with point/AABB/sphere queries
//! - Tagged-union narrow-phase registry (stub interpreters)
//!
//! Phase B (WEFT-717): canonical tags live in
//! [`weftos_leaf_types::spatial`] and are re-exported here as
//! [`SpatialLeafTag`] and [`tags`].
//!
//! Chain binding, COW branches, and `SpatialBackend` live in later phases
//! (kernel adapters).

#![warn(missing_docs)]

mod aabb;
mod leaf;
mod query;
mod registry;
mod tree;

pub use aabb::{Aabb, Ray, Vec3};
pub use leaf::{IdentityKind, Leaf, LeafId};
pub use query::{RayHit, query_aabb, query_point, query_ray, query_sphere};
pub use registry::{NarrowPhaseFn, SpatialRegistry};
pub use tree::BvhTree;

/// Canonical spatial tag registry (`weftos-leaf-types`, ADR-056 Phase B).
pub use weftos_leaf_types::spatial::{
    SpatialLeafTag, UnknownSpatialTag, primitives as spatial_primitives, tag_consts as tags_consts,
};

/// `u32` tag constants (same discriminants as [`SpatialLeafTag`]).
pub use leaf::tags;

/// Crate version string for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
