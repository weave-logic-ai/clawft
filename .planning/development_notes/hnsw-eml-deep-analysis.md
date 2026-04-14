# HNSW + EML Deep Analysis

**Date**: 2026-04-14
**Paper**: Odrzywolel 2026, "All elementary functions from a single operator" (arXiv:2603.21852v2)
**Key insight**: EML doesn't just approximate functions — it **discovers the closed-form** relationship from data, with weights that snap to {0,1}. The resulting formula is interpretable, not a black box.

---

## What the paper ACTUALLY gives us

The paper's contribution isn't "another function approximator" — neural nets already do that. The unique properties are:

1. **Weight snapping**: trained weights converge to exactly {0,1}, meaning the result IS the exact closed-form formula, not an approximation
2. **Interpretability**: you can read the formula and understand what it computes
3. **Composability**: EML trees compose — the output of one tree is a valid input to another
4. **All elementary functions**: exp, ln, sin, cos, sqrt, +, -, *, / are all expressible
5. **Small parameter count**: 34-50 params for useful functions (vs millions in neural nets)

This means: for any relationship in HNSW that IS an elementary function (or close to one), EML can discover the exact formula. Then you hardcode that formula — it becomes O(1) with zero parameters.

---

## Deep HNSW Analysis: 10 EML Opportunities

### 1. ADAPTIVE ef (beam width) — OBVIOUS, ALREADY IDENTIFIED
Learn optimal beam width per query. Skip.

### 2. LEARNED DISTANCE — OBVIOUS, ALREADY IDENTIFIED
Domain-specific distance function. Skip.

### 3. COSINE SIMILARITY DECOMPOSITION

Current implementation (line 64 of hnsw_store.rs):
```rust
fn distance(&self, other: &Self) -> f32 {
    1.0 - cosine_similarity(&self.embedding, &other.embedding)
}
```

Cosine similarity requires a full dot product + two norms = O(d) operations for d dimensions. For 384-dim embeddings, that's 1,152 multiply-adds per distance computation.

**EML opportunity**: For a SPECIFIC knowledge graph, most vectors cluster in a low-dimensional subspace. EML can discover which dimensions actually matter.

Train: compute full cosine on 10K pairs → fit an EML tree on (dim_0, dim_3, dim_47, ...) → discover the 5-10 most discriminative dimensions → reduce distance computation from O(384) to O(10).

This is NOT dimensionality reduction (PCA/random projection). It's **learned dimension selection** — EML discovers which specific dimensions separate your data, and the formula for combining them.

**Expected impact**: 10-30x faster distance computation for domain-specific data.
**When it works**: when the embedding space has low intrinsic dimensionality (most real data does).
**When it fails**: truly uniform high-dimensional data (rare in practice).

### 4. SEARCH PATH PREDICTION (Novel)

During HNSW greedy search, the algorithm visits node A, computes distance to A's neighbors, picks the closest, moves there, repeats. The path through the graph is deterministic given the query.

**Key observation**: for a given graph, search paths are NOT random. Queries in the same region follow similar paths. The path is a function of the query vector.

**EML can learn**: given (query_region_features) → predict (first 3 nodes in the search path).

If the first 3 nodes of the path are predicted correctly, the search starts 3 hops closer to the answer — effectively skipping 60-80% of the traversal.

Implementation:
1. Cluster queries into 16-32 regions (K-means on query vectors)
2. For each region, record the search paths of 100 queries
3. For each region, the first 2-3 nodes are almost always the same (the "highway on-ramps")
4. Build a lookup: query_region → entry_nodes (no EML needed — just a learned routing table)

Actually, this IS what EML lookup tables are for: the region → entry_node mapping is a discrete function that EML can represent as `entry_node_id = eml(region_centroid_features)`.

**Expected impact**: 2-5x faster search (skip most of the top-layer traversal).
**This is the biggest win.**

### 5. NEIGHBOR QUALITY PREDICTION (Novel)

When HNSW builds the graph, it must decide which neighbors to connect. The Navarro heuristic keeps neighbors that are "diverse" — not too close to each other in terms of the points they can reach.

**EML can learn**: given (candidate_distance, candidate_degree, avg_neighbor_distance, layer) → will keeping this candidate improve search quality?

Train: build graphs with different neighbor selections, measure recall, fit EML to predict quality.

The trained formula tells you the EXACT rule for when a candidate neighbor improves the graph. You can then hardcode that rule — it becomes a simple branch instead of a heuristic.

### 6. REBUILD COST PREDICTION (Novel)

