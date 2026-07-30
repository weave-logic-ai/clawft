# WEFT-595 result — leaf-display single-buffer disambiguation (BUG-1)

**Branch:** `wave0a/weft-595-leaf-double-buffer`  
**Date:** 2026-07-30  
**Status:** Code fix shipped; **hardware reflash not run in this environment**

## Ticket

ws08 residual visual gap: gutter coords correct on the wire (Q24.8
`x=12800` = 50 px, `drawn=9`) but CrowPanel content does not visibly
move. Prime suspect: `lgfx-bus-rgb-rs` double-buffer page flip /
stale offscreen under dirty-rect redraws.

## What shipped

### 1. Accept single-buffer as edge-pad production default

`crates/clawft-edge-pad/Cargo.toml` keeps:

```toml
lgfx-bus-rgb-rs = { path = "../lgfx-bus-rgb-rs", default-features = false }
```

Comments updated to record WEFT-595 disposition: single-buffer is the
hardware-proven path (2026-05-15) and remains default until a CrowPanel
flash confirms the double-buffer repairs below.

Boot log now prints `double_buffer=true|false` so a serial session can
confirm which path is flashed without decoding ELF features.

### 2. Double-buffer repair (for optional re-enable) — `lgfx-bus-rgb-rs` 0.2.2

| Change | Why |
|--------|-----|
| `src/page_flip.rs` + 6 unit tests | Pure page-flip state machine; proves `offscreen = scanning ^ 1`, one-shot present, no double-swap on spurious VSYNC, documents toggle-skipped failure mode |
| `BusRgb::copy_scanning_to_offscreen()` | After a flip, offscreen still holds frame N−1. Dirty-rect clear/redraw alone presents hybrid N−1 + N damage → tearing / “gutter never moves” while wire is correct |
| `DpiSurface::begin_frame` | When double-buffered **and** damage is partial, blit scanning → offscreen before clearing damage |

Root-cause class addressed: **dirty-rect + double-buffer without blit**
(not only an ISR index bug). ISR swap order was already coherent with
the pure model; the missing coherence step was seeding the offscreen
buffer before partial redraws.

### 3. Host-runnable tests (no CrowPanel / no Xtensa)

```bash
rustc --edition 2021 --test crates/lgfx-bus-rgb-rs/src/page_flip.rs \
  -o /tmp/page_flip_test && /tmp/page_flip_test
# 6 passed
```

`scripts/build.sh check` — green (workspace; edge-pad / bus crates are
out-of-workspace by design).

## Hardware verification status

| Step | Status |
|------|--------|
| Disable double-buffer in edge-pad Cargo.toml | **Done** (pre-existing + documented) |
| Unit / sim coverage of page-flip invariants | **Done** (6/6 host tests) |
| Blit-before-partial-damage for double-buffer | **Done** (code path; only active if feature re-enabled) |
| Reflash CrowPanel + observe gutter | **Not run** — no panel in CI/agent environment |
| Re-enable double-buffer after HW OK | **Deferred** — keep `default-features = false` until checklist below passes |

## Manual flash checklist (operator)

Requires: ESP32-S3 CrowPanel DIS08070H, `esp` toolchain, USB serial.

### A. Single-buffer confirmation (current production build)

```bash
cd crates/clawft-edge-pad
# Cargo.toml must have: lgfx-bus-rgb-rs = { path = "../lgfx-bus-rgb-rs", default-features = false }
cargo build --release
# flash with your usual espflash / probe-rs flow, e.g.:
#   espflash flash --monitor target/xtensa-esp32s3-none-elf/release/clawft-edge-pad
```

Expect serial:

```text
[edge-pad] DpiSurface up — framebuffer @ 0x........ (align%64 = 0) double_buffer=false
```

Then push a scene with a large gutter (e.g. `GUTTER=50` via
`scripts/leaf-push-ps.sh` / mesh leaf producer) and confirm text is
visibly inset. Mild tearing during writes is **acceptable** on
single-buffer.

### B. Double-buffer verification (optional, after A passes)

1. In `crates/clawft-edge-pad/Cargo.toml`, change to:

   ```toml
   lgfx-bus-rgb-rs = { path = "../lgfx-bus-rgb-rs" }  # default features = double-buffer
   ```

2. Rebuild, flash, expect `double_buffer=true`.
3. Same gutter push: content must move with gutter **without** the
   hybrid/stale layout of BUG-1.
4. If still broken → pivot to `clawft-edge-pad-idf` + LovyanGFX sidecar
   (handoff option; not implemented in this ticket).

### C. Recoverability snapshot

`.planning/actors/inkpad-snapshots/2026-05-15-fallout-glitch/` remains
the last-known alternate DPI path if a bad flash needs rollback.

## Files touched

- `crates/lgfx-bus-rgb-rs/src/page_flip.rs` (new)
- `crates/lgfx-bus-rgb-rs/src/bus.rs` (`copy_scanning_to_offscreen`)
- `crates/lgfx-bus-rgb-rs/src/lib.rs`
- `crates/lgfx-bus-rgb-rs/Cargo.toml` (0.2.2)
- `crates/lgfx-bus-rgb-rs/README.md`
- `crates/clawft-edge-pad/Cargo.toml` (disposition comments)
- `crates/clawft-edge-pad/src/drivers/dpi_surface.rs` (partial-damage blit)
- `crates/clawft-edge-pad/src/main.rs` (boot log)
- `docs/plans/wave-0a-WEFT-595-result.md` (this file)

## Acceptance criteria

- [x] Single-buffer disambiguation applied in edge-pad Cargo.toml  
- [x] If single-buffer is the safe path: accept it as production default **and** repair double-buffer for re-enable (blit + page_flip tests)  
- [x] Unit/sim test + manual flash docs (HW not available here)  
- [x] `scripts/build.sh check` green for workspace  
- [ ] CrowPanel reflash observation (operator)  
