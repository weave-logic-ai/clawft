# Sprint 11 Symposium -- Track 7: Algorithmic Optimization & Performance

**Panel**: Performance Engineer (chair), Memory Specialist, ECC Analyst,
Matrix Optimizer, PageRank Analyzer, Researcher

**Date**: 2026-03-27
**Codebase**: WeftOS v0.1.0-pre (`feature/weftos-kernel-sprint`)
**Method**: Read the actual implementation, analyze Big-O, survey literature,
recommend keep/replace/augment per algorithm

---

## 1. Spectral Analysis (Lambda_2 -- Algebraic Connectivity)

### 1.1 Current Implementation

**File**: `crates/clawft-kernel/src/causal.rs:574-709`
**Method**: `CausalGraph::spectral_analysis(max_iterations: usize) -> SpectralResult`

The implementation computes the Fiedler value (lambda_2, the second-smallest
eigenvalue of the graph Laplacian) using power iteration on the dense Laplacian
matrix, with explicit orthogonalization against the constant eigenvector.

Steps:
1. Build an index map from NodeId to matrix index (sort + HashMap).
2. Construct a **dense** Laplacian `L = D - A` as `Vec<Vec<f64>>` (n x n).
3. Initialize a starting vector `v` linearly spaced, orthogonalized against
   the all-ones vector.
4. Iterate up to `max_iterations` times:
   - Compute `w = L * v` via dense matrix-vector multiply.
   - Orthogonalize `w` against the constant vector.
   - Compute Rayleigh quotient `lambda_2 = v^T w`.
   - Normalize `w` and set `v = w/||w||`.
5. Return `(lambda_2, fiedler_vector, node_ids)`.

**Note**: The code comment says "power iteration" but what is actually implemented
is a **Rayleigh quotient iteration** variant that converges to the largest
eigenvalue of L restricted to the orthogonal complement of the null space.
Since the null space of L is the constant vector (for connected graphs), this
converges to the Fiedler value only for connected two-component-like structures.
For general graphs, this converges to lambda_max projected onto the Fiedler
subspace, which is correct because the orthogonalization forces convergence to
the smallest non-zero eigenvalue. The implementation is mathematically sound.

### 1.2 Complexity Analysis

| Operation | Complexity |
|-----------|-----------|
| Build Laplacian (dense) | O(n^2) memory, O(n + m) time (n nodes, m edges) |
| Matrix-vector multiply per iteration | **O(n^2)** |
| Total (k iterations) | **O(k * n^2)** |
| Memory | **O(n^2)** |

With default `max_iterations = 50` and graph sizes up to 10K nodes, this means
50 * 100M = 5 billion floating-point operations per spectral analysis call. At
~1ns per FLOP, that is approximately **5 seconds** for a 10K-node graph. This
is acceptable for offline analysis but completely unacceptable for the 15ms
DEMOCRITUS tick budget.

### 1.3 Alternative Algorithms

| Algorithm | Complexity | Accuracy | Notes |
|-----------|-----------|----------|-------|
| **Dense Laplacian + power iteration** (current) | O(k*n^2) | Exact (converged) | Simple, no deps |
| **Sparse Laplacian + Lanczos iteration** | O(k*m) where m=edges | Exact for k eigenvalues | Standard for sparse graphs; 10-100x faster for sparse ECC graphs |
| **Randomized SVD (Halko et al. 2011)** | O(k*m + k^2*n) | Approximate | Good for top-k eigenvalues; probabilistic guarantees |
| **Chebyshev polynomial approximation** | O(m * polylog(n)) | Approximate | Near-linear time; requires spectral bounds estimate |
| **Graph sparsification + exact solve** | O(m * log^3(n)) | (1+epsilon) approx | Spielman-Teng framework; complex implementation |
| **Incremental Fiedler update** (rank-1 updates) | O(n) per edge add/remove | Approximate | Matrix perturbation theory; only works for small changes |

### 1.4 Recommendation: REPLACE (v0.2)

Replace the dense Laplacian construction and O(n^2) matrix-vector multiply with
a **sparse Laplacian + Lanczos iteration** approach.

