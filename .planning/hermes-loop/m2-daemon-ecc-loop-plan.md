# M2 — Change Plan, Test Plan, Phasing

Companion to `m2-daemon-ecc-loop-design.md`. All file paths absolute-from-repo-root.
Line anchors are current-as-of-recon (2026-07-04) and will drift; treat as landmarks.

---

## 1. File-level change plan

Ordered by dependency. Sizes are net-added-line estimates; the 500-line ceiling per
CLAUDE.md is called out where a file is close.

### 1.1 `crates/clawft-kernel/src/view_resolver.rs` (NEW, ~40 lines)
Define the injection seam that lets the multiplexed loop resolve per-conversation views.
```rust
pub trait ViewResolver: Send + Sync {
    fn view_for(&self, conv_id: &str) -> Option<Arc<SessionView>>;
}
/// Trivial single-view resolver (voice's TalkForest, tests).
pub struct SingleViewResolver(Arc<SessionView>);
impl ViewResolver for SingleViewResolver { /* returns the one view for any conv_id */ }
```
Re-export from `lib.rs`. Kept in its own file (not `context_graft.rs`, which is already
large) to stay clear of the ceiling.

### 1.2 `crates/clawft-kernel/src/talk_loop.rs` (MODIFY, ~+70/−15; currently 461 → watch ceiling)
The multiplex refactor (Decision 1 + 4). If this pushes past 500, split the
`SystemService` impl + `run_talk_loop` into a sibling `talk_loop_service.rs`.
- `TalkModeLoop::new`: replace `view: Arc<SessionView>` param with
  `views: Arc<dyn ViewResolver>`. **Breaking change (D19-approved).**
- `struct TurnRef`: add `conv_id: String`.
- `register_turn(chain_seq, node, uid)` → `register_turn(chain_seq, node, uid, conv_id)`.
- `TalkInner.current_turn: Option<u64>` → per-conv `DashMap<String, u64>` (or drop the
  single slot; text commit uses explicit `chain_seq` from payload and never reads it).
- `commit_turn`/`prune_turn`: resolve `views.view_for(&turn_ref.conv_id)` before
  `mirror_state`; fall back to no-op (log) when the view is gone (conversation reaped).
- `read_floor`: iterate the resolved per-conv view's `live_seqs()` scoped to the
  impulse's conv (degenerate for text — single frontier).
- NEW `end_conversation(&self, conv_id: &str)`: evict all `lineage` entries and the
  `current_turn` slot for `conv_id` (Decision 6, prevents unbounded growth).
- **FIX (Decision 8):** `tick()` SENSE must not drop overflow. Change
  `impulses.truncate(max)` semantics so the overflow stays queued (drain only `max`, or
  re-emit the tail). Add a unit test that a >max flood commits every seq across ticks.

### 1.3 `crates/clawft-service-agent/src/session_tier.rs` (MODIFY, ~+45 lines)
- `impl ViewResolver for SessionTier { fn view_for(c) { self.existing_view(c) } }`.
- `with_talk_loop(mut self, loop_: Arc<TalkModeLoop>) -> Self` builder + field
  `talk_loop: Option<Arc<TalkModeLoop>>`.
- In `index_turn`, after `dual_write_turn(...)` (capture its returned `CausalNodeId`):
  when `talk_loop` is set, compute `uid = session_forest::turn_universal_id(conv_id,
  chain_seq, text)`, call `loop_.register_turn(chain_seq, node, uid, conv_id)`, then emit
  `EndOfUtterance` onto the forest's `ImpulseQueue` with payload `{chain_seq, conv_id}`.
  The impulse queue handle is threaded in via `with_talk_loop` (or a sibling
  `with_impulses`) — the tier needs the `Arc<ImpulseQueue>` the loop drains.
- Note: `dual_write_turn` currently returns the node id but `index_turn` discards it —
  capture it now.

### 1.4 `crates/clawft-service-agent/src/session_forest.rs` (MODIFY, ~+2 lines)
`turn_universal_id` is already `pub(crate)`. No structural change; confirm `dual_write_turn`
returns the `CausalNodeId` (it does) and that the emotion/goal `None,None` call sites are
unaffected.

