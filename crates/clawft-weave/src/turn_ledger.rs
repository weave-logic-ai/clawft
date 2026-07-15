//! Daemon-side [`TurnLedger`] — the chain half of the WEFT-616 Phase 3
//! DualStateBridge (`.planning/ruv/integration/agenticow-integration-plan.md`
//! §7). Witnesses the `clawft-core::agent::turn_checkpoint` bracket's three
//! transitions onto the append-only exochain via [`ChainManager`].
//!
//! The trait ([`clawft_core::agent::turn_ledger::TurnLedger`]) lives in
//! clawft-core so the bracket can call it without depending on chain types;
//! this is the production implementation, wired at daemon boot only when a
//! cow-memory bracket is actually attached (a ledger with nothing to
//! observe is pointless — see `daemon.rs`'s cow-attach block).
//!
//! All three methods only ever APPEND. `ChainManager` is hash-linked and
//! append-only by design (chain.rs's own doc comment); there is no
//! truncating revert, and there must never be one here either — a reverted
//! turn's memory is discarded, but the *fact* that it was discarded is
//! witnessed forever (plan §7).

use std::sync::Arc;

use clawft_core::agent::turn_ledger::{PromotedLineage, TurnLedger};
use clawft_kernel::chain::ChainManager;
use tracing::{info, warn};

/// Hard cap on `turn_error` text written into a chain payload. Chain events
/// are witnessed forever — an unbounded error string (a stack trace, a raw
/// LLM dump) must not become a permanent storage liability.
const TURN_ERROR_CHAR_CAP: usize = 500;

/// Chain-witnessing [`TurnLedger`] over a real [`ChainManager`].
pub struct DaemonTurnLedger {
    chain: Arc<ChainManager>,
}

impl DaemonTurnLedger {
    pub fn new(chain: Arc<ChainManager>) -> Self {
        Self { chain }
    }
}

impl TurnLedger for DaemonTurnLedger {
    /// Witnesses the checkpoint two ways: a real `ChainManager::checkpoint()`
    /// — an O(1) in-memory marker (`ChainCheckpoint { chain_id, sequence,
    /// last_hash, timestamp, events_since_last }`, chain.rs:847), not a full
    /// state snapshot, so it is cheap and idiomatic to take one per turn —
    /// plus a labeled `cow.turn.checkpoint` event. The chain checkpoint
    /// alone is anonymous (keyed only by sequence number); pairing it with
    /// a label-carrying event makes it greppable back to the turn that
    /// requested it, without paying for anything heavier than a struct push
    /// and one append.
    fn on_checkpoint(&self, label: &str) {
        let cp = self.chain.checkpoint();
        self.chain.append(
            "turn_ledger",
            "cow.turn.checkpoint",
            Some(serde_json::json!({
                "label": label,
                "chain_checkpoint_sequence": cp.sequence,
            })),
        );
        info!(label, sequence = cp.sequence, "cow.turn.checkpoint witnessed");
    }

    /// Compensating `TurnReverted` event (plan §7): the memory branch was
    /// discarded (or a discard was attempted — `memory_rolled_back` reports
    /// which), and this appends the fact of that discard. History is never
    /// truncated to erase the checkpoint that preceded it.
    fn on_revert(&self, label: &str, turn_error: &str, memory_rolled_back: bool) {
        let truncated_error = truncate_chars(turn_error, TURN_ERROR_CHAR_CAP);
        self.chain.append(
            "turn_ledger",
            "cow.turn.reverted",
            Some(serde_json::json!({
                "label": label,
                "turn_error": truncated_error,
                "memory_rolled_back": memory_rolled_back,
            })),
        );
        if memory_rolled_back {
            info!(label, "cow.turn.reverted witnessed");
        } else {
            // The chain event stands regardless — the revert was attempted
            // and its outcome is exactly what we're witnessing — but flag
            // loudly since the memory branch may now be dangling.
            warn!(
                label,
                "cow.turn.reverted witnessed, but memory rollback itself failed — \
                 the discarded branch may be dangling on disk"
            );
        }
    }

    /// With `lineage` (the production path — `with_turn_checkpoint` fills it
    /// from `PromoteReport`, and `parent_hash` is real: RVF witnesses every
    /// ingest/delete by default), this is a full
    /// [`ChainManager::record_lineage`] witness — chain-verifiable via
    /// `verify_lineage()`. Without it, this downgrades honestly to a plain
    /// `cow.turn.promoted` event: still witnessed, just not verifiable
    /// lineage.
    ///
    /// `derivation_type` is [`rvf_types::DerivationType::Clone`]: a
    /// checkpoint's `working` node is produced by `MemoryNode::derive_from`
    /// — a COW clone of its parent at the moment of the checkpoint
    /// (`clawft-cow-memory/src/ops.rs::checkpoint`) — so "clone" honestly
    /// describes *how the child came to exist*. `mutation_count` is the
    /// separate field that captures how far it diverged from that clone by
    /// promote time; `Merge` was considered and rejected because this
    /// crate's lineage is single-parent (one child collapsing into one
    /// base), not the multi-parent-into-one shape `Merge` describes.
    fn on_promote(&self, label: &str, memory_promoted: bool, lineage: Option<PromotedLineage>) {
        let lineage_witnessed = lineage.is_some();
        if let Some(lineage) = lineage {
            self.chain.record_lineage(
                lineage.child_id,
                lineage.parent_id,
                lineage.parent_hash,
                rvf_types::DerivationType::Clone,
                lineage.mutation_count,
                label,
            );
        }
        // Always append the plain event alongside — greppable by kind
        // without decoding a lineage record, and it carries
        // `memory_promoted` (whether the promote itself landed), which
        // `record_lineage`'s payload does not.
        self.chain.append(
            "turn_ledger",
            "cow.turn.promoted",
            Some(serde_json::json!({
                "label": label,
                "memory_promoted": memory_promoted,
                "lineage_witnessed": lineage_witnessed,
            })),
        );
        if memory_promoted {
            info!(label, lineage_witnessed, "cow.turn.promoted witnessed");
        } else {
            warn!(
                label,
                lineage_witnessed,
                "cow.turn.promoted witnessed, but the memory promote itself failed — \
                 the checkpoint chain was left in place for the next turn"
            );
        }
    }
}

/// Truncate `s` to at most `max` chars, appending an ellipsis when cut.
/// Char-based (not byte-based) so a truncation never lands mid-codepoint.
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut truncated: String = s.chars().take(max).collect();
        truncated.push('…');
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_chars_short_string_unchanged() {
        assert_eq!(truncate_chars("hello", 10), "hello");
    }

    #[test]
    fn truncate_chars_long_string_capped_with_ellipsis() {
        let long = "a".repeat(600);
        let truncated = truncate_chars(&long, TURN_ERROR_CHAR_CAP);
        assert_eq!(truncated.chars().count(), TURN_ERROR_CHAR_CAP + 1);
        assert!(truncated.ends_with('…'));
    }
}
