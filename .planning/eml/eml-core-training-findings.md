# eml-core — training findings from the ⊃μBus tube-modeling experiment

**From**: ⊃μBus project (vector-synth), 2026-04-22
**Context**: We tried to fit a 12AX7 triode's plate-current curve (Koren form) using `eml-core` as the function approximator. Infrastructure shipped; the fit didn't converge well. Sharing findings in case they're useful upstream.

## TL;DR
- What works: warm-start from analytical params via `params_slice_mut()`; two-phase Adam + finite-diff over the public API; bucket-balanced training targets.
- What doesn't: `EmlModel::train()` finds the right basin in only about 1-in-6 random seeds at its internal K=200; and even after gradient-based refinement the hard clamps inside `evaluate_with_params` fence off the function space needed to fit an exp/log composition with multi-decade dynamic range. We plateau at SNR ≈ 27 dB (training MSE ~6e-5) regardless of trainer effort.
- Suggested upstream changes (ranked by value:cost): (1) configurable clamps in `evaluate_with_params`, (2) analytic `predict_with_grad()` API, (3) basin-escape option for `train()` (Adam / L-BFGS over `params_slice_mut()`).

## What we tried to fit

Norman Koren's 12AX7 plate current (closed form, used as ground truth):

`Ip = (1/kg1) · (kp · ln(1 + exp(kp · (1/mu + v_gk / sqrt(kvb + v_pk²)))))^kx`

Expected: `eml(x, y) = exp(x) − ln(y)` plus a depth-4 tree should be able to represent this in principle (EML universality). Two inputs `(v_gk, v_pk)` normalized to `[0, 1]`, one output `Ip` normalized to `[0, 1]` against a 15 mA ceiling.

Achieved:
- Cold `EmlModel::train()` at depth 4: predicts ≈ 0 mA everywhere, hold-out SNR ≈ 0 dB.
- Depth 5 cold: predicts ≈ 70 mA everywhere (opposite saturation failure).
- Warm-started + two-phase Adam + long-tail polish at depth 4: hold-out SNR = 27.4 dB, training MSE 5.95e-5. Tracks the shape (0.72/0.52, 3.33/3.40, 10.91/10.92 mA at representative points) but with ~10-30% relative error in mid-drive.
- Koren analytical at runtime: SNR ∞, ≈ 33 ns per inference (vs EML-warm's ≈ 418 ns).

## What worked (no upstream change required)

- **Warm-start via `EmlModel::params_slice_mut()`** — the public API at `crates/eml-core/src/model.rs:119` was sufficient. No upstream change needed to inject a warm-start initial condition. See `crates/vector-synth-trainer/src/tube_warm.rs::random_best_of_k` for the call pattern.
- **Two-phase Adam with finite-difference gradients** — coarse lr=0.03 for 200 steps then fine lr=0.005 for 200 steps, polish at 1e-3 / 2e-4. Two-sided FD with h=1e-4; no autodiff dep. `tube_warm.rs::finite_diff_adam`.
- **Bucket-balanced training samples** — uniform random sampling over `(v_gk, v_pk)` concentrates in cutoff. Rejection-sampling so cutoff / active / driven buckets are each ~1/3 gives the trainer signal on the extreme regions. Without this, MSE collapses to "always output zero." `tube_warm.rs::build_training_set`.
- **Best-of-K random inits + multi-seed** — K=4096 `params_slice_mut()` candidates + 6 outer seeds. About 1-in-6 seeds escape the predict-zero attractor. Given how cheap `EmlModel::new()` is this is a fine user-land workaround; flagging as a trainer-level concern below.

## What didn't, and why

### Issue 1: Basin-hit rate in `EmlModel::train()` (trainer-level)

`EmlModel::train()` at `crates/eml-core/src/model.rs:492-554` is random-restart (100 or 200 inits depending on param count, `model.rs:503`) + coordinate descent with step set `[-0.1, -0.01, -0.001, 0.001, 0.01, 0.1]` (`model.rs:515`). Convergence flag is `MSE < 0.01` (`model.rs:536`). For the Koren plate curve — a monotone function with high dynamic range (0 to ~70 mA) — 5-of-6 random inits stay trapped in the predict-zero attractor. At depth 5 the attractor moves to predict-mean rather than predict-zero; deeper trees don't solve it.

The attractor exists because loss is nearly flat when most samples' predicted outputs hit the output clamps at `model.rs:429-431` and `model.rs:438` (non-negative clamp on heads). Coordinate descent's smallest step (`±0.001`) is too coarse to escape, and there's no gradient-based fallback.

**Suggested fix**: add an Adam or L-BFGS escape path to `train()`. Because `params_slice_mut()` is already public, an Adam loop using finite differences over `evaluate_with_params` / `predict_primary` would require no API changes and no autodiff dep — it's exactly what we built out-of-tree in `tube_warm.rs::finite_diff_adam`. Guardable behind a feature or a `TrainConfig::Adam { steps, lr }` variant if you want to keep the gradient-free default.

### Issue 2: Hard clamps in `evaluate_with_params` (architectural)

`EmlModel::evaluate_with_params` (`crates/eml-core/src/model.rs:265-442`) hard-clamps intermediate values at several layers:

- Level-0 affine outputs: `.clamp(-10.0, 10.0)` at `model.rs:273`.
- Level-2 right-side mix (arg to `ln`): `.clamp(0.01, 10.0)` at `model.rs:302`, `327`, `345`, `370`, `393`, `412`.
- Level-3 / level-4 left mix: `.clamp(-10.0, 10.0)` at `model.rs:341`, `387`, `408`.
- Head outputs: `.clamp(-10.0, 10.0)` / `.clamp(0.01, 10.0)` at `model.rs:429-430`, `438`, plus a `.max(0.0)` at `model.rs:438`.

These also appear in `EmlTree::evaluate` (`crates/eml-core/src/tree.rs:130`, `165`, `170-171`, `186-187`, `191-192`) and more loosely in `eml_safe` itself (clamps `x` to `[-20, 20]`, `operator.rs:22-30`).

These are reasonable numerical-stability guards, but they fence off exactly the region `ln(1 + exp(k·x))^kx` compositions need when `k·x` grows. We measured gradients going to zero at these clamp boundaries during FD-Adam — that's the plateau source. For bounded monotonic heuristics (the use cases EML's `README` emphasizes) these are fine. For analytical approximations of physical curves with multi-decade dynamic range, they're the ceiling.

