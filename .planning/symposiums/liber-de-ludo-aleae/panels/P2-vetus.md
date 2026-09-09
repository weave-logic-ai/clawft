# P2 — Room Vetus

**Phase**: I — separate rooms. Vetus only.
**Date**: 2026-08-13 (first Grok host)
**Chair inside the room**: scoring architect (inventory), not Cardano.
**Forbidden**: inventing Cardano's new doctrine; writing the combining contract; visiting Room Nova.

---

## Who sat

| Seat | Voice | What they owned in this room |
|------|-------|------------------------------|
| `governance-counsel` | three-branch gate, EffectVector, trajectory | V1–V2, V12, V17, V19–V20, V28–V30. Flagged their own briefing as a marked deck. |
| `ecc-analyst` | spectral / HNSW / coherence | V10, V23–V25. Named the four-way *coherence* homonym. |
| `defi-networker` | bond / slash / trust ladder | V11. Economic stake is not epistemic odds. |
| scoring architect (`weftos-scoring-architect`) | every numeric surface | V3–V9, V13–V16, V18, V21–V22, V26–V27, V31. Tabulated circuit? odds? edge? ruin? calibration? without proposing a merge. |

Recorder for the seed was `doc-weaver`. This panel deepens that seed; Nova still may not edit `deliverables/04-existing-spaces.md`.

---

## What was inspected

Read, not guessed. Symbols cited below are the load-bearing ones.

| Path | Symbols / section |
|------|-------------------|
| `docs/adr/adr-034-effect-algebra-scoring.md` | 5D `EffectVector`, L2, default `risk_threshold` 0.7, C9 deferred |
| `crates/clawft-core/src/agent/effects.rs` | `EffectVector`, `magnitude`, `effect_for_tool`, `effect_for_binding_thread` |
| `crates/clawft-kernel/src/governance.rs` | kernel `EffectVector`, `GovernanceDecision`, `GateEffectKind`, `TrajectoryRecord`, `GovernanceEngine::risk_threshold` |
| `crates/clawft-kernel/src/gate.rs` | kernel `GateDecision` {Permit, Defer, Deny} |
| `crates/clawft-core/src/agent/gate.rs` | core `GateDecision` {Permit, Defer, Deny} |
| `crates/clawft-core/src/scoring.rs` | RVF `QualityScorer`, `NoopScorer` (0.5), `BasicScorer` |
| `crates/clawft-core/src/pipeline/scorer.rs` | pipeline `NoopScorer` (1.0), `FitnessScorer`, `DEFAULT_ERROR_INDICATORS` |
| `crates/clawft-core/src/pipeline/traits.rs` | `QualityScore`, `Trajectory` |
| `crates/exo-resource-tree/src/scoring.rs` | `NodeScoring`, `SCORING_DIMS`, `blend`, `to_hash_bytes` |
| `crates/clawft-core/src/agent/cost_budget.rs` | `ConversationBudget`, `BudgetUsage.circuit_open` |
| `crates/clawft-core/src/planning.rs` | `PlanningConfig.circuit_breaker_no_op_limit`, `TerminationReason::CircuitBreaker` |
| `crates/clawft-core/src/complexity.rs` | `TaskComplexityAnalyzer::analyze` |
| `crates/clawft-services/src/delegation/mod.rs` | `DelegationEngine::complexity_estimate` |
| `crates/clawft-kernel/src/eml_kernel.rs` | `GovernanceScorerModel` |
| `crates/clawft-kernel/src/eml_coherence.rs` | `CoherencePrediction` {`lambda_2`, `fiedler_norm`, `uncertainty`} |
| `crates/clawft-graphify/src/domain/forensic.rs` | `coherence_score` (density × confidence) |
| `crates/clawft-cli/src/commands/assess_cmd.rs` | assessment `coherence_score` (doc/code × 100) |
| `docs/skills/clawft/SOUL.md` | Minimax + EV commandment |
| `docs/weftos/k2-symposium/04-industry-landscape.md` §5 | Explicit UQ gap |
| `agents/weftos/governance-counsel.md` | Drifted 5D sketch |
| `agents/weftos/ecc-analyst.md` | Spectral / coherence toolkit |
| `agents/weftos/defi-networker.md` | Bond / slash / trust ladder |
| `docs/adr/adr-096-metaharness-foundation.md` | Score / genome / flywheel / no auto-promote |
| `docs/guides/routing.md` | KeywordClassifier live; 7-factor **stub** |
| `docs/guides/auto-delegation-classifier.md` | WEFT-201 remaining-work classifier |
| `docs/research/kolbe-conative-integration.md` | Conative style, not odds |