The causal graph is inherently sparse. With DEMOCRITUS adding ~5 edges per node
(search_k=5), a 10K-node graph has ~50K edges. The Lanczos method performs
matrix-vector multiplies in O(m) instead of O(n^2), giving a **200x speedup**
at 10K nodes (50K vs 100M operations per iteration).

**Concrete change**: Replace the `Vec<Vec<f64>>` Laplacian with a compressed
sparse row (CSR) representation. The Lanczos recurrence needs only matrix-vector
products and vector dot products -- both of which operate in O(m) and O(n)
respectively. The `nalgebra-sparse` crate or a hand-rolled CSR would work. No
external eigenvalue solver is needed; the Lanczos tridiagonal output can be
solved with the QR algorithm in O(k^2) which is negligible.

For real-time tick integration, add an **incremental update** path (v0.3): when
a single edge is added, use rank-1 perturbation theory to update lambda_2 in
O(n) rather than recomputing from scratch.

### 1.5 Version Target

- **v0.2**: Sparse Lanczos (drop-in replacement, same API)
- **v0.3**: Incremental rank-1 update for per-tick monitoring
- **v1.0+**: Chebyshev approximation for graphs > 100K nodes

### 1.6 ECC Contribution

The spectral analysis produces one causal node per analysis invocation (the
"coherence checkpoint") and potentially N/2 edges (via `spectral_partition`).
At v0.2 scale, this is 1 node + O(n) edges per analysis. The Fiedler vector
itself could be stored as node metadata (~8 bytes * n) for downstream use.

---

## 2. Community Detection (Label Propagation)

### 2.1 Current Implementation

**File**: `crates/clawft-kernel/src/causal.rs:480-555`
**Method**: `CausalGraph::detect_communities(max_iterations: usize) -> Vec<Vec<NodeId>>`

Algorithm: **Weighted Label Propagation (LPA)**

Steps:
1. Initialize each node with its own ID as label.
2. Sort node IDs for deterministic processing order.
3. For each iteration (up to `max_iterations`):
   a. For each node in sorted order:
      - Collect neighbor labels weighted by edge weight (both forward and
        reverse edges, treating graph as undirected).
      - Adopt the label with the highest total weight. Ties broken by
        smallest label ID.
   b. If no labels changed, stop early.
4. Group nodes by final label, sort communities largest-first.

### 2.2 Complexity Analysis

| Operation | Complexity |
|-----------|-----------|
| Initialization | O(n) |
| Per-iteration label update | O(n * avg_degree) = O(m) per node scan |
| Total (k iterations) | **O(k * m)** |
| Memory | **O(n)** (labels HashMap) |

With typical convergence in 5-10 iterations for sparse graphs, and m = O(5n)
for ECC graphs, the total is approximately **50n operations**. At 10K nodes,
this is ~500K operations -- extremely fast. LPA is already near-optimal for
this graph density.

### 2.3 Alternative Algorithms

