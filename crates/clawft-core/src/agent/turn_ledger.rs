//! Chain-side witness of the turn-checkpoint bracket (WEFT-616 Phase 3 —
//! the DualStateBridge, agenticow plan §7).
//!
//! The bridge's two halves have opposite mutation disciplines: **memory is
//! genuinely reversible** (cheap COW discard via `BranchableMemory`), while
//! **the witness chain is append-only** — there is no truncating revert in
//! `ChainManager`, and there must never be. A [`TurnLedger`] therefore only
//! ever APPENDS: a checkpoint marker when a turn starts, a compensating
//! `TurnReverted` event when a turn aborts (the memory branch is discarded;
//! the *fact of the revert* is witnessed forever), and a lineage record when
//! a turn promotes.
//!
//! The trait lives here (clawft-core) so the bracket can call it, but the
//! production implementation is daemon-side over `ChainManager` — the same
//! inversion as `ConversationSink`/`EffectGate`. It is deliberately NOT
//! feature-gated: implementors need no cow types, and a no-op ledger is
//! valid (chain coupling off).

/// Lineage identifiers for a promoted turn, when the memory layer can supply
/// them (`RvfStore::file_id` of the promoted child and its parent). `None`
/// downgrades the promote witness from a full `record_lineage` to a plain
/// witnessed event — honest, but less verifiable; prefer supplying them.
#[derive(Debug, Clone, Copy)]
pub struct PromotedLineage {
    /// `file_id` of the promoted (child) store.
    pub child_id: [u8; 16],
    /// `file_id` of the parent (base) store the child collapsed into.
    pub parent_id: [u8; 16],
    /// Content hash of the parent at promote time (chain-verifiable).
    pub parent_hash: [u8; 32],
    /// Number of mutations the promote replayed (ingests + deletes).
    pub mutation_count: u32,
}

/// Observer the turn bracket drives at its three transition points. All
/// methods are infallible fire-and-forget from the bracket's perspective —
/// a ledger failure must never change turn semantics (log loudly inside the
/// implementation instead).
pub trait TurnLedger: Send + Sync {
    /// A turn checkpoint was taken; `label` identifies the turn (the same
    /// label recorded on the memory checkpoint, e.g. `"turn:{conv_id}"`).
    fn on_checkpoint(&self, label: &str);

    /// The turn failed and its memory branch was discarded. `turn_error` is
    /// the turn's own error display; `memory_rolled_back` reports whether the
    /// rollback itself succeeded (false = the branch may be dangling — the
    /// witness must say so).
    fn on_revert(&self, label: &str, turn_error: &str, memory_rolled_back: bool);

    /// The turn succeeded. `memory_promoted` reports whether the promote
    /// landed; `lineage` carries the store identifiers when available.
    fn on_promote(&self, label: &str, memory_promoted: bool, lineage: Option<PromotedLineage>);
}

/// No-op ledger: chain coupling disabled. Valid production state — the
/// bracket's memory semantics are unchanged; nothing is witnessed.
pub struct NoopTurnLedger;

impl TurnLedger for NoopTurnLedger {
    fn on_checkpoint(&self, _label: &str) {}
    fn on_revert(&self, _label: &str, _turn_error: &str, _memory_rolled_back: bool) {}
    fn on_promote(&self, _label: &str, _memory_promoted: bool, _lineage: Option<PromotedLineage>) {}
}
