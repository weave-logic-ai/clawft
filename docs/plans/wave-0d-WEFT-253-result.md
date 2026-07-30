# WEFT-253 result — chat panel inline streaming via `agent.chat_stream`

**Branch:** `wave0d/weft-253-chat-stream`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Status:** Shipped  
**Worktree:** this agent worktree  

## Ticket

ws08: chat panel — inline streaming via `agent.chat_stream`.

Chat panel was sync-only; long answers felt frozen until the full response. Needs the `agent.chat_stream` verb, progressive token rendering, in-flight progress affordance, and native + WASM support.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Add `agent.chat_stream` to extension allowlist (timeout policy) | **Done** — `STATIC_ALLOWED_METHODS` + `LLM_TIMEOUT_MS` (300s) |
| Wire daemon-side streaming into chat panel; render tokens as they arrive | **Done** — progressive substrate frames + live draft bubble |
| Surface heartbeat label / progress affordance during in-flight stream | **Done** — phase label (`thinking`/`generating`/…) + "streaming…" hint |
| Native + WASM transports both supported | **Done** — substrate stream path + `Command::Raw`; no connection-takeover |

## Design

Progressive frames ride **substrate** (not UDS connection-takeover), so both native Live and the VSCode/WASM proxy work without multi-line RPC readers:

```
panel ──agent.chat_stream──► daemon (long RPC, same as agent.chat)
  │                              │
  │                              ├─ publish thinking frame
  │                              ├─ AgentService::dispatch
  │                              ├─ cascade word-ish generating frames
  │                              └─ publish done + return AgentChatResult
  │
  └── while pending: substrate.read(stream_path) every ~120ms
        └── grow live assistant draft bubble
```

Stream path (sibling of status/meta under the existing `chat` DerivedWriteGrant):

```
substrate/_derived/chat/<conv_id>/stream
```

Frame shape (`clawft_types::agent_chat::AgentChatStreamFrame`):

```json
{ "text": "accumulated…", "phase": "generating", "seq": 3, "done": false, "ts_ms": … }
```

`text` is **accumulated** (not delta) so a missed poll is self-healing.

Until the agent loop exposes mid-generation LLM token callbacks, the daemon cascades the final assistant text as typewriter frames (≤8-char word-ish chunks, 4ms yield) before returning — concurrent panel polls paint growth. True per-token LLM streaming is a follow-up that writes the same frame shape mid-generation.

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-types/src/agent_chat.rs` | `AgentChatStreamFrame`, `chat_stream_path`, helpers + tests |
| `crates/clawft-weave/src/protocol.rs` | Re-export stream types |
| `crates/clawft-weave/src/capability.rs` | `agent.chat_stream` → Chat |
| `crates/clawft-weave/src/daemon.rs` | `handle_agent_chat_stream` + publish helper |
| `crates/clawft-gui-egui/src/explorer/chat.rs` | Stream state machine, draft bubble, poll, heartbeat phase |
| `crates/clawft-gui-egui/src/apps/chat.rs` | Comment: sidebar hosts same `ChatView` stream path |
| `extensions/vscode-weft-panel/src/extension.ts` | Allowlist + 300s timeout |

## Tests

```bash
scripts/build.sh check
# ok

cargo test -p clawft-types agent_chat --lib
# 10 passed (incl. chat_stream_path + stream_frame_round_trips)

cargo test -p clawft-gui-egui explorer::chat --lib
# 22 passed (incl. 4 new WEFT-253 stream tests)

cargo test -p clawft-weave capability --lib
# 9 passed (agent.chat_stream classified Chat / anonymous-allowed)
```

## Follow-ups

- Mid-generation LLM tokens: wire `clawft-llm::complete_stream` through the agent tool loop and publish `generating` frames as tokens arrive (same substrate path; panel unchanged).
- T08-65 heartbeat polish was already largely present (WEFT-257); this ticket reuses and extends it with stream phase.
- Optional: fall back to `agent.chat` if an older daemon rejects `agent.chat_stream`.
