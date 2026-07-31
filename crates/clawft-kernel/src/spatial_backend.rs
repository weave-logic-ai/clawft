//! Unified spatial index backend trait (ADR-056 §8 / WEFT-718).
//!
//! Mirrors [`crate::vector_backend::VectorBackend`] conventions: epoch
//! versioning, `backend_name`, `Send + Sync` for `Arc` sharing. Compiled
//! only when the `ecc` feature is enabled.

use clawft_bvh::{
    Aabb, BranchId, BranchMeta, DiffEntry, Frustum, Leaf, LeafId, Ray, RayHit, Vec3,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error ────────────────────────────────────────────────────────────────

/// Errors from spatial backend operations.
#[derive(Debug, Error)]
pub enum SpatialError {
    /// Configured leaf capacity exceeded.
    #[error("capacity exceeded: max {max} leaves (current {current})")]
    CapacityExceeded {
        /// Configured max.
        max: usize,
        /// Current count.
        current: usize,
    },

    /// Optimistic concurrency conflict.
    #[error("epoch conflict: expected {expected}, actual {actual}")]
    EpochConflict {
        /// Caller's epoch.
        expected: u64,
        /// Store epoch.
        actual: u64,
    },

    /// Unknown branch id.
    #[error("unknown branch {0}")]
    UnknownBranch(u64),

    /// I/O during persistence.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic backend error.
    #[error("{0}")]
    Other(String),
}

/// Result alias for spatial backend operations.
pub type SpatialResult<T> = Result<T, SpatialError>;

// ── Backend kind (re-export shape for kernel callers) ────────────────────

/// Which spatial backend to construct (from config).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SpatialBackendKind {
    /// In-memory BVH (Phase C only variant).
    #[default]
    Bvh,
}

// ── Trait ────────────────────────────────────────────────────────────────

/// Unified spatial index backend interface (ADR-056 §8).
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async tasks via `Arc`.
pub trait SpatialBackend: Send + Sync {
    /// Insert a leaf; returns the assigned id.
    fn insert(&self, leaf: Leaf) -> SpatialResult<LeafId>;

    /// Remove a leaf by id. Returns `true` if it was present.
    fn remove(&self, id: LeafId) -> bool;

    /// Fetch a leaf by id.
    fn get(&self, id: LeafId) -> Option<Leaf>;

    /// Leaves whose AABB contains `p`.
    fn query_point(&self, p: Vec3) -> Vec<LeafId>;

    /// Leaves intersecting `bb`.
    fn query_aabb(&self, bb: Aabb) -> Vec<LeafId>;

    /// Leaves intersecting sphere.
    fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<LeafId>;

    /// Leaves hit by ray (sorted by enter `t`).
    fn query_ray(&self, ray: Ray, max_t: f32) -> Vec<RayHit>;

    /// Leaves intersecting a frustum.
    fn query_frustum(&self, frustum: &Frustum) -> Vec<LeafId>;

    /// k nearest leaves by AABB-center distance.
    fn query_knn(&self, p: Vec3, k: usize) -> Vec<LeafId>;

    /// Derive a child branch (COW in Phase D; full clone in Phase C).
    fn derive_branch(&self, meta: BranchMeta) -> SpatialResult<BranchId>;

    /// Presence-only branch diff inside `region`.
    fn branch_diff(&self, a: BranchId, b: BranchId, region: Aabb) -> Vec<DiffEntry>;

    /// Monotonic epoch (bumped on every mutation).
    fn epoch(&self) -> u64;

    /// Number of leaves on the active branch.
    fn len(&self) -> usize;

    /// True when the active branch has no leaves.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Human-readable backend name (e.g. `"bvh"`).
    fn backend_name(&self) -> &str;

    /// Optional capacity limit.
    fn max_leaves(&self) -> Option<usize> {
        None
    }

    /// Insert only if store epoch matches `parent_epoch`.
    fn insert_with_epoch(&self, leaf: Leaf, parent_epoch: u64) -> SpatialResult<LeafId> {
        let current = self.epoch();
        if parent_epoch < current {
            return Err(SpatialError::EpochConflict {
                expected: parent_epoch,
                actual: current,
            });
        }
        self.insert(leaf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_default_is_bvh() {
        assert_eq!(SpatialBackendKind::default(), SpatialBackendKind::Bvh);
    }

    #[test]
    fn error_display() {
        let err = SpatialError::CapacityExceeded {
            max: 10,
            current: 10,
        };
        assert!(format!("{err}").contains("10"));
        let err = SpatialError::EpochConflict {
            expected: 1,
            actual: 2,
        };
        assert!(format!("{err}").contains('1'));
    }
}
