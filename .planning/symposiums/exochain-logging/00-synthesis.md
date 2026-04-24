# ExoChain Logging Redesign — Symposium Synthesis

**Authors:** four-specialist synthesis (exochain / kernel / format-research / governance)
**Date:** 2026-04-22
**Status:** proposed — needs user call on three open questions before implementation
**Siblings:** `01-exochain-specialist.md` · `02-kernel-specialist.md` · `03-format-research.md` · `04-governance-specialist.md`

---

## TL;DR

1. **Split the chain.** Four hash-linked shards: `Governance` / `Fabric` / `Stream` / `Diag`. Governance stays canonical, dual-signed, verified-on-boot, on RVF. The other three drain the high-rate chatter off the main bus and cross-anchor via BLAKE3 digests at shard rotation.
2. **Envelope v2.** Add `{cluster, node, service, agent, channel, k_level, severity, correlation_id, labels}` as first-class fields bound into the chain hash. Old envelopes infer synthesized K-level at read-time via a static lookup table (migration Option B).
3. **`k_level` and `severity` are orthogonal.** `k_level` is privilege/impact (K0 raw-data → K5 sovereign decisions); `severity` is `Debug/Info/Warn/Error/Fatal`. A Fatal shader crash (K3, Fatal) is strictly less consequential than a Warn peer-key-rotation (K5, Warn). Ship both.
4. **Storage.** Governance chain = RVF subtype 0x02 uncompressed, dual-signed. Fabric / Stream / Diag = new **WELF** format (Arrow IPC streaming + dict-encoded tag columns + delta-of-delta timestamps + trained-zstd dict on `body` column + BLAKE3 footer-chain). Hot-tier in WELF; cold tier transcodes to Parquet for DuckDB/Polars audit replay.
5. **Stream lifecycle.** New `StreamLifecycle` service at boot. Seven-state FSM `New → Connecting → Active → IdleWarning(30s) → Disabled(60s) → Disconnecting → Closed`, plus terminal `Failed` off `Connecting`. No auto-resume from `Disabled` — re-subscribe required (flap protection).
6. **Manifest cadence.** Central `ManifestFlusher`, fixed minute-aligned boundaries with per-stream jitter `hash(id) mod 500ms`. `O(streams)` once per minute. Runs through the new generalized `RollingWindowAnchor` (see §6) so we stop producing one SHAKE-256 per `peer.envelope` / `reconciler.tick` / `hnsw.eml.observe`.
7. **Gate = observe, not block.** Errors (K3) pass through a new `GovernanceGate::observe()` that feeds a rate counter into the scorer and emits `governance.anomaly` (K5) on bursts. It **never denies the log write**. Quarantine goes through the reconciler desired-state, not through gate denial.

---

## 1. Where the four specialists converged

All four independently recommended:

- Separate network chatter from the governance bus (every doc).
- Hash-anchor the separated stream back to the canonical chain (01 + 03).
- A first-class tag schema on the envelope (01 + 02 + 04).
- `k_level` as a privilege/impact dimension distinct from severity (04, accepted by the rest).
- Window-anchoring as the primary noise-suppression lever (01 + 02).
- The 0.6.19 `StreamWindowAnchor` generalizes into `RollingWindowAnchor` (01 + 02).

Where they diverged:

| Axis | 01 exochain | 02 kernel | 03 format | 04 governance | Resolution |
|---|---|---|---|---|---|
| # of shards | 4 (Gov/Fabric/Stream/Diag) | not in scope | any ≥ 2 works | 2 minimum (Gov / Non-Gov) | **4** — matches the manifest cadence classes in 02 |
| Container for non-gov | RVF subtype 0x42 | not in scope | WELF (Arrow IPC++) | not in scope | **WELF**, because 03 showed RVF runtime hard-codes `compression: 0` and SCF-1 is QR-sized |
| `k_level` as gate-policy input? | flagged open | n/a | n/a | observe-only | **Observe-only v1**; revisit after 6 weeks of production traces |
| Migration | new envelope + header version | n/a | n/a | infer at read-time via lookup | **Both** — Envelope v2 + static inference table for v1 events |
| `Fatal` variant | implicit | called out | n/a | missing from `LogLevel` | **Add `Fatal` to `LogLevel`** |

