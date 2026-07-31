//! Tagged-union narrow-phase registry (ADR-056 §3).

use crate::aabb::{Aabb, Ray, Vec3};
use crate::leaf::Leaf;
use std::collections::HashMap;

/// Narrow-phase test: does this leaf payload refine a broad-phase hit?
/// Return true to keep the leaf as a hit.
pub type NarrowPhaseFn = fn(query_aabb: Aabb, leaf: &Leaf) -> bool;

/// Registry of tag → interpreter. Phase A ships with a default AABB-only interpreter.
#[derive(Default)]
pub struct SpatialRegistry {
    map: HashMap<u32, NarrowPhaseFn>,
}

impl SpatialRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) a tag interpreter.
    pub fn register(&mut self, tag: u32, f: NarrowPhaseFn) {
        self.map.insert(tag, f);
    }

    /// Run narrow-phase for a leaf; missing tags default to AABB-only accept.
    pub fn refine_aabb(&self, query: Aabb, leaf: &Leaf) -> bool {
        if let Some(f) = self.map.get(&leaf.tag) {
            f(query, leaf)
        } else {
            // Default: broad-phase already matched.
            let _ = (query, leaf);
            true
        }
    }
}

/// Default narrow-phase: accept if leaf AABB intersects query (identity).
#[allow(dead_code)]
pub fn default_aabb_narrow(query: Aabb, leaf: &Leaf) -> bool {
    leaf.bound.intersects_aabb(query)
}

/// Placeholder for future sphere-in-payload refinement.
#[allow(dead_code)]
pub fn always_accept(_query: Aabb, _leaf: &Leaf) -> bool {
    true
}

/// Ray refine stub (broad-phase already filtered).
#[allow(dead_code)]
pub fn ray_accept(_ray: Ray, _origin: Vec3, _leaf: &Leaf) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leaf::{IdentityKind, Leaf};

    #[test]
    fn register_and_default() {
        let mut r = SpatialRegistry::new();
        r.register(1, default_aabb_narrow);
        let leaf = Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 1);
        assert!(r.refine_aabb(Aabb::unit(), &leaf));
        let other = Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 99);
        assert!(r.refine_aabb(Aabb::unit(), &other));
    }
}
