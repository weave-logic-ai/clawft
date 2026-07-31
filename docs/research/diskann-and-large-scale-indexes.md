# DiskANN and large-scale indexes — how they alleviate (and don’t replace) batch graphs

**Status:** Living research note (companion to **ADR-095**)  
**Date:** 2026-07-31  
**Audience:** vector-backend, ECC memory, sensor fusion, world-model  
**In-tree status:** DiskANN wired via `ruvector-diskann`; **HNSW primary** for live
ECC; DiskANN = deferred cold / static tier until build economics + bugfixes
(see bench note).

---

## 1. Problem DiskANN solves

**Approximate nearest neighbor (ANN)** over high-dimensional embeddings when:

- the vector set is large (millions → billions of points in the literature),
- full RAM residency of all vectors + a rich in-memory graph is expensive,
- query latency must stay interactive (sub-ms to low-ms class on good hardware).

DiskANN (Subramanya et al., NeurIPS 2019) stores a **search graph + compressed
vectors on SSD**, with careful beam search and product quantization (PQ) so
most distance work hits sequential/disk-friendly layout rather than requiring
the full float32 corpus in RAM.

This is the right tool when the question is:

> “Which stored embeddings are nearest to this query vector?”

It is **not** the right tool when the question is:

> “Which nodes share a connected component after multi-hop identity edges?”  
> “What is the global PageRank of this association graph?”

Those are **batch graph analytics** (ADR-095 / `batch-graph-analytics-disk-spill.md`).

---

## 2. How DiskANN works (operator-level)

### 2.1 Vamana graph

DiskANN’s classic index is a directed graph over points (often called the
**Vamana** graph construction):

1. Start from a (possibly random) graph over all points.
2. For each node, run a **greedy / beam search** toward the node’s own vector
   (or a sample of targets) to discover useful long- and short-range neighbors.
3. **Robust prune** edges so out-degree stays bounded while preserving
   navigability (alpha parameter controls aggressiveness).
4. Optionally two passes (α = 1 then α > 1) for quality.

Search: beam search from one or more entry points, expanding neighbors,
ranking by distance (full precision or PQ-approximated), until the beam
stabilizes.

### 2.2 Product quantization (PQ)

Vectors are split into subspaces; each subspace is coded with a small codebook.
Distance estimates use asymmetric distance computation (ADC) so the bulk of the
index can stay compressed on disk / in limited RAM, with full precision
reranking of a short candidate list.

### 2.3 mmap / SSD layout

The on-disk layout aims for **localized** neighbor fetches so SSD sequential
bandwidth dominates. Query threads pin working sets; the rest stays cold.

### 2.4 What “graph” means here (vocabulary trap)

| Term | Meaning |
|------|---------|
| **Vamana / DiskANN graph** | Index structure over **vectors** for ANN navigation |
| **Application graph** (graphify, causal, association) | Domain edges (calls, caused-by, co-observed, same-id) |
| **BVH** | Spatial hierarchy over AABBs — geometry, not feature ANN |

Never say “we have DiskANN so large-scale graph analytics are done.”

---

## 3. WeftOS in-tree posture

### 3.1 Wiring

- Workspace dep: `ruvector-diskann` (see root `Cargo.toml`).
- Kernel / ECC: `vector_diskann` path; Hybrid hot HNSW + cold DiskANN design.
- Context router embedding index: `DiskAnnEmbeddingIndex` when
  `embedding-router` feature is on (`clawft-core` embedding module).
- Feature flag: `diskann` (implies `ecc`); without it, hybrid/diskann config
  degrades to stub / brute-force with warnings (`docs/development/feature-flags.md`).

### 3.2 Benchmark verdict (2026-07, WEFT-366)

Source: `docs/brain/vector-backend-bench-2026-07.md`.

| Finding | Implication |
|---------|-------------|
| HNSW strong on streaming ECC inserts | **HNSW stays live/primary** |
| DiskANN query recall excellent (~0.994 @10K) | Cold tier quality is real |
| Vamana `build()` serial / very expensive | Unfit for continuous streaming rebuilds today |
| WEFT-660 search id bug; WEFT-661 hybrid metric mix | Hybrid cold tier blocked until fixed |
| 100K+ build time unacceptable on serial path | Revisit when parallel/incremental build ships |

**Operational rule:** DiskANN alleviates **cold, large, relatively static** vector
corpora (nightly snapshots, promoted long-term memories, large sensor feature
archives). It does **not** replace incremental HNSW for turn-by-turn indexing.

### 3.3 Hybrid backend (intent)

```text
Hot:  HNSW  — recent / frequently accessed embeddings (RAM)
Cold: DiskANN — aged / bulk archive (SSD + PQ)
Search: merge results (metric normalization required — WEFT-661)
Promotion: access counts / LRU between tiers
```