---

## 2. Architecture (one picture)

```
                ┌────────────── Producers ──────────────┐
                │  agents · services · sensors · mesh   │
                └───────────────────┬───────────────────┘
                                    │  LogContext (task-local)
                                    ▼
                 ┌──────────── LogRouter ────────────┐
                 │  classify by {channel, k_level}   │
                 └──┬──────────┬──────────┬──────────┘
                    │          │          │
       ┌────────────┴──┐  ┌────┴────┐  ┌──┴──────────┐
       │   Governance  │  │ Fabric  │  │   Stream    │  ┌─────┐
       │   ChainMgr    │  │ ChainMgr│  │  ChainMgr   │  │ Diag│
       │   RVF 0x02    │  │   WELF  │  │    WELF     │  │ WELF│
       │   dual-signed │  │         │  │ + anchor    │  │     │
       └───────┬───────┘  └────┬────┘  └─────┬───────┘  └──┬──┘
               │                │             │             │
               │                │  BLAKE3     │  BLAKE3     │  BLAKE3
               │◀───────────────┼─────────────┼─────────────┘
               │                │             │
               │                └─────────────┴──── cross-anchor
               │                  every shard rotation
               │
               └──── verified-on-boot; canonical audit root
```

**Rule of thumb.** Anything that changes *what the system is permitted to do* lands on Governance. Anything that reports *what the system is doing* lands on Fabric / Stream / Diag with a BLAKE3 anchor back.

---

## 3. Envelope v2

Bound into the hash input (breaking change → `ChainEventHeader { version: 2, .. }`):

```rust
pub struct ChainEventV2 {
    // v1 fields (retained)
    pub sequence:     u64,
    pub chain_id:     u32,
    pub timestamp:    DateTime<Utc>,
    pub prev_hash:    [u8; 32],
    pub hash:         [u8; 32],
    pub payload_hash: [u8; 32],
    pub kind:         &'static str,   // dict-coded
    pub payload:      Option<serde_json::Value>,

    // v2 additions (all hash-bound)
    pub origin:         Origin,           // {cluster, node, service, agent?}
    pub channel:        Channel,          // Governance | Fabric | Stream | Diag
    pub k_level:        u8,               // 0..=5
    pub severity:       Severity,         // Debug..=Fatal
    pub correlation_id: Option<Uuid>,     // cross-event trace
    pub labels:         BTreeMap<String, String>, // optional escape hatch; size-capped
}
```

**K-level assignments** (proposed defaults; revisit after trace data):

| K | Examples | Channel |
|---|---|---|
| **K0** | raw frame payloads (not chained — opaque to governance) | — |
| **K1** | `peer.envelope`, `stream.manifest`, heartbeats, `reconciler.tick` | Fabric |
| **K2** | `stream.connect`, `stream.disconnect`, `service.state`, health transitions | Stream |
| **K3** | `*.error`, `sandbox.execute`, `wasm.fs.*`, `hnsw.insert` | Diag (errors) / Governance (side-effect-bearing) |
| **K4** | `container.start/stop/configure`, credential rotations | Governance |
| **K5** | cluster.peer.add/remove, policy change, key rotation, chain genesis, quorum changes | Governance |

The full assignment table is in `04-governance-specialist.md` §taxonomy and `01-exochain-specialist.md` §1.2.

---

## 4. Channels

| Channel | Event rate | Durability | Container | Anchor |
|---|---|---|---|---|
| **Governance** | ~10⁰–10¹/s | per-event fsync, dual-sig | RVF 0x02 (existing) | self — verified on boot |
| **Fabric** | ~10²–10⁴/s | group-commit, ≤1s loss | WELF shard (Arrow+zstd-dict) | BLAKE3 footer → Governance at rotation |
| **Stream** | ~10³–10⁵/s | group-commit, ≤1s loss | WELF shard | BLAKE3 footer → Governance at rotation + per-(topic, window) anchor |
| **Diag** | ~10¹–10³/s | group-commit, ≤1s loss | WELF shard | BLAKE3 footer → Governance at rotation |

