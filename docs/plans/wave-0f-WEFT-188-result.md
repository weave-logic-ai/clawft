# WEFT-188 result — weft mcp add/list/remove CLI

**Ticket:** WEFT-188  
**Branch:** `wave0f/weft-188-mcp-cli`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb478-7733-7932-bf2f-90062494c4b0`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-188 (wave-0f)

## Problem

`commands/mcp_cmd.rs` for `weft mcp add/list/remove` was specified (docs +
`McpServerManager` comments) but not shipped. Users had to edit config by hand
under `tools.mcpServers` / `tools.mcp_servers`.

## What shipped

### CLI — `weft mcp`

| Subcommand | Behavior |
|------------|----------|
| `weft mcp add <name> --command <cmd> [--arg …] [--env KEY=VAL]…` | Stdio transport entry |
| `weft mcp add <name> --url <endpoint>` | HTTP transport entry |
| `weft mcp add <name> -- <cmd> [args…]` | Claude-style trailing command |
| `weft mcp list` | Table of configured servers |
| `weft mcp remove <name>` | Delete entry by name |

Flags: `--internal-only` (default `true`), `--config` override.

### Config mutation

- **Writable path order:** `--config` → `CLAWFT_CONFIG` → cwd `clawft.toml` →
  cwd `weave.toml` → `~/.clawft/config.json` (created on write).
- **Formats:** JSON and TOML (by extension). Ticket wording “clawft.toml” is
  honored when that file exists; the day-to-day user path is still
  `config.json`.
- **Atomic write:** same-dir temp file + `rename`, with direct-write fallback
  (same pattern as skills autogen / WEFT-67).
- **Key style preserved:** existing `mcp_servers` vs `mcpServers` is kept;
  new JSON defaults to `mcpServers`, TOML to `mcp_servers`.

### Reload

After add/remove, best-effort daemon RPC: `mcp.reload` then `config.reload`.
If neither is available (current state; hot-reload is WEFT-187), prints:

```text
Hint: restart the agent/gateway/daemon, or wait for config hot-reload (WEFT-187) to pick up the change.
```

Does **not** block on a watcher.

### Help

- Top-level help lists `mcp`
- Topic: `weft help mcp`

## Files

| Path | Change |
|------|--------|
| `crates/clawft-cli/src/commands/mcp_cmd.rs` | **new** — add/list/remove + atomic persist + unit tests |
| `crates/clawft-cli/src/commands/mod.rs` | export `mcp_cmd` |
| `crates/clawft-cli/src/main.rs` | `Commands::Mcp` + parse tests |
| `crates/clawft-cli/src/help_text.rs` | `mcp` topic + general help line |
| `crates/clawft-cli/tests/cli_integration.rs` | binary-level add/list/remove roundtrip tests |

## Acceptance

| Criterion | Status |
|-----------|--------|
| `weft mcp add <name> --command/--url/--env …` | Yes |
| `weft mcp list` | Yes |
| `weft mcp remove <name>` | Yes |
| Atomic config edit + reload hint / optional RPC | Yes |
| CLI tests | Yes (unit + integration + clap parse) |
| `scripts/build.sh test clawft-cli` green | Yes (448 passed, 1 pre-existing leaky) |

## Verification

```text
scripts/build.sh test clawft-cli
# Summary: 448 tests run: 448 passed (1 leaky), 0 skipped
# (doctest step prints "no library targets found" for the bin-only package — pre-existing)
```

Unit coverage in `mcp_cmd` includes env parse, trailing command resolution,
JSON/TOML roundtrip, key-style preservation, missing remove, empty transport.

## Notes / non-goals

- No new daemon RPC verbs (`mcp.add` / `mcp.list` / `mcp.remove`) — out of
  scope; list may still render live state via existing `tools.mcp` when the
  daemon is up.
- Hot-reload watcher is **WEFT-187** (concurrent); this ticket only surfaces
  a hint when reload cannot be triggered.
- `weft mcp-server` (outbound shell) is unchanged.
