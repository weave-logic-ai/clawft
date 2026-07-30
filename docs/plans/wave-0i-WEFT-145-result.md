# WEFT-145 result — incremental Merkle hash updates

**Branch:** `wave0i/weft-145-merkle-incr`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4c3-9a5a-7d11-afcd-7e95de5e5288`  
**Base:** `release/0.8-staging`  
**Ticket:** ws02: mesh — incremental Merkle hash updates (replace full recompute_all)

## Problem

`ResourceTree::recompute_all()` was a full-tree rehash on every mutation
(insert / remove / update_meta / agent register / remote apply). No dirty-flag
propagation. O(tree) work per mutation made large trees expensive and blocked
downstream Merkle-diff work (row #17 / WEFT pairing).

## What shipped

### API (`exo-resource-tree`)

| API | Role |
|-----|------|
| `ResourceNode.dirty` | Runtime dirty flag (`#[serde(skip)]` — not checkpointed) |
| `ResourceTree::mark_dirty(id)` | Mark `id` + ancestors dirty (O(depth)); restores dirty⇒ancestors-dirty if a new node started dirty alone |
| `ResourceTree::recompute_dirty()` | Bottom-up rehash of dirty set only; returns count |
| `ResourceTree::recompute_path(id)` | Structural path recompute: aggregate origin + ancestors, O(depth) |
| `ResourceTree::recompute_path_preserve_scoring(id)` | Scoring mutations: keep origin scoring, re-aggregate ancestors only |
| `ResourceTree::has_dirty` / `dirty_count` | Introspection |
| `recompute_all` | Retained for bootstrap / checkpoint consistency; clears dirty flags |

`update_scoring` / `blend_scoring` now bubble to root via
`recompute_path_preserve_scoring` (parents re-aggregate; origin scoring kept).

### Call sites (`clawft-kernel` TreeManager)

All mutation paths that previously called `recompute_all()` now use
`recompute_path` (or path from former parent after remove):

- `insert`, `remove`, `update_meta`
- `register_agent`, `unregister_agent`
- `apply_remote_mutation` (Create / Remove / UpdateMeta)

Bootstrap still uses `recompute_all` via `bootstrap_fresh` (correct for cold start).

## Acceptance

| Criterion | Status |
|-----------|--------|
| Dirty-flag propagation up the tree | Yes — `mark_dirty` |
| Recompute scoped to affected subtree | Yes — `recompute_path` / `recompute_dirty` |
| Benchmark: large tree mutation is O(depth) not O(size) | Yes — `recompute_path_is_o_depth_not_o_size` (50×20 ≈ 1000 nodes; one leaf touch recomputes 21 = depth+1) |
| Tests verify hash equivalence with the old path | Yes — insert / remove / meta / scoring path tests compare root + per-node hashes against `recompute_all` |

## Files

| File | Change |
|------|--------|
| `crates/exo-resource-tree/src/model.rs` | `dirty` field on `ResourceNode` |
| `crates/exo-resource-tree/src/tree.rs` | Incremental Merkle APIs + WEFT-145 tests |
| `crates/exo-resource-tree/src/lib.rs` | Crate docs mention path recompute |
| `crates/clawft-kernel/src/tree_manager.rs` | Mutations use `recompute_path` |
| `docs/plans/wave-0i-WEFT-145-result.md` | This report |

## Verification

```text
scripts/build.sh test exo-resource-tree
# 77 passed

cargo nextest run -p clawft-kernel --lib tree_manager
# 32 passed (tree_manager + related boot hook)
```

## Notes / follow-ups

- Full `recompute_all` remains the correctness oracle and bootstrap path.
- Scoring on non-leaves is preserved by the scoring path APIs; a later
  `recompute_all` still rewrites non-leaf scoring from children (pre-existing
  aggregate semantics).
- Pairs with Merkle diff API (audit row #17) which can now assume O(changed)
  hash maintenance.
- No push (wave branch for lead merge).