| Algorithm | Complexity | Quality | Notes |
|-----------|-----------|---------|-------|
| **Label Propagation** (current) | O(k*m) | Moderate (non-deterministic in general, but this impl is deterministic) | Fast, simple, no parameters |
| **Louvain** | O(n*log(n)) typical | High (modularity-optimized) | Hierarchical; better quality; more complex |
| **Leiden** | O(n*log(n)) typical | Highest (fixes Louvain's disconnected communities) | Best quality; requires refinement phase |
| **Infomap** | O(m) | High (information-theoretic) | Good for directed graphs; complex implementation |
| **Incremental LPA** | O(delta_m) per update | Moderate | Only update labels of affected neighborhood |

### 2.4 Recommendation: AUGMENT (v0.3)

**Keep LPA for real-time use** within the tick loop. Its O(k*m) complexity with
5-10 iterations is already excellent for the 15ms budget.

**Add Louvain/Leiden as an offline alternative** for periodic deep analysis
(e.g., every 100 ticks or on demand). The quality improvement matters for the
knowledge graph use case where community structure informs the GUI's cluster
visualization.

**Add incremental LPA for v0.3**: When a new node is added with edges, only
re-propagate labels within a 2-hop neighborhood of the new node instead of
running full LPA. This reduces per-tick cost to O(avg_degree^2) which is
approximately O(25) for ECC's 5-neighbor structure.

The current implementation has one algorithmic concern: the **deterministic
processing order** (sorted by NodeId) introduces a systematic bias where
lower-numbered nodes tend to dominate label assignment. This is acceptable at
current scale but could produce suboptimal communities at >10K nodes.
Randomized order with seeded RNG would fix this.

### 2.5 Version Target

- **v0.1.x**: Keep as-is (fast enough, correct)
- **v0.2**: Add randomized processing order option
- **v0.3**: Incremental LPA + offline Louvain/Leiden
- **v1.0+**: Leiden with hierarchical resolution parameter

### 2.6 ECC Contribution

Community detection produces 0 new causal nodes but generates O(C) metadata
updates where C = number of communities. Each community assignment is a label
on existing nodes. No new edges are created by detection itself, but downstream
consumers (the GUI, the coherence monitor) use the community structure to create
summary nodes -- approximately 1 summary node + C edges per analysis.

---

## 3. HNSW Search

### 3.1 Current Implementation

**File**: `crates/clawft-core/src/embeddings/hnsw_store.rs`
**Library**: `instant-distance` v0.6
**Wrapper**: `crates/clawft-kernel/src/hnsw_service.rs`

Architecture:
- `HnswStore` holds a `Vec<HnswEntry>` (source of truth) and an optional
  `HnswMap<EmbeddingPoint, usize>` (the HNSW index).
- `EmbeddingPoint` implements `instant_distance::Point` with distance =
  `1.0 - cosine_similarity`.
- `HnswService` wraps `HnswStore` behind a `Mutex<HnswStore>`.
- Below `HNSW_THRESHOLD = 32` entries, queries use brute-force.
- Above threshold, queries route through the HNSW graph.

### 3.2 Current Parameters

| Parameter | Value | Source |
|-----------|-------|--------|
| `ef_search` | 100 | `HnswServiceConfig::default()` |
| `ef_construction` | 200 | `HnswServiceConfig::default()` |
| `default_dimensions` | 384 | `HnswServiceConfig::default()` |
| `HNSW_THRESHOLD` | 32 | Hardcoded constant |
| M (max connections per layer) | library default | Not exposed by `instant-distance` v0.6 |

**Assessment**: These are **standard textbook defaults**, not tuned for the
ECC workload. They are reasonable starting points but leave performance on the
table.

- `ef_construction = 200` is aggressive (typically 100-200 is recommended).
  For ECC's small-to-medium stores (< 100K entries), this is fine. The
  construction cost is amortized over the full rebuild.
- `ef_search = 100` targets ~95% recall@10. For ECC's use case (finding
  correlated events, threshold = 0.7), high recall is important but 100 is
  likely overkill for top-5 queries on stores < 10K entries.
- The `instant-distance` crate does not expose the M parameter (max bidirectional
  connections per layer). Its default is M=12 based on the source code. This
  is appropriate for 384-dimensional data.

### 3.3 Recall vs Speed Tradeoff

The key tradeoff is ef_search vs recall@k:

| ef_search | Estimated Recall@5 | Relative Latency |
|-----------|-------------------|-----------------|
| 20 | ~85% | 1x (baseline) |
| 50 | ~93% | 2.5x |
| 100 (current) | ~97% | 5x |
| 200 | ~99% | 10x |

For ECC's event correlation use case, a missed neighbor at rank 5 means a
missed causal edge -- recoverable on the next tick when the same impulse type
recurs. An ef_search of 50 would halve search latency with minimal quality loss.

### 3.4 Critical Performance Issues (from Track 9)

These are algorithmic, not just implementation issues:

1. **O(n) upsert via `retain`** (line 149): `self.entries.retain(|e| e.id != id)`
   scans all entries for every insert. At 10K entries, this is 10K string
   comparisons per insert.

