# Phase K1: Supervisor + RBAC + ExoChain Integration

**Phase ID**: K1
**Workstream**: W-KERNEL
**Duration**: Week 3-4
**Status**: Complete
**Goal**: Wire agent supervisor to actually execute agents, enforce capability-based RBAC, integrate agent lifecycle with resource tree and chain, persist chain across restarts, and add `weaver agent` CLI commands.

---

## S -- Specification

### What K0 Delivered (Starting Point)

K0 created `AgentSupervisor<P>` with `spawn()`, `stop()`, `restart()`, and `inspect()` methods that manage `ProcessEntry` records in the process table. However:

1. `spawn()` created a ProcessEntry but didn't execute any agent work
2. `CapabilityChecker` existed with full logic but was never called
3. No agent presence in the resource tree or chain
4. Chain was ephemeral (fresh genesis on every boot)
5. No `weaver agent` CLI commands
6. IPC broadcast-only, no per-PID routing or access control
7. No shared RVF payload type in IPC messages

### What K1 Changed

| Area | Change |
|------|--------|
| **Supervisor** | `spawn_and_run()` accepts a work closure, runs as tokio task, tracks JoinHandle |
| **Running agents** | `DashMap<Pid, JoinHandle>` tracks running tasks; `abort_all()` on shutdown |
| **Exochain wiring** | Supervisor holds optional `TreeManager` + `ChainManager` (cfg exochain) |
| **Agent tree nodes** | `tree_manager.register_agent()` / `unregister_agent()` on spawn/stop |
| **Chain events** | `agent.spawn`, `agent.exit`, `agent.restart` events in chain |
| **Chain persistence** | `save_to_file()` / `load_from_file()` with checkpoint path config |
| **Boot restore** | Chain restores from checkpoint file on boot, falls back to fresh genesis |
| **GateBackend** | Trait abstracting Permit/Defer/Deny decisions (replaces binary check) |
| **IPC RBAC** | `send_checked()` enforces IpcScope via CapabilityChecker (cfg exochain) |
| **RVF IPC** | `MessagePayload::Rvf { segment_type, data }` for typed agent messages |
| **CLI** | `weaver agent spawn/stop/restart/inspect/list/send/attach` commands |
| **Daemon RPC** | 7 new dispatch handlers for agent lifecycle + IPC send |

### Files Created

| File | Purpose | LOC |
|------|---------|-----|
| `crates/clawft-kernel/src/gate.rs` | `GateBackend` trait, `GateDecision` enum, `CapabilityGate` impl | ~80 |
| `crates/clawft-weave/src/commands/agent_cmd.rs` | `weaver agent` CLI command group | ~220 |

### Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/supervisor.rs` | `spawn_and_run()`, running agent tracking, exochain wiring |
| `crates/clawft-kernel/src/chain.rs` | `save_to_file()`, `load_from_file()`, `LocalChain::from_events()` |
| `crates/clawft-kernel/src/tree_manager.rs` | `register_agent()`, `unregister_agent()`, `update_agent_state()` |
| `crates/clawft-kernel/src/ipc.rs` | `send_checked()` (cfg exochain), `MessagePayload::Rvf` variant |
| `crates/clawft-kernel/src/boot.rs` | Chain restore from checkpoint, supervisor exochain wiring, shutdown persistence |
| `crates/clawft-kernel/src/lib.rs` | `pub mod gate` (exochain-gated), re-exports |
| `crates/clawft-types/src/config/kernel.rs` | `checkpoint_path: Option<String>` in ChainConfig |
| `crates/clawft-weave/src/commands/mod.rs` | `pub mod agent_cmd` |
| `crates/clawft-weave/src/main.rs` | Agent command routing |
| `crates/clawft-weave/src/protocol.rs` | Agent + IPC RPC types |
| `crates/clawft-weave/src/daemon.rs` | 7 agent dispatch handlers |

---

## P -- Pseudocode

### spawn_and_run (the K1 bridge)

