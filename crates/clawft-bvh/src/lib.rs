//! `clawft-bvh` — BVH broad-phase spatial index (ADR-056 Phase A / WEFT-716).
//!
//! Standalone crate with **no** `clawft-kernel` dependency. Phase A ships:
//! - AABB / Vec3 / Ray / Frustum / Plane primitives
//! - Leaf + LeafId + IdentityKind
//! - Top-down median-split BVH with point / AABB / sphere / ray / frustum / knn
//! - Tagged-union narrow-phase registry with OQ1 recursion ban
//! - In-memory [`BvhStore`] (insert / remove / get + queries; optional
//!   [`ChainSink`] stub — no chain coupling until Phase C)
//!
//! Chain binding, COW branches, and kernel `SpatialBackend` live in later
//! phases. Splat / world-model tag constants remain provisional in
//! [`leaf::tags`] until Phase B moves them to `weftos-leaf-types`.

#![warn(missing_docs)]

mod aabb;
mod leaf;
mod query;
mod registry;
mod store;
mod tree;

pub use aabb::{Aabb, Frustum, Plane, Ray, Vec3};
pub use leaf::{IdentityKind, Leaf, LeafId};
pub use query::{
    RayHit, query_aabb, query_frustum, query_knn, query_point, query_ray, query_sphere,
};
pub use registry::{
    NARROW_PHASE_RECURSION_BAN, NarrowPhaseFn, SpatialRegistry, always_accept, default_aabb_narrow,
    ray_accept,
};
pub use store::{BvhError, BvhStore, BvhStoreConfig, ChainSink, NullChainSink};
pub use tree::BvhTree;

/// Crate version string for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