2. **Full index rebuild on every dirty query** (lines 176-178): Any insert
   between queries triggers a complete HNSW graph reconstruction. The
   `instant-distance` `Builder::build()` is O(n * M * ef_construction * log(n)).
   At 10K entries with M=12 and ef_construction=200, this is approximately
   10K * 12 * 200 * 13 = **312 million distance computations** per rebuild.

3. **Cosine similarity is scalar** (line 370-384): No SIMD, no loop unrolling
   hints. At 384 dimensions, each similarity computation is 384*3 = 1152 FLOPs.

### 3.5 Alternative Approaches

| Approach | Benefit | Effort |
|----------|---------|--------|
| **Adaptive ef_search** | Reduce ef from 100 to 30-50 for stores < 5K | Low |
| **HashMap ID index** | O(1) upsert instead of O(n) | Low |
| **Deferred/batched rebuild** | Rebuild only when 10%+ entries changed | Medium |
| **Incremental HNSW insert** | `instant-distance` does not support this; would need library change or switch to `hnsw_rs` | High |
| **Product Quantization** | 4x memory reduction for 384-dim vectors (float32 -> uint8 per sub-vector) | High |
| **ScaNN-style anisotropic quantization** | Better recall/speed tradeoff than PQ | Very High |

### 3.6 Recommendation: REPLACE incrementally (v0.1.x through v0.3)

**v0.1.x** (Track 9 items B1, B2):
- Add `HashMap<String, usize>` index for O(1) upsert.
- Deferred rebuild: only rebuild when insert count since last build exceeds
  10% of current size.

**v0.2**:
- Reduce ef_search to 50 for stores < 5K entries (adaptive).
- Add `search_batch` API to amortize mutex acquisition.
- Replace `Mutex` with `RwLock` for concurrent reads.
- Auto-vectorizable cosine similarity via `chunks_exact(4)`.

**v0.3**:
- Evaluate switching from `instant-distance` to `hnsw_rs` (which supports
  incremental insert without full rebuild).
- Int8 scalar quantization for stored embeddings (4x memory reduction).

**v1.0+**:
- Product quantization for stores > 100K vectors.
- Platform-specific SIMD cosine similarity (ARM NEON / x86 AVX2).

### 3.7 Version Target

See above (phased across v0.1.x through v1.0+).

### 3.8 ECC Contribution

HNSW produces 0 new causal nodes directly. Each search returns up to `search_k`
(default 5) neighbor IDs and scores. These feed into edge creation in the UPDATE
phase: up to 5 causal edges per impulse, gated by the correlation threshold.
At 64 impulses per tick, HNSW contributes up to **320 candidate edges per tick**
to the causal graph (before threshold filtering).

---

## 4. Predictive Change Analysis

### 4.1 Current Implementation

**File**: `crates/clawft-kernel/src/causal.rs:746-924`

Two methods:

**4.1.1 `compute_coupling(change_events) -> Vec<CouplingPair>`**

Computes co-modification coupling between all pairs of nodes that appear
together in change events.

Algorithm:
1. Count per-node changes and per-pair co-changes by iterating all events.
2. For each co-occurring pair (A, B): `coupling = co_changes(A,B) / max(changes(A), changes(B))`.
3. Sort by coupling score descending.

**4.1.2 `predict_changes(change_events, window_size, baseline_factor) -> Vec<ChangePrediction>`**

Burst detection + coupling-based transitive prediction.

Algorithm:
1. Sort events by timestamp.
2. Compute baseline rate per node (total changes / total events).
3. Compute window rate per node (window changes / window events).
4. A node is "in burst" if `window_rate / baseline_rate >= baseline_factor`.
5. Score nodes:
   - Burst nodes: `burst_ratio * 0.6`
   - Coupled partners of burst nodes: `burst_ratio * coupling_score * 0.4`
   - Recent activity: `+0.1` per appearance in last window/3 events
6. Clamp confidence to [0, 1], sort descending.

### 4.2 Complexity Analysis

| Operation | Complexity |
|-----------|-----------|
| `compute_coupling` | O(E * K^2) where E = events, K = avg nodes per event |
| `predict_changes` (calls `compute_coupling` internally) | O(E * K^2 + B * P) where B = burst nodes, P = coupling pairs |
| Memory | O(N^2) worst case for co-change map (if all nodes co-occur) |