### 1.5 `crates/clawft-types/src/agent_chat.rs` (MODIFY, ~+10 lines)
Add to `AgentChatParams` (Decision 5):
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
```
Update the two existing test constructors (`params_for` in `service.rs` tests, the inline
tests here) to set `metadata: None`.

### 1.6 `crates/clawft-service-agent/src/service.rs` (MODIFY, ~+8 lines)
`inbound_from_params`: replace `metadata: HashMap::new()` with a copy of
`params.metadata` (JSON `Map` → `HashMap<String, Value>`). Add a unit test asserting
`skill_instructions` survives the hop (mirrors `inbound_from_params_picks_last_user_content`).

### 1.7 `crates/clawft-types/src/config/kernel.rs` (MODIFY, ~+6 lines)
Add `talk_loop: bool` (serde `default = false`) to `AgentAnchorConfig`. `any_enabled()`
is unchanged (talk_loop implies the others; the daemon validates rather than folding it
into `any_enabled`).

### 1.8 `crates/clawft-weave/src/daemon.rs` (MODIFY, ~+65 lines)
Boot wiring, in the existing agent-sink block (`~940–1051`) and RPC arms:
- After the `SessionTier` is built and forest-joined (`~1018`), when
  `anchor_cfg.talk_loop && tier_owns_forest`:
  - build `TalkModeLoop::new(ecc_impulses, ecc_causal, ecc_crossrefs, tier.clone() as
    Arc<dyn ViewResolver>, ecc_tick.clone(), TalkModeConfig::default())`;
  - `tier = tier.with_talk_loop(loop_.clone())` **before** the `Arc::new(tier)` (ordering:
    the tier must hold the loop handle + impulse queue);
  - register the loop as a `SystemService` and `tokio::spawn(run_talk_loop(loop_,
    cancel_child))` under the daemon's cancellation token;
  - stash the loop in a `OnceLock` (like `DAEMON_AGENT`) so `agent.chat.end` and the
    reaper can call `end_conversation`.
  - log a warning + skip if `talk_loop` set but `tier_owns_forest` false (Decision 7).
- NEW idle reaper: a `tokio::spawn` interval task (60s) over
  `session_tier.active_conversations()`; for idle convs, run the same postmortem/promote
  path `agent.chat.end` uses, then `talk_loop.end_conversation(conv_id)` (Decision 6).
  Needs per-conv last-activity tracking — add a `DashMap<String, Instant>` touched in the
  `agent.chat`/`agent.turn.record` arms, or read the newest turn ts from the view.
- `agent.chat.end` arm (`~4427`): after `end_conversation`, also call
  `talk_loop.end_conversation(&conv_id)`.

### 1.9 `crates/clawft-voice-talk/src/forest.rs` (MODIFY, ~+8 lines)
`TalkForest::new`: wrap the bookkeeping `view` in `SingleViewResolver` and pass it to the
new `TalkModeLoop::new` signature. `LoopObserver::dual_write_turn` already calls
`register_turn(seq, node, uid)` — update to the 4-arg form with `self.forest.conv_id()`.
Keeps voice green (Decision 4).

---

## 2. Test plan

### 2.1 Kernel unit (`clawft-kernel/src/talk_loop_tests.rs`)
- **`multiplex_commits_two_convs_independently`**: register one turn in `conv-a` and one
  in `conv-b` (distinct `chain_seq`), emit EOU for each, one `tick()`, assert each conv's
  resolved view committed its own seq and neither touched the other.
- **`eou_with_explicit_chain_seq_commits_named_seq`**: EOU payload `chain_seq` commits
  that seq regardless of `current_turn`.
- **`end_conversation_evicts_lineage`**: after `end_conversation(conv)`, a subsequent EOU
  for an evicted seq is a no-op (lineage gone), and `lineage` length dropped.
- **`drain_overflow_not_dropped`** (Decision 8 fix): emit `max+10` EOUs, tick until
  drained, assert all `max+10` seqs committed (none silently lost).

### 2.2 Service-agent unit
- `SessionTier::view_for` returns the same `Arc` `graft_block` uses (pointer/identity check).
- `index_turn` with a mock `TalkModeLoop` handle registers the turn and emits exactly one
  `EndOfUtterance` carrying the right `{chain_seq, conv_id}`.
- `inbound_from_params` threads `metadata.skill_instructions` into `InboundMessage.metadata`.

### 2.3 Integration — the headline test (text analogue of `live_native_talk_session`)
New `crates/clawft-service-agent/tests/text_ecc_commit.rs` (or a daemon-level test):
1. Build a `SessionTier` forest-joined to a real `CausalGraph`/`CrossRefStore`, a
   `TalkModeLoop` over a shared `ImpulseQueue`, `tier.with_talk_loop(loop)`.
2. Drive turn 1: `index_turn(conv, seq1, "user", "hello")`. Assert the `SessionView`
   chunk for `seq1` is `Frontier` and the causal node `"state"` is `"frontier"` and one
   `EndOfUtterance` is queued.
3. `loop.tick()`. Assert `seq1` transitioned **Frontier→Committed** in the view **and**
   the causal node `"state"` → `"committed"`.
4. Drive turn 2 (assistant reply) `index_turn(conv, seq2, "assistant", "hi there")`;
   `tick()`. Assert `seq2` committed **and** a `Follows` edge exists seq1→seq2 (the
   committed reply lineage).
5. Assert graft (`graft_block`) over the committed conversation still returns both turns
   (commit did not evict them from the index).

This proves the M2 thesis end-to-end: **text → impulse → tick commit → committed reply on
the shared forest.** Runs without weights (Mock embedder) and without a live daemon
socket, mirroring `live_native_talk_session`'s device-free assembly contract.

### 2.4 Regression / green-keeping
- `cargo test -p clawft-voice-talk` — the `SingleViewResolver` adapter keeps voice's
  `forest_tests`/`session_tests` green.
- `scripts/build.sh test` (workspace) + `scripts/build.sh gate` before any commit.
- Feature-matrix: build with and without `exochain`/`voice`/`native` — the loop host is
  behind `talk_loop` config + the same `#[cfg(feature="native")] run_talk_loop` gate that
  exists today.

