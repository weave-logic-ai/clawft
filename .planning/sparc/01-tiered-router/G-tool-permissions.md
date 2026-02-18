# SPARC Implementation Plan: Phase G - Tool Permission Enforcement

## Agent Instructions

### Context

This is Phase G of the Tiered Router sprint. The clawft project is a Rust AI assistant framework. The CLI binary is named `weft`.

### Dependencies

**Depends on Phase B (UserPermissions) completing first:**
- `UserPermissions` struct with `tool_access: Vec<String>` and `tool_denylist: Vec<String>` fields
- `AuthContext` struct with `permissions: UserPermissions` field
- `PermissionResolver` and merge logic

Phase F (AuthContext threading) should also be complete or in progress so that `ChatRequest` carries an `auth_context` field that the agent loop can pass to tool execution.

### Planning Documents to Reference

- `.planning/08-tiered-router.md` -- Section 8.3 (Plugin-Provided Permissions), Section 9.4 (Tool Access Enforcement in ToolRegistry)
- `.planning/sparc/01-tiered-router/B-permissions-resolution.md` -- UserPermissions with `tool_access` and `tool_denylist` fields
- `crates/clawft-core/src/tools/registry.rs` -- Existing `ToolRegistry` and `Tool` trait
- `crates/clawft-core/src/pipeline/traits.rs` -- Pipeline types, `ChatRequest`

### Files to Modify

- `crates/clawft-core/src/tools/registry.rs` (extend `execute` signature, add permission checks, add `ToolMetadata`, add `metadata()` to `Tool` trait)
- `crates/clawft-core/src/agent/loop_core.rs` (thread permissions into tool execution call)

### Branch

Work on branch: `weft/tiered-router`

---

## 1. Specification

### 1.1 Problem

The current `ToolRegistry::execute()` dispatches tool calls unconditionally. Any caller can invoke any registered tool. Once the tiered router introduces permission levels (zero_trust, user, admin), tool access must be gated by the caller's `UserPermissions`. The existing per-tool security layer (`CommandPolicy` for shell/spawn, `UrlPolicy` for web_fetch) validates inputs within a tool execution. Tool permission enforcement is a separate, outer concern: it decides whether a caller can invoke the tool at all, before the tool implementation ever runs.

### 1.2 Requirements

1. **Extend `ToolRegistry::execute()` to accept `&UserPermissions`.** The new signature is `execute(name, args, permissions: Option<&UserPermissions>)`. When `permissions` is `None`, all tools are allowed (backward compatibility for `StaticRouter` mode, CLI without routing config, and unit tests).

2. **Check `tool_access` allowlist.** The `tool_access` field on `UserPermissions` controls which tools a user can invoke:
   - `["*"]` = all tools are allowed (admin default).
   - `["read_file", "write_file", ...]` = only those named tools, checked by exact match or glob pattern.
   - `[]` (empty) = no tools are allowed (zero_trust default).

3. **Support glob patterns in `tool_access`.** Patterns like `"file_*"` match tool names like `"file_read"` and `"file_write"`. Patterns like `"myserver__*"` match all MCP tools from `myserver`. Use Unix-style glob matching (the `glob_match` crate or manual `*` / `?` matching). The `"*"` entry is a special case handled before glob evaluation.

4. **Check `tool_denylist` (always checked, even when `tool_access` = `["*"]`).** If a tool name appears in `tool_denylist`, deny it regardless of what `tool_access` says. Denylist entries also support glob patterns. This allows configurations like "admin with all tools except `exec_*`".

5. **MCP server tools declare `required_permission_level` in metadata.** Tools can optionally declare a minimum permission level via a `ToolMetadata` struct. When present, the user's `permissions.level` must be >= `tool.required_permission_level`. MCP tools extract this from their tool declaration JSON's `required_permission_level` field. Built-in tools declare it via a new `metadata()` method on the `Tool` trait (with a default returning `None`).

6. **MCP tools declare `required_custom_permissions` in metadata.** Tools can require specific entries in `permissions.custom_permissions`. For example, `{"exec_enabled": true}` requires the user to have `custom_permissions["exec_enabled"] == json!(true)`.

7. **Structured `ToolError::PermissionDenied` variant.** Replace the existing `PermissionDenied(String)` variant with a structured variant that includes:
   - `tool`: the tool name that was denied
   - `reason`: human-readable explanation of why access was denied
   This improves error reporting and allows downstream code to programmatically inspect the denied tool name.

8. **Order of checks.** The permission checker evaluates layers in this order:
   1. **Denylist first** -- if the tool is in `tool_denylist`, deny immediately.
   2. **Allowlist second** -- if the tool is not matched by any `tool_access` entry, deny.
   3. **MCP required level third** -- if the tool metadata declares `required_permission_level` and the user's level is below it, deny.
   4. **MCP required custom permissions fourth** -- if the tool metadata declares custom permission requirements that the user does not satisfy, deny.

9. **Agent loop must thread permissions to tool execution context.** In `loop_core.rs`, extract `permissions` from `request.auth_context` and pass it to `ToolRegistry::execute()`.

10. **Backward compatibility.** The `Tool` trait adds `metadata()` with a default implementation returning `None`. No existing tool implementations need changes. The `execute()` signature change requires updating all call sites, but passing `None` preserves existing behavior.

### 1.3 Integration with Existing Security

The permission layer is additive and sits above existing tool-level security:

```
Layer 1 (NEW):      ToolPermissionChecker  -- "Can this user invoke this tool at all?"
Layer 2 (EXISTING): CommandPolicy          -- "Is this shell command on the allowlist?"
Layer 3 (EXISTING): UrlPolicy             -- "Is this URL safe from SSRF?"
Layer 4 (EXISTING): Workspace sandbox     -- "Is this file path within the workspace?"
```

