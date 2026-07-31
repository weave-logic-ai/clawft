//! Top-down median-split BVH (Phase A — SAH deferred).

use crate::aabb::Aabb;
use crate::leaf::{Leaf, LeafId};

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

/// In-memory BVH store (no chain / COW yet).
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
        self.leaves.iter().find(|(lid, _)| *lid == id).map(|(_, l)| l)
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
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
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

    /// Collect leaf indices intersecting a predicate on AABB.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::Vec3;
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
}