**Why four, not two.** `Fabric` is interconnect chatter the reconciler cares about (peer adds, mesh envelopes, heartbeat jitter). `Stream` is data-plane activity the user wants to tail live. `Diag` is error tail + debug logging — higher retention, heavier compression, almost never queried in real time. Merging Fabric + Stream makes it hard to rate-limit one without starving the other; merging Stream + Diag makes live tailing noisy.

---

## 5. WELF — WeftOS Event Log Format

From `03-format-research.md` §3. Summary:

- **Arrow IPC streaming** as the record-batch wire format (append-friendly, columnar, crash-safe frame framing).
- **Dict-encoded columns:** `cluster`, `node`, `service`, `agent`, `kind`. Dict grows append-only; shard rotation reseeds.
- **Delta-of-delta timestamps** (Gorilla-style) on `timestamp` column. Typical ratio: 15-40× on monotonic streams.
- **Trained zstd dict on `body` column** — train once per day from the previous day's corpus, ship the dict inline at shard head so the shard is self-describing.
- **Per-frame fsync-group-commit** — 10ms coalescing window. Loss bound: ≤1s.
- **BLAKE3 footer-chain across shards** — every shard's header links the previous shard's BLAKE3. Anchor fires on rotation (not per-event) and emits one `chain.shard_anchor` event onto Governance.
- **Cold-tier transcode to Parquet** (ClickHouse codec recipe: `Delta, LZ4HC` for ints, `ZSTD(3)` for strings) for DuckDB/Polars replay.

**What we are explicitly NOT doing:**

- Not extending RVF runtime. `rvf-runtime-0.2.0` hard-codes `compression: 0` and the only compressor shipped is SCF-1 (1.4×–2.5× LZ77, 4 KB window, QR-seed-sized). Extending it would mean re-implementing Arrow-equivalent columnar infrastructure inside RVF from scratch.
- Not using OTLP / Loki / Grafana as the primary store. They're fine as downstream subscribers via a WELF → OTLP bridge, but we don't want an ops stack on the critical path.
- Not using journald — Linux-only and assumes a global `/run/log/journal` convention we don't want to take a dependency on.

---

## 6. `RollingWindowAnchor` (generalization of `StreamWindowAnchor`)

The 0.6.19 `StreamWindowAnchor` auditor at `crates/clawft-kernel/src/stream_anchor.rs` anchors exactly `stream.window_commit`. Generalize it so any high-rate kind can get window-anchored instead of one-event-per-observation:

```rust
pub struct RollingWindowAnchor {
    pub kind_pattern: Glob,         // e.g. "peer.envelope", "reconciler.tick.*"
    pub window: Duration,           // 1s / 10s / 60s
    pub aggregator: Box<dyn WindowAggregator>,  // count, hash, percentile, …
    pub chain: Arc<ChainManager>,   // typically Fabric
    pub anchor_kind: &'static str,  // e.g. "fabric.window_commit"
}
```

**Immediate callers to migrate** (all land on Fabric after the cutover):

| Current (v1) | Frequency | New anchor |
|---|---|---|
| `peer.envelope` (1 per inbound mesh IPC frame) | ~10²–10⁴/s | `fabric.envelope.window` — count + BLAKE3 over body hashes + per-peer first/last seq |
| `reconciler.tick` (1 per tick) | ~1/s, bursty | `fabric.reconciler.window` — tick count + action count + failures |
| `hnsw.eml.observe` (1 per EML observation) | ~10¹–10²/s | `fabric.hnsw.window` — count + residual statistics |

This is *independent of* the channel split — even if we kept one chain, these three producers need window-anchoring. But it composes: window-anchoring + channel-split is how we keep Governance readable.

---

## 7. Stream lifecycle (the user's specific ask)

From `02-kernel-specialist.md` §2. Seven-state FSM:

```
   ┌──────┐    ┌────────────┐    ┌────────┐
   │ New  │───▶│ Connecting │───▶│ Active │──┐
   └──────┘    └─────┬──────┘    └────────┘  │
                     │                  ▲    │
                     ▼                  │    ▼
                  ┌────────┐         ┌──┴─────────┐
                  │ Failed │         │ IdleWarning│ (30s of silence)
                  └────────┘         └──┬─────────┘
                                        │
                                        ▼
                                    ┌──────────┐
                                    │ Disabled │ (60s of silence)
                                    └────┬─────┘
                                         │ (re-subscribe required — no auto-resume)
                                         ▼
                                    ┌───────────────┐
                                    │ Disconnecting │
                                    └───────┬───────┘
                                            ▼
                                       ┌────────┐
                                       │ Closed │
                                       └────────┘
```

