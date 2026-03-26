# OS Gap-Filling: K0-K6 Pattern Completion

**Document ID**: 08
**Workstream**: W-KERNEL
**Duration**: Weeks 17-22 (parallel across K-levels)
**Goal**: Complete all missing core OS patterns across K1-K6 to make WeftOS production-ready: self-healing, observability, reliable IPC, resource enforcement, and operational services
**Depends on**: K0-K6 (all prior phases)
**Source**: OS Gap Analysis (2026-03-26) + Microkernel Comparative Analysis

---

## S -- Specification

### What Changes

K0-K6 delivered process management, IPC, WASM sandboxing, containers,
applications, ExoChain, governance, and mesh networking. Each phase shipped
the primary feature but left certain OS-level patterns for later. This
document maps every gap back to the K-level where it naturally belongs,
then fills them all in a coordinated effort.

This is not a new "K7 phase" -- it is a gap-filling exercise that
strengthens each existing K-level. The work is organized by K-level so
that each change is reviewed in context of the module it extends.

### Gap Summary by K-Level

| K-Level | Gap Area | New Lines | Changed Lines | Priority |
|---------|----------|:---------:|:------------:|----------|
| K1 | Process & Supervision | ~700 | ~200 | P0 (Critical) |
| K2 | IPC & Communication | ~1,000 | ~200 | P1 (High) |
| K2b | Hardening | ~500 | ~150 | P1 (High) |
| K3 | WASM Sandbox | ~400 | ~50 | P2 (Important) |
| K5 | App Framework | ~950 | ~100 | P2 (Important) |
| K6 | Mesh Networking | ~300 | ~100 | P2 (Important) |
| Cross-cutting | Observability & Timers | ~1,250 | ~150 | P1 (High) |
| **Total** | | **~5,100** | **~950** | |

### Feature Gate Structure

```toml
[features]
os-patterns = ["exochain"]       # Self-healing, probes, reliable IPC, timers, config
os-full = ["os-patterns", "blake3"]  # Adds content-addressed artifact store
```

All gap-filling code is gated behind `#[cfg(feature = "os-patterns")]` to
preserve the existing kernel build. The `blake3` dependency is only required
for the artifact store (K3 gap).

---

## K1 Gaps: Process & Supervision

**Why K1**: The `AgentSupervisor` (`supervisor.rs`), `ProcessTable`
(`process.rs`), and `AgentCapabilities` (`capability.rs`) were all
introduced in K1. These gaps extend that foundation with the restart
strategies, crash notification, and resource enforcement that a
production supervisor requires.

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

## K2 Gaps: IPC & Communication

**Why K2**: The `A2ARouter` (`a2a.rs`), `KernelIpc` (`ipc.rs`),
`KernelMessage`, `KernelSignal`, and `TopicRouter` (`topic.rs`) were all
introduced in K2. These gaps extend IPC with reliability, dead letter
handling, persistent channels, and tracing.

### K2-G1: Dead Letter Queue (~150 lines)

**Why K2**: Extends `A2ARouter::send()` behavior when delivery fails.
Currently undeliverable messages are silently dropped.

**Files**: New `crates/clawft-kernel/src/dead_letter.rs`

**Types**:
```rust
/// Queue for messages that could not be delivered.
pub struct DeadLetterQueue {
    letters: RwLock<VecDeque<DeadLetter>>,
    max_size: usize,
}

pub struct DeadLetter {
    pub message: KernelMessage,
    pub reason: DeadLetterReason,
    pub timestamp: DateTime<Utc>,
    pub retry_count: u32,
}

pub enum DeadLetterReason {
    /// Target PID not found in ProcessTable.
    TargetNotFound { pid: Pid },
    /// Target inbox channel is full (capacity: DEFAULT_INBOX_CAPACITY = 1024).
    InboxFull { pid: Pid },
    /// Delivery timed out.
    Timeout { duration_ms: u64 },
    /// GovernanceGate denied delivery.
    GovernanceDenied { reason: String },
    /// Target agent exited before delivery.
    AgentExited { pid: Pid },
}
```

**Behavior**:
- `A2ARouter::send()` routes to DLQ instead of dropping on delivery failure
- Configurable max size (default 10,000 entries)
- FIFO eviction when full
- Queryable: by target PID, by reason variant, by time range
- Optional retry: `retry(msg_id)` re-delivers via `A2ARouter::send()`
- Chain event: `ipc.dead_letter` with reason and message metadata
- Registered as `SystemService` at boot

**Tests**:
- Message to nonexistent PID goes to DLQ
- Full inbox sends message to DLQ
- DLQ evicts oldest at capacity
- Query by reason filters correctly
- Query by time range returns correct subset
- Retry redelivers message
- Retry increments `retry_count`
- Chain event emitted for each dead letter

### K2-G2: Reliable Message Delivery (~400 lines)

**Why K2**: Builds on `A2ARouter::request()` correlation_id pattern from K2.
Adds retry logic and ack tracking.

**Files**: New `crates/clawft-kernel/src/reliable_queue.rs`

**Types**:
```rust
/// Reliable message delivery with acknowledgment tracking.
pub struct ReliableQueue {
    pending: DashMap<String, PendingDelivery>,
    config: ReliableConfig,
    dead_letter: Arc<DeadLetterQueue>,
}

pub struct PendingDelivery {
    pub message: KernelMessage,
    pub attempts: u32,
    pub first_sent: Instant,
    pub last_attempt: Instant,
    pub ack_deadline: Instant,
}

pub struct ReliableConfig {
    pub max_retries: u32,          // default: 3
    pub initial_timeout: Duration, // default: 5s
    pub max_timeout: Duration,     // default: 30s
    pub backoff_multiplier: f64,   // default: 2.0
}

pub enum DeliveryResult {
    Acknowledged { msg_id: String, ack_time: Duration },
    MaxRetriesExceeded { msg_id: String, attempts: u32 },
    DeadLettered { msg_id: String, reason: String },
}
```