Each layer is independent. A request must pass all applicable layers. The new layer does not replace or modify any existing layer.

---

## 2. Pseudocode

### 2.1 ToolMetadata

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Permission metadata that a tool can declare.
///
/// Built-in tools define this via the `Tool::metadata()` trait method.
/// MCP tools derive this from their server's tool declaration JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Minimum permission level required to invoke this tool.
    /// `None` means no level requirement beyond the allowlist check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_permission_level: Option<u8>,

    /// Custom permission keys and values that must match.
    /// Example: `{"exec_enabled": true}` requires the user to have
    /// `custom_permissions["exec_enabled"] == true`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub required_custom_permissions: HashMap<String, serde_json::Value>,
}
```

### 2.2 Extended Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;

    /// Optional permission metadata for this tool.
    ///
    /// Override to declare minimum permission levels or required
    /// custom permissions. Default: no requirements (returns None).
    fn metadata(&self) -> Option<ToolMetadata> {
        None
    }
}
```

### 2.3 Structured ToolError::PermissionDenied

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),

    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// Permission denied with structured context.
    /// `tool` is the name of the tool that was denied.
    /// `reason` explains why (not in allowlist, explicitly denied,
    /// insufficient level, missing custom permission).
    #[error("permission denied for tool '{tool}': {reason}")]
    PermissionDenied { tool: String, reason: String },

    #[error("not found: {0}")]
    FileNotFound(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("timeout after {0}s")]
    Timeout(u64),
}
```

### 2.4 Glob Pattern Matching Helper

```rust
/// Match a tool name against a glob pattern.
///
/// Supports `*` (matches zero or more characters) and `?` (matches exactly one
/// character). This is a minimal implementation sufficient for tool access
/// patterns without pulling in a full glob crate.
///
/// Examples:
///   glob_matches("file_*", "file_read")     -> true
///   glob_matches("file_*", "file_write")    -> true
///   glob_matches("file_*", "web_search")    -> false
///   glob_matches("*", "anything")           -> true
///   glob_matches("server__?ool", "server__tool") -> true
fn glob_matches(pattern: &str, text: &str) -> bool {
    // Use a two-pointer / DP approach for * and ? matching.
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (plen, tlen) = (pattern.len(), text.len());

    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi = None;
    let mut star_ti = 0;

    while ti < tlen {
        if pi < plen && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < plen && pattern[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(spi) = star_pi {
            pi = spi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    // Consume trailing stars in pattern.
    while pi < plen && pattern[pi] == '*' {
        pi += 1;
    }

    pi == plen
}
```

### 2.5 check_tool_permission Function

```rust
/// Check whether the given permissions allow invoking the named tool.
///
/// This is a pure function with no side effects -- suitable for
/// unit testing without async or I/O.
///
/// Evaluation order:
///   1. Denylist (checked first, deny overrides everything)
///   2. Allowlist (empty = deny all, ["*"] = allow all, else pattern match)
///   3. Tool metadata required_permission_level
///   4. Tool metadata required_custom_permissions
///
/// Returns Ok(()) if allowed, Err(ToolError::PermissionDenied) if not.
pub fn check_tool_permission(
    tool_name: &str,
    permissions: &UserPermissions,
    tool_metadata: Option<&ToolMetadata>,
) -> Result<(), ToolError> {
    // Step 1: Denylist check (deny overrides everything, even ["*"] allowlist).
    if matches_any_pattern(tool_name, &permissions.tool_denylist) {
        return Err(ToolError::PermissionDenied {
            tool: tool_name.to_string(),
            reason: "tool is explicitly denied for this user".to_string(),
        });
    }

    // Step 2: Allowlist check.
    let allowed = if permissions.tool_access.is_empty() {
        // Empty allowlist = no tools allowed (zero_trust default).
        false
    } else if permissions.tool_access.iter().any(|s| s == "*") {
        // Wildcard entry = all tools allowed (admin default).
        true
    } else {
        // Check each pattern in tool_access for a match.
        matches_any_pattern(tool_name, &permissions.tool_access)
    };

    if !allowed {
        return Err(ToolError::PermissionDenied {
            tool: tool_name.to_string(),
            reason: format!(
                "tool is not in the allowed tools for permission level {}",
                permissions.level,
            ),
        });
    }

    // Step 3: Tool metadata -- required permission level.
    if let Some(meta) = tool_metadata {
        if let Some(required_level) = meta.required_permission_level {
            if permissions.level < required_level {
                return Err(ToolError::PermissionDenied {
                    tool: tool_name.to_string(),
                    reason: format!(
                        "tool requires permission level {} but user has level {}",
                        required_level, permissions.level,
                    ),
                });
            }
        }

        // Step 4: Tool metadata -- required custom permissions.
        for (key, required_value) in &meta.required_custom_permissions {
            match permissions.custom_permissions.get(key) {
                None => {
                    return Err(ToolError::PermissionDenied {
                        tool: tool_name.to_string(),
                        reason: format!(
                            "tool requires custom permission '{}' which is not set",
                            key,
                        ),
                    });
                }
                Some(actual) if actual != required_value => {
                    return Err(ToolError::PermissionDenied {
                        tool: tool_name.to_string(),
                        reason: format!(
                            "tool requires {}={} but user has {}={}",
                            key, required_value, key, actual,
                        ),
                    });
                }
                Some(_) => {} // Value matches, continue.
            }
        }
    }

    Ok(())
}

/// Check whether a tool name matches any pattern in the given list.
/// Each entry in `patterns` is either an exact name or a glob pattern.
fn matches_any_pattern(tool_name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.contains('*') || pattern.contains('?') {
            glob_matches(pattern, tool_name)
        } else {
            pattern == tool_name
        }
    })
}
```

### 2.6 Modified ToolRegistry::execute()

```rust
impl ToolRegistry {
    /// Execute a tool by name with optional permission enforcement.
    ///
    /// When `permissions` is `None`, all tools are allowed (backward
    /// compatibility for StaticRouter mode and unit tests).
    /// When `Some`, permissions are checked before the tool runs.
    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
        permissions: Option<&UserPermissions>,
    ) -> Result<serde_json::Value, ToolError> {
        // Look up the tool first (NotFound fires before PermissionDenied).
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        // Permission check (only when permissions are provided).
        if let Some(perms) = permissions {
            let meta = self.metadata.get(name);
            check_tool_permission(name, perms, meta)?;
        }

        debug!(tool = %name, "executing tool");
        tool.execute(args).await
    }
}
```

### 2.7 Modified AgentLoop Tool Dispatch

```rust
// In loop_core.rs, the tool execution call changes to pass
// the auth context's permissions when available.

