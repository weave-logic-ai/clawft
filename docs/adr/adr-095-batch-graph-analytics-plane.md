# ADR-095: Batch graph analytics plane (disk-spill join-agg) — research hold for sensor scale

- **Status**: Draft (Proposed — research hold; not scheduled for 0.8.x ship)
- **Date**: 2026-07-31
- **Deciders**: Pending (ECC / graphify / sensor-fusion / vector-backend maintainers)
- **Tracks**: Research only until activation criteria (below). No WEFT ship ticket required for the hold itself.
- **Related**:
  - ADR-011 (raw HNSW; FrankenSearch deferred)
  - ADR-056 (BVH spatial index)
  - ADR-062 (ECC graph-walk conversation)
  - ADR-077 / ADR-078 (splat capture → structured world model)
  - ADR-079 (Urth multi-scale twin vision)
  - ADR-082 (graphify Rust port)
  - ADR-084 (dependency-graph retrieval — *hot* structural path)
  - ADR-085 (entity dedup HNSW pre-filter)
  - ADR-087 (spatio-temporal dual-branch sensors — K-STEMIT)
  - ADR-088 / ADR-093 (VectorRef + BVH×HNSW join)
  - ADR-091 (LightRAG dual-level retrieval)
  - ADR-046 (forest of trees — durable **sources**)
  - ADR-058 (`SessionView` — session-scoped projection)
  - ADR-067 (conversation graph **UI** — renders a graph; not the same as a Graph View object)
  - ADR-069 (atom / panopticon locators across projections)
