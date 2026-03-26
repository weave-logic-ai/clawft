# 08a Self-Healing & Process Management

**Document ID**: 08a
**Workstream**: W-KERNEL
**Duration**: Weeks 17-20
**Goal**: Complete K1 process supervision and K2b hardening gaps -- restart strategies, crash notification, resource enforcement, reconciliation, and health probes
**Depends on**: K0-K6 (all prior phases)
**Orchestrator**: `08-orchestrator.md`
**Priority**: P0 (Critical) -- must complete K1-G1 before 08b DLQ work starts

---

## S -- Specification

### What Changes

K1 delivered the `AgentSupervisor`, `ProcessTable`, and `AgentCapabilities`.
K2b hardened IPC with backpressure and circuit breakers. Both phases left
certain production-essential patterns for later:

- Supervisor restart strategies (Erlang-inspired one-for-one/all/rest)
- Process links and monitors (bidirectional crash notification)
- Continuous resource limit enforcement (not just spawn-time checks)
- Per-agent disk quotas
- Reconciliation controller (desired vs actual state)
- Liveness and readiness probes

This phase fills all six gaps, making WeftOS self-healing at the single-node
level.

### Gap Summary

| Gap ID | Area | New Lines | Changed Lines | Priority |
|--------|------|:---------:|:------------:|----------|
| K1-G1 | Supervisor restart strategies | ~250 | ~100 | P0 |
| K1-G2 | Process links and monitors | ~250 | ~50 | P0 |
| K1-G3 | Continuous resource enforcement | ~250 | ~50 | P0 |
| K1-G4 | Per-agent disk quotas | ~100 | ~20 | P1 |
| K2b-G1 | Reconciliation controller | ~300 | ~30 | P1 |
| K2b-G2 | Liveness + readiness probes | ~200 | ~50 | P1 |
| **Total** | | **~1,350** | **~300** | |

### Feature Gate

All code gated behind `#[cfg(feature = "os-patterns")]`.

```toml
[features]
os-patterns = ["exochain"]
```

---

### K1-G1: Supervisor Restart Strategies (~250 lines)

**Source**: Erlang/BEAM supervisor model (R5)

**Files to modify**: `crates/clawft-kernel/src/supervisor.rs`

**Types**:
```rust
/// Supervisor restart strategy (Erlang-inspired).
///
/// Determines what happens to sibling agents when one agent fails.
/// Configured per AppManifest or per supervisor instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestartStrategy {
    /// Restart only the failed child.
    OneForOne,
    /// Restart all children if one fails.
    OneForAll,
    /// Restart the failed child and all children started after it.
    RestForOne,
}

/// Restart budget: max N restarts within M seconds.
///
/// When the budget is exceeded, the supervisor escalates (stops itself
/// and notifies its parent). Prevents infinite restart loops.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartBudget {
    pub max_restarts: u32,
    pub within_secs: u64,
}

/// Restart state tracking per supervised agent.
pub struct RestartTracker {
    /// Number of restarts in the current window.
    pub restart_count: u32,
    /// When the current budget window started.
    pub window_start: Instant,
    /// When the last restart occurred.
    pub last_restart: Option<Instant>,
    /// Current backoff delay (exponential: 100ms -> 30s max).
    pub backoff_ms: u64,
}
```

**Integration with existing types**:
- `AgentSupervisor<P: Platform>` gains a `restart_strategy: RestartStrategy` field
- `AgentSupervisor.running_agents: Arc<DashMap<Pid, JoinHandle<()>>>` -- the
  existing `JoinHandle` is already tracked; the watchdog task `.await`s it
- `SpawnRequest` gains `restart_budget: Option<RestartBudget>`
- `AgentRestartPolicy` from `agency.rs` (`Never`, `OnFailure`, `Always`) controls
  per-agent policy; `RestartStrategy` controls cross-agent behavior

**Behavior**:
- Background watchdog task spawned per child, awaiting `JoinHandle`
- On failure: check `AgentRestartPolicy` -- if `Never`, skip restart
- Apply `RestartStrategy`:
  - `OneForOne`: restart only the failed agent
  - `OneForAll`: stop all siblings, then restart all
  - `RestForOne`: stop all agents spawned after the failed one, then restart those
