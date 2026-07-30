# WEFT-66 result — wire improve_skill_instructions / generate_skill_md_with_learning into agent-loop

**Status:** done  
**Branch:** `wave0g/weft-66-autogen-learner`  
**Base:** `release/0.8-staging`  
**Plane id:** `349515d0-a632-4029-8942-83cf824cbaa1`  
**Date:** 2026-07-30

## Problem

`improve_skill_instructions` and `generate_skill_md_with_learning` were
implemented and unit-tested in `skill_autogen.rs`, but the agent loop still
called plain `generate_skill_md` and reconstructed a bare `AutogenConfig`
without a `TrajectoryLearner`. Learning-driven skill mutation was dead from
the loop's perspective.

## What shipped

| Surface | Change |
|---------|--------|
| `AutogenConfig` | New `learner: Option<Arc<TrajectoryLearner>>` + `with_learner` |
| `PatternDetector` | Accessors: `config`, `install_dir`, `learner`, `learner_arc`, `set_learner` |
| `build_learner_parts` | Returns `(Arc<dyn LearningBackend>, Option<Arc<TrajectoryLearner>>)` |
| `PipelineRegistry` | `trajectory_learner` handle + `with_trajectory_learner` |
| `AppContext` | `trajectory_learner()` before `into_agent_loop` |
| Agent loop | Uses `generate_skill_md_with_learning`; latches pipeline learner onto detector if missing |
| CLI agent / gateway | Builds `AutogenConfig` with active trajectory learner |
| Learning quality | `improve_skill_instructions` no longer double-counts poor trajectories (was selecting RemoveIneffective → no-op on fresh SKILL.md) |
| Docs | `docs/skills/autogen.md` — Trajectory learning section |

### Generation path (when autogen enabled)

```text
tool dispatch → PatternDetector::record_tool_call / detect_candidates
             → AutogenConfig.learner (or pipeline.trajectory_learner fallback)
             → generate_skill_md_with_learning(pattern, learner)
             → install_pending_skill (.pending until approve)
```

## Acceptance criteria

| Criterion | Status |
|-----------|--------|
| `loop_core` builds / uses `AutogenConfig` with active `TrajectoryLearner` | Yes — detector config carries learner; loop latches pipeline handle if absent |
| When autogen enabled, learning-driven instruction mutation runs | Yes — `generate_skill_md_with_learning` |
| E2E test: synthetic trajectories improve SKILL.md | Yes — `e2e_autogen_config_learner_improves_skill_md` |
| Documented in `docs/skills/autogen.md` | Yes |

## Verification

```text
cargo test -p clawft-core --lib skill_autogen
# 31 passed (includes e2e_autogen_config_learner_improves_skill_md,
#   generate_skill_md_with_learning_uses_learner, config_with_learner_attaches_arc)

cargo test -p clawft-core --lib build_learner_parts
# 2 passed

cargo check -p clawft-core -p clawft-cli
# ok
```

Workspace run via `scripts/build.sh test clawft-core clawft-cli` hits one
**pre-existing** failure unrelated to this change:
`workspace::config::tests::load_merged_config_mcp_servers` (null MCP config
JSON). All skill_autogen / learner factory tests pass.

## Files

- `crates/clawft-core/src/agent/skill_autogen.rs`
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-core/src/pipeline/mod.rs`
- `crates/clawft-core/src/pipeline/traits.rs`
- `crates/clawft-core/src/pipeline/llm_adapter.rs`
- `crates/clawft-core/src/bootstrap.rs`
- `crates/clawft-cli/src/commands/agent.rs`
- `crates/clawft-cli/src/commands/gateway.rs`
- `docs/skills/autogen.md`
- `docs/plans/wave-0g-WEFT-66-result.md`

## How to test

```bash
# Unit / e2e skill learning
cargo test -p clawft-core --lib skill_autogen

# Enable both surfaces in config, then run agent:
# {
#   "skills": { "autogen": { "enabled": true, "threshold": 3 } },
#   "pipeline": { "learner": "trajectory", "scorer": "fitness" }
# }
# After threshold repetitions of a tool sequence, pending skills under
# ~/.clawft/skills/ may include a "## Successful Examples" section when
# the shared TrajectoryLearner has high-quality trajectories.
```

## Follow-ups

- Wire autogen + learner on the daemon `build_daemon_agent_loop` path the same
  way as CLI/gateway (currently daemon does not attach PatternDetector).
- Intersects ws03 evolution_ready wiring (03-17) for prompt-level GEPA.
