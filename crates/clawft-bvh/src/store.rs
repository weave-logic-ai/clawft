//! Integrated in-memory BVH store (Phase A — no chain coupling).
//!
//! [`BvhStore`] is the consumer-facing surface for insert / remove / get and
//! all broad-phase query types. Kernel chain anchoring lands in Phase C via
//! the optional [`ChainSink`] plug-in; Phase A ships a no-op default.

use crate::aabb::{Aabb, Frustum, Ray, Vec3};
use crate::leaf::{Leaf, LeafId};
use crate::query::{self, RayHit};
use crate::registry::SpatialRegistry;
use crate::tree::BvhTree;

/// Errors from store mutations (Phase A surface; expanded in later phases).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BvhError {
    /// Store would exceed [`BvhStoreConfig::max_leaves`].
    StoreFull {
        /// Configured capacity.
        max_leaves: usize,
    },
}

impl std::fmt::Display for BvhError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BvhError::StoreFull { max_leaves } => {
                write!(f, "BVH store full (max_leaves={max_leaves})")
            }
        }
    }
}

impl std::error::Error for BvhError {}

/// Store configuration (in-memory Phase A subset of PLAN.md `SpatialConfig`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BvhStoreConfig {
    /// Hard cap on live leaves (insert fails with [`BvhError::StoreFull`]).
    pub max_leaves: usize,
}

impl Default for BvhStoreConfig {
    fn default() -> Self {
        Self {
            max_leaves: 1_000_000,
        }
    }
}

/// Chain event sink stub (kernel implements this in Phase C).
///
/// Phase A never requires a sink; when present, insert/remove notify it
/// after the in-memory mutation succeeds. No tokio / kernel types here.
pub trait ChainSink: Send {
    /// Called after a successful insert.
    fn on_insert(&mut self, id: LeafId, leaf: &Leaf);
    /// Called after a successful remove.
    fn on_remove(&mut self, id: LeafId);
}

/// No-op chain sink (default).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullChainSink;

impl ChainSink for NullChainSink {
    fn on_insert(&mut self, _id: LeafId, _leaf: &Leaf) {}
    fn on_remove(&mut self, _id: LeafId) {}
}

/// In-memory spatial store: tree + registry + optional chain sink.
///
/// **No** COW branches, determinism phase, or kernel deps (Phases C–D).
pub struct BvhStore {
    tree: BvhTree,
    registry: SpatialRegistry,
    config: BvhStoreConfig,
    epoch: u64,
    chain: Option<Box<dyn ChainSink>>,
}

impl Default for BvhStore {
    fn default() -> Self {
        Self::new(BvhStoreConfig::default())
    }
}

impl BvhStore {
    /// Create an empty store with the given config and no chain sink.
    pub fn new(config: BvhStoreConfig) -> Self {
        Self {
            tree: BvhTree::new(),
            registry: SpatialRegistry::new(),
            config,
            epoch: 0,
            chain: None,
        }
    }

    /// Attach a chain sink (Phase C kernel plugs real audit here).
    pub fn with_chain_sink(mut self, sink: Box<dyn ChainSink>) -> Self {
        self.chain = Some(sink);
        self
    }

    /// Mutable access to the narrow-phase registry.
    pub fn registry_mut(&mut self) -> &mut SpatialRegistry {
        &mut self.registry
    }

    /// Shared registry access.
    pub fn registry(&self) -> &SpatialRegistry {
        &self.registry
    }

    /// Config snapshot.
    pub fn config(&self) -> BvhStoreConfig {
        self.config
    }

