# WEFT-328 result — Plumb tool_calls / tokens / model / identity_source through OutboundMessage

**Ticket:** WEFT-328  
**Branch:** `wave0g/weft-328-outbound-fields`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb489-7de2-72f2-89c6-2bcf9fc785d9`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-328 (wave-0g)

## Problem

`AgentChatResult` fields `tool_calls`, `prompt_tokens`, `completion_tokens`,
`model`, and `identity_source` defaulted to empty/zero/`None` at the wire
because `OutboundMessage` is a generic bus envelope. Deferred item #8 in
`.planning/reviews/0.7.0-release-gate/11-agent-core-v1.md` and the C1 shape
tests in service-agent pinned the shortfall so the panel kept tolerating
defaults across cutover. M4 D8 had already threaded `tool_calls` /
`finish_reason` / `iterations` / `spawned_tasks` (and partial token/model
fields) via `AgentLoopResultMeta` in `OutboundMessage.metadata`, but
`identity_source` was still hardcoded `None` in `result_from_outbound`, and
tests did not assert real token/model values.

## What shipped

### `clawft-types` — `AgentLoopResultMeta` / `AgentChatResult`

| Item | Detail |
|------|--------|
| `AgentLoopResultMeta.identity_source` | New optional field; serde-default + skip when `None` (C1 back-compat) |
| Docs | `AgentChatResult` / meta docs updated for WEFT-328 path |
| Tests | Meta round-trip includes identity; partial object defaults all five fields; wire deserializes non-zero tokens + model + identity |

### `clawft-core` — loop + system prompt

| Item | Detail |
|------|--------|
| `BuiltSystemPrompt` | `{ body, identity_source }` from `SystemPromptBuilder::build_with_meta` |
| `SystemPromptBuilder::build` | Thin wrapper over `build_with_meta` (unchanged external return) |
| `handle_turn` | Captures `identity_source` when the identity prompt builds successfully |
| `AgentLoopResultMeta` stash | Includes `identity_source` with tokens/model/tool_calls already set by the tool loop |
| E2e transport | Emits `metadata["model"] = "e2e-test-model"` so loop tests see a real model |

### `clawft-service-agent` — wire mapping

| Item | Detail |
|------|--------|
| `result_from_outbound` | Maps `identity_source` from meta (no longer hardcoded `None`) |
| Defaults | Absent meta still yields C1 zeros/`None` so panel tolerates partial payloads |
| Logging | Debug includes tokens, model, identity_source |

### Panel (egui)

| Item | Detail |
|------|--------|
| Chat response test | Asserts full WEFT-328 payload shape (tool_calls + tokens + model + identity) and canonical identity → no drift warning |

## Acceptance

| Criterion | Status |
|-----------|--------|
| OutboundMessage (or sibling envelope) carries the five fields | Yes — via `AgentLoopResultMeta` under `AGENT_LOOP_RESULT_META_KEY` |
| Loop populates them on every turn | Yes — tokens/model from LLM usage; tool_calls from tool loop; identity_source when builder attached |
| `AgentChatResult` deserializes non-zero values | Yes — types unit test + service mapping test |
| Panel UI tests assert real values | Yes — egui `ok_response_accepts_assistant_text_field` |
| Existing C1 shape tests still pass | Yes — absent-meta defaults test retained |

## Tests

**`clawft-types`**

- `agent_loop_result_meta_round_trips_through_metadata_value` (identity included)
- `agent_loop_result_meta_partial_object_uses_defaults` (all five default)
- `agent_chat_result_deserializes_nonzero_token_and_identity_fields` (new)

**`clawft-service-agent`**

- `result_from_outbound_marks_known_shortfalls` (C1 defaults)
- `result_from_outbound_reads_enriched_loop_meta` (asserts tokens/model/identity)

**`clawft-core`**

- `handle_turn_threads_real_tool_calls_and_iterations` (tokens 40/22, model, no identity without builder)
- `handle_turn_prepends_identity_system_prompt` (identity_source `"stub"` + tokens/model)
- `build_with_meta_exposes_identity_source` (new)

**`clawft-gui-egui`**

- `ok_response_accepts_assistant_text_field` (full real payload)

## Verification

```text
scripts/build.sh test clawft-types clawft-service-agent  → 523 passed
scripts/build.sh test clawft-core --no-fail-fast         → 1460 passed, 1 pre-existing fail
  (workspace::config::tests::load_merged_config_mcp_servers — unrelated MCP config JSON null; not touched)
scripts/build.sh test clawft-gui-egui                    → 416 passed
scripts/build.sh check                                   → ok
```

## Files changed

- `crates/clawft-types/src/agent_chat.rs`
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-core/src/agent/system_prompt.rs`
- `crates/clawft-service-agent/src/service.rs`
- `crates/clawft-gui-egui/src/explorer/chat.rs`
- `docs/plans/wave-0g-WEFT-328-result.md`

## Notes

- Envelope not widened: still uses free-form `OutboundMessage.metadata` (sibling-envelope approach from M4 D8).
- `identity_source` is `None` when no `SystemPromptBuilder` is attached or identity load fails (degraded prompt path).
- Token counts are cumulative across tool-loop LLM iterations; model is last reported `metadata["model"]`.
