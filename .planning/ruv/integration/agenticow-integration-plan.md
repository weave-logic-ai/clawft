# agenticow → WeftOS Integration Plan

**Status**: Proposal (planning only — no code written)
**Author**: agenticow-integration-planner
**Date**: 2026-07-03
**Source deep-dive**: `.planning/ruv/packages/agenticow/overview.md`
**Upstream**: `https://github.com/ruvnet/agenticow` (MIT, v0.2.3), backed by
`@ruvector/rvf-node@^0.2.0`, which is a napi binding over the Rust crate
`rvf-runtime` — **which WeftOS already depends on** (`Cargo.toml:208`,
`rvf-runtime = "0.2"`).

---

## 0. TL;DR — the architecture decision

**Recommendation: skip the JS layer entirely. Reimplement agenticow's ~650-line
orchestration as a small Rust module (`clawft-cow-memory`) directly over
`rvf_runtime::RvfStore`.** Do **not** run agenticow as a Node sidecar and do
**not** bind it via napi.

Why this is not a close call:

- agenticow is a **thin JS wrapper**. `src/index.js` is 657 lines of pure
  orchestration (lineage-chain walk, tombstone `Set`s, an edit log, a
  `promote()` replay, a `save()/load()` JSON manifest). It has **one runtime
  dependency**: `@ruvector/rvf-node`.
- Every heavy primitive agenticow calls on `RvfDatabase` — `derive`, `branch`,
  `ingestBatch`, `query`, `delete`, `status`, `fileId`, `dimension` — is a napi
  passthrough to a method that **already exists as public Rust API on
  `rvf_runtime::store::RvfStore`** (verified in the extracted crate source at
  `~/.cargo/registry/src/index.crates.io-*/rvf-runtime-0.2.0/src/store.rs`):

  | agenticow JS call | `RvfStore` Rust method | line |
  |---|---|---|
  | `RvfDatabase.create` | `RvfStore::create(path, RvfOptions)` | store.rs:74 |
  | `RvfDatabase.open` / `openReadonly` | `RvfStore::open` / `open_readonly` | store.rs:127 / 178 |
  | `db.derive(childPath, opts)` | `RvfStore::derive(child, DerivationType, opts)` | store.rs:1371 |
  | `db.branch(childPath)` (native COW) | `RvfStore::branch(child_path)` | store.rs:1282 |
  | `db.ingestBatch(flat, ids)` | `RvfStore::ingest_batch(...)` | store.rs:224 |
  | `db.query(vec, k, opts)` | `RvfStore::query(vec, k, &QueryOptions)` | store.rs:314 |
  | (audit variant) | `RvfStore::query_audited(...)` → witness | store.rs:499 |
  | `db.delete(ids)` | `RvfStore::delete(&[u64])` | store.rs:525 |
  | `db.status()` | `RvfStore::status() -> StoreStatus` | store.rs:594 |
  | `db.fileId()` | `RvfStore::file_id() -> &[u8;16]` | store.rs:1263 |
  | freeze / is-child / stats | `freeze` / `is_cow_child` / `cow_stats` | store.rs:1327 / 1342 / 1347 |
  | (lineage provenance) | `parent_id` / `lineage_depth` / `parent_path` | store.rs:1268 / 1273 / 1362 |

  The COW machinery itself (`CowEngine`, `CowMap`, `CowMapEntry::{LocalOffset,
  ParentRef, Unallocated}`, copy-parent-slab-on-write) is native in
  `rvf-runtime-0.2.0/src/cow.rs` and re-exported from the crate root
  (`pub use cow::{CowEngine, CowStats, WitnessEvent}`).

- Our kernel is Rust and single-process. A Node sidecar would add a runtime, an
  IPC hop, a serialization boundary for every vector, a second copy of the RVF
  files' access path, and an operational failure mode (node crash / version
  skew) — to re-wrap methods we can call in-process for free. It also fights the
  "single-file, embedded, in-process, no-server" property that is agenticow's
  own headline.

**What we lose by skipping JS: nothing we can use on our platform.** The only
capability that lives *above* the published `rvf-runtime` crate is the native
**dual-graph HNSW merge across the COW boundary** (`nativeAnn`, RuVector
PR #617/#618), and it ships **only for `linux-x64-gnu`**. On macOS (our dev box,
`darwin`) agenticow itself degrades to the exact JS chain-walk — the same merge
we will write in Rust. See §6.

