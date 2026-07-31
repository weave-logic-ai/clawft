# EML heuristics tracking (WEFT-57)

**Date**: 2026-07-31  
**Ticket**: WEFT-57  
**Status**: Tracking page accepted (research backlog)  
**Source scan**: [`.planning/development_notes/eml-synergy-scan.md`](../../.planning/development_notes/eml-synergy-scan.md) (scanned 2026-04-04)  
**Audit parent**: `.planning/reviews/0.7.0-release-gate/03-pipeline-routing.md` (Deferred items / From eml-synergy-scan.md)

## Purpose

Single inventory of hardcoded heuristics flagged as EML replacement candidates.
This page **does not claim experiments were run**. It records status and review cadence
so the backlog is honest and greppable.

## Status vocabulary

| Status | Meaning |
|--------|---------|
| **scheduled** | Concrete follow-up work item or partial implementation path exists; next action is known |
| **deferred** | Valid EML candidate; no active 0.8 ship commitment; revisit on annual cadence or when training data appears |
| **won't-do** | Not a learnable heuristic (hardware limit, schema default, pure UI chrome, discrete label map) |

## Scope decision (0.8 / 0.9 / 1.0)

- **Out of 0.8 ship**: wholesale replacement of the 50+ candidates.
- **Related work already closed elsewhere** (do not re-open here):
  - **WEFT-53** — FitnessScorer weight decision (pipeline Q10).
  - **WEFT-54** — FitnessScorer error-indicator allowlist.
  - **WEFT-512** — “drive top 5 synergy-scan rows to implementation” (closed; verify code before treating as done — see Tier 1 notes).
- Default for remaining candidates: **deferred** under cycle **1.0.x** research tail unless a dedicated ticket is filed.

## Annual review cadence

| Field | Value |
|-------|--------|
| Cadence | **Annual** (every July, aligned with mid-year release planning) |
| Next review | **2027-07** |
| Owner | ws03-pipeline + ws17-research (whichever owns the module) |
| Exit criteria for a row | Training corpus exists **and** a Plane item is filed with measurable AC, **or** row reclassified **won't-do** with rationale |

Review checklist:

1. Re-scan for new magic numbers in the same modules (or `git log` since last review).
2. Confirm any **scheduled** row still has an open Plane item or delete the schedule.
3. Promote at most a handful of **deferred** rows only when offline labels / telemetry exist.
4. Update this file’s “Last reviewed” line and link any new WEFT-N items.

**Last reviewed**: 2026-07-31 (initial inventory from eml-synergy-scan; no new EML swaps claimed).

---

## Master inventory

Line numbers refer to the 2026-04-04 scan. Paths may have drifted; treat module + description as the identity.

### graphify / analyze (surprise + structure)

| ID | Location (scan) | Heuristic | Status | Notes |
|----|-----------------|-----------|--------|-------|
| G-A01 | analyze.rs:209-213 | Confidence ordinal → surprise bonus | deferred | Part of composite surprise scorer |
| G-A02 | analyze.rs:226 | Cross file-type bonus `+2` | deferred | Composite |
| G-A03 | analyze.rs:232 | Cross-repo/dir bonus `+2` | deferred | Composite |
| G-A04 | analyze.rs:242 | Cross-community bonus `+1` | deferred | Composite |
| G-A05 | analyze.rs:248 | Semantic similarity `×1.5` | deferred | Composite |
| G-A06 | analyze.rs:254-255 | Peripheral-hub degree gap + bonus | deferred | Composite |
| G-A07 | analyze.rs:204-269 | **Composite surprise scorer** (7 feats) | deferred | Tier-1 #1; WEFT-512 claimed top-5 — no EmlModel wiring observed in graphify analyze as of this review |
| G-A08 | analyze.rs:526 | Bridge betweenness approx | deferred | |
| G-A09 | analyze.rs:534 | `.take(3)` bridge nodes | deferred | Low priority; adaptive top-N |
| G-A10 | analyze.rs:582 | `god_nodes(..., 5)` | deferred | |
| G-A11 | analyze.rs:590 | `inferred.len() >= 2` question threshold | deferred | |
| G-A12 | analyze.rs:661 | Low-cohesion community threshold | deferred | Tier-1 #2 |
| G-A13 | analyze.rs:373-374 | Inverse degree product betweenness | deferred | |
| G-A14 | analyze.rs:79 | Stub file node `degree <= 1` | deferred | |

