# ADR-070: MCP server registry ownership — CLI durable config vs daemon runtime

- **Status**: Accepted (2026-07-30)
- **Closes**: WEFT-494
- **Related**: WEFT-188 (CLI `weft mcp add/list/remove`), WEFT-187 (config
  hot-reload watcher), ADR-054 (claude-flow user-install path),
  `crates/clawft-services/src/mcp/discovery.rs` (`McpServerManager`)

## Context

`McpServerManager` lives in `clawft-services` and exposes runtime
add/remove/list plus drain-and-swap hot-reload. The operator CLI
(`weft mcp add|list|remove`, WEFT-188) mutates durable config under
`tools.mcp_servers` / `tools.mcpServers`. The audit left an open question:
are MCP servers managed **per-process** (one CLI invocation = one config
edit) or **through the daemon** (shared registry, hot-reload)? Doc comments
on `McpServerManager` implied CLI verbs while the daemon dispatch had no
`mcp.*` arms, so ownership depended on which crate you were reading.

## Decision

**Hybrid ownership — config is durable truth; the daemon owns the live
registry.**

| Surface | Owns | Does not own |
|---------|------|--------------|
| **CLI** (`weft mcp add/list/remove`, WEFT-188) | Atomic writes to the config file; best-effort `mcp.reload` after mutation | Live transport sessions / in-flight drain |
| **Daemon RPC** (`mcp.add` / `mcp.list` / `mcp.remove` / `mcp.reload`) | Shared in-process `McpServerManager` for the running kernel | Permanent config persistence (callers that need durability still edit config via CLI or file) |

### Preferred operator path

```text
weft mcp add <name> …     # write tools.mcp_servers (durable)
  → mcp.reload            # daemon apply_config_diff from config path
  → (or WEFT-187 watcher) # file change → same apply path
```

### Programmatic / panel path

```text
mcp.add / mcp.remove      # mutate live registry only
mcp.list / tools.mcp      # inspect live status + tools
mcp.reload { path? }      # re-read durable config into live registry
```

`tools.mcp` is an alias of `mcp.list` for CLI callers that already probe
that method name (WEFT-188 list fallback).

## Rationale

1. **Durability belongs on disk.** A daemon restart must rehydrate from
   config. Putting sole authority on ephemeral RPC would lose servers on
   crash/restart.
2. **Runtime must be shared.** Agents, tools, and the panel all talk to one
   kernel process. Per-CLI-process registries cannot share live sessions or
   drain state with the daemon.
3. **CLI stays offline-capable.** `weft mcp` works without a running daemon
   (edit config + print reload hint). RPC is best-effort enhancement, not a
   hard dependency of the operator UX.
4. **Matches WEFT-188 notes.** That ticket explicitly deferred
   `mcp.add|list|remove` RPCs; this ADR completes that split rather than
   collapsing everything into either layer.

## Capability classes

| Method | Capability |
|--------|------------|
| `mcp.list`, `tools.mcp` | Read |
| `mcp.add`, `mcp.remove`, `mcp.reload` | Write |

Local UDS `DaemonClient` continues to attach admin auth (filesystem
socket trust), so operator CLI keep working after WEFT-479 gating.

## Implications

- Daemon boot seeds `McpServerManager` from the loaded `Config.tools.mcp_servers`.
- `mcp.add` / `mcp.remove` do **not** rewrite config files; operators who
  need persistence use `weft mcp` (or edit the file) then `mcp.reload`.
- `mcp.reload` without `path` walks standard config locations
  (`CLAWFT_CONFIG`, cwd `clawft.toml` / `weave.toml`, `~/.clawft/config.json`).
- Discovery module docs must state this split explicitly so future readers
  do not re-open the ownership question.
- VSCode `ALLOWED_METHODS` may add `mcp.list` when the panel needs live
  MCP status (hand-maintained allowlist; not required for CLI).

## Followups

- Wire `start_config_watcher` at daemon boot against the resolved config
  path (complements WEFT-187 / WEFT-493).
- Optional: CLI flag `--runtime-only` that proxies add/remove to RPC without
  touching the file (out of scope for WEFT-494).
