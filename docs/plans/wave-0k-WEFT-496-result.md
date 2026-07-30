# WEFT-496 result — webview vs daemon allowlist / substrate.publish gate

**Ticket:** WEFT-496  
**Branch:** `wave0k/weft-496-publish-gate`  
**SHA:** `b933f1140591ce7339e5d16ded5e8edfb2f0dae9`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4dd-9e94-7350-bce2-782ff5a15f53`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-496 (wave-0k)

## Problem

`substrate.publish` was omitted from the VSCode panel allowlist with the
comment "the webview is a viewer, not a writer." That comment was false:

1. The panel already proxies intentional mutators (`agent.chat`,
   `terminal.*`, `control.set_enabled`, `cron.add`, `service.*`, …).
2. `agent.chat` (post-D3) runs a full tool registry and daemon sinks that
   write substrate under grant-gated `_derived/` paths.
3. WEFT-250 unions daemon-advertised methods into the runtime allowlist,
   so a static omission of `substrate.publish` was not durable if
   introspection listed the method.

## Decision (ADR-072)

**Mediated mutators yes; raw substrate pen no.**

| Surface | Policy |
|---------|--------|
| Webview → `substrate.publish` | **Hard deny** (`WEBVIEW_DENIED_METHODS`) |
| Webview → high-level RPCs | **Allow** when on allowlist |
| Daemon sinks / services | **Grant-gated** `publish_gated(_with_grants)` |
| Agent tools in `agent.chat` | Workspace / registry under governance; **no** `substrate.publish` tool |

Do **not** shrink the agent tool surface for this ticket — the integrity
boundary is the denylist + daemon grants, not read-only tools.

## Audit: agent.chat tool surface vs substrate

### Tools registered at daemon boot (`clawft_tools::register_all`)

| Tool | Mutates | Hits substrate? |
|------|---------|-----------------|
| `read_file` | No (workspace read) | No |
| `list_directory` | No | No |
| `write_file` | Workspace FS | No |
| `edit_file` | Workspace FS | No |
| `exec_shell` (`native-exec`) | Process / FS via shell | No |
| `spawn` (`native-exec`) | Child process | No |
| `memory_read` | No | No (reads `MEMORY.md`) |
| `memory_write` | Home workspace `MEMORY.md` | No |
| `web_fetch` / `web_search` | Network | No |
| `agent_spawn` / `agent_message` / `task_status` / `task_result` | Subagent registry / child conv | Child turns use same sink path (daemon), not a tool-level `substrate.publish` |
| `voice_*` / `audio_*` (feature) | Voice I/O | Sensor paths are daemon services, not these tools |
| `render_ui` (feature) | Canvas topic | Not substrate |

**Spike-time surface (`read_file`, `list_directory`) is not exhaustive.**
D3 wired the full registry. None of the tools implement a substrate write;
substrate mutations on the chat path are **daemon sinks**, not tools.

### Daemon-mediated substrate writes on the chat / panel path

| Writer | Path prefix | Gate |
|--------|-------------|------|
| `SubstrateConversationSink` | `substrate/_derived/chat/<conv>/turns|status|meta` | `chat` DerivedWriteGrant |
| `agent.chat_stream` frames | `substrate/_derived/chat/<conv>/stream` | `publish_gated_with_grants` |
| `SubstrateSoulJournal` | `substrate/_derived/soul_journal/…` | `soul_journal` grant |
| `SubstrateRouterDecisionLog` | `substrate/_derived/agent/routing/recent/…` | `agent` grant |
| Terminal service | session output paths | `publish_gated` as daemon node |

These fire only when the corresponding high-level RPC / service runs —
never because the webview called `substrate.publish`.

## What shipped

| Path | Change |
|------|--------|
| `extensions/vscode-weft-panel/src/allowlist.ts` | **new** — denylist, static seed, `isMethodAllowed` / `mergeAllowlist` |
| `extensions/vscode-weft-panel/src/allowlist.test.ts` | **new** — pure unit tests (no VSCode host) |
| `extensions/vscode-weft-panel/src/extension.ts` | Use helpers; revise comments; distinct deny error string |
| `docs/adr/adr-072-webview-substrate-publish-gate.md` | **new** decision |
| `docs/adr/README.md` | Index + Security category |
| `.planning/reviews/0.7.0-release-gate/15-mcp-integration.md` | Open Q closed with WEFT-496 / ADR-072 |
| `docs/plans/plane-board-inventory.md` | WEFT-496 → Done |
| `docs/plans/wave-0k-WEFT-496-result.md` | this report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Audit agent.chat tool surface; enumerate substrate mutators | **Yes** — tables above |
| Decide: tighten tools vs revise viewer-only | **Yes** — revise invariant (ADR-072); keep tools |
| Comment block in `extension.ts` updated | **Yes** |
| Audit row closed with WEFT-N | **Yes** — WEFT-496 / ADR-072 |

## Verification

```text
cd extensions/vscode-weft-panel
npx --yes tsx --test src/allowlist.test.ts
# pass: WEBVIEW_DENIED_METHODS, STATIC_ALLOWED_METHODS, isMethodAllowed, mergeAllowlist

npx --yes tsc -p . --noEmit
# clean (allowlist + extension wire-up)
```

## Residual / follow-ups

- Daemon-side per-caller capability tags (sibling open Q in same audit) so
  non-panel UDS clients share the same write floor.
- Expand `WEBVIEW_DENIED_METHODS` if new raw-write RPCs appear
  (e.g. unconstrained memory / MCP runtime mutations from the panel).
- Optional: panel allowlist entry for `mcp.list` when the UI needs live MCP
  status (read-only; not part of this ticket).
