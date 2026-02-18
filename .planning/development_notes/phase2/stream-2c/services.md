# Stream 2C: Services Crate -- Development Notes

**Agent**: services-engineer (coder)
**Date**: 2026-02-17
**Phase**: 2 / Stream 2C
**Crate**: `clawft-services`
**Modules**: `cron_service/`, `heartbeat/`, `mcp/`, `error`

## Summary

Implemented the `clawft-services` crate from scratch (was a stub with one comment line). Contains three services: `CronService` for scheduled job execution, `HeartbeatService` for periodic health pings, and `McpClient` for Model Context Protocol tool integration. Includes a unified `ServiceError` type with 8 variants.

## Files Written

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 10 | Module declarations |
| `error.rs` | 82 | `ServiceError` (8 variants) + `Result<T>` alias |
| `cron_service/mod.rs` | 311 | `CronService` -- add/remove/enable/disable/list/run_job_now/start |
| `cron_service/scheduler.rs` | 299 | `CronScheduler` -- HashMap-backed, cron expression validation |
| `cron_service/storage.rs` | 300 | JSONL append-only persistence (create/update/delete events) |
| `heartbeat/mod.rs` | 172 | `HeartbeatService` -- configurable interval, `CancellationToken` |
| `mcp/mod.rs` | 282 | `McpClient` -- `list_tools`/`call_tool`, `ToolDefinition` |
| `mcp/transport.rs` | 254 | `McpTransport` trait, `StdioTransport`, `HttpTransport`, `MockTransport` |
| `mcp/types.rs` | 140 | JSON-RPC 2.0 wire types (`JsonRpcRequest`, `JsonRpcResponse`, etc.) |

**Total new lines**: ~1,850

## Architecture Decisions

### No circular dependency on clawft-core
The SPARC plan had CronService trigger messages via the MessageBus. Rather than
adding a dependency on `clawft-core` (which would create a cycle), CronService
accepts an `mpsc::UnboundedSender<InboundMessage>` directly. The bootstrap code
in `clawft-core::bootstrap` can pass `bus.inbound_sender()` when wiring up the
service.

### JSONL append-only persistence
Cron job storage uses a JSONL (JSON Lines) append-only log. Each line is a
`StorageEvent` enum: `Created { job }`, `Updated { job }`, or `Deleted { id }`.
The current state is reconstructed by replaying the log on startup. This is
simpler and more resilient than a mutable JSON file (no partial-write corruption).

### 7-field cron expressions
The `cron` crate v0.15 uses 7-field format: `sec min hour dom month dow year`.
Users typically expect 5-field format. The `CronScheduler` validates expressions
through the `cron::Schedule` parser, and the CLI layer in stream 2D provides a
`normalize_cron_expr()` helper that prepends `0 ` and appends ` *` to 5-field
expressions.

### HeartbeatService with CancellationToken
Uses `tokio_util::sync::CancellationToken` for graceful shutdown. The heartbeat
loop runs at a configurable interval (default 60s) and invokes a callback on each
tick. The service is `Send + Sync` and can be spawned as a background task.

### MCP transport abstraction
`McpTransport` is an async trait with a single method: `send(request) -> response`.
Three implementations are provided:
- `StdioTransport`: Spawns a child process, writes JSON-RPC to stdin, reads from stdout
- `HttpTransport`: POST JSON-RPC to an HTTP endpoint
- `MockTransport`: In-memory response queue for testing

### JSON-RPC 2.0 compliance
All MCP wire types follow JSON-RPC 2.0 exactly: `jsonrpc: "2.0"`, `id`, `method`,
`params` for requests; `result` or `error` (with `code`/`message`/`data`) for
responses. The `McpClient` validates the `jsonrpc` field and maps protocol errors
to `ServiceError::McpProtocol`.

## Dependencies Added

- `cron` (workspace): Cron expression parsing and schedule computation
- `tokio-util` (workspace): `CancellationToken` for graceful service shutdown
- `reqwest` (workspace): HTTP transport for MCP client
- `uuid` (workspace): Job ID generation

## Test Coverage (59 tests)

- **error.rs** (3 tests): Display formatting, `From<io::Error>`, `From<serde_json::Error>`
- **cron_service/mod.rs** (12 tests): Add/remove/enable/disable/list jobs, duplicate names, run_job_now
- **cron_service/scheduler.rs** (10 tests): Parse expressions, next_fire_time, invalid expressions, job lifecycle
- **cron_service/storage.rs** (11 tests): Append events, replay log, create/update/delete roundtrip, empty file
- **heartbeat/mod.rs** (8 tests): Start/stop, interval configuration, callback invocation, cancellation
- **mcp/mod.rs** (6 tests): List tools, call tool, error handling, malformed responses
- **mcp/transport.rs** (5 tests): MockTransport FIFO, empty queue error, request/response serialization
- **mcp/types.rs** (4 tests): JSON-RPC request/response serde roundtrips, error object

## Quality Gates

| Check | Result |
|-------|--------|
| `cargo build -p clawft-services` | PASS |
| `cargo test -p clawft-services` | PASS (59 tests, 0 failures) |
| `cargo clippy -p clawft-services -- -D warnings` | PASS (0 warnings) |

## Integration Points

- **clawft-types::config**: `CronJobConfig`, `MCPServerConfig` from config schema
- **clawft-types::event**: `InboundMessage` for cron-triggered messages
- **clawft-core::bootstrap**: Wire `CronService` and `HeartbeatService` into `AppContext`
- **clawft-cli**: `weft cron` subcommands interact with `CronService` storage
- **clawft-core::tools**: MCP tools discovered via `McpClient::list_tools()` can be registered in `ToolRegistry`

## Next Steps

1. Wire `CronService` into `AppContext` bootstrap
2. Wire `McpClient` tool discovery into `ToolRegistry`
3. Add cron job execution history tracking
4. Add MCP server health monitoring via `HeartbeatService`
5. Implement `SseTransport` for server-sent events MCP connections
