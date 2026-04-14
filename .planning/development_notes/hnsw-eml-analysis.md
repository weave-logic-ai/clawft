# HNSW-EML Integration Analysis

Deep analysis of EML (exp(x) - ln(y)) opportunities within the WeftOS and
ruvector HNSW stacks. EML derives learned functions from data the same way
it builds lookup tables -- this is the key insight driving every proposal
below.

Date: 2026-04-04

---

## 1. Codebase Inventory

### 1.1 WeftOS HNSW (clawft-core / clawft-kernel)

| File | Purpose |
|------|---------|
| `crates/clawft-core/src/embeddings/hnsw_store.rs` | Primary HNSW store -- wraps `instant-distance` crate. Distance = `1 - cosine_similarity`. Brute-force fallback below 32 entries. Lazy rebuild with 100-mutation threshold. |
| `crates/clawft-core/src/embeddings/micro_hnsw.rs` | WASM micro-HNSW -- single-layer graph, max 1024 vectors, 16 neighbors/node. Greedy search from entry node 0. |
| `crates/clawft-kernel/src/hnsw_service.rs` | Kernel service wrapping `HnswStore` behind `Mutex`. Exposes `insert`, `search`, `search_batch`, `clear`, `save_to_file`, `load_from_file`. |
| `crates/clawft-kernel/src/vector_hnsw.rs` | `VectorBackend` trait impl over `HnswService`. Adds epoch versioning, optimistic concurrency, soft-delete, compaction, capacity limits. |
| `crates/clawft-kernel/src/vector_diskann.rs` | DiskANN backend (Vamana graph + PQ + mmap). Product Quantization with configurable `pq_num_chunks`. |

**Key observations:**

- `HnswStore` delegates graph construction to `instant-distance::Builder`.
  The library uses the standard Malkov algorithm internally but its
  construction parameters (ef_search, ef_construction) are the only knobs
  exposed. Layer selection, neighbor pruning, and search beam logic are
  all inside the opaque `instant-distance` crate.
- `MicroHnsw` is hand-rolled: single-layer, brute-force neighbor finding
  during insert (`find_nearest_indices` does a full scan), greedy search
  from node 0 with a visited-set expansion. No beam width parameter.
- The `HnswService` uses a global `Mutex<HnswStore>`, serializing all
  operations. `search_batch` amortizes lock acquisition.
- Rebuild is triggered when `inserts_since_rebuild >= rebuild_threshold`
  (default 100). Between rebuilds, new entries are brute-force scanned
  and merged with HNSW results (hybrid search).

### 1.2 ruvector HNSW (patches/hnsw_rs)

Full Malkov 2016/2018 implementation (1872 lines). Key algorithmic details:

| Component | Implementation |
|-----------|---------------|
| **Distance** | Generic via `anndists::Distance<T>` trait. Supports L2, cosine, Hamming, custom. |
| **Layer generation** | `LayerGenerator::generate()`: `floor(-ln(U(0,1)) * scale)` where `scale = 1/ln(max_nb_connection)`. Max 16 layers. Overflow redistributed uniformly. |
| **Neighbor selection** | Navarro heuristic (Algorithm 4 from paper): candidate is kept iff no already-selected neighbor is closer to the candidate than the candidate is to the query. Optionally extends candidates by exploring neighbor-of-neighbor. `keep_pruned` flag appends discarded candidates at the end. |
| **Search** | `search_layer()`: beam search with binary heap. Candidates stored with negated distances. Early termination when nearest candidate > farthest result. `ef` parameter controls beam width. |
| **Insert** | Top-down: greedy (ef=1) from entry point through layers above insertion level, then beam search (ef=ef_construction) through insertion level down to layer 0. Bidirectional links with shrinking. Layer 0 gets `2 * max_nb_connection` neighbors. |
| **Entry point** | Single global entry point (highest-layer point). Updated on insertion when a new point has a higher layer. |
| **Parallelism** | Rayon-based `parallel_insert` and `parallel_search`. `parking_lot` RwLock on neighbor lists. |
| **Filtering** | `FilterT` trait for predicate-based search filtering. |

