# Browser OPFS conversation history (WEFT-399)

Persistent conversation history and per-group identity for
`clawft-wasm` when built with `--features browser-opfs`.

## Problem

Before WEFT-399, browser conversation state lived only in the WASM
module heap (`OnceLock<BrowserRuntime>` / in-memory sink). A page
reload wiped turns. The 0.7.0 browser audit called for a
**CLAUDE.md-per-group** layout in OPFS, mirroring openbrowserclaw-style
group workspaces.

## Design summary

| Concern | Store | Path |
|---------|--------|------|
| Conversation turns (superset JSONL) | `LocalFileSink` via platform FS | `/clawft/.clawft/workspace/sessions/{percent-encoded web:group}.jsonl` |
| Last `init` config snapshot (P6.4) | OPFS file | `/clawft/.clawft/config.json` |
| Env vars (WEFT-14) | OPFS snapshot | `/clawft/.clawft/env.json` |
| Per-group identity (CLAUDE.md analogue) | OPFS markdown | `/clawft/workspace/groups/{group_id}/CLAWFT.md` |
| Agent workspace files (WEFT-392) | OPFS | `/clawft/workspace/…` |

Without `browser-opfs`, the same virtual paths work against the
in-memory `BrowserFileSystem` (session-only).

## Group identity (CLAUDE.md-per-group)

Each browser conversation is a **group** identified by `group_id`
(default: `browser`).

- **Session / sink key**: `web:{group_id}` (channel `web`, chat_id =
  group). Same scheme as native `LocalFileSink` / M3 store-collapse.
- **Group workspace**: `/clawft/workspace/groups/{group_id}/`
- **Identity file**: `CLAWFT.md` in that directory (WeftOS equivalent of
  Claude Code’s `CLAUDE.md`). Optional; empty when unset.
- **JS APIs**: `get_group_clawft_md` / `set_group_clawft_md`,
  `get_history` / `clear_history`, `send_message_to(text, group_id)`.

`group_id` is validated (no `/`, `\`, `..`, control chars) before any
path join.

## Serialization lifecycle

```
init(config_json, env_json?)
  → open OPFS FS + env (when browser-opfs)
  → AppContext + LocalFileSink(sessions_dir)
  → persist config_json → /clawft/.clawft/config.json   (P6.4)
  → AgentLoop ready; prior turns already on disk for hydrate_session

send_message / send_message_to
  → handle_turn → sink.append_turn (JSONL lines)
  → OPFS write survives reload

reload → new init()
  → same OPFS root → LocalFileSink.read_turns on first hydrate
  → get_history(group) returns prior turns without calling the LLM
```

No separate in-memory `Vec<ChatMessage>` is required; the durable
store is the session JSONL (superset: user / assistant / tool turns).

## Config persistence (P6.4)

On successful `init`, the raw `config_json` string is written to
`/clawft/.clawft/config.json` when the FS backend is OPFS (or always
written when a platform FS is available — memory backends lose it on
reload, which is correct).

`load_persisted_config()` returns that snapshot for JS UIs that want
to re-hydrate the form after reload. **Security**: this may contain
provider API keys (same origin-scoped trade-off as WEFT-14 env
persistence). Prefer short-lived tokens.

## Tests

| Suite | Feature | What |
|-------|---------|------|
| `history_layout` unit tests | `browser` | Path constants + `validate_group_id` |
| `browser_history_persist` | `browser-opfs` | JSONL + CLAWFT.md + config survive FS reopen (reload sim) |
| Existing `browser_opfs` / `browser_env_persist` | `browser-opfs` | FS + env (WEFT-13/14) |

```bash
FEATURES=browser-opfs scripts/build.sh test-browser
```

## Related

- WEFT-13 / WEFT-392 — OPFS `BrowserFileSystem`
- WEFT-14 — OPFS `BrowserEnvironment`
- M3 store collapse — `LocalFileSink` JSONL format
- `docs/browser/architecture.md` — feature flag matrix
- `docs/guides/browser.md` — operator notes
