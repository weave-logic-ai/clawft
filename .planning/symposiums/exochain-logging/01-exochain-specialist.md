# ExoChain Logging Redesign — Chain-Level Findings

**Author:** exochain-specialist
**Date:** 2026-04-22
**Status:** design / pre-symposium
**Scope:** the append-only chain envelope, its event vocabulary, RVF
persistence, and the interaction between governance-bearing events and
high-rate network chatter. Out of scope: network transport, GUI,
telemetry dashboards — those are for the other specialists.

---

## 1. Current state

### 1.1 ChainEvent envelope

File: `crates/clawft-kernel/src/chain.rs:141-163`

```rust
pub struct ChainEvent {
    pub sequence:    u64,
    pub chain_id:    u32,
    pub timestamp:   DateTime<Utc>,
    pub prev_hash:   [u8; 32],
    pub hash:        [u8; 32],
    pub payload_hash:[u8; 32],
    pub source:      String,              // free-form subsystem name
    pub kind:        String,              // dotted verb, e.g. "agent.spawn"
    pub payload:     Option<serde_json::Value>,
}
```

Everything the network cares about today — cluster, node, service,
agent, severity, importance — is either absent or stuffed into the
free-form `source` string and the JSON `payload`. There is no first-
class notion of "where did this happen?" or "how much does this
matter?" on the envelope itself. Consumers that want to filter
chatter must deserialize every payload and dig.

The hash input (`chain.rs:224-244`) is
`shake256(seq ‖ chain_id ‖ prev_hash ‖ source ‖ 0x00 ‖ kind ‖ 0x00 ‖ ts ‖ payload_hash)`.
**Any new envelope field we want cryptographically bound has to be
added to that hash input**, which is a breaking change to the chain
hash scheme. That means a versioned header (see §3.3).

### 1.2 Event vocabulary (91 `EVENT_KIND_*` constants)

Grepped from `crates/clawft-kernel/src/chain.rs:254-703`. Grouped by
intended K-level of the underlying subsystem:

| Group | Kinds | Representative K-level | Governance-bearing? |
|-------|-------|-----------------------|---------------------|
| `chain.*` | `chain.genesis`, checkpoints (implicit) | K0 | n/a — always chained |
| `capability.*` | `capability.revoked`, `capability.elevate` | K1 | yes |
| `auth.*` (5) | `auth.credential.register/rotate`, `auth.token.issue/revoke`, `auth.attempt` | K1 | yes |
| `config.*` (3) | `config.set`, `config.delete`, `config.secret.set` | K1 | yes |
| `agent.*` | `agent.hierarchy.add_child`, `agent.hierarchy.remove_child` | K1 | yes |
| `process.*` (3) | `process.register`, `process.deregister`, `process.state` | K1 | mostly — state may churn |
| `app.*` (5) | `app.install/remove/start/stop/transition` | K5 | yes |
| `cron.*` (3) | `cron.add/remove/execute` | K1 | `execute` is high-rate |
| `container.*` (3) | `container.start/stop/configure` | K4 | yes |
| `wasm.fs.*` (5) | `wasm.fs.write/remove/create_dir/copy/move`, `wasm.execute` | K3 | yes, high-rate under agents |
| `tool.*` | `tool.register`, `tool.deploy`, `tool.signed`, `tool.version.revoke` | K3 | yes |
| `service.contract.*` | `service.contract.register` (D8) | K2 | yes |
| `sandbox.*` | `sandbox.execute`, `sandbox.sudo.override` | K3 | yes |
| `shell.*` | `shell.exec` | K3 | yes |
| `session.*` | `session.create/destroy` | K2 | yes |
| `workspace.*` | `workspace.create/config` | K5 | yes |
| `env.*` (3) | `env.register/switch/remove` | K6 | yes |
| `cluster.peer.*` (3) | `cluster.peer.add/remove/state` (`chain.rs:432-444`) | K6 | `state` is high-rate! |
| `mesh.*` (7) | `mesh.service.register/deregister`, `mesh.artifact.store/fetch`, `mesh.ipc.send`, `mesh.peer.add/remove` | K6 | `mesh.ipc.send` is very high-rate |
| `peer.envelope` | emitted by `mesh_runtime.rs:308` for every inbound mesh IPC frame | K6 | **NOT governance — pure fabric** |
| `stream.window_commit` | 1-per-window anchor for streaming topics (`stream_anchor.rs:196`) | K6 | audit anchor, not governance |
| `kernel.*` | `kernel.save/load` | K0 | yes |
| `reconciler.*` (4) | action, tick, desired.set/remove | K1 | `tick` is high-rate! |
| `causal.*` (5) | node/edge add/remove, clear | K3 | yes |
| `artifact.*` | `artifact.store/remove` | K5 | yes |
| `graphify.*` (4) | build, ingest, pipeline, hook | K5 | yes |
| `project.*`, `profile.*` (4) | init, create/delete/switch, vector.insert | K5 | yes |
| `hnsw.*` (4) | `hnsw.insert/clear/save/load` | K3 | `insert` is high-rate |
| `eml.*` (4) + `hnsw.eml.*` (4) | trained, drift, saved, loaded, observe, recall, trained, triage | K3 | `observe/recall` are extremely high-rate |