- **Source**:
  - [Sinchenko 2026 — DataFusion billion-edge graphs on 10GB RAM](https://semyonsinchenko.github.io/ssinchenko/post/datafusion-graphs-cc-2/)
  - [graphframes-rs](https://github.com/SemyonSinchenko/graphframes-rs) (Pregel / WCC / PageRank as join+agg)
  - Bögeholz et al., *In-database connected component analysis* (arXiv:1802.09478)
  - DiskANN (Subramanya et al., NeurIPS 2019) + in-tree `ruvector-diskann` / Hybrid backend
  - `docs/research/batch-graph-analytics-disk-spill.md`
  - `docs/research/diskann-and-large-scale-indexes.md`
  - `docs/research/graph-views.md` (**Graph Views** — purpose-built multi-source graphs)
  - `docs/brain/vector-backend-bench-2026-07.md` (WEFT-366 DiskANN verdict)

## Context

WeftOS already has several **graph-shaped** subsystems:

| Surface | Role today | Typical scale |
|---------|------------|---------------|
| `clawft-graphify` + petgraph | Code / forensic knowledge graphs, community detection, god-nodes | 10³–10⁶ edges (repo-sized) |
| ECC `CausalGraph` + HNSW | Conversation / memory causal walk, semantic seed | Streaming, tick-budgeted |
| AgentDB / RuVector graph modes | k-hop, semantic, small PageRank-mode ranking | Interactive, complexity budgets |
| BVH + VectorRef (ADR-088/093) | Spatial containment × feature join | Spatial objects / events |
| Sensor & fusion plans (ADR-077/078/087, Urth, sonobuoy, multi-cam splat) | Dense observations → structure leaves, proximity graphs, fusion | **Projected** growth to very large edge tables |

**Hot-path** algorithms (k-hop, BFS pipeline retrieval ADR-084, local CC on modest causal graphs, geometric BVH queries) correctly assume the working set fits RAM and must respect the cognitive tick (ADR-047).

Sensor fusion and multi-scale twins will push **global** graph algorithms into a different regime:

- Weakly connected components over multi-sensor **entity / identity** graphs (same object across cameras, nodes, sessions).
- PageRank / centrality over huge **observation → object → agent** influence graphs.
- Community / component labels over monorepo *or* planetary-scale association graphs (Urth LOD, multi-site mesh).

Classical in-memory libraries (NetworkX-class, full petgraph materialization) fail when edges no longer fit RAM. Sinchenko’s DataFusion work shows that **PageRank and WCC can be expressed as bulk-synchronous join + aggregate over vertex/edge tables**, with DataFusion handling spill, sort-merge joins, and planning — billion-edge scale on a laptop with hard memory caps (5–10 GB).

Separately, **DiskANN** (and our Hybrid HNSW+DiskANN path) already addresses a *related* scale problem on the **vector** side: approximate nearest-neighbor search when the embedding corpus does not fit comfortably in RAM, via a disk-resident Vamana graph + product quantization. That **alleviates** the need to run full-graph analytics for pure “find similar embeddings” queries, but it does **not** replace WCC, global PageRank, or multi-hop structural analytics over edge tables.

Without an explicit two-plane doctrine, future sensor work will either (a) keep forcing petgraph until it breaks, or (b) prematurely vendor a heavy analytics stack into the 0.8 critical path.

## Decision (Draft)

### 1. Two-plane graph doctrine (normative research stance)

WeftOS **adopts** the following split as the long-horizon architecture for graph work. Implementation of the batch plane is **not** committed for 0.8.x.

```text
┌─ Hot plane (shipped / in-flight) ─────────────────────────────┐
│  petgraph · RuVector / AgentDB k-hop · ECC walk · BVH        │
│  ADR-084 BFS pipelines · HNSW / Hybrid DiskANN (feature ANN) │
│  complexityBudget / maxNodes / tick budget                    │
│  Latency: µs–ms · Working set: RAM-resident                   │
└───────────────────────────────┬───────────────────────────────┘
                                │ seeds, ranks, component labels
                                ▼
┌─ Batch plane (research hold — activate on criteria) ──────────┐
│  vertices + edges as columnar tables (Parquet/Arrow)          │
│  Pregel-style join + agg + materialize state (disk spill)     │
│  PageRank · WCC · global centrality · large CC                │
│  Optional engine: DataFusion / graphframes-rs class           │
│  Latency: minutes–hours · Memory-capped offline jobs          │
└───────────────────────────────────────────────────────────────┘
```

**Write-back contract:** batch jobs produce **node features** (e.g. `component_id`, `pagerank`, community label) that re-enter hot indexes as metadata / payload fields — never raw billion-edge dumps into LLM context or the cognitive tick.

### 1b. Graph Views as the unit of composition (research stance)

Product and sensor work will often need graphs that are **not** “the whole
forest” and **not** a single source structure. **Graph Views** (capital V —
see `docs/research/graph-views.md`) are the intended unit:

| Property | Meaning |
|----------|---------|
| **Purpose-built** | Created for a job (conversation, room identity, monorepo assessment, mesh health) |
| **Multi-source** | May attach forest trees, BVH regions, graphify KGs, sensor streams, peer Views, foreign edge lists |
| **Live or snapshot** | May subscribe to impulses/events or freeze for audit/export |
| **Plane-aware** | Small/live Views stay on the **hot** plane; large association Views export columnar edge tables to the **batch** plane |
| **Feature write-back** | Batch WCC/PageRank/community labels land on the **View** (and optionally promote into source metadata) |

**Normative research rules:**

1. Prefer running global algorithms (**WCC, PageRank**) on a **named View’s**
   edge table, not on an unbounded union of all forest edges.
2. Live sensor feeds attach to a View with **caps + windowing**; overflow
   spills to snapshot/batch rather than growing the cognitive hot set without bound.
3. Soft edges from ANN (HNSW/DiskANN) may feed a View as *candidate* edges;
   hard identity still needs structural/batch treatment when required.
4. **Do not** conflate Graph Views with GUI “views” (ADR-067 is a renderer of
   graph data). A Graph View is a **data object**; UI is a consumer.
5. Graph Views are **usually projections** over ADR-046 forest sources — not a
   replacement for durable domain trees — unless a View is later promoted to a
   first-class `StructureTag` (open question in the research note).

**0.8.x:** no mandatory `view.*` RPC surface. Capture the concept so sensor
fusion, Urth, and multi-source agent context designs share vocabulary.

### 2. Keep batch analytics *in view*; do not ship DataFusion in 0.8.x

- **Do not** add Apache DataFusion (or graphframes-rs) as a workspace dependency for the 0.8 publish track.
- **Do** retain design recipes, activation criteria, and related-work links in this ADR + companion research docs so sensor-fusion and Urth planning can cite them.
- Prefer **pattern transfer** (edge tables, spill, join-agg iterations, hard `MemoryMax`-style tests) even if a future implementation uses a different engine (DuckDB, custom Arrow pipelines, Spark only if truly multi-node).

### 3. DiskANN / Hybrid as the vector-side scale relief (already in-tree)

**Decision:** Treat DiskANN (and Hybrid hot-HNSW + cold-DiskANN) as the **first-line** mitigation when **embedding cardinality** grows with sensors — before any batch graph engine.

| Need | Prefer | Why |
|------|--------|-----|
| “Nearest embeddings to query / leaf feature” | HNSW (live) → DiskANN / Hybrid (cold / huge) | ANN on disk; no global graph algo |
| “Same entity across sensors (transitive IDs)” | Batch **WCC** when edge table is huge | Structural connectivity, not cosine |
| “Load-bearing hubs in association graph” | Batch **PageRank** offline | Global rank ≠ local k-NN |
| “What is near *here* and similar?” | BVH × VectorRef join (ADR-093) + ANN | Spatial-first; already designed |

DiskANN **alleviates** pressure to run batch analytics for pure similarity and keeps ECC streaming inserts viable (HNSW primary; DiskANN deferred cold tier — see bench verdict). It **does not** make WCC/PageRank free at billion-edge scale.

Details: `docs/research/diskann-and-large-scale-indexes.md`.

### 4. Activation criteria (when to schedule implementation)

Promote batch-plane work from research hold to a cycle ticket when **any** of:

1. **Measured cliff:** a production or soak **Graph View** (or raw graphify multi-repo, ExoChain association, multi-camera identity, sonobuoy proximity, Urth LOD association) exceeds a documented RAM budget for in-memory WCC/PageRank/community detection, **or**
2. **Sensor fusion product need:** identity resolution / global influence labels are required for world-model publish (ADR-078) and cannot be approximated by HNSW + local k-hop, **or**
3. **Ops request:** nightly / mesh-wide analytics job with hard memory caps on edge devices or CI agents, **or**
4. **Multi-source View product:** a shipping feature needs named, live, multi-source Graph Views with caps/export (implementation may start hot-only; batch activates when that View cliffs).

Until then: hot plane only; export edge tables opportunistically if cheap (e.g. Parquet dump from graphify or a purpose-scoped View for experiments).

### 5. Algorithm ownership (when activated)

| Algorithm | Plane | Notes |
|-----------|-------|--------|
| k-hop / BFS path retrieval | Hot | ADR-084, AgentDB traverse |
| Label-prop / SASE community (repo-scale) | Hot (graphify) | Until cliff |
| Spatial queries | Hot (BVH) | ADR-056 |
| Feature ANN | Hot (HNSW) / cold (DiskANN) | Not batch graph |
| Global PageRank | Batch | Pregel join+agg |
| Weakly connected components (entity resolution) | Batch | In-DB CC / contraction style |
| Full-graph spectral (Lanczos on huge L) | Batch or specialized | ADR-009 remains; may compose with batch edge tables |

### 6. Explicit non-goals (this draft)

- Not replacing petgraph, HNSW, or BVH for interactive paths.
- Not putting DataFusion (or any batch engine) on the cognitive tick.
- Not requiring billion-edge support before sensor pipelines generate that data.
- Not resolving WEFT-660/661 DiskANN correctness here (separate; still blockers for Hybrid cold tier).
- Not choosing Spark/GraphFrames-on-JVM as the default (Rust/local-first preference).

## Consequences

### Positive

- Sensor fusion and Urth-scale planning can **cite** a clear off-ramp before petgraph becomes a dead end.
- DiskANN / Hybrid remains the right **vector** investment; batch graph remains the right **structural** investment — no false dichotomy.
- 0.8 ship surface stays free of a large analytics dependency.
- Aligns with AgentDB `complexityBudget` culture: hot path always capped; batch path memory-capped offline.

### Negative / risks

- Research may go stale if not re-checked when fusion ships; mitigate via this ADR + research docs + activation criteria.
- FairSpillPool / SMJ limitations in DataFusion (upstream) may force a different engine later — recipe still holds.
- Operators may confuse DiskANN “graph” (Vamana ANN graph) with analytics graphs — research doc must keep vocabulary distinct.

### Neutral

- Companion research docs are the living detail; this ADR is the decision spine.
- Optional future: thin `GraphFrame { vertices, edges }` façade in-tree *without* DataFusion, for testable algorithm ports at medium scale.

## Alternatives considered

| Alternative | Why not (for now) |
|-------------|-------------------|
| Always scale RAM / single-node petgraph | Fails sensor + Urth projections; contradicts edge/laptop constraints |
| Ship DataFusion in 0.8 | No measured consumer; dependency + build cost; not on critical path |
| Spark / GraphFrames-on-JVM only | Heavy ops surface; conflicts with Rust daemon-first story |
| DiskANN alone for all large-scale graph needs | Solves ANN, not WCC / global rank / structural components |
| GNN message-passing only (K-STEMIT path) | Complementary (ADR-087); still needs neighborhood definition; not a spill WCC substitute |

## Open questions

1. First concrete edge-table export format (Parquet schema for graphify / ExoChain / WM association edges)?
2. Prefer DataFusion vs DuckDB vs custom Arrow when activation fires?
3. Where do batch job results land (leaf payload metadata, separate side table, chain event)?
4. Mesh-distributed batch (partition edges by region) vs single-node spill for Urth LOD?
5. Relationship to RuLake witness-anchored retrieval for *verified* batch outputs?

## Follow-ups

- [x] Research: `docs/research/batch-graph-analytics-disk-spill.md`
- [x] Research: `docs/research/diskann-and-large-scale-indexes.md` (how DiskANN alleviates vector scale)
- [x] Research: `docs/research/graph-views.md` (purpose-built multi-source / live Graph Views)
- [ ] When activation fires: Plane ticket + cycle assignment; prototype WCC or PageRank on a **named View** edge export under `MemoryMax`
- [ ] Keep WEFT-660/661 and DiskANN parallel-build watch as vector-side enablers (`docs/brain/vector-backend-bench-2026-07.md`)
- [ ] Cross-link from sensor fusion / world-model docs when those next revise
- [ ] If product commits to `view.*` APIs: promote Graph Views from research note to a dedicated ADR (or Accept §1b here)

## References

1. S. Sinchenko, *Algorithms on billion-scale graph using 10GB RAM: I love DataFusion!*, 2026.  
   <https://semyonsinchenko.github.io/ssinchenko/post/datafusion-graphs-cc-2/>
2. SemyonSinchenko/graphframes-rs — Pregel, PageRank, WCC implementations.  
   <https://github.com/SemyonSinchenko/graphframes-rs>
3. Bögeholz et al., *In-database connected component analysis*, arXiv:1802.09478.
4. Subramanya et al., *DiskANN: Fast Accurate Billion-point Nearest Neighbor Search on a Single Node*, NeurIPS 2019.
5. WeftOS vector backend bench & DiskANN deferral: `docs/brain/vector-backend-bench-2026-07.md`.
6. Stanford CME 323 Lecture 8 (Pregel dataflow) — linked from Sinchenko’s post.
