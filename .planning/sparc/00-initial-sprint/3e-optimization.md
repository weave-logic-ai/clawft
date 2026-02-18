# SPARC Implementation Plan: Phase 3E - WASM Allocator & Size Optimization

**Phase**: 3E - WASM Allocator & Size Optimization
**Status**: Draft
**Timeline**: 1-2 weeks (5-8 days LOE)
**Depends On**: Phase 3A (WASM crate created), Phase 3B (CI/CD + benchmarks), Phase 3C (Rust 1.93)
**Parallel With**: Phase 3D (WASI HTTP/FS + Docker) -- different concerns on same WASM crate
**Deliverables**: Optimized WASM binary (<= 300 KB), allocator benchmarks, wasm-opt pipeline, size budget CI gate, memory profiling harness, startup time benchmarks
**Priority**: High -- directly impacts WASM < 300 KB budget and gates all subsequent WASM feature work

---

## 0. Current Baseline

| Metric | Value | Source |
|--------|-------|--------|
| Native binary size (weft, release, x86_64) | 4,479 KB (4.4 MB) | Rust 1.93 benchmark |
| .text section | 2,668 KB | readelf -S |
| .rodata section | 705 KB | readelf -S |
| .eh_frame + .eh_frame_hdr + .gcc_except_table | 529 KB | readelf -S |
| .rela.dyn | 319 KB | readelf -S |
| .data.rel.ro | 233 KB | readelf -S |
| Startup time (P50) | 3 ms | 50-run benchmark |
| Startup time (mean) | 3.26 ms | 50-run benchmark |
| Throughput | 420 invocations/s | 100-iteration benchmark |
| WASM binary size (cdylib, wasip2) | **57.9 KB raw / 24.3 KB gzipped** | Measured 2026-02-17 (opt-level=z, LTO, strip) |
| WASM allocator | dlmalloc (GlobalDlmalloc) | clawft-wasm/src/allocator.rs |
| Release profile | opt-level=z, lto=true (fat), codegen-units=1, panic=abort, strip=true | Cargo.toml |
| Release-wasm profile | inherits release, opt-level=z | Cargo.toml |
| Rust toolchain | 1.93 (edition 2024) | rust-toolchain.toml |
| WASM target (primary) | wasm32-wasip2 | User requirement; Rust 1.93 has Tier 2 wasip2 |
| WASM target (fallback) | wasm32-wasip1 | WAMR compatibility |

### Key Observations

1. **dlmalloc is the current WASM allocator** -- the technical requirements and 3A plan both specify talc, but `clawft-wasm/src/allocator.rs` uses `dlmalloc::GlobalDlmalloc`. dlmalloc contributes ~10 KB to WASM Code section. This is the primary allocator comparison target.
2. **Fat LTO is already enabled** -- `lto = true` in release profile. Thin LTO comparison needed for build-time tradeoff data.
3. **.eh_frame is 529 KB** -- unwind tables are ~12% of native binary. With `panic=abort` already set, these can be stripped.
4. **wasm-opt is not integrated** -- the build pipeline does not run binaryen post-processing. Expected 15-30% WASM size reduction.
5. **No WASM size profiling exists** -- twiggy has not been run on the WASM output.
6. **No WASM startup time measurement** -- instantiation cost unknown.
7. **`clawft-types` pulls `chrono`, `uuid`, `dirs`** -- these transitives may inflate the WASM binary with unused code. Requires profiling.

---

## 1. Specification

### 1.1 Problem Statement

The `clawft-wasm` crate produces a WASM binary targeting `wasm32-wasip2` (primary) with
`wasm32-wasip1` as a fallback for WAMR. The binary uses `dlmalloc` as its global allocator,
inherited from the default Rust WASM toolchain. No post-compilation optimization (`wasm-opt`)
is applied. No CI gate enforces a size budget. No tooling exists to profile allocation
patterns, identify binary size contributors, or measure WASM instantiation time.

The crate currently contains stubs (727 LOC across 6 files) and depends on `clawft-types`,
`serde`, and `serde_json`. As real implementations land (Phase 3D: HTTP/FS, Phase 3F: agents),
the binary will grow. Without proactive size optimization infrastructure, the WASM binary
will exceed the 300 KB target before the first real feature ships.

This phase builds the optimization infrastructure before the binary grows, establishing
baselines, tooling, and CI guardrails that protect the size budget through all subsequent
development.

### 1.2 Goals

| # | Goal | Measurable Outcome |
|---|------|--------------------|
| G1 | Compare WASM allocators and select optimal one | Benchmarked comparison of dlmalloc, talc, lol_alloc with binary size and allocation throughput data |
| G2 | Integrate `wasm-opt` into build pipeline | Automated post-build optimization with `-Oz` passes; validated output |
| G3 | Establish binary size analysis tooling | `twiggy` profiling scripts identify top-20 size contributors |
| G4 | Enforce size budget in CI | GitHub Actions step fails PR if WASM binary exceeds threshold |
| G5 | Profile memory allocation patterns | Tracing allocator wrapper reports allocation counts by size class |
| G6 | Profile heap usage and fragmentation | Measurement of peak allocation, live bytes, and fragmentation ratio |
| G7 | Optimize WASM startup/instantiation time | Benchmark with custom harness; identify and eliminate slow initialization paths |
| G8 | Ensure WASM builds exclude unnecessary features | Verify `channels`, `services`, `native-exec`, `vector-memory` are not compiled into WASM |
| G9 | Tune build profiles for minimum WASM size | LTO, codegen-units, opt-level, strip, panic=abort verified and documented |

### 1.3 Success Metrics

| Metric | Target | How Measured |
|--------|--------|-------------|
| WASM binary size (post wasm-opt, uncompressed) | <= 300 KB | `wc -c clawft_wasm.opt.wasm` |
| WASM binary size (gzipped) | <= 120 KB | `gzip -9 -c clawft_wasm.opt.wasm \| wc -c` |
| Allocator binary overhead | Documented delta per allocator | Build each, compare `wc -c` |
| `wasm-opt` size reduction | >= 5% from pre-opt | Script output comparison |
| CI size gate | Blocks PRs exceeding budget | GitHub Actions check status |
| Allocation tracing functional | Reports size-class distribution | Custom allocator wrapper output |
| WASM instantiation time | Baselined (target: < 5ms cold) | Wasmtime benchmark harness |
| Test suite regression | 0 failures | `cargo test --workspace` |
| Clippy clean | 0 warnings | `cargo clippy --workspace -- -D warnings` |
| Feature exclusion verified | No banned symbols in WASM binary | `twiggy` / `wasm-objdump` check |

### 1.4 Non-Goals

These items are explicitly out of scope for Phase 3E:

- **Real WASI HTTP/FS implementations** -- That is Phase 3D
- **Switching the default allocator without data** -- 3E compares allocators and makes a recommendation; the actual switch happens after benchmarks are reviewed
- **Arena/pool allocator for serde_json hot paths** -- Designed here as a pattern; implementation deferred to when the real agent loop exists (Phase 3F+)
- **Docker image optimization** -- That is Phase 3D
- **musl static binary optimization** -- Nice-to-have; not gating
- **PGO execution and benchmarking** -- PGO build script is created; execution is optional

### 1.5 Functional Requirements

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-01 | Compare dlmalloc, talc, and lol_alloc for WASM allocator | MUST | Benchmark report: binary size (KB), code overhead, fragmentation notes for each |
| FR-02 | Feature-gated allocator selection in `allocator.rs` | MUST | `--features alloc-talc` or `--features alloc-lol` selects alternative; default remains dlmalloc |
| FR-03 | Integrate `wasm-opt -Oz` into build pipeline | MUST | `scripts/build/wasm-opt.sh` runs; validates output with `wasmtime compile` |
| FR-04 | Profile WASM binary with `twiggy` top/dominators/monos | MUST | `scripts/bench/wasm-twiggy.sh` generates report; top-20 contributors identified |
| FR-05 | CI size gate for WASM binary | MUST | GitHub Actions step checks uncompressed (<= 300 KB) and gzipped (<= 120 KB) |
| FR-06 | Verify LTO + dead code elimination for WASM | MUST | `twiggy` shows no large symbols from prunable code; `profile.release-wasm` confirmed |
| FR-07 | Allocation tracing wrapper (`alloc-tracing` feature) | SHOULD | Custom `GlobalAlloc` wrapper counts by size class; exportable stats via JSON |
| FR-08 | WASM memory usage profiling harness | SHOULD | Script measures linear memory layout; data segments; initial footprint |
| FR-09 | WASM instantiation time benchmark | SHOULD | Harness measures cold-start and warm-start time; baseline recorded |
| FR-10 | Verify feature exclusion for WASM target | MUST | No `channels`, `services`, `native-exec`, `vector-memory` code in binary |
| FR-11 | Extend benchmark baseline with WASM metrics | MUST | `baseline.json` gains `wasm_size_kb`, `wasm_gzip_kb`, `wasm_instantiation_ms` |
| FR-12 | PGO build script for native binary | COULD | `scripts/build/pgo-build.sh` exists; execution optional |
| FR-13 | Document `eh_frame` reduction experiment | COULD | Test `-C force-unwind-tables=no`; document tradeoffs |
| FR-14 | Thin LTO vs fat LTO comparison | COULD | Build time and binary size data |

### 1.6 Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-01 | WASM binary size (uncompressed, post wasm-opt) | <= 300 KB |
| NFR-02 | WASM binary size (gzipped) | <= 120 KB |
| NFR-03 | WASM instantiation time (Wasmtime, cold) | Baselined; target < 5 ms |
| NFR-04 | WASM instantiation time (Wasmtime, warm/cached) | Baselined; target < 1 ms |
| NFR-05 | Allocator overhead (10K mixed allocs) | < 5% of total linear memory wasted to fragmentation |
| NFR-06 | No regression in existing test suite | 100% pass (all crates) |
| NFR-07 | Build time (release-wasm, clean) | Baselined; no more than 20% regression from changes |
| NFR-08 | Zero clippy warnings | `cargo clippy --workspace -- -D warnings` |

### 1.7 Constraints

