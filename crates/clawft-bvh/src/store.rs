//! Chain-anchored multi-branch `BvhStore` (ADR-056 §5 / §7, Phase D).
//!
//! - Pending mutations buffer per branch until [`BvhStore::seal_phase`].
//! - Seal sorts by `(priority_tier, exochain_seq)` then rebuilds the branch tree.
//! - [`BvhStore::derive_branch`] COW-shares sealed leaf maps via `Arc`.
//! - [`BvhStore::branch_diff`] returns exact region diffs between branches.

use crate::aabb::{Aabb, Ray, Vec3};
use crate::branch::{DiffEntry, branch_diff_maps};
use crate::chain::{ChainEvent, ChainEventKind, ChainSink, NullChainSink};
use crate::determinism::{PendingInsert, PendingMutation, PendingRemove, sort_pending};
use crate::leaf::{BranchId, BranchMeta, Leaf, LeafId};
use crate::query::{RayHit, query_aabb, query_point, query_ray, query_sphere};
use crate::tree::BvhTree;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

/// Errors from store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BvhError {
    /// Unknown branch id.
    UnknownBranch(BranchId),
    /// Soft capacity guard (`BvhStoreConfig::max_leaves`).
    StoreFull,
    /// Invalid phase end (must be ≥ last sealed seq).
    InvalidPhaseEnd {
        /// Last sealed sequence on the branch.
        last_sealed: u64,
        /// Requested phase end.
        requested: u64,
    },
}

impl std::fmt::Display for BvhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BvhError::UnknownBranch(id) => write!(f, "unknown branch {}", id.0),
            BvhError::StoreFull => write!(f, "BVH store full"),
            BvhError::InvalidPhaseEnd {
                last_sealed,
                requested,
            } => write!(
                f,
                "invalid phase end {requested} (last sealed {last_sealed})"
            ),
        }
    }
}

impl std::error::Error for BvhError {}

/// Result alias for store ops.
pub type BvhResult<T> = Result<T, BvhError>;

/// Optional capacity / phase knobs (mirrors planned `SpatialConfig` subset).
#[derive(Debug, Clone)]
pub struct BvhStoreConfig {
    /// Max sealed + pending leaves across all branches (soft guard).
    pub max_leaves: usize,
    /// Auto-seal when pending length reaches this (0 = never auto).
    pub phase_commit_threshold: usize,
}

impl Default for BvhStoreConfig {
    fn default() -> Self {
        Self {
            max_leaves: 1_000_000,
            phase_commit_threshold: 0,
        }
    }
}

struct BranchState {
    /// Stored for future status/CLI introspection (matches map key today).
    #[allow(dead_code)]
    id: BranchId,
    parent: Option<BranchId>,
    meta: BranchMeta,
    /// Sealed leaves — `Arc` shared COW with parent until first seal diverges.
    sealed: Arc<BTreeMap<LeafId, Leaf>>,
    /// Materialized broad-phase over sealed leaves.
    tree: BvhTree,
    pending: Vec<PendingMutation>,
    last_sealed_seq: u64,
}

/// Multi-branch BVH store with determinism-phase commit and COW derive.
pub struct BvhStore {
    branches: HashMap<BranchId, BranchState>,
    root: BranchId,
    next_branch: u64,
    next_leaf: u64,
    sink: Box<dyn ChainSink>,
    config: BvhStoreConfig,
    epoch: u64,
}

impl BvhStore {
    /// New store with a custom chain sink and config.
    pub fn new(sink: Box<dyn ChainSink>, config: BvhStoreConfig) -> Self {
        let root = BranchId(0);
        let mut branches = HashMap::new();
        branches.insert(
            root,
            BranchState {
                id: root,
                parent: None,
                meta: BranchMeta::new("root"),
                sealed: Arc::new(BTreeMap::new()),
                tree: BvhTree::new(),
                pending: Vec::new(),
                last_sealed_seq: 0,
            },
        );
        Self {
            branches,
            root,
            next_branch: 1,
            next_leaf: 1,
            sink,
            config,
            epoch: 0,
        }
    }

    /// Store with [`NullChainSink`] and default config.
    pub fn with_null_sink() -> Self {
        Self::new(Box::new(NullChainSink), BvhStoreConfig::default())
    }

    /// Root branch id (always exists).
    pub fn root_branch(&self) -> BranchId {
        self.root
    }

