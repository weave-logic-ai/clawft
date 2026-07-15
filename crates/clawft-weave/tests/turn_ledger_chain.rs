//! Integration test for [`clawft_weave::turn_ledger::DaemonTurnLedger`]
//! (WEFT-616 Phase 3 — the chain half of the DualStateBridge,
//! `.planning/ruv/integration/agenticow-integration-plan.md` §7/§8).
//!
//! Drives the ledger directly against a real `ChainManager` (the same unit
//! the daemon boot wires up), the way `subagent_spawn_commit.rs` drives
//! kernel+chain fixtures without a full daemon socket. `clawft-core`'s own
//! `turn_checkpoint.rs` tests already cover the bracket→ledger call
//! sequence with a recording stub; this test is the chain-witnessing half:
//! real `ChainManager` events, real `record_lineage`, and the Phase 3 exit
//! criterion — `verify_lineage()` passing.

use std::sync::Arc;

use clawft_core::agent::turn_ledger::{PromotedLineage, TurnLedger};
use clawft_core::clawft_cow_memory::{BranchableMemory, NodeRole, VectorItem};
use clawft_kernel::chain::ChainManager;
use clawft_weave::turn_ledger::DaemonTurnLedger;

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("clawft_turn_ledger_{prefix}_{pid}_{nanos}"))
}