// Before (current):
let result = self.tools.execute(&name, input.clone()).await;

// After:
let permissions = request.auth_context
    .as_ref()
    .map(|ctx| &ctx.permissions);
let result = self.tools.execute(&name, input.clone(), permissions).await;
```

### 2.8 MCP Tool Metadata Extraction

```rust
/// Extract permission metadata from an MCP tool declaration JSON.
///
/// MCP tool declarations may include:
///   "required_permission_level": 2
///   "required_custom_permissions": {"exec_enabled": true}
fn extract_mcp_metadata(tool_decl: &serde_json::Value) -> ToolMetadata {
    let required_level = tool_decl
        .get("required_permission_level")
        .and_then(|v| v.as_u64())
        .map(|v| v as u8);

    let required_custom = tool_decl
        .get("required_custom_permissions")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();

    ToolMetadata {
        required_permission_level: required_level,
        required_custom_permissions: required_custom,
    }
}
```

---

## 3. Architecture

### 3.1 File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-core/src/tools/registry.rs` | **MODIFY** | Add `ToolMetadata` struct; add `metadata()` to `Tool` trait with default impl; add `check_tool_permission()` function; add `glob_matches()` helper; add `matches_any_pattern()` helper; change `ToolError::PermissionDenied` from `PermissionDenied(String)` to `PermissionDenied { tool, reason }`; change `execute()` signature to accept `Option<&UserPermissions>`; add `metadata: HashMap<String, ToolMetadata>` field to `ToolRegistry`; add `register_with_metadata()` method; add `get_metadata()` method; optionally add `schemas_for_user()` method. |
| `crates/clawft-core/src/agent/loop_core.rs` | **MODIFY** | Extract `permissions` from `request.auth_context` and pass to `tools.execute()`. Approximately 3 lines changed. |
| All call sites of `ToolRegistry::execute()` | **MODIFY** | Add `None` as third argument at every existing call site that does not have an `AuthContext` available. |
| All pattern matches on `ToolError::PermissionDenied` | **MODIFY** | Update to destructure `PermissionDenied { tool, reason }` instead of `PermissionDenied(msg)`. |
| `crates/clawft-tools/tests/security_integration.rs` | **MODIFY** | 20 `matches!()` macro sites: change `PermissionDenied(_)` to `PermissionDenied { .. }`. See Section 4.6.1 items 11-30. |
| `crates/clawft-tools/src/shell_tool.rs` | **MODIFY** | 1 construction site (line 95) + 6 `matches!()` sites (lines 280-399). See Section 4.6.1 items 3, 31-36. |
| `crates/clawft-tools/src/spawn_tool.rs` | **MODIFY** | 1 construction site (line 115). See Section 4.6.1 item 4. |
| `crates/clawft-tools/src/web_fetch.rs` | **MODIFY** | 1 construction site (line 97). See Section 4.6.1 item 5. |
| `crates/clawft-services/src/mcp/provider.rs` | **MODIFY** | Separate `ToolError` enum definition (line 76) + 1 test construction (line 379). Must stay in sync with core. See Section 4.6.1 items 2, 38. |
| `crates/clawft-services/src/mcp/middleware.rs` | **MODIFY** | 2 construction sites (lines 286, 298) + 3 destructuring match sites (lines 513, 531, 560). See Section 4.6.1 items 6-10. |

### 3.1.1 execute() Call Site Inventory (FIX-12)

The `execute()` signature change from `execute(name, args)` to
`execute(name, args, permissions: Option<&UserPermissions>)` requires updating
every existing call site. The following is the complete inventory of call sites
that must be updated (pass `None` for backward compatibility unless an
`AuthContext` is available at the call site):

| # | File | Function / Context | Change |
|---|------|--------------------|--------|
| 1 | `crates/clawft-core/src/agent/loop_core.rs` | `execute_tool_loop()` -- main agent tool dispatch | Pass `Some(&permissions)` extracted from `request.auth_context` |
| 2 | `crates/clawft-core/src/agent/loop_core.rs` | Any other `tools.execute()` call in loop_core | Pass `None` if no auth context available, or `Some(&perms)` if available |
| 3 | `crates/clawft-core/src/tools/registry.rs` | Unit tests calling `registry.execute()` | Update to pass `None` (existing tests) or `Some(&perms)` (new permission tests) |
| 4 | `crates/clawft-core/src/agent/tool_dispatch.rs` (if exists) | Tool dispatch helper | Pass `None` or thread permissions from caller |
| 5 | `tests/integration/*.rs` | Any integration test calling `execute()` directly | Pass `None` |
| 6 | Any MCP server handler calling `execute()` | MCP tool invocation path | Pass `None` initially; future phases can thread permissions |