---

## 1. Goal

Give each actor / hermes-loop turn a **branchable, checkpointable view of the
ruvector brain** so we can:

1. **Checkpoint** an actor's memory before a speculative turn (O(1), ~162 B,
   ~0.5 ms regardless of brain size).
2. **Roll back** to that checkpoint if the turn fails, is rejected by the gate,
   or produces a low-coherence / hallucinated result — discarding the turn's
   memory writes cleanly (~0.57 ms p50).
3. **Promote** (git-style merge) the turn's learnings back into the shared brain
   only on success (~897 µs), with the merge **witnessed on the exochain**.

This is the memory-branching primitive WeftOS currently lacks. Today a turn that
ingests embeddings into the brain has no cheap undo; a bad turn permanently
pollutes the shared store.

---

## 2. Architecture (text diagram)

```
                         hermes loop  (crates/clawft-core/src/agent/loop_core.rs)
                         AgentLoop::run()  → handle_turn(msg)   [loop_core.rs:565]
                                   │
             ┌── before turn ──────┼───────── after turn ──────────┐
             │  checkpoint()       │        Ok → promote()          │
             │                     │        Err → rollback()        │
             ▼                     ▼                                ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │  clawft-cow-memory   (NEW crate — the agenticow orchestration in Rust)│
   │                                                                       │
   │   BranchableMemory {                                                  │
   │     working:   RvfStore          // current writable COW child        │
   │     ancestors: Vec<RvfStore>     // checkpoints … base (newest→oldest)│
   │     tombstones/edit-log/texts    // per-node, mirrors agenticow Node  │
   │   }                                                                   │
   │   fn checkpoint/rollback/branch/fork/promote/diff/lineage/query       │
   └─────────────┬─────────────────────────────────────────────────────────┘
                 │  RvfStore::{branch, derive, ingest_batch, query,
                 │             query_audited, delete, freeze, status}
                 ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │  rvf_runtime  (crates.io 0.2, already a workspace dep — Cargo.toml:208)│
   │  CowEngine · CowMap · RvfStore · witness · MembershipFilter           │
   └─────────────────────────────────────────────────────────────────────┘
                 │
                 ▼  brain .rvf files on disk (COW children are separate files,
                    parent-ref clusters; base stays read-only after fork)

   DualStateBridge coupling (Phase 3):
     ChainManager (clawft-kernel/src/chain.rs)  ── append witness/lineage ──▶ exochain
        checkpoint()  ⇄  BranchableMemory.checkpoint()
        turn revert   ⇄  BranchableMemory.rollback()  + append compensating chain event
        turn commit   ⇄  BranchableMemory.promote()   + record_lineage() witness
```

---

## 3. Exact API mapping (our operation → agenticow semantics → Rust call)

| WeftOS operation | agenticow verb | Rust implementation (in `clawft-cow-memory`) |
|---|---|---|
| Snapshot brain view before a turn | `checkpoint(label)` | `working.freeze()`; push `working` to `ancestors`; `working = frozen.derive(new_child, Clone, opts)` |
| Isolated per-actor memory view | `fork(label)` | `base.derive(child)` (or `base.branch(child)` on linux for native read-through); base stays read-only |
| Two-way isolated split | `branch(label)` | freeze working; `derive` a fresh child for *both* the continuing parent and the branch |
| Add turn learnings to the view | `ingest([{id,vector,text}])` | normalize (cosine→L2 on unit vectors); `working.ingest_batch(flat, ids)`; record in edit-log + `texts` map |
| Hide a memory from this view | `delete(ids)` | `working.delete(ids)` if local; always add to this node's `tombstones` set (masks ancestor) |
| Read brain (view-aware kNN) | `query(vec, k)` | **exact chain-walk**: query each node newest→oldest, child wins on id collision, mask `tombstones`, re-rank by exact distance, slice k (see §6 for the native-linux fast path) |
| Discard a bad turn | `rollback(ckptId?)` | close+`fs::remove` owned children newer than target; `working = target.derive(fresh)`; truncate `ancestors` |
| Commit a good turn into the brain | `promote(target?)` | replay `working.edit_vecs` → `base.ingest_batch`; replay `tombstones` → `base.delete`; carry `texts` |
| What changed this turn | `diff()` | set-diff `working.edit_ids` vs ancestors → `{added, overridden, deleted}` |
| Provenance / audit trail | `lineage()` | walk chain → `[{role, file_id, label, parent, created_at, mutations, tombstones}]` (pairs with exochain witness) |

