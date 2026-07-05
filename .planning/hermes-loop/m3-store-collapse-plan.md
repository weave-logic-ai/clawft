# M3 — Store Collapse: Plan

Phased, file-level change plan for the design in
`m3-store-collapse-design.md`. Design-only artifact — no code here.
Anchors are HEAD `fd2d6c87`. Build/test via `scripts/build.sh` only.

**Guiding invariant:** every phase keeps `scripts/build.sh test` green, and
the assembled prompt for a recorded conversation stays **byte-identical**
until the phase that intentionally changes the source (P3), which is guarded
by the golden test (T1) landed *before* it.

---

## Phase 0 — Regression harness first (T1). ~0.5 day

Land the guard **before** touching the write path.

- **New:** `clawft-core/tests/prompt_identity_golden.rs`.
  - Record a fixed multi-turn conversation (user → assistant, one turn with a
    tool call + tool result + final assistant, one plain turn) into a
    `SessionManager` **and** a `ConversationSink` stub, driving the *current*
    `handle_turn`.
  - Snapshot the assembled `Vec<LlmMessage>` from
    `ContextBuilder::build_messages` (serialize to a golden `.snap`).
  - Assert: prompt from store-1 read == prompt from the future
    hydrate-and-filter path (the filter of D2 applied to the sink superset).
  - This test **fails** until P3's filter is correct — that is the point.
- Reuse the existing snapshot harness pattern
  (`clawft-kernel/tests/snapshots/…`).

**Exit:** golden captured; filter-equivalence assertion written (may be
`#[ignore]` until P3 wires the path).

---

## Phase 1 — `ConversationSink::history` + `meta` read API. ~1 day

Additive; no behavior change.

- **Edit** `clawft-core/src/agent/sink.rs`: add to the `ConversationSink`
  trait:
  - `async fn history(&self, conv_id: &str, window: usize) -> Vec<Turn>`
  - `async fn meta(&self, conv_id: &str) -> serde_json::Value`
  - `async fn set_meta(&self, conv_id: &str, meta: serde_json::Value)`
  - Default impls: `history` → `vec![]`, `meta` → `Null`, `set_meta` → no-op
    (so `InMemorySink` and tests compile untouched).
- **Edit** `clawft-service-agent/src/substrate_sink.rs`: implement `history`
  via `SubstrateClient::list("_derived/chat/<conv_id>/turns", …)` + fetch +
  parse `Turn`; sorted by the ULID+counter path prefix (already ordered,
  `:406-410`). Implement `meta`/`set_meta` against
  `_derived/chat/<conv_id>/status` (or sibling `…/meta`) via the gated publish
  already used for the heartbeat.
- **Unit tests:** round-trip append→history; meta set→get; ordering across a
  same-ms burst (exercises the counter prefix).

**Exit:** sink can be read back; existing tests green.

---

## Phase 2 — `LocalFileSink` (in-process backend). ~1 day

- **New:** `clawft-core/src/agent/local_file_sink.rs` implementing
  `ConversationSink` over `~/.clawft/workspace/sessions/{encoded}.jsonl`.
  - Reuse `SessionManager`'s path resolution + key-encode + JSONL read/write
    (`session.rs:395` `encoded` scheme, load/save). Refactor those into shared
    helpers rather than duplicating.
  - `append_turn` → append a JSONL line; `history` → read + filter; `meta` →
    a header/sidecar line.
  - Keyed on `{channel}:{chat_id}` (D5) — matches existing filenames exactly.
- **Tests:** existing `~/.clawft/workspace/sessions` file reads unchanged
  (point a temp HOME at a fixture dir); append+history round-trip.

**Exit:** in-process durability available without `SessionManager` as a
distinct store; existing files readable.

---

## Phase 3 — Cut the read/write path over in `handle_turn`. ~1.5 days (riskiest)

This is where the collapse happens. Land T1's assertion **un-ignored** here.

- **Edit** `clawft-core/src/agent/loop_core.rs`:
  - `:730` `conv_id = msg.session_key()` (was `msg.chat_id`) — D5.
  - Add `filter_store1_subset(turns) -> Vec<(role, content)>`: keep `user`
    turns + the final `assistant` per exchange; drop turns with
    `tool_calls`/`tool_call_id`. Unit-tested in isolation.
  - `:776` hydrate: `let mut session = self.hydrate_session(&conv_id).await?`
    building `Session` from `sink.history` + filter + `sink.meta`. Keep
    `get_or_create` fallback for the no-sink test path.
  - `:1087` / `:1169` / `:1220` remove `save_session` on the daemon path;
    replace with `sink.set_meta(conv_id, hallucination_score)`.
  - Session metadata read `:957` now comes from the hydrated `meta` (D3).
  - Leave `ContextBuilder::build_messages` and `session.get_history`
    **untouched** (D2).
- **Wiring:** ensure the daemon path passes the `SubstrateConversationSink`
  (already constructed `daemon.rs:1237`) and the in-process/CLI path passes
  `LocalFileSink` where `InMemorySink` is used today (find the CLI/bootstrap
  construction site in `clawft-core/src/bootstrap.rs`).