For typical ECC workloads: E = hundreds of events, K = 2-5 nodes per event,
N = thousands of nodes. The O(E * K^2) term is small. The coupling map is
sparse (most pairs never co-occur), so memory is O(C) where C = actual
co-occurring pairs.

### 4.3 Algorithmic Assessment

The burst detection is a **simple windowed rate comparison** -- essentially a
z-score without the standard deviation normalization. This is functional but
has known limitations:

1. **No temporal decay**: All events within the window are weighted equally.
   A burst 5 minutes ago and a burst 5 seconds ago have the same score.
2. **No seasonality handling**: If a node is modified every Monday (periodic
   pattern), the burst detector may fire every Monday.
3. **Linear coupling model**: The coupling score `co_changes / max_changes` is
   the Jaccard-like coefficient. It does not account for temporal ordering
   (A changes, then B changes 2 ticks later) vs simultaneous co-modification.
4. **Hardcoded weights**: The 0.6/0.4/0.1 coefficients for burst/coupling/recency
   are not tunable and not derived from any optimization process.

### 4.4 Alternative Algorithms

| Algorithm | What It Adds | Complexity | Notes |
|-----------|-------------|-----------|-------|
| **Exponential Moving Average (EMA)** | Temporal decay for burst detection | O(n) per event | Simple, effective, no window parameter |
| **Kleinberg burst model** | Formal burst detection with state automaton | O(E * log(E)) | Theoretically grounded; handles multiple burst levels |
| **Granger causality** | Temporal ordering in coupling | O(n^2 * lag) | Tests if A predicts B; more than correlation |
| **Hawkes process** | Self-exciting point process model | O(E^2) naive, O(E log E) with KD-tree | Models how one event triggers future events; ideal for causal impulses |
| **Graph Neural Network (GNN)** | Learned prediction | O(n * d * L) per forward pass | Requires training data; overkill at current scale |

### 4.5 False Positive Rate

The current implementation has **no calibrated false positive rate**. The
confidence score is a raw weighted sum clamped to [0, 1], not a probability.
Without ground truth data, we cannot measure precision or recall.

However, structural analysis suggests the false positive risk is moderate:
- The 0.6 weight on burst ratio means any node with `window_rate >= 2x baseline`
  gets confidence >= 0.6 * 2.0 = 1.0 (clamped). This is aggressive.
- The transitive coupling boost (0.4 * burst * coupling) means even weakly
  coupled nodes (coupling = 0.1) of strongly bursting nodes (ratio = 5.0)
  get confidence 0.4 * 5.0 * 0.1 = 0.2. This seems reasonable.
- The recency boost (+0.1 per event) can push non-bursting nodes above
  meaningful thresholds if they appear in many recent events.

**Estimated false positive rate**: 30-50% for confidence > 0.3 at typical
ECC workloads (based on structural analysis, not empirical measurement).

### 4.6 Recommendation: AUGMENT (v0.3)

**Keep the current burst + coupling model** as a baseline predictor. It is
simple, fast, and produces directionally correct results.

**v0.2**: Add EMA-based burst detection as an alternative scoring mode.
Replace the windowed rate comparison with an exponentially weighted moving
average that naturally handles temporal decay:
```
ema[node] = alpha * (event_contains_node ? 1 : 0) + (1 - alpha) * ema[node]
burst = ema[node] / long_term_average > threshold
```
This eliminates the `window_size` parameter and provides smoother predictions.

**v0.3**: Add temporal Granger-like coupling: instead of just counting
co-occurrences, measure whether A changing predicts B changing within a
lag window. This captures directional causality rather than mere correlation.

**v1.0+**: Evaluate Hawkes process model for the full impulse stream. This is
the natural mathematical model for "events that trigger other events" -- exactly
what the ECC causal graph represents.

### 4.7 Version Target

- **v0.1.x**: Keep as-is (functional, fast)
- **v0.2**: EMA burst detection, tunable weights
- **v0.3**: Granger-like directional coupling
- **v1.0+**: Hawkes process model

