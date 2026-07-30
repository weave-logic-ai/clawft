# WEFT-323 result — Per-iteration CancellationToken into run_tool_loop

**Ticket:** WEFT-323  
**Branch:** `wave0f/weft-323-cancel-token`  
**SHA:** branch tip (`git rev-parse wave0f/weft-323-cancel-token`)
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb478-7734-71b1-b768-fb3530ed8be5`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-323 (wave-0f)

## Problem

AC-10 cancel only caught whole-turn cancellations via the outer
`tokio::select!` in `AgentService::dispatch`. An in-flight multi-iteration
tool loop could not break at an iteration boundary: the user had to wait for
the current LLM call / tool dispatch to drain (or for the future to be
dropped mid-await). Phase D2 TODO in
`crates/clawft-service-agent/src/service.rs` flagged the missing
per-iteration path.

## What shipped

### `clawft-types`

| Item | Detail |
|------|--------|
| `ClawftError::Cancelled { conv_id }` | Typed error returned when the cancel token is observed at a tool-loop iteration boundary |

### `clawft-core` — `AgentLoop`

| Item | Detail |
|------|--------|
| `handle_turn(&self, msg, cancel: &CancellationToken)` | Required cancel param; threaded into inner pipeline |
| `handle_turn_inner(..., cancel)` | Passes cancel into `run_tool_loop` |
| `run_tool_loop(..., cancel)` | Checks `cancel.is_cancelled()` at the **top of every iteration** before LLM / budget / tool work |
| `fresh_cancel_token()` | Helper for CLI / browser / tests with no per-conv cancel signal |
| Bus `run()` | Threads loop-level token (or a never-cancelled stand-in) into each turn |

On cancel at the boundary: returns `Err(ClawftError::Cancelled { conv_id })`
without starting another LLM call.

### `clawft-service-agent` — `AgentService`

| Item | Detail |
|------|--------|
| `AgentLoopHandle::handle_turn(msg, cancel: CancellationToken)` | Trait takes token by value (async_trait-friendly; Arc-backed clone) |
| `dispatch` | Clones per-conv token into `handle_turn`; outer `select!` kept for mid-await abort |
| Loop-error mapping | `"was cancelled"` Display text → `AgentServiceError::Cancelled` + re-arm fresh token |
| Phase D2 TODO | Removed (wired) |

### Call-site updates

- All `loop_core` unit tests pass a never-cancelled token
- WASM `send_message` uses `fresh_cancel_token()`
- Stub `AgentLoopHandle` impls (dispatch, interrupt, subagent, weave) accept cancel

## Acceptance

| Criterion | Status |
|-----------|--------|
| `AgentLoop::handle_turn` accepts `cancel: &CancellationToken` | Yes |
| `run_tool_loop` checks token at iteration boundary | Yes |
| `AgentService::dispatch` threads per-conv token | Yes |
| Mid-turn cancel test (dispatch + loop_core) | Yes |
| No regression on loop_core / service-agent tokio tests | Yes (72 loop_core + full service-agent suite) |

## Tests added

**`clawft-core` (`loop_core::tests`)**

- `run_tool_loop_breaks_on_cancel_at_iteration_boundary` — pre-cancelled token fails before any LLM call
- `run_tool_loop_breaks_mid_turn_on_next_boundary` — cancel after first iteration; second boundary returns `Cancelled`

**`clawft-service-agent` (`tests/dispatch.rs`)**

- `cancel_breaks_mid_turn_via_threaded_token` — cancel while stub is in-flight; asserts `Cancelled` without release Notify; Drop guard keeps `in_progress` accurate under future-drop cancel

## Verification

```text
scripts/build.sh check
# ok

cargo test -p clawft-core --lib agent::loop_core
# 72 passed (incl. 2 new cancel tests)

cargo test -p clawft-service-agent --test dispatch
# 14 passed (incl. cancel_breaks_mid_turn_via_threaded_token)

cargo test -p clawft-service-agent
# all packages/tests ok

cargo test -p clawft-weave --test subagent_caps
# 3 passed
```

## Files changed

- `crates/clawft-types/src/error.rs`
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-service-agent/src/service.rs`
- `crates/clawft-service-agent/src/subagent.rs`
- `crates/clawft-service-agent/tests/dispatch.rs`
- `crates/clawft-service-agent/tests/interrupt_execute.rs`
- `crates/clawft-weave/tests/subagent_caps.rs`
- `crates/clawft-wasm/src/lib.rs`
- `docs/plans/wave-0f-WEFT-323-result.md` (this file)

## Notes

- Outer `select!` remains: cancel during a long LLM `complete()` still aborts by dropping the future (WEFT-655 Drop guard rolls back COW checkpoints). Per-iteration check covers the multi-tool-iteration case AC-10 described.
- Trait uses owned `CancellationToken` to avoid non-`'static` lifetimes under `async_trait`; production impl passes `&cancel` into `AgentLoop::handle_turn`.
