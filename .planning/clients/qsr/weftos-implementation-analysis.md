# QSR on WeftOS — Implementation Analysis

**Status**: Draft 1
**Date**: 2026-04-21
**Client codename**: QSR (Quick Food Services — anonymous)
**Source brief**: `.planning/clients/qsr/INTIIAL_INFO.md`
**Primary substrate**: WeftOS ECC (`crates/clawft-kernel/src/`) + RVF (`.planning/04-rvf-integration.md`)
**Pattern reuse**: cold-case gap-analysis engine (`.planning/symposiums/cold-case-ecc/ecc-application-mapping.md`)

> **Anonymization note.** All brand, store, region, employee, franchisee, menu, promotion, vendor, and certification names in this document are **synthetic labels** (`Brand-A`..`Brand-D`, `Metro-Alpha`, `store_a_0001`, etc.). No real QSR identifiers, counterparty names, or PII appear here. Scale numbers are rounded to orders of magnitude.

---

## 0. Executive summary

QSR is a multi-brand quick-service restaurant operator running on the order of tens of thousands of restaurants across 100+ countries, with aggregate daily sales of ~$125M (~11M orders/day at ~$11 average ticket). They run four brands, referenced here as **Brand-A**, **Brand-B**, **Brand-C**, **Brand-D**. They want WeftOS to answer cross-cutting questions like *"if the stores in Metro-Alpha miss budget this week, what does quarterly revenue look like?"* on top of operational, financial, and org-chart data that already lives in their data lakes.

This is a natural ECC workload **but not a natural RVF workload at the raw-transaction layer**. The right architecture is a three-tier split:

1. **Data lake stays the source of truth for transactions** (whatever QSR already runs — Iceberg / Delta / Parquet on Snowflake / Databricks / BigQuery / similar). No attempt to replace it.
2. **ECC holds a typed causal graph of entities, relationships, and rollup summaries** with embeddings for semantic search and `lambda_2` coherence scoring. Each rollup node carries a `lake_ref` pointer back to the authoritative row-level data.
3. **Streaming ingest via the impulse queue**, not batch. The brief's "real-time, lots of little deltas" shape maps directly onto DEMOCRITUS (`democritus.rs`) — it was built for exactly this. Each 5-minute store micro-rollup is one `Impulse`.

Two of QSR's three asks (scenario reasoning, org-chart gap analysis) reuse existing kernel primitives almost verbatim from the cold-case work. The third (multi-year ingest throughput at full-scale streaming) is the engineering push and needs a test harness before contractual commitment.

---

## 1. Workload decomposition

The brief bundles three distinct workloads. Separating them drives every downstream decision.

| Workload | Volume | Latency | Access pattern | Substrate |
| --- | --- | --- | --- | --- |
| **W1. Streaming operational ingest** | ~500 orders/sec global average (peak ~2K), ~30K stores × 288 micro-rollups/day | Seconds, near-real-time | Append-only, high fanout | Data lake + impulse queue |
| **W2. Ad-hoc scenario reasoning** | 10s–100s of queries/day | 1–30s per query acceptable | Subgraph traversal + counterfactual | ECC (`causal.rs` + `causal_predict.rs`) |
| **W3. Structural gap analysis** | Continuous / daily scan | Minutes per sweep | Pattern-matching over case graph | ECC (same engine as cold-case) |

W1 is the hard problem because it runs continuously at global scale. W2 and W3 are the **value-generating** workloads; they use the graph W1 produces. Treating them separately lets us use a warehouse for W1's high-volume math and ECC for W2/W3's semantic reasoning, each playing to its strengths.

---

## 2. Streaming ingest architecture

**This section is the centre of gravity.** The brief confirms data will arrive continuously as small deltas, not as a nightly dump. Every other design choice downstream falls out of this.

### 2.1 The pipeline, end to end

```text
 POS / labor / inventory / promo     per-store event streams
                |
                v
 Edge collector (per-store or per-brand DC)
                |  normalises to canonical event schema
                v
 Brand-scoped message bus (Kafka / Kinesis / Pub/Sub — likely whatever QSR already runs)
                |  topics: brand-a.orders, brand-a.labor, brand-b.orders, ...
                v
 +--------------------------+--------------------------+
 |                          |                          |
 v                          v                          v
 Data lake writer      Micro-rollup aggregator     Streaming signal extractor
 (Iceberg / Delta)     (5-min windows, per-store)   (promos, stock-outs, anomalies)
                                |                          |
                                v                          v
                          Impulse emitter ───────────────<
                                |
                                v
                    ⟵ ECC impulse queue (impulse.rs) ⟶
                                |
                                v
                         DEMOCRITUS loop (democritus.rs)
                            SENSE → EMBED → SEARCH → UPDATE → COMMIT
                                |
                                v
                         ExoChain audit (chain.rs)
                                |
                                v
                    Sharded RVF case graph (per brand / region / quarter)
                                |
                                v
                    Query tier: W2 scenarios + W3 gap sweeps
```

The edge collector and bus layers are almost certainly **already in place at QSR** — they have to be, to move $125M of sales daily. We plug in at the bus layer with a dedicated consumer group.

### 2.2 Why impulses fit "lots of little deltas"

Look at what `ImpulseQueue` already does (`crates/clawft-kernel/src/impulse.rs`):

- HLC-timestamped for causal ordering across distributed sources
- `drain_ready()` batches work per tick, so we don't spin one-event-per-tick
- `ImpulseType::NoveltyDetected` / `CoherenceAlert` / `BeliefUpdate` are first-class
- Decays if unread — a lost delta isn't a correctness disaster

`DemocritusConfig.max_impulses_per_tick = 64` with a 50ms tick = 1,280 impulses/sec per worker thread. Rough math:

- ~30K stores × 12 micro-rollups/hour ≈ 360K micro-rollups/hour ≈ **~100/sec sustained**
- Plus structural signals (promo start, stock-out, schedule change): maybe **+50/sec**
- Peak multiplier at lunch across continental regions: **3–5x**

Even at peak, a single DEMOCRITUS worker is comfortably underutilised. Per-brand worker sharding (4 brands × 1 worker) gives ~10x headroom.

### 2.3 What becomes an impulse, what doesn't

**Impulses (go into ECC):**

| Event | Frequency | Why this tier |
| --- | --- | --- |
| Store 5-min micro-rollup (ticket count, revenue, avg ticket, payment-mix) | 288/day/store | Needed for intra-day coherence and anomaly signals |
| Hour-close rollup | 24/day/store | Structural node — used by `TIMELINE_PRECEDES` chains |
| Day-close rollup | 1/day/store | Canonical "daily" node — basis for weekly/quarterly aggregation |
| Promo activation / deactivation | ~10/day/brand | Direct `CAUSES` edge source |
| Stock-out / supply event | Variable | `INHIBITS` edges to menu items |
| Schedule publication | ~weekly/store | Updates ORG subgraph |
| Hire / termination / promotion | Variable | Updates PERSON / ROLE edges |
| Manager alert (waste spike, safety) | Variable | `CoherenceAlert` impulses |
| Menu change / price change | ~weekly/brand | New `MENU_ITEM` nodes or metadata update |

**Not impulses (stay in lake, referenced by `lake_ref`):**

- Every individual order line ("signature item + side + drink")
- Per-employee minute-by-minute clock events
- Full transaction payload (PCI-scoped, in any case)
- Customer-identifiable data (privacy constraint; see §12)
- Raw sensor telemetry from equipment

**Rule of thumb:** if a query would aggregate across it, it lives in the lake. If a query would *reason about it* or *connect it to other things semantically*, it lives in ECC.

### 2.4 Micro-rollup as the atomic unit

The 5-minute store-level rollup is the compromise between:

- **Fine enough** to detect intra-day anomalies (*"Metro-Alpha Brand-A district down 30% vs. forecast at 12:45 local"*) and respond with a `CoherenceAlert` impulse before the lunch rush ends.
- **Coarse enough** to keep the graph from exploding (~30K × 288/day ≈ 8.6M nodes/day at this granularity, already aggressive).

The rollup's metadata payload: `store_id`, `window_start`, `window_end`, `tickets`, `revenue`, `avg_ticket`, `payment_mix_hash`, `staff_count_on_shift`, `promo_codes_active`, `lake_ref` to the 5-minute raw slice.

**At day-close**, the 288 micro-rollups collapse into one `DAILY_ROLLUP` node with richer metadata; the micro-rollups become cold (reconstructable from the lake if needed). This keeps the hot graph at day-grain for longer queries while preserving the intra-day signal on the day it was fresh.

### 2.5 Out-of-order, late-arriving, and missing deltas

Real streaming is messy. The HLC timestamps in the impulse queue handle *causal* ordering, but we need explicit policy for:

- **Late arrivals** (store connection down for 4 hours): allowed up to a watermark (24h default); beyond that the rollup is flagged `reconstructed_from_lake: true` and the impulse is tagged `ImpulseType::BeliefUpdate` instead of `NoveltyDetected` so downstream graph updates know this is a revision, not a first observation.
- **Out-of-order across stores** (bus reordering): tolerable within a window; HLC sorts it out.
- **Missing deltas** (store submitted nothing for a window): this is itself an impulse — `CoherenceAlert { "reason": "missing_window" }`. Triggers a W3 gap-analysis pattern.
- **Duplicate deltas** (retries): idempotency key = `(store_id, window_start)`; `ImpulseQueue.emit()` already dedupes on HLC + source node.

### 2.6 Backpressure and governance

If ingest spikes beyond budget (`tick_budget_us` exceeded repeatedly):

1. DEMOCRITUS emits `budget_exceeded` events — already wired in the config path.
2. Backpressure policy: drop low-priority impulses (low-volume store micro-rollups) before high-priority (day-close, promo events, manager alerts).
3. Governance (`governance.rs`) validates: dropping data above a threshold is a `Blocking` severity event requiring `EscalateToHuman`.
4. Alternative: scale workers horizontally; this is already topology-aware via `hierarchical-mesh`.

---

## 3. Data model

### 3.1 Node types

Every node is a `CausalNode` (`causal.rs`) with label convention `{type}:{subtype}:{identifier}` and a `metadata: serde_json::Value` payload. A `StructureTag::Custom(0x20)` would be allocated for "RestaurantOps" to keep this tenant's nodes separate from other WeftOS workloads.

#### STORE

```json
{
  "label": "store:brand_a:metro_alpha_0001",
  "metadata": {
    "node_type": "STORE",
    "brand": "brand-a",
    "store_number": "0001",
    "country_code": "XX",
    "region_code": "region-1",
    "metro_code": "metro-alpha",
    "franchise_model": "franchised",
    "franchisee_org_ref": "org:franchise:franchisee_primary",
    "opened_date": "2003-08-14",
    "sqft": 2800,
    "drive_thru": true,
    "curbside": true,
    "delivery_platforms": ["delivery-x", "delivery-y", "delivery-z"],
    "pos_system": "pos-vendor-primary",
    "timezone": "UTC-05",
    "capacity_seats": 48,
    "baseline_daily_sales": 5400,
    "currency": "USD",
    "lake_ref_prefix": "lake://qsr/brand-a/region-1/metro-alpha/0001/"
  }
}
```

#### DAILY_ROLLUP (and MICRO_ROLLUP / HOUR_ROLLUP)