| ID | Constraint | Rationale |
|----|------------|-----------|
| C-01 | Rust toolchain pinned to 1.93.x | `rust-toolchain.toml` channel = "1.93" |
| C-02 | Primary WASM target: `wasm32-wasip2` | Rust 1.93 has Tier 2 wasip2 support |
| C-03 | Fallback WASM target: `wasm32-wasip1` | WAMR only supports preview 1 |
| C-04 | `clawft-wasm` depends only on `clawft-types`, `serde`, `serde_json`, allocator | No new heavy deps |
| C-05 | `opt-level = "z"` for WASM release profile | Size is the optimization priority |
| C-06 | `panic = "abort"` for all release profiles | No unwinding overhead |
| C-07 | No `unsafe` without documented safety justification | Allocator wrappers only exception |
| C-08 | Feature flags for allocator selection, not runtime dispatch | Compile-time only |
| C-09 | All scripts use `set -euo pipefail` | Fail-fast error handling |
| C-10 | `clawft-types` transitives (`chrono`, `uuid`, `dirs`) must be profiled for WASM impact | Known weight risk |

### 1.8 Dependency Weight Analysis

The `clawft-types` crate depends on `chrono`, `uuid`, `dirs`, and `thiserror`. These
transitively affect WASM binary size:

```
clawft-wasm
  +-- clawft-types
  |     +-- serde (derive)           ~  workspace
  |     +-- serde_json               ~  workspace
  |     +-- chrono (serde feature)   ~  time parsing, formatting (~15-25 KB in WASM)
  |     +-- thiserror                ~  error derive macro (zero runtime cost)
  |     +-- uuid (v4, serde)         ~  random UUID generation (pulls getrandom/rand)
  |     +-- dirs                     ~  home directory detection (OS APIs)
  +-- serde (derive)                 ~  workspace
  +-- serde_json                     ~  workspace (JSON parser ~60-80 KB in WASM)
  +-- dlmalloc (wasm32 only)         ~  allocator (~2-10 KB code)
```

**Known risks**:
- `serde_json` is the single largest dependency (~60-80 KB in optimized WASM). Required; no alternative.
- `chrono` formatting code adds ~15-25 KB. If unused by WASM paths, it can be feature-gated.
- `uuid` v4 pulls `getrandom`/`rand` which may not compile for WASM without an adapter.
- `dirs` uses OS APIs that do not exist on WASM; should be behind `#[cfg(not(target_arch = "wasm32"))]`.

This analysis is a key input to the `twiggy` profiling work.

### 1.9 Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Alternative allocator causes memory bugs | HIGH | LOW | Keep dlmalloc default; feature-gate alternatives; stress test |
| `wasm-opt` produces invalid module | MEDIUM | LOW | Validate with `wasmtime compile`; keep pre-opt binary |
| `chrono`/`uuid`/`dirs` inflate WASM unexpectedly | HIGH | MEDIUM | Profile with `twiggy`; feature-gate in `clawft-types` if needed |
| `serde_json` dominates binary (60-80 KB) | MEDIUM | HIGH | Expected; document as baseline; explore `miniserde` for future |
| `wasm32-wasip2` target not in Rust 1.93 | HIGH | LOW | Fallback to `wasm32-wasip1`; verify with `rustup target list` |
| CI size gate too strict during 3D development | MEDIUM | MEDIUM | Parameterized thresholds; raise when 3D ships HTTP/FS |
| `getrandom` (via uuid v4) fails on WASM | HIGH | MEDIUM | May need WASI feature flag or stub UUID on WASM |
| lol_alloc unsuitable for long-running agent | LOW | HIGH | Expected; only recommend for per-invocation patterns |

### 1.10 Use Cases

**UC-01: Developer selects WASM allocator for comparison**
1. Developer runs `cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm` (default: dlmalloc)
2. Developer runs same with `--features alloc-talc` (talc allocator)
3. Developer runs same with `--features alloc-lol` (lol_alloc)
4. Developer compares binary sizes with `scripts/bench/alloc-compare.sh`
5. Developer records findings in comparison matrix

**UC-02: CI validates WASM binary size on every PR**
1. PR triggers `wasm-build.yml` workflow
2. Workflow builds `clawft-wasm` with `--profile release-wasm --target wasm32-wasip2`
3. Workflow runs `wasm-opt -Oz` on the output
4. Workflow runs `scripts/bench/wasm-size-gate.sh` (checks 300 KB / 120 KB limits)
5. Workflow runs `scripts/bench/wasm-twiggy.sh` and uploads report as artifact
6. Workflow fails if size thresholds exceeded; posts comment on PR

**UC-03: Developer profiles allocation patterns**
1. Developer enables `alloc-tracing` feature
2. Custom `GlobalAlloc` wrapper counts allocations by size class
3. Stats exported via `alloc_trace::stats_json()` function
4. Developer identifies hot allocation paths (e.g., `serde_json::from_str`)

**UC-04: Developer measures WASM instantiation time**
1. Developer runs `scripts/bench/wasm-startup.sh`
2. Script measures cold instantiation (module compile + instantiate)
3. Script measures warm instantiation (pre-compiled module)
4. Results recorded in `baseline.json`

---

## 2. Pseudocode

### 2.1 Feature-Gated Allocator Selection

```rust
// File: crates/clawft-wasm/src/allocator.rs
//
// Feature-gated allocator selection for WASM targets.
// Only one allocator is active at compile time.
// Default: dlmalloc (proven in Phase 3A, well-tested for WASM).
//
// Selection rules:
//   - No features:              dlmalloc (default)
//   - --features alloc-talc:    talc (Rust-native, fast, ~1-2 KB code)
//   - --features alloc-lol:     lol_alloc (tiny code, never frees, ~200 bytes)
//   - --features alloc-tracing: wraps dlmalloc with counting layer

//! WASM global allocator configuration.
//!
//! Uses feature flags to select between allocators for benchmarking.
//! Only one allocator can be active at a time. The default is `dlmalloc`,
//! which is lightweight and well-tested for WebAssembly.
//!
//! # Feature Flags
//!
//! - `alloc-talc`: Use the `talc` allocator (fast, Rust-native, small code)
//! - `alloc-lol`: Use `lol_alloc` (minimal code, never frees -- short-lived modules only)
//! - `alloc-tracing`: Wrap the active allocator with allocation counting
//!
//! # Safety
//!
//! The `#[global_allocator]` attribute requires `GlobalAlloc`. All three allocators
//! are well-known WASM allocators with documented safety properties.

// ──────────────────────────────────────────────────────────────
// Option A: dlmalloc (current default)
// Proven, well-tested. ~2-10 KB code depending on optimization.
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    not(feature = "alloc-talc"),
    not(feature = "alloc-lol"),
    not(feature = "alloc-tracing"),
))]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// ──────────────────────────────────────────────────────────────
// Option B: talc (Rust-native, fast, ~1-2 KB code footprint)
//
// talc uses ClaimOnOom to grow from WASM linear memory on demand.
// On wasm32 (single-threaded), locking is effectively a no-op.
// Smaller minimum chunk size (3 * usize) than dlmalloc.
//
// See: https://github.com/SFBdragon/talc
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    feature = "alloc-talc",
    not(feature = "alloc-tracing"),
))]
#[global_allocator]
static ALLOC: talc::Talck<talc::locking::AssumeUnlockable, talc::ClaimOnOom> =
    talc::Talc::new(unsafe {
        // SAFETY: ClaimOnOom::new(Span::empty()) creates an allocator that
        // claims WASM linear memory pages on first allocation. The empty span
        // means no pre-reserved memory; all memory comes from memory.grow.
        // This is the standard pattern for WASM allocators.
        talc::ClaimOnOom::new(talc::Span::empty())
    })
    .lock();

// ──────────────────────────────────────────────────────────────
// Option C: lol_alloc (minimal code footprint, ~200 bytes)
//
// lol_alloc NEVER frees memory. It is a bump allocator that only
// grows. Suitable for short-lived WASM modules where the entire
// linear memory is discarded after each invocation.
//
// WARNING: Do NOT use for long-running WASM agents. Memory will
// grow unbounded. Only suitable for request-per-invocation patterns.
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    feature = "alloc-lol",
    not(feature = "alloc-tracing"),
))]
#[global_allocator]
static ALLOC: lol_alloc::AssumeSingleThreaded<lol_alloc::LeakingAllocator> =
    // SAFETY: WASM is single-threaded. AssumeSingleThreaded is the documented
    // pattern for wasm32 targets where no threading exists.
    unsafe { lol_alloc::AssumeSingleThreaded::new(lol_alloc::LeakingAllocator) };

// ──────────────────────────────────────────────────────────────
// Tracing wrapper (enabled with alloc-tracing feature)
// Wraps dlmalloc with allocation counting. See alloc_trace.rs.
// ──────────────────────────────────────────────────────────────
#[cfg(all(
    target_arch = "wasm32",
    feature = "alloc-tracing",
))]
#[global_allocator]
static ALLOC: crate::alloc_trace::TracingAllocator<dlmalloc::GlobalDlmalloc> =
    crate::alloc_trace::TracingAllocator::new(dlmalloc::GlobalDlmalloc);
```

### 2.2 Allocation Tracing Wrapper

```rust
// File: crates/clawft-wasm/src/alloc_trace.rs
//
// Optional allocation-counting wrapper. Enabled with `alloc-tracing` feature.
// Wraps any GlobalAlloc and counts allocations by size class.
// Stats are exported via stats_json() for the WASM host to read.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Size class buckets for allocation tracking.
/// These match common allocation sizes in serde_json workloads:
///   - Small (<=64):   serde_json keys, short strings, Value enum variants
///   - Medium (<=256): typical JSON string values, small objects
///   - Large (<=1024): medium objects, small arrays
///   - XLarge (<=4096): large objects, conversation message arrays
///   - Huge (>4096):   complete JSON documents, large buffers
static ALLOC_SMALL: AtomicUsize = AtomicUsize::new(0);    // <= 64 bytes
static ALLOC_MEDIUM: AtomicUsize = AtomicUsize::new(0);   // <= 256 bytes
static ALLOC_LARGE: AtomicUsize = AtomicUsize::new(0);    // <= 1024 bytes
static ALLOC_XLARGE: AtomicUsize = AtomicUsize::new(0);   // <= 4096 bytes
static ALLOC_HUGE: AtomicUsize = AtomicUsize::new(0);     // > 4096 bytes
static TOTAL_ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_FREED_BYTES: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL_FREE_COUNT: AtomicUsize = AtomicUsize::new(0);
static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Allocation-counting wrapper around any `GlobalAlloc`.
///
/// Tracks allocations by size class and total bytes allocated/freed.
/// Use with the `alloc-tracing` feature flag.
///
/// # Safety
///
/// This wrapper delegates all actual allocation to the inner allocator.
/// The counting uses `Relaxed` ordering which is safe for single-threaded
/// WASM (no data races possible) and provides minimal overhead.
pub struct TracingAllocator<A: GlobalAlloc> {
    inner: A,
}

