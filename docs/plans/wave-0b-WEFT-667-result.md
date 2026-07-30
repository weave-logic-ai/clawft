# WEFT-667 result — edge-pad firmware tilde-pin esp-hal / esp-radio

**Status:** done  
**Branch:** `wave0b/weft-667-tilde-pin-esp`  
**Base:** `release/0.8-staging`  
**Plane id:** `604a32db-79a6-4cfa-a2fb-5886952a0321`

## What changed

Constraint-only pin tighten for crates that enable esp-* `unstable` features.
No version bump (WEFT-668 deliberately not in scope).

| Crate | Dep | Before (caret-equivalent) | After |
|-------|-----|---------------------------|--------|
| `crates/clawft-edge-pad` | `esp-hal` | `"1.0.0"` (= `^1.0.0`) | `"~1.0"` |
| `crates/clawft-edge-pad` | `esp-radio` | `"0.17.0"` (= `^0.17.0`) | `"~0.17"` |
| `crates/lgfx-bus-rgb-rs` | `esp-hal` | `"1.0.0"` (= `^1.0.0`) | `"~1.0"` |

Manifest comments document **why** tilde: esp-hal upstream policy allows
breaking changes to `unstable` APIs in **minor** releases; caret would
permit those minors. Do not revert while `unstable` is enabled.

## Audit (other in-repo esp-* + unstable)

| Path | `unstable`? | Action |
|------|-------------|--------|
| `crates/clawft-edge-pad/Cargo.toml` | yes (`esp-hal`, `esp-radio`) | tilde-pinned |
| `crates/lgfx-bus-rgb-rs/Cargo.toml` | yes (`esp-hal`) | tilde-pinned |
| `crates/clawft-edge-pad-idf/Cargo.toml` | no (esp-idf-svc/hal stack) | none |
| `crates/clawft-edge-bench/Cargo.toml` | no (esp-idf stack) | none |
| `crates/weftos-leaf-touch-gt911` | no esp-hal dep | none |

## Cargo.lock

Standalone locks already resolve within the new ranges:

- `crates/clawft-edge-pad/Cargo.lock`: `esp-hal 1.0.0`, `esp-radio 0.17.0`
- `crates/lgfx-bus-rgb-rs/Cargo.lock`: `esp-hal 1.0.0`

Locks **unchanged** (constraint tightening only; no `cargo update`).

## Build note

`clawft-edge-pad` and `lgfx-bus-rgb-rs` are **out-of-workspace** (empty
`[workspace]` tables) so Xtensa does not enter the host workspace.
`scripts/build.sh` does not target these crates on host. Full firmware
build needs the esp/Xtensa toolchain from each crate’s
`rust-toolchain.toml`. This change does not move resolved versions, so
a previously green firmware tree remains resolvable under `~1.0` / `~0.17`.

## Acceptance criteria

- [x] `esp-hal = { version = "~1.0", ... }` in clawft-edge-pad
- [x] `esp-radio = { version = "~0.17", ... }` in clawft-edge-pad
- [x] Audited other unstable esp-* crates; lgfx-bus-rgb-rs treated the same
- [x] Cargo.lock unchanged (constraint only)
- [x] WHY-tilde comments in manifests
- [x] WEFT-668 major bump **not** done

## Files

- `crates/clawft-edge-pad/Cargo.toml`
- `crates/lgfx-bus-rgb-rs/Cargo.toml`
- `docs/plans/wave-0b-WEFT-667-result.md`
