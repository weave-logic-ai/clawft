# Batch graph analytics (disk-spill join-agg) — research hold

**Status:** Living research note (companion to **ADR-095**)  
**Date:** 2026-07-31  
**Audience:** ECC, graphify, sensor fusion, world-model, vector-backend  
**Not a ship plan for 0.8.x** — activation criteria live in ADR-095.

---

## 1. Why this exists

Sensor fusion, multi-camera splat association, mesh multi-site identity, and
Urth-scale digital-twin plans will eventually emit **association graphs** that
do not fit comfortably in process RAM. Interactive WeftOS paths (petgraph,
AgentDB k-hop, BVH, HNSW) are the right tools for **local** structure and
**tick-budgeted** work. They are the wrong tools for **global** algorithms over
billion-class edge tables.

This note captures:

1. The external recipe that already works at that scale (DataFusion + Pregel-style
   join/agg).
2. How it maps onto WeftOS surfaces.
3. What **not** to do on the hot path.
4. Links to DiskANN (vector-side relief) and related ADRs.

**Primary external source:**  
[Semyon Sinchenko — DataFusion graphs, billion edges, 5–10 GB RAM](https://semyonsinchenko.github.io/ssinchenko/post/datafusion-graphs-cc-2/)  
Implementation: [graphframes-rs](https://github.com/SemyonSinchenko/graphframes-rs)

---

## 2. Core idea (recipe, not library lock-in)

### 2.1 Graph as tables

```rust
// Conceptual GraphFrames shape
struct GraphFrame {
    vertices: DataFrame, // id, + state columns
    edges: DataFrame,    // src, dst, + optional weight/type
}
```

Edges and vertices live as **columnar** datasets (Parquet on disk). Algorithms
must favor **bulk scans and joins**, not random adjacency-list pointer chasing.

### 2.2 Pregel as join + aggregate

Bulk-synchronous parallel (BSP) / Pregel supersteps become:

1. **Join** edges to current vertex state (message generation).
2. **Group-by + aggregate** messages per destination vertex.
3. **Join / update** vertex state.
4. **Materialize** state (and often edges) to disk to break lineage and bound memory.

PageRank is the textbook instance; WCC uses contraction iterations after
symmetrizing directed edges (union of `(src,dst)` and `(dst,src)` + distinct).

### 2.3 Measured scale (external, not WeftOS)

| Workload | Dataset | Edges | Memory cap | Notes |
|----------|---------|-------|------------|-------|
| PageRank | graph500-26 | ~1.05B | 5 GB | ~15 iters; SMJ or HJ; correctness vs ground truth |
| WCC | twitter_mpi | ~1.96B (prep ~3.2B after symmetrize) | 10 GB | Contraction collapses edges rapidly after early iters |

**Engine issues called out upstream:** `FairSpillPool` deadlocks under extreme
stress; SMJ may re-sort large edge sides each iteration; Parquet vs other formats
open. Treat as **known risks** if/when we adopt DataFusion specifically.

### 2.4 WCC as entity resolution

Sinchenko’s product framing matches sensor fusion: **transitive ID linking**
across systems is a weakly connected components problem. For WeftOS that maps to:

- Same physical object across phone / Pi / multi-cam rig / time.
- Same human across channels / mesh principals (related product: multi-user auth).
- Same room volume across rescans (world-model object stability, ADR-078).

---

## 3. Mapping onto WeftOS

### 3.1 Hot plane (do not replace)

| System | ADR / home | Use |
|--------|------------|-----|
| graphify petgraph | ADR-082, ADR-091 | Code/forensic KG, communities at repo scale |
| Dependency BFS retrieval | ADR-084 | Multi-step “how does X become Y” — local structure |
| ECC causal walk | ADR-062 | Conversation as graph walk |
| AgentDB k-hop / pagerank-mode | RuVector / ruflo ADR-130-class tools | Budgeted interactive graph query |
| BVH | ADR-056 | Geometry |
| BVH × HNSW join | ADR-088, ADR-093 | Spatial-first / feature-first composition |
| Dual-branch sensor GNN | ADR-087 | Learned fusion over **local** neighborhoods |

### 3.2 Batch plane (this research)

| Future consumer | Why batch |
|-----------------|-----------|
| Multi-sensor identity / association | Global WCC over huge observation graphs |
| World-model re-id across sites | Component labels as stable object clusters |
| Graphify multi-monorepo federation | God-node / PageRank offline when petgraph cliffs |
| ExoChain long-horizon influence | Offline rank for postmortems / GEPA context |
| Urth LOD association | Partitioned or spill WCC/PageRank per LOD layer |

**Write-back:** batch outputs become **features** on hot nodes/leaves
(`component_id`, `rank`, community), not full edge dumps into prompts.

### 3.3 Sensor fusion scale path (why we keep this in view)

Planned / accepted sensor-related work that **grows edges**:

- ADR-077 / ADR-078 — splat capture → structure leaves (objects, volumes, events).
- Multi-cam rig + free-form quilt — multi-evidence association over time.
- ADR-087 — proximity / topology graphs for array sensors (sonobuoy, etc.).
- ADR-079 Urth — multi-scale sparse twin; association across LODs and feeds.
- Mesh multi-node — events and objects federated across peers.

None of these require DataFusion **today**. All of them benefit from knowing the
off-ramp **before** petgraph is load-bearing at the wrong scale.

---

## 4. Related work (short bibliography)

| Work | Relation |
|------|----------|
| Sinchenko DataFusion posts + graphframes-rs | Primary recipe (join-agg Pregel, spill) |
| Spark GraphFrames | Same API mental model; JVM/cluster default |
| arXiv:1802.09478 (in-database CC) | WCC algorithm lineage used in graphframes-rs |
| Pregel (Malewicz et al.) / BSP notes | Superstep model |
| DiskANN (NeurIPS 2019) | Disk ANN — **vector** scale, not structural WCC |
| RuVector graph-node / AgentDB | Interactive graph DB + k-hop; complexity budgets |
| LightRAG / SGKR / CodaRAG (ADR-091/084/085) | Retrieval quality; still hot-plane |
| ADR-009 sparse Lanczos | Spectral analysis; may consume batch edge tables later |

---

## 5. DiskANN relationship (summary)

Full write-up: **`docs/research/diskann-and-large-scale-indexes.md`**.

- DiskANN builds a **Vamana graph over vectors** for disk-backed ANN.
- That graph is **not** the application association graph (camera↔object↔session).
- Use DiskANN/Hybrid when the pain is **embedding cardinality / RAM for ANN**.
- Use batch join-agg when the pain is **global connectivity or centrality** over
  explicit edges.

Both can compose: ANN proposes candidate edges (similar embeddings / tracks);
batch WCC merges them into identity components under memory caps.

---

## 6. Activation checklist (copy of ADR-095 spirit)

Schedule implementation work only when:

1. Measured in-memory cliff on a real or soak graph, **or**
2. Product need for identity/global labels that local k-hop + ANN cannot fake, **or**
3. Ops needs a memory-capped offline job.

Prototype gate when activated:

- [ ] Edge + vertex Parquet (or Arrow IPC) schema documented
- [ ] One algorithm (prefer WCC for identity, or PageRank for hubs) under hard
      memory limit (`systemd-run` / cgroup equivalent)
- [ ] Write-back path into leaf/metadata or side table
- [ ] No import of batch engine into tick-critical crates without feature gate

---

## 7. What not to do

- Do not run billion-edge PageRank on the cognitive tick.
- Do not dump full components into LLM context — top-K hubs + labels only.
- Do not treat “we have DiskANN” as “we solved large graphs.”
- Do not block 0.8 graphify/BVH/sensor MVP on this plane.

---

## 8. Document history

| Date | Change |
|------|--------|
| 2026-07-31 | Initial note from multi-expert review of Sinchenko post + WeftOS graph/sensor map; ADR-095 drafted |
