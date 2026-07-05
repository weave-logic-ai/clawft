# ADR-069: Atom Primary Index & Locator Resolver (the Panopticon) — one reverse-resolvable map over every projection

**Date**: 2026-07-05
**Status**: Proposed
**Deciders**: system-architect design agent + team-lead, from the user's "primary index / panopticon" directive; grounded against HEAD `dcb70de1` (`feat/hermes-loop-base`)
**Depends-On**: **ADR-046** (Forest of Trees — the ExoChain / Causal / HNSW / CrossRef structures this maps between), **ADR-058** (per-conversation context memory tier — the L2 `SessionView`/`chain_seq` substrate + its index-as-projection framing), ADR-062 (ECC graph-walk conversation — the anchor→`index_turn`→dual-write path the locator mints on), ADR-067 (conversation graph view — the `conversation.graph` RPC this generalizes)
**Relates-To**: ADR-056 (BVH-on-RVF 4D index — the deferred spatial lens, "cross-keys BVH leaves … by chain sequence"; the panopticon is where that key is made real), ADR-059 (embedder — the semantic lens the e5 study upgrades), ADR-063 (signed-envelope A2A — federation carrying locators), `.planning/research/e5-rvf-integration-study.md` (Part 6 overlay table: "one atom stream, many projections, joined by `chain_seq`" — this ADR is the resolver that overlay assumes), `.planning/research/panopticon-primary-index.md` (the full ground-truth analysis behind this decision)
**Motivating live defect**: the ECC brain HNSW (`democritus.rs:293`) inserts vectors keyed by causal-node id with `chain_seq` hardcoded `0` (`:315`) — a projection that looks populated but cannot join back to the atom stream. No audit exists to catch this class; this ADR adds one.

## Context

WeftOS builds a family of indexes ("lenses") over the same conversation atoms —
temporal (witness chain), causal (`CausalGraph` + `CrossRefStore`), semantic (HNSW
`SessionView` / `ecc_vector_backend`), lifecycle (`NodeState`), and — planned —
spatial (BVH, ADR-056). The e5/RVF study (Part 6) named the design principle: *one
atom stream, many projections, all joined by `chain_seq`.* But that overlay is only
as good as the join. **At HEAD the join is conventional and mostly one-directional,
not guaranteed and reverse-resolvable.** Two findings crystallize the gap:

1. **The substrate — the store the "we have this already" intuition points at — is
   the one projection that does not carry `chain_seq`.** In
   `SubstrateConversationSink::append_turn` the JSONL record is **published
   (`substrate_sink.rs:560`) before the chain append that mints the sequence
   (`:566` → `anchor_turn` → `chain.append` `:285`).** So at write time `chain_seq`
   does not yet exist; the record is keyed by an opaque `turn_id` (`{counter}-{ULID}`),
   and the only artifact binding `turn_id ↔ chain_seq ↔ conv_id` is the chain event's
   JSON payload — reachable only by an O(n) chain scan.

2. **Every reverse lookup is O(n) or impossible, and one lens has a broken key.**
   Given an atom's key, "where is it in each store?" costs: chain O(n) (`tail_from`,
   no `get(seq)`); substrate not-directly-possible; causal O(n) (`nodes_for_conv`,
   no `uid→node` index); ECC brain HNSW cannot join at all (`chain_seq = 0`). The one
   O(1)-reverse-resolvable lens is `SessionView` HNSW (label = `chain_seq.to_string()`
   twinned with a `chunks: DashMap<u64, ChunkMeta>`, `context_graft.rs:228-229`) — and
   that is exactly the shape this ADR generalizes to all lenses.

**What already exists and is load-bearing** (so this is a small addition, not new
machinery): every atom — user/assistant/tool text turns (`loop_core.rs`), voice
turns via `agent.turn.record` (`daemon.rs:5007`), subagent spawns — passes **one
choke point** (`append_turn` → `KernelTurnAnchor::anchor_turn`), which mints **one
global monotone key** (`ChainManager::append` → `event.sequence`, universally called
`chain_seq`; single counter across all conversations) and a **content-addressed
identity** (`UniversalNodeId` = blake3 of `StructureTag + conv_id + chain_seq + text
+ "turn"`, `session_forest.rs:98-106`). The keys are born at one seam; only the
reverse map is missing.

**Verdict on the thesis (recorded for the record):** *half-true.* The substrate +
witness chain is the de-facto **spine** (all keys originate there) but **not** a
de-facto **primary index** (no store is reverse-resolvable without a scan; two keys
are missing entirely). This ADR supplies the missing reverse-resolution layer.

## Decision

