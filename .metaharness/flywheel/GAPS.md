# WeftOS × MetaHarness flywheel — gap matrix (ecosystem-first)

## The string

```
SEE → WIRE → BUILD → UPSTREAM
```

**Primary docs:** `.metaharness/flywheel/STRING.md` ·
`docs/research/ruv-ecosystem-synergy-flywheel.md`  
**Not the goal:** hacking ADR-041 scorecard dimensions.  
**Goal:** cross-compatibility with **rUv + Cognitum** + **Grok+Ruflo** pathfinding
(+ Cognitum gates/MaaS when cloud). Darwin: **traverse → compare → classify →
mutate harness → measure → promote.** Optional: WeftOS brain (S7) so compare is
a graph join with ruvbrain.

## Target architecture (rUv)

```
READ (ADR-150)              WRITE (ADR-153)              PROMOTE (flywheel)
score + genome              darwin evolve                meetsPromotionRule
mcp-scan / threat / oia     .metaharness/variants only   Ed25519 + lineage
                            --confirm required           holdout + frozen anchor
```

Grok = executor · Ruflo = orchestrator · MH = evolve/promote · WeftOS Rust = product.

## Intervention modes (prefer in order)

| Mode | Do this when… |
|------|----------------|
| **SEE** | Already built; agents can’t find it (index, patterns, tasks, doctor) |
| **WIRE** | Exists but not on agent path (MCP, gate, seed, score+genome together) |
| **BUILD** | Missing product capability |
| **UPSTREAM** | Must live in rUv/Cognitum (host-grok, inventory, Seed/Fugu contracts) |

Many “gaps” are SEE/WIRE on assets we already have (crates, TileZero, genome, team bus).

## Alignment axes (what “good” means)

| Axis | Proof | Status |
|------|-------|--------|
| Dual-host pattern recall | Grok store → Claude/Ruflo retrieve (patterns ns) | Partial (seed + rules; no automated round-trip) |
| Grok+Ruflo pathfinding | Feature work without Claude Code host | **In progress** (`.grok/`, team bus) — pathfinder |
| Fusion promote discipline | ViewSpec eval → receipt → human promote | Partial (eval+measure; no formal promote) |
| Removability (ADR-150) | `build.sh gate` without Node MH | **Have** |
| World twin vocabulary | Views ↔ WorldGraph provenance map | Research only |
| Pin honesty | ruflo pin documented + receipt on bump | Partial (`weftos.rufloPin`) |
| Router optional | `@metaharness/router` + savings | **Gap** |
| Cognitum TileZero | `cognitum-gate-tilezero` + `tilezero` feature | **Have** (dep); test depth TBD |
| Cognitum MaaS/Fugu | metered tiers, cog_ keys, approval pods | **Gap** (optional provider) |
| Cognitum Seed peer | edge vector pair/push/query | **Gap** (doc + optional probe) |

## Tooling status

| Capability | Status |
|------------|--------|
| Foundation + OS surface score | Have |
| Genome + scorecard READ | Have |
| Flywheel measure + signed receipts | Have (`flywheel-measure.mjs`) |
| Multi-gen promote with real proposers | Gap |
| Darwin WRITE | Gap (needs approval) |
| Upstream `host-grok` | **Missing in rUv** — we invent overlay |
| MCP metaharness_* from Grok | Partial / scripts fallback |
| Production promote keys | Gap |

## Big changes — need your approval

| ID | Change | Why |
|----|--------|-----|
| **S1** | Package **Grok host reference** for upstream contribution | Pathfinder → rUv host matrix |
| **S2** | Optional `@metaharness/router` + savings ledger | Align ADR-148/149 |
| **S3** | Darwin evolve wrapper (`--confirm`, variants only) | WRITE layer |
| **S4** | ViewSpec ↔ WorldGraph schema bridge | Twin stack interop |
| **S5** | CI promote keys + `verifyReplayBundle` | Real provenance |
| **S6** | Dual-host pattern round-trip CI | Memory cross-compat proof |
| **C1** | Cognitum/Fugu-compatible optional LLM provider | Cloud path matches MaaS |
| **C2** | Seed peer map + probe | Edge hardware story vs Seed |
| **C3** | TileZero receipt CI smoke | Gate kinship with promote |
| **S7** | **WeftOS brain** | **Scaffolded** — `weftos-brain.mjs` (216 docs) |
| **S8** | **crosscut** | **Scaffolded** — `crosscut.mjs` + research table |

## Frozen without human ADR

ECC R1–R5 · gate phases (stricter only) · dual-sign kinds · substrate default-deny direction

## Anti-goals

- Primary KPI = ADR-041 memoryUsefulness / taskCoverage  
- Silent ViewSpec or ECC policy promote  
- `weft` runtime depends on Node/MH  
- Fork MH for score formulas  
