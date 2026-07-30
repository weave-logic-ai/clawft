# WEFT-427 result — extract canon types; move composer to clawft-surface

**Ticket:** WEFT-427  
**Branch:** `wave0k/weft-427-canon-types`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-427 (wave-0k)

## Problem

The composer runtime lived in `clawft-gui-egui::surface_host` rather
than in `clawft-surface`. M1.5-D (`f5e40c3`) broke a cargo-rejected
cycle by relocating the composer into gui-egui; the proper fix —
extract shared canon types, move the composer back — was deferred.

Source: `.planning/reviews/0.7.0-release-gate/13-app-substrate-surface.md`
(task 18; deferred §clawft-surface; cross-cutting §3).

## What shipped

### New crate: `clawft-canon`

Primitive canon widgets + return-signal types (ADR-001 / ADR-006)
extracted from `clawft-gui-egui::canon`:

- All 21-ish primitives (`Chip`, `Gauge`, `Table`, `Stack`, …)
- Head types (`Affordance`, `Confidence`, `VariantId`, …)
- `CanonResponse` / `CanonWidget` trait
- Pure **egui** dependency (no `eframe` app runner)

### Composer back in `clawft-surface`

| Before | After |
|--------|--------|
| `clawft_gui_egui::surface_host::{compose, render_headless}` | `clawft_surface::compose::{compose, render_headless}` |
| Cycle broken by placement | Cycle broken by shared crate |

Module layout:

```
crates/clawft-surface/src/compose/
  mod.rs          — re-exports
  runtime.rs      — compose / permits / honest_affordances
  test_harness.rs — headless egui frame helpers
```

Public re-exports on `clawft_surface::{compose, compose_with_permits,
ComposeOutcome, ComposePermits, PendingDispatch, render_headless, …}`.

### Compatibility shims in `clawft-gui-egui`

Historical import paths still work:

- `crate::canon` / `clawft_gui_egui::canon` → `pub use clawft_canon::*`
- `crate::surface_host` / `clawft_gui_egui::surface_host` →
  `pub use clawft_surface::compose::*`

Shell (`desktop.rs`) and integration tests need no call-site changes.

### Dependency graph (no cycle)

```
clawft-canon  (egui widgets)
     ↑
clawft-surface  (IR + eval + composer)
     ↑
clawft-gui-egui  (shell; re-exports canon + surface_host)
```

## Acceptance

| Criterion | Status |
|-----------|--------|
| New shared canon-types crate created and consumed by both clawft-surface and clawft-gui-egui | **Done** — `clawft-canon` |
| Composer runtime relocated into clawft-surface | **Done** — `compose` module |
| No cyclic deps; gate/check passes | **Done** |

## Tests

```bash
cargo check -p clawft-canon -p clawft-surface -p clawft-gui-egui
cargo test -p clawft-canon -p clawft-surface --lib
cargo test -p clawft-surface --tests
cargo test -p clawft-gui-egui --lib
cargo test -p clawft-gui-egui --test admin_app_e2e --test surface_headless_render \
  --test compose_extra_iris --test chip_surfaces
```

| Suite | Result |
|-------|--------|
| clawft-canon lib | 9 passed |
| clawft-surface lib | 40 passed (incl. 8 honest_affordances) |
| clawft-surface integration | 12 passed |
| clawft-gui-egui lib | 371 passed |
| gui-egui surface suite (4 tests crates) | 21 passed |

## Files

| Path | Change |
|------|--------|
| `crates/clawft-canon/**` | **New** shared canon crate |
| `crates/clawft-surface/src/compose/**` | **New** composer + harness |
| `crates/clawft-surface/src/lib.rs` | `pub mod compose` + re-exports |
| `crates/clawft-surface/Cargo.toml` | deps: clawft-canon, egui, egui_extras |
| `crates/clawft-gui-egui/src/canon.rs` | Re-export shim |
| `crates/clawft-gui-egui/src/surface_host.rs` | Re-export shim |
| `crates/clawft-gui-egui/src/canon/*` | Removed (moved) |
| `crates/clawft-gui-egui/src/surface_host/*` | Removed (moved) |
| `Cargo.toml` / `Cargo.lock` | workspace member + dep |

## Residual / follow-ups

1. **Task 17** (drop unused egui from surface) — surface now
   intentionally depends on egui for the composer; no longer applicable
   as “drop unused”. Optional future: feature-gate composer behind
   `compose` / `egui` so pure-IR consumers skip UI deps.
2. **Non-egui renderers** — `clawft-canon` still embeds egui widgets;
   a thinner head-types-only crate remains possible if a non-egui
   host needs only IRIs / affordance metadata.
3. **Import path migration** — new code should prefer
   `clawft_surface::compose` / `clawft_canon` over the gui-egui shims.
4. **WEFT-277 / WEFT-431** etc. — unchanged; composer behaviour is
   identical post-move.

## Worktree

- Path: this agent worktree on branch `wave0k/weft-427-canon-types`
- No push (per wave instructions)