    /// Epoch bumped on every seal / derive.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Sealed leaf count on a branch.
    pub fn len(&self, branch: BranchId) -> BvhResult<usize> {
        Ok(self.branch(branch)?.sealed.len())
    }

    /// Pending mutation count on a branch.
    pub fn pending_len(&self, branch: BranchId) -> BvhResult<usize> {
        Ok(self.branch(branch)?.pending.len())
    }

    /// Snapshot sealed leaves for dual-store equality checks.
    pub fn sealed_snapshot(&self, branch: BranchId) -> BvhResult<Vec<(LeafId, Leaf)>> {
        let b = self.branch(branch)?;
        Ok(b.sealed.iter().map(|(k, v)| (*k, v.clone())).collect())
    }

    /// Queue an insert into the determinism-phase buffer.
    ///
    /// `LeafId` is assigned immediately (store-global) so callers can remove
    /// or reference it before seal. Visibility in queries includes pending.
    pub fn queue_insert(
        &mut self,
        branch: BranchId,
        leaf: Leaf,
        priority_tier: u8,
        exochain_seq: u64,
    ) -> BvhResult<LeafId> {
        self.ensure_capacity(1)?;
        let id = LeafId(self.next_leaf);
        self.next_leaf = self.next_leaf.saturating_add(1);
        {
            let b = self.branch_mut(branch)?;
            b.pending.push(PendingMutation::Insert(PendingInsert {
                leaf_id: id,
                leaf,
                priority_tier,
                exochain_seq,
            }));
        }
        let (pending_len, max_seq) = {
            let b = self.branch(branch)?;
            let max_seq = b
                .pending
                .iter()
                .map(|m| m.sort_key().1)
                .max()
                .unwrap_or(b.last_sealed_seq);
            (b.pending.len(), max_seq)
        };
        let threshold = self.config.phase_commit_threshold;
        if threshold > 0 && pending_len >= threshold {
            self.seal_phase(branch, max_seq)?;
        }
        Ok(id)
    }

    /// Queue a remove into the determinism-phase buffer.
    pub fn queue_remove(
        &mut self,
        branch: BranchId,
        leaf_id: LeafId,
        priority_tier: u8,
        exochain_seq: u64,
    ) -> BvhResult<()> {
        let b = self.branch_mut(branch)?;
        b.pending.push(PendingMutation::Remove(PendingRemove {
            leaf_id,
            priority_tier,
            exochain_seq,
        }));
        Ok(())
    }

    /// Seal the determinism phase: sort pending, apply, rebuild, emit chain.
    pub fn seal_phase(&mut self, branch: BranchId, exochain_seq_end: u64) -> BvhResult<()> {
        let last_sealed = self.branch(branch)?.last_sealed_seq;
        if exochain_seq_end < last_sealed {
            return Err(BvhError::InvalidPhaseEnd {
                last_sealed,
                requested: exochain_seq_end,
            });
        }

        let mut pending = {
            let b = self.branch_mut(branch)?;
            std::mem::take(&mut b.pending)
        };

        if pending.is_empty() {
            self.branch_mut(branch)?.last_sealed_seq = exochain_seq_end;
            return Ok(());
        }

        sort_pending(&mut pending);

        let mut chain_events = Vec::with_capacity(pending.len() + 1);
        let mut applied_payload = Vec::new();

        {
            let b = self.branch_mut(branch)?;
            let sealed = Arc::make_mut(&mut b.sealed);
            for m in pending {
                match m {
                    PendingMutation::Insert(ins) => {
                        applied_payload.extend_from_slice(&ins.leaf_id.0.to_le_bytes());
                        applied_payload.push(b'I');
                        sealed.insert(ins.leaf_id, ins.leaf);
                        chain_events.push(ChainEvent {
                            kind: ChainEventKind::InsertLeaf,
                            branch,
                            parent: None,
                            phase_end: exochain_seq_end,
                            payload: ins.leaf_id.0.to_le_bytes().to_vec(),
                        });
                    }
                    PendingMutation::Remove(rem) => {
                        applied_payload.extend_from_slice(&rem.leaf_id.0.to_le_bytes());
                        applied_payload.push(b'R');
                        sealed.remove(&rem.leaf_id);
                        chain_events.push(ChainEvent {
                            kind: ChainEventKind::RemoveLeaf,
                            branch,
                            parent: None,
                            phase_end: exochain_seq_end,
                            payload: rem.leaf_id.0.to_le_bytes().to_vec(),
                        });
                    }
                }
            }
            let snapshot: Vec<(LeafId, Leaf)> =
                sealed.iter().map(|(k, v)| (*k, v.clone())).collect();
            b.tree.load_leaves(snapshot);
            b.last_sealed_seq = exochain_seq_end;
        }

        chain_events.push(ChainEvent {
            kind: ChainEventKind::RebalanceSeal,
            branch,
            parent: None,
            phase_end: exochain_seq_end,
            payload: applied_payload,
        });

        for ev in chain_events {
            self.sink.emit(ev);
        }
        self.epoch = self.epoch.saturating_add(1);
        Ok(())
    }

