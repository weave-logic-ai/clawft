# ADR-085: Entity Deduplication via HNSW Pre-filter (CodaRAG)

- **Status**: Proposed (Candidate)
- **Closes / tracks**: WEFT-372 (candidate write-up); implementation not yet scheduled
- **Date**: 2026-07-31
- **Deciders**: Pending (graphify / kernel HNSW maintainers)
- **Historical alias**: Phase-2 paper survey listed this as **“ADR-051”**. Numbers
  **050–053 were already claimed**. Next free indices start at **084**. This
  document is the canonical candidate for survey item 051.
- **Related**: ADR-011 (raw HNSW), ADR-059 (embedding provider), ADR-082
  (graphify port), ADR-084 (dependency-graph retrieval — benefits from cleaner graphs)
- **Source**:
  - `.planning/development_notes/knowledge-graph-paper-survey-phase2.md` (Paper 5: CodaRAG)
  - ArXiv 2604.10426 — *CodaRAG: Associative Retrieval via Complementary Learning Systems*
  - `crates/clawft-graphify/entity.rs`, `cluster.rs`
  - `crates/clawft-kernel/hnsw_service.rs`, `democritus.rs`

## Context

Graphify inserts entities from many files and extractors. Entities that are
**the same real-world concept** under different surface forms (e.g. `Google` vs
`Google Inc.` vs module path aliases, or the same class name in re-exports) often
become **separate nodes** unless cross-file resolution happens to unify them.

CodaRAG (ArXiv 2604.10426) describes a Complementary Learning Systems-style RAG
pipeline. Three pieces matter for WeftOS; this ADR scopes the first:

1. **Fragmented entity merging** — embedding-similarity gate, then high-confidence
   pairs to a judge (LLM in the paper).
2. Three-pathway associative navigation (semantic / Personalized PageRank /
   Fast Random Projection) — **out of scope for v0** of this ADR; follow-up.
3. Interference elimination (LLM filter) — **rejected for cognitive tick** (too
   slow); EML-based filter deferred.

Phase-2 survey: entity dedup is **P1**, effort **S**, high impact (estimated
10–30% graph size reduction on real codebases). PPR / FRP remain P1 design work
but are not required to accept the dedup gate.

## Decision (Proposed)

### 1. HNSW near-duplicate pre-filter on entity insert

**Propose** that before committing a new graphify `Entity` (standalone or via
`GraphifyBridge` when HNSW is available):

1. Embed the entity’s canonical text (name + type + optional signature / path).
2. Query HNSW (or bridge-backed multi-key index) for nearest neighbors.
3. If best cosine ≥ **threshold T** (survey suggests ~**0.92**; tunable), treat
   as **merge candidate** rather than blind insert.

### 2. Merge policy (v0)

| Confidence band | Action |
|-----------------|--------|
| cosine ≥ T_high (e.g. 0.97) + same `EntityType` | Auto-merge / attach alias; no LLM |
| T ≤ cosine < T_high | Record candidate edge or queue for offline review; default **no auto-merge** in CI |
| cosine < T | Insert new entity |

**v0 prefers precision over recall**: false merges corrupt EntityId stability and
downstream causal links. Prefer aliases / `same_as` relationships over rewriting
BLAKE3 `EntityId`s (ADR-082 identity model).

### 3. Standalone vs kernel-bridged

| Mode | Behavior |
|------|----------|
| Standalone graphify | Optional in-process mini-index or skip if no embedder; document flag |
| `kernel-bridge` | Use existing `HnswService` multi-key indexing (preferred path) |

Do not force a `clawft-kernel` dependency into default graphify features.

### 4. Explicit non-goals (v0)

- **No** LLM judge in the hot path (CodaRAG stage 2) for DEMOCRITUS / tick.
- **No** mandatory Personalized PageRank or FRP structural embeddings in this ADR
  (survey follow-ups; may become ADR-08x later).
- **No** change to frozen entity type discriminants.

### 5. Observability

Emit merge / skip / candidate counts suitable for ExoChain or local metrics so
threshold tuning is data-driven.

## Consequences

### Positive

- Smaller, cleaner graphs → better community detection, god-node scores, and
  ADR-084 pipeline retrieval.
- Reuses HNSW + embedder stack (ADR-011, ADR-059); small code surface.
- Quick win relative to multi-hop / temporal ECC work.

### Negative / risks

- Aggressive thresholds merge distinct types with similar names (e.g. `Config`
  in two crates).
- Embedding drift across model versions can change merge behavior; pin model +
  document reindex.
- Standalone mode without HNSW gets weaker dedup unless a local index is added.

### Follow-ups

- Threshold calibration on fixture + real monorepos.
- Optional offline LLM merge review batch (not tick-path).
- PPR as secondary DEMOCRITUS ranking signal (survey CodaRAG pathway 2).
- FRP / structural embeddings vs `hnsw_eml` distance model (larger design).

## Alternatives considered

| Alternative | Why not (for now) |
|-------------|-------------------|
| Name-only string equality | Misses aliases and near-duplicates CodaRAG targets |
| Always-LLM merge judge | Latency / cost; survey rejects for tick loop |
| Label-propagation only | Needs edges; isolates stay fragmented |

## Open questions

1. Default T / T_high values and whether they are config vs compile-time constants.
2. Alias representation: new relationship type vs entity metadata list.
3. Interaction with `EntityId` when two already-persisted graphs are merged offline.
