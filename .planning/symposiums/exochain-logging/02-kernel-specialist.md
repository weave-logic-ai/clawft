# ExoChain Logging Symposium — Kernel Specialist Findings

**Author:** kernel subsystem specialist
**Date:** 2026-04-22
**Scope:** Node-side streaming lifecycle, emission plumbing, manifest cadence,
error taxonomy, idle watchdog, tag propagation. Pairs with the exochain-specialist's
envelope schema and multi-channel architecture.

---

## 1. Current-state audit

### 1.1 Where streaming nodes send frames today

A "streaming node" in WeftOS today is almost always one of three things:

1. **A topic publisher into `A2ARouter`** — the agent calls
   `router.send(KernelMessage { target: Topic("sensor.mic"), .. })`.
   The `TopicRouter` then fans out to subscribers which may be
   (a) in-kernel PIDs (`SubscriberSink::PidInbox`), or
   (b) external unix-socket clients (`SubscriberSink::ExternalStream`).
   See `crates/clawft-kernel/src/topic.rs:105-210`.

2. **A substrate publisher** — the agent calls `SubstrateService::publish`,
   which writes through to `Entry::value` (most-recent cache), bumps the
   per-path tick, and fans out one JSON line per subscriber via
   `SubstrateService::fanout` (`substrate_service.rs:290-311`). This is
   the channel the `ui://heatmap`, `ui://waveform`, sensor feeds, and the
   GUI tray use.

3. **A mesh-bridged envelope** — for remote-node delivery,
   `MeshRuntime::handle_envelope` unpacks a `MeshIpcEnvelope`, logs a
   `peer.envelope` chain event when a `ChainManager` is attached, and
   re-injects into the local `A2ARouter` (`mesh_runtime.rs:244-330`).

All three eventually land on the same control-plane auditor:
`StreamWindowAnchor` in `stream_anchor.rs:99-233`. That is the single
place we currently produce chain-backed stream audit artifacts, and it
emits exactly **one** event kind: `stream.window_commit`.

### 1.2 Existing chain event kinds

Grepping `cm.append(..)` and `chain.append(..)` across the kernel crate:

| Source   | Kind                    | Emitted from                                      |
| -------- | ----------------------- | ------------------------------------------------- |
| `stream` | `stream.window_commit`  | `stream_anchor.rs:196`                            |
| `mesh`   | `peer.envelope`         | `mesh_runtime.rs:308`                             |
| `ipc`    | `ipc.dead_letter`       | `dead_letter.rs:127-135`                          |
| `kernel` | `boot.*`, `cron.fire`   | `boot.rs:785`, `daemon.rs:205-216`                |
| `agent`  | `agent.watchdog_reap`   | `supervisor.rs:1122`                              |

**Gaps vs. the user's asks:**

- No `stream.connect` / `stream.disconnect` kind anywhere.
- No `stream.error` kind — errors are either
  (a) `tracing::error!` logs (ephemeral),
  (b) returned as `KernelError::Mesh(..)` / `MeshError::*` (local),
  (c) dropped into the DLQ as `ipc.dead_letter` (narrow: IPC only).
- No 1-minute performance manifest — `stream.window_commit` is the
  closest thing but (i) configured per-topic, (ii) fires only when the
  window has frames, (iii) has no latency percentiles or error counts,
  (iv) has no explicit connect/disconnect bracketing.
- No idle-detection. The anchor's consumer loop will happily emit empty
  windows only by skipping empty ones (`flush_window` early-returns at
  `stream_anchor.rs:188-189`) — there is no disable path and no event
  telling anyone that a stream has fallen silent.

### 1.3 Error surfacing today

Three parallel surfaces, none of them chain-integrated:

1. **`tracing::error!` / `tracing::warn!`** — `tracing` handlers set up in
   `daemon.rs` and tests. Goes to stderr, optionally to
   `clawft-sinks`/`clawft-logger`. 17 error-level callsites in
   `clawft-kernel`. Not audited, not replayable, not correlatable with
   chain events unless the operator manually cross-references timestamps.

2. **Return `Err(KernelError::*)`** — the typed error channel. Examples:
   `KernelError::Mesh(..)` bubbles out of
   `MeshRuntime::handle_envelope`. These errors are caller-consumed and
   get dropped on the floor if the caller doesn't log them. There is no
   global "unhandled kernel error" pipeline.