**Implementation note**: Before implementing, run `grep -rn "\.execute(" crates/`
to find the exact current call sites. The list above is based on the known
architecture; the implementer must verify completeness at implementation time.

**All pattern matches on `ToolError::PermissionDenied`** are fully inventoried in
**Section 4.6.1** (38 sites across 6 files). The implementer should still run
`grep -rn "PermissionDenied" crates/` at implementation time to verify no new
sites have been added since this inventory was compiled.

### 3.2 ToolRegistry Internal Storage

The registry gains a second HashMap for metadata:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    metadata: HashMap<String, ToolMetadata>,
}
```

When a tool is registered via `register()`, metadata is extracted from `tool.metadata()` if the tool provides it. When a tool is registered via `register_with_metadata()`, the caller provides metadata explicitly (used for MCP tools whose metadata comes from JSON declarations rather than the Rust trait).

### 3.3 Permission Context Flow

```
Channel Plugin (Telegram/Slack/Discord/CLI)
  |  identifies sender, resolves permissions (Phase B)
  v
InboundMessage { metadata: { sender_id, ... } }
  |
  v
AgentLoop::process_message()
  |  resolves UserPermissions via PermissionResolver (Phase B)
  |  attaches AuthContext to ChatRequest (Phase F)
  v
AgentLoop::execute_tool_loop()
  |  extracts permissions from request.auth_context
  v
ToolRegistry::execute(name, args, Some(&permissions))
  |
  |  check_tool_permission(name, permissions, metadata)
  |    1. Denylist check         -> PermissionDenied
  |    2. Allowlist check        -> PermissionDenied
  |    3. Metadata level check   -> PermissionDenied
  |    4. Metadata custom check  -> PermissionDenied
  |    All pass                  -> continue
  v
tool.execute(args)
  |  Tool's own security (CommandPolicy, UrlPolicy, sandbox)
  v
Result<serde_json::Value, ToolError>
```

### 3.4 Dependency Graph

```
clawft-types/src/routing.rs (Phase A)
    |  UserPermissions type definition
    v
clawft-core/src/pipeline/permissions.rs (Phase B)
    |  UserPermissions resolution logic, AuthContext
    v
clawft-core/src/tools/registry.rs (Phase G -- this file)
    |  ToolMetadata, check_tool_permission(), modified execute()
    v
clawft-core/src/agent/loop_core.rs (Phase F + G)
    |  Passes permissions from AuthContext to ToolRegistry::execute()
