# ExoChain Logging Redesign — Project Plan

**Parent:** `00-synthesis.md` (read that first for the full design)
**Created:** 2026-04-22
**Status:** not started — design approved, awaiting Phase 1 kickoff
**Estimated total effort:** 12 engineering-weeks, 7 phases
**Target landing window:** TBD (carry-forward; pick up whenever this project surfaces)

> This is the "come-back-to-later" document. Start here if you're picking this up after a gap. Every section has a **RESUME →** pointer telling you what to do next.

---

## 0. Where we left off

**Decisions locked by the user (2026-04-22 synthesis review):**

| # | Question | Answer |
|---|---|---|
| 1 | `k_level` as gate-policy input? | **Observe-only v1.** Revisit after 6 weeks of production traces. |
| 2 | Four shards or three? | **Four.** User renamed `Diag` → **`Error`**. Keep 4: `Governance / Fabric / Stream / Error`. |
| 3 | WELF or raw Arrow IPC? | **Raw Arrow IPC first (Phase 4).** WELF deferred to Phase 4b, evaluated on real trace data. |
| 4 | Retention policy? | **Approved:** Gov = forever · Fabric = 30d hot / 1y cold · Stream = 7d hot / 90d cold · Error = 14d hot / 180d cold. |
| 5 | Fatal → auto-restart? | **Yes, with backoff.** Fatal events trip `RestartStrategyModel` with exponential backoff (details in ADR-0005). |

**Nothing has been implemented yet.** All five ADRs are unwritten. The existing `StreamWindowAnchor` (0.6.19) is the only piece of scaffolding in place.

**RESUME →** Start with Phase 0 (ADR drafting). Do not skip ADRs — they're the contract that keeps the 7 phases coherent across a long timeline.

---

## 1. Milestones

```
┌─────────────────────────────────────────────────────────────────┐
│ M0   ADRs drafted & approved             ┐                      │
│ M1   Envelope v2 read-path shipping      ├─ Backbone (weeks 1-4)│
│ M2   RollingWindowAnchor live            │                      │
│ M3   4-channel split live                ┘                      │
├─────────────────────────────────────────────────────────────────┤
│ M4   Arrow IPC hot-tier live             ┐                      │
│ M5   Stream lifecycle + manifest live    ├─ Data-plane (5-9)    │
│ M6   Gate observe-only + Fatal restart   ┘                      │
├─────────────────────────────────────────────────────────────────┤
│ M7   Cold-tier Parquet + query UX        ┐                      │
│ M8   6-week trace review → revisit K     ├─ Wrap (10-12+)       │
│ M9   WELF vs. Arrow IPC decision (4b)    ┘                      │
└─────────────────────────────────────────────────────────────────┘
```

**Critical path:** M0 → M1 → M3 → M5. Everything else can slip or parallelize. M5 is the one the user personally asked for (connect / manifest / error / idle_disable / disconnect lifecycle events).

---

## 2. Phase breakdown (ticket-ready)

Each phase lists: entry criteria, deliverables (as potential `br` beads), exit criteria, dependencies, risks, rollback plan.

### Phase 0 — ADRs (3 days, scribe work)

**Entry:** Synthesis signed off (done).
**Deliverables:**
- `docs/adrs/0001-exochain-channel-separation.md` — authoritative §2 + §4 of synthesis.
- `docs/adrs/0002-chainevent-envelope-v2.md` — §3 + §9.
- `docs/adrs/0003-arrow-ipc-hot-tier.md` — §5 with the WELF-deferred note.
- `docs/adrs/0004-stream-lifecycle-manifest.md` — §7.
- `docs/adrs/0005-governance-observe-only.md` — §8 + Fatal-with-backoff.
- `docs/adrs/index.md` — link all five.

**Exit:** All five ADRs merged to master via PR.
**Rollback:** None needed (docs only).

### Phase 1 — Envelope v2 read-path (1 week, low risk)