impl<A: GlobalAlloc> TracingAllocator<A> {
    /// Create a new tracing wrapper around the given allocator.
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TracingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();

        // Classify by size bucket
        match size {
            0..=64 => { ALLOC_SMALL.fetch_add(1, Ordering::Relaxed); }
            65..=256 => { ALLOC_MEDIUM.fetch_add(1, Ordering::Relaxed); }
            257..=1024 => { ALLOC_LARGE.fetch_add(1, Ordering::Relaxed); }
            1025..=4096 => { ALLOC_XLARGE.fetch_add(1, Ordering::Relaxed); }
            _ => { ALLOC_HUGE.fetch_add(1, Ordering::Relaxed); }
        }

        TOTAL_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        let new_total = TOTAL_ALLOC_BYTES.fetch_add(size, Ordering::Relaxed) + size;

        // Track peak live bytes (alloc - freed high-water mark)
        let freed = TOTAL_FREED_BYTES.load(Ordering::Relaxed);
        let live = new_total.saturating_sub(freed);
        let mut current_peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
        while live > current_peak {
            match PEAK_LIVE_BYTES.compare_exchange_weak(
                current_peak,
                live,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_peak = actual,
            }
        }

        // SAFETY: caller guarantees layout is valid (non-zero size, valid alignment)
        unsafe { self.inner.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TOTAL_FREE_COUNT.fetch_add(1, Ordering::Relaxed);
        TOTAL_FREED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);

        // SAFETY: caller guarantees ptr was allocated by this allocator with this layout
        unsafe { self.inner.dealloc(ptr, layout) }
    }
}

/// Allocation statistics snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AllocStats {
    pub small_count: usize,       // <= 64 bytes
    pub medium_count: usize,      // <= 256 bytes
    pub large_count: usize,       // <= 1024 bytes
    pub xlarge_count: usize,      // <= 4096 bytes
    pub huge_count: usize,        // > 4096 bytes
    pub total_alloc_count: usize,
    pub total_free_count: usize,
    pub total_alloc_bytes: usize,
    pub total_freed_bytes: usize,
    pub live_bytes: usize,        // alloc - freed
    pub peak_live_bytes: usize,
    pub fragmentation_ratio: f64, // 1.0 - (live / peak), lower is better
}

/// Read current allocation statistics.
pub fn read_stats() -> AllocStats {
    let total_alloc = TOTAL_ALLOC_BYTES.load(Ordering::Relaxed);
    let total_freed = TOTAL_FREED_BYTES.load(Ordering::Relaxed);
    let live = total_alloc.saturating_sub(total_freed);
    let peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);

    let frag = if peak > 0 {
        1.0 - (live as f64 / peak as f64)
    } else {
        0.0
    };

    AllocStats {
        small_count: ALLOC_SMALL.load(Ordering::Relaxed),
        medium_count: ALLOC_MEDIUM.load(Ordering::Relaxed),
        large_count: ALLOC_LARGE.load(Ordering::Relaxed),
        xlarge_count: ALLOC_XLARGE.load(Ordering::Relaxed),
        huge_count: ALLOC_HUGE.load(Ordering::Relaxed),
        total_alloc_count: TOTAL_ALLOC_COUNT.load(Ordering::Relaxed),
        total_free_count: TOTAL_FREE_COUNT.load(Ordering::Relaxed),
        total_alloc_bytes: total_alloc,
        total_freed_bytes: total_freed,
        live_bytes: live,
        peak_live_bytes: peak,
        fragmentation_ratio: frag,
    }
}

/// Reset all counters to zero.
pub fn reset_stats() {
    ALLOC_SMALL.store(0, Ordering::Relaxed);
    ALLOC_MEDIUM.store(0, Ordering::Relaxed);
    ALLOC_LARGE.store(0, Ordering::Relaxed);
    ALLOC_XLARGE.store(0, Ordering::Relaxed);
    ALLOC_HUGE.store(0, Ordering::Relaxed);
    TOTAL_ALLOC_COUNT.store(0, Ordering::Relaxed);
    TOTAL_FREE_COUNT.store(0, Ordering::Relaxed);
    TOTAL_ALLOC_BYTES.store(0, Ordering::Relaxed);
    TOTAL_FREED_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
}

/// Export stats as JSON string.
pub fn stats_json() -> String {
    serde_json::to_string_pretty(&read_stats()).unwrap_or_else(|_| "{}".to_string())
}
```

### 2.3 Cargo.toml Feature Configuration

```toml
# File: crates/clawft-wasm/Cargo.toml
#
# Additions to the existing Cargo.toml.

[features]
default = []
alloc-talc = ["dep:talc"]
alloc-lol = ["dep:lol_alloc"]
alloc-tracing = []

# NOTE: alloc-talc and alloc-lol are mutually exclusive.
# Enabling both causes a compile error (two #[global_allocator] definitions).
# alloc-tracing wraps dlmalloc by default.

[target.'cfg(target_arch = "wasm32")'.dependencies]
dlmalloc = { version = "0.2", features = ["global"] }
talc = { version = "4", optional = true }
lol_alloc = { version = "0.4", optional = true }
```

### 2.4 Release Profile Tuning

```toml
# File: Cargo.toml (workspace root)
#
# Current profile.release is already well-configured:
#   opt-level = "z", lto = true, strip = true, codegen-units = 1, panic = "abort"
#
# Additions for comparison/benchmarking:

[profile.release-wasm]
inherits = "release"
opt-level = "z"       # Explicit for documentation

# PGO iteration profile (native, faster rebuilds)
[profile.release-pgo]
inherits = "release"
lto = "thin"
codegen-units = 1

# Thin LTO comparison profile
[profile.release-thin]
inherits = "release"
lto = "thin"
```

### 2.5 Cargo Config for WASM Targets

```toml
# File: .cargo/config.toml
#
# Updated from current content to add wasip2 support and aliases.

[target.wasm32-wasip2]
rustflags = ["-C", "opt-level=z"]
runner = "wasmtime"

[target.wasm32-wasip1]
rustflags = ["-C", "opt-level=z"]
runner = "wasmtime"

[alias]
wasm = "build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm"
wasm-p1 = "build --target wasm32-wasip1 --profile release-wasm -p clawft-wasm"
wasm-check = "check --target wasm32-wasip2 -p clawft-wasm"
```

### 2.6 wasm-opt Integration Script

```bash
#!/usr/bin/env bash
# File: scripts/build/wasm-opt.sh
#
# Run wasm-opt -Oz on the clawft-wasm binary.
# Assumes cargo build has already been run.
#
# Usage:
#   scripts/build/wasm-opt.sh [TARGET] [PROFILE]
#   scripts/build/wasm-opt.sh                          # defaults: wasm32-wasip2, release-wasm
#   scripts/build/wasm-opt.sh wasm32-wasip1             # wasip1 fallback
#
# Passes applied:
#   -Oz                     Size-optimize all functions
#   --strip-debug           Remove DWARF debug sections
#   --strip-dwarf           Remove DWARF info
#   --strip-producers       Remove producers section (compiler metadata)
#   --zero-filled-memory    Optimize zero-initialized data segments
#   --converge              Re-run passes until output stabilizes

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="${1:-wasm32-wasip2}"
PROFILE="${2:-release-wasm}"

INPUT="$WORKSPACE_ROOT/target/$TARGET/$PROFILE/clawft_wasm.wasm"
OUTPUT="$WORKSPACE_ROOT/target/$TARGET/$PROFILE/clawft_wasm.opt.wasm"

# ── Verify input ──────────────────────────────────────────────
if [ ! -f "$INPUT" ]; then
    echo "ERROR: WASM binary not found: $INPUT"
    echo ""
    echo "Build first:"
    echo "  cargo build -p clawft-wasm --target $TARGET --profile $PROFILE"
    exit 1
fi

# ── Check wasm-opt is available ──────────────────────────────
if ! command -v wasm-opt &>/dev/null; then
    echo "ERROR: wasm-opt not found."
    echo ""
    echo "Install binaryen:"
    echo "  apt-get install binaryen        # Debian/Ubuntu"
    echo "  brew install binaryen           # macOS"
    echo "  cargo install wasm-opt          # From source"
    exit 1
fi

# ── Record pre-optimization size ─────────────────────────────
PRE_SIZE=$(stat -c%s "$INPUT" 2>/dev/null || stat -f%z "$INPUT")

# ── Run wasm-opt ─────────────────────────────────────────────
wasm-opt -Oz \
    --strip-debug \
    --strip-dwarf \
    --strip-producers \
    --zero-filled-memory \
    --converge \
    -o "$OUTPUT" \
    "$INPUT"

# ── Record post-optimization size ────────────────────────────
POST_SIZE=$(stat -c%s "$OUTPUT" 2>/dev/null || stat -f%z "$OUTPUT")

# ── Calculate gzipped size ───────────────────────────────────
GZIP_SIZE=$(gzip -9 -c "$OUTPUT" | wc -c)

# ── Calculate reduction ──────────────────────────────────────
REDUCTION=$((PRE_SIZE - POST_SIZE))
if [ "$PRE_SIZE" -gt 0 ]; then
    PCT=$(awk "BEGIN { printf \"%.1f\", $REDUCTION * 100 / $PRE_SIZE }")
else
    PCT="0.0"
fi

echo "=== wasm-opt Results ==="
echo "Target:    $TARGET"
echo "Profile:   $PROFILE"
echo "Input:     $INPUT ($(( PRE_SIZE / 1024 )) KB)"
echo "Output:    $OUTPUT ($(( POST_SIZE / 1024 )) KB)"
echo "Gzipped:   $(( GZIP_SIZE / 1024 )) KB"
echo "Reduction: $(( REDUCTION / 1024 )) KB ($PCT%)"

# ── Validate with wasmtime ───────────────────────────────────
if command -v wasmtime &>/dev/null; then
    echo ""
    echo "Validating with wasmtime..."
    if wasmtime compile "$OUTPUT" -o /dev/null 2>/dev/null; then
        echo "PASS: Optimized WASM module is valid"
    else
        echo "FAIL: Optimized WASM module failed validation!"
        echo "Keeping pre-opt binary as fallback."
        exit 1
    fi