Observations:

- **Rate mismatch.** `mesh.ipc.send`, `peer.envelope`, `reconciler.tick`,
  `hnsw.insert`, `hnsw.eml.observe`, `hnsw.eml.recall` are all on the
  same chain as `governance.deny` and `capability.revoked`. One
  substrate frame = one SHAKE-256 hash + one `Mutex<LocalChain>`
  acquisition + one event allocation. The governance-critical events
  get drowned in sensor-frame log spam.
- **No location.** `cluster.peer.add` carries the peer ID in the JSON
  payload, but the *originating* node ID is nowhere. When we merge
  chains from two nodes via `ChainBridgeEvent` (`mesh_chain.rs:38-48`)
  we can no longer tell which node observed which event without
  reading the bridge-anchor timestamps.
- **No severity.** `auth.attempt` covers both "admin logged in
  successfully" and "brute-force attack blocked", distinguishable only
  by payload inspection.
- **Ad-hoc sources.** `"kernel"`, `"mesh"`, `"cluster"`, `"service.cron"`,
  `"supervisor"`, `"governance"`, `"ipc"`, `"stream"` — seven different
  spellings. No registry; drift guaranteed.

### 1.3 StreamWindowAnchor (the 0.6.19 prototype)

File: `crates/clawft-kernel/src/stream_anchor.rs` (284 lines).

Already half of the answer. Per-topic rolling BLAKE3 hash + message/
byte counters. Every `window` (configurable duration) the accumulator
is flushed into a single `stream.window_commit` chain event
(`stream_anchor.rs:196`) whose payload is:

```json
{
  "topic": "sensor.mic",
  "window_start_ms": 1714..., "window_end_ms": 1714...,
  "first_msg_ms": ..., "last_msg_ms": ...,
  "sample_count": 240, "byte_count": 61440, "chunk_count": 240,
  "blake3": "<64-char hex>"
}
```

This is the blueprint the user is asking us to generalize: high-rate
data stays off the chain; a tamper-evident summary goes on. Today it
only applies to data topics explicitly plumbed through the a2a router
(`stream_anchor.rs:110-136`). It does not cover mesh peer envelopes,
reconciler ticks, HNSW inserts, or EML observations — they still emit
one chain event each.

### 1.4 Mesh chain bridge

File: `crates/clawft-kernel/src/mesh_chain.rs`.

Already declares `SyncStreamType { Control, Chain, Tree, Causal, Hnsw,
CrossRef, Impulse, Ipc }` with per-stream priorities (`mesh_chain.rs:201-212`).
This is the quiet proof the system already *wants* separate lanes —
but only at the QUIC-sync level, not in the chain itself. The chain
that gets replicated over `SyncStreamType::Chain` is still the single
monolithic append-only log.

### 1.5 Persistence

File: `crates/clawft-kernel/src/chain.rs:1326-1438` (`save_to_rvf`).

Per-event layout:

```
+------------------+------------------------+----------------+
| RVF seg header   | ExoChainHeader (64B)   | CBOR payload   |
| (32B)            | magic / seq / prev_... | source/kind/.. |
+------------------+------------------------+----------------+
(padded to 64B)

... (one segment per event) ...

+------------------+------------------------+----------------+
| Checkpoint seg   | ExoChainHeader 0x41    | checkpoint CBOR|
+------------------+------------------------+----------------+
| Ed25519 footer (optional)                                  |
| ML-DSA-65 footer (optional, dual-signing)                  |
+------------------------------------------------------------+
```

One segment per event. With `mesh.ipc.send` on a busy fabric this is
a non-trivial storage cost (~200 B + padding per envelope plus the
SHAKE-256 round-trip). `RvfChainPayload` (`chain.rs:827-837`) is a
CBOR map with `source`, `kind`, `payload` (opaque JSON), and the two
hex-encoded hashes. There is no dictionary coding, no delta
compression, no shared segment schema — each envelope carries full
`source` and `kind` strings.

`rvf-runtime` / `rvf-wire` are external crates pulled in as optional
deps (`clawft-kernel/Cargo.toml:87-90`). I could not locate a batched
or columnar writer in this workspace; `save_to_rvf` iterates
event-by-event. Any batched path is a new build-out, not a reuse.

### 1.6 Governance gate coupling

Governance decisions are chained via `GovernanceDecisionEvent`
(`chain.rs:2312-2355`) through `ChainLoggable`. Gate checks happen
**before** the chain append for every gated operation
(e.g. `cluster.rs:558-628` for `cluster.peer.add`). This means:

- A deny produces a `governance.deny` event but *no* corresponding
  `cluster.peer.add`.
- A permit produces both a `governance.permit` and the gated event —
  two chain entries per user-visible action.

Today the gate only runs on events that hit `GateBackend::check()`.
`peer.envelope`, `reconciler.tick`, `stream.window_commit`, and the
HNSW/EML observation events all bypass the gate. This is correct —
they are fabric/telemetry, not policy. But it does mean the chain
already has two classes of events even if we don't label them.

---

## 2. Proposed tag/category system on the envelope

### 2.1 Fields (typed struct, not a `HashMap`)

The user's list is almost right; the one addition is `chain_version`
so we can migrate the hash-input scheme without breaking old RVF
files. Recommended shape:

```rust
pub struct ChainEventV2 {
    // --- unchanged chain-link fields ---
    pub sequence:     u64,
    pub chain_id:     u32,
    pub timestamp:    DateTime<Utc>,
    pub prev_hash:    [u8; 32],
    pub hash:         [u8; 32],
    pub payload_hash: [u8; 32],

    // --- new envelope header ---
    pub env_version:  u16,      // bump to 2
    pub origin:       Origin,   // typed location tag (cluster/node/service/agent)
    pub channel:      Channel,  // Governance | Fabric | Stream | Diag | Lineage
    pub k_level:      u8,       // 0..=7, mapped per §2.2
    pub severity:     Severity, // Debug | Info | Notice | Warn | Error | Critical
    pub kind:         KindCode, // 32-bit interned code OR 2-level enum

    // --- optional labels (freeform, not hashed) ---
    pub labels:       SmallVec<[(LabelKey, LabelVal); 4]>,

    // --- payload unchanged, but now optional at the chain level ---
    pub payload:      Option<serde_json::Value>,
}

pub struct Origin {
    pub cluster_id: [u8; 16], // UUID/ULID, zeroed in single-node mode
    pub node_id:    [u8; 16],
    pub service:    ServiceCode, // interned
    pub agent_id:   Option<[u8; 16]>, // None for kernel-originated
}
```

Why a typed struct instead of `HashMap<String, String>`:

1. Every extra string on a hot-path event is a malloc. With
   `peer.envelope` at 10k/s this is not academic.
2. The hash input must be deterministic. Canonicalizing a
   `HashMap` (sort keys, length-prefix, etc.) is possible but
   error-prone; a fixed struct is one-and-done.
3. The fields we care about today are bounded (six of them).
   Freeform labels belong in a bounded `SmallVec<[(K, V); 4]>`
   where keys are a small interned set and overflow spills to the
   payload.

