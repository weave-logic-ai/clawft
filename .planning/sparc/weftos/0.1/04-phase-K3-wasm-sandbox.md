# Phase K3: WASM Tool Sandboxing

**Phase ID**: K3
**Workstream**: W-KERNEL
**Duration**: Week 3-4 (parallel with K1)
**Goal**: Execute tools in isolated Wasmtime WASM sandboxes with fuel metering, memory limits, and host filesystem isolation

---

## S -- Specification

### What Changes

This phase adds a WASM tool execution runtime to the kernel using Wasmtime. Tools can be compiled to WASM and executed in an isolated sandbox with configurable fuel (CPU) limits, memory caps, and no direct host filesystem access. The WASM runner integrates with the existing `ToolRegistry` as a new tool source and reuses the `PluginSandbox` patterns from `clawft-wasm`.

This is feature-gated behind `wasm-sandbox` to avoid adding Wasmtime to the default binary size.

### Files to Create

| File | Purpose |
|---|---|
| `crates/clawft-kernel/src/wasm_runner.rs` | `WasmToolRunner` -- Wasmtime-based tool execution engine |

### Files to Modify

| File | Change |
|---|---|
| `crates/clawft-kernel/Cargo.toml` | Add `wasmtime` dep behind `wasm-sandbox` feature |
| `crates/clawft-kernel/src/lib.rs` | Conditionally export `wasm_runner` module |
| `crates/clawft-core/src/tools/registry.rs` | Add `WasmToolSource` integration point |
| `crates/clawft-core/src/agent/sandbox.rs` | Add WASM sandbox policy variant to `SandboxEnforcer` |

### Key Types

**WasmToolRunner** (`wasm_runner.rs`):
```rust
pub struct WasmToolRunner {
    engine: wasmtime::Engine,
    linker: wasmtime::Linker<ToolState>,
    config: WasmSandboxConfig,
}

pub struct WasmSandboxConfig {
    pub max_fuel: u64,
    pub max_memory_bytes: usize,
    pub max_execution_time: Duration,
    pub allowed_host_calls: Vec<String>,
    pub wasi_enabled: bool,
}

pub struct ToolState {
    pub tool_name: String,
    pub stdin: Vec<u8>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub env: HashMap<String, String>,
}

pub struct WasmToolResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub fuel_consumed: u64,
    pub memory_peak: usize,
    pub execution_time: Duration,
}

impl WasmToolRunner {
    pub fn new(config: WasmSandboxConfig) -> Result<Self>;
    pub async fn load_tool(&self, name: &str, wasm_bytes: &[u8]) -> Result<WasmTool>;
    pub async fn execute(&self, tool: &WasmTool, input: serde_json::Value) -> Result<WasmToolResult>;
    pub fn validate_wasm(&self, wasm_bytes: &[u8]) -> Result<WasmValidation>;
}

pub struct WasmTool {
    module: wasmtime::Module,
    name: String,
    schema: Option<serde_json::Value>,
}

pub struct WasmValidation {
    pub valid: bool,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub estimated_memory: usize,
    pub warnings: Vec<String>,
}
```

### Default Sandbox Limits

| Limit | Default | Rationale |
|---|---|---|
| Fuel | 1,000,000 | ~100ms of execution on modern hardware |
| Memory | 16 MB | Sufficient for most tools, prevents allocation bombs |
| Execution time | 30 seconds | Wall-clock timeout as safety net |
| Host calls | `[]` (none) | Tools must explicitly declare needed host APIs |
| WASI | false | Opt-in; enables basic I/O but no filesystem |

---

## P -- Pseudocode

### WASM Tool Loading

```
fn WasmToolRunner::load_tool(name, wasm_bytes):
    // 1. Validate WASM module
    validation = validate_wasm(wasm_bytes)?
    if not validation.valid:
        return Err(InvalidWasm(validation.warnings))

    // 2. Check estimated memory against limit
    if validation.estimated_memory > config.max_memory_bytes:
        return Err(MemoryLimitExceeded)

    // 3. Compile module
    module = wasmtime::Module::new(engine, wasm_bytes)?

    // 4. Extract tool schema from exports (if present)
    schema = if module.exports("tool_schema"):
        // Call schema export to get JSON schema
        Some(extract_schema(module))
    else:
        None

    return WasmTool { module, name, schema }
```

### WASM Tool Execution

