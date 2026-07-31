//! `clawft-bvh` — BVH broad-phase spatial index (ADR-056).
//!
//! Phase A: AABB / Vec3 / Ray, Leaf, median-split tree, point/AABB/sphere/ray.
//! Phase E scaffold (WEFT-720): multi-branch [`store::BvhStore`], chain event
//! log + replay, membership-filter wrapper (OQ4).
//!
//! Kernel `SpatialBackend` / ExoChain dual-sign land in Phases C–D.

#![warn(missing_docs)]

mod aabb;
mod chain;
mod filter;
mod leaf;
mod query;
mod registry;
mod store;
mod tree;

pub use aabb::{Aabb, Ray, Vec3};
pub use chain::{BvhChainEvent, ChainSink, MemoryChainSink};
pub use filter::{MembershipFilter, apply_membership_filter};
pub use leaf::{BranchId, BranchMeta, IdentityKind, Leaf, LeafId};
pub use query::{RayHit, query_aabb, query_point, query_ray, query_sphere};
pub use registry::{NarrowPhaseFn, SpatialRegistry};
pub use store::{
    BvhError, BvhResult, BvhStore, DiffEntry, DiffKind, parse_aabb6, parse_identity, parse_tag,
    parse_vec3,
};
pub use tree::BvhTree;

/// Crate version string for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
