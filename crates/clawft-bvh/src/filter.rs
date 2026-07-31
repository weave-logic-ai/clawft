//! Membership filters (OQ4 — resolved for Phase E).
//!
//! ADR-056 §1 names multi-tenant membership filters as a v1 capability, but
//! the §8 `SpatialBackend` trait does not take a filter argument. **Decision
//! (WEFT-720 / OQ4):** keep query methods filter-free; apply membership via
//! a post-query wrapper [`apply_membership_filter`]. Kernel / CLI callers
//! that need tenancy scope compose:
//!
//! ```text
//! let ids = backend.query_aabb(bb);
//! let ids = apply_membership_filter(store, branch, ids, &filter);
//! ```
//!
//! Rationale: filter arg on every query freezes a tenancy model into the
//! broad-phase trait before product tenancy is defined; a wrapper keeps
//! `SpatialBackend` stable and lets agents / gate layers own policy.

use crate::leaf::{IdentityKind, LeafId};
use crate::store::BvhStore;

/// Optional post-query membership constraints.
#[derive(Debug, Clone, Default)]
pub struct MembershipFilter {
    /// If set, only leaves whose registry tag is in this list pass.
    pub allowed_tags: Option<Vec<u32>>,
    /// If set, only this identity kind passes.
    pub identity: Option<IdentityKind>,
}

impl MembershipFilter {
    /// No filtering (identity).
    pub fn none() -> Self {
        Self::default()
    }

    /// Tag allow-list only.
    pub fn tags(tags: impl Into<Vec<u32>>) -> Self {
        Self {
            allowed_tags: Some(tags.into()),
            identity: None,
        }
    }

    /// Whether this filter is a no-op.
    pub fn is_empty(&self) -> bool {
        self.allowed_tags.is_none() && self.identity.is_none()
    }
}

/// Apply [`MembershipFilter`] to a broad-phase hit list (OQ4 wrapper).
///
/// Leaves missing from `store` (stale ids) are dropped.
pub fn apply_membership_filter(
    store: &BvhStore,
    branch: crate::leaf::BranchId,
    ids: impl IntoIterator<Item = LeafId>,
    filter: &MembershipFilter,
) -> Vec<LeafId> {
    if filter.is_empty() {
        return ids.into_iter().collect();
    }
    ids.into_iter()
        .filter(|&id| {
            let Some(leaf) = store.get(branch, id) else {
                return false;
            };
            if let Some(ref tags) = filter.allowed_tags {
                if !tags.contains(&leaf.tag) {
                    return false;
                }
            }
            if let Some(kind) = filter.identity {
                if leaf.identity_kind != kind {
                    return false;
                }
            }
            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::{Aabb, Vec3};
    use crate::leaf::{BranchId, IdentityKind, Leaf};

    #[test]
    fn filter_by_tag() {
        let mut s = BvhStore::new();
        let a = s
            .insert(
                BranchId::MAIN,
                Leaf::empty_payload(
                    Aabb::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)),
                    IdentityKind::Object,
                    10,
                ),
            )
            .unwrap();
        let b = s
            .insert(
                BranchId::MAIN,
                Leaf::empty_payload(
                    Aabb::from_min_max(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)),
                    IdentityKind::Object,
                    20,
                ),
            )
            .unwrap();
        let hits = s
            .query_aabb(
                BranchId::MAIN,
                Aabb::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(10.0, 10.0, 10.0)),
            )
            .unwrap();
        assert_eq!(hits.len(), 2);
        let filtered = apply_membership_filter(
            &s,
            BranchId::MAIN,
            hits,
            &MembershipFilter::tags([10u32]),
        );
        assert_eq!(filtered, vec![a]);
        assert!(!filtered.contains(&b));
    }
}