**Introduce a primary atom index — the Panopticon — a derived, reverse-resolvable
map minted at the anchor choke point. It is not a new authority and not a new lens:
it observes every projection and maps between them, and nothing depends on it.**

### D1 — Atom identity model: `chain_seq` + `uid`, born at the anchor seam

Every atom is canonically identified by two keys, both minted where they already are
today: the global monotone **`chain_seq`** (`ChainManager::append`'s `event.sequence`
— temporal/ordinal primary key) and the content-addressed **`uid`**
(`UniversalNodeId`, `turn_universal_id(conv_id, chain_seq, text)` — globally stable
identity, the cross-ref / federation key). Both are derivable at
`KernelTurnAnchor::anchor_turn` the moment the chain append returns; no producer
changes its own id scheme.

### D2 — The invariant (the load-bearing rule this ADR ratifies)

> **Every projection over the atom stream MUST key each entry by `chain_seq` or
> `uid`, and MUST be reverse-resolvable from that key back to the atom — either by
> carrying the key on its own entries, or by registering its label with the
> locator.**

A projection that cannot be reverse-resolved is out of compliance. This binds all
present and future lenses: semantic (ADR-059/e5), causal (ADR-062), spatial (ADR-056
BVH — this is where its deferred "keyed by chain sequence" becomes a *requirement*),
lifecycle, and federation (ADR-063). The current violators — substrate JSONL (no
`chain_seq`) and the ECC brain HNSW (`chain_seq = 0`) — are the first compliance
targets.

### D3 — The `AtomLocator` — coordinates, not content

```rust
struct AtomLocator {
    // Durable identity — always present, minted at the anchor seam (D1):
    chain_seq: u64,
    uid: UniversalNodeId,
    conv_id: String,
    turn_id: String,          // → substrate/_derived/chat/<conv_id>/turns/<turn_id>
    content_hash: String,     // matches ChunkMeta.content_hash — dedup / integrity
    role: String,
    kind: String,             // agent.chat.turn | agent.turn.record | spawn.goal | ...
    ts_ms: u64,

    // Lifecycle disposition (D6) — where the atom ENDED UP, not whether it survived:
    disposition: Disposition, // Committed | Superseded | Pruned | AbandonedBranch{branch}
    branch: Option<BranchId>, // Some for agenticow COW / speculative branches

    // Projection refs — Some iff that projection indexed the atom; Rebuildable for
    // ephemeral lenses (D5):
    causal_node: Option<u64>,                // CausalGraph NodeId
    view_seq:   Option<Rebuildable<u64>>,    // == chain_seq in SessionView (L2, ephemeral)
    hnsw_label: Option<Rebuildable<String>>, // == chain_seq.to_string() (ephemeral)
    bvh_leaf:   Option<LeafId>,              // reserved — ADR-056, deferred
}
```

The locator holds **references** (`turn_id` → substrate path, `content_hash`,
`causal_node`), never the turn text or vectors. It maps; it does not store content.

### D4 — Materialized at the anchor seam; the `AtomRegistry` + `atom.locate` surface

The locator is **materialized** — written once, at the anchor, into an
`AtomRegistry` (two O(1) forward maps, by `chain_seq` and by `uid`, plus an optional
flat substrate sibling for durability). The anchor is the **only** point where all
keys are simultaneously in hand (`conv_id`, `turn_id`, `event.sequence`,
`content_hash`, `role`, `kind`, and `uid` derivable on the spot), so the mint is one
insert with zero extra scans:

```rust
// inside KernelTurnAnchor::anchor_turn, right after `let event = chain.append(...)`:
let uid = session_forest::turn_universal_id(conv_id, event.sequence, &turn.content);
if let Some(registry) = &self.registry {           // optional wiring — see D7
    registry.record(AtomLocator { chain_seq: event.sequence, uid, /* … */ });
}
```

Surface:

```rust
enum AtomKey { ByChainSeq(u64), ByUid(UniversalNodeId) }
trait AtomRegistry {
    fn locate(&self, key: AtomKey) -> Option<AtomLocator>;  // O(1)
    fn record(&self, loc: AtomLocator);                     // once, at anchor time
    fn audit(&self) -> ConsistencyReport;                   // D8
}
```

An **RPC `atom.locate`** (daemon, feature-gated like `conversation.graph`,
`daemon.rs:5942`) returns the locator JSON for `{ chain_seq? | uid? }` — the seam
tools and the GUI use to jump to the *same* atom from any lens.

### D5 — Ephemeral projection refs are `Rebuildable`, never resolved-into

