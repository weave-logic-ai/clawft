# WEFT-681 result — wasmtime advisory deferred 2026-06-28 never tracked

**Ticket:** WEFT-681  
**Branch:** `wave0b/weft-681-wasmtime-advisory`  
**Date:** 2026-07-30  
**Disposition:** **Risk accepted (defer)** — no in-range bump; fix vehicle is **WEFT-551**

## Summary

The 2026-06-28 patch round (`0be16e2e`) fixed quinn-proto / memmap2 / rkyv and
**knowingly deferred** three new wasmtime / wasmtime-wasi advisories into the
existing WEFT-551 ignore group. That deferral lived only in the commit body, so
the open security debt was invisible from the tracker.

WEFT-681 re-audited the current lockfile, confirmed the cluster is still open
on **wasmtime 33.0.2 / wasmtime-wasi 33.0.2**, established reachability for the
WASI FilePerms class, and recorded a **dated risk acceptance** with a hard
review date. The major version bump remains **WEFT-551** (target floor raised
to **46.0.1** for full clear of the newest WASI IDs).

## Audit evidence (2026-07-30)

```text
Locked: wasmtime 33.0.2, wasmtime-wasi 33.0.2
Workspace pins: Cargo.toml wasmtime = "33", wasmtime-wasi = "33"
  features: cranelift, async, wat / preview1  (no winch, no pooling default)
```

`cargo audit` (unfiltered) still reports **19** advisories on this pair.
There is **no in-range fix** on the 33.x line for any of them; solutions require
≥36.x and, for full clear of RUSTSEC-2026-0182 / 0188, **≥46.0.1**.

### June-deferred trio (the original gap)

| ID | Crate | Sev | Title | Fix floor |
|----|-------|-----|-------|-----------|
| RUSTSEC-2026-0114 | wasmtime | MED 5.9 | Panic allocating table past host address space | ≥36.0.8 / ≥43.0.2 |
| RUSTSEC-2026-0149 | wasmtime-wasi | **HIGH 7.5** | `path_open(TRUNCATE)` bypasses `FilePerms::WRITE` | ≥36.0.10 / ≥44.0.2 |
| RUSTSEC-2026-0182 | wasmtime-wasi | LOW 2.3 | Leak in WASIp1 `fd_renumber` | ≥36.0.11 / ≥44.0.3 / ≥45.0.2 |

### New since June (found this audit)

| ID | Crate | Sev | Title | Fix floor |
|----|-------|-----|-------|-----------|
| RUSTSEC-2026-0188 | wasmtime-wasi | MED 6.5 | Hard links / renames bypass `FilePerms` on destination | ≥36.0.12 / ≥45.0.3 / **≥46.0.1** |

### Pre-existing WEFT-551 cluster (still open on 33.0.2)

RUSTSEC-2025-0118, 2026-0006, 0020, 0021, 0085–0089, 0091–0096  
(notable: **RUSTSEC-2026-0095** CRITICAL Winch sandbox escape; **RUSTSEC-2026-0096** CRITICAL aarch64 Cranelift sandbox escape).

## Exposure analysis (WeftOS configuration)

| Path | Reachable? | Notes |
|------|------------|-------|
| K3 sandbox WASI preopens / host FS | **No** (default) | `wasm_runner::execute_bytes` builds `WasiCtxBuilder` with **stdio pipes only** — no `.preopened_dir` / FilePerms. **0149 / 0188 / 0182 FilePerms class not reachable** unless a future caller grants preopens. |
| Compiler backend | Cranelift only | Workspace features pin `cranelift`; **Winch-only** advisories (0094, 0095, 0086, 0089) are **not exercised** in our build. |
| Component-model string / flags | Low | Host path is classic core modules + preview1 linker / custom `host::*` funcs, not component-model string transcoding. |
| aarch64 Cranelift heap miscompile (0096 CRITICAL) | **Residual** | Reachable if untrusted guest modules run on aarch64 with Cranelift. Mitigated by fuel + epoch + memory limits; still a real residual for untrusted-code hosts. |
| Pooling allocator leakage (0088) | Low | We construct plain `Engine` / per-call `Store`s; pooling not configured. |

**Bottom line:** the June HIGH (0149) is **config-mitigated today** (no FS preopens). The residual that still argues for prioritizing WEFT-551 is **untrusted-guest + aarch64 Cranelift (0096)** and any future preopen-enabled WASI tool path.

## Why not bump now

