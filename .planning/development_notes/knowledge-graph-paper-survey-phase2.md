# Knowledge Graph Paper Survey -- Phase 2

**Date**: 2026-04-04
**Surveyor**: Research agent (claude-opus-4-6)
**Scope**: 7 arxiv papers (2604.xxxxx series, April 2026) evaluated against
WeftOS graphify, ECC, HNSW, EML, and sonobuoy subsystems.

---

## Codebase Anchors (current state read from source)

| Subsystem | Crate / file | Current technique |
|-----------|-------------|-------------------|
| KG builder | `clawft-graphify` | AST extraction, label-prop community detection, god-node scoring, surprise scoring |
| KG model | `clawft-graphify/model.rs` | petgraph DiGraph, Entity + Relationship nodes, Hyperedge support |
| Cross-file | `clawft-graphify/extract/cross_file.rs` | Two-pass import resolution, BFS stem->class mapping |
| ECC causal DAG | `clawft-kernel/causal.rs` | DashMap concurrent DAG, typed/weighted edges (Causes/Inhibits/Correlates/etc.) |
| Causal predict | `clawft-kernel/causal_predict.rs` | O(1) delta-lambda2 via first-order perturbation, evidence ranking, cycle detection |
| DEMOCRITUS | `clawft-kernel/democritus.rs` | SENSE->EMBED->SEARCH->UPDATE->COMMIT tick loop |
| Cognitive tick | `clawft-kernel/cognitive_tick.rs` | Adaptive tick interval, two-tier EML/Lanczos coherence |
| HNSW service | `clawft-kernel/hnsw_service.rs` | Multi-key indexing, ef_search=100, ef_construction=200, 384d default |
| HNSW EML | `clawft-kernel/hnsw_eml.rs` | 4 learned models: distance (progressive dim), ef (adaptive beam), path (entry-point), rebuild |
| EML core | `eml-core/model.rs` | Multi-head O(1) function approx, coordinate descent training, depth 2-5 trees |
| EML coherence | `clawft-kernel/eml_coherence.rs` | O(1) lambda_2 prediction from GraphFeatures, exp(x)-ln(y) master formula |
| Graphify bridge | `clawft-graphify/bridge.rs` | Maps KG entities into CausalGraph + HNSW + CrossRefStore |
| Semantic extract | `clawft-graphify/semantic_extract.rs` | LLM-based entity/relationship extraction from non-code docs |

---

## Paper 1: RoMem -- Continuous Phase Rotation for Temporal KGs

### Paper 1: RoMem

- **Authors**: Weixian Waylon Li, Jiaxin Zhang, Xianan Jim Yang, Tiejun Ma, Yiwen Guo
- **ArXiv**: 2604.11544
- **Key Contribution**: Treats time as continuous phase rotation in complex
  vector space rather than discrete metadata. Obsolete facts rotate out of
  phase ("geometric shadowing") so temporally correct answers naturally rank
  higher -- no deletion needed.
- **Techniques Introduced**:
  - **Phase rotation**: theta_r(tau) = s * alpha_r * tau * omega.
    Scoring: s_kge = sum_c <v_r^c(e_h, tau), v_r^c(e_t, tau)>. O(kd) per
    fact at inference.
  - **Semantic speed gate**: alpha_r = sigma(MLP(phi(r))). Maps relation
    embeddings to volatility [0,1]. Self-supervised pretraining via BCE on
    temporal transitions. "president_of" rotates fast; "born_in" stays
    locked.
  - **Geometric shadowing**: Phase difference proportional to |tau_query -
    t_happen| causes natural score decay for stale facts without deletion.
  - **Temporal contrastive loss**: Listwise ranking with Gaussian kernel
    favoring validity-center timestamps.
- **Results**: 72.6 MRR on ICEWS05-15 (SOTA), 2-3x MRR on MultiTQ, zero
  degradation on static facts.

