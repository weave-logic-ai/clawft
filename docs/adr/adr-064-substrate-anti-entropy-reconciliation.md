# ADR-064: Fail-closed anti-entropy reconciliation for cross-node substrate sync

**Date**: 2026-07-03
**Status**: Proposed
**Deciders**: Substrate/mesh security review 2026-07-03 (AgentBBS pattern study)
**Depends-On**: ADR-063 (substrate signed envelope), ADR-025 (Ed25519 node
identity), ADR-028 (ML-DSA dual signing), ADR-022 (ExoChain audit),
ADR-039 (SWIM failure detection), ADR-040 (LWW-CRDT process table)
**Interlocks-With**: ADR-063 (the envelope is the unit reconciled),
ADR-066 (capability tokens gate who may sync a subtree)

## Context

ADR-063 makes a single substrate write self-authenticating. It says nothing
about **convergence**: how two daemons that each hold part of the substrate
tree bring their views into agreement after a partition, a reboot, or a late
join. Today `crates/clawft-substrate` has no cross-node sync at all — the
`mesh` adapter (`src/mesh.rs`) only *reads* `cluster.status` / `cluster.nodes`
from the local daemon, and the substrate map is per-process.

The mesh is explicitly heading toward multi-node substrate:
JOURNALED-NODE-ESP32.md §8 colocates node and Actor state in the substrate;
ADR-057 exists precisely because "subscriber-only nodes" (the watch, HMI
displays) will read another node's subtree across the mesh. Once a subtree
authored on node A must appear on node B, we need a reconciliation protocol —
and a hostile-peer-tolerant one, because ADR-025's whole premise is that a
mesh peer is authenticated but **not trusted to be honest**.