**Suggested fix**: make them configurable:

```rust
pub struct EmlConfig {
    pub affine_clamp: (f64, f64),      // default (-10.0, 10.0)
    pub ln_arg_clamp: (f64, f64),      // default (0.01, 10.0)
    pub output_clamp: (f64, f64),      // default (-10.0, 10.0)
    pub head_floor: f64,                // default 0.0
}

impl EmlModel {
    pub fn with_config(depth: usize, input_count: usize, head_count: usize, cfg: EmlConfig) -> Self { ... }
}
```

Defaults stay bit-exact with today's behavior, `EmlModel::new` calls `with_config(EmlConfig::default())`. This is the biggest-payoff change — it's what blocks the universality theorem in practice for us.

### Issue 3: Finite-difference overhead (efficiency)

Not a bug, a note. Depth-4 at 46 params (`model.rs:679-682`) means two-sided FD costs ~92 forward passes per Adam step. Our 6-seed × 400-step warm-start run took ~340 s; an analytic-gradient path would be ~4 s.

**Suggested fix**: add

```rust
fn predict_with_grad(&self, inputs: &[f64]) -> (Vec<f64>, Vec<f64>);
```

returning `(outputs, ∂output/∂params)`. Every operator in the tree (`softmax3`, `eml_safe` = `exp` − `ln`, affine mixes) has a closed-form derivative; the clamps can use straight-through estimators. Not a blocker — we routed around it — but it would make gradient-based training idiomatic for external trainers.

## Where EML shines (consistent with our experience)

- Bounded monotonic heuristics (coherence, governance, causal-prediction thresholds).
- Smooth blends of a few inputs where MSE < 0.01 is an acceptable convergence bar (matches the existing `trained` flag logic at `model.rs:536`).
- Function-shape learning without an analytical ground truth.

## Where EML struggles (our finding)

- Functions requiring exp/log compositions across multiple decades of dynamic range (tube I-V, diode saturation, tape bias).
- Functions where tail/extreme-region accuracy matters (cutoff + saturated drive for analog modeling).
- Functions where the loss landscape has a wide low-gradient attractor near a trivial solution (predict-zero / predict-mean), which coordinate descent at its fixed step set cannot escape.

## Reproducer

In the vector-synth repo:

```
crates/vector-synth-trainer/src/tube_warm.rs          — the warm-start library
crates/vector-synth-trainer/src/bin/train_tube_warm.rs — standalone trainer, prints SNR + grid
crates/vector-synth-trainer/src/bin/tube_ab.rs         — Koren vs EML-warm vs Hybrid CSV A/B
```

Both binaries are deterministic (LCG seeded) and produce stable numbers.

## Verdict for our use case

We're shipping Koren analytical at runtime for tubes (ADR-0020 Phase A, already landed). The `TubeModel` trait at `crates/vector-synth-core/src/patch_graph/step/_primitives/tube_model.rs` keeps an EML slot reserved under the `tube-eml` feature — if upstream addresses the clamps and basin-hit rate, we'll re-evaluate per-tube-type (especially power pentodes like 6L6/EL34 where Koren fits the secondary-emission hump poorly and an EML-fit-from-captures could win).

## Attachments (optional)

- `docs/adr/research/tube-eml-warm-start-findings.md` — fuller ⊃μBus-side findings memo.
- `docs/adr/0020-tube-simulator-substrate.md` — ADR where EML lives as a reserved Phase-B slot.

---

*Happy to open a PR against any of the three suggested fixes if the shape looks right to the maintainer. (1) is the smallest and highest-leverage; (3) is easy but invasive to the `train()` signature; (2) is the most work.*
