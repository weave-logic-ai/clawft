//! Multi-branch `BvhStore` with chain event log (Phase E scaffold).
//!
//! Full COW node sharing + determinism-phase seal land in Phase D; this
//! store clones trees on [`BvhStore::derive_branch`] and records every
//! mutation so CLI E2E can prove insert → query → branch → diff + replay.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::aabb::{Aabb, Ray, Vec3};
use crate::chain::{BvhChainEvent, ChainSink, MemoryChainSink};
use crate::leaf::{BranchId, BranchMeta, IdentityKind, Leaf, LeafId};
use crate::query::{self, RayHit};
use crate::tree::BvhTree;

/// How a leaf differs between two branches inside a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    /// Present in `b` but not `a`.
    Added,
    /// Present in `a` but not `b`.
    Removed,
    /// Same id in both but bound/tag/payload differ.
    Changed,
}

/// One leaf-level difference from [`BvhStore::branch_diff`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Leaf id (stable across object branches when identity is Object).
    pub leaf_id: LeafId,
    /// Diff classification.
    pub kind: DiffKind,
}

/// Errors from store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BvhError {
    /// Unknown branch id.
    UnknownBranch(BranchId),
    /// Leaf not found on the branch.
    UnknownLeaf(LeafId),
}

impl std::fmt::Display for BvhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBranch(id) => write!(f, "unknown branch {}", id.0),
            Self::UnknownLeaf(id) => write!(f, "unknown leaf {}", id.0),
        }
    }
}

impl std::error::Error for BvhError {}

/// Result alias for store ops.
pub type BvhResult<T> = Result<T, BvhError>;

struct BranchState {
    tree: BvhTree,
    meta: BranchMeta,
    /// Parent branch when derived; `None` for main.
    #[allow(dead_code)] // exposed via future status / COW Phase D
    parent: Option<BranchId>,
}

/// In-memory multi-branch BVH with append-only chain log.
pub struct BvhStore {
    branches: HashMap<BranchId, BranchState>,
    next_branch: u64,
    /// Shared leaf-id allocator so object ids stay unique across derives.
    next_leaf: u64,
    epoch: u64,
    sink: MemoryChainSink,
}

impl Default for BvhStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BvhStore {
    /// Empty store with main branch `0`.
    pub fn new() -> Self {
        let mut branches = HashMap::new();
        branches.insert(
            BranchId::MAIN,
            BranchState {
                tree: BvhTree::new(),
                meta: BranchMeta::named("main"),
                parent: None,
            },
        );
        Self {
            branches,
            next_branch: 1,
            next_leaf: 0,
            epoch: 0,
            sink: MemoryChainSink::new(),
        }
    }

