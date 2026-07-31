# WEFT-285 result — vscode panel WSP-0.1 verb support

**Ticket:** WEFT-285  
**Branch:** `feat/weft-285-wsp-verbs`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-31  
**Agent:** developer (Grok Build / Ruflo)

## Problem

The VSCode / Cursor panel spoke only raw daemon JSON-RPC
(`kernel.*`, `agent.chat`, …). WSP-0.1 (protocol-spec / ADR-005)
defines a 17-verb surface protocol; the panel had no adapter, so
composers and future renderers could not use the protocol surface
through the extension host.

## Acceptance

| Criterion | Status |
|-----------|--------|
| WSP-0.1 verb surface for common panel operations | **Yes** — 17 verbs in `src/wsp.ts` |
| Map verbs to existing RPC | **Yes** — `RESOURCE_RPC_MAP` + `AFFORDANCE_RPC_MAP` |
| Migrate at least one existing surface to WSP path | **Yes** — kernel status via `subscribe` on ready |
| Backward compat retained for raw RPC | **Yes** — `rpc-request` unchanged |
| Tests / docs for verb set | **Yes** — `wsp.test.ts`, SMOKE §10, this report |

## What shipped

| Path | Change |
|------|--------|
| `extensions/vscode-weft-panel/src/wsp.ts` | **new** verb catalog, resource/affordance maps, `translateWsp` |
| `extensions/vscode-weft-panel/src/wsp.test.ts` | **new** unit tests (no VSCode host) |
| `extensions/vscode-weft-panel/src/extension.ts` | `wsp-request` handler, bootstrap on ready, shared `proxyDaemonRpc` |
| `extensions/vscode-weft-panel/package.json` | `test:wsp`, `test:unit` |
| `extensions/vscode-weft-panel/SMOKE.md` | §10 WSP-0.1 |
| `extensions/vscode-weft-panel/README.md` | WSP surface note |
| `docs/plans/wave-0-WEFT-285-result.md` | This report |

## Design notes

- **Adapter, not full WSP kernel.** Session / surface / subscription
  state lives in the extension host. Mapped verbs hop the same UDS
  path as raw RPC (allowlist + multi-user scopes).
- **Hard cap of 17 verbs** matches ADR-005. New capability = ontology
  URI, not a new verb.
- **Denylist absolute:** no path maps to `substrate.publish`.
- **Migrated surface:** on webview `ready` → `session.initialize` →
  `subscribe resource://kernel/status` → `kernel.status` snapshot as
  `wsp-notification` / `substrate.update`.
- **Wire messages:**
  - `wsp-request` / `wsp-response`
  - `wsp-hello` (server-caps after auto-init)
  - `wsp-notification` (bootstrap status + future streams)

### Resource map (excerpt)

| Resource URI | RPC |
|--------------|-----|
| `resource://kernel/status` | `kernel.status` |
| `resource://kernel/ps` | `kernel.ps` |
| `resource://kernel/services` | `kernel.services` |
| `resource://cluster/status` | `cluster.status` |
| `substrate://…` | `substrate.subscribe` / `.read` |

### Affordance map (excerpt)

| Affordance | RPC |
|------------|-----|
| `chat` / `chat_stream` | `agent.chat` / `agent.chat_stream` |
| `terminal_write` | `terminal.write` |
| `service_start` | `service.start` |
| `control_set_enabled` | `control.set_enabled` |

## Verification

```bash
cd extensions/vscode-weft-panel
npm install   # first time
npm run compile
npm run test:wsp
# expected: all tests pass
```

Optional full unit suite:

```bash
npm run test:unit
```

## Out of scope (follow-ups)

- Full active-radar `observation.update` stream (observe opens id only).
- Wasm Live loop rewrite to prefer WSP over raw RPC (compat path is enough for AC).
- Daemon-native WSP frame type on mesh (protocol-spec §3.2).
