# M4 — Change Plan, Test Plan, Phasing

Sibling: `m4-agent-initiated-work-design.md`. Estimates are engineer-hours for a
focused implementer who knows the crates. All new/edited files must stay under the
500-line ceiling (CLAUDE.md); files already near it are flagged.

---

## 1. File-level change plan

### Phase A — spawn substrate (no LLM surface yet)

**A.1 `crates/clawft-core/src/agent/spawn.rs`** (NEW, ~140 lines) — *2.5 h*
The `SubagentSpawner` trait + POD types: `SpawnSpec {goal, persona, skills, await,
budget, group_id, notify_on_complete, depth, backend}`, `SpawnBackend`
(enum mirror of the kernel's, default `Conversation`), `SpawnHandle {task_id,
child_conv_id, status}`, `TaskStatus`, `TaskOutcome`, `SpawnError` (incl.
`BackendNotAvailable`, `DepthExceeded`, `ConcurrencyExceeded`, `Denied`).
No daemon deps — pure types + the async trait. Add `pub mod spawn;` to
`agent/mod.rs`.

**A.2 `crates/clawft-service-agent/src/subagent.rs`** (NEW, ~220 lines — split if it
grows) — *6 h*
`DaemonSubagentSpawner` implementing `SubagentSpawner`:
- Holds `Weak<AgentService<…>>` (break the Arc cycle), an `Arc<SpawnRegistry>`, the
  `ChainManager` handle (witnessing), a `Arc<CausalGraph>` + `Arc<CrossRefStore>`
  (forest edges), and caps/config.
- `spawn()`: check depth + concurrency + gate-precheck; mint
  `child_conv_id = "sub:<parent>:<ulid>"`; register in `SpawnRegistry`; witness
  `agent.spawn`; draw `C.goal --TriggeredBy--> parent_turn` crossref; then either
  dispatch inline (`await:true`) or `tokio::spawn` a detached driver.
- The **child driver**: build `AgentChatParams {conv_id: C, messages:[{user, goal}],
  metadata:{persona, skills, spawn_depth}}` and call the same dispatch path the RPC
  uses; on completion draw `R_c --EvidenceFor--> parent_turn`, witness
  `agent.complete`, store the outcome in `SpawnRegistry`.
- `status/result/message/cancel` delegate to `SpawnRegistry` + `agent.chat.cancel`.

**A.3 `crates/clawft-service-agent/src/spawn_registry.rs`** (NEW, ~130 lines) — *2 h*
`SpawnRegistry`: `DashMap<TaskId, TaskRecord>` + `DashMap<GroupId, Vec<TaskId>>` +
per-parent-conv atomic live-count. `TaskRecord {task_id, parent_conv, child_conv,
group_id, status, outcome, join_handle}`. Insert/update/get/by-group/live-count.

**A.4 `crates/clawft-kernel/src/causal.rs` (or session_forest helper)** — *1.5 h*
Confirm cross-conversation `CrossRef` links between two conv nodes are expressible with
the existing `CrossRefStore` API. If `session_forest::dual_write_turn`'s crossref
helper is `pub(crate)`, add a thin `pub(crate) fn link_cross_conv(store, from_uid,
to_uid, edge)` in `session_forest.rs` (~15 lines). No enum changes —
`TriggeredBy`/`EvidenceFor` already exist.

### Phase B — LLM tool surface

**B.1 `crates/clawft-tools/src/subagent_tools.rs`** (NEW, ~240 lines — watch ceiling,
split per tool if needed) — *5 h*
Four `Tool` impls (`agent_spawn`, `task_status`, `task_result`, `agent_message`), each
holding `Arc<dyn SubagentSpawner>`. Param validation, JSON schemas per design D4, map
`SpawnError`→`ToolError`. Gated behind a new `subagent` feature (or reuse `delegate`).

**B.2 `crates/clawft-tools/src/lib.rs`** (MODIFY, ~+18 lines) — *1 h*
`register_all` grows an optional `Option<Arc<dyn SubagentSpawner>>` param; when
`Some`, register the four tools. Update the four existing callers
(`daemon.rs`, `cli/commands/mod.rs`, `cli/commands/gateway.rs`, `wasm/lib.rs`) to pass
`None` except the daemon. Reword `SpawnTool`'s description ("raw OS subprocess — NOT a
WeftOS agent; use agent_spawn for that").

**B.3 `crates/clawft-core/src/agent/effects.rs`** (MODIFY, ~+14 lines) — *0.5 h*
Add match arms: `agent_spawn`, `agent_message`, `task_status`, `task_result` per
design D6. Add unit asserts alongside the existing `read_tools_score_privacy` tests.

### Phase C — wiring + result enrichment

**C.1 `crates/clawft-weave/src/daemon.rs`** (MODIFY, ~+40 lines; file is large — keep
the addition tight) — *3 h*
At agent-service boot (~`daemon.rs:947`, where `register_all` is called): construct the
`SpawnRegistry` + `DaemonSubagentSpawner` (wire `Weak<AgentService>` after the service
Arc exists — set via a `OnceLock`/`set_spawner` back-reference, same late-wiring shape
as the `A2ARouter` gate `OnceLock`), pass it into `register_all`. Seed caps/timeout from
`KernelConfig.agent`.

**C.2 `crates/clawft-core/src/agent/loop_core.rs`** (MODIFY, ~+30 lines) — *3 h*
Enrich `ToolLoopResult` (`:152`) with `tool_calls: Vec<AgentChatToolCall>` and
`spawned_tasks: Vec<SpawnedTaskSummary>`; populate them in `run_tool_loop` as tools run
(detect `agent_spawn` results by shape, or have the tool stash a summary via a channel).
Thread up through `handle_turn`'s `OutboundMessage`.

**C.3 `crates/clawft-types/src/agent_chat.rs`** (MODIFY, ~+12 lines) — *0.5 h*
Add `SpawnedTaskSummary {task_id, child_conv_id, status}` and
`spawned_tasks: Vec<SpawnedTaskSummary>` to `AgentChatResult`
(`#[serde(default, skip_serializing_if = "Vec::is_empty")]`).

