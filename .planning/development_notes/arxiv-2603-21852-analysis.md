# Analysis: arXiv 2603.21852v2 -- "All elementary functions from a single operator"

**Paper**: Odrzywolel, A. "All elementary functions from a single operator."
arXiv:2603.21852v2 [cs.SC], April 2026. Jagiellonian University, Krakow.

## Summary

The paper discovers a single binary operator `eml(x,y) = exp(x) - ln(y)`
that, combined with the constant `1`, can reconstruct ALL elementary
mathematical functions: arithmetic (+, -, *, /), exponentials, logarithms,
trigonometry, roots, and complex constants (e, pi, i). This is the
continuous-mathematics analog of the NAND gate in Boolean logic.

The key contribution is threefold:

1. **Existence proof**: The EML operator is a "Sheffer operator" for
   continuous mathematics. Every expression forms a binary tree under
   the grammar `S -> 1 | eml(S,S)`.

2. **Gradient-based symbolic regression**: EML trees serve as trainable
   circuits. A "master formula" at depth d has `5*2^d - 6` parameters.
   Using softmax-constrained weights (alpha_i + beta_i + gamma_i = 1),
   standard optimizers (Adam) can recover exact closed-form elementary
   functions from numerical data, with weights snapping to {0,1}.

3. **Complexity analysis**: Functions have varying EML complexity (K =
   RPN code length). exp(x) = K=3, ln(x) = K=7, multiplication = K=17
   (direct search), pi = K>53.

## Mathematical Techniques

### Core Definition
```
eml(x,y) = exp(x) - ln(y)
```
Variants: `edl(x,y) = exp(x)/ln(y)` with constant e.

### Iterative Bootstrapping
Maintains two lists: S_i (verified primitives) and C_i (functions to
compute). Searches for expressions computing elements of C_i using only
S_i, iterating until C_i is empty. Uses Kolmogorov complexity K <= 9 as
search bound.

### Numerical Verification via Schanuel's Conjecture
Substitutes algebraically independent transcendental constants (Euler-
Mascheroni gamma, Glaisher-Kinkelin A) for variables, then checks
numerical equivalence. Coincidental equality between exp-log expressions
at such points is vanishingly unlikely.

### Trainable Circuit Architecture
Level-n master formula: each EML node's inputs are linear combinations
`alpha_i + beta_i*x + gamma_i*f` where f is a previous EML result.
Softmax normalization forces alpha+beta+gamma=1, making gradient descent
push weights toward exact {0,1} values.

### Experimental Results
- Depth 2: 100% recovery from random initialization
- Depth 3-4: ~25% recovery
- Depth 5: <1% recovery
- Depth 6: 0% recovery from random init
- With correct weights + Gaussian noise: 100% convergence at all depths

## Applicability to RuVector Libraries

### ruvector-diskann (DiskANN/Vamana)

**Low direct applicability.** DiskANN operates in high-dimensional
Euclidean/cosine space. The EML operator's domain is scalar
elementary functions, not vector distance computations.

However, one indirect connection exists: the **Product Quantization
codebook training** in DiskANN uses k-means, which involves iterative
distance optimization. The paper's master-formula approach could
theoretically provide a learned distance function that composes
exp/ln operations. This is speculative and not practical at current
complexity levels.

### ruvector-raft (Distributed Consensus)

**No applicability.** Raft is a discrete state machine protocol
(leader election, log replication, term management). The paper's
continuous-function representation has no bearing on consensus
algorithms.

### ruvector-cluster (Clustering/Sharding)

**No direct applicability.** Consistent hashing, shard routing, and
node management are discrete/combinatorial problems. The EML operator
does not offer improvements here.

### ruvector-replication (Data Replication)

**No applicability.** Replication is a systems-level concern
(data synchronization, conflict resolution) outside the paper's scope.

### ruvector-attention (Attention Mechanisms)

**MEDIUM applicability.** This is the most relevant ruvector crate.

