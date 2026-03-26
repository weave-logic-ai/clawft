# 08b Reliable IPC & Observability

**Document ID**: 08b
**Workstream**: W-KERNEL
**Duration**: Weeks 17-22
**Goal**: Complete K2 IPC reliability gaps and cross-cutting observability -- dead letter queue, reliable delivery, named pipes, trace IDs, signal vocabulary, metrics, structured logging, and timer service
**Depends on**: K0-K6 (all prior phases), 08a K1-G1 (restart strategies must be done before DLQ starts)
**Orchestrator**: `08-orchestrator.md`
**Priority**: P1 (High)

---

## S -- Specification

### What Changes

K2 delivered the `A2ARouter`, `KernelIpc`, `KernelMessage`, `KernelSignal`,
and `TopicRouter`. Messages that fail delivery are currently silently dropped.
There is no ack-based retry, no persistent channels, no distributed tracing,
and no centralized metrics or structured logging.

This phase fills all IPC reliability gaps and adds the cross-cutting
observability stack that every other kernel subsystem depends on.

### Gap Summary

| Gap ID | Area | New Lines | Changed Lines | Priority |
|--------|------|:---------:|:------------:|----------|
| K2-G1 | Dead letter queue | ~150 | ~30 | P1 |
| K2-G2 | Reliable message delivery | ~400 | ~30 | P1 |
| K2-G3 | Named pipes | ~250 | ~20 | P1 |
| K2-G4 | Distributed trace IDs | ~100 | ~50 | P1 |
| K2-G5 | Expanded KernelSignal vocabulary | ~50 | ~20 | P0 (needed by 08a) |
| Cross-G1 | Metrics collection | ~350 | ~50 | P1 |
| Cross-G2 | Structured log service + rotation | ~500 | ~50 | P1 |
| Cross-G3 | Timer service | ~200 | ~30 | P1 |
| **Total** | | **~2,000** | **~280** | |

### Feature Gate

All code gated behind `#[cfg(feature = "os-patterns")]`.

```toml
[features]
os-patterns = ["exochain"]
```

### Scheduling Note

K2-G5 (KernelSignal vocabulary) is needed by 08a (K1-G2 uses `LinkExit` and
`MonitorDown`, K1-G3 uses `ResourceWarning`). Schedule K2-G5 as the first
item in this phase (week 17), or implement it as part of 08a if the
process-supervisor agent handles it.

Cross-G1 (metrics) can start in week 17 in parallel with 08a -- it has no
dependency on restart strategies.

---

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

---

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

---

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

---

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

---

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

### Cross-G1: Metrics Collection (~350 lines)

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

---

### Cross-G2: Structured Log Service + Rotation (~500 lines)

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

---

### Cross-G3: Timer Service (~200 lines)

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

### 1. Dead Letter Queue Delivery + Retry (K2-G1)

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

### 2. Reliable Message Send with Ack Tracking (K2-G2)

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

### 3. Metrics Collection Hot Path (Cross-G1)

```
fn counter_increment(registry: &MetricsRegistry, name: &str):
    // Lock-free: DashMap lookup + atomic increment
    match registry.counters.get(name):
        Some(counter) -> counter.fetch_add(1, Ordering::Relaxed)
        None ->
            registry.counters.insert(name.to_string(), AtomicU64::new(1))

fn histogram_record(registry: &MetricsRegistry, name: &str, value: f64):
    match registry.histograms.get(name):
        Some(hist) ->
            // Find bucket and increment
            for (bound, count) in &hist.buckets:
                if value <= *bound:
                    count.fetch_add(1, Ordering::Relaxed)
                    break
            hist.sum.fetch_add(value.to_bits(), Ordering::Relaxed)
            hist.count.fetch_add(1, Ordering::Relaxed)
        None ->
            // Create with default buckets: 1, 5, 10, 25, 50, 100, 250, 500, 1000
            let hist = Histogram::new(DEFAULT_BUCKETS)
            hist.record(value)
            registry.histograms.insert(name.to_string(), hist)

fn snapshot_all(registry: &MetricsRegistry) -> Vec<MetricSnapshot>:
    let mut snapshots = vec![]
    for entry in registry.counters.iter():
        snapshots.push(Counter { name: entry.key().clone(), value: entry.value().load(Relaxed) })
    for entry in registry.gauges.iter():
        snapshots.push(Gauge { name: entry.key().clone(), value: entry.value().load(Relaxed) })
    for entry in registry.histograms.iter():
        snapshots.push(entry.value().snapshot(entry.key().clone()))
    snapshots
```

### 4. Log Service Query + Subscription (Cross-G2)

