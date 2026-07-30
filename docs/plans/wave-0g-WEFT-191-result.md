# WEFT-191 result — McpSession durability (keepalive, reconnect, is_alive, cancel)

**Ticket:** WEFT-191  
**Branch:** `wave0g/weft-191-mcp-session`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb489-7de2-72f2-89c6-2baccd974257`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-191 (wave-0g)

## Problem

`McpSession::connect()` was a one-shot handshake. Long gateway sessions had:

- no `is_alive()` on `StdioTransport` (child crash went undetected until write/hang)
- no reconnect-with-backoff
- no MCP `ping` / keepalive loop
- no graceful `notifications/cancelled` on shutdown

Source: 3H MAJ-03 / audit deferred item #18.

## What shipped

### Transport — `clawft-services::mcp::transport`

| Item | Detail |
|------|--------|
| `McpTransport::is_alive` | Default `true`; `StdioTransport` uses `child.try_wait()` |
| `McpTransport::close` | Default no-op; stdio kills + reaps child, clears pending waiters |
| `StdioTransport` spawn config | Retains command/args/env; `to_transport_spec()`, `kill_on_drop(true)` |
| Fail-fast sends | `send_request` / `send_notification` reject when child is dead |
| `TransportReconnect` + `SpecReconnect` | Trait for session reconnect; factory+spec production adapter |
| `MockTransport` | `set_alive`, `close`, `SharedMockTransport` for post-session asserts |

### Session — `clawft-services::mcp` (`mod.rs`)

| Item | Detail |
|------|--------|
| `DurabilityConfig` | keepalive interval, max attempts, initial/max backoff; `default` / `disabled` / `for_tests` |
| `McpSession::connect_with_options` | transport + optional `Arc<dyn TransportReconnect>` + durability |
| `is_alive` / `ping` | Transport liveness + MCP `ping` method |
| `ensure_connected` | Dead peer → `reconnect_with_backoff` (used by `list_tools` / `call_tool`) |
| `reconnect_with_backoff` | Exponential backoff, serialized via mutex, re-runs initialize handshake |
| `start_keepalive` | Background interval: ping → on failure reconnect (if configured) |
| `shutdown` | `notifications/cancelled` for in-flight IDs, abort keepalive, `transport.close()` |
| `McpClient` transport slot | `Mutex<Arc<dyn McpTransport>>` so reconnect can swap without holding lock across I/O |

### Defaults

| Constant | Value |
|----------|-------|
| `DEFAULT_KEEPALIVE_INTERVAL` | 30s |
| `DEFAULT_MAX_RECONNECT_ATTEMPTS` | 5 |
| `DEFAULT_INITIAL_BACKOFF` | 100ms |
| `DEFAULT_MAX_BACKOFF` | 10s |

Keepalive does **not** auto-start on `connect`; callers that want a long-lived loop call `start_keepalive` on an `Arc<McpSession>`.

## Acceptance

| Criterion | Status |
|-----------|--------|
| `is_alive()` on StdioTransport | **Yes** |
| Reconnect-with-backoff in McpSession | **Yes** |
| Periodic ping/keepalive | **Yes** (`start_keepalive` + `ping`) |
| Graceful `notifications/cancelled` on shutdown | **Yes** |
| Tests including child-crash recovery | **Yes** (stdio exit + mock reconnect path) |
| `scripts/build.sh test clawft-services` | **Yes** — **335 passed** |

## Gaps / follow-ups

| Gap | Notes |
|-----|-------|
| Gateway auto-wire | Runtime/daemon should pass `SpecReconnect` + call `start_keepalive` for long-lived MCP servers |
| Handshake field refresh | After reconnect, public `server_info` / `protocol_version` keep first-connect values (transport is swapped; metadata is diagnostic) |
| `listChanged` | Still deferred (WEFT-200) |
| In-flight cancel coverage | Cancel emits for tracked IDs; mid-request hang cancel relies on caller/timeout paths |

## Tests

```bash
scripts/build.sh test clawft-services
# → 335 passed, 0 failed
```

Key new tests:

- `stdio_is_alive_while_child_running` / `stdio_is_alive_false_after_child_exits`
- `session_ping_sends_ping_method`
- `session_is_alive_false_when_transport_dead`
- `session_shutdown_sends_cancelled_and_closes`
- `session_reconnect_with_backoff_recovers_from_crash`
- `session_ensure_connected_reconnects_then_lists_tools`
- `session_reconnect_exhausts_attempts`
- `session_keepalive_triggers_reconnect_on_dead_peer`
- `child_crash_recovery_stdio_is_alive_and_recreate`

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-services/src/mcp/transport.rs` | is_alive/close, SpecReconnect, mock/shared, stdio tests |
| `crates/clawft-services/src/mcp/mod.rs` | DurabilityConfig, session reconnect/keepalive/shutdown, tests |
| `docs/plans/wave-0g-WEFT-191-result.md` | This report |

## How to test

```bash
# Package tests (includes all WEFT-191 cases)
scripts/build.sh test clawft-services

# Focused filters
cargo test -p clawft-services --lib mcp::tests::session_
cargo test -p clawft-services --lib mcp::transport::tests::stdio_
```

## Commit

See git log on `wave0g/weft-191-mcp-session`.