1. **Activation function universality**: The paper shows that standard
   activation functions (ReLU, sigmoid, etc.) are special cases of EML
   trees. An EML-based activation function could serve as a universal
   replacement, but current complexity (depth 6+ has 0% convergence
   from random init) makes this impractical.

2. **Interpretable attention**: The paper's key insight is that trained
   EML weights snap to {0,1}, recovering exact closed-form expressions.
   If attention score computation were reformulated as shallow EML
   trees, the trained result would be interpretable as an explicit
   elementary function rather than opaque weights. This could be
   explored for the graph attention module.

3. **Symbolic regression of learned patterns**: After training
   ruvector-attention's mechanisms on data, one could use EML symbolic
   regression to reverse-engineer what function the attention learned,
   providing interpretability.

## Applicability to WeftOS Math

### HNSW Search (hnsw_service.rs)

**Low applicability.** HNSW search is a graph traversal + distance
computation problem. The EML operator does not improve distance
functions or greedy search heuristics. The search is dominated by
memory access patterns, not arithmetic complexity.

### Spectral Analysis / Community Detection (causal.rs)

**MEDIUM-HIGH applicability.** This is the most promising connection.

1. **Lanczos iteration optimization**: Our `spectral_analysis()` uses
   Lanczos iteration to compute lambda_2 (algebraic connectivity) of
   the graph Laplacian. The core operation is sparse matrix-vector
   multiplication `L*x = D*x - A*x`. The paper does not directly
   improve this, BUT:

2. **Fiedler vector interpretation**: After computing the Fiedler
   vector, we use its sign to partition the graph. The paper's symbolic
   regression could discover a closed-form approximation to the
   partition function -- given graph statistics (degree distribution,
   edge density), predict the optimal partition without full
   eigendecomposition. This would be a precomputation shortcut for
   `spectral_partition()`.

3. **Coherence scoring (lambda_2)**: Our coherence score IS algebraic
   connectivity. The paper shows that ALL elementary functions can be
   expressed as EML trees. If we could express our coherence score as
   an explicit function of observable graph features (|V|, |E|, max
   degree, avg degree), we could avoid eigendecomposition entirely for
   approximate coherence. The symbolic regression approach (Section 4.3)
   is the mechanism: train an EML master formula on (graph_features ->
   lambda_2) data.

   **Concrete proposal**: Collect (graph_features, lambda_2) pairs from
   our causal graphs during normal operation. Fit an EML depth-3 or
   depth-4 master formula to predict lambda_2 from features. If
   successful, this gives us O(1) coherence estimation instead of
   O(k*m) Lanczos iteration. At depth 3, the master formula has only
   34 parameters.

### Causal Graph Algorithms (Traverse, Path Finding, Coupling)

**Low applicability.** Graph traversal is combinatorial/discrete. The
EML operator is defined over continuous functions and does not improve
BFS/DFS/Dijkstra-type algorithms.

However, **coupling computation** (if it involves continuous weighting
functions on paths) could potentially benefit from a learned
closed-form coupling function via EML symbolic regression.

### ECC Calibration (calibration.rs)

**MEDIUM applicability.**

The calibration system computes p50/p95 percentiles and auto-selects
tick intervals from a set of bands. The mapping from p95 to optimal
tick band is currently a lookup table. If the tick selection needed to
be a smooth function (for adaptive/predictive calibration), EML symbolic
regression could discover the optimal mapping: `tick_interval = f(p95,
vector_count, edge_count)`.

More practically: the percentile estimation itself (sorting + indexing)
is not improvable by EML. But **predicting future p95 from historical
data** could use a learned EML function as a lightweight forecaster.

### Coherence Scoring (Algebraic Connectivity / lambda_2)

See the "Spectral Analysis" section above. The key proposal is:

**Train EML master formula to approximate lambda_2 from graph features,
avoiding eigendecomposition for fast approximate coherence checks.**