Kind encoding: replace the 91 `EVENT_KIND_*` strings with a two-level
enum — `KindNamespace` (u8: governance, cluster, mesh, stream, hnsw,
eml, …) plus `KindOp` (u16 within the namespace). That is 3 bytes
instead of ~22 avg bytes for strings like `"cluster.peer.state"`,
and the chain hash input shortens proportionally. Keep a
`Display`/`FromStr` so the existing CLI/JSON surface is unchanged.

### 2.2 K-level mapping

The current effect algebra (`governance.rs:117-201`) is an
`EffectVector { risk, fairness, privacy, novelty, security }` — five
continuous dimensions, **not** the K0..K7 integer ladder. The
K-ladder lives in documentation
(`docs/weftos/feature-flags.md:271-278` and
`docs/weftos/kernel-modules.md:8-287`) as a *development phase*
marker, not a runtime quantity.

**Proposal: introduce a runtime `k_level: u8` field whose definition
mirrors the K-phase ladder.** This is the user's intent. Concrete
mapping:

| k_level | Name           | Examples today                                                   | Target channel |
|---------|----------------|------------------------------------------------------------------|----------------|
| 0       | Foundation     | `chain.genesis`, `kernel.save/load`                              | Governance     |
| 1       | Supervision    | `auth.*`, `config.*`, `process.register`, `agent.hierarchy.*`    | Governance     |
| 2       | IPC            | `ipc.dead_letter`, `session.create/destroy`, `service.contract.*` | Governance     |
| 3       | Tooling / ECC  | `tool.*`, `sandbox.*`, `shell.exec`, `wasm.*`, `causal.*`, `hnsw.*`, `eml.*` | Mixed (see below) |
| 4       | Containers     | `container.*`                                                    | Governance     |
| 5       | Apps           | `app.*`, `workspace.*`, `graphify.*`, `artifact.*`               | Governance     |
| 6       | Mesh / Fabric  | `cluster.peer.*`, `mesh.*`, `env.*`, `peer.envelope`, `stream.*` | **Fabric / Stream** |
| 7       | Cognitive      | `eml.drift`, future reasoning events                             | Diag           |

Heartbeats — `cluster.peer.state`, `reconciler.tick` — are
*telemetry at K6*, not governance. They route to Fabric.
Streaming frame anchors (`stream.window_commit`) are Stream.
`hnsw.eml.observe`/`recall` are Diag because they are
sub-ms-rate and purely observational.

**K3 splits between channels.** `tool.exec` is governance-bearing
(capability enforcement). `hnsw.insert` during a training loop is
not. Rule of thumb: if the gate could have denied it, it's
Governance; if it merely records that something happened, it's Fabric
or Diag. The per-kind routing table lives in a
`kind_catalog.rs`-style registry (see §3.4).

### 2.3 Severity

Six levels, mapped to classic syslog minus two:

```
Debug    — development only, off in prod
Info     — normal operation (default)
Notice   — noteworthy but not actionable (peer joined, window commit)
Warn     — possibly bad (rate-limited request, retry, stale peer)
Error    — definitely bad but recoverable (ipc dead-letter, cron miss)
Critical — governance deny, auth breach, chain fork
```

Severity is *independent* of K-level. A K6 `peer.envelope` is
`Info`, a K6 `peer.envelope` that fails signature verification is
`Error`. A K1 `auth.attempt` can be `Info` (success) or `Warn`
(failure) or `Critical` (brute-force threshold).

### 2.4 Backward compatibility for the hash

Because the hash input changes, we cannot preserve SHAKE-256 values
across the v1→v2 migration. Two options:

1. Bump `env_version` in the `ExoChainHeader` (`chain.rs:65-76`),
   include it in the hash input, write a **migration checkpoint**
   that anchors the last v1 hash as the `prev_hash` of the first v2
   event, and ship a `chain.migration` event recording the transition.
   Old RVF files load read-only; new files are v2-only.
2. Write-side dual-hash for a transition period: hash both v1 and
   v2 schemas, persist both hashes side-by-side, let consumers pick.
   Expensive; do not recommend.

Go with option 1. The `weaver chain` CLI already has `export` and
`verify` subcommands — add `weaver chain migrate --from-v1 --to-v2`.

---

## 3. Channel architecture — A / B / C trade-off

