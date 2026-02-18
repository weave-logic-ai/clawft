# Stream 3H-C: CompositeToolProvider + McpServerShell

## Files Created

- `crates/clawft-services/src/mcp/composite.rs` (~100 lines)
- `crates/clawft-services/src/mcp/server.rs` (~265 lines)

## Files Modified

- `crates/clawft-services/src/mcp/mod.rs` -- added `pub mod composite;` and `pub mod server;`

## Design Decisions

### CompositeToolProvider

- Aggregates multiple `ToolProvider` implementations behind a single interface.
- Tool names are prefixed with `"{namespace}__{tool_name}"` when listed via `list_tools_all()`.
- Routing splits on the first `"__"` separator to find the correct provider by namespace.
- If no `"__"` separator is present, providers are tried in registration order (first match wins).
- Non-`NotFound` errors short-circuit the fallback loop (e.g., `ExecutionFailed` stops iteration).
- Implements `Default` for ergonomic construction.

### McpServerShell

- Generic over `AsyncBufRead + AsyncWrite` so it can be driven by stdio, TCP, or in-memory buffers.
- Reads newline-delimited JSON-RPC messages in a loop until EOF.
- Handles `initialize` handshake: sets `initialized` flag, returns protocol version + capabilities + server info.
- `notifications/initialized` is a no-op (notification, no response).
- Requests before initialization receive JSON-RPC error code `-32002`.
- `tools/list` returns tools from `CompositeToolProvider::list_tools_all()`, filtered through middleware `filter_tools`.
- `tools/call` dispatches through middleware `before_call`, then to `CompositeToolProvider::call_tool()`, then middleware `after_call`.
- Tool execution errors are returned as `CallToolResult` with `is_error: true` (not as JSON-RPC errors), following MCP protocol conventions.
- Unknown methods return JSON-RPC error code `-32601`.
- Malformed JSON returns error code `-32600`.
- Notifications (messages without `id`) never produce a response.
- Empty lines are silently skipped.

### Integration with Sibling Modules

- Uses `ToolProvider` trait from `provider.rs` (created by agent 3H-B).
- Uses `Middleware` trait and `ToolCallRequest` from `middleware.rs` (created by agent 3H-D).
- Uses `ToolDefinition` from `mod.rs` (pre-existing).
- Uses `CallToolResult`, `ContentBlock`, `ToolError` from `provider.rs`.

## Test Coverage

### composite.rs (10 tests)

- `default_is_empty` -- empty provider has no tools
- `list_tools_all_prefixes_names` -- verifies namespace prefixing
- `list_tools_all_preserves_descriptions` -- descriptions survive prefixing
- `call_tool_routes_by_namespace` -- correct dispatch to alpha/beta
- `call_tool_unknown_namespace_returns_not_found` -- unknown namespace errors
- `call_tool_unknown_tool_in_namespace_returns_not_found` -- tool not in provider
- `call_tool_without_namespace_tries_all` -- fallback across all providers
- `call_tool_without_namespace_not_found` -- empty provider returns not found
- `provider_count` -- registration counting
- `call_tool_propagates_non_not_found_errors` -- ExecutionFailed stops iteration

### server.rs (12 tests)

- `initialize_handshake` -- correct handshake response
- `not_initialized_rejection` -- -32002 before init
- `tools_list_returns_tools` -- tools/list after init
- `tools_call_routes_correctly` -- tools/call dispatches and returns content
- `tools_call_not_found` -- missing tool returns is_error result
- `unknown_method_returns_error` -- -32601 for unknown methods
- `notification_produces_no_response` -- notifications don't generate output
- `malformed_json_returns_parse_error` -- -32600 for bad JSON
- `empty_lines_are_skipped` -- whitespace-only lines ignored
- `middleware_filter_tools_applied` -- filter_tools hook works
- `middleware_before_call_can_reject` -- before_call rejection
- `middleware_after_call_applied` -- after_call modification
- `full_session_flow` -- complete init, list, call, unknown sequence

## Verification

- `cargo test -p clawft-services --lib` -- 144 tests pass (22 new)
- `cargo clippy -p clawft-services -- -D warnings` -- clean (excluding pre-existing clawft-types warning)
