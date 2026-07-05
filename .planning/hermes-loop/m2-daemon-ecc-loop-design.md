# M2 — Daemon-Hosted ECC Loop for Text

**Milestone**: M2 of the "one conversation engine" effort
**Branch**: `feat/hermes-loop-base`
**Concretizes**: WEFT-606 (host the TalkModeLoop in the daemon) + WEFT-614 (give text an impulse source)
**Design authority**: ADR-062 (ECC graph-walk conversation), build-target #5 "Join the graphs"
**Status**: DESIGN ONLY — no code changes in this milestone doc
**Companion**: `m2-daemon-ecc-loop-plan.md` (file-level change plan, test plan, phasing, non-goals)

---

## 1. Goal

Make **text a first-class modality of the ECC conversation engine**. Today a text
turn that arrives via `agent.chat` is dual-written into the kernel-global forest as
an **inert** `"state":"frontier"` causal node + a `Frontier` `SessionView` chunk
(`session_forest::dual_write_turn`, driven by `SessionTier::index_turn`) — and
**nothing ever commits it**. There is no `TalkModeLoop` running in the daemon, so no
tick ever advances those nodes Frontier→Committed. Voice, by contrast, runs its own
separate `TalkForest` (a private `ImpulseQueue`/`CausalGraph`/`CrossRefStore`/
`SessionView`/`TalkModeLoop`) and only re-enters the shared daemon forest by re-recording
its committed turns through `agent.turn.record` — as fresh inert frontier nodes.

M2 closes this: **host one `TalkModeLoop` in the daemon, bound to the kernel-global
forest, and give text an impulse source so text turns commit Frontier→Committed on the
same forest voice will eventually share.**

## 2. Load-bearing facts established during recon

1. **`ecc_impulses` has no consumer today.** The daemon's DEMOCRITUS loop
   (`run_democritus_loop`, spawned at `boot.rs:1583`) works off the causal graph's
   *spectral coherence* — it does **not** drain `ImpulseQueue`. So a hosted
   `TalkModeLoop` becomes the **sole** consumer of `ecc_impulses` — no queue contention,
   no impulse-stealing between loops. (This was the single biggest risk; it is retired.)
2. **The impulse/floor/commit machinery is modality-neutral.** `TalkModeLoop::tick`
   is pure, synchronous, I/O-free graph work; EOU commit already accepts an explicit
   `chain_seq` in the impulse payload (`talk_loop.rs:299`, `payload_u64(imp,"chain_seq")`).
3. **`chain_seq` is globally unique.** It is assigned by the kernel-global
   `ChainManager::append` inside `KernelTurnAnchor::anchor_turn` — one monotone sequence
   across all conversations. The loop's `lineage: DashMap<u64, TurnRef>` keyed by
   `chain_seq` is therefore already safe across conversations.
4. **`agent.chat` and `agent.turn.record` converge on `ConversationSink::append_turn`
   → `KernelTurnAnchor::anchor_turn`.** `KernelTurnAnchor` is constructed **only** in the
   daemon (`daemon.rs:1005`), with real kernel handles; the in-process CLI path uses
   `InMemorySink`/`NoopTurnAnchor` and never touches a kernel. This makes the anchor the
   unique convergence seam that fires for both RPC paths and is *absent* for CLI.
5. **`SessionView` already has the full lifecycle** (`context_graft.rs`): `insert_chunk`
   (enters `Frontier`), `commit`, `transition`, `speculative`, `live_seqs`,
   `set_state`. What is missing is an *actor* that calls `commit`.
6. **`SessionTier` owns per-conversation `SessionView`s** in `DashMap<String, Arc<SessionView>>`.
   The `TalkModeLoop` holds a *single* `Arc<SessionView>`. This mismatch is the core
   structural reconciliation M2 must make (Decision 4).
7. **Wire metadata is dropped.** `inbound_from_params` (`service.rs:436`) builds
   `metadata: HashMap::new()` — so `skill_instructions`/`allowed_tools` that the in-process
   path reads from `InboundMessage.metadata` (`loop_core.rs:788/845`) never reach the
   daemon loop. `AgentChatParams` has no metadata field to carry them.

---

## 3. Architecture (text turn → committed node)