    /// Derive a COW child branch sharing the parent's sealed leaf map.
    pub fn derive_branch(&mut self, parent: BranchId, meta: BranchMeta) -> BvhResult<BranchId> {
        let (sealed, last_sealed_seq) = {
            let parent_state = self.branch(parent)?;
            (
                Arc::clone(&parent_state.sealed),
                parent_state.last_sealed_seq,
            )
        };
        let mut tree = BvhTree::new();
        tree.load_leaves(sealed.iter().map(|(k, v)| (*k, v.clone())));

        let id = BranchId(self.next_branch);
        self.next_branch = self.next_branch.saturating_add(1);
        let label_bytes = meta.label.as_bytes().to_vec();
        self.branches.insert(
            id,
            BranchState {
                id,
                parent: Some(parent),
                meta,
                sealed,
                tree,
                pending: Vec::new(),
                last_sealed_seq,
            },
        );
        self.epoch = self.epoch.saturating_add(1);
        self.sink.emit(ChainEvent {
            kind: ChainEventKind::DeriveBranch,
            branch: id,
            parent: Some(parent),
            phase_end: last_sealed_seq,
            payload: label_bytes,
        });
        Ok(id)
    }

    /// Exact volume branch-diff over sealed state (pending excluded).
    pub fn branch_diff(
        &self,
        a: BranchId,
        b: BranchId,
        region: Aabb,
    ) -> BvhResult<Vec<DiffEntry>> {
        let ba = self.branch(a)?;
        let bb = self.branch(b)?;
        Ok(branch_diff_maps(&ba.sealed, &bb.sealed, region))
    }

    /// Query point on sealed + pending-visible state.
    pub fn query_point(&self, branch: BranchId, p: Vec3) -> BvhResult<Vec<LeafId>> {
        let b = self.branch(branch)?;
        let mut hits = query_point(&b.tree, p);
        merge_pending_point(b, p, &mut hits);
        hits.sort_by_key(|id| id.0);
        hits.dedup();
        Ok(hits)
    }

    /// Query AABB on sealed + pending-visible state.
    pub fn query_aabb(&self, branch: BranchId, bb: Aabb) -> BvhResult<Vec<LeafId>> {
        let b = self.branch(branch)?;
        let mut hits = query_aabb(&b.tree, bb);
        merge_pending_aabb(b, bb, &mut hits);
        hits.sort_by_key(|id| id.0);
        hits.dedup();
        Ok(hits)
    }

    /// Query sphere on sealed + pending-visible state.
    pub fn query_sphere(
        &self,
        branch: BranchId,
        center: Vec3,
        radius: f32,
    ) -> BvhResult<Vec<LeafId>> {
        let b = self.branch(branch)?;
        let mut hits = query_sphere(&b.tree, center, radius);
        merge_pending_sphere(b, center, radius, &mut hits);
        hits.sort_by_key(|id| id.0);
        hits.dedup();
        Ok(hits)
    }

    /// Query ray against sealed leaves (pending not ray-cast for Phase D).
    pub fn query_ray(
        &self,
        branch: BranchId,
        ray: Ray,
        max_t: f32,
    ) -> BvhResult<Vec<RayHit>> {
        let b = self.branch(branch)?;
        Ok(query_ray(&b.tree, ray, max_t))
    }

    /// Get sealed leaf (not pending).
    pub fn get_sealed(&self, branch: BranchId, id: LeafId) -> BvhResult<Option<Leaf>> {
        Ok(self.branch(branch)?.sealed.get(&id).cloned())
    }

    /// Parent branch of a COW child (`None` for root).
    pub fn parent_of(&self, branch: BranchId) -> BvhResult<Option<BranchId>> {
        Ok(self.branch(branch)?.parent)
    }

