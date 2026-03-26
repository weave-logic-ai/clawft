# Phase K2b: Kernel Work-Loop Hardening

**Phase ID**: K2b
**Workstream**: W-KERNEL
**Duration**: ~1 day (continuation of K2)
**Status**: Planned
**Goal**: Close the 6 gaps identified in K2 gap analysis — health monitor loop, agent watchdog, graceful shutdown coordination, resource usage tracking, agent suspend/resume, and gate-checked commands — so that K2 delivers a hardened, production-quality kernel substrate for K3+.

---

## S -- Specification

### Gap Analysis

K2 delivered functional A2A IPC, topic pub/sub, RVF deep integration, and cryptographic chain integrity. But a systematic review of all kernel work-loops identified 6 gaps where infrastructure exists but is either unused, undriven, or incomplete. These gaps mean the kernel silently ignores agent failures, never runs health checks, never updates resource counters, and doesn't enforce capabilities at command execution time.

### Gap 1: Health Monitor Loop (Critical)

**What exists**: `HealthSystem.aggregate()` (`health.rs:86-131`) queries all registered services and produces `OverallHealth`. `health_check_interval_secs` is configured at boot (default 30s). `HealthStatus` and `OverallHealth` types are complete with Healthy/Degraded/Unhealthy/Down variants. CronService implements `SystemService::health_check()`.

**What's missing**: No background loop calls `aggregate()`. The interval field is set but never used. The daemon has a cron tick loop (every 1s) but no health tick loop.

**Impact**: Services (cron, cluster) could go unhealthy with zero detection. `weaver kernel status` reports static state, not live health.

**Fix**: Background tokio task in `daemon.rs` alongside the cron tick loop. Calls `health.aggregate()` every `N` seconds, logs results to chain (`health.check` event), emits kernel event log entries.

### Gap 2: Agent Watchdog Sweep (Critical)

**What exists**: `AgentSupervisor` tracks `JoinHandle<()>` per agent in `running_agents: DashMap<Pid, JoinHandle>`. The task completion handler (lines 253-308 of supervisor.rs) transitions to Exited, blends scoring, unregisters from tree, logs chain event, and removes from DashMap.

**What's missing**: If the tokio task panics *before* reaching the cleanup handler, or if `handle.abort()` cancels it mid-cleanup, the PID stays "Running" in the process table forever. Nobody polls `is_finished()` on the handles.

**Impact**: Phantom "Running" processes in the process table. `weaver kernel ps` shows ghosts. Agents that panic are never cleaned up.

**Fix**: New `watchdog_sweep()` method on `AgentSupervisor`. Iterates `running_agents`, checks `is_finished()` on each handle, transitions stale PIDs to `Exited(-2)` (watchdog-killed), logs chain event. Called from a periodic daemon task every 5 seconds.

### Gap 3: Graceful Shutdown Coordination (Critical)

**What exists**: `supervisor.abort_all()` (supervisor.rs:506-511) iterates `running_agents` and calls `handle.abort()` on every agent, then clears the map.

**What's missing**: `abort()` cancels the tokio future mid-execution. The cleanup handler in `spawn_and_run` (scoring blend at line 277, tree unregister at line 284, chain exit event at line 292) **never runs** because the future is aborted. On daemon Ctrl+C, agents lose their exit audit trail — no chain events, no tree cleanup, no scoring updates.

**Impact**: Every daemon restart loses the exit audit trail for all running agents. Chain integrity is preserved but completeness is not — there's no `agent.exit` event for agents that were running at shutdown.

**Fix**: New `shutdown_all(timeout: Duration) -> Vec<(Pid, i32)>` method. Cancels all agent cancellation tokens (triggering graceful exit via the `cancel.cancelled()` branch in `kernel_agent_loop`), then `tokio::select!` on `join_all(handles)` vs `tokio::time::sleep(timeout)`. On timeout, abort survivors. Returns list of (pid, exit_code). Daemon shutdown calls this instead of `abort_all()`.

### Gap 4: Resource Usage Tracking (Critical)

**What exists**: `ResourceUsage` struct (process.rs:72-82) with `memory_bytes`, `cpu_time_ms`, `tool_calls`, `messages_sent`. `ProcessTable::update_resources()` accepts a `ResourceUsage` and stores it. All 4 counters default to 0.

**What's missing**: Nobody increments any counter, ever. The agent loop sends replies but doesn't count them. The A2ARouter delivers messages but doesn't update the sender's stats. `process_table.update_resources()` is never called.

