# WEFT-651 result — agent loop identical tool-call retry breaker

**Ticket:** WEFT-651  
**Branch:** `wave0b/weft-651-tool-retry-breaker`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb432-7bcf-7812-bb58-6dfaf1ca3bb1`  
**Date:** 2026-07-30  
**N (identical failure limit):** `3`

## Problem

During the §W2.1 live smoke (conv `w21-smoke2`), Hermes answered "write a
haiku" by calling the canvas tool repeatedly with invalid args —

```json
{"error":"invalid arguments: invalid canvas command: missing field content"}
```

— for the full 20-iteration budget, then failed the turn (`max tool
iterations (20) exceeded`). The voice failure arm handled it correctly, but
the turn produced no reply and burned ~20 LLM round-trips.

Identical invalid tool calls were retried with no feedback escalation.

## Fix

Two complementary changes in `crates/clawft-core/src/agent/loop_core.rs`:

### 1. Schema-echo on `InvalidArgs` (`execute_tool_with_guards`)

When a tool returns `ToolError::InvalidArgs`, the error body now appends:

- a truncated `Expected parameters schema: …` (from `tool.parameters()`)
- an explicit `Do not retry with the same invalid arguments.`

so the model can correct the call instead of blind-retrying.

### 2. Repeated-identical-failure circuit breaker (`run_tool_loop`)

Tracks consecutive failures keyed by **(tool name, compact args JSON, error
text)**. After **N = 3** identical failures within one turn:

1. Replaces the tool-result body with an escalated
   `IDENTICAL TOOL FAILURE BREAKER` message (clear stop instruction).
2. Persists that result to the sink / conversation history.
3. **Fail-fast** with `ClawftError::Provider` naming the breaker, tool, and
   count — **before** the max-iteration budget is exhausted.

Success or a different (tool / args / error) fingerprint resets the streak.

Constant: `IDENTICAL_TOOL_FAILURE_LIMIT = 3` (matches the planning
circuit-breaker default).

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/src/agent/loop_core.rs` | Schema-echo, breaker helpers + integration, unit tests |
| `docs/plans/wave-0b-WEFT-651-result.md` | This result note |

## Tests

```bash
scripts/build.sh check
scripts/build.sh test clawft-core
```

- **check:** pass  
- **tests:** 1430 passed, 0 failed (clawft-core nextest + doctests)

New coverage:

| Test | What it proves |
|------|----------------|
| `identical_failure_key_none_on_success` | Success does not fingerprint |
| `identical_failure_key_matches_same_tool_args_error` | Same (tool, args, error) matches |
| `identical_failure_key_differs_when_args_change` | Arg change breaks match |
| `record_identical_failure_trips_at_limit` | Counter trips at N=3 |
| `record_identical_failure_resets_on_success_or_different_key` | Reset semantics |
| `identical_failure_breaker_message_is_clear` | Escalated body text |
| `run_tool_loop_trips_identical_failure_breaker` | End-to-end fail-fast before max iter |
| `invalid_args_error_echoes_parameter_schema` | Schema-echo on InvalidArgs |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Breaker after N identical tool+args failures | **Met** — N=3 |
| Clear error to model | **Met** — schema-echo + escalated breaker body + Provider message |
| Does not burn full max-iteration budget on identical invalid calls | **Met** — trips at 3 |
| Unit tests + check | **Met** |

## Commit

- **Implementation:** `ea6ff4aa4bd24a54961f257fb8296bbb1e0fc3f7`
- **Branch tip:** `wave0b/weft-651-tool-retry-breaker` (see `git log -1`)
