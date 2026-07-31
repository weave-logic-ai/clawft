# WEFT-354 / KG-013 — K-STEMIT spatio-temporal GNN (sonobuoy)

**Branch:** `feat/weft-354-k-stemit`  
**Plane:** WEFT-354 · cycle 0.8.x · ws12-knowledge-graph  
**Status:** Scaffold + real pure-Rust forward/train path landed

## Ticket / AC

| Criterion | Status |
|-----------|--------|
| Module path in kernel/graph stack | **Done** — `clawft-kernel::sensor_graph` (`--features sensor`) |
| K-STEMIT-style space + time interface | **Done** — GraphSAGE + Temporal GLU + physics residual + α fusion |
| Train/eval or forward scaffold + synthetic fixtures | **Done** — `fixture` + `train_sgd` / `evaluate` |
| Unit tests (no GPU) | **Done** — graph, layers, model, fixture, train |
| Design note under `docs/plans/` | **This file** |

## Why this module

K-STEMIT (arXiv:2604.09922) is a dual-branch spatio-temporal GNN for ice radar.
The sonobuoy mapping (`.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`)
translates:

| K-STEMIT | Sonobuoy |
|----------|----------|
| Haversine GraphSAGE | Buoy array graph, inverse-distance / delay edges |
| GLU temporal branch | Hydrophone energy / band time series |
| MAR physics priors | SSP, thermocline, sea state, current, bottom type |
| Adaptive α fusion | Detect (temporal) vs bearing (spatial) vs species (both) |

A thin graph + mean-agg stub already lived in `sensor_graph.rs` (CHANGELOG
KG-013). WEFT-354 expands that into a **learnable** dual-branch model with
analytical SGD, synthetic arrays, and train/eval helpers — still pure Rust,
no GPU / ndarray.

## Module layout

```text
crates/clawft-kernel/src/sensor_graph/
  mod.rs      — public re-exports, feature-gated via `sensor`
  graph.rs    — SensorNode / SensorEdge / SensorGraph (+ radius / full mesh)
  layers.rs   — Linear, TemporalGlu, GraphSageLayer, softmax CE helpers
  model.rs    — KstemitModel + KstemitConfig + ForwardOutput
  fixture.rs  — synthetic fleet + ambient / vessel / mammal_call series
  train.rs    — train_sgd, evaluate, metrics
```

Enable with:

```bash
scripts/build.sh test clawft-kernel -- --features sensor
# or:
cargo test -p clawft-kernel --features sensor sensor_graph
```

## Forward path

```text
temporal window ──► TemporalGlu (proj → GLU) ──► h_t ──┐
                                                       │ α = σ(α_logit)
node feats + AGG ─► GraphSAGE (W_self, W_neigh, ReLU) ► h_s ─┼── fuse
                                                       │
physics[5] ───────► Linear ──────────────────────────► h_p ─┘ · γ
                                                              │
                                                         Linear head
                                                              ▼
                                                           logits / softmax
```

- **Spatial:** `h_s = ReLU(W_self x + W_neigh AGG(N))` with inverse-distance mean AGG.
- **Temporal:** `h_t = a ⊙ σ(g)` where `[a;g] = W · window + b`.
- **Fusion:** `h = α h_s + (1-α) h_t + γ h_p` (`γ` = `physics_scale`, default 0.25).
- **Head:** class logits for 3-way synthetic labels (ambient / vessel / mammal).

## Training

`train_step` uses **analytical** gradients through Linear, GLU, GraphSAGE ReLU,
and α (via sigmoid). SGD + optional L2 on weights. `train_sgd` shuffles
samples each epoch over a [`SyntheticDataset`].

## Deferred (explicit)

- Full 5-branch K-STEMIT active-imaging / SAS branch (WEFT-545).
- Real geodesic + BELLHOP / KRAKEN propagation (still Euclidean + 1500 m/s).
- Multi-hop *learned* GraphSAGE stack (only 1-hop learned; multi-hop remains
  the unlearned baseline on `SensorGraph::aggregate_neighbors`).
- ONNX / GPU export; federated FedAvg path from sonobuoy FL notes.
- Wiring into mesh/sensor services or clawft-graphify export formats.
- Standalone `clawft-sensor` crate (AC allowed kernel path; stayed in kernel
  to match existing `sensor` feature + CHANGELOG KG-013 home).

## Tests (representative)

```bash
cargo test -p clawft-kernel --features sensor sensor_graph
```

Covers: graph construction, GLU/GraphSAGE grads (finite-diff spot check),
forward shapes, single-sample loss drop, synthetic energy separation,
end-to-end train accuracy > chance.

## References

- arXiv:2604.09922 (K-STEMIT)
- `.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`
- ADR candidate ADR-053 (spatio-temporal dual-branch) — not authored here
- ADR-082 graphify port table (KG-013 row)
