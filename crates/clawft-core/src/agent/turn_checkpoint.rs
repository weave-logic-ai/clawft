//! Per-turn COW memory checkpoint/promote/rollback bracket (WEFT-616 Phase 2).
//!
//! [`with_turn_checkpoint`] wraps a single turn's execution with a
//! `clawft-cow-memory::BranchableMemory` checkpoint: freeze `working` into a
//! checkpoint before the turn runs, `promote()` it into `base` on success,
//! `rollback()` to the pre-turn checkpoint on failure. See
//! `.planning/ruv/integration/agenticow-integration-plan.md` §2/§4 and
//! `.planning/fork-adoption-build-plan-2026-07.md` §1 Phase 2.
//!
//! # What this brackets today
//!
//! [`AgentLoop::handle_turn`](crate::agent::loop_core::AgentLoop::handle_turn)
//! does not currently write vector memory mid-turn -- `memory_bootstrap` is a
//! one-time indexing step run outside the turn path (index `MEMORY.md` once
//! at startup), and [`MemoryStore`](crate::agent::memory::MemoryStore) is a
//! markdown-file store unrelated to `RvfStore`/`BranchableMemory`. So today
//! this bracket's `checkpoint`/`promote`/`rollback` calls replay empty
//! edit-logs -- the lifecycle is exercised and correct, but there is nothing
//! yet for a turn to actually ingest into the checkpointed memory. Routing a
//! turn's writes into `working` instead of directly into the brain store is
//! Phase 3/tools' job -- see `clawft-cow-memory`'s crate-level "Not yet in
//! scope" list.
//!
//! # Failure semantics
//!
//! - If `checkpoint()` itself fails, the turn is not attempted at all (fail
//!   closed -- isolation cannot be promised without it) and the checkpoint
//!   error is returned as the turn's result.
//! - If the turn errors, `rollback()` is attempted. A rollback failure is
//!   logged loudly but does **not** replace the turn's own error -- that is
//!   what the caller actually needs to see.
//! - If the turn succeeds, `promote()` is attempted. A promote failure is
//!   logged loudly but does **not** discard the turn's successful result --
//!   the user-visible reply is not memory-infrastructure's to withhold. The
//!   unpromoted checkpoint chain is left in place; the next turn's own
//!   `checkpoint()` call builds on top of it rather than losing it.

use std::future::Future;
use std::sync::{Arc, Mutex};

use clawft_cow_memory::BranchableMemory;
use clawft_types::error::ClawftError;
use tracing::error;

use super::turn_ledger::TurnLedger;

/// Run `turn` bracketed by a checkpoint/promote/rollback cycle on `mem`.
///
/// `turn` is called at most once, after the checkpoint succeeds. `label`
/// names the checkpoint (visible via [`BranchableMemory::lineage`] for
/// debugging/audit) -- callers should pass something that identifies the
/// turn (e.g. `"turn:{channel}:{chat_id}"`).
pub(crate) async fn with_turn_checkpoint<F, Fut, T>(
    mem: &Arc<Mutex<BranchableMemory>>,
    ledger: Option<&Arc<dyn TurnLedger>>,
    label: impl Into<String>,
    turn: F,
) -> clawft_types::Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = clawft_types::Result<T>>,
{
    let label: String = label.into();
    // `rollback(Some(id))` discards everything *newer* than `id` but keeps
    // `id` itself (it is a "roll back to this point", not "roll back past
    // it") -- see `ops.rs::rollback`'s doc comment. So targeting the
    // checkpoint we are about to create would keep it, which is exactly
    // the turn's own (to-be-discarded) work. What we actually want on
    // failure is "undo this turn's checkpoint too" -- i.e. roll back to
    // whatever checkpoint was the head *before* this call, which is `None`
    // (collapse to `base`) in the steady state, since `promote`/`rollback`
    // always collapse the chain back to depth 0 after each prior turn.
    let pre_turn_head = {
        let guard = mem.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard
            .lineage()
            .into_iter()
            .find(|entry| matches!(entry.role, clawft_cow_memory::NodeRole::Checkpoint))
            .map(|entry| entry.id)
    };

    {
        let mut guard = mem.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.checkpoint(label.clone()).map_err(cow_err)?;
    }
    // Witness AFTER the memory checkpoint succeeded — the chain must never
    // claim a checkpoint that doesn't exist (WEFT-616 Phase 3, §7).
    if let Some(ledger) = ledger {
        ledger.on_checkpoint(&label);
    }

    let result = turn().await;

    match &result {
        Ok(_) => {
            let promoted = {
                let mut guard = mem.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                match guard.promote() {
                    Ok(_report) => true,
                    Err(e) => {
                        error!(
                            error = %e,
                            "clawft-cow-memory: promote failed after successful turn; \
                             checkpoint chain left in place for the next turn to build on"
                        );
                        false
                    }
                }
            };
            if let Some(ledger) = ledger {
                // Lineage ids: supplied once BranchableMemory exposes file
                // ids (Phase 3 write-routing work); None downgrades the
                // witness to a plain promote event.
                ledger.on_promote(&label, promoted, None);
            }
        }
        Err(turn_err) => {
            let rolled_back = {
                let mut guard = mem.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                match guard.rollback(pre_turn_head) {
                    Ok(()) => true,
                    Err(e) => {
                        error!(
                            error = %e,
                            "clawft-cow-memory: rollback failed after turn error; \
                             original turn error still returned to the caller"
                        );
                        false
                    }
                }
            };
            if let Some(ledger) = ledger {
                // Append-only compensation: memory was discarded; the FACT of
                // the revert is witnessed forever (never truncate history).
                ledger.on_revert(&label, &turn_err.to_string(), rolled_back);
            }
        }
    }

    result
}

