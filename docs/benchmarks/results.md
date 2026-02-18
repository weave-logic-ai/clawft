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
