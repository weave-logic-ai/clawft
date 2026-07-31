//! Brute-force differential tests for every broad-phase query type (WEFT-716).
//!
//! Runs ≥ 10k random scenarios across point / AABB / sphere / ray / frustum / knn.

use clawft_bvh::{
    Aabb, BvhStore, BvhStoreConfig, BvhTree, Frustum, IdentityKind, Leaf, LeafId, Ray, RayHit, Vec3,
    query_aabb, query_frustum, query_knn, query_point, query_ray, query_sphere,
};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashSet;

const SCENARIOS: usize = 10_000;
const MAX_LEAVES: usize = 48;
const WORLD: f32 = 20.0;

fn rand_vec3(rng: &mut StdRng, lo: f32, hi: f32) -> Vec3 {
    Vec3::new(
        rng.gen_range(lo..hi),
        rng.gen_range(lo..hi),
        rng.gen_range(lo..hi),
    )
}

fn rand_aabb(rng: &mut StdRng) -> Aabb {
    let c = rand_vec3(rng, -WORLD, WORLD);
    let e = Vec3::new(
        rng.gen_range(0.05..2.0),
        rng.gen_range(0.05..2.0),
        rng.gen_range(0.05..2.0),
    );
    Aabb::from_min_max(
        Vec3::new(c.x - e.x, c.y - e.y, c.z - e.z),
        Vec3::new(c.x + e.x, c.y + e.y, c.z + e.z),
    )
}

fn build_scene(rng: &mut StdRng) -> BvhTree {
    let n = rng.gen_range(0..=MAX_LEAVES);
    let mut tree = BvhTree::new();
    for _ in 0..n {
        tree.insert(Leaf::empty_payload(
            rand_aabb(rng),
            IdentityKind::Object,
            0,
        ));
    }
    tree
}

fn sorted_ids(mut v: Vec<LeafId>) -> Vec<LeafId> {
    v.sort_by_key(|id| id.0);
    v
}

fn brute_point(tree: &BvhTree, p: Vec3) -> Vec<LeafId> {
    sorted_ids(
        tree.iter_leaves()
            .filter(|(_, l)| l.bound.contains_point(p))
            .map(|(id, _)| id)
            .collect(),
    )
}

fn brute_aabb(tree: &BvhTree, bb: Aabb) -> Vec<LeafId> {
    sorted_ids(
        tree.iter_leaves()
            .filter(|(_, l)| l.bound.intersects_aabb(bb))
            .map(|(id, _)| id)
            .collect(),
    )
}

fn brute_sphere(tree: &BvhTree, c: Vec3, r: f32) -> Vec<LeafId> {
    sorted_ids(
        tree.iter_leaves()
            .filter(|(_, l)| l.bound.intersects_sphere(c, r))
            .map(|(id, _)| id)
            .collect(),
    )
}

fn brute_ray(tree: &BvhTree, ray: Ray, max_t: f32) -> Vec<RayHit> {
    let mut hits: Vec<RayHit> = tree
        .iter_leaves()
        .filter_map(|(id, l)| l.bound.intersects_ray(ray, max_t).map(|t| RayHit { id, t }))
        .collect();
    hits.sort_by(|a, b| {
        a.t.partial_cmp(&b.t)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.0.cmp(&b.id.0))
    });
    hits
}

fn brute_frustum(tree: &BvhTree, f: &Frustum) -> Vec<LeafId> {
    sorted_ids(
        tree.iter_leaves()
            .filter(|(_, l)| l.bound.intersects_frustum(f))
            .map(|(id, _)| id)
            .collect(),
    )
}

fn brute_knn(tree: &BvhTree, p: Vec3, k: usize) -> Vec<LeafId> {
    if k == 0 {
        return Vec::new();
    }
    let mut items: Vec<(f32, LeafId)> = tree
        .iter_leaves()
        .map(|(id, l)| (l.bound.distance_sq_point(p), id))
        .collect();
    items.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1 .0.cmp(&b.1 .0))
    });
    items.into_iter().take(k).map(|(_, id)| id).collect()
}

fn ray_hits_equal(a: &[RayHit], b: &[RayHit]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| {
        x.id == y.id && (x.t - y.t).abs() < 1e-4
    })
}