- Exponential backoff: 100ms, 200ms, 400ms, 800ms, ... max 30s
- If `RestartBudget` exceeded: escalate -- stop supervisor, emit chain event
  `supervisor.escalation` with agent_id, reason, total_restarts
- Chain event on each restart: `supervisor.restart` with agent_id, reason,
  attempt number, backoff_ms

**Tests**:
- Agent crash triggers automatic restart within 1s
- `OneForAll` restarts all siblings when one crashes
- `RestForOne` restarts only later siblings
- `RestartBudget` exceeded triggers escalation (supervisor stops)
- Exponential backoff delays increase correctly (100ms -> 200ms -> 400ms)
- Backoff caps at 30s maximum
- `AgentRestartPolicy::Never` prevents restart
- Restart resets agent `ProcessState` to `Starting`
- Restart preserves original `SpawnRequest` configuration
- Chain events recorded for each restart and escalation

---

### K1-G2: Process Links and Monitors (~250 lines)

**Source**: Erlang links/monitors (R6)

**Why K1**: Links and monitors are process-level primitives. They extend
`ProcessEntry` in `process.rs` (K1 module) and deliver signals via
`KernelSignal` in `ipc.rs`.

**Files**: New `crates/clawft-kernel/src/monitor.rs`, modify `process.rs`, `supervisor.rs`

**Types**:
```rust
/// Bidirectional crash notification link between two processes.
pub struct ProcessLink {
    pub pid_a: Pid,
    pub pid_b: Pid,
}

/// Unidirectional process monitor.
pub struct ProcessMonitor {
    pub watcher: Pid,
    pub target: Pid,
    pub ref_id: String,
}

/// Notification sent when a monitored process exits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDown {
    pub pid: Pid,
    pub reason: ExitReason,
    pub ref_id: String,
}

/// Why a process exited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitReason {
    /// Normal completion (exit code 0).
    Normal,
    /// Crashed with error message.
    Crash(String),
    /// Killed by supervisor or operator.
    Killed,
    /// Timed out (resource limit).
    Timeout,
}
```

**Integration with existing types**:
- `ProcessEntry` in `process.rs` gains:
  - `links: Vec<Pid>` -- PIDs this process is linked to
  - `monitors: Vec<ProcessMonitor>` -- monitors watching this process
- `KernelSignal` in `ipc.rs` gains:
  - `LinkExit { pid: Pid, reason: ExitReason }` -- delivered to linked process
  - `MonitorDown(ProcessDown)` -- delivered to monitoring process

**Behavior**:
- `link(pid_a, pid_b)` -- bidirectional crash notification
- `monitor(watcher, target)` -- unidirectional DOWN message; returns `ref_id`
- `unlink(pid_a, pid_b)` -- removes link
- `demonitor(ref_id)` -- removes monitor
- When process transitions to `ProcessState::Exited(_)`:
  - Deliver `LinkExit` to all linked PIDs
  - Deliver `ProcessDown` to all monitoring PIDs
- Extend across mesh: remote monitors fire when `HeartbeatTracker` detects
  node death (all processes on that node emit `ProcessDown`)

**Tests**:
- Link delivers crash signal to partner
- Monitor delivers `ProcessDown` to watcher
- Unlink removes notification
- Normal exit delivers `ExitReason::Normal`
- Remote monitor via mesh fires on node death
- Multiple monitors on same target all fire
- Link to already-exited process delivers immediate signal

---

### K1-G3: Continuous Resource Limit Enforcement (~250 lines)

**Why K1**: `ResourceLimits` and `AgentCapabilities` are K1 types
(`capability.rs`). Currently limits are checked at spawn time but not
continuously enforced.

**Files to modify**: `crates/clawft-kernel/src/agent_loop.rs`, `crates/clawft-kernel/src/supervisor.rs`

**Integration with existing types**:
- `ResourceLimits` in `capability.rs` already defines:
  - `max_memory_bytes: u64` (default 256 MiB)
  - `max_cpu_time_ms: u64` (default 300,000 = 5 minutes)
  - `max_tool_calls: u64` (default 1,000)
  - `max_messages: u64` (default 5,000)