**Events emitted on each transition** — all land on the `Stream` channel:

| Transition | Event kind | K | Severity |
|---|---|---|---|
| → Connecting | `stream.connect.begin` | K2 | Info |
| Connecting → Active | `stream.connect.established` | K2 | Info |
| Connecting → Failed | `stream.connect.failed` | K3 | Error |
| → IdleWarning | `stream.idle_warn` | K2 | Warn |
| → Disabled | `stream.idle_disable` | K2 | Warn |
| any → Failed | `stream.failed` | K3 | Error / Fatal |
| → Disconnecting | `stream.disconnect.begin` | K2 | Info |
| → Closed | `stream.disconnect.closed` | K2 | Info |
| 1-minute tick while Active | `stream.manifest` | K1 | Info |

**The user's explicit constraint:** if idle → Disabled transitions, the `stream.idle_disable` event fires **before** the next scheduled `stream.manifest`, and the manifest still fires once more with final window stats.

**Implementation home.** New `StreamLifecycle` service registered in `boot.rs`, owning `Arc<StreamRegistry>`. Hooks:
- `TopicRouter::publish` taps `StreamRegistry::record_frame(topic_id)` — **this is the kernel specialist's flagged hot-path risk**. Mitigation: atomic counter only, no lock, no allocation per frame.
- `SubstrateService::publish` + `MeshRuntime::handle_envelope` similarly tap.
- `ManifestFlusher` task: `tokio::time::interval_at(minute_start + jitter)` — one timer for the whole service, not per-stream.

---

## 8. Governance gate = observe, not block

From `04-governance-specialist.md` §gate-interaction. New API:

```rust
impl GovernanceGate {
    /// Rate-observes the event, updates the scorer, emits
    /// `governance.anomaly` (K5, Warn) on burst. Never denies.
    pub fn observe(&self, event: &ChainEventV2) { .. }
}
```

**Policy:**
- K0–K2 events: no gate interaction. Log-through.
- K3 events: `observe()` only. Never blocked.
- K4–K5 events: `check()` as today. May be denied.