### graphify / cluster

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| G-C01 | cluster.rs:13 | `MAX_COMMUNITY_FRACTION = 0.25` | deferred | Tier-2 #8 |
| G-C02 | cluster.rs:15 | `MIN_SPLIT_SIZE = 10` | deferred | With G-C01 |
| G-C03 | cluster.rs:119 | Label-prop max 50 iters | won't-do | Safety valve; learning convergence predictor is not worth it |
| G-C04 | cluster.rs:236 | Cohesion round to 2 decimals | won't-do | Display formatting, not a decision heuristic |

### graphify / export html (viz)

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| G-H01 | html.rs:18 | `MAX_NODES_FOR_VIZ = 5000` | won't-do | Hardware / UX hard cap |
| G-H02 | html.rs:52 | Node size linear in degree | deferred | Tier-2 #7 |
| G-H03 | html.rs:53-56 | Label visibility top 15% degree | deferred | Tier-2 #7 |
| G-H04 | html.rs:94-95 | Edge width/opacity by confidence | deferred | Minor |
| G-H05 | html.rs:246-253 | ForceAtlas2 6-param physics + stabilization | deferred | Tier-2 #6 composite physics tuner |
| G-H06 | html.rs:257 | `tooltipDelay: 100` | won't-do | UI preference |
| G-H07 | html.rs:263 | Edge roundness 0.2 | won't-do | Cosmetic |
| G-H08 | html.rs:389 | Hyperedge hull 1.15 | deferred | Low |
| G-H09 | html.rs:376,394 | Hyperedge alpha | won't-do | Cosmetic |

### graphify / pipeline + report + forensic

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| G-P01 | pipeline.rs:46,48,49 | Default top-N god/surprise/questions | deferred | Adaptive top-N |
| G-R01 | report.rs:178 | `.take(8)` labels | won't-do | Report layout |
| G-R02 | report.rs:215,222,225 | Isolation / thin community / ambiguity % | deferred | Tier-3 #13 |
| G-F01 | forensic.rs:141 | Unlinked evidence `deg <= 1` | deferred | |
| G-F02 | forensic.rs:227 | Coherence `density * avg_confidence` | deferred | Tier-1 #3 |
| G-F03 | forensic.rs:260-273 | Counterfactual delta (linear model) | deferred | Same model as G-F02 |
| G-B01 | build.rs:127 | Default edge weight 1.0 | won't-do | Schema default |

### kernel

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| K-G01 | governance.rs:141-156 | EffectVector L2 / per-dim thresholds | deferred | Tier-1 #4 |
| K-G02 | governance.rs:359 | Global magnitude threshold | deferred | With K-G01 |
| K-G03 | governance.rs:448 | Production `×0.5` risk threshold | deferred | Tier-3 #15 |
| K-S01 | supervisor.rs:65-66 | Restart budget 5/60s | deferred | Tier-2 #9 |
| K-S02 | supervisor.rs:102-104 | Backoff base + 30s cap | deferred | Tier-2 #9 |
| K-S03 | supervisor.rs:219 | Resource warn at 80% | deferred | Tier-3 #16 |
| K-H01 | health.rs:158-164 | Liveness/readiness intervals + fail/success thresholds | deferred | Tier-2 #10 |
| K-K01 | cluster.rs:134-143 | Heartbeat / suspect / unreachable | deferred | Tier-2 #11 |
| K-D01 | dead_letter.rs:17 | DLQ capacity 10_000 | won't-do | Memory budget |
| K-X01 | assessment/analyzers/complexity.rs:39 | `line_count > 500` | deferred | Tier-3 #14 |