- `ResourceUsage` in `process.rs` tracks current usage

**Behavior**:
- Background task per agent samples resource usage every 1s
- Memory: query `ResourceUsage.memory_bytes` (already tracked)
- CPU time: track cumulative `Instant` elapsed across work periods
- Message count: `A2ARouter` increments on send
- At 80% of any limit: emit chain event `resource.limit.warning`
- At 100%: send `KernelSignal::Shutdown` (graceful), wait 5s grace period,
  then force cancel via `CancellationToken`
- Chain event: `resource.limit.exceeded` with limit type and value
- Resource usage resets on restart (new `ProcessEntry`)

**Tests**:
- Message count limit triggers stop at threshold
- CPU time limit triggers stop
- Warning emitted at 80%
- Grace period allows graceful shutdown
- Force cancel after grace period
- Resource usage resets after restart
- Unlimited resources (0 = unlimited) not enforced

---

### K1-G4: Per-Agent Disk Quotas (~100 lines)

**Why K1**: Extends `ResourceLimits` in `capability.rs` (K1).

**Files to modify**: `crates/clawft-kernel/src/capability.rs`

**Changes**:
- Add `max_disk_bytes: u64` to `ResourceLimits` (default: 100 MiB)
- Tree manager checks quota before `insert()` / `update()`
- Quota tracked by summing node payload sizes under `/agents/{agent_id}/`

**Tests**:
- Quota exceeded prevents tree writes (`KernelError::QuotaExceeded`)
- Quota tracking accurate across multiple writes
- Deleting tree nodes frees quota

---

### K2b-G1: Reconciliation Controller (~300 lines)

**Source**: Kubernetes controllers (R12)

**Why K2b**: Reconciliation is a hardening pattern -- it makes the system
self-correcting. It watches `ProcessTable` (K1) and `AppManager` (K5) to
detect drift between desired and actual state.

**Files**: New `crates/clawft-kernel/src/reconciler.rs`

**Types**:
```rust
/// Reconciliation controller: desired state vs actual state.
pub struct ReconciliationController {
    interval: Duration,
    desired: DashMap<String, DesiredAgentState>,
    drifts: Arc<RwLock<Vec<DriftEvent>>>,
    process_table: Arc<ProcessTable>,
    app_manager: Arc<AppManager>,
    supervisor: Arc<AgentSupervisor<NativePlatform>>,
}

pub struct DesiredAgentState {
    pub app_id: String,
    pub agent_id: String,
    pub agent_type: String,
    pub replicas: u32,
    pub capabilities: AgentCapabilities,
}

pub enum DriftEvent {
    AgentMissing { agent_id: String, app_id: String },
    ExtraAgent { pid: Pid, agent_id: String },
    WrongState { pid: Pid, expected: ProcessState, actual: ProcessState },
    CapabilityMismatch { pid: Pid, agent_id: String },
}
```

**Behavior**:
- Registered as `SystemService` with `ServiceType::Core`
- Background task ticks every 5s (configurable)
- Compares `ProcessTable` against `AppManifest` desired state
- Spawns missing agents, stops extra agents, logs state mismatches
- Governance-gated: checks `GovernanceGate` before corrective action
- Chain event: `reconciler.drift` with drift details
- Recent drifts stored (last 100) for diagnostics

**Tests**:
- Dead agent detected and respawned within one tick
- Extra agent detected and stopped
- Multiple drifts resolved in one tick
- Governance gate can block corrective action
- Registered as `SystemService`
- Respects `AgentRestartPolicy::Never`

---

### K2b-G2: Liveness + Readiness Probes (~200 lines)

**Source**: Kubernetes probes (R13)

**Why K2b**: Probes are a hardening mechanism for service reliability.
They extend `SystemService` (K1/K2) and `HealthSystem` (K1).

**Files to modify**: `crates/clawft-kernel/src/health.rs`, `crates/clawft-kernel/src/service.rs`

**Types**:
```rust
pub enum ProbeResult {
    Live,
    NotLive { reason: String },
    Ready,
    NotReady { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    pub liveness_interval: Duration,
    pub readiness_interval: Duration,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}
```

