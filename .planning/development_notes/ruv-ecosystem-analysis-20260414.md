# RuVector Ecosystem Analysis -- 2026-04-14

## Workspace Version

All crates at **v2.1.0** (workspace-level version). Our dependency crates
(ruvector-cluster, ruvector-raft, ruvector-replication, ruvector-diskann) all
use `version.workspace = true`, so they track this version.

## Recent Activity (last 20 commits)

Focus areas in the last month:

1. **mcp-brain performance** (ADR-149): SIMD cosine search, quality-gated
   search, batch graph rebuild, incremental LoRA. Pre-normalized embeddings +
   gzip compression.
2. **Brain hypothesis engine** (ADR-148): Autonomous discovery pipeline with
   Gemini 2.5, DiskANN-backed quality scoring, PageRank via forward-push,
   meta-mincut noise filtering.
3. **Boundary-first detection**: 17 experiments across 11 domains proving
   boundary-first detection (research PR #347).
4. **KV-cache compression**: TriAttention + TurboQuant stacked compression
   analysis (ADR-147).
5. **Musica**: Structure-first audio separation via dynamic mincut (feat PR #337).
6. **DiskANN matured**: ADR-144 design doc finalized; Vamana ANN + PQ + NAPI
   bindings shipped with 14 tests, 1.0 recall, 90us search.

## Our 4 Dependency Crates

### ruvector-cluster (v2.1.0)

- **Status**: Stable, no breaking changes since v2.0.
- **API**: `ClusterManager`, `ConsistentHashRing`, `ShardRouter`,
  `DagConsensus`, `GossipDiscovery`, `StaticDiscovery`.
- **Config defaults**: replication_factor=3, shard_count=64, heartbeat=5s,
  node_timeout=30s, min_quorum_size=2.
- **No new features** since our last integration. The code is clean and mature.

### ruvector-raft (v2.1.0)

- **Status**: Stable, no recent commits touching this crate.
- **No breaking changes**.

### ruvector-replication (v2.1.0)

- **Status**: Stable. Minor README polish only.
- **No breaking changes**.

### ruvector-diskann (v2.1.0)

- **Status**: Recently matured (ADR-144).
- **Key features**: Vamana two-pass graph build, alpha-robust pruning,
  Product Quantization, mmap graph loading, SIMD distance (optional `simd`
  feature via simsimd), GPU stubs (optional `gpu` feature).
- **Optimizations**: FlatVectors (contiguous f32), VisitedSet (generation
  counter), 4-accumulator ILP, flat PQ distance table, parallel medoid via
  rayon, zero-copy save, mmap load.
- **Performance**: 1.0 recall, 90us search latency in benchmarks.
- **Breaking changes**: None from our perspective -- this crate is newly
  integrated and we are at the latest.

## New/Interesting Crates Since Last Check

### ruvector-hyperbolic-hnsw

Poincare ball embeddings integrated with HNSW. Hierarchy-aware vector search
with tangent-space pruning (cheap Euclidean distance in tangent space before
exact hyperbolic ranking). Per-shard curvature. Dual-space index with
Euclidean fallback.

**WeftOS relevance**: HIGH. Our knowledge graph has natural hierarchies
(entity types, file trees, community containment). Hyperbolic HNSW could
improve recall on deep-leaf entities without increasing memory. Relevant to
K0 (HNSW service) and the graphify analyzer.

### ruvector-solver

Iterative sparse linear solvers (CSR format). Solvers: Neumann series,
Conjugate Gradient, Forward-Push, Backward-Push, Hybrid Random Walk, BMSSP.
SIMD-accelerated. Budget-bounded execution.

**WeftOS relevance**: HIGH. Our Lanczos iteration in `causal.rs` for
spectral analysis could delegate the tridiagonal eigensolver to this crate.
The Forward-Push solver is already used for PageRank in the brain's
hypothesis engine -- we could use it for our graphify surprise scoring.

### ruvector-mincut

Subpolynomial-time dynamic minimum cut with real-time monitoring.

**WeftOS relevance**: MEDIUM. Could replace or augment our spectral
partition (Fiedler vector bisection) with exact mincut for community
detection and noise filtering in graphify.

### ruvector-attention

Comprehensive attention mechanisms: scaled dot-product, multi-head, graph
attention, hyperbolic attention, sparse attention, FlashAttention-3, MLA
with KV-cache compression, Mamba SSM, speculative decoding. Includes
curvature-aware attention and MoE routing.

**WeftOS relevance**: LOW-MEDIUM. Not directly applicable to current WeftOS
math, but graph attention could improve our causal graph traversal if we
move to learned scoring.

### ruvector-gnn

Graph Neural Networks: tensor ops, GNN layers, compression, differentiable
search.

**WeftOS relevance**: MEDIUM. Could be used for learned community detection
or surprise scoring in graphify, replacing our hand-crafted heuristics.

### mcp-brain / mcp-brain-server

MCP server for shared brain. RVF cognitive containers, witness chains,
Ed25519 signatures, differential privacy. The hypothesis engine (ADR-148)
adds: hypothesis generation, DiskANN-backed quality scoring, PageRank via
forward-push, meta-mincut noise filtering.

**WeftOS relevance**: REFERENCE. The hypothesis engine pattern (cross-domain
hypothesis generation + quality scoring + noise filtering) maps well onto
graphify's analysis pipeline.

## Recent ADRs of Interest

| ADR | Title | Relevance |
|-----|-------|-----------|
| ADR-144 | DiskANN/Vamana implementation | Direct -- we use this crate |
| ADR-147 | TriAttention + TurboQuant stacked KV-cache compression | Reference for attention optimization |
| ADR-148 | Brain hypothesis engine | Pattern for graphify enhancement |
| ADR-149 | Brain SIMD + quality gate + batch graph + incremental LoRA | Pattern for HNSW service optimization |

## Upgrade Recommendations

1. **No breaking changes** -- safe to stay on v2.1.0 for all four crates.
2. **Consider adopting ruvector-solver** for spectral analysis eigensolvers
   and PageRank computation in graphify.
3. **Evaluate ruvector-hyperbolic-hnsw** for hierarchy-aware search in the
   knowledge graph.
4. **Monitor ruvector-mincut** for potential adoption in graphify community
   detection / noise filtering.
5. **Apply ADR-149 pattern** (SIMD cosine, quality gating) to our own
   HnswService -- we currently use HnswStore's built-in distance which may
   not be SIMD-accelerated.
