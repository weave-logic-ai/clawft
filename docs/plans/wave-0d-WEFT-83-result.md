# WEFT-83 result — `agents.workspace_root` config key

**Ticket:** WEFT-83  
**Branch:** `wave0d/weft-83-workspace-root`  
**SHA:** `4822d59516f9daf1c93d84e0a20c538cb5230380`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb45e-9f20-75c3-aedd-b56bd8fefeeb`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-83 (wave-0d)

## Problem

Daemon identity (`SOUL.md` / `IDENTITY.md`) and tool workspace paths always
resolved from process CWD. systemd units and multi-project operators need a
fixed root independent of launch directory (plan §15.4 / MW-5).

## What shipped

### Config — `clawft-types`

| Item | Detail |
|------|--------|
| `AgentsConfig::workspace_root` | `Option<PathBuf>`, serde default `None`, alias `workspaceRoot` |
| `AgentsConfig::resolve_workspace_root` | Configured path (with `~/` expand on native) or `current_dir()` |
| `AgentsConfig::resolve_workspace_root_or` | Same, with injectable CWD fallback for tests |

Distinct from `agents.defaults.workspace` (nanobot-style file-tool string).

### Identity — `clawft-core`

| Item | Detail |
|------|--------|
| `IdentityLoader::from_agents_config` | Builds loader from `agents.workspace_root` / CWD |
| `IdentityLoader::workspace` | Exposes resolved root |
| Docs on `IdentityLoader::new` | Updated to reference the config key |

### Daemon — `clawft-weave`

Snapshots `config.agents.workspace_root` before `Kernel::boot` takes
ownership of `config`. Agent service wiring uses
`AgentsConfig::resolve_workspace_root()` for identity loader, tools, and
loop workspace (replaces hard-coded `current_dir()` only).

### Docs

- `docs/guides/agents.md` — identity + `workspace_root` operator guide
- `docs/guides/configuration.md` — agents section table
- `docs/reference/config.md` — schema entry

## Acceptance

| Criterion | Status |
|-----------|--------|
| `agent.workspace_root: Option<PathBuf>` on AgentsConfig | Yes (`agents.workspace_root`) |
| IdentityLoader reads key when present, else CWD | Yes (`from_agents_config` + daemon wire) |
| Test: two workspaces / distinct identities | Yes (`two_workspaces_load_distinct_identities`) |
| Documented in `docs/guides/agents.md` | Yes |

**Note on multi-workspace RPC:** A single daemon process still binds **one**
root at boot. Per-RPC workspace switching remains a later 0.8.x story (ticket
notes). Until then: one daemon per workspace or restart with new config.

## Verification

```text
scripts/build.sh check
# ok (~28s workspace)

cargo test -p clawft-types workspace_root
# 3 passed

cargo test -p clawft-core --lib identity::
# 11 passed (incl. two_workspaces + CWD fallback)

cargo check -p clawft-weave
# ok
```

## Files changed

- `crates/clawft-types/src/config/mod.rs`
- `crates/clawft-core/src/agent/identity.rs`
- `crates/clawft-weave/src/daemon.rs`
- `docs/guides/agents.md` (new)
- `docs/guides/configuration.md`
- `docs/reference/config.md`
- `docs/plans/wave-0d-WEFT-83-result.md` (this file)