- **WeftOS Applicability**:
  - **Module**: `clawft-kernel/causal.rs`, `clawft-kernel/causal_predict.rs`,
    `clawft-graphify/model.rs`
  - **Current approach**: CausalEdge has an `hlc_timestamp` field and typed
    edges (Causes, Inhibits, etc.) but no temporal decay or volatility
    scoring. Evidence ranking uses static Fiedler-vector perturbation.
    `CausalGraph::prune_old_edges` does hard deletion by age.
  - **Paper's improvement**: Replace hard age-pruning with geometric
    shadowing on the CausalGraph. Each CausalEdgeType gets a learned
    volatility alpha via the semantic speed gate. "Correlates" edges would
    rotate fast (ephemeral statistical relationships), while "Causes" edges
    rotate slowly (stable causal assertions). The EML coherence model could
    incorporate phase-adjusted edge weights into the lambda_2 prediction,
    making the DEMOCRITUS loop temporally aware without increasing tick
    cost.
  - **Expected impact**: Eliminates false-positive causal pruning (current
    hard cutoff loses valid old evidence). Based on RoMem's 2-3x MRR
    improvement on agentic memory tasks, expect measurable improvement in
    the ECC's evidence retrieval accuracy during multi-session
    conversations.
  - **Implementation difficulty**: M -- Requires complex-valued
    representations for causal edges (currently f32 weights). The speed
    gate MLP can be approximated by an EML model (already have the
    infrastructure in eml-core). Phase rotation itself is O(1) per edge,
    fitting within the 15ms tick budget.
- **Priority**: **P1** -- High value for ECC temporal reasoning, but
  requires careful integration with the existing Fiedler-perturbation
  prediction pipeline.

---

## Paper 2: TRACE -- Experiential Multi-hop KGQA

### Paper 2: TRACE

- **Authors**: Yingxu Wang, Jiaxin Huang, Mengzhu Wang, Nan Yin
- **ArXiv**: 2604.11193
- **Key Contribution**: Unifies LLM reasoning with knowledge graph traversal
  via three mechanisms: dynamic path narration (converting traversal into
  natural language), experiential priors (reusable exploration patterns),
  and dual-feedback re-ranking.
- **Techniques Introduced**:
  - **Dynamic context generation**: C_q^(t) = f_ctx(q, R_q^(t)) --
    converts evolving relation sequences into narrative text to preserve
    semantic continuity across hops.
  - **Exploration generalization**: Abstracts failed/successful
    trajectories into reusable prior patterns F = f_gen({D_q}).
  - **Dual-feedback re-ranking**: Stage 1 retrieval + Stage 2 scoring
    s(r) = f_rank(q, R_q^(t), r, F). Beam-search style with k=3-4
    candidates, max 4 hops, 30 iterations.
  - **Token efficiency**: 8,782 tokens vs 10,680 baseline per question.
- **Results**: 91.6% Hits@1 on WebQSP, 76.9% on CWQ. Outperforms RwT by
  +4.6% / +4.5% respectively. 18% fewer API calls.

- **WeftOS Applicability**:
  - **Module**: `clawft-graphify/analyze.rs` (question generation),
    `clawft-kernel/democritus.rs` (SEARCH phase),
    `clawft-graphify/bridge.rs` (traversal)
  - **Current approach**: Graphify generates questions from god-nodes and
    surprising connections but does not perform multi-hop traversal or
    maintain exploration priors. The DEMOCRITUS SEARCH phase does
    single-hop HNSW nearest-neighbor lookup.
  - **Paper's improvement**: Add a multi-hop traversal mode to the
    GraphifyBridge that uses accumulated exploration patterns as priors.
    The key insight is the *experiential prior* -- WeftOS could maintain a
    persistent store of traversal patterns (which edge-type sequences
    successfully resolved past queries) and reuse them to prune future
    searches. This directly applies to the `rank_candidates` function in
    causal_predict.rs, which currently ranks edges independently.
  - **Expected impact**: For the WeftOS "question generation" feature in
    graphify, TRACE's dynamic narration approach could improve question
    quality by 10-20%. For ECC multi-hop causal reasoning, exploration
    priors would reduce redundant graph traversals. Token savings of ~18%
    are directly relevant to clawft-llm cost optimization.
  - **Implementation difficulty**: M -- The beam-search traversal is
    straightforward to implement in Rust. The experiential prior store
    maps naturally to the existing CrossRefStore pattern. The LLM
    dependency for narration is already available via clawft-llm.
- **Priority**: **P1** -- Multi-hop reasoning is a gap in the current
  graphify/ECC pipeline. TRACE's architecture maps cleanly onto existing
  WeftOS patterns.

---

## Paper 3: SevenNet-Nano -- Lightweight Universal ML Interatomic Potential

### Paper 3: SevenNet-Nano

