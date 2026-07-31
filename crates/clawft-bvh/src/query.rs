//! Broad-phase query helpers (point / AABB / sphere / ray / frustum / knn).

use crate::aabb::{Aabb, Frustum, Ray, Vec3};
use crate::leaf::LeafId;
use crate::tree::BvhTree;

/// Ray hit against a leaf bound (broad-phase only).
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Leaves hit by ray, sorted by enter distance `t` ascending.
pub fn query_ray(tree: &BvhTree, ray: Ray, max_t: f32) -> Vec<RayHit> {
    tree.collect_ray(ray, max_t)
}

/// Leaves whose AABB intersects the frustum volume.
pub fn query_frustum(tree: &BvhTree, frustum: &Frustum) -> Vec<LeafId> {
    tree.collect_where(|b| b.intersects_frustum(frustum))
}

/// `k` nearest leaves by squared distance from `p` to each leaf AABB.
///
/// Distance is the Euclidean distance to the closest point on the leaf
/// bound (zero when `p` is inside). Results are ordered nearest-first;
/// ties break by `LeafId` ascending.
pub fn query_knn(tree: &BvhTree, p: Vec3, k: usize) -> Vec<LeafId> {
    tree.collect_knn(p, k)
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

    #[test]
    fn frustum_and_knn() {
        let mut t = BvhTree::new();
        let near = t.insert(Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.5, 0.5)),
            IdentityKind::Object,
            0,
        ));
        let far = t.insert(Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(10.0, 0.0, 0.0), Vec3::new(10.5, 0.5, 0.5)),
            IdentityKind::Object,
            0,
        ));
        let region = Aabb::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let hits = query_frustum(&t, &Frustum::from_aabb(region));
        assert!(hits.contains(&near));
        assert!(!hits.contains(&far));

        let knn = query_knn(&t, Vec3::new(0.25, 0.25, 0.25), 1);
        assert_eq!(knn, vec![near]);
        let knn2 = query_knn(&t, Vec3::new(0.25, 0.25, 0.25), 2);
        assert_eq!(knn2.len(), 2);
        assert_eq!(knn2[0], near);
    }

    #[test]
    fn ray_sorted() {
        let mut t = BvhTree::new();
        let a = t.insert(Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(1.0, -0.1, -0.1), Vec3::new(1.5, 0.1, 0.1)),
            IdentityKind::Object,
            0,
        ));
        let b = t.insert(Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(3.0, -0.1, -0.1), Vec3::new(3.5, 0.1, 0.1)),
            IdentityKind::Object,
            0,
        ));
        let hits = query_ray(
            &t,
            Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            100.0,
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, a);
        assert_eq!(hits[1].id, b);
        assert!(hits[0].t < hits[1].t);
    }
}
