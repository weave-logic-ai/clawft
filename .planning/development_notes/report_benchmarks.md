# Benchmark Report -- clawft (weft)

**Generated**: 2026-02-17 12:28 UTC
**Platform**: Linux x86_64 (WSL2, kernel 6.6.87.2-microsoft-standard-WSL2)
**Binary**: `weft` (release build, LTO, strip, opt-level=z, codegen-units=1, panic=abort)

---

## Build Environment

| Property | Value |
|----------|-------|
| Rust version | rustc 1.85.1 (4eb161250 2025-03-15) |
| Edition | 2024 |
| OS | Linux (Debian/WSL2) x86_64 |
| Kernel | 6.6.87.2-microsoft-standard-WSL2 |
| Release profile | opt-level=z, LTO=true, strip=true, codegen-units=1, panic=abort |
| Linking | Dynamic (glibc) |

---

## Measured Results

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Binary size (release, Linux x86_64) | 4.6 MB | < 15 MB | **PASS** |
| Startup time (--version, mean of 50) | 3.5 ms | < 50 ms | **PASS** |
| Startup time (--version, P50) | 3 ms | < 50 ms | **PASS** |
| CLI invocation rate | ~418/s | > 100/s | **PASS** |
| Dynamic libraries | 4 | N/A | INFO |
| Memory RSS (idle) | (see note) | < 10 MB | N/A |

> **Note on Memory RSS**: The `weft --version` command exits in ~3 ms, too fast for /proc-based
> RSS measurement. GNU `time` (`/usr/bin/time -v`) is not installed in this WSL2 environment,
> and valgrind is not available. RSS measurement deferred to CI (ubuntu-latest with GNU time).
> Given the 4.6 MB binary and minimal runtime allocations for `--version`, idle RSS is expected
> to be well under 10 MB.

---

## Performance Targets

| Metric | Target | Measured | Status |
|--------|--------|----------|--------|
| Startup time | < 50 ms | 3.5 ms (mean) | **PASS** -- 14x under target |
| Memory RSS (idle) | < 10 MB | deferred | Pending CI |
| CLI invocation rate | > 100/s | ~418/s | **PASS** -- 4.2x over target |
| Binary size (Linux x86_64) | < 15 MB | 4.6 MB | **PASS** -- 3.3x under target |
| Binary size (WASM, wasip1) | < 300 KB | Pending (R4 -- deps decoupled, build in progress) | Pending |

---

## Startup Time (50 runs of `weft --version`)

| Statistic | Value |
|-----------|-------|
| Runs | 50 |
| Min | 2 ms |
| Max | 11 ms |
| Mean | 3.5 ms |
| Median (P50) | 3 ms |
| P90 | 4 ms |
| P95 | 5 ms |
| P99 | 6 ms |

**Raw data (ms, sorted):**
```
2 2 2 2 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3
3 3 3 3 3 3 3 3 3 3 3 3 3 3 4 4 4 4 4 4
4 4 4 4 4 5 5 5 6 11
```

---

## Binary Size

### Release binary

| Property | Value |
|----------|-------|
| File | `target/release/weft` |
| Size | 4,822,304 bytes (4.6 MB) |
| Format | ELF 64-bit LSB PIE executable, x86-64 |
| Stripped | Yes |
| Linking | Dynamic (glibc) |

### Section breakdown

| Section | Size | Purpose |
|---------|------|---------|
| `.text` | 2,787.1 KB | Executable code |
| `.rodata` | 696.7 KB | Read-only data (strings, constants) |
| `.eh_frame` | 470.7 KB | Exception/unwind tables |
| `.rela.dyn` | 378.0 KB | Dynamic relocations |
| `.data.rel.ro` | 276.6 KB | Relocated read-only data |
| `.eh_frame_hdr` | 68.9 KB | Exception frame index |
| `.data` | 6.9 KB | Mutable global data |
| `.gcc_except_table` | 6.9 KB | C++ exception tables |
| `.dynsym` | 3.6 KB | Dynamic symbol table |
| `.dynstr` | 2.0 KB | Dynamic string table |
| `.got` | 1.8 KB | Global offset table |

> **Observation**: `.text` (code) is 2.7 MB, making up ~58% of the binary. `.rodata` and
> `.eh_frame` together add another 1.1 MB. The exception/unwind tables (`.eh_frame` +
> `.eh_frame_hdr` + `.gcc_except_table` = 547 KB) could potentially be reduced if
> `panic=abort` is combined with `-C force-unwind-tables=no`, though this may affect
> backtraces.