- **Authors**: Sangmin Oh, Jinmu You, Jaesun Kim, Jiho Lee, Hyungmin An,
  Seungwu Han, Youngho Kang
- **ArXiv**: 2604.10887
- **Key Contribution**: Knowledge distillation from a large GNN teacher
  model to a compact student model for atomistic simulation, achieving
  >10x speedup with minimal accuracy loss.
- **Techniques Introduced**:
  - **GNN knowledge distillation**: Teacher (SevenNet-Omni) trained on
    diverse materials datasets; student (Nano) learns from teacher
    predictions rather than raw data.
  - **Unified chemical representation**: Preserves transferability across
    chemical systems through shared embedding space.
  - **Scalable simulation**: Enables thousands-of-atoms MD simulations
    with minimal fine-tuning.
- **Results**: >10x computational speedup, strong transferability across
  equilibrium and extreme conditions.

- **WeftOS Applicability**:
  - **Module**: `eml-core/model.rs` (model architecture),
    `clawft-kernel/hnsw_eml.rs` (learned HNSW optimization)
  - **Current approach**: EML models use gradient-free coordinate descent
    with 30-80 parameters. No knowledge distillation between EML model
    instances.
  - **Paper's improvement**: The knowledge distillation paradigm is
    directly applicable to WeftOS's EML model lifecycle. When a
    high-fidelity EML model (e.g., the coherence model trained with many
    Lanczos ground-truth samples) exists, it could serve as a teacher for
    a lighter student model deployed on edge/WASM targets where the full
    model is too expensive. This is especially relevant for the 4
    HNSW-EML models (distance, ef, path, rebuild) -- a well-trained
    instance could distill into a smaller-depth tree for clawft-wasm.
  - **Expected impact**: 5-10x inference speedup for WASM-deployed EML
    models. Model size reduction from depth-4 (50+ params) to depth-2
    (20 params) with teacher-guided training instead of cold-start
    coordinate descent.
  - **Implementation difficulty**: S -- The EML model already supports
    multiple depths and the `record()`/`train()` interface. Distillation
    is simply: run teacher on input grid, use teacher outputs as training
    targets for student. Requires ~50 lines of new code in eml-core.
- **Priority**: **P2** -- Useful optimization for WASM/edge deployment
  but not blocking any feature. Implement when WASM perf becomes a
  bottleneck.

---

## Paper 4: SGKR -- Structure-Grounded Knowledge Retrieval via Code Dependencies

### Paper 4: SGKR

- **Authors**: Xinyi Huang, Mingzhe Lu, Haoyu Dong
- **ArXiv**: 2604.10516
- **Key Contribution**: Replaces embedding-similarity retrieval with
  code-dependency-graph-based retrieval for multi-step data reasoning.
  Builds a directed graph from AST-extracted function calls, then uses BFS
  path finding between semantic I/O tags to retrieve structurally relevant
  code rather than textually similar code.
- **Techniques Introduced**:
  - **AST-based dependency graph**: Parse source into AST, extract
    function definitions, traverse each function's AST for call sites,
    create directed edges caller->callee.
  - **Semantic I/O tag extraction**: Map query keywords to graph nodes
    representing data entities, attributes, and derived quantities.
  - **BFS path retrieval**: From input semantic nodes to output semantic
    nodes, discovering computational pipelines.
  - **Subgraph assembly**: Union of all discovered paths forms the
    context for LLM code generation.
- **Results**: Consistent improvement over embedding-based and no-retrieval
  baselines across multi-step analysis benchmarks.

