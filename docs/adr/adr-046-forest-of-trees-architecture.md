# ADR-046: Forest of Trees Architecture (Polyglot Tree Ensemble)

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: ECC Symposium (D2, D7), ClawStage Architecture Heritage

## Context

The Cognitive Model Visualization Graph (CMVG) is the core data architecture of the WeftOS cognitive platform. The design question was whether CMVG should be implemented as a single unified graph (one monolithic DAG containing all domain data) or as a collection of domain-specific structures.

The ECC Symposium (Q2) investigated this question by examining ClawStage, the conversational AI engine from which CMVG was extracted. ClawStage implements a **polyglot tree ensemble** -- five specialized engines (DCTE, DSTE, RSTE, EMOT, SCEN) with distinct data structures, bound together by cross-engine references, impulses, a shared HNSW embedding space, and a linear witness chain. The ClawStage architecture explicitly rejects a single monolithic DAG in favor of multiple domain-specific structures for four reasons: separation of concerns (different domains need different data structures), independent evolution (engines can be replaced without touching others), targeted optimization (different access patterns per domain), and composability (new engines added without modifying existing ones).

The ECC Symposium (D7) further established that each tree defines its own N-dimensional scoring (EffectVector, NodeScoring, domain-specific vectors), while what is uniform is the CrossRef indexing system -- Universal Node IDs enable linking across any pair of structures regardless of their internal dimensionality.

## Decision

The CMVG is implemented as a forest of domain-specific structures, not one unified graph. Each structure uses data structures appropriate to its domain:

| Structure | Module | Data Structure | Role |
|-----------|--------|---------------|------|
| ExoChain | `chain.rs` | Append-only hash-linked log | Witness chain, tamper-evident audit trail (stays linear) |
| Resource Tree | `tree_manager.rs` | Hierarchical tree with Merkle proofs | Namespace management, resource organization |
| Causal Graph | `causal.rs` | Typed weighted DAG (`DashMap`-backed) | Causal reasoning with 8 edge types (Causes, Inhibits, Correlates, Enables, Follows, Contradicts, TriggeredBy, EvidenceFor) |
| HNSW Index | `hnsw_service.rs` | Hierarchical Navigable Small World graph | Vector similarity search (wraps `clawft_core::HnswStore`) |

The structures are linked by two mechanisms:

**CrossRefs** (`crossref.rs`): Typed directed edges between nodes in any two structures, indexed by `UniversalNodeId` (BLAKE3 hashes). The `CrossRefStore` maintains concurrent forward and reverse indexes via `DashMap`. `StructureTag` (ExoChain=0x01, ResourceTree=0x02, CausalGraph=0x03, HnswIndex=0x04, Custom=0x10+) identifies which forest structure a node belongs to. CrossRefs enable "grafting" (linking) and "shaking" (pruning) across structures.

**Impulses** (`impulse.rs`): Ephemeral causal events that flow between structures, processed in HLC (Hybrid Logical Clock) order via the `ImpulseQueue`. Five built-in impulse types: `BeliefUpdate` (causal -> hnsw), `CoherenceAlert` (spectral -> causal), `NoveltyDetected` (hnsw -> causal), `EdgeConfirmed` (cloud -> edge), `EmbeddingRefined` (cloud -> edge), plus `Custom(u8)` for extensions.

ExoChain stays linear as the witness chain. It does NOT become a DAG. Causal DAG semantics live in the CausalGraph structure. This separation ensures the witness chain's append-only integrity guarantee is not compromised by causal graph operations.

Adding a new domain tree means: (1) implementing the tree's internal data structure, (2) defining a `StructureTag` variant, (3) implementing CrossRef linkage to existing structures, (4) defining Impulse types for inter-tree communication.

## Consequences

### Positive
- Each structure uses optimal data structures for its domain (append-only log vs. DAG vs. HNSW vs. hierarchical tree)
- Structures can be independently optimized, tested, and evolved without cascading changes
- The CrossRef indexing system provides uniform cross-structure queries regardless of internal dimensionality
- New domain trees can be added without modifying existing structures (only CrossRef and Impulse integration needed)
- ExoChain's append-only integrity guarantee is preserved by keeping it linear and separate from the CausalGraph

### Negative
- Queries spanning multiple structures require CrossRef traversal, which is more complex than querying a single unified graph
- Synchronization in multi-node deployments is per-structure: each tree syncs independently (chain sync, tree sync, CRDT gossip), increasing protocol complexity
- The CrossRef store is an additional data structure that must be maintained, persisted, and synchronized
- Consistency across structures is eventual, not transactional: a CrossRef may reference a node that has not yet been replicated to the local node

### Neutral
- The `ImpulseQueue` is ephemeral by design -- impulses are processed and discarded, not persisted, reducing storage burden but losing inter-tree communication history
- Per-tree scoring (D7) means governance rules must be aware of which structure's scoring dimensions apply to a given operation
- The forest architecture mirrors ClawStage's proven 5-engine model, providing confidence from production heritage but also inheriting its complexity