else
    echo ""
    echo "WARN: wasmtime not found; skipping validation"
fi

# ── Machine-readable summary ─────────────────────────────────
echo ""
echo "--- JSON ---"
echo "{\"pre_bytes\":$PRE_SIZE,\"post_bytes\":$POST_SIZE,\"gzip_bytes\":$GZIP_SIZE,\"reduction_pct\":$PCT}"
```

### 2.7 twiggy Size Profiling Script

```bash
#!/usr/bin/env bash
# File: scripts/bench/wasm-twiggy.sh
#
# Profile WASM binary with twiggy: top contributors, dominators, monomorphizations.
#
# Usage:
#   scripts/bench/wasm-twiggy.sh [WASM_FILE] [TOP_N]

set -euo pipefail

WASM_FILE="${1:-target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"
TOP_N="${2:-20}"

if [ ! -f "$WASM_FILE" ]; then
    echo "WASM file not found: $WASM_FILE"
    echo "Build with: cargo wasm"
    exit 1
fi

if ! command -v twiggy &>/dev/null; then
    echo "twiggy not found. Install: cargo install twiggy"
    exit 1
fi

SIZE_BYTES=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")

echo "=== WASM Size Profile (twiggy) ==="
echo "File: $WASM_FILE"
echo "Size: $(( SIZE_BYTES / 1024 )) KB ($SIZE_BYTES bytes)"
echo ""

echo "--- Top $TOP_N Size Contributors ---"
twiggy top -n "$TOP_N" "$WASM_FILE"
echo ""

echo "--- Dominators (what keeps what alive) ---"
twiggy dominators "$WASM_FILE" | head -40
echo ""

echo "--- Monomorphizations (generic bloat from serde, etc.) ---"
twiggy monos "$WASM_FILE" | head -30
echo ""

echo "--- Sections ---"
if command -v wasm-objdump &>/dev/null; then
    wasm-objdump -h "$WASM_FILE"
else
    echo "(wasm-objdump not available; install wabt for section analysis)"
fi

echo ""
echo "=== Analysis Complete ==="
echo ""
echo "Key things to look for:"
echo "  1. serde_json functions dominating top contributors (expected, ~60-80 KB)"
echo "  2. Unexpected monomorphizations from generic code in clawft-types"
echo "  3. chrono/uuid/dirs code that should not be in WASM binary"
echo "  4. Allocator code size (dlmalloc: ~2-10 KB, talc: ~1-2 KB, lol_alloc: ~200 B)"
```

### 2.8 CI Size Gate Script

```bash
#!/usr/bin/env bash
# File: scripts/bench/wasm-size-gate.sh
#
# CI size gate: fails if WASM binary exceeds thresholds.
# Called from GitHub Actions after wasm-opt runs.
#
# Usage:
#   scripts/bench/wasm-size-gate.sh [WASM_FILE] [MAX_KB] [MAX_GZIP_KB]
#
# Exit codes:
#   0 = PASS (within budget)
#   1 = FAIL (over budget)

set -euo pipefail

WASM_FILE="${1:-target/wasm32-wasip2/release-wasm/clawft_wasm.opt.wasm}"
MAX_KB="${2:-300}"
MAX_GZIP_KB="${3:-120}"

if [ ! -f "$WASM_FILE" ]; then
    echo "FAIL: WASM file not found: $WASM_FILE"
    exit 1
fi

SIZE_BYTES=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
SIZE_KB=$(( SIZE_BYTES / 1024 ))

GZIP_BYTES=$(gzip -9 -c "$WASM_FILE" | wc -c)
GZIP_KB=$(( GZIP_BYTES / 1024 ))

echo "=== WASM Size Gate ==="
echo "Binary:     $WASM_FILE"
echo "Size:       $SIZE_KB KB (limit: $MAX_KB KB)"
echo "Gzipped:    $GZIP_KB KB (limit: $MAX_GZIP_KB KB)"

PASS=true

if [ "$SIZE_KB" -gt "$MAX_KB" ]; then
    echo ""
    echo "FAIL: Uncompressed size $SIZE_KB KB exceeds limit of $MAX_KB KB"
    echo "  Over by $(( SIZE_KB - MAX_KB )) KB"
    echo ""
    echo "  To investigate:"
    echo "    cargo install twiggy"
    echo "    twiggy top -n 20 $WASM_FILE"
    PASS=false
fi

if [ "$GZIP_KB" -gt "$MAX_GZIP_KB" ]; then
    echo ""
    echo "FAIL: Gzipped size $GZIP_KB KB exceeds limit of $MAX_GZIP_KB KB"
    echo "  Over by $(( GZIP_KB - MAX_GZIP_KB )) KB"
    PASS=false
fi

echo ""
if [ "$PASS" = true ]; then
    echo "RESULT: PASS"
    echo "{\"status\":\"pass\",\"size_kb\":$SIZE_KB,\"gzip_kb\":$GZIP_KB}"
    exit 0
else
    echo "RESULT: FAIL"
    echo "{\"status\":\"fail\",\"size_kb\":$SIZE_KB,\"gzip_kb\":$GZIP_KB}"
    exit 1
fi
```

### 2.9 Allocator Comparison Script

```bash
#!/usr/bin/env bash
# File: scripts/bench/alloc-compare.sh
#
# Build clawft-wasm with each allocator and compare binary sizes.
#
# Usage:
#   scripts/bench/alloc-compare.sh [--wasm-opt]

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="wasm32-wasip2"
PROFILE="release-wasm"
APPLY_WASM_OPT=false

if [ "${1:-}" = "--wasm-opt" ]; then
    APPLY_WASM_OPT=true
fi

OUTPUT_DIR="$WORKSPACE_ROOT/target/$TARGET/$PROFILE"

echo "=== Allocator Comparison ==="
echo "Target:   $TARGET"
echo "Profile:  $PROFILE"
echo "wasm-opt: $APPLY_WASM_OPT"
echo ""
printf "%-12s  %8s  %8s  %8s\n" "Allocator" "Raw (KB)" "Opt (KB)" "Gzip (KB)"
printf "%-12s  %8s  %8s  %8s\n" "---------" "--------" "--------" "---------"

for ALLOC_NAME in dlmalloc talc lol_alloc; do
    case "$ALLOC_NAME" in
        dlmalloc)  FLAGS="" ;;
        talc)      FLAGS="--features alloc-talc" ;;
        lol_alloc) FLAGS="--features alloc-lol" ;;
    esac

    # Build
    # shellcheck disable=SC2086
    cargo build -p clawft-wasm \
        --target "$TARGET" \
        --profile "$PROFILE" \
        $FLAGS \
        2>/dev/null

    RAW_FILE="$OUTPUT_DIR/clawft_wasm.wasm"
    RAW_BYTES=$(stat -c%s "$RAW_FILE" 2>/dev/null || stat -f%z "$RAW_FILE")
    RAW_KB=$(( RAW_BYTES / 1024 ))

    OPT_KB="-"
    GZIP_KB="-"

    if [ "$APPLY_WASM_OPT" = true ] && command -v wasm-opt &>/dev/null; then
        OPT_FILE="$OUTPUT_DIR/clawft_wasm.opt.wasm"
        wasm-opt -Oz \
            --strip-debug --strip-dwarf --strip-producers \
            --zero-filled-memory --converge \
            -o "$OPT_FILE" "$RAW_FILE" 2>/dev/null
        OPT_BYTES=$(stat -c%s "$OPT_FILE" 2>/dev/null || stat -f%z "$OPT_FILE")
        OPT_KB=$(( OPT_BYTES / 1024 ))
        GZIP_BYTES=$(gzip -9 -c "$OPT_FILE" | wc -c)
        GZIP_KB=$(( GZIP_BYTES / 1024 ))
    fi

    printf "%-12s  %8s  %8s  %8s\n" "$ALLOC_NAME" "$RAW_KB" "$OPT_KB" "$GZIP_KB"
done

echo ""
echo "=== Comparison Complete ==="
```

### 2.10 WASM Instantiation Time Benchmark

```bash
#!/usr/bin/env bash
# File: scripts/bench/wasm-startup.sh
#
# Measure WASM module instantiation time using wasmtime.
# Two modes:
#   1. Cold start: compile + instantiate (first load)
#   2. Warm start: instantiate from pre-compiled module
#
# Usage:
#   scripts/bench/wasm-startup.sh [WASM_FILE] [ITERATIONS]

set -euo pipefail

WASM_FILE="${1:-target/wasm32-wasip2/release-wasm/clawft_wasm.opt.wasm}"
ITERATIONS="${2:-100}"

if [ ! -f "$WASM_FILE" ]; then
    echo "WASM file not found: $WASM_FILE"
    exit 1
fi

if ! command -v wasmtime &>/dev/null; then
    echo "wasmtime not found. Install from https://wasmtime.dev"
    exit 1
fi

echo "=== WASM Instantiation Benchmark ==="
echo "File:       $WASM_FILE"
echo "Iterations: $ITERATIONS"
echo ""

# ── Cold start: compile module from .wasm bytes ─────────────
echo "--- Cold Start (compile + instantiate) ---"
COLD_TOTAL_NS=0
for i in $(seq 1 "$ITERATIONS"); do
    START=$(date +%s%N)
    wasmtime compile "$WASM_FILE" -o /dev/null 2>/dev/null
    END=$(date +%s%N)
    COLD_TOTAL_NS=$((COLD_TOTAL_NS + END - START))
done
COLD_AVG_MS=$(awk "BEGIN { printf \"%.2f\", $COLD_TOTAL_NS / $ITERATIONS / 1000000 }")
echo "  Average: ${COLD_AVG_MS} ms ($ITERATIONS iterations)"

# ── Warm start: pre-compiled module ─────────────────────────
echo ""
echo "--- Warm Start (pre-compiled, instantiate only) ---"
CWASM_FILE="${WASM_FILE%.wasm}.cwasm"
wasmtime compile "$WASM_FILE" -o "$CWASM_FILE" 2>/dev/null

WARM_TOTAL_NS=0
for i in $(seq 1 "$ITERATIONS"); do
    START=$(date +%s%N)
    wasmtime run --allow-precompiled "$CWASM_FILE" 2>/dev/null || true
    END=$(date +%s%N)
    WARM_TOTAL_NS=$((WARM_TOTAL_NS + END - START))