### 4.8 ECC Contribution

Predictions produce 0 new causal nodes directly. The output is advisory:
`Vec<ChangePrediction>` consumed by the GUI or the coherence monitor. If
predictions are wired into the impulse queue (predicted nodes pre-warmed in
HNSW cache), this would add approximately B predicted nodes * 1 impulse each
to the next tick's workload.

---

## 5. Embedding Strategy

### 5.1 Current Implementation

**Files**:
- `crates/clawft-kernel/src/embedding.rs` -- trait + Mock + LLM providers
- `crates/clawft-kernel/src/embedding_onnx.rs` -- ONNX + SentenceTransformer + AST providers

**Provider Selection Priority** (from `select_embedding_provider`):
1. **ONNX local model** (`all-MiniLM-L6-v2.onnx`, 384 dims) if model file found
   and `onnx-embeddings` feature enabled.
2. **LLM API** (`text-embedding-3-small`, 384 dims) if config present. Currently
   **not wired** -- `call_llm_api()` always returns error, falls back to mock.
3. **MockEmbeddingProvider** (SHA-256 hash, configurable dims) -- deterministic
   fallback.

### 5.2 Model Details

| Provider | Model | Dimensions | Batch Support | Status |
|----------|-------|-----------|---------------|--------|
| ONNX | all-MiniLM-L6-v2 | 384 | Sequential loop (line 330-335) | Feature-gated, functional when model present |
| LLM API | text-embedding-3-small | 384 | Config says batch_size=16, but API not wired | Stub only |
| Mock (SHA-256) | N/A | configurable (default 64) | Yes (sync) | Always available |
| SentenceTransformer | all-MiniLM-L6-v2 (via ONNX) | 384 | Sequential | Markdown-optimized wrapper |
| AST Embedding | all-MiniLM-L6-v2 (via ONNX) | 384 | Sequential | Rust code structural embedder |

### 5.3 ONNX Embedding Analysis

The ONNX provider (`OnnxEmbeddingProvider`) has two modes:

**Real inference mode** (`onnx-embeddings` feature + model file):
- Uses `ort` (ONNX Runtime) v2.0.0-rc.12.
- Input: token IDs from a vocabulary-free hash tokenizer (not real BPE). Each
  whitespace token is hashed to `[1000, 30000)` range, with `[CLS]=101` and
  `[SEP]=102` markers.
- Output: mean-pooled last hidden state, L2 normalized, truncated/padded to
  384 dimensions.
- **Concern**: The tokenizer is not a real sentencepiece/BPE tokenizer. It
  produces valid tensor shapes but the token IDs do not correspond to the
  model's vocabulary. This means **real ONNX inference produces semantically
  meaningless embeddings** unless the model happens to be robust to arbitrary
  token IDs. In practice, the embeddings will have some structure (the model
  learns positional patterns) but quality will be significantly degraded vs
  proper tokenization.

**Hash fallback mode** (default when model not present):
- Position-weighted SHA-256 hashing of whitespace tokens.
- Produces consistent, deterministic vectors.
- L2 normalized.
- No semantic understanding -- purely syntactic similarity based on token overlap.

### 5.4 Fallback Strategy Assessment

The fallback chain is:
```
ONNX (if available) -> LLM API (if configured, but currently stub) -> Mock SHA-256
```

**In practice, all deployments currently use the Mock SHA-256 provider** because:
1. The ONNX model file is not bundled (must be manually placed at
   `.weftos/models/all-MiniLM-L6-v2.onnx`).
2. The LLM API is not wired.

