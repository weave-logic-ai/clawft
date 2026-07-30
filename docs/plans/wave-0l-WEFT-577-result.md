# WEFT-577 result — VSCode panel wasm bundle trim toward 4500/1500 KB

**Status:** implemented (raw ceiling met; gz one step short of long-term goal)  
**Branch:** `wave0l/weft-577-wasm-trim`  
**Date:** 2026-07-30  
**Agent:** developer (coder-577)

## Problem

After the M7+M7b feature wave the panel WASM was **~7.28 MB raw / ~3.39 MB gz**.
The WEFT-484 budget (4500/1500) had been raised to **7600/3500** in
`scripts/build.sh@cmd_wasm_panel` so the gate would not block ship.
WEFT-577 is the optimisation pass to move back toward 4500/1500.

## Measured outcome

| Metric | Pre (issue) | Post (this branch) | Gate (new) | Long-term goal |
|--------|-------------|--------------------|------------|----------------|
| Raw KB | ~7280 | **4487** | 4500 | 4500 |
| Gz KB  | ~3390 | **1576** | 1600 | 1500 |

- Raw restored to the original WEFT-484 ceiling (**PASS**).
- Gz stepped **7600/3500 → 4500/1600**; still **~76 KB** above 1500 gz.

```text
scripts/build.sh wasm-panel
# Panel WASM (post-opt): 4.38 MB (4594853 bytes)
# Raw: 4487 KB  Gzipped: 1576 KB
# Budget: 4500 / 1600 → PASS
```

## Changes

### Dependency / feature trims

| Change | Files |
|--------|-------|
| `egui_extras`: `all_loaders` → `image` + `datepicker` | `crates/clawft-gui-egui/Cargo.toml`, `crates/clawft-canon/Cargo.toml`, `crates/clawft-surface/Cargo.toml` |
| `egui_demo_lib` moved to native-only deps (was enabling `egui_extras/svg` → **resvg** on wasm) | `crates/clawft-gui-egui/Cargo.toml` |
| `egui` `default-features = false` on panel graph | same three crates |
| eframe wasm: drop `default_fonts`; install Latin subsets in app | `crates/clawft-gui-egui/Cargo.toml`, `src/app.rs` |

### Assets

| Asset | Before | After |
|-------|--------|-------|
| `weftos-gold.png` splash | 636 KB @ 840×736 | ~60 KB @ 420×368 (pngquant) |
| Wasm proportional font | full Ubuntu-Light (~362 KB) | `UbuntuLight-WeftLatin.ttf` (~38 KB) |
| Wasm mono font | full Hack (~309 KB) | `Hack-WeftLatin.ttf` (~84 KB) |
| Emoji fonts (Noto + emoji-icon) | ~743 KB | **not linked** |

Regeneration notes: `crates/clawft-gui-egui/assets/fonts/README.md`.

### Build pipeline

| Change | File |
|--------|------|
| Default cargo profile `release-wasm` (`opt-level = "z"`) | `extensions/vscode-weft-panel/scripts/build-wasm.sh` |
| Shared `run_wasm_opt` (also after wasm-pack path) | same |
| Gate defaults **4500 / 1600** (was 7600 / 3500) | `scripts/build.sh` |
| Panel section + residual | `docs/architecture/wasm-bundle-size.md` |

## Acceptance criteria

| Criterion | Status |
|-----------|--------|
| twiggy top + bloat investigation | Done — large data blobs were fonts + logo + post-M7 code; resvg confirmed via `cargo tree` pre-trim |
| Audit `egui_commonmark` features | Confirmed: only `pulldown_cmark`; no further trim |
| Audit `jiff` surface | Direct: `std` only; datepicker adds `tz-system`+`js`; no tzdb |
| Audit `egui_extras` / `egui_plot` | extras trimmed; plot kept (oscilloscope + canon Plot) |
| Consider lazy-loading viewers | Documented deferred (multi-module design) |
| Lower gate by explicit step | **7600/3500 → 4500/1600** |

## Residual (gz 1500 not met)

**~76 KB gzip** remains above the original 1500 KB goal. Follow-ups:

1. Multi-module / deferred viewers (core shell vs Health/Sensor/plot).
2. Optional wasm feature to gate `egui_plot` + showroom oscilloscope.
3. Tighter Hack subset if monospace glyph needs shrink further.
4. Profile remaining `data[*]` sections with twiggy after next feature freeze.

## How to test

```bash
# Size gate (primary AC)
scripts/build.sh wasm-panel
# expect: raw ≤ 4500, gz ≤ 1600

# Long-term ceiling (expected FAIL on gz by ~76 KB today)
scripts/build.sh wasm-panel 4500 1500

# Graph sanity (no resvg / no emoji pack)
cargo tree -p clawft-gui-egui --target wasm32-unknown-unknown --no-default-features -i resvg
# → nothing to print

# Native still links demo_lab + full fonts
cargo check -p clawft-gui-egui --features native

# Optional symbol size dump
twiggy top -n 30 extensions/vscode-weft-panel/webview/wasm/clawft_gui_egui_bg.wasm
```

## Intentional commit set

- `crates/clawft-gui-egui/**` (Cargo.toml, app.rs, fonts, splash PNG)
- `crates/clawft-canon/Cargo.toml`
- `crates/clawft-surface/Cargo.toml`
- `extensions/vscode-weft-panel/scripts/build-wasm.sh`
- `scripts/build.sh`
- `docs/architecture/wasm-bundle-size.md`
- `docs/plans/wave-0l-WEFT-577-result.md`
- `Cargo.lock` (if deps resolve changed)

**Not committed:** worktree noise (deleted skills, plane JSON dumps, agentdb, `.grok/`).