    /// Branch metadata label from derive.
    pub fn branch_meta(&self, branch: BranchId) -> BvhResult<&BranchMeta> {
        Ok(&self.branch(branch)?.meta)
    }

    /// Last sealed ExoChain sequence on the branch.
    pub fn last_sealed_seq(&self, branch: BranchId) -> BvhResult<u64> {
        Ok(self.branch(branch)?.last_sealed_seq)
    }

    fn branch(&self, id: BranchId) -> BvhResult<&BranchState> {
        self.branches
            .get(&id)
            .ok_or(BvhError::UnknownBranch(id))
    }

    fn branch_mut(&mut self, id: BranchId) -> BvhResult<&mut BranchState> {
        self.branches
            .get_mut(&id)
            .ok_or(BvhError::UnknownBranch(id))
    }

    fn ensure_capacity(&self, extra: usize) -> BvhResult<()> {
        let mut total = extra;
        for b in self.branches.values() {
            total = total
                .saturating_add(b.sealed.len())
                .saturating_add(b.pending.len());
        }
        if total > self.config.max_leaves {
            Err(BvhError::StoreFull)
        } else {
            Ok(())
        }
    }
}

fn merge_pending_point(b: &BranchState, p: Vec3, hits: &mut Vec<LeafId>) {
    let mut removed = HashSet::new();
    for m in &b.pending {
        match m {
            PendingMutation::Insert(ins) => {
                if ins.leaf.bound.contains_point(p) {
                    hits.push(ins.leaf_id);
                }
            }
            PendingMutation::Remove(rem) => {
                removed.insert(rem.leaf_id);
            }
        }
    }
    hits.retain(|id| !removed.contains(id));
}

fn merge_pending_aabb(b: &BranchState, bb: Aabb, hits: &mut Vec<LeafId>) {
    let mut removed = HashSet::new();
    for m in &b.pending {
        match m {
            PendingMutation::Insert(ins) => {
                if ins.leaf.bound.intersects_aabb(bb) {
                    hits.push(ins.leaf_id);
                }
            }
            PendingMutation::Remove(rem) => {
                removed.insert(rem.leaf_id);
            }
        }
    }
    hits.retain(|id| !removed.contains(id));
}

