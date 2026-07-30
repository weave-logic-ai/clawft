# WEFT-179 result — FlowDelegator retire

**Ticket:** WEFT-179 — ws07: FlowDelegator — implement `delegation/flow.rs` or formally retire the Flow target  
**Branch:** `wave0c/weft-179-flow-delegator`  
**Date:** 2026-07-30  
**Agent:** coder-179 (team wave-0c)

## Decision: **RETIRE**

Do **not** implement `crates/clawft-services/src/delegation/flow.rs`.

### Rationale

1. `flow.rs` never existed; element-09 tracker falsely claimed Done (see D-MF-001 / WEFT-202).
2. Runtime already treated Flow as Claude fallback (`resolve_availability`); `flow_available` was gone.
3. `claude_flow_enabled` had no runtime consumer — a silent no-op for users who set it.
4. Product architecture is **MCP-first** (`docs/guides/mcp-integration.md`): persistent sessions + skill-scoped tools replace subprocess `claude --print` / Flow spawn (context bloat, tool duplication, CLI coupling).
5. M-Foundation subprocess design conflicts with the MCP posture; re-implementing it would reintroduce the wrong model.

Recorded as **D-MF-003** in  
`.planning/development_notes/09-multi-agent-routing/phase-M-foundation/decisions.md`.

## What shipped

### Code

| Change | Path |
|--------|------|
| Remove `claude_flow_enabled` | `crates/clawft-types/src/delegation.rs` |
| Remove `DelegationTarget::Flow` | same |
| Serde aliases: `"flow"`/`"Flow"` → `Claude` | same |
| Drop Flow arm in engine; update tests | `crates/clawft-services/src/delegation/mod.rs` |
| Comment cleanup | `crates/clawft-services/src/delegation/claude.rs` |
| CLI test uses Claude | `crates/clawft-cli/src/commands/agent.rs` |

### Docs / decisions

- `docs/reference/config.md` — field table + migration note  
- `docs/guides/routing.md` — Flow retired section  
- `docs/guides/configuration.md` — align with real config surface  
- `docs/guides/testing-mcp-delegation.md` — examples + §6.3  
- `docs/guides/mcp-integration.md` — source table  
- `docs/architecture/overview.md` — services blurb  
- phase-M-foundation `decisions.md` — D-MF-003  

### Migration path (users)

| Old | Action |
|-----|--------|
| `"target": "flow"` / `"Flow"` | Still loads as `claude`. Prefer `"claude"`. |
| `claudeFlowEnabled` / `claude_flow_enabled` | Ignored; remove key. |
| Expectation of subprocess Flow | Use MCP + skills (`docs/guides/mcp-integration.md`). |

## Verification

```text
scripts/build.sh test clawft-types clawft-services   # 628 passed
scripts/build.sh test clawft-cli                     # 415 passed
scripts/build.sh check                               # ok
```

## Follow-ups (not in this ticket)

- WEFT-304: dashboard delegation API should use live `DelegationEngine` / events (not FlowDelegator).
- WEFT-180: recursive-delegation depth guard (independent).
- Optional: WEFT tickets about half-wired claude-flow MCP default session (distinct from FlowDelegator).
