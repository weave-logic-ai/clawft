# M4 — Agent-Initiated Work (Spawn Tasks, Subagents, Swarms)

Status: DESIGN. No code in this milestone. Sibling: `m4-agent-initiated-work-plan.md`.
Branch base: `feat/hermes-loop-base` @ `0416b77e`. Builds on M2 (daemon-hosted
multiplexed `TalkModeLoop`, turns commit Frontier→Committed on the kernel-global forest).

---

## 1. Goal

The conversational agent must be able to **kick off work from inside a turn** —
spawn a subagent, fan out a swarm, hand a task off — and have the **result flow
back into the conversation as first-class ECC state**. Tools already work
(`run_tool_loop`, 20 iterations, gate + sandbox guards). Spawning does not: the
LLM's only "spawn-ish" tools are `spawn` (a raw OS subprocess, cap 5) and
`delegate_task` (a nested Anthropic-API call that needs an API key). Neither
creates a WeftOS agent, and neither leaves a trace in the forest.

M4 closes that gap by making **spawn a first-class conversational move** that
reuses the entire M2 commit pipeline, so spawned work is visible in the ADR-067
graph view for free.

---

## 2. Load-bearing facts established during recon (verified this session)

1. **Two disjoint tool worlds.**
   - *Kernel world* — `BuiltinTool` trait in `clawft-kernel/src/wasm_runner/`.
     `AgentSpawnTool`/`AgentStopTool`/`AgentListTool` (`tools_agent.rs`) operate
     directly on `ProcessTable`. They are wired into `kernel_agent_loop`
     (`agent_loop.rs:31`), which is a **JSON-command processor** (`{"cmd":"ping"|"exec"
     |"cron.add"…}`), *not* an LLM loop. The daemon's `agent.spawn` RPC
     (`daemon.rs:4717`) builds a kernel `ToolRegistry` and runs this loop via
     `supervisor.spawn_and_run`.
   - *LLM world* — `Tool` trait in `clawft-core/src/tools/registry.rs:446`.
     Registered by `clawft_tools::register_all` (`clawft-tools/src/lib.rs`).
     Dispatched by `AgentLoop::run_tool_loop` through
     `execute_tool_with_guards` (`loop_core.rs:1209`: gate → sandbox → registry →
     64 KB truncation). This is the world the chat agent lives in. Its tools:
     `read_file`/`write_file`/`edit_file`/`list_directory`/`exec_shell`/`spawn`/
     `delegate_task`/`web_fetch`/`web_search`/`memory_*`/`message`.
   - **These worlds never meet.** The LLM cannot call `AgentSpawnTool`.
2. **Existing "spawn-ish" LLM tools are dead ends for real agents.**
   - `SpawnTool` (`clawft-tools/src/spawn_tool.rs`, `native-exec`): raw
     `Command` subprocess, `MAX_CONCURRENT_SPAWNS = 5`, `CommandPolicy`-gated,
     60 s timeout. Not a WeftOS agent, no forest presence.
   - `DelegateTaskTool` (`clawft-tools/src/delegate_tool.rs`, `delegate` feature):
     a **nested Anthropic-API conversation** via `ClaudeDelegator`. Needs an API
     key; `DelegationTarget::Flow` fell back to Claude when `FlowDelegator` was
     removed. WEFT-180 caps recursion depth. This is delegation *off-box*, not a
     local subagent.
3. **The M2 pipeline is exactly the substrate a subagent needs.** Every daemon
   text turn traverses `handle_turn → append_turn → KernelTurnAnchor::anchor_turn
   (mints global `chain_seq`) → SessionTier::index_turn` (`session_tier.rs:222`),
   which dual-writes a `CausalGraph` node (Frontier) + `Follows`/`Speaker` edges,
   `register_turn`s it with `TalkModeLoop`, and emits `EndOfUtterance`. The
   multiplexed daemon `TalkModeLoop` commits it Frontier→Committed on its tick.
   `conv_id` is the session key; per-conv dispatch is serialized by
   `AgentService`'s `DashMap<ConvId, Mutex>`. **A subagent is just another
   `conv_id` on this same engine.**