### 3.1 Option A — single chain, required `channel` field

Pros: one file, one hash chain, one signing key, one signature footer.
Governance auditor never has to cross-reference two stores. Migration
is small (just add the field to the envelope).

Cons: **the chain still grows at the rate of the loudest producer.**
SHAKE-256 on every `peer.envelope` is ~1 µs of CPU and one mutex
acquisition; at 10k envelopes/s that's 10 ms/s of wall time on the
chain mutex. Worse, RVF writes are synchronous and per-event, so
fabric chatter pins the disk. `verify_integrity()`
(`ChainVerifyResult`, `chain.rs:181-193`) has to walk every event
including the chatter, which makes boot-time verification O(chatter)
instead of O(governance).

### 3.2 Option B — `ChainManager` per channel

Proposal: four `ChainManager` instances, each with its own RVF shard,
its own signing key (or shared Ed25519 + channel-scoped subkey — see
symposium D11 dual signing), its own in-memory event vector:

| Channel       | Purpose                                   | Checkpoint cadence | Verify on boot? |
|---------------|-------------------------------------------|--------------------|-----------------|
| `governance`  | Gate decisions, lineage, capability, auth | every 100 events   | **yes**, full walk |
| `fabric`      | Mesh peer state, reconciler ticks, cluster | every 10k events   | hash-only spot check |
| `stream`      | `stream.window_commit` per topic          | every 1k events    | hash-only spot check |
| `diag`        | HNSW/EML observations, verbose debug      | every 100k events  | no, truncate on restart |

Pros:

- Governance chain stays small and hot — full verification remains
  feasible on every boot.
- Fabric/stream shards can use a different storage strategy
  (batched writes, zstd compression, periodic rotation).
- Blast radius: a corrupted fabric shard does not invalidate the
  governance chain.
- Rate-limit enforcement per-channel is trivial (separate queue).

Cons:

- Four cryptographic roots. Cross-channel correlation requires a
  **channel-anchor event**: every N fabric writes, the fabric chain
  emits its current head hash, and a `fabric.anchor` event on the
  governance chain records it (exactly the pattern already used by
  `ChainBridgeEvent`, `mesh_chain.rs:37-48`). This is the best of
  both worlds — still tamper-evident, but the expensive verification
  surface is bounded.
- Migration effort: ~40 call sites that call `chain.append(...)`
  need to route to the right channel. Mechanical but touches every
  subsystem.
- Lineage across channels: if a governance event (K5 app.start)
  *causes* 1000 fabric events (peer envelopes), we lose the direct
  causal link unless we carry an optional `caused_by: (channel, seq)`
  label. Easy to add.

### 3.3 Option C — governance chain canonical, sidecar for the rest

The sidecar is a **journald-style ring** or column-oriented file with
periodic hash anchors onto the governance chain. Effectively Option B
but asymmetric: only the governance chain is a proper hash-linked
exochain; the sidecar is a compact log-structured store (think
`tracing` with structure-preserving writes).

Pros:

- Cheapest option — the sidecar skips per-event SHAKE-256 entirely,
  only hashing at flush time (say, every 1s or every 1MiB).
- Reuses the existing `StreamWindowAnchor` pattern at full
  generality — the sidecar *is* a giant anchor buffer.
- Matches how log aggregators (journald, Vector, OpenTelemetry
  collectors) expect to consume network chatter.

Cons:

- Breaks the "every observable kernel event is chained" invariant
  that K2:D9 "universal witness by default" depends on.
- Fabric writers cannot replay their own history as hash-linked
  events; they can only read a timestamped ordered log plus the
  periodic anchors.
- Two storage formats to maintain.

### 3.4 My recommendation: **Option B with a C-style escape hatch**

- **Default:** Option B. Four hash-linked `ChainManager`s with
  channel anchors (`fabric.anchor`, `stream.anchor`, `diag.anchor`)
  crossing onto `governance`.
- **Escape hatch for the loudest producers:** inside the fabric and
  stream channels, the `StreamWindowAnchor` pattern is used to batch
  very-high-rate events (`peer.envelope`, `hnsw.eml.observe`) into
  N-second rolling windows. The per-event record goes to the
  sidecar ring; the window summary is the actual chain event.
  Configurable per (`channel`, `kind`) pair.

