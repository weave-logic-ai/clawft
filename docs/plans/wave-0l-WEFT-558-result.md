# WEFT-558 result — VSCode panel E2E chip-icon DOM assertion

**Ticket:** WEFT-558  
**Branch:** `wave0l/weft-558-chip-e2e`  
**SHA:** branch tip (`git rev-parse HEAD` on `wave0l/weft-558-chip-e2e`)  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4ea-5b24-73d2-a8c3-9e9a36f5eed8`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-558 (wave-0l)  
**Followup to:** WEFT-486 (M6-B scaffold)

## Problem

WEFT-486 shipped the headless VSCode smoke harness (`@vscode/test-electron`
+ Mocha): activation + `weft.openPanel` resolve. Tray chips paint **inside
the egui canvas** (wasm), so DOM-side `webview.html` introspection cannot
see them. The chip assertion stayed `test.skip` until this followup.

## Decision (option a)

**Test-only mock-provider inject into a DOM-side a11y strip** — not
screenshot/canvas-pixel harness (option b).

| Concern | Choice |
|---------|--------|
| Where chips land in the DOM | `#weft-chip-a11y[data-weft-status-strip]` visually hidden |
| Stable ids | tray `ChipId`s: `kernel`, `mesh`, `exochain`, `explorer` |
| Production | strip container only (empty) unless `WEFT_PANEL_E2E=1` |
| Test path | `weft._test.chipStripSnapshot` injects `MOCK_E2E_CHIPS`, parses `webview.html` |
| CI | existing `vscode-panel-build` job + new pure unit step |

## What shipped

| Path | Change |
|------|--------|
| `extensions/vscode-weft-panel/src/chipA11y.ts` | **new** pure helpers: render / parse / inject / E2E mode |
| `extensions/vscode-weft-panel/src/chipA11y.test.ts` | **new** host-free unit tests (`npm run test:chip-a11y`) |
| `extensions/vscode-weft-panel/src/extension.ts` | empty strip in HTML; seed under E2E; `weft._test.chipStripSnapshot` |
| `extensions/vscode-weft-panel/test/suite/extension.test.ts` | unskipped case: `>=1` chip + `id === "kernel"` |
| `extensions/vscode-weft-panel/test/runTest.ts` | `WEFT_PANEL_E2E=1`; short tmp user-data/extensions dirs |
| `extensions/vscode-weft-panel/test/suite/index.ts` | comment refresh |
| `extensions/vscode-weft-panel/package.json` | `test:chip-a11y`; `@vscode/test-electron@3.1.0` |
| `extensions/vscode-weft-panel/package-lock.json` | lock for 3.1.0 |
| `extensions/vscode-weft-panel/tsconfig.json` | `declaration: true` so compile-tests resolve `out/*.d.ts` |
| `extensions/vscode-weft-panel/SMOKE.md` | §9 chip DOM assertion |
| `.github/workflows/pr-gates.yml` | unit step + updated WEFT-558 comments |
| `docs/plans/plane-board-inventory.md` | WEFT-558 → Done |
| `docs/plans/wave-0l-WEFT-558-result.md` | this report |

### Harness fixes bundled (needed for green E2E)

1. **`@vscode/test-electron` 2.5.2 → 3.1.0** — VS Code 1.110+ ships
   `Contents/MacOS/Code` instead of `Electron`; 2.x spawn was `ENOENT` on
   modern downloads. 3.1.0 resolves the sole MacOS binary.
2. **Short `--user-data-dir` / `--extensions-dir`** under `os.tmpdir()` —
   deep worktree paths exceed the ~103-char Unix socket limit
   (`EINVAL` on `…/user-data/1.13-main.sock`).

## Acceptance mapping

| Criterion | Status |
|-----------|--------|
| Skipped test (or replacement) wired and passing under `vscode-panel-build` | **Yes** — unskipped E2E case + CI unit step |
| Chip strip exposes ≥1 chip with stable id | **Yes** — `data-chip-id="kernel"` (plus mesh/exochain/explorer) |
| Documented in `SMOKE.md` | **Yes** — §9 |

## Verification

```bash
cd extensions/vscode-weft-panel

# Pure unit (no VSCode host) — 11/11 pass
npm run test:chip-a11y

# Full headless host (downloads VS Code once; xvfb on Linux CI)
npm run pretest && npm test
# expected: 15 passing (panelAuth + smoke including chip strip)
```

Local run (2026-07-30, darwin-arm64): **15 passing**.

## Residual / follow-ups

- Live tone sync from wasm tray → DOM a11y strip (today E2E uses mock
  inject; canvas tones remain canvas-only in production).
- Optional: promote mock inject into a real a11y mirror driven by
  `Live` snapshot for screen readers (out of scope for WEFT-558).

## How to merge

Branch `wave0l/weft-558-chip-e2e` off `release/0.8-staging`. No push from
this agent. Lead merges intentional files only (see commit).
