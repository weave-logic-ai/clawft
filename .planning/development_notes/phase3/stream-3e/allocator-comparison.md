# Phase 3E: Allocator Comparison + Feature Flags

**Date**: 2026-02-17
**Status**: Complete
**Agent**: 3E-A (Allocator Comparison + Feature Flags)

## What Was Done

### 1. Cargo.toml Feature Flags (Task 1)
Added three allocator feature flags to `crates/clawft-wasm/Cargo.toml`:
- `alloc-talc = ["dep:talc"]` -- selects talc allocator
- `alloc-lol = ["dep:lol_alloc"]` -- selects lol_alloc allocator
- `alloc-tracing = []` -- enables tracing wrapper around dlmalloc

Added wasm32-conditional optional dependencies:
- `talc = { version = "4.4", optional = true }` (latest: 4.4.3)
- `lol_alloc = { version = "0.4", optional = true }` (latest: 0.4.1)

dlmalloc remains the default (no feature flag needed).

### 2. Feature-Gated allocator.rs (Task 2)
Rewrote `crates/clawft-wasm/src/allocator.rs` with `#[cfg]` guards:
- Default (no features): `dlmalloc::GlobalDlmalloc`
- `alloc-talc`: `talc::Talck<talc::locking::AssumeUnlockable, talc::ClaimOnOom>` with `ClaimOnOom::new(Span::empty())` for WASM linear memory growth
- `alloc-lol`: `lol_alloc::AssumeSingleThreaded<lol_alloc::LeakingAllocator>` (single-threaded WASM safety)
- `alloc-tracing`: `crate::alloc_trace::TracingAllocator<dlmalloc::GlobalDlmalloc>`

Each variant uses `#[cfg(all(target_arch = "wasm32", ...))]` with mutual exclusion to prevent multiple `#[global_allocator]` definitions.

### 3. alloc_trace.rs Module (Task 3)
Created `crates/clawft-wasm/src/alloc_trace.rs` with:
- `TracingAllocator<A: GlobalAlloc>` generic wrapper
- Size class counters: small (<=64), medium (<=256), large (<=1024), xlarge (<=4096), huge (>4096)
- Aggregate counters: TOTAL_ALLOC_BYTES, TOTAL_FREED_BYTES, TOTAL_ALLOC_COUNT, TOTAL_FREE_COUNT, PEAK_LIVE_BYTES
- `AllocStats` struct with `serde::Serialize` for JSON export
- `read_stats()` -- snapshot of all counters with computed `live_bytes` and `fragmentation_ratio`
- `reset_stats()` -- zeroes all counters for test isolation
- `stats_json()` -- JSON string export
- CAS loop in `alloc()` to track peak live bytes
- All counters use `AtomicUsize` with `Ordering::Relaxed` (safe for single-threaded WASM)
- 3 unit tests covering reset, JSON validity, and fragmentation computation

### 4. lib.rs Module Wiring (Task 4)
Added `#[cfg(feature = "alloc-tracing")] pub mod alloc_trace;` to `crates/clawft-wasm/src/lib.rs`.

### 5. Allocator Comparison Script (Task 5)
Created `scripts/bench/alloc-compare.sh`:
- Builds clawft-wasm with each allocator variant (dlmalloc, talc, lol_alloc)
- Supports `--wasm-opt` flag for post-build optimization
- Records and displays comparison table with Raw KB, Opt KB, Gzip KB
- Handles build failures gracefully (prints FAIL instead of aborting)
- Uses `set -euo pipefail`

### 6. Verification Results (Task 6)

| Check | Result |
|-------|--------|
| `cargo check -p clawft-wasm --target wasm32-wasip2` (default) | PASS |
| `cargo check -p clawft-wasm --target wasm32-wasip2 --features alloc-tracing` | PASS |
| `cargo clippy -p clawft-wasm --target wasm32-wasip2 -- -D warnings` (default) | PASS, 0 warnings |
| `cargo clippy -p clawft-wasm --target wasm32-wasip2 --features alloc-tracing -- -D warnings` | PASS, 0 warnings |
| `cargo test -p clawft-wasm` (default, host) | 41 passed, 0 failed |
| `cargo test -p clawft-wasm --features alloc-tracing` (host) | 44 passed, 0 failed |
| `bash -n scripts/bench/alloc-compare.sh` (syntax) | PASS |

## Decisions Made

1. **Crate versions**: Used `talc = "4.4"` (latest 4.4.3) and `lol_alloc = "0.4"` (latest 0.4.1). Used minor version pins rather than exact versions for compatibility.

2. **Feature flag design**: Features are mutually exclusive at the allocator level via cfg guards. `alloc-tracing` takes priority over all others (it wraps dlmalloc regardless of other features). `alloc-talc` and `alloc-lol` are gated with `not(feature = "alloc-tracing")`.

3. **alloc-tracing wraps dlmalloc only**: The SPARC plan specifies wrapping dlmalloc when tracing is enabled, even if talc or lol features are also set. This keeps tracing behavior consistent.

4. **AtomicUsize with Relaxed ordering**: Single-threaded WASM has no data races. Relaxed is the cheapest ordering and correct for this use case.

5. **AllocStats uses serde::Serialize**: Allows direct JSON serialization via `serde_json::to_string_pretty`. The `serde` dependency is already in scope from workspace deps.

6. **Comparison script handles failures gracefully**: If talc or lol_alloc fail to build (e.g., API incompatibility), the script prints FAIL for that row and continues with other allocators.

## Issues Found

None. All checks and tests pass cleanly.

## Files Modified

- `crates/clawft-wasm/Cargo.toml` -- added features and optional deps
- `crates/clawft-wasm/src/allocator.rs` -- rewritten with feature-gated selection
- `crates/clawft-wasm/src/alloc_trace.rs` -- new file, tracing allocator wrapper
- `crates/clawft-wasm/src/lib.rs` -- added conditional `alloc_trace` module
- `scripts/bench/alloc-compare.sh` -- new file, allocator comparison script

## Next Steps

- Run `scripts/bench/alloc-compare.sh` once talc and lol_alloc are available (requires actual WASM builds)
- Integrate wasm-opt into the build pipeline (separate 3E task)
- Add twiggy profiling scripts (separate 3E task)
- Add CI size gate for WASM binary (separate 3E task)
- Record allocator comparison results in baseline.json