**Why.** The cost of losing a Fatal error event (so we can't post-mortem a crash) is strictly worse than the cost of logging one. The gate's job on K3 is to *notice anomalies*, not to police the log write.

**Anomaly detector.** Rolling-window counter per `(service, kind)`. Threshold trained by `GovernanceScorerModel` (from the 0.6.19 EML-swap wiring). Fires `governance.anomaly` when z-score > N (default 3.0). That event lands on Governance at K5.

**`Fatal` variant.** Add to `LogLevel` (`console.rs:58`) — currently missing. Fatal events also trip `RestartStrategyModel` directly (bypassing any observation rate-limit).

---

## 9. Migration

From `04-governance-specialist.md` §migration. **Option B**: version the envelope, infer K-level at read-time via a static lookup table for v1 events.

- `ChainEventHeader { version: 2 }` for new writes.
- `chain.rs::read_event()` switches on version. v1 events get `k_level` + `severity` filled from a static lookup, `channel` synthesized from `kind` prefix, `origin` defaulted to `(local_cluster, local_node, "legacy", None)`.
- The lookup table lives at `crates/clawft-types/src/chain/k_level_legacy.rs` — generated from the 91 `EVENT_KIND_*` constants audit in `01-exochain-specialist.md` §1.2.
- No rewrite of old chain data. Readers can upgrade at their own pace.

---

## 10. Implementation plan (phased)

**Phase 1 — Envelope + lookup table** (1 week, low risk)
- Add `ChainEventHeader` versioning; keep writing v1 but start reading v1+v2.
- Land the static `k_level_legacy.rs` lookup table.
- No behavior change yet — just the plumbing.

**Phase 2 — Generalize `StreamWindowAnchor` → `RollingWindowAnchor`** (1 week, medium risk)
- Refactor `stream_anchor.rs` into a pluggable `WindowAggregator` trait.
- Port the three loudest producers (`peer.envelope`, `reconciler.tick`, `hnsw.eml.observe`) to window anchors.
- **Expected reduction:** ~95% of chain events, measured on a 1-hour mesh bench.

**Phase 3 — Channel split (4 shards)** (2 weeks, medium risk)
- New `ChainManager` instances for Fabric / Stream / Diag.
- `LogRouter` routes by `channel` field (fallback = Governance on missing channel, for v1 back-compat).
- Cross-anchor hook: on shard rotation emit `chain.shard_anchor` onto Governance.
- v1 readers continue reading the Governance chain alone and see roughly the same audit surface as before.

**Phase 4 — WELF hot-tier** (3 weeks, higher risk — new format)
- New `clawft-welf` crate. Arrow IPC streaming + dict columns + delta-delta ts + trained-zstd dict.
- Fabric / Stream / Diag `ChainManager` writes to WELF instead of RVF.
- Governance stays on RVF.
- Parquet transcode as a background cold-tier job (retention-driven).

**Phase 5 — Stream lifecycle + manifest** (2 weeks, medium risk)
- `StreamLifecycle` service, `StreamRegistry`, seven-state FSM.
- `ManifestFlusher` with minute-aligned cadence and jitter.
- Hot-path tap in `TopicRouter::publish` / `SubstrateService::publish` / `MeshRuntime::handle_envelope`.
- Idle watchdog @ 30s / 60s.
- Full event vocabulary from §7 wired up.

**Phase 6 — Gate observe-only** (1 week, low risk — policy only)
- `GovernanceGate::observe()` API.
- Rolling anomaly detector wired to `GovernanceScorerModel`.
- Add `Fatal` to `LogLevel`; wire to `RestartStrategyModel`.

**Phase 7 — Cold-tier + query UX** (2 weeks, green-field)
- WELF → Parquet transcode on retention.
- DuckDB / Polars query helpers (`weaver chain query --channel stream --where "kind = 'stream.manifest' and service = 'audio'"`).

**Total: 12 weeks of focused work.** Phases 1–3 are the backbone; 4–7 can parallelize across teams.

---

## 11. Open questions the user has to call

1. **Is `k_level` a routing tag only, or a governance-policy input?** (Flagged by 01.) v1 = routing tag only (observe-only). Revisit after 6 weeks of production traces. Needed now: a one-line "observe-only v1" confirmation.
confirmed

2. **Four shards or three?** (Fabric / Stream / Diag vs. Fabric+Diag merged.) 01 recommends four; 04 would accept three. Four is the default in this synthesis; collapse to three if disk budget is tight.
four. User Error instead of diag. Keep 4

3. **WELF or raw Arrow IPC?** (03's recommendation vs. runner-up.) WELF is ~2 weeks extra work for ~2× the ratio. Raw Arrow IPC is the safer v1; promote to WELF in v2 with forward-compatible shards. Default in this synthesis: **Raw Arrow IPC in Phase 4, WELF in Phase 4b.**
arrow is good, we can evaluate welf later

4. **Retention policy.** Unspecified. Proposal: Governance = forever; Fabric = 30 days hot / 1 year cold; Stream = 7 days hot / 90 days cold; Diag = 14 days hot / 180 days cold. Each retention class is a separate rotation policy on the `ChainManager`.
Good

5. **`Fatal` semantics.** Does a Fatal event auto-quarantine via reconciler, or just increment a counter? 04 says "trip `RestartStrategyModel`". Confirm.
Yeah fatal should be a restart. There probably should be a backoff.
---

## 12. Artifacts to produce

- **ADR-0001:** ExoChain Channel Separation — (authoritative version of §2 + §4)
- **ADR-0002:** ChainEvent Envelope v2 + Legacy K-level Inference — (§3 + §9)
- **ADR-0003:** WELF Storage Format — (§5)
- **ADR-0004:** Stream Lifecycle + Manifest Protocol — (§7)
- **ADR-0005:** Governance Observe-Only Policy — (§8)

Each ADR is ~1 page, linked from `docs/adrs/index.md`. Target: have all five drafted in a single pass before Phase 1 lands.
