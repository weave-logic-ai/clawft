# Tools-as-nodes: deterministic spawn-edge rooting (WEFT-637)

**Status**: Foundation documented + existing code path confirmed  
**Seam**: M2 D3 / M4 A.4 cross-conversation forest edges  
**Related**: ADR-067 (conversation graph view), `DaemonSubagentSpawner`,
`session_forest::link_cross_conv`

## Problem

Spawn edges between parent conversation `P` and child `C` must be
**deterministically rooted** so replay and the ADR-067 graph view reproduce the
same topology. Free-floating or wall-clock-dependent edge anchors break
tools-as-nodes enrichment (tool invocations as graph nodes).

## Existing deterministic rooting (shipped)

| Edge | Source | Target | Determinism |
|------|--------|--------|-------------|
| Spawn | `spawn_goal_uid(child_conv)` | `resolve_parent_uid(parent_turn_uid, parent_conv)` | BLAKE3-style `UniversalNodeId` from structure tag + conv + salt |
| Goal | goal node | spawn goal | fixed salt `"goal"` |
| Completion | `spawn_result_uid(child_conv, task_id)` | parent turn | task_id salt |

Implementation:

- `crates/clawft-service-agent/src/subagent.rs` — `spawn_goal_uid`,
  `spawn_result_uid`, `conv_anchor_uid`, `resolve_parent_uid`
- `crates/clawft-service-agent/src/session_forest.rs` — `link_cross_conv`
  (`CrossRefType::TriggeredBy` / `EvidenceFor`)
- Ambient parent context: `clawft-core::agent::spawn_context::SpawnContext`
  (`parent_turn_uid`, `conv_id`, `agent_id`, `depth`)

### Parent uid resolution tiers

1. Explicit `SpawnSpec.parent_turn_uid` (64-hex) from the tool-call turn
2. Session tier lookup of the latest assistant tool-call turn in `P`
3. Deterministic per-conversation anchor `conv_anchor_uid(P)` so the tree
   still roots even when no turn uid is available

Replay with the same conv ids, task ids, and parent turn uids yields the same
`UniversalNodeId` bytes and the same `CrossRef` topology.

## Tools-as-nodes enrichment (design)

When a tool invocation is represented as a graph node (tools-as-nodes):

1. **Tool node id** is deterministic over `(conv_id, turn_uid, tool_name, call_index)`.
2. For `agent_spawn`, the tool node **is** (or is linked 1:1 to) the parent
   turn that carries `parent_turn_uid`.
3. The spawn `TriggeredBy` edge roots at that tool/turn node — never at a
   random or wall-clock id.
4. Child goal / result nodes remain salts under `child_conv` as above.

No change to the causal BFS lineage walk: cross-conv spawn links stay on
`CrossRefStore`, not `CausalGraph` edges (avoids parent turns leaking into
child recall).

## Tests covering the D3 seam

- `await_spawn_draws_forest_edges`
- `spawn_edge_roots_at_parent_turn_via_session_tier`
- `parent_target_uid` unit path (hint vs anchor)

## Follow-ups (not blocking this ticket)

- Explicit tool-node structure tag if tools-as-nodes becomes a first-class
  `StructureTag` (today tool calls ride the assistant turn node)
- Panel/graph UI rendering of tool nodes as first-class vertices
- WASM/browser parity (spawn is daemon-native)

## Acceptance (WEFT-637)

- [x] Tool invocations root spawn edges deterministically (via parent turn uid +
      fixed salts; documented)
- [x] Replay reproduces the same edge topology (deterministic `UniversalNodeId`)
- [x] Tests cover the D3 seam (existing spawner tests)
