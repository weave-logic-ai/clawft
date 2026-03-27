# Panel 1: Tool Catalog & Registry Audit

**Date**: 2026-03-04
**Presenter**: Kernel Auditor
**Scope**: 27-tool catalog, ToolRegistry, BuiltinTool trait, reference implementations
**Branch**: feature/weftos-kernel-sprint
**File Under Review**: `crates/clawft-kernel/src/wasm_runner.rs` (1,639 lines)

---

## 1. Executive Summary

K3 delivers a complete built-in tool catalog of 27 tools across 3 categories,
a type-safe registry with trait-based dispatch, and 2 fully operational
reference implementations. The catalog is well-structured with JSON Schema
parameters, governance gate actions, and effect vectors for every tool.

**Verdict**: The catalog is production-grade for the current phase. All tools
have valid schemas, unique names, correct categories, and appropriate risk
vectors.

---

## 2. Catalog Structure

### 2.1 BuiltinToolSpec Type

```rust
pub struct BuiltinToolSpec {
    pub name: String,           // e.g. "fs.read_file"
    pub category: ToolCategory, // Filesystem | Agent | System | User
    pub description: String,
    pub parameters: Value,      // JSON Schema (type: "object")
    pub gate_action: String,    // e.g. "tool.fs.read"
    pub effect: EffectVector,   // 5D governance scoring
    pub native: bool,           // can run without WASM
}
```

All 27 tools have:
- Dotted naming convention (`category.action`)
- JSON Schema with `type: "object"` and `properties`
- Gate action strings following `tool.{category}.{action}` pattern
- EffectVector with calibrated risk/privacy/security scores

### 2.2 Complete Catalog

#### Filesystem Tools (10)

| # | Name | Gate Action | Risk | Notes |
|---|------|-------------|------|-------|
| 1 | `fs.read_file` | `tool.fs.read` | 0.1 | **Implemented** -- offset/limit, metadata |
| 2 | `fs.write_file` | `tool.fs.write` | 0.4 | Spec only |
| 3 | `fs.read_dir` | `tool.fs.read` | 0.1 | Spec only |
| 4 | `fs.create_dir` | `tool.fs.write` | 0.3 | Spec only |
| 5 | `fs.remove` | `tool.fs.delete` | 0.7 | Highest risk in fs category |
| 6 | `fs.copy` | `tool.fs.write` | 0.3 | Spec only |
| 7 | `fs.move` | `tool.fs.write` | 0.5 | Spec only |
| 8 | `fs.stat` | `tool.fs.read` | 0.05 | Minimal risk |
| 9 | `fs.exists` | `tool.fs.read` | 0.05 | Minimal risk |
| 10 | `fs.glob` | `tool.fs.read` | 0.1 | Spec only |

#### Agent Tools (7)

| # | Name | Gate Action | Risk | Notes |
|---|------|-------------|------|-------|
| 1 | `agent.spawn` | `tool.agent.spawn` | 0.5 | **Implemented** -- PID allocation |
| 2 | `agent.stop` | `tool.agent.stop` | 0.4 | Spec only |
| 3 | `agent.list` | `tool.agent.read` | 0.05 | Spec only |
| 4 | `agent.inspect` | `tool.agent.read` | 0.1 | Spec only |
| 5 | `agent.send` | `tool.agent.ipc` | 0.2 | Spec only |
| 6 | `agent.suspend` | `tool.agent.suspend` | 0.3 | Spec only |
| 7 | `agent.resume` | `tool.agent.resume` | 0.2 | Spec only |

#### System Tools (10)

| # | Name | Gate Action | Risk | Notes |
|---|------|-------------|------|-------|
| 1 | `sys.service.list` | `tool.sys.read` | 0.05 | Spec only |
| 2 | `sys.service.health` | `tool.sys.read` | 0.05 | Spec only |
| 3 | `sys.chain.status` | `tool.sys.read` | 0.05 | Spec only |
| 4 | `sys.chain.query` | `tool.sys.read` | 0.1 | Spec only |
| 5 | `sys.tree.read` | `tool.sys.read` | 0.05 | Spec only |
| 6 | `sys.tree.inspect` | `tool.sys.read` | 0.1 | Spec only |
| 7 | `sys.env.get` | `tool.sys.env` | 0.2 | Privacy: 0.3 |
| 8 | `sys.cron.add` | `tool.sys.cron` | 0.4 | Spec only |
| 9 | `sys.cron.list` | `tool.sys.read` | 0.05 | Spec only |
| 10 | `sys.cron.remove` | `tool.sys.cron` | 0.3 | Spec only |

### 2.3 Catalog Invariants (Verified by Tests)

- `builtin_catalog_has_27_tools` -- exact count
- `all_tools_have_valid_schema` -- every `parameters` field is a JSON object with `type: "object"`
- `all_tools_have_gate_action` -- no empty gate_action strings
- `tool_names_are_unique` -- no duplicate names
- `tool_categories_correct` -- 10 Filesystem + 7 Agent + 10 System

---

## 3. ToolRegistry Architecture

### 3.1 BuiltinTool Trait

```rust
pub trait BuiltinTool: Send + Sync {
    fn name(&self) -> &str;
    fn spec(&self) -> &BuiltinToolSpec;
    fn execute(&self, args: Value) -> Result<Value, ToolError>;
}
```

Design decisions:
- **Synchronous execute** -- both reference tools are synchronous; async not needed yet
- **Send + Sync** -- required for Arc wrapping and cross-thread dispatch
- **Value in, Value out** -- JSON-based for maximum flexibility
- **ToolError return** -- typed errors with Display impl