```json
{
  "label": "day:brand_a:0001:2026-04-20",
  "metadata": {
    "node_type": "DAILY_ROLLUP",
    "store_ref": "store:brand_a:metro_alpha_0001",
    "business_date": "2026-04-20",
    "tickets": 347,
    "revenue": 4927.18,
    "cogs": 1478.15,
    "labor": 1182.52,
    "labor_hours": 64.5,
    "waste": 118.25,
    "avg_ticket": 14.20,
    "payment_mix": { "card": 0.71, "cash": 0.12, "app": 0.14, "gift": 0.03 },
    "daypart_revenue": { "breakfast": 812, "lunch": 2104, "dinner": 1601, "late": 410 },
    "budget_revenue": 5200,
    "budget_variance_pct": -0.052,
    "promo_codes_active": ["promo-2for6-signature", "promo-app-exclusive-25"],
    "weather_proxy_ref": "weather:metro-alpha:2026-04-20",
    "lake_ref": "lake://qsr/brand-a/region-1/metro-alpha/0001/orders/2026-04-20.parquet",
    "verified": true,
    "source_stream_hlc": "2026-04-21T04:02:11.873Z/0012"
  }
}
```

- `budget_variance_pct` is pre-computed at ingest so it can be embedded and semantically searched (*"find me all days that looked like this"*).
- `verified: false` means a rollup was reconstructed from lake after a late arrival, not streamed natively.

#### PERSON / ROLE / POSITION

```json
{
  "label": "person:employee:brand_a_0001_e001",
  "metadata": {
    "node_type": "PERSON",
    "subtype": "hourly",
    "employee_id_hashed": "blake3:f8a2…",
    "hire_date": "2024-11-03",
    "roles_held": ["crew", "shift_lead"],
    "active_role_ref": "role:shift_lead",
    "home_store_ref": "store:brand_a:metro_alpha_0001",
    "certifications": ["food-safety-national-2026"],
    "turnover_risk_score": 0.34
  }
}
```

```json
{
  "label": "position:brand_a_0001_gm",
  "metadata": {
    "node_type": "POSITION",
    "store_ref": "store:brand_a:metro_alpha_0001",
    "role_template": "general_manager",
    "required_certifications": ["food-safety-national", "brand-a-gm-cert"],
    "filled_by_ref": "person:employee:brand_a_0001_e047",
    "filled_since": "2023-02-01",
    "vacancy_coverage_gap_days": 0,
    "critical": true
  }
}
```

Note the `employee_id_hashed` field: QSR's internal employee ID is **never** stored in ECC in the clear. We take a BLAKE3 hash keyed with a tenant-scoped salt at ingest. PII stays in the data lake behind the lake's existing access controls; ECC stores only the hash, which is sufficient for linking nodes and computing subgraphs but not for re-identification without the salt.

The **POSITION** node is the key to gap analysis — it represents the *slot that should exist* per the franchise template, independent of who is filling it. A vacant POSITION is a structural gap in the org graph.

#### MENU_ITEM / PROMOTION / SUPPLY_EVENT / BUDGET_TARGET / HYPOTHESIS / WEATHER / REGION / BRAND / FRANCHISEE

Each gets a similar schema pattern: `node_type`, domain-specific attributes, `lake_ref` where relevant, embedding source text for HNSW.

#### HYPOTHESIS (scenario node)

```json
{
  "label": "hypothesis:scenario:metro_alpha_miss_q2_2026",
  "metadata": {
    "node_type": "HYPOTHESIS",
    "subtype": "counterfactual",
    "description": "Metro-Alpha Brand-A stores miss weekly budget by 8% for week 17",
    "created_by": "user:analyst:u001",
    "created_at": "2026-04-21T14:32:00Z",
    "intervention": {
      "scope": { "brand": "brand-a", "metro": "metro-alpha", "week": "2026-W17" },
      "operation": "scale_revenue",
      "factor": 0.92
    },
    "baseline_lambda_2": 0.648,
    "counterfactual_lambda_2": null,
    "projected_quarter_revenue": null,
    "status": "pending"
  }
}
```

### 3.2 Edge types

Reuse the existing `CausalEdgeType` enum. Semantic overlay:

| Edge | `CausalEdgeType` | Source → target | Weight |
| --- | --- | --- | --- |
| `OPERATES_IN` | `Correlates` | STORE → REGION | 1.0 |
| `SELLS` | `Enables` | STORE → MENU_ITEM | availability ratio |
| `EMPLOYS` | `Correlates` | STORE → PERSON | 1.0 |
| `FILLS_POSITION` | `EvidenceFor` | PERSON → POSITION | tenure/fit score |
| `REPORTS_TO` | `Follows` | ROLE → ROLE | 1.0 |
| `CLOSED_DAY` | `Correlates` | STORE → DAILY_ROLLUP | 1.0 |
| `TIMELINE_PRECEDES` | `Follows` | DAILY_ROLLUP → DAILY_ROLLUP | 1.0 |
| `AGGREGATES_TO` | `Correlates` | DAILY → WEEK → MONTH → QUARTER | 1.0 |
| `CAUSES` (promo lift) | `Causes` | PROMOTION → DAILY_ROLLUP | learned causal strength (see §6) |
| `INHIBITS` (stock-out) | `Inhibits` | SUPPLY_EVENT → MENU_ITEM | severity |
| `APPLIES_TO` (budget) | `Correlates` | BUDGET_TARGET → STORE | 1.0 |
| `VIOLATES` (variance) | `Contradicts` | DAILY_ROLLUP → BUDGET_TARGET | variance magnitude |
| `CORROBORATES` (weather) | `EvidenceFor` | WEATHER → DAILY_ROLLUP | correlation |
| `GAPS_IN` | `Correlates` w/ weight=0 | POSITION → PERSON (null) | absence signal |

