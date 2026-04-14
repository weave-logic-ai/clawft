# EML Coherence Approximation

**Module**: `crates/clawft-kernel/src/eml_coherence.rs`
**Feature gate**: `#[cfg(feature = "ecc")]`
**Paper**: Odrzywolel 2026, arXiv:2603.21852v2

## Problem

`spectral_analysis()` uses Lanczos iteration to compute lambda_2 (algebraic
connectivity) in O(k*m) time. On large causal graphs this can take 500+ us
per tick, which is too expensive for the DEMOCRITUS cognitive loop's
every-tick coherence check.

## Solution

A depth-3 EML master formula with 34 trainable parameters that predicts
lambda_2 from 7 graph statistics in O(1) time (~0.1 us). The model
self-trains from exact Lanczos results collected during normal operation.

### Architecture

```
Level 0: 8 linear combos of 7 features (softmax-constrained)  -- 24 params
Level 1: 4 EML nodes (exp(x) - ln(y))                         --  0 params
Level 2: 2 mixing + EML nodes                                  --  8 params
Level 3: 1 output mixing + EML                                 --  2 params
                                                          Total: 34 params
```

### Input Features

| Feature         | Source             | Cost |
|-----------------|--------------------|------|
| node_count      | AtomicU64 load     | O(1) |
| edge_count      | AtomicU64 load     | O(1) |
| avg_degree      | 2*E/V              | O(1) |
| max_degree      | scan node_ids      | O(n) |
| min_degree      | scan node_ids      | O(n) |
| density         | 2*E/(V*(V-1))      | O(1) |
| component_count | BFS connected comp  | O(n+m) |

Note: `from_causal_graph` is O(n+m) due to `connected_components()`.
For truly O(1) feature extraction, component_count should be maintained
incrementally (future work).

### Training

- Gradient-free: random restart (100 seeds) + coordinate descent
- Threshold: MSE < 0.01 to mark model as "trained"
- Minimum 50 training points required
- Training data sourced from graph families with known lambda_2:
  - Complete K_n, Star S_n, Cycle C_n, Path P_n, Erdos-Renyi G(n,p)

### Two-Tier Cadence (not yet wired)

```
every tick:          coherence_fast()      ~0.1 us
when drift > thresh: spectral_analysis()   ~500 us  + model.record()
every 1000 exact:    model.train()         ~10 ms   (one-time)
```

## Integration Points

1. **EccSubsystem**: `eml_coherence: Option<EmlCoherenceModel>` field
2. **Kernel accessor**: `kernel.ecc_eml_coherence()`
3. **CausalGraph**: `graph.coherence_fast(&model)` convenience method
4. **HTTP facade**: `GET /api/ecc/coherence` -> `ecc.coherence` RPC
5. **RPC response shape**: `{ fast: f64, model_trained: bool, training_samples: usize, last_exact: f64 }`

## Files Changed

- `src/eml_coherence.rs` -- new module (EML operator, GraphFeatures, model, tests)
- `src/lib.rs` -- module registration behind `#[cfg(feature = "ecc")]`
- `src/boot.rs` -- EccSubsystem field + init + accessor
- `src/http_facade.rs` -- `/api/ecc/coherence` route + test