### 1.3 ruvector Hyperbolic (ruvector-attention)

Poincare ball model operations: `poincare_distance`, `mobius_add`,
`mobius_scalar_mult`, `exp_map`, `log_map`. Uses `acosh`/`atanh` -- both
are elementary functions reconstructable by EML.

### 1.4 EML Core (eml-core)

- Operator: `eml(x, y) = exp(x) - ln(y)` -- universal, like NAND for
  continuous math.
- `EmlTree`: depth 2-5, affine mixing at level 0 (softmax3), EML pairing
  at higher levels. 30-80 params typical.
- `EmlModel`: multi-head model, coordinate descent training with random
  restarts. Records (inputs, targets) pairs, trains gradient-free.
- Already used in `eml_coherence.rs` to predict algebraic connectivity
  from graph statistics (two-tier pattern: fast EML every tick, expensive
  spectral analysis periodically for ground truth).

---

## 2. EML Opportunities in HNSW

### 2a. Learned Distance Functions

**Current state:** `HnswStore` hardcodes `1 - cosine_similarity`. ruvector
supports pluggable distance via `Distance<T>` trait but all options are
fixed mathematical functions.

**EML opportunity:** Train an EML model to learn a domain-specific distance
function from relevance feedback. The model takes two vectors (or their
difference/interaction features) and outputs a scalar "semantic distance."

**Training signal:** (query, result, user_relevance_score) triplets. The
EML model learns: given `features(q, r)`, what distance ranking best
predicts user relevance?

**Why EML fits:** The learned distance IS an elementary function. EML with
depth 4 can represent compositions of exp, ln, trig, polynomials -- far
more expressive than linear Mahalanobis transforms but still O(1) per
evaluation (30-60 ns).

**Concrete example:** In WeftOS's knowledge graph, "function A calls
function B" should be semantically closer than "function A is defined near
function C", even if embedding L2 says otherwise. EML learns the
correction term.

**Implementation sketch:**

```rust
// In crates/clawft-core/src/embeddings/hnsw_store.rs

/// EML-learned distance correction applied on top of cosine.
struct LearnedDistance {
    model: EmlModel,  // depth=4, 8 input features, 1 head
}

impl LearnedDistance {
    fn distance(&self, a: &[f32], b: &[f32]) -> f32 {
        // Extract interaction features
        let dot: f64 = a.iter().zip(b).map(|(x,y)| (*x as f64) * (*y as f64)).sum();
        let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let cosine = dot / (norm_a * norm_b + 1e-8);
        let l2_sq: f64 = a.iter().zip(b).map(|(x,y)| ((*x-*y) as f64).powi(2)).sum();
        let dim_ratio = a.len() as f64 / 384.0;  // normalize

        let features = [cosine, l2_sq.sqrt(), norm_a, norm_b,
                        (norm_a - norm_b).abs(), dim_ratio,
                        cosine * cosine, l2_sq];
        self.model.predict_primary(&features) as f32
    }
}
```

**Impact:** Recall improvement +2-5% on domain-specific queries. Distance
evaluation overhead: ~60 ns per call (vs ~20 ns for raw cosine). Net search
time increase negligible because HNSW evaluates O(ef * M) distances, and
ef*M is typically 1000-3000.

### 2b. Adaptive Search Beam Width (ef)

**Current state:** `HnswServiceConfig::ef_search` is a fixed constant (100).
All queries use the same beam width regardless of query characteristics.

**EML opportunity:** Learn the optimal ef per query. Some queries are
"easy" (the answer is a tight cluster) and need ef=20. Others are
"hard" (ambiguous, spread across the graph) and need ef=200.

**Training signal:** Run queries at multiple ef values, measure recall
against brute-force ground truth. Record:
`(query_norm, query_entropy, graph_size, avg_neighbor_distance) -> min_ef_for_95%_recall`

**Input features (5):**
1. Query vector norm (||q||)
2. Query "peakiness" (max component / mean component)
3. Current graph size (n)
4. Distance to entry point (available for free during top-layer descent)
5. Ratio of top-2 distances in greedy phase (proxy for query difficulty)

