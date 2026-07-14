# Vector backend benchmark — first results & verdict (2026-07-14, WEFT-366)

Harness: `crates/clawft-kernel/benches/vector_backend_bench.rs` (committed
`858b75c7`). Clustered-gaussian fixed-seed corpus (recall-discriminative),
brute-force ground truth per metric, env-parameterized scale
(`WEFT_BENCH_N/DIM/QUERIES/SEED`). Run:
`scripts/build.sh bench clawft-kernel vector_backend_bench --features diskann`.

## Measured (10K vectors, 384-dim, 100 queries, M-series dev machine)

| backend | metric | insert vps | recall@10 | p50 µs | p95 µs | RSS MB | disk | build |
|---|---|---|---|---|---|---|---|---|
| hnsw | cosine | 2.49M | 0.943 | 398 | 545 | 76 | 0 | ~0 |
| diskann PQ-on (real) | sqL2 | 3.54M | 0.994 | 363 | 454 | 118 | 18.9MB | **128s** |
| diskann PQ-off (real) | sqL2 | 3.85M | 0.994 | 427 | 585 | 124 | 18.0MB | **77s** |
| hybrid (real cold tier) | mixed | 0.99M | **0.113** | 755 | 957 | 134 | 18.9MB | 130s |

A 100K sweep was killed after ~91 min CPU (single core pegged), mid-Vamana
build — consistent with the projected 25–35 min **per DiskANN arm** at 100K
and hours at 1M.

## Why the build is slow (analyzed, not a config accident)

`ruvector-diskann v2.1.0`'s Vamana `build()` is a correct but **serial**
textbook implementation: one thread walks every node (98.6% CPU = one
core), two full passes (alpha=1.2), beam-128 greedy search + robust-prune
per node, plus an inline re-prune for every saturated back-edge — ≈40K
full-dimension distance evals per node. At 10K/384d that is ~128s on one
core; the math matches the measurement exactly. PQ does not accelerate the
build (query-side only in this implementation). Reference DiskANN
parallelizes the node loop; a rayon-over-nodes port is the obvious ~10×.

## Correctness bugs found by the harness (filed, open)

- **WEFT-660**: real `DiskAnnBackend::search` returns `SearchResult.id = 0`
  for every hit (only `.key` is usable).
- **WEFT-661**: `HybridBackend::merge_results` compares cosine (hot) and
  squared-Euclidean (cold) raw distances unnormalized — recall@10 = 0.113
  with the real cold tier. Masked under default features (stub cold tier is
  also cosine).

## Verdict — HNSW primary; DiskANN = COME BACK TO IT (deferred, not disqualified)

- **HNSW stays the live/primary backend.** The ECC workload is streaming
  (`index_turn` inserts continuously); HNSW is incremental with no build
  step and holds 0.94 recall @ ~400µs at this scale.
- **DiskANN is deferred, revisit when any of:** (a) upstream
  `ruvector-diskann` ships a parallel and/or incremental build (watch
  releases; an upstream issue proposing rayon-over-nodes is the cheap ask);
  (b) WEFT-660/661 land; (c) a genuine cold/static tier materializes
  (e.g. nightly off-path snapshot builds over promoted memories) where
  batch build cost is off the hot path. Its query profile is genuinely
  good (0.994 recall, p50 363µs, beats HNSW) — the blocker is build
  economics on a streaming workload, not retrieval quality.
- **Hybrid is blocked on WEFT-661** and unmeasurable until fixed.
- **500K/1M sweeps intentionally not run** — the serial build cost
  disqualifies the current implementation from the live tier at any larger
  scale; re-run the ladder when the revisit conditions trigger (harness is
  ready, one command).