4. **`run_tool_loop` fires inside the committed-turn bracket** (M2 D3): the tick
   timeline is `user-turn EOU/commit → run_tool_loop (LLM + tools) → assistant-turn
   EOU/commit`. M2 explicitly reserved this seam: "M4's tools will emit per-tool
   causal nodes/impulses between the two commits, linked `Enables`/`EvidenceFor`
   to the reply node." A spawn call happens here.
5. **Forest edge vocabulary already covers spawn causality.**
   `CausalEdgeType` (`clawft-kernel/src/causal.rs:38`): `Causes`, `Inhibits`,
   `Correlates`, `Enables`, `Follows`, `Contradicts`, `TriggeredBy`, `EvidenceFor`.
   Cross-conversation links are `CrossRef`s in the kernel-global `CrossRefStore`.
   `CausalGraph::{add_node, set_node_state, link}` and
   `session_forest::{turn_universal_id, dual_write_turn}` are the write API.
6. **Governance is a solved seam.** `execute_tool_with_guards` already calls
   `gate.check(agent_id, "tool.<name>", &effect)` where `effect =
   effect_for_tool(name, args)` (`effects.rs:95`, a static table;
   unknown tool ⇒ zero vector ⇒ Permit). `EffectVector` is 5-D
   (risk/fairness/privacy/novelty/security), magnitude vs a threshold (default 0.8).
   The child subagent runs through the SAME gate under its OWN `agent_id`, so
   spawning is dual-gated by construction (D7 defense-in-depth from K2).
   **As-built note (ruling 2026-07-05):** the "own agent_id" half is DEFERRED —
   children currently gate under the shared concierge principal (loop_core's
   agent-id precedence: routed > daemon_agent_id > channel:sender; the daemon
   always sets daemon_agent_id). The gate's CONTROL function fully applies to
   children (magnitude+rules are id-independent), and attribution survives via
   chain-witnessed spawn/complete events + forest lineage. Distinct per-child
   principals ride the already-documented per-user-agent-ids future phase
   (daemon.rs:1160) — Plane follow-up, not an M4 gap.
7. **Budget + depth + cancel primitives exist.** `cost_budget.rs`
   (`max_iterations_per_conv`, tokens, usd). WEFT-180 recursion depth cap
   (`loop_core.rs:82`, default configurable). `agent.chat.cancel` per-conv
   (`daemon.rs:4580`). M2's idle reaper walks `active_conversations()` and ends
   idle convs.
8. **`AgentChatResult` already has the fields; they are hardcoded empty.**
   `agent_chat.rs:128` defines `tool_calls: Vec<AgentChatToolCall>`,
   `finish_reason`, `iterations`. `result_from_outbound` (**`clawft-service-agent/
   src/service.rs:478`**, not clawft-core) fills them with `Vec::new()`/`"stop"`/`0`
   because `OutboundMessage` carries none of it. The code comments already promise
   "richer plumbing lands when the loop's result type is enriched in C2/D3."

---

## 3. Architecture (spawn → child work → result back into the parent)

