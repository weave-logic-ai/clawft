# Stream 3H-B: ToolProvider Trait + BuiltinToolProvider

## Deliverables

**File created**: `crates/clawft-services/src/mcp/provider.rs`

**Module registered in**: `crates/clawft-services/src/mcp/mod.rs`

## Types Introduced

| Type | Purpose |
|------|---------|
| `ContentBlock` | Tagged enum for tool output blocks (currently `Text` variant) |
| `CallToolResult` | MCP-compatible result with `content` blocks and `is_error` flag |
| `ToolError` | Error enum: `NotFound`, `ExecutionFailed`, `PermissionDenied` |
| `ToolProvider` | Async trait abstracting a source of tools |
| `BuiltinToolProvider` | Implementation wrapping local tools via a dispatch closure |

## Design Decisions

1. **Decoupled from ToolRegistry**: `BuiltinToolProvider` does not depend directly on `clawft_core::tools::registry::ToolRegistry`. Instead it accepts a `Vec<ToolDefinition>` and a dispatcher closure. This keeps the dependency graph clean (clawft-services does not depend on clawft-tools or clawft-core's tool registry) and makes testing trivial.

2. **Separate ToolError from core**: The `provider::ToolError` is intentionally distinct from `clawft_core::tools::registry::ToolError`. The provider error covers the abstraction layer (not found in provider, execution failed at dispatch boundary, permission denied). Conversion between the two error types will be handled at the integration site when wiring `BuiltinToolProvider` to the real `ToolRegistry`.

3. **Dispatcher returns Result<String, String>**: The dispatch function signature uses `Result<String, String>` (not `Result<Value, ToolError>`) so that `BuiltinToolProvider` can wrap results uniformly into `CallToolResult::text()` / `CallToolResult::error()`. The JSON serialization/deserialization of tool output is handled by the caller or the tool itself.

4. **MCP-compatible serde**: `CallToolResult` uses `#[serde(rename = "isError")]` and `ContentBlock` uses `#[serde(tag = "type")]` to match the MCP wire format exactly.

5. **Existence check before dispatch**: `call_tool()` verifies the tool name exists in the definitions list before calling the dispatcher, returning `ToolError::NotFound` early. This prevents dispatching to unknown tools.

## Re-exports

From `crates/clawft-services/src/mcp/mod.rs`:
```rust
pub use provider::{BuiltinToolProvider, CallToolResult, ContentBlock, ToolError, ToolProvider};
```

## Tests (12 total)

- `namespace_returns_builtin` - verifies namespace is "builtin"
- `list_tools_returns_registered_tools` - verifies tool listing
- `call_tool_dispatches_correctly` - echo tool dispatch
- `call_tool_add_dispatches_correctly` - add tool dispatch
- `call_tool_not_found` - unknown tool returns ToolError::NotFound
- `call_tool_dispatcher_error_returns_error_result` - dispatcher Err wraps to CallToolResult with is_error=true
- `call_tool_result_text_convenience` - CallToolResult::text() helper
- `call_tool_result_error_convenience` - CallToolResult::error() helper
- `call_tool_result_serde_roundtrip` - JSON serialization roundtrip
- `content_block_serde_roundtrip` - ContentBlock JSON roundtrip
- `call_tool_result_is_error_defaults_false` - is_error defaults to false when absent
- `tool_error_display` - error Display implementations
- `debug_format` - Debug impl for BuiltinToolProvider

## Verification

```
cargo test -p clawft-services   # 90 passed (12 new)
cargo clippy -p clawft-services -- -D warnings  # 0 warnings
```