#[test]
fn differential_all_query_types_10k() {
    let mut rng = StdRng::seed_from_u64(0x00B7_0716);
    let mut counts = [0usize; 6]; // point, aabb, sphere, ray, frustum, knn

    for i in 0..SCENARIOS {
        let tree = build_scene(&mut rng);
        let kind = i % 6;
        counts[kind] += 1;

        match kind {
            0 => {
                let p = rand_vec3(&mut rng, -WORLD * 1.2, WORLD * 1.2);
                let got = sorted_ids(query_point(&tree, p));
                let exp = brute_point(&tree, p);
                assert_eq!(got, exp, "point mismatch scenario {i}");
            }
            1 => {
                let bb = rand_aabb(&mut rng);
                let got = sorted_ids(query_aabb(&tree, bb));
                let exp = brute_aabb(&tree, bb);
                assert_eq!(got, exp, "aabb mismatch scenario {i}");
            }
            2 => {
                let c = rand_vec3(&mut rng, -WORLD, WORLD);
                let r = rng.gen_range(0.1..5.0);
                let got = sorted_ids(query_sphere(&tree, c, r));
                let exp = brute_sphere(&tree, c, r);
                assert_eq!(got, exp, "sphere mismatch scenario {i}");
            }
            3 => {
                let origin = rand_vec3(&mut rng, -WORLD, WORLD);
                let dir = rand_vec3(&mut rng, -1.0, 1.0);
                // Avoid zero dir
                let dir = if dir.length_sq() < 1e-8 {
                    Vec3::new(1.0, 0.0, 0.0)
                } else {
                    dir
                };
                let ray = Ray::new(origin, dir);
                let max_t = rng.gen_range(1.0..80.0);
                let got = query_ray(&tree, ray, max_t);
                let exp = brute_ray(&tree, ray, max_t);
                assert!(
                    ray_hits_equal(&got, &exp),
                    "ray mismatch scenario {i}: got={got:?} exp={exp:?}"
                );
            }
            4 => {
                // Mix orthographic box frustum and perspective
                let f = if rng.gen_bool(0.5) {
                    Frustum::from_aabb(rand_aabb(&mut rng))
                } else {
                    let eye = rand_vec3(&mut rng, -WORLD, WORLD);
                    let forward = {
                        let d = rand_vec3(&mut rng, -1.0, 1.0);
                        if d.length_sq() < 1e-8 {
                            Vec3::new(0.0, 0.0, 1.0)
                        } else {
                            d
                        }
                    };
                    Frustum::perspective(
                        eye,
                        forward,
                        Vec3::new(0.0, 1.0, 0.0),
                        rng.gen_range(0.4..1.8),
                        rng.gen_range(0.5..2.0),
                        0.1,
                        rng.gen_range(5.0..40.0),
                    )
                };
                let got = sorted_ids(query_frustum(&tree, &f));
                let exp = brute_frustum(&tree, &f);
                assert_eq!(got, exp, "frustum mismatch scenario {i}");
            }
            _ => {
                let p = rand_vec3(&mut rng, -WORLD, WORLD);
                let k = rng.gen_range(0..=12);
                let got = query_knn(&tree, p, k);
                let exp = brute_knn(&tree, p, k);
                assert_eq!(got, exp, "knn mismatch scenario {i}");
            }
        }
    }

    let total: usize = counts.iter().sum();
    assert!(total >= SCENARIOS);
    // Each query class gets a fair share (~1666+)
    for (i, &c) in counts.iter().enumerate() {
        assert!(c >= SCENARIOS / 6, "query class {i} under-tested ({c})");
    }
}

#[test]
fn store_query_path_matches_tree() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut store = BvhStore::new(BvhStoreConfig {
        max_leaves: 10_000,
    });
    for _ in 0..64 {
        store
            .insert(Leaf::empty_payload(
                rand_aabb(&mut rng),
                IdentityKind::Object,
                0,
            ))
            .unwrap();
    }
    let p = Vec3::new(0.0, 0.0, 0.0);
    let tree_hits: HashSet<_> = query_point(store.tree(), p).into_iter().collect();
    let store_hits: HashSet<_> = store.query_point(p).into_iter().collect();
    assert_eq!(tree_hits, store_hits);

    let knn_t = query_knn(store.tree(), p, 5);
    let knn_s = store.query_knn(p, 5);
    assert_eq!(knn_t, knn_s);
}
