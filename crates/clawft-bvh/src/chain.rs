//! Chain event surface for BVH mutations (ADR-056 §6).
//!
//! Phase E scaffold: in-memory event log + replay. Kernel `ChainSink`
//! wiring (ExoChain dual-sign) lands with Phase C (`SpatialService`).

use serde::{Deserialize, Serialize};

use crate::leaf::{BranchId, BranchMeta, Leaf, LeafId};

/// One BVH mutation or branch lifecycle event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BvhChainEvent {
    /// Leaf inserted on a branch.
    Insert {
        /// Target branch.
        branch: BranchId,
        /// Assigned leaf id.
        leaf_id: LeafId,
        /// Leaf payload + bound.
        leaf: Leaf,
    },
    /// Leaf removed from a branch.
    Remove {
        /// Target branch.
        branch: BranchId,
        /// Removed leaf id.
        leaf_id: LeafId,
    },
    /// COW-style branch derivation.
    Derive {
        /// Parent branch id.
        parent: BranchId,
        /// New child branch id.
        child: BranchId,
        /// Operator metadata.
        meta: BranchMeta,
    },
    /// Determinism-phase seal (Phase D full; scaffold records epoch only).
    RebalanceSeal {
        /// Branch sealed.
        branch: BranchId,
        /// Epoch after seal.
        epoch: u64,
    },
}

/// Consumer of BVH chain events (kernel plugs ExoChain here in Phase C).
pub trait ChainSink: Send {
    /// Append one event (order is authoritative for replay).
    fn append(&mut self, event: BvhChainEvent);
}

/// In-memory sink used by the Phase E CLI / E2E scaffold.
#[derive(Debug, Default, Clone)]
pub struct MemoryChainSink {
    events: Vec<BvhChainEvent>,
}

impl MemoryChainSink {
    /// Empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recorded events in append order.
    pub fn events(&self) -> &[BvhChainEvent] {
        &self.events
    }

    /// Drain events (e.g. for export).
    pub fn into_events(self) -> Vec<BvhChainEvent> {
        self.events
    }

    /// Clear without dropping capacity.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl ChainSink for MemoryChainSink {
    fn append(&mut self, event: BvhChainEvent) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::{Aabb, Vec3};
    use crate::leaf::IdentityKind;

    #[test]
    fn memory_sink_records() {
        let mut sink = MemoryChainSink::new();
        let leaf = Leaf::empty_payload(
            Aabb::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)),
            IdentityKind::Object,
            1,
        );
        sink.append(BvhChainEvent::Insert {
            branch: BranchId::MAIN,
            leaf_id: LeafId(0),
            leaf,
        });
        assert_eq!(sink.events().len(), 1);
    }
}
