# Graphify extraction + graph_ops benchmarks (WEFT-370)

Harness module: [`crates/clawft-graphify/src/bench/`](../../crates/clawft-graphify/src/bench/).

Criterion targets (MASTER_PLAN GRAPH-041):

| Bench | Path | What it measures |
|-------|------|------------------|
| `extraction` | `benches/extraction.rs` | Mock extraction throughput + `build::build` merge |
| `graph_ops` | `benches/graph_ops.rs` | Construct / neighbor / cluster / god_nodes / suite at 1K–50K nodes |

## Why

Phase 6 of the graphify port called for dedicated benchmarks:

1. **Extraction throughput** — target ~1K files/sec (AST); 10K files in &lt; 2 min  
2. **Graph ops** — 50K-entity graph queries  
3. **Memory soft budget** — 50K entities under ~500 MB (structural estimate)  
4. **Token reduction** — corpus tokens vs graph-query surface  
5. **Regression** — optional fail if &gt; 10% slower than a baseline  

CI runs the **mock path** only (no tree-sitter, no LLM, sub-second).

## Default thresholds

| Metric | Default limit | Notes |
|--------|---------------|-------|
| Extraction mean files/sec | ≥ 10 | Loose for CI; real AST target is 1000 |
| Graph-ops suite total (CI scale) | ≤ 5000 ms | CI uses 128 nodes |
| Memory estimate | ≤ 500 MiB | Structural model, not RSS |
| Regression vs baseline | ≤ +10% | Only when a baseline ms is supplied |

Override via `GraphifyBenchThresholds` or JSON
(`GraphifyBenchThresholds::from_json`).

## Running

```bash
# CI-friendly unit + integration tests (mock data)
scripts/build.sh test clawft-graphify

# Or scoped cargo
cargo test -p clawft-graphify
cargo test -p clawft-graphify --test bench_harness_integration

# Optional ignored soaks (1K files / 50K nodes)
cargo test -p clawft-graphify --test bench_harness_integration -- --ignored

# Criterion microbenches (not part of `scripts/build.sh test`)
cargo bench -p clawft-graphify --bench extraction
cargo bench -p clawft-graphify --bench graph_ops

# WEFT-516: label-propagation vs SASE clustering only
cargo bench -p clawft-graphify --bench graph_ops -- graph_ops_cluster_method
```

### Cluster method comparison (WEFT-516)

`graph_ops_cluster_method` times `ClusterMethod::LabelPropagation` vs
`ClusterMethod::Sase` at 1K and 10K nodes on the same synthetic ring+skip
graphs. SASE is always available via `cluster_with`; Cargo feature
`sase-cluster` only switches the default for `cluster()`.

Reports serialize to JSON (`GraphifyBenchReport`) for CI artifacts.

## API sketch

```rust
use clawft_graphify::bench::extraction::{ExtractionHarness, CI_FILE_COUNT, CI_ENTITIES_PER_FILE};
use clawft_graphify::bench::graph_ops::{GraphOpsHarness, CI_NODE_COUNT};
use clawft_graphify::bench::thresholds::GraphifyBenchThresholds;
use clawft_graphify::bench::report::GraphifyBenchReport;

let thresholds = GraphifyBenchThresholds::default();

let mut ex = ExtractionHarness::new();
ex.run_mock_batch(CI_FILE_COUNT, CI_ENTITIES_PER_FILE).unwrap();
let stats = ex.stats().unwrap();

let mut g = GraphOpsHarness::new();
let suite = g.run_suite(CI_NODE_COUNT);

let eval = thresholds.evaluate(Some(&stats), Some(&suite), /* baseline_ms */ None);
let report = GraphifyBenchReport::from_evaluation(thresholds, eval, Some(stats), Some(suite));
println!("{}", report.to_json_pretty().unwrap());
```

### Real AST extractors

Implement `clawft_graphify::bench::extraction::FileExtractor` around
`extract::ast` (feature `ast-extract` / `lang-*`) and pass it to
`ExtractionHarness::run_batch`. Keep default CI on `MockExtractor`.

## Relationship to other benches

| Crate / path | Role |
|--------------|------|
| `clawft-graphify::bench` | Graphify extraction + graph ops (this guide) |
| `clawft-bench-voice` | Voice latency / WER / CPU |
| `clawft-edge-bench` | ESP32-S3 edge scoring (out-of-workspace) |
| `clawft-core` / `clawft-kernel` criterion | Pipeline, map contention, vector backends |
| `scripts/bench/` | Native binary startup / bundle size |

## Planning references

- `.planning/graphify-rs/MASTER_PLAN.md` — GRAPH-041  
- `docs/adr/adr-082-graphify-port.md` — Phase 6 benches still open until this ticket  
- Plane: **WEFT-370**