**Entry:** ADR-0002 merged.
**Deliverables (beads):**
- `P1.1` — Add `ChainEventHeader { version: u8, .. }` to `crates/clawft-kernel/src/chain.rs`. Keep writing v1.
- `P1.2` — Add `ChainEventV2` type with new fields (`origin`, `channel`, `k_level`, `severity`, `correlation_id`, `labels`) behind `chain::v2` module. Not yet wired into writes.
- `P1.3` — Add `Severity` enum with `Fatal` variant to `crates/clawft-kernel/src/console.rs` alongside the existing `LogLevel`. Update `BootEvent` + `KernelEventLog::filter_level` to accept `Fatal`.
- `P1.4` — Generate `crates/clawft-types/src/chain/k_level_legacy.rs` — static lookup table from the 91 `EVENT_KIND_*` constants (audit in `01-exochain-specialist.md` §1.2).
- `P1.5` — `chain.rs::read_event()` switches on `version`. v1 events synthesize `{k_level, severity, channel, origin}` from the lookup + `kind` prefix.
- `P1.6` — Property tests: any v1 event reads back with correctly-inferred tags; round-trip of v2 events preserves all fields; hash-binding includes the new fields.

**Exit:** v2-aware readers parse v1 and v2 identically; existing chain files load unchanged; no write-path change yet.
**Dependencies:** None.
**Risks:** Envelope struct churn during 91-event audit — budget 2 extra days for taxonomy drift.
**Rollback:** Revert the module; readers fall back to v1-only parsing.

### Phase 2 — RollingWindowAnchor (1 week, medium risk)

**Entry:** Phase 1 merged.
**Deliverables:**
- `P2.1` — Refactor `crates/clawft-kernel/src/stream_anchor.rs` into a `WindowAggregator` trait + three impls (`CountAggregator`, `BlakeRollupAggregator`, `PercentileAggregator`).
- `P2.2` — Introduce `RollingWindowAnchor { kind_pattern, window, aggregator, chain, anchor_kind }`. Existing `StreamWindowAnchor` becomes a thin wrapper.
- `P2.3` — Port `peer.envelope` (`mesh_runtime.rs:308`) to `fabric.envelope.window` — count + BLAKE3 body-hash rollup + per-peer first/last seq.
- `P2.4` — Port `reconciler.tick` to `fabric.reconciler.window` — tick count + action count + failure count.
- `P2.5` — Port `hnsw.eml.observe` to `fabric.hnsw.window` — observation count + residual stats.
- `P2.6` — 1-hour mesh bench under simulated load: measure per-event chain append rate before and after. **Target: ≥90% event-rate reduction.**

**Exit:** Bench shows ≥90% reduction; `weaver chain local --kind peer.envelope*` returns window events, not per-envelope events.
**Dependencies:** Phase 1 (for `channel` field on anchor events).
**Risks:** Window-aggregator semantic drift — consumers reading `peer.envelope` directly must migrate to `fabric.envelope.window` or the per-envelope body hashes via the rollup. Audit all existing consumers before landing.
**Rollback:** Flip a config flag; fall back to per-event emission.

### Phase 3 — 4-channel split (2 weeks, medium risk)

**Entry:** Phase 2 merged.
**Deliverables:**
- `P3.1` — Instantiate 3 additional `ChainManager` at boot: `Fabric` / `Stream` / `Error`. Each has its own checkpoint file + RVF shard.
- `P3.2` — `LogRouter` routes events by `channel` field. Fallback: missing channel → Governance (v1 back-compat).
- `P3.3` — Shard rotation hook: on rotation, emit `chain.shard_anchor { channel, shard_hash, seq_range }` onto Governance.
- `P3.4` — `weaver chain local --channel <name>` — per-channel tail + query.
- `P3.5` — Retention policy config: `[chain.retention.<channel>] hot = "7d", cold = "90d"`. Governance = none (forever).
- `P3.6` — Cross-channel integration test: spawn workload, watch all 4 channels, verify cross-anchors chain back to Governance.