**DESIGN CONSTRAINT — single vector space per lineage (added 2026-07-05, from
`.planning/research/e5-rvf-integration-study.md` §2.3).** COW chain-walk queries
compare child vectors against ancestor vectors, so a whole lineage MUST share one
embedding producer. Bake in from Phase 0: stamp an `embedder_id` (model name +
revision + prefix convention + dims) into the base store's META segment at
`create`; `branch`/`derive` inherit it and REFUSE to open/derive/promote across a
differing `embedder_id`. An embedder migration (e.g. hash→e5-small-v2, WEFT-640)
is always *fork a new base + re-embed the promoted set*, never an in-place
producer swap on a live lineage. RVF 0.2 does not enforce this itself — stored
vectors are not self-describing about their space; WeftOS must write and check
the stamp deliberately.
| Persist/reopen a chain | `save/load(manifest)` | JSON manifest of `[{path, label, tombstones, edit_ids, edit_vecs}]`; reopen base+ancestors read-only, working writable |
| Store health | `status()` | `working.status()` + chain depth + dim + metric |

Two behaviours to copy verbatim from `src/index.js` because they are correctness
load-bearing, not incidental:

- **Cosine via normalized-L2.** agenticow drives the engine metric as `l2` over
  L2-normalized vectors because the shipped rvf binding does not persist the
  cosine setting on reopen; on unit vectors L2 order == cosine order, so top-K is
  preserved (`index.js:48`, `l2normalize`, and the `_engineMetric` logic at
  `index.js:99-105`). Our Rust module must normalize identically or a reopened
  brain silently mis-ranks.
- **Global monotonic auto-id.** A process-wide counter (`index.js:36`,
  `GLOBAL_AUTO_ID`) prevents a base and its forks from colliding on
  auto-assigned ids — a per-instance counter would hand the same id to both
  sides and a later `promote()` would silently overwrite. If we auto-assign ids
  we need the same discipline (or require caller-supplied ids and sidestep it).

---

## 4. Where it plugs in on our side (file citations)

| Plug-in point | File | What changes |
|---|---|---|
| **Turn bracket** (checkpoint→turn→promote/rollback) | `crates/clawft-core/src/agent/loop_core.rs:565` (`handle_turn`), driven by `AgentLoop::run` at `:528` | Wrap `handle_turn`: `checkpoint()` before; on `Ok` `promote()`; on `Err` `rollback()`. The loop already distinguishes Ok/Err here (the error-reply arm is `:571-590`). |
| **Brain store handle** | `crates/clawft-core/src/memory_bootstrap.rs:126` (`RvfStore::create`), `:308/:417` (`open`) | Today opens the **stub** store (see §5, prerequisite). The COW layer wraps whatever `RvfStore` the brain opens. |
| **Actor / conversation memory** | `crates/clawft-core/src/agent/memory.rs` (`MemoryStore`), `crates/clawft-core/src/vector_store.rs` (`VectorStore`) | Candidate owners of a per-actor `BranchableMemory` handle; `VectorStore::search` is the read the chain-walk replaces. |
| **Graphify brain ingest** | `crates/clawft-graphify/src/ingest.rs:359` (`ingest`), `:452` (`save_query_result`) | Ingests into the brain — the write side a fork isolates. Emits chain event kind `graphify.ingest` (`ingest.rs:356`). |
| **Chain coupling (DualStateBridge)** | `crates/clawft-kernel/src/chain.rs` — `ChainManager::checkpoint` (:1099), `record_lineage` (:1684), `verify_lineage` (:1737), `save_to_rvf`/`load_from_rvf` (:1406/:1523) | Bracket a memory checkpoint with a chain checkpoint; witness the promote via `record_lineage`. See §7. |
| **Resource-tree snapshot analogue** | `crates/exo-resource-tree/src/boot.rs` — `to_checkpoint`/`from_checkpoint` (:81/:69), `mutation.rs` `MutationLog` | Existing checkpoint/restore pattern to mirror; the tree is the "ops" side, memory is the new "data" side of the bridge. |

---

## 5. Prerequisite — the brain runs on a *stub* RvfStore today