3. **`DeadLetterQueue::intake(msg, reason)`** — `dead_letter.rs`. The
   most audit-friendly path: every `intake` with `chain_manager` attached
   produces an `ipc.dead_letter` chain event
   (`dead_letter.rs:122-136`). But it's **IPC-only** — it handles
   undeliverable `KernelMessage`s, not transport-level stream errors,
   decode errors, or protocol violations.

There is also `LogService` (`log_service.rs`) which has `LogEntry`
records with `trace_id`, `source_pid`, `source_service`, structured
`fields`, and severity. It's essentially a queryable ring buffer with
subscription support — exactly the right shape to become the canonical
kernel log sink — but it's not currently populated by the stream-anchor
or mesh-runtime paths.

### 1.4 Is there a `StreamRegistry`?

**No.** There is no type tracking "which streams are currently active."
What we have:

- `TopicRouter::subscriptions: DashMap<String, Vec<(SubscriberId, SubscriberSink)>>`
  — per-topic subscriber list, but oriented to **consumers** (subscribers),
  not to **producers** (publishers). A silent publisher is invisible to
  the router.
- `SubstrateService::entries: DashMap<String, Entry>` — `Entry { value,
  tick, sensitivity, sinks }`. Holds the most-recent state plus the
  subscriber sinks. Again: describes *what has been published*, not *who
  is currently publishing*.
- `MeshRuntime::peers: DashMap<String, PeerConnection>` — peer-level,
  not stream-level. A peer can source many streams.
- `StreamWindowAnchor::start_topic` returns a `TopicAnchor` handle, which
  the daemon stuffs into `Vec<TopicAnchor>` (`daemon.rs:153,176`). This
  is the **only** per-stream handle we have, and it's a configuration
  artifact (one per operator-declared topic pattern), not a liveness
  artifact.

**Conclusion:** we need a `StreamRegistry` (or equivalent) to track
*active logical streams* with lifecycle state and last-activity
timestamps. It does not exist today.

### 1.5 Existing health/probe surface

`HealthSystem` (`health.rs:69-134`) aggregates per-service health via
`ServiceRegistry::health_all()`. Every type that implements
`SystemService` provides `async fn health_check() -> HealthStatus`.
`HealthStatus` is `Healthy` / `Degraded(msg)` / `Unhealthy(msg)` /
`Unknown`, which maps cleanly onto our future severity taxonomy.

Under the `os-patterns` feature, there is a `ProbeConfig` /
`ProbeState` / `ProbeResult` trio (`health.rs:139-306`) with
failure/success thresholds that can be learned from a
`HealthThresholdModel` (`eml_kernel.rs`). This is threshold-aware and
anti-flap — it's the right shape to drive idle-detection logic too, if
we wanted to unify.

The service-level health model is per-service and poll-based; it does
**not** observe per-stream liveness. That gap is the core of the idle
watchdog proposal below.

---

## 2. Proposed streaming lifecycle state machine

### 2.1 States

```
           +-----+        subscribe
           | New |----------------------+
           +-----+                      |
                                        v
+--------+  connect ok  +-----------+  first frame  +--------+
|Connect-|------------->|Connecting |-------------->|Active  |
| Request|              +-----------+               +--------+
+--------+                   |                          |
    ^                        | transport/handshake err  | 30s silence
    |                        v                          v
    |                  +-----------+              +--------------+
    |                  | Failed    |              | IdleWarning  |
    |                  +-----------+              +--------------+
    |                                                   |
    |                                                   | 60s silence
    |                                                   v
    |                                             +--------------+
    |                                             |  Disabled    |
    |                                             +--------------+
    |                                                   |
    |                         disconnect request        |
    |                                                   v
    |                                             +-----------------+
    |                                             | Disconnecting   |
    |                                             +-----------------+
    |                                                   |
    +-------re-subscribe---------------------------- Closed
```

States:

- `New` — registered but no transport opened yet.
- `Connecting` — transport being established (TCP handshake, mesh Noise,
  subscribe ACK). Must complete within a bounded window (e.g. 10s) or
  transitions to `Failed`.