    /// Backend name for status surfaces.
    pub fn backend_name(&self) -> &'static str {
        "bvh-memory"
    }

    /// Monotonic epoch (bumped on seal / mutate).
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Total leaves across all branches (not unique objects).
    pub fn len(&self) -> usize {
        self.branches.values().map(|b| b.tree.len()).sum()
    }

    /// True when every branch is empty.
    pub fn is_empty(&self) -> bool {
        self.branches.values().all(|b| b.tree.is_empty())
    }

    /// Recorded chain events (Phase E scaffold; ExoChain later).
    pub fn events(&self) -> &[BvhChainEvent] {
        self.sink.events()
    }

    /// Leaf count on one branch.
    pub fn branch_len(&self, branch: BranchId) -> BvhResult<usize> {
        self.branches
            .get(&branch)
            .map(|b| b.tree.len())
            .ok_or(BvhError::UnknownBranch(branch))
    }

    /// List known branches with metadata.
    pub fn list_branches(&self) -> Vec<(BranchId, &BranchMeta, usize)> {
        let mut out: Vec<_> = self
            .branches
            .iter()
            .map(|(id, st)| (*id, &st.meta, st.tree.len()))
            .collect();
        out.sort_by_key(|(id, _, _)| id.0);
        out
    }

    /// Insert on `branch`; emits `Insert` chain event.
    pub fn insert(&mut self, branch: BranchId, leaf: Leaf) -> BvhResult<LeafId> {
        let id = LeafId(self.next_leaf);
        self.next_leaf += 1;
        {
            let st = self
                .branches
                .get_mut(&branch)
                .ok_or(BvhError::UnknownBranch(branch))?;
            // Use tree's insert but we need stable id — push via public API
            // by temporarily using tree.insert then rewriting id is messy;
            // rebuild path: remove default insert and use internal assign.
            let assigned = st.tree.insert_with_id(id, leaf.clone());
            debug_assert_eq!(assigned, id);
        }
        self.epoch += 1;
        self.sink.append(BvhChainEvent::Insert {
            branch,
            leaf_id: id,
            leaf,
        });
        Ok(id)
    }

    /// Remove leaf from branch; emits `Remove`.
    pub fn remove(&mut self, branch: BranchId, id: LeafId) -> BvhResult<bool> {
        let st = self
            .branches
            .get_mut(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        let removed = st.tree.remove(id);
        if removed {
            self.epoch += 1;
            self.sink.append(BvhChainEvent::Remove {
                branch,
                leaf_id: id,
            });
        }
        Ok(removed)
    }

    /// Fetch leaf on branch.
    pub fn get(&self, branch: BranchId, id: LeafId) -> Option<&Leaf> {
        self.branches.get(&branch)?.tree.get(id)
    }

    /// Point query.
    pub fn query_point(&self, branch: BranchId, p: Vec3) -> BvhResult<Vec<LeafId>> {
        let st = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        Ok(query::query_point(&st.tree, p))
    }

    /// AABB overlap query.
    pub fn query_aabb(&self, branch: BranchId, bb: Aabb) -> BvhResult<Vec<LeafId>> {
        let st = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        Ok(query::query_aabb(&st.tree, bb))
    }

    /// Sphere overlap query.
    pub fn query_sphere(
        &self,
        branch: BranchId,
        center: Vec3,
        radius: f32,
    ) -> BvhResult<Vec<LeafId>> {
        let st = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        Ok(query::query_sphere(&st.tree, center, radius))
    }

    /// Ray cast (sorted by enter distance).
    pub fn query_ray(
        &self,
        branch: BranchId,
        ray: Ray,
        max_t: f32,
    ) -> BvhResult<Vec<RayHit>> {
        let st = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        Ok(query::query_ray(&st.tree, ray, max_t))
    }

    /// k nearest leaves by AABB center distance (scaffold; Phase A knn later).
    pub fn query_knn(
        &self,
        branch: BranchId,
        p: Vec3,
        k: usize,
    ) -> BvhResult<Vec<LeafId>> {
        let st = self
            .branches
            .get(&branch)
            .ok_or(BvhError::UnknownBranch(branch))?;
        let mut scored: Vec<(f32, LeafId)> = st
            .tree
            .iter_leaves()
            .map(|(id, leaf)| {
                let c = leaf.bound.center();
                let d = (c - p).length_sq();
                (d, id)
            })
            .collect();
        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(k).map(|(_, id)| id).collect())
    }

    /// Derive a child branch (tree clone; true COW in Phase D).
    pub fn derive_branch(&mut self, parent: BranchId, meta: BranchMeta) -> BvhResult<BranchId> {
        let parent_tree = {
            let st = self
                .branches
                .get(&parent)
                .ok_or(BvhError::UnknownBranch(parent))?;
            st.tree.clone()
        };
        let child = BranchId(self.next_branch);
        self.next_branch += 1;
        self.branches.insert(
            child,
            BranchState {
                tree: parent_tree,
                meta: meta.clone(),
                parent: Some(parent),
            },
        );
        self.epoch += 1;
        self.sink.append(BvhChainEvent::Derive {
            parent,
            child,
            meta,
        });
        Ok(child)
    }

    /// Leaves that differ in `region` between branches `a` and `b`.
    pub fn branch_diff(
        &self,
        a: BranchId,
        b: BranchId,
        region: Aabb,
    ) -> BvhResult<Vec<DiffEntry>> {
        let ta = self
            .branches
            .get(&a)
            .ok_or(BvhError::UnknownBranch(a))?;
        let tb = self
            .branches
            .get(&b)
            .ok_or(BvhError::UnknownBranch(b))?;

        let ids_a: Vec<LeafId> = query::query_aabb(&ta.tree, region);
        let ids_b: Vec<LeafId> = query::query_aabb(&tb.tree, region);

        let set_a: std::collections::HashSet<LeafId> = ids_a.iter().copied().collect();
        let set_b: std::collections::HashSet<LeafId> = ids_b.iter().copied().collect();

        let mut out = Vec::new();
        for id in &ids_a {
            if !set_b.contains(id) {
                out.push(DiffEntry {
                    leaf_id: *id,
                    kind: DiffKind::Removed,
                });
            } else {
                let la = ta.tree.get(*id);
                let lb = tb.tree.get(*id);
                if la.map(|l| (l.tag, l.identity_kind, &l.bound, &l.payload))
                    != lb.map(|l| (l.tag, l.identity_kind, &l.bound, &l.payload))
                {
                    out.push(DiffEntry {
                        leaf_id: *id,
                        kind: DiffKind::Changed,
                    });
                }
            }
        }
        for id in &ids_b {
            if !set_a.contains(id) {
                out.push(DiffEntry {
                    leaf_id: *id,
                    kind: DiffKind::Added,
                });
            }
        }
        out.sort_by_key(|e| e.leaf_id.0);
        Ok(out)
    }

    /// Record a phase seal event (scaffold).
    pub fn seal_phase(&mut self, branch: BranchId) -> BvhResult<()> {
        if !self.branches.contains_key(&branch) {
            return Err(BvhError::UnknownBranch(branch));
        }
        self.epoch += 1;
        let epoch = self.epoch;
        self.sink.append(BvhChainEvent::RebalanceSeal { branch, epoch });
        Ok(())
    }

    /// Rebuild a store by replaying recorded events (deterministic).
    pub fn replay(events: &[BvhChainEvent]) -> Self {
        let mut store = Self::new();
        // Drop the empty main events from construction; replay applies all.
        store.sink.clear();
        store.epoch = 0;
        for ev in events {
            match ev {
                BvhChainEvent::Insert {
                    branch,
                    leaf_id,
                    leaf,
                } => {
                    // Ensure branch exists (derive may not have run if log is partial).
                    if !store.branches.contains_key(branch) {
                        store.branches.insert(
                            *branch,
                            BranchState {
                                tree: BvhTree::new(),
                                meta: BranchMeta::named(format!("branch-{}", branch.0)),
                                parent: None,
                            },
                        );
                        store.next_branch = store.next_branch.max(branch.0 + 1);
                    }
                    let st = store.branches.get_mut(branch).expect("branch just ensured");
                    st.tree.insert_with_id(*leaf_id, leaf.clone());
                    store.next_leaf = store.next_leaf.max(leaf_id.0 + 1);
                    store.epoch += 1;
                    store.sink.append(ev.clone());
                }
                BvhChainEvent::Remove { branch, leaf_id } => {
                    if let Some(st) = store.branches.get_mut(branch) {
                        st.tree.remove(*leaf_id);
                        store.epoch += 1;
                        store.sink.append(ev.clone());
                    }
                }
                BvhChainEvent::Derive {
                    parent,
                    child,
                    meta,
                } => {
                    let parent_tree = store
                        .branches
                        .get(parent)
                        .map(|s| s.tree.clone())
                        .unwrap_or_default();
                    store.branches.insert(
                        *child,
                        BranchState {
                            tree: parent_tree,
                            meta: meta.clone(),
                            parent: Some(*parent),
                        },
                    );
                    store.next_branch = store.next_branch.max(child.0 + 1);
                    store.epoch += 1;
                    store.sink.append(ev.clone());
                }
                BvhChainEvent::RebalanceSeal { branch: _, epoch } => {
                    store.epoch = (*epoch).max(store.epoch);
                    store.sink.append(ev.clone());
                }
            }
        }
        store
    }
}