    /// Monotonic epoch bumped on every successful mutation.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Live leaf count.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Backend name for diagnostics (mirrors `SpatialBackend::backend_name`).
    pub fn backend_name(&self) -> &'static str {
        "clawft-bvh"
    }

    /// Underlying tree (broad-phase only).
    pub fn tree(&self) -> &BvhTree {
        &self.tree
    }

    /// Insert a leaf. Fails if at capacity.
    pub fn insert(&mut self, leaf: Leaf) -> Result<LeafId, BvhError> {
        if self.tree.len() >= self.config.max_leaves {
            return Err(BvhError::StoreFull {
                max_leaves: self.config.max_leaves,
            });
        }
        let id = self.tree.insert(leaf);
        self.epoch = self.epoch.wrapping_add(1);
        if let Some(sink) = self.chain.as_mut() {
            if let Some(l) = self.tree.get(id) {
                // Clone payload path: get returns ref into tree; notify after.
                // Re-fetch is fine for Phase A sizes.
                let leaf_ref = l;
                sink.on_insert(id, leaf_ref);
            }
        }
        Ok(id)
    }

    /// Remove by id. Returns whether a leaf was removed.
    pub fn remove(&mut self, id: LeafId) -> bool {
        let removed = self.tree.remove(id);
        if removed {
            self.epoch = self.epoch.wrapping_add(1);
            if let Some(sink) = self.chain.as_mut() {
                sink.on_remove(id);
            }
        }
        removed
    }

    /// Fetch leaf by id.
    pub fn get(&self, id: LeafId) -> Option<&Leaf> {
        self.tree.get(id)
    }

    /// Point query with optional narrow-phase refine.
    pub fn query_point(&self, p: Vec3) -> Vec<LeafId> {
        self.refine_ids(query::query_point(&self.tree, p), |bb| {
            Aabb::from_min_max(p, p).union(bb)
        })
    }

    /// AABB overlap query with optional narrow-phase refine.
    pub fn query_aabb(&self, bb: Aabb) -> Vec<LeafId> {
        self.refine_ids(query::query_aabb(&self.tree, bb), |_| bb)
    }

    /// Sphere overlap query (narrow-phase uses AABB of the sphere).
    pub fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<LeafId> {
        let query_bb = Aabb::from_min_max(
            Vec3::new(center.x - radius, center.y - radius, center.z - radius),
            Vec3::new(center.x + radius, center.y + radius, center.z + radius),
        );
        self.refine_ids(query::query_sphere(&self.tree, center, radius), |_| query_bb)
    }

    /// Ray query (broad-phase hits; narrow-phase not applied to ray hits in Phase A).
    pub fn query_ray(&self, ray: Ray, max_t: f32) -> Vec<RayHit> {
        query::query_ray(&self.tree, ray, max_t)
    }

    /// Frustum cull with optional narrow-phase refine against frustum AABB approx.
    pub fn query_frustum(&self, frustum: &Frustum) -> Vec<LeafId> {
        // For narrow-phase we pass each leaf's own bound as the query volume
        // proxy is weak; use empty→identity via leaf bound union of hits is
        // unnecessary — default interpreters ignore query tightness.
        let ids = query::query_frustum(&self.tree, frustum);
        self.refine_ids(ids, |leaf_bb| leaf_bb)
    }

    /// k-NN by AABB distance (narrow-phase not applied; ordering is geometric).
    pub fn query_knn(&self, p: Vec3, k: usize) -> Vec<LeafId> {
        query::query_knn(&self.tree, p, k)
    }

    fn refine_ids(
        &self,
        ids: Vec<LeafId>,
        mut query_for: impl FnMut(Aabb) -> Aabb,
    ) -> Vec<LeafId> {
        ids.into_iter()
            .filter(|&id| {
                if let Some(leaf) = self.tree.get(id) {
                    let q = query_for(leaf.bound);
                    self.registry.refine_aabb(q, leaf)
                } else {
                    false
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leaf::IdentityKind;
    use std::sync::{Arc, Mutex};

    struct RecordingSink {
        inserts: Vec<LeafId>,
        removes: Vec<LeafId>,
    }

    impl ChainSink for RecordingSink {
        fn on_insert(&mut self, id: LeafId, _leaf: &Leaf) {
            self.inserts.push(id);
        }
        fn on_remove(&mut self, id: LeafId) {
            self.removes.push(id);
        }
    }

    // ChainSink requires ownership via Box; RecordingSink is used directly.
    // Wrap in a cell-like adapter for shared observation in tests.
    struct SharedSink(Arc<Mutex<RecordingSink>>);
    impl ChainSink for SharedSink {
        fn on_insert(&mut self, id: LeafId, leaf: &Leaf) {
            self.0.lock().unwrap().on_insert(id, leaf);
        }
        fn on_remove(&mut self, id: LeafId) {
            self.0.lock().unwrap().on_remove(id);
        }
    }

    #[test]
    fn insert_get_remove_query() {
        let mut s = BvhStore::new(BvhStoreConfig { max_leaves: 8 });
        let id = s
            .insert(Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 0))
            .unwrap();
        assert_eq!(s.len(), 1);
        assert!(s.get(id).is_some());
        assert!(s.query_point(Vec3::ZERO).contains(&id));
        assert!(s.remove(id));
        assert!(s.is_empty());
        assert_eq!(s.epoch(), 2);
    }

    #[test]
    fn store_full() {
        let mut s = BvhStore::new(BvhStoreConfig { max_leaves: 1 });
        s.insert(Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 0))
            .unwrap();
        let err = s
            .insert(Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 0))
            .unwrap_err();
        assert_eq!(err, BvhError::StoreFull { max_leaves: 1 });
    }

    #[test]
    fn chain_sink_notified() {
        let rec = Arc::new(Mutex::new(RecordingSink {
            inserts: vec![],
            removes: vec![],
        }));
        let mut s = BvhStore::new(BvhStoreConfig::default())
            .with_chain_sink(Box::new(SharedSink(Arc::clone(&rec))));
        let id = s
            .insert(Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 1))
            .unwrap();
        s.remove(id);
        let g = rec.lock().unwrap();
        assert_eq!(g.inserts, vec![id]);
        assert_eq!(g.removes, vec![id]);
    }
}
