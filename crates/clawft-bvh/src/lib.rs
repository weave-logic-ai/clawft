//! `clawft-bvh` — BVH broad-phase spatial index (ADR-056).
//!
//! ## Phases
//!
//! - **A**: AABB / Vec3 / Ray, Leaf, median-split tree, point/AABB/sphere/ray
//! - **D (WEFT-719)**: multi-branch [`BvhStore`], determinism-phase seal,
//!   COW [`BvhStore::derive_branch`], volume [`BvhStore::branch_diff`]
//!
//! Chain dual-sign + kernel `SpatialBackend` adapters land in Phase C (kernel);
//! this crate exposes [`ChainSink`] so they plug in without pulling `tokio`.
//!
//! Splat integration will register `splat.*` leaf tags via `weftos-leaf-types`
//! once Phase B lands.

#![warn(missing_docs)]

mod aabb;
mod branch;
mod chain;
mod determinism;
mod leaf;
mod query;
mod registry;
mod store;
mod tree;

pub use aabb::{Aabb, Ray, Vec3};
pub use branch::{DiffEntry, DiffPresence};
pub use chain::{ChainEvent, ChainEventKind, ChainSink, NullChainSink, RecordingChainSink};
pub use determinism::{PendingInsert, PendingMutation, PendingRemove, sort_pending};
pub use leaf::{BranchId, BranchMeta, IdentityKind, Leaf, LeafId};
pub use query::{RayHit, query_aabb, query_point, query_ray, query_sphere};
pub use registry::{NarrowPhaseFn, SpatialRegistry};
pub use store::{BvhError, BvhResult, BvhStore, BvhStoreConfig};
pub use tree::BvhTree;

/// Crate version string for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