fn cow_err(e: clawft_cow_memory::CowMemoryError) -> ClawftError {
    ClawftError::CowMemory {
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Recording ledger: captures the bracket's witness sequence.
    #[derive(Default)]
    struct RecordingLedger {
        events: Mutex<Vec<String>>,
    }
    impl TurnLedger for RecordingLedger {
        fn on_checkpoint(&self, label: &str) {
            self.events.lock().unwrap().push(format!("checkpoint:{label}"));
        }
        fn on_revert(&self, label: &str, _err: &str, rolled_back: bool) {
            self.events
                .lock()
                .unwrap()
                .push(format!("revert:{label}:rolled_back={rolled_back}"));
        }
        fn on_promote(&self, label: &str, promoted: bool, _lineage: Option<super::super::turn_ledger::PromotedLineage>) {
            self.events
                .lock()
                .unwrap()
                .push(format!("promote:{label}:promoted={promoted}"));
        }
    }

    #[tokio::test]
    async fn ledger_witness_sequence_ok_and_err() {
        let dir = TempDir::new().unwrap();
        let mem = Arc::new(Mutex::new(BranchableMemory::create(dir.path(), 4).unwrap()));
        let concrete = Arc::new(RecordingLedger::default());
        let ledger: Arc<dyn TurnLedger> = concrete.clone();

        let ok: clawft_types::Result<u8> =
            with_turn_checkpoint(&mem, Some(&ledger), "t-ok", || async { Ok(1u8) }).await;
        assert!(ok.is_ok());
        let err: clawft_types::Result<u8> = with_turn_checkpoint(&mem, Some(&ledger), "t-err", || async {
            Err(ClawftError::CowMemory { reason: "boom".into() })
        })
        .await;
        assert!(err.is_err());

        let events = concrete.events.lock().unwrap().clone();
        assert_eq!(
            events,
            vec![
                "checkpoint:t-ok".to_string(),
                "promote:t-ok:promoted=true".to_string(),
                "checkpoint:t-err".to_string(),
                "revert:t-err:rolled_back=true".to_string(),
            ],
            "append-only witness order: checkpoint precedes its outcome, \
             revert witnesses the discard"
        );
    }

    fn test_mem(dir: &TempDir) -> Arc<Mutex<BranchableMemory>> {
        let mem = BranchableMemory::create(dir.path(), 4).expect("create BranchableMemory");
        Arc::new(Mutex::new(mem))
    }

    #[tokio::test]
    async fn successful_turn_promotes_writes() {
        let dir = TempDir::new().unwrap();
        let mem = test_mem(&dir);

        let result: clawft_types::Result<()> = with_turn_checkpoint(&mem, None, "t1", || async {
            let mut guard = mem.lock().unwrap();
            guard
                .ingest(&[clawft_cow_memory::VectorItem::new(vec![1.0, 0.0, 0.0, 0.0])])
                .map_err(|e| ClawftError::CowMemory {
                    reason: e.to_string(),
                })?;
            Ok(())
        })
        .await;

        assert!(result.is_ok());

        let guard = mem.lock().unwrap();
        // Promoted into base and visible via a fresh query.
        let hits = guard.query(&[1.0, 0.0, 0.0, 0.0], 5).expect("query");
        assert_eq!(hits.len(), 1, "promoted vector should be queryable");
    }

    #[tokio::test]
    async fn failed_turn_rolls_back_writes() {
        let dir = TempDir::new().unwrap();
        let mem = test_mem(&dir);

        let result: clawft_types::Result<()> = with_turn_checkpoint(&mem, None, "t1", || async {
            let mut guard = mem.lock().unwrap();
            guard
                .ingest(&[clawft_cow_memory::VectorItem::new(vec![1.0, 0.0, 0.0, 0.0])])
                .map_err(|e| ClawftError::CowMemory {
                    reason: e.to_string(),
                })?;
            drop(guard);
            Err(ClawftError::Timeout {
                operation: "simulated turn failure".into(),
            })
        })
        .await;

        assert!(result.is_err());

        let guard = mem.lock().unwrap();
        let hits = guard.query(&[1.0, 0.0, 0.0, 0.0], 5).expect("query");
        assert!(
            hits.is_empty(),
            "rolled-back turn's writes must not be queryable"
        );
    }

    #[tokio::test]
    async fn disabled_memory_untouched_by_a_turn_that_never_wraps_it() {
        // Sanity check for the "zero behavior change when disabled" claim:
        // a turn that simply never calls into cow_memory leaves it exactly
        // as `create` left it (empty, dimension unchanged).
        let dir = TempDir::new().unwrap();
        let mem = test_mem(&dir);
        let guard = mem.lock().unwrap();
        let status = guard.status();
        assert_eq!(status.dimension, 4);
        assert_eq!(status.own_checkpoint_depth, 0);
    }
}