**Implementation location:**
`crates/clawft-core/src/embeddings/hnsw_store.rs`, modify `query()` to
call `adaptive_ef_model.predict_primary(&features)` before the HNSW search
call.

```rust
// Inside HnswStore::query(), before search:
fn compute_adaptive_ef(&self, query: &[f32]) -> usize {
    if let Some(ref model) = self.ef_model {
        let norm: f64 = query.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
        let max_comp = query.iter().map(|x| x.abs()).fold(0.0f32, f32::max) as f64;
        let mean_comp = query.iter().map(|x| x.abs()).sum::<f32>() as f64 / query.len() as f64;
        let peakiness = max_comp / (mean_comp + 1e-8);
        let n = self.entries.len() as f64;
        let features = [norm, peakiness, n.ln(), n.sqrt(), max_comp];
        let predicted = model.predict_primary(&features);
        (predicted.round() as usize).clamp(10, 500)
    } else {
        self.ef_search
    }
}
```

**Impact:** 1.5-3x speedup on easy queries (ef drops from 100 to 20-40),
neutral on hard queries. No recall loss by design (trained to achieve
95% target). This is the highest-ROI opportunity.

### 2c. Learned Layer Assignment

**Current state:** ruvector uses `floor(-ln(U(0,1)) / ln(M))` -- the
standard exponential distribution from Malkov 2016. WeftOS (`instant-
distance`) uses the same internally. All vectors have identical
probability of landing on each layer, regardless of their properties.

**EML opportunity:** Learn: given a vector's properties and the current
graph state, which layer maximizes search quality? High-degree hub
vectors should be promoted to higher layers; outliers should stay low.

**Training signal:** After building an index, measure per-vector
"reachability" (how many queries route through this vector). Vectors
with high reachability should be on higher layers. Record:
`(vector_norm, distance_to_centroid, local_density, current_graph_stats) -> optimal_layer`

**Why this works:** The current random assignment wastes upper-layer
capacity on unimportant vectors. EML discovers the relationship between
vector features and optimal layer, replacing the blind exponential with
a data-driven function.

**Caveat:** This requires modifying the ruvector `LayerGenerator` or
building a custom layer assignment in the WeftOS micro-HNSW. Not
applicable to `instant-distance` without forking.

### 2d. Learned Neighbor Pruning

**Current state:** ruvector `select_neighbours()` implements the Navarro
heuristic: keep candidate c if no already-selected neighbor n satisfies
`d(c, n) <= d(c, query)`. This is a hard geometric rule.

**EML opportunity:** Replace the binary keep/discard decision with a
learned score. The EML model takes:
`(candidate_distance, candidate_degree, layer, num_selected_so_far,
  min_dist_to_selected, max_dist_to_selected)`
and outputs a keep-score. Keep if score > threshold.

**Training signal:** Build indices with different pruning strategies,
measure recall and search speed. Use the best-performing neighbor sets
as positive labels.

**Impact:** Modest recall improvement (+1-2%) with potential for better
graph connectivity in clustered datasets where Navarro over-prunes.

### 2e. Learned Entry Point Routing

**Current state:** Both implementations use a single fixed entry point
(the highest-layer node in ruvector; implicit in `instant-distance`).

**EML opportunity:** Maintain K entry points (e.g., K=4-8 cluster
centroids). Train EML to predict: given query features, which entry
point minimizes total search path length?

**Input features:** Query's projection onto each entry point's direction
(K dot products). EML outputs a softmax-style routing score.

**Impact:** 1.3-2x speedup for multi-modal distributions. The search
starts closer to the answer.

**Implementation:** This requires the most invasive changes (modifying
the search entry logic), but has clear precedent in recent HNSW variants
(FINGER, DiskANN multi-probe).

### 2f. Prefetch Prediction

**Current state:** During `search_layer()`, neighbors are visited in
heap order. Memory access is effectively random.

**EML opportunity:** Given (current_node_id, query_features), predict
which of current_node's neighbors are most likely to be visited next.
Prefetch their data before the distance computation.

**Training signal:** Instrument `search_layer` to log visit sequences.
Train EML on: `(current_node_features, query_features) -> next_node_rank`

