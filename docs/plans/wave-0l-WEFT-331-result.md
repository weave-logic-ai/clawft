# Wave 0l — WEFT-331 result

**Ticket:** WEFT-331 — interactive Defer UX prompt-and-resume in panel  
**Branch:** `wave0l/weft-331-defer-ux`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4ea-5b24-73d2-a8c3-9e628783b92d`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-331 (wave-0l)

## Problem

D2 surfaces `GateDecision::Defer { reason }` as a structured tool-result
`{"deferred":true,"reason":…}` so the LLM can re-plan. Real interactive
defer (panel-side prompt with human-in-the-loop) was v1.1 — the loop
never suspended and the panel never offered allow/deny/cancel.

## What shipped

### `clawft-types` — wire shapes

| Item | Detail |
|------|--------|
| `DeferUserDecision` | `allow` / `deny` / `cancel` (serde + parse) |
| `DeferPromptEvent` | `{ defer_id, conv_id, tool, reason, arguments_preview, timeout_ms, ts_ms? }` |
| `AgentChatDeferDecideParams` / `Result` | `agent.chat.defer_decide` RPC |
| `DEFER_DEFAULT_TIMEOUT_MS` | `120_000` (120s) — **default-deny on expiry** |
| `STREAM_PHASE_AWAITING_DEFER` | `"awaiting_defer"` |
| `chat_defer_path(conv)` | `substrate/_derived/chat/<conv>/defer` |
| `AgentChatStreamFrame.defer` | optional prompt on progressive frames |
| `AgentChatStreamFrame::awaiting_defer` | helper constructor |

### `clawft-core` — loop suspend

| Item | Detail |
|------|--------|
| `agent::defer` | `DeferInteractor` trait, `DeferRequest`, `DeferOutcome` |
| `AlwaysAllowDefer` / `AlwaysDenyDefer` | test interactors |
| `AgentLoop::with_defer_interactor` / `set_defer_interactor` | OnceLock attach |
| `execute_tool_with_guards` | on `Defer`: if interactor set → await decision; allow falls through to sandbox+dispatch; deny/cancel/timeout → structured envelope. Without interactor → v1 `{"deferred":true}` |

### `clawft-service-agent` — broker

| Item | Detail |
|------|--------|
| `InteractiveDeferBroker` | oneshot map keyed by `defer_id`; implements `DeferInteractor` |
| `DeferPromptPublisher` | publish / clear hooks (daemon writes substrate) |
| `AgentService::defer_decide` | panel RPC entry |
| `AgentService::with_defer_broker` | wire broker into service |

### `clawft-weave` — daemon + RPC

| Item | Detail |
|------|--------|
| Boot | builds broker + `DaemonDeferPublisher`, `set_defer_interactor` on loop, `with_defer_broker` on service |
| `agent.chat.defer_decide` | resolves pending waiter |
| Capability | Chat (anonymous panel allowed) |
| Stream publish | `awaiting_defer` frame + dedicated defer path |

### Panel (`clawft-gui-egui` + VSCode allowlist)

| Item | Detail |
|------|--------|
| `PendingDeferPrompt` | local parse of wire prompt |
| Stream frame / defer payload handlers | set / clear prompt |
| Inline prompt UI | Allow / Deny / Cancel buttons → `agent.chat.defer_decide` |
| Allowlist | `agent.chat.defer_decide` in static seed + panelAuth |

## Timeout / default-deny (documented)

1. Default wait budget: **`DEFER_DEFAULT_TIMEOUT_MS` = 120 seconds**.
2. On expiry the broker returns `DeferOutcome::Timeout`.
3. The loop maps Timeout → tool result  
   `{"denied":true,"reason":"defer timed out (default-deny): …","timeout":true}`  
   — the tool does **not** run; the turn continues so the model can re-plan.
4. `agent.chat.cancel` (per-conv token) while waiting → `DeferOutcome::Cancelled`  
   → similar deny envelope with `"cancelled":true`.
5. Explicit **deny** / **cancel** decisions never run the tool; only **allow**
   proceeds to sandbox + dispatch.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Panel surfaces Defer { reason } as a modal / inline prompt | **Done** — inline framed prompt in chat panel |
| User decision (allow/deny/cancel) returned via new RPC | **Done** — `agent.chat.defer_decide` |
| Loop suspends until decision arrives | **Done** — oneshot waiter in `InteractiveDeferBroker` |
| Timeout / default-deny path documented | **Done** — this section + type docs |
| E2E test: deferred tool resumes after user approval | **Done** — unit E2E: `gate_defer_allow_resumes_tool_execution` + broker `decide_allow_unblocks_waiter` |

## How to test

```bash
cargo test -p clawft-types --lib agent_chat
cargo test -p clawft-core --lib agent::defer
cargo test -p clawft-core --lib gate_defer
cargo test -p clawft-service-agent --lib defer_broker
cargo test -p clawft-gui-egui --lib explorer::chat
cargo test -p clawft-weave --lib capability
```

## Files changed

- `crates/clawft-types/src/agent_chat.rs`
- `crates/clawft-core/src/agent/defer.rs` (new)
- `crates/clawft-core/src/agent/mod.rs`
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-core/src/agent/gate.rs` (docs only if touched — comment path via loop)
- `crates/clawft-service-agent/src/defer_broker.rs` (new)
- `crates/clawft-service-agent/src/lib.rs`
- `crates/clawft-service-agent/src/protocol.rs`
- `crates/clawft-service-agent/src/service.rs`
- `crates/clawft-weave/src/daemon.rs`
- `crates/clawft-weave/src/capability.rs`
- `crates/clawft-weave/src/protocol.rs`
- `crates/clawft-gui-egui/src/explorer/chat.rs`
- `extensions/vscode-weft-panel/src/allowlist.ts`
- `extensions/vscode-weft-panel/src/allowlist.test.ts`
- `extensions/vscode-weft-panel/src/panelAuth.ts`
- `docs/plans/wave-0l-WEFT-331-result.md` (this file)

## Follow-ups

1. **WEFT-258** — panel handling of completed-turn `{ deferred: true }` (non-suspend path) remains separate.
2. **WEFT-345 allow/abort/refine** — can reuse `agent.chat.defer_decide` / broker pattern for post-escalation resume.
3. Optional: poll dedicated `chat_defer_path` in panel (stream frame is primary today).
4. Optional: richer modal with tool-arg diff / always-allow-session policy.