Edge weights for `CAUSES` and `EvidenceFor` are the important ones; see §6 for how they're learned from history.

### 3.3 Aggregation chain

Each store emits `DAILY_ROLLUP` nodes. At week-close, a `WEEKLY_ROLLUP` node is created and connected via `AGGREGATES_TO` to the 7 dailies. Same for month, quarter, year. This lets subgraph queries stay at the right grain:

- "Metro-Alpha Q2 projection" → walks `QUARTER_ROLLUP` nodes, doesn't touch 80K micro-rollups.
- "Metro-Alpha week 17 anomaly hunt" → walks `DAILY_ROLLUP` and `MICRO_ROLLUP`.

The aggregation is **itself a causal edge** so counterfactual propagation naturally flows up and down the time hierarchy.

---

## 4. Partitioning strategy

Sharding is how we avoid one 600 GB RVF nobody can load. The partition scheme has to match both **read patterns** (queries) and **write patterns** (streaming).

### 4.1 Shard layout

```text
rvf://qsr/
  entities/
    global.rvf                  brands, regions, countries, currencies, franchisee orgs
                                ~300 MB, always hot, rebuilt monthly
    menu/{brand}.rvf            menu items + variants per brand
                                ~50 MB each, hot, rebuilt on menu change

  org/
    {brand}/{region}.rvf        people, positions, roles, assignments
                                ~50–200 MB each, warm, rebuilt nightly from delta events

  ops/
    {brand}/{region}/{yyyy-qq}/
      rollups.rvf               DAILY_ROLLUP + WEEKLY_ROLLUP + QUARTER_ROLLUP
                                ~500 MB–3 GB, hot for current q, warm for prior 4, cold beyond
      micro.rvf                 MICRO_ROLLUP + HOUR_ROLLUP (optional — only if queries demand)
                                ~5–15 GB, hot for current week only, dropped after 30d

  events/
    {brand}/{yyyy-qq}/
      promos.rvf                promotions + activations
      supply.rvf                supply events, stock-outs
      budgets.rvf               budget targets + variances

  hypotheses/
    scenario-{uuid}.rvf         ephemeral scratch space for W2 scenarios
                                TTL 7–30 days, user-deletable

  cross/
    global-index.rvf            cross-shard HNSW index heads + POLICY_KERNEL routing
                                always loaded
```

### 4.2 Sharding dimensions and why

| Dimension | Why | Tradeoff |
| --- | --- | --- |
| **Brand** (Brand-A / -B / -C / -D) | Different menus, different franchise contracts, different seasonal shapes. Queries rarely cross brands except at corporate finance level. | Cross-brand queries need a coordinator; global supply chain spans brands |
| **Region** (continental → country → state) | Queries are naturally region-scoped. Locality of reference = faster queries. Regulatory boundaries are regional. | Some franchisees cross regions; need global entity shard for them |
| **Time (quarter)** | Archive-friendly; quarterly close is an existing corporate ritual; keeps hot shard size bounded | Year-over-year queries span 4+ time shards |
| **Grain (micro/day/week/quarter)** | Most queries only need one grain; the aggregation edges let us pull up or down | Drill-through queries touch two grain shards |

### 4.3 Hot / warm / cold tiers

| Tier | Contents | Load policy | Storage |
| --- | --- | --- | --- |
| **Hot** | `entities/global.rvf`, `menu/*.rvf`, `cross/global-index.rvf`, current-quarter `ops/` shards | Loaded at boot, kept resident | Fast SSD / memory-mapped |
| **Warm** | Prior 4 quarters of `ops/`, all `org/` shards | Lazy-loaded on first query; LRU eviction | Local SSD |
| **Cold** | `ops/` older than 1 year, archived `micro.rvf` | Rehydrated on demand; slow queries acceptable | Object storage / glacier-tier |

Temperature-based quantization from `.planning/04-rvf-integration.md` already does this *within* a shard; the tier system extends it *across* shards.

### 4.4 Cross-partition entities

Some entities span shards (a supplier serving multiple brands, a franchisee org with 400 stores across 3 brands, a regional manager). These need stable global IDs:

- `UniversalNodeId = BLAKE3(structure_tag, context_id, hlc, content_hash, parent_id)` from `crossref.rs` already does this.
- A supplier node lives **once** in `entities/global.rvf`.
- References from `ops/brand-a/...` and `ops/brand-c/...` shards point to it via `CrossRef::EvidenceFor`.
- Query resolver follows crossrefs across shard boundaries transparently.

### 4.5 Streaming writes across shards

The impulse emitter needs to know which shard(s) each impulse updates. A routing table lives in `cross/global-index.rvf` and is refreshed whenever a new shard is created:

```text
impulse { store_id=brand_a_0001, type=day-close, date=2026-04-20 }
  → route:     ops/brand-a/region-1/2026-Q2/rollups.rvf
  → secondary: ops/brand-a/region-1/2026-Q2/micro.rvf (linked micro-rollups)
  → index:     cross/global-index.rvf (HNSW head for this new embedding)
```