```
 agent.chat RPC ─┐                        (in-process CLI: InMemorySink/NoopAnchor,
 (daemon.rs:4352)│                         NO kernel → NO anchor → NO impulse. Correct.)
                 ▼
   AgentService::dispatch (per-conv Mutex; serialized)   [service.rs:246]
     inbound_from_params  ── + threads params.metadata → InboundMessage.metadata  (Decision 5)
                 ▼
   AgentLoop::handle_turn                                  [loop_core.rs:617]
     ├─ append user  Turn ─┐
     │  run_tool_loop (LLM + tools)   ← M4 will insert tool-nodes here (Decision 3)
     └─ append assistant Turn ─┐
                              ▼
   SubstrateConversationSink::append_turn  (durable JSONL first)  [substrate_sink.rs:584]
                              ▼
   KernelTurnAnchor::anchor_turn  ── chain.append() ⇒ GLOBAL chain_seq  [substrate_sink.rs:304]
                              ▼
   SessionTier::index_turn(conv_id, chain_seq, role, text)          [session_tier.rs:155]
     ├─ view.index_chunk(...)                → SessionView chunk  (Frontier)
     ├─ session_forest::dual_write_turn(...) → CausalGraph node   (Frontier) + Follows + Speaker xref
     └─ NEW (Decision 2):
          talk_loop.register_turn(chain_seq, node, uid, conv_id)
          impulses.emit(EndOfUtterance, {chain_seq, conv_id}, hlc)
                              ▼
   ══════════ kernel-global forest: ecc_causal / ecc_crossrefs / ecc_impulses ══════════
                              ▼
   TalkModeLoop  (daemon-hosted SystemService, run_talk_loop on ecc_tick cadence)  (Decision 1)
     tick():  SENSE drain_ready(ecc_impulses)
              FLOOR (degenerate for text)
              MUTATE  EndOfUtterance → commit_turn(seq):
                        view = views.view_for(conv_id)     (ViewResolver → SessionTier)  (Decision 4)
                        mirror_state(view, seq, ecc_causal, node, Committed)
                              ▼
              SessionView chunk  Frontier→Committed   +   CausalGraph node "state" → committed
```

The user turn and the assistant reply **both** traverse `append_turn → anchor →
index_turn`, so **both** get registered and EOU-committed. The reply commits as a
`Committed` node linked `Follows` to the user turn (the `Follows` edge is already drawn
by `dual_write_turn` from the previous turn). That is the text `CommittedReply`.

**Idle lifecycle** (Decision 6): a coarse-cadence daemon reaper (not the 50ms tick)
walks `SessionTier::active_conversations()`, and for each conversation idle beyond a
timeout runs the existing end path (postmortem → `promote_and_drop`/`drop_view`) and
calls a new `talk_loop.end_conversation(conv_id)` to evict per-conv loop state.

---

## 4. Decisions

### D1 — Loop hosting model: ONE multiplexed daemon loop

**Chosen:** a single daemon-hosted `TalkModeLoop`, registered as a `SystemService`
alongside the DEMOCRITUS coherence loop, driven by `run_talk_loop` on `ecc_tick`'s
self-calibrating cadence, multiplexing **all** conversations (text now; voice later).

**Rationale:**
- **Idle conversations burn nothing.** The loop only does work when impulses are
  pending; an idle conversation emits no impulses, so a multiplexed loop's tick is a
  cheap empty drain. Per-conversation loops would each spawn a tokio task sleeping on
  the tick cadence — N idle tasks for N conversations, unbounded.
- **`chain_seq` is globally unique** (fact 3), so one `lineage` map keyed by `chain_seq`
  already serves all conversations. Per-conv dispatch serialization (the `AgentService`
  `DashMap<ConvId, Mutex>`) means at most one turn per conversation is mid-flight.
- **Voice's convergence target.** In a later milestone voice stops running its own
  `TalkForest` and instead emits its capture impulses into *this* shared loop's queue;
  the multiplexed daemon loop is the thing voice becomes a client of. Building it
  per-conv now would have to be undone.
- **`ecc_impulses` has no other consumer** (fact 1), so the multiplexed loop can own it
  outright.