Rebuilding the HNSW index is O(n log n). Currently triggered after a fixed number of inserts. But the actual need to rebuild depends on how much the new inserts degraded search quality.

**EML can learn**: given (inserts_since_rebuild, fraction_deleted, avg_query_score_delta, graph_density) → estimated recall loss

Train: measure actual recall periodically, correlate with graph state.

The formula tells you: "recall has dropped ~3%" — you rebuild when the predicted loss exceeds your tolerance. No more wasted rebuilds, no more missed degradation.

### 7. LAYER PROBABILITY OPTIMIZATION (Novel)

The standard HNSW layer assignment uses `floor(-ln(random) * m_L)` where m_L = 1/ln(M). This is a fixed formula from the original paper (Malkov 2016).

**The paper's key insight applies here**: the layer assignment formula IS an elementary function (it uses ln). EML could discover a BETTER layer assignment function from data — one that produces a graph with shorter search paths for YOUR specific data distribution.

The standard formula assumes uniform data. Real data is clustered. EML could learn: given (vector_features, graph_size, cluster_membership) → optimal layer.

This requires replacing `instant-distance` with a custom HNSW where layer assignment is pluggable.

### 8. PROGRESSIVE DIMENSIONALITY (Novel — may be the most impactful)

This combines ideas 3 and 4. Instead of computing full 384-dim cosine at every step of the search:

**Layer 2 (top, coarse)**: use a 4-dim EML-learned projection → O(4) distance
**Layer 1 (middle)**: use a 16-dim EML-learned projection → O(16) distance  
**Layer 0 (bottom, fine)**: use full 384-dim cosine → O(384) distance

The EML tree for each layer learns WHICH dimensions matter at that granularity level:
- Top layer: only the grossest features matter (is it code vs prose vs data?)
- Middle layer: finer features (is it Python vs Rust vs Go?)
- Bottom layer: all details (is it this specific function?)

This is hierarchical learned hashing, but with interpretable EML formulas at each level.

**Expected impact**: 5-20x faster search by avoiding full-dimensional distance on the upper layers where coarse filtering suffices.

### 9. CACHE-AWARE TRAVERSAL (Novel)

During HNSW search, neighbor vectors are accessed in unpredictable order — poor cache locality. L2/L3 cache misses dominate search time for large indexes.

**EML can learn**: given (current_node, query_region) → predicted access order of neighbors.

If we know which neighbors will be visited, we can prefetch them. The prediction doesn't need to be perfect — even 50% correct prefetching improves cache hit rate dramatically.

Train: record actual neighbor visit patterns for queries in each region, fit EML predictor.

### 10. QUANTIZATION-AWARE DISTANCE (Novel)

DiskANN uses Product Quantization (PQ) — compressed vector representations. The distance between quantized vectors is approximate. The approximation error is NOT random — it depends on the codebook and the vectors.

**EML can learn**: given (pq_distance, codebook_id, quantization_residual_norm) → corrected_distance

This learns the error correction function for your specific PQ codebook, improving recall without recomputing exact distances.

---

## Priority Ranking

| # | Opportunity | Impact | Effort | Needs Custom HNSW? |
|---|------------|--------|--------|-------------------|
| **8** | Progressive dimensionality | **5-20x search speed** | L | No — wrap distance function |
| **4** | Search path prediction | **2-5x search speed** | M | No — add entry point cache |
| **3** | Cosine decomposition | **10-30x distance speed** | M | No — wrap distance function |
| **1** | Adaptive ef | 1.5-3x search speed | S | No |
| **6** | Rebuild cost prediction | Operational efficiency | S | No |
| **9** | Cache-aware traversal | 1.5-2x (memory-bound) | M | Partially |
| **5** | Neighbor quality | Better recall | L | Yes |
| **7** | Layer probability | Better graph structure | L | Yes |
| **10** | Quantization correction | Better PQ recall | M | DiskANN only |
| **2** | Learned distance | Better recall | M | No |

---

## The Meta-Insight

The paper's real contribution to HNSW isn't any single optimization — it's the ability to **discover the exact formula** for relationships that were previously hand-tuned or hardcoded.

Every heuristic in HNSW (distance function, beam width, rebuild threshold, layer assignment, neighbor selection) was designed by a human who made assumptions about data distribution. EML replaces human assumptions with learned formulas that are:
- Exact (weights snap to {0,1})
- Interpretable (you can read the formula)
- Domain-specific (trained on YOUR data)
- Self-improving (retrain as data changes)

The standard HNSW algorithm is optimal for GENERIC data. EML makes it optimal for YOUR data.
