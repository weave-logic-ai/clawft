# WEFT-495 result — WASM panel auth (token / capability model)

**Ticket:** WEFT-495  
**Branch:** `wave0k/weft-495-panel-auth`  
**SHA:** branch tip of `wave0k/weft-495-panel-auth` (`git rev-parse HEAD` after merge)  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4dd-9e94-7350-bce2-781c36b0c2cd`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-495 (wave-0k)

## Problem

The webview panel proxies RPC over the local UDS with only a method
allowlist. There was no token, capability set, or per-panel identity on
the proxy layer. Single-user workstations are fine (socket FS perms are
the trust boundary); multi-user / multi-tenant kernels need scoped
panel identity.

## Decision (ADR-071)

**Token-based per-panel identity**, host-held, gated behind multi-user mode:

| Mode | Enable | Behaviour |
|------|--------|-----------|
| Single-user (default) | unset | Unchanged 0.7 allowlist-only proxy |
| Multi-user | `WEFTOS_MULTI_USER=1` or `weft.multiUser: true` | Mint `PanelSession` per panel; enforce CapScopes; attach WEFT-479 `auth` on UDS |

Default multi-user scopes: **`read` + `chat`** (viewer / concierge).
Write/Admin methods are **denied-by-identity** unless the session is
explicitly elevated. Webview never receives the token.

Wire format reuses WEFT-479 literal scopes (`"read,chat"`, `"admin"`, …)
so the daemon capability gate re-checks defense-in-depth.

Note: the audit row cited ADR-042 for multi-user; ADR-042 is cognitive
Act/Analyze/Generate. Tenancy is documented as a deployment mode in
ADR-071.

## What shipped

| Path | Change |
|------|--------|
| `docs/adr/adr-071-wasm-panel-auth.md` | **new** design ADR |
| `docs/adr/README.md` | Index row + Security category |
| `extensions/vscode-weft-panel/src/panelAuth.ts` | **new** mode / session / authorize |
| `extensions/vscode-weft-panel/src/extension.ts` | Wire multi-user session + deny path |
| `extensions/vscode-weft-panel/src/rpc.ts` | Forward optional `auth` on UDS |
| `extensions/vscode-weft-panel/package.json` | `weft.multiUser` setting + `test:panel-auth` |
| `extensions/vscode-weft-panel/test/suite/panelAuth.test.ts` | Mocha coverage |
| `extensions/vscode-weft-panel/scripts/test-panel-auth.mjs` | Host-free unit runner |
| `extensions/vscode-weft-panel/SMOKE.md` | Multi-user section |
| `.planning/reviews/0.7.0-release-gate/15-mcp-integration.md` | Open Q closed |
| `docs/plans/plane-board-inventory.md` | WEFT-495 → Done |
| `docs/plans/wave-0k-WEFT-495-result.md` | This report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Design recorded for per-panel identity + capability scoping | **Yes** — ADR-071 |
| Implementation gated behind multi-user mode | **Yes** — env + VSCode setting; default off |
| Tests cover at least one denied-by-identity case | **Yes** — viewer → `terminal.spawn` / no session / expired |
| Audit row marked closed with WEFT-N | **Yes** — WEFT-495 / ADR-071 |

## Verification

```bash
cd extensions/vscode-weft-panel
npm run test:panel-auth
# expected: all passed
```

Optional (full VSCode host smoke, needs electron deps):

```bash
cd extensions/vscode-weft-panel && npm test
```

No Rust daemon changes required: WEFT-479 already enforces `auth` scopes
when the proxy attaches them.

## Residual / follow-ups

- AuthService-registered `panel:<id>` tokens with revoke + ExoChain audit
  (ADR-066 path).
- Optional `kernel.multi_user` config flag (proxy currently needs no
  daemon round-trip on open).
- Elevated operator panel UX (mint write-scoped sessions from a command).
