# WEFT-277 result — honest_affordances real GEPA / governance intersection

**Ticket:** WEFT-277  
**Branch:** `wave0l/weft-277-honest-affordances`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-277 (wave-0l)

## Problem

`honest_affordances` after WEFT-430 filtered on session permits
(manifest `influences` + ADR-012 capture grants) but still lacked the
**goal-aggregate / GEPA** half of ADR-006 rule 2:

```text
affordances = raw ∩ governance.permit(caller, primitive, resource)
```

Audit gap (T08-33 / compositional-ui ADR-008): the hook existed; the
policy contract did not. GEPA-gated governance intersection had been
deferred to “M2 active-radar loop”.

## What shipped

### Policy contract — `ComposeGovernance`

New module `clawft-surface/src/compose/governance.rs`:

| Field | Role (ADR-008) |
|-------|----------------|
| `goal_id` | Active goal id (default `adhoc-scratch`) |
| `granted_affordances` | Optional allow-set (name **or** normalised verb) |
| `denied_affordances` | Deny set; **deny wins** over grants |
| `effect_ceiling` | Optional 5-D ceiling (`risk` / `fairness` / `privacy` / `novelty` / `security`) |
| `gepa_mutate_allowed` | When `false`, hide `mutate` / `surface.mutate` / `*.mutate` |
| `surface_scope` | Optional node path prefix (resource scope) |

Constructors: `open()`, `closed()`, builders `with_goal_id`,
`with_grants`, `with_denies`, `with_effect_ceiling`,
`with_gepa_mutate_allowed`, `with_surface_scope`.

Helpers: `normalize_verb`, `is_gepa_mutate_verb`,
`estimate_verb_effect` (compose-time heuristic; kernel double-gate
still authoritative at dispatch).

### Intersection wiring

`ComposePermits` gains `governance: ComposeGovernance` (default open).

```text
honest_affordances =
  influences ∩ capture_grants ∩ ComposeGovernance.allows(node, aff)
```

- `node` is used for `surface_scope` path checks.
- Hosts that never set governance keep WEFT-430 behaviour.
- Builder: `ComposePermits::… .with_governance(ComposeGovernance::…)`.

### ADR-006 alignment note

Added to
`.planning/symposiums/compositional-ui/adrs/adr-006-primitive-head.md`
under rule 2 — documents WEFT-430 + WEFT-277 layering and residual
kernel Goal aggregate ownership.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Define governance/GEPA policy contract | **Done** — `ComposeGovernance` + `EffectCeiling` |
| Implement intersection: only return affordances permitted by current governance state | **Done** — layered into `honest_affordances` |
| Smoke test with allow + deny fixtures | **Done** — unit tests (grant, deny-wins, scope, GEPA, ceiling) |
| Update ADR-006 alignment note | **Done** |

## Tests

```bash
cargo test -p clawft-surface --lib compose::
```

- **compose unit tests:** 23 passed (13 WEFT-430 permit + 10 WEFT-277 governance)

## Residual / follow-ups

1. **Kernel Goal aggregate** — full ADR-008 aggregate + `governance.goal.*`
   chain events not yet substrate-resident; hosts must build
   `ComposeGovernance` from whatever goal source they have.
2. **Desktop host wiring** — admin shell still uses open governance
   (manifest permits only). Attach active-goal snapshot when Goal
   tray / envelope lands.
3. **Effect estimate parity** — compose heuristics are independent of
   `clawft-core::agent::effects`; keep aligned when tool table grows.
4. **Active-radar return schema** (WEFT-283) — variant-id echo still
   separate from this honesty filter.

## Files

| Path | Change |
|------|--------|
| `crates/clawft-surface/src/compose/governance.rs` | **New** — policy contract + unit tests |
| `crates/clawft-surface/src/compose/runtime.rs` | Wire governance into `ComposePermits` / `honest_affordances` |
| `crates/clawft-surface/src/compose/mod.rs` | Re-exports |
| `crates/clawft-surface/src/lib.rs` | Public API + scope note |
| `crates/clawft-surface/src/tree.rs` | AffordanceDecl doc |
| `.planning/symposiums/compositional-ui/adrs/adr-006-primitive-head.md` | Alignment note |
| `docs/plans/wave-0l-WEFT-277-result.md` | This result |