```

### 3.5 No New Crates or Dependencies

All types (`UserPermissions`, `AuthContext`) come from `clawft-types` (Phase A/B). The glob matching is implemented inline (approximately 30 lines) rather than pulling in a crate dependency. The logic is simple enough that a crate is unnecessary and avoids dependency bloat.

---

## 4. Refinement

### 4.1 Glob Pattern Matching

Tool access patterns support `*` (zero or more characters) and `?` (exactly one character). This follows Unix glob conventions. The implementation uses a two-pointer algorithm that handles `*` by backtracking, which is correct for the pattern language we support.

Examples:

| Pattern | Tool Name | Result |
|---------|-----------|--------|
| `"file_*"` | `"file_read"` | match |
| `"file_*"` | `"file_write"` | match |
| `"file_*"` | `"web_search"` | no match |
| `"*"` | `"anything"` | match (but handled as special case before glob eval) |
| `"myserver__*"` | `"myserver__search"` | match |
| `"myserver__*"` | `"otherserver__search"` | no match |
| `"read_?"` | `"read_a"` | match |
| `"read_?"` | `"read_file"` | no match (? matches exactly one char) |

Glob patterns are applied to both `tool_access` and `tool_denylist` entries. An entry without `*` or `?` characters is matched by exact string equality (fast path, no glob evaluation needed).

### 4.2 Order of Checks: Denylist First

The check order is denylist -> allowlist -> MCP metadata. The rationale:

1. **Denylist first.** If a tool is explicitly denied, there is no point evaluating whether the allowlist permits it. This is the strongest signal ("never allow this tool") and should short-circuit. Denylist takes absolute priority over allowlist.

2. **Allowlist second.** After denylist clears, the allowlist determines whether the user has the right to invoke the tool at all. Empty allowlist = no tools. Wildcard = all tools. Otherwise, pattern match.

3. **MCP required level third.** Even if the allowlist grants access (including via wildcard), a tool can declare "I require level 2" in its metadata. A level 1 user with `tool_access: ["*"]` is still denied if the tool requires level 2. This enables MCP server authors to protect sensitive tools.

4. **MCP required custom permissions fourth.** Similar to required level, but checks arbitrary key/value pairs in `custom_permissions`.

This ordering means: denylist is absolute, allowlist is necessary but not sufficient, and metadata requirements are the final gate.

### 4.3 Performance

The permission check is O(n) where n = max(|tool_access|, |tool_denylist|). For each pattern, the glob match is O(p * t) worst case where p is pattern length and t is tool name length, but in practice both are short strings (<50 chars). With typical pattern lists of <20 entries, the total cost per tool call is negligible (microseconds) compared to actual tool execution (milliseconds to seconds).

For the common admin case (`tool_access: ["*"]`), the allowlist check is O(1) -- a single string comparison against the literal `"*"` entry, handled before glob evaluation. The denylist check for admins with empty denylist is also O(1) (empty list, immediate pass).

### 4.4 Edge Cases

| Edge Case | Expected Behavior |
|-----------|-------------------|
| `tool_access: []`, `tool_denylist: []` | Deny all tools. Empty allowlist = no access. |
| `tool_access: ["*"]`, `tool_denylist: []` | Allow all tools. |
| `tool_access: ["*"]`, `tool_denylist: ["exec_shell", "spawn"]` | Allow all except exec_shell and spawn. |
| `tool_access: ["exec_shell"]`, `tool_denylist: ["exec_shell"]` | Deny. Denylist is checked first. |
| `tool_access: ["file_*"]`, tool name `"file_read"` | Allow. Glob matches. |
| `tool_access: ["file_*"]`, tool name `"web_search"` | Deny. Glob does not match. |
| `tool_denylist: ["exec_*"]`, tool name `"exec_shell"` | Deny. Glob match in denylist. |
| MCP tool `required_permission_level: 2`, user level 1, `tool_access: ["*"]` | Deny. Metadata level check fails even with wildcard allowlist. |
| MCP tool metadata `{"exec_enabled": true}`, user lacks the key | Deny. Missing custom permission. |
| MCP tool metadata `{"exec_enabled": true}`, user has `{"exec_enabled": false}` | Deny. Value mismatch. |
| `permissions` is `None` (no auth context) | Allow all tools. Backward compatibility. |
| Tool name contains special characters (MCP namespaced: `server__tool`) | Treated as-is. No special handling. Glob `*` and `?` work on the full string. |

### 4.5 Security Considerations

- **Zero-trust is deny-all by default.** Level 0 has `tool_access: []`, meaning no tools. Even if a tool has no metadata requirements, zero_trust users cannot invoke it.
- **exec_shell and spawn are the highest-risk tools.** Level 0 and Level 1 defaults exclude these. Only Level 2 (admin) with `tool_access: ["*"]` includes them.
- **Denylist cannot be bypassed.** Because denylist is checked first and applies even when `tool_access: ["*"]`, an operator can always block specific tools regardless of the allowlist.
- **No privilege escalation via tool calls.** `UserPermissions` are resolved from config at request entry (Phase B) and are immutable throughout the tool loop. A tool cannot modify its caller's permissions.
- **Defense in depth.** Permission enforcement is additive on top of existing `CommandPolicy`, `UrlPolicy`, and workspace sandbox checks. Removing the permission layer leaves existing security intact.

### 4.6 ToolError::PermissionDenied Migration

The existing `PermissionDenied(String)` variant changes to `PermissionDenied { tool: String, reason: String }`. All existing pattern matches must be updated:

Before:
```rust
ToolError::PermissionDenied(msg) => { /* ... */ }
```

After:
```rust
ToolError::PermissionDenied { tool, reason } => { /* ... */ }
```

The `Display` impl via `thiserror` produces: `"permission denied for tool 'exec_shell': tool is explicitly denied for this user"`. The existing test `tool_error_display` for `PermissionDenied` must be updated to match the new format.

#### 4.6.1 Complete ToolError::PermissionDenied Migration Inventory

The following is the exhaustive inventory of every `PermissionDenied` site in the
codebase, generated by `grep -rn "PermissionDenied" crates/`. Each site must be
updated atomically when the variant changes from `PermissionDenied(String)` to
`PermissionDenied { tool: String, reason: String }`.

**A. Variant Definitions (2 sites)**

| # | File | Line | Current Code | Required Change |
|---|------|------|--------------|-----------------|
| 1 | `crates/clawft-core/src/tools/registry.rs` | 36 | `PermissionDenied(String)` | `PermissionDenied { tool: String, reason: String }` |
| 2 | `crates/clawft-services/src/mcp/provider.rs` | 76 | `PermissionDenied(String)` (separate `ToolError` enum in provider module) | `PermissionDenied { tool: String, reason: String }` (must stay in sync with core) |

**B. Construction Sites -- `Err(ToolError::PermissionDenied(...))` (5 sites)**

| # | File | Line | Current Code | Required Change |
|---|------|------|--------------|-----------------|
| 3 | `crates/clawft-tools/src/shell_tool.rs` | 95 | `Err(ToolError::PermissionDenied(e.to_string()))` | `Err(ToolError::PermissionDenied { tool: "exec_shell".into(), reason: e.to_string() })` |
| 4 | `crates/clawft-tools/src/spawn_tool.rs` | 115 | `Err(ToolError::PermissionDenied(e.to_string()))` | `Err(ToolError::PermissionDenied { tool: "spawn".into(), reason: e.to_string() })` |
| 5 | `crates/clawft-tools/src/web_fetch.rs` | 97 | `Err(ToolError::PermissionDenied(e.to_string()))` | `Err(ToolError::PermissionDenied { tool: "web_fetch".into(), reason: e.to_string() })` |
| 6 | `crates/clawft-services/src/mcp/middleware.rs` | 286 | `ToolError::PermissionDenied(format!("command rejected for tool '{}': {reason}", request.name))` | `ToolError::PermissionDenied { tool: request.name.clone(), reason: format!("command rejected: {reason}") }` |
| 7 | `crates/clawft-services/src/mcp/middleware.rs` | 298 | `ToolError::PermissionDenied(format!("URL rejected for tool '{}': {reason}", request.name))` | `ToolError::PermissionDenied { tool: request.name.clone(), reason: format!("URL rejected: {reason}") }` |

**C. Pattern Match Sites -- Destructuring (3 sites in middleware.rs)**

| # | File | Line | Current Pattern | Required Change |
|---|------|------|-----------------|-----------------|
| 8 | `crates/clawft-services/src/mcp/middleware.rs` | 513 | `ToolError::PermissionDenied(msg) => { assert!(msg.contains("command rejected")) }` | `ToolError::PermissionDenied { tool, reason } => { assert!(reason.contains("command rejected")) }` |
| 9 | `crates/clawft-services/src/mcp/middleware.rs` | 531 | `ToolError::PermissionDenied(msg) => { assert!(msg.contains("dangerous pattern")) }` | `ToolError::PermissionDenied { tool, reason } => { assert!(reason.contains("dangerous pattern")) }` |
| 10 | `crates/clawft-services/src/mcp/middleware.rs` | 560 | `ToolError::PermissionDenied(msg) => { assert!(msg.contains("URL rejected")) }` | `ToolError::PermissionDenied { tool, reason } => { assert!(reason.contains("URL rejected")) }` |

**D. Pattern Match Sites -- `matches!()` Macro (26 sites)**

All of these use `matches!(err, ToolError::PermissionDenied(_))` and must change
to `matches!(err, ToolError::PermissionDenied { .. })`.

| # | File | Line | Context |
|---|------|------|---------|
| 11 | `crates/clawft-tools/tests/security_integration.rs` | 84 | `shell_allowlist_rejects_unlisted_command` |
| 12 | `crates/clawft-tools/tests/security_integration.rs` | 122 | `shell_dangerous_pattern_blocked_denylist_mode` |
| 13 | `crates/clawft-tools/tests/security_integration.rs` | 141 | `shell_dangerous_pattern_blocked_in_allowlist_mode` |
| 14 | `crates/clawft-tools/tests/security_integration.rs` | 160 | `shell_dangerous_pattern_blocked_in_denylist_mode` |
| 15 | `crates/clawft-tools/tests/security_integration.rs` | 183 | `shell_custom_allowlist` (rejection branch) |
| 16 | `crates/clawft-tools/tests/security_integration.rs` | 200 | `shell_empty_allowlist_blocks_everything` (echo) |
| 17 | `crates/clawft-tools/tests/security_integration.rs` | 208 | `shell_empty_allowlist_blocks_everything` (ls) |
| 18 | `crates/clawft-tools/tests/security_integration.rs` | 224 | `shell_piped_command_dangerous_pattern` |
| 19 | `crates/clawft-tools/tests/security_integration.rs` | 270 | `shell_env_var_injection_attempt` |
| 20 | `crates/clawft-tools/tests/security_integration.rs` | 324 | `spawn_unlisted_command_rejected` |
| 21 | `crates/clawft-tools/tests/security_integration.rs` | 341 | `spawn_dangerous_sudo_rejected` |
| 22 | `crates/clawft-tools/tests/security_integration.rs` | 360 | `spawn_dangerous_mkfs_rejected` |
| 23 | `crates/clawft-tools/tests/security_integration.rs` | 396 | `spawn_path_traversal_rejected` |
| 24 | `crates/clawft-tools/tests/security_integration.rs` | 544 | `cross_tool_policy_consistency` (shell_err) |
| 25 | `crates/clawft-tools/tests/security_integration.rs` | 545 | `cross_tool_policy_consistency` (spawn_err) |
| 26 | `crates/clawft-tools/tests/security_integration.rs` | 557 | `cross_tool_policy_consistency` (shell_err, sudo) |
| 27 | `crates/clawft-tools/tests/security_integration.rs` | 558 | `cross_tool_policy_consistency` (spawn_err, sudo) |
| 28 | `crates/clawft-tools/tests/security_integration.rs` | 609 | `allowlist_vs_denylist_mode_difference` |
| 29 | `crates/clawft-tools/tests/security_integration.rs` | 647 | `cross_tool_dangerous_always_blocked` (shell_err) |
| 30 | `crates/clawft-tools/tests/security_integration.rs` | 648 | `cross_tool_dangerous_always_blocked` (spawn_err) |
| 31 | `crates/clawft-tools/src/shell_tool.rs` | 280 | `test_dangerous_rm_rf` |
| 32 | `crates/clawft-tools/src/shell_tool.rs` | 293 | `test_dangerous_sudo` |
| 33 | `crates/clawft-tools/src/shell_tool.rs` | 306 | `test_dangerous_mkfs` |
| 34 | `crates/clawft-tools/src/shell_tool.rs` | 319 | `test_dangerous_dd` |
| 35 | `crates/clawft-tools/src/shell_tool.rs` | 332 | `test_dangerous_fork_bomb` |
| 36 | `crates/clawft-tools/src/shell_tool.rs` | 399 | `test_policy_rejects_unlisted_command` |

**E. Test Construction Sites (2 sites)**

| # | File | Line | Current Code | Required Change |
|---|------|------|--------------|-----------------|
| 37 | `crates/clawft-core/src/tools/registry.rs` | 512 | `ToolError::PermissionDenied("no exec access".into())` | `ToolError::PermissionDenied { tool: "test".into(), reason: "no exec access".into() }` |
| 38 | `crates/clawft-services/src/mcp/provider.rs` | 379 | `ToolError::PermissionDenied("nope".into())` | `ToolError::PermissionDenied { tool: "test".into(), reason: "nope".into() }` |

**Total: 38 sites across 6 files.**

#### 4.6.2 Migration Checklist

- [ ] Update variant definition in `crates/clawft-core/src/tools/registry.rs:36`
- [ ] Update variant definition in `crates/clawft-services/src/mcp/provider.rs:76` (separate ToolError enum)
- [ ] Update all 5 construction sites (items 3-7 above)
- [ ] Update all 3 destructuring match sites in `middleware.rs` (items 8-10)
- [ ] Update all 26 `matches!()` macro sites to use `{ .. }` instead of `(_)` (items 11-36)
- [ ] Update 2 test construction sites (items 37-38)
- [ ] Update `#[error(...)]` format string from `"permission denied: {0}"` to `"permission denied for tool '{tool}': {reason}"`
- [ ] Update `tool_error_display` test assertion in `registry.rs` to match new format
- [ ] Update `tool_error_display` test assertion in `provider.rs` to match new format
- [ ] Verify no external crates depend on the tuple variant (`grep -rn "PermissionDenied" .` outside `crates/`)
- [ ] Run `cargo build --all-targets` to catch any missed sites
- [ ] Run `cargo test --workspace` to verify all updated tests pass
- [ ] Run `cargo clippy --workspace -- -D warnings` for lint cleanliness