```
fn AgentSupervisor::spawn_and_run(request, work_closure):
    // 1. Create process entry via existing spawn()
    result = self.spawn(request)?

    // 2. Register in resource tree (exochain)
    if tree_manager.is_some():
        tree_manager.register_agent(agent_id, pid, capabilities)

    // 3. Transition to Running
    process_table.update_state(pid, Running)

    // 4. Spawn tokio task
    handle = tokio::spawn(async {
        exit_code = work_closure(pid, cancel_token).await

        // Cleanup
        process_table.update_state(pid, Exited(exit_code))
        tree_manager?.unregister_agent(agent_id, pid, exit_code)
        chain_manager?.append("supervisor", "agent.exit", {pid, exit_code})
        running_agents.remove(pid)
    })

    running_agents.insert(pid, handle)
    return result
```

### GateBackend (unified permission interface)

```
trait GateBackend:
    fn check(agent_id, action, context) -> GateDecision

enum GateDecision:
    Permit { token: Option<Vec<u8>> }
    Defer { reason: String }
    Deny { reason: String, receipt: Option<Vec<u8>> }

// CapabilityGate: always Permit or Deny (binary, no Defer)
// TileZeroGate (future tilezero feature): three-way with crypto receipts
```

### Chain Persistence

```
fn ChainManager::save_to_file(path):
    events = self.local.events()
    checkpoint = { events, metadata: { chain_id, sequence, prev_hash } }
    fs::write(path, serde_json::to_string_pretty(checkpoint))

fn ChainManager::load_from_file(path, checkpoint_interval):
    contents = fs::read_to_string(path)
    checkpoint = serde_json::from_str(contents)
    verify_integrity(checkpoint.events)
    chain = LocalChain::from_events(events, checkpoint_interval)
    return ChainManager { local: Mutex::new(chain) }
```

---

## A -- Architecture

### Component Relationships

```
AgentSupervisor<P>
  |
  +-- ProcessTable (shared with Kernel)
  |     +-- ProcessEntry { capabilities, cancel_token }
  |
  +-- running_agents: DashMap<Pid, JoinHandle>
  |
  +-- KernelIpc
  |     +-- send_checked() [cfg exochain]
  |     +-- MessagePayload::Rvf { segment_type, data }
  |
  +-- TreeManager [cfg exochain, optional]
  |     +-- register_agent() -> tree node + chain event
  |     +-- unregister_agent() -> tree update + chain event
  |
  +-- ChainManager [cfg exochain, optional]
  |     +-- save_to_file() / load_from_file()
  |     +-- agent.spawn / agent.exit / agent.restart events
  |
  +-- GateBackend (trait object)
        +-- CapabilityGate (default: binary Permit/Deny)
        +-- TileZeroGate (future: three-way + crypto receipts)
```

### Weaver CLI Flow

```
weaver agent spawn <id>
  |
  +-> DaemonClient -> Unix socket -> daemon.rs
  |     method: "agent.spawn"
  |     params: { agent_id, parent_pid }
  |
  +-> Daemon handler:
  |     kernel.supervisor.spawn(request)
  |     -> ProcessEntry created, PID returned
  |
  +-> Response: { pid, agent_id }
```

### Feature Gates

| Feature | What it enables |
|---------|----------------|
| (default) | Supervisor spawn/stop/restart, process table, CapabilityChecker |
| `exochain` | TreeManager wiring, ChainManager wiring, send_checked(), GateBackend, chain persistence |
| `tilezero` (planned) | cognitum-gate-tilezero as GateBackend implementation |

---

## R -- Refinement

### Decisions Made

1. **Factory closure pattern** (Option A from plan): Supervisor accepts `FnOnce(Pid, CancellationToken) -> Future<Output = i32>` instead of holding AppContext. Keeps supervisor in `clawft-kernel` without depending on `clawft-core` internals.

2. **DashMap for JoinHandles**: Reuses the workspace `dashmap` dependency for concurrent running agent tracking.

3. **Builder pattern for exochain**: `with_exochain(tree_manager, chain_manager)` avoids changing the existing constructor signature.

4. **Force stop aborts directly**: When force-stopping, the task handle is aborted and cleanup (tree/chain) happens in the stop method itself, not the spawned task.