**Impact**: `weaver agent inspect <pid>` always shows zero for all resource counters. No data for resource limit enforcement (K3). No data for scoring/training (K5/K6).

**Fix**: Instrument `kernel_agent_loop` to maintain a local `ResourceUsage` counter. Increment `messages_sent` on each reply. Increment `tool_calls` on `exec` command. Track wall-clock time as `cpu_time_ms`. Call `process_table.update_resources()` after each message processing cycle. Pass process table handle into the agent loop.

### Gap 5: Agent Suspend/Resume (Important)

**What exists**: `ProcessState::Suspended` with valid transitions `Running -> Suspended -> Running`. `ProcessSignal::Suspend` and `ProcessSignal::Resume` message variants. The process table state machine enforces these transitions.

**What's missing**: No mechanism to suspend. The agent loop doesn't check process state — it processes messages regardless. No IPC command to trigger suspend. No CLI command.

**Impact**: `ProcessState::Suspended` is dead code. Agents cannot be paused for debugging, migration, or resource conservation.

**Fix**: In `kernel_agent_loop`, check process state after receiving each message. If `Suspended`, enter a secondary `tokio::select!` loop that only listens for resume signals or cancellation. Add `suspend`/`resume` as built-in commands that update process state.

### Gap 6: Gate-Checked Commands (Important)

**What exists**: `GateBackend` trait (gate.rs) with `CapabilityGate` and `TileZeroGate` implementations. `GateDecision` enum (Permit/Defer/Deny). `CapabilityGate::check()` routes to `CapabilityChecker` based on action prefix (`tool.`, `ipc.`, `service.`). TileZeroGate logs chain events.

**What's missing**: The agent loop (`agent_loop.rs`) executes all commands without capability checking. An agent with `can_exec_tools: false` can still run `exec`. An agent with `IpcScope::None` can still run `cron.add` which creates kernel-level jobs.

**Impact**: Capability system is declarative but not enforced at the agent command level. Fine for dev/testing, unacceptable for multi-tenant or security-sensitive deployments.

**Fix**: Pass `Arc<dyn GateBackend>` into `kernel_agent_loop`. Before executing `exec`, `cron.add`, or `cron.remove`, call `gate.check(agent_id, action, context)`. On `Deny`, return error response with reason. On `Defer`, return "pending approval" response (placeholder for K5 human-in-the-loop).

---

## P -- Pseudocode

### Health Monitor Loop (daemon.rs)

```
fn spawn_health_monitor(kernel, shutdown_rx):
    interval = kernel.health_check_interval_secs
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(interval))
        loop {
            select! {
                _ = timer.tick() => {
                    let k = kernel.read().await
                    let (overall, results) = k.health().aggregate(k.services()).await
                    for (name, status) in results:
                        if status != Healthy:
                            k.event_log().warn("health", format!("{name}: {status}"))
                    if exochain:
                        cm.append("health", "health.check", json!({
                            "overall": overall.to_string(),
                            "services": results.len(),
                        }))
                }
                _ = shutdown_rx.changed() => break
            }
        }
    })
```

### Agent Watchdog Sweep (supervisor.rs)

```
fn AgentSupervisor::watchdog_sweep() -> Vec<(Pid, i32)>:
    let mut reaped = Vec::new()
    let finished: Vec<Pid> = self.running_agents.iter()
        .filter(|e| e.value().is_finished())
        .map(|e| *e.key())
        .collect()

    for pid in finished:
        if let Some((_, handle)) = self.running_agents.remove(&pid):
            let exit_code = match handle.await:
                Ok(()) => -2  // watchdog reaped (cleanup ran in task)
                Err(e) if e.is_panic() => -3  // task panicked
                Err(e) if e.is_cancelled() => -4  // task was cancelled
                Err(_) => -5  // unknown error

            // Only update if still showing as Running (not already cleaned up by task)
            if let Some(entry) = self.process_table.get(pid):
                if entry.state == Running:
                    process_table.update_state(pid, Exited(exit_code))
                    reaped.push((pid, exit_code))

            // Log chain event for unexpected exits
            if exochain && exit_code < 0:
                cm.append("supervisor", "agent.watchdog_reap",
                    json!({"pid": pid, "exit_code": exit_code}))

    reaped
```

### Graceful Shutdown All (supervisor.rs)

