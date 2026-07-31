# Benchmark Results

This document records clawft performance benchmarks for the native binary
(`weft`) and the WASM build (`wasm32-wasip2`).  All values come from the
automated benchmark scripts in `scripts/bench/` and the baseline snapshot
stored in `scripts/bench/baseline.json`.

## Baseline (2026-02-17)

| Metric | Value | Direction |
|--------|-------|-----------|
| Startup time | 3.5 ms | lower is better |
| Binary size (release, stripped) | 4,710 KB | lower is better |
| Tool invocation throughput | 418 invocations/s | higher is better |

### WASM build (`wasm32-wasip2`)

| Metric | Value |
|--------|-------|
| Raw `.wasm` size | 57.9 KB |
| Gzipped `.wasm` size | 24.3 KB |
| Allocator | dlmalloc |
| Cold instantiation | not yet measured |
| Warm instantiation | not yet measured |

### Browser WASM (`wasm32-unknown-unknown`, WEFT-407)

Profiling surface for **load / init / first-msg / memory** (BW6 targets).
CI runs a schema stub; live numbers come from the `www/` harness
(`window.__clawftPerf`). See [docs/browser/performance.md](../browser/performance.md).

```bash
scripts/build.sh browser-perf
# Live: scripts/build.sh browser && scripts/build.sh serve → dump __clawftPerf
```

| Metric | Target | Checked-in baseline |
|--------|--------|---------------------|
| WASM load | &lt; 500 ms | null (stub until live run) |
| `init()` | &lt; 200 ms | null |
| First message (incl. LLM) | &lt; 3 s | null |
| Subsequent message | &lt; 2 s | null |
| WASM heap | &lt; 32 MB | null |

## Test Suite

The workspace contains **294+** unit and integration tests across 9 crates.
A full `cargo test --workspace` run completes in under 10 seconds on a
standard development machine.

Key crate test counts (approximate):

| Crate | Tests |
|-------|-------|
| clawft-core | ~80 |
| clawft-tools | ~50 |
| clawft-llm | ~40 |
| clawft-channels | ~30 |
| clawft-platform | ~20 |
| clawft-services | ~20 |
| clawft-types | ~15 |
| clawft-cli | ~25 |
| clawft-wasm | ~10 |

## WEFT-370 — Graphify extraction + graph_ops (GRAPH-041)

Harness: `clawft_graphify::bench` (mock CI path + criterion benches).

```bash
# CI-friendly (mock extraction, 128-node suite)
scripts/build.sh test clawft-graphify

# Criterion
cargo bench -p clawft-graphify --bench extraction
cargo bench -p clawft-graphify --bench graph_ops
```

See [docs/guides/graphify-bench.md](../guides/graphify-bench.md) for thresholds,
API, and soak (`-- --ignored`) instructions. Numbers are host-dependent; the
gate is structural (tests + optional regression ratio), not a checked-in
baseline table.

## CONS-002 / WEFT-34 — Map contention (CostTracker / RateLimiter)

**Decision (2026-07-30): keep `RwLock<HashMap>`; do not switch to DashMap.**

Criterion microbench models cost-reserve and rate-check hot paths:

```bash
cargo bench -p clawft-core --bench map_contention_bench
```

| Scenario (representative) | Result |
|---------------------------|--------|
| cost-style, 4 threads, 8 keys (small team) | **RwLock ~1.6× DashMap** |
| rate-style, 4–8 threads, 8 keys | DashMap ~1.8–3.3× (still µs/op) |
| wide fan-out 100 keys × 16 threads | DashMap 5–12× |

At the design envelope (1–10 concurrent users) absolute map cost is
negligible vs LLM latency, so a production `dashmap` dependency is not
justified. Full table and rationale:
`.planning/development_notes/01-tiered-router/consensus-log.md` (CONS-002).

## Voice pipeline (WEFT-229)

