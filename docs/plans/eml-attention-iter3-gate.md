# EML attention Iteration 3 gate — tracked decision (WEFT-41)

**Date**: 2026-07-31  
**Ticket**: WEFT-41  
**Status**: **Not run / deferred** — tracked decision, **not** an executed experiment  
**Sources**:
- [`.planning/development_notes/eml_model_development_assessment.md`](../../.planning/development_notes/eml_model_development_assessment.md) (lines 93–100, Iter-3 gate)
- [`.planning/development_notes/eml_model_development.md`](../../.planning/development_notes/eml_model_development.md)
- Audit: `.planning/reviews/0.7.0-release-gate/03-pipeline-routing.md` Task 03-20 / Q13
- Code: `crates/eml-core` feature `experimental-attention`, example `attention_gate`

## Decision

**Iteration 3 multi-param coordinated perturbation was not executed** for this close.

WEFT-41 is closed as a **tracked decision + re-run recipe**, not as a green (or red) gate run. No fabricated MSE numbers. Iterations 4–5+ remain aspirational per the assessment note.

### Why not run now

1. **Gate target is research-only** (cycle 1.0.x). Not on the 0.8 publish path.
2. **Existing gate binary is still Iter-2 shaped.** `crates/eml-core/examples/attention_gate.rs` documents Iteration-2 criteria (e2e CD ≥5% MSE reduction, p99 latency, etc.), not the Iter-3 multi-param / ≥80% reduction / `final_mse < 5e-2` targets.
3. Running a multi-minute training gate without a committed multi-param optimizer would only re-measure Iter-2 single-param CD and invite false “PASS/FAIL” against the wrong bar.
4. Substrate pieces exist (`ToyEmlAttention`, `SafeTree`, self-distill train path) but **coordinated multi-param perturbation is not implemented as a first-class training mode** for the attention gate.

## Gate definition (from assessment — unchanged)

> Iteration 3 gate: implement multi-param coordinated perturbation (pattern search or small-batch gradient-free updates) on top of the SafeTree architecture.  
> **Target**: MSE reduction ≥ **80%** on per-position-mean at browser shape `(seq_len=4, d_model=8)` and **`final_mse < 5e-2`**.

If Iteration 3 fails: record substrate limit; evaluate hybrid float + EML attention as Iteration 4.

## Prior iterations (historical; already shipped in experimental feature)

| Iter | Approx version | Outcome (from assessment) |
|------|----------------|---------------------------|
| 0 | 0.6.8 | Hybrid toy attention; G2–G5 pass; G1 identity fail expected |
| 1 | 0.6.9 | Joint CD ≥5% on mean task **PASS** (~57.8% at that shape) |
| 2 | 0.6.10 | SafeTree; G1 relaxed 5% **PASS** (~7.3%); identity + tight MSE deferred to Iter 3 |
| **3** | — | **Not run** (this ticket) |

## Failure-mode placeholder (to fill when run)

Use this table when Iter-3 is actually executed. Leave cells empty until then.

| Field | Value |
|-------|-------|
| Run date | _TBD_ |
| Commit SHA | _TBD_ |
| Command | see re-run recipe below |
| Shape | `seq_len=4`, `d_model=8` (gate); also record d_k / depth |
| Optimizer | multi-param coordinated perturbation / pattern search / other: _TBD_ |
| baseline_mse | _TBD_ |
| final_mse | _TBD_ |
| mse_reduction | _TBD_ (need ≥ 0.80) |
| wall_clock | _TBD_ |
| Gate result | PASS / FAIL / BLOCKED |
| Observed failure mode (if not PASS) | _e.g. single-param plateau, ln(ε) saturation, coord-descent instability, OOM, feature disabled_ |
| Follow-up | _Iter-4 hybrid eval / re-scope gate / abandon_ |

### Anticipated failure modes (hypothesis only — not measured)

1. **Plateau**: single-param CD already plateaued at browser-scale param counts (reason Iter-2 deferred the tight MSE target); multi-param may still stall if tree expressivity is insufficient.
2. **Saturation**: historical identity-task issues with `ln(ε)` under Rust `EmlTree` composition may reappear on hard targets.
3. **Joint submodel coupling**: five attention submodels with hybrid float matmuls may not train end-to-end under gradient-free multi-param steps without a custom schedule.
4. **Latency / cost**: large trial budgets (Iter-2 used 15k trials in the example) may make CI integration impractical — gate remains optional offline.

## How to re-run when ready

### Prerequisites

1. `eml-core` builds with feature `experimental-attention`.
2. Multi-param coordinated perturbation implemented and hooked into `ToyEmlAttention::train` (or a dedicated train entrypoint).
3. Gate example (or new `attention_gate_iter3`) enforces Iter-3 thresholds, not Iter-2’s 5% bar.
4. Prefer release mode for wall-clock honesty.

### Baseline (current tree — Iter-2 gate only)

```bash
# From repo root. Does NOT satisfy WEFT-41 Iter-3 AC; only proves feature compiles/runs.
cargo run -p eml-core --example attention_gate --features experimental-attention --release
```

Expected today: Iter-2 style G1–G5 printout. Exit non-zero on any G fail.

### Intended Iter-3 command (after optimizer lands)

```bash
# Placeholder name — implement when multi-param path exists:
cargo run -p eml-core --example attention_gate --features experimental-attention --release -- --iter 3 \
  --seq-len 4 --d-model 8 --target-reduction 0.80 --max-final-mse 0.05
```

Until that CLI exists, document the exact binary/flags in the failure-mode table when first run.

### What to capture

- Full stdout of the gate
- `baseline_mse`, `final_mse`, `mse_reduction`, training rounds
- p99 inference if still reported
- Host (CPU) and whether `SafeTree` path was active
- Commit SHA in the Plane close or a follow-up comment

### Optional CI

CI integration remains **optional** (ticket AC). Prefer `#[ignore]` heavy benches over hard-gating the main `scripts/build.sh gate` until Iter-3 is green offline.

## Acceptance criteria mapping

| AC | Outcome |
|----|---------|
| Run G3 / Iter-3 benchmark; record MSE / wall-clock | **Deferred** — not run; placeholder table above |
| Pass gate (≥80% reduction) **or** document failure mode + adjust gate | **Documented as not run** + failure-mode placeholder + re-run recipe |
| CI optional | Affirmed optional |

## Related

- [eml-heuristics-tracking.md](./eml-heuristics-tracking.md) (WEFT-57)
- [hnsw-eml-opportunities.md](./hnsw-eml-opportunities.md) (WEFT-58)
- Feature flag: `experimental-attention` in `crates/eml-core/Cargo.toml`