**Behavior**:
- `send_reliable(msg)` sends with ack tracking via `correlation_id`
- Background task monitors `ack_deadline` for each pending delivery
- On timeout: retry with `backoff_multiplier` applied
- After `max_retries`: route to `DeadLetterQueue`
- Chain event: `ipc.reliable.timeout` / `ipc.reliable.ack`

**Tests**:
- Message acked on first attempt
- Timeout triggers retry
- Max retries exceeded sends to DLQ
- Backoff increases per attempt
- Backoff caps at `max_timeout`
- Concurrent reliable sends tracked independently
- Late ack does not cause double-delivery

### K2-G3: Named Pipes (~250 lines)

**Why K2**: Persistent IPC channels are a natural extension of the
`A2ARouter` messaging model from K2.

**Files**: New `crates/clawft-kernel/src/named_pipe.rs`

**Types**:
```rust
/// Registry of named pipes for persistent IPC channels.
pub struct NamedPipeRegistry {
    pipes: DashMap<String, NamedPipe>,
}

pub struct NamedPipe {
    pub name: String,
    pub capacity: usize,
    pub created_at: DateTime<Utc>,
    pub sender: mpsc::Sender<KernelMessage>,
    pub receiver_count: AtomicU32,
    pub sender_count: AtomicU32,
    pub ttl_after_empty: Duration,
    pub last_active: RwLock<Instant>,
}
```

**Behavior**:
- Create, connect, receive named pipe operations
- Pipes survive agent restarts
- Tree registration at `/kernel/pipes/{name}`
- TTL cleanup removes unused pipes (default 60s after last reference)
- Capacity limits enforced
- Respects `IpcScope` capabilities

**Tests**:
- Create + connect + send + receive roundtrip
- Pipe survives agent restart
- Multiple senders to one pipe (fan-in)
- Capacity limit returns error when full
- TTL cleanup removes unused pipes
- Access denied for `IpcScope::Restricted` agents

### K2-G4: Distributed Trace IDs (~100 lines)

**Why K2**: Trace IDs propagate through `KernelMessage` (K2 type) via
`A2ARouter` (K2 module).

**Files to modify**: `crates/clawft-kernel/src/ipc.rs`, `crates/clawft-kernel/src/a2a.rs`

**Changes**:
- Add `trace_id: Option<String>` to `KernelMessage` (with `#[serde(default)]`)
- External messages entering the kernel get new UUID v4 trace_id
- Internal messages inherit parent trace_id via correlation linkage
- `LogEntry.trace_id` carries the trace for correlated log queries

**Tests**:
- Trace ID propagated through IPC chain
- External messages get new trace_id
- Internal messages inherit parent trace_id
- trace_id survives serialization roundtrip

### K2-G5: Expanded KernelSignal Vocabulary (~50 lines)

**Why K2**: `KernelSignal` is defined in `ipc.rs` (K2). Currently has
`Shutdown`, `Suspend`, `Resume`, `Ping`, `Pong`. Gap-filling adds signals
needed by other K-level gaps.

**Files to modify**: `crates/clawft-kernel/src/ipc.rs`

**New variants**:
```rust
pub enum KernelSignal {
    // ... existing variants ...
    /// Crash notification from a linked process (K1-G2).
    LinkExit { pid: Pid, reason: ExitReason },
    /// Monitor DOWN notification (K1-G2).
    MonitorDown(ProcessDown),
    /// Resource usage warning at 80% of limit (K1-G3).
    ResourceWarning { resource: String, current: u64, limit: u64 },
}
```

**Tests**:
- New signal variants serialize/deserialize correctly
- Signal delivery through existing `A2ARouter` pipeline

---

## K2b Gaps: Hardening

**Why K2b**: The K2 hardening phase (`03b-phase-K2-hardening.md`) focused
on robustness. These gaps continue that hardening with reconciliation and
health probes.

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

## K3 Gaps: WASM Sandbox

**Why K3**: The `WasmToolRunner` (`wasm_runner.rs`) and WASM module loading
were introduced in K3. Content-addressed storage for WASM modules ensures
tamper-proof, deduplicated module management.

### K3-G1: Content-Addressed Artifact Store (~400 lines)

**Source**: IPFS CAS (R16)

**Files**: New `crates/clawft-kernel/src/artifact_store.rs`

**Types**:
```rust
/// Content-addressed artifact store using BLAKE3 hashes.
pub struct ArtifactStore {
    artifacts: DashMap<String, StoredArtifact>,
    backend: ArtifactBackend,
    total_size: AtomicU64,
}

pub struct StoredArtifact {
    pub hash: String,
    pub size: u64,
    pub content_type: ArtifactType,
    pub stored_at: DateTime<Utc>,
    pub reference_count: AtomicU32,
}

pub enum ArtifactType {
    WasmModule,
    AppManifest,
    ConfigBundle,
    Generic,
}

pub enum ArtifactBackend {
    Memory(DashMap<String, Vec<u8>>),
    File { base_path: PathBuf },
}
```

**Behavior**:
- `store(content, type)` -- compute BLAKE3 hash, store if new
- `load(hash)` -- retrieve content, verify hash on load
- Deduplication: same content = same hash = one copy
- Reference counting for garbage collection
- Tree registration at `/kernel/artifacts/{hash}`
- `WasmToolRunner::load_module()` modified to load by hash

**Tests**:
- Store and load roundtrip
- Hash verification on load (tampered content detected)
- Duplicate store returns same hash
- File backend correct path structure
- Reference counting works
- `WasmToolRunner` loads from artifact store