Each shard gets its own DEMOCRITUS worker (cheap — they're tick loops, not threads). Writes within a shard are serialised; writes across shards parallelise. This is the `hierarchical-mesh` topology the project already defaults to.

### 4.6 Shard rollover and compaction

- **Quarter-close**: at Q-end + 72h (settle late-arriving data), the current-quarter shard is sealed, compacted, crypto-attested via ExoChain, and demoted to warm.
- **Year-close**: quarterly shards consolidate into a year-granularity shard with `MICRO_ROLLUP` dropped.
- **Menu change**: `menu/{brand}.rvf` rebuilds; prior version retained with version tag.
- **Org restructuring**: `org/{brand}/{region}.rvf` snapshots before rebuild (audit requirement).

---

## 5. Synthetic corpus generator

We cannot test at full scale with QSR data (NDA, integration timeline, no ground truth for "what would've happened"). We need a **synthetic corpus generator** that produces data indistinguishable from real QSR streams at the schema level, with **known ground-truth causal structure** underneath so we can score predictions.

### 5.1 What the generator produces

Four coordinated output streams, all reproducible from a single seed:

1. **Static dimension tables** (written once): stores, menu items, employees, positions, franchisees, regions, suppliers — mirrors what the client's dimension warehouse would produce.
2. **Truth graph** (written once, *hidden from the system under test*): the actual parametric causal structure the generator used — which promos actually cause lift, which weather events actually suppress demand, which staffing patterns actually predict turnover. This is the oracle for scoring.
3. **Event streams** (continuous, paced): POS orders, labor punches, inventory movements, promo activations, schedule publications, hire/term events, weather records. Emitted as JSONL with HLC timestamps; shape matches the expected data-lake schema.
4. **"Reality" rollups** (derivable from the stream, but provided for convenience): pre-computed daily/weekly/quarterly actuals so the scoring harness can check ECC outputs against them.

### 5.2 Parametric knobs

```text
SEED                              master RNG seed
SCALE_TIER                        tiny (10 stores, 1 month) | small (500, 1 quarter)
                                  | medium (5K, 1 year) | full (30K, 5 years)
BRANDS                            [brand-a, brand-b, brand-c, brand-d] with per-brand menu sizes
REGIONS                           hierarchical: continents → countries → states → metros
STORE_CLASS_MIX                   % corporate / % franchisee / % new / % mature / % struggling
BASELINE_SALES                    per-store-class distribution + noise floor
SEASONALITY                       weekly (dow), yearly (holidays), monthly (payday)
                                  each as Fourier series with configurable amplitude
TREND                             long-term per-brand growth / decline rates
PROMO_CATALOG                     parametric: discount_pct, duration, reach, true_causal_lift
WEATHER_COUPLING                  per-region weather sensitivity (rain = -X%, heat = +Y%)
MACRO_SHOCKS                      scripted events: reopening, supply crisis, recession
LABOR_MODEL                       staffing formula per daypart; turnover curves;
                                  certification time-to-fill
SUPPLY_MODEL                      SKU-level reorder cycles, disruption frequency
BUDGET_MODEL                      budget-setting behaviour (optimistic / realistic / sandbagged)
ANOMALY_INJECTION                 frequency, severity, types of anomalies
                                  (ghost sales, clock manipulation, data loss)
LATE_ARRIVAL_DIST                 bus delay distribution; % stores with connectivity issues
DUPLICATE_RATE                    stream duplication rate (tests idempotency)
OUT_OF_ORDER_RATE                 within-bus reordering
```

Every run of the generator with the same knob values produces byte-identical output. Seed bumping produces a different draw from the same parametric distribution — useful for statistical tests.

### 5.3 Scale tiers

| Tier | Stores | Duration | Events | Output size | Runtime target |
| --- | --- | --- | --- | --- | --- |
| **tiny** | 10 | 1 month | ~10M | ~500 MB | <30s |
| **small** | 500 | 1 quarter | ~1.5B | ~80 GB | <10 min |
| **medium** | 5,000 | 1 year | ~60B | ~3 TB | <4h |
| **full** | ~30,000 | 5 years | ~2T | ~100 TB | offline, distributed |

`tiny` and `small` are laptop-scale for dev iteration. `medium` is a CI nightly target. `full` only runs as an integration milestone — you don't need to regenerate 100 TB to validate a change.

### 5.4 Generator operational modes

- **Batch dump**: produce a full corpus for a scale tier, write to parquet/JSONL files on disk. Used for ingest throughput testing and for seeding the warm-start of longer experiments.
- **Replay**: read a pre-generated batch dump and emit events through a real bus (local Kafka) at a configurable rate (real-time, 10x, 100x, as-fast-as-possible). Simulates the actual streaming pipeline.
- **Live**: generate events in real time (or accelerated real time) without ever writing a full batch dump. Required for long-horizon soak tests; doesn't need disk space for the full corpus.
- **Chaos mode**: inject disruptions mid-run (drop 10% of events for 1h, reorder window, duplicate a day's worth, skew a store's clock by 4 min).

### 5.5 Ground truth and scoring

The generator emits a parallel **truth manifest** the system-under-test never sees:

```text
truth/
  causal_graph.json                 actual {promo_id, true_lift_pct, p_value} pairs
  counterfactual_answers/
    metro_alpha_miss_q2.json        actual quarterly revenue under each tested intervention
    brand_d_labor_shock.json        actual impact under each scenario
  anomaly_ledger.json               all injected anomalies with exact timestamps
  org_gap_ledger.json               all intentionally-created org gaps
```

Scoring harness:

```text
predicted = ecc.run_scenario(scenario_spec)
actual    = truth_manifest.lookup(scenario_spec.id)
score     = {
  directional_accuracy:          sign(predicted.delta) == sign(actual.delta),
  magnitude_error:               abs(predicted.delta - actual.delta) / abs(actual.delta),
  within_ci_80:                  actual.delta in predicted.ci_80,
  within_ci_95:                  actual.delta in predicted.ci_95,
  counterfactual_edge_recall:    (predicted_causal_edges ∩ true_edges) / true_edges,
  counterfactual_edge_precision: (predicted_causal_edges ∩ true_edges) / predicted_edges,
  latency_p50 / p95 / p99:       wall-clock per query
}
```

A prediction "passes" if directional accuracy > 80% and `within_ci_80` > 75% on the medium tier. These thresholds are the numbers we commit to the client.

### 5.6 Generator implementation stack

- Rust crate `clawft-casestudy-gen-qsr` under `crates/` — not in release binary, dev-dep only.
- Reuses the existing `chain.rs` HLC for timestamps.
- Reuses `embedding.rs` only if we want pre-computed embeddings in the corpus (optional; saves CI time but large output).
- Writes parquet via `arrow2` or `parquet2`.
- Kafka producer via `rdkafka` for replay mode.
- Deterministic RNG via `rand_chacha` seeded from `SEED`.

### 5.7 Corpus artifacts to version-control

Not the corpus itself (too big), but:

- The generator configuration YAML for each named scenario (`metro_alpha_miss_q2.yaml`, `supply_shock.yaml`, `labor_turnover.yaml`).
- Truth manifest JSONs for published benchmark scenarios.
- Golden-artifact hashes for the scoring harness.

Corpus data lives in object storage, keyed by `(scenario_name, seed, generator_version)`.

---

## 6. Counterfactual scenario engine (W2)

This is the money query — *"if Metro-Alpha misses budget this week, what does Q2 revenue look like?"*

### 6.1 Counterfactual semantics

A scenario is an **intervention** on a subgraph:

```text
Intervention ::= {
  scope:     SubgraphSelector,                   // which nodes
  operation: Replace | Scale | Delete | Insert,  // what change
  target:    MetadataField | EdgeWeight | NodeExistence,
  value:     Number | Distribution               // by how much
}
```

Examples:

- `scope = {brand=brand-a, metro=metro-alpha}, operation=scale, target=daily_rollup.revenue, value=0.92` → "Metro-Alpha Brand-A revenue 8% below actual"
- `scope = {position.critical=true, region=region-2}, operation=delete` → "all critical positions in region-2 vacant"
- `scope = {promo_id=promo-spring-2for6}, operation=delete` → "what if the spring 2-for-$6 promo didn't run"

### 6.2 Execution pipeline

```text
1. Parse scenario spec → intervention object
2. Resolve scope → list of affected nodes (often across multiple shards)
3. Shadow clone: create hypothesis:scenario:{uuid} HYPOTHESIS node;
   fork the affected subgraph into hypotheses/scenario-{uuid}.rvf
4. Apply intervention: modify cloned subgraph per operation
5. Propagate:
   a. Walk outgoing edges from affected nodes via CAUSES / EvidenceFor / AGGREGATES_TO
   b. For each downstream node, compute new value as
      new_value = baseline_value + Σ (edge_weight × upstream_delta)
   c. Continue until reaching QUARTER_ROLLUP (the question's target grain)
6. Spectral correction: run spectral_analysis() on shadow subgraph;
   lambda_2 delta signals how much coherence the intervention breaks
7. EML refinement: pass the analytical delta through CausalCollapseModel
   to get learned residual correction (seasonality, cannibalisation, spillover)
8. Uncertainty: Monte Carlo across edge_weight distributions (each weight has
   a learned posterior, not a point estimate) → produce CI bands
9. Attribution: walk back from the result to show which edges contributed
   ("63% direct Metro-Alpha miss, 22% cross-metro cannibalisation, 15% supply-chain feedback")
10. Commit: ExoChain audit entry; hypothesis node metadata populated;
    return to caller
```

Step 7 is where WeftOS earns its keep over a spreadsheet forecast — the `CausalCollapseModel` in `causal_predict.rs` learns systematic residuals from historical scenarios and applies them.

### 6.3 Edge-weight learning

The causal edges have to come from somewhere. Three sources, in decreasing order of reliability:

1. **A/B tests and holdouts** (gold): if the client has historical promo holdout data, edge weight = measured lift from the holdout. Edge carries a `provenance: "ab_test"` tag.
2. **Quasi-experiments** (silver): staggered rollouts, diff-in-diff on promo launch windows, synthetic-control methods over matched store panels. Weight comes with a confidence interval.
3. **Observational correlation** (bronze): historical regression of daily rollups on promo activity, controlling for seasonality/weather/labour. Lowest-confidence; edges flagged `provenance: "observational"` and scenarios using them label their output as such.

The ECC layer does not invent causal weights. QSR has to provide A/B data or accept bronze-tier weights for features that lack it. This is the honest conversation to have up front (§12).

### 6.4 Mid-day vs end-of-day queries

At 2pm local time in Metro-Alpha, the graph has partial data for "today." A "what does today look like" query must:

- Use actual rollups for windows that have closed.
- Use projected rollups (from the generator of the hour) for the open windows.
- Propagate uncertainty through the open-window projection.
- Wider CI bands than the same query at 11pm.

The distinction is implicit in the HLC watermark for each store; the engine flags `as_of_hlc` in the response.

### 6.5 Hypothesis lifecycle

HYPOTHESIS nodes have a TTL and state machine:

```text
pending → computing → complete → {archived | active | eliminated}
```

- `pending`: user submitted, queued.
- `computing`: engine running the pipeline above.
- `complete`: has predicted outcome + CI + attribution.
- `active`: user has favourited or subscribed; re-runs nightly as new data arrives.
- `eliminated`: user has dismissed; keep for audit, exclude from UI.
- `archived`: TTL expired, summary kept, detail pruned.

Storage lives in `hypotheses/scenario-{uuid}.rvf` and is cheap to discard.

### 6.6 Scenario types QSR will actually ask

Catalog these as named templates so they're not one-off:

| Template | Scope | Operation |
| --- | --- | --- |
| `geo_miss` | region / metro × week | scale revenue down |
| `labor_shock` | region × month | scale labor_hours up (wage increase) |
| `supply_disruption` | menu_item × region × duration | inhibit menu item |
| `promo_pull` | promotion × brand | delete promotion |
| `store_closure` | store set | delete store |
| `store_opening` | hypothetical store | insert store with baseline |
| `menu_launch` | brand × new item | insert menu_item with projected attach rate |
| `franchisee_divestiture` | franchisee_org | delete franchisee, orphan stores |
| `weather_season` | region × season | modify weather-coupling edges |
| `competitive_entry` | region × competitor | insert demand-sink edge |

Each template is a scenario-builder wizard (UI) plus a parameterised code path (backend).

---

## 7. Org chart and gap analysis (W3)

Direct lift from the cold-case engine. See `.planning/symposiums/cold-case-ecc/ecc-application-mapping.md` §3 for the full pattern catalog. Translation table:

| Cold-case pattern | QSR equivalent | Detection |
| --- | --- | --- |
| Uninterviewed witness | Vacant POSITION that should be filled | POSITION with no FILLS_POSITION edge |
| Untested evidence | Pending manager certification past due | PERSON with cert expiration in window, no renewal event |
| Unverified alibi | Self-reported training without LMS record | training_event without CORROBORATES edge to LMS event |
| Timeline gap | Shift-coverage gap | missing PERSON × SHIFT coverage |
| Similar MO cases | Stores with similar staffing anti-patterns | HNSW on staffing feature vectors |
| Cell records not subpoenaed | Inventory count not performed | STORE without INVENTORY_EVENT node in period |
| Associates not investigated | Turnover-cluster contagion | PERSON with ASSOCIATED_WITH weight>0.5 to recently-exited employees |
| Surveillance never pulled | Drive-thru camera audit not run | LOCATION with audit_required, no audit EVENT |

### 7.1 Coherence scoring per store

Each store's org subgraph gets a `lambda_2` coherence score. Lower score = weaker structural integrity = higher attention warranted.

Composite "ops health" for a store = weighted combination of:

- Org `lambda_2` (how coherent is the team structure?)
- Rollup variance signal (how steady are daily metrics?)
- Inventory accuracy (how often do counts match ledger?)
- Labor efficiency variance (sales-per-labor-hour stability)
- Gap count × gap severity

The dashboard surfaces the bottom-N stores nightly — operations team's morning queue.

### 7.2 Org gap → ECC impulse

Gap detections aren't passive reports — they emit `ImpulseType::CoherenceAlert` impulses back into the loop. Next tick, they show up as actionable items in the user-facing console, each with:

- What's missing (the gap).
- What it affects (downstream edges / coherence delta if closed).
- Suggested action (hire, certify, audit, pull records).
- Urgency score (degradation deadline equivalent).

The "counterfactual `lambda_2` delta" from the cold-case doc applies directly: *"Filling the vacant GM position at store `brand_a_0001` would improve its org coherence from 0.31 to 0.52."*

---

## 8. Test harness (formal)

Three layers, each serves a different purpose.

### 8.1 Unit / integration tier

- Schema validators: every node type's metadata conforms.
- Edge-classification logic: `democritus::classify_edge` correctness on canonical fixtures.
- Governance: SOX and franchisee-boundary rules actually block.
- ExoChain: every mutation produces exactly one chain event, chain integrity verifies.
- Shard routing: impulses land in the right shard, cross-shard refs resolve.

Run time: seconds. Runs on every PR.

### 8.2 Scale / performance tier

Against synthetic corpus tiers from §5:

| Test | Tier | Metric | Target |
| --- | --- | --- | --- |
| Cold-start ingest replay | small | events/sec sustained | >5K/sec per worker |
| Steady-state ingest | medium | events/sec sustained | >2K/sec per worker (more overhead) |
| Scenario query cold | medium | latency p95 | <10s |
| Scenario query warm | medium | latency p95 | <2s |
| Gap sweep full | medium | wall-clock | <15 min |
| Spectral analysis on 1M-node subgraph | medium | wall-clock | <30s |
| HNSW build 10M vectors | medium | wall-clock | <4h |
| Shard rollover (quarter-close) | medium | wall-clock | <10 min |
| Memory footprint resident | medium | RSS | <32 GB per worker |

Run nightly on CI against `medium` tier; weekly against `full` tier on a dedicated box.

### 8.3 Correctness / scoring tier

Against the truth manifest from §5.5:

- Scenario template × scale tier matrix (10 templates × 4 tiers = 40 benchmark scenarios).
- Each logs to a golden artifact file.
- Regressions trigger on: directional accuracy drop, CI coverage drop, magnitude error increase.
- A/B compare against baseline (prior release) on every nightly run.

The thresholds in §5.5 become the release criteria.

### 8.4 Chaos / resilience tier

- Drop % of events for N minutes → verify gap detection, verify recovery.
- Reorder window → verify HLC reconciliation.
- Duplicate storm → verify idempotency, verify no double-count.
- Clock skew → verify HLC handles it, verify no infinite loops.
- Shard storage slow (1000ms p99) → verify backpressure, verify governance escalation.
- Worker crash mid-tick → verify impulse queue durability, verify replay on restart.

Runs weekly, not on every PR.

### 8.5 Client-data replay (once available)

After contract + NDA, swap synthetic corpus for anonymised QSR historical data:

- Import a year of historical data into a staging RVF shard set.
- Run every benchmark scenario against the staging shards.
- Compare to outcomes the client actually observed (they know what did happen).
- Iterate on edge weights and EML model until scores clear threshold.
- Only after this step do we make forward-looking predictions for real.

---

## 9. Upper limits and proof points needed

Things we cannot know from the spec alone; they require building §8 and measuring:

| Unknown | Why it matters | How we find out |
| --- | --- | --- |
| HNSW build at 10M vectors | Determines shard-rollover feasibility | Scale tier `medium`, timing test |
| Spectral analysis on 1M-node subgraph | Determines scenario latency at scale | Synthetic graph with known `lambda_2`, measure |
| Impulse queue sustained throughput | Determines worker count | Replay at 1x, 10x, 100x, find knee |
| RVF load-time for 5 GB shard | Determines cold-query latency | Cold-start bench |
| Progressive HNSW Layer-A recall for financial features | Determines intra-tick query quality | Golden-artifact test with known answers |
| EML residual stability over long runs | Determines when to retrain | Long-horizon soak test |
| Cross-shard crossref resolution latency | Determines when to denormalise | Benchmarks with varying cross-shard ratio |

We cannot commit SLAs to the client without measurements on at least the `medium` tier.

---

## 10. Phased build

### Phase 0 (weeks 1–2): Synthetic corpus foundation

- `crates/clawft-casestudy-gen-qsr` skeleton.
- `tiny` tier end-to-end: generate → replay → ingest → query.
- Schema for STORE, DAILY_ROLLUP, PERSON, POSITION, PROMOTION.
- Truth manifest + scoring harness shell.
- First 3 scenario templates.

### Phase 1 (weeks 3–5): Streaming ingest spine

- Impulse emitter pipeline with HLC + dedupe + late-arrival.
- DEMOCRITUS worker per brand.
- RVF sharding + global routing index.
- Governance rules (SOX, franchisee boundary).
- `small` tier ingest passing.

### Phase 2 (weeks 6–8): Scenario engine

- Intervention parser + subgraph cloner.
- Propagation via existing `causal_predict.rs`.
- EML model training on synthetic truth.
- Monte Carlo uncertainty.
- 6 scenario templates + dashboards.
- `medium` tier scenario queries passing.

### Phase 3 (weeks 9–10): Gap analysis & org

- All 8 cold-case gap patterns adapted.
- Store coherence scoring.
- Nightly sweep + CoherenceAlert impulses.
- Ops dashboard MVP.

### Phase 4 (weeks 11–13): Hardening

- Chaos tests green.
- `full` tier smoke test.
- Full audit trail end-to-end.
- Security / privacy review (see §12).

### Phase 5 (client integration, post-contract)

- Historical data replay.
- Edge-weight tuning per QSR reality.
- Calibration against client-known outcomes.
- Forward-deployment plan.

---

## 11. Risks

**Substrate risks** (WeftOS itself):

- RVF has not been exercised at 10–60 GB sharded scale. Proof required.
- Spectral Lanczos at 1M nodes is theoretical; `calibration.rs` has `spectral_capable` for a reason.
- `causal_predict.rs::CausalCollapseModel` is recent; EML stability over months of synthetic data unproven.
- Sharded HNSW with cross-shard crossrefs isn't a default-tested path.

**Modelling risks**:

- Edge weights need ground truth. Without A/B history, predictions are observational. Must be stated clearly.
- Temporal granularity: sub-daily scenarios explode graph size. Need to confirm QSR accepts daily-grain forecasting.
- Holiday/promo calendar is its own schema nightmare across 100+ countries.

**Operational risks**:

- Streaming pipeline tolerates some data loss; forward-looking forecasts degrade with data loss. Monitoring needs to surface lost-data rate.
- Franchisee data-sharing boundaries are **legal**, not technical. Governance rules must be drafted with QSR legal before ingestion starts.
- Employee data (PII) is regulated differently in every region (GDPR, PIPEDA, CCPA, LGPD, …). Org graph design must be privacy-first.

**Commercial risks**:

- A client saying "thousands of stores daily" means different things at the edges. Scope creep likely. Contract should tier by store-count and brand-count.
- "What-if" answers are predictions, not promises. The output UI must communicate CI bands prominently or the system will be blamed for every missed forecast.

---

## 12. Open questions for the client

Before a scoping call:

1. **Data-lake shape.** Which platform (Snowflake / Databricks / BigQuery / something bespoke), what format (Iceberg / Delta / Parquet), what the canonical order/labor/inventory schemas look like.
2. **Streaming vs. batch.** Is the "real-time" stream actually real-time at the source, or is there an hourly/daily aggregation step the data lake already does? If the latter, what's the finest grain we can consume?
3. **Historical causal data.** Any A/B test artifacts? Holdout panels? Staggered rollouts with clean treatment groups? This gates edge-weight quality.
4. **Temporal resolution.** Is daily-grain forecasting sufficient, or do they need intra-day ("this week is shaping up to miss budget based on lunch")?
5. **Scope: single-brand or cross-brand scenarios.** Do they want "what if Metro-Alpha Brand-A misses" alone, or "what if Metro-Alpha across all four brands misses"?
6. **Franchisee boundaries.** Which franchisee data can cross the tenant boundary into the corporate view, which can't? Has legal drafted this already?
7. **PII regime.** Which employee attributes can flow into ECC? Can certifications (yes/no) cross? Performance scores? Turnover risk? Each answer changes the schema.
8. **Who uses the scenarios.** Finance running quarterly projections? Ops running weekly gap analysis? Both? Which gets priority if we have to trade?
9. **Sensitivity and disclosure.** A forward-looking scenario output, if leaked, is material non-public information. What's the access control and retention policy they require?
10. **Success metric for the engagement.** What does "working" mean to them? Scenario latency? Directional accuracy? Coverage of store universe? Decisions influenced? We need the metric we'll be measured against.

---

## 13. TL;DR

- **Fit: strong** on W2/W3 (scenario, gap analysis) — direct reuse of cold-case kernel machinery.
- **Fit: conditional** on W1 (ingest) — streaming works; scale needs proof via §5 synthetic corpus and §8 harness.
- **Don't put raw transactions in RVF.** Micro-rollups are the right atomic unit.
- **Shard by (brand, region, quarter)** with a global entities shard + global routing index.
- **Build order**: synthetic corpus first, then ingest, then scenarios, then gaps. Don't move to client data until the synthetic harness is green.
- **Edge weights are only as good as the client's A/B history.** This is a conversation before a contract.