```
fn AgentSupervisor::shutdown_all(timeout: Duration) -> Vec<(Pid, i32)>:
    // 1. Cancel all agent tokens
    for entry in self.running_agents.iter():
        if let Some(proc) = process_table.get(*entry.key()):
            proc.cancel_token.cancel()

    // 2. Collect all handles
    let handles: Vec<(Pid, JoinHandle)> = self.running_agents
        .iter().map(|e| (*e.key(), /* need to remove */)).collect()
    // Actually drain the map
    let mut pids_handles = Vec::new()
    for (pid, handle) in self.running_agents.drain():
        pids_handles.push((pid, handle))

    // 3. Wait with timeout
    let results = Vec::new()
    let wait_all = async {
        for (pid, handle) in pids_handles:
            match handle.await:
                Ok(()) => results.push((pid, 0))
                Err(e) => results.push((pid, -1))
    }

    select! {
        _ = wait_all => {}
        _ = sleep(timeout) => {
            // Abort any remaining
            warn!("shutdown timeout, aborting remaining agents")
        }
    }

    results
```

### Resource Usage Tracking (agent_loop.rs)

```
fn kernel_agent_loop(pid, cancel, inbox, a2a, cron, process_table, ...):
    let started = Instant::now()
    let mut usage = ResourceUsage::default()

    loop {
        select! {
            _ = cancel.cancelled() => {
                // Final update
                usage.cpu_time_ms = started.elapsed().as_millis() as u64
                process_table.update_resources(pid, usage)
                return 0
            }
            msg = inbox.recv() => {
                match msg:
                    Some(message) => {
                        handle_message(...)
                        usage.messages_sent += 1
                        if cmd == "exec": usage.tool_calls += 1
                        // Periodic update (every 10 messages)
                        if usage.messages_sent % 10 == 0:
                            usage.cpu_time_ms = started.elapsed().as_millis() as u64
                            process_table.update_resources(pid, usage.clone())
                    }
                    None => return 0
            }
        }
    }
```

### Agent Suspend/Resume (agent_loop.rs)

```
fn handle_suspend(pid, cancel, inbox, process_table):
    // Entered when process state is Suspended
    loop {
        select! {
            _ = cancel.cancelled() => return Some(0)  // exit
            msg = inbox.recv() => {
                match msg:
                    Some(message) => {
                        // Only process resume command while suspended
                        if message.payload is Json({"cmd": "resume"}):
                            process_table.update_state(pid, Running)
                            return None  // resume normal loop
                        else:
                            // Queue or reject non-resume messages
                            reply with {"error": "agent suspended", "pid": pid}
                    }
                    None => return Some(0)
            }
        }
    }
```

### Gate-Checked Commands (agent_loop.rs)

```
fn handle_message_gated(pid, msg, gate, agent_id, ...):
    let cmd = extract_cmd(msg)

    // Gate check for privileged commands
    if cmd in ["exec", "cron.add", "cron.remove"]:
        let action = match cmd:
            "exec" => "tool.exec"
            "cron.add" => "service.cron.add"
            "cron.remove" => "service.cron.remove"
        let ctx = json!({"pid": pid, "cmd": cmd})
        match gate.check(agent_id, action, &ctx):
            Permit { .. } => proceed
            Deny { reason, .. } => return json!({"error": reason, "denied": true})
            Defer { reason } => return json!({"deferred": true, "reason": reason})

    // Execute command normally
    handle_message(pid, msg, ...)
```

---

## A -- Architecture

### Component Changes

```
daemon.rs
  |
  +-- Cron tick loop (existing, 1s interval)
  +-- Health monitor loop (NEW, configurable interval)
  +-- Agent watchdog loop (NEW, 5s interval)
  +-- Shutdown handler calls shutdown_all() (CHANGED from abort_all())

supervisor.rs
  |
  +-- spawn_and_run() (existing)
  +-- watchdog_sweep() (NEW)
  +-- shutdown_all(timeout) (NEW, replaces abort_all() for normal shutdown)
  +-- abort_all() (existing, kept for emergency/force shutdown)

agent_loop.rs
  |
  +-- kernel_agent_loop() (MODIFIED)
  |     +-- Resource usage tracking (NEW)
  |     +-- Suspend/resume handling (NEW)
  |     +-- Gate-checked commands (NEW)
  |
  +-- New parameters: process_table, gate (Arc<dyn GateBackend>)
```

### Data Flow

