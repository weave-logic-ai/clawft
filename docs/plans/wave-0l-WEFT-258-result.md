# Wave 0l — WEFT-258 result

**Ticket:** WEFT-258 — chat panel real interactive defer (resume on `{ deferred: true }`)  
**Branch:** `wave0l/weft-258-interactive-defer`  
**SHA:** branch tip of `wave0l/weft-258-interactive-defer` (impl `6e970acd`)  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4ea-5b24-73d2-a8c3-9e52631e5ad2`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-258 (wave-0l)

## Problem

When the effect gate returned `GateDecision::Defer`, the tool loop
surfaced `{"deferred": true, "reason": ...}` as a tool result and
**continued** so the model could re-plan. The chat panel treated that
turn like any other completion — no reason card, no approve/deny, no
resume path. AC (T08-66 / ws08 triage):

1. Detect `deferred: true` in the response.
2. Render the `reason` and an inline input/affordance.
3. Resume the conversation with user-supplied input.

## What shipped

### Wire (`clawft-types`)

| Item | Detail |
|------|--------|
| `FINISH_REASON_DEFERRED` | `"deferred"` — panel switch key |
| `DeferredActionEvent` | `{ deferred: true, reason, tool, conv_id, arguments_preview, summary }` |
| `AgentChatResult.deferred` | optional event (skipped when `None`) |
| `AgentLoopResultMeta.deferred` | loop → service mapping |

### Agent loop (`clawft-core`)

| Item | Detail |
|------|--------|
| `gate_deferral_reason` | parses only `{"deferred":true,"reason":…}` |
| `run_tool_loop` | on first gate Defer → halt with `finish_reason=deferred` + event (mirrors WEFT-345 halt pattern) |
| Tool-result body | still `{"deferred":true,"reason":…}` (unchanged for sink / history) |

Prior behaviour (loop continues so the model re-plans) is replaced by
**halt for human review**. Resume is a follow-up `agent.chat` /
`agent.chat_stream` user turn carrying a structured governance message
(`[governance:approve|deny|guide] …`). A dedicated mid-loop suspend
RPC (shared with WEFT-345 allow/abort/refine) remains a follow-up.

### Service (`clawft-service-agent`)

| Item | Detail |
|------|--------|
| `result_from_outbound` | maps `meta.deferred` → `AgentChatResult.deferred` |

### Chat panel (`clawft-gui-egui`)

| Item | Detail |
|------|--------|
| `PendingDefer` | panel state for reason / tool / args / note draft |
| `extract_pending_defer` | accepts structured event, top-level flag, `finish_reason`, tool_calls preview, or assistant_text JSON |
| `on_response_ok` | arms `pending_defer` when deferred is detected |
| `paint_defer_prompt` | amber card: reason, tool, args, note field, **Approve** / **Deny** |
| `resume_defer` | builds governance user turn and fires `agent.chat_stream` |
| Free-form Send while deferred | treated as `"guide"` resume |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Detect `deferred: true` in response | **Done** — multi-shape extractor + structured event |
| Render the reason and an inline input/affordance | **Done** — `paint_defer_prompt` Approve/Deny + note |
| Resume conversation with user-supplied input | **Done** — `resume_defer` / guide via Send |
| Agent-side defer protocol stable for panel | **Done** — halt + `DeferredActionEvent` on wire |

## Verification

```bash
scripts/build.sh test clawft-types
scripts/build.sh test clawft-service-agent
scripts/build.sh test clawft-gui-egui
cargo test -p clawft-core --lib gate_defer
cargo test -p clawft-core --lib gate::tests
scripts/build.sh check
```

**Results (this worktree):**

- `clawft-types`: 346 passed
- `clawft-service-agent`: 205 passed (incl. `result_from_outbound_reads_weft258_deferred`)
- `clawft-gui-egui`: 407 passed (incl. WEFT-258 extract / arm / clear tests)
- `clawft-core` gate_defer / gate::tests / three_gate / two_gate: **all pass**
- `scripts/build.sh check`: ok
- Note: full `scripts/build.sh test clawft-core` hit one **unrelated** failure
  (`workspace::config::tests::load_merged_config_mcp_servers` — null MCP
  config) that cancelled remaining tests; not introduced by this change.

## Files changed

- `crates/clawft-types/src/agent_chat.rs`
- `crates/clawft-core/src/agent/gate.rs`
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-service-agent/src/service.rs`
- `crates/clawft-gui-egui/src/explorer/chat.rs`
- `docs/plans/plane-board-inventory.md` (WEFT-258 → Done)
- `docs/plans/wave-0l-WEFT-258-result.md` (this report)

## Residual / follow-ups

1. **Mid-loop suspend RPC** — true in-turn allow/abort/refine without a
   new user message (shared with WEFT-345 W-UI/15 escalation prompt).
2. **Escalation panel card** — reuse the defer card pattern when
   `finish_reason == escalate_to_human` / `escalation` present.
3. **Timeout / default-deny** — agent-core interactive-defer ticket AC
   for suspended loops; not required for panel resume-via-user-turn.
4. **Prove-of-permission Permit token** on resume-approve (separate ticket).
