# WEFT-187 result — MCP config hot-reload watcher

**Ticket:** WEFT-187  
**Branch:** `wave0f/weft-187-mcp-hot-reload`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb478-7733-7932-bf2f-901d2110586b`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-187 (wave-0f)

## Problem

`McpServerManager::apply_config_diff` returned `(added, removed, changed)`
counts but nothing invoked it when `clawft.toml` / config files changed.
The M-Advanced notify watcher with 500ms debounce was specified but not
wired.

## What shipped

### Core — `clawft-services::mcp::discovery`

| Item | Detail |
|------|--------|
| `load_mcp_servers_from_path` / `_from_str` | Parse `tools.mcp_servers` (alias `mcpServers`) from TOML or JSON |
| `apply_config_diff_validated` | Run transport factory validators, skip bad entries, then `apply_config_diff` |
| `reload_from_path` | File → parse → validate → apply |
| `start_config_watcher` / `_with_debounce` | `notify` on parent dir, **500ms** default debounce, emits `ConfigReloadResult` |
| `McpConfigWatcherHandle` | Drop / `stop()` shuts down the background task |
| `ConfigReloadResult` | `added` / `removed` / `changed` / `validation_errors` |
| Change detection | Full `McpServerConfig` equality (command, args, env, url) |

### Dependencies

- `notify` + `toml` added to `crates/clawft-services/Cargo.toml` (workspace versions).

### Re-exports

`mcp` module re-exports watcher surface for callers:

- `start_config_watcher`, `start_config_watcher_with_debounce`
- `ConfigReloadResult`, `McpConfigWatcherHandle`, `SharedMcpServerManager`
- `load_mcp_servers_from_path`, `load_mcp_servers_from_str`

## Acceptance

| Criterion | Status |
|-----------|--------|
| notify watcher on config with 500ms debounce | **Yes** (`DEBOUNCE_MS = 500`; parent-dir watch for atomic renames) |
| Reload triggers `apply_config_diff` + validation | **Yes** (`reload_from_path` → `apply_config_diff_validated`) |
| Integration/unit test: edit config → added/removed/changed | **Yes** (`reload_from_path_applies_add_remove_change`, `watcher_edit_config_emits_added_removed_changed`) |
| package tests pass | **Yes** — `scripts/build.sh test clawft-services` → **319 passed** |

## Gaps / follow-ups

| Gap | Notes |
|-----|-------|
| No daemon wiring | Watcher API is ready; runtime/daemon still must call `start_config_watcher` on boot (pairs with WEFT-188 CLI). |
| Drain on file-remove | Hot-reload uses synchronous `remove_server` inside `apply_config_diff` (existing behavior). Async drain remains available via `remove_server_drain` for CLI paths. |
| Strict command-path allowlist | Validators use the manager's factory config; default manager is lenient (empty `allowed_paths`). Production should use `TransportFactoryConfig` with allowlists. |
| Env-only change | Now counted as `changed` (full `PartialEq`); previously only command/args. |

## Tests

```bash
cargo test -p clawft-services --lib mcp::discovery
# → 28 passed (includes WEFT-187 parse / validate / reload / watcher)

scripts/build.sh test clawft-services
# → 319 passed, 0 failed
```

Key new tests:

- `load_mcp_servers_from_toml_tools_section`
- `load_mcp_servers_from_json_camel_case_alias`
- `apply_config_diff_reports_changed_on_command_update`
- `apply_config_diff_validated_skips_bad_http_url`
- `reload_from_path_applies_add_remove_change`
- `watcher_edit_config_emits_added_removed_changed`
- `debounce_default_is_500ms`

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-services/src/mcp/discovery.rs` | Watcher, parse, validated reload, tests |
| `crates/clawft-services/src/mcp/mod.rs` | Public re-exports |
| `crates/clawft-services/Cargo.toml` | `notify`, `toml` deps |
| `docs/plans/wave-0f-WEFT-187-result.md` | This report |

## Commit

*(not committed by agent — leave for lead merge from worktree branch)*
