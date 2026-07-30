# WEFT-256 result — chat panel model / provider switcher in chip strip

**Branch:** `wave0l/weft-256-model-switcher`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Status:** Shipped  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4ea-5b23-7e42-9171-754f15419eca`  
**Pushed:** no (lead merge)

## Ticket

ws08: chat panel — model / provider switcher in chip strip.

No runtime way to choose model/provider from the panel; users had to edit configs out-of-band. Blocked on daemon-side enumeration.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Chip-strip selector (or dropdown) listing available providers/models from daemon | **Done** — horizontal chip strip above history, populated from `llm.models` |
| Selection plumbs through `agent.chat` call | **Done** — `metadata.model` on wire; agent loop honors it; `LlmClient` per-call model override |
| Persists across panel reload | **Done** — egui persisted memory key `weft.chat.selected_model` shared across sidebar + Explorer chat |

## Design

```
panel ──llm.models──► daemon
  │                      ├─ default from LlmClient config
  │                      └─ optional live GET /v1/models merge
  │
  ├── chip strip select → ChatView.selected_model
  │                        └─ egui insert_persisted (panel reload)
  │
  └── agent.chat_stream { metadata: { model } }
         └─ loop_core: metadata.model → ChatRequest.model
              └─ ServiceLlmAdapter → LlmClient.complete_with_tools(..., Some(model))
```

### Wire: `llm.models` result

```json
{
  "default_model": "hermes-4.3-36b",
  "default_provider": "local",
  "base_url": "http://127.0.0.1:8090",
  "models": [
    {
      "id": "hermes-4.3-36b",
      "provider": "local",
      "label": "hermes-4.3-36b",
      "is_default": true,
      "live": true
    }
  ],
  "probe_error": null
}
```

When the live probe fails, the daemon still returns the configured default (UI always has at least one chip). `probe_error` is informational only.

### Model override path

- Panel principal already has `model_override: true` (WEFT-31 audit path).
- `AgentChatParams.metadata.model` was already documented; `inbound_from_params` threaded it; the loop previously ignored it and always stamped `agents.defaults.model`.
- WEFT-256: non-empty `metadata.model` wins for `ChatRequest.model`.
- `ServiceLlmAdapter` forwards the routed model id into `LlmClient` so the HTTP body matches the selection (llama-server ignores routing; multi-model gateways honor it).

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-weave/src/protocol.rs` | `LlmModelEntry`, `LlmModelsResult` |
| `crates/clawft-weave/src/daemon.rs` | `handle_llm_models` + dispatch |
| `crates/clawft-weave/src/capability.rs` | `llm.models` → Read + tests |
| `crates/clawft-weave/src/llm_service.rs` | contract methods include `llm.models` |
| `crates/clawft-service-llm/src/client.rs` | `list_models`, per-call model override, tests |
| `crates/clawft-core/src/pipeline/service_llm_adapter.rs` | forward model to client |
| `crates/clawft-core/src/agent/loop_core.rs` | honor `metadata.model` |
| `crates/clawft-gui-egui/src/explorer/chat.rs` | chip strip UI, poll, persist, tests |
| `crates/clawft-gui-egui/src/apps/chat.rs` | sentinel comment |
| `extensions/vscode-weft-panel/src/allowlist.ts` | static seed `llm.models` |
| `extensions/vscode-weft-panel/src/allowlist.test.ts` | allowlist coverage |
| `docs/plans/wave-0l-WEFT-256-result.md` | this result |

## Tests

```bash
scripts/build.sh check   # packages check clean (touched crates)
cargo test -p clawft-gui-egui explorer::chat --lib
# 28 passed (incl. 6 new WEFT-256 tests)

cargo test -p clawft-service-llm --lib
# 38 passed (incl. list_models + model override)

cargo test -p clawft-weave capability --lib
# 12 passed (llm.models Read / anonymous-allowed)

cargo test -p clawft-weave llm_service --lib
# contract methods include llm.models
```

## How to test (for tester)

1. Boot daemon with local Hermes (or any OpenAI-compat) on the configured URL.
2. Open Chat (sidebar or Explorer chat sentinel).
3. Confirm chip strip shows at least the default model; if `/v1/models` is live, additional ids appear.
4. Click a chip → header `chat · <id>` updates; selection survives closing/reopening the panel in-session.
5. Send a turn; daemon logs / `model_override` audit should show the selected id; response `model` echoes when upstream provides it.
6. Optional: `weaver` / raw RPC `{"method":"llm.models","params":{}}` returns catalog JSON.

## Follow-ups

- Cross-process durable preference via `config.set` / Settings (Phase 3) instead of egui memory only.
- Multi-provider catalog when daemon hosts more than one `LlmClient` endpoint.
- Mid-generation stream already uses selected model for the whole turn; no mid-turn switch.

## Commit

Branch tip: `git log -1 --oneline` on `wave0l/weft-256-model-switcher`  
Subject: `feat(weft-256): chat panel model/provider switcher in chip strip`
