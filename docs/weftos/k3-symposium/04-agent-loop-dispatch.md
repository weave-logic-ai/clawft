# Panel 4: Agent Loop & Dispatch Wiring

**Date**: 2026-03-04
**Presenter**: Integration Architect
**Scope**: exec handler, tool_registry parameter threading, daemon integration
**Branch**: feature/weftos-kernel-sprint
**Files Under Review**: `crates/clawft-kernel/src/agent_loop.rs` (1,353 lines), `crates/clawft-weave/src/daemon.rs` (1,647 lines)

---

## 1. Executive Summary

The K3 exec handler transforms the agent loop from a message echo service
into a tool dispatch engine. The `ToolRegistry` parameter threads cleanly
through the agent loop signature, handle_message, and into the exec match
arm. The daemon correctly constructs and wires the registry into every
spawned agent. Chain logging captures both success and failure events.

**Verdict**: Integration is clean and well-structured. The backwards-
compatible echo mode preserves existing behavior. The daemon wiring is
correct but creates a new ToolRegistry per agent spawn -- this should
be shared.

---

## 2. Agent Loop Signature

### 2.1 Updated Signature

```rust
pub async fn kernel_agent_loop(
    pid: Pid,
    cancel: CancellationToken,
    mut inbox: mpsc::Receiver<KernelMessage>,
    a2a: Arc<A2ARouter>,
    cron: Arc<CronService>,
    process_table: Arc<ProcessTable>,
    tool_registry: Option<Arc<ToolRegistry>>,     // K3 addition
    #[cfg(feature = "exochain")] chain: ...,
    #[cfg(feature = "exochain")] gate: ...,
) -> i32
```

The `tool_registry` is:
- `Option<Arc<...>>` -- optional and thread-safe
- Positioned after `process_table`, before cfg-gated params
- Passed to `handle_message` as `tool_registry.as_deref()`

### 2.2 Parameter Threading

```
kernel_agent_loop(tool_registry: Option<Arc<ToolRegistry>>)
  |
  v
handle_message(tool_registry: Option<&ToolRegistry>)
  |
  v
"exec" match arm: registry.execute(tool_name, args)
```

The Arc is unwrapped to a reference for handle_message -- no unnecessary
cloning per message.

---

## 3. Exec Handler Implementation

### 3.1 Dispatch Logic

```rust
"exec" => {
    let tool_name = cmd_value.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let args = cmd_value.get("args").cloned().unwrap_or(json!({}));

    if tool_name.is_empty() {
        // Backwards compat: echo mode
        json!({"status": "ok", "echo": text, "pid": pid})
    } else if let Some(registry) = tool_registry {
        match registry.execute(tool_name, args) {
            Ok(result) => {
                // Chain log: tool.exec (ok)
                json!({"status": "ok", "tool": tool_name, "result": result, "pid": pid})
            }
            Err(e) => {
                // Chain log: tool.exec (error)
                json!({"error": e.to_string(), "tool": tool_name, "pid": pid})
            }
        }
    } else {
        json!({"error": "tool registry not available", "tool": tool_name, "pid": pid})
    }
}
```

### 3.2 Three Code Paths

| Path | Condition | Response |
|------|-----------|----------|
| Echo (legacy) | `tool` field empty | `{status: "ok", echo: text, pid}` |
| Tool dispatch | `tool` present + registry available | `{status: "ok", tool, result, pid}` or `{error, tool, pid}` |
| No registry | `tool` present but no registry | `{error: "tool registry not available", tool, pid}` |

### 3.3 Chain Logging

On success:
```json
{
    "source": "tool",
    "kind": "tool.exec",
    "payload": {"tool": "fs.read_file", "pid": 1, "status": "ok"}
}
```

On error:
```json
{
    "source": "tool",
    "kind": "tool.exec",
    "payload": {"tool": "fs.read_file", "pid": 1, "status": "error", "error": "..."}
}
```

Both paths emit chain events (when exochain is enabled).

---

## 4. Daemon Integration

