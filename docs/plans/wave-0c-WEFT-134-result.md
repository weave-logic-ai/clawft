# WEFT-134 result — clawft-kernel `--lib` aggregate hang

**Branch:** `wave0c/weft-134-kernel-test-hang`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb446-80b9-7ea1-b9bd-3531b4acbd59`  
**Base:** `release/0.8-staging`

## Root cause

Not a deadlock, missing `select!` timeout, or stuck channel. The aggregate suite appeared to hang because three unit tests in `hnsw_eml` called `run_hnsw_benchmark` with full-scale protocol parameters:

| Test | Params | Effect |
|------|--------|--------|
| `benchmark_4_phase_runs` | 500 vecs × 32 dims | Phase 2: 20×128 queries, `train_every_n=100` |
| `benchmark_produces_json` | 200 × 16 | Same learning loop shape |
| `benchmark_full_report` | 5000 × 128 | Plus phase-4 sweep to 5k |

Each mid-stream `train_all()` bottoms out in `eml_core::EmlModel::train`, which does 100–200 random restarts plus up to 1000 coordinate-descent iterations of full-set MSE evaluation. With ~25 train cycles in phase 2, wall time is multi-minute (CPU-bound, not parked). Targeted runs of other modules passed; the hang only showed when the full `--lib` suite reached `hnsw_eml::tests::benchmark_4_phase_runs`.

Reproduced with `--test-threads=1 --nocapture`: suite stuck on that test; `sample` showed 88% CPU in `EmlModel::train` / `evaluate_mse` / `softmax3`.

## Fix

Files:

- `crates/clawft-kernel/src/hnsw_eml.rs`
- `crates/clawft-kernel/src/lib.rs` (re-exports)

Changes:

1. **`HnswBenchmarkParams`** — explicit scale knobs (`learning_passes`, `n_queries`, `train_every_n`, `phase4`, …).
2. **`run_hnsw_benchmark_with`** — parameterized runner; `run_hnsw_benchmark` keeps full-scale defaults for manual/perf use.
3. **`HnswBenchmarkParams::smoke()`** — 48×8, 1 pass × 8 queries, `train_every_n=10_000` (no mid-stream train storm).
4. Default unit tests use smoke; full-scale `benchmark_4_phase_runs` and `benchmark_full_report` are `#[ignore]` (run with `--ignored`).
5. `benchmark_produces_json` also uses smoke so CI still checks serde shape.

## Verification

```text
cargo test -p clawft-kernel --lib hnsw_eml::tests::benchmark
# 2 passed; 2 ignored; finished in 0.50s

cargo test -p clawft-kernel --lib
# 2040 passed; 0 failed; 2 ignored; finished in 36.90s

scripts/build.sh check
# ok (workspace cargo check)
```

## Acceptance

| Criterion | Status |
|-----------|--------|
| Hang reproduced in controlled harness | Yes (`--test-threads=1` + `sample`) |
| Root cause identified | Yes (EML train cost in full-scale bench tests) |
| Fix landed; full `--lib` deterministic | Yes (~37s, 2040 pass / 2 ignore) |
| CI gate reinstated | Full `--lib` green; ignored benches are intentional opt-in |

## How to re-run full benches (optional)

```bash
cargo test -p clawft-kernel --lib benchmark_4_phase_runs -- --ignored --nocapture
cargo test -p clawft-kernel --lib benchmark_full_report -- --ignored --nocapture
```