This keeps the cryptographic story clean (everything is hash-linked;
you just pick the granularity), makes the governance chain
verification-cheap, and gives network/streaming their own write
budget. It's structurally compatible with the
`SyncStreamType { Control, Chain, Tree, Causal, Hnsw, Ipc, … }` split
that `mesh_chain.rs:171-183` already defined for QUIC replication,
so mesh sync naturally fans out to per-channel streams.

A registry crate (`crates/clawft-kernel/src/kind_catalog.rs`, new)
owns the static mapping:

```rust
pub struct KindSpec {
    pub kind:            KindCode,
    pub name:            &'static str,
    pub channel:         Channel,
    pub default_k_level: u8,
    pub default_severity:Severity,
    pub expected_rate:   RateClass,  // OneOff | Occasional | Steady | HighRate
    pub anchor_strategy: AnchorStrategy, // Direct | RollingWindow { ms } | Sampled { 1_in_n }
}
```

At kernel boot, every event kind is registered with its channel and
anchoring strategy. `ChainManager::append` becomes
`ChainRouter::log(KindCode, Origin, Severity, payload)` and routes to
the right backend. The existing `ChainLoggable` trait
(`chain.rs:2243-2252`) gets a `fn kind_spec(&self) -> KindSpec;`
method and we're done.

---

## 4. Format recommendation

### 4.1 Keep RVF as the container

RVF already gives us: per-segment content hash, Ed25519 + ML-DSA-65
dual signing (`chain.rs:1409-1427`), known-format header, existing
`weaver chain verify`. Do not invent a new container for governance.

### 4.2 Introduce a **batched columnar segment subtype** for fabric/stream

RVF supports per-segment `subtype` (`chain.rs:1343` uses 0x40 for
Event, 0x41 for Checkpoint). Claim `0x42` for `ExochainBatch`:

```
ExoChainHeader {
    magic:        EXOCHAIN_MAGIC,
    version:      2,
    subtype:      0x42,  // ExochainBatch
    flags:        BATCH_ZSTD | BATCH_DICT_V1,
    chain_id:     u32,
    sequence:     u64,   // first seq in batch
    timestamp:    u64,   // batch start
    prev_hash:    [u8;32],
}
```

Payload (after the 64-byte header):

```
BatchHeader {
    event_count: u32,
    columns:     ColumnMask (bitset over fields),
    dict_id:     u16,  // pre-shared zstd dictionary id (0 = none)
}

// Column-major layout — each column zstd-compressed with shared dict.
ColumnBlock { kind:   repeat of KindCode, u24 varint-packed }
ColumnBlock { origin: dedup'd against column dict }
ColumnBlock { severity, k_level }
ColumnBlock { timestamp_delta_delta } // ts_0 + delta-of-delta u16s
ColumnBlock { payload_hash [32 * N] }
ColumnBlock { payload_cbor }  // only for events that have payloads
```

- Delta-of-delta on timestamps: `peer.envelope` at steady-state
  produces near-constant inter-arrival; DoD compresses to 2 bits/event.
- Pre-shared dictionary per channel+kind: kind names, source nodes,
  and common label keys are dictionary-coded once, not per-event.
- Per-event `hash` is derivable from the batch segment content hash
  plus event index — we don't need to store it explicitly. Consumers
  that want the per-event hash recompute it on read.

This gives us ~30-50x storage compression on `peer.envelope`-heavy
workloads vs. today's one-segment-per-event format, without inventing
a new top-level file format.

### 4.3 Governance channel stays uncompressed

Governance events are rare, latency-sensitive on verify, and the
highest-value audit artifact. Keep `subtype: 0x40` (one segment per
event), no batching, full Ed25519 + ML-DSA-65 footers as today.

### 4.4 No reusable batched path exists today

I scanned `clawft-kernel/Cargo.toml:87-90` and the import list at
`chain.rs:33-43`. The `rvf-runtime` / `rvf-wire` / `rvf-crypto` /
`rvf-types` crates are external workspace deps; they expose
`write_segment`, `read_segment`, `validate_segment`, and the
signing primitives, but nothing batched or columnar. §4.2 above is
net-new. Candidate crate name: `weftos-rvf-batched` or a new module
inside `weftos-rvf-wire`.

