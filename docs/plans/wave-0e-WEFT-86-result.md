# WEFT-86 result — WorkspaceManager::delete aligns with FR-W06

**Ticket:** WEFT-86  
**Branch:** `wave0e/weft-86-workspace-delete`  
**SHA:** `c3abf1ada654895e14f42ff8c1b1d45dd979ca5e`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef4-7621-9495-d5d59245e889`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-86 (wave-0e)

## Problem

`WorkspaceManager::delete` removed the registry entry but explicitly did
**not** delete files on disk ("caller's responsibility"). FR-W06 says
`--keep-data` is the opt-in — default should remove `.clawft/` +
`CLAWFT.md`. Behavior was inverted vs the functional requirement (MW-8).

## What shipped

### Core — `WorkspaceManager::delete`

| Item | Detail |
|------|--------|
| Signature | `delete(&mut self, name: &str, keep_data: bool)` |
| Default (`keep_data = false`) | Remove registry entry **and** `.clawft/` + `CLAWFT.md` |
| Opt-out (`keep_data = true`) | Registry-only (pre-0.8 behavior) |
| Safety | Does **not** remove project root or other user source files |
| Idempotency | Missing markers treated as already gone |
| Failure mode | File-removal errors leave the registry entry so callers can retry |

### CLI — `weft workspace delete`

- New flag: `--keep-data`
- Confirmation copy reflects whether files will be removed
- RPC params include `keep_data`; local fallback uses the same API

### Daemon — `workspace.delete`

- Accepts optional `keep_data` (default `false`)
- Response: `{ "deleted": name, "keep_data": bool }`

### Docs

- `docs/guides/workspaces.md` — behavior + **migration note** for users of the old no-op file delete
- `docs/reference/cli.md` — `--keep-data` option and examples

## Migration note (operators / scripts)

**Breaking change (WEFT-86 / FR-W06):** older versions only unregistered the
workspace and never deleted files. Scripts that assumed a no-op file delete
must pass `--keep-data` (or `keep_data: true` over RPC) to restore that
behavior. Default `weft workspace delete` now removes `.clawft/` and
`CLAWFT.md`.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Default delete removes `.clawft/` + `CLAWFT.md` | Yes |
| `--keep-data` opts out (registry-only) | Yes |
| CLI `weft workspace delete` matches | Yes |
| Tests for both branches | Yes (default, keep_data, already-gone, not-found) |
| Migration note in result + workspace guide | Yes |

## Verification

```text
scripts/build.sh check
# ok

cargo test -p clawft-core --lib workspace::tests::
# 17 passed (incl. delete default / keep_data / idempotent / not_found)

cargo check -p clawft-cli -p clawft-weave
# ok
```

Note: full `scripts/build.sh test clawft-core` hit an **unrelated** fail in
`workspace::config::tests::load_merged_config_mcp_servers` (null MCPServerConfig
JSON). Not introduced by this change; WorkspaceManager delete tests all pass.

## Files changed

- `crates/clawft-core/src/workspace/mod.rs` — delete API + tests
- `crates/clawft-cli/src/commands/workspace_cmd.rs` — `--keep-data` + messaging
- `crates/clawft-weave/src/daemon.rs` — RPC `keep_data`
- `docs/guides/workspaces.md` — FR-W06 behavior + migration
- `docs/reference/cli.md` — flag docs
- `docs/plans/wave-0e-WEFT-86-result.md` — this file