fn merge_pending_sphere(b: &BranchState, center: Vec3, radius: f32, hits: &mut Vec<LeafId>) {
    let mut removed = HashSet::new();
    for m in &b.pending {
        match m {
            PendingMutation::Insert(ins) => {
                if ins.leaf.bound.intersects_sphere(center, radius) {
                    hits.push(ins.leaf_id);
                }
            }
            PendingMutation::Remove(rem) => {
                removed.insert(rem.leaf_id);
            }
        }
    }
    hits.retain(|id| !removed.contains(id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branch::DiffPresence;
    use crate::chain::RecordingChainSink;
    use crate::leaf::IdentityKind;

    fn box_leaf(x: f32) -> Leaf {
        Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 0.5, 0.5, 0.5)),
            IdentityKind::Object,
            1,
        )
    }

    fn dual_stores() -> (BvhStore, BvhStore) {
        let a = BvhStore::new(
            Box::new(RecordingChainSink::new()),
            BvhStoreConfig::default(),
        );
        let b = BvhStore::new(
            Box::new(RecordingChainSink::new()),
            BvhStoreConfig::default(),
        );
        (a, b)
    }

    #[test]
    fn dual_store_deterministic_replay() {
        let (mut a, mut b) = dual_stores();
        let root = a.root_branch();
        assert_eq!(root, b.root_branch());

        for store in [&mut a, &mut b] {
            let _ = store.queue_insert(root, box_leaf(2.0), 2, 20).unwrap();
            let _ = store.queue_insert(root, box_leaf(0.0), 1, 5).unwrap();
            let mid = store.queue_insert(root, box_leaf(1.0), 1, 10).unwrap();
            store.queue_remove(root, mid, 1, 11).unwrap();
            store.seal_phase(root, 50).unwrap();
            let child = store
                .derive_branch(root, BranchMeta::new("experiment"))
                .unwrap();
            // Child inherits parent's last_sealed_seq; phase_end must be ≥ that.
            let extra = store.queue_insert(child, box_leaf(5.0), 0, 51).unwrap();
            store.queue_remove(child, extra, 0, 52).unwrap();
            let _keep = store.queue_insert(child, box_leaf(7.0), 0, 53).unwrap();
            store.seal_phase(child, 60).unwrap();
        }

        assert_eq!(
            a.sealed_snapshot(root).unwrap(),
            b.sealed_snapshot(root).unwrap()
        );
        let child = BranchId(1);
        assert_eq!(
            a.sealed_snapshot(child).unwrap(),
            b.sealed_snapshot(child).unwrap()
        );
        let region = Aabb::from_min_max(
            Vec3::new(-100.0, -100.0, -100.0),
            Vec3::new(100.0, 100.0, 100.0),
        );
        assert_eq!(
            a.branch_diff(root, child, region).unwrap(),
            b.branch_diff(root, child, region).unwrap()
        );
    }

    #[test]
    fn branch_diff_volume_exactness() {
        let mut store = BvhStore::with_null_sink();
        let a = store.root_branch();
        let mut ids = Vec::new();
        for i in 0..5 {
            let id = store
                .queue_insert(a, box_leaf(i as f32), 0, i as u64)
                .unwrap();
            ids.push(id);
        }
        store.seal_phase(a, 100).unwrap();

        let b = store.derive_branch(a, BranchMeta::new("b")).unwrap();
        store.queue_remove(b, ids[0], 0, 1).unwrap();
        store.queue_remove(b, ids[1], 0, 2).unwrap();
        let m0 = store.queue_insert(b, box_leaf(20.0), 0, 3).unwrap();
        let m1 = store.queue_insert(b, box_leaf(21.0), 0, 4).unwrap();
        store.seal_phase(b, 200).unwrap();

        let region =
            Aabb::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(30.0, 2.0, 2.0));
        let diff = store.branch_diff(a, b, region).unwrap();
        let only_a: Vec<_> = diff
            .iter()
            .filter(|d| d.presence == DiffPresence::OnlyInA)
            .map(|d| d.leaf_id)
            .collect();
        let only_b: Vec<_> = diff
            .iter()
            .filter(|d| d.presence == DiffPresence::OnlyInB)
            .map(|d| d.leaf_id)
            .collect();
        assert_eq!(only_a, vec![ids[0], ids[1]]);
        assert_eq!(only_b, vec![m0, m1]);
        assert_eq!(diff.len(), 4);

        let near =
            Aabb::from_min_max(Vec3::new(-0.1, -0.1, -0.1), Vec3::new(3.0, 1.0, 1.0));
        let near_diff = store.branch_diff(a, b, near).unwrap();
        assert_eq!(near_diff.len(), 2);
        assert!(near_diff
            .iter()
            .all(|d| d.presence == DiffPresence::OnlyInA));
    }

    #[test]
    fn cow_parent_unaffected_by_child_seal() {
        let mut store = BvhStore::with_null_sink();
        let root = store.root_branch();
        let id = store.queue_insert(root, box_leaf(0.0), 0, 1).unwrap();
        store.seal_phase(root, 1).unwrap();
        let child = store
            .derive_branch(root, BranchMeta::new("c"))
            .unwrap();
        store.queue_remove(child, id, 0, 2).unwrap();
        store.seal_phase(child, 2).unwrap();
        assert!(store.get_sealed(root, id).unwrap().is_some());
        assert!(store.get_sealed(child, id).unwrap().is_none());
        assert_eq!(store.len(root).unwrap(), 1);
        assert_eq!(store.len(child).unwrap(), 0);
    }

    #[test]
    fn seal_sort_applies_lower_tier_first() {
        let mut store = BvhStore::with_null_sink();
        let root = store.root_branch();
        let hi = store.queue_insert(root, box_leaf(1.0), 5, 1).unwrap();
        let lo = store.queue_insert(root, box_leaf(2.0), 0, 99).unwrap();
        store.seal_phase(root, 100).unwrap();
        let snap = store.sealed_snapshot(root).unwrap();
        // LeafIds assigned in queue order; both present after seal.
        assert_eq!(snap.len(), 2);
        assert!(snap.iter().any(|(id, _)| *id == lo));
        assert!(snap.iter().any(|(id, _)| *id == hi));
    }

    #[test]
    fn pending_visible_before_seal() {
        let mut store = BvhStore::with_null_sink();
        let root = store.root_branch();
        let id = store.queue_insert(root, box_leaf(0.0), 0, 1).unwrap();
        let hits = store
            .query_point(root, Vec3::new(0.25, 0.25, 0.25))
            .unwrap();
        assert_eq!(hits, vec![id]);
        assert_eq!(store.len(root).unwrap(), 0);
    }
}