**Exit:** All new events route to the correct channel; `weaver chain local` without `--channel` defaults to Governance (old behavior preserved).
**Dependencies:** Phase 1, Phase 2.
**Risks:**
- Boot-time I/O — 4 `ChainManager` instances × verification-on-boot could slow startup. Mitigation: verify Governance synchronously, defer Fabric/Stream/Error to a background task.
- Disk budget — 4 shards × retention. Confirm the projected 216 GB/day raw → 8–14 GB/day compressed still fits the target host profile.

**Rollback:** Config flag `chain.channels = ["governance"]` — collapses back to single-chain behavior.

### Phase 4 — Arrow IPC hot-tier (2 weeks, higher risk)

**Entry:** Phase 3 merged + 1 week of production burn-in on RVF multi-shard.
**Deliverables:**
- `P4.1` — New `crates/clawft-arrow-log` crate. Arrow IPC streaming writer with dict-encoded tag columns.
- `P4.2` — `ArrowChainManager` — drop-in replacement for `ChainManager` that writes to Arrow IPC shards instead of RVF.
- `P4.3` — Switch Fabric / Stream / Error to `ArrowChainManager`. Governance stays on RVF.
- `P4.4` — Shard header includes: schema, dict tables, BLAKE3 of prior shard, writer metadata.
- `P4.5` — Group-commit writer: 10ms coalescing window, fsync on close. Durability bound: ≤1s loss.
- `P4.6` — Compression ratio bench on a 24-hour production trace. **Target: ≥15× on Fabric, ≥10× on Stream, ≥20× on Error.**
- `P4.7` — Crash-recovery test: kill `-9` during active write; verify last-complete-frame recoverability.

**Exit:** Compression targets hit; crash-recovery test green; `weaver chain local --channel fabric` works on Arrow shards.
**Dependencies:** Phase 3 + burn-in.
**Risks:**
- `arrow-rs` dep footprint — measure binary size impact. Mitigation: feature-gate the Arrow writer.
- Schema evolution — lock the Arrow schema version in the shard header; add a schema-registry test.

**Rollback:** Flip `chain.format.<channel> = "rvf"` — writes go back to RVF shards.

### Phase 4b — WELF evaluation (deferred, 2 weeks if green-lit)

**Entry:** Phase 4 merged + 2 weeks of production data + user go-ahead on the re-evaluation (Open Question 3 of the synthesis says "evaluate WELF later").
**Deliverables:** TBD — pick up from `03-format-research.md` §3 if the compression numbers from Phase 4 aren't good enough. Skip if raw Arrow IPC meets targets.

### Phase 5 — Stream lifecycle + manifest (2 weeks, medium risk) — **user's headline ask**

**Entry:** Phase 3 merged (channels available). Phase 4 not strictly required but preferred for compression headroom.
**Deliverables:**
- `P5.1` — `crates/clawft-kernel/src/stream_lifecycle.rs` — 7-state FSM (`New / Connecting / Active / IdleWarning / Disabled / Disconnecting / Closed / Failed`).
- `P5.2` — `StreamRegistry` — concurrent map of `StreamId → StreamState`. Atomic counters only on the hot path.
- `P5.3` — `StreamLifecycle` system service, registered in `boot.rs`. Owns the registry + one `ManifestFlusher` task.
- `P5.4` — Hot-path taps:
  - `TopicRouter::publish` → `StreamRegistry::record_frame(topic_id)` (atomic increment, no alloc).
  - `SubstrateService::publish` → same.
  - `MeshRuntime::handle_envelope` → same.
- `P5.5` — `ManifestFlusher`: `tokio::time::interval_at(minute_start + hash(id) mod 500ms)`. One timer, not per-stream.
- `P5.6` — Idle watchdog: at each minute-tick, scan streams; emit `stream.idle_warn` at 30s silence, `stream.idle_disable` at 60s.
- `P5.7` — Full event vocabulary from synthesis §7:
  - `stream.connect.begin` / `.established` / `.failed`
  - `stream.manifest` (1/min while Active; K1)
  - `stream.idle_warn` / `stream.idle_disable`
  - `stream.failed`
  - `stream.disconnect.begin` / `.closed`