**Why EML fits:** The prediction function is a simple pattern (which
direction is the search going?). Depth-2 EML can capture this.

**Impact:** 1.2-1.5x speedup on large indices where cache misses
dominate. Negligible benefit on small in-memory indices.

### 2g. Learned Rebuild Threshold

**Current state:** `HnswStore::rebuild_threshold` is a fixed constant
(100). Rebuilds are triggered after 100 mutations regardless of impact
on search quality.

**EML opportunity:** Learn: given
`(inserts_since_rebuild, deletes_since_rebuild, search_quality_delta,
  graph_size, last_rebuild_cost_ms)`
should we rebuild now?

**Training signal:** Measure recall degradation as mutations accumulate.
Record the mutation count at which recall drops below target.

**Impact:** Avoids unnecessary rebuilds (saves CPU) and catches
quality degradation earlier (improves recall). Small implementation
cost.

**Implementation location:** `HnswStore::query()`, line 269:
```rust
// Current: self.inserts_since_rebuild >= self.rebuild_threshold
// Learned: self.should_rebuild_model.predict_primary(&features) > 0.5
```

### 2h. Learned Quantization (DiskANN PQ)

**Current state:** `vector_diskann.rs` uses standard Product Quantization
with `pq_num_chunks` subspaces and fixed codebooks.

**EML opportunity:** Learn a domain-specific quantization function that
preserves distance ordering better than axis-aligned PQ. The quantization
function maps a d-dimensional vector to a compact code, and the inverse
maps codes to approximate distances. Both mappings ARE elementary
functions; EML can represent them.

**Training signal:** (vector, nearest_neighbors) pairs. Train EML to
minimize distance ranking errors after quantization.

**Impact:** 5-15% recall improvement for compressed search at the same
memory budget. High implementation complexity.

---

## 3. Ranked Impact Assessment

| # | Opportunity | Difficulty | Speedup | Recall Delta | Needs Training Data? | Priority |
|---|-----------|------------|---------|-------------|---------------------|----------|
| **2b** | Adaptive ef (beam width) | S | **1.5-3x** | 0% (by design) | Yes, but auto-generated from brute-force ground truth | **P0** |
| **2a** | Learned distance function | M | 0.9x (slower per-eval) | **+2-5%** | Yes, relevance feedback | **P1** |
| **2g** | Learned rebuild threshold | S | 1.1-1.3x | +0-1% | Auto-generated from mutation/recall tracking | **P1** |
| **2e** | Multi-entry-point routing | M | **1.3-2x** | +1-2% | Yes, from search path traces | **P2** |
| **2f** | Prefetch prediction | M | 1.2-1.5x | 0% | Yes, from visit traces | **P2** |
| **2c** | Learned layer assignment | L | 1.1-1.5x | +1-3% | Yes, reachability analysis | **P3** |
| **2d** | Learned neighbor pruning | L | 1.0x | +1-2% | Yes, comparative index builds | **P3** |
| **2h** | Learned PQ codebooks | L | 1.0x | +5-15% (compressed) | Yes, neighbor pairs | **P4** |

---

## 4. Implementation Plan for Top 3

### 4.1 Adaptive ef (P0)

**Files to modify:**
- `crates/clawft-core/src/embeddings/hnsw_store.rs` -- add `ef_model: Option<EmlModel>` field to `HnswStore`, modify `query()` to call `compute_adaptive_ef()`.
- `crates/clawft-kernel/src/hnsw_service.rs` -- expose `train_ef_model()` and `set_ef_model()` on `HnswService`.

**EML configuration:** Depth 3, 5 inputs, 1 head (~30 params).

**Self-training loop (no external data needed):**
1. Every N searches, run the same query at ef=5,10,20,50,100,200,500 against brute-force ground truth.
2. Record `(features, min_ef_achieving_95%_recall)`.
3. After 200 samples, call `model.train()`.
4. Enable adaptive ef by setting `ef_model = Some(trained_model)`.

**Risk:** Low. If the model predicts poorly, ef is clamped to [10, 500] and the worst case is normal performance.

### 4.2 Learned Distance Function (P1)

