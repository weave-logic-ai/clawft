//! Determinism-phase buffer and seal ordering (ADR-056 §5).
//!
//! Pending mutations buffer per branch until [`crate::store::BvhStore::seal_phase`].
//! Within a phase, commits sort by `(priority_tier, exochain_seq)` ascending —
//! matching the F7 within-tier resolution rule used elsewhere in the substrate.

use crate::leaf::{Leaf, LeafId};
use serde::{Deserialize, Serialize};

/// One buffered insert awaiting phase seal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingInsert {
    /// Pre-assigned stable id (store-global monotonic).
    pub leaf_id: LeafId,
    /// Leaf body.
    pub leaf: Leaf,
    /// Priority tier (lower first).
    pub priority_tier: u8,
    /// ExoChain sequence for within-tier tie-break (ascending).
    pub exochain_seq: u64,
}

/// One buffered remove awaiting phase seal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRemove {
    /// Leaf to remove from sealed set.
    pub leaf_id: LeafId,
    /// Priority tier (lower first).
    pub priority_tier: u8,
    /// ExoChain sequence for within-tier tie-break (ascending).
    pub exochain_seq: u64,
}

/// Pending mutation in a determinism phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PendingMutation {
    /// Insert at seal.
    Insert(PendingInsert),
    /// Remove at seal.
    Remove(PendingRemove),
}

impl PendingMutation {
    /// Sort key: `(priority_tier, exochain_seq)` ascending.
    pub fn sort_key(&self) -> (u8, u64) {
        match self {
            PendingMutation::Insert(i) => (i.priority_tier, i.exochain_seq),
            PendingMutation::Remove(r) => (r.priority_tier, r.exochain_seq),
        }
    }

    /// Stable secondary key so equal keys still order deterministically
    /// (insert before remove at the same key would be surprising; we use
    /// leaf_id + kind tag instead).
    fn secondary_key(&self) -> (u8, u64) {
        match self {
            PendingMutation::Insert(i) => (0, i.leaf_id.0),
            PendingMutation::Remove(r) => (1, r.leaf_id.0),
        }
    }
}

/// Sort pending mutations into the seal order required by ADR-056 §5.
///
/// Primary: `(priority_tier, exochain_seq)` ascending.  
/// Secondary: insert before remove, then `LeafId` ascending — pure
/// determinism when callers stamp identical tiers/seqs.
pub fn sort_pending(pending: &mut [PendingMutation]) {
    pending.sort_by(|a, b| {
        a.sort_key()
            .cmp(&b.sort_key())
            .then_with(|| a.secondary_key().cmp(&b.secondary_key()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::{Aabb, Vec3};
    use crate::leaf::IdentityKind;

    fn leaf_at(x: f32) -> Leaf {
        Leaf::empty_payload(
            Aabb::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 0.5, 0.5, 0.5)),
            IdentityKind::Object,
            1,
        )
    }

    #[test]
    fn seal_order_priority_then_seq() {
        let mut pending = vec![
            PendingMutation::Insert(PendingInsert {
                leaf_id: LeafId(3),
                leaf: leaf_at(3.0),
                priority_tier: 2,
                exochain_seq: 1,
            }),
            PendingMutation::Insert(PendingInsert {
                leaf_id: LeafId(1),
                leaf: leaf_at(1.0),
                priority_tier: 1,
                exochain_seq: 99,
            }),
            PendingMutation::Insert(PendingInsert {
                leaf_id: LeafId(2),
                leaf: leaf_at(2.0),
                priority_tier: 1,
                exochain_seq: 10,
            }),
            PendingMutation::Remove(PendingRemove {
                leaf_id: LeafId(9),
                priority_tier: 1,
                exochain_seq: 10,
            }),
        ];
        sort_pending(&mut pending);
        let ids: Vec<u64> = pending
            .iter()
            .map(|m| match m {
                PendingMutation::Insert(i) => i.leaf_id.0,
                PendingMutation::Remove(r) => r.leaf_id.0,
            })
            .collect();
        // tier1 seq10 insert(2), tier1 seq10 remove(9), tier1 seq99 insert(1), tier2 insert(3)
        assert_eq!(ids, vec![2, 9, 1, 3]);
    }
}
