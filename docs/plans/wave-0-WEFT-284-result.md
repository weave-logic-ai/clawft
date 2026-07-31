# WEFT-284 result — ThreadDock primitive for per-agent parallel output

**Ticket:** WEFT-284  
**Branch:** `feat/weft-284-threaddock`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-31  
**Agent:** developer (Grok Build / Ruflo)

## Problem

No ThreadDock primitive — multiple parallel agent outputs collapse
together into a single 1-D scroll. Multi-agent swarm UX gap (session-7
§Multi-agent parallel output; SMOKE.md known gap).

## Acceptance

| Criterion | Status |
|-----------|--------|
| Add ThreadDock canon primitive (or extension on Dock) | **Done** — `ui://thread-dock` composition over StreamView columns |
| Compose with chat panel for parallel agent threads | **Done** — `ChatPanel` paints ThreadDock when ≥2 agent columns |
| Smoke test with ≥2 parallel streams | **Done** — Rust + TypeScript unit tests |

## What shipped

| Path | Change |
|------|--------|
| `crates/clawft-canon/src/thread_dock.rs` | **new** `ThreadDock`, `AgentThread`, `ThreadDockState`, `ThreadPhase` + 8 unit tests |
| `crates/clawft-canon/src/lib.rs` | module + re-exports |
| `crates/clawft-gui-egui/src/canon.rs` | re-export `thread_dock` submodule |
| `crates/clawft-gui-egui/src/canon_demos.rs` | Blocks demo tab (coder/reviewer/tester, ≥2 streams) |
| `crates/clawft-gui-egui/src/explorer/chat.rs` | `ChatPanel` host: upsert/feed/clear + paint when parallel + 3 tests |
| `extensions/vscode-weft-panel/src/threadDock.ts` | **new** wire helpers (panel bridge) |
| `extensions/vscode-weft-panel/src/threadDock.test.ts` | **new** 8 unit tests (no VSCode host) |
| `extensions/vscode-weft-panel/package.json` | `test:thread-dock` + `test:unit` |
| `extensions/vscode-weft-panel/SMOKE.md` | gap closed + §WEFT-284 |
| `docs/plans/wave-0-WEFT-284-result.md` | This report |

## Design notes

- **Composition, not ADR-001 row 22.** ROADMAP M2: ThreadDock is an
  app-level composition over StreamView-style line tails, exposed as a
  `CanonWidget` so chat + panel share one type. Frozen 21-item table
  unchanged.
- **Column-per-agent, never interleaved.** Each `AgentThread` owns a
  monotonic `lines` buffer; focus lane is a single index; non-focused
  columns show phase pip + token rate.
- **Chat composition.** Idle single-conversation chat is unchanged.
  Hosts call `upsert_agent_thread` / `feed_agent_stream`; when
  `is_parallel()` (≥2), paint shows ThreadDock above the transcript.
- **Interrupt** affordance on focused active threads (session-7:
  interrupt is a first-class verb). Agent columns never raise modals.
- **Wire.** Panel TypeScript mirrors Rust state helpers for host-side
  reasoning without wasm; full swarm RPC plumbing is a follow-up.

## Verification

```bash
cargo test -p clawft-canon thread_dock --lib
# 8 passed

cargo test -p clawft-gui-egui --lib thread_dock_hidden
cargo test -p clawft-gui-egui --lib parallel_agent_streams
cargo test -p clawft-gui-egui --lib upsert_replaces_same_id
# 3 passed

cd extensions/vscode-weft-panel && npm run test:thread-dock
# 8 passed
```

## Follow-ups

- Daemon multi-agent stream topics → automatic `feed_agent_stream`.
- Voice verb "switch to the planner" → `focus_by_id`.
- Tray chips for agent-raised attention (no modal from columns).
