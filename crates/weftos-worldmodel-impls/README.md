# weftos-worldmodel-impls

Concrete implementations of [`weftos-worldmodel-core`](../weftos-worldmodel-core)
traits for the LeWM latent world model (WEFT-521).

## What ships today (honest AC)

| Component | Status | Notes |
|-----------|--------|--------|
| `Null*` / stub `Encoder`, `Predictor`, `LatentPlanner`, `LatticeApi`, `SigRegMonitor` | **Done** | Default feature set; unit-tested; no ML weights |
| `ActionEncoder` + `NullActionEncoder` / `HashActionEncoder` | **Done** | Maps control bytes → fixed-width `Action` code |
| `StubLattice` composition | **Done** | Wires encoder + predictor + planner for service scaffolding |
| `candle` feature skeleton (`VitTinyConfig`, `CandleVitEncoder`, `AdaLnPredictor`) | **Skeleton** | Module layout + `Unavailable` without weights; no trained checkpoint |
| Full ViT-tiny weights / training loop / AdaLN bake-off | **Not in this crate yet** | Follow-ups: WEFT-529 (pred_φ + CEM), training pipeline TBD |

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

## Remaining candle / training work

1. Pin a workspace-compatible candle line (or switch to ONNX / another edge runtime).
2. Implement ViT-tiny patch embed + transformer blocks under `src/candle/`.
3. Train / export SIGReg-aligned weights (`LATENT_DIM = 192`, isotropic Gaussian prior).
4. Implement AdaLN-modulated `pred_φ(z_t, a_t)` with action conditioning.
5. Wire weight loading (file path / CAS) and model version tags for ExoChain attestation.
6. CEM planner real samples (WEFT-529) on the 10 Hz background path — not 1 ms servo.

## Usage (stubs)

```rust
use weftos_worldmodel_core::{Encoder, Predictor, LatentPlanner, Action};
use weftos_worldmodel_impls::{
    NullEncoder, NullPredictor, NullPlanner, NullActionEncoder, ActionEncoder,
};

let enc = NullEncoder;
let z = enc.encode(b"frame").unwrap();
let a = NullActionEncoder.encode_bytes(b"noop").unwrap();
let z_hat = NullPredictor.predict(&z, &a).unwrap();
let plan = NullPlanner::default().plan(&z, 4).unwrap();
assert_eq!(plan.steps.len(), 4);
let _ = z_hat;
```

## Related tickets

- WEFT-520 — `weftos-worldmodel-core` traits (landed)
- **WEFT-521** — this crate
- WEFT-522 — facade re-export
- WEFT-529 — production pred_φ + CEM planner
- WEFT-543 — latent dim = 192 contract