#### 4.6.3 Files NOT in Phase G Ownership Matrix That Require Changes

The following files contain `PermissionDenied` references but are **not listed** in
the Phase G file ownership matrix (Section 3.1). They must be added or explicitly
assigned:

| File | Sites | Owner Recommendation |
|------|-------|---------------------|
| `crates/clawft-tools/tests/security_integration.rs` | 20 match sites (lines 84-648) | Phase G implementer (mechanical `(_)` -> `{ .. }` change) |
| `crates/clawft-tools/src/shell_tool.rs` | 1 construction + 6 match sites | Phase G implementer (construction requires tool name; matches are mechanical) |
| `crates/clawft-tools/src/spawn_tool.rs` | 1 construction site | Phase G implementer |
| `crates/clawft-tools/src/web_fetch.rs` | 1 construction site | Phase G implementer |
| `crates/clawft-services/src/mcp/provider.rs` | 1 definition + 1 test construction | Phase G implementer (separate ToolError enum must stay in sync) |
| `crates/clawft-services/src/mcp/middleware.rs` | 2 construction + 3 match sites | Phase G implementer |

---

## 5. Completion

### 5.1 Success Criteria

- [ ] Level 0 (zero_trust) users are blocked from all tools (empty `tool_access` default)
- [ ] Level 0 users are specifically blocked from `exec_shell` and `spawn`
- [ ] Level 1 (user) users are limited to their configured tool set (`read_file`, `write_file`, `edit_file`, `list_dir`, `web_search`, `web_fetch`, `message`)
- [ ] Level 1 users cannot invoke `exec_shell` or `spawn`
- [ ] Level 2 (admin) users are unrestricted (`tool_access: ["*"]`)
- [ ] Denylist overrides allowlist in all cases, including wildcard
- [ ] Glob patterns in `tool_access` work (e.g., `"file_*"` matches `"file_read"`)
- [ ] Glob patterns in `tool_denylist` work (e.g., `"exec_*"` denies `"exec_shell"`)
- [ ] MCP tool `required_permission_level` is enforced
- [ ] MCP tool `required_custom_permissions` is enforced
- [ ] `ToolError::PermissionDenied` includes tool name and reason
- [ ] Passing `None` for permissions preserves existing behavior (all tools allowed)
- [ ] All existing tests pass (with updated call sites)
- [ ] No new `unsafe` blocks
- [ ] Code compiles without warnings (`cargo clippy --workspace`)