**C.4 `crates/clawft-service-agent/src/service.rs`** (MODIFY, ~+20 lines) — *1.5 h*
`result_from_outbound` (`:478`): stop hardcoding — read the enriched fields off the
threaded loop result (requires `OutboundMessage` to carry them, or dispatch to return
the richer type directly). Populate `tool_calls`, `finish_reason`, `iterations`,
`spawned_tasks`.

### Phase D — config + docs

**D.1 `crates/clawft-types/src/config/kernel.rs`** (MODIFY, ~+8 lines) — *0.5 h*
`agent.subagents { enabled:bool, max_per_conv:u32=5, max_depth:u32=3,
timeout_secs:u64, notify_on_complete:bool=false }`.

**D.2 ADR** — *1 h* — new `docs/weftos/adr/ADR-069-agent-initiated-work.md` capturing
the subagent-as-conversation decision, the forest edge semantics, and the deferred
`SpawnBackend` tiering. (Docs go in `docs/`, per CLAUDE.md — not `.planning/`.)

**Deferred (designed, not built): proactive injection.** New
`ImpulseType::TaskCompleted`, a daemon completion hook that injects a synthetic parent
turn when idle + flagged, behind the `notify_on_complete` config. Confirm no existing
`ImpulseType` variant already fits before adding one.

---

## 2. Test plan

### 2.1 Core unit (`clawft-core`)
- `effects.rs`: `agent_spawn` magnitude > 0.8 (forces a decision); reads ≈ 0.
- `spawn.rs`: `SpawnSpec` serde round-trip; `SpawnBackend::Native` ⇒ tool/spawner
  returns `BackendNotAvailable` (mirror `supervisor` backend test).

### 2.2 Service-agent unit (`clawft-service-agent`)
- `spawn_registry.rs`: insert/get/by-group/live-count; concurrency cap refuses the
  6th; depth cap refuses past 3.
- `subagent.rs` with a **mock `AgentService`**: `spawn(await:true)` returns
  `completed` + result; forest gets `TriggeredBy` + `EvidenceFor` edges; chain gets
  `agent.spawn`+`agent.complete`. `spawn(await:false)` returns `running`, then
  `result()` resolves after the mock finishes.