When hybrid is correct and fixed, **most** “the sensor corpus is huge” pressure
on **similarity search** is handled without batch WCC.

---

## 4. How DiskANN alleviates large-scale *pressure* (degree of relief)

### 4.1 What it buys

| Scale pain | DiskANN / Hybrid help? | How |
|------------|------------------------|-----|
| Too many embeddings for pure HNSW RAM | **Yes** | Cold tier on disk + PQ |
| ANN latency under large N | **Yes** (query path) | Beam search + PQ; measured competitive p50 |
| Continuous insert of every frame embedding into one RAM graph | **Partial** | Keep hot HNSW small; batch-build DiskANN offline |
| Need top-K similar objects for a leaf / frame | **Yes** | Feature-first path (ADR-093) via VectorRef |
| “Is this the same object as last week?” via embedding only | **Partial** | ANN candidates; may still need temporal/spatial filters |

### 4.2 What it does **not** buy

| Scale pain | DiskANN help? | Need instead |
|------------|---------------|--------------|
| Transitive identity over explicit multi-hop links | **No** | Batch **WCC** (ADR-095) |
| Global influence / hub detection | **No** | Batch **PageRank** |
| Community structure over association edges | **No** | Hot communities until cliff, then batch |
| Geometric “what is in this volume?” | **No** | BVH (ADR-056) |
| Call-graph pipeline retrieval | **No** | ADR-084 BFS |

### 4.3 Composition pattern (recommended mental model)

```text
Sensor observations
    │
    ├─► embeddings ──► HNSW (hot) / DiskANN (cold) ──► ANN candidates
    │                                                      │
    ├─► spatial leaves ──► BVH ──► spatial candidates      │
    │                         │                            │
    │                         └──── VectorRef join (093) ──┘
    │                                      │
    └─► explicit association edges ──► [if huge] batch WCC / PageRank
                                              │
                                              ▼
                                    component_id / rank write-back
```

DiskANN shrinks the **candidate generation** problem. Batch analytics (when
activated) solves **global structure** over edges that ANN alone cannot invent
(or should not invent without provenance).

**Graph Views:** ANN soft-edges and `VectorRef` columns attach to a **named
View** (purpose-built multi-source graph — `docs/research/graph-views.md`).
The View’s hard association edges remain structural; DiskANN does not own
View identity.

---

## 5. Sensor fusion relevance

Sensor plans that grow **vector** volume (DiskANN-relevant):

- Multi-cam / free-form quilt embeddings and appearance codes.
- Per-frame / per-track features (SkyGraph-class track embeddings are an
  external analogue in rUv SkyGraph).
- Dual-branch sensor models (ADR-087) producing node embeddings over time.

Sensor plans that grow **edge** volume (batch-graph-relevant):

- Co-observation / co-track / re-id edges across devices and sessions.
- Multi-site Urth LOD association graphs.
- Mesh federated “same object” links with transitive closure needs.

**Practical guidance for planners:**

1. Prefer **feature ANN + spatial join** (HNSW/DiskANN + BVH + VectorRef) as
   long as identity can be approximated by “near in space and similar in
   feature.”
2. When product requires **hard transitive components** (legal/forensic
   identity, multi-hop same-as edges, global dedup), budget for **batch WCC**
   under ADR-095 activation criteria — do not stretch DiskANN into a fake CC.
3. Keep Vamana build **offline** relative to capture pipelines; never block
   live capture on serial full rebuilds (matches current bench verdict).

---

## 6. Related documents

| Doc | Role |
|-----|------|
| ADR-095 | Decision: two-plane graph doctrine + DiskANN as vector relief |
| `docs/research/batch-graph-analytics-disk-spill.md` | Join-agg / DataFusion recipe |
| `docs/brain/vector-backend-bench-2026-07.md` | Measured DiskANN vs HNSW |
| ADR-011 | HNSW sufficient at small scale; FrankenSearch deferred |
| ADR-088 / ADR-093 | VectorRef + dual-index join |
| ADR-056 / ADR-078 | Spatial world model growth path |
| ADR-087 | Learned sensor dual-branch (local neighborhoods) |
| CHANGELOG DiskANN / Hybrid bullets | Shipped capability summary |

---

## 7. Open watches (vector side)

- Upstream `ruvector-diskann`: parallel and/or incremental Vamana build.
- WEFT-660: search result id correctness.
- WEFT-661: hybrid merge metric normalization.
- Re-run 100K/1M bench ladder when revisit conditions fire.

---

## 8. Document history

| Date | Change |
|------|--------|
| 2026-07-31 | Initial note linking DiskANN mechanics to ADR-095 batch plane and sensor scale |
