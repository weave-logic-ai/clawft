//! ChainSink — plug-in audit hook for BVH mutations (ADR-056 §6).
//!
//! `clawft-bvh` has no kernel / tokio dependency. The kernel implements
//! [`ChainSink`] over `ChainManager` (CBOR/JSON payloads, dual-sign when
//! the chain path is live) and passes it into [`crate::store::BvhStore`].

use crate::leaf::{BranchId, BranchMeta, Leaf, LeafId};

/// Kind of mutation that must be witnessed on the chain (ADR-022).
#[derive(Debug, Clone, PartialEq)]
pub enum BvhChainKind {
    /// Leaf inserted (payload includes full leaf for replay).
    Insert {
        /// Assigned leaf id.
        leaf_id: LeafId,
        /// Leaf body (bound, tag, payload, identity).
        leaf: Leaf,
        /// Branch the insert landed on.
        branch: BranchId,
    },
    /// Leaf removed.
    Remove {
        /// Removed leaf id.
        leaf_id: LeafId,
        /// Branch the remove applied to.
        branch: BranchId,
    },
    /// COW / clone branch derivation.
    Derive {
        /// Parent branch.
        parent: BranchId,
        /// Newly created child.
        child: BranchId,
        /// Caller metadata.
        meta: BranchMeta,
    },
    /// Determinism-phase seal after rebalance (Phase D fills details).
    RebalanceSeal {
        /// Sealed branch.
        branch: BranchId,
        /// Leaf count at seal.
        leaf_count: usize,
        /// Epoch after seal.
        epoch: u64,
    },
}

/// Sink for BVH chain events. Implemented by the kernel; unit tests use
/// [`RecordingChainSink`].
pub trait ChainSink: Send + Sync {
    /// Observe a mutation. Implementations must not panic; failures are
    /// logged at the sink side (chain append is best-effort for liveness,
    /// but ADR-022 expects every write path to attempt append).
    fn on_event(&self, event: &BvhChainKind);
}

/// No-op sink (offline tooling / pure geometry benches).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullChainSink;

impl ChainSink for NullChainSink {
    fn on_event(&self, _event: &BvhChainKind) {}
}

/// In-memory recorder for tests and offline replay drivers.
#[derive(Debug, Default)]
pub struct RecordingChainSink {
    events: std::sync::Mutex<Vec<BvhChainKind>>,
}

impl RecordingChainSink {
    /// Empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of recorded events (clone).
    pub fn events(&self) -> Vec<BvhChainKind> {
        self.events
            .lock()
            .expect("RecordingChainSink lock poisoned")
            .clone()
    }

    /// Number of recorded events.
    pub fn len(&self) -> usize {
        self.events
            .lock()
            .expect("RecordingChainSink lock poisoned")
            .len()
    }

    /// True when no events recorded.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ChainSink for RecordingChainSink {
    fn on_event(&self, event: &BvhChainKind) {
        self.events
            .lock()
            .expect("RecordingChainSink lock poisoned")
            .push(event.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aabb::{Aabb, Vec3};
    use crate::leaf::IdentityKind;

    #[test]
    fn recording_sink_collects() {
        let sink = RecordingChainSink::new();
        let leaf = Leaf::empty_payload(Aabb::unit(), IdentityKind::Object, 1);
        sink.on_event(&BvhChainKind::Insert {
            leaf_id: LeafId(0),
            leaf,
            branch: BranchId::MAIN,
        });
        sink.on_event(&BvhChainKind::Remove {
            leaf_id: LeafId(0),
            branch: BranchId::MAIN,
        });
        assert_eq!(sink.len(), 2);
        let _ = Vec3::ZERO;
    }
}