The failure we must design against: node B receives a bulk substrate sync from
node A (or from a relay claiming to carry A's data) and must not (a) accept a
single forged delta, (b) accept a partially-valid batch that leaves its tree in
a half-applied inconsistent state, or (c) be driven into divergence by a peer
that selectively drops or reorders envelopes.

## Decision

Adopt a **digest / reconcile / snapshot** anti-entropy protocol over the
ADR-063 signed envelope, with **fail-closed batch verification** and
**egress-only trust**. Convergence is defined per **substrate subtree**
(a path prefix such as `substrate/<node-id>/sensor/**` or
`substrate/<mesh-id>/acl/**`), which is the same granularity ADR-057 gates
reads at and ADR-066 issues capability tokens for.

### Three signed payloads

All three are themselves ADR-063-signed envelopes (sealed by the sending
node's identity, dual-signed because they are cross-node per ADR-028):

1. **`SubstrateDigest { subtree, have }`** — the set of `EnvelopeId`s the
   sender currently holds for `subtree`, up to a bound. It is a compact "here
   is what I have"; it carries no values, so publishing a digest leaks only
   *which* writes exist (their content-addressed ids), gated by the same read
   ACL that governs the subtree.
2. **`SubstrateReconcile`** — the **convergence delta**: on receiving a peer's
   `SubstrateDigest`, a node computes the set of envelopes it holds for that
   subtree whose ids are **absent** from the peer's `have` set, and replicates
   exactly those back. This is a set difference, nothing more; it is inherently
   idempotent because the receiver applies each envelope by ADR-063
   `apply_signed` (duplicate ids are no-ops).
3. **`SubstrateSnapshot { subtree, envelopes }`** — a bootstrap payload for a
   node that holds *nothing* for a subtree (fresh join, post-wipe). Carries the
   full envelope set (metadata + values) so the joiner converges in one shot
   rather than by many reconcile rounds.

### Fail-closed batch verification (the load-bearing rule)

When ingesting **any** multi-envelope payload (`SubstrateReconcile` or
`SubstrateSnapshot`), the receiver MUST:

1. Verify the **outer** envelope signature of the sending node first. A forged
   or tampered outer envelope is rejected before anything inside is examined,
   and emits a `substrate.sync.bad_envelope` ExoChain event.
2. Verify **every inner envelope** — id-recompute + Ed25519 (+ ML-DSA
   cross-node) + ADR-063 path-ownership — **before applying any of them**. If a
   single inner envelope fails, the **entire batch is rejected** and nothing is
   stored. A snapshot with one forged delta is discarded wholesale.
3. Only after the whole batch verifies, apply each envelope idempotently via
   `apply_signed`, then emit one `substrate.sync.received` ExoChain event
   summarizing `{subtree, count}`.

Step 2 is the invariant this ADR exists to state: **partial application is
forbidden.** A peer cannot smuggle one bad write past us by burying it in a
thousand good ones, and cannot leave our tree half-updated by truncating the
stream mid-apply.

### Trust is egress-only; ingress always re-verifies

Reusing ADR-025's authenticated-but-untrusted-peer stance and the
`TrustLevel` ladder (`Unknown → Linked → Trusted`):

- **Egress** (what we *send*): only `Trusted` peers receive our digests,
  reconcile deltas, and snapshots. Trust governs disclosure and bandwidth, not
  correctness.
- **Ingress** (what we *accept*): **every** inbound envelope is re-verified
  regardless of the sender's trust level. A `Trusted` peer gets no verification
  discount — trust decides *whether we talk to them*, never *whether we believe
  them*. This is the single most important pattern imported from AgentBBS
  ADR-0007: the relayer's signature vouches only that they faithfully relayed;
  it never vouches for the authorship of the inner writes.
- **Peer discovery never grants trust.** A newly discovered peer (via SWIM
  membership, ADR-039, or explicit peer exchange) lands at `Unknown` and
  receives nothing until promoted. Promotion is a governance decision
  (ADR-066), not an automatic consequence of being reachable.

### Ordering and conflict resolution

`seq` on the ADR-063 envelope is a per-author monotonic ordering *aid*, not an
anti-replay window — idempotent content-addressed apply is what actually
defends against replay. When two authenticated writes from the *same owner*
target the same path, last-writer-wins by `(ts_ms, seq, id)` lexical tiebreak,
consistent with the LWW-CRDT discipline already used for the process table
(ADR-040). Cross-owner conflicts cannot occur for owned subtrees: ADR-063
path-ownership means only one identity may author `substrate/<owner>/**`.

### PII discipline on egress

A substrate subtree can contain sensor content an author never consented to
share cross-mesh. Before a digest/reconcile/snapshot leaves the node, free-form
value fields pass through the egress PII scrub already used elsewhere in the
stack (the AIDefence path), mirroring AgentBBS ADR-0007's `strip_pii` on
announce/snapshot egress. Content-addressed ids are computed over the
**pre-scrub** bytes so ids stay stable; scrubbing applies only to the
transmitted value, and a scrubbed value that changes the bytes is transmitted
as a distinct, separately-signed envelope authored by the relaying node (never
by forging the original author's signature over altered bytes).

## Consequences

### Positive

- Convergence without a trusted coordinator: any two nodes reconcile a subtree
  by exchanging id-sets and back-filling the difference, tolerant of
  partitions and reboots.
- A malicious or buggy peer cannot corrupt a receiver's tree: fail-closed batch
  verification rejects any batch containing a single bad envelope, and
  idempotent apply makes replays and reorderings harmless.
- Trust and correctness are cleanly separated — operators tune *who syncs with
  whom* (a bandwidth/disclosure knob) without ever weakening authentication.
- Composes with ADR-057: a digest only names ids for a subtree the requester is
  allowed to read, so anti-entropy cannot become a read-ACL bypass.
- Every sync outcome (bad envelope, batch reject, successful apply) is on the
  ExoChain (ADR-022), so divergence and attack attempts are auditable.

### Negative

- Digest exchange is O(writes-per-subtree) in the id-set size; a hot subtree
  (high-rate sensor stream) produces large digests. The bound + subtree
  granularity keep this tractable, but very chatty paths may need a windowed
  digest (recent-N ids) rather than a full one — flagged as an open question.
- Fail-closed batch rejection means one corrupt envelope wastes the whole
  snapshot transfer. Against a peer that is merely faulty (not malicious) this
  is pessimistic; the mitigation is smaller batches so a bad envelope voids
  less work, trading transfer count against blast radius.
- Full re-verification on ingest (id-recompute + dual-sig verify for every
  inner envelope) is CPU-heavy for large snapshots. Batch/parallel verification
  is required; a naive per-envelope serial verify is a performance bug on the
  join path.
- LWW conflict resolution can silently drop a concurrent write from the same
  owner (two daemons briefly sharing one edge node's key during a botched
  provisioning). This is an operator error surface, not a protocol bug, but it
  is invisible unless the ExoChain log is consulted.

### Neutral

- The protocol is defined over subtrees, not the whole tree; there is no global
  substrate root hash. This keeps reconciliation scoped and read-ACL-aligned,
  at the cost of no single "are these two nodes fully converged" answer — you
  ask per subtree.
- Anti-entropy is pull-shaped (a node offers a digest, the peer computes and
  returns the delta). A push-shaped variant (proactively replicate every new
  write to trusted peers, AgentBBS's `broadcast`) is compatible and may be
  layered on for low-latency subtrees; the digest path remains the
  convergence-of-last-resort.

## Alternatives considered

- **Trust-gated verification** (skip inner re-verification for `Trusted`
  peers). Tempting for performance, but it collapses the trust/correctness
  separation and means one compromised trusted peer poisons every downstream
  tree. Rejected outright — this is exactly the shortcut AgentBBS ADR-0007
  refuses.
- **Best-effort partial apply** (store the valid envelopes in a batch, drop the
  bad ones). Faster and more forgiving, but leaves the receiver in a state no
  single author ever signed, and lets an attacker shape a peer's tree by
  choosing which envelopes to corrupt. Rejected in favor of all-or-nothing
  batch semantics.
- **Whole-tree Merkle sync** (exchange a Merkle root, walk down to divergent
  leaves). Bandwidth-optimal for large mostly-converged trees, but heavier to
  implement, and the tree shape (a flat `BTreeMap`, `snapshot.rs`) does not
  give a natural Merkle structure without imposing one. Deferred as a possible
  successor for very large subtrees; the id-set digest is simpler and adequate
  for the near-term mesh size.
- **Vector clocks for ordering** instead of LWW-by-timestamp. More precise
  causal ordering, but heavier state per path and unnecessary given
  path-ownership makes cross-author conflict impossible for owned subtrees.
  Rejected in favor of reusing the ADR-040 LWW discipline.

## References

- **AgentBBS** (github.com/ruvnet/AgentBBS) — **FSL-1.1-MIT** licensed (from
  late.sh), *not* MIT/Apache. Read for design patterns; no code copied. The
  digest/reconcile/snapshot shape and the fail-closed batch rule were
  reimplemented from the design described in:
  - ADR-0007 (Zero-Trust Federation) — signed envelopes, egress-only
    `TrustLevel`, "ingress always re-verifies regardless of trust," idempotent
    replay via content-addressed ids, `strip_pii` on egress. The observable
    anti-entropy shape (a `BoardDigest` "have" set, a reconcile that returns the
    convergence delta, and a snapshot that "verifies EVERY contained message
    before storing any, so a snapshot with one forged post is rejected
    wholesale") is the direct model for this ADR's fail-closed rule.
  - ADR-0026 (Capability Gap / peer discovery) and ADR-0043 (Web-of-Trust) —
    "discovery never grants trust; new nodes land at Unknown," and rooted,
    depth-bounded trust promotion.
- WeftOS internal: ADR-063, ADR-025, ADR-028, ADR-057, ADR-022, ADR-039,
  ADR-040; `crates/clawft-substrate/src/mesh.rs`, `snapshot.rs`;
  `.planning/sensors/JOURNALED-NODE-ESP32.md` §8.