```
fn WasmToolRunner::execute(tool, input):
    // 1. Create store with fuel and memory limits
    store_config = wasmtime::Config::new()
    store_config.consume_fuel(true)
    store_config.max_memory(config.max_memory_bytes)

    store = wasmtime::Store::new(engine, ToolState::new(tool.name))
    store.set_fuel(config.max_fuel)

    // 2. Instantiate module with linker
    instance = linker.instantiate(store, tool.module)?

    // 3. Write input to WASM memory
    input_json = serde_json::to_string(input)?
    write_to_wasm_memory(instance, "input", input_json)

    // 4. Execute with timeout
    start = Instant::now()
    result = tokio::time::timeout(config.max_execution_time, async {
        // Call the tool's main export
        entry = instance.get_func("execute")?
        entry.call(store, &[])?
    }).await

    // 5. Read output
    match result:
        Ok(Ok(_)):
            stdout = read_wasm_memory(instance, "stdout")
            stderr = read_wasm_memory(instance, "stderr")
            fuel_consumed = config.max_fuel - store.get_fuel()
            return Ok(WasmToolResult {
                stdout, stderr,
                exit_code: 0,
                fuel_consumed,
                memory_peak: store.memory_usage(),
                execution_time: start.elapsed(),
            })
        Ok(Err(trap)):
            if trap.is_fuel_exhausted():
                return Err(FuelExhausted(config.max_fuel))
            if trap.is_memory_limit():
                return Err(MemoryLimitExceeded)
            return Err(WasmTrap(trap))
        Err(_timeout):
            return Err(ExecutionTimeout(config.max_execution_time))
```

### Tool Registry Integration

```
fn ToolRegistry::register_wasm_tool(name, wasm_bytes, config):
    runner = WasmToolRunner::new(config)?
    tool = runner.load_tool(name, wasm_bytes)?

    // Register as a normal tool with WASM execution backend
    self.register(ToolDefinition {
        name,
        schema: tool.schema,
        executor: ToolExecutor::Wasm { runner, tool },
    })
```

---

## A -- Architecture

### Component Relationships

```
WasmToolRunner
  |
  +-- wasmtime::Engine (shared, one per runner)
  |     +-- Config (fuel enabled, memory limits)
  |
  +-- wasmtime::Linker<ToolState>
  |     +-- Host function bindings (if any)
  |     +-- WASI bindings (if wasi_enabled)
  |
  +-- WasmTool (loaded module + metadata)
  |
  +-- Per-execution:
        +-- wasmtime::Store<ToolState> (isolated per call)
        +-- wasmtime::Instance (module instantiation)
        +-- Fuel tracking
        +-- Memory tracking
```

### Integration Points

1. **ToolRegistry**: `WasmToolRunner` registers tools as a new `ToolExecutor::Wasm` variant. The existing tool call dispatch checks the executor type and routes WASM tools through the runner.

2. **PluginSandbox reuse**: The existing `PluginSandbox` in `clawft-wasm/src/sandbox.rs` has similar patterns (WASI filesystem stubs, memory limits). K3 reuses these patterns but upgrades to Wasmtime's native fuel metering (PluginSandbox uses a basic instruction counter).

3. **SandboxEnforcer**: The `SandboxEnforcer` in `clawft-core/src/agent/sandbox.rs` gets a new variant for WASM tools that delegates to `WasmToolRunner` instead of doing filesystem-level sandboxing.

4. **Feature gate**: `wasm-sandbox` feature in `clawft-kernel/Cargo.toml` controls Wasmtime dependency. When disabled, `WasmToolRunner` is not compiled and WASM tools are rejected at registration time.

### Security Boundaries

```
Host Process (weft)
  |
  +-- Kernel (normal process permissions)
  |     |
  |     +-- WasmToolRunner
  |           |
  |           +-- [WASM Sandbox] (isolated)
  |                 - No host filesystem access
  |                 - No network access (unless explicitly bound)
  |                 - Fuel-limited CPU
  |                 - Memory-capped
  |                 - Only declared host calls available
```

### Ruvector Integration (Doc 07)

When the `ruvector-wasm` feature gate is enabled, an ultra-minimal WASM runtime from
ruvector supplements wasmtime for micro-tools. The wasmtime-only approach remains the
fallback when the feature gate is disabled. See `07-ruvector-deep-integration.md`
for full adapter code.

| Custom Component | Ruvector Replacement | Feature Gate | Benefit |
|---|---|---|---|
| Wasmtime-only WASM execution | `rvf-wasm` micro-runtime (5.5KB, 14 C exports) + wasmtime | `ruvector-wasm` | Sub-ms execution for micro-tools; 1000x faster than wasmtime cold-start for simple ops |
| Custom fuel metering via wasmtime | `ruvector-cognitive-container::EpochController` budget | `ruvector-wasm` | Unified budget system (`try_budget()`, `consume()`) across WASM and native execution |
| (none -- new capability) | RVF containers can package WASM tools in `.rvf` format | `ruvector-containers` | Single-file packaging of WASM tools with cryptographic attestation |

A `WasmDispatcher` routes execution to the appropriate runtime: `rvf-wasm` for modules
under 64KB without WASI imports (micro-tools), wasmtime for everything else (full tools).

Cross-reference: `07-ruvector-deep-integration.md`, Section 3 "Phase K3: WASM Tool Sandboxing".

### K2 Symposium Decisions

The following decisions from the K2 Symposium directly shape K3 scope and implementation.
Reference doc: `docs/weftos/k2-symposium/08-symposium-results-report.md`