---

## 3. Phased implementation order (M2 swarm)

Each phase is independently reviewable; P1 and P2 have no dependency and can run in
parallel.

- **P1 — Wire parity (independent, shippable alone).** §1.5 + §1.6 + the metadata
  threading test (§2.2). No loop involvement. Closes the "metadata dropped" gap.
- **P2 — Kernel multiplex (independent of P1).** §1.1 + §1.2 (incl. the drain-overflow
  fix) + kernel unit tests (§2.1). Voice adapter §1.9 lands here to keep the workspace
  compiling. Delivers a multiplexed `TalkModeLoop` that no daemon path yet drives.
- **P3 — Impulse source.** §1.3 + §1.4 + config flag §1.7 + service-agent unit tests
  (§2.2). Depends on P2 (needs the new `register_turn` signature + `ViewResolver`).
- **P4 — Daemon host.** §1.8 — boot the loop as a `SystemService`, spawn `run_talk_loop`,
  wire `with_talk_loop`, the idle reaper, and the `agent.chat.end` eviction hook. Depends
  on P2 + P3.
- **P5 — Integration + enablement.** §2.3 headline test; set
  `anchor_chain=anchor_causal=talk_loop=true` in the dev `.clawft/config.json`; run the
  full end-to-end on the live daemon (`:8090`) per the current-state memory. Depends on
  all prior.

Suggested swarm shape (hierarchical, per CLAUDE.md): `kernel-specialist` owns P2;
`weaver-specialist` owns P1 + P4; a `coder` owns P3; a `tester` owns P5 + the regression
sweep; `reviewer` gates each phase. Coordinate via `SendMessage` (P2→P3→P4→P5 pipeline;
P1 fans out independently).

---

## 4. Open items to resolve during implementation (not blockers)

- **Impulse queue handle to `SessionTier`.** `index_turn` needs the `Arc<ImpulseQueue>`
  the loop drains. Cleanest: `with_talk_loop` also stashes `loop.impulses()` (add a public
  accessor on `TalkModeLoop`), or pass the queue explicitly via `with_impulses`. Decide in
  P3; prefer deriving it from the loop handle to keep one source of truth.
- **Last-activity tracking for the reaper.** Either a daemon `DashMap<String, Instant>`
  touched in the RPC arms, or read the newest turn's `ts_ms` from the view. Prefer the
  latter (no new state) if the view exposes it cheaply.
- **HLC source for the text EOU.** Voice uses a monotone `AtomicU64` stand-in
  (`LoopObserver::tick`). Text should use the same HLC the rest of the daemon forest uses
  (the chain's / a kernel HLC) so impulse ordering is consistent across modalities — check
  what `ecc_impulses` producers currently stamp and reuse it.
- **Reconciliation sweep on loop restart** (Decision 8) — commit aged `Frontier` chunks
  left by a pre-restart crash. Explicitly deferred out of M2; note it as a follow-up
  Plane item.
