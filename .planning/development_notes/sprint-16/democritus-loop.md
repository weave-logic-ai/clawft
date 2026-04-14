# DEMOCRITUS Two-Tier Coherence Loop

**Date**: 2026-04-04
**Sprint**: 16
**Status**: Implemented, not committed

## What Changed

Wired the actual DEMOCRITUS tick loop with two-tier EML coherence into the
WeftOS kernel. Previously, `CognitiveTick` was a passive stats tracker.
Now it drives a real background loop that continuously monitors graph
coherence.

### Files Modified

1. **`crates/clawft-kernel/src/cognitive_tick.rs`**
   - Added `run_democritus_loop()` async function (gated behind `#[cfg(feature = "ecc")]`)
   - Implements SENSE -> THINK(fast) -> DETECT DRIFT -> THINK(exact) -> COMMIT cycle
   - Fast path: O(1) EML prediction via `EmlCoherenceModel::predict()`
   - Exact path: O(k*m) Lanczos via `CausalGraph::spectral_analysis(50)` -- only when drift > 5% or every 100 ticks
   - EML model retraining every 1000 exact samples once 50+ samples collected
   - Structured logging at key events (first coherence, drift detected, model retrained)
   - Mutex held briefly (only for predict/record), never during spectral analysis

2. **`crates/clawft-kernel/src/boot.rs`**
   - Changed `EccSubsystem::eml_coherence` from `Option<EmlCoherenceModel>` to `Option<Arc<Mutex<EmlCoherenceModel>>>` for thread-safe sharing
   - After `tick.start().await`, spawns background `tokio::spawn` for the loop
   - `eml_model` Arc created in the ECC init block, shared between loop and EccSubsystem
   - Updated `ecc_eml_coherence()` accessor return type accordingly
   - Added boot log entry: "DEMOCRITUS two-tier coherence loop spawned"

## Design Decisions

- **Mutex vs RwLock**: Used `std::sync::Mutex` because writes (record/train) happen
  on the exact path which is the only frequent accessor. The fast-path predict
  is very quick (~0.1us) so contention is minimal.
- **Poison handling**: On lock poison, the loop logs an error and breaks out.
  This is acceptable because a poisoned lock means a panic occurred somewhere,
  and continuing would be unsafe.
- **No new file**: The loop function lives in `cognitive_tick.rs` alongside the
  `CognitiveTick` service it operates on, rather than in `democritus.rs` which
  handles the SENSE-EMBED-SEARCH-UPDATE-COMMIT impulse pipeline.

## Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| `drift_threshold` | 0.05 | 5% delta triggers exact recomputation |
| `exact_every_n` | 100 | Force exact every 100 ticks as calibration baseline |
| `train_every_n` | 1000 | Retrain EML model every 1000 exact samples |
| `spectral_k` | 50 | Lanczos iterations (matches existing usage) |

## Verification

- `scripts/build.sh check` -- passes clean
- `scripts/build.sh test` -- all tests pass (3,400+ tests)
- No new dependencies added
- Existing `CognitiveTick` tests unaffected (they test the stats tracker, not the loop)
