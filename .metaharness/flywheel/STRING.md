# Flywheel string (canonical)

```
SEE → WIRE → BUILD → UPSTREAM
```

**Expand:**

1. **SEE** — capability exists; agents can’t find it (index, patterns, tasks, doctor, brain).  
2. **WIRE** — capability exists; not on the agent path (MCP, gate, seed, score+genome).  
3. **BUILD** — missing product feature.  
4. **UPSTREAM** — must live in rUv/Cognitum (or we contribute reference).

**Companion truths:**

- Alignment and honest scores are **compatible** when score is a side-effect of SEE/WIRE (and real BUILD).  
- Vanity ADR-041 hacks without a mode are **rejected**.  
- Darwin view of the loop: **traverse the capability graph → compare WeftOS ↔ rUv/Cognitum → classify each node SEE|WIRE|BUILD|UPSTREAM → mutate harness/policy only → measure → promote.**

**Pathfinder:** Grok (executor) + Ruflo (orchestrator) + MetaHarness (promote) + Cognitum (gates/meter when cloud) + WeftOS (kernel/edge).

**Sensor-synergy analogy (for code/features):** multi-source fusion over the capability DAG — not one score sensor. Layer signals (foundation, genome, alignment axes, dual-host, TileZero, crates) the way Graph Views layer BVH + live + chain.

## Run the loop (keep going)

```bash
npm run metaharness:loop
# = brain index → crosscut classify → darwin dry plan + flywheel measure
```

**Yes — looping is the point.** Stop only when **goals** hold (not when bored):

| Goal | Met when |
|------|----------|
| Foundation | `weftosFoundationScore >= 80` (target 100) |
| Genome | publish_readiness ≥ 0.8 and test_confidence ≥ 0.8 |
| Alignment | flywheel measure `ok` (Grok path, patterns, Cognitum gate, pins, …) |
| Graph maturity | crosscut has **no blocking BUILD** on pathfinder nodes, or only approved deferred |
| Open WIRE | Prefer drive WIRE → SEE each generation; UPSTREAM tracked not blocked |

Each generation: pick one **WIRE** (or one BUILD if no WIRE) → change harness surface → re-index → crosscut → measure. MetaHarness is the instrument; **WeftOS maturity** is the product.

WeftOS brain: `.metaharness/brain/README.md`  
Crosscut JSON: `.metaharness/brain/crosscut-latest.json`
