# eml-core — plan for addressing the ⊃μBus tube-fit findings

**Companion to**: `eml-core-training-findings.md` (⊃μBus memo, 2026-04-22)
**Status**: deferred — parked while GUI work continues
**Target crate**: `/home/aepod/dev/tidbyt-rs/weftos/crates/eml-core/` (sibling repo)

## TL;DR

Three fixes proposed upstream. Keep the memo's ordering (#1 → #3 → #2) but **expand #1's content** based on a numerical review: the dominant optimiser-killer is the head `.max(0.0)` ReLU and the `0.01` hard floor inside every `ln`-arg clamp, not the saturating `±10` clamps. Naively making the bounds configurable wouldn't unblock the tube-fit use case — soft activations do.

## Reviewed, converged findings

- All line-number claims in the memo verified against source (`model.rs:265-442`, `model.rs:492-554`, `model.rs:322-326`, `operator.rs:22-30`, `tree.rs:130-192`).
- ADR-0020 Phase A (Koren-analytical at runtime, EML slot reserved) stays. 12AX7 is a decisive win for Koren (33 ns vs 418 ns, SNR ∞ vs 27 dB).
- Realistic EML payoff is **6L6 / EL34 power pentodes** — secondary-emission hump is where Koren fits worst.
- Opportunity cost of doing nothing is low. No near-term song, phase, or release depends on it.

## What to ship, in order

### Phase 1 — Soft activations + configurable clamps (expanded Fix #1)

**Why it's expanded**: the memo's Fix #1 (configurable clamp bounds) is necessary but not sufficient. The real zero-gradient fence is the head `.max(0.0)` at `model.rs:438` — STE-ing through it *creates* the predict-zero attractor rather than escaping it. The `0.01` hard floor in every `ln`-arg clamp is a second dead-zone. Both need softening, not just widening.

**Changes**:

- New `EmlConfig`:
  ```rust
  #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
  pub struct EmlConfig {
      pub affine_clamp: (f64, f64),     // default (-10.0, 10.0)
      pub ln_arg_clamp: (f64, f64),     // default (0.01, 10.0)
      pub output_clamp: (f64, f64),     // default (-10.0, 10.0)
      pub head_activation: HeadActivation, // default Relu (bit-exact)
  }

  pub enum HeadActivation { Relu, Softplus, Identity }
  ```
  `Default` impl = today's behaviour byte-for-byte.

- `EmlModel` carries the config as a runtime field. Add `#[serde(default)]` on it so existing on-disk snapshots deserialise into the bit-exact default.

- Replace head `.max(0.0)` at `model.rs:438` with `head_activation.apply(...)`.

- Replace the `0.01` hard floor inside `ln`-arg clamps (`model.rs:302, 327, 345, 370, 393, 412`) with a soft-`ln` path that preserves gradient: `ln(max(y, ε))` with a smooth transition rather than a hard floor where the gradient dies.

- **Critical**: propagate `EmlConfig` into `EmlTree` — `tree.rs:130, 165, 170-192` hold a parallel set of clamps that would silently diverge from the `EmlModel` behaviour otherwise.

- New public API:
  ```rust
  impl EmlModel {
      pub fn with_config(depth: usize, input_count: usize, head_count: usize, cfg: EmlConfig) -> Self;
  }
  ```
  `EmlModel::new` delegates with `EmlConfig::default()`. Validate `lo < hi` in `with_config` and panic (matches `new`'s style).

- Estimate: ~1 day. One PR. Bit-exact on default path.

### Phase 2 — `train_with_config()` + Adam escape (Fix #3)

**Do not change `train()`'s signature** — every existing caller including `ToyEmlAttention` depends on it.

**Changes**:

- Add:
  ```rust
  pub enum TrainConfig {
      CoordDescent, // today's behaviour
      Adam { steps: usize, lr: f64 },
  }

  impl EmlModel {
      pub fn train_with_config(&mut self, samples: &[(Vec<f64>, Vec<f64>)], cfg: TrainConfig) -> bool;
  }
  ```
  `train()` delegates to `train_with_config(TrainConfig::CoordDescent)`.

- Gate the Adam path behind a default-off `gradient-training` feature. Finite-difference over `params_slice_mut()` is sufficient for v1 — reference implementation already exists in ⊃μBus (`tube_warm.rs::finite_diff_adam`).

- This only pays off *after* Phase 1 lands. Adam on a dead-ReLU landscape doesn't escape any better than coord-descent does.

- Estimate: ~1 day.

### Phase 3 — `predict_with_grad()` (Fix #2, deferred)

Ship last, only if we commit to capture-based EML fits as a production workflow.

**Changes**:

- Shape: `(Vec<f64>, Vec<Vec<f64>>)` = `(outputs, [head_count][param_count])`. Not flat — the memo's proposed single `Vec<f64>` breaks multi-head.

- Gotcha: `softmax3` at `model.rs:322-326` shares parameters between left and right mixes (`params[base] + 0.5`, `params[base+1] - 0.5`). Derivatives must account for the coupling or sensitivity is underestimated ~2×.

- Clamp derivatives: straight-through where inside bounds, zero outside — but only meaningful *after* Phase 1 swaps the hard dead-ReLU for softplus/soft-ln, otherwise STE creates the attractor it's supposed to escape.

- Ship `#[doc(hidden)]` behind a `gradients` feature until semantics settle.

- Estimate: ~3 days.

## ⊃μBus-side counterpart (only if Phase 1 lands)

- Add `TubeModel::EmlCapture` variant in `vector-synth-core/src/patch_graph/step/_primitives/tube_model.rs`.
- Capture-ingestion path in `vector-synth-trainer` (measured-curve → EML-fit pipeline).
- Keep the `tube-eml` feature gate; no ADR-0001 tension.
- Estimate: ~1 week.
- **First target**: 6L6 or EL34. Not 12AX7 (Koren already wins).

## Open questions (before any PR)

- **`EmlConfig` persistence story**: `#[serde(default)]` handles loading old snapshots as default. Do we need a version field on saved models to distinguish "saved under defaults" vs "saved under tuned config" for reproducibility? Check if any on-disk users exist yet — if not, defer.
- **`HeadActivation::Softplus` numerical tuning**: softplus has its own saturation (`softplus(x) ≈ x` for large `x`, `≈ 0` for `x ≪ 0`). Verify gradient stays alive in the Koren regime (`x` spans roughly `[-5, 8]` in normalised units) before declaring it solved.
- **Feature-flag posture**: both `gradient-training` and `gradients` default-off — confirm that matches how `eml-core`'s downstream consumers (`ToyEmlAttention`, etc.) want the crate surfaced. Don't bloat MSRV or WASM footprint for a rarely-used path.
- **Test fixtures**: add the Koren plate-current fit as a canonical regression test inside `eml-core/examples/` — gives us a concrete "did we unblock the thing we said we unblocked" artefact, and keeps the memo's repro reachable from the crate.

## Relevant files

- `.planning/eml/eml-core-training-findings.md` (the memo this plan responds to)
- `/home/aepod/dev/tidbyt-rs/weftos/crates/eml-core/src/model.rs`
- `/home/aepod/dev/tidbyt-rs/weftos/crates/eml-core/src/tree.rs`
- `/home/aepod/dev/tidbyt-rs/weftos/crates/eml-core/src/operator.rs`
- `/home/aepod/dev/tidbyt-rs/weftos/crates/eml-core/src/lib.rs`
- `/home/aepod/dev/vector-synth/crates/vector-synth-trainer/src/tube_warm.rs` (reference `finite_diff_adam` impl)