### 2.3 Tools unit (`clawft-tools`)
- Each tool: schema validity, param validation, `SpawnError`→`ToolError` mapping, with
  a mock `SubagentSpawner`.
- Gate integration: `execute_tool_with_guards("agent_spawn", …)` denied by a
  threshold-0 gate returns `{denied}`, not `{error}`.

### 2.4 Integration — the headline test (`clawft-weave/tests/`)
`agent_spawns_subagent_result_commits_into_parent`: boot a daemon with a **stub LLM**
whose parent turn emits a single `agent_spawn{goal:"answer 2+2", await:true}` tool call
then a final text turn, and whose child turn answers "4". Assert:
1. Tool result carried `{status:"completed", result:"4"}`.
2. Parent's `AgentChatResult.spawned_tasks` has one `completed` entry.
3. The kernel-global forest has: parent turn node, a child conv node, a
   `TriggeredBy` edge child→parent, an `EvidenceFor` edge child-result→parent.
4. The chain log has `agent.spawn` and `agent.complete` for the task_id.
5. Both the parent reply AND the child turns are `Committed` (M2 pipeline held).

This is the text analogue of M2's `live_native_talk_session` and the milestone's
acceptance test.

### 2.5 Async + swarm
- `agent_spawn{await:false}` → `task_status` shows `running` → child finishes →
  `task_result` returns the value.
- Fan-out: two spawns share a `group_id`; `task_status(group_id)` aggregates both.

### 2.6 Regression / green-keeping
- `scripts/build.sh test` (full workspace).
- `scripts/build.sh test` with `--features exochain` (forest/chain/gate paths).
- `scripts/build.sh clippy` and `scripts/build.sh gate` before any commit.
- Confirm the four `register_all` callers still compile with the new arg.

---

## 3. Phased implementation order (M4 swarm)

Hierarchical, specialized. Spawn ALL workers in ONE message, `run_in_background: true`,
SendMessage-coordinated (CLAUDE.md pattern). Suggested shape:

```
architect  → owns spawn.rs trait + SpawnBackend seam + ADR-069  (Phase A.1, D.2)
             SendMessage the frozen trait signatures to coder-core + coder-daemon
coder-daemon → SpawnRegistry + DaemonSubagentSpawner + forest edges + witnessing
               + daemon wiring                                    (Phase A.2/A.3/A.4/C.1)
coder-tools  → four Tool impls + register_all plumbing + effects  (Phase B)
coder-core   → ToolLoopResult/OutboundMessage/AgentChatResult enrichment
               + result_from_outbound + config                    (Phase C.2/C.3/C.4/D.1)
tester       → waits for coder-daemon + coder-tools; writes 2.2–2.5;
               owns the headline 2.4 integration test
reviewer     → gate-class correctness (D6), Arc-cycle/Weak review (D2),
               500-line ceilings, cascade-cancel (D5)
```

Dependency order: **A (substrate) → B ∥ C-core (surface + enrichment) → C-daemon (wire)
→ tests**. The trait in A.1 is the contract that unblocks B, C-core, and A.2 to proceed
in parallel — freeze it first.

Rough total: **~32 engineer-hours** of implementation + ~8 h test/review.

---

## 4. Open items to resolve during implementation (not blockers)

1. **Arc-cycle break mechanism** — `Weak<AgentService>` vs an mpsc spawn-queue the
   service drains. Prefer whichever the existing `A2ARouter` gate `OnceLock` late-wiring
   most resembles, for consistency.
2. **Child agent identity** — does each subagent get a distinct principal in the
   `AgentRegistry` (like the concierge at `daemon.rs:1164`), or share the parent's? A
   distinct principal makes the child's gate checks meaningful; decide before C.1.
3. **`OutboundMessage` vs direct richer return** — cheapest path to get `tool_calls`/
   `spawned_tasks` into `result_from_outbound` without widening the bus envelope. May
   favor returning the richer loop result type directly to `AgentService::dispatch`.
4. **`ImpulseType::TaskCompleted`** — only add if no existing variant fits; needed only
   for the deferred proactive-injection increment, so it can wait.
5. **Feature gating** — new `subagent` feature vs reuse `delegate`. Reusing `delegate`
   keeps the feature matrix smaller but couples local-spawn to the Claude-delegation
   dependency; a separate flag is cleaner.
