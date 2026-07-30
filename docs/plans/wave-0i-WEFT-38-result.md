# Wave 0i — WEFT-38 result

**Ticket:** WEFT-38 — ws03: pipeline — wire `evolution_ready` flag → mutation.rs GA loop (ADR-017 flywheel)  
**Branch:** `wave0i/weft-38-evolution-ga`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4c3-9a59-77d1-814d-c792c4f9b08c`  
**Base:** `release/0.8-staging`  
**Commit:** branch tip `wave0i/weft-38-evolution-ga` (see `git log -1 --oneline`)
**Date:** 2026-07-30  
**Agent:** coder-38 (wave-0i)

## Problem

`TrajectoryLearner` set `evolution_ready: true` after enough poor trajectories,
and stage 3.5 called `evolve_prompt`, but mutation was a single-shot string
operator — **not** a GA population (selection / crossover / elitism), with no
persistence and no end-to-end guarantee that a champion system prompt reaches
transport. Audit task 03-17 / Plane WEFT-38: ADR-017 flywheel existed as code,
not as a running feature.

(Note: the audit text claimed `mutation.rs` already had a full GA; it only had
four string strategies. This ticket implements the real GA and wires it.)

## What shipped

### `pipeline/mutation/` (module split)

| Item | Detail |
|------|--------|
| `strategies.rs` | Prior operators (rephrase, add examples, remove ineffective, emphasize) |
| `ga.rs` | `GaConfig`, `PromptCandidate`, `Population`, `EvolutionResult`, `evolve()`, `select_evolved_prompt` |
| GA operators | Deterministic tournament selection, line-level crossover, elitism, strategy-cycled mutation |
| Fitness | Heuristic over `TrajectoryHint`s (structure markers, good-snippet bonus, poor-phrase penalty) |
| Persist | `Population::save` / `load` (JSON) |

### `TrajectoryLearner` (stage 6)

| Item | Detail |
|------|--------|
| `TrajectoryLearnerConfig.ga` | GA hyperparameters |
| `TrajectoryLearnerConfig.population_path` | Optional JSON path |
| `with_population_path` | Builder + warm-start load |
| `evolve_prompt` | On `evolution_ready`: seed/continue population → `Population::evolve` → optional persist → clear flag + reset `poor_count` |
| `last_evolved_prompt` / `population_snapshot` | Observability for tests / admin |

### Production trigger surface

**In-request stage 3.5** (already present; now backed by GA):

`PipelineRegistry::complete` / `complete_stream` → `apply_prompt_evolution` →
`LearningBackend::evolve_prompt` when the trajectory learner has armed
`evolution_ready`. No separate background tick/RPC in this ticket (documented
as follow-up in ADR-017).

### Docs

- `docs/adr/adr-017-gepa-prompt-evolution.md` amended with WEFT-38 wiring table + flow

## Acceptance

| Criterion | Status |
|-----------|--------|
| Production trigger that consumes `evolution_ready` and runs GA `evolve` | **Done** — stage 3.5 + `TrajectoryLearner::evolve_prompt` → `Population::evolve` / `mutation::evolve` |
| Persist evolved population | **Done** — optional JSON via `population_path` |
| E2E: synthetic trajectories → evolution → prompt reaches agent/pipeline path | **Done** — `e2e_evolution_ready_ga_prompt_reaches_transport` |
| ADR-017 updated with chosen wiring | **Done** |

## Tests

```text
cargo test -p clawft-core --lib -- mutation:: learner:: e2e_evolution_ready apply_prompt_evolution
# 41 passed

cargo test -p clawft-core --lib -- skill_autogen
# 31 passed (WEFT-66 path still green)
```

Key cases:

- `mutation::ga::*` — seed, evolve, crossover, fitness, persist roundtrip, min_improvement
- `evolve_prompt_clears_flag_when_ready` / `evolve_prompt_ga_changes_instruction_prompt` / `evolve_prompt_persists_population_to_disk`
- `e2e_evolution_ready_ga_prompt_reaches_transport` — CaptureTransport sees champion system prompt

## How to test

```bash
# From worktree on branch wave0i/weft-38-evolution-ga
cargo test -p clawft-core --lib -- mutation::ga
cargo test -p clawft-core --lib -- e2e_evolution_ready_ga_prompt_reaches_transport
cargo test -p clawft-core --lib -- evolve_prompt

# Enable in config (trajectory learner already factory-wired):
# { "pipeline": { "learner": "trajectory", "scorer": "fitness" } }
# Optional: set TrajectoryLearnerConfig.population_path in code/bootstrap later.
```

## Files changed

- `crates/clawft-core/src/pipeline/mutation/mod.rs` (new module root)
- `crates/clawft-core/src/pipeline/mutation/strategies.rs` (moved from `mutation.rs`)
- `crates/clawft-core/src/pipeline/mutation/ga.rs` (new GA loop)
- `crates/clawft-core/src/pipeline/learner.rs` (wire + persist + config)
- `crates/clawft-core/src/pipeline/traits.rs` (e2e + apply_prompt_evolution unit)
- `docs/adr/adr-017-gepa-prompt-evolution.md` (amendment)
- `docs/plans/wave-0i-WEFT-38-result.md` (this file)

## Follow-ups (not in this ticket)

1. Default `population_path` under workspace data dir when trajectory mode is on.
2. Causal-graph lineage edges (`MutatedTo` / `MergedInto`) per research design.
3. Governance approval gate before deploying champions.
4. Optional daemon / admin tick calling `evolve_prompt` off the hot path.
5. Online fitness via shadow eval (run champion vs baseline on held-out trajectories).

## Lead merge notes

- Branch: `wave0i/weft-38-evolution-ga`
- Worktree path above
- **No push** (per wave instructions)
- Intentional commit only of the files listed under “Files changed”
