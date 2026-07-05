# The Panopticon — Atom Primary Index & Locator Resolver

**Status**: Research / design (planning only — no code). Leave uncommitted.
**Author**: system-architect (design agent)
**Date**: 2026-07-05
**Branch**: `feat/hermes-loop-base` · HEAD `dcb70de1`
**Trigger (user's thesis, verbatim)**: *"I think there should be a primary index,
a sort of panopticon — this will allow the mapping between the indexes, no matter
what the types/storage etc. I feel like we have this with the existing substrate."*
**Framing metaphors (user)**: *the Time Lord city in Doctor Who — outside of time,
able to see all of time and space*; and *Bentham's original panopticon — one guard
sees every cell, no inmate can see the guard.* Three design properties fall out and
are made load-bearing invariants below (§2.5): the resolver lives **outside all
projections** (it is not itself a lens — §2.5.1); it **sees pruned timelines**
(superseded / rolled-back / abandoned atoms stay resolvable — §2.5.2); and it has
**asymmetric visibility** (it observes every projection; no projection observes it —
§2.5.3). Together these fix the resolver's character completely: **neutral,
all-seeing, invisible, removable, read-only.** A concrete name for the resolver
surface (`AtomRegistry`, `Panopticon`, `Gallifrey`, …) is the user's call — a naming
hook, not a decision made here.
**Reads against ground truth at HEAD**:
`crates/clawft-service-agent/src/{substrate_sink,session_tier,session_forest}.rs`,
`crates/clawft-kernel/src/{chain,causal,crossref,context_graft,context_graft_state,view_resolver}.rs`,
`crates/clawft-weave/src/daemon.rs` (`conversation.graph`, anchor wiring),
`.planning/research/e5-rvf-integration-study.md` (the overlay frame),
ADR-056 (BVH), ADR-058 (index table), ADR-062 (forest join), ADR-067 (graph view).

---

## 0. TL;DR

1. **Verdict on the thesis: half-true, and the useful half is the half that's
   missing.** The user's instinct is correct about the *bones* — WeftOS already
   funnels **every atom through one choke point** (`SubstrateConversationSink::append_turn`
   → `KernelTurnAnchor::anchor_turn`), which mints **one global monotone key**
   (`ChainManager::append` → `event.sequence`, universally called `chain_seq`) and
   a **content-addressed identity** (`UniversalNodeId`, blake3 of
   `conv_id + chain_seq + text`). Those two keys are the de-facto spine. **But the
   "primary index" as a *resolvable mapping* does not exist yet.** Every reverse
   lookup ("given chain_seq 8633, where is it in each store?") is an **O(n) scan or
   is outright impossible today**, and the substrate JSONL — the store the user
   points at as "we have this" — is the **one projection that does not carry
   `chain_seq` at all**. So: the substrate is the right *spine*, but the connective
   tissue (a reverse-resolvable locator) is not there.

2. **The gap is structural, not incidental.** In `append_turn` the JSONL record is
   **published before** the chain append runs (`substrate_sink.rs:560` publish, then
   `:566` anchor → `:285` `chain.append`). So at the moment the substrate record is
   written, `chain_seq` **does not exist yet** — which is exactly why it isn't in the
   record. The substrate is keyed by an opaque `turn_id` (`{counter}-{ULID}`); the
   only artifact that binds `turn_id ↔ chain_seq ↔ conv_id` is the chain event's
   JSON payload, reachable only by scanning the chain.

3. **Recommendation: a materialized locator minted at the anchor seam — not a new
   store, not a computed resolver.** The choke point is the one place where *all*
   keys are simultaneously in hand (`conv_id`, `turn_id`, `event.sequence`,
   `content_hash`, `role`, `kind`, and `uid` derivable on the spot). Writing one
   sibling `AtomLocator` record there — ~30–50 lines inside `anchor_turn`, plus a
   small `AtomRegistry` — closes **every** reverse-lookup gap at once for O(1) cost,
   and incidentally is the first place the substrate ever carries `chain_seq`. A
   *computed* resolver cannot do this: it would pay O(n) scans per lookup and still
   **cannot** resolve `uid → node` or `chain_seq → turn_id` without a scan, because
   no reverse index exists in any store.

4. **What it unlocks is the whole overlay thesis, made safe.** The e5/RVF study
   already established "one atom stream, many projections, joined by `chain_seq`."
   The panopticon is what makes that join **guaranteed and O(1)** instead of
   conventional and O(n): composite queries (semantic ∩ causal ∩ temporal),
   every lens (graph / scrubber / BVH / semantic) navigating to the *same* atom, a
   one-call debug view, and — the highest immediate value — a **cross-index
   consistency audit** that would flag the "projection looks alive but doesn't join
   back" class instantly (it exists in the tree right now: the ECC brain HNSW is
   keyed by causal-node id with `chain_seq` hardcoded `0`).