### Dynamic library dependencies (4 libraries)

| Library | Path |
|---------|------|
| linux-vdso.so.1 | (virtual) |
| libgcc_s.so.1 | /lib/x86_64-linux-gnu/libgcc_s.so.1 |
| libm.so.6 | /lib/x86_64-linux-gnu/libm.so.6 |
| libc.so.6 | /lib/x86_64-linux-gnu/libc.so.6 |

> For a fully static binary (musl target), use `--target x86_64-unknown-linux-musl`.
> This would eliminate all dynamic dependencies but may slightly increase binary size.

---

## CLI Throughput

| Metric | Value |
|--------|-------|
| Invocations | 100 |
| Total time | 239 ms |
| Rate | ~418 invocations/s |
| Mean per invocation | ~2.4 ms |

---

## Memory Usage (RSS)

| Metric | Value |
|--------|-------|
| Peak RSS | Not measurable in this environment |
| **Target** | < 10 MB |

**Why not measured**: The `weft --version` subcommand completes in ~3 ms, exiting before
`/proc/$PID/status` can be read. GNU `time -v` (which captures peak RSS) is not available
in this WSL2 installation. The `valgrind` suite is also not installed.

**Expected range**: With a 4.6 MB binary, dynamic linking to glibc, and minimal heap
allocation for a `--version` invocation, peak RSS is expected to be 5-8 MB. This will
be validated in CI where GNU time is available.

---

## Comparison: Rust (weft) vs Python (nanobot)

| Metric | Rust (weft) | Python (nanobot) | Improvement |
|--------|-------------|------------------|-------------|
| Startup time | 3.5 ms | ~800 ms | **~229x faster** |
| Memory RSS (idle) | est. 5-8 MB | ~60 MB | **est. ~8-12x less** |
| Binary/install size | 4.6 MB | ~50 MB (venv) | **~11x smaller** |
| CLI invocation rate | ~418/s | ~1.2/s | **~348x faster** |

---

## Rust 1.93.1 Comparison

**Date**: 2026-02-17 16:01 UTC
**Rust version**: rustc 1.93.1 (01f6ddf75 2026-02-11)
**Baseline Rust version**: rustc 1.85.1 (4eb161250 2025-03-15)
**Regression threshold**: 10%

### Summary

All metrics **PASS** -- no regressions detected. The Rust 1.93.1 toolchain produces a smaller, equally fast binary compared to the 1.85.1 baseline.

### Metric Comparison

| Metric | Rust 1.85.1 (baseline) | Rust 1.93.1 | Change | Status |
|--------|------------------------|-------------|--------|--------|
| Binary size | 4,822,304 B (4,710 KB) | 4,586,936 B (4,479 KB) | **-4.9%** (231 KB smaller) | **PASS** |
| Startup time (mean, 50 runs) | 3.50 ms | 3.26 ms | **-6.9%** (faster) | **PASS** |
| Startup time (P50) | 3 ms | 3 ms | 0% | **PASS** |
| Startup time (P90) | 4 ms | 3 ms | -25% (faster) | **PASS** |
| Throughput | 418 invocations/s | 420 invocations/s | **+0.5%** | **PASS** |
| Dynamic libraries | 4 | 4 | No change | INFO |

### Binary Size Section Breakdown (1.85 vs 1.93)

| Section | Rust 1.85.1 | Rust 1.93.1 | Change |
|---------|-------------|-------------|--------|
| `.text` | 2,787.1 KB | 2,668.3 KB | -118.8 KB (-4.3%) |
| `.rodata` | 696.7 KB | 704.7 KB | +8.0 KB (+1.1%) |
| `.eh_frame` | 470.7 KB | 454.6 KB | -16.1 KB (-3.4%) |
| `.rela.dyn` | 378.0 KB | 318.8 KB | -59.2 KB (-15.7%) |
| `.data.rel.ro` | 276.6 KB | 233.4 KB | -43.2 KB (-15.6%) |
| `.eh_frame_hdr` | 68.9 KB | 67.7 KB | -1.2 KB (-1.7%) |
| `.data` | 6.9 KB | 9.4 KB | +2.5 KB (+36.2%) |
| `.gcc_except_table` | 6.9 KB | 6.6 KB | -0.3 KB (-4.3%) |

