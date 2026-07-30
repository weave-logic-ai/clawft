# WEFT-494 result — mcp.add/list/remove daemon RPCs + CLI vs RPC ownership

**Ticket:** WEFT-494  
**Branch:** `wave0j/weft-494-mcp-rpc`  
**SHA:** `ff07bcbe23f99e6b74c5475fae9ea73f2a2c1d7c`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4cf-e145-7911-8cfa-12052b8f2aa1`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-494 (wave-0j)

## Problem

`McpServerManager` exposed add/remove/list + drain-and-swap, while docs
implied `weft mcp add/list/remove` without saying whether ownership was
per-process (CLI config edit) or daemon-managed (shared registry). CLI
landed in **WEFT-188** (durable config only; no RPC verbs). Daemon
dispatch had no `mcp.*` arms; CLI best-effort `mcp.reload` / `tools.mcp`
always fell through.

## Decision (ADR-070)

**Hybrid ownership:**

| Layer | Owns |
|-------|------|
| **CLI** `weft mcp …` (WEFT-188) | Durable `tools.mcp_servers` / `tools.mcpServers` file writes |
| **Daemon RPC** `mcp.add` / `mcp.list` / `mcp.remove` / `mcp.reload` | Live shared `McpServerManager` in the kernel process |

Preferred operator path: CLI writes config → `mcp.reload` (or WEFT-187
watcher) applies the diff. Direct `mcp.add`/`mcp.remove` are **runtime
only** (do not rewrite config).

Recorded in `docs/adr/adr-070-mcp-registry-ownership.md`.

## What shipped

### Daemon RPCs (`clawft-weave`)

| Method | Cap | Behavior |
|--------|-----|----------|
| `mcp.add` | Write | Register/replace server in live registry (validated via transport factory) |
| `mcp.list` | Read | Live rows + `output` text table |
| `tools.mcp` | Read | Alias of `mcp.list` (CLI compat) |
| `mcp.remove` | Write | Drain (default) or sync remove from live registry |
| `mcp.reload` | Write | `reload_from_path` with `params.path` or standard path walk |

Boot seeds the registry from `Config.tools.mcp_servers` and remembers a
best-effort config path for path-less `mcp.reload`.

### Capability table

`mcp.list` / `tools.mcp` → Read; `mcp.add` / `mcp.remove` / `mcp.reload` → Write.

### CLI alignment

- `weft mcp list` probes `mcp.list` then `tools.mcp` before local config.
- Module docs state ADR-070 ownership; post-mutate still calls `mcp.reload`
  then `config.reload`.

### Docs / audit

- ADR-070 + README index row
- `discovery.rs` module docs rewritten for CLI vs RPC split
- Audit open question closed with WEFT-494 / ADR-070 reference
- Plane inventory row → Done

## Files

| Path | Change |
|------|--------|
| `docs/adr/adr-070-mcp-registry-ownership.md` | **new** decision |
| `docs/adr/README.md` | Index row |
| `crates/clawft-weave/src/mcp_rpc.rs` | **new** handlers + tests |
| `crates/clawft-weave/src/lib.rs` | export `mcp_rpc` |
| `crates/clawft-weave/src/daemon.rs` | seed + dispatch arms; test ChainConfig `external_anchor` |
| `crates/clawft-weave/src/capability.rs` | method classes + test |
| `crates/clawft-weave/Cargo.toml` | `clawft-services` dep |
| `crates/clawft-services/src/mcp/discovery.rs` | ownership docs |
| `crates/clawft-cli/src/commands/mcp_cmd.rs` | list RPC order + docs |
| `.planning/reviews/0.7.0-release-gate/15-mcp-integration.md` | open Q closed |
| `docs/plans/plane-board-inventory.md` | WEFT-494 Done |
| `docs/plans/wave-0j-WEFT-494-result.md` | this report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Decision recorded (ADR) on per-process vs daemon-managed | **Yes** — hybrid ADR-070 |
| If daemon-managed: `mcp.add`/`mcp.list`/`mcp.remove` on dispatch | **Yes** (+ `mcp.reload`, `tools.mcp` alias) |
| CLI stays durable-config owner (WEFT-188); proxies reload | **Yes** |
| discovery.rs docs updated | **Yes** |
| Audit row closed with WEFT-N | **Yes** |

## Verification

```text
scripts/build.sh test clawft-weave
# Summary: 189 tests run: 189 passed, 1 skipped

cargo test -p clawft-weave --lib mcp_rpc
# 5 passed

cargo test -p clawft-weave --lib mcp_registry_verbs
# 1 passed

cargo test -p clawft-cli mcp
# unit + cli_integration mcp_* green
```

## Residual / follow-ups

- Wire `start_config_watcher` at daemon boot (WEFT-187 API ready; WEFT-493).
- Optional CLI `--runtime-only` to call `mcp.add`/`mcp.remove` without
  touching the config file.
- VSCode `ALLOWED_METHODS` may add `mcp.list` when the panel needs live MCP
  status (hand-maintained allowlist).
