# WEFT-493 result — wire McpServerManager file-watcher at daemon/gateway boot

**Ticket:** WEFT-493  
**Branch:** `wave0j/weft-493-mcp-watcher-wire`  
**SHA:** `7c3de142` (implementation commit; tip may advance with doc-only fixes)  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4cf-e145-7911-8cfa-11f78347a35f`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-493 (wave-0j)

## Decision

**Land the watcher** (do not remove hot-reload affordances).

WEFT-187 already shipped `start_config_watcher` / `apply_config_diff` /
drain-and-swap. The audit gap was that nothing called the watcher at
process boot. This ticket wires the production callsite.

## What shipped

### Core — `boot_mcp_manager_with_watcher` (`clawft-services::mcp::discovery`)

| Item | Detail |
|------|--------|
| `McpConfigWatcherBoot` | Host guard: shared manager + path + initial seed + watcher handle |
| `boot_mcp_manager_with_watcher(path)` | Seed manager from path (if exists) → start 500ms debounce watcher → log reloads on a background task |
| Drop / `stop()` | Shuts down the notify task for process teardown |

This is the production API for long-lived hosts. Callers keep the
returned guard for the process lifetime.

### Gateway boot wire (`weft gateway` / `weft ui`)

At gateway start (after tool registry init):

1. Resolve watch path: `--config` → discovery (`CLAWFT_CONFIG` /
   `~/.clawft/config.json` / …) → fallback `~/.clawft/config.json`.
2. Call `boot_mcp_manager_with_watcher`.
3. Hold `_mcp_config_watcher` until Ctrl+C shutdown so the watcher
   survives the full gateway lifetime.

`weft ui` delegates through the same path.

### CLI hint (`weft mcp`)

Reload hint text updated to reference the live gateway watcher (WEFT-493)
instead of the pre-wire WEFT-187-only wording.

### Tests

| Test | Covers |
|------|--------|
| `boot_mcp_manager_with_watcher_seeds_and_applies_edit` | Boot seeds, config edit applies drain-and-swap (add/remove) |
| `boot_mcp_manager_with_watcher_missing_file_still_starts` | Missing path still starts watcher |
| `watcher_edit_config_emits_added_removed_changed` | Hardened against zero-diff FS races |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Decide: land watcher or remove affordances | **Landed** |
| Integration/unit test: config edit → drain-and-swap | **Yes** (`boot_mcp_manager_with_watcher_seeds_and_applies_edit`) |
| Production callsite at long-lived host boot | **Yes** (`weft gateway` / `weft ui`) |
| API surface retained (not removed) | **Yes** |

## Gaps / follow-ups

| Gap | Notes |
|-----|-------|
| ToolRegistry not re-synced on reload | Watcher updates `McpServerManager` registry; live `ToolRegistry` sessions from `register_mcp_tools` still seed once at boot. Full session reconnect is a follow-up (pairs with WEFT-494 daemon verbs). |
| Kernel `weaver` daemon | Does not depend on `clawft-services`; MCP host today is the gateway. Kernel RPC `mcp.reload` remains WEFT-494. |
| Strict allowlists | Default manager remains lenient; production hosts may inject `TransportFactoryConfig` later. |

## Verification

```bash
scripts/build.sh test clawft-services
# → 865 passed (1 leaky pre-existing), includes boot watcher tests

cargo nextest run -p clawft-services --lib mcp::discovery
# → 32 passed

scripts/build.sh test clawft-cli
# → package tests green (compile + unit/integration for mcp_cmd path)
```

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-services/src/mcp/discovery.rs` | `boot_mcp_manager_with_watcher`, `McpConfigWatcherBoot`, tests, docs |
| `crates/clawft-services/src/mcp/mod.rs` | Re-export boot surface |
| `crates/clawft-cli/src/commands/gateway.rs` | Wire watcher at gateway boot; `config_watch_path` param |
| `crates/clawft-cli/src/commands/ui_cmd.rs` | Pass config path into gateway |
| `crates/clawft-cli/src/commands/mcp_cmd.rs` | Reload hint text (WEFT-493) |
| `docs/plans/wave-0j-WEFT-493-result.md` | This report |

## Commit

See git log on branch `wave0j/weft-493-mcp-watcher-wire`. No push (wave branch for lead merge).
