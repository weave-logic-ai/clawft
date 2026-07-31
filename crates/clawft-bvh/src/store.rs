//! Integrated [`BvhStore`] — multi-branch shell over [`BvhTree`] + [`ChainSink`].
//!
//! Phase C ships full insert/remove/query + chain emission. Phase D deepens
//! COW node sharing and determinism-phase seal; Phase C `derive` clones the
//! parent tree wholesale (correct, not yet share-efficient).

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::aabb::{Aabb, Frustum, Ray, Vec3};
use crate::chain::{BvhChainKind, ChainSink, NullChainSink};
use crate::leaf::{BranchId, BranchMeta, DiffEntry, Leaf, LeafId};
use crate::query::{RayHit, query_aabb, query_knn, query_point, query_ray, query_sphere};
use crate::tree::BvhTree;

/// Errors from store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BvhError {
    /// Configured max leaf capacity exceeded.
    CapacityExceeded {
        /// Configured max.
        max: usize,
        /// Current leaf count on the target branch.
        current: usize,
    },
    /// Unknown branch id.
    UnknownBranch(BranchId),
    /// Generic message (kept small; no alloc-heavy variants for no_std path).
    Other(&'static str),
}

impl std::fmt::Display for BvhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BvhError::CapacityExceeded { max, current } => {
                write!(f, "capacity exceeded: {current}/{max} leaves")
            }
            BvhError::UnknownBranch(id) => write!(f, "unknown branch {}", id.0),
            BvhError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for BvhError {}

/// Result alias for store ops.
pub type BvhResult<T> = Result<T, BvhError>;

/// Store configuration (mirrors `SpatialConfig` defaults at the type layer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BvhStoreConfig {
    /// Soft max leaves per branch (store-full guard). `0` = unbounded.
    pub max_leaves: usize,
    /// Pending mutations before a forced phase seal (Phase D; recorded only).
    pub phase_commit_threshold: usize,
}

impl Default for BvhStoreConfig {
    fn default() -> Self {
        Self {
            max_leaves: 1_000_000,
            phase_commit_threshold: 4096,
        }
    }
}

/// Per-branch tree handle.
struct BranchState {
    tree: BvhTree,
    /// Branch metadata (name, priority) — used by Phase D seal sort.
    #[allow(dead_code)]
    meta: BranchMeta,
    /// Parent branch for lineage (COW depth in Phase D).
    #[allow(dead_code)]
    parent: Option<BranchId>,
}

/// Integrated BVH store with optional chain sink.
///
/// Mutations always bump `epoch` and notify the sink. Queries read the
/// active (or specified) branch without chain traffic.
pub struct BvhStore {
    config: BvhStoreConfig,
    branches: HashMap<BranchId, BranchState>,
    active: BranchId,
    next_branch: u64,
    epoch: u64,
    sink: Arc<dyn ChainSink>,
}

impl Default for BvhStore {
    fn default() -> Self {
        Self::new(BvhStoreConfig::default())
    }
}

impl BvhStore {
    /// New store with null chain sink and a single main branch.
    pub fn new(config: BvhStoreConfig) -> Self {
        Self::with_sink(config, Arc::new(NullChainSink))
    }

    /// New store with an explicit chain sink.
    pub fn with_sink(config: BvhStoreConfig, sink: Arc<dyn ChainSink>) -> Self {
        let mut branches = HashMap::new();
        branches.insert(
            BranchId::MAIN,
            BranchState {
                tree: BvhTree::new(),
                meta: BranchMeta {
                    name: "main".into(),
                    priority_tier: 0,
                },
                parent: None,
            },
        );
        Self {
            config,
            branches,
            active: BranchId::MAIN,
            next_branch: 1,
            epoch: 0,
            sink,
        }
    }

    /// Replace the chain sink (e.g. after kernel attaches `ChainManager`).
    pub fn set_sink(&mut self, sink: Arc<dyn ChainSink>) {
        self.sink = sink;
    }

    /// Borrow config.
    pub fn config(&self) -> &BvhStoreConfig {
        &self.config
    }

    /// Monotonic epoch (bumped on every mutation).
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Active branch id.
    pub fn active_branch(&self) -> BranchId {
        self.active
    }

    /// Switch the default branch for insert/query when branch is omitted.
    pub fn set_active_branch(&mut self, id: BranchId) -> BvhResult<()> {
        if !self.branches.contains_key(&id) {
            return Err(BvhError::UnknownBranch(id));
        }
        self.active = id;
        Ok(())
    }