```
fn log_ingest(service: &LogService, entry: LogEntry):
    let mut entries = service.entries.write()
    while entries.len() >= service.max_entries:
        entries.pop_front()
    entries.push_back(entry.clone())
    drop(entries)  // release write lock before notifying subscribers

    // Fan-out to subscribers (non-blocking)
    let mut dead_subs = vec![]
    for sub in service.subscribers.iter():
        if sub.value().try_send(entry.clone()).is_err():
            dead_subs.push(sub.key().clone())
    for key in dead_subs:
        service.subscribers.remove(&key)

fn log_query(service: &LogService, query: LogQuery) -> Vec<LogEntry>:
    let entries = service.entries.read()
    entries.iter()
        .filter(|e| query.pid.map_or(true, |pid| e.source_pid == Some(pid)))
        .filter(|e| query.service.as_ref().map_or(true, |s| e.source_service.as_ref() == Some(s)))
        .filter(|e| query.level_min.map_or(true, |min| e.level >= min))
        .filter(|e| query.since.map_or(true, |since| e.timestamp >= since))
        .filter(|e| query.until.map_or(true, |until| e.timestamp <= until))
        .filter(|e| query.trace_id.as_ref().map_or(true, |tid| e.trace_id.as_ref() == Some(tid)))
        .take(query.limit)
        .cloned()
        .collect()

fn log_rotation_task(service: &LogService, data_dir: PathBuf):
    let current_file = None
    let current_size = 0

    loop:
        sleep(Duration::from_secs(10))

        let today = Utc::now().format("%Y-%m-%d").to_string()
        let path = data_dir.join("logs").join(format!("kernel-{}.jsonl", today))

        // Check rotation conditions
        if current_file != Some(path.clone()) or current_size >= 10_000_000:
            // Start new file
            current_file = Some(path)
            current_size = 0

        // Flush buffered entries to file
        let entries_to_flush = drain_flush_buffer()
        append_jsonl(&current_file, &entries_to_flush)
        current_size += entries_to_flush.encoded_size()

        // Cleanup old files (> 7 days)
        cleanup_expired_logs(&data_dir.join("logs"), Duration::from_secs(7 * 86400))
```

---

## A -- Architecture

### Component Integration Diagram

```
+------------------------------------------------------------------+
|                      EXISTING KERNEL (K0-K6)                      |
|                                                                   |
|  +-----------+  +----------+  +----------+  +-------------------+ |
|  | Process   |  | A2A      |  | Service  |  | tracing           | |
|  | Table     |  | Router   |  | Registry |  | subscriber        | |
|  | (K1)      |  | (K2)     |  | (K1)     |  |                   | |
|  +-----------+  +----+-----+  +----------+  +-------------------+ |
|                      |                            |                |
+----------------------|----------------------------|----------------+
                       |                            |
+----------------------|----------------------------|----------------+
| 08b                  |                            |                |
| FILLS           K2-G1|                            |                |
|  +-------------------v--+  +------------------+   |                |
|  | Dead Letter Queue    |  | Reliable Queue   |   |                |
|  | (dead_letter.rs)     |<-| (reliable_queue  |   |                |
|  |                      |  |  .rs)            |   |                |
|  +----------------------+  +------------------+   |                |
|                                                   |                |
|  +----------------------+  +------------------+   |                |
|  | Named Pipe Registry  |  | Trace ID         |   |                |
|  | (named_pipe.rs)      |  | propagation      |   |                |
|  | K2-G3                |  | (ipc.rs, a2a.rs) |   |                |
|  +----------------------+  | K2-G4            |   |                |
|                            +------------------+   |                |
|                                                   |                |
|  +----------------------+  +------------------+   v                |
|  | Metrics Registry     |  | Log Service      +---+               |
|  | (metrics.rs)         |  | (log_service.rs) |                   |
|  | Cross-G1             |  | Cross-G2         |                   |
|  | Lock-free counters,  |  | Ring buffer,     |                   |
|  | gauges, histograms   |  | query, rotation  |                   |
|  +----------------------+  +------------------+                   |
|                                                                   |
|  +----------------------+  +------------------+                   |
|  | Timer Service        |  | KernelSignal     |                   |
|  | (timer.rs)           |  | new variants     |                   |
|  | Cross-G3             |  | K2-G5            |                   |
|  +----------------------+  +------------------+                   |
+------------------------------------------------------------------+
```

### Component Integration Map

| Gap ID | New Component | Files | Depends On | Registered As | Tree Path |
|--------|--------------|-------|-----------|---------------|-----------|
| K2-G1 | `DeadLetterQueue` | new `dead_letter.rs` | `A2ARouter`, `ChainManager` | `SystemService::Core` | `/kernel/services/dead_letter` |
| K2-G2 | `ReliableQueue` | new `reliable_queue.rs` | `A2ARouter`, `DeadLetterQueue` | `SystemService::Core` | `/kernel/services/reliable_queue` |
| K2-G3 | `NamedPipeRegistry` | new `named_pipe.rs` | `A2ARouter`, `CapabilityChecker` | `SystemService::Core` | `/kernel/pipes/{name}` |
| K2-G4 | `trace_id` on `KernelMessage` | `ipc.rs`, `a2a.rs` | `KernelMessage`, `A2ARouter` | Field addition | -- |
| K2-G5 | New `KernelSignal` variants | `ipc.rs` | `KernelSignal` | Enum extension | -- |
| Cross-G1 | `MetricsRegistry` | new `metrics.rs` | All kernel subsystems | `SystemService::Core` | `/kernel/services/metrics` |
| Cross-G2 | `LogService` | new `log_service.rs` | `tracing` subscriber | `SystemService::Core` | `/kernel/services/log` |
| Cross-G3 | `TimerService` | new `timer.rs` | `A2ARouter`, `CancellationToken` | `SystemService::Core` | `/kernel/services/timer` |

