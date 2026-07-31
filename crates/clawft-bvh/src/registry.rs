//! Tagged-union narrow-phase registry (ADR-056 §3).
//!
//! # OQ1 — narrow-phase recursion ban (WEFT-716 / PLAN.md)
//!
//! A leaf's narrow-phase interpreter **must not** re-enter the BVH, call
//! other spatial queries, or invoke another interpreter. Narrow-phase is
//! leaf-local geometry only (payload vs query volume).
//!
//! Enforcement (Phase A):
//! 1. **Type-level**: [`NarrowPhaseFn`] receives only `(Aabb, &Leaf)` — no
//!    tree, store, or registry handle can be passed through the signature.
//! 2. **Registry-time / runtime**: [`SpatialRegistry::refine_aabb`] holds a
//!    thread-local re-entry guard. Nested calls return `false` and, in
//!    debug builds, panic. See [`NARROW_PHASE_RECURSION_BAN`].

use crate::aabb::{Aabb, Ray, Vec3};
use crate::leaf::Leaf;
use std::cell::Cell;
use std::collections::HashMap;

/// Documented resolution of PLAN.md OQ1: narrow-phase must stay leaf-local.
///
/// Value is the human-readable policy string for diagnostics / audits.
pub const NARROW_PHASE_RECURSION_BAN: &str =
    "OQ1: narrow-phase interpreters must not re-enter BVH/spatial queries; \
     enforced by NarrowPhaseFn signature (no tree handle) and thread-local \
     re-entry depth guard in SpatialRegistry::refine_aabb";

thread_local! {
    /// Depth of active narrow-phase calls on this thread (0 = idle).
    static NARROW_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Narrow-phase test: does this leaf payload refine a broad-phase hit?
/// Return true to keep the leaf as a hit.
///
/// **OQ1**: this signature intentionally omits any BVH / registry handle so
/// interpreters cannot re-enter spatial structures without `unsafe` or globals.
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

    /// Register (or replace) a **leaf-local** tag interpreter.
    ///
    /// Callers must ensure `f` does not re-enter spatial queries (OQ1).
    /// The runtime guard in [`Self::refine_aabb`] backs this contract.
    pub fn register(&mut self, tag: u32, f: NarrowPhaseFn) {
        self.map.insert(tag, f);
    }

    /// Number of registered interpreters.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when no custom interpreters are registered.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Run narrow-phase for a leaf; missing tags default to AABB-only accept.
    ///
    /// Returns `false` (and panics in debug) if called re-entrantly — OQ1 ban.
    pub fn refine_aabb(&self, query: Aabb, leaf: &Leaf) -> bool {
        let depth = NARROW_DEPTH.with(|d| d.get());
        if depth > 0 {
            #[cfg(debug_assertions)]
            panic!(
                "OQ1 narrow-phase recursion ban violated (depth={depth}): {}",
                NARROW_PHASE_RECURSION_BAN
            );
            #[cfg(not(debug_assertions))]
            {
                let _ = depth;
                return false;
            }
        }

        NARROW_DEPTH.with(|d| d.set(1));
        let result = if let Some(f) = self.map.get(&leaf.tag) {
            f(query, leaf)
        } else {
            // Default: broad-phase already matched.
            let _ = (query, leaf);
            true
        };
        NARROW_DEPTH.with(|d| d.set(0));
        result
    }

    /// Current re-entry depth (0 outside narrow-phase). Test / diagnostics aid.
    pub fn reentry_depth() -> u32 {
        NARROW_DEPTH.with(|d| d.get())
    }
}

/// Default narrow-phase: accept if leaf AABB intersects query (identity).
pub fn default_aabb_narrow(query: Aabb, leaf: &Leaf) -> bool {
    leaf.bound.intersects_aabb(query)
}

/// Placeholder for future sphere-in-payload refinement.
pub fn always_accept(_query: Aabb, _leaf: &Leaf) -> bool {
    true
}

/// Ray refine stub (broad-phase already filtered).
///
/// Reserved for Phase B/C narrow-phase ray tests; kept public so callers can
/// register it explicitly without inventing a private twin.
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
        assert!(!NARROW_PHASE_RECURSION_BAN.is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "OQ1")]
    fn reentry_panics_in_debug() {
        fn evil(query: Aabb, leaf: &Leaf) -> bool {
            // Re-enter refine via a static-like local registry would need
            // capture; simulate by calling refine on a fresh registry while
            // depth is already elevated by outer call.
            let r = SpatialRegistry::new();
            r.refine_aabb(query, leaf)
        }
        let mut r = SpatialRegistry::new();
        r.register(7, evil);
        let leaf = Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 7);
        let _ = r.refine_aabb(Aabb::unit(), &leaf);
    }
}