**Integration**:
- `SystemService` trait gains optional `liveness_check()` / `readiness_check()`
- Default implementations return `Live` / `Ready` (backward compatible)
- `HealthSystem` runs probe background tasks per service

**Behavior**:
- Failed liveness (>= threshold): trigger restart via `AgentSupervisor`
- Failed readiness (>= threshold): remove from `ServiceRegistry` endpoints
- Recovery (>= success threshold): re-add to `ServiceRegistry`
- Chain events: `probe.liveness.failed`, `probe.readiness.failed`, `probe.readiness.recovered`

**Tests**:
- Failed liveness triggers restart
- Failed readiness removes from registry
- Recovery re-adds to registry
- Threshold prevents single-failure flapping
- Default probe methods return `Live` / `Ready`

---

## P -- Pseudocode

### 1. Supervisor Restart with Backoff and Escalation (K1-G1)

```
fn supervisor_watchdog(
    child_pid: Pid,
    join_handle: JoinHandle<()>,
    strategy: RestartStrategy,
    budget: RestartBudget,
    tracker: &mut RestartTracker,
    siblings: &DashMap<Pid, JoinHandle<()>>,
    spawn_request: SpawnRequest,
):
    // Await child completion
    result = join_handle.await

    match result:
        Ok(()) ->
            // Normal exit -- no restart needed
            process_table.set_state(child_pid, Exited(0))
            deliver_exit_signals(child_pid, ExitReason::Normal)
            return

        Err(join_error) ->
            reason = format!("{}", join_error)
            process_table.set_state(child_pid, Exited(1))
            deliver_exit_signals(child_pid, ExitReason::Crash(reason.clone()))
            metrics.increment("kernel.agent_crashes")

            // Check per-agent restart policy
            agent_policy = get_restart_policy(child_pid)
            if agent_policy == AgentRestartPolicy::Never:
                chain.append("supervisor", "restart.skipped", { agent_id, reason })
                return

            // Check budget window -- reset if window expired
            now = Instant::now()
            if now.duration_since(tracker.window_start).as_secs() > budget.within_secs:
                tracker.restart_count = 0
                tracker.window_start = now

            tracker.restart_count += 1
            if tracker.restart_count > budget.max_restarts:
                // ESCALATE: budget exceeded
                chain.append("supervisor", "supervisor.escalation", {
                    agent_id: spawn_request.agent_id,
                    reason: "restart budget exceeded",
                    total_restarts: tracker.restart_count,
                })
                stop_supervisor()
                return

            // Apply restart strategy
            match strategy:
                OneForOne:
                    apply_backoff_and_restart(child_pid, spawn_request, tracker)

                OneForAll:
                    for (sibling_pid, sibling_handle) in siblings.iter():
                        if sibling_pid != child_pid:
                            cancel(sibling_pid)
                            sibling_handle.await
                    for stored_request in all_spawn_requests():
                        apply_backoff_and_restart(stored_request.pid, stored_request, tracker)

                RestForOne:
                    later_siblings = siblings_spawned_after(child_pid)
                    for (sib_pid, sib_handle) in later_siblings.reverse():
                        cancel(sib_pid)
                        sib_handle.await
                    apply_backoff_and_restart(child_pid, spawn_request, tracker)
                    for sib_request in later_siblings:
                        apply_backoff_and_restart(sib_request.pid, sib_request, tracker)

fn apply_backoff_and_restart(pid, request, tracker):
    delay = min(100 * 2^(tracker.restart_count - 1), 30_000)
    tracker.backoff_ms = delay
    tracker.last_restart = Some(Instant::now())

    sleep(Duration::from_millis(delay))

    chain.append("supervisor", "supervisor.restart", {
        agent_id: request.agent_id,
        attempt: tracker.restart_count,
        backoff_ms: delay,
    })

    new_handle = supervisor.spawn(request.clone())
    siblings.insert(new_pid, new_handle)
    spawn(supervisor_watchdog(new_pid, new_handle, strategy, budget, tracker, siblings, request))
```

### 2. Reconciliation Controller Tick (K2b-G1)

