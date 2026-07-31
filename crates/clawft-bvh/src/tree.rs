//! Top-down median-split BVH (Phase A — SAH deferred).

use crate::aabb::{Aabb, Ray, Vec3};
use crate::leaf::{Leaf, LeafId};
use crate::query::RayHit;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone)]
enum Node {
    Internal {
        bound: Aabb,
        left: usize,
        right: usize,
    },
    Leaf {
        bound: Aabb,
        leaf_idx: usize,
    },
}

/// In-memory BVH tree (broad-phase structure; use [`crate::store::BvhStore`]
/// for the integrated insert/query surface).
#[derive(Debug, Default)]
pub struct BvhTree {
    leaves: Vec<(LeafId, Leaf)>,
    nodes: Vec<Node>,
    root: Option<usize>,
    next_id: u64,
}

impl BvhTree {
    /// Empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of leaves.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Insert a leaf; rebuilds the tree (fine for Phase A sizes).
    pub fn insert(&mut self, leaf: Leaf) -> LeafId {
        let id = LeafId(self.next_id);
        self.next_id += 1;
        self.leaves.push((id, leaf));
        self.rebuild();
        id
    }

    /// Remove by id; rebuilds.
    pub fn remove(&mut self, id: LeafId) -> bool {
        let before = self.leaves.len();
        self.leaves.retain(|(lid, _)| *lid != id);
        if self.leaves.len() == before {
            return false;
        }
        self.rebuild();
        true
    }

    /// Fetch leaf by id.
    pub fn get(&self, id: LeafId) -> Option<&Leaf> {
        self.leaves
            .iter()
            .find(|(lid, _)| *lid == id)
            .map(|(_, l)| l)
    }

    /// Root bound if any.
    pub fn root_bound(&self) -> Option<Aabb> {
        self.root.map(|i| match &self.nodes[i] {
            Node::Internal { bound, .. } | Node::Leaf { bound, .. } => *bound,
        })
    }

    /// Iterate all leaves.
    pub fn iter_leaves(&self) -> impl Iterator<Item = (LeafId, &Leaf)> {
        self.leaves.iter().map(|(id, l)| (*id, l))
    }

    fn rebuild(&mut self) {
        self.nodes.clear();
        self.root = None;
        if self.leaves.is_empty() {
            return;
        }
        let mut indices: Vec<usize> = (0..self.leaves.len()).collect();
        let root = self.build_range(&mut indices);
        self.root = Some(root);
    }

    fn build_range(&mut self, indices: &mut [usize]) -> usize {
        debug_assert!(!indices.is_empty());
        if indices.len() == 1 {
            let leaf_idx = indices[0];
            let bound = self.leaves[leaf_idx].1.bound;
            let node = Node::Leaf { bound, leaf_idx };
            let i = self.nodes.len();
            self.nodes.push(node);
            return i;
        }

        let mut bound = Aabb::empty();
        for &i in indices.iter() {
            bound = bound.union(self.leaves[i].1.bound);
        }
        let extents = bound.extents();
        let axis = if extents.x >= extents.y && extents.x >= extents.z {
            0
        } else if extents.y >= extents.z {
            1
        } else {
            2
        };
        indices.sort_by(|a, b| {
            let ca = self.leaves[*a].1.bound.center();
            let cb = self.leaves[*b].1.bound.center();
            let (va, vb) = match axis {
                0 => (ca.x, cb.x),
                1 => (ca.y, cb.y),
                _ => (ca.z, cb.z),
            };
            va.partial_cmp(&vb).unwrap_or(Ordering::Equal)
        });
        let mid = indices.len() / 2;
        let (left_slice, right_slice) = indices.split_at_mut(mid);
        // Recurse into owned buffers to avoid aliasing issues with self.nodes.
        let left_idx = {
            let mut left = left_slice.to_vec();
            self.build_range(&mut left)
        };
        let right_idx = {
            let mut right = right_slice.to_vec();
            self.build_range(&mut right)
        };
        let node = Node::Internal {
            bound,
            left: left_idx,
            right: right_idx,
        };
        let i = self.nodes.len();
        self.nodes.push(node);
        i
    }

    /// Collect leaf ids whose AABB satisfies `pred` (tree-pruned).
    pub(crate) fn collect_where(&self, mut pred: impl FnMut(Aabb) -> bool) -> Vec<LeafId> {
        let mut out = Vec::new();
        let Some(root) = self.root else {
            return out;
        };
        let mut stack = vec![root];
        while let Some(ni) = stack.pop() {
            match &self.nodes[ni] {
                Node::Internal { bound, left, right } => {
                    if pred(*bound) {
                        stack.push(*left);
                        stack.push(*right);
                    }
                }
                Node::Leaf { bound, leaf_idx } => {
                    if pred(*bound) {
                        out.push(self.leaves[*leaf_idx].0);
                    }
                }
            }
        }
        out
    }

