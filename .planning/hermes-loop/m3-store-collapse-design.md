# M3 — Store Collapse: Design

**Status:** design-only (no code, no commits). Branch `feat/hermes-loop-base`.
**Goal:** one engine → one store. Collapse the duplicate conversation-state
stores so LLM-context assembly and the durable/witnessed record read from a
single source (ADR-062 build-target #5 "Join the graphs").

Re-verified against HEAD `fd2d6c87` (M2 + M4 landed). All `file:line`
anchors below are current-HEAD.

---

## 1. What actually exists at HEAD (recon, re-verified)

The turn write path is `AgentLoop::handle_turn` (`clawft-core/src/agent/loop_core.rs:711`).
Per turn it writes **two** independent per-turn stores under **two** key
schemes, then fans one of them out to three more:

| # | Store | Key | Written at | Read for prompt? |
|---|-------|-----|-----------|------------------|
| 1 | **SessionManager JSONL** (`clawft-core/src/session.rs`) — `~/.clawft/workspace/sessions/{encoded}.jsonl` | `"{channel}:{chat_id}"` (`clawft-types/src/event.rs:45`) | `add_message` user `:783`, assistant `:1062`; `save_session` `:1087` | **YES** — the only prompt source |
| 2 | **ConversationSink turn log** (`agent/sink.rs` + `substrate_sink.rs`) — `_derived/chat/<conv_id>/turns/<ulid>` | `conv_id = chat_id` (`loop_core.rs:730`) | `append_turn` user `:791`, assistant `:1070`, **tool intermediates** `:1606/:1771` | no |
| 3 | **Causal forest** (`CausalGraph` + crossrefs) | chain seq | via `SessionTier::with_forest` (`daemon.rs:1159`) | no (graft only) |
| 4 | **SessionTier L2 `SessionView`** (semantic HNSW graft index) | chain seq | `tier.index_turn` (`substrate_sink.rs:335`) | via graft splice `loop_core.rs:852` |
| 5 | **Redundant raw `ecc_hnsw` insert** (hash-embed) | `turn_id` | `substrate_sink.rs:347-359` | KPI/attest only (see §4) |

**The read path.** `ContextBuilder::build_messages(&session, …)`
(`loop_core.rs:780`) → `build_messages_inner` → `session.get_history(memory_window)`
(`context.rs:534-535`). It reads **store 1 only**. Stores 2-5 are write-only
from the LLM's perspective; the graft (store 4) splices a *separate* block, it
does not supply history.

**The asymmetry that dominates this milestone.** Store 1 receives only
`user` + **final** `assistant` messages. Store 2 receives those **plus every
tool-intermediate turn** (`run_tool_loop` appends at `:1606/:1771`; the
comment at `:1064-1067` says so explicitly). **Store 2 is a superset of
store 1.** Any "read history from the sink" design must reproduce store 1's
exact record subset or the assembled prompt changes.

**Fan-out nuance (committed M2 config).** When chain + causal + crossrefs are
all on, the daemon hands the anchor `None` for causal (`daemon.rs:1132-1137`,
`tier_owns_forest`) so store 3 is written by `SessionTier::with_forest`, not by
`KernelTurnAnchor`. The anchor keeps its own causal write only in the legacy
chain-less config. Store 3 is therefore already single-writer in production.

---

## 2. Decisions

### D1 — Canonical durable store: the ConversationSink. SessionManager is retired from the turn path.

The substrate turn log is durable, witnessed (chain seq per turn,
`substrate_sink.rs:329`), and already feeds the forest + tier that ADR-062
makes the conversation's source of truth. SessionManager JSONL duplicates its
text under a second key and buys nothing the sink lacks — **except** it is the
current read path and the current home of session metadata. So we do not
delete SessionManager's *file format*; we demote it to a **backend of the
sink** for the no-daemon case (D4) and move its metadata role out (D3).

The in-memory `Session` struct **stays** — it remains the per-turn assembly
buffer `ContextBuilder` reads. What changes: it is **hydrated from the sink**
and **no longer independently persisted**. `save_session` becomes a no-op on
the daemon path; the sink already persisted each turn as it happened.

### D2 — Read path unchanged in shape; hydration changes its source.

`ContextBuilder::build_messages(&session,…)` and
`session.get_history(window)` are **not touched**. This is deliberate: keeping
the assembly input (an in-memory `Session` with `.messages`) byte-identical is
the strongest guarantee that prompts do not drift. The change is upstream —
`get_or_create` hydrates `Session.messages` from `sink.history(conv_id)`
instead of from the JSONL file.

**New trait method:** `ConversationSink::history(&self, conv_id, window) ->
Vec<Turn>`. Substrate impl lists `_derived/chat/<conv_id>/turns/` (ULID +
counter-prefixed, already sortable — `substrate_sink.rs:406-410`) and replays;
local impl (D4) reads the file.

**Hydration filter (the load-bearing rule).** Because store 2 is a superset
(§1), `get_or_create` must reconstruct exactly store 1's subset: `user` turns
+ the **final** `assistant` turn of each exchange, **dropping tool
intermediates** (turns with `tool_calls`/`tool_call_id` set, and
non-final assistant turns). This rule is what the identical-prompt regression
test (Plan §T1) exists to prove.

### D3 — Session metadata (hallucination score) moves to a per-conv sidecar.

`Session.metadata[HALLUCINATION_SCORE_KEY]` is read at `loop_core.rs:957` and
updated at `:1048`. It persists in store 1 today. On collapse it moves to a
per-conv substrate metadata frame — reuse the sink's existing per-conv status
path (`_derived/chat/<conv_id>/status`, `substrate_sink.rs:5`) or a sibling
`…/meta` node — written on save, read at `get_or_create`. In-process
`LocalFileSink` writes it as a header line in the JSONL. This keeps the K/V
alive without keeping SessionManager as a store.

### D4 — In-process (no daemon): `LocalFileSink` reusing the existing JSONL layout.

Today in-process CLI uses `InMemorySink` (no-op) → sink is empty and store 1
carries everything. Replace the no-op with a **`LocalFileSink`** that
reads/writes the **existing** `~/.clawft/workspace/sessions/{encoded}.jsonl`
files. This is the migration story for in-process: **the old session files
become the new sink's backing store, unchanged.** No copy, no conversion.
`SessionManager`'s file I/O (`session.rs` load/save/encode) is refactored into
`LocalFileSink` rather than deleted.

### D5 — Canonical key: `"{channel}:{chat_id}"` (adopt SessionManager's scheme).

Two schemes today: store 1 = `{channel}:{chat_id}`, store 2 = bare `chat_id`.
Bare `chat_id` risks cross-channel collision (two channels, same chat id → one
sink dir). Adopt the **superset** key `{channel}:{chat_id}` as `conv_id`
everywhere:
- `LocalFileSink` keys on it → existing JSONL files (already named
  `{channel}:{chat_id}`) need **no rename**.
- Substrate turn path gains the channel prefix for **new** convs. Old convs
  (bare `chat_id`) are historical; mid-flight convs handled by a dual-read
  fallback in `sink.history` (try `{channel}:{chat_id}`, fall back to bare
  `chat_id`) kept for one release, then dropped.
- `loop_core.rs:730` changes from `msg.chat_id.clone()` to `msg.session_key()`.

### D6 — Kill store 5 (redundant hash-embed HNSW insert), repoint its readers.

`substrate_sink.rs:347-359` inserts every turn into the kernel-global
`ecc_hnsw` with a **non-semantic hash embedding**, duplicating what
`SessionTier::index_turn` already does semantically into `SessionView`.
Delete the block. Readers (§4) are repointed, not orphaned.

### D7 — Voice `TalkForest` is out of scope; M3 must not block ADR-068.

`clawft-voice-talk/src/forest.rs` (its own `TalkForest`) re-enters the shared
stores only via `agent.turn.record`. M3 touches the **text** turn path and the
**shared** sink/forest/tier. It does **not** touch `clawft-voice-talk`. The
sink's `TurnContent` enum is already voice-forward-compat (`Text`/`Audio`/
`Mixed`, `substrate_sink.rs:68-79`); the `history()` method and hydration
filter operate on `Turn` role/text and must **not** assume text-only in a way
that would reject `Audio`/`Mixed` when voice cuts over. That is the only
coupling; honoring it is sufficient.