### 4.1 ToolRegistry Construction

In the `"agent.spawn"` RPC handler (daemon.rs:1163-1171):

```rust
let tool_registry: Arc<ToolRegistry> = {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(FsReadFileTool::new()));
    registry.register(Arc::new(AgentSpawnTool::new(
        k.process_table().clone(),
    )));
    Arc::new(registry)
};
```

### 4.2 Closure Wiring

The registry is cloned into the spawn closure:

```rust
let tool_reg_clone = tool_registry.clone();

move |pid, cancel| {
    let inbox = a2a_clone.create_inbox(pid);
    async move {
        kernel_agent_loop(
            pid, cancel, inbox,
            a2a_clone, cron_clone, pt_clone,
            Some(tool_reg_clone),  // <-- ToolRegistry
            chain_clone, gate,
        ).await
    }
}
```

### 4.3 GovernanceGate Construction

The daemon also constructs a GovernanceGate for each spawned agent:

- Threshold: 0.8 (80% risk tolerance)
- Rules:
  - `exec-guard`: Judicial branch, Blocking severity
  - `cron-warn`: Executive branch, Warning severity
- Chain integration: if chain_manager exists

---

## 5. Findings

### 5.1 Strengths

1. **Clean parameter threading** -- no global state, explicit dependency injection
2. **Backwards-compatible** -- empty tool name falls back to echo mode
3. **Three-state dispatch** -- handles tool present/absent/registry missing
4. **Chain logging on both paths** -- success and failure both audited
5. **All 12 agent_loop tests pass** -- including 4 exochain-gated tests

### 5.2 Gaps

1. **ToolRegistry created per agent spawn** -- FsReadFileTool::new() calls
   `builtin_tool_catalog()` each time, allocating 27 BuiltinToolSpec structs.
   Should be a shared kernel-level singleton.

2. **AgentSpawnTool gets a ProcessTable clone** -- this is correct but
   `k.process_table().clone()` clones an `Arc`, not the table itself,
   so there's no data duplication. Just worth noting the reference semantics.

3. **No fuel/memory metrics in chain event** -- the `tool.exec` chain event
   records tool name and status but not `fuel_consumed` or `memory_peak`.
   These metrics are available from `WasmToolResult` but not plumbed
   through to chain logging yet.

4. **Gate check uses generic action** -- as noted by Panel 3 (Security),
   the gate check at line 140 uses `"tool.exec"` for all exec commands
   rather than the tool-specific `gate_action`.

### 5.3 Recommendations

| ID | Recommendation | Phase | Priority |
|----|---------------|-------|----------|
| DR-1 | Move ToolRegistry to kernel boot, share across agents | K4 | High |
| DR-2 | Include fuel/memory metrics in tool.exec chain events | K4 | Medium |
| DR-3 | Use tool-specific gate_action (echoes SR-1 from Panel 3) | K4 | High |
| DR-4 | Add tool execution timeout metric to WasmToolResult | K4 | Low |

---

## 6. Test Results (Live Run)

Agent loop tests (with exochain):

```
running 12 tests
test agent_loop::tests::cancellation_exits_cleanly ... ok
test agent_loop::tests::gate_deny_blocks_exec ... ok
test agent_loop::tests::cron_add_via_agent ... ok
test agent_loop::tests::chain_logs_ipc_recv_ack ... ok
test agent_loop::tests::gate_permit_allows_exec ... ok
test agent_loop::tests::ping_command ... ok
test agent_loop::tests::chain_logs_suspend_resume ... ok
test agent_loop::tests::rvf_json_payload_processed ... ok
test agent_loop::tests::resource_usage_increments ... ok
test agent_loop::tests::unknown_command ... ok
test agent_loop::tests::suspend_resume_cycle ... ok
test agent_loop::tests::rvf_opaque_binary_acknowledged ... ok
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured
```

**Verdict**: PASS -- all 12 tests green. Gate deny/permit both verified.
Chain logging for IPC recv/ack verified. Suspend/resume cycle verified.