```
 PARENT conversation  (conv_id = P)                       CHILD conversation (conv_id = C)
 ─────────────────────────────────                        ────────────────────────────────
  handle_turn(P)
    append user turn  ──► commit  (node T_user@P)
    run_tool_loop(P):
      LLM emits tool_use: agent_spawn{goal, persona, await}
      execute_tool_with_guards("agent_spawn", …)
        gate.check(parent_id,"tool.agent_spawn",EFFECT_SPAWN)  ← NEW effect class
        ├─ Deny  → {denied, reason}   (parent LLM sees a policy decision)
        └─ Permit→ SubagentSpawner.spawn(SpawnSpec)  ─────────────┐
                     • mint child conv_id C = "sub:<P>:<ulid>"     │
                     • depth = parent.depth+1  (WEFT-180 cap)      │
                     • concurrency guard (per-P cap)               │
                     • SpawnRegistry.insert(task_id→{P,C,status})  │
                     • witness chain: agent.spawn {task_id,P,C}    │
                     • forest: CrossRef  C.goal --TriggeredBy--> T_user@P
                     ▼                                             │
             await? ──yes──► dispatch(C, goal) INLINE  ───────────┤ AgentService::dispatch(C)
                     │         (bounded by tool call)              │   handle_turn(C):
                     │                                             │     goal turn ─► commit
                     │                                             │     run_tool_loop(C)  ← child
                     │                                             │       (own gate, own budget)
                     │         result = child final assistant text │     reply turn  ─► commit (R_c@C)
                     │◄────────────────────────────────────────────┘
                     │   forest: R_c@C --EvidenceFor--> T_user@P
                     │   witness chain: agent.complete {task_id, finish_reason}
                     │   tool result = {task_id, status:"completed", result, child_conv_id:C}
                     └──no──► return {task_id, status:"running", child_conv_id:C}
                              (child runs as detached tokio task;
                               parent polls task_result / task_status later)
      … LLM weaves result into its answer …
    append assistant turn ──► commit  (node T_reply@P,  Follows T_user@P)
```

Both `P` and `C` are ordinary conversations on the one multiplexed daemon loop.
Every child turn commits to the **same kernel-global forest** as the parent — the
spawn edge (`TriggeredBy`) and the result edge (`EvidenceFor`) are cross-conversation
`CrossRef`s in the shared `CrossRefStore`. **The ADR-067 graph view therefore renders
the spawn tree with zero extra work**: a parent turn with a `TriggeredBy` fan-out to
child goal nodes, each child a `Follows` chain, each result an `EvidenceFor` back-edge.

---

## 4. Decisions

### D1 — A subagent is a daemon-hosted child conversation, not a kernel process (for M4)

**Chosen:** a subagent is a new `conv_id` driven by the SAME
`AgentService`/`AgentLoop` engine, given its own agent identity/persona, tracked by
a parent→child `SpawnRegistry`. The kernel supervisor path
(`supervisor.spawn_and_run` → `kernel_agent_loop`) is the **escalation** path, not M4.

**Rationale:**
- The kernel `kernel_agent_loop` is a JSON-command processor, not a thinker. To make
  a supervised process actually reason you would have to run an `AgentLoop` *inside*
  the spawn closure instead of `kernel_agent_loop` — that is real work and is exactly
  what `SpawnBackend::Native` isolation should mean later.
- A child conversation inherits the entire M2 investment for free: durable JSONL
  sink, chain anchoring, global `chain_seq`, `SessionTier`, gate, forest commit,
  idle reaping. Building spawn on the supervisor path would rebuild all of that.
- The shared LLM (local Hermes on :8090) already serves every conversation; a child
  conversation is just more turns through it. No API key (fixes the `delegate_task`
  pain), no second model server.

**Cost / entailment:** no OS/Wasm isolation — a runaway child burns the shared LLM
and shares the daemon's blast radius. Bounded by per-conv budget + concurrency cap +
depth cap (D5). If a caller needs true isolation, that is `SpawnBackend::Native`
(deferred; seam designed in D2).

**Rejected:** (b) supervisor-backed process per spawn — heavier, needs an
LLM-in-supervisor path that does not exist, throws away the M2 pipeline. (c) both,
tiered, in M4 — too much surface; the tier boundary is designed but only the
conversation tier is wired.

### D2 — The spawn seam: an injected `SubagentSpawner` trait (no layering violation)

**Chosen:** define a narrow trait in `clawft-core`:

```rust
// clawft-core/src/agent/spawn.rs  (NEW, ~120 lines)
#[async_trait]
pub trait SubagentSpawner: Send + Sync {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnHandle, SpawnError>;
    async fn status(&self, task_id: &str) -> Option<TaskStatus>;
    async fn result(&self, task_id: &str, wait: Option<Duration>) -> TaskOutcome;
    async fn message(&self, task_id: &str, text: &str) -> Result<(), SpawnError>;
    async fn cancel(&self, task_id: &str) -> Result<(), SpawnError>;
}
```