5. **Chain restore is best-effort**: If checkpoint file is corrupt, log error and start fresh genesis. Don't block boot.

6. **Agent tree nodes persist after exit**: `unregister_agent()` updates metadata (state=exited, exit_code, stop_time) but does NOT remove the node. This preserves the audit trail.

### Edge Cases Handled

- Spawn with table full: returns `KernelError::ProcessTableFull`
- Stop already-exited process: idempotent, returns Ok
- Force stop after task already exited: `running_agents.remove()` returns None, no abort needed
- Chain checkpoint path missing: skip save, no error
- Chain checkpoint file corrupt: log warning, start fresh genesis
- Restart logs chain event linking old PID to new PID

---

## C -- Completion

### Exit Criteria (all met)

- [x] `spawn_and_run()` creates process entry AND runs work as tokio task
- [x] Spawned agent appears in process table as `Running`
- [x] Agent transitions to `Exited(exit_code)` when work completes
- [x] Graceful stop cancels via CancellationToken
- [x] Force stop aborts task handle via `JoinHandle::abort()`
- [x] `abort_all()` cancels all running agents on shutdown
- [x] Agent restart creates new PID linked via `parent_pid`, chain event logged
- [x] `GateBackend` trait with Permit/Defer/Deny decisions
- [x] `CapabilityGate` implements GateBackend (binary Permit/Deny)
- [x] Agent spawn creates tree node + chain event (exochain)
- [x] Agent stop updates tree node + chain event (exochain)
- [x] Chain saves to checkpoint file on shutdown
- [x] Chain restores from checkpoint file on boot
- [x] `send_checked()` enforces IpcScope (exochain)
- [x] `MessagePayload::Rvf` variant carries segment_type + data
- [x] `weaver agent spawn/stop/restart/inspect/list/send` CLI wired
- [x] Daemon dispatch handlers for all agent operations
- [x] All 270 kernel tests pass (including 4 new supervisor tests)
- [x] All workspace tests pass (1100+ total)
- [x] Clippy clean for default and exochain features

### Test Summary

| Test | Verifies |
|------|----------|
| `spawn_and_run_executes_work` | Work closure runs, exit code captured, state transitions |
| `spawn_and_run_respects_cancellation` | CancellationToken stops the work closure |
| `spawn_and_run_force_stop_aborts` | Force stop aborts task handle |
| `abort_all_clears_running_agents` | All running tasks cancelled on bulk abort |

### Verification Commands

```bash
# Compile check (all feature combos)
scripts/build.sh check
cargo check -p clawft-kernel --features exochain

# Tests
scripts/build.sh test
cargo test -p clawft-kernel -- supervisor

# Clippy
scripts/build.sh clippy
cargo clippy -p clawft-kernel --features exochain -- -D warnings

# CLI smoke test (requires daemon)
weaver agent spawn test-agent
weaver agent list
weaver agent inspect <pid>
weaver agent send <pid> "hello"
weaver agent stop <pid>
weaver chain local -c 20   # shows agent.spawn/agent.stop events
weaver resource tree        # shows /kernel/agents/test-agent node
```

### Deferred to K2+

- **cognitum-gate-tilezero integration**: `GateBackend` trait is ready; TileZero adapter behind `tilezero` feature gate is K2 work
- **rvf-wire daemon RPC**: Replace JSON-over-Unix-socket with RVF segment framing (stretch goal)
- **Per-PID message routing**: Agents subscribe and only receive targeted messages (K2 A2A)
- **Pre-tool-call hook in AgentLoop**: Capability check before each tool execution (K2)
- **Agent attach**: `weaver agent attach <pid>` is stubbed (prints not-yet-implemented)
- **NodeScoring lifecycle integration**: Agent exit scoring via supervisor (compute performance observation from runtime metrics, blend into agent's node scoring), gate decision trust nudges (Permit -> trust boost alpha=0.1, Deny -> trust penalty alpha=0.1)
- **NodeScoring CLI/RPC**: `weaver resource score <id>`, `weaver resource rank [--weights] [--count]` commands and daemon handlers (`resource.score`, `resource.rank` dispatch)
