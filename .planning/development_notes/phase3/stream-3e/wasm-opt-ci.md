# Phase 3E: wasm-opt Pipeline, Twiggy Profiling, and CI Size Gate

**Date**: 2026-02-17
**Stream**: 3E - WASM Optimization
**Status**: Implemented and validated

---

## What Was Done

### 1. Added `cdylib` crate-type to clawft-wasm

The `crates/clawft-wasm/Cargo.toml` was missing `crate-type = ["cdylib", "rlib"]` in its
`[lib]` section. Without `cdylib`, cargo only produces an `.rlib` for library crates on
WASM targets, not a `.wasm` file. Added the `[lib]` section to produce both formats.

### 2. Created `scripts/build/wasm-opt.sh`

Post-processing script that runs `wasm-opt -Oz` on the clawft WASM binary.

**Features:**
- Accepts optional input/output file paths with smart defaults
- Defaults to `target/wasm32-wasip2/release-wasm/clawft_wasm.wasm`
- Falls back to wasip1 target if wasip2 not found
- Checks for wasm-opt installation with helpful install hints
- Detects WASM binary type (core module vs component model)
- For core modules: runs `wasm-opt -Oz --enable-bulk-memory --enable-sign-ext` directly
- For component model (wasip2): uses `wasm-tools component unbundle` to extract core
  modules, optimizes each with wasm-opt, reports estimated component savings
- Prints before/after sizes with reduction percentage
- Prints gzipped size
- Optionally validates output with `wasmtime compile` if available
- Color-coded output consistent with existing script patterns

### 3. Created `scripts/bench/wasm-twiggy.sh`

WASM binary profiling script using twiggy.

**Features:**
- Detects binary type and extracts core module from components (twiggy requires core modules)
- Runs `twiggy top` (top 20 size contributors), `twiggy dominators`, and `twiggy monos`
- Outputs all reports to `target/wasm-profile/` with timestamps
- Prints top 5 size contributors to stdout
- Falls back to wasip1 or .opt.wasm variants
- Checks for twiggy installation with install hint
- Gracefully handles failures for each twiggy subcommand independently

### 4. Created `scripts/bench/wasm-size-gate.sh`

CI size gate script that enforces WASM binary size budgets.

**Features:**
- Configurable raw and gzipped thresholds (default: 300 KB / 120 KB)
- Measures raw size with `wc -c` and gzipped size with `gzip -9 -c | wc -c`
- Clear PASS/FAIL output with color coding
- Exit 0 on pass, exit 1 on fail
- Suggests twiggy profiling when budget is exceeded
- Falls back to wasip1 and .opt.wasm variants

### 5. Created `.github/workflows/wasm-build.yml`

GitHub Actions workflow for WASM build, optimization, and size gating.

**Pipeline stages:**
1. Install Rust toolchain with `wasm32-wasip2` target
2. Install binaryen (provides wasm-opt), bc, and wasm-tools
3. Build WASM binary with `release-wasm` profile
4. Run wasm-opt post-processing (with component model support)
5. Run size gate on the original component binary (300 KB / 120 KB)
6. Run twiggy profiling (optional, continues on error)
7. Post size comparison as GitHub job summary (table format)
8. Upload twiggy reports and WASM binaries as artifacts

---

## Script Descriptions

| Script | Purpose | Location |
|--------|---------|----------|
| `wasm-opt.sh` | Post-build optimization with binaryen | `scripts/build/wasm-opt.sh` |
| `wasm-twiggy.sh` | Binary size profiling with twiggy | `scripts/bench/wasm-twiggy.sh` |
| `wasm-size-gate.sh` | CI size budget enforcement | `scripts/bench/wasm-size-gate.sh` |

All scripts use `set -euo pipefail`, are idempotent, and work both locally and in CI.

---

## CI Changes

Added new workflow `.github/workflows/wasm-build.yml` that:
- Triggers on push/PR to main
- Caches cargo registry, git, and target directories
- Installs binaryen from apt and wasm-tools from cargo
- Installs twiggy from cargo (optional, non-blocking)
- Size gate runs on the original component binary (not the extracted core module)
- Posts size summary table to GitHub job summary
- Uploads artifacts: WASM binaries and twiggy reports (30-day retention)

---

## Test Results