```
fn reconciliation_tick(controller: &ReconciliationController):
    drifts = []

    // Build desired state from installed AppManifests
    for app in app_manager.list_installed():
        for agent_spec in app.manifest.agents:
            desired_key = format!("{}/{}", app.name, agent_spec.id)
            controller.desired.insert(desired_key, DesiredAgentState {
                app_id: app.name,
                agent_id: agent_spec.id,
                agent_type: agent_spec.agent_type,
                replicas: agent_spec.replicas.unwrap_or(1),
                capabilities: agent_spec.capabilities.clone(),
            })

    // Check desired agents exist
    for (key, desired) in controller.desired.iter():
        matching = process_table.find_by_agent_id(&desired.agent_id)
        running_count = matching.filter(|p| p.state == Running).count()

        if running_count < desired.replicas:
            for _ in 0..(desired.replicas - running_count):
                drifts.push(DriftEvent::AgentMissing {
                    agent_id: desired.agent_id.clone(),
                    app_id: desired.app_id.clone(),
                })

    // Check for extra agents
    for entry in process_table.list_all():
        if entry.state != Running: continue
        if entry.agent_id.starts_with("kernel.") or entry.agent_id.starts_with("system."):
            continue
        if !controller.desired.values().any(|d| d.agent_id == entry.agent_id):
            drifts.push(DriftEvent::ExtraAgent { pid: entry.pid, agent_id: entry.agent_id })

    // Governance-gated corrective action
    for drift in &drifts:
        match drift:
            AgentMissing { agent_id, app_id }:
                if governance.evaluate("reconciler.spawn", { agent_id }) == Permit:
                    supervisor.spawn(SpawnRequest { agent_id, .. })
                    chain.append("reconciler", "reconciler.spawn", { agent_id, app_id })

            ExtraAgent { pid, agent_id }:
                if governance.evaluate("reconciler.stop", { pid }) == Permit:
                    supervisor.stop(*pid)
                    chain.append("reconciler", "reconciler.stop", { pid, agent_id })

    // Store drift history (bounded)
    controller.drifts.write().extend(drifts)
    while controller.drifts.read().len() > 100:
        controller.drifts.write().remove(0)
```

### 3. Resource Limit Enforcement Loop (K1-G3)

```
fn resource_enforcement_loop(pid, limits, process_table, cancel_token):
    let warned = HashSet::new()

    loop:
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {}
            _ = cancel_token.cancelled() => return
        }

        let usage = match process_table.get_resource_usage(pid):
            Some(u) => u
            None => return  // process gone

        let checks = [
            ("memory", usage.memory_bytes, limits.max_memory_bytes),
            ("cpu_time", usage.cpu_time_ms, limits.max_cpu_time_ms),
            ("messages", usage.messages_sent, limits.max_messages),
            ("tool_calls", usage.tool_calls, limits.max_tool_calls),
        ]

        for (name, current, limit) in checks:
            if limit == 0: continue  // unlimited
            let ratio = current as f64 / limit as f64

            if ratio >= 0.8 and ratio < 1.0 and !warned.contains(name):
                warned.insert(name)
                chain.append("resource", "resource.limit.warning", { pid, name, current, limit })
                a2a.send(KernelMessage::new(0, Process(pid),
                    Signal(ResourceWarning { resource: name, current, limit })))

            if ratio >= 1.0:
                chain.append("resource", "resource.limit.exceeded", { pid, name, current, limit })
                a2a.send(KernelMessage::new(0, Process(pid), Signal(Shutdown)))

                tokio::select! {
                    _ = sleep(Duration::from_secs(5)) => {
                        cancel_token.cancel()
                        process_table.set_state(pid, Exited(137))
                    }
                    _ = wait_for_exit(pid) => {}  // graceful
                }
                return
```

---

## A -- Architecture

### Component Integration Diagram

