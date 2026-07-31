//! COW branch helpers and volume branch-diff (ADR-056 §7).

use crate::aabb::Aabb;
use crate::leaf::{Leaf, LeafId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a leaf differs between two branches in a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffPresence {
    /// Present (and intersecting region) only on branch A.
    OnlyInA,
    /// Present (and intersecting region) only on branch B.
    OnlyInB,
    /// Same `LeafId` on both, content differs, bound intersects region.
    Changed,
}

/// One entry from [`crate::store::BvhStore::branch_diff`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Differing leaf.
    pub leaf_id: LeafId,
    /// Diff kind.
    pub presence: DiffPresence,
}

/// Exact volume branch-diff over sealed leaf maps.
///
/// A leaf is considered when its sealed AABB intersects `region`.
/// Returns entries sorted by `LeafId` for deterministic dual-store compare.
pub fn branch_diff_maps(
    a: &BTreeMap<LeafId, Leaf>,
    b: &BTreeMap<LeafId, Leaf>,
    region: Aabb,
) -> Vec<DiffEntry> {
    let mut out = Vec::new();
    let mut ids: Vec<LeafId> = a
        .keys()
        .chain(b.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    ids.sort_by_key(|id| id.0);

    for id in ids {
        let la = a.get(&id);
        let lb = b.get(&id);
        match (la, lb) {
            (Some(x), None) => {
                if x.bound.intersects_aabb(region) {
                    out.push(DiffEntry {
                        leaf_id: id,
                        presence: DiffPresence::OnlyInA,
                    });
                }
            }
            (None, Some(y)) => {
                if y.bound.intersects_aabb(region) {
                    out.push(DiffEntry {
                        leaf_id: id,
                        presence: DiffPresence::OnlyInB,
                    });
                }
            }
            (Some(x), Some(y)) => {
                if x != y && (x.bound.intersects_aabb(region) || y.bound.intersects_aabb(region))
                {
                    out.push(DiffEntry {
                        leaf_id: id,
                        presence: DiffPresence::Changed,
                    });
                }
            }
            (None, None) => unreachable!(),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::Vec3;
    use crate::leaf::IdentityKind;

    fn box_leaf(x: f32, tag: u32) -> Leaf {
        Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0)),
            IdentityKind::Object,
            tag,
        )
    }

    #[test]
    fn volume_filters_and_changed() {
        let mut a = BTreeMap::new();
        let mut b = BTreeMap::new();
        a.insert(LeafId(1), box_leaf(0.0, 1));
        a.insert(LeafId(2), box_leaf(10.0, 1)); // outside region
        b.insert(LeafId(1), box_leaf(0.0, 2)); // changed
        b.insert(LeafId(3), box_leaf(0.5, 1)); // only B
        let region = Aabb::from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(2.0, 2.0, 2.0));
        let d = branch_diff_maps(&a, &b, region);
        assert_eq!(
            d,
            vec![
                DiffEntry {
                    leaf_id: LeafId(1),
                    presence: DiffPresence::Changed,
                },
                DiffEntry {
                    leaf_id: LeafId(3),
                    presence: DiffPresence::OnlyInB,
                },
            ]
        );
    }
}