**Files to modify:**
- `crates/clawft-core/src/embeddings/hnsw_store.rs` -- add `distance_model: Option<EmlModel>` field, modify `cosine_similarity()` call sites to optionally use learned distance.
- New: `crates/clawft-core/src/embeddings/learned_distance.rs` -- feature extraction from vector pairs, EML evaluation.
- `crates/clawft-kernel/src/vector_hnsw.rs` -- expose training API on `HnswBackend`.

**EML configuration:** Depth 4, 8 inputs, 1 head (~50 params).

**Feature vector for a pair (a, b):**
```
[cosine(a,b), l2(a,b), ||a||, ||b||, |norm_diff|, dim/384, cosine^2, l2^2]
```

**Training requires relevance labels.** Two acquisition paths:
- Explicit: user marks search results as relevant/irrelevant.
- Implicit: clickthrough data, dwell time, or graph traversal frequency.

**Fallback:** If no relevance data, use graph-derived signal: vectors
connected by short causal paths in the knowledge graph should have
small learned distances.

### 4.3 Learned Rebuild Threshold (P1)

**Files to modify:**
- `crates/clawft-core/src/embeddings/hnsw_store.rs` -- add `rebuild_model: Option<EmlModel>`, modify the rebuild decision at line 269, instrument `query()` to periodically sample ground-truth recall.

**EML configuration:** Depth 2, 5 inputs, 1 head (~18 params).

**Input features:**
```
[inserts_since_rebuild / rebuild_threshold,
 (entries.len - index_built_len) as f64 / entries.len,
 ln(entries.len),
 last_sampled_recall,
 last_rebuild_duration_ms / 1000.0]
```

**Self-training loop:**
1. Every 50 queries, run one query against brute-force and record recall.
2. When a rebuild IS triggered (by the fixed threshold), record the
   features-at-decision-time as a positive sample.
3. When a rebuild was triggered but recall was already >98%, record as
   a negative sample (rebuild was unnecessary).
4. After 100 samples, train and switch to learned thresholds.

---

## 5. Cross-Cutting: EML + Hyperbolic HNSW

The ruvector `poincare_distance` function uses `acosh`:
```rust
(1.0 / sqrt_c) * arg.max(1.0).acosh()
```

`acosh(x) = ln(x + sqrt(x^2 - 1))` -- this is an elementary function
perfectly representable by EML. An EML-accelerated Poincare distance
could:
1. Approximate `acosh` with a depth-2 EML tree (2 params), trained on
   the actual input distribution.
2. Avoid the expensive `sqrt` + `ln` chain when the argument falls in a
   common range.
3. Provide 2-4x speedup on hyperbolic distance evaluation.

This connects EML to the entire ruvector-attention stack: fused attention
with curvature, tangent space projections, and component quantizers all
use elementary functions that EML can learn from runtime data.

---

## 6. Two-Tier Pattern (from eml_coherence.rs)

The existing `eml_coherence.rs` establishes the canonical two-tier
pattern. All HNSW-EML integrations should follow the same architecture:

```
Fast path (every operation):  EML prediction    ~0.1 us
Slow path (periodic):         Ground truth       ~500 us
Feedback loop:                model.record() + model.train()
```

This ensures:
- Zero regression risk (the slow path provides exact answers when needed).
- Continuous improvement (the model gets better over time).
- Graceful degradation (untrained model falls back to fixed defaults).

---

## 7. Dependency Notes

- `eml-core` is already a workspace crate with no external dependencies.
  Adding it to `clawft-core/Cargo.toml` requires only:
  `eml-core = { path = "../eml-core" }`
- All EML models are serializable (serde). They can be persisted alongside
  the HNSW index in `StoreSnapshot` and rebuilt on load.
- The `instant-distance` crate does not expose layer assignment or neighbor
  selection hooks. Opportunities 2c and 2d require either:
  (a) switching to the ruvector `hnsw_rs` backend, or
  (b) implementing a custom HNSW in `micro_hnsw.rs` with EML hooks.
- The micro-HNSW (`micro_hnsw.rs`) is the better target for rapid
  prototyping: it is hand-rolled, single-file, and already has modifiable
  search and insert logic.