The `agent_spawn`/`task_status`/`task_result`/`agent_message` **tools live in
`clawft-tools`** (LLM world) and hold an `Arc<dyn SubagentSpawner>`. The concrete
impl (`DaemonSubagentSpawner`) lives in `clawft-weave`/`clawft-service-agent` where it
can reach `AgentService::dispatch`. It is injected at registration exactly like the
existing `ConversationSink`, `chat_gate`, `identity_provider` handles threaded through
`bootstrap::build_daemon_agent_loop` (`daemon.rs:1315`).

**Why a trait, not a direct call:** `clawft-tools` sits *below* `clawft-weave`; a tool
cannot name `AgentService`. Inversion also breaks the Arc cycle (`AgentService` owns the
tool registry which owns a tool that dispatches back into `AgentService`) — the spawner
holds a `Weak<AgentService>` (or dispatches via an mpsc spawn-queue the service drains),
never a strong back-reference.

**`SpawnBackend` seam:** `SpawnSpec` carries an optional
`backend: SpawnBackend` (mirror the kernel enum: `Conversation` default | `Native` |
`Wasm` | …). M4 implements `Conversation`; the rest return
`SpawnError::BackendNotAvailable` exactly like `supervisor.rs:657`. This bakes the
tiering in now (K2 D2/D3) without implementing it.

### D3 — Result return: forest-native, poll-primary, inject-optional

This is the key design move. A completed task's result must enter the PARENT
conversation as first-class ECC state.

**Chosen — two channels, both always drawn in the forest:**

1. **Forest edges (always, both sync and async).** On spawn:
   `C.goal --TriggeredBy--> T_call@P` (the parent turn that made the call). On
   completion: `R_c@C --EvidenceFor--> T_call@P` (the parent turn the result is
   evidence for). Cross-conv `CrossRef`s in the shared store. This is what makes the
   spawn tree visible in ADR-067 — it is not optional and does not depend on how the
   parent consumes the result.
   **Rooting rule (final ruling 2026-07-05, as-built):** `T_call@P` is the parent
   conversation's **latest non-empty committed turn at spawn time**, resolved by the
   spawner via `SessionTier::latest_turn_uid` — never the `conv_anchor` phantom, and
   always `Follows`-connected to the user turn. In practice this is the **assistant
   tool-call turn when the model narrates the call** (that turn is appended+anchored
   immediately before tool dispatch), and the **user turn when the model emits a
   text-less pure `tool_use`** — because `SessionTier::index_turn` skips empty-text
   turns (`session_tier.rs` empty-text guard), so a text-less tool-call turn creates
   no causal node and cannot be a root. Verified dynamically by the M4 headline
   integration test (stub emits text-less tool_use → edge roots at the user turn).
   Deterministic assistant-turn rooting arrives with the deferred **tools-as-nodes
   enrichment** (per-tool causal nodes inside the committed-turn bracket, M2 D3 seam)
   — a Plane follow-up, deliberately not a quick `last_uid` patch, which would root
   edges at a uid with no node behind it (the phantom problem one layer down).

2. **Value return to the LLM — tiered by `await`:**
   - **`await: true` (synchronous, the MVP path).** The tool dispatches the child
     inline and returns `{status:"completed", result}` in the tool result. The next
     `run_tool_loop` iteration hands it to the LLM, which weaves it into the reply.
     Trivially correct, bounded by the tool call, no reentrancy. This is the
     end-to-end test the milestone is graded on.
   - **`await: false` (asynchronous).** The tool returns `{status:"running", task_id}`
     immediately; the child runs as a detached task. The parent retrieves the result
     later with **`task_result(task_id, wait_ms?)`** (blocks up to `wait_ms`, else
     returns `running`) or **`task_status`**. Poll-based, so no runaway auto-dispatch.
   - **Proactive injection (opt-in, guarded, later increment).** If a spawn is flagged
     `notify_on_complete` AND the parent conversation is idle, completion emits a
     `TaskCompleted` impulse; a daemon hook injects a synthetic turn
     ("Subagent <task_id> finished: <summary>") into P and dispatches it, so the parent
     agent reacts unprompted. **Guarded** by: parent must be idle (no mid-flight turn —
     the per-conv `Mutex` enforces serialization), a per-conv auto-dispatch count cap,
     and the depth cap. Shipped behind a config flag; **not** in the M4 core because
     reentrant self-dispatch is the riskiest behavior in the whole design.