**Cost / entailment:** the current `TalkModeLoop` is subtly single-conversation — it
holds one `current_turn: Option<u64>` and one `view: Arc<SessionView>`. Multiplexing
requires: (a) view resolution per conversation (Decision 4), and (b) making the
in-flight-turn slot per-conversation. Because text always emits EOU with an **explicit**
`chain_seq` in the payload, commit never depends on `current_turn`; `current_turn`
becomes a per-conv `DashMap<String,u64>` used only by the floor read and barge-in, both
degenerate for text. This is a breaking change to `TalkModeLoop::new`, **approved
pre-1.0 under K2 Symposium D19**.

**Rejected:** per-conversation loop instances (voice's current model) — N idle tick
tasks, re-creates the per-conversation-forest fragmentation M2 exists to end, and would
be thrown away at voice cutover.

### D2 — Text ImpulseSource seam: `SessionTier::index_turn`, at the anchor convergence point

**Chosen:** emit `register_turn` + `EndOfUtterance` from inside `SessionTier::index_turn`,
immediately after `session_forest::dual_write_turn` returns the causal node id.

**Rationale — this is the *only* seam that satisfies all three constraints:**
- **Downstream of `chain_seq` assignment.** `chain_seq` is minted by `chain.append` in
  `anchor_turn`, which then calls `index_turn(conv_id, chain_seq, …)`. The impulse must
  carry `chain_seq`; only code at/after `index_turn` has it.
- **Shared by both RPC paths.** `agent.chat` and `agent.turn.record` both reach
  `append_turn → anchor_turn → index_turn`. Emitting here covers voice turns recorded
  via `agent.turn.record` for free — satisfying the "must also work for
  agent.turn.record voice turns" constraint.
- **Absent for in-process CLI.** CLI uses `NoopTurnAnchor`; `index_turn` is only ever
  called from `KernelTurnAnchor`, which only exists in the daemon with a live kernel.
  CLI never fires an impulse — satisfying "must not fire for the in-process fallback."

`index_turn` already computes everything needed: `conv_id`, `chain_seq`, the causal node
id (currently discarded by `dual_write_turn`'s return), and `turn_universal_id` (already
`pub(crate)` in `session_forest`). The change is: capture the returned node id, recompute
the uid, and — when a `TalkModeLoop` handle is attached — `register_turn` + `emit(EndOfUtterance)`.

**Rejected:**
- **`AgentService::dispatch`** — runs before the sink append, has no `chain_seq`, and
  `agent.turn.record` bypasses `AgentService` entirely (it calls the sink directly), so
  voice turns would be missed.
- **`loop_core::handle_turn`** — fires for CLI too (cannot cleanly detect kernel
  presence), has no `chain_seq` (assigned downstream), and misses `agent.turn.record`.
- **`KernelTurnAnchor::anchor_turn` directly** — workable (it has the node id via a small
  refactor) but `index_turn` is the natural owner of the forest dual-write; keeping the
  impulse emission adjacent to the dual-write keeps one module responsible for the
  forest projection.

### D3 — Reply commit path + `run_tool_loop` timing

**Chosen for M2:** the assistant reply is a **directly-Committed** node. Because the
assistant `Turn` flows through the same `append_turn → anchor → index_turn` path (D2),
it is registered and EOU-committed exactly like the user turn, linked `Follows` to it.
There is **no** Speculative→Committed supersession for text — that lifecycle
(ADR-062 D3: a fast spoken ack node superseded by the deep answer) is *voice-specific*
and stays in voice's `LoopObserver`. Text has no fast/slow tier, so the reply is born
`Committed`.

**`run_tool_loop` seam for M4 (designed, not wired):** `run_tool_loop` runs *inside*
`handle_turn`, between the user `append_turn` and the assistant `append_turn`. So the
tick timeline is already: **user turn EOU/commit → run_tool_loop (LLM + tools) →
assistant turn EOU/commit**. M4's "tools as part of a committed turn" slots in exactly
there: `run_tool_loop` will emit per-tool causal nodes/impulses between the two commits,
linked `Enables`/`EvidenceFor` to the reply node. M2 leaves `run_tool_loop` untouched
and only guarantees the two bracketing commits exist, so M4 has a stable frame to insert
into. The richer reply-role edges (`EvidenceFor` reply→turn, `Enables` spec→committed)
that voice's `LoopObserver` draws are **not** added for text in M2; the plain `Follows`
lineage is sufficient and a later increment can enrich it.

### D4 — SessionView frontier unification: one view type, one instance, resolved by injection

**Chosen:** `SessionView` (which already carries the full frontier lifecycle) is the
single view type. `SessionTier` remains the sole owner of per-conversation
`SessionView`s. The `TalkModeLoop` stops holding a single `Arc<SessionView>` and instead
holds an injected `Arc<dyn ViewResolver>` (`fn view_for(&self, conv_id: &str) ->
Option<Arc<SessionView>>`). `SessionTier` implements `ViewResolver` by wrapping its
existing `existing_view`. The loop's `TurnRef` gains `conv_id` so `commit_turn`/`prune_turn`
resolve the correct per-conversation view before calling `mirror_state`.

**Rationale:** the entire point of M2 is that a text turn commits on the *same*
structure graft reads from. If the loop committed a *different* view than the one
`SessionTier` grafts from, we'd have re-created the double-view problem. Making the loop
resolve `SessionTier`'s view keeps commit and graft on one object. `mirror_state` already
does `view.transition(seq, Committed)` + causal `set_state` atomically per the legal-step
gate — it just needs the *right* view.

**Voice stays green** with a one-line adapter: `TalkForest` passes a trivial
`SingleViewResolver` returning its one bookkeeping `SessionView` for any `conv_id`.

**Rejected:** giving the loop its own dedicated per-conv "lifecycle view" separate from
`SessionTier`'s "graft view" — that is the exact double-view fragmentation M2 removes.

### D5 — Wire parity: `AgentChatParams.metadata`

**Chosen:** add an optional free-form `metadata: Option<serde_json::Map<String, Value>>`
to `AgentChatParams` (serde `default`, `skip_serializing_if=None` — fully
backward-compatible). `inbound_from_params` copies it into `InboundMessage.metadata`.
Documented known keys, consumed by already-existing `loop_core` reads:
- `skill_instructions: String` → injected as a system note (`loop_core.rs:788`)
- `allowed_tools: [String]` → tool-subset filter (`loop_core.rs:845`)
- `model: String` → model override, carried in `ContextRequest.metadata` to the router
  (router-side consumption is out of M2 scope; the field is threaded now)
- `provenance` (e.g. `{"impulse_source":"agent.chat"}`) → diagnostic, carried for the
  witness/audit trail.

**Rationale:** this is the "wire-parity gap" — the daemon loop must see the same
per-turn metadata the in-process REPL injects. A free-form map is chosen over four typed
fields because the consuming keys already live in `loop_core` as string/array lookups on
a `HashMap`, and new keys (impulse provenance) are expected as the ECC path grows; a map
avoids a wire-schema bump per key. It is additive and independent of the loop work — it
can ship first (Phase P1).

**Rejected:** typed fields per key — churns the wire schema every time a new metadata key
is needed and duplicates the `loop_core` map-lookup contract.

### D6 — Lifecycle: idle-reaper owns it; explicit `agent.chat.end` stays an override

**Chosen:** text conversations have no natural "end" signal (the panel just stops
sending), so ownership goes to a **daemon-side idle reaper**: a coarse-cadence task
(propose 60s interval, **not** the 50ms talk tick) that walks
`SessionTier::active_conversations()` and, for each conversation whose last turn is older
than an idle timeout (propose 30 min, configurable), runs the existing end path
(postmortem digest → `promote_and_drop` when a durable fact exists, else `drop_view`) and
calls a **new** `talk_loop.end_conversation(conv_id)` that evicts that conversation's
`lineage`/`current_turn` entries.

The existing explicit `agent.chat.end` RPC (`daemon.rs:4399`) stays as an override (e.g.
panel tab-close) and additionally calls `talk_loop.end_conversation`.

**On end:** committed causal nodes **stay** on the kernel-global graph (they are
witnessed/immutable — the durable record). The ephemeral L2 `SessionView` is dropped
(existing behaviour). The loop's per-conv `lineage` is evicted (new) — without this,
`lineage` grows unbounded across the daemon's lifetime.

**Rationale:** something must own eviction or `lineage` (and the per-conv views) leak for
the process lifetime. The reaper is the minimum viable owner; explicit `end` is a latency
optimization, not a correctness requirement.

### D7 — Config: ship an opt-in flag; do NOT flip global defaults in M2

**Chosen:** add `talk_loop: bool` (default `false`) to `AgentAnchorConfig`
(`config/kernel.rs`). The daemon spawns the `TalkModeLoop` and wires
`SessionTier.with_talk_loop` **only** when `talk_loop` is on. The text-ECC-commit path
also structurally requires `anchor_chain=true` (for `chain_seq`) and `anchor_causal=true`
(for the causal graph + cross-refs, i.e. the `tier_owns_forest` branch at
`daemon.rs:991`). The daemon logs a clear warning if `talk_loop=true` but its
prerequisites are off, and treats `talk_loop` as inert in that case (no partial wiring).

M2's dev `.clawft/config.json` (already dirty in the working tree) carries
`anchor_chain=anchor_causal=talk_loop=true` for the end-to-end test loop.

**Rationale:** flipping global defaults changes behaviour for *every* existing deployment
(adds per-turn embed + causal writes + commit ticks). That is a release-risk decision
belonging to the milestone that declares ECC-text GA — **not** M2, whose job is to make
the path *work* behind a flag. `anchor_hnsw` (the redundant hash-embed insert) stays
independent and untouched; collapsing it is M3.

**Rejected:** flipping `anchor_chain`/`anchor_causal`/new-flag defaults to true in M2 —
unscoped behaviour change for all deployments.

### D8 — Failure modes

- **Loop crash mid-conversation.** `TalkModeLoop::tick` is pure, synchronous,
  I/O/await-free; the only realistic panic is lock poisoning. Register the loop with the
  daemon's existing `SERVICE_RESTART_COUNTS`/`service.restart` supervision. On restart the
  `lineage` map is lost but the views + causal graph persist; re-registration happens on
  the next turn. Turns that were `Frontier` at crash time stay `Frontier` forever — but
  they remain in durable substrate JSONL, so nothing is lost; only the immutability
  witness is missing. Accept + log for M2; a start-time reconciliation sweep (commit
  aged frontier chunks) is a follow-up, not M2.
- **Impulse emitted but commit never happens** (loop starved / not running). The turn
  stays `Frontier`. **Graft reads `Frontier` *and* `Committed`** (`live_seqs` + committed
  chunks), so the conversation still functions — commit is about the witness/immutability
  guarantee, not reply correctness. **Graceful degradation to "uncommitted but
  functional."** Extend the loop's existing `health_check`: if the oldest pending EOU
  impulse ages beyond N ticks, report `HealthStatus::Degraded`.
- **Backpressure / tick starvation — and a latent data-loss bug.** `ImpulseQueue::drain_ready()`
  removes **all** pending impulses, then the loop does `impulses.truncate(max_impulses_per_tick)`
  (`talk_loop.rs:216`) — **the overflow beyond `max` (default 64) is dropped on the floor,
  not re-queued.** Under a flood, EOUs past 64/tick would be silently lost and those turns
  would never commit. Text volume won't hit 64 EOU/tick, but M2 should fix this while it is
  in the code: either drain only up to `max` (leave the rest queued) or re-emit the
  truncated overflow. Flagged as a required small fix in the change plan.

---

## 5. Non-goals (explicit)

- **M3 store collapse** — `SessionManager` JSONL vs `ConversationSink` turn log; the
  anchor's redundant `hash_embed` `HnswService` insert duplicating `SessionTier`'s
  semantic one; voice's separate `TalkForest`. M2 draws the design boundary (Decision 4
  unifies the *view*; it does not collapse the duplicate *stores*).
- **M4 tool repositioning** — tools as part of a committed turn. M2 designs the
  `run_tool_loop` seam (Decision 3) but wires nothing.
- **Voice cutover** — voice keeps its own `TalkForest` + `agent.turn.record` in M2; it
  only gains the one-line `SingleViewResolver` adapter to stay green. Voice becoming a
  client of the daemon loop is a later milestone.
- **Speculative-ack / overt-repair for text** — no fast/slow tier for text (Decision 3).
- **Learned floor weights / non-degenerate floor for text** — the floor read stays
  degenerate for the single-frontier text case (ADR-062 defers learned weights anyway).

---

See `m2-daemon-ecc-loop-plan.md` for the file-level change plan, test plan, and the
phased implementation order for the M2 swarm.