---

## 3. Post-collapse data flow

```
handle_turn(msg):
  conv_id = msg.session_key()                # D5: {channel}:{chat_id}
  session = get_or_create(conv_id):          # D2 hydrate
      turns   = sink.history(conv_id, window)     # superset
      msgs    = filter_store1_subset(turns)       # D2 rule
      meta    = sink.meta(conv_id)                # D3 sidecar
      Session { messages: msgs, metadata: meta }
  context = ContextBuilder::build_messages(&session)   # UNCHANGED
  … graft splice (store 4, unchanged) …
  … LLM + tool loop → sink.append_turn(user, tool*, assistant) …   # one durable writer
  sink.set_meta(conv_id, hallucination_score)          # D3
  # NO SessionManager.save_session on daemon path
```

One durable store (sink → chain → tier → forest). One read (hydrate from sink).
Graft unchanged. Prompt bytes preserved because the assembly input `Session` is
reconstructed to store 1's exact subset.

---

## 4. Store 5 readers and their repoint (D6)

`ecc_hnsw` is **kernel-global**, not chat-only — other subsystems write it too,
so deleting the chat insert reduces the count, it does not zero it.

| Reader | `file:line` | Today | After D6 |
|--------|-------------|-------|----------|
| `ecc.status` → `hnsw_count` KPI | `daemon.rs:5624` | counts chat turns among others | count drops by chat turns; **accept** (it double-counted store 4 semantically). Optionally add `session_view_turns` sourced from `SessionTier` for a chat-specific KPI. |
| `ecc.search` → `entries`/`search_count` | `daemon.rs:5670-5674` | ditto | same; diagnostic only |
| `custody.attest` → `vector_count` | `daemon.rs:5745-5749` | **falls back** to `hnsw.len()` and **signs it** | attestation already prefers `ecc_vector_backend()` first (`:5739`); the `ecc_hnsw` branch is fallback-only. **This is the one load-bearing reader** — confirm the deployed config populates `ecc_vector_backend`; if it relies on the fallback, repoint the fallback to `session_tier`/vector-backend count before deleting, else the signed count changes. |

