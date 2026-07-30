# WEFT-114 result — CI cargo check wasm32-unknown-unknown (no mesh)

**Ticket:** WEFT-114  
**Branch:** `wave0d/weft-114-wasm-ci`  
**Wave:** 0d  
**Date:** 2026-07-30  
**Base:** `release/0.8-staging`

## Problem

No CI gate ensured `clawft-kernel` builds for `wasm32-unknown-unknown` with
mesh (and every other default feature) off. Browser / no-mesh consumers risk
silent breakage when non-browser code lands in always-on modules.

## Fix

### CI hard gate

New job in `.github/workflows/pr-gates.yml`:

```yaml
wasm-kernel-no-mesh:
  name: Kernel WASM check (no mesh)
  # …
  - run: cargo check -p clawft-kernel --target wasm32-unknown-unknown --no-default-features
```

Failure blocks merge (no `continue-on-error`).

### `scripts/build.sh`

- `scripts/build.sh check` now runs workspace `cargo check`, then (when
  `wasm32-unknown-unknown` is installed) the same kernel no-mesh check.
- Phase gate gains check **14**: `kernel WASM no-mesh`.
- Help text documents the command and the CI twin job.

### Kernel compile fixes (so the gate is green)

Bare `--no-default-features --target wasm32-unknown-unknown` did not compile
before this ticket. Minimal feature-gating / deps:

| Area | Change |
|------|--------|
| `Cargo.toml` | Target-specific `uuid` features `["js"]` for `wasm32` + `unknown` OS |
| `topic.rs` | Gate `SubscriberSink::ExternalStream` + tokio mpsc behind `native` |
| `governance.rs` | Gate EML scorer / `EffectVector::score(model)` behind `ecc` |
| `cluster.rs` | Gate EML `recommended_heartbeat_secs(model, …)` behind `ecc` |
| `complexity.rs` | Gate `ComplexityModel` path behind `ecc` |

Default-feature native builds unchanged. Mesh remains default-on for native.

## Verification

```bash
cargo check -p clawft-kernel --target wasm32-unknown-unknown --no-default-features
# Finished `dev` profile … (exit 0)

cargo check -p clawft-kernel
# Finished `dev` profile … (exit 0) — defaults still green

scripts/build.sh check --dry-run
# shows workspace check + kernel wasm no-mesh step
```

## Acceptance criteria

| Criterion | Status |
|-----------|--------|
| CI runs `cargo check --target wasm32-unknown-unknown --no-default-features` (or feature-equivalent) | **Met** — job `wasm-kernel-no-mesh` |
| Failure blocks merge | **Met** — hard job, no soft skip |
| Documented in `scripts/build.sh` | **Met** — `check` + `gate` + `--help` |

## Files changed

- `.github/workflows/pr-gates.yml` — `wasm-kernel-no-mesh` job
- `scripts/build.sh` — `check_kernel_wasm_no_mesh`, extend `cmd_check` + gate 14, help
- `crates/clawft-kernel/Cargo.toml` — wasm32 uuid `js`
- `crates/clawft-kernel/src/topic.rs` — native-gate ExternalStream
- `crates/clawft-kernel/src/governance.rs` — ecc-gate EML scorer path
- `crates/clawft-kernel/src/cluster.rs` — ecc-gate gossip timing path
- `crates/clawft-kernel/src/assessment/analyzers/complexity.rs` — ecc-gate model path
- `docs/plans/wave-0d-WEFT-114-result.md` (this file)

## Commit

**SHA:** `3d7c2c3f6104816538b956dcfdfab1d9aff94871`  
Branch: `wave0d/weft-114-wasm-ci`