- `Active` — receiving/producing frames normally. Tracks `last_frame_at`.
- `IdleWarning` — silence threshold crossed (default 30s). Emits an
  observable event but the stream is still considered "live" — frames
  can arrive and snap back to `Active`.
- `Disabled` — hard silence threshold crossed (default 60s). Emits
  `stream.idle_disable`. Subscriber sink is dropped. New frames will
  **not** auto-re-enable; the producer must re-subscribe (see §5.3).
- `Disconnecting` — graceful teardown in progress (flushing, final
  manifest).
- `Closed` — terminal. Emits `stream.disconnect`. Entry remains in the
  registry for N minutes for post-mortem queries, then evicted.
- `Failed` — terminal from `Connecting`. Emits `stream.error` with
  category `protocol_error` or `timeout`.

Each transition fires exactly one chain event and at least one
`LogService` entry:

| From → To                        | Chain event            | Notes                              |
| -------------------------------- | ---------------------- | ---------------------------------- |
| `New → Connecting`               | `stream.connect_begin` | Initial registration               |
| `Connecting → Active`            | `stream.connect`       | First frame or handshake complete  |
| `Connecting → Failed`            | `stream.error`         | Severity `error`                   |
| `Active → IdleWarning`           | `stream.idle_warn`     | 30s no data, severity `warn`       |
| `IdleWarning → Active`           | `stream.resume`        | Frame arrived                      |
| `IdleWarning → Disabled`         | `stream.idle_disable`  | 60s no data, severity `warn`       |
| `Active → Disconnecting`         | `stream.disconnect_begin` | Requested or upstream close     |
| `* → Closed`                     | `stream.disconnect`    | Final state                        |
| `Active` → (periodic, no state change) | `stream.manifest` | 60s rolling, carries metrics   |

### 2.2 Where the state machine lives

**Recommendation: a new `StreamLifecycle` service registered in `boot.rs`,
implementing the `SystemService` trait, owning a `StreamRegistry: Arc<DashMap<StreamId, StreamEntry>>`.**

Rationale:

- Matches the existing service pattern (`CronService`, `SubstrateService`,
  `LogService`, etc.). Gets free hooks for start/stop/health via the
  `SystemService` trait.
- Keeps the state machine out of `TopicRouter` and `SubstrateService`,
  which are already doing fanout and don't need lifecycle concerns.
- Can expose RPC endpoints (`streams.list`, `streams.inspect`,
  `streams.disable`) via the daemon without touching `TopicRouter`.
- Plays nicely with `ServiceRegistry::health_all()` — can report overall
  stream health as `Degraded { .. }` when any stream is in
  `IdleWarning`/`Failed`/`Disabled`.

Integration points:

1. **Topic router tap.** Add a hook in `TopicRouter::publish` that calls
   `StreamRegistry::record_frame(stream_id, bytes_len, now)`. Cheap —
   lock-free atomic increments on the stream entry.
2. **Substrate tap.** Same idea in `SubstrateService::publish` and
   `notify`. Each substrate path is effectively a stream.
3. **Mesh tap.** `MeshRuntime::handle_envelope` already has a chain
   hook; add a `record_frame` call next to it. Supports per-peer stream
   tracking for free (`node_id` becomes part of the `StreamId`).
4. **Anchor replacement.** The existing `StreamWindowAnchor` becomes a
   thin compatibility shim that subscribes to `stream.manifest` events
   emitted by the lifecycle service and re-emits them as
   `stream.window_commit` for back-compat (until we decide to retire
   that kind).

### 2.3 StreamId shape

```
StreamId := {
  kind: "topic" | "substrate" | "mesh_peer_stream" | "service",
  name: String,         // topic name / substrate path / peer+stream
  cluster: String,
  node: String,
  service: Option<String>,
  agent: Option<AgentId>,
}
```

Interned; hash-map key is a `u64` FNV hash of the tuple. Keeps per-frame
overhead down to a single `DashMap` lookup + a handful of atomic
operations.

---

## 3. 1-minute manifest mechanics

### 3.1 Rolling vs. fixed-boundary

**Recommendation: fixed-boundary, aligned to UTC minute-start, with a
small jitter budget.**

Arguments for fixed-boundary:

- **Cross-node alignment.** When we aggregate manifests for a cluster
  overview, fixed minute buckets make stitching trivial: `window_start ==
  floor(now / 60)`. Rolling windows force a merge step that doesn't
  scale to many nodes.
- **Chain audit.** The exochain-specialist's multi-channel architecture
  wants `stream.manifest` events to be the audit substrate. Fixed
  windows mean the manifest-chain interleaves cleanly in time.
- **Predictable cadence.** Operators pulling `weaver chain local --kind
  stream.manifest` can count events: one per stream per minute. Trivial
  gap detection.

Rolling is easier *per-stream* but costs us every downstream analytic.

**Jitter budget.** Pin each stream's flush to `minute_boundary +
hash(stream_id) mod 500ms` to smear chain writes. Keeps the chain
append pressure smooth rather than spiky at the 60-second boundary.

### 3.2 Counters in each manifest

```rust
StreamManifest {
  stream_id: StreamId,
  window_start_ms: u64,
  window_end_ms: u64,
  // Volume
  byte_count: u64,
  frame_count: u64,
  // First/last
  first_frame_ms: Option<u64>,
  last_frame_ms: Option<u64>,
  last_seq: Option<u64>,    // sequence number if producer supplies it
  // Errors
  error_count: u32,
  error_by_category: BTreeMap<ErrorCategory, u32>,
  // Latency (populated when producer records arrival + serve times)
  p50_latency_ms: Option<f64>,
  p95_latency_ms: Option<f64>,
  p99_latency_ms: Option<f64>,
  // Integrity
  window_hash: String,      // BLAKE3 over concatenated frame bytes
  // State at end-of-window
  lifecycle_state: LifecycleState,
  // Tags (replicated in envelope too, but here for payload self-description)
  tags: LogContext,
}
```

Reuse the existing `Histogram` in `metrics.rs:42-127` for latency.
Per-stream histograms are cheap because `Histogram` is already
atomic-per-bucket. Snapshot at window-close via
`Histogram::percentile(0.95)` (`metrics.rs:129-133`).

### 3.3 Who drives cadence

**Recommendation: a single `ManifestFlusher` task in the
`StreamLifecycle` service, driven by `tokio::time::interval_at` pinned
to the minute boundary, iterating all registered streams.**

Why not the `WeaverEngine` tick:

- `WeaverEngine` is a cognitive/agent scheduler, not a telemetry
  scheduler. Coupling them muddles responsibility boundaries.
- Cognitive ticks are variable frequency (they back off under load); we
  want manifest cadence to be exactly 60s regardless of load so operators
  can spot outages.

Why not a dedicated `StreamWindowCommit` per-stream task:

- 1000 streams × 1 tokio task + 1 `interval()` each = 1000 timer wakes
  per minute per node. The central flusher is O(streams) atomic reads,
  done once per minute, on a single task.
- Central flusher is also the natural place to order chain appends so
  they don't interleave weirdly with other chain writes.

Structure:

```rust
async fn flusher_loop(registry: Arc<StreamRegistry>, chain: Option<Arc<ChainManager>>) {
    let mut ticker = tokio::time::interval_at(
        next_minute_boundary_utc(),
        Duration::from_secs(60),
    );
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        for mut entry in registry.iter_mut() {
            let manifest = entry.snapshot_and_reset(now_ms);
            if let Some(cm) = &chain {
                cm.append("stream", "stream.manifest", Some(manifest.to_payload()));
            }
            log_service.ingest(
                LogEntry::new(LogLevel::Info, "stream.manifest")
                    .with_fields(manifest.to_log_fields()),
            );
        }
    }
}
```

### 3.4 Back-pressure

If the chain is backed up (the chain has its own append queue — see
`chain.rs`'s bounded channel), `cm.append` returns quickly because
today's `ChainManager` is synchronous and in-memory. But under an
exochain-specialist's proposed multi-channel architecture, the chain
channel has a real queue.

**Rule: manifest flush must NEVER block frame ingest.** The frame-ingest
hot path is `StreamRegistry::record_frame` which does three atomic ops
and returns. The flusher runs on its own task and takes a snapshot
lock per stream; that lock is only contended with other manifest ops,
not with ingest.

If the chain queue is full at manifest-emit time:

1. **Retain the counters in the stream entry** — do NOT reset. The next
   minute's manifest will accumulate on top.
2. **Emit a `stream.manifest_deferred` log entry** (not chain) with the
   reason. Operators see the back-pressure directly.
3. **Bump a metric** (`kernel.stream.manifest_deferred_count`) so the
   issue is visible in dashboards.
4. **At recovery**, emit one coalesced manifest covering the deferred
   duration, with a `coalesced_windows: N` field in the payload.

**Do NOT drop manifests silently.** The whole point of this subsystem is
audit; silent drops destroy that.

---

## 4. Error event taxonomy

### 4.1 Categories

```rust
#[non_exhaustive]
pub enum ErrorCategory {
    // Transport / I/O
    IoError,          // socket reset, disk write fail, EOF
    Timeout,          // operation exceeded deadline
    Backpressure,     // queue full, reader lagged