done
WARM_AVG_MS=$(awk "BEGIN { printf \"%.2f\", $WARM_TOTAL_NS / $ITERATIONS / 1000000 }")
echo "  Average: ${WARM_AVG_MS} ms ($ITERATIONS iterations)"

rm -f "$CWASM_FILE"

echo ""
echo "--- JSON ---"
echo "{\"cold_avg_ms\":$COLD_AVG_MS,\"warm_avg_ms\":$WARM_AVG_MS,\"iterations\":$ITERATIONS}"
```

### 2.11 WASM Memory Profiling Harness

```bash
#!/usr/bin/env bash
# File: scripts/bench/wasm-memory.sh
#
# Analyze WASM module memory layout: sections, data segments, linear memory limits.
#
# Usage:
#   scripts/bench/wasm-memory.sh [WASM_FILE]

set -euo pipefail

WASM_FILE="${1:-target/wasm32-wasip2/release-wasm/clawft_wasm.opt.wasm}"

if [ ! -f "$WASM_FILE" ]; then
    echo "WASM file not found: $WASM_FILE"
    exit 1
fi

SIZE_BYTES=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")

echo "=== WASM Memory Profile ==="
echo "File: $WASM_FILE"
echo "Size: $(( SIZE_BYTES / 1024 )) KB ($SIZE_BYTES bytes)"
echo ""

# Module sections analysis
if command -v wasm-objdump &>/dev/null; then
    echo "--- Module Sections ---"
    wasm-objdump -h "$WASM_FILE"
    echo ""

    echo "--- Memory Section Details ---"
    wasm-objdump --details -j Memory "$WASM_FILE" 2>/dev/null || echo "(no Memory section)"
    echo ""

    echo "--- Data Segments (first 20 lines) ---"
    wasm-objdump --details -j Data "$WASM_FILE" 2>/dev/null | head -20 || echo "(no Data section)"
    echo ""

    echo "--- Exports ---"
    wasm-objdump --details -j Export "$WASM_FILE" 2>/dev/null | head -20 || echo "(no Export section)"
    echo ""
else
    echo "(wasm-objdump not available; install wabt for detailed analysis)"
    echo ""
fi

echo "=== Memory Profile Complete ==="
echo ""
echo "For runtime allocation profiling, build with --features alloc-tracing"
echo "and call alloc_trace::stats_json() after workload."
```

### 2.12 Feature Exclusion Verification

```bash
#!/usr/bin/env bash
# File: scripts/bench/wasm-feature-check.sh
#
# Verify that WASM binary does not contain code from excluded features.
# Checks for symbols indicating channels, services, native-exec, or
# vector-memory code was linked in.
#
# Usage:
#   scripts/bench/wasm-feature-check.sh [WASM_FILE]

set -euo pipefail

WASM_FILE="${1:-target/wasm32-wasip2/release-wasm/clawft_wasm.wasm}"

if [ ! -f "$WASM_FILE" ]; then
    echo "WASM file not found: $WASM_FILE"
    exit 1
fi

echo "=== Feature Exclusion Check ==="
echo "File: $WASM_FILE"
echo ""

# Banned symbol patterns indicating leaked features
BANNED_PATTERNS=(
    "clawft_channels"     # channels crate
    "clawft_services"     # services crate
    "clawft_cli"          # CLI crate
    "clawft_tools"        # tools crate (native exec)
    "tokio"               # async runtime (native only)
    "reqwest"             # HTTP client (native only)
    "exec_shell"          # native shell execution
    "telegram"            # Telegram channel
    "slack"               # Slack channel
    "discord"             # Discord channel
    "vector_memory"       # vector-memory feature
    "hnsw"                # HNSW index
)

FOUND_BANNED=false

for PATTERN in "${BANNED_PATTERNS[@]}"; do
    if command -v wasm-objdump &>/dev/null; then
        MATCHES=$(wasm-objdump -x "$WASM_FILE" 2>/dev/null | grep -ci "$PATTERN" || true)
    else
        MATCHES=$(strings "$WASM_FILE" 2>/dev/null | grep -ci "$PATTERN" || true)
    fi

    if [ "$MATCHES" -gt 0 ]; then
        echo "  FAIL: Found $MATCHES references to '$PATTERN'"
        FOUND_BANNED=true
    else
        echo "  PASS: No references to '$PATTERN'"
    fi
done

echo ""
if [ "$FOUND_BANNED" = true ]; then
    echo "RESULT: FAIL - Banned features found in WASM binary"
    exit 1
else
    echo "RESULT: PASS - No banned features in WASM binary"
    exit 0
fi
```

### 2.13 PGO Build Pipeline (Optional, Native Binary)

```bash
#!/usr/bin/env bash
# File: scripts/build/pgo-build.sh
#
# Profile-Guided Optimization build for the `weft` native binary.
# Steps: instrumented build -> profile workloads -> merge -> optimized build
#
# Prerequisites: rustup component add llvm-tools
# Usage: scripts/build/pgo-build.sh

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PGO_DIR="$WORKSPACE_ROOT/target/pgo-profiles"
MERGED_PROF="$PGO_DIR/merged.profdata"

echo "=== PGO Build Pipeline ==="
echo ""

# Step 1: Instrumented build
echo "[1/4] Building instrumented binary..."
rm -rf "$PGO_DIR"
mkdir -p "$PGO_DIR"

RUSTFLAGS="-C profile-generate=$PGO_DIR" \
    cargo build -p clawft-cli --release \
    --manifest-path "$WORKSPACE_ROOT/Cargo.toml"

INSTRUMENTED="$WORKSPACE_ROOT/target/release/weft"
if [ ! -f "$INSTRUMENTED" ]; then
    echo "ERROR: Instrumented binary not found"
    exit 1
fi

# Step 2: Profiling workloads
echo "[2/4] Running profiling workloads..."

for _ in $(seq 1 50); do
    "$INSTRUMENTED" --version > /dev/null 2>&1 || true
done
echo "  50x --version"

"$INSTRUMENTED" --help > /dev/null 2>&1 || true
"$INSTRUMENTED" agent --help > /dev/null 2>&1 || true
echo "  help text"

"$INSTRUMENTED" config show 2>/dev/null || true
echo "  config show"

PROF_COUNT=$(find "$PGO_DIR" -name "*.profraw" 2>/dev/null | wc -l)
echo "  Profile files: $PROF_COUNT"

if [ "$PROF_COUNT" -eq 0 ]; then
    echo "ERROR: No .profraw files generated"
    exit 1
fi

# Step 3: Merge profiles
echo "[3/4] Merging profiles..."

SYSROOT="$(rustc --print sysroot)"
HOST="$(rustc -vV | grep host | awk '{print $2}')"
LLVM_PROFDATA=""

for candidate in \
    "$SYSROOT/lib/rustlib/$HOST/bin/llvm-profdata" \
    "llvm-profdata"; do
    if command -v "$candidate" &>/dev/null || [ -x "$candidate" ]; then
        LLVM_PROFDATA="$candidate"
        break
    fi
done

if [ -z "$LLVM_PROFDATA" ]; then
    echo "ERROR: llvm-profdata not found. Run: rustup component add llvm-tools"
    exit 1
fi