### Size Gate (passing)
```
=== WASM Size Gate ===
File:    target/wasm32-wasip2/release-wasm/clawft_wasm.wasm
Raw:     57.9 KB (59381 bytes)
Gzipped: 24.2 KB (24844 bytes)
Budget:  300 KB raw / 120 KB gzipped
PASS: Raw size: 57.9 KB <= 300 KB budget
PASS: Gzipped: 24.2 KB <= 120 KB budget
```

### Size Gate (failing -- intentional threshold test)
```
Budget: 50 KB raw / 20 KB gzipped
FAIL: Raw size: 57.9 KB > 50 KB budget
FAIL: Gzipped: 24.2 KB > 20 KB budget
```

### wasm-opt (component model)
```
Input:     57.9 KB (component model, wasip2)
Core module: 49.0 KB -> 39.3 KB (20% reduction)
Estimated optimized component: 48.3 KB
Gzipped optimized core: 18.1 KB
```

### Twiggy profiling
```
Top 5 size contributors (core module, 49.0 KB):
  code[149]  5,390 bytes (10.73%)
  data[0]    4,825 bytes  (9.60%)
  code[152]  1,668 bytes  (3.32%)
  code[102]  1,089 bytes  (2.17%)
  code[150]  1,066 bytes  (2.12%)
Total: 274 items across code and data sections
```

### Missing tool handling
- wasm-opt.sh: Clear error message with install instructions (exit 1)
- wasm-twiggy.sh: Clear error message with install instructions (exit 1)
- wasm-size-gate.sh: Clear error when file not found (exit 1)
- All scripts handle component model binaries gracefully

---

## Issues Found

1. **Missing `cdylib` crate-type**: The `clawft-wasm` crate only had default lib type
   (rlib), which does not produce `.wasm` output files. Added `[lib]` section with
   `crate-type = ["cdylib", "rlib"]` to enable WASM binary generation.

2. **Component model incompatibility with wasm-opt and twiggy**: The `wasm32-wasip2`
   target produces component model binaries (version 0d), not core modules (version 01).
   wasm-opt 116 and twiggy do not support component model binaries. Solved by using
   `wasm-tools component unbundle` to extract core modules before processing.

3. **Recomposition not yet reliable**: After optimizing the extracted core module, the
   binary cannot be reliably recomposed into a component with `wasm-tools component new`
   because wasm-opt strips the WIT metadata custom sections. The CI size gate therefore
   runs on the original component binary. The core module optimization data is reported
   for informational purposes.

4. **wasip1 cdylib produces empty binary**: Building with `wasm32-wasip1` + `cdylib`
   produces a 75-byte stub because there are no `#[no_mangle] pub extern "C" fn` exports.
   The wasip2 target uses component model which handles exports differently.

5. **Twiggy output shows function indices, not names**: Because the release-wasm profile
   strips symbols (`strip = true`), twiggy can only show function indices (e.g., `code[149]`)
   rather than demangled names. To get named output, build without strip temporarily.

---

## Current Baseline

| Metric | Value |
|--------|-------|
| WASM component (raw, pre-opt) | 57.9 KB (59,381 bytes) |
| WASM component (gzip, pre-opt) | 24.2 KB (24,844 bytes) |
| Core module (raw, pre-opt) | 49.0 KB (50,246 bytes) |
| Core module (raw, post-opt) | 39.3 KB (40,341 bytes) |
| Core module (gzip, post-opt) | 18.1 KB (18,621 bytes) |
| wasm-opt core reduction | 20.0% |
| Estimated optimized component | 48.3 KB |
| Budget (raw) | 300 KB |
| Budget (gzip) | 120 KB |
| Headroom (raw) | 242.1 KB (81% of budget remaining) |
| Headroom (gzip) | 95.8 KB (80% of budget remaining) |

---

## Next Steps

1. Integrate WASM size metrics into `scripts/bench/baseline.json`
2. Phase 3E continues with allocator comparison (dlmalloc vs talc vs lol_alloc)
3. Phase 3E continues with allocation tracing wrapper
4. Phase 3E continues with WASM instantiation time benchmarking
5. When wasm-opt gains component model support, update script to optimize in-place
6. Consider building without strip for one-off profiling sessions (richer twiggy output)