    // Protocol / encoding
    ProtocolError,    // handshake violation, framing error
    DecodeError,      // JSON/CBOR/binary decode failure
    InvariantViolation, // internal consistency check failed

    // Authorization / policy
    AuthFail,         // authentication failed
    GovernanceDenied, // policy gate denied
    RateLimit,        // quota exceeded

    // External
    UpstreamError,    // remote node rejected
    ResourceExhausted, // OOM, FD limit, etc.
}
```

Each category maps 1:1 onto existing error surfaces:

| Category            | Current source                                            |
| ------------------- | --------------------------------------------------------- |
| `IoError`           | `MeshError::Io`, `tokio::io::Error` anywhere              |
| `Timeout`           | `MeshError::Timeout`, `DeadLetterReason::Timeout`         |
| `Backpressure`      | `try_send` rejection in `topic.rs:210`, substrate fanout  |
| `ProtocolError`     | `MeshError::Handshake`, `MeshError::GenesisMismatch`      |
| `DecodeError`       | `MeshIpcEnvelope::from_bytes` serde errors                |
| `InvariantViolation`| `.expect(..)`/assertion failures in hot paths              |
| `AuthFail`          | auth_service denial paths                                 |
| `GovernanceDenied`  | `DeadLetterReason::GovernanceDenied`                      |
| `RateLimit`         | future (not present today)                                |
| `UpstreamError`     | remote mesh errors surfaced back                          |
| `ResourceExhausted` | process-table full, inbox full                            |

### 4.2 Severity

Map onto `LogLevel` + a new `Fatal`:

```
Debug  → informational, not fired as chain event
Info   → chain event, low volume (e.g. connect/disconnect)
Warn   → chain event, operator attention optional (idle, degraded)
Error  → chain event, operator attention required
Fatal  → chain event + trips RestartTracker + may trigger node isolation
```

`Fatal` is reserved for invariant violations, cryptographic failures,
or governance-level breaches. When fired, the kernel:

1. Emits `stream.error` with severity `fatal`.
2. Calls `RestartTracker::record(fatal=true)` which feeds the
   `RestartStrategyModel` (`eml_kernel.rs:117-128`) for backoff
   decisions.
3. Optionally notifies `ClusterMembership` to quarantine the offending
   peer.

### 4.3 Trace ID correlation

`KernelMessage` already carries `trace_id: Option<String>`
(`ipc.rs:218`), and `LogEntry` already has `trace_id`
(`log_service.rs:41`). Both plumbing exist — we just need to:

1. **Propagate trace_id into stream events.** Every `stream.*` chain
   event payload includes `trace_id` when available.
2. **Wrap the emission site in a `tracing::span!`** scoped to the same
   `trace_id`. Then any `tracing::error!` emitted while handling that
   stream event will bear the trace, making cross-referencing chain
   events and tracing logs a grep-for-uuid operation.
3. **Thread trace_id into `StreamManifest`** via the `LogContext`
   (§6 below).

---

## 5. Idle watchdog design

### 5.1 Where the watchdog lives

**Recommendation: one central reaper task inside the
`StreamLifecycle` service, NOT per-stream tasks.**

Per-stream `tokio::time::timeout` tasks cost a timer registration per
stream. At 1000 active streams that's 1000 timers. The central reaper
is O(N) per second and pays for itself after ~50 streams.

```rust
async fn reaper_loop(registry: Arc<StreamRegistry>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    loop {
        ticker.tick().await;
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        for mut entry in registry.iter_mut() {
            let idle_ms = now_ms.saturating_sub(entry.last_frame_ms);
            match entry.state {
                LifecycleState::Active if idle_ms >= 30_000 => {
                    entry.state = LifecycleState::IdleWarning;
                    entry.emit_event(EventKind::IdleWarn, ...);
                }
                LifecycleState::IdleWarning if idle_ms >= 60_000 => {
                    entry.state = LifecycleState::Disabled;
                    entry.emit_event(EventKind::IdleDisable, ...);
                    entry.disable_sink();
                }
                _ => {}
            }
        }
    }
}
```

### 5.2 Clean vs. dirty shutdown on self-disable

Ordering requested by the user: **idle_disable event BEFORE the next
manifest.** The reaper emits `stream.idle_disable` immediately on
transition to `Disabled`, then the regular minute flusher emits the
next `stream.manifest` as normal (which will carry
`lifecycle_state: Disabled` and reset counters).

Clean shutdown sequence in `entry.disable_sink()`:

1. Mark state `Disabled` (atomic).
2. **Flush outstanding counters** — snapshot current accumulation into
   a `final_manifest` payload. Don't wait for the next boundary.
3. Emit `stream.idle_disable` chain event carrying both the idle reason
   AND the partial-window counters so no data is lost.
4. Remove the `SubscriberSink` from `TopicRouter`/`SubstrateService`
   (so any lingering frames arriving late hit the dead-subscriber
   cleanup path rather than re-arming the stream).
5. Next minute flush sees the entry in `Disabled` and emits a final
   `stream.manifest` with zero counters. Two options:
   - Emit the zero manifest (helps with "I expected N per minute").
   - Skip emission and rely on chain gap-detection.
   Recommend **emit** for consistency — one manifest per stream per
   minute is a contract worth keeping.

### 5.3 Resume semantics

**Recommendation: re-subscribe required. A frame arriving on a
`Disabled` stream does NOT auto-re-enable.**

Reasons:

- Prevents "bursty-then-silent" producers from spamming reconnect
  cycles. Each re-enable costs one `stream.connect` event; we don't
  want flappers.
- Puts the re-subscribe decision on the producer, who is the only
  component that knows whether silence was expected (intentional pause)
  or a bug (hung sensor).
- On re-subscribe, a fresh `StreamId` (with a rotated instance
  suffix) is issued, giving cleaner audit trail: each subscribe
  instance is its own lifecycle.

Behavior when a frame arrives on a `Disabled` entry:

1. Log a `kernel.warn` with `{stream_id, reason: "frame-after-disable"}`.
2. Increment a metric.
3. Drop the frame. Do not re-enable, do not forward.

The subscriber then needs to call the re-subscribe RPC (e.g.
`streams.subscribe`) to start a fresh lifecycle.

### 5.4 Tunability

Make both thresholds configurable in `KernelConfig`:

```toml
[kernel.streams]
warn_idle_secs = 30
disable_idle_secs = 60
# Optionally per-stream overrides:
[[kernel.streams.override]]
match = "sensor.heartbeat.*"
warn_idle_secs = 300
disable_idle_secs = 600
```

The override list lets operators say "this low-rate sensor is expected
to be quiet for minutes at a time." Without overrides, low-rate streams
will false-positive idle-disable.

---

## 6. Tag propagation

### 6.1 Who knows what

| Field     | Source                                                             |
| --------- | ------------------------------------------------------------------ |
| `cluster` | `ClusterMembership::config.cluster_id` (not plumbed yet, add)      |
| `node`    | `ChainManager::node_id`, `ClusterIdentity::node_id` (`cluster.rs`) |
| `service` | `SystemService::name()` at registration; carried in `ServiceType` |
| `agent`   | `AgentLoop::agent_id`, `AgentSupervisor::agent_id`                  |
| `kind`    | Known at emission site (e.g. `stream.manifest`)                    |
| `severity`| Mapped from `LogLevel`/`HealthStatus`                              |

At least the node and cluster identity want to be set **once at boot
time** and stored in the `Kernel<P>` struct. `ChainManager` already
holds `node_id`; that's the right canonical home.

### 6.2 Proposed `LogContext`

```rust
#[derive(Debug, Clone, Default)]
pub struct LogContext {
    pub cluster: Option<String>,
    pub node: Option<String>,
    pub service: Option<String>,
    pub agent: Option<String>,
    pub pid: Option<Pid>,
    pub trace_id: Option<String>,
}

