# ADR-017: GEPA Prompt Evolution for pipeline/learner.rs

**Date**: 2026-03-28
**Status**: Accepted (amended 2026-07-30 — WEFT-38 production wiring)
**Deciders**: Hermes Integration Analysis + Sprint 11 Symposium; Wave 0i coder-38

## Context

The clawft pipeline includes a 6-stage processing chain: Classifier → Router →
Assembler → Transport → Scorer → Learner. Early versions shipped a no-op
`NoopLearner`. Hermes's GEPA (Genetic Evolution of Prompt Architectures)
pattern shows that genetic evolution of prompts, using quality scores as
fitness, creates a self-improvement flywheel — provided governance and
provenance constraints stay in place.

By 0.7.x the codebase had:

- `FitnessScorer` / `TrajectoryLearner` (ring buffer, `evolution_ready` flag)
- string-level mutation operators in `pipeline/mutation` (rephrase, add
  examples, remove ineffective, emphasize)
- stage 3.5 `apply_prompt_evolution` in `PipelineRegistry::complete`

but **no GA population** (selection / crossover / elitism), **no
persistence**, and the audit gap (task 03-17 / WEFT-38) still described the
flywheel as "shipped as code, not as a running feature."

## Decision

### Learning + mutation surface

1. Expand stage 6 to `TrajectoryLearner` (Level 1), retaining `NoopLearner`
   as the config default (`pipeline.learner = "noop" | "trajectory"`).
2. Use `QualityScorer` overall / relevance / coherence as the fitness signal
   that increments `poor_count` and eventually sets `evolution_ready`.
3. Keep mutation **LLM-free** for v1: deterministic string operators plus a
   deterministic GA loop (no `rand` dependency; tournament salt + strategy
   cycle). LLM-backed reflection remains a future level.

### WEFT-38 production wiring (chosen design)

| Concern | Choice |
|---------|--------|
| **Trigger surface** | **In-request stage 3.5** — `PipelineRegistry::complete` / `complete_stream` call `apply_prompt_evolution` → `LearningBackend::evolve_prompt` when the learner has set `evolution_ready`. No separate background tick or admin RPC in this ticket; a daemon tick can call `evolve_prompt` later without changing the GA API. |
| **GA loop** | `pipeline/mutation/ga.rs`: `Population`, `PromptCandidate`, `GaConfig`, `evolve()`, tournament selection, line-level crossover, elitism, heuristic fitness over `TrajectoryHint`s. |
| **Operators** | Existing strategies in `mutation/strategies.rs` used both to diversify the seed population and as mutation operators inside each generation. |
| **Persistence** | Optional JSON via `Population::save` / `load`. `TrajectoryLearnerConfig.population_path` + `with_population_path`; written after each successful evolution. Substrate/kernel causal lineage is **out of scope** (follow-up). |
| **Acceptance of offspring** | `select_evolved_prompt` applies `GaConfig.min_improvement` (default `0.0`). |
| **Skill path** | WEFT-66 already shares the same `TrajectoryLearner` Arc with skill autogen; prompt-level GA is the system-message path only. |

### Flow

```text
record(trajectory) ──► poor_count / check_interval ──► evolution_ready=true
                                                          │
complete() stage 3.5 ──► evolve_prompt(system) ◄──────────┘
                              │
                              ▼
                     Population::seed | load
                              │
                              ▼
                     Population::evolve (selection → mutate/crossover → elitism)
                              │
                              ▼
                     persist JSON (optional) + clear evolution_ready
                              │
                              ▼
                     transport sees champion system prompt
```

## Consequences

### Positive

- Self-improvement flywheel is a **running** feature when
  `pipeline.learner = "trajectory"`: poor outcomes → GA → new system prompt
  on the next request.
- Deterministic GA is unit-testable without LLM cost or nondeterminism.
- Population JSON gives a simple audit/debug artifact; path is opt-in so
  default installs stay side-effect free.
- ECC causal lineage and governance approval gates remain available as
  later hardening (see follow-ups).

### Negative

- Heuristic fitness is a proxy, not true online eval of the mutated prompt;
  poorly calibrated structure bonuses could prefer verbose prompts.
- In-request evolution adds a small CPU cost on the request that trips
  `evolution_ready` (bounded by `population_size × generations`).
- No human approval gate before the champion prompt is used (v1 trust model:
  opt-in trajectory learner only).

### Neutral

- Scorer false-positive rate still matters; operators should prefer
  `pipeline.scorer = "fitness"` with trajectory mode.
- Background / RPC trigger and substrate-backed lineage deferred.
- **WEFT-54**: `FitnessScorer.error_indicators` defaults are English-only
  substring markers for GEPA fitness — not a safety filter. Catalog, FP
  sanity tests, and jailbreak/localization limits:
  `docs/guides/fitness-scorer-error-indicators.md`.

## Follow-ups

1. Emit causal-graph edges (`MutatedTo` / `MergedInto`) per research design.
2. Governance `require_approval` before deploying champions.
3. Optional daemon tick that calls `evolve_prompt` off the hot path.
4. Wire default `population_path` under the workspace data dir when trajectory
   mode is enabled in production config.

## References

- `crates/clawft-core/src/pipeline/mutation/{mod,strategies,ga}.rs`
- `crates/clawft-core/src/pipeline/learner.rs`
- `crates/clawft-core/src/pipeline/traits.rs` (`apply_prompt_evolution`, stage 3.5)
- Plane WEFT-38 / audit task 03-17
- `docs/research/gepa-prompt-evolution-analysis.md`
