# Stream 3H-D: MCP Middleware Pipeline

## File

`crates/clawft-services/src/mcp/middleware.rs`

## Summary

Implemented a composable middleware pipeline for MCP tool operations. The module provides a `Middleware` trait with three hooks (`filter_tools`, `before_call`, `after_call`) and four concrete implementations:

### Components

1. **`Middleware` trait** - Async trait with default pass-through implementations for all three hooks. Any middleware only needs to override the hooks it cares about.

2. **`SecurityGuard`** - Enforces command and URL safety policies:
   - For tools with "exec" or "shell" in the name: validates the `command` arg against `CommandPolicy`
   - For tools with "fetch" or "search" in the name: validates the `url` arg against `UrlPolicy`
   - Returns `ToolError::PermissionDenied` on rejection

3. **`PermissionFilter`** - Restricts visible tools to an allowlist. `None` means all tools are allowed; `Some(list)` restricts to only those names.

4. **`ResultGuard`** - Truncates text content blocks that exceed `max_bytes` (default 64KB). Appends `\n[truncated]` sentinel. Respects char boundaries.

5. **`AuditLog`** - Logs tool name, args summary, error status, and content size at `info!` level. Pure passthrough; does not modify data.

### Design Decisions

- **Local policy types**: Since `clawft-services` does not depend on `clawft-core` or `clawft-tools`, simplified `CommandPolicy` and `UrlPolicy` structs are defined locally. They mirror the essential validation logic from `clawft_tools::security_policy::CommandPolicy` and `clawft_tools::url_safety::UrlPolicy`.
- **Lightweight URL parsing**: A local `extract_host` function avoids pulling in the `url` crate. It handles scheme, userinfo, port, and IPv6 bracket notation.
- **Types from provider.rs**: Reuses `CallToolResult`, `ContentBlock`, and `ToolError` from the sibling `provider` module. Reuses `ToolDefinition` from `mcp/mod.rs`.

### Test Coverage

31 tests covering:
- SecurityGuard: rejects disallowed commands, dangerous patterns, private IPs, metadata endpoints; allows safe commands and public URLs; ignores unrelated tools
- PermissionFilter: allowlist filtering, None passthrough, empty allowlist, multi-tool allowlist
- ResultGuard: truncation of oversized content, passthrough of short content, preservation of is_error flag
- AuditLog: passthrough for before_call, after_call, and filter_tools
- Helper functions: extract_host edge cases, CommandPolicy validation, UrlPolicy validation, JSON summarization

### Verification

- `cargo test -p clawft-services` -- 121 tests pass (31 new + 90 existing)
- `cargo clippy -p clawft-services` -- zero warnings from middleware.rs
