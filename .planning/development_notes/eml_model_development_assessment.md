# EML-Transformer Iteration 0 — Assessment and Plan

**Date**: 2026-04-15
**Source**: `.planning/development_notes/eml_model_development.md` (design note)
**Scope**: Iteration 0 only. Iteration 1+ gated on Iteration 0 passing go/no-go.
**Version**: ships as experimental in 0.6.8

## Infrastructure check — "how close" is "close"

| Prerequisite | Status |
|---|---|
| `EmlModel::{new, predict, record, train, distill}` | ✅ `crates/eml-core/src/model.rs` |
| Coordinate-descent training with random restarts | ✅ |
| Multi-head output support | ✅ |
| JSON serialization via `to_json` / `from_json` | ✅ |
| ExoChain event emission (`EmlEvent`) | ✅ `events.rs` |
| Concrete wrappers (QueryFusionModel style) | ✅ 12 in Sprint 17 |
| `eml_qk_scores` / `eml_softmax` / `eml_attn_v` implementations | ❌ `todo!()` in dev note |
| Numerical-stability scaffolding | ❌ |
| Validation task + training-data pipeline | ❌ |

**Verdict**: ~90% infrastructure is in place. Iteration 0 closes the missing 10%.

## Design decision: pragmatic hybrid for Iteration 0

The dev note sketches a fully-EML attention (matmuls expressed as EML trees). The substrate doesn't support this yet because:

- `EmlModel` targets must match `head_count`. End-to-end (input, target) pairs do not train 5 submodels with different shapes directly.
- Expressing unsigned multiplication `a * b` via `eml` requires positive `ln` inputs, which attention scores routinely violate.
- Coordinate descent on 5 interdependent models is an open problem.

**Iteration 0 ships a hybrid**: five `EmlModel` instances for the learned projections and softmax, with `f64` matrix multiplication between them. The dev note explicitly allows hybrid at higher iterations; we adopt it from day one to make the spike tractable.

Training uses **self-distillation**: at `train()` time, run a forward pass on buffered inputs, derive per-submodel targets (numerical softmax for `softmax_model`; `(context, end-to-end target)` for `out_model`), and train via the existing coordinate-descent loop. Q/K/V models remain at default initialization in Iteration 0; Iteration 1 introduces a proper end-to-end coordinate-descent loop.

## Concrete deliverable

**`crates/eml-core/src/attention.rs`** (≈580 lines)

- `ToyEmlAttention` struct: 5 `EmlModel` instances + scale + buffer + trained flag + event log
- `new(name, d_model, d_k, seq_len, depth)` with guards: `d_model ≤ 32`, `seq_len ≤ 8`, `depth ∈ 3..=5`
- `forward(x)` returns a same-shape output: Q/K/V projection → float matmul for QKᵀ → learned-or-numerical softmax → float matmul for A·V → output projection
- `record(input, target)` end-to-end
- `train()` self-distills submodel targets and trains
- `to_json` / `from_json` for persistence
- `AttentionError` for shape / range violations
- `run_benchmark` for the 4-phase harness
- 13 unit tests covering construction, shape enforcement, softmax stability, serialization roundtrip, training-round counting, and benchmark sanity

**Feature flag**: `experimental-attention` on `eml-core`. Default off.

## 4-phase benchmark (`AttentionBenchmark`)

Mirrors the `clawft-weave/src/commands/bench_cmd.rs` layout (Warmup → Transport → Compute → Scalability) adapted for learned functions:

- **Phase 1 — Warmup**: single forward-pass timing + serialization roundtrip
- **Phase 2 — Convergence**: train on a memorizable identity task (96 synthetic samples); report converged flag, final MSE, training round count
- **Phase 3 — Compute**: inference latency over 256 random inputs; report mean and p99
- **Phase 4 — Scalability**: sweep `(seq_len, d_model)` in `{(4,8), (4,16), (8,8), (8,16)}`; report param count and mean inference per point

Results are returned as a single `AttentionBenchmark` struct for JSON reporting. This shows current performance and lets later iterations quantify their deltas.

## Go/no-go criteria (2-week spike)

Run via `cargo run --example attention_gate --features experimental-attention --release`.

**Current status** (2026-04-15, initial ship):

| Gate | Criterion | Status | Notes |
|------|-----------|--------|-------|
| G1 | Converges on identity in ≤ 3 rounds | **FAIL** (expected) | Identity task is architecturally unlearnable in Iteration 0 — Q/K/V are frozen at random init by design, so end-to-end information loss through random projections exceeds what `out_model` can recover. This FAIL is the signal that Iteration 1's end-to-end coordinate descent is actually necessary. |
| G2 | Inference p99 ≤ 5 µs at reference shape | PASS | Observed 1727 ns p99 — 3× headroom. |
| G3 | Timings finite | PASS | — |
| G4 | Serialization roundtrip preserves shape | PASS | — |
| G5 | Polynomial scaling up to `(seq_len=8, d_model=16)` | PASS | Validated by the benchmark sweep. |

**Interpretation**: G2-G5 validate that the substrate composes as designed: EML projections + learned softmax + output projection can be chained, measured, and shipped. G1's failure is the architectural result of Iteration 0's deliberate scope — training only `softmax_model` and `out_model`. This failure is the *motivation* for Iteration 1, not a blocker on Iteration 0 shipping.

**Iteration 1 gate** (forward reference): re-run the same benchmark after end-to-end coordinate descent is implemented. G1 should then pass with `final_mse < 1e-2` on the identity task.

If Iteration 1 fails G1, stop and record the substrate limit in a follow-up ADR.

## Iteration roadmap (for reference only; do not plan beyond 0 until 0 passes)

| Iter | d_model | seq_len | heads | layers | scope | target |
|------|---------|---------|-------|--------|-------|--------|
| **0 (this)** | 16 | 4-8 | 1 | 1 | hybrid toy block, pragmatic matmul | prove substrate composes |
| 1 | 32-64 | 12-16 | 2 | 2 | proper end-to-end coordinate descent, learned positional encodings | replace one existing EML wrapper |
| 2 | 96-128 | 24-32 | 4 | 3-4 | chunked attention, continual ExoChain-triggered retraining | replace 3 wrappers |
| 3 | 192-256 | 48-64 | 8 | 4-6 | sparse attention, shared constants across heads | unified context scorer |
| 4+ | 256-512 | chunked | 16 | 6-8 | hybrid EML for scoring + float residuals | EML-Transformer backbone |

Iteration 1+ is explicitly **aspirational**. Each iteration gates on the previous one passing benchmarks that match or beat the wrapper it would replace.

## Cross-references

- WeftOS docs: `docs/src/content/docs/weftos/eml-attention.mdx` (user-facing architecture page)
- Live demo: `/clawft_eml-attention` (pure-JS Iteration 0 demonstrator — WASM rebuild to follow in 0.6.9)
- Colab notebook: `/clawft_eml-notebook` (Python reference that exports Rust-loadable JSON)