/// Parse CLI tag names / numeric literals into registry tags.
pub fn parse_tag(s: &str) -> Result<u32, String> {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "sphere" | "aabb" | "object" | "wm_object" => Ok(crate::leaf::tags::WM_OBJECT),
        "surface" | "wm_surface" => Ok(crate::leaf::tags::WM_SURFACE),
        "volume" | "wm_volume" => Ok(crate::leaf::tags::WM_VOLUME),
        "segment" | "wm_segment" => Ok(crate::leaf::tags::WM_SEGMENT),
        "sensor" | "wm_sensor_fov" => Ok(crate::leaf::tags::WM_SENSOR_FOV),
        "affordance" | "wm_affordance" => Ok(crate::leaf::tags::WM_AFFORDANCE),
        "splat_scene" => Ok(crate::leaf::tags::SPLAT_SCENE),
        "splat_camera" => Ok(crate::leaf::tags::SPLAT_CAMERA),
        "splat_yardstick" => Ok(crate::leaf::tags::SPLAT_YARDSTICK),
        _ => {
            if let Some(hex) = lower.strip_prefix("0x") {
                u32::from_str_radix(hex, 16).map_err(|e| e.to_string())
            } else {
                s.parse::<u32>().map_err(|e| e.to_string())
            }
        }
    }
}