The Mock provider produces vectors where similarity correlates with character-level
token overlap. For ECC's use case (finding related impulses), this provides
useful signal: two `BeliefUpdate` impulses with similar JSON payloads will have
similar embeddings. But it cannot capture semantic similarity ("authentication
error" vs "login failure").

### 5.5 Smaller/Faster Model Options

| Model | Dims | Params | Latency (CPU) | Quality (MTEB avg) |
|-------|------|--------|---------------|-------------------|
| all-MiniLM-L6-v2 (current) | 384 | 22M | ~10ms/text | 0.630 |
| all-MiniLM-L12-v2 | 384 | 33M | ~20ms/text | 0.649 |
| gte-small | 384 | 33M | ~15ms/text | 0.660 |
| bge-small-en-v1.5 | 384 | 33M | ~15ms/text | 0.662 |
| snowflake-arctic-embed-xs | 384 | 22M | ~10ms/text | 0.655 |
| nomic-embed-text-v1 | 768 | 137M | ~50ms/text | 0.704 |

**Assessment**: `all-MiniLM-L6-v2` at 384 dimensions is already the optimal
choice for the ECC use case. It is the smallest high-quality model available.
Reducing dimensions below 384 (e.g., Matryoshka truncation to 128 or 256)
would save memory but is not supported by the current ONNX model file.

The **real bottleneck** is not the model but the tokenizer and batch processing:
1. The hash tokenizer needs to be replaced with proper BPE for real semantic quality.
2. The `embed_batch` on `OnnxEmbeddingProvider` calls `embed()` in a serial
   loop (line 330-335). True ONNX batch inference (multiple sequences in one
   forward pass) would give 3-8x throughput improvement.

### 5.6 Recommendation: FIX then AUGMENT

**v0.1.x**: No change. The mock SHA-256 fallback is sufficient for initial
release. The ECC operates correctly with hash-based embeddings; semantic quality
is a nice-to-have.

**v0.2**:
1. Bundle the `all-MiniLM-L6-v2.onnx` model (22MB) or provide a download
   script in the build system.
2. Integrate a proper tokenizer (`tokenizers` crate from HuggingFace, or a
   pre-computed vocabulary lookup).
3. Implement true batch ONNX inference: pad sequences to common length, run
   a single forward pass for up to 16 texts.
4. Wire the `LlmEmbeddingProvider` to the actual clawft-llm provider layer
   for cloud deployments.

**v0.3**:
- Add Matryoshka dimension reduction (384 -> 128) for HNSW storage savings
  (3x memory reduction) while keeping 384-dim for similarity computation.
- Evaluate `gte-small` or `snowflake-arctic-embed-xs` as drop-in replacements
  with better quality scores.

**v1.0+**:
- GPU-accelerated ONNX inference (`ort` already supports CUDA/CoreML backends).
- Model distillation for a WeftOS-specific embedding model trained on the
  target domain (code + documentation + change events).

### 5.7 Version Target

See above (phased across v0.1.x through v1.0+).

### 5.8 ECC Contribution

Each embedding call produces one 384-dimensional vector stored in HNSW.
At 64 impulses per tick, the embedding phase produces **64 vectors * 384 dims
* 4 bytes = 98 KB** of embedding data per tick. Over 1000 ticks (50 seconds),
this accumulates to ~96 MB. Memory management for expired embeddings is not
currently implemented -- all entries persist indefinitely in the HNSW store.

---

## 6. Cross-Algorithm Interaction Matrix

The five algorithms form a pipeline within the DEMOCRITUS tick loop and
periodic analysis cycles. Their interactions affect optimization strategy:

```
IMPULSE QUEUE
    |
    v
EMBEDDING (5)  -->  HNSW SEARCH (3)  -->  CAUSAL GRAPH
    |                                         |
    |                                    COMMUNITY (2)
    |                                    SPECTRAL (1)
    |                                    PREDICTION (4)
    v
tick budget enforcement
```

| Optimization in... | Affects... | How |
|--------------------|-----------|-----|
| Embedding (smaller dims) | HNSW (faster distance, less memory) | Linear relationship: 50% dim reduction = 50% distance speedup |
| HNSW (lower ef_search) | DEMOCRITUS UPDATE (fewer neighbors) | Fewer edges to evaluate per impulse |
| Community (incremental) | Spectral (smaller subgraphs) | Run spectral per-community instead of whole graph |
| Spectral (sparse Lanczos) | Prediction (fresher lambda_2) | More frequent spectral monitoring enables better coherence alerts |
| Prediction (EMA) | HNSW (pre-warming) | Predicted nodes can be pre-fetched into HNSW cache |

**Key insight**: The highest-leverage optimization is at the **HNSW layer**
(item 3) because it sits on the critical path of every tick and its performance
issues are the most severe (O(n) upsert, full rebuild). Fixing HNSW alone
likely accounts for 60-70% of the achievable tick latency reduction.

---

## 7. Summary Table

| # | Algorithm | Current | Big-O | Recommendation | Version | ECC Nodes/Edges |
|---|-----------|---------|-------|----------------|---------|----------------|
| 1 | Spectral (lambda_2) | Dense Laplacian + power iteration | O(k*n^2) | REPLACE with sparse Lanczos O(k*m) | v0.2 | 1 node, O(n) edges |
| 2 | Community Detection | Weighted Label Propagation | O(k*m) | AUGMENT: add Louvain offline, incremental LPA | v0.3 | 0 nodes, 0 edges (metadata only) |
| 3 | HNSW Search | instant-distance v0.6, full rebuild | O(n) upsert, O(n*M*ef*log(n)) rebuild | REPLACE incrementally: HashMap index, deferred rebuild, adaptive ef | v0.1.x-v0.3 | 0 nodes, up to 320 edges/tick |
| 4 | Predictive Analysis | Windowed burst + coupling | O(E*K^2) | AUGMENT: add EMA burst, directional coupling | v0.3 | 0 (advisory output) |
| 5 | Embedding | Mock SHA-256 / ONNX (broken tokenizer) | O(d) per text | FIX: proper tokenizer + batch ONNX | v0.2 | 0 nodes (produces vectors for HNSW) |

---

## 8. Panel Consensus Notes

**Performance Engineer** (chair): The algorithmic priorities are clear. HNSW
fixes (Track 9 items B1, B2) are the single highest-impact change. Spectral
analysis is a time bomb at scale -- it must move to sparse Lanczos before any
graph exceeds 1K nodes. Community detection is already well-suited. Prediction
needs calibration data more than algorithm changes.

**Memory Specialist**: The embedding memory story is concerning. 384-dim float32
vectors at 64 impulses per tick accumulate without bound. Before optimizing the
algorithms, we need an eviction policy for the HNSW store. Suggestion: LRU or
TTL-based pruning of entries older than N ticks.

**ECC Analyst**: The spectral analysis code is mathematically correct but the
dense Laplacian is the wrong data structure for a sparse graph. The ECC causal
graph has average degree ~10 (5 forward + 5 reverse from DEMOCRITUS search_k=5).
At 10K nodes, the dense matrix wastes 99.9% of its entries on zeros. Sparse CSR
would reduce both memory and computation by ~100x.

**Matrix Optimizer**: The power iteration convergence for lambda_2 is not
guaranteed to be fast for graphs with clustered eigenvalues. The current code
uses 50 iterations which is generous, but for graphs where lambda_2 is close
to lambda_3, convergence can be very slow. Lanczos with implicit restart
(ARPACK-style) would be more robust. Consider the `sprs` + `arpack-ng` via
FFI, or implement the Lanczos recurrence directly (only ~50 lines of code
for the tridiagonal method).

**PageRank Analyzer**: The causal graph has natural PageRank-like structure:
highly-connected "hub" nodes that receive many causal edges. A lightweight
PageRank computation (O(k*m) same as LPA) would identify the most influential
nodes for the GUI's "focus" feature. Recommend adding this as a v0.2 analysis
alongside community detection. It shares the same sparse graph structure and
would benefit from the same sparse representation.

**Researcher**: The embedding tokenizer issue in the ONNX provider is a
correctness bug, not a performance issue. The hash-based "tokenizer" produces
arbitrary token IDs that do not correspond to the model's vocabulary. Either
fix the tokenizer or remove the ONNX inference path and keep only the hash
fallback (which is at least honest about what it computes). Shipping a model
with a broken tokenizer is worse than no model -- it gives false confidence
in semantic quality.

---

*Document generated by Sprint 11 Symposium Track 7 panel.*
*Methodology: Read actual implementation, analyze complexity, survey literature,
recommend per algorithm with version target.*