For ephemeral lenses (L2 `SessionView`, in-mem ECC HNSW) the locator returns the
*durable* coordinates unconditionally and marks `view_seq` / `hnsw_label` as
`Rebuildable` (rebuildable-from-chain) rather than promising a live hit into a
possibly-reaped view. This mirrors the existing contract — the chain is the source
of truth, L2 is disposable — and the reaped-conversation behavior the `ViewResolver`
already encodes (`view_resolver.rs:18-21`: `None` on a reaped view is a logged
no-op).

### D6 — The three metaphor-derived character properties (named requirements)

From the user's two framings — Doctor Who's Gallifrey ("outside of time, sees all of
time and space") and Bentham's original panopticon ("one guard sees every cell; no
inmate sees the guard") — three non-negotiable properties:

- **P1 — Non-participation (OUTSIDE all projections).** The resolver is not itself a
  lens. **No circular resolution**: resolving *into* a projection must never require
  *querying* that projection (no HNSW search to find an HNSW label, no
  `nodes_for_conv` scan to find a causal node — coordinates come from the flat
  locator or a projection's own direct forward map). **Neutral storage**: the locator
  is a flat record at the anchor, keyed by the same universal keys the lenses use;
  it never inserts entries into any lens (no `chain_seq` of its own, not a causal
  node, not embedded in HNSW — so it never appears in its own `audit()` or in
  `conversation.graph`). This neutrality is what makes cross-lens navigation
  unbiased and O(1) from the shared key.

- **P2 — Sees pruned timelines (disposition, D3).** Atoms in superseded,
  rolled-back, or abandoned branches remain **resolvable**. `locate()` **never fails
  because history moved on** — it reports *where the atom ended up*
  (`Committed`/`Superseded`/`Pruned`/`AbandonedBranch`), not merely whether it
  survived onto the trunk. This is not hypothetical: `NodeState::{Stale, Pruned}` are
  live (`context_graft_state.rs`), voice emits superseded Speculative acks
  (ADR-061), M2 leaves cancelled-turn frontier residue, and agenticow adds COW
  rollback branches. Because the locator is minted at witness time — *before* any
  supersede/prune decision — a later lifecycle transition **updates `disposition`,
  it never deletes the locator**. Pruning is a frontier state change, never a
  retraction from the append-only witness the resolver keys off.

- **P3 — Asymmetric visibility (unidirectional dependency).** Dependency arrows
  point **only** resolver → projections; **no projection observes, reads from,
  couples to, or depends on the resolver.** No producer imports the `AtomRegistry`
  on its write path; the registry is injected into the *anchor* (the observation
  seam), never into a lens.
  - *Corollary — removability (zero back-pressure).* The system MUST be fully
    functional with the resolver absent. The locator write is **fire-and-forget /
    best-effort**, matching the pattern already in the codebase: `index_turn` is
    "**Non-fatal: indexing failure is logged, never propagated (the turn already
    landed on the chain)**" (`session_tier.rs:271-275`). A failed or disabled
    locator write degrades observability, never the atom. Absence is a supported mode
    (like `NoopTurnAnchor`).
  - *Corollary — read-only (no privileged mutation).* `atom.locate` and `audit`
    **never mutate** any projection. The registry writes only its own flat locator
    record, about an atom that already exists; it never writes *into* a lens. (This
    is what distinguishes it from `dual_write_turn`, a producer: the registry
    observes producers, it is not one.) It sees pruned timelines but does not
    resurrect them — disposition is descriptive, not an undo lever.

### D7 — Optional wiring; audit-first sequencing

The registry is optional daemon wiring, gated like the other ECC features. The
implementation lands in three slices, cheapest and most valuable first:
1. **`audit()`** (diagnostic-only command) — flags the ECC-brain-HNSW `chain_seq = 0`
   row and any non-compliant projection immediately.
2. **`record` + `locate` + the `atom.locate` RPC** — the reverse map itself; the
   substrate finally gains a durable `chain_seq ↔ turn_id` binding via the locator
   sibling.
3. **GUI wiring** — the graph view / scrubber / semantic pane cross-navigation.

## Consequences

### Positive
- **Guaranteed O(1) overlay joins.** The e5 study's headline composite —
  "semantically similar AND within 2 causal hops AND in the last hour" — becomes a
  cheap key-intersection instead of O(n) cross-lens scans. The overlay stops assuming
  a join it does not have.
- **Every lens navigates to the same atom.** Graph view (`uid`), scrubber
  (`chain_seq` order), semantic pane (HNSW label), future BVH view (`LeafId`) all
  resolve through one `atom.locate` — including to superseded/abandoned atoms (P2),
  so "what almost happened" is a first-class destination.
- **One-call debugging.** `locate(ByChainSeq(8633))` returns substrate path, causal
  node, `uid`, `content_hash`, disposition, and cross-refs in one shot.
- **A consistency audit that catches the live defect class.** `audit()` would have
  flagged the `democritus.rs` `chain_seq = 0` join as a red row on first run — the
  "projection looks alive but doesn't join back" class, instantly.
- **Federation-ready (ADR-063).** A signed-envelope atom can carry its locator so a
  receiving node places it by globally-content-addressed `uid` without replaying the
  sender's chain.

### Negative
- **One more write at the choke point.** A per-atom `record` on the anchor path —
  mitigated by being fire-and-forget (P3) and a single DashMap insert.
- **A derived index to keep honest.** The registry can drift from the lenses; the
  `audit()` (D8) exists precisely to detect that, and the registry is rebuildable
  from the chain on any doubt.
- **Disposition upkeep.** Lifecycle transitions must update `disposition`; if a
  transition path forgets to, an atom shows a stale disposition (resolution still
  succeeds — it never *fails* — but the label may lag). The audit covers this too.

### Neutral
- **Not a new source of truth.** The chain remains the sole authority; the registry
  is a derived cache with a durable sibling, replayable from the chain. This is the
  property that makes adding it — and removing it — safe.
- **Naming is deferred.** `AtomRegistry` / `Panopticon` / `Gallifrey` for the
  surface is the user's call; this ADR fixes the behavior, not the name.

## Non-goals

- **It maps; it does not store content.** References only (`turn_id`,
  `content_hash`, `causal_node`) — never turn text or vectors. (It adds nothing to
  the text-at-rest already gated into causal metadata at `session_forest.rs:176`.)
- **It does not replace any projection.** HNSW still does semantic search; the causal
  graph still walks; the chain is still the witness. It only makes them mutually
  reverse-resolvable, and is subordinate to all of them.
- **Nothing may depend on it (P3).** If any producer ever needs the registry to
  function, that is a design regression, not a feature.
- **It does not re-key producers.** It observes at the anchor seam; it does not force
  HNSW or the chain to change internal id schemes (though D2 does require new
  projections to be reverse-resolvable via their own key or registration).
- **It does not resurrect pruned timelines.** Reporting a superseded/abandoned atom
  is a read; it never promotes a `Pruned` atom, re-grafts a reaped chunk, or un-rolls
  a branch.

## Alternatives considered

- **Computed resolver (derive each mapping on demand, no materialization).**
  *Rejected on capability, not merely cost.* It structurally cannot answer
  `locate(uid)` or `chain_seq → turn_id` without a full scan, because the reverse
  indexes it would need (`uid → node`, `chain_seq → turn_id`) do not exist in any
  store. It also violates P2: the moment a projection reaps an atom (an L2 view drops
  a `Stale` chunk), a computed resolver has no record the atom ever existed. The
  materialized locator is the *only* artifact binding `turn_id ↔ chain_seq ↔ uid ↔
  causal_node`, and the anchor is the only moment they are co-present.
- **A new authoritative store (atoms live in the panopticon).** *Rejected.* It would
  make the resolver load-bearing, violating P3 (removability) and duplicating content
  the substrate/chain already own. The registry is deliberately a derived cache,
  never an authority.
- **Per-projection reverse indexes (add a `uid → node` map to the causal graph, a
  `chain_seq → path` map to the substrate, etc.).** *Rejected as the primary
  mechanism.* It scatters the join across N lenses (N places to keep consistent, N
  audits, and each lens must learn about the universal keys), and still yields no
  single place to ask "where is this atom across *all* lenses" — which is the whole
  point. Individual lenses may still add forward maps for their own needs; the
  panopticon is the one cross-lens reverse map.

## Acceptance criteria (for ratification → MUST-HAVE)

1. `locate(ByChainSeq)` and `locate(ByUid)` are O(1) and return the same
   `AtomLocator` for the same atom.
2. Resolution of a `Superseded` / `Pruned` / `AbandonedBranch` atom succeeds and
   reports the correct `disposition` (P2 — resolution never fails because history
   moved on).
3. Disabling / removing the registry leaves every read and write path of every lens
   fully functional; a forced locator-write failure never fails the turn (P3
   removability), and no lens code imports the registry (P3 asymmetry).
4. `atom.locate` / `audit` perform no mutation on any projection (P3 read-only).
5. `audit()` flags the ECC-brain-HNSW `chain_seq = 0` projection (`democritus.rs:315`)
   as non-compliant, and the substrate carries a durable `chain_seq ↔ turn_id`
   binding via the locator sibling.
6. The locator holds no turn text or vectors — references only (non-goal #1).

## Reference

Full ground-truth analysis, the per-projection join-key table, the
materialized-vs-computed weighing, and the metaphor derivation:
`.planning/research/panopticon-primary-index.md`.