### llm

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| L-R01 | retry.rs:23-28 | max_retries / base / max delay / jitter | deferred | Tier-2 #12 |
| L-R02 | retry.rs:64 | `2^n` exponential backoff shape | deferred | Optional curve learning |
| L-R03 | retry.rs:186 | mpsc buffer 256 | won't-do | Capacity tuning |
| L-RT01 | router.rs | (none found) | won't-do | Deterministic prefix routing |
| L-F01 | failover.rs | (none found) | won't-do | Ordered chain |

### weave bench

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| W-B01 | bench_cmd.rs weights 25/25/20/15/15 | Dimension weights | **scheduled** | Tier-1 #5; experimental path in `crates/clawft-weave/src/commands/bench_eml.rs` (`EmlModel` scorers + composite). Default `bench_cmd` literals remain authoritative until trained offline and promoted |
| W-B02 | bench breakpoints (throughput/latency/scalability/stability/endurance) | Piecewise linear scorers | **scheduled** | Same `bench_eml` path |
| W-B03 | Grade thresholds A+/A/… | Discrete labels | won't-do | Label map, not continuous model |
| W-B04 | Scalability/endurance quality strings | Classification bands | deferred | Cosmetic/report; can ride W-B01 if ever learned |

### pipeline-adjacent (called out in WEFT-57 description, outside scan tables)

| ID | Location | Heuristic | Status | Notes |
|----|----------|-----------|--------|-------|
| P-S01 | pipeline FitnessScorer weights 0.4/0.2/0.2/0.2 | Fitness fusion | deferred | Decision tracked under **WEFT-53** (closed); weights frozen unless reopened |
| P-A01 | assessment/effects.rs weighted scores | Assessment effects | deferred | Needs labeled outcome corpus |
| P-L01 | LLM provider cost-vs-quality ($0.01/1K class thresholds) | Cost gate | deferred | Economics policy; not pure EML |

### modules scanned with zero candidates

skills / skills_v2 / context, mesh_assess, domain/code — **won't-do** (no hardcoded scoring heuristics found in scan).

---

## Priority tier summary (from scan)

| Tier | Scan IDs | Tracking IDs | Disposition |
|------|----------|--------------|-------------|
| Tier 1 | composite surprise, cohesion Q, forensic coherence, governance magnitude, bench scorers | G-A07, G-A12, G-F02, K-G01, W-B01/02 | deferred except bench **scheduled** via `bench_eml` |
| Tier 2 | ForceAtlas2, node size, community split, restart/backoff, health probes, cluster HB, retries | G-H05, G-H02/03, G-C01/02, K-S01/02, K-H01, K-K01, L-R01 | deferred |
| Tier 3 | report thresholds, complexity 500, prod risk ×0.5, resource 80% | G-R02, K-X01, K-G03, K-S03 | deferred |

## Counts (this inventory)

| Bucket | Count |
|--------|------:|
| Rows with status **deferred** | ~48 |
| Rows with status **scheduled** | 2 (W-B01, W-B02) |
| Rows with status **won't-do** | ~15 |
| Scan original “EML-replaceable” claim | 56 |
| Distinct composite EmlModel targets if ever pursued | ~16 |

## Explicit non-claims

- No offline training runs, MSE tables, or A/B product experiments are attached to this ticket.
- Closing WEFT-57 means **the inventory and cadence exist**, not that heuristics were replaced.
- If WEFT-512’s “top 5 implemented” claim is audited later and found incomplete, **do not re-open WEFT-57** — file module-specific tickets from this table.

## Related docs

- [hnsw-eml-opportunities.md](./hnsw-eml-opportunities.md) — HNSW-specific EML opportunities (WEFT-58)
- [eml-attention-iter3-gate.md](./eml-attention-iter3-gate.md) — attention Iter-3 gate (WEFT-41)
- `.planning/development_notes/eml-synergy-scan.md` — source scan