```
+------------------------------------------------------------------+
|                      EXISTING KERNEL (K0-K6)                      |
|                                                                   |
|  +-----------+  +----------+  +----------+  +-------------------+ |
|  | Process   |  | A2A      |  | Service  |  | AgentSupervisor   | |
|  | Table     |  | Router   |  | Registry |  | (supervisor.rs)   | |
|  | (K1)      |  | (K2)     |  | (K1)     |  | (K1)              | |
|  +-----------+  +----------+  +----------+  +-------------------+ |
|       ^              |             ^              ^               |
+-------|--------------|-------------|--------------|---------------+
        |              |             |              |
+-------|--------------|-------------|--------------|---------------+
| 08a   |              |        K2b-G2              |               |
| FILLS |              |  +-----------+         K1-G1               |
|  +----+--------+     |  | Probes    |    +--------+------------+ |
|  | Reconciler  |     |  | (liveness |    | Restart Strategies   | |
|  | Controller  |     |  |  readiness|    | + Watchdog           | |
|  | (K2b-G1)    |     |  | )         |    | (K1-G1)              | |
|  +------+------+     |  +-----------+    +--------+------------ + |
|         |            |                            |               |
|         v            |                            v               |
|  +------+------+     |                   +--------+------------ + |
|  | AppManager  |     |                   | Process Links        | |
|  | (K5, for    |     |                   | & Monitors           | |
|  |  desired    |     |                   | (K1-G2)              | |
|  |  state)     |     |                   +---------------------+  |
|  +-------------+     |                                            |
|                      |  +---------------------+                   |
|                      |  | Resource Enforcement |                  |
|                      |  | Loop (K1-G3)         |                  |
|                      |  | + Disk Quotas (K1-G4)|                  |
|                      |  +---------------------+                   |
+------------------------------------------------------------------+
```

### Component Integration Map

| Gap ID | New Component | Files | Depends On | Registered As |
|--------|--------------|-------|-----------|---------------|
| K1-G1 | `RestartStrategy` + watchdog | `supervisor.rs` | `AgentSupervisor`, `ProcessTable` | Part of `AgentSupervisor` |
| K1-G2 | `ProcessLink`, `ProcessMonitor` | new `monitor.rs`, `process.rs` | `ProcessTable`, `A2ARouter` | Part of `ProcessTable` |
| K1-G3 | Resource enforcement loop | `agent_loop.rs`, `supervisor.rs` | `ResourceLimits`, `ProcessTable` | Per-agent background task |
| K1-G4 | `max_disk_bytes` | `capability.rs` | `ResourceLimits`, `TreeManager` | Part of `ResourceLimits` |
| K2b-G1 | `ReconciliationController` | new `reconciler.rs` | `ProcessTable`, `AppManager`, `GovernanceEngine` | `SystemService::Core` at `/kernel/services/reconciler` |
| K2b-G2 | Liveness/readiness probes | `health.rs`, `service.rs` | `SystemService`, `HealthSystem` | Per-service probe task |

### Boot Sequence (08a additions)

```
Existing boot (K0-K6):
  1-12. [unchanged]

08a additions (when os-patterns enabled):
  18. ReconciliationController (K2b-G1, needs ProcessTable + AppManager)
  23. Restart watchdogs      (K1-G1, attached per spawned agent)
  24. Probe runners          (K2b-G2, attached per registered service)
  25. Resource enforcers     (K1-G3, attached per spawned agent)
```

Note: boot positions 13-17 and 19-22 are reserved for 08b and 08c services.

---

## R -- Refinement

### Performance Considerations

- Resource sampling: 1 check/second per agent -- for 100 agents, 100
  lightweight checks/second (no allocations on hot path)
- `ReconciliationController` interval configurable (default 5s) -- each tick
  iterates `ProcessTable` + `AppManifest`, both O(n) with small n
- Restart watchdog: one `tokio::spawn` per child, awaiting `JoinHandle` --
  zero CPU cost while child is running
- Probe runners: similar to resource enforcement, one lightweight task per
  service with configurable interval
- `DashMap` used for `links` and `monitors` to avoid lock contention during
  concurrent process exits

### Security Considerations

- Resource limit enforcement prevents runaway agent DoS
- Reconciliation controller is governance-gated: `GovernanceEngine` can block
  corrective actions (e.g., during maintenance windows)
- Process links/monitors respect IPC scope -- restricted agents cannot monitor
  kernel processes
- Disk quota enforcement prevents storage exhaustion by rogue agents
- All corrective actions (restart, reconcile, probe-triggered restart) logged
  to ExoChain for tamper-evident audit

### Migration Path

- `AgentRestartPolicy` (`Never`/`OnFailure`/`Always`) preserved;
  `RestartStrategy` adds cross-agent behavior on top