5. **Two invariants from the "Gallifrey" metaphor (§2.5).** *Outside all
   projections*: the resolver is not itself a lens — no circular resolution (never
   query a projection to resolve into it), and its storage is a flat locator at the
   anchor, not entries in any lens. *Sees pruned timelines*: superseded acks,
   rolled-back COW branches, and cancelled-turn frontier residue stay **resolvable**
   — the locator carries a `disposition` (Committed / Superseded / Pruned /
   AbandonedBranch) and `locate()` never fails because history moved on; it reports
   where the atom ended up. The materialized-at-anchor design satisfies both for
   free (minted at witness time, before any prune decision; a lifecycle transition
   updates disposition, never deletes the locator).

6. **Third invariant — asymmetric visibility (§2.5.3, Bentham's guard).**
   Dependency arrows point **only** resolver → projections, never the reverse; the
   lenses stay completely unaware the resolver exists. Corollaries: the system is
   **fully functional with the resolver absent** (pure observation, zero
   back-pressure — the anchor-time locator write is fire-and-forget/best-effort,
   exactly like the existing `index_turn` "non-fatal" and `anchor_turn` best-effort
   pattern: a failed locator write degrades observability, never the atom), and the
   surface is **strictly read-only** over the projections (`atom.locate` mutates
   nothing — the guard watches, it does not reach into cells).

7. **Sequencing: ADR-069 (the invariant) + one small Plane item (the resolver).**
   This is genuinely ADR-shaped — a cross-cutting invariant constraining every
   present and future projection (semantic, causal, spatial/BVH, federation). Land
   the invariant first (cheap, documents the discipline the e5, agenticow, and BVH
   work must already follow); the resolver implementation is a fast-follow because
   the mint site is a seam we already own and that already computes the `uid`.

---

## 1. Confirm or refute: is substrate + witness chain the de-facto primary index?

**Confirmed as the spine; refuted as an existing resolver.**

Two things are unambiguously true at HEAD and they are the load-bearing truth in
the user's instinct:

- **One choke point.** `SubstrateConversationSink::append_turn`
  (`substrate_sink.rs:536-568`) is the single seam every atom crosses: it publishes
  the JSONL record, then calls `anchor.anchor_turn(conv_id, turn_id, turn)`
  (`:566`). `KernelTurnAnchor::anchor_turn` (`:259-333`) is where the atom is
  witnessed. The atom types that pass through it: `user` / `assistant` / `tool`
  text turns from the agent loop (`loop_core.rs:1385`, `:1693`, `:1859`), voice
  Talk-Mode turns via the `agent.turn.record` daemon arm (`daemon.rs:5007`), and
  subagent spawn turns (`subagent_spawn_commit.rs:244`). The in-process CLI uses
  `NoopTurnAnchor` (`:189-194`) and mints no `chain_seq` — deliberately, so only
  daemon-witnessed atoms get a sequence.
- **One global key + one identity.** `ChainManager::append` (`chain.rs:1041`)
  increments a single monotone `sequence` (`:834`) across *all* conversations
  (`conv_id` is payload data, never a chain dimension) and returns a `ChainEvent`
  whose `sequence` field (`:140`) is the value everything downstream calls
  `chain_seq`. `UniversalNodeId::new` (`crossref.rs:71`) hashes
  `StructureTag + conv_id + chain_seq + text + "turn"` into a 32-byte blake3
  identity (`session_forest.rs:98-106`).

So the substrate/witness pair **is** the origin of every key. That is why the
thesis *feels* right — "we have this with the existing substrate" is true at the
level of *where keys are born*.

**What refutes "we have a primary index" is reverse-resolution.** A primary index
must answer "given an atom's key, where is it in each store?" Today that answer is
O(n) or nonexistent, and the keys are not even present uniformly. The actual join
keys per projection:

| Projection | Key(s) it stores | Where stored | Reverse-lookup by `chain_seq`/`uid` today? | Cost |
|---|---|---|---|---|
| **Witness chain** (`ChainManager`) | `sequence` (=`chain_seq`, global monotone); payload `{conv_id, turn_id, role, content_hash, ts_ms}` — **no `uid`** | in-mem `Vec<ChainEvent>`; opt. `save_to_file` NDJSON (`chain.rs:1313`) | by seq: **O(n)** scan (`tail_from`, `:1141`) — no `get(seq)` index | O(n) |
| **Substrate JSONL** (`SubstrateSink`) | filename `{counter}-{ULID}` = `turn_id`; body `{turn_id, role, content, tool_calls, ts_ms, content_type}` — **no `chain_seq`, no `uid`** | `substrate/_derived/chat/<conv>/turns/<turn_id>` (`:544`) | by `chain_seq`: **not possible directly** — must O(n)-scan chain → `turn_id` → O(1) path read | O(n) + O(1) |
| **Causal node** (`CausalGraph`) | numeric `NodeId`; metadata `{conv_id, chain_seq, role, state, uid, +classification, +text, +voice_analysis}` | in-mem graph | by `chain_seq`: **O(n)** (`nodes_for_conv`, `causal.rs:719`); by `uid`: **O(n)** (no `uid→node` index) | O(n) |
| **Cross-refs** (`CrossRefStore`) | forward/reverse maps keyed by `UniversalNodeId` | `DashMap` fwd + rev (`crossref.rs:201-203`) | by `uid`: **O(1)** fwd/rev; but `all()` is unscoped → conv-scoping is O(n) intersect (`daemon.rs:6068`) | O(1) / O(n) scope |
| **SessionView HNSW** (semantic L2) | label = `chain_seq.to_string()`; twin `chunks: DashMap<u64, ChunkMeta>` | ephemeral, per-conv (`context_graft.rs:228-229`) | by `chain_seq`: **O(1)** → `ChunkMeta` (text, `content_hash`, `state`, `kind`) (`:245-246`) | **O(1)** — the gold standard |
| **ConvForest lineage** | `seq_to_node`, `seq_to_uid` (forward only) | ephemeral, per-conv (`session_forest.rs:46-49`) | seq→node O(1), seq→uid O(1); **no reverse** (uid→seq absent) | O(1) fwd |
| **ECC brain HNSW** (`democritus`) | label = **causal node id** string (**not** `chain_seq`, **not** `uid`); `chain_seq` hardcoded `0` at insert (`democritus.rs:293`, `:315`) | in-mem | label → causal node only; → `chain_seq` **not wired** | broken join |
| **BVH / `SpatialBackend`** | `LeafId` → chain via `exochain_seq` ordering | — | **design-only / deferred** — `clawft-bvh` + `weftos-leaf-types/spatial` absent (ADR-056 §5-6; ADR-058:113) | not built |
| **leaf-scene display AABB** | `NodeId = [DisplayId:8 | PathHash:24]` | `weftos-leaf-scene/src/id.rs:34` | unrelated to `chain_seq` (display path hash; not a causal index) | n/a |

**Places the mapping is implicit / conventional rather than guaranteed:**

1. **HNSW label → `chain_seq`** is a *string-parse convention* plus a twin-write
   (`insert_vector` writes the same `chain_seq` into both the HNSW store as a
   stringified id and the `chunks` DashMap as a u64 key, `context_graft.rs:228-229`).
   It is 1:1 *by construction*, not type-enforced. Reliable, but a convention any
   future inserter could break.
2. **`uid` = blake3(`conv_id + chain_seq + text`)** is derivable only when you hold
   all three inputs, and is **not reversible**. There is **no `uid → node` index
   anywhere** — `ConvForest` has only forward maps; `CausalGraph.nodes` is keyed by
   numeric id. So the `conversation.graph` RPC's own edges resolve `uid`s only
   because it builds an ad-hoc `numeric_to_uid` map while walking the conv
   (`daemon.rs:6008`).
3. **`turn_id` (ULID) ↔ `chain_seq`** is bridged **only** by the chain event's JSON
   payload. The substrate record itself has no `chain_seq`; the chain has no
   durable `turn_id → seq` index. Break the chain's in-mem `Vec` and this join is
   gone.
4. **ECC brain HNSW keyed by causal-node id with `chain_seq = 0`** — a projection
   that *looks* populated (vectors present, searchable) but **cannot join back to
   the atom stream**. This is the concrete "M2 false-alarm class" living in the tree
   today: a store whose consistency with the spine is silently absent.
5. **BVH leaf → `chain_seq`** ("this ADR cross-keys BVH leaves into that store by
   chain sequence", ADR-056:51-54) is **design text only** — no crate exists.

**Conclusion.** The substrate + witness chain is the *de-facto spine* (all keys are
born there) but **not** a *de-facto primary index* (no store is reverse-resolvable
without an O(n) scan, and two of the keys — `chain_seq` in JSONL, any usable key in
the ECC brain HNSW — are missing entirely). The panopticon is precisely the missing
reverse-resolution layer, and the substrate is the right place to hang it.

---

## 2. The resolver: minimal formalization

### 2.1 It is NOT a new store — it is one sibling write at a seam we already own

The instinct to avoid a new subsystem is correct. The panopticon is a **derived
index** (a projection *of* the atom stream, itself rebuildable from the chain), not
a new authority. Two parts:

**(a) The invariant (the doc).** *Every projection over the atom stream MUST key
each entry by `chain_seq` or `uid`, and MUST be reverse-resolvable from the atom's
key back to that entry.* A projection that cannot be reverse-resolved (today: the
ECC brain HNSW; substrate JSONL) is out of compliance and either carries the key or
registers its label in the locator. This invariant is what makes the overlay
composable and auditable; it is the real deliverable.

**(b) The resolver API — `atom(uid | chain_seq) -> AtomLocator`.**

```rust
/// The canonical coordinates of one atom across every projection.
/// It MAPS; it does not STORE content (refs only — see §4).
struct AtomLocator {
    // --- Durable identity (always present; minted at the anchor seam) ---
    chain_seq: u64,             // primary key — the global monotone witness seq
    uid: UniversalNodeId,       // content-addressed identity (32B blake3)
    conv_id: String,
    turn_id: String,            // → substrate/_derived/chat/<conv_id>/turns/<turn_id>
    content_hash: String,       // dedup / integrity (matches ChunkMeta.content_hash)
    role: String,
    kind: String,               // agent.chat.turn | agent.turn.record | spawn.goal | ...
    ts_ms: u64,

    // --- Lifecycle disposition (§2.5.2 — "sees pruned timelines") ---
    // Where this atom ENDED UP, not whether it survived. Resolution never fails
    // because history moved on; a superseded ack or a rolled-back branch atom
    // still resolves, tagged with how it left the frontier.
    disposition: Disposition,   // Committed | Superseded | Pruned | AbandonedBranch{branch}
    branch: Option<BranchId>,   // Some for agenticow COW / speculative branches

    // --- Projection refs (Some iff that projection indexed this atom) ---
    causal_node: Option<u64>,       // CausalGraph NodeId (durable while graph lives)
    view_seq: Option<Rebuildable<u64>>,     // == chain_seq in SessionView (L2, ephemeral)
    hnsw_label: Option<Rebuildable<String>>, // == chain_seq.to_string() (ephemeral)
    bvh_leaf: Option<LeafId>,       // deferred (ADR-056) — reserved
}

enum AtomKey { ByChainSeq(u64), ByUid(UniversalNodeId) }

trait AtomRegistry {
    fn locate(&self, key: AtomKey) -> Option<AtomLocator>;
    fn record(&self, loc: AtomLocator);          // called once, at anchor time
    fn audit(&self) -> ConsistencyReport;        // §3.4 health check
}
```

`Rebuildable<T>` marks a coordinate into an **ephemeral** projection (L2
`SessionView`, ECC in-mem HNSW): the locator promises the *durable* coordinates
(`chain_seq`, `uid`, `turn_id`, `causal_node`) unconditionally, and marks
`view_seq`/`hnsw_label` as "rebuildable from chain" rather than resolving into a
possibly-reaped view (§4). This mirrors the existing contract — the chain is the
source of truth, L2 is disposable (`context_graft.rs` "drop it at session end").

### 2.2 Materialized registry vs computed resolver — recommendation and costs

| | **Materialized** (record at anchor time) | **Computed** (derive on demand) |
|---|---|---|
| **Mint cost** | one DashMap insert + one sibling NDJSON append per atom, at a seam that already runs per atom | zero write |
| **`locate(chain_seq)`** | **O(1)** (two forward maps: by seq, by uid) | O(n) chain scan → `turn_id`; O(n) `nodes_for_conv` → causal node; view/HNSW O(1) |
| **`locate(uid)`** | **O(1)** | **not possible** without a full scan — no `uid → node` index exists anywhere |
| **`chain_seq → turn_id`** | O(1) (stored on the locator) | O(n) chain scan (payload holds the pair) |
| **Fixes JSONL-missing-`chain_seq`** | **yes** — the locator sibling is the first place substrate carries `chain_seq`, without rewriting the already-published turn record | no |
| **Consistency audit (§3.4)** | trivial (compare registry vs each projection) | expensive (re-scan every store per atom) |
| **Rebuild after crash** | replay the chain → re-emit locators (it is a derived cache) | n/a |
| **Marginal complexity** | ~1 small type + ~30–50 lines at the seam | scattered scan helpers, still can't answer `uid→` |

**Recommendation: materialized, written at the anchor seam.** The computed
approach is disqualified not by cost alone but by *capability* — it structurally
cannot answer `locate(uid)` or `chain_seq → turn_id` without a scan, because the
reverse indexes it would need do not exist. The materialized record is the *only*
artifact where `turn_id ↔ chain_seq ↔ uid ↔ causal_node` are bound together, and
the anchor seam is the *only* moment they are all in hand at once.

### 2.3 Where it mints (resolver-at-anchor-time spec, ~lines)

Inside `KernelTurnAnchor::anchor_turn` (`substrate_sink.rs:259-333`), immediately
after the chain append returns (`:285`, where `event.sequence` becomes known — the
`content_hash` was already computed at `:264-271`):

```rust
// (existing) let event = chain.append("agent", "agent.chat.turn", Some(payload));
let uid = session_forest::turn_universal_id(conv_id, event.sequence, &turn.content);
if let Some(registry) = &self.registry {
    registry.record(AtomLocator {
        chain_seq: event.sequence,
        uid,
        conv_id: conv_id.to_string(),
        turn_id: turn_id.to_string(),
        content_hash,                 // already in scope
        role: turn.role.clone(),
        kind: "agent.chat.turn".into(),
        ts_ms,                        // already in the payload
        causal_node: None,            // filled by index_turn's dual-write (see below)
        view_seq: None, hnsw_label: None, bvh_leaf: None,
    });
}
```

The projection refs (`causal_node`, `view_seq`, `hnsw_label`) are best filled by a
one-line back-reference from the sites that already own them: `dual_write_turn`
returns the `CausalNodeId` (`session_forest.rs:189`) and `SessionView::insert_vector`
already knows its label == `chain_seq` — each calls `registry.attach_*(chain_seq, …)`.
Alternatively, keep the locator minimal (durable fields only) and let `locate()`
resolve the ephemeral refs live through the existing `ViewResolver`/`ConvForest`
(both already O(1) forward). Recommend the minimal locator + live ephemeral
resolution: it keeps the mint on the hot path to a single insert and avoids
duplicating the ephemeral projections' lifecycle.

**Estimate**: `AtomRegistry` type + two forward `DashMap`s + optional substrate
sibling writer ≈ 80–120 lines in `clawft-service-agent` (next to `SessionTier`,
which already holds every dependency); the anchor-seam mint ≈ 30 lines; the
`atom.locate` RPC ≈ 40 lines mirroring `conversation.graph`.

### 2.4 API surface / who calls it

- **Kernel / service-agent (in-process)**: `AtomRegistry::locate(AtomKey)` for the
  daemon's own composite queries, the postmortem/promotion path, and the subagent
  spawner (which today reaches for `ConvForest::latest_turn_uid`,
  `session_tier.rs:245` — a one-off forward lookup the registry generalizes).
- **RPC `atom.locate`** (daemon, feature-gated like `conversation.graph`): params
  `{ chain_seq?: u64, uid?: string }`, returns the `AtomLocator` JSON. This is the
  seam tools and the GUI call to jump to the *same* atom from any lens
  (§3.2). Sits naturally beside `conversation.graph` (`daemon.rs:5942`) and
  `voice_watch`.
- **Not a per-atom hot-path dependency for producers** — producers keep writing to
  their own projections; the registry only *observes* at the anchor seam and
  *serves* on query.

### 2.5 The two Gallifrey invariants (from the metaphor)

The "outside of time, sees all of time and space" framing is not decoration — it
names two properties the resolver must have to be trustworthy, and both are
falsifiable against the design above.

#### 2.5.1 Invariant OUTSIDE — the resolver is not itself a projection

*The panopticon must not be one of the lenses.* It is not temporal, causal,
semantic, or spatial; it observes and maps between the lenses without living inside
any. Concretely:

- **No circular resolution.** Resolving *into* a projection must never require
  *querying* that same projection. `locate(chain_seq)` must not run an HNSW search
  to find the HNSW label, nor a `nodes_for_conv` scan to find the causal node — the
  coordinate is either stored on the flat locator (durable refs) or derived by a
  *direct forward map* the projection already owns (`ConvForest::seq_to_node`,
  `SessionView.chunks`), never by *searching* the lens. This is why the mint
  captures `turn_id` / `causal_node` at the anchor rather than re-deriving them on
  query.
- **Neutral storage.** If materialized, the locator is a **flat record at the
  anchor** (its own two forward maps + a substrate sibling), *not* entries inserted
  into any lens. The registry does **not** get a `chain_seq`, does **not** appear as
  a causal node, is **not** embedded into HNSW. It sits beside the lenses, keyed by
  the same universal keys they are keyed by, so it can join them without biasing any
  one of them. (This also keeps it from showing up in its own `audit()` or in
  `conversation.graph`.)
- **Why it matters.** Non-participation is exactly what makes cross-lens navigation
  *unbiased*: a resolver that lived inside (say) the causal graph would privilege
  causal reachability and inherit that lens's O(n) scan. Standing outside all four,
  it treats every lens as an equal peer reachable in O(1) from the shared key. This
  is the structural meaning of "outside of time."

#### 2.5.2 Invariant PRUNED — resolution sees abandoned timelines

*The panopticon sees all of time, including the timelines that did not happen.*
Atoms in superseded, rolled-back, or abandoned branches remain **resolvable** — the
locator carries the atom's **disposition** rather than dropping dead timelines.

- **The invariant, stated flatly**: `locate(key)` **never fails because history
  moved on.** It reports *where the atom ended up* (`Committed` / `Superseded` /
  `Pruned` / `AbandonedBranch`), not merely whether the atom survived onto the main
  line. A `Pruned` atom resolves exactly like a `Committed` one; only its
  `disposition` differs.
- **This is not hypothetical — the dead timelines already exist at HEAD and near
  it**: `NodeState::{Stale, Pruned}` are live states in the L2 model
  (`context_graft_state.rs:13-25`; `Stale` = "aged out, origin retained on chain,
  re-graftable"; `Pruned` = "contradicted/revised, dropped off `main_line`"); voice
  emits **Speculative acks that get superseded** when the real reply lands (ADR-061
  / voice ack pre-render); M2 D8 leaves **cancelled-turn frontier residue**; and the
  agenticow plan adds **COW rollback branches** whose atoms are witnessed but never
  promoted. Each of these is an atom that was witnessed on the chain (so it has a
  `chain_seq` and a `uid`) but is not on the surviving trunk. The scrubber and the
  audit both need to navigate to what *almost* happened as much as to what happened.
- **Why the materialized-at-anchor design already satisfies this for free**: the
  locator is minted at witness time (`chain.append` succeeded → `chain_seq` exists),
  which is **before** any supersede/prune/rollback decision. Once minted, the atom
  is in the registry permanently; a later lifecycle transition **updates
  `disposition`, it does not delete the locator**. Because the chain is append-only
  and the registry is a projection of it, a rolled-back branch's atoms are still
  chain events — so they are still locators. Pruning is a *state change on the
  frontier*, never a *retraction from the witness*, and the resolver keys off the
  witness. (A *computed* resolver would violate this the moment a projection reaped
  the atom — e.g. an L2 view dropped its `Stale` chunk — because it has no record
  the atom ever existed; this is a second, independent reason to prefer
  materialized.)
- **Interaction with the OUTSIDE invariant**: disposition is read/updated on the
  flat locator, not by asking a lens "are you still holding this?" The registry is
  the authority on disposition precisely because it stands outside the lenses that
  come and go.

#### 2.5.3 Invariant ASYMMETRIC — the guard sees the cells; the cells never see the guard

Bentham's original panopticon: one guard observes every cell; no inmate can observe,
address, or depend on the guard. In WeftOS that is a **unidirectional dependency**
rule — the hardest of the three, and the one that keeps the resolver from quietly
becoming load-bearing.

- **The invariant, stated flatly**: dependency arrows point **only** resolver →
  projections. **No projection observes, reads from, couples to, or depends on the
  resolver.** The chain, the causal graph, the HNSW, the substrate sink, the future
  BVH — each must remain completely unaware the panopticon exists. Concretely at the
  code level: no producer imports the `AtomRegistry` type on its write path; the
  registry is injected into the *anchor* (the observation seam), never into a lens.
- **Corollary 1 — removability (zero back-pressure).** The system must be **fully
  functional with the resolver absent.** It is pure observation with no load-bearing
  role in any read or write path of any lens. If materialized, the locator write at
  the anchor is **fire-and-forget / best-effort**, exactly matching the pattern the
  codebase already uses: `index_turn` is documented "**Non-fatal: indexing failure
  is logged, never propagated (the turn already landed on the chain)**"
  (`session_tier.rs:271-275`), and the anchor's causal/tier writes are all
  best-effort after the chain append. A failed or disabled locator write **degrades
  observability, never the atom** — the turn still witnesses, indexes, and grafts.
  This is the same shape as `KernelTurnAnchor::any_enabled` / `NoopTurnAnchor`: the
  observation layer is optional wiring, and its absence is a supported mode.
- **Corollary 2 — no privileged mutation (read-only).** The guard watches; the guard
  does not reach into cells. The resolver surface is **strictly read-only over the
  projections**: `atom.locate` (and `audit`) **never mutate** the chain, the graph,
  the HNSW, the substrate, or any lens. The only thing the registry writes is its
  *own* flat locator record, at the anchor, about an atom that already exists — it
  never writes *into* a projection. (This is what distinguishes it from
  `dual_write_turn`, which is a producer: the registry observes producers, it is not
  one.)
- **Honest note on the metaphor's connotation.** Bentham/Foucault's panopticon is
  about surveillance changing the *inmate's* behavior through the mere possibility of
  being watched. That connotation maps cleanly — and benignly — here: the "observed"
  are our own data structures, not people, and the mapping is to WeftOS's **existing
  witness discipline** — every atom is *always-already observable* the instant it is
  appended to the chain (that is what witnessing *is*). The resolver adds no new
  observation and no behavioral pressure; it only makes the observation that the
  witness chain already guarantees **navigable**. The panopticon does not introduce
  surveillance; it indexes a transparency the substrate already has by construction.
- **Why this is the load-bearing invariant of the three.** OUTSIDE keeps it neutral
  and PRUNED keeps it complete, but ASYMMETRIC is what keeps it *safe to add and safe
  to remove*: because nothing depends on it, it can be shipped incrementally
  (audit-only first, §3.4), disabled under load, or deleted wholesale without
  touching a single lens. A resolver that any projection depended on would be a new
  coupling at the exact center of the system — the opposite of the neutral observer
  the metaphor demands.

---

## 3. What it unlocks (concrete)

### 3.1 Guaranteed composite / overlay joins
The e5/RVF study's headline query — *"semantically similar to this turn AND within
2 causal hops AND in the last hour"* — is a three-lens intersection (HNSW top-N →
`CausalGraph` 2-hop → `chain_seq`/HLC window). Today each narrowing step re-derives
its join by convention and the cross-lens hops cost O(n) scans (`nodes_for_conv`,
chain `tail_from`). With the registry every candidate resolves O(1) to its
coordinates in every other lens, so the composite is a cheap key-intersection —
which is the mechanism the overlay frame *assumes* but does not yet have.

### 3.2 Every lens navigates to the same atom
Graph view (keyed by `uid`), scrubber (replays in `chain_seq` order), a future BVH
view (keyed by `LeafId`), and the semantic pane (HNSW label) all resolve through a
single `atom.locate`. Select an atom in any pane → `locate` → highlight it in the
other three. Today this is impossible for the substrate pane (no `chain_seq`) and
O(n) for the graph pane (no `uid → node` index). And because the locator sees pruned
timelines (§2.5.2), the scrubber can navigate to a **superseded** or **abandoned**
atom — a voice ack that was replaced, a rolled-back branch — and each pane can render
it with its disposition (greyed, struck, branch-tinted), so "what almost happened"
is a first-class destination, not a gap in the timeline.

### 3.3 One-call debugging
*"Show me everything the system knows about chain_seq 8633"* becomes a single
`locate(ByChainSeq(8633))`: substrate path (`conv_id` + `turn_id`), causal node id,
`uid`, `content_hash`, `state`, plus the cross-refs (via the existing O(1)
`CrossRefStore` by `uid`). Today this is a manual scan across three stores.

### 3.4 Cross-index consistency audit — the highest immediate value
`AtomRegistry::audit()`: for every witnessed atom (every chain event), assert it
appears in every **mandatory** projection and that each projection's key
reverse-resolves to the same atom. This catches the *"projection looks alive but
doesn't join back"* class **instantly** — and that class is not hypothetical: the
ECC brain HNSW is keyed by causal-node id with `chain_seq = 0` (`democritus.rs:315`),
so it silently fails to join to the spine. An audit over the registry would have
flagged the M2 false-alarm class as a red row on first run, rather than a
human-hours investigation. This slice is worth shipping first, even as a
diagnostic-only command before the full RPC.

### 3.5 Federation (ADR-063)
A signed-envelope A2A atom (ADR-063) can carry its `AtomLocator` so a receiving node
can place the foreign atom in its own overlay by `uid` (globally content-addressed)
without replaying the sender's whole chain. The locator is the natural envelope
payload for "here is this atom and where it sits."

---

## 4. Boundaries — what the panopticon is NOT

- **It maps; it does not store content.** The locator holds *references*
  (`turn_id` → substrate path, `content_hash`, `causal_node`), never the turn text
  or vectors. Content stays in substrate / blobs / chain. (Note the `text` already
  duplicated into causal metadata under the classification gate,
  `session_forest.rs:176`, is a *different* decision — the locator does not add to
  text-at-rest.)
- **It does not replace any projection.** HNSW still does semantic search; the
  causal graph still walks lineage; the chain is still the witness. The registry
  only makes them mutually reverse-resolvable. It is subordinate to all of them.
- **Nothing may depend on it (§2.5.3).** It is not a hard dependency of any read or
  write path; a lens must never import or call it. Its absence is a supported mode
  (removability), and it never mutates a projection (read-only). If any producer ever
  needs the registry to function, that is a design regression, not a feature.
- **It is itself rebuildable from the chain.** The registry is a *derived cache*
  with a durable substrate sibling, not a new source of truth. Lose it → replay the
  chain → re-emit locators. This keeps the chain as the sole authority (no new
  authority is introduced), which is the property that makes adding it safe.
- **Ephemeral projections are marked rebuildable, not resolved-into.** For L2
  `SessionView` and the in-mem ECC HNSW, `locate()` returns the durable coordinates
  unconditionally and marks `view_seq` / `hnsw_label` as `Rebuildable` rather than
  promising a live hit into a possibly-reaped view. This is the exact
  reaped-conversation contract the `ViewResolver` already encodes
  (`view_resolver.rs:18-21` — `None` on a reaped view is a logged no-op).
- **It does not re-key producers.** It observes at the anchor seam; it does not
  demand that HNSW or the chain change their internal id schemes (though the
  invariant *does* ask new projections to be reverse-resolvable — via their own key
  or via registration).
- **It sees pruned timelines; it does not resurrect them.** Reporting that a
  superseded/abandoned atom exists and where it ended up (§2.5.2) is a *read* — the
  panopticon never promotes a `Pruned` atom back onto the trunk, never re-grafts a
  reaped chunk, never un-rolls a branch. Disposition is descriptive, not an undo
  lever. (It is not itself a lens — §2.5.1 — so it cannot and must not act as one.)

---

## 5. Sequencing recommendation

**Both an ADR and a Plane item — ADR first.**

This is genuinely ADR-shaped: a cross-cutting *invariant* that constrains every
present and future projection (semantic/e5, causal/forest, spatial/BVH, lifecycle,
federation). It is exactly the kind of "how the whole substrate stays joinable"
decision ADR-056 gestures at ("cross-keys BVH leaves … by chain sequence") and the
e5 study assumes ("all joined by `chain_seq`") but that no ADR yet *states as a
rule*. Recommend:

1. **ADR-069 — "Atom Primary Index & Locator Resolver (the Panopticon)."** Records:
   the atom-identity model (`chain_seq` + `uid`, born at the anchor seam); the
   **invariant** ("every projection MUST key by `chain_seq`/`uid` and MUST be
   reverse-resolvable"); the `AtomLocator` / `AtomRegistry` / `atom.locate` surface;
   the materialized-at-anchor decision and its rationale; the **three character
   invariants** (§2.5 — resolver-outside-all-projections, resolution-sees-pruned-
   timelines carried as `disposition` on the locator, and asymmetric-visibility /
   unidirectional-dependency with its removability + read-only corollaries); and the
   explicit non-goals (§4). Cite it forward from the e5 study's overlay table and
   from
   ADR-056/062/067. Cost: documentation; unblocks the discipline the e5, agenticow,
   and BVH work should already be following.

2. **One Plane item (0.7.x or early 0.8.x) — "AtomRegistry + `atom.locate`
   resolver."** Scope: the `AtomRegistry` type + anchor-seam mint (~30–50 lines) +
   the `atom.locate` RPC + the **consistency-audit** health check. Acceptance:
   `locate(ByChainSeq)` and `locate(ByUid)` are O(1) and agree; the audit flags the
   ECC-brain-HNSW `chain_seq = 0` row as non-compliant; substrate finally carries a
   durable `chain_seq ↔ turn_id` binding via the locator sibling. Dependency: none
   blocking — the mint site (`anchor_turn`) already computes `uid` and holds every
   field. Slice order within the item: **audit first** (§3.4 — cheapest, catches
   live bugs), then `locate` + RPC, then GUI wiring.

The two are sized right: the invariant is the load-bearing artifact (it changes how
every future index is built), and the resolver is small precisely because the hard
part — funnelling every atom through one seam that mints one key and one identity —
**is already done**. The panopticon is not new machinery; it is the reverse index
the machinery has been missing.