- Jumping 33 → 46 is a **major** API surface move (component model, WASI, linker types) across `clawft-kernel` wasm_runner and `clawft-wasm` engine.
- Already scoped and priority-high as **WEFT-551** (0.8.x). Doing it inside WEFT-681 would silently duplicate that workstream and risk a half-migrated sandbox.
- Safe path: keep 33.0.2 + ignore-list + this acceptance until WEFT-551 lands.

## Risk acceptance (explicit, dated)

| Field | Value |
|-------|--------|
| **Accepted** | 2026-07-30 |
| **Accepted under** | WEFT-681 (tracks the 2026-06 deferral that previously lived only in `0be16e2e`) |
| **Fix vehicle** | **WEFT-551** — bump wasmtime / wasmtime-wasi to **≥46.0.1** (floor raised from the original “43” note; 46.0.1 is the lowest version that clears 0188) |
| **Review date** | **2026-10-30** (or earlier if WEFT-551 is claimed / if any production path adds WASI preopens or runs untrusted modules on aarch64 without additional isolation) |
| **Revisit triggers** | (1) WEFT-551 claim; (2) adding filesystem preopens to the sandbox; (3) enabling Winch; (4) new CRITICAL on 33.x with no config mitigation; (5) quarterly dep sweep WEFT-473 |
| **Residual risk** | aarch64 Cranelift sandbox-escape class (0096) if untrusted guests are hosted; any future preopen path re-opens 0149/0188 |

## What shipped in this ticket

1. **Tracking** — June deferral is no longer commit-only; WEFT-681 + this result + release.md table.
2. **Ignore-list hygiene**
   - Added **RUSTSEC-2026-0188** under the WEFT-551 group in `scripts/build.sh`.
   - **Synced** `.github/workflows/pr-gates.yml` with the three June IDs that were in `build.sh` but **missing from CI** (`0114`, `0149`, `0182`) plus `0188`. (CI was silently out of date with the local gate — called out as a secondary finding.)
3. **Docs** — `docs/deployment/release.md` WEFT-551 row updated (19 IDs, target 46+, link WEFT-681, review date).
4. **No version bump** — Cargo.toml / Cargo.lock unchanged.

## Acceptance criteria

| AC | Status |
|----|--------|
| Identify specific wasmtime advisory (RUSTSEC) + affected version range via fresh `cargo audit` | **Met** — 19 IDs on 33.0.2; see tables above |
| Establish whether WeftOS is exposed | **Met** — FilePerms/WASI-FS class config-mitigated; residual aarch64 Cranelift 0096 |
| Patch/bump **or** dated risk acceptance with reason | **Met** — acceptance above; bump deferred to WEFT-551 |
| If accepted, set a review date | **Met** — **2026-10-30** |
| Link WEFT-551 | **Met** |

## Secondary findings (out of WEFT-681 scope)

`scripts/build.sh audit` still fails on **non-wasmtime** items not in the ignore list (as of 2026-07-30):

- RUSTSEC-2026-0194 / 0195 — `quick-xml` 0.39.2 (HIGH DoS)
- RUSTSEC-2026-0204 — `crossbeam-epoch` 0.9.18
- RUSTSEC-2026-0190 — `anyhow` unsound (warning)
- RUSTSEC-2020-0036 / 2019-0036 — `failure` (unmaintained/unsound)
- yanked `spin` 0.9.8

These need separate tracked items (or a fresh dep-sweep under WEFT-473); they are **not** part of the wasmtime deferral.

Also: CI ignore-list drift (fixed here for the wasmtime group) is exactly the “gate not failing on it” failure mode the ticket notes — worth a one-line check whenever `CARGO_AUDIT_IGNORES` changes.

## Files touched

| File | Change |
|------|--------|
| `scripts/build.sh` | WEFT-551 ignore group + 0188 + WEFT-681/review comments; target floor 46+ |
| `.github/workflows/pr-gates.yml` | Sync wasmtime ignores (0114, 0149, 0182, 0188) |
| `docs/deployment/release.md` | WEFT-551 row: 19 IDs, 46+, link WEFT-681 |
| `docs/plans/wave-0b-WEFT-681-result.md` | This result |

## How to re-verify

```bash
# Locked versions
grep -A2 'name = "wasmtime"' Cargo.lock | head -6

# Full unfiltered picture
cargo audit 2>&1 | grep -E 'wasmtime|RUSTSEC-2026-01(14|49|82|88)'

# Gate with ignores (wasmtime cluster should be silent; other new deps may still fail)
scripts/build.sh audit
```

## Commit

Branch: `wave0b/weft-681-wasmtime-advisory`

Tip SHA is the return value of this wave item (`git rev-parse HEAD` on the branch).