- **WeftOS Applicability**:
  - **Module**: `clawft-graphify/extract/ast.rs`,
    `clawft-graphify/extract/cross_file.rs`,
    `clawft-graphify/bridge.rs`
  - **Current approach**: Graphify already does AST extraction
    (tree-sitter based, with lang-specific extractors for Rust, Python,
    JS, Go) and cross-file import resolution. However, the current
    cross-file resolver only handles Python `from .module import Name`
    patterns (GRAPH-011). The resulting graph is used for visualization
    and question generation, not for retrieval-augmented code generation.
  - **Paper's improvement**: SGKR's BFS-over-dependency-graph retrieval
    is a natural extension of graphify's existing cross-file module. The
    key missing piece is: (a) extending cross-file resolution beyond
    Python to Rust/JS/Go (the extractors exist but cross-file linking
    doesn't), and (b) adding a `retrieve_pipeline(input_tag,
    output_tag)` method to KnowledgeGraph that returns the subgraph of
    function nodes connecting the two tags via call dependencies.
  - **Expected impact**: This is a *high-impact* improvement for WeftOS's
    core value proposition of understanding/documenting client systems.
    Current graphify can show you *what* exists; SGKR enables showing
    *how data flows* from input to output. For the "automate client
    systems" GTM use case, this is the difference between a static map
    and an executable understanding.
  - **Implementation difficulty**: M -- The AST infrastructure exists.
    BFS is trivial on petgraph. The main work is extending cross-file
    resolution to all 4 supported languages and adding semantic I/O tag
    extraction (can bootstrap from existing entity metadata).
- **Priority**: **P0** -- Directly advances the GTM priority of
  system understanding/automation. The graphify crate already has 80% of
  the infrastructure; this paper provides the missing retrieval algorithm.

---

## Paper 5: CodaRAG -- Associative Retrieval via Complementary Learning Systems

### Paper 5: CodaRAG

- **Authors**: Cheng-Yen Li, Xuanjun Chen, Claire Lin, Wei-Yu Chen,
  Wenhua Nie, Hung-Yi Lee, Jyh-Shing Roger Jang
- **ArXiv**: 2604.10426
- **Key Contribution**: Three-stage RAG pipeline inspired by Complementary
  Learning Systems (CLS): knowledge consolidation (entity merging into
  stable KG), associative navigation (three-pathway retrieval), and
  interference elimination (filtering irrelevant associations).
- **Techniques Introduced**:
  - **Entity type discovery**: Suggest-refine paradigm for schema-free
    entity typing. Document-level proposals aggregated into shared
    inventory.
  - **Fragmented entity merging**: Embedding-similarity gate forwards
    high-confidence pairs to LLM judge for dedup (Google vs Google Inc.
    vs Alphabet's Google).
  - **Three-pathway associative navigation**:
    1. Semantic: S_SemA(e) = alpha*sim_rel + beta*sim_ent (local
       neighborhood expansion)
    2. Contextualized: Personalized PageRank with damping d=0.85
       (global saliency via iterative message passing)
    3. Functional: Fast Random Projection embeddings for topological
       analogy (higher-order relational patterns)
  - **Interference elimination**: LLM-based filtering of retrieved
    entities against query relevance.
- **Results**: +7-10% retrieval recall, +3-11% generation accuracy on
  GraphRAG-Bench (factual, reasoning, creative tasks).

- **WeftOS Applicability**:
  - **Module**: `clawft-graphify/entity.rs` (entity dedup),
    `clawft-graphify/cluster.rs` (community detection),
    `clawft-kernel/hnsw_service.rs` (retrieval),
    `clawft-kernel/democritus.rs` (SEARCH phase)
  - **Current approach**: Graphify does entity extraction without
    automated dedup (entities from different files with the same name
    are separate nodes unless cross-file resolution catches them).
    HNSW search is single-pathway cosine similarity. No PageRank or
    topological retrieval.
  - **Paper's improvement**: Three specific adoptable techniques:
    1. **Entity merging**: Add an embedding-similarity pre-filter to
       graphify's entity pipeline. Before inserting a new entity, check
       HNSW for near-duplicates (threshold ~0.92 cosine). This piggybacks
       on the existing multi-key HNSW infrastructure.
    2. **Personalized PageRank**: Add PPR to graphify's KnowledgeGraph
       (petgraph supports iterative matrix operations). Use as a
       secondary ranking signal alongside HNSW cosine similarity in the
       DEMOCRITUS SEARCH phase. The damping factor d=0.85 is standard.
    3. **Fast Random Projection**: Replace or augment HNSW's learned
       distance model (hnsw_eml.rs distance model) with FRP-based
       structural embeddings for topological analogy retrieval.
  - **Expected impact**: Entity dedup alone would reduce graphify graph
    sizes by 10-30% on real-world codebases (common pattern: same class
    referenced across many files). PPR-augmented retrieval would improve
    the DEMOCRITUS SEARCH phase recall by the 7-10% reported. The
    three-pathway approach makes retrieval robust to different query types.
  - **Implementation difficulty**: M -- Entity dedup is straightforward
    with existing HNSW. PPR requires ~100 lines of iterative computation
    on petgraph. FRP structural embeddings are a larger lift (~200 lines).
- **Priority**: **P1** -- Entity dedup is a quick win (S difficulty for
  high impact). PPR integration requires more design but aligns with
  the ECC's multi-signal philosophy.

---

## Paper 6: TransFIR -- Inductive Reasoning for TKGs with Emerging Entities

### Paper 6: TransFIR

- **Authors**: Ze Zhao, Yuhui He, Lyuwen Wu, Gu Tang, Bin Lu, Xiaoying
  Gan, Luoyi Fu, Xinbing Wang, Chenghu Zhou
- **ArXiv**: 2604.10164 (Accepted at ICLR 2026)
- **Key Contribution**: Handles entities that appear in temporal KGs
  without any training history. Uses a codebook-based classifier to assign
  emerging entities to latent semantic clusters, then transfers interaction
  chain patterns from known entities in the same cluster.
- **Techniques Introduced**:
  - **Codebook vector quantization**: Learnable codebook C = {c_1,...,c_K}.
    pi(e) = argmin_k ||h_e - c_k||^2. Frozen BERT embeddings ensure
    emerging entities can be encoded without interaction history.
  - **Interaction chain encoding**: Chronological past interactions
    filtered by relation similarity, encoded via Transformer with
    relation-guided attention.
  - **Cluster-pooled pattern transfer**: Dynamic prototypes
    c^dyn_k = mean(h_e^IC for e in cluster k). Transfer vector
    omega_e = Psi(z_e). Updated repr: h_tilde_e = h_e + omega * c^dyn.
  - **Anti-collapse**: Codebook + commitment losses prevent
    representation collapse (collapse ratio 0.0055 -> 0.8677).
- **Results**: 28.6% average MRR improvement across ICEWS14/18/05-15 and
  GDELT. 50.5% MRR improvement on GDELT alone. Accepted at ICLR 2026.

- **WeftOS Applicability**:
  - **Module**: `clawft-kernel/causal.rs` (new node handling),
    `clawft-kernel/hnsw_service.rs` (multi-key indexing),
    `clawft-kernel/democritus.rs` (UPDATE phase),
    `clawft-graphify/cluster.rs` (community detection)
  - **Current approach**: When a new entity appears in the CausalGraph
    (e.g., a new code module added to a project), it starts with zero
    edges and no historical context. The DEMOCRITUS loop discovers
    connections via HNSW nearest-neighbor search, but the initial
    embedding is the only signal. In graphify, community detection via
    label propagation assigns new nodes based on immediate neighbors --
    if a node has no edges, it becomes an isolate community of one.
  - **Paper's improvement**: The codebook classifier solves the
    cold-start problem for new entities in the ECC. Implementation:
    1. Maintain a codebook of K=64 semantic cluster prototypes in HNSW
       (separate from the main index).
    2. When a new causal node is created, classify it via nearest
       codebook entry. Inherit the cluster's dynamic prototype
       (aggregated interaction chains from known cluster members).
    3. Use the transferred pattern to *pre-populate* causal edges with
       predicted weights, which the DEMOCRITUS loop then validates or
       prunes via the normal coherence cycle.
    This is especially valuable for WeftOS's GTM use case: when graphify
    scans a new codebase module, TransFIR's codebook can immediately
    predict likely relationships based on similar modules seen before.
  - **Expected impact**: 28.6% MRR improvement on entity-level prediction
    tasks. Eliminates the "cold start" period where new entities are
    poorly connected. For graphify, this means faster convergence to
    useful knowledge graphs when scanning incrementally updated codebases.
  - **Implementation difficulty**: L -- Requires Transformer-based
    interaction chain encoding (currently no Transformer inference in
    WeftOS). The codebook classifier itself is simple VQ, implementable
    in ~150 lines. A pragmatic shortcut: use the existing EML model as
    the pattern transfer mechanism instead of a Transformer, losing some
    accuracy but fitting the existing architecture.
- **Priority**: **P1** -- The cold-start problem is real in production
  graphify usage. The codebook-only approach (without the full
  Transformer chain encoding) is a tractable P1; the full pipeline is P2.

---

## Paper 7: K-STEMIT -- Spatio-Temporal GNN for Subsurface Estimation from Radar

### Paper 7: K-STEMIT

- **Authors**: Zesheng Liu, Maryam Rahnemoonfar
- **ArXiv**: 2604.09922
- **Key Contribution**: Multi-branch GNN that decouples spatial and temporal
  feature learning for estimating ice layer thickness from radar data.
  Integrates atmospheric model (MAR) physical priors via node features.
  Achieves 15.23% RMSE reduction vs SOTA at 2.4x speed.
- **Techniques Introduced**:
  - **Dual-branch architecture**: Spatial branch (GraphSAGE on geographic
    proximity graphs) + Temporal branch (gated temporal convolution with
    GLU activation).
  - **Adaptive feature fusion**: Learnable scalar alpha in [0,1] balances
    h = alpha*h_spatial + (1-alpha)*h_temporal.
  - **Physics-informed node features**: Surface mass balance, temperature,
    meltwater refreezing, melt-induced height change, snowpack depth from
    MAR climate model integrated via 2D Delaunay triangulation interpolation.
  - **GraphSAGE spatial processing**: Neighborhood aggregation on fully
    connected graphs with haversine-distance edge weights.
    x'(v) = W1*x(v) + W2*AGG_{u in N(v)} x(u).
  - **Gated temporal convolution**: Three parallel 2D convolutions ->
    GLU gating: h_temporal = ReLU(P * sigma(Q) + R).
  - **Dimensionality reduction**: Spatial input concatenates across time;
    temporal input strips static geographic coords, retains dynamic
    features.

- **WeftOS Applicability (general)**:
  - **Module**: `clawft-kernel/mesh_*.rs` (distributed mesh),
    `clawft-kernel/hnsw_service.rs` (spatial indexing),
    `clawft-kernel/eml_coherence.rs` (feature fusion)
  - **Current approach**: The WeftOS mesh subsystem uses TCP/mDNS/Kademlia
    for peer discovery and heartbeat-based liveness. No spatial feature
    learning. The HNSW index treats all dimensions equally (no spatial vs
    temporal decomposition).
  - **Paper's improvement**: The dual-branch spatial-temporal decoupling
    maps onto WeftOS's need to separate *structural* graph features
    (node degree, community membership -- "spatial") from *temporal*
    features (edge age, interaction frequency -- "temporal") in the EML
    coherence model. Currently, GraphFeatures extracts 7 features that
    mix structural and temporal information. Decomposing into two
    branches with adaptive alpha fusion could improve lambda_2 prediction
    accuracy.
  - **Expected impact**: 10-15% improvement in EML coherence prediction
    (based on the 15-21% RMSE improvements K-STEMIT reports).
  - **Implementation difficulty**: M -- The EML model already supports
    multi-head output; adding a branch-fusion layer requires extending
    the EmlModel architecture in eml-core.
- **Priority (general)**: **P2** -- Useful refinement of EML coherence
  but not urgent.

### Paper 7: Sonobuoy/Underwater Sensor Analysis

This is the most directly relevant paper for the sonobuoy project. While
K-STEMIT addresses *ice-penetrating radar*, the techniques map onto
*underwater acoustic sensing* with strong parallels:

#### Radar-to-Acoustic Signal Processing Mapping

| Radar (K-STEMIT) | Acoustic (Sonobuoy) | Mapping strength |
|-------------------|----------------------|-------------------|
| Radargram (2D backscatter image) | Spectrogram (2D time-frequency) | **Strong** -- both are 2D representations of 1D signal returns |
| Ice layer boundaries (reflections) | Species vocalizations (acoustic signatures) | **Strong** -- both are pattern recognition in noisy returns |
| Speckle noise + acquisition artifacts | Ocean ambient noise + multipath | **Strong** -- both degrade signal quality |
| S-C band (6 GHz bandwidth) | Sonar bands (1 Hz - 200 kHz) | **Moderate** -- different physics but same signal processing abstractions |
| Snow Radar flight lines (spatial track) | Buoy array geometry (spatial distribution) | **Strong** -- both are spatially distributed sensor networks |
| MAR atmospheric model (physical priors) | Ocean acoustic propagation model (physical priors) | **Very strong** -- both inject domain physics into ML |

#### Specific Techniques Applicable to Sonobuoy

1. **GraphSAGE spatial processing on buoy array geometry**:
   K-STEMIT builds fully-connected graphs from 256 radar trace points
   using haversine distance. For the distributed buoy array, construct
   a graph where each buoy is a node, edges weighted by acoustic
   propagation delay (function of distance, sound speed profile, and
   bathymetry). GraphSAGE's neighborhood aggregation
   (x'(v) = W1*x(v) + W2*AGG x(u)) then learns spatial features that
   encode array geometry, enabling the model to implicitly learn
   beamforming-like spatial filtering without explicit beamformer design.

2. **Gated temporal convolution for acoustic time series**:
   The GLU-gated temporal branch directly applies to sonobuoy
   hydrophone time series. Each buoy produces continuous acoustic data;
   the gated convolution (P * sigma(Q) + R) can learn to extract
   transient acoustic events (whale calls, ship propeller signatures,
   sonar pings) while suppressing stationary noise. This is analogous
   to matched filtering but learned from data rather than designed.

3. **Adaptive spatial-temporal fusion for detection/classification**:
   The learnable alpha = [0,1] that balances spatial and temporal
   branches is directly applicable to the sonobuoy's dual task:
   - **Detection** (is something there?) is primarily temporal -- a
     single buoy can detect transient energy.
   - **Bearing estimation** is primarily spatial -- requires
     cross-correlation across the array.
   - **Species ID** requires both -- spectral signature (temporal) +
     source location (spatial) for disambiguation.
   The adaptive alpha lets the model learn to weight these differently
   for different targets.

4. **Physics-informed node features from ocean models**:
   K-STEMIT integrates 5 MAR atmospheric variables as node features.
   For sonobuoy, inject:
   - Sound speed profile (depth-dependent, from CTD or climatology)
   - Thermocline depth (affects propagation paths)
   - Sea state / wind speed (affects surface noise)
   - Current velocity (affects bearing estimation via Doppler)
   - Bottom type (affects bottom-bounce propagation)
   These physical priors would be integrated via the same Delaunay
   triangulation interpolation K-STEMIT uses.

5. **Dimensionality reduction strategy**:
   K-STEMIT strips static geographic coordinates from the temporal
   branch, concatenates them only into the spatial branch. For
   sonobuoy: strip buoy GPS positions from the acoustic feature
   branch (they don't change per sample), keep only dynamic
   acoustic features (spectral energy, zero-crossing rate, mel
   coefficients). This prevents the temporal model from overfitting
   to buoy identity rather than acoustic content.

#### Detection Range / Species ID / Bearing Estimation Impact

- **Detection range**: K-STEMIT's physics-informed approach achieved 21%
  RMSE reduction over pure data-driven baselines. For sonobuoy, injecting
  sound speed profile and thermocline data as node features should
  improve detection range estimation by correcting for propagation loss
  that pure ML models cannot learn from acoustic data alone.

- **Species ID**: The gated temporal convolution with GLU activation
  provides learned matched filtering. Unlike fixed-template matched
  filters (which require a priori species call libraries), the learned
  filter can generalize to intra-species call variation. Combined with
  GraphSAGE's spatial context (nearby buoys seeing the same source at
  different angles), this should improve species ID accuracy for
  vocalizing species.

- **Bearing estimation**: GraphSAGE's neighborhood aggregation on the
  array geometry graph implicitly learns time-difference-of-arrival
  (TDOA) relationships. The haversine-weighted edges encode propagation
  delays. For a distributed array where buoy positions drift, this is
  superior to conventional TDOA beamforming because it adapts to the
  actual (possibly irregular) array geometry rather than assuming a
  fixed array.

#### Beamforming / Array Processing Applicability

K-STEMIT does **not** explicitly introduce beamforming or array processing
techniques. However, its GraphSAGE spatial processing on
haversine-weighted edges is *functionally equivalent to learned
beamforming*:

- Conventional delay-and-sum beamforming: y = sum_i w_i * x_i(t - tau_i)
  where tau_i are steering delays and w_i are array weights.
- GraphSAGE aggregation: x'(v) = W1*x(v) + W2*AGG_{u in N(v)} x(u)
  where AGG is a learned function over weighted neighbors.

The key advantage: GraphSAGE learns the aggregation function from data,
which can capture non-linear propagation effects (refraction, multipath,
scattering) that linear delay-and-sum beamforming cannot. For a
distributed buoy array with irregular geometry and time-varying sound
speed profiles, this learned aggregation may outperform conventional
beamforming.

**Limitation**: K-STEMIT's graphs are fully connected (all 256 nodes
interconnected). For a buoy array with N=20-100 buoys, full connectivity
is feasible. For larger arrays, a k-nearest-neighbors graph (k=8-16)
would be more computationally efficient while preserving local spatial
structure.

- **Priority (sonobuoy)**: **P0** -- The dual-branch spatio-temporal
  architecture is the highest-value contribution for the sonobuoy
  project. It provides a unified framework for detection, bearing
  estimation, and species ID that replaces three separate signal
  processing pipelines with one learned model.

---

## Cross-Paper Synthesis

### Synergy Map

```
SGKR (P4) --> graphify/extract     AST dependency retrieval
  |                                  (extends existing cross-file)
  v