    /// Leaf count on the active branch.
    pub fn len(&self) -> usize {
        self.branches
            .get(&self.active)
            .map(|b| b.tree.len())
            .unwrap_or(0)
    }

    /// Empty active branch?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of branches.
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Insert on the active branch.
    pub fn insert(&mut self, leaf: Leaf) -> BvhResult<LeafId> {
        self.insert_on(self.active, leaf)
    }

    /// Insert on a specific branch.
    pub fn insert_on(&mut self, branch: BranchId, leaf: Leaf) -> BvhResult<LeafId> {
        let state = self
            .branches
            .get_mut(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        if self.config.max_leaves > 0 && state.tree.len() >= self.config.max_leaves {
            return Err(BvhError::CapacityExceeded {
                max: self.config.max_leaves,
                current: state.tree.len(),
            });
        }
        let id = state.tree.insert(leaf.clone());
        self.epoch = self.epoch.saturating_add(1);
        self.sink.on_event(&BvhChainKind::Insert {
            leaf_id: id,
            leaf,
            branch,
        });
        Ok(id)
    }

    /// Remove from the active branch.
    pub fn remove(&mut self, id: LeafId) -> bool {
        self.remove_on(self.active, id)
    }

    /// Remove from a specific branch.
    pub fn remove_on(&mut self, branch: BranchId, id: LeafId) -> bool {
        let Some(state) = self.branches.get_mut(&branch) else {
            return false;
        };
        if !state.tree.remove(id) {
            return false;
        }
        self.epoch = self.epoch.saturating_add(1);
        self.sink.on_event(&BvhChainKind::Remove {
            leaf_id: id,
            branch,
        });
        true
    }

    /// Fetch leaf on active branch.
    pub fn get(&self, id: LeafId) -> Option<&Leaf> {
        self.get_on(self.active, id)
    }

    /// Fetch leaf on a branch.
    pub fn get_on(&self, branch: BranchId, id: LeafId) -> Option<&Leaf> {
        self.branches.get(&branch)?.tree.get(id)
    }

    /// Query point on active branch.
    pub fn query_point(&self, p: Vec3) -> Vec<LeafId> {
        self.query_point_on(self.active, p)
    }

    /// Query point on a branch.
    pub fn query_point_on(&self, branch: BranchId, p: Vec3) -> Vec<LeafId> {
        self.branches
            .get(&branch)
            .map(|b| query_point(&b.tree, p))
            .unwrap_or_default()
    }

    /// Query AABB on active branch.
    pub fn query_aabb(&self, bb: Aabb) -> Vec<LeafId> {
        self.query_aabb_on(self.active, bb)
    }

    /// Query AABB on a branch.
    pub fn query_aabb_on(&self, branch: BranchId, bb: Aabb) -> Vec<LeafId> {
        self.branches
            .get(&branch)
            .map(|b| query_aabb(&b.tree, bb))
            .unwrap_or_default()
    }

    /// Query sphere on active branch.
    pub fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<LeafId> {
        self.query_sphere_on(self.active, center, radius)
    }

    /// Query sphere on a branch.
    pub fn query_sphere_on(&self, branch: BranchId, center: Vec3, radius: f32) -> Vec<LeafId> {
        self.branches
            .get(&branch)
            .map(|b| query_sphere(&b.tree, center, radius))
            .unwrap_or_default()
    }

    /// Query ray on active branch.
    pub fn query_ray(&self, ray: Ray, max_t: f32) -> Vec<RayHit> {
        self.query_ray_on(self.active, ray, max_t)
    }

    /// Query ray on a branch.
    pub fn query_ray_on(&self, branch: BranchId, ray: Ray, max_t: f32) -> Vec<RayHit> {
        self.branches
            .get(&branch)
            .map(|b| query_ray(&b.tree, ray, max_t))
            .unwrap_or_default()
    }

    /// Query frustum on active branch.
    pub fn query_frustum(&self, frustum: &Frustum) -> Vec<LeafId> {
        self.query_frustum_on(self.active, frustum)
    }

    /// Query frustum on a branch.
    pub fn query_frustum_on(&self, branch: BranchId, frustum: &Frustum) -> Vec<LeafId> {
        self.branches
            .get(&branch)
            .map(|b| {
                b.tree
                    .collect_where(|bb| frustum.intersects_aabb(bb))
            })
            .unwrap_or_default()
    }

    /// k-NN by AABB-center distance on active branch.
    pub fn query_knn(&self, p: Vec3, k: usize) -> Vec<LeafId> {
        self.query_knn_on(self.active, p, k)
    }

    /// k-NN on a branch.
    pub fn query_knn_on(&self, branch: BranchId, p: Vec3, k: usize) -> Vec<LeafId> {
        self.branches
            .get(&branch)
            .map(|b| query_knn(&b.tree, p, k))
            .unwrap_or_default()
    }

    /// Derive a child branch (Phase C: full tree clone; Phase D: COW).
    pub fn derive(&mut self, meta: BranchMeta) -> BvhResult<BranchId> {
        self.derive_from(self.active, meta)
    }

    /// Derive from a specific parent branch.
    pub fn derive_from(&mut self, parent: BranchId, meta: BranchMeta) -> BvhResult<BranchId> {
        let parent_state = self
            .branches
            .get(&parent)
            .ok_or(BvhError::UnknownBranch(parent))?;
        // Phase C: wholesale clone. Phase D replaces with COW node sharing.
        let tree = parent_state.tree.clone();
        let child = BranchId(self.next_branch);
        self.next_branch = self.next_branch.saturating_add(1);
        self.branches.insert(
            child,
            BranchState {
                tree,
                meta: meta.clone(),
                parent: Some(parent),
            },
        );
        self.epoch = self.epoch.saturating_add(1);
        self.sink.on_event(&BvhChainKind::Derive {
            parent,
            child,
            meta,
        });
        Ok(child)
    }

    /// Diff two branches inside `region` (presence-only).
    pub fn branch_diff(&self, a: BranchId, b: BranchId, region: Aabb) -> BvhResult<Vec<DiffEntry>> {
        let ta = self
            .branches
            .get(&a)
            .ok_or(BvhError::UnknownBranch(a))?;
        let tb = self
            .branches
            .get(&b)
            .ok_or(BvhError::UnknownBranch(b))?;

        let mut in_a: HashMap<LeafId, &Leaf> = HashMap::new();
        for (id, leaf) in ta.tree.iter_leaves() {
            if leaf.bound.intersects_aabb(region) {
                in_a.insert(id, leaf);
            }
        }
        let mut in_b: HashMap<LeafId, &Leaf> = HashMap::new();
        for (id, leaf) in tb.tree.iter_leaves() {
            if leaf.bound.intersects_aabb(region) {
                in_b.insert(id, leaf);
            }
        }

        let mut out = Vec::new();
        for (id, leaf) in &in_a {
            if !in_b.contains_key(id) {
                out.push(DiffEntry {
                    leaf_id: *id,
                    only_in: a,
                    bound: leaf.bound,
                });
            }
        }
        for (id, leaf) in &in_b {
            if !in_a.contains_key(id) {
                out.push(DiffEntry {
                    leaf_id: *id,
                    only_in: b,
                    bound: leaf.bound,
                });
            }
        }
        out.sort_by_key(|d| (d.leaf_id.0, d.only_in.0));
        Ok(out)
    }

    /// Emit a rebalance/seal chain event (Phase D will sort + rebalance).
    pub fn rebalance_seal(&mut self, branch: BranchId) -> BvhResult<()> {
        let count = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?
            .tree
            .len();
        // Rebuild is already done on every insert/remove in Phase A tree;
        // seal just witnesses the epoch boundary.
        self.epoch = self.epoch.saturating_add(1);
        self.sink.on_event(&BvhChainKind::RebalanceSeal {
            branch,
            leaf_count: count,
            epoch: self.epoch,
        });
        Ok(())
    }

    /// Snapshot all leaves on a branch (for equality / restart tests).
    pub fn snapshot_leaves(&self, branch: BranchId) -> BvhResult<Vec<(LeafId, Leaf)>> {
        let state = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        let mut v: Vec<(LeafId, Leaf)> = state
            .tree
            .iter_leaves()
            .map(|(id, l)| (id, l.clone()))
            .collect();
        v.sort_by_key(|(id, _)| id.0);
        Ok(v)
    }

    /// Replay a recorded chain event onto this store (restart path).
    ///
    /// Leaf ids from the event are **not** re-used by `BvhTree::insert`
    /// (which assigns its own sequence). For byte-identical replay tests,
    /// prefer [`Self::restore_leaves`] after collecting a snapshot, or
    /// compare by bound/tag/payload rather than LeafId.
    pub fn apply_chain_event(&mut self, event: &BvhChainKind) -> BvhResult<()> {
        match event {
            BvhChainKind::Insert {
                leaf_id: _,
                leaf,
                branch,
            } => {
                // Ensure branch exists (derive events create branches first).
                if !self.branches.contains_key(branch) {
                    return Err(BvhError::UnknownBranch(*branch));
                }
                let _ = self.insert_on(*branch, leaf.clone())?;
                Ok(())
            }
            BvhChainKind::Remove { leaf_id, branch } => {
                let _ = self.remove_on(*branch, *leaf_id);
                Ok(())
            }
            BvhChainKind::Derive {
                parent,
                child: _,
                meta,
            } => {
                let _ = self.derive_from(*parent, meta.clone())?;
                Ok(())
            }
            BvhChainKind::RebalanceSeal { branch, .. } => self.rebalance_seal(*branch),
        }
    }

    /// Replace active-branch leaves with an exact snapshot (restart helper).
    ///
    /// Clears the branch tree and re-inserts leaves in id order, preserving
    /// the original `LeafId` assignment via [`BvhTree::insert_with_id`].
    pub fn restore_leaves(
        &mut self,
        branch: BranchId,
        leaves: &[(LeafId, Leaf)],
    ) -> BvhResult<()> {
        let state = self
            .branches
            .get_mut(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        state.tree.clear();
        for (id, leaf) in leaves {
            state.tree.insert_with_id(*id, leaf.clone());
        }
        self.epoch = self.epoch.saturating_add(1);
        Ok(())
    }

    /// Iterate leaf ids on active branch (debug / status).
    pub fn leaf_ids(&self) -> Vec<LeafId> {
        self.branches
            .get(&self.active)
            .map(|b| b.tree.iter_leaves().map(|(id, _)| id).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::RecordingChainSink;
    use crate::leaf::IdentityKind;

    fn unit_leaf(tag: u32) -> Leaf {
        Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, tag)
    }

    #[test]
    fn insert_query_and_chain() {
        let sink = Arc::new(RecordingChainSink::new());
        let mut store = BvhStore::with_sink(BvhStoreConfig::default(), sink.clone());
        let id = store.insert(unit_leaf(7)).unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.query_point(Vec3::ZERO).contains(&id));
        assert_eq!(sink.len(), 1);
        assert_eq!(store.epoch(), 1);
    }

    #[test]
    fn derive_and_diff() {
        let mut store = BvhStore::new(BvhStoreConfig::default());
        let a = store
            .insert(Leaf::empty_payload(
                Aabb::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
                IdentityKind::Object,
                1,
            ))
            .unwrap();
        let child = store
            .derive(BranchMeta {
                name: "b".into(),
                priority_tier: 1,
            })
            .unwrap();
        store
            .insert_on(
                child,
                Leaf::empty_payload(
                    Aabb::from_min_max(Vec3::new(5.0, 0.0, 0.0), Vec3::new(6.0, 1.0, 1.0)),
                    IdentityKind::Event,
                    2,
                ),
            )
            .unwrap();
        store.remove_on(child, a);
        let region = Aabb::from_min_max(Vec3::new(-10.0, -10.0, -10.0), Vec3::new(10.0, 10.0, 10.0));
        let diff = store
            .branch_diff(BranchId::MAIN, child, region)
            .unwrap();
        assert!(diff.len() >= 2);
    }

    #[test]
    fn restore_leaves_roundtrip() {
        let mut store = BvhStore::new(BvhStoreConfig::default());
        let id = store.insert(unit_leaf(3)).unwrap();
        let snap = store.snapshot_leaves(BranchId::MAIN).unwrap();
        let mut store2 = BvhStore::new(BvhStoreConfig::default());
        store2.restore_leaves(BranchId::MAIN, &snap).unwrap();
        assert_eq!(store2.get(id).map(|l| l.tag), Some(3));
        assert_eq!(
            store.snapshot_leaves(BranchId::MAIN).unwrap(),
            store2.snapshot_leaves(BranchId::MAIN).unwrap()
        );
    }

    #[test]
    fn capacity_guard() {
        let mut store = BvhStore::new(BvhStoreConfig {
            max_leaves: 1,
            ..Default::default()
        });
        store.insert(unit_leaf(1)).unwrap();
        assert!(matches!(
            store.insert(unit_leaf(2)),
            Err(BvhError::CapacityExceeded { .. })
        ));
    }
}
