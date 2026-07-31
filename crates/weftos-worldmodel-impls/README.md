# weftos-worldmodel-impls

Concrete implementations of [`weftos-worldmodel-core`](../weftos-worldmodel-core)
traits for the LeWM latent world model (WEFT-521).

## What ships today (honest AC)

| Component | Status | Notes |
|-----------|--------|--------|
| `Null*` / stub `Encoder`, `Predictor`, `LatentPlanner`, `LatticeApi`, `SigRegMonitor` | **Done** | Default feature set; unit-tested; no ML weights |
| `LinearPredPhi` (`pred_φ`) | **Done** | Weights-free linear residual action-conditioned dynamics (WEFT-529) |
| `CemPlanner` / `MppiWarmPlanner` / `GradientPlanner` | **Done** | CEM default @ 10 Hz path; toy planning tests (WEFT-529) |
| `WelfordSigRegMonitor` | **Done** | Online Welford + auto-rollback at 0.85 / 30 s (WEFT-528) |
| `FourConditionRollbackGate` | **Done** | AND of SIGReg / held-out probe / VoE diff / temporal straighten (WEFT-530) |
| `ActionEncoder` + `NullActionEncoder` / `HashActionEncoder` | **Done** | Maps control bytes → fixed-width `Action` code |
| `StubLattice` composition | **Done** | Wires encoder + `pred_φ` + CEM for service scaffolding |
| `candle` feature skeleton (`VitTinyConfig`, `CandleVitEncoder`, `AdaLnPredictor`) | **Skeleton** | Module layout + `Unavailable` without weights; no trained checkpoint |
| Full ViT-tiny weights / training loop / AdaLN bake-off | **Residual** | Needs training pipeline + checkpoint I/O (WEFT-531+) |

Default builds **never** depend on `candle-core` / `candle-nn` (same idiom as
`clawft-voice-onnx` and its empty default features). CI and
`scripts/build.sh test weftos-worldmodel-impls` exercise only the stub path.

## Feature flags

```toml
weftos-worldmodel-impls = { workspace = true }                 # stubs only
weftos-worldmodel-impls = { workspace = true, features = ["std"] }
weftos-worldmodel-impls = { workspace = true, features = ["candle"] }  # experimental
```

- **default** — null/stub trait impls, `no_std` + `alloc`
- **std** — host marker forwarded to core
- **candle** — optional ML skeleton (`candle-core` / `candle-nn` 0.8). Without
  loaded weights, encoder/predictor return `WorldModelError::Unavailable`.
  **Do not enable in default CI** until a toolchain-compatible pin is proven
  (candle-core 0.2.x is blocked — see WEFT-216).

## Residual training work (honest)

Runtime monitors / planners ship without trained neural weights:

1. Pin a workspace-compatible candle line (or switch to ONNX / another edge runtime).
2. Implement ViT-tiny patch embed + transformer blocks under `src/candle/`.
3. Train / export SIGReg-aligned weights (`LATENT_DIM = 192`, isotropic Gaussian prior).
4. Replace [`LinearPredPhi`] with AdaLN-modulated `pred_φ(z_t, a_t)` once weights exist.
5. Wire weight loading (file path / CAS) and model version tags for ExoChain attestation.
6. Optional: learn CEM cost / dynamics residuals from logged rollouts (WEFT-531+).

## Usage (runtime defaults)

```rust
use weftos_worldmodel_core::{Encoder, Predictor, LatentPlanner, Action};
use weftos_worldmodel_impls::{
    NullEncoder, LinearPredPhi, CemPlanner, WelfordSigRegMonitor, NullActionEncoder, ActionEncoder,
};

let enc = NullEncoder;
let z = enc.encode(b"frame").unwrap();
let a = NullActionEncoder.encode_bytes(b"noop").unwrap();
let z_hat = LinearPredPhi::default().predict(&z, &a).unwrap();
let plan = CemPlanner::default().plan(&z, 4).unwrap();
assert_eq!(plan.steps.len(), 4);
let mut mon = WelfordSigRegMonitor::new(1);
let _ = mon.update(&z).unwrap();
let _ = z_hat;
```

## Related tickets

- WEFT-520 — `weftos-worldmodel-core` traits (landed)
- **WEFT-521** — this crate
- WEFT-522 — facade re-export (`weftos-worldmodel`) — landed; prefer that for consumers
- WEFT-528 — Welford SIGReg + auto-rollback (landed)
- WEFT-529 — pred_φ + CEM planner (landed; neural weights residual)
- WEFT-530 — four-condition AND rollback / promotion gate (landed)
- WEFT-543 — latent dim = 192 contract