---

## K3c Gaps: ECC Cognitive Substrate

**Why K3c**: The ECC modules (`causal.rs`, `cognitive_tick.rs`, `crossref.rs`,
`hnsw_service.rs`, `impulse.rs`, `calibration.rs`) were introduced in K3c.
These gaps extend the cognitive substrate with the WeaverEngine service and
the EmbeddingProvider abstraction required for HNSW vectorization.

### K3c-G1: WeaverEngine SystemService (~500 lines)

**Source**: ECC Weaver SPARC Plan (09-ecc-weaver-crate.md)

Register `WeaverEngine` as a `SystemService` at boot (when `ecc` feature enabled):
- `ModelingSession` management (start, resume, stop)
- Confidence-driven modeling loop within CognitiveTick
- Multi-source data ingestion (git, files, docs, CI, issues)
- `weave-model.json` export for edge deployment
- Meta-Loom tracking (Weaver's own evolution as causal events)
- WeaverKnowledgeBase for cross-domain learning
- A2ARouter IPC message handling for `weaver ecc` CLI commands
- Tree registration at `/kernel/services/weaver`

**Files**: New `crates/clawft-kernel/src/weaver.rs` or `crates/ecc-weaver/`
**Estimate**: ~500 lines for core service + ~300 lines for data source ingestion
**Priority**: P1 (High -- enables ECC to actually process real data)

**Exit criteria**:
- [ ] WeaverEngine registers as SystemService at boot
- [ ] Modeling session start/stop via IPC
- [ ] Confidence evaluation produces gap analysis
- [ ] Model export produces valid weave-model.json
- [ ] Git log ingestion creates causal nodes and edges
- [ ] File tree ingestion creates namespace structure
- [ ] Meta-Loom records Weaver's own decisions
- [ ] CognitiveTick handler respects budget

### K3c-G2: EmbeddingProvider Trait (~150 lines)

Trait for pluggable embedding backends (local ONNX, remote LLM API, mock
for testing). Required by WeaverEngine for HNSW vectorization of ingested
data.

**Files**: New `crates/clawft-kernel/src/embedding.rs`
**Estimate**: ~150 lines
**Priority**: P1 (High -- required by WeaverEngine for HNSW operations)

**Exit criteria**:
- [ ] EmbeddingProvider trait defined
- [ ] Mock implementation for testing
- [ ] ONNX backend (behind feature flag)

---

## K5 Gaps: App Framework

**Why K5**: The `AppManager` (`app.rs`), `AppManifest`, and application
lifecycle were introduced in K5. Config, secrets, and auth are operational
services that applications depend on. Per-agent tree views are a namespace
mechanism for application isolation.

### K5-G1: Config and Secrets Service (~350 lines)

**Source**: Kubernetes ConfigMaps/Secrets (R14)

**Files**: New `crates/clawft-kernel/src/config_service.rs`

**Types**:
```rust
/// Configuration and secrets service backed by the ResourceTree.
pub struct ConfigService {
    tree: Arc<TreeManager>,
    subscribers: DashMap<String, Vec<mpsc::Sender<ConfigChange>>>,
    encryption_key: [u8; 32],
}

pub struct ConfigChange {
    pub namespace: String,
    pub key: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub changed_by: Pid,
    pub timestamp: DateTime<Utc>,
}

pub struct SecretRef {
    pub namespace: String,
    pub key: String,
    pub expires_at: DateTime<Utc>,
    pub scoped_to: Vec<Pid>,
}
```

**Behavior**:
- Config at `/kernel/config/{namespace}/{key}` in ResourceTree
- Secrets at `/kernel/secrets/{namespace}/{key}` -- encrypted at rest
- `subscribe(namespace)` for change notifications
- Secrets encrypted using BLAKE3-derived key from ExoChain genesis
- Config changes logged to ExoChain; secret reads logged for audit

**Tests**:
- Set and get config roundtrip
- Config change notification delivered
- Secret encrypted at rest
- Unauthorized PID cannot read secrets
- Secret expiry works
- Config changes logged to ExoChain

### K5-G2: Auth Agent (Factotum) (~300 lines)

**Source**: Plan 9 Factotum (R11)

**Why K5**: Applications need credential management. The Factotum pattern
prevents agents from holding raw credentials.

**Files**: New `crates/clawft-kernel/src/auth_service.rs`

**Types**:
```rust
/// Centralized credential management service (Plan 9 Factotum pattern).
pub struct AuthService {
    credentials: DashMap<String, StoredCredential>,
    active_tokens: DashMap<String, IssuedToken>,
    config_service: Arc<ConfigService>,
}

pub struct StoredCredential {
    pub name: String,
    pub credential_type: CredentialType,
    pub encrypted_value: Vec<u8>,
    pub allowed_agents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

pub enum CredentialType {
    ApiKey,
    BearerToken,
    Certificate,
    Custom(String),
}

pub struct IssuedToken {
    pub token_id: String,
    pub credential_name: String,
    pub issued_to: Pid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub scope: Vec<String>,
}
```

**Behavior**:
- `register_credential(name, type, value, allowed_agents)` -- store credential
- `request_token(credential_name, requester_pid, scope, ttl)` -- issue scoped token
- Agents never receive raw credentials
- Token expiry enforced; `allowed_agents` checked
- Token issuance and credential access logged to ExoChain
- Credentials stored via `ConfigService.set_secret()` (encrypted at rest)

**Tests**:
- Register credential and request token
- Unauthorized agent cannot request token
- Expired token rejected
- Token scope restricts operations
- Raw credential value never exposed
- Credential rotation: update credential, existing tokens continue working

### K5-G3: Per-Agent Tree Views (~300 lines)

**Source**: Plan 9 namespaces (R9)

**Why K5**: Application agents need isolated views of the ResourceTree.
`IpcScope` from K1 provides the permission model; tree views apply it at
the storage layer.

**Files**: Extend `crates/clawft-kernel/src/tree_manager.rs`

**Types**:
```rust
/// Filtered view of the ResourceTree scoped to an agent's capabilities.
pub struct AgentTreeView {
    pub agent_id: String,
    pub pid: Pid,
    pub allowed_paths: Vec<String>,
    pub tree: Arc<TreeManager>,
}
```

**Behavior**:
- `IpcScope::Restricted`: sees only `/agents/{agent_id}/**` and `/kernel/config/**` (read-only)
- `IpcScope::Full`: sees entire tree
- `IpcScope::NamespaceOnly(ns)`: sees `/agents/{agent_id}/**` and `/namespaces/{ns}/**`
- All tree operations pass through view filter
- Unauthorized access returns `KernelError::PermissionDenied`

**Tests**:
- Restricted agent sees only own subtree
- Restricted agent cannot read `/kernel/secrets/`
- Full agent sees entire tree
- Namespace-scoped agent sees correct paths
- Write to unauthorized path returns `PermissionDenied`

---

## K6 Gaps: Mesh Networking

**Why K6**: The mesh transport, framing, and peer discovery were introduced
in K6. These gaps extend mesh with artifact exchange and cross-node log
aggregation.

### K6-G1: Artifact Exchange Protocol (~200 lines)

**Source**: IPFS Bitswap (R17)

**Why K6**: Artifact exchange requires the mesh transport (K6) to transfer
content-addressed blobs between nodes.

**Files**: Modify `crates/clawft-kernel/src/mesh_framing.rs`, extend `artifact_store.rs`

**Changes**:
- Add `FrameType::ArtifactRequest { hash: String }` to mesh framing
- Add `FrameType::ArtifactResponse { hash: String, data: Vec<u8> }` to mesh framing
- Nodes that have an artifact serve it on request
- `artifact_store.request_remote(hash, node_id)` asks mesh peer for artifact
- BLAKE3 hash verified on receipt
- New artifact announcements gossiped via mesh topic

**Tests**:
- Request artifact from remote node
- Hash verified on receipt (mismatch rejected)
- Non-existent artifact returns error
- Artifact gossip announces new hashes

### K6-G2: Cross-Node Log Aggregation (~100 lines)

**Why K6**: With mesh networking, logs from multiple nodes should be
queryable from a single point.

**Files**: Extend `crates/clawft-kernel/src/log_service.rs`

**Behavior**:
- Log entries include `node_id` field when received from remote nodes
- `LogQuery` gains `node_id: Option<String>` filter
- Remote log query: `mesh.request()` to peer's `LogService` via `ServiceApi`
- Aggregated results merged by timestamp

**Tests**:
- Remote log entries carry source `node_id`
- Log query filters by `node_id`
- Aggregated query merges results from multiple nodes

---

## Cross-Cutting Gaps

These gaps apply across all K-levels and are registered as `SystemService`
instances at boot time.

### CC-G1: Metrics Collection (~350 lines)

**Why cross-cutting**: Every kernel subsystem emits metrics. The
`MetricsRegistry` collects from `A2ARouter` (K2), `AgentSupervisor` (K1),
`WasmToolRunner` (K3), `GovernanceEngine` (K5), `ChainManager` (ExoChain).

**Files**: New `crates/clawft-kernel/src/metrics.rs`

**Types**:
```rust
/// Registry of all kernel metrics. Lock-free on hot path.
pub struct MetricsRegistry {
    counters: DashMap<String, AtomicU64>,
    gauges: DashMap<String, AtomicI64>,
    histograms: DashMap<String, Histogram>,
}

pub struct Histogram {
    buckets: Vec<(f64, AtomicU64)>,
    sum: AtomicU64,
    count: AtomicU64,
}

pub enum MetricSnapshot {
    Counter { name: String, value: u64 },
    Gauge { name: String, value: i64 },
    Histogram { name: String, buckets: Vec<(f64, u64)>, sum: f64, count: u64 },
}
```

**Built-in metrics**:

| Name | Type | Source Module |
|------|------|--------------|
| `kernel.messages_sent` | counter | `a2a.rs` (K2) |
| `kernel.messages_delivered` | counter | `a2a.rs` (K2) |
| `kernel.messages_dropped` | counter | `dead_letter.rs` (K2-G1) |
| `kernel.agent_spawns` | counter | `supervisor.rs` (K1) |
| `kernel.agent_crashes` | counter | `supervisor.rs` (K1-G1) |
| `kernel.tool_executions` | counter | `wasm_runner.rs` (K3) |
| `kernel.active_agents` | gauge | `process.rs` (K1) |
| `kernel.active_services` | gauge | `service.rs` (K1) |
| `kernel.chain_length` | gauge | `chain.rs` (ExoChain) |
| `kernel.ipc_latency_ms` | histogram | `a2a.rs` (K2) |
| `kernel.tool_execution_ms` | histogram | `wasm_runner.rs` (K3) |
| `kernel.governance_evaluation_ms` | histogram | `governance.rs` (K5) |

**Tests**:
- Counter increment/get atomic under concurrent access
- Gauge set/increment/decrement
- Histogram record + percentile query (p50, p95, p99)
- Registry list all metrics
- Built-in metrics populated during agent lifecycle
- `MetricsRegistry` registered as `SystemService`

### CC-G2: Structured Log Service + Rotation (~500 lines)

**Why cross-cutting**: Log ingestion comes from all kernel subsystems via
the `tracing` subscriber.

**Files**: New `crates/clawft-kernel/src/log_service.rs`

**Types**:
```rust
/// Structured logging service with ring buffer and query support.
pub struct LogService {
    entries: RwLock<VecDeque<LogEntry>>,
    max_entries: usize,
    subscribers: DashMap<String, mpsc::Sender<LogEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source_pid: Option<Pid>,
    pub source_service: Option<String>,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub trace_id: Option<String>,
}

pub struct LogQuery {
    pub pid: Option<Pid>,
    pub service: Option<String>,
    pub level_min: Option<LogLevel>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: usize,
    pub trace_id: Option<String>,
}
```

Note: `LogLevel` already exists in the kernel (`console::LogLevel`).

**Behavior**:
- Ring buffer, configurable max (default 100,000)
- Query by PID, service, level, time range, trace_id
- Real-time subscription via `mpsc` channel
- File persistence: `{data_dir}/logs/kernel-{date}.jsonl`
- Rotation at 10MB or midnight
- Retention: 7 days default (configurable)
- Registered as `SystemService`

**Tests**:
- Log entries stored and queryable
- Query by PID returns only matching
- Ring buffer evicts oldest at capacity
- Subscriber receives entries in real-time
- Level filter works
- File rotation at size threshold
- Expired log files cleaned up

### CC-G3: Timer Service (~200 lines)

**Why cross-cutting**: Timers are used by self-healing (K1 restart backoff),
reliable IPC (K2 ack timeout), probes (K2b), and application logic (K5).

**Files**: New `crates/clawft-kernel/src/timer.rs`

**Types**:
```rust
/// Timer service: one-shot and repeating timers with message delivery.
///
/// Complements CronService (cron expressions, minute granularity)
/// with sub-second precision timers.
pub struct TimerService {
    timers: DashMap<String, TimerEntry>,
    next_id: AtomicU64,
}

pub struct TimerEntry {
    pub id: String,
    pub owner_pid: Pid,
    pub fire_at: DateTime<Utc>,
    pub repeat_interval: Option<Duration>,
    pub payload: MessagePayload,
    pub cancel_token: CancellationToken,
}
```

**Behavior**:
- One-shot and repeating timer creation
- On fire: deliver `KernelMessage` to `owner_pid` via `A2ARouter`
- Cancel support via `CancellationToken`
- Registered as `SystemService` alongside `CronService`
- Owner PID death cancels all owned timers (via `ProcessDown` monitor)
- Sub-second precision via `tokio::time::sleep()`

**Tests**:
- One-shot timer fires at specified time
- Repeating timer fires multiple times
- Sub-second precision (100ms timer within 50ms tolerance)
- Cancel prevents fire
- Owner PID death cancels timers
- Registered as `SystemService`

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

### 3. Dead Letter Queue Delivery + Retry (K2-G1)

```
fn dead_letter_intake(dlq: &DeadLetterQueue, message: KernelMessage, reason: DeadLetterReason):
    letter = DeadLetter { message, reason, timestamp: Utc::now(), retry_count: 0 }

    let mut letters = dlq.letters.write()
    while letters.len() >= dlq.max_size:
        letters.pop_front()  // FIFO eviction

    letters.push_back(letter.clone())

    chain.append("ipc", "ipc.dead_letter", {
        msg_id: letter.message.id,
        target: format!("{:?}", letter.message.target),
        reason: format!("{:?}", letter.reason),
    })
    metrics.increment("kernel.messages_dropped")

fn dead_letter_retry(dlq: &DeadLetterQueue, msg_id: &str, a2a: &A2ARouter) -> Result<()>:
    let mut letters = dlq.letters.write()
    let idx = letters.iter().position(|l| l.message.id == msg_id)
        .ok_or(KernelError::NotFound)?
    let mut letter = letters.remove(idx).unwrap()
    letter.retry_count += 1

    match a2a.send(letter.message.clone()).await:
        Ok(()) -> chain.append("ipc", "ipc.dead_letter.retry.success", { msg_id })
        Err(e) ->
            letters.push_back(letter)  // re-add with incremented retry
            Err(e)
```

### 4. Reliable Message Send with Ack Tracking (K2-G2)

```
fn send_reliable(queue: &ReliableQueue, msg: KernelMessage) -> DeliveryResult:
    if msg.correlation_id.is_none():
        msg.correlation_id = Some(uuid::new_v4().to_string())
    let correlation_id = msg.correlation_id.clone().unwrap()

    queue.pending.insert(correlation_id, PendingDelivery {
        message: msg.clone(),
        attempts: 1,
        first_sent: Instant::now(),
        last_attempt: Instant::now(),
        ack_deadline: Instant::now() + queue.config.initial_timeout,
    })

    a2a.send(msg.clone()).await

    let mut timeout = queue.config.initial_timeout
    let mut attempts = 1

    loop:
        match tokio::time::timeout(timeout, ack_receiver.recv()).await:
            Ok(ack_msg) ->
                let ack_time = Instant::now().duration_since(pending.first_sent)
                queue.pending.remove(&correlation_id)
                chain.append("ipc", "ipc.reliable.ack", { msg_id, ack_time_ms })
                return Acknowledged { msg_id, ack_time }

            Err(_) ->
                attempts += 1
                chain.append("ipc", "ipc.reliable.timeout", { msg_id, attempt: attempts })

                if attempts > queue.config.max_retries:
                    queue.pending.remove(&correlation_id)
                    queue.dead_letter.intake(msg, Timeout { duration_ms })
                    return MaxRetriesExceeded { msg_id, attempts }

                timeout = min(
                    timeout * queue.config.backoff_multiplier,
                    queue.config.max_timeout
                )
                if let Some(mut p) = queue.pending.get_mut(&correlation_id):
                    p.attempts = attempts
                    p.last_attempt = Instant::now()
                    p.ack_deadline = Instant::now() + timeout

                a2a.send(msg.clone()).await  // retry
```

### 5. Resource Limit Enforcement Loop (K1-G3)

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
|       ^              |  ^          ^              ^               |
+-------|--------------|--|----------|--------------|---------------+
        |              |  |          |              |
+-------|--------------|--|----------|--------------|---------------+
| GAP   |              |  |          |              |               |
| FILLS |         K2-G1|  |  K2b-G2 |         K1-G1|               |
|  +----+--------+  +--v--+--+  +---+--------+  +--+------------+  |
|  | Reconciler  |  | Dead   |  | Probes     |  | Restart       |  |
|  | Controller  |  | Letter |  | (liveness/ |  | Strategies    |  |
|  | (K2b-G1)    |  | Queue  |  |  readiness)|  | + Watchdog    |  |
|  |             |  | (K2-G1)|  | (K2b-G2)   |  | (K1-G1)       |  |
|  +------+------+  +--------+  +------------+  +------+--------+  |
|         |              ^                             |            |
|         v              |                             v            |
|  +------+------+  +----+-------+              +------+--------+  |
|  | AppManager  |  | Reliable   |              | Process Links |  |
|  | (K5, for    |  | Queue      |              | & Monitors    |  |
|  |  desired    |  | (K2-G2)    |              | (K1-G2)       |  |
|  |  state)     |  +------------+              +---------------+  |
|  +-------------+                                                 |
|                                                                   |
|  +-------------+  +------------+  +-------------+  +-----------+ |
|  | Metrics     |  | Log        |  | Timer       |  | Config/   | |
|  | Registry    |  | Service    |  | Service     |  | Secrets   | |
|  | (CC-G1)     |  | (CC-G2)    |  | (CC-G3)     |  | (K5-G1)  | |
|  +------+------+  +------+-----+  +------+------+  +-----+-----+|
|         ^                ^                ^               |       |
|    ALL KERNEL       tracing           CronService    TreeManager  |
|    SUBSYSTEMS       subscriber        (existing)     (existing)   |
|                                                                   |
|  +-------------+  +------------+  +-------------+                |
|  | Auth        |  | Artifact   |  | Agent Tree  |                |
|  | Service     |  | Store      |  | Views       |                |
|  | (K5-G2)     |  | (K3-G1)    |  | (K5-G3)     |                |
|  +------+------+  +------+-----+  +------+------+                |
|         |                |                |                       |
|    ConfigService    WasmToolRunner   AgentCapabilities            |
|    (K5-G1)          (K3, module     (K1, IpcScope                |
|                      load by hash)   filter)                      |
+------------------------------------------------------------------+
```

### Component Integration Map

| Gap ID | New Component | K-Level | Depends On | Registered As | Tree Path |
|--------|--------------|---------|-----------|---------------|-----------|
| K1-G1 | `RestartStrategy` + watchdog | K1 | `AgentSupervisor`, `ProcessTable` | Part of `AgentSupervisor` | -- |
| K1-G2 | `ProcessLink`, `ProcessMonitor` | K1 | `ProcessTable`, `A2ARouter` | Part of `ProcessTable` | -- |
| K1-G3 | Resource enforcement loop | K1 | `ResourceLimits`, `ProcessTable` | Per-agent background task | -- |
| K1-G4 | `max_disk_bytes` | K1 | `ResourceLimits`, `TreeManager` | Part of `ResourceLimits` | -- |
| K2-G1 | `DeadLetterQueue` | K2 | `A2ARouter`, `ChainManager` | `SystemService::Core` | `/kernel/services/dead_letter` |
| K2-G2 | `ReliableQueue` | K2 | `A2ARouter`, `DeadLetterQueue` | `SystemService::Core` | `/kernel/services/reliable_queue` |
| K2-G3 | `NamedPipeRegistry` | K2 | `A2ARouter`, `CapabilityChecker` | `SystemService::Core` | `/kernel/pipes/{name}` |
| K2-G4 | `trace_id` on `KernelMessage` | K2 | `KernelMessage`, `A2ARouter` | Field addition | -- |
| K2-G5 | New `KernelSignal` variants | K2 | `KernelSignal` | Enum extension | -- |
| K2b-G1 | `ReconciliationController` | K2b | `ProcessTable`, `AppManager`, `GovernanceEngine` | `SystemService::Core` | `/kernel/services/reconciler` |
| K2b-G2 | Liveness/readiness probes | K2b | `SystemService`, `HealthSystem` | Per-service probe task | -- |
| K3-G1 | `ArtifactStore` | K3 | `blake3`, `TreeManager` | `SystemService::Core` | `/kernel/artifacts/{hash}` |
| K5-G1 | `ConfigService` | K5 | `TreeManager`, `ChainManager` | `SystemService::Core` | `/kernel/config/{ns}/{key}` |
| K5-G2 | `AuthService` | K5 | `ConfigService`, `ChainManager` | `SystemService::Core` | `/kernel/services/auth` |
| K5-G3 | `AgentTreeView` | K5 | `TreeManager`, `AgentCapabilities` | Part of `TreeManager` | -- |
| K6-G1 | Artifact exchange frames | K6 | `ArtifactStore`, mesh transport | `FrameType` extension | -- |
| K6-G2 | Cross-node log aggregation | K6 | `LogService`, mesh `ServiceApi` | Extension of `LogService` | -- |
| CC-G1 | `MetricsRegistry` | All | All kernel subsystems | `SystemService::Core` | `/kernel/services/metrics` |
| CC-G2 | `LogService` | All | `tracing` subscriber | `SystemService::Core` | `/kernel/services/log` |
| CC-G3 | `TimerService` | All | `A2ARouter`, `CancellationToken` | `SystemService::Core` | `/kernel/services/timer` |

### Boot Sequence Integration

Gap-filling services register during `boot.rs` startup, after existing
K0-K6 services. Order respects dependencies:

```
Existing boot (K0-K6):
  1. ProcessTable            (K0)
  2. KernelIpc + MessageBus  (K0)
  3. A2ARouter + TopicRouter (K2)
  4. ServiceRegistry         (K1)
  5. HealthSystem            (K1)
  6. AgentSupervisor         (K1)
  7. WasmToolRunner          (K3)
  8. AppManager              (K5)
  9. CronService             (K5)
  10. GovernanceEngine       (K5)
  11. ChainManager + TreeManager (ExoChain)
  12. Mesh                   (K6, if enabled)

Gap-fill additions (when os-patterns enabled):
  13. MetricsRegistry        (CC-G1)
  14. LogService             (CC-G2)
  15. DeadLetterQueue        (K2-G1, needs A2ARouter)
  16. ReliableQueue          (K2-G2, needs A2ARouter + DLQ)
  17. NamedPipeRegistry      (K2-G3, needs A2ARouter)
  18. ReconciliationController (K2b-G1, needs ProcessTable + AppManager)
  19. TimerService           (CC-G3, needs A2ARouter)
  20. ConfigService          (K5-G1, needs TreeManager)
  21. AuthService            (K5-G2, needs ConfigService)
  22. ArtifactStore          (K3-G1, needs TreeManager, os-full only)
  23. Restart watchdogs      (K1-G1, attached per spawned agent)
  24. Probe runners          (K2b-G2, attached per registered service)
  25. Resource enforcers     (K1-G3, attached per spawned agent)
```

---

## R -- Refinement

### Performance Considerations

- `MetricsRegistry` uses `AtomicU64`/`AtomicI64` -- no lock contention on
  hot path (counter increment = single atomic add)
- `Histogram` uses fixed buckets with atomic counters -- O(buckets) per
  record, no dynamic allocation
- `LogService` ring buffer: `RwLock<VecDeque>` -- many readers, one writer
- `ReliableQueue` retries use `tokio::time::sleep()`, not busy-wait
- `ReconciliationController` interval configurable (default 5s)
- Resource sampling: 1 check/second per agent -- for 100 agents, 100
  lightweight checks/second
- `DeadLetterQueue`: `RwLock<VecDeque>` with FIFO eviction, bounded memory
- `ArtifactStore` file backend: two-level directory sharding
  (`{hash[0..2]}/{hash}`) avoids filesystem limits

### Security Considerations

- `AuthService` encrypts credentials via `ConfigService.set_secret()`
- Named pipes respect `IpcScope` capabilities
- `DeadLetterQueue` governance-gated: `IpcScope::Full` required to read
- `MetricsRegistry` read-only for non-kernel agents
- Config changes logged to ExoChain (tamper-evident audit)
- `AuthService` never exposes raw credentials: scoped tokens only
- `AgentTreeView` enforces capability-based access at storage layer
- Resource limit enforcement prevents runaway agent DoS

### Testing Strategy

- Each gap is independently testable
- Integration test flows:
  - Agent crash -> K1-G1 restart -> K2b-G1 reconciler -> ExoChain event
  - Message undeliverable -> K2-G1 DLQ -> retry -> delivery or re-DLQ
  - K1-G3 resource exceeded -> warning -> shutdown -> force cancel
  - K2b-G2 readiness failure -> remove from registry -> recovery -> re-add
- `ReconciliationController` tested via `CancellationToken::cancel()`
- `ReliableQueue` tested via `MockClock` from K6
- All tests independent of `mesh` feature (single-node)
- K6 gaps use `InMemoryTransport` from K6

### Dependency Minimization

- `blake3` is the only new external dependency (artifact store, `os-full` only)
- All other code uses workspace-existing crates: `dashmap`, `tokio`, `chrono`,
  `serde`, `uuid`
- `os-patterns` feature does not pull networking dependencies
- `blake3` gated behind `os-full`, not `os-patterns`

### Migration Path

- `AgentRestartPolicy` (`Never`/`OnFailure`/`Always`) preserved;
  `RestartStrategy` adds cross-agent behavior on top
- `HealthStatus` enum unchanged; `ProbeResult` is separate
- `KernelMessage` gains `trace_id` with `#[serde(default)]` -- backward
  compatible deserialization
- `ResourceLimits` gains `max_disk_bytes` with default function
- `KernelSignal` gains new variants -- exhaustive matches need updating
  (compile-time enforcement, not runtime surprise)

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

#### K2 IPC & Communication Gate (P1)
- [ ] Dead letter queue captures messages to nonexistent PIDs
- [ ] Dead letter queue captures messages when inbox is full
- [ ] Dead letter queue queryable by target PID, reason, time range
- [ ] Dead letter queue retries messages on demand
- [ ] Dead letter queue size bounded with FIFO eviction
- [ ] Reliable send tracks acknowledgment with timeout
- [ ] Retry with exponential backoff on ack timeout
- [ ] Max retries exceeded routes to dead letter queue
- [ ] Named pipes created and connected successfully
- [ ] Named pipes survive agent restart
- [ ] Named pipes support multiple concurrent senders
- [ ] Named pipe capacity limits enforced
- [ ] Unused named pipes cleaned up after TTL
- [ ] Named pipe access respects `IpcScope` capabilities
- [ ] Distributed trace IDs propagate through IPC chain
- [ ] External messages receive new trace IDs at entry
- [ ] New `KernelSignal` variants serialize/deserialize correctly

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

#### K3 WASM Sandbox Gate (P2)
- [ ] Artifacts stored by BLAKE3 hash
- [ ] Artifact retrieval verifies hash on load
- [ ] Duplicate artifacts deduplicated automatically
- [ ] WASM modules loaded from artifact store by hash
- [ ] File backend writes to correct path structure
- [ ] Reference counting and garbage collection work

#### K5 App Framework Gate (P2)
- [ ] Config service stores and retrieves configuration
- [ ] Config change notifications delivered to subscribers
- [ ] Config changes logged to ExoChain
- [ ] Secret service encrypts at rest
- [ ] Secret service delivers scoped, time-limited access
- [ ] Unauthorized PID cannot read secrets
- [ ] Auth service registers and manages credentials
- [ ] Auth service issues scoped, time-limited tokens
- [ ] Agents never receive raw credentials
- [ ] Credential access logged to ExoChain
- [ ] Per-agent tree views filter by capabilities
- [ ] Restricted agents cannot see unauthorized tree paths
- [ ] Namespace-scoped agents see correct paths
- [ ] Write to unauthorized tree path returns `PermissionDenied`

#### K6 Mesh Networking Gate (P2)
- [ ] Artifact exchange protocol transfers between mesh nodes
- [ ] Hash mismatch on receipt is rejected
- [ ] Artifact gossip announces new hashes
- [ ] Remote log entries carry source `node_id`
- [ ] Log query filters by `node_id`
- [ ] Aggregated query merges from multiple nodes

#### Cross-Cutting Gate (P1)
- [ ] Counter metric type: increment and get (atomic)
- [ ] Gauge metric type: set, increment, decrement
- [ ] Histogram metric type: record and percentile query (p50, p95, p99)
- [ ] Built-in kernel metrics populated (messages, agents, tools, chain)
- [ ] `MetricsRegistry` registered as `SystemService` at boot
- [ ] Structured log entries stored in ring buffer
- [ ] Log query by PID, service, level, time range, trace_id
- [ ] Log subscription delivers entries in real-time
- [ ] Log rotation at configurable size/time thresholds
- [ ] Expired log files cleaned up automatically
- [ ] `LogService` registered as `SystemService` at boot
- [ ] One-shot timer fires at specified time
- [ ] Repeating timer fires at specified interval
- [ ] Sub-second timer precision (100ms tolerance)
- [ ] Timer cancellation prevents fire
- [ ] Owner PID death cancels owned timers
- [ ] Timer service registered as `SystemService`

### Full Phase Gate
- [ ] All existing tests pass (843+ without gap fills)
- [ ] All gap-fill tests pass (target: 150+ new tests)
- [ ] Clippy clean for all new code (`scripts/build.sh clippy`)
- [ ] Feature gated appropriately (`os-patterns` / `os-full`)
- [ ] No mandatory new dependencies for existing build
- [ ] `blake3` only required when `os-full` enabled
- [ ] Documentation updated in Fumadocs site
- [ ] `os-gap-gate.sh` script passes all exit criteria
- [ ] `scripts/build.sh gate` passes with `os-patterns` feature

### Testing Verification Commands

```bash
# Build with OS patterns feature
scripts/build.sh native --features os-patterns

# Build with full OS features (includes blake3)
scripts/build.sh native --features os-full

# Run gap-fill tests
scripts/build.sh test -- --features os-patterns

# Verify base build unchanged (no gap-fill deps)
scripts/build.sh check

# Full phase gate
scripts/build.sh gate
```

### Implementation Order

The implementation order respects dependencies while allowing parallelism
within each priority tier:

```
Priority 0 (Critical) -- do first:
  K1-G1: Restart strategies (foundation for self-healing)
  K1-G2: Process links/monitors (crash notification)
  K2-G5: Expanded KernelSignal (needed by K1-G2, K1-G3)

Priority 1 (High) -- parallel after P0:
  Stream A:                    Stream B:                 Stream C:
  K2-G1: Dead letter queue     K2b-G1: Reconciler       CC-G1: Metrics
  K2-G2: Reliable queue        K2b-G2: Probes           CC-G2: Log service
  K2-G3: Named pipes           K1-G3: Resource enforce  CC-G3: Timer service
  K2-G4: Trace IDs             K1-G4: Disk quotas

Priority 2 (Important) -- after P1:
  Stream D:                    Stream E:
  K5-G1: Config service        K3-G1: Artifact store
  K5-G2: Auth service          K6-G1: Artifact exchange
  K5-G3: Tree views            K6-G2: Log aggregation
```

### Agent Spawning Execution Plan

| Agent | Type | Gap IDs | Responsibility |
|-------|------|---------|----------------|
| `k1-supervisor-agent` | `coder` | K1-G1, K1-G2, K1-G3, K1-G4 | Restart, links, resource enforcement |
| `k2-ipc-agent` | `coder` | K2-G1, K2-G2, K2-G3, K2-G4, K2-G5 | DLQ, reliable queue, pipes, tracing, signals |
| `k2b-harden-agent` | `coder` | K2b-G1, K2b-G2 | Reconciler, probes |
| `k3-artifact-agent` | `coder` | K3-G1 | Content-addressed store |
| `k5-ops-agent` | `coder` | K5-G1, K5-G2, K5-G3 | Config, auth, tree views |
| `k6-mesh-gap-agent` | `coder` | K6-G1, K6-G2 | Artifact exchange, log aggregation |
| `cc-observability-agent` | `coder` | CC-G1, CC-G2, CC-G3 | Metrics, logging, timers |
| `gap-test-agent` | `tester` | All | Integration tests, gate verification |
| `gap-review-agent` | `reviewer` | All | Code review, security audit |