/// Parse `x,y,z` into [`Vec3`].
pub fn parse_vec3(s: &str) -> Result<Vec3, String> {
    let parts: Vec<_> = s.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!("expected x,y,z got {s:?}"));
    }
    let x: f32 = parts[0].parse().map_err(|e| format!("x: {e}"))?;
    let y: f32 = parts[1].parse().map_err(|e| format!("y: {e}"))?;
    let z: f32 = parts[2].parse().map_err(|e| format!("z: {e}"))?;
    Ok(Vec3::new(x, y, z))
}

/// Parse `minx,miny,minz,maxx,maxy,maxz` into [`Aabb`].
pub fn parse_aabb6(s: &str) -> Result<Aabb, String> {
    let parts: Vec<_> = s.split(',').map(str::trim).collect();
    if parts.len() != 6 {
        return Err(format!("expected minx,miny,minz,maxx,maxy,maxz got {s:?}"));
    }
    let vals: Result<Vec<f32>, _> = parts.iter().map(|p| p.parse()).collect();
    let vals = vals.map_err(|e| e.to_string())?;
    Ok(Aabb::from_min_max(
        Vec3::new(vals[0], vals[1], vals[2]),
        Vec3::new(vals[3], vals[4], vals[5]),
    ))
}

/// Parse identity kind from CLI string.
pub fn parse_identity(s: &str) -> Result<IdentityKind, String> {
    match s.to_ascii_lowercase().as_str() {
        "object" | "obj" => Ok(IdentityKind::Object),
        "event" | "evt" => Ok(IdentityKind::Event),
        other => Err(format!("unknown identity kind {other:?} (object|event)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_leaf(tag: u32) -> Leaf {
        Leaf::empty_payload(
            Aabb::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)),
            IdentityKind::Object,
            tag,
        )
    }

    #[test]
    fn insert_query_branch_diff_replay() {
        let mut s = BvhStore::new();
        let id = s.insert(BranchId::MAIN, unit_leaf(1)).unwrap();
        let hits = s
            .query_point(BranchId::MAIN, Vec3::new(0.5, 0.5, 0.5))
            .unwrap();
        assert_eq!(hits, vec![id]);

        let child = s
            .derive_branch(BranchId::MAIN, BranchMeta::named("exp"))
            .unwrap();
        let id2 = s
            .insert(
                child,
                Leaf::empty_payload(
                    Aabb::from_min_max(Vec3::new(5.0, 0.0, 0.0), Vec3::new(6.0, 1.0, 1.0)),
                    IdentityKind::Object,
                    2,
                ),
            )
            .unwrap();
        s.remove(child, id).unwrap();

        let region = Aabb::from_min_max(Vec3::new(-10.0, -10.0, -10.0), Vec3::new(10.0, 10.0, 10.0));
        let diff = s.branch_diff(BranchId::MAIN, child, region).unwrap();
        assert!(diff.iter().any(|d| d.leaf_id == id && d.kind == DiffKind::Removed));
        assert!(diff.iter().any(|d| d.leaf_id == id2 && d.kind == DiffKind::Added));

        let events = s.events().to_vec();
        let s2 = BvhStore::replay(&events);
        assert_eq!(s2.branch_len(BranchId::MAIN).unwrap(), 1);
        assert_eq!(s2.branch_len(child).unwrap(), 1);
        assert_eq!(
            s2.query_point(BranchId::MAIN, Vec3::new(0.5, 0.5, 0.5))
                .unwrap(),
            vec![id]
        );
        assert!(
            s2.query_point(child, Vec3::new(0.5, 0.5, 0.5))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            s2.query_point(child, Vec3::new(5.5, 0.5, 0.5)).unwrap(),
            vec![id2]
        );
    }

    #[test]
    fn parse_helpers() {
        assert_eq!(parse_tag("wm_object").unwrap(), crate::leaf::tags::WM_OBJECT);
        assert_eq!(parse_tag("0x10").unwrap(), 16);
        let bb = parse_aabb6("0,0,0,1,1,1").unwrap();
        assert!(bb.contains_point(Vec3::new(0.5, 0.5, 0.5)));
    }
}