---

## 5. Lifecycle events for streaming nodes

User's requested five events, mapped to the new schema:

| Event                | `k_level` | `channel` | `severity` | `kind`                  | Gate? |
|----------------------|-----------|-----------|------------|-------------------------|-------|
| Connect              | 6         | Fabric    | Notice     | `stream.connect`        | **yes** (one-shot, governance can deny a new streamer) |
| 1-min perf manifest  | 6         | Stream    | Info       | `stream.manifest`       | no    |
| Error                | 6         | Stream    | Error      | `stream.error`          | no    |
| Disconnect           | 6         | Fabric    | Notice     | `stream.disconnect`     | no    |
| Idle self-disable    | 6         | Fabric    | Warn       | `stream.idle_disable`   | no    |

### 5.1 Connect: the one gated event

A new streamer joining gets exactly one gate check, at the moment
the peer presents credentials and a declared manifest (codec,
expected rate, topics). Outcomes:

- `Permit` → emit `stream.connect` on Fabric with an `allowed_until`
  deadline label. The streamer proceeds.
- `Deny` → emit `governance.deny` on Governance, no `stream.connect`.
- `PermitWithWarning` → `stream.connect` on Fabric + a
  `governance.warn` on Governance referencing the same sequence via
  a `caused_by` label.

The gate check uses the existing `GateBackend::check()`
(`cluster.rs:561-572` pattern). After this one check, no further gate
round-trips on the streaming hot path.

### 5.2 Manifest: rolling window, already half-built

The `stream.manifest` is the `StreamWindowAnchor` generalized. At a
1-minute cadence:

- Aggregate BLAKE3 over all frames in the window (already done by
  `stream_anchor.rs:60-68`).
- Compute p50 / p95 / p99 inter-arrival and byte-rate.
- Counters: frames, bytes, drops, reorders.
- Emit one `stream.manifest` on the **Stream** channel.

The individual frames are **never** chained. They go onto the
substrate bus only. The chain sees one manifest event per (topic,
minute). This is the core of the redesign.

### 5.3 Error: direct, but rate-limited

`stream.error` goes directly onto the Stream channel (one chain
event per error). Rate-limit at source: no more than N errors/window
reach the chain; the rest are counted and folded into the next
`stream.manifest` as `dropped_errors: u32`.

### 5.4 Idle watchdog + self-disable

The user specified: 60s with no frames → emit `stream.idle_disable`
**before** the next scheduled `stream.manifest`. Implementation
sketch inside `StreamWindowAnchor::run_consumer`
(`stream_anchor.rs:139-185`):

```rust
// In the select! loop, add an idle timer = 60s - window
_ = idle_timer.tick() => {
    if current.is_empty() && !self.idle_disabled {
        emit_chain_event(Channel::Fabric, Severity::Warn,
            KindCode::StreamIdleDisable,
            json!({ "topic": topic, "idle_since_ms": ... }));
        self.idle_disabled = true;
        unsubscribe_from_router();
    }
}
```

The `stream.idle_disable` preempts the next manifest because once
fired it tears down the consumer subscription. When the streamer
reconnects, it goes through `stream.connect` again (full gate
check).

### 5.5 Does any of this need governance approval mid-stream?

No — and that is the whole point of the sidecar/channel split.
Governance authorizes the streamer at connect time (`stream.connect`)
and that decision is durable. Mid-stream, the gate sees nothing. If
policy changes require revocation, that's a **new** governance event
(`capability.revoked` on Governance) that the streamer observes via
its own capability-check path, not via a per-frame gate.

---

## 6. Open questions for the other specialists

### For the network/mesh specialist

1. **QUIC priority vs. Channel.** `mesh_chain.rs:201-212` already
   defines per-stream QUIC priorities. Should `Channel::Governance`
   map 1:1 onto `SyncStreamType::Chain` (priority 1) and
   `Channel::Fabric` onto a lower-priority stream? Or does mesh
   sync want to keep its own axis (chain replication vs. sensor
   flood) orthogonal to the logging channels?
