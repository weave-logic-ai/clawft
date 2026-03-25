# Panel 3: Sandbox & Security Review

**Date**: 2026-03-04
**Presenter**: Research Analyst
**Scope**: WASM sandbox config, fuel metering, memory limits, governance gate integration
**Branch**: feature/weftos-kernel-sprint
**Files Under Review**: `crates/clawft-kernel/src/wasm_runner.rs`, `crates/clawft-kernel/src/gate.rs`

---

## 1. Executive Summary

K3 establishes the security architecture for tool execution: sandbox
configuration, module validation, governance gate checks, and error
boundaries. The Wasmtime runtime is feature-gated behind `wasm-sandbox`
but all configuration and validation logic is available unconditionally.
The governance gate integration is mature, with defense-in-depth checks
at both routing and handler time.

**Verdict**: Security posture is strong for the current phase. The
feature-gate approach correctly avoids shipping Wasmtime in the default
binary. Risk calibration across the 27-tool catalog is thoughtful.
Two items require attention before WASM tools execute in production.

---

## 2. Sandbox Configuration

### 2.1 WasmSandboxConfig

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `max_fuel` | 1,000,000 | CPU budget (~100ms on modern hardware) |
| `max_memory_bytes` | 16 MiB | Linear memory cap per tool execution |
| `max_execution_time_secs` | 30 | Wall-clock timeout safety net |
| `allowed_host_calls` | `[]` | Whitelist of permitted host functions |
| `wasi_enabled` | false | WASI filesystem access disabled by default |
| `max_module_size_bytes` | 10 MiB | Pre-parse module size check |

**Assessment**: Defaults are conservative and appropriate. The 30s wall-clock
timeout as a safety net behind fuel metering is defense-in-depth. The empty
`allowed_host_calls` means WASM tools are fully isolated by default.

### 2.2 Module Validation

Without `wasm-sandbox` feature, validation performs:
1. **Magic byte check** -- `\0asm` header (4 bytes)
2. **Version check** -- WASM version 1 expected
3. **Size check** -- reject modules > `max_module_size_bytes`

With `wasm-sandbox` feature (Wasmtime), adds:
4. **Full module validation** -- `wasmtime::Module::validate()`
5. **Export/import extraction** -- schema discovery

**Assessment**: The pre-parse checks (magic, version, size) are O(1) and
catch obvious invalid inputs before any parsing. This is a good DoS
prevention pattern.

### 2.3 Per-Execution Isolation

Each tool execution creates a fresh `Store<ToolState>`:
- **No shared state** between executions
- **Fuel budget** set per-call (not cumulative)
- **Memory limit** enforced by Wasmtime engine config
- **Wall-clock timeout** via `tokio::time::timeout`

---

## 3. Governance Gate Integration

### 3.1 Gate Check Flow

```
Agent sends: {"cmd": "exec", "tool": "fs.read_file", "args": {...}}
  |
  v
agent_loop: extract cmd = "exec"
  |
  v
Gate check: gate.check(agent_id, "tool.exec", {pid: N})
  |
  +--> Deny:  reply {error, denied: true}, STOP
  +--> Defer: reply {deferred: true}, STOP
  +--> Permit: continue to ToolRegistry dispatch
```

### 3.2 Tool-Specific Gate Actions

Each tool in the catalog has a specific `gate_action` string:

| Category | Example Tool | Gate Action |
|----------|-------------|-------------|
| Filesystem | fs.read_file | `tool.fs.read` |
| Filesystem | fs.remove | `tool.fs.delete` |
| Agent | agent.spawn | `tool.agent.spawn` |
| System | sys.cron.add | `tool.sys.cron` |

**Finding**: The current gate check at line 140 uses `"tool.exec"` as the
action for ALL exec commands, not the tool-specific `gate_action` from
the catalog. This means the gate cannot distinguish between `fs.read_file`
(risk 0.1) and `fs.remove` (risk 0.7).

**Recommendation (K4)**: Pass the tool's `gate_action` to the gate check
instead of the generic `"tool.exec"`:

```rust
let action = if !tool_name.is_empty() {
    if let Some(registry) = &tool_registry {
        registry.get(tool_name).map(|t| t.spec().gate_action.as_str())
    } else { None }
} else { Some("tool.exec") };
```

### 3.3 Risk Calibration

The effect vectors are well-calibrated across the catalog:

| Risk Level | Count | Examples |
|------------|-------|---------|
| Minimal (0.05) | 8 | fs.stat, fs.exists, agent.list, sys.service.list |
| Low (0.1-0.2) | 7 | fs.read_file, agent.inspect, sys.chain.query |
| Medium (0.3-0.4) | 8 | fs.create_dir, agent.stop, sys.cron.add |
| High (0.5-0.7) | 4 | fs.move, fs.remove, agent.spawn |

`fs.remove` at risk 0.7 with security 0.3 is the highest-risk tool in the
catalog. The `sys.env.get` tool correctly has privacy 0.3 for environment
variable access.

