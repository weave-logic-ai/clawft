# Development Notes: `weft mcp-server` CLI Command

**Agent**: 3H-E (MCP Server Command)
**Status**: Complete (with TODO markers for sibling agent integration)
**Date**: 2026-02-17

## Files Created / Modified

### Created
- `crates/clawft-cli/src/commands/mcp_server.rs` (~280 lines)
  - `McpServerArgs` struct (clap Args derive)
  - `run()` async entry point
  - `build_tool_definitions()` helper
  - `build_builtin_provider()` helper
  - `run_stdio_loop()` minimal JSON-RPC server
  - `write_response()` stdout helper
  - Unit tests (6 tests)

### Modified
- `crates/clawft-cli/src/main.rs`
  - Added `McpServer` variant to `Commands` enum
  - Added dispatch: `Commands::McpServer(args) => commands::mcp_server::run(args).await?`
  - Updated `cli_has_all_subcommands` test to include `"mcp-server"`
  - Added 3 new CLI parsing tests
- `crates/clawft-cli/src/commands/mod.rs`
  - Added `pub mod mcp_server;`

## Architecture Decisions

### 1. Tool Registry Reuse
The command reuses the exact same `register_all` + `register_mcp_tools` pattern
from `agent.rs` and `gateway.rs`. This ensures the MCP server exposes the same
tools available in interactive and gateway mode.

### 2. BuiltinToolProvider Bridge
The `ToolRegistry` (from `clawft-core`) is bridged to `BuiltinToolProvider`
(from `clawft-services::mcp::provider`) via:
- Converting registry schemas to `Vec<ToolDefinition>`
- Wrapping `registry.execute()` in a dispatcher closure
- The registry is moved into an `Arc` for shared ownership

### 3. Inline Stdio Loop (Temporary)
A minimal inline JSON-RPC stdio server handles:
- `initialize` -- returns server capabilities
- `tools/list` -- lists all registered tools
- `tools/call` -- dispatches to the BuiltinToolProvider
- Notifications -- silently acknowledged
- Unknown methods -- returns -32601 error

This is temporary until Agent 3H-D completes `McpServerShell`.

## TODO Items (Waiting on Sibling Agents)

### Agent 3H-C: CompositeToolProvider
Once `composite.rs` exists, replace the single `BuiltinToolProvider` with a
`CompositeToolProvider` that aggregates multiple providers.

### Agent 3H-D: McpServerShell + Middleware
Once `server.rs` and `middleware.rs` exist, replace `run_stdio_loop()` with:
```rust
let middleware: Vec<Box<dyn Middleware>> = vec![
    Box::new(SecurityGuard::new(/* from config */)),
    Box::new(PermissionFilter::new(config.tools.allowed_tools.clone())),
    Box::new(ResultGuard::new(64 * 1024)),
    Box::new(AuditLog),
];
let shell = McpServerShell::new(composite, middleware);
shell.run(stdin, stdout).await?;
```

## Test Results

```
cargo test -p clawft-cli  -- 193 passed, 0 failed
cargo test -p clawft-cli  -- 29 integration tests passed
cargo clippy -p clawft-cli  -- 0 warnings (clawft-cli scope)
```

## MCP Protocol Support

The server currently supports JSON-RPC 2.0 over stdio (newline-delimited):
- Protocol version: `2025-06-18`
- Capabilities: `tools.listChanged: false`
- Server info: `clawft` + `CARGO_PKG_VERSION`