- **T1 un-ignored:** assembled prompt byte-identical before/after.

**Exit:** one durable store; one hydrated read; golden green. `save_session`
gone from the daemon turn path.

---

## Phase 4 — Kill store 5 + repoint readers. ~0.5 day

- **Precondition check (gate):** confirm the shipping daemon config populates
  `ecc_vector_backend` so `custody.attest` (`daemon.rs:5739`) does **not** hit
  the `ecc_hnsw` fallback (`:5745`). If it does, repoint the fallback first.
- **Edit** `substrate_sink.rs`: delete the `if let Some(ref hnsw)` block
  (`:347-359`) and the now-unused `hash_embed` (`:~260-300`) + `hnsw` field +
  the `anchor_hnsw` plumbing (`daemon.rs:1114-1117`, `:1146`).
- **Optional** `daemon.rs:5624`: add `session_view_turns` KPI from
  `SessionTier` for a chat-specific count; document the `hnsw_count` drop.
- **Tests:** `ecc.status` still returns; attestation count sourced from the
  vector backend; no reader references the deleted insert.

**Exit:** store 5 gone; KPIs/attestation intact or explicitly re-sourced.

---

## Phase 5 — Retire SessionManager as a store; sunset scaffolding. ~0.5 day

- `SessionManager` file I/O now lives in `LocalFileSink`; remove its use as a
  turn store. Keep `sessions.rs` CLI (`clawft-cli/src/commands/sessions.rs`)
  working by pointing it at `LocalFileSink::history` (list/inspect still read
  the same files).
- Keep the D5 dual-read fallback (`{channel}:{chat_id}` → bare `chat_id`) with
  a `// remove after v0.x` marker.
- Docs: note the new single-store model in the ADR-062 / ADR-058 update stubs.

**Exit:** no code path writes two per-turn stores.

---

## Test plan

| ID | Test | Guards |
|----|------|--------|
| **T1** | `prompt_identity_golden.rs` — assembled `Vec<LlmMessage>` byte-identical, store-1 read vs sink-hydrate+filter, on a recorded conv incl. a tool-call exchange | The whole refactor. The single strongest regression guard. |
| T2 | `sink.history` ordering across same-ms burst | ULID+counter ordering vs append order |
| T3 | `filter_store1_subset` unit: drops tool intermediates + non-final assistant, keeps user + final assistant | D2 rule correctness in isolation |
| T4 | `LocalFileSink` reads a pre-existing fixture `.jsonl` unchanged | In-process migration (D4) |
| T5 | metadata sidecar round-trip; absent-meta → 0.0 default | D3 |
| T6 | key: two channels same `chat_id` → distinct convs | D5 collision fix |
| T7 | `custody.attest` count sourced from vector backend, not deleted insert | D6 attestation safety |
| T8 | full `scripts/build.sh test` + `scripts/build.sh gate` | workspace regression |
| T9 | voice smoke: `agent.turn.record` path still commits (feature `voice`) | D7 non-blocking |

**Golden discipline:** capture T1 at HEAD before P1; re-run at every phase.
Any diff is a stop-the-line event, not a snapshot-refresh.

---

## Estimates & sequencing

~5 dev-days. Strict order **P0 → P1 → P2 → P3 → P4 → P5** (P4 may run parallel
to P3 only after the attestation gate clears). P0 must precede all writes.

## Swarm shape

Small, sequential — this is a correctness-critical refactor on the live turn
path, not fan-out work.

- **researcher** (done — this design). Hands off to architect.
- **coder-sink** — P1 + P2 (sink read API + LocalFileSink). Isolated, testable.
- **coder-loop** — P3 (handle_turn cutover). Depends on coder-sink; owns T1.
- **coder-ecc** — P4 (store 5 kill) + attestation gate. Parallel after gate.
- **tester** — owns T1 golden capture, runs T8/T9, stop-the-line on any prompt
  diff.
- **reviewer** — gates P3 and P4 (the two behavior-changing phases).

Pipeline: `coder-sink → coder-loop → tester ↔ reviewer`, `coder-ecc` joins
after the attestation gate. Keep max 4 active; hierarchical.

## Riskiest calls (carried to the handoff)

1. **The superset filter (D2/T3).** Store 2 has tool intermediates store 1
   never had; the hydration filter must reproduce store 1's exact subset or
   prompts drift. T1 is the guard; get it green before trusting P3.
2. **`custody.attest` signs `hnsw.len()` as a fallback** (`daemon.rs:5745`).
   Killing store 5 changes a **signed** count if the deployed config relies on
   the fallback. P4 is gated on verifying `ecc_vector_backend` is populated.
3. **Session-metadata home (D3).** Losing the hallucination-score K/V when
   SessionManager stops persisting would silently disable the verification
   feedback loop. The sidecar + T5 cover it; regression is invisible without a
   test, so T5 is mandatory, not optional.