| Decision | Summary | Commitment |
|----------|---------|------------|
| D3 | `SpawnBackend::Wasm` baked into the API now -- WASM is a first-class spawn target, not an afterthought | C1 |
| D4 | Layered protocol: kernel IPC -> ServiceApi -> adapters. K3 defines the `ServiceApi` trait; Shell and MCP are the first two adapters | C2 |
| D7 | Defense in depth: dual-layer gate in `A2ARouter` + `agent_loop`. Both layers must pass before a WASM tool call proceeds | C4 |
| D8 | Immutable API contracts stored in ExoChain. Service registration writes the contract to the chain; runtime enforcement verifies against it | C3 |
| D9 | Universal witness by default, configurable per service. Every WASM tool execution emits a witness entry unless the service opts out | -- |
| D10 | Shell commands: WASM compilation + container sandbox. Shell pipelines compile to WASM modules that are chain-linked and sandboxed | C5 |
| D11 | Post-quantum dual signing enabled now. Ed25519 + ML-DSA-65 signatures on WASM module attestation | C6 |
| D17 | Tiny-dancer routing hints layered on governance enforcement. `ruvector-tiny-dancer-core` provides semantic routing hints that inform tool dispatch without overriding governance gates | -- |
| D20 | Configurable N-dimensional `EffectVector`. Tool execution metrics feed into an N-dim vector (default 6-dim, extensible) for scoring and anomaly detection | C9 |

Key crates for K3:
- **ruvector-tiny-dancer-core**: semantic routing hints for tool dispatch (D17)
- **cognitum-gate-kernel**: kernel-level audit verification for dual-layer gate (D7)
- **ruvector-snapshot**: point-in-time state snapshots for WASM execution checkpoints

---

## R -- Refinement

### Edge Cases

1. **Invalid WASM binary**: `validate_wasm()` rejects with specific error (not a WASM module, missing exports, unsupported imports)
2. **Fuel exhaustion mid-execution**: Wasmtime trap caught, execution result returned with `FuelExhausted` error and partial output
3. **Memory allocation bomb**: Wasmtime's `max_memory` setting prevents allocation beyond limit; trap caught cleanly
4. **Infinite loop**: Wall-clock timeout (30s default) kills execution even if fuel hasn't run out (fuel counting has edge cases with host calls)
5. **Concurrent tool executions**: Each execution gets its own `Store`, so multiple WASM tools can run concurrently safely
6. **Large WASM modules**: Module size checked before compilation; reject modules > 10MB (configurable)
7. **WASI filesystem access**: When `wasi_enabled`, only virtual filesystem is available (no host paths mapped by default)

### Backward Compatibility

- Feature-gated: `wasm-sandbox` not in default features, so existing builds unchanged
- `ToolRegistry` changes are additive; existing native tool execution unaffected
- `SandboxEnforcer` changes are additive; new variant, existing variants unchanged

### Error Handling

- `WasmError` enum: `InvalidModule`, `CompilationFailed`, `FuelExhausted`, `MemoryLimitExceeded`, `ExecutionTimeout`, `WasmTrap`, `HostCallDenied`, `ModuleTooLarge`
- All errors include tool name, resource limits that were configured, and actual usage at failure point
- Fuel exhaustion and memory limit errors are expected operational conditions, not panics

---

## C -- Completion

### Exit Criteria

- [x] `WasmToolRunner` compiles with `--features wasm-sandbox`
- [x] WASM tool loads from bytes and validates exports
- [x] Tool executes and returns stdout/stderr/exit_code
- [x] Fuel exhaustion terminates execution cleanly (not a panic)
- [x] Memory limit prevents allocation beyond configured cap — Wasmtime Store memory limits
- [x] Wall-clock timeout terminates long-running tools — tokio::time::timeout
- [x] Invalid WASM binary rejected with clear error
- [x] Module size check rejects oversized modules
- [x] `ToolRegistry` accepts WASM tools via `register_wasm_tool()`
- [x] WASM tools appear in tool listing alongside native tools
- [x] Host filesystem not accessible from WASM sandbox — WASI with no preopens
- [x] Multiple WASM tools execute concurrently without interference — per-call Store isolation
- [x] Feature gate: `clawft-kernel` compiles without `wasm-sandbox` feature
- [x] All workspace tests pass — 579 with all features, 487 baseline
- [x] Clippy clean
- [x] ServiceApi trait defined and Shell/MCP adapters implemented (C2)
- [x] Dual-layer gate in A2ARouter operational (C4)
- [x] Chain-anchored service contracts on registration (C3)
- [x] WASM-compiled shell pipeline produces chain-linked modules (C5)
- [ ] Training data collection active for all WASM execution metrics (D18) — deferred to K5

### Testing Verification

```bash
# WASM runner unit tests (requires wasm-sandbox feature)
cargo test -p clawft-kernel --features wasm-sandbox -- wasm_runner

# Test fuel exhaustion
cargo test -p clawft-kernel --features wasm-sandbox -- test_fuel_exhaustion

# Test memory limit
cargo test -p clawft-kernel --features wasm-sandbox -- test_memory_limit

# Test without feature (should compile, WASM tools rejected)
cargo test -p clawft-kernel

# Regression check
scripts/build.sh test

# Clippy with feature
cargo clippy -p clawft-kernel --features wasm-sandbox -- -D warnings
```