This is the single most actionable item from this paper for WeftOS.

### Graphify Analysis (Surprise Scoring, Question Generation, God Nodes)

**MEDIUM applicability.**

1. **Surprise scoring**: Our `surprise_score()` in graphify uses a
   hand-crafted linear combination of signals (confidence bonus +
   cross-file-type bonus + cross-repo bonus + cross-community bonus +
   peripheral-hub bonus + semantic similarity multiplier). The paper's
   symbolic regression approach could discover a non-linear composition
   that better predicts human-perceived "surprise." Train an EML master
   formula on (edge_features -> human_surprise_rating) data.

2. **God node detection**: Currently uses raw degree. A more
   sophisticated "god-ness" score combining degree, betweenness, and
   community membership could be learned via EML regression.

3. **Community cohesion**: The `cohesion_score()` function used by
   graphify could be replaced with a learned function that captures
   non-linear interactions between intra-community density and
   inter-community sparsity.

## Vector Search Backend Improvements (HNSW, DiskANN, Hybrid)

**Low direct applicability.** Vector search is dominated by:
- Memory access patterns (cache lines, prefetching, mmap)
- SIMD distance computation (L2, cosine, inner product)
- Graph navigation heuristics (greedy search, beam width)

The EML operator works on scalar functions and does not improve any of
these. The paper explicitly notes that EML has "exponential asymptotics"
that cause numerical issues -- the opposite of what you want in a
high-performance distance kernel.

One speculative application: **learned distance functions**. If a
domain-specific distance metric outperforms L2/cosine for a particular
knowledge graph embedding, EML symbolic regression could discover
the closed-form of that metric from training data. This is a research
direction, not a practical optimization.

## Key Takeaways for WeftOS

### Actionable (Worth Implementing)

1. **Lambda_2 approximation via EML regression** (coherence scoring):
   Collect training data from causal graph operations, fit a shallow
   EML formula to predict algebraic connectivity from graph statistics.
   Could convert O(k*m) Lanczos to O(1) for approximate checks.

2. **Learned surprise scoring**: Replace the hand-crafted linear
   surprise_score with a non-linear EML-discovered function trained on
   user feedback data.

### Worth Monitoring

3. **EML-based interpretable attention**: If ruvector-attention adds
   EML-tree attention, evaluate for graph attention in graphify.

4. **Symbolic regression of calibration curves**: If ECC calibration
   becomes adaptive/predictive, EML regression could discover the
   optimal tick-selection function.

### Not Applicable

5. Vector search distance computation
6. Consensus protocols
7. Data replication
8. Graph traversal algorithms
9. Shard routing

## Implementation Notes

If pursuing item (1) above (lambda_2 approximation):

- The EML compiler and Rust verification tool are available at the
  paper's GitHub repository.
- A depth-3 master formula has 34 parameters; depth-4 has 74.
- The paper uses PyTorch with `torch.complex128` for training.
- Key challenge: EML uses complex intermediates (for pi, trig via
  Euler's formula). For real-valued lambda_2 approximation, this is
  not needed -- the formula would stay in the real domain.
- The `ruvector-solver` crate (Neumann solver, CG solver) could be
  useful for the inner training loop if a custom optimizer is needed.
- Success metric: MSE between predicted and actual lambda_2, measured
  on held-out causal graphs.

## Paper Quality Assessment

The paper is mathematically rigorous with constructive proofs. The
symbolic regression experiments are honest about limitations (0%
convergence at depth 6 from random init). The core EML discovery is
verified both symbolically (Mathematica) and numerically (C, NumPy,
PyTorch, mpmath). The Rust re-implementation provides fast verification.

The main weakness is practical applicability -- the EML operator has
exponential asymptotics that cause numerical overflow in deep trees,
and complex-domain intermediates add implementation complexity. The
author acknowledges these as open problems and suggests searching for
operator variants with better numerical properties.