### Boot Sequence (08b additions)

```
Existing boot (K0-K6):
  1-12. [unchanged]

08b additions (when os-patterns enabled):
  13. MetricsRegistry        (Cross-G1)
  14. LogService             (Cross-G2)
  15. DeadLetterQueue        (K2-G1, needs A2ARouter)
  16. ReliableQueue          (K2-G2, needs A2ARouter + DLQ)
  17. NamedPipeRegistry      (K2-G3, needs A2ARouter)
  19. TimerService           (Cross-G3, needs A2ARouter)
```

Note: boot position 18 is ReconciliationController (08a). Positions 20-22
are 08c services.

---

## R -- Refinement

### Performance Considerations

- `MetricsRegistry` uses `AtomicU64`/`AtomicI64` -- no lock contention on
  hot path (counter increment = single atomic add)
- `Histogram` uses fixed buckets with atomic counters -- O(buckets) per
  record, no dynamic allocation
- `LogService` ring buffer: `RwLock<VecDeque>` -- many readers, one writer.
  Write lock held only during push/evict, not during subscriber fan-out
- `ReliableQueue` retries use `tokio::time::sleep()`, not busy-wait
- `DeadLetterQueue`: `RwLock<VecDeque>` with FIFO eviction, bounded memory
- Named pipe capacity enforced via bounded `mpsc::channel` -- backpressure
  automatic
- Timer service: one `tokio::spawn` per timer, cleaned up on cancel or fire

### Security Considerations

- Named pipes respect `IpcScope` capabilities -- `Restricted` agents cannot
  create or connect to arbitrary pipes
- `DeadLetterQueue` governance-gated: `IpcScope::Full` required to read DLQ
  contents (prevents information leakage from failed messages)
- `MetricsRegistry` read-only for non-kernel agents
- Trace IDs do not contain sensitive information (UUID v4 only)
- Log service respects capability scoping -- agents can only query their own
  logs unless `IpcScope::Full`

### Migration Path

- `KernelMessage` gains `trace_id` with `#[serde(default)]` -- backward
  compatible deserialization
- `KernelSignal` gains new variants -- exhaustive matches need updating
  (compile-time enforcement, not runtime surprise)
- Existing `A2ARouter::send()` callers unchanged; DLQ integration is internal

### Testing Strategy

- Each gap is independently testable
- Integration test flows:
  - Message undeliverable -> K2-G1 DLQ -> retry -> delivery or re-DLQ
  - Reliable send -> timeout -> retry -> ack or max-retries -> DLQ
  - Named pipe create -> send -> restart agent -> reconnect -> receive
  - Trace ID propagation through multi-hop message chain
- `ReliableQueue` tested via `MockClock` from K6
- All tests independent of `mesh` feature (single-node)

### Dependency Minimization

- No new external dependencies -- all code uses workspace-existing crates:
  `dashmap`, `tokio`, `chrono`, `serde`, `uuid`
- `os-patterns` feature does not pull networking dependencies

---

## C -- Completion

### Exit Criteria

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

#### Phase Gate
- [ ] All K2 exit criteria pass
- [ ] All cross-cutting exit criteria pass
- [ ] All existing tests pass (843+ baseline)
- [ ] New tests: target 80+ for this phase
- [ ] Clippy clean for all new code (`scripts/build.sh clippy`)
- [ ] Feature gated behind `os-patterns`
- [ ] No new external dependencies

### Testing Verification Commands

```bash
# Build with OS patterns feature
scripts/build.sh native --features os-patterns

# Run IPC + observability tests
scripts/build.sh test -- --features os-patterns -p clawft-kernel

# Verify base build unchanged
scripts/build.sh check

# Clippy
scripts/build.sh clippy
```

### Implementation Order

```
Week 17 (parallel with 08a):
  K2-G5: Expanded KernelSignal vocabulary (needed by 08a)
  Cross-G1: Metrics collection (no dependency on 08a)

Week 18 (after K1-G1 restart strategies done):
  K2-G1: Dead letter queue
  K2-G4: Distributed trace IDs

Week 19:
  K2-G2: Reliable message delivery

Week 20:
  K2-G3: Named pipes

Week 21:
  Cross-G2: Structured log service + rotation
  Cross-G3: Timer service

Week 22:
  Integration testing across all 08b gaps
  Review and polish
```