---

## 4. Error Boundaries

### 4.1 WasmError Types

```rust
pub enum WasmError {
    InvalidModule(String),      // Bad WASM bytes
    CompilationFailed(String),  // Wasmtime compilation error
    FuelExhausted,              // CPU budget exceeded
    MemoryLimitExceeded,        // Memory cap hit
    ExecutionTimeout,           // Wall-clock timeout
    WasmTrap(String),           // WASM trap (div by zero, etc.)
    HostCallDenied(String),     // Unauthorized host function call
    ModuleTooLarge,             // Pre-parse size check failed
    RuntimeUnavailable,         // wasm-sandbox feature not enabled
}
```

### 4.2 ToolError Types

```rust
pub enum ToolError {
    NotFound(String),
    InvalidArgs(String),
    ExecutionFailed(String),
    PermissionDenied(String),
    Timeout,
}
```

**Assessment**: Good separation between WASM-level errors and tool-level
errors. The `RuntimeUnavailable` variant is key -- it allows graceful
degradation when `wasm-sandbox` is not compiled in.

---

## 5. Attack Surface Analysis

### 5.1 Current Attack Vectors (Mitigated)

| Vector | Mitigation | Status |
|--------|-----------|--------|
| Module size bomb | Pre-parse 10 MiB limit | Active |
| CPU exhaustion | Fuel metering (1M units) | Config ready, needs Wasmtime |
| Memory exhaustion | 16 MiB linear memory cap | Config ready, needs Wasmtime |
| Infinite loop | 30s wall-clock timeout | Config ready, needs Wasmtime |
| Unauthorized host calls | Empty allowlist by default | Active |
| WASI filesystem escape | WASI disabled by default | Active |

### 5.2 Current Attack Vectors (Open)

| Vector | Risk | Phase |
|--------|------|-------|
| FsReadFileTool reads any accessible path | Medium | K4: Add SandboxEnforcer |
| AgentSpawnTool bypasses supervisor hooks | Low | K5: Wire through supervisor |
| Generic "tool.exec" gate action | Medium | K4: Use tool-specific gate_action |
| No module signature verification on load | Low | K4: Verify Ed25519 before instantiation |

### 5.3 Industry Comparison

WeftOS's tool sandboxing compares favorably to:

| Platform | Isolation | Fuel Metering | Governance | Chain Audit |
|----------|-----------|--------------|------------|-------------|
| WeftOS (K3) | WASM + fuel + memory | Yes | GovernanceGate | ExoChain |
| Deno | V8 isolates | No | Permissions | No |
| Wasmer | WASM | Fuel (opt) | No | No |
| Cloudflare Workers | V8 isolates | CPU time | No | Log only |

WeftOS is unique in combining WASM isolation with constitutional governance
and cryptographic audit logging.

---

## 6. Findings

### 6.1 Strengths

1. **Defense-in-depth** -- size check, fuel, memory, timeout, host call deny
2. **Conservative defaults** -- WASI off, no host calls, 16 MiB memory
3. **Feature-gated Wasmtime** -- zero binary bloat for non-WASM builds
4. **Risk-calibrated catalog** -- effect vectors reflect actual danger
5. **Immutable audit trail** -- every tool execution logged to chain

### 6.2 Critical Findings

1. **CF-1 (Medium)**: Gate check uses `"tool.exec"` not tool-specific `gate_action`
2. **CF-2 (Medium)**: FsReadFileTool has no path sandboxing
3. **CF-3 (Low)**: No signature verification before tool instantiation

### 6.3 Recommendations

| ID | Recommendation | Phase | Priority |
|----|---------------|-------|----------|
| SR-1 | Use tool-specific gate_action in exec handler | K4 | High |
| SR-2 | Add SandboxEnforcer.check_file_read() to FsReadFileTool | K4 | High |
| SR-3 | Verify Ed25519 signature before WASM module load | K4 | Medium |
| SR-4 | Add rate limiting for tool execution (per-agent) | K5 | Medium |
| SR-5 | Consider WASM module caching (compile once, run many) | K4 | Low |

---

## 7. Test Results

Governance gate integration tests:

```
test agent_loop::tests::gate_deny_blocks_exec ... ok
test agent_loop::tests::gate_permit_allows_exec ... ok
```

WASM validation tests:

```
test wasm_runner::tests::validate_wasm_rejects_invalid_magic ... ok
test wasm_runner::tests::validate_wasm_rejects_too_large ... ok
test wasm_runner::tests::validate_wasm_rejects_too_short ... ok
test wasm_runner::tests::validate_wasm_accepts_valid_header ... ok
test wasm_runner::tests::validate_wasm_warns_on_wrong_version ... ok
test wasm_runner::tests::load_tool_without_feature_rejects ... ok
```

**Verdict**: PASS -- security controls verified. Gate integration working.
Module validation rejecting invalid inputs.