---

## Live instrument (this session)

MetaHarness score on `weftos` — cited as given, not re-thrown:

| Die | Face | This cast |
|-----|------|-----------|
| harnessFit | 75 | |
| compileConfidence | 100 | |
| taskCoverage | 65 | |
| toolSafety | 90 | |
| memoryUsefulness | 53 | |
| estCostPerRunUsd | 0.024 | not a sixth die; a price tag |

Five named faces, one cost sidecar, **no *n***, **no interval**, **no declared favorable set**. ADR-096's own baseline (2026-07-31) was `harnessFit 75 / taskCoverage 65 / memoryUsefulness 51`. Fit and coverage have not moved; memory ticked 51 → 53. That is a snapshot pair, not a calibration.

---

## Inventory (classed, not merged)

Full table with citations lives in [`deliverables/04-existing-spaces.md`](../deliverables/04-existing-spaces.md). Seed V1–V18 survive. Vetus **adds** V19–V31 after reading the files. Classification only: **rhyme** (same question), **cousin** (same family, different primitive), **false friend** (shared word, different meaning).

Corrections to the seed, spoken in the room:

1. **V2 is two gates, not one.** `GovernanceDecision` is four-way (`Permit` / `PermitWithWarning` / `EscalateToHuman` / `Deny`). Both `GateDecision` enums are three-way (`Permit` / `Defer` / `Deny`). The seed's "Permit / Warn / Escalate / Deny" is the governance enum with nicknames. Do not collapse them.
2. **V6 is two breakers, not one.** `ConversationBudget` (WEFT-322, spend/tokens/iterations) and `PlanningConfig.circuit_breaker_no_op_limit` (consecutive no-ops). Same English word. Same *stop-loss* family. Different trips.
3. **V8's "ruvllm 7-factor" is a stub.** `docs/guides/routing.md` §10 lists it under unimplemented. Live complexity is `KeywordClassifier` + `TaskComplexityAnalyzer::analyze` + `DelegationEngine::complexity_estimate`.
4. **V10 does not own the word *coherence*.** Four in-tree scores share the name and none share a formula (see homonyms below).
5. **Threshold house-rule is not even one house rule.** ADR-034 and `GovernanceEngine` tests use `0.7`. `effect_for_tool("agent_spawn")` and the binding-thread mismatch path assert against `0.8`. Two bars on the same table.

Every live surface: **no enumerated circuit, no odds, no house-edge index, no ruin probability, no calibration of a declared *p***. Closest approaches: `CoherencePrediction.uncertainty` (a model head, not a circuit interval) and `NodeScoring::blend` (EMA over time, not *n* of a named sample space).

---

## The circuit / circuit-breaker homonym

Spoken by the scoring architect; `defi-networker` nodded at *ruin*; `governance-counsel` refused the rename.

WeftOS already spent the word **circuit** on *stop-loss*. Cardano's *circuitus* is *sample space*. Room Vetus verdict, unchanged from the seed: **keep both words, never collapse them.**

| Word in-tree / in-book | Meaning we already have | Do not |
|------------------------|-------------------------|--------|
| `BudgetUsage.circuit_open` | Conversation spend/token/iteration cap tripped | Do not call this a circuitus |
| `PlanningConfig.circuit_breaker_no_op_limit` | Abort after N consecutive no-ops (default 3) | Do not rename to circuitus |
| `TerminationReason::CircuitBreaker` | The no-op abort reason | Same |
| routing.md Level-2 "CircuitBreaker" | **Stub** — provider demotion when slow/erroring | Do not inventory as shipped |
| *circuitus* / circuit (the book) | Enumerated outcomes of a score | Do not hang this name on a cap |

`stake` is fine in both rooms (tokens, time, treasury, bond). `fairness` is **not** already the book's equality-of-conditions — see V1.

---

## Governance-counsel EffectVector drift (marked deck)

`governance-counsel` read their own briefing aloud.

`agents/weftos/governance-counsel.md` still sketches:

```rust
pub struct EffectVector {
    pub cpu: f32,
    pub memory: f32,
    pub network: f32,
    pub storage: f32,
    pub trust_delta: f32,
}
```

and a CLI example `weave governance evaluate --vector '{"cpu":0.8,"memory":0.6,"network":0.3}'`.

ADR-034 and both live structs (`clawft_core::agent::effects::EffectVector`, `clawft_kernel::governance::EffectVector`) use `risk`, `fairness`, `privacy`, `novelty`, `security`.

Two dice on the same table, inside our own agent briefing. This is not a Cardano import. It is an **unequal-conditions** example we already own. Coniunctio may use it as a worked marked-deck; Vetus does not "fix" the agent file in this room.

Related in-house inequality (same family, not the same card): default magnitude bar `0.7` (ADR-034 / engine tests) vs `0.8` (`agent_spawn` D6 test, binding-thread mismatch).

---

## Three already-honest scores

Closest things WeftOS has to a *cast you can recount*. None of them name a circuit.

### 1. MetaHarness 5-die (this session)

`75 / 100 / 65 / 90 / 53`, cost `0.024` USD. Faces are named. Favorable set is not. `memoryUsefulness 53` has no interval. Flywheel receipts (ADR-096) are the nearest thing to *n* casts — and they are optional, off the `weft` runtime path.

### 2. EffectVector L2

```
magnitude = sqrt(risk² + fairness² + privacy² + novelty² + security²)
max = √5 ≈ 2.236
```

`EffectVector::magnitude` in both core and kernel. `GovernanceScorerModel::predict` falls back to the same L2 when untrained. Fast. Silent about how the five faces were assigned. `effect_for_tool` ignores `args` today (`args_are_currently_ignored`). Unknown tools are the zero vector (silent Permit). `fairness` is a 0–1 "equitable treatment" vibe, not equality-of-conditions.

### 3. FitnessScorer

Weighted sum: task_completion 0.4 + efficiency 0.2 + tool_accuracy 0.2 + coherence 0.2 (`FitnessScorerWeights`, frozen for 0.8.x). Refusal substring hits `DEFAULT_ERROR_INDICATORS` once for −0.2. Documented as **not** a safety control (WEFT-54). That honesty is already the right kind of claim: name what the die cannot do.

Sibling blank dice: pipeline `NoopScorer` always `1.0`; RVF `NoopScorer` always `0.5`. Same name, two constants.

---

## Homonyms the room will not collapse

| Word | In-tree meanings (do not merge) |
|------|----------------------------------|
| **circuit** | stop-loss breaker vs Cardano sample space |
| **fairness** | EffectVector dim vs equality-of-conditions |
| **coherence** | Fitness readability; ECC/EML λ₂; assessment doc/code %; forensic density×confidence |
| **trajectory** | pipeline `Trajectory` (request/routing/response/quality) vs governance `TrajectoryRecord` (action/outcome FIFO) |
| **NoopScorer** | 1.0 (pipeline) vs 0.5 (RVF) |
| **GateDecision** | two crates, same three variants, different token/receipt shapes |
| **uncertainty** | K2 gap on EffectVector vs `CoherencePrediction.uncertainty` (EML head, fallback `lambda_2 * 0.5`) |

---

## What Vetus hands Coniunctio

- The classed table **V1–V31** in `deliverables/04-existing-spaces.md` (seed V1–V18 plus surfaces the seed missed).
- The circuit / circuit-breaker homonym, with the extra stub (routing Level-2) called out as unshipped.
- The four-way *coherence* homonym and the two-Noop / two-gate / two-trajectory twins.
- The three live scores, including this session's MH cast.
- The governance-counsel 5D drift, plus the 0.7 / 0.8 threshold split, as **marked-deck** examples of unequal conditions we already host.
- A request, not a contract: **do not smash a new 5D into genesis**. Sidecar any Cardano-shaped score first. C9 stays deferred until a sidecar proves lift.
- Explicit non-delivery: **no combining contract, no LDA-ADR text, no Cardano arithmetic**. Those wait for Nova's primitives and for P3.

Room Nova is not allowed to edit the inventory file. Coniunctio may cite it.

---

## Room close

Vetus counted the dice already on the table. They are many. They do not share a sample space. They sometimes do not even share a name honestly. That is the whole finding.
