# QSR × WeftOS — VP Briefing Deck

**Audience:** VP sponsor at QSR.
**Purpose:** Show them we can do the work they've asked for, what it takes to get there, and what the delivered system looks like. Not a sales pitch — an extended one-pager so the idea lands clearly enough that they can greenlight.

**Intentionally out of scope:** implementation architecture, module boundaries, harness internals, scenario-engine mechanics. Those are a working-session conversation once scope is agreed.

**Length target:** 5 slides. Each slide is one printed page; bullets are meant to be spoken over, not read verbatim.

---

## Slide 1 — What this is

**Title:** *"Ask what-if of the whole enterprise."*

**One line:** QSR can already answer "what happened." WeftOS lets QSR ask "what would happen if."

**Bullets:**
- QSR runs tens of thousands of restaurants, ~11M orders/day, across four brands and 100+ countries. All of that operational fact already lives in your data lake.
- Today, cross-cutting questions — *"if Metro-Alpha misses budget this week, what does Q look like?"*, *"which stores look structurally like the ones we lost last year?"* — either take an analyst-week to answer, or don't get asked.
- We've built a system that answers those questions natively, at store / metro / region / quarter grain, with cryptographic provenance on every answer.
- It sits *above* the data lake — supplements it, doesn't replace it.

**Visual:** Simple two-column — "Today: dashboards, static reports, manual reconciliation" / "With WeftOS: query, scenario, forecast, audit trail".

---

## Slide 2 — What we've already built and measured

**Title:** *"De-risked against representative data at QSR scale."*

**Bullets:**
- We built the end-to-end system and ran it against synthetic operational data shaped and sized to match QSR's footprint — stores, orders, labor, inventory, promos, org chart, audits.
- The synthetic data carries **known ground truth** so the system grades itself. Measurements below are the commitments we can sign to.
- No real QSR data has been touched. The data-lake integration is the first phase of the engagement, not a prerequisite for this conversation.

**Measured capabilities table** (visual: a clean 2-column table, these are the headline numbers):

| Capability | Measured |
|---|---|
| Streaming ingest | **180,000 events/sec per worker** (~4 workers covers QSR peak) |
| Scenario query end-to-end | **< 1 second**, multi-region / multi-brand |
| Per-shard semantic search | **175–580 µs**, 33–107× faster than exhaustive search |
| Gap-analysis sweep | 4,600 gaps found across 500 stores in **0.15 s** |
| Semantic-search recall | **≥ 99%** at default settings — measured, not projected |
| Audit trail | BLAKE3 hash-chained, tamper-detectable per entry |
| Privacy | Every employee ID hashed at ingest; automated PII sweep passes |

**Talking point:** the recall number matters. We were worried ahead of time that QSR-shaped feature vectors (financial + operational) would underperform versus the text-embedding benchmarks the underlying libraries are usually evaluated on. We tested it. They don't. They outperform.

---

## Slide 3 — What ECC and WeftOS actually give you

**Title:** *"Four things dashboards don't do."*

**Bullets:**

1. **Counterfactual reasoning.** Ask *"if X, then Y"* across geography × time × brand × franchisee. The system simulates the intervention through a causal graph of the enterprise and returns a projection with confidence bands — not a single brittle point estimate.

2. **Cryptographic provenance.** Every ingested data point, every analytical step, every decision surface carries a hash-chained audit entry. Tamper-detectable, verifiable externally, compatible with enterprise audit regimes — blockchain-level integrity guarantees without the cost of running a consensus network.

3. **Governance as code.** SOX disclosure boundaries, franchisee data-sharing rules, regional PII regimes (GDPR / PIPEDA / CCPA / LGPD) are expressed as rules the substrate enforces *at write-time*, not PDFs the team has to remember. Violations are blocked before they land in the data surface.

4. **Programmable substrate.** Beyond the immediate scenario engine, the same platform supports cryptographic multi-party workflows: franchisee settlement, supply-chain attestation, programmable revenue splits between corporate and franchisee. Adjacent capabilities QSR can layer on without changing platform — but not in the initial scope.

**Important framing:** WeftOS does not replace the data lake. The lake remains the authoritative record of transactions. WeftOS sits above it holding the *reasoning* layer — the causal graph, the semantic index, the audit chain, the policy engine.