#[test]
fn checkpoint_revert_promote_witnessed_and_lineage_verifies() {
    let dir = temp_dir("mem");
    let mut mem = BranchableMemory::create(&dir, 4).expect("create BranchableMemory");

    let chain = Arc::new(ChainManager::new(0, 1000));
    let ledger = DaemonTurnLedger::new(chain.clone());
    let chain_before = chain.len();

    // ── checkpoint ──────────────────────────────────────────────────────
    ledger.on_checkpoint("turn:t1");

    let events = chain.tail(0);
    let checkpoint_event = events
        .iter()
        .find(|e| e.kind == "cow.turn.checkpoint")
        .expect("checkpoint event witnessed on the chain");
    assert_eq!(checkpoint_event.source, "turn_ledger");
    assert_eq!(
        checkpoint_event
            .payload
            .as_ref()
            .and_then(|p| p.get("label"))
            .and_then(|v| v.as_str()),
        Some("turn:t1"),
        "checkpoint event carries the turn label"
    );

    // ── revert ──────────────────────────────────────────────────────────
    // Also exercises the 500-char truncation cap on `turn_error`.
    let long_error = "boom ".repeat(200); // 1000 chars, well over the cap
    ledger.on_revert("turn:t1", &long_error, true);

    let events = chain.tail(0);
    let revert_event = events
        .iter()
        .find(|e| e.kind == "cow.turn.reverted")
        .expect("revert event witnessed on the chain");
    let payload = revert_event.payload.as_ref().expect("revert payload present");
    assert_eq!(payload.get("label").and_then(|v| v.as_str()), Some("turn:t1"));
    assert_eq!(
        payload.get("memory_rolled_back").and_then(|v| v.as_bool()),
        Some(true)
    );
    let stored_error = payload
        .get("turn_error")
        .and_then(|v| v.as_str())
        .expect("turn_error present");
    assert!(
        stored_error.chars().count() <= 501,
        "turn_error is capped at 500 chars plus the ellipsis marker, got {}",
        stored_error.chars().count()
    );
    assert!(
        stored_error.ends_with('…'),
        "a truncated error ends with the ellipsis marker"
    );

    // ── promote, with real lineage read off a real BranchableMemory ──────
    mem.ingest(&[VectorItem::new(vec![1.0, 0.0, 0.0, 0.0])])
        .expect("ingest a turn's write into `working`");
    mem.checkpoint("turn:t2").expect("checkpoint before promote");

    let lineage = mem.lineage();
    let base_entry = lineage
        .iter()
        .find(|e| e.role == NodeRole::Base)
        .expect("base entry present in the lineage walk");
    let checkpoint_entry = lineage
        .iter()
        .find(|e| e.role == NodeRole::Checkpoint)
        .expect("the just-taken checkpoint entry present");

    // `ChainManager::verify_lineage` requires a non-root parent to already
    // be a known child on the chain (chain.rs::verify_lineage walks
    // `lineage.derivation` events and rejects an unrecognized parent_id).
    // `base` here genuinely IS the lineage root — `BranchableMemory::create`
    // makes it via `MemoryNode::create_root`, with no earlier ancestor of
    // its own — so seed the chain with that fact directly. This is a test
    // fixture (simulating a "base genesis" witness that isn't itself part
    // of the turn bracket — `TurnLedger` has no "base created" transition),
    // not a workaround: it's the honest way to make a *first* promote in a
    // fresh lineage verifiable, since there is nothing prior on the chain
    // to link to otherwise.
    let placeholder_hash = [0xABu8; 32];
    chain.record_lineage(
        base_entry.file_id,
        [0u8; 16],
        placeholder_hash,
        rvf_types::DerivationType::Snapshot,
        0,
        "test-fixture: base genesis",
    );

    ledger.on_promote(
        "turn:t2",
        true,
        Some(PromotedLineage {
            child_id: checkpoint_entry.file_id,
            parent_id: base_entry.file_id,
            parent_hash: placeholder_hash,
            mutation_count: checkpoint_entry.mutation_count as u32,
        }),
    );

    let events = chain.tail(0);
    let promote_event = events
        .iter()
        .find(|e| e.kind == "cow.turn.promoted")
        .expect("promote event witnessed on the chain");
    let payload = promote_event.payload.as_ref().expect("promote payload present");
    assert_eq!(payload.get("label").and_then(|v| v.as_str()), Some("turn:t2"));
    assert_eq!(
        payload.get("memory_promoted").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        payload.get("lineage_witnessed").and_then(|v| v.as_bool()),
        Some(true),
        "a Some(lineage) promote flags lineage_witnessed=true in the plain event too"
    );

    let lineage_derivations = events
        .iter()
        .filter(|e| e.kind == "lineage.derivation")
        .count();
    assert_eq!(
        lineage_derivations, 2,
        "the base-genesis fixture record plus the turn's own record_lineage witness"
    );

    // ── Phase 3 exit criterion (plan §8): verify_lineage() passes ────────
    let verified = chain.verify_lineage().expect("lineage chain verifies");
    assert_eq!(verified, 2);

    assert!(
        chain.len() > chain_before,
        "the chain advanced across the whole checkpoint/revert/promote bracket"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `on_promote` with `lineage: None` must NOT call `record_lineage` — it
/// downgrades honestly to a plain `cow.turn.promoted` event with
/// `lineage_witnessed: false`. This is the shape the real production call
/// site exercises today (`turn_checkpoint.rs` always passes `None` until
/// `BranchableMemory::promote` exposes file ids — see that file's comment).
#[test]
fn promote_without_lineage_downgrades_to_plain_event() {
    let chain = Arc::new(ChainManager::new(0, 1000));
    let ledger = DaemonTurnLedger::new(chain.clone());

    ledger.on_promote("turn:no-lineage", true, None);

    let events = chain.tail(0);
    assert!(
        events.iter().all(|e| e.kind != "lineage.derivation"),
        "no lineage record without a supplied PromotedLineage"
    );
    let promote_event = events
        .iter()
        .find(|e| e.kind == "cow.turn.promoted")
        .expect("plain promote event still witnessed");
    let payload = promote_event.payload.as_ref().expect("promote payload present");
    assert_eq!(
        payload.get("lineage_witnessed").and_then(|v| v.as_bool()),
        Some(false)
    );

    // A `verify_lineage()` over zero lineage records is trivially `Ok(0)`.
    assert_eq!(chain.verify_lineage().expect("verifies"), 0);
}
