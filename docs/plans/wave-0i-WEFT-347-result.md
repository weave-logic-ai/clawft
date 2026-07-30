# WEFT-347 result — Phase 4 MemoryConsolidator periodic distillation

**Branch:** `wave0i/weft-347-memory-consolidator`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4c3-9a5a-7d11-afcd-7ebe0765face`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30

## Problem

`ConversationSink` (per-turn substrate / JSONL) and `MemoryStore`
(`MEMORY.md` / `HISTORY.md`) never shared a path, and nothing bridged them.
Phase 4 planned `crates/clawft-core/src/agent/learning/` for periodic
distillation; the module did not exist (audit deferred item #20;
chat-agent-v1 Phase 4; system-architect B1).

## What shipped

`MemoryConsolidator` — offline, deterministic distillation from a turn window
into long-term memory, with configurable cadence and fingerprint-based
idempotency.

| Surface | Behavior |
|---------|----------|
| `distill_turns(conv_id, turns)` | Pure: same turns → same facts + SHA-256 fingerprint |
| `consolidate` / `consolidate_turns` | Always run; write MEMORY.md + HISTORY.md when new |
| `run_if_due` | Honor cadence (every K turns **or** every T minutes) |
| `tick(conv_ids)` | Multi-conversation timer/worker entry |
| Idempotency | Skip write if `<!-- consolidator:fp=<hex> -->` already in MEMORY.md |
| Distillation | Extractive heuristics (preference / fact cues); no LLM dependency |

Defaults: `max_turns=32`, every `10` turns **or** every `30` minutes,
`enabled=true`. Cadence is driven by the caller (post-turn hook and/or
timer) so the module stays WASM-friendly (no internal tokio task).

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/src/agent/learning/mod.rs` | **new** — learning subsystem root + re-exports |
| `crates/clawft-core/src/agent/learning/consolidator.rs` | **new** — `MemoryConsolidator`, config, distill, tests |
| `crates/clawft-core/src/agent/mod.rs` | `pub mod learning` |
| `docs/plans/wave-0i-WEFT-347-result.md` | this report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| New module `agent/learning/consolidator.rs` | Done |
| Periodic task reads N recent turns → MEMORY.md | Done (`max_turns`, `consolidate` / `run_if_due` / `tick`) |
| Configurable cadence (every K turns or every T minutes) | Done (`ConsolidationConfig::{every_k_turns, every_t_minutes, with_cadence}`) |
| Idempotent: same turns → same distillation; no double-write | Done (pure distill + fingerprint marker) |
| Tests for consolidation correctness | Done — 14 unit tests, all pass |

## Verification

```bash
cargo nextest run -p clawft-core consolidator --lib
# 14 passed, 0 failed

# Via project script (package scope):
scripts/build.sh test clawft-core
```

Focused tests:

- `distill_is_deterministic` / `distill_fingerprint_changes_with_content`
- `cadence_every_k_turns` / `cadence_every_t_minutes` / `cadence_or_of_both_triggers`
- `consolidate_writes_memory_and_history`
- `consolidate_is_idempotent_on_same_turns`
- `run_if_due_respects_turn_cadence`
- `tick_processes_multiple_convs`
- `max_turns_window_is_honoured`

## Follow-ups

- Wire `run_if_due` into `AgentLoop` / `AgentService` post-turn path and a
  daemon timer that calls `tick` over known `conv_id`s.
- Optional LLM summarizer behind the same pure-shape contract
  (replace extractive heuristic when a provider is available).
- Promote `last_consolidated` index on `LocalFileSink` / session header so
  cadence survives process restart without in-memory state.
- Surface `ConsolidationConfig` in `clawft.toml` / config loader
  (`memoryConsolidation` camelCase already normalizes to
  `memory_consolidation` in the key normalizer tests).