**Rejected:** "repoint the KPI to SessionTier's index" as a hard requirement.
`SessionView` is per-conversation, not a global count; wiring a global rollup
is more work than the KPI's value justifies. Recommendation: accept the
`ecc.status` delta (document it), and gate D6 on verifying `custody.attest`
does not depend on the `ecc_hnsw` fallback in the shipping config.

---

## 5. Rejected alternatives

- **Keep SessionManager as the read store; just drop the sink's duplication.**
  Can't — the sink is what is witnessed/durable/forested; dropping it loses the
  chain seq + graft index. This keeps two stores, not one. Rejected.
- **Make the sink turn log the *direct* assembly source (no `Session`
  reconstruction).** Would require teaching `ContextBuilder`/`assemble_windowed`
  to consume `Turn`s and skip intermediates inline — a bigger blast radius on
  the exact code whose output must stay byte-identical. Reconstructing the
  `Session` keeps the change upstream of the frozen assembly. Rejected as
  higher-risk for the same end state.
- **Canonical key = bare `chat_id`** (drop channel prefix). Loses channel
  disambiguation and forces a rename of every existing JSONL file. Rejected in
  favor of D5.
- **Delete SessionManager outright.** Its file I/O is the cheapest in-process
  backend; D4 reuses it. Rejected — demote, don't delete.
- **One-shot migration job to copy JSONL → substrate.** Unnecessary: daemon
  convs are re-hydrated lazily from the sink; in-process convs keep using the
  same files via `LocalFileSink`. No bulk migration needed (§ Plan compat).

---

## 6. Compatibility / failure modes

- **Old sink dirs keyed by bare `chat_id`** (pre-D5): `sink.history` dual-reads
  (`{channel}:{chat_id}` then bare) for one release. Documented sunset.
- **Mid-flight conversations across the upgrade:** first turn after upgrade
  hydrates from whatever the sink has; if the daemon was writing store 2 all
  along (anchors on), history is intact. If anchors were off, store 2 is empty
  → history starts fresh (same failure as any log-only store; acceptable, and
  loud via a one-time warn).
- **In-process, pre-existing JSONL:** read as-is by `LocalFileSink`. Zero
  migration.
- **Sink write failure:** already non-fatal (`loop_core.rs:804` warns and
  swallows). With the sink now canonical, a persistent write failure means the
  next turn hydrates without the lost turn — degrade, don't crash. Keep the
  warn; consider a health counter (Plan, optional).
- **Metadata sidecar absent** (old conv): `get_or_create` defaults
  hallucination score to 0.0 exactly as `loop_core.rs:960` does today.
