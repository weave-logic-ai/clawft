//! `clawft-bvh` — BVH broad-phase spatial index (ADR-056 Phase A).
//!
//! Standalone, no `clawft-kernel` dependency. Phase A ships:
//! - AABB / Vec3 / Ray primitives
//! - Leaf + LeafId + IdentityKind
//! - Top-down median-split BVH with point/AABB/sphere queries
//! - Tagged-union narrow-phase registry (stub interpreters)
//!
//! Chain binding, COW branches, and `SpatialBackend` live in later phases
//! (kernel adapters). Splat integration will register `splat.*` leaf tags
//! via `weftos-leaf-types` once that module lands.

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

/// Crate version string for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
