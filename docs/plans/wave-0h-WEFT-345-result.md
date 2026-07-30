# Wave 0h — WEFT-345 result

**Ticket:** WEFT-345 — agent-core after-3-denials `EscalateToHuman` governance path  
**Branch:** `wave0h/weft-345-escalate-human`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bee-7ac0-92b1-c51609d153ca`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-345 (wave-0h)

## Problem

After repeated effect-gate denials the tool loop either burned the remaining
iteration budget or (per plan text) would halt with a silent
`agent: gate denied tool calls 3x; halting` string. Governance
recommendation 4 / `chat-agent-v1.md` §5.5 requires a first-class
`EscalateToHuman` decision instead of a silent halt.

## What shipped (agent-core scope)

### `clawft-types` — wire event

| Item | Detail |
|------|--------|
| `GateDenialRecord` | `{ tool, reason }` one denied tool call |
| `EscalateToHumanEvent` | `decision = "EscalateToHuman"`, `denial_count`, `conv_id`, `denials[]`, `summary` |
| `FINISH_REASON_ESCALATE_TO_HUMAN` | `"escalate_to_human"` |
| `GOVERNANCE_DECISION_ESCALATE_TO_HUMAN` | `"EscalateToHuman"` (matches kernel enum spelling) |
| `AgentLoopResultMeta.escalation` | optional event on outbound loop meta |
| `AgentChatResult.escalation` | optional event on wire result (panel seam) |

### `clawft-core` — gate streak + loop

| Item | Detail |
|------|--------|
| `GATE_DENIAL_ESCALATION_LIMIT` | `3` (in `agent/gate.rs`) |
| `gate_denial_reason` | parses only `{"denied":true,"reason":…}` — sandbox/runtime errors do **not** count |
| `record_gate_denial_streak` | increments on deny; resets on non-deny |
| `run_tool_loop` | after 3 consecutive gate Denys → `Ok(ToolLoopResult)` with `finish_reason = escalate_to_human` + `escalation` event; assistant text = event summary |
| Priority | WEFT-345 runs **before** WEFT-651 identical-failure breaker so policy denials escalate as governance, not generic Provider error |

### `clawft-service-agent` — wire mapping

| Item | Detail |
|------|--------|
| `result_from_outbound` | maps `meta.escalation` → `AgentChatResult.escalation` |

## Acceptance

| Criterion | Status |
|-----------|--------|
| After 3 consecutive Denys the loop emits EscalateToHuman decision | **Done** — meta + wire `escalation.decision = "EscalateToHuman"`, `finish_reason = escalate_to_human` |
| Panel UI shows escalation prompt (W-UI/15) | **Deferred** — cross-stream panel UX; field + finish_reason are the seam |
| User decision (allow/abort/refine) via new RPC | **Deferred** — blocked by interactive Defer UX |
| Loop resumes on allow with refined plan | **Deferred** — needs allow/abort/refine RPC |
| Test: 3-deny path produces escalation event | **Done** |

## Tests

**`clawft-types`**

- `escalate_to_human_event_from_denials`
- `agent_chat_result_escalation_round_trips`

**`clawft-core` (`agent::gate`)**

- `gate_denial_reason_*`
- `record_gate_denial_streak_trips_at_limit`
- `record_gate_denial_streak_resets_on_non_deny`

**`clawft-core` (`agent::loop_core`)**

- `three_gate_denials_emit_escalate_to_human` — always-deny gate + infinite tool-use transport → escalation event, 3 tool calls, halt
- `two_gate_denials_do_not_escalate` — 2 denials then text → `finish_reason=stop`, no event

**`clawft-service-agent`**

- `result_from_outbound_reads_weft345_escalation`

## How to test

```bash
scripts/build.sh test clawft-types
cargo test -p clawft-core --lib three_gate_denials
cargo test -p clawft-core --lib two_gate_denials
cargo test -p clawft-core --lib gate::
cargo test -p clawft-service-agent --lib result_from_outbound
```

## Files changed

- `crates/clawft-types/src/agent_chat.rs`
- `crates/clawft-core/src/agent/gate.rs`
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-service-agent/src/service.rs`
- `docs/plans/wave-0h-WEFT-345-result.md` (this file)

## Follow-ups (not in this ticket)

1. **W-UI/15** — panel escalation prompt when `finish_reason == escalate_to_human` / `escalation` present.
2. **Allow/abort/refine RPC** — resume loop on allow with refined plan (shared with interactive Defer UX).
3. Optional: map kernel `GovernanceDecision::EscalateToHuman` single-call defer into the same event shape for non-streak escalations.
