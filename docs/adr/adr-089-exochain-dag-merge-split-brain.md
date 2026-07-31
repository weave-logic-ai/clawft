# ADR-089: ExoChain DAG merge strategy and split-brain handling

**Date**: 2026-07-31  
**Status**: Accepted  
**Deciders**: 0.8 decision batch (WEFT-109)  
**Depends-On**: ADR-022 (mandatory audit), ADR-033 (three-branch governance),
ADR-040 (LWW-CRDT process table), ADR-041 (ChainAnchor), ADR-043 (BLAKE3),
ADR-064 (substrate anti-entropy)  
**Closes**: WEFT-109  
**Unblocks**: K6.4 chain replay / merge implementation (audit row #16)

## Context

K5 symposium open questions Q1 and Q5 blocked K6.4:

- **Q1**: Should chain merge use leader-based consensus or a DAG?
- **Q5**: How to handle split-brain when the cluster partitions?

Related posture already in tree:

- Governance genesis hash is cluster identity; foreign
  `governance.root.supersede` is rejected (ADR-033).
- Process table uses LWW-CRDT, not Paxos (ADR-040).
- Substrate anti-entropy uses content-addressed idempotent apply
  (ADR-064), not leader lease.
- Hash migration path points at `exo-dag::DagStore` for K6 (ADR-043).

## Decision

### Merge strategy: **DAG, not leader**

ExoChain merge and cross-node chain reconciliation use a **content-addressed
DAG** with multi-parent merge commits:

1. Each node appends locally under its node identity (Ed25519, ADR-025).
2. On reconnect / anti-entropy, peers exchange missing segments by hash.
3. Concurrent tips are retained; a merge node records both parents when
   a linear presentation is required (CLI replay, audit export).
4. **No single leader** is elected for chain append rights. Leader-based
   consensus is rejected for K6.4 scope (operational complexity, split-brain
   fencing, conflict with multi-transport mesh including browser nodes).

Ordering for audit presentation uses happens-before where available
(HLC / sequence numbers) and stable hash order as tie-break — same family
of rules as substrate signed envelopes (ADR-063/064).

### Split-brain handling

| Condition | Behavior |
|-----------|----------|
| Same genesis, temporary partition | Both sides continue append-only locally; on heal, DAG merge + anti-entropy (availability first). |
| Different genesis lineage | Reject foreign governance root; **halt synchronization** with that peer (ADR-033). No silent merge across constitutions. |
| Conflicting process-table entries | LWW-CRDT (ADR-040), not chain leader. |
| Conflicting substrate objects | Content-addressed idempotent apply (ADR-064). |

Detection:

- Compare `governance.genesis` (first 16 hex) on join / handshake.
- SWIM failure detection (ADR-039) signals partition; it does **not**
  trigger a leader election for the chain.

Recovery:

1. Re-establish mesh membership after partition heals.
2. Run chain anti-entropy (hash inventory → fetch missing → link tips).
3. Emit ExoChain audit event for merge completion (ChainLoggable, ADR-020).

## Consequences

### Positive

- Aligns chain story with exo-dag, CRDT process table, and substrate
  anti-entropy — one consistency model family.
- Browser / flaky edge nodes can append offline and merge later.
- Avoids Raft/Paxos operational surface for 0.8–0.9 mesh.

### Negative

- Audit UI must present DAG tips, not a single global height.
- Application-level conflict resolution may still be needed for
  non-CRDT domain state (out of scope here).

### Neutral

- Chain replay (audit row #16) must walk multi-parent graphs; implementers
  of WEFT-105 / K6.4 must reference this ADR.
- Full libp2p or external consensus remains optional later for *other*
  services; it is not required for ExoChain merge.

## Implementation notes (non-normative)

- Prefer BLAKE3 content ids (ADR-043 migration) for segment addressing.
- Wire format stays CBOR / rvf-wire (ADR-030/031).
- Tests: two-node concurrent append → heal → both tips visible → merge
  parent links; foreign-genesis peer rejected.
