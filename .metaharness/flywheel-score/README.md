# MetaHarness score flywheel (WEFT-730)

Dedicated harness loop to **improve WeftOS** under MetaHarness discipline,
starting with score dimensions and real foundation assets.

> Freeze the model. Evolve the harness / policies. Promote only what proves lift.

## Two score systems

| Score | Meaning | Ceiling |
|-------|---------|---------|
| **weftosFoundationScore** | Tasks, ViewSpecs, anchors, patterns, ADRs, surfaces | **100** (target) — `scripts/metaharness/weftos-score.sh` |
| **Upstream ADR-041** (`metaharness score`) | Shallow HIGH_SIGNAL inventory + recommended archetype surface | **Structural caps** — see `ceilings.md` |

**Primary objective:** keep `weftosFoundationScore ≥ 80` and expand *real*
capability (tests, MCP safety docs, fusion fixtures, pattern quality).

**Secondary:** push upstream dimensions where the formula allows (e.g.
`toolSafety` already 90; `compileConfidence` 100; `harnessFit` via clearer
MCP/agent signals). Do **not** claim ADR-041 100/100 if the formula cannot
reach it.

## Loop

```bash
# 1. Measure
scripts/metaharness/score.sh
scripts/metaharness/weftos-score.sh

# 2. Evaluate candidates against anchors (no champion mutation)
scripts/metaharness/flywheel-score-eval.sh

# 3. Propose improvements (human or Darwin on harness surfaces only)
#    - more tasks / commands
#    - richer ViewSpec fixtures
#    - AgentDB patterns
#    - CONTRIBUTING / MCP policy files for inventory
#    - never auto-weaken gate or ADR-090 R1–R5

# 4. Promote only with confirm + PR / keys
```

## Surfaces allowed to mutate (candidates)

| Surface | Examples |
|---------|----------|
| harness tasks | `.metaharness/tasks/*` |
| ViewSpec fixtures | `config/views/*` |
| patterns | `seed-patterns.sh` keys |
| docs/score tooling | `weftos-score.sh`, CONTRIBUTING |
| MCP policy pointers | catalog path for scan cleanliness |

## Surfaces frozen without human ADR

- ECC R1–R5 (ADR-090)
- Substrate security denylists (default-deny direction only)
- Dual-sign / chain mandatory kinds
- Gate phase requirements (can only get stricter via promote)

## Plane

WEFT-730 parent. Child improvements should open follow-up Plane items when
they are multi-session product work (not just fixture tweaks).
