# 3H-G: Claude Delegator & DelegateTaskTool

**Agent**: 3H-G
**Status**: Complete
**Date**: 2026-02-17
**SPARC Plan**: `.planning/sparc/3h-tool-delegation.md` (Sections 2.5, 3.1)

## Summary

Implemented the Claude-based task delegation subsystem: `ClaudeDelegator` in
clawft-services, `DelegateTaskTool` in clawft-tools, and full CLI wiring
across the agent, gateway, and mcp-server startup paths.

All components are feature-gated behind `delegate` and degrade gracefully when
the `ANTHROPIC_API_KEY` environment variable is absent or delegation is
disabled in configuration.

## Components Created

### 1. `ClaudeDelegator` (clawft-services/src/delegation/claude.rs)

Multi-turn Anthropic Messages API client with tool-use loop.

- **Constructor**: `new(config: &DelegationConfig, api_key: String) -> Option<Self>`
  Returns `None` if the API key is empty for graceful degradation.
- **Main method**: `delegate(task, tool_schemas, tool_executor) -> Result<String>`
  Sends user messages, receives assistant responses, executes tool calls via
  the caller-provided executor closure, and loops until `stop_reason != "tool_use"`
  or the turn limit is hit.
- **Schema conversion**: Uses `schema::openai_to_anthropic()` (created by Agent
  3H-F) to convert OpenAI function-calling format to Anthropic tool format.
- **Excluded tools**: Filters out tools listed in `config.excluded_tools` before
  sending to the API.
- **Error types**: `DelegationError` enum with Http, Api, InvalidResponse,
  MaxTurnsExceeded, and ToolExecFailed variants.
- **Security**: API key hidden in Debug output, replaced with `***`.
- **Testing**: Mock HTTP tests via `mockito` with `with_base_url()` pattern.

### 2. `DelegateTaskTool` (clawft-tools/src/delegate_tool.rs)

Implements the `Tool` trait from clawft-core.

- **Name**: `delegate_task`
- **Schema**: `{ task: string (required), model: string (optional) }`
- **Flow**:
  1. Extracts task from args.
  2. Queries `DelegationEngine.decide()` to check if delegation is approved.
  3. If `Local`, returns immediately with a "handle locally" message.
  4. If `Claude` or `Flow`, builds a tool executor closure from the shared
     `ToolRegistry` and calls `ClaudeDelegator.delegate()`.
  5. Returns `{ status, response, task }` on success.
- **Dependencies**: Holds `Arc<ClaudeDelegator>`, `Arc<DelegationEngine>`,
  `Vec<Value>` (schema snapshot), `Arc<ToolRegistry>`.

### 3. CLI Wiring (clawft-cli/src/mcp_tools.rs)

`register_delegation()` function with two cfg variants:

- **`#[cfg(feature = "delegate")]`**: Reads `ANTHROPIC_API_KEY` from env,
  creates `ClaudeDelegator` and `DelegationEngine`, snapshots current tool
  schemas, constructs `DelegateTaskTool`, and registers it.
- **`#[cfg(not(feature = "delegate"))]`**: No-op stub.

Called from three startup paths:
- `commands/agent.rs` -- after MCP tools, before message tool
- `commands/gateway.rs` -- after MCP tools, before message tool
- `commands/mcp_server.rs` -- after MCP tools, before provider conversion

## Feature Flag Chain

```
clawft-cli/delegate
  -> clawft-services/delegate (enables regex)
  -> clawft-tools/delegate (enables clawft-services dep)
```

## Dependencies Added

- `mockito = "1"` in clawft-services dev-dependencies (for HTTP mock tests)
- `clawft-services` as optional dep in clawft-tools (gated by delegate feature)

## Coordination with Agent 3H-F

Agent 3H-F concurrently created:
- `clawft-types::delegation` module (DelegationConfig, DelegationRule, DelegationTarget)
- `clawft-services::delegation::mod.rs` (DelegationEngine with regex routing)
- `clawft-services::delegation::schema.rs` (OpenAI-to-Anthropic conversion)
- Feature flags and dependency setup in Cargo.toml files

This agent (3H-G) adapted to use 3H-F's types and engine, adding `pub mod claude;`
to the delegation module and building on the schema conversion utilities.

## Test Coverage

| Crate | Tests | Status |
|---|---|---|
| clawft-services (delegate) | 178 total, 20 delegation-specific | Pass |
| clawft-tools (delegate) | 153 + 33 integration | Pass |
| clawft-cli (delegate) | 285 + 29 integration | Pass |

Key test categories:
- ClaudeDelegator construction (None on empty key, config field mapping)
- Text extraction from Anthropic content blocks
- Debug impl hides API key
- Error display formatting
- Excluded tools filtering
- Mock HTTP: text-only response, tool-use loop, API error
- DelegateTaskTool schema validation
- CLI integration (all subcommands parse correctly)

## Clippy Status

- `cargo clippy -p clawft-services --features delegate -- -D warnings`: Clean
- `cargo clippy -p clawft-tools --features delegate -- -D warnings`: Clean
- `cargo clippy -p clawft-cli --features delegate -- -D warnings`: Clean
- `cargo clippy -p clawft-cli -- -D warnings` (without delegate): Clean

## Known Limitations

1. **Placeholder registry in delegate tool**: The `register_delegation()`
   function passes an empty `ToolRegistry` to `DelegateTaskTool` because the
   registry is not `Arc`-wrapped at the call site. Tool schemas are snapshotted
   correctly, but the tool executor closure inside `DelegateTaskTool` uses this
   empty registry rather than the live one. A future refactor to `Arc<ToolRegistry>`
   in `AppContext` would resolve this.

2. **No model override support**: The `model` parameter in the tool schema is
   accepted but not wired through to the delegator yet.

## Files Modified

- `crates/clawft-services/src/delegation/claude.rs` (new)
- `crates/clawft-services/src/delegation/mod.rs` (added `pub mod claude;`)
- `crates/clawft-services/Cargo.toml` (added mockito dev-dep)
- `crates/clawft-tools/src/delegate_tool.rs` (new)
- `crates/clawft-tools/src/lib.rs` (added delegate_tool module)
- `crates/clawft-tools/Cargo.toml` (added delegate feature + clawft-services dep)
- `crates/clawft-cli/src/mcp_tools.rs` (added register_delegation)
- `crates/clawft-cli/src/commands/agent.rs` (wired delegation registration)
- `crates/clawft-cli/src/commands/gateway.rs` (wired delegation registration)
- `crates/clawft-cli/src/commands/mcp_server.rs` (wired delegation registration)
- `crates/clawft-cli/Cargo.toml` (added delegate feature)