---

## Slide 4 — Size, speed, scale, hardware

**Title:** *"Fits on a commodity server. Scales by adding more."*

**Bullets (the numbers the VP will screenshot):**

**Storage (5-year retention, current year daily, prior years rolled up to weekly or monthly):**
- Total footprint: **~5 GB** across ~192 logical shards
- Per-shard hot / warm / cold tiering manages working set automatically
- Data-lake untouched; WeftOS stores aggregates + graph + audit + embeddings, not raw transactions

**Memory (HNSW index — the structure that makes semantic queries millisecond-class):**
- **~4–6 GB** resident for 5 years of enterprise-wide rolled-up data (~20M semantic points)
- Per-query working set: **~1 GB** — fits comfortably inside any production server

**Query and build:**
- Scenario query end-to-end: **< 1 second**
- Per-shard index build: **< 30 seconds**
- Full cold-start build: **< 1 minute** wall-clock, parallelised across shards

**Hardware sizing for QSR-scale production:**
- One dual-socket server, 64 GB RAM, 1 TB NVMe — already overprovisioned
- One analogous machine for warm replica / query fan-out / audit node
- Budget-class hardware. No specialised accelerators, no GPU dependency.

**Why that matters:** this is not a datacenter conversation. It's a two-rack-unit conversation.

---

## Slide 5 — What it takes to deliver

**Title:** *"13 weeks from kick-off to production handover."*

**Our side (already done):** Specification, ingest pipeline, scenario engine, gap analysis, hardening, audit, privacy. Built, tested, measured end-to-end against synthetic data. The hardest technical risks are behind us.

**Remaining work (~13 weeks from contract):**

| Phase | Weeks | What lands |
|---|---|---|
| Data-lake integration | 1–3 | Streaming read from QSR's lake, schema adapter, operational dashboards live on real data |
| Historical replay | 4–6 | One year of QSR history imported into staged shards |
| Calibration | 7–9 | Causal edge weights tuned against QSR's observed historical outcomes |
| Scenario acceptance | 10–11 | The VP-nominated headline scenario passes accuracy threshold |
| Production rollout | 12–13 | Dashboards, audit stream, runbook handover to QSR ops |

**Team:** 2–3 engineers from our side, full-time. One data-lake owner from QSR, part-time.

**What we need from QSR to start:**
1. Data-lake read access (schema + a sample, not full production yet)
2. Any existing A/B / holdout / staggered-rollout data — this calibrates the scenario engine from observational-only (bronze) to experimentally-grounded (gold)
3. Legal sign-off on franchisee data-sharing boundaries, so governance rules can be authored correctly
4. **A nominated first scenario** the VP wants working on day one — that becomes our acceptance test

**Not in scope for the initial engagement** (all addressable in a second phase if QSR wants): sub-hourly intra-store forecasting, menu-item-level supply modelling, franchisee self-service portal, cross-brand supply-chain attestation. Mentioned so the boundary is explicit, not because any of it is hard to add later.

---

## Deck meta

**Recommended delivery format:** 5 PowerPoint slides, one per section above. Slide 2 and Slide 4 should carry the tables; the other three can be mostly text with a single supporting visual each.

**Presenter notes** (don't put these in the deck itself, but keep them in your head):
- The VP cares about: *can you do it, how long, how much, what does it look like, what's the risk?* — this deck answers those in order.
- If they push on "how does it actually work under the hood": offer a follow-up working session. Do not pull out the implementation slides here.
- If they ask about blockchain / DeFi / Web3 specifically: lean on Slide 3 bullet 2 (cryptographic provenance) and bullet 4 (programmable substrate). Do not volunteer a crypto roadmap unsolicited — QSR leadership may or may not want that framing in the room.
- If they ask "why not just build this ourselves on Snowflake / Databricks": the one-line answer is *"you can build dashboards on those. You cannot build a governed, audit-chained, counterfactual reasoning layer on those. That's the WeftOS delta."*

**What this deck deliberately does not include:**
- Module or crate architecture
- Detailed data-model schemas
- The synthetic test harness and scoring machinery
- The causal-graph propagation internals
- Our own phase structure and test counts

Those live in a separate working-session document, to be shared after a mutual NDA or after kick-off, whichever comes first.
