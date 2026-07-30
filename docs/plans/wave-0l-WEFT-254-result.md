# WEFT-254 result — chat panel multi-conversation sidebar UI

**Branch:** `wave0l/weft-254-chat-sidebar`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Status:** Shipped  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4ea-5b23-7e42-9171-753943acc3c3`  

## Ticket

ws08: chat panel — multi-conversation sidebar UI.

Panel previously showed one conversation. No way to switch between concurrent
threads or revisit history. Multi-tab terminal also foreshadowed only.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Sidebar listing conversations (id + last-active timestamp + first message snippet) | **Done** — left strip shows display id, age label, first user snippet |
| New-conversation affordance | **Done** — `+ New conversation` button mints a fresh `ChatSession` |
| Selection switches active stream + history | **Done** — `ChatPanel::select` swaps active slot; per-session stream/history preserved |
| In-memory persistence acceptable for first cut; on-disk persistence is a follow-up | **Done** — panel-lifetime slots only; no substrate.list rehydrate yet |

## Design

```
┌──────────────┬─────────────────────────────┐
│ Conversations│ chat · local · panel-…      │
│ [+ New …]    │ Local LLM                   │
│ ──────────── │ [history / stream draft]    │
│ ▸ panel-…    │ [system prompt]             │
│   2m ago     │ [input / Send / Clear]      │
│   first msg… │                             │
│ ○ local-…    │                             │
│   just now   │                             │
│   New conv…  │                             │
└──────────────┴─────────────────────────────┘
```

- **`ChatSession`**: one in-memory slot (`local_id`, `created_ms`,
  `last_active_ms`, embedded `ChatView`).
- **`ChatPanel`**: `Vec<ChatSession>` + `active` index. Default = one empty
  session. Hosts (`Desktop.chat`, `Explorer.chat_view`) now own `ChatPanel`
  instead of a bare `ChatView`.
- **Drain policy**: every paint drains RPC replies + stream polls for **all**
  sessions so a background turn still commits when another is selected.
- **Display id**: `panel-…` / daemon `conv_id` once minted; otherwise
  `local-…` draft key.
- **Widget ids**: system prompt / draft / history scroll use per-session
  `id_salt` so egui state does not bleed across slots.

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-gui-egui/src/explorer/chat.rs` | `ChatSession` + `ChatPanel`; sidebar paint; `paint` takes panel; 8 WEFT-254 unit tests |
| `crates/clawft-gui-egui/src/shell/desktop.rs` | `chat: ChatPanel` |
| `crates/clawft-gui-egui/src/explorer/mod.rs` | `chat_view: ChatPanel` |
| `crates/clawft-gui-egui/src/apps/chat.rs` | Docs: hosts multi-conversation panel |
| `docs/plans/wave-0l-WEFT-254-result.md` | this report |

## Tests

```bash
scripts/build.sh check
# ok

cargo test -p clawft-gui-egui explorer::chat --lib
# 30 passed (incl. 8 new WEFT-254 multi-conversation tests)
```

Focused new tests:

- `panel_starts_with_one_empty_session`
- `new_conversation_appends_and_selects`
- `select_switches_active_history`
- `select_preserves_per_session_stream_state`
- `snippet_truncates_and_collapses_newlines`
- `display_id_prefers_minted_conv_id`
- `truncate_snippet_helpers`
- `touch_updates_last_active_ordering_input`

## Follow-ups

- On-disk / substrate rehydrate: `substrate.list` under
  `substrate/_derived/chat/*` to rebuild the sidebar across panel restarts
  (chat-agent-v1 §17 / cancelled WEFT-346 notes).
- Conversation rename / archive (out of WEFT-254 AC; was on WEFT-346).
- Optional: close/delete slot affordance and max-session cap.
- Multi-tab terminal (WEFT-263) can mirror the same panel-local slot pattern.