```
Health Monitor Loop:
  timer tick -> health.aggregate() -> (OverallHealth, results)
    -> chain event "health.check"
    -> event_log entries for degraded/unhealthy

Agent Watchdog:
  timer tick -> supervisor.watchdog_sweep()
    -> check is_finished() on each JoinHandle
    -> transition stale PIDs to Exited(-2/-3/-4)
    -> chain event "agent.watchdog_reap"

Graceful Shutdown:
  Ctrl+C -> shutdown_all(5s timeout)
    -> cancel all tokens -> agents detect, exit cleanly
    -> wait for handles with timeout
    -> chain events for each agent exit
    -> fallback abort for stragglers

Resource Tracking:
  message recv -> handle_message() -> increment counters
    -> every 10 messages: process_table.update_resources()
    -> on exit: final update

Gate Enforcement:
  message recv -> extract cmd -> gate.check(agent, action, ctx)
    -> Permit: execute normally
    -> Deny: return error response
    -> Defer: return deferred response
```

---

## R -- Refinement

### Edge Cases

1. **Health check during shutdown**: Health loop exits on shutdown signal before starting a new check
2. **Watchdog vs normal exit**: `watchdog_sweep()` checks process table state; if already `Exited`, skips (normal cleanup ran)
3. **Shutdown timeout too short**: 5s default; configurable. Agents that don't exit in time are aborted with chain warning
4. **Resource counter overflow**: u64 counters; overflow at ~18 exabytes or ~18 quintillion messages — not a practical concern
5. **Gate check on kernel PID 0**: Kernel process is always permitted; skip gate check when `pid == 0`
6. **Suspend during message processing**: Suspend takes effect after current message completes (not mid-processing)
7. **Concurrent shutdown_all + watchdog_sweep**: shutdown_all drains running_agents; watchdog_sweep finds empty map — idempotent

### Breaking Changes

None. All changes are additive:
- `kernel_agent_loop` gains new parameters (process_table, gate) but existing tests pass with defaults
- `shutdown_all` is a new method; `abort_all` remains for backward compatibility
- Resource counters go from zero to nonzero — no API changes

---

## C -- Completion

### K2b Hardening Gate

- [x] **Gap 1**: Health monitor loop runs in daemon, checks fire on interval, chain events logged
- [x] **Gap 2**: Watchdog sweep detects finished/panicked agent tasks, transitions to Exited
- [x] **Gap 3**: `shutdown_all(timeout)` gives agents grace period, cleanup handlers run
- [x] **Gap 4**: Resource counters (messages_sent, tool_calls, cpu_time_ms) increment correctly
- [x] **Gap 5**: Agent suspend/resume via IPC command, process state transitions
- [x] **Gap 6**: `exec` and `cron.add`/`cron.remove` gate-checked before execution
- [x] All workspace tests pass (`scripts/build.sh test`)
- [x] Clippy clean for both default and exochain features (`scripts/build.sh clippy`)
- [x] 363 kernel tests pass (7 new tests, up from 356)

### Tests Added

| File | Tests | Count |
|------|-------|-------|
| `agent_loop.rs` | resource tracking increments, suspend/resume cycle, gate deny blocks exec, gate permit allows exec | 4 |
| `supervisor.rs` | watchdog_sweep reaps finished task, shutdown_all graceful, shutdown_all timeout aborts | 3 |
| `health.rs` | (existing tests sufficient, health loop tested via daemon integration) | 0 |
| `daemon.rs` | (integration tested via manual playbook) | 0 |

**7 new tests**, 363 total kernel tests with exochain feature.

### Actual File Changes (commit `1de2fdc`)

| File | Action | Lines |
|------|--------|-------|
| `crates/clawft-kernel/src/supervisor.rs` | Add `watchdog_sweep()` + `shutdown_all()` | +266 |
| `crates/clawft-kernel/src/agent_loop.rs` | Add resource tracking, suspend/resume, gate checks | +653/-114 |
| `crates/clawft-weave/src/daemon.rs` | Add health monitor loop, watchdog loop, use shutdown_all | +303 |
| `crates/clawft-kernel/src/a2a.rs` | Allow Suspended agents to send IPC replies | +158 |
| `crates/clawft-kernel/src/service.rs` | Add `snapshot()` for Send-safe health iteration | +12 |
| `crates/clawft-kernel/Cargo.toml` | Add `futures` dependency | +7 |
| **Total** | | **+1285/-114** |