**Load-bearing caveat.** `clawft-core`'s vector-memory path does **not** use the
real `rvf-runtime::RvfStore`. It uses an in-memory stand-in:
`crates/clawft-core/src/embeddings/rvf_stub.rs` — its own doc comment says it
"mirrors the API shape of `rvf-runtime::RvfStore` (create, open, ingest, query,
delete…)". `memory_bootstrap.rs:15` imports `RvfStore` **from that stub module**,
not from the crate. The real `rvf-runtime` is currently used only for
witness/governance/scorecard types (`chain.rs`, `governance.rs`).

The stub has **no `derive` / `branch` / COW** (`grep` of `rvf_stub.rs` shows only
`create/open/ingest/query/delete/compact/len/get`). So there are two paths:

- **Path A (preferred): base the COW layer on the *real* `rvf_runtime::RvfStore`.**
  It already has `branch`/`derive`/`freeze`/`cow_stats`. This means the brain's
  vector-memory feature must migrate from the stub to the real crate (or the
  new `clawft-cow-memory` crate depends on real `rvf-runtime` directly and the
  brain is re-pointed at it). This is the right long-term move regardless of
  agenticow — the stub was always a placeholder.
- **Path B (fallback): add COW to the stub.** Re-implement `derive`/tombstones
  on the in-memory stub. Cheap to prototype, throwaway, and it can't give the
  on-disk 162-byte / O(1) branch guarantee (the stub is RAM-only). Use only for
  a Phase-1 spike if migrating the brain store is out of scope that sprint.

Recommendation: **Path A.** Treat "migrate brain vector-memory from `rvf_stub`
to real `rvf-runtime::RvfStore`" as an explicit prerequisite work item (it
unblocks on-disk COW, native linux read-through, and witness-emitting
`query_audited` — all things the stub can never provide).

---

## 6. macOS fallback implications

- **The `RvfStore::query()` in crates.io 0.2.0 does NOT read through the COW
  boundary.** Verified: it scans `self.vectors.ids()` only (store.rs:314-368) and
  honors its own `deletion_bitmap` + `filter`. `branch()` wires up `cow_engine`
  + a `MembershipFilter` marking parent vectors visible, but the shipped
  `query()` does not consult the parent. The **dual-graph HNSW merge across the
  boundary** (`query_via_index_cow`, recall@10 ≈ 1.0) is the linux-x64-gnu-only
  native binary path exposed through `@ruvector/rvf-node`, **not** in the
  published Rust `query()`.