- `P5.8` — Constraint test: force a stream to idle; assert `stream.idle_disable` timestamp < final `stream.manifest` timestamp.
- `P5.9` — Hot-path benchmark on `TopicRouter::publish`. **Regression budget: ≤50ns per published frame.**

**Exit:** User-visible: `weaver chain local --channel stream --tail` shows full lifecycle for every stream.
**Dependencies:** Phase 3.
**Risks:**
- **Hot-path regression on `TopicRouter::publish`** (kernel specialist flagged this as the #1 risk). Mitigation: atomic-counter-only tap, bench gate in CI.
- Chain flood from 1000 streams × 1/min manifests. Back-pressure policy: if Stream channel queue > N, coalesce consecutive manifests into a single `stream.manifest_batch` event. Default N = 512 pending events.

**Rollback:** Disable the service at boot; streams still work, just no lifecycle events.

### Phase 6 — Gate observe-only + Fatal restart (1 week, low risk)

**Entry:** Phase 1 merged (for `Severity::Fatal`). Phase 5 not required.
**Deliverables:**
- `P6.1` — `GovernanceGate::observe(&ChainEventV2)` method. Per-`(service, kind)` rolling-window counter. Emits `governance.anomaly` (K5, Warn) when z-score > `GovernanceScorerModel.threshold()`.
- `P6.2` — Wire `observe()` at every K3 emission site. Never blocks.
- `P6.3` — `Severity::Fatal` trips `RestartStrategyModel` directly, bypassing `observe()`'s rate limit.
- `P6.4` — Exponential backoff on Fatal-triggered restart: `1s, 2s, 4s, 8s, 16s, capped at 60s`. Reset after 5 minutes of clean running. Implemented in `RestartStrategyModel`.
- `P6.5` — Tests: flood 1000 K3 errors → single `governance.anomaly`, not 1000; Fatal event restarts service with backoff.

**Exit:** K3 error storms produce one `governance.anomaly` per burst, not per event; Fatal events restart services with backoff.
**Dependencies:** Phase 1.
**Risks:** Low — observe-only is pure additive plumbing. Backoff math is the only thing with real failure modes.
**Rollback:** Config flag `governance.gate.observe = false` — disables anomaly detection.

### Phase 7 — Cold-tier + query UX (2 weeks, greenfield)

**Entry:** Phase 4 merged + at least one shard rotation boundary.
**Deliverables:**
- `P7.1` — Background transcoder: Arrow IPC shard → Parquet on retention boundary. Codec recipe: `Delta, LZ4HC` for ints, `ZSTD(3)` for strings (ClickHouse-style).
- `P7.2` — `weaver chain query --channel <name> --where "<sql-like predicate>"` — DuckDB or Polars under the hood.
- `P7.3` — Retention sweep — expire cold shards past their retention window. Emit `chain.shard_expired` on Governance.
- `P7.4` — Docs: `docs/src/content/reference/chain-query.md` with example queries per channel.

**Exit:** Operator can query last 90 days of Stream events in <5s.
**Dependencies:** Phase 4.
**Risks:** DuckDB binary size — feature-gate behind `chain-query` so the default `weft` binary stays lean.
**Rollback:** Disable query command; Parquet transcode is background and safe to turn off.

---

## 3. Dependency graph

```
 Phase 0 (ADRs)
    │
    ▼
 Phase 1 (Envelope v2)
    │
    ├──▶ Phase 2 (RollingWindowAnchor)
    │       │
    │       ▼
    │    Phase 3 (Channel split)
    │       │
    │       ├──▶ Phase 4 (Arrow IPC) ──▶ Phase 4b (WELF eval)
    │       │       │
    │       │       └──▶ Phase 7 (Cold-tier + query)
    │       │
    │       └──▶ Phase 5 (Stream lifecycle)  ← user's headline ask
    │
    └──▶ Phase 6 (Gate observe-only + Fatal restart)
```

Phase 6 can start in parallel with Phase 2 (needs only Phase 1). Phase 5 needs Phase 3 but not Phase 4.

---

## 4. Metrics / acceptance

These are the numbers to hit at each milestone. Dashboards live at `docs/src/content/observability/exochain-logging-dashboard.md` (not yet written — first deliverable of Phase 7).

| Metric | Baseline (pre-project) | Target |
|---|---|---|
| Chain append rate (Governance) | ~10³/s under load | ≤10¹/s after Phase 2+3 |
| Governance chain readability (manual eyeball) | drowns in chatter | only governance-bearing events |
| Compression ratio on Fabric/Stream/Error | 1× (uncompressed JSON) | ≥15× after Phase 4 |
| Crash durability bound | per-event fsync (safe but slow) | ≤1s loss, group-commit |
| Stream lifecycle visibility | zero (no connect/disconnect events) | full FSM visible |
| Idle stream cleanup | manual | automatic after 60s |
| K3 error observability | scattered across tracing logs | one `governance.anomaly` per burst |
| Fatal-event restart behavior | undefined | exponential backoff, reset after 5min |

---

## 5. Risks and mitigations (master list)

| # | Risk | Mitigation | Owner (TBD) |
|---|---|---|---|
| R1 | Hot-path regression in `TopicRouter::publish` from P5.4 tap | Atomic counter only; CI perf gate; benchmark at P5.9 | kernel |
| R2 | Chain flood at 1000 streams × 1 manifest/min | Coalesce into `stream.manifest_batch` when queue > 512 | kernel |
| R3 | RVF runtime's hard-coded `compression: 0` blocks any RVF-native compression | Don't extend RVF; use Arrow IPC instead (decision locked) | n/a |
| R4 | `arrow-rs` dep bloats binary size | Feature-gate `chain-arrow` | kernel |
| R5 | Schema evolution across Arrow IPC versions | Lock schema version in shard header; schema-registry test | kernel |
| R6 | 4-channel boot-time I/O slows startup | Verify Governance sync; defer Fabric/Stream/Error to background task | kernel |
| R7 | Taxonomy drift during 91-event K-level lookup table generation | Budget 2 extra days in Phase 1 for audit iteration | exochain |
| R8 | Consumers reading `peer.envelope` directly break after window-anchoring | Grep for consumers in P2.6; migrate them to `fabric.envelope.window` | kernel |
| R9 | 6-week trace data disproves the proposed K-level assignments | Revisit table after M8; K-level is schema, not hash-bound on meaning | governance |
| R10 | `governance.anomaly` rate tuning — false positives during legitimate bursts | Z-score threshold trained by `GovernanceScorerModel`; config override per `(service, kind)` | governance |

---

## 6. Open sub-decisions (for Phase 0 ADRs to pin down)

These are below the synthesis's five big questions but still need a call before the relevant phase starts:

- **P1** — Does `k_level_legacy.rs` live in `clawft-types` or `clawft-kernel`? (Types probably — consumed by readers without full kernel dep.)
- **P2** — `WindowAggregator` trait: sync fn or async? (Sync — runs inside the existing tokio task at anchor emit time.)
- **P3** — Default channel for events missing a `channel` field in legacy code: Governance (proposed) or reject the write? (Governance default preserves back-compat.)
- **P3** — File layout: `.weftos/chain/<channel>/<shard-N>.rvf`? Or `<channel>.chain` single-file with rotation-on-size?
- **P4** — Arrow IPC schema: one schema for all 4 channels (union type on payload), or per-channel schema? (Probably per-channel; payload shapes differ.)
- **P5** — Where does `StreamRegistry` live? New crate `clawft-stream-lifecycle`, or inside `clawft-kernel`? (Inside kernel for now; lift out if it grows.)
- **P5** — Manifest batch threshold (R2 mitigation): static `512` or configurable? (Config, default 512.)
- **P6** — Fatal backoff schedule: `1, 2, 4, 8, 16, 60s` (proposed) or fixed `1min` after first crash? (Exponential — proposed.)
- **P7** — DuckDB vs. Polars for query backend. (Likely DuckDB — better Parquet support, embedded SQL. Polars needs Rust DataFrame API hosted in the command.)

Each of these maps to a line in the relevant ADR.

---

## 7. Artifacts (produced + pending)

**Produced (this symposium):**
- `.planning/symposiums/exochain-logging/00-synthesis.md` — design synthesis (with user's in-line answers)
- `.planning/symposiums/exochain-logging/01-exochain-specialist.md` — chain schema findings
- `.planning/symposiums/exochain-logging/02-kernel-specialist.md` — kernel lifecycle findings
- `.planning/symposiums/exochain-logging/03-format-research.md` — format comparison
- `.planning/symposiums/exochain-logging/04-governance-specialist.md` — gate + K-level findings
- `.planning/symposiums/exochain-logging/PROJECT-PLAN.md` — this doc

**Pending (Phase 0):**
- `docs/adrs/0001-exochain-channel-separation.md`
- `docs/adrs/0002-chainevent-envelope-v2.md`
- `docs/adrs/0003-arrow-ipc-hot-tier.md`
- `docs/adrs/0004-stream-lifecycle-manifest.md`
- `docs/adrs/0005-governance-observe-only.md`
- `docs/adrs/index.md` (link above five)

**Pending (later phases):**
- `crates/clawft-arrow-log/` (Phase 4)
- `crates/clawft-kernel/src/stream_lifecycle.rs` (Phase 5)
- `docs/src/content/reference/chain-query.md` (Phase 7)
- `docs/src/content/observability/exochain-logging-dashboard.md` (Phase 7)

---

## 8. Naming decisions (locked)

- **Channel 4** is `Error`, **not `Diag`**. (User call 2026-04-22.)
- **Format** is "Arrow IPC" in v1. WELF is the upgrade slot, evaluated after Phase 4 burn-in.
- `StreamWindowAnchor` → `RollingWindowAnchor` (generalization, not rename — the old name remains as a thin wrapper until migration is complete).
- `LogRouter` is the new event router. Distinct from `TopicRouter` (message router, unrelated).
- `StreamLifecycle` is the new system service. Distinct from `StreamWindowAnchor` (which becomes a `RollingWindowAnchor` consumer).

---

## 9. How to resume

If you're picking this up after a long gap:

1. **Read the synthesis** (`00-synthesis.md`) — don't skip it. The user's in-line answers in §11 are load-bearing.
2. **Check git log for any partial phase work:**
   ```
   git log --all --oneline --grep="ADR-000[1-5]\|exochain-logging\|chain.*channel\|stream.*lifecycle\|RollingWindowAnchor"
   ```
3. **Check for started branches:**
   ```
   git branch -a | grep -iE "logging|chain-channel|stream-lifecycle|arrow-log|rolling-window"
   ```
4. **Skim the existing `stream_anchor.rs`** (`crates/clawft-kernel/src/stream_anchor.rs`) — this is the scaffolding that exists today. Phase 2 generalizes it; if it's already been modified, figure out how far Phase 2 got.
5. **Look at Phase 0 ADRs in `docs/adrs/`** — if they exist, design is frozen and you're in implementation. If not, start with Phase 0.

**Next concrete step (assuming nothing has started):** write `docs/adrs/0001-exochain-channel-separation.md` from §2 + §4 of the synthesis. ~1 page. Open a PR. That unblocks everything else.

---

## 10. Out of scope (for this project — possibly future)

- GUI for live log tailing (egui panel). Could be added in Phase 7 or as its own follow-up.
- OTLP export bridge. WELF → OTLP subscriber would let operators ship logs to Grafana Loki / Honeycomb without us running that stack.
- Multi-tenant log isolation (per-workspace log streams). Current design is single-tenant-per-kernel.
- Log search indexing (Tantivy / full-text). Parquet + DuckDB covers structured query; free-text search is a separate problem.
- Replay/time-travel debugging (replay events through a reconstructed state). Would require deterministic-replay infrastructure we don't have yet.