impl LogContext {
    pub fn current() -> Self { /* read from tokio task-local */ }
    pub fn scope<T>(self, f: impl FnOnce() -> T) -> T { /* set task-local */ }
    pub fn to_tags(&self, kind: &str, severity: LogLevel) -> BTreeMap<String, String> { ... }
}
```

**Propagation strategy: tokio task-local, seeded at three boundary
points.**

1. **Boot root.** At `Kernel::boot`, seed task-local with
   `{cluster, node}` derived from `ClusterMembership` and
   `ChainManager`. All tasks spawned via `tokio::spawn(...)` inherit
   nothing automatically, so we need to wrap the spawn.

2. **Agent loop.** `kernel_agent_loop` wraps its body in
   `LogContext { agent, pid, ..current() }.scope(async move { ... })`.
   Every emission inside the agent automatically gets tagged.

3. **Service start.** `ServiceRegistry::start_service` wraps the
   service's `start()`/`stop()`/`health_check()` calls in a scope with
   `service = s.name()`. Same for `SubstrateService` publish etc.

A `tracing::Layer` can be wired up to mirror the task-local into
`tracing` span attributes, so `tracing::info!("thing")` inside a scope
carries the tags too without any code changes.

### 6.3 Emission-time convenience

Provide a helper:

```rust
pub fn chain_append_tagged(
    cm: &ChainManager,
    source: &str,
    kind: &str,
    mut payload: serde_json::Value,
) {
    let ctx = LogContext::current();
    payload["_tags"] = serde_json::to_value(ctx.to_tags(kind, ...)).unwrap();
    cm.append(source, kind, Some(payload));
}
```

Then every existing chain append site just needs `s/cm.append/chain_append_tagged/`
and all the tags show up automatically. No hand-plumbing per callsite.

### 6.4 Envelope vs. payload

The exochain-specialist owns the envelope schema. My recommendation:

- **Envelope carries `cluster`, `node`, `kind`, `severity`** — these are
  the routing-relevant ones (multi-channel selection).
- **Payload carries `service`, `agent`, `pid`, `trace_id`** — these are
  payload-specific metadata.

This keeps the envelope small and fixed-shape while letting the payload
grow.

---

## 7. Open questions

1. **Manifest retention on-chain.** At 1000 streams × 1 manifest/minute
   = 1.44M chain events per day. Is that acceptable to `ChainManager` /
   the checkpoint cadence? May need a separate "telemetry chain"
   (exochain-specialist's multi-channel argument). Or manifest events
   could be aggregated before appending (one `stream.manifest_batch`
   event per 60s carrying all streams).

2. **Who owns the jitter clock?** If we use `floor(now/60)*60` to align
   boundaries across nodes, we depend on NTP sync accuracy. Do we need
   a node-vs-cluster clock skew tracker? `MeshClockSync` exists in
   `mesh_heartbeat.rs` — likely the right home.

3. **Does `stream.error` go to the chain unconditionally, or are
   `debug`/`info`-severity errors kept in `LogService` only?** My
   intuition: only `warn+` to the chain; debug/info errors are
   ephemeral logs. User's "emit an event on any error" suggests
   **anything classified as an error** (any `ErrorCategory`) hits the
   chain regardless of severity — but I'd like to confirm before
   committing. The DB sizing differs by ~20x between the two options.

4. **Should `StreamRegistry` be a service, or a field on
   `Kernel<P>`?** Services have lifecycle hooks (`start`/`stop`/
   `health_check`) which we want. But every emission site has to
   `Arc<ServiceRegistry>::get::<StreamLifecycleService>("stream-lifecycle")`
   which is a string lookup. For hot paths (frame ingest), a direct
   `Arc<StreamRegistry>` on `Kernel<P>` is faster. Probably: the
   service wraps an `Arc<StreamRegistry>`, and `Kernel<P>` holds a
   clone of the Arc for hot-path access — best of both worlds.

5. **Bridge with the existing `StreamWindowAnchor`?** Retire it entirely
   (emit `stream.manifest` instead), or keep `stream.window_commit` as
   an "operator-declared anchor" that is distinct from automatic
   lifecycle manifests? The tests in
   `crates/clawft-kernel/tests/stream_anchor_test.rs` will need updates
   either way.

6. **Per-stream config vs. global default for idle thresholds.**
   Low-rate sensor streams vs. high-rate audio streams want different
   thresholds. The override-list proposal in §5.4 works, but adds
   config complexity. Alternative: producers declare their expected
   cadence in a `StreamHint { min_interval, typical_interval }` when
   they subscribe, and the lifecycle service derives idle thresholds
   from the hint (e.g. `warn = 10×typical`, `disable = 20×typical`).

7. **Interaction with `DeadLetterQueue`.** If a subscriber inbox is
   full and the message is dead-lettered, that's currently an
   `ipc.dead_letter` event. Under the new taxonomy it's also a
   `stream.error { category: Backpressure }`. Two events for one
   incident is noisy. Do we:
   (a) keep DLQ as the canonical IPC event and skip `stream.error`
       when the event is already in the DLQ,
   (b) emit both and treat the DLQ event as subordinate, or
   (c) collapse DLQ into the stream taxonomy (DLQ becomes
       `stream.error.backpressure` + a queue entry)?

8. **Relationship to `HealthSystem`.** If three streams are
   `Disabled`, is the service hosting them `Healthy`, `Degraded`, or
   `Unhealthy`? Need a mapping rule. Proposal: `>0` IdleWarning → svc
   `Degraded`; `>25%` Disabled or any `Failed` → svc `Unhealthy`.

---

## 8. Implementation risk register

Ranked most-to-least scary:

| Risk                                                                   | Mitigation                                                       |
| ---------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Hot-path regression in `TopicRouter::publish` from `record_frame` tap | Benchmark before/after; target < 100ns added per frame           |
| Manifest events swamp chain throughput                                 | Aggregated `manifest_batch` fallback; telemetry channel split    |
| Idle false-positives for legitimately low-rate streams                 | `StreamHint` declared cadence; per-stream config overrides       |
| `StreamRegistry` DashMap contention at 10K streams                     | Shard by hash(StreamId) mod 16; rotate on shard imbalance        |
| `LogContext` task-local not inherited across `tokio::spawn`            | Provide `LogContext::spawn_scoped(fut)` helper; lint stock spawn |
| Back-pressure during manifest flush causes data loss                   | Defer + coalesce on chain backpressure; never silently drop      |
| Dual error surfaces (DLQ + stream.error) cause double-counting         | Pick one canonical event per incident; document subordination    |
| Trace ID plumbing gaps cause orphaned logs                             | Audit all `KernelMessage::new` callsites; add lint               |

---

## 9. File references

- Stream anchor today: `crates/clawft-kernel/src/stream_anchor.rs:99-233`
- Dead letter queue: `crates/clawft-kernel/src/dead_letter.rs:83-255`
- IPC envelope + trace_id: `crates/clawft-kernel/src/ipc.rs:192-275`
- Topic router fanout: `crates/clawft-kernel/src/topic.rs:105-260`
- Substrate service fanout: `crates/clawft-kernel/src/substrate_service.rs:225-311`
- Mesh runtime peer logging: `crates/clawft-kernel/src/mesh_runtime.rs:275-330`
- Health system + probes: `crates/clawft-kernel/src/health.rs:69-306`
- Log service (ring + trace_id): `crates/clawft-kernel/src/log_service.rs:25-100`
- Metrics + histograms: `crates/clawft-kernel/src/metrics.rs:42-355`
- Event log: `crates/clawft-kernel/src/console.rs:181-310`
- Kernel boot wiring: `crates/clawft-kernel/src/boot.rs:618-810`
- Daemon anchor setup: `crates/clawft-weave/src/daemon.rs:148-185`
- Anchor config schema: `crates/clawft-types/src/config/kernel.rs:166-297`
- Restart strategy model: `crates/clawft-kernel/src/eml_kernel.rs:117-130`
- Cluster identity: `crates/clawft-kernel/src/cluster.rs:263-320,395-560`