### 5.2 Test Plan

Minimum 20 tests covering the full permission matrix.

#### Allowlist / Denylist Tests (8 tests)

| # | Test Name | Scenario |
|---|-----------|----------|
| 1 | `test_wildcard_allows_all_tools` | `tool_access: ["*"]`, `tool_denylist: []` -- check("exec_shell") = Ok |
| 2 | `test_empty_allowlist_denies_all_tools` | `tool_access: []`, `tool_denylist: []` -- check("read_file") = Err |
| 3 | `test_explicit_allowlist_permits_listed_tool` | `tool_access: ["read_file", "write_file"]` -- check("read_file") = Ok |
| 4 | `test_explicit_allowlist_denies_unlisted_tool` | `tool_access: ["read_file", "write_file"]` -- check("exec_shell") = Err |
| 5 | `test_denylist_overrides_wildcard` | `tool_access: ["*"]`, `tool_denylist: ["exec_shell"]` -- check("exec_shell") = Err, check("read_file") = Ok |
| 6 | `test_denylist_overrides_explicit_allow` | `tool_access: ["exec_shell"]`, `tool_denylist: ["exec_shell"]` -- check("exec_shell") = Err |
| 7 | `test_glob_allowlist_pattern` | `tool_access: ["file_*"]` -- check("file_read") = Ok, check("file_write") = Ok, check("web_search") = Err |
| 8 | `test_glob_denylist_pattern` | `tool_access: ["*"]`, `tool_denylist: ["exec_*"]` -- check("exec_shell") = Err, check("exec_spawn") = Err, check("read_file") = Ok |

#### MCP Metadata Tests (4 tests)

| # | Test Name | Scenario |
|---|-----------|----------|
| 9 | `test_metadata_required_level_blocks_low_user` | level=1, metadata.required_permission_level=2 -- Err |
| 10 | `test_metadata_required_level_allows_sufficient_user` | level=2, metadata.required_permission_level=2 -- Ok |
| 11 | `test_metadata_custom_permission_missing` | user has no custom_permissions, metadata requires `exec_enabled: true` -- Err |
| 12 | `test_metadata_custom_permission_wrong_value` | user has `exec_enabled: false`, metadata requires `exec_enabled: true` -- Err |

#### MCP Namespaced Tool Tests (2 tests)

| # | Test Name | Scenario |
|---|-----------|----------|
| 13 | `test_mcp_namespaced_tool_exact_match` | `tool_access: ["myserver__search"]` -- check("myserver__search") = Ok, check("myserver__exec") = Err |
| 14 | `test_mcp_namespaced_tool_glob_match` | `tool_access: ["myserver__*"]` -- check("myserver__search") = Ok, check("otherserver__search") = Err |

#### Registry Integration Tests (3 tests)

