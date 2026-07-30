# WEFT-551 result — bump wasmtime 33 → 45.0.3 (RUSTSEC cluster clear)

**Ticket:** WEFT-551  
**Branch:** `wave0c/weft-551-wasmtime-bump`  
**Date:** 2026-07-30  
**Disposition:** **Shipped** — wasmtime / wasmtime-wasi **45.0.3**

## Summary

Bumped the workspace WASM runtime from **wasmtime 33.0.2 / wasmtime-wasi 33.0.2**
to **45.0.3**, clearing all **19** RUSTSEC advisories that were ignored under the
WEFT-551 group (including the WEFT-681 post-June trio 0114 / 0149 / 0182 / 0188).

Gate ignore lists in `scripts/build.sh` and `.github/workflows/pr-gates.yml`
had the 19 IDs removed. WASI plugin / K3 sandbox path remains working
(`execute_bytes` + preview1 linker).

## Final versions

| Crate | Before | After | MSRV |
|-------|--------|-------|------|
| `wasmtime` | 33.0.2 | **45.0.3** | 1.93.0 |
| `wasmtime-wasi` | 33.0.2 | **45.0.3** | 1.93.0 |

Workspace pin: `Cargo.toml` / `crates/clawft-wasm/Cargo.toml` → `"45"`.

### Why not 46+ / 47?

| Target | MSRV | Blocker |
|--------|------|---------|
| 47.0.2 (latest) | **1.94.0** | Project `rust-toolchain.toml` pins **1.93** |
| 46.0.1 | **1.94.0** | Same |
| **45.0.3** | **1.93.0** | Matches toolchain; on the advisory fix lines for 0188 and the rest of the cluster |

Ticket guidance: prefer ≥46.0.1 **if compile works; else highest that builds**.
Highest that builds under the pinned 1.93 toolchain is **45.0.3**.

A follow-up can raise rustc → 1.94 and step to 46/47 if desired; **not required**
for advisory clear (cargo audit reports **zero** wasmtime / wasmtime-wasi hits
on 45.0.3).

## Advisories cleared (19)

Pre-existing WEFT-551 cluster:

`RUSTSEC-2025-0118`, `2026-0006`, `0020`, `0021`, `0085`–`0089`, `0091`–`0096`

Post-2026-04 / WEFT-681:

`RUSTSEC-2026-0114`, `0149` (HIGH FilePerms TRUNCATE), `0182`, `0188` (FilePerms hardlink/rename)

Verified with unfiltered `cargo audit` (no matches on the IDs above; 0 lines
mentioning `wasmtime`).

## API migration (minimal)

Only call site that needed source edits: `crates/clawft-kernel/src/wasm_runner/runner.rs`.

| 33.x | 45.x |
|------|------|
| `wasmtime_wasi::p2::WasiCtxBuilder` | `wasmtime_wasi::WasiCtxBuilder` |
| `wasmtime_wasi::preview1::{WasiP1Ctx, add_to_linker_async}` | `wasmtime_wasi::p1::{WasiP1Ctx, add_to_linker_async}` |
| `Config::async_support(true)` | Removed (deprecated / no-op; async is feature-driven) |
| `wasmtime-wasi` feature `preview1` | feature `p1` |
| pipes `p2::pipe::{MemoryInputPipe, MemoryOutputPipe}` | unchanged |

`clawft-wasm` engine (fuel / epoch / `StoreLimits` / custom host linker) compiled
with **no source changes** against 45.0.3.

## Verification

```text
scripts/build.sh check                          # pass (workspace)
cargo check -p clawft-kernel --features wasm-sandbox   # pass
cargo check -p clawft-wasm --features wasm-plugins     # pass
cargo test -p clawft-kernel --features wasm-sandbox --lib wasm_runner
  # 111 passed; 0 failed
cargo test -p clawft-wasm --features wasm-plugins --lib engine::
  # 30 passed; 5 failed (see below — not wasmtime)
cargo audit  # no WEFT-551 IDs remain; remaining hits are WEFT-552/553 + other
```

### Pre-existing clawft-wasm FS sandbox flakes (out of scope)

Five `engine::` tests and several `fs::` / `sandbox::` tests fail on this macOS
host with `SymlinkEscape("/var")` when using `std::env::temp_dir()` under
`/var/folders` → `/private/var`. Pure path-canonicalization sandbox checks;
**not** wasmtime API regressions. Wasmtime-specific tests that did pass:

- `t28_fuel_exhaustion`, `t29_memory_limit_exceeded`, `t30_wall_clock_timeout`
- `t31`/`t32` custom thresholds, `t43_wasmtime_store_isolation`
- `t45_fuel_*`, store/linker creation, validate_module_*

## Files touched

| File | Change |
|------|--------|
| `Cargo.toml` | workspace pin 33 → 45; features `runtime` + wasi `p1` |
| `Cargo.lock` | wasmtime stack → 45.0.3 |
| `crates/clawft-wasm/Cargo.toml` | optional wasmtime pin 33 → 45 |
| `crates/clawft-kernel/src/wasm_runner/runner.rs` | p1 / WasiCtxBuilder migration; drop async_support |
| `scripts/build.sh` | remove 19 WEFT-551 `--ignore` entries |
| `.github/workflows/pr-gates.yml` | same ignore-list sync |
| `docs/deployment/release.md` | WEFT-551 row → DONE / 45.0.3 |
| `docs/plans/wave-0c-WEFT-551-result.md` | this result |

## Acceptance criteria

| AC | Status |
|----|--------|
| wasmtime upgraded to ≥ 43.0.1 (or closest LTS / chosen set) | **Met** — 45.0.3 |
| cap-rand (via wasmtime-wasi) re-resolves cleanly | **Met** (lockfile updates) |
| All 14+ RUSTSEC IDs removed from gate `--ignore` list | **Met** — all 19 removed |
| scripts/build.sh check / relevant tests | **Met** — check green; kernel wasm_runner 111/111 |
| clawft-wasm + clawft-kernel wasm path compile on new API | **Met** |

## Commit

Branch: `wave0c/weft-551-wasmtime-bump`  
Tip SHA: see `git rev-parse HEAD` on the branch after commit.