> **Analysis**: The code section (`.text`) shrank by 118.8 KB (4.3%), reflecting improved codegen
> in Rust 1.93.1. Dynamic relocations (`.rela.dyn`) and relocated read-only data (`.data.rel.ro`)
> saw the largest proportional reductions (~16%), suggesting better linker optimization. The small
> increase in `.rodata` (+8 KB) and `.data` (+2.5 KB) is negligible and likely reflects updated
> compiler metadata or slightly different constant folding decisions. Net result: 231 KB smaller binary.

### Startup Time Distribution (Rust 1.93.1, 50 runs)

| Statistic | 1.85.1 | 1.93.1 |
|-----------|--------|--------|
| Min | 2 ms | 2 ms |
| Mean | 3.5 ms | 3.26 ms |
| Median (P50) | 3 ms | 3 ms |
| P90 | 4 ms | 3 ms |
| Max | 11 ms | 4 ms |

> **Note**: The max value dropped from 11 ms to 4 ms, indicating tighter worst-case latency.
> This is likely a combination of smaller binary (fewer pages to load) and reduced variance
> in this particular WSL2 session. Both runs are well within the <50 ms target.

### Regression Check Output

```
=== Benchmark Regression Check ===
Results:   /tmp/bench-1.93.json
Baseline:  scripts/bench/baseline.json
Threshold: 10%

  PASS  startup_time_ms: baseline=3.5 result=3.26 (-6.85% change)
  PASS  binary_size_kb: baseline=4710 result=4479 (-4.90% change)
  PASS  throughput_invocations_per_sec: baseline=418 result=420.1 (+0.50% change)

RESULT: PASS -- all metrics within 10% threshold
```

### Conclusion

Rust 1.93.1 is a safe upgrade from 1.85.1 for the clawft/weft binary. All three tracked metrics
improved or held steady:
- Binary is **231 KB smaller** (4.9% reduction), primarily from `.text` and `.rela.dyn` savings
- Startup is **6.9% faster** on average, with tighter tail latency
- Throughput is essentially unchanged (+0.5%, within noise)

No action required. The baseline can optionally be updated to reflect the new compiler's numbers.

---

## Benchmark Infrastructure

### Scripts

| Script | Path | Purpose |
|--------|------|---------|
| `startup.sh` | `clawft/scripts/bench/startup.sh` | Measures average cold-start time (10 runs) |
| `memory.sh` | `clawft/scripts/bench/memory.sh` | Measures peak RSS via `/usr/bin/time -v` |
| `throughput.sh` | `clawft/scripts/bench/throughput.sh` | Measures CLI invocation rate (100 iterations) |
| `size-check.sh` | `clawft/scripts/bench/size-check.sh` | Checks binary size against target limits |
| `run-all.sh` | `clawft/scripts/bench/run-all.sh` | Orchestrator: runs all benchmarks in sequence |

### CI Workflow

Benchmarks run automatically via `.github/workflows/benchmarks.yml`:
- **Trigger**: Push to main, pull requests
- **Runner**: `ubuntu-latest` for consistency
- **Flow**: Build release binary, run all 4 benchmarks, post results as PR comment
- **Artifacts**: Results stored as GitHub Actions artifacts for historical tracking

## How to Run Locally

```bash
# 1. Build the release binary (with LTO + strip for accurate size measurement)
cargo build --release --manifest-path clawft/Cargo.toml -p clawft-cli

# 2. Run all benchmarks at once
bash clawft/scripts/bench/run-all.sh clawft/target/release/weft

# 3. Or run individual benchmarks
bash clawft/scripts/bench/startup.sh clawft/target/release/weft
bash clawft/scripts/bench/memory.sh clawft/target/release/weft
bash clawft/scripts/bench/throughput.sh clawft/target/release/weft
bash clawft/scripts/bench/size-check.sh clawft/target/release/weft
```

**Prerequisites**: Linux with `/usr/bin/time` (GNU time, not shell builtin). On macOS, install `gtime` via Homebrew and adjust scripts accordingly.

---

## Methodology

- **Startup time**: Mean/median/percentiles of 50 runs of `weft --version` measured with `date +%s%N` (nanosecond precision)
- **Memory RSS**: Peak RSS from `/usr/bin/time -v` output (not available in this environment; deferred to CI)
- **Throughput**: CLI invocation rate for `weft --version` over 100 iterations
- **Binary size**: `ls -la` of release binary built with LTO + strip + opt-level=z
- **Section analysis**: `readelf -S` for ELF section breakdown; `ldd` for dynamic dependencies
- **Python baseline**: Estimated from prior `nanobot --version` measurements in comparable environments