| # | Test Name | Scenario |
|---|-----------|----------|
| 15 | `test_registry_execute_with_none_permissions` | permissions=None -- execute("echo", args, None) = Ok (backward compat) |
| 16 | `test_registry_execute_with_denied_permissions` | permissions with tool_access=[] -- execute("echo", args, Some(&perms)) = Err(PermissionDenied) |
| 17 | `test_registry_execute_not_found_before_permission_check` | tool does not exist -- execute("nonexistent", args, Some(&perms)) = Err(NotFound), not PermissionDenied |

#### Security-Critical Tests (5 tests)

| # | Test Name | Scenario |
|---|-----------|----------|
| 18 | `test_zero_trust_blocked_from_exec_and_spawn` | level=0, tool_access=[] -- check("exec_shell") = Err, check("spawn") = Err |
| 19 | `test_user_level_blocked_from_exec_and_spawn` | level=1, tool_access=["read_file", ...] -- check("exec_shell") = Err, check("spawn") = Err |
| 20 | `test_admin_can_call_all_tools` | level=2, tool_access=["*"] -- check("exec_shell") = Ok, check("spawn") = Ok, check("read_file") = Ok, check("myserver__tool") = Ok |
| 21 | `test_permission_denied_error_includes_tool_name` | Verify ToolError::PermissionDenied { tool, reason } contains the correct tool name |
| 22 | `test_permission_denied_error_display_format` | Verify Display output matches "permission denied for tool 'X': reason" |

#### Glob Matching Unit Tests (3 tests)

| # | Test Name | Scenario |
|---|-----------|----------|
| 23 | `test_glob_star_matches` | "file_*" matches "file_read", does not match "web_search" |
| 24 | `test_glob_question_mark` | "read_?" matches "read_a", does not match "read_file" |
| 25 | `test_glob_exact_match_fast_path` | "read_file" (no wildcards) matches "read_file", does not match "write_file" |

#### Admin + CommandPolicy Integration Test (FIX-12, 1 test)

| # | Test Name | Scenario |
|---|-----------|----------|
| 26 | `test_admin_wildcard_still_subject_to_command_policy` | Admin user with `tool_access: ["*"]` passes the Phase G permission check for `exec_shell`, but the tool's own `CommandPolicy` still enforces its allowlist. Verifies that permission layer (Phase G) and command policy layer (existing) are independent. An admin can _invoke_ `exec_shell` (tool permission check passes), but `exec_shell` itself rejects a command not on the `CommandPolicy` allowlist. This confirms defense-in-depth: `tool_access: ["*"]` does NOT bypass `CommandPolicy`. |

This test requires:
1. A `ToolRegistry` with a real (or mock) `exec_shell` tool that has `CommandPolicy` enforcement.
2. An admin `UserPermissions` with `tool_access: ["*"]` and `level: 2`.
3. Calling `execute("exec_shell", args_with_disallowed_command, Some(&admin_perms))`.
4. Asserting that the result is `Err(ToolError::ExecutionFailed(...))` (from `CommandPolicy`), NOT `Err(ToolError::PermissionDenied { .. })`.

This proves that the permission layer (outer) grants access, but the command policy layer (inner) still blocks disallowed commands.

### 5.3 Branch

`weft/tiered-router`

### 5.4 Effort

0.5 days (approximately 200-300 lines of new/modified Rust code, 200+ lines of tests).

### 5.5 Dependencies

- **Depends on Phase B** (UserPermissions struct with `tool_access`, `tool_denylist`, `level`, `custom_permissions` fields must exist in `clawft-types`)
- **Integrates with Phase F** (AuthContext threading -- the agent loop must have auth context on ChatRequest to extract permissions for tool calls)

### 5.6 Verification Commands

```bash
# Build
cargo build --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Test all
cargo test --workspace

# Test only tool registry
cargo test --package clawft-core tools::registry

# Test only permission checker
cargo test --package clawft-core tools::registry::tests::test_
```

### 5.7 What This Phase Does NOT Include

- No changes to `UserPermissions` struct definition (Phase B)
- No permission resolution or merge logic (Phase B)
- No routing logic (Phase C)
- No cost tracking (Phase D)
- No rate limiting (Phase E)
- No AuthContext creation from channel plugins (Phase F)
- No config validation (Phase H)
- No schema filtering by permission (optional future enhancement; denied tools return clear errors)

---

## Remediation Applied

**Date**: 2026-02-18
**Source**: `/home/aepod/dev/clawft/.planning/sparc/01-tiered-router/remediation-plan.md`

The following fixes from the remediation plan have been applied to this document:

### FIX-12: Inventory ALL execute() Call Sites
- **Section 3.1.1 (NEW)**: Added complete inventory of all `execute()` call sites
  that must be updated for the signature change from `execute(name, args)` to
  `execute(name, args, permissions: Option<&UserPermissions>)`. Covers loop_core.rs,
  registry.rs unit tests, tool_dispatch.rs, integration tests, and MCP server handlers.
  Includes instructions for the implementer to run `grep` to verify completeness.
- Also inventories all `ToolError::PermissionDenied` pattern match sites that need
  updating from `PermissionDenied(msg)` to `PermissionDenied { tool, reason }`.

### FIX-12: Admin + CommandPolicy Integration Test
- **Section 5.2, test #26 (NEW)**: Added integration test
  `test_admin_wildcard_still_subject_to_command_policy` verifying that an admin user
  with `tool_access: ["*"]` passes the Phase G permission check but is still subject
  to the existing `CommandPolicy` enforcement within `exec_shell`. This proves
  defense-in-depth: the permission layer and command policy layer are independent.
  `tool_access: ["*"]` does NOT bypass `CommandPolicy`.

---

**End of SPARC Plan**
