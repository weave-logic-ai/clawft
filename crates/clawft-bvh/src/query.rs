//! Broad-phase query helpers.

use crate::aabb::{Aabb, Ray, Vec3};
use crate::leaf::LeafId;
use crate::tree::BvhTree;

/// Ray hit against a leaf bound (broad-phase only).
#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    /// Leaf id.
    pub id: LeafId,
    /// Enter distance along ray.
    pub t: f32,
}

/// Leaves whose AABB contains `p`.
pub fn query_point(tree: &BvhTree, p: Vec3) -> Vec<LeafId> {
    tree.collect_where(|bb| bb.contains_point(p))
}

/// Leaves intersecting `bb`.
pub fn query_aabb(tree: &BvhTree, bb: Aabb) -> Vec<LeafId> {
    tree.collect_where(|b| b.intersects_aabb(bb))
}

/// Leaves intersecting sphere.
pub fn query_sphere(tree: &BvhTree, center: Vec3, radius: f32) -> Vec<LeafId> {
    tree.collect_where(|b| b.intersects_sphere(center, radius))
}

/// Leaves hit by ray, sorted by `t`.
pub fn query_ray(tree: &BvhTree, ray: Ray, max_t: f32) -> Vec<RayHit> {
    let mut hits = Vec::new();
    for (id, leaf) in tree.iter_leaves() {
        if let Some(t) = leaf.bound.intersects_ray(ray, max_t) {
            hits.push(RayHit { id, t });
        }
    }
    hits.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    hits
}

/// k nearest leaves by squared distance from `p` to AABB center.
pub fn query_knn(tree: &BvhTree, p: Vec3, k: usize) -> Vec<LeafId> {
    if k == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(f32, LeafId)> = tree
        .iter_leaves()
        .map(|(id, leaf)| {
            let c = leaf.bound.center();
            let d = (c - p).length_sq();
            (d, id)
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored.into_iter().map(|(_, id)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::Aabb;
    use crate::leaf::{IdentityKind, Leaf};
    use crate::tree::BvhTree;

    #[test]
    fn point_and_sphere() {
        let mut t = BvhTree::new();
        let id = t.insert(Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
            IdentityKind::Object,
            0,
        ));
        assert_eq!(query_point(&t, Vec3::new(0.5, 0.5, 0.5)), vec![id]);
        assert!(query_sphere(&t, Vec3::new(0.5, 0.5, 0.5), 0.1).contains(&id));
        assert!(query_point(&t, Vec3::new(3.0, 3.0, 3.0)).is_empty());
    }
}