CodaRAG (P5) --> graphify/entity   Entity dedup + PPR retrieval
  |              hnsw_service        (three-pathway search)
  v
RoMem (P1) --> causal.rs           Temporal phase rotation
  |            causal_predict.rs     (replaces hard age-pruning)
  v
TransFIR (P6) --> causal.rs        Cold-start entity handling
  |               cluster.rs        (codebook-based classification)
  v
TRACE (P2) --> democritus.rs       Multi-hop with exploration priors
  |            analyze.rs            (beam-search traversal)
  v
K-STEMIT (P7) --> eml_coherence    Spatial-temporal decomposition
                  sonobuoy stack    (dual-branch GNN)
                  mesh_*.rs
```

### Priority-Ordered Implementation Roadmap

| Priority | Paper | First deliverable | Effort | Crate |
|----------|-------|-------------------|--------|-------|
| **P0** | SGKR (4) | `KnowledgeGraph::retrieve_pipeline(in, out)` | M | clawft-graphify |
| **P0** | K-STEMIT (7) | Dual-branch spatio-temporal model for sonobuoy | L | new sonobuoy crate |
| **P1** | CodaRAG (5) | Entity dedup via HNSW near-duplicate check | S | clawft-graphify |
| **P1** | RoMem (1) | Geometric shadowing for CausalEdge scoring | M | clawft-kernel |
| **P1** | TRACE (2) | Exploration prior store + multi-hop traversal | M | clawft-graphify |
| **P1** | TransFIR (6) | Codebook classifier for new causal nodes | M | clawft-kernel |
| **P2** | SevenNet (3) | EML knowledge distillation (teacher->student) | S | eml-core |
| **P2** | K-STEMIT (7) | Branch-fusion in EmlModel architecture | M | eml-core |

### Combined Impact Estimate

If all P0+P1 items are implemented:
- **Graphify retrieval**: Dependency-graph retrieval (SGKR) + entity dedup
  (CodaRAG) + multi-hop traversal (TRACE) = fundamentally more useful
  knowledge graphs for the GTM use case.
- **ECC temporal reasoning**: Phase rotation (RoMem) + cold-start handling
  (TransFIR) = robust causal DAG that handles both temporal dynamics and
  new entity appearance.
- **Sonobuoy**: Dual-branch spatio-temporal GNN (K-STEMIT) replaces ad-hoc
  signal processing with a unified learned model.

### What NOT to Adopt

- **SevenNet-Nano's GNN architecture** (Paper 3): The graph neural network
  itself is domain-specific to atomistic simulation. Only the distillation
  *methodology* transfers.
- **TRACE's LLM-narration dependency** (Paper 2): The full dynamic context
  generation requires LLM calls per hop. For real-time DEMOCRITUS ticks
  (15ms budget), use a lightweight embedding-based narration instead.
- **CodaRAG's LLM-based interference elimination** (Paper 5): Too slow
  for the cognitive tick loop. Use EML-based filtering instead.
- **TransFIR's full Transformer chain encoding** (Paper 6): No
  Transformer runtime in WeftOS currently. Use EML approximation for
  the pattern transfer step.

---

## ADR Candidates

Based on this survey, the following ADRs should be drafted:

1. **ADR-049: Temporal Phase Rotation for CausalEdge** -- adopt RoMem's
   geometric shadowing, deprecate hard age-pruning.
2. **ADR-050: Dependency-Graph Retrieval in Graphify** -- adopt SGKR's
   BFS-over-AST-dependencies for pipeline retrieval.
3. **ADR-051: Entity Deduplication via HNSW Pre-filter** -- adopt
   CodaRAG's embedding-similarity gate for entity merging.
4. **ADR-052: Codebook Cold-Start for Emerging Entities** -- adopt
   TransFIR's VQ codebook for new CausalGraph nodes.
5. **ADR-053: Spatio-Temporal Dual-Branch Architecture for Sensor
   Systems** -- adopt K-STEMIT's architecture for the sonobuoy project.