### 3.2 Registry Dispatch

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn BuiltinTool>>,
}
```

Methods:
- `new()` -- empty registry
- `register(tool: Arc<dyn BuiltinTool>)` -- name extracted from tool
- `get(name) -> Option<&Arc<dyn BuiltinTool>>` -- lookup
- `execute(name, args) -> Result<Value, ToolError>` -- dispatch
- `list() -> Vec<String>` -- sorted names
- `len()`, `is_empty()` -- size queries

---

## 4. Reference Implementations

### 4.1 FsReadFileTool

**File**: `wasm_runner.rs:1057-1147`
**Implements**: `fs.read_file` from catalog

Behavior:
- Reads file at `args.path` (required)
- Optional `offset` (byte offset, default 0)
- Optional `limit` (byte count, default file size)
- Max read size: 8 MiB (`MAX_READ_SIZE`)
- Returns `{ content, size, modified }` JSON
- UTF-8 lossy conversion for binary files
- `modified` as RFC3339 timestamp

Error cases:
- File not found -> `ToolError::ExecutionFailed("file not found: ...")`
- Read failure -> `ToolError::ExecutionFailed`

Tests:
- `fs_read_file_reads_content` -- write temp file, read back
- `fs_read_file_with_offset_limit` -- partial read verification
- `fs_read_file_not_found` -- error handling
- `fs_read_file_returns_metadata` -- size + modified fields present

### 4.2 AgentSpawnTool

**File**: `wasm_runner.rs:1149-1219`
**Implements**: `agent.spawn` from catalog

Behavior:
- Reads `args.agent_id` (required)
- Optional `args.backend` (defaults to "native", rejects "wasm")
- Creates `ProcessEntry` directly in `ProcessTable`
- Returns `{ pid, agent_id, state: "running" }` JSON

Dependencies:
- Holds `Arc<ProcessTable>` (injected at construction)
- Does NOT use `AgentSupervisor` -- direct table insertion for simplicity

Error cases:
- Missing agent_id -> `ToolError::InvalidArgs`
- WASM backend requested -> `ToolError::ExecutionFailed("WASM backend not available")`
- ProcessTable full -> `ToolError::ExecutionFailed`

Tests:
- `agent_spawn_creates_process` -- PID in table
- `agent_spawn_with_wasm_backend_fails` -- backend rejection
- `agent_spawn_returns_pid` -- return shape

---

## 5. Findings

### 5.1 Strengths

1. **Complete catalog with governance integration** -- every tool has gate_action + effect vector
2. **JSON Schema parameters** -- enables validation before execution
3. **Trait-based dispatch** -- clean extension point for K4+ tools
4. **Reference impls demonstrate both patterns** -- stateless (FsReadFile) and stateful (AgentSpawn)
5. **ToolVersion with crypto** -- SHA-256 + Ed25519 ready for signed deployments

### 5.2 Gaps

1. **25 tools are spec-only** -- no execute implementation
2. **FsReadFileTool has no sandbox enforcement** -- reads any path the process can access
3. **AgentSpawnTool bypasses supervisor** -- direct ProcessTable insertion misses spawn hooks
4. **No async execute** -- will need `async_trait` when WASM tools are I/O-bound
5. **ToolRegistry is not global** -- created per agent spawn, not shared kernel-wide

### 5.3 Recommendations

- **K4**: Implement `fs.write_file`, `fs.remove`, `agent.stop` (next 3 highest-value tools)
- **K4**: Add `SandboxEnforcer` path validation to FsReadFileTool
- **K5**: Wire AgentSpawnTool through supervisor for proper lifecycle hooks
- **Future**: Consider `async fn execute` when WASM tools need I/O

---

## 6. Test Results (Live Run)

```
running 31 tests
test wasm_runner::tests::agent_spawn_with_wasm_backend_fails ... ok
test wasm_runner::tests::agent_spawn_returns_pid ... ok
test wasm_runner::tests::agent_spawn_creates_process ... ok
test wasm_runner::tests::all_tools_have_gate_action ... ok
test wasm_runner::tests::all_tools_have_valid_schema ... ok
test wasm_runner::tests::builtin_catalog_has_27_tools ... ok
test wasm_runner::tests::default_config ... ok
test wasm_runner::tests::config_serde_roundtrip ... ok
test wasm_runner::tests::builtin_tool_spec_serde_roundtrip ... ok
test wasm_runner::tests::execution_timeout_duration ... ok
test wasm_runner::tests::fs_read_file_not_found ... ok
test wasm_runner::tests::load_tool_without_feature_rejects ... ok
test wasm_runner::tests::module_hash_deterministic ... ok
test wasm_runner::tests::module_hash_differs_for_different_input ... ok
test wasm_runner::tests::registry_not_found ... ok
test wasm_runner::tests::registry_register_and_execute ... ok
test wasm_runner::tests::tool_categories_correct ... ok
test wasm_runner::tests::fs_read_file_reads_content ... ok
test wasm_runner::tests::fs_read_file_returns_metadata ... ok
test wasm_runner::tests::tool_state_default ... ok
test wasm_runner::tests::tool_version_serde_roundtrip ... ok
test wasm_runner::tests::fs_read_file_with_offset_limit ... ok
test wasm_runner::tests::tool_names_are_unique ... ok
test wasm_runner::tests::validate_wasm_accepts_valid_header ... ok
test wasm_runner::tests::validate_wasm_rejects_invalid_magic ... ok
test wasm_runner::tests::validate_wasm_rejects_too_large ... ok
test wasm_runner::tests::validate_wasm_rejects_too_short ... ok
test wasm_runner::tests::validate_wasm_warns_on_wrong_version ... ok
test wasm_runner::tests::wasm_error_display ... ok
test wasm_runner::tests::wasm_tool_result_serde_roundtrip ... ok
test wasm_runner::tests::wasm_validation_serde_roundtrip ... ok
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured
```

**Verdict**: PASS -- all 31 tests green.