**Rationale:** the forest edges give the ECC-native visibility the milestone wants
regardless of consumption; poll-primary keeps control flow deterministic and free of
runaway loops; `await:true` is the simplest thing that fully satisfies "spawn a
subagent that answers and the result comes back into the conversation."

**Rejected:** push-only (always inject a parent turn on completion) — reentrancy into a
possibly mid-turn conversation, runaway spawn↔inject loops, hard to reason about.
Blocking the whole parent turn on every spawn (no async at all) — throws away the
fan-out value a swarm needs.

### D4 — Tool surface (minimal, four tools)

All live in `clawft-tools`, registered by `register_all` only when a
`SubagentSpawner` is injected (daemon path); absent for the in-process CLI fallback.

| Tool | Params | Returns |
|------|--------|---------|
| `agent_spawn` | `goal:string` (req); `persona?:string`; `skills?:string[]`; `await?:bool=false`; `budget?:{tokens?,usd?,iterations?}`; `group_id?:string`; `notify_on_complete?:bool=false` | `{task_id, child_conv_id, status:"running"|"completed", result?, finish_reason?}` |
| `task_status` | `task_id?:string` OR `group_id?:string` (one required) | `{tasks:[{task_id, status, child_conv_id, iterations, group_id?}]}` |
| `task_result` | `task_id:string`; `wait_ms?:number` | `{status, result?, finish_reason?, error?}` |
| `agent_message` | `task_id:string`; `message:string` | `{delivered:bool}` — inject a turn into the child conv (A2A) |

`status` ∈ `running | completed | failed | exhausted | cancelled | denied`.
Caps: `MAX_SUBAGENTS_PER_CONV` (default 5, mirrors `spawn_tool`), depth cap (WEFT-180,
default 3). `goal` length bound; `skills`/`persona` validated against the identity
loader's known personas.

**`delegate_task` relationship:** *retarget, don't deprecate.* `delegate_task` stays as
the **Claude-API escalation** (off-box, needs a key). `agent_spawn` is the new
**local-subagent primitive** (on-box, local LLM, no key) and covers the common case that
previously forced `delegate_task`. Optionally, a later increment folds them under one
tool with `target: local|claude`. `spawn` (OS subprocess) stays as a raw-process tool
but is documented as "not an agent"; its description is reworded so the LLM stops
reaching for it when it means "spawn an agent."

### D5 — Lifecycle & limits (reuse, don't reinvent)

- **Budget:** each child conv gets a `CostBudget` seeded from `spec.budget` (default =
  the daemon's `max_iterations_per_conv`, tokens, usd). Child exhaustion ⇒
  `finish_reason:"max_iterations"`/`"budget"`, status `exhausted`, partial result
  returned.
- **Depth:** `SpawnSpec.depth = parent.depth + 1`; refuse past the WEFT-180 cap
  (default 3) with a structured error — reuses the exact guard delegation uses.
- **Concurrency:** `MAX_SUBAGENTS_PER_CONV` live children per parent conv (atomic
  counter à la `spawn_tool::ACTIVE_SPAWNS`); refuse with `"max concurrent subagents"`.
- **Timeout:** per-child dispatch timeout (default from config); on timeout the child
  conv is cancelled and status→`failed`.
- **Cancellation:** `task_result`/explicit cancel → `agent.chat.cancel` on the child
  conv (the machinery already exists per-conv).
- **Orphan reaping:** children are ordinary conversations, so M2's idle reaper ends
  idle children automatically. **Plus** a parent-end cascade: ending P (idle-reap or
  explicit `agent.chat.end`) cancels P's still-running children so none are orphaned.

### D6 — Governance & witnessing

- **New effect class for spawn.** Add to `effect_for_tool` (`effects.rs`):
  `agent_spawn ⇒ EffectVector { risk:0.5, novelty:0.6, security:0.5 }` (magnitude
  ≈ 0.93 > 0.8 default threshold) so **every spawn forces a governance decision**
  (Permit/Defer/Deny), not a silent Permit. `agent_message` ⇒ moderate
  (`security:0.3`). `task_status`/`task_result` ⇒ near-zero (reads).
- **Dual gate (K2 D7).** Parent's spawn is gated at the tool boundary (may this agent
  spawn?). The child then runs through the SAME `execute_tool_with_guards` under its
  OWN `agent_id`, so the child's *actions* are independently gated (what may the child
  do?). Defense-in-depth falls out of reusing the pipeline.
