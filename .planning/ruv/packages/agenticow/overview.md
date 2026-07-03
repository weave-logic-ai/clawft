# agenticow — "Git for Agent Memory" (copy-on-write vector branching)

- **Repo**: https://github.com/ruvnet/agenticow
- **Language**: TypeScript/JavaScript API over a Rust-native RVF backend
  (`@ruvector/rvf-node@0.2.0` prebuilt bindings). Fully embedded, in-process,
  single-file — no server.
- **License**: **MIT** © ruvnet (built on ruvector RVF). Vendorable, unlike AgentBBS.
- **Pushed**: 2026-07-03 · **Stars**: ~38
- **MCP tools (via claude-flow)**: `agenticow_checkpoint`, `agenticow_branch`,
  `agenticow_rollback`, `agenticow_promote`

## What it is

A copy-on-write (COW) vector-memory system that makes agent knowledge bases
**branchable like git**. A branch records only its own edits plus a parent
pointer, so branch creation is constant-time and constant-space: **~162 bytes and
~0.5 ms regardless of base size**. 1,000 isolated agent memories cost 943× less
disk than 1,000 full copies. "Turns memory from a static database into a
branchable runtime primitive for agents."

## The COW model

- **Query** walks the lineage chain (child → … → base), merges each store's
  results, lets the child win on any id collision, masks anything the branch
  tombstoned (deletes), and re-ranks by exact distance (cosine default, or L2).
- **Native ANN across the branch boundary** ships for `linux-x64-gnu` via a
  **dual-graph HNSW merge** (recall@10 ≈ 1.0). Other platforms degrade
  gracefully to exact read-through — always correct, just slower.
- Deliberately **not** optimized for raw search throughput (~6.3× behind hnswlib
  at 1M vectors). It competes on versioning, isolation, and rollback.

## API surface

```ts
open(path, { dimension, metric?, track? })   // metric: "cosine" | "l2"
mem.ingest([{ id, vector }])
mem.query(vector, k?, { efSearch?, overscan? })
mem.delete(ids)
mem.branch(label?)        // isolated COW child; auto-isolates parent
mem.fork(label?)          // lighter branch for fan-out
mem.checkpoint(label?)    // 162 B freeze / restore point
mem.rollback(checkpointId?)   // ~0.57 ms p50 — discard edits to checkpoint
mem.promote(target)       // ~897 µs — replay branch deltas into target (git-style merge)
mem.diff()                // { added, overridden, deleted }
mem.lineage()             //
mem.status()
mem.save(manifestPath); AgenticMemory.load(manifestPath); mem.close()
```

CLI: `agenticow init|ingest|branch|query|diff|demo|bench|acceptance`.

| Op | Cost |
|----|------|
| branch/fork | 162 B, ~0.5 ms |
| checkpoint | 162 B |
| rollback | ~0.57 ms p50 |
| promote | ~897 µs |
| branch create @1M vectors (496 MB) | 472 µs vs 67 ms full copy (**142×**) |

## Integration model — DualStateBridge (ADR-202)

agenticow pairs with `@metaharness/jujutsu` (which wraps `agentic-jujutsu`, a
Rust op-log with a QuantumDAG) to give a **1:1 mapping between code/ops branches
and memory branches**:

| Harness op | Code/ops (jujutsu) | Memory (agenticow) |
|-----------|-------------------|--------------------|
| spawn | new op | `fork()` + `checkpoint('spawn')` |
| learn | — | `ingest()` trajectory embeddings |
| revert | `jj undo` | `rollback()` to spawn checkpoint |
| merge | `jj squash` | `promote()` into base |

No explicit MCP registration inside agenticow; the bridge (and claude-flow's
`agenticow_*` tools) drive the public API.

## Maturity

Three tiers documented: **Practical** (branch/checkpoint/rollback/read-through —
proven, validated by `npm run bench` + `acceptance`: 1,000 branches, recall@10 =
100%), **Platform** (promotion pipelines, lineage/compliance, A/B at scale —
demonstrated), **Exotic** (parallel selves, simulated orgs — PoC mechanics only).

## WeftOS relevance — HIGH

This is the memory-branching primitive WeftOS lacks. Direct fits:

| agenticow | WeftOS opportunity |
|-----------|-------------------|
| `fork()`+`checkpoint('spawn')` per agent | Actor / hermes-loop trajectory checkpointing — snapshot an agent's memory before a speculative turn, `rollback()` if it goes bad |
| `promote()` git-style merge | Merge a successful agent branch's learnings back into the shared brain (exochain-witnessed) |
| `lineage()` + tombstone masking | Auditable memory provenance — pairs with exochain witness chain for "what did this actor know and when" |
| DualStateBridge (ops ↔ memory) | Couple `exo-resource-tree` / chain op-log to actor memory so a chain revert also reverts memory state |
| 162 B branches, 943× disk saving | Cheap per-actor isolated memory at fleet scale (many leaf/edge actors) |
| RVF backend (`@ruvector/rvf-node`) | Same RVF format WeftOS already uses for the ruvector brain — storage-compatible |

**Concrete integration points on our side**:
- `crates/clawft-graphify/` and the ruvector brain namespaces — agenticow could
  wrap the brain store to give branchable per-actor views.
- The hermes loop (see `.planning/development_notes/` + memory
  `weftos-current-state.md`) — checkpoint before a turn, rollback on failure.
- `crates/exo-resource-tree/` (exochain) — pair chain lineage with memory
  lineage per the DualStateBridge pattern.
- Because it's MIT and RVF-backed, it is a candidate for **direct vendoring** (JS
  side) or reimplementation of the COW-over-RVF idea in Rust for the native path.