    /// Ray-cast over the tree; hits sorted by enter distance ascending.
    pub(crate) fn collect_ray(&self, ray: Ray, max_t: f32) -> Vec<RayHit> {
        let mut hits = Vec::new();
        let Some(root) = self.root else {
            return hits;
        };
        let mut stack = vec![root];
        while let Some(ni) = stack.pop() {
            match &self.nodes[ni] {
                Node::Internal { bound, left, right } => {
                    if bound.intersects_ray(ray, max_t).is_some() {
                        stack.push(*left);
                        stack.push(*right);
                    }
                }
                Node::Leaf { bound, leaf_idx } => {
                    if let Some(t) = bound.intersects_ray(ray, max_t) {
                        hits.push(RayHit {
                            id: self.leaves[*leaf_idx].0,
                            t,
                        });
                    }
                }
            }
        }
        hits.sort_by(|a, b| {
            a.t.partial_cmp(&b.t)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        hits
    }

    /// k nearest leaves by squared distance from `p` to each leaf AABB.
    ///
    /// Uses branch-and-bound pruning: a subtree is skipped when its
    /// lower-bound distance is greater than the current k-th best.
    /// Ties break by `LeafId` ascending for deterministic results.
    pub(crate) fn collect_knn(&self, p: Vec3, k: usize) -> Vec<LeafId> {
        if k == 0 || self.is_empty() {
            return Vec::new();
        }
        let Some(root) = self.root else {
            return Vec::new();
        };

        // Max-heap of best k (farthest of the kept set on top).
        let mut best: BinaryHeap<KnnEntry> = BinaryHeap::new();
        // Min-heap of nodes by lower-bound distance.
        let mut open: BinaryHeap<NodeCand> = BinaryHeap::new();
        let root_bound = match &self.nodes[root] {
            Node::Internal { bound, .. } | Node::Leaf { bound, .. } => *bound,
        };
        open.push(NodeCand {
            dist_sq: root_bound.distance_sq_point(p),
            node: root,
        });

        while let Some(NodeCand { dist_sq, node }) = open.pop() {
            if best.len() >= k {
                if let Some(worst) = best.peek() {
                    if dist_sq > worst.dist_sq {
                        continue;
                    }
                }
            }
            match &self.nodes[node] {
                Node::Internal { left, right, .. } => {
                    for child in [*left, *right] {
                        let b = match &self.nodes[child] {
                            Node::Internal { bound, .. } | Node::Leaf { bound, .. } => *bound,
                        };
                        let d = b.distance_sq_point(p);
                        if best.len() < k || d <= best.peek().map(|e| e.dist_sq).unwrap_or(f32::MAX)
                        {
                            open.push(NodeCand {
                                dist_sq: d,
                                node: child,
                            });
                        }
                    }
                }
                Node::Leaf { bound, leaf_idx } => {
                    let id = self.leaves[*leaf_idx].0;
                    let d = bound.distance_sq_point(p);
                    let entry = KnnEntry { dist_sq: d, id };
                    if best.len() < k {
                        best.push(entry);
                    } else if let Some(worst) = best.peek() {
                        // Prefer closer; on equal distance prefer smaller id
                        // (KnnEntry Ord is max-heap oriented for the worst).
                        if entry.cmp_better(worst) {
                            best.pop();
                            best.push(entry);
                        }
                    }
                }
            }
        }

        let mut out: Vec<KnnEntry> = best.into_vec();
        out.sort_by(|a, b| {
            a.dist_sq
                .partial_cmp(&b.dist_sq)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        out.into_iter().map(|e| e.id).collect()
    }
}

/// Candidate node for knn branch-and-bound (BinaryHeap as min-heap via reverse Ord).
#[derive(Clone, Copy)]
struct NodeCand {
    dist_sq: f32,
    node: usize,
}

impl PartialEq for NodeCand {
    fn eq(&self, other: &Self) -> bool {
        self.dist_sq == other.dist_sq && self.node == other.node
    }
}
impl Eq for NodeCand {}
impl PartialOrd for NodeCand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NodeCand {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse: smaller dist_sq pops first from BinaryHeap.
        other
            .dist_sq
            .partial_cmp(&self.dist_sq)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.node.cmp(&self.node))
    }
}

/// Best-k entry. Ord is max-heap oriented: larger dist (then larger id) is greater
/// so `BinaryHeap::peek` yields the current worst of the kept set.
#[derive(Clone, Copy)]
struct KnnEntry {
    dist_sq: f32,
    id: LeafId,
}

impl KnnEntry {
    /// True if `self` should replace `worst` in the best-k set.
    fn cmp_better(&self, worst: &Self) -> bool {
        match self.dist_sq.partial_cmp(&worst.dist_sq).unwrap_or(Ordering::Equal) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => self.id.0 < worst.id.0,
        }
    }
}

impl PartialEq for KnnEntry {
    fn eq(&self, other: &Self) -> bool {
        self.dist_sq == other.dist_sq && self.id == other.id
    }
}
impl Eq for KnnEntry {}
impl PartialOrd for KnnEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for KnnEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist_sq
            .partial_cmp(&other.dist_sq)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.id.0.cmp(&other.id.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leaf::IdentityKind;

    fn box_leaf(x: f32) -> Leaf {
        Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 0.5, 0.5, 0.5)),
            IdentityKind::Object,
            1,
        )
    }

    #[test]
    fn insert_query_remove() {
        let mut t = BvhTree::new();
        let a = t.insert(box_leaf(0.0));
        let b = t.insert(box_leaf(5.0));
        assert_eq!(t.len(), 2);
        let hits = t.collect_where(|bb| {
            bb.intersects_aabb(Aabb::from_min_max(
                Vec3::new(-0.1, -0.1, -0.1),
                Vec3::new(1.0, 1.0, 1.0),
            ))
        });
        assert!(hits.contains(&a));
        assert!(!hits.contains(&b));
        assert!(t.remove(a));
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn knn_nearest() {
        let mut t = BvhTree::new();
        let near = t.insert(box_leaf(0.0));
        let _far = t.insert(box_leaf(20.0));
        let ids = t.collect_knn(Vec3::new(0.0, 0.0, 0.0), 1);
        assert_eq!(ids, vec![near]);
    }
}