- `HealthStatus` enum unchanged; `ProbeResult` is separate
- `ResourceLimits` gains `max_disk_bytes` with default function
- `KernelSignal` gains new variants (`LinkExit`, `MonitorDown`,
  `ResourceWarning`) -- exhaustive matches need updating (compile-time
  enforcement, not runtime surprise)

### Testing Strategy

- Each gap is independently testable
- Integration test flows:
  - Agent crash -> K1-G1 restart -> K2b-G1 reconciler -> ExoChain event
  - K1-G3 resource exceeded -> warning -> shutdown -> force cancel
  - K2b-G2 readiness failure -> remove from registry -> recovery -> re-add
- `ReconciliationController` tested via `CancellationToken::cancel()`
- All tests independent of `mesh` feature (single-node)
- K1-G2 remote monitors tested with `InMemoryTransport` from K6

### Dependency Minimization

- No new external dependencies -- all code uses workspace-existing crates:
  `dashmap`, `tokio`, `chrono`, `serde`, `uuid`
- `os-patterns` feature does not pull networking dependencies

---

## C -- Completion

### Exit Criteria

#### K1 Process & Supervision Gate (P0)
- [ ] Agent crash triggers automatic restart within 1 second
- [ ] `OneForOne` strategy restarts only the failed agent
- [ ] `OneForAll` strategy restarts all sibling agents
- [ ] `RestForOne` strategy restarts the failed agent and later siblings
- [ ] Restart budget exceeded triggers supervisor escalation
- [ ] Exponential backoff prevents restart storms (100ms -> 30s max)
- [ ] `AgentRestartPolicy::Never` prevents restart
- [ ] Restart resets agent `ProcessState` to `Starting`
- [ ] Restart preserves original `SpawnRequest` configuration
- [ ] Process links deliver crash signals bidirectionally
- [ ] Process monitors deliver `ProcessDown` unidirectionally
- [ ] Unlink removes crash notification
- [ ] Remote monitors fire on node death (via mesh heartbeat)
- [ ] Resource limits enforced continuously (not just at spawn)
- [ ] CPU time limit triggers agent stop
- [ ] Message count limit enforced per agent
- [ ] Warning emitted at 80% of limit threshold
- [ ] Grace period before force cancel (5 seconds)
- [ ] Per-agent disk quota tracked and enforced
- [ ] All self-healing actions logged to ExoChain

#### K2b Hardening Gate (P1)
- [ ] Reconciliation controller detects dead agents within 5 seconds
- [ ] Reconciliation controller spawns replacements automatically
- [ ] Reconciliation controller detects extra agents and stops them
- [ ] Reconciliation controller respects governance gate
- [ ] Reconciliation controller registered as `SystemService`
- [ ] Liveness probe failure triggers restart
- [ ] Readiness probe failure removes from `ServiceRegistry`
- [ ] Readiness recovery re-adds to `ServiceRegistry`
- [ ] Probe failure threshold prevents flapping
- [ ] Default probe methods return `Live` / `Ready` (backward compatible)

#### Phase Gate
- [ ] All K1 exit criteria pass
- [ ] All K2b exit criteria pass
- [ ] All existing tests pass (843+ baseline)
- [ ] New tests: target 60+ for this phase
- [ ] Clippy clean for all new code (`scripts/build.sh clippy`)
- [ ] Feature gated behind `os-patterns`
- [ ] No new external dependencies

### Testing Verification Commands

```bash
# Build with OS patterns feature
scripts/build.sh native --features os-patterns

# Run self-healing tests
scripts/build.sh test -- --features os-patterns -p clawft-kernel

# Verify base build unchanged
scripts/build.sh check

# Clippy
scripts/build.sh clippy
```

### Implementation Order

```
Week 17:
  K1-G1: Restart strategies (foundation for all self-healing)
  K1-G2: Process links/monitors (crash notification)

Week 18:
  K1-G3: Continuous resource enforcement
  K2b-G1: Reconciliation controller

Week 19:
  K1-G4: Per-agent disk quotas
  K2b-G2: Liveness + readiness probes

Week 20:
  Integration testing across all 08a gaps
  Review and polish
```