- **Therefore our Rust module must implement the chain-walk merge itself** — hold
  a `Vec<RvfStore>` for the lineage, query each, merge child-wins + tombstone
  mask + exact re-rank. This is exactly what agenticow's JS default path does
  (`index.js:301-324`) and it is **correct on every platform** (it's the "always
  correct, just slower" fallback). On macOS this is our only path; that's fine.
- **On linux-x64 (likely production / daemon host)** we get an optional fast
  path: fork with `RvfStore::branch()` and, *if* a future `rvf-runtime`
  publishes the cluster-level read-through `query` (PR #617/#618 landing in a
  ≥0.3 release), route a single `query()` through it. Design the module so
  `query()` picks native-read-through-vs-chain-walk behind one method, mirroring
  agenticow's `_nativeCow` flag (`index.js:121`, `fork({nativeAnn:true})`).
  **Provenance caveat (added 2026-07-03 brain-informed review):** the "≥0.3
  release" is a *projection*, not a confirmed roadmap fact — no published
  `rvf-runtime` 0.3 exists today and neither the primary agenticow README nor the
  ruvnet-brain corpus names a version. What *is* verified: the native dual-graph
  HNSW merge across the COW boundary **already ships**, but only inside
  `@ruvector/rvf-node` (napi, `linux-x64-gnu` binary) — the agenticow README
  states it plainly: *"Native ANN search ACROSS the COW boundary — now shipped
  (was roadmap) … the native binary ships for linux-x64-gnu today"*
  `[verified: ruvnet/agenticow README]`. The ruvnet-brain agenticow primer is
  **stale/understated** here — it still lists "single ANN index spanning COW
  boundary" and "native cluster-level read-through" as *roadmap / not yet
  implemented* `[brain: kb/agenticow-primer.md]`. This review resolves that
  conflict in favor of the primary README (per charter): the capability is real
  on linux but lives above the *published Rust `rvf-runtime` crate*, exactly as
  §6's first two bullets already state — so the plan's direct-Rust decision and
  the chain-walk default stand unchanged; only the "0.3" label should be read as
  a hoped-for landing point, not a scheduled one.
- **Performance framing (from agenticow's own bench):** this is a *versioning*
  primitive, not a throughput primitive — it is ~6.3× behind hnswlib at 1 M
  vectors on raw search. The brain is not that large and the chain-walk overscan
  (`k*4` per node, `index.js:304`) is bounded by chain depth, which stays small
  (a turn = 1 checkpoint). Acceptable.

---

## 7. DualStateBridge & chain coupling — respect the append-only witness chain

agenticow's reference bridge (ADR-202) maps ops-branches to memory-branches 1:1
via `@metaharness/jujutsu`. Our analogue already half-exists, with **one
important difference**: our `ChainManager` (`chain.rs`) is an **append-only,
hash-linked, witnessed** log. There is **no truncating `revert`/`undo`** — the
only "rollback" in chain.rs is a *witness scorecard metric* (`rollback_count`,
:1846-1896), never event deletion. This is correct event-sourcing and must not
change.

So the WeftOS DualStateBridge is:

| Harness op | agenticow (memory) | WeftOS ops side |
|---|---|---|
| spawn actor / start turn | `fork()` + `checkpoint('spawn')` | `ChainManager::checkpoint()` marker (`chain.rs:1099`) + `exo-resource-tree` `to_checkpoint` |
| learn | `ingest()` turn embeddings | (write stays isolated in the fork) |
| **revert** | `rollback()` to spawn checkpoint | **append a compensating `TurnReverted` chain event** — do NOT truncate history; memory is discarded, the *fact of the revert* is witnessed |
| **merge / commit** | `promote()` into base | `ChainManager::record_lineage()` (:1684) witnesses the promoted delta; brain base now carries it |

Net: memory state is genuinely reversible (cheap COW discard); chain state is
append-only and records that a revert happened. "A chain revert also reverts
memory" becomes "a turn abort discards the memory branch **and** appends a
witnessed revert event." This is the honest, event-sourced form of the
DualStateBridge idea for our substrate.

---

## 8. Phased rollout, with effort estimates

Estimates assume one engineer familiar with the crates; `S`=≤1 day, `M`=2–4 days,
`L`=1–2 weeks.

### Phase 0 — Prerequisite: brain on real `RvfStore`  ·  **M**
- Migrate `clawft-core` vector-memory from `rvf_stub` to real
  `rvf_runtime::RvfStore` behind the existing `rvf` / `vector-memory` features
  (`memory_bootstrap.rs`, `embeddings/`). Keep the stub for `--no-default`/wasm
  builds where the native crate can't link.
- Exit: brain ingest/query round-trips through real `RvfStore` on native;
  `scripts/build.sh test` green.

### Phase 1 — Prototype: `clawft-cow-memory` crate (standalone)  ·  **M**
- New crate `crates/clawft-cow-memory` wrapping `RvfStore` with
  `BranchableMemory` (`checkpoint/rollback/branch/fork/promote/diff/lineage/query`),
  porting `src/index.js` semantics (chain-walk query, tombstones, edit-log,
  cosine-normalized-L2, global auto-id).
- Port agenticow's `bench/acceptance.js` as a Rust integration test: 1,000
  branches, recall@10 = 100% on the chain-walk path, rollback/promote
  round-trips. No kernel wiring yet.
- Exit: crate passes the ported acceptance test on macOS (chain-walk path).

### Phase 2 — Hermes-loop wiring (opt-in)  ·  **M**
- Wrap `AgentLoop::handle_turn` (`loop_core.rs:565`) with
  checkpoint→(Ok:promote / Err:rollback). Gate behind an `AgentsConfig` flag
  (default off) so existing behaviour is preserved (same pattern the loop already
  uses for `sandbox`/`autogen`/`gate` being `Option`al).
- Add a rollback trigger beyond `Err`: low coherence / gate denial (there's
  already an `EffectGate` at `loop_core.rs:235` and coherence primitives in
  `rvf-runtime` `agi_coherence`).
- Exit: a turn that errors leaves the brain byte-identical to pre-turn (assert
  via `diff()` / status); a good turn's embeddings are queryable after.

### Phase 3 — Chain coupling (DualStateBridge)  ·  **L**
- Couple memory checkpoint/rollback/promote to `ChainManager`
  (`checkpoint`/`record_lineage`) per §7; append `TurnReverted` on abort; witness
  the promote. Mirror `exo-resource-tree` checkpoint on the ops side.
- Exit: for a reverted turn, the chain shows a witnessed revert event and
  `verify_lineage()` passes; the brain shows no residue.

### Phase 4 — linux native read-through fast path (optional)  ·  **S–M, blocked on upstream**
- When `rvf-runtime ≥ 0.3` publishes cluster-level COW read-through `query`
  (PR #617/#618), add the native branch fast path behind the `nativeAnn`-style
  flag; keep chain-walk as the cross-platform default/fallback. Blocked until
  the crate ships it (today it's rvf-node/linux-only).

---

## 9. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Brain still on stub `RvfStore` (no COW) | **High** — blocks the whole plan | Phase 0 is an explicit prerequisite; don't start Phase 1 on the stub except as a throwaway RAM spike (Path B). |
| Cosine metric not persisted on reopen → silent mis-rank | High | Copy agenticow's normalized-L2 discipline verbatim (§3); add a reopen-and-rank regression test. |
| Chain-walk query cost grows with chain depth | Low | Depth stays tiny (1 checkpoint/turn); `promote()` collapses back to base; cap depth + auto-compact if needed. |
| Auto-id collision across fork/promote | Med | Port the global monotonic counter, or require caller-supplied ids. |
| Native read-through never lands in published crate | Low | Chain-walk is correct and sufficient on all platforms; native path is pure upside, isolated behind a flag. |
| COW children leak `.rvf` files on crash between checkpoint and rollback | Med | Track owned children (agenticow's `_owned` set, `index.js:109`); reap on open via the `save/load` manifest; add a startup sweep of orphaned `*.<slug>-<hex>.rvf`. |
| Divergence: memory rolled back but chain event already dispatched | Med | Phase 3 orders it correctly — memory rollback first, then append the compensating event; never truncate the chain. |
| WASM/browser builds can't link native `rvf-runtime` | Med | Keep the COW layer native-only (feature-gated); browser/wasm keeps the stub / non-branchable path. |

---

## 10. Test plan

- **Ported acceptance (Phase 1):** Rust translation of `bench/acceptance.js` —
  1,000 forks off one base, assert branch create is O(1) in base size and
  recall@10 = 100% on the chain-walk path; rollback restores exactly; promote
  merges exactly (`diff()` before/after).
- **Isolation invariant:** after `fork`, writes to the child are invisible to the
  base and vice-versa (agenticow's freeze-then-derive-both guarantee,
  `index.js:359-379`). Property test over random ingest/delete interleavings.
- **Cosine reopen regression:** create cosine store, ingest, close, reopen,
  query — top-K identical to pre-close. Guards the normalized-L2 gotcha.
- **Turn-bracket integration (Phase 2):** inject a turn that errors mid-way →
  assert brain `status()`/`diff()` byte-identical to pre-turn; inject a good turn
  → assert its embeddings are queryable and present in `base` after `promote`.
- **Chain coupling (Phase 3):** reverted turn ⇒ chain has a `TurnReverted` event
  and `verify_lineage()`/`verify_integrity()` pass; committed turn ⇒
  `record_lineage` witness present, `aggregate_scorecard` reflects it.
- **Crash-recovery:** kill between checkpoint and rollback; on restart the orphan
  child `.rvf` is reaped and the base is intact.
- All via `scripts/build.sh test` / `gate` (per CLAUDE.md — no raw cargo).

---

## 11. One-paragraph recommendation

Build a small native Rust crate, `clawft-cow-memory`, that reimplements
agenticow's copy-on-write branching directly on `rvf_runtime::RvfStore` — the
same Rust crate `@ruvector/rvf-node` binds and one WeftOS already depends on
(`Cargo.toml:208`). This skips Node.js, napi, and any sidecar; the entire
agenticow API surface (`branch/derive/ingest/query/delete/freeze/cow_stats`) is
already public Rust. The single prerequisite is migrating the brain's
vector-memory from the placeholder `rvf_stub` to the real `RvfStore` (Phase 0).
Wire it into `AgentLoop::handle_turn` (checkpoint→turn→promote-on-ok /
rollback-on-err), then couple it to the append-only exochain as a witnessed
DualStateBridge. On macOS we use the exact chain-walk merge (agenticow's own
cross-platform fallback); the linux-only native dual-graph HNSW is optional
upside behind a flag once it ships in a published `rvf-runtime`.