- **Witness every transition** on the chain (ADR-033): `agent.spawn {task_id, parent,
  child, goal_hash, depth}`, `agent.complete {task_id, finish_reason, iterations}`,
  `agent.fail {task_id, reason}`, `agent.cancel {task_id}`. These append via the same
  `ChainManager` the turn anchor uses, so spawn events interleave with `chain_seq` in
  the audit log.

### D7 — Swarms = N spawns + a group id (no first-class object in M4)

A swarm is N `agent_spawn` calls sharing a `group_id` (caller-supplied or minted by the
first spawn), tracked by the `SpawnRegistry`. `task_status(group_id)` aggregates;
`agent_message` gives sibling A2A. The forest already shows the group as a fan-out from
the parent turn. A first-class `Swarm` object (topology, consensus, shared memory
namespace) is deferred — it is a coordinator over these primitives, and the K2 clustering
work (K5) is where that belongs.

### D8 — Wire/result visibility (finish the C2/D3 enrichment)

`AgentChatResult` already declares `tool_calls`/`finish_reason`/`iterations` but
`result_from_outbound` (`service.rs:478`) hardcodes them. M4:
1. Enrich the loop result (`ToolLoopResult` in `loop_core.rs:152`) to carry the real
   `tool_calls` summary **and a new `spawned_tasks: Vec<SpawnedTaskSummary>`**
   (`{task_id, child_conv_id, status}`).
2. Thread it through `OutboundMessage`/dispatch so `result_from_outbound` populates real
   values instead of defaults.
3. Add `spawned_tasks: Vec<SpawnedTaskSummary>` to `AgentChatResult`
   (`agent_chat.rs`), `#[serde(default, skip_serializing_if = "Vec::is_empty")]` so it
   is wire-compatible. The CLI/GUI can then render "agent kicked off 3 subagents (2
   done, 1 running)".

---

## 5. Failure modes

| Failure | Handling |
|---------|----------|
| Local LLM down at spawn | dispatch fails fast; witness `agent.fail`; tool returns `{status:"failed", error}` |
| Child exceeds budget | child `finish_reason:"max_iterations"`; status `exhausted`; partial result returned, not an error |
| Child times out | cancel child conv; status `failed`; `agent.fail{reason:"timeout"}` |
| Recursion bomb | depth cap (WEFT-180) refuses spawn before dispatch; `{denied, reason:"depth"}` |
| Concurrency exhaustion | atomic guard refuses; `{error:"max concurrent subagents"}` |
| Parent ends mid-child | parent-end cascade cancels children (D5) — no orphans |
| Gate denies spawn | `{denied, reason}` (distinct from a runtime error, per `execute_tool_with_guards`) |
| Child → parent deadlock (await:true) | `agent_message` to the parent only **queues** (never synchronously re-enters P while P awaits C) |
| Async result never polled | idle reaper ends the child; result stays in `SpawnRegistry` until the parent conv is reaped |

---

## 6. Non-goals (explicit)

- OS/Wasm/container isolation for subagents (`SpawnBackend::Native+`) — seam only.
- First-class swarm objects, topologies, consensus — K5.
- Proactive completion injection in M4 core — designed (D3.2), flag-gated, deferred.
- Cross-node spawn (`SpawnBackend::Remote`) — K6 (mesh).
- Folding `delegate_task` and `agent_spawn` into one `target`-parameterized tool —
  optional later increment.
