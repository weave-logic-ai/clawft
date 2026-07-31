//! ChainSink plug-in surface for BVH mutations (ADR-056 §6).
//!
//! Kernel adapters (Phase C) implement [`ChainSink`] and dual-sign events.
//! Standalone / offline tooling uses [`NullChainSink`] or
//! [`RecordingChainSink`] for deterministic replay tests (Phase D).

use crate::leaf::BranchId;
use serde::{Deserialize, Serialize};

/// Kind of BVH mutation written to the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainEventKind {
    /// Leaf queued then sealed into a branch.
    InsertLeaf,
    /// Leaf removed at phase seal.
    RemoveLeaf,
    /// COW child branch derived from a parent.
    DeriveBranch,
    /// Determinism-phase seal + lazy rebalance.
    RebalanceSeal,
}

/// Chain event payload shell (CBOR at the kernel boundary later).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainEvent {
    /// Event kind.
    pub kind: ChainEventKind,
    /// Branch the event applies to (child for derive).
    pub branch: BranchId,
    /// Parent branch for derive events.
    pub parent: Option<BranchId>,
    /// ExoChain sequence end of the sealed phase (0 for derive).
    pub phase_end: u64,
    /// Opaque payload bytes (leaf ids, meta label, etc.).
    pub payload: Vec<u8>,
}

/// Sink that receives BVH chain events.
///
/// Implementations must be deterministic for dual-store replay tests:
/// same call sequence → same stored event order.
pub trait ChainSink: Send {
    /// Emit one chain event.
    fn emit(&mut self, event: ChainEvent);
}

/// Discarding sink (offline / query-only tooling).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullChainSink;

impl ChainSink for NullChainSink {
    fn emit(&mut self, _event: ChainEvent) {}
}

/// Recording sink for dual-store determinism tests and CLI dry-runs.
#[derive(Debug, Default, Clone)]
pub struct RecordingChainSink {
    events: Vec<ChainEvent>,
}

impl RecordingChainSink {
    /// Empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recorded events in emission order.
    pub fn events(&self) -> &[ChainEvent] {
        &self.events
    }

    /// Drain recorded events.
    pub fn into_events(self) -> Vec<ChainEvent> {
        self.events
    }
}

impl ChainSink for RecordingChainSink {
    fn emit(&mut self, event: ChainEvent) {
        self.events.push(event);
    }
}
