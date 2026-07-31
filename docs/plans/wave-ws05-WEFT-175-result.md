# WEFT-175 — iMessage scope: formal drop (no AppleScript bridge)

**Status:** Done (decision: drop)  
**Branch / worktree:** `docs/weft-175-imessage-drop`  
**Path:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb69a-23ae-7e01-9cdd-90a8b013f31f`  
**Base:** `release/0.8-staging`  
**Plane ID:** `5edb90e4-e4cb-4dc0-a103-df475c58a495`  
**Date:** 2026-07-31

## Acceptance

| Criterion | Result |
|-----------|--------|
| Decision: implement AppleScript bridge **or** remove with recorded rationale | **Drop** — [ADR-081](../adr/adr-081-no-imessage-applescript-bridge.md) |
| If implemented: feature flag, module, factory, tests | **N/A** (not implemented) |
| If dropped: tracker entry updated; orphan callouts removed | **Yes** — Plane close + audit/orchestrator/guide updates |

## Decision summary

**No first-party iMessage AppleScript / Messages.app channel for 0.8.x**
(and not as open deferred E4 scope). Rationale: macOS GUI-session–bound
transport, missing CI capacity, security/Automation surface, and honest
product matrix. Signal remains E4. Reopen only via a **new** Plane item
with design + test plan.

## What changed

| Artifact | Change |
|----------|--------|
| `docs/adr/adr-081-no-imessage-applescript-bridge.md` | New Accepted ADR |
| `docs/adr/README.md` | Index ADR-080 + ADR-081 |
| `docs/guides/channels.md` | Explicit “iMessage out of scope” note |
| `.planning/sparc/phase4/06-channel-enhancements/00-orchestrator.md` | E4 = Signal only; drop `imessage/` path |
| `.planning/reviews/0.7.0-release-gate/05-channels.md` | Orphan / Task #19 resolved → ADR-081 |
| `docs/plans/plane-board-inventory.md` (row + detail) | WEFT-175 → Done / drop |
| `docs/plans/plane-wave-plan.md` | Mark closed |
| Plane WEFT-175 | In Progress → **Done** with close comment |

## Code / tests

- No Rust changes (no `imessage` tree existed).
- Build/test: **N/A** for product code; docs-only.

## Follow-ups

- None required for 0.8.x.
- Optional later: third-party skill/bridge or new ticket if product demand + macOS automation story appears.