Latency (speech-end → first-response-byte), WER, and CPU wake-budget helpers live
in `crates/clawft-bench-voice`. Default thresholds: p95 ≤ 500 ms, WER ≤ 15%,
wake &lt; 2% of one core, full pipeline &lt; 10%. CI uses mock/fixture paths only
(no ONNX). See [docs/guides/voice-bench.md](../guides/voice-bench.md).

```bash
scripts/build.sh test clawft-bench-voice
```

## WEFT-361 / KG-004 — Spectral lambda₂ path selection

Benchmark harness comparing **Lanczos**, **RFF**, and **EML** estimates of
algebraic connectivity on synthetic graphs (default: unit-weight cycle
`C_n` with closed-form reference).

```bash
# Default sizes: 1K, 10K (CI-friendly; 100K optional)
cargo bench -p clawft-kernel --bench spectral_lambda2_bench

# Full scale including 100K (Lanczos auto-skipped above 20K unless overridden)
WEFT_BENCH_SIZES=1000,10000,100000 cargo bench -p clawft-kernel \
    --bench spectral_lambda2_bench
```

### Decision rule (shipped in `select_spectral_method`)

| Node count `n` | Path | Notes |
|----------------|------|-------|
| `n < 10_000` | **Lanczos** | Sparse O(k·m); Fiedler quality preferred |
| `10_000 ≤ n < 100_000` | **RFF** | O(m) features; ~3–6× faster, ~5% accuracy loss |
| `n ≥ 100_000` | **EML** | O(1) feature model for tick rate; re-anchor with RFF |

Constants: `SPECTRAL_RFF_MIN_NODES = 10_000`, `SPECTRAL_EML_MIN_NODES = 100_000`
in `clawft-kernel::causal`. `CausalGraph::spectral_analysis_auto` executes
Lanczos or RFF (EML needs `EmlCoherenceModel` and is selected by policy only).

DEMOCRITUS exact path uses `spectral_analysis_auto` (size-dispatched).

## Running Benchmarks

All scripts live in `scripts/bench/`.  They require a release build of `weft`:

```bash
cargo build --release
```

### Individual benchmarks

```bash
# Startup time (10 iterations, median)
scripts/bench/startup-time.sh target/release/weft 10

# Peak RSS memory usage
scripts/bench/memory-usage.sh target/release/weft

# Tool invocation throughput (100 iterations)
scripts/bench/throughput.sh target/release/weft 100

# WASM binary size (requires wasm32-wasip2 target)
scripts/bench/wasm-size.sh
```

### Combined run

```bash
# Run all benchmarks and print a summary
scripts/bench/run-all.sh target/release/weft

# Run all and save results as JSON
scripts/bench/run-all.sh target/release/weft --save-results results.json

# Run all with regression check against baseline
scripts/bench/run-all.sh target/release/weft --check-regression
```

### Regression detection

The `regression-check.sh` script compares a results JSON against the
baseline and exits non-zero if any metric regresses by more than the
threshold (default 10%):

```bash
scripts/bench/regression-check.sh results.json scripts/bench/baseline.json 10
```

For each metric the direction matters:

- **Startup time** and **binary size**: lower is better -- an *increase*
  beyond the threshold is a regression.
- **Throughput**: higher is better -- a *decrease* beyond the threshold is
  a regression.

### WASM profiling

```bash
# Section-level size analysis (requires twiggy)
scripts/bench/wasm-twiggy.sh

# Size gate -- fails if .wasm exceeds a limit
scripts/bench/wasm-size-gate.sh

# Compare allocators (dlmalloc vs wee_alloc)
scripts/bench/alloc-compare.sh

# Feature-flag size impact
scripts/bench/wasm-feature-check.sh
```

## CI Integration

The `benchmarks.yml` GitHub Actions workflow runs the full benchmark suite
on every push to `main` and on pull requests.  It uploads results as
artifacts and runs regression detection against the committed baseline.