"$LLVM_PROFDATA" merge -o "$MERGED_PROF" "$PGO_DIR"/*.profraw

# Step 4: PGO-optimized build
echo "[4/4] Building PGO-optimized binary..."

RUSTFLAGS="-C profile-use=$MERGED_PROF -C llvm-args=-pgo-warn-missing-function" \
    cargo build -p clawft-cli --release \
    --manifest-path "$WORKSPACE_ROOT/Cargo.toml"

OPTIMIZED="$WORKSPACE_ROOT/target/release/weft"
OPT_SIZE=$(stat -c%s "$OPTIMIZED" 2>/dev/null || stat -f%z "$OPTIMIZED")

echo ""
echo "=== PGO Build Complete ==="
echo "Binary: $OPTIMIZED"
echo "Size:   $(( OPT_SIZE / 1024 )) KB"
```

### 2.14 Baseline JSON

```json
{
    "version": "3e",
    "timestamp": "2026-02-17T00:00:00Z",
    "toolchain": "1.93",
    "native": {
        "binary_size_kb": 4479,
        "startup_p50_ms": 3.0,
        "startup_p99_ms": 6.0,
        "invocations_per_sec": 420,
        "text_section_kb": 2668,
        "eh_frame_kb": 455,
        "rodata_kb": 705
    },
    "wasm": {
        "target": "wasm32-wasip2",
        "allocator": "dlmalloc",
        "raw_size_kb": null,
        "opt_size_kb": null,
        "gzip_size_kb": null,
        "instantiation_cold_avg_ms": null,
        "instantiation_warm_avg_ms": null
    },
    "thresholds": {
        "wasm_max_size_kb": 300,
        "wasm_max_gzip_kb": 120,
        "native_max_size_kb": 5120,
        "startup_max_p99_ms": 10,
        "regression_pct": 10
    }
}
```

---

## 3. Architecture

### 3.1 Module Layout (Changes)

```
crates/clawft-wasm/
  Cargo.toml                     # MODIFIED: add features, optional allocator deps
  src/
    lib.rs                       # MODIFIED: conditional `mod alloc_trace`
    allocator.rs                 # MODIFIED: feature-gated 3-way allocator selection
    alloc_trace.rs               # NEW: allocation counting wrapper + stats export
    env.rs                       # UNCHANGED
    fs.rs                        # UNCHANGED
    http.rs                      # UNCHANGED
    platform.rs                  # UNCHANGED

Cargo.toml (workspace root)      # MODIFIED: add release-pgo, release-thin profiles
.cargo/config.toml               # MODIFIED: add wasip2 target config, aliases

scripts/
  build/
    wasm-opt.sh                  # NEW: wasm-opt -Oz integration
    pgo-build.sh                 # NEW: PGO pipeline for native binary (optional)
  bench/
    wasm-twiggy.sh               # NEW: twiggy size profiling
    wasm-size-gate.sh            # NEW: CI size budget enforcement
    wasm-startup.sh              # NEW: instantiation time benchmark
    wasm-memory.sh               # NEW: linear memory profiling
    wasm-feature-check.sh        # NEW: feature exclusion verification
    alloc-compare.sh             # NEW: allocator comparison build + size report
    baseline.json                # NEW: benchmark baseline data
```

### 3.2 Crate Dependency Graph (WASM Only)

```
clawft-wasm
  +-- clawft-types            (workspace)
  |     +-- serde 1           (derive)
  |     +-- serde_json 1      (~60-80 KB in WASM; largest single contributor)
  |     +-- chrono 0.4        (serde feature; ~15-25 KB; needs profiling)
  |     +-- thiserror 2       (proc macro only; zero runtime cost)
  |     +-- uuid 1            (v4+serde; pulls getrandom; WASM compat TBD)
  |     +-- dirs 6            (OS APIs; dead code on WASM; should be gated)
  +-- serde 1 (derive)        (workspace)
  +-- serde_json 1            (workspace)
  +-- dlmalloc 0.2            (wasm32, default; ~2-10 KB code)
  +-- talc 4                  (wasm32, optional, alloc-talc; ~1-2 KB code)
  +-- lol_alloc 0.4           (wasm32, optional, alloc-lol; ~200 B code)
```

### 3.3 Build Profile Summary

| Profile | opt-level | LTO | codegen-units | panic | strip | Use Case |
|---------|-----------|-----|---------------|-------|-------|----------|
| `release` | z | fat | 1 | abort | true | Native production binary |
| `release-wasm` | z | fat (inherited) | 1 (inherited) | abort (inherited) | true (inherited) | WASM production binary |
| `release-pgo` | z | thin | 1 | abort | true | PGO iteration (faster rebuilds) |
| `release-thin` | z | thin | 1 | abort | true | LTO comparison benchmark |
| `dev` | 0 | false | 256 (default) | unwind | false | Development iteration |

### 3.4 CI Pipeline Integration

The WASM build CI workflow gains these steps after the existing cargo build:

```yaml
# .github/workflows/wasm-build.yml additions

jobs:
  wasm-build:
    steps:
      # ... existing checkout, toolchain, build steps ...

      - name: Install binaryen (wasm-opt)
        run: sudo apt-get update -qq && sudo apt-get install -y -qq binaryen

      - name: Install twiggy
        run: cargo install twiggy --locked

      - name: Run wasm-opt
        run: bash scripts/build/wasm-opt.sh wasm32-wasip2 release-wasm

      - name: Size gate check
        run: bash scripts/bench/wasm-size-gate.sh

      - name: Feature exclusion check
        run: bash scripts/bench/wasm-feature-check.sh

      - name: twiggy size profile
        run: bash scripts/bench/wasm-twiggy.sh > twiggy-report.txt 2>&1 || true

      - name: Upload twiggy report
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: twiggy-report
          path: twiggy-report.txt

      - name: Comment on PR if over budget
        if: failure()
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            let report = '';
            try { report = fs.readFileSync('twiggy-report.txt', 'utf8'); } catch {}
            if (context.issue && context.issue.number) {
              github.rest.issues.createComment({
                owner: context.repo.owner,
                repo: context.repo.repo,
                issue_number: context.issue.number,
                body: `## WASM Size Budget Exceeded\n\n\`\`\`\n${report.slice(0, 3000)}\n\`\`\``
              });
            }
```

### 3.5 Size Budget Table

| Component | Phase 3E (stubs) | Phase 3D (after HTTP/FS) | Notes |
|-----------|-----------------|-------------------------|-------|
| Raw binary (pre wasm-opt) | <= 350 KB | <= 400 KB | Cargo output |
| Optimized binary (post wasm-opt) | <= 300 KB | <= 350 KB | CI gate threshold |
| Gzipped binary | <= 120 KB | <= 140 KB | CI gate threshold |

Thresholds are parameterized in `wasm-size-gate.sh`. When Phase 3D adds real HTTP/FS code,
the CI step arguments are updated (one-line change).

### 3.6 Allocator Comparison Decision Matrix

| Criterion | Weight | dlmalloc | talc | lol_alloc |
|-----------|--------|----------|------|-----------|
| Binary size overhead | 30% | ~2-10 KB | ~1-2 KB | ~0.2 KB |
| Allocation throughput | 20% | Baseline (1.0x) | ~1.2-1.5x faster | ~2x (no bookkeeping) |
| Fragmentation resistance | 20% | Good | Good (smaller min chunk) | N/A (never frees) |
| Memory reclamation | 15% | Yes | Yes | No (leak-only) |
| Code maturity | 15% | High (LLVM default) | Medium (Rust-native, active) | Low (minimal maintenance) |
| **Weighted score** | 100% | TBD (post-benchmark) | TBD (post-benchmark) | TBD (post-benchmark) |

**Decision criteria:** Select the allocator with highest weighted score.
Default to dlmalloc if scores are within 10% (prefer proven stability).
lol_alloc is only viable for per-invocation lifecycle (module fresh per message).

### 3.7 Recommended clawft-types WASM Feature Gate

If `twiggy` profiling reveals that `dirs` or `uuid` inflate the WASM binary, apply
conditional compilation in `clawft-types`:

```toml
# crates/clawft-types/Cargo.toml (proposed change)
[features]
default = ["native"]
native = ["dep:dirs"]

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
dirs = { workspace = true, optional = true }
```

Then in `clawft-wasm/Cargo.toml`:
```toml
[dependencies]
clawft-types = { workspace = true, default-features = false }
```

This change affects all crates and must be validated with `cargo test --workspace`.

---

## 4. Refinement (TDD Test Plan)

### 4.1 Week 1 (Days 1-4): Allocator Infrastructure, wasm-opt, Size Analysis

#### Day 1-2: Allocator Feature Gates + Tracing Module

**Files to modify:**
- `crates/clawft-wasm/Cargo.toml` -- add features and optional deps
- `crates/clawft-wasm/src/allocator.rs` -- feature-gated 3-way selection
- `crates/clawft-wasm/src/lib.rs` -- add conditional `mod alloc_trace`

**Files to create:**
- `crates/clawft-wasm/src/alloc_trace.rs` -- tracing wrapper

**Verification commands:**
```bash
# Must all succeed
cargo check -p clawft-wasm --target wasm32-wasip2                          # dlmalloc
cargo check -p clawft-wasm --target wasm32-wasip2 --features alloc-talc    # talc
cargo check -p clawft-wasm --target wasm32-wasip2 --features alloc-lol     # lol_alloc
cargo check -p clawft-wasm --target wasm32-wasip2 --features alloc-tracing # tracing
cargo test -p clawft-wasm                                                   # all tests pass
cargo clippy -p clawft-wasm -- -D warnings                                 # zero warnings
```

**Test Cases (allocator.rs):**

```rust
#[cfg(test)]
mod tests {
    // Test: Default allocator works for basic Vec allocation
    #[test]
    fn default_allocator_vec_growth() {
        let mut v: Vec<u8> = Vec::new();
        for i in 0..1000 {
            v.push(i as u8);
        }
        assert_eq!(v.len(), 1000);
    }

    // Test: HashMap allocation (serde_json uses HashMap internally)
    #[test]
    fn hashmap_allocation_pattern() {
        let mut map = std::collections::HashMap::new();
        for i in 0..100 {
            map.insert(format!("key-{i}"), format!("value-{i}"));
        }
        assert_eq!(map.len(), 100);
        for i in 0..50 {
            map.remove(&format!("key-{i}"));
        }
        assert_eq!(map.len(), 50);
    }

    // Test: serde_json round-trip (most common allocation pattern in agent loop)
    #[test]
    fn serde_json_roundtrip() {
        let input = serde_json::json!({
            "role": "user",
            "content": "Hello, world!",
            "metadata": { "timestamp": "2026-02-17T12:00:00Z", "tokens": 42 }
        });
        let serialized = serde_json::to_string(&input).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(input, deserialized);
    }

    // Test: Large conversation allocation (simulates real workload)
    #[test]
    fn large_conversation_allocation() {
        let messages: Vec<serde_json::Value> = (0..200)
            .map(|i| serde_json::json!({
                "role": if i % 2 == 0 { "user" } else { "assistant" },
                "content": format!("Message {i} with padding text for size")
            }))
            .collect();
        let json = serde_json::json!({ "messages": messages });
        let text = serde_json::to_string(&json).unwrap();
        assert!(text.len() > 10_000);
        let _parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    }

    // Test: Fragmentation stress (mixed sizes, reverse dealloc order)
    #[test]
    fn fragmentation_stress() {
        for _ in 0..50 {
            let mut buffers: Vec<Vec<u8>> = Vec::new();
            for &size in &[16, 64, 256, 1024, 4096, 16384] {
                buffers.push(vec![0xAA; size]);
            }
            while let Some(_buf) = buffers.pop() {}
        }
    }
}
```

**Test Cases (alloc_trace.rs):**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_start_at_zero() {
        reset_stats();
        let stats = read_stats();
        assert_eq!(stats.total_alloc_count, 0);
        assert_eq!(stats.total_alloc_bytes, 0);
        assert_eq!(stats.live_bytes, 0);
    }

    #[test]
    fn stats_json_is_valid() {
        reset_stats();
        let json = stats_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total_alloc_count"], 0);
        assert!(parsed["fragmentation_ratio"].as_f64().is_some());
    }

    #[test]
    fn reset_clears_all_counters() {
        // The tracing allocator is only #[global_allocator] on wasm32 with alloc-tracing.
        // On native, this tests the reset mechanism itself.
        reset_stats();
        let stats = read_stats();
        assert_eq!(stats.total_alloc_count, 0);
        assert_eq!(stats.total_alloc_bytes, 0);
        assert_eq!(stats.peak_live_bytes, 0);
    }
}
```

#### Day 2-3: wasm-opt Integration + twiggy Profiling + Size Gate

**Files to create:**
- `scripts/build/wasm-opt.sh`
- `scripts/bench/wasm-twiggy.sh`
- `scripts/bench/wasm-size-gate.sh`
- `scripts/bench/alloc-compare.sh`

**Verification:**
```bash
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm
bash scripts/build/wasm-opt.sh
bash scripts/bench/wasm-size-gate.sh
bash scripts/bench/wasm-twiggy.sh
bash scripts/bench/alloc-compare.sh --wasm-opt
```

**Shell script syntax validation:**
```bash
for script in scripts/build/wasm-opt.sh scripts/bench/wasm-twiggy.sh \
    scripts/bench/wasm-size-gate.sh scripts/bench/alloc-compare.sh; do
    bash -n "$script" || echo "SYNTAX ERROR: $script"
done
```

#### Day 3-4: Config, Profiles, Feature Exclusion, Baseline

**Files to modify:**
- `Cargo.toml` (workspace) -- add `release-pgo`, `release-thin` profiles
- `.cargo/config.toml` -- add `wasm32-wasip2` config, aliases

**Files to create:**
- `scripts/bench/wasm-feature-check.sh`
- `scripts/bench/baseline.json`

**Verification:**
```bash
cargo wasm                                            # alias works (wasip2 + release-wasm)
cargo wasm-check                                      # check alias works
bash scripts/bench/wasm-feature-check.sh              # no banned features
```

**Week 1 Acceptance Criteria:**

- [ ] `clawft-wasm/Cargo.toml` has `alloc-talc`, `alloc-lol`, `alloc-tracing` features
- [ ] `allocator.rs` compiles with each allocator variant
- [ ] `alloc_trace.rs` compiles; stats_json() returns valid JSON
- [ ] `cargo check --target wasm32-wasip2 -p clawft-wasm` passes for default, talc, lol
- [ ] `cargo test -p clawft-wasm` passes (existing + new tests)
- [ ] `scripts/build/wasm-opt.sh` reduces binary size; validates with wasmtime
- [ ] `scripts/bench/wasm-twiggy.sh` produces top-20 contributor report
- [ ] `scripts/bench/wasm-size-gate.sh` enforces 300 KB / 120 KB thresholds
- [ ] `scripts/bench/alloc-compare.sh` produces comparison table
- [ ] `scripts/bench/wasm-feature-check.sh` confirms no banned symbols
- [ ] `.cargo/config.toml` updated with wasip2 target and aliases
- [ ] `baseline.json` created with initial WASM measurements
- [ ] All scripts pass `bash -n` syntax check

### 4.2 Week 2 (Days 5-8): Benchmarking, CI Integration, Optional Native Work

#### Day 5-6: Startup Benchmarks + Memory Profiling

**Files to create:**
- `scripts/bench/wasm-startup.sh`
- `scripts/bench/wasm-memory.sh`

**Verification:**
```bash
bash scripts/bench/wasm-startup.sh         # cold/warm instantiation times
bash scripts/bench/wasm-memory.sh          # linear memory layout
```

#### Day 6-7: CI Pipeline Integration

**Files to modify or create:**
- `.github/workflows/wasm-build.yml` -- add size gate, twiggy, feature check steps

**Verification:**
```bash
# Validate YAML
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/wasm-build.yml'))"

# Dry-run full pipeline locally
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm
bash scripts/build/wasm-opt.sh
bash scripts/bench/wasm-size-gate.sh
bash scripts/bench/wasm-feature-check.sh
bash scripts/bench/wasm-twiggy.sh
```

#### Day 7-8: Optional -- PGO, LTO Comparison, eh_frame

**Optional experiments (COULD priority):**

1. **PGO pipeline**: Create `scripts/build/pgo-build.sh`; run if llvm-tools installed
2. **Thin vs Fat LTO**: Build with `release-thin`, compare binary size and build time
3. **eh_frame reduction**: Build with `RUSTFLAGS="-C force-unwind-tables=no"`, measure

These produce documentation data but do not gate Phase 3E completion.

**Week 2 Acceptance Criteria:**

- [ ] `scripts/bench/wasm-startup.sh` measures cold/warm instantiation times
- [ ] `scripts/bench/wasm-memory.sh` reports linear memory layout
- [ ] `baseline.json` updated with all measured values
- [ ] CI workflow includes wasm-opt, size gate, twiggy, feature check steps
- [ ] CI workflow YAML validates
- [ ] Allocator comparison report produced
- [ ] `cargo test --workspace` passes (no regressions)
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] All shell scripts pass `bash -n` syntax check
- [ ] PGO build script exists (execution optional)

---

## 5. Completion

### 5.1 Deliverables Checklist

#### Code Changes

| File | Path (relative to clawft/) | Type |
|------|---------------------------|------|
| WASM crate Cargo.toml | `crates/clawft-wasm/Cargo.toml` | MODIFIED |
| Allocator module | `crates/clawft-wasm/src/allocator.rs` | MODIFIED |
| Allocation tracer | `crates/clawft-wasm/src/alloc_trace.rs` | NEW |
| WASM lib entrypoint | `crates/clawft-wasm/src/lib.rs` | MODIFIED |
| Workspace Cargo.toml | `Cargo.toml` | MODIFIED |
| Cargo config | `.cargo/config.toml` | MODIFIED |

#### Build Scripts

| File | Path | Type |
|------|------|------|
| wasm-opt integration | `scripts/build/wasm-opt.sh` | NEW |
| PGO build pipeline | `scripts/build/pgo-build.sh` | NEW |

#### Benchmark Scripts

| File | Path | Type |
|------|------|------|
| twiggy profiling | `scripts/bench/wasm-twiggy.sh` | NEW |
| CI size gate | `scripts/bench/wasm-size-gate.sh` | NEW |
| Startup benchmark | `scripts/bench/wasm-startup.sh` | NEW |
| Memory profiling | `scripts/bench/wasm-memory.sh` | NEW |
| Feature exclusion | `scripts/bench/wasm-feature-check.sh` | NEW |
| Allocator comparison | `scripts/bench/alloc-compare.sh` | NEW |
| Baseline data | `scripts/bench/baseline.json` | NEW |

#### CI Workflows

| File | Path | Type |
|------|------|------|
| WASM build + gates | `.github/workflows/wasm-build.yml` | MODIFIED or NEW |

### 5.2 Test Coverage

| Module | Existing | New | Total | Coverage Goal |
|--------|----------|-----|-------|---------------|
| `clawft-wasm/allocator.rs` | 0 | 5 | 5 | 100% of feature paths |
| `clawft-wasm/alloc_trace.rs` | 0 | 3 | 3 | > 80% |
| `clawft-wasm` crate total | 41 | 8+ | 49+ | > 85% |
| Shell scripts (bash -n) | 0 | 9 | 9 | 100% syntax valid |
| Workspace total | existing | +8 | existing+8 | No regressions |

### 5.3 Exit Criteria

#### MUST HAVE (Phase 3E gate -- all required)

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | WASM binary builds for `wasm32-wasip2` with `release-wasm` | `cargo wasm` succeeds |
| 2 | `wasm-opt` runs; produces smaller, valid binary | `scripts/build/wasm-opt.sh` shows reduction; wasmtime validates |
| 3 | WASM binary <= 300 KB (post wasm-opt, uncompressed) | `scripts/bench/wasm-size-gate.sh` exits 0 |
| 4 | WASM binary <= 120 KB (gzipped) | `scripts/bench/wasm-size-gate.sh` exits 0 |
| 5 | Allocator comparison completed (3 allocators) | `scripts/bench/alloc-compare.sh` produces size table |
| 6 | Feature-gated allocator selection works | `cargo check` with each `--features alloc-*` succeeds |
| 7 | Feature exclusion verified | `scripts/bench/wasm-feature-check.sh` exits 0 |
| 8 | No test regressions | `cargo test --workspace` all pass |
| 9 | No clippy warnings | `cargo clippy --workspace -- -D warnings` exits 0 |
| 10 | CI size gate in workflow | `.github/workflows/wasm-build.yml` includes size gate step |
| 11 | All shell scripts valid syntax | `bash -n` passes on all 9 scripts |
| 12 | Baseline established | `scripts/bench/baseline.json` has WASM metrics |

#### SHOULD HAVE

| # | Criterion | Verification |
|---|-----------|-------------|
| 13 | `twiggy` profiling in CI | Workflow uploads twiggy artifact |
| 14 | Allocation tracing module works | `alloc_trace.rs` compiles; `stats_json()` valid JSON |
| 15 | WASM instantiation time baselined | `wasm-startup.sh` completes; values in baseline.json |
| 16 | Linear memory profiling harness | `wasm-memory.sh` produces section/memory report |
| 17 | PGO build script exists | `pgo-build.sh` passes `bash -n` |
| 18 | `wasm32-wasip2` cargo alias works | `cargo wasm` targets wasip2 |

#### NICE TO HAVE

| # | Criterion | Verification |
|---|-----------|-------------|
| 19 | PGO pipeline runs end-to-end | `pgo-build.sh` produces optimized binary |
| 20 | Thin LTO vs Fat LTO documented | Build time + size comparison data |
| 21 | `.eh_frame` reduction documented | force-unwind-tables experiment results |
| 22 | `clawft-types` WASM feature gate proposed | PR or issue for `dirs` conditional compilation |

### 5.4 Risk Register

| # | Risk | Impact | Likelihood | Mitigation | Status |
|---|------|--------|------------|------------|--------|
| R1 | `wasm32-wasip2` target not in Rust 1.93 | HIGH | LOW | Fallback to wasip1; verify `rustup target list` | Open |
| R2 | Alternative allocator causes memory bugs | HIGH | LOW | Stress tests; dlmalloc remains default; feature-gate | Open |
| R3 | `wasm-opt` produces invalid module | MEDIUM | LOW | Validate with `wasmtime compile`; keep pre-opt binary | Open |
| R4 | `serde_json` dominates binary (~60-80 KB) | MEDIUM | HIGH | Expected; document as baseline; no alternative | Accepted |
| R5 | `chrono`/`uuid`/`dirs` inflate WASM | HIGH | MEDIUM | Profile with twiggy; feature-gate if needed | Open |
| R6 | CI gate too strict during 3D dev | MEDIUM | MEDIUM | Parameterized thresholds; raise when 3D ships | Open |
| R7 | `getrandom` (via uuid v4) fails on WASM | HIGH | MEDIUM | May need WASI feature or stub UUID | Open |
| R8 | lol_alloc unsuitable for long-running agent | LOW | HIGH | Document; only for per-invocation | Accepted |
| R9 | Tools not available in CI (binaryen, twiggy) | MEDIUM | LOW | apt-get / cargo install in workflow | Open |

### 5.5 Integration with Other Phases

**Phase 3D (WASI HTTP/FS + Docker) -- Parallel:**
- 3D adds real HTTP/FS to `clawft-wasm`, increasing binary size
- 3E's wasm-opt pipeline, twiggy, and CI size gate protect against uncontrolled growth
- When 3D lands, CI thresholds may need: 300 KB -> 350 KB, 120 KB -> 140 KB
- 3D should use `cargo wasm` alias from 3E

**Phase 3F (Agents & Skills):**
- Agent loop is the primary allocation hot path
- Allocation tracing data from 3E informs memory pool design
- `alloc_trace.rs` should be used during 3F dev to profile real workloads

**Phase 4 (Vector Search / Full Agent):**
- Vector search significantly increases WASM binary size
- twiggy profiling and size gate from 3E are critical guardrails
- Alternative allocator (talc) may become necessary

### 5.6 Handoff Artifacts

1. **Optimized WASM build pipeline**: `cargo wasm` + `wasm-opt.sh` produces size-optimized `.wasm`
2. **CI size guardrail**: Every PR checked against 300 KB / 120 KB thresholds
3. **Allocator comparison data**: Binary size + characteristics for dlmalloc, talc, lol_alloc
4. **Profiling toolkit**: twiggy, allocation tracing, memory profiling, startup benchmarks
5. **Baseline data**: `baseline.json` with initial measurements for regression detection
6. **Feature exclusion gate**: Automated check that banned features stay out of WASM

### 5.7 Recommendations for Follow-Up

1. **If talc is significantly smaller** (>= 1 KB savings): Switch default allocator in follow-up PR.
2. **If PGO shows >= 5% improvement**: Integrate into release pipeline in Phase 4.
3. **If chrono/uuid inflate WASM**: Feature-gate in `clawft-types` (Section 3.7 proposal).
4. **When 3D lands HTTP/FS**: Re-run profiling suite; update CI thresholds; document size delta.
5. **For long-running WASM agents**: Design arena/pool for serde_json hot path using tracing data.

### 5.8 Key File Paths Reference

All paths relative to `clawft/` workspace root:

| Artifact | Path |
|----------|------|
| Workspace Cargo.toml | `Cargo.toml` |
| WASM crate Cargo.toml | `crates/clawft-wasm/Cargo.toml` |
| Allocator module | `crates/clawft-wasm/src/allocator.rs` |
| Allocation tracer | `crates/clawft-wasm/src/alloc_trace.rs` |
| WASM lib.rs | `crates/clawft-wasm/src/lib.rs` |
| Cargo config | `.cargo/config.toml` |
| Rust toolchain | `rust-toolchain.toml` |
| wasm-opt script | `scripts/build/wasm-opt.sh` |
| PGO script | `scripts/build/pgo-build.sh` |
| twiggy profiling | `scripts/bench/wasm-twiggy.sh` |
| CI size gate | `scripts/bench/wasm-size-gate.sh` |
| Startup benchmark | `scripts/bench/wasm-startup.sh` |
| Memory profiling | `scripts/bench/wasm-memory.sh` |
| Feature exclusion | `scripts/bench/wasm-feature-check.sh` |
| Allocator comparison | `scripts/bench/alloc-compare.sh` |
| Baseline data | `scripts/bench/baseline.json` |
| WASM build CI | `.github/workflows/wasm-build.yml` |

---

## Appendix A: Allocator Comparison Methodology

### A.1 Binary Size Measurement

Build each allocator variant and measure raw, post-wasm-opt, and gzipped sizes:

```bash
# dlmalloc (default)
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm
stat -c%s target/wasm32-wasip2/release-wasm/clawft_wasm.wasm

# talc
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm --features alloc-talc
stat -c%s target/wasm32-wasip2/release-wasm/clawft_wasm.wasm

# lol_alloc
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm --features alloc-lol
stat -c%s target/wasm32-wasip2/release-wasm/clawft_wasm.wasm
```

The `alloc-compare.sh` script automates this with optional `--wasm-opt` pass.

### A.2 Allocation Throughput Measurement

Using `alloc-tracing` feature, instrument standard workloads:

1. **Light**: `process_message("Hello")` + `capabilities()`
2. **Medium**: Deserialize 50-message conversation JSON
3. **Heavy**: 100x (deserialize + serialize + drop)

Record: total count, total bytes, peak live bytes, size class distribution.

### A.3 Fragmentation Assessment

Using allocation tracing after heavy workload:
1. `total_alloc_bytes - total_freed_bytes` should be near 0
2. `peak_live_bytes` vs `total_alloc_bytes` ratio indicates peak pressure
3. For lol_alloc: skip (no deallocation; fragmentation is 100%)

### A.4 Expected Results

| Allocator | Binary Delta (vs dlmalloc) | Throughput | Fragmentation | Recommendation |
|-----------|---------------------------|-----------|---------------|----------------|
| dlmalloc | 0 (baseline) | Baseline | Good | Default; proven |
| talc | -0.5 to -8 KB | 1.2-1.5x faster | Good (smaller chunks) | Best alternative |
| lol_alloc | -2 to -10 KB | 2x+ faster | N/A (never frees) | Per-invocation only |

---

## Appendix B: wasm-opt Pass Reference

| Pass | Effect |
|------|--------|
| `-Oz` | Optimize all functions for minimal code size |
| `--strip-debug` | Remove DWARF debug sections |
| `--strip-dwarf` | Remove DWARF info |
| `--strip-producers` | Remove producers section (compiler metadata) |
| `--zero-filled-memory` | Optimize zero-initialized data segments |
| `--converge` | Re-run passes until no further improvement |

Additional passes for future investigation:
- `--vacuum` -- Remove unreachable code
- `--dce` -- Dead code elimination
- `--remove-unused-module-elements` -- Remove unused functions/globals
- `--coalesce-locals` -- Reduce local variable count
- `--reorder-functions` -- Reorder functions for better compression
- `--low-memory-unused` -- Optimize load/store offsets (WASI targets where low addresses unused)

---

## Appendix C: clawft-types Transitive Dependency WASM Impact

| Dependency | Version | WASM-Safe? | Estimated Size | Action |
|-----------|---------|-----------|---------------|--------|
| `serde` | 1.x | Yes | ~10-20 KB (derive at build time) | Keep |
| `serde_json` | 1.x | Yes | ~60-80 KB (parser + formatter) | Keep (required) |
| `chrono` | 0.4.x | Mostly | ~15-25 KB (time formatting) | Profile; consider gate |
| `thiserror` | 2.x | Yes | ~0 KB (proc macro only) | Keep |
| `uuid` | 1.x (v4) | Depends | ~5-10 KB + getrandom | Profile; check WASM compat |
| `dirs` | 6.x | No (OS APIs) | ~1-2 KB dead code | Feature-gate (Section 3.7) |

---

## Appendix D: Native Binary Section Targets (Reference)

| Section | Current (1.93) | Target | Technique |
|---------|---------------|--------|-----------|
| `.text` | 2,668 KB | < 2,600 KB | LTO tuning, PGO |
| `.rodata` | 705 KB | ~705 KB | No change expected |
| `.eh_frame` | 455 KB | < 50 KB | `-C force-unwind-tables=no` |
| `.eh_frame_hdr` | 68 KB | < 10 KB | Same as .eh_frame |
| `.gcc_except_table` | 7 KB | < 1 KB | Same as .eh_frame |
| `.rela.dyn` | 319 KB | ~319 KB | No change expected |
| `.data.rel.ro` | 233 KB | ~233 KB | No change expected |
| **Total** | **4,479 KB** | **< 3,932 KB** | |

The largest opportunity is `.eh_frame` elimination (~530 KB). This trades off
backtraces in release builds. Dev builds are unaffected.

---

## Appendix E: Quick Reference

```bash
# ── Build ─────────────────────────────────────────────────────
cargo wasm                          # Build WASM (wasip2 + release-wasm)
cargo wasm-p1                       # Build WASM (wasip1 fallback)
cargo wasm-check                    # Check WASM (no codegen, fast)
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm --features alloc-talc
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm --features alloc-lol
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm --features alloc-tracing

# ── Optimize ──────────────────────────────────────────────────
bash scripts/build/wasm-opt.sh      # wasm-opt -Oz on built binary

# ── Analyze ───────────────────────────────────────────────────
bash scripts/bench/wasm-size-gate.sh          # CI size check
bash scripts/bench/wasm-twiggy.sh             # Size profiling
bash scripts/bench/wasm-feature-check.sh      # Feature exclusion
bash scripts/bench/alloc-compare.sh --wasm-opt # Compare allocators

# ── Benchmark ─────────────────────────────────────────────────
bash scripts/bench/wasm-startup.sh            # Instantiation time
bash scripts/bench/wasm-memory.sh             # Memory layout

# ── Native (Optional) ────────────────────────────────────────
bash scripts/build/pgo-build.sh               # PGO pipeline
```

---

## Appendix F: Research References

### Allocators
- [talc: A fast and flexible allocator for no_std and WebAssembly](https://github.com/sfbdragon/talc)
- [talc WASM setup guide](https://github.com/SFBdragon/talc/blob/master/talc/README_WASM.md)
- [dlmalloc is way too big (rustwasm/team#25)](https://github.com/rustwasm/team/issues/25)
- [lol_alloc: A laughably simple WASM allocator](https://github.com/nickmass/lol_alloc)
- [Avoiding allocations in Rust to shrink WASM modules](https://nickb.dev/blog/avoiding-allocations-in-rust-to-shrink-wasm-modules/)

### Binary Size Optimization
- [Shrinking .wasm Size (Rust and WebAssembly book)](https://rustwasm.github.io/book/reference/code-size.html)
- [min-sized-rust: How to minimize Rust binary size](https://github.com/johnthagen/min-sized-rust)
- [Optimizing WASM Binary Size (Leptos book)](https://book.leptos.dev/deployment/binary_size.html)
- [Binary Size Optimization (Rust Project Primer)](https://rustprojectprimer.com/building/size.html)

### wasm-opt / Binaryen
- [Binaryen Optimizer Cookbook](https://github.com/WebAssembly/binaryen/wiki/Optimizer-Cookbook)
- [wasm-opt-rs on GitHub](https://github.com/brson/wasm-opt-rs)

### twiggy
- [twiggy: A code size profiler (guide)](https://rustwasm.github.io/twiggy/index.html)
- [twiggy on GitHub](https://github.com/rustwasm/twiggy)

### LTO and Compile Time
- [Build Configuration (Rust Performance Book)](https://nnethercote.github.io/perf-book/build-configuration.html)
- [How I Improved My Rust Compile Times by 75%](https://benw.is/posts/how-i-improved-my-rust-compile-times-by-seventy-five-percent)
- [Speeding up rustc without changing code (Kobzol)](https://kobzol.github.io/rust/rustc/2022/10/27/speeding-rustc-without-changing-its-code.html)

### build-std / panic_immediate_abort
- [panic_immediate_abort + build-std bug (rust-lang/rust#146974)](https://github.com/rust-lang/rust/issues/146974)
- [Panic infrastructure size in WASM (rustwasm/team#19)](https://github.com/rustwasm/team/issues/19)