2. **Cross-node origin.** My `Origin { cluster_id, node_id, ... }`
   assumes a 128-bit ULID for cluster and node. Is this consistent
   with the mesh's existing node-identity scheme? `cluster.rs`
   uses `pub type NodeId = String` (`cluster.rs:23`) today.
3. **Anchor propagation.** When node A's fabric chain emits an
   anchor onto node A's governance chain, and node B replicates
   A's chain, does B need to re-verify A's fabric anchor? Or is
   the per-node governance signature sufficient?

### For the governance specialist

4. **Is `k_level` a policy input?** Should governance rules be able
   to say "deny any K6 event from agent X"? Or is k_level a pure
   routing/audit tag?
5. **Severity and gate decisions.** When the gate returns
   `PermitWithWarning`, what severity should the resulting chain
   event have — `Notice` or `Warn`? The current
   `GovernanceDecisionEvent` (`chain.rs:2329-2355`) flattens this
   to a single event kind string.
6. **Channel for `governance.defer` (EscalateToHuman).** Does that
   go on Governance (yes, probably — it's a policy outcome) or on a
   dedicated Human-in-the-loop channel?

### For the observability / weaver specialist

7. **CLI surface.** `weaver chain tail --channel=fabric --severity=warn+`
   — is this the right UX? Or do we want a single stream with
   server-side filter, like `journalctl`?
8. **Retention.** Governance: forever. Fabric: rotate at 7d? 30d?
   Configurable. Stream: probably rotate at 24h — the manifests are
   enough. Diag: rotate at 1h. Who decides the defaults?
9. **GUI heatmap integration.** The existing `ui://heatmap`
   primitive from commit `613b58a` could consume `stream.manifest`
   directly. Should the chain expose a materialized view, or do we
   let the GUI pull from the Stream channel live?

### For the kernel/boot specialist

10. **Feature gating.** Today `exochain` is one feature flag. Should
    we gate per-channel? `exochain-governance` (required),
    `exochain-fabric`, `exochain-stream`, `exochain-diag` (all
    optional)? Embedded/edge nodes may want Governance-only.
11. **Migration tool.** Who owns `weaver chain migrate`? The kernel
    has `save_to_rvf`/`load_from_rvf`; the weaver adds CLI surface.
12. **Default for a fresh 0.7.0 node.** Four channels enabled by
    default, or single-chain-with-channel-field (Option A) as a
    transition, and Option B becomes the 0.8.0 default?

---

## 7. Summary of proposed changes

1. **Envelope v2** with typed `Origin`, `Channel`, `k_level: u8`,
   `Severity` fields; all cryptographically bound via SHAKE-256;
   `env_version: u16` bumped to 2; one-time migration checkpoint.
2. **Four `ChainManager` instances** (Governance, Fabric, Stream,
   Diag) with distinct checkpoint cadences and distinct RVF shard
   files. Cross-channel anchors carry tamper-evidence onto the
   governance chain.
3. **`kind_catalog.rs` registry** mapping every event kind to
   `{ channel, default_k_level, default_severity, anchor_strategy,
   rate_class }`. Replaces the 91 string constants with a typed
   enum; string display preserved.
4. **RVF subtype `0x42` — `ExochainBatch`** columnar + zstd-with-
   pre-shared-dictionary for Fabric/Stream/Diag channels. Governance
   stays one-segment-per-event.
5. **`StreamWindowAnchor` generalized** into a first-class
   `RollingWindowAnchor` driven by the `AnchorStrategy` from the
   kind catalog. Applied to `peer.envelope`, `reconciler.tick`,
   `hnsw.eml.observe`, and the streaming lifecycle.
6. **Streaming lifecycle**: `stream.connect` (gated, Fabric),
   `stream.manifest` (1-min rolling, Stream), `stream.error`
   (rate-limited direct, Stream), `stream.disconnect` (Fabric),
   `stream.idle_disable` (60s watchdog, Fabric, fires before next
   manifest).

This lands incrementally on top of the 0.6.19 `StreamWindowAnchor`
work without throwing it away — the anchor becomes the kernel of a
generalized pattern rather than a one-off for audio/sensor topics.
