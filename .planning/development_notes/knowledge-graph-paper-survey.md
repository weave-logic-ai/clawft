# Knowledge Graph Research Paper Survey for WeftOS Graphify

**Date**: 2026-04-04
**Scope**: Papers relevant to graphify analyze, causal.rs, causal_predict.rs, cluster.rs, HNSW, and the broader WeftOS knowledge graph stack.

---

## WeftOS Modules Reference

Before the survey, a brief map of which modules exist and what they do, so paper-to-module mapping is concrete:

| Module | Path | Function |
|---|---|---|
| `graphify analyze` | `clawft-graphify/src/analyze.rs` | God nodes, surprising connections, surprise scoring (EML), graph diff |
| `graphify cluster` | `clawft-graphify/src/cluster.rs` | Label propagation community detection, oversized community splitting, cohesion scoring |
| `graphify extract` | `clawft-graphify/src/extract/` | AST-based entity extraction (Rust, Python, Go, JS), cross-file relationship detection |
| `graphify build` | `clawft-graphify/src/build.rs` | Knowledge graph construction from extraction results |
| `graphify pipeline` | `clawft-graphify/src/pipeline.rs` | End-to-end: detect -> extract -> build -> cluster -> analyze |
| `causal.rs` | `clawft-kernel/src/causal.rs` | Concurrent DAG with typed edges, Lanczos spectral analysis, Fiedler vector, spectral partitioning |
| `causal_predict.rs` | `clawft-kernel/src/causal_predict.rs` | O(1) delta-lambda2 perturbation prediction, evidence ranking, conversation cycle detection, EML correction model |
| `eml_models.rs` | `clawft-graphify/src/eml_models.rs` | EML models for surprise scoring and cluster thresholds |
| `bridge.rs` | `clawft-graphify/src/bridge.rs` | Bridge detection between graph components |
| HNSW | `clawft-kernel` | Hierarchical navigable small world index for vector similarity search |

---

## P0 -- Implement Now

### Paper 1: From Local to Global: A Graph RAG Approach to Query-Focused Summarization (Microsoft GraphRAG)

- **Authors**: Darren Edge, Ha Trinh, Newman Cheng, Joshua Bradley, Alex Chao, Apurva Mody, Steven Truitt, Dasha Metropolitansky, Robert Osazuwa Ness, Jonathan Larson
- **Year**: 2024 (revised Feb 2025)
- **ArXiv/URL**: https://arxiv.org/abs/2404.16130
- **Key Contribution**: Two-stage indexing pipeline that (1) extracts entity knowledge graphs from source documents, then (2) pregenerates community summaries for related entity groups. Answers global sensemaking questions by hierarchically aggregating partial responses from community summaries.
- **Techniques**:
  - Entity knowledge graph extraction from text
  - Community detection followed by LLM-generated community summaries
  - Hierarchical partial-response aggregation for global queries
  - Map-reduce pattern over community summaries
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify pipeline`, `graphify cluster`, `graphify query`
  - **What specific function could be improved?** The `run_query()` function in `graphify_cmd.rs` currently does naive keyword matching against node labels. GraphRAG's community-summary approach would allow global questions ("What are the main architectural patterns?") to be answered by pre-computing cluster-level summaries during the pipeline's analyze phase, then aggregating them at query time.
  - **Implementation plan**: (1) After `cluster()` runs in the pipeline, generate a text summary for each community by concatenating member entity labels, source files, and relationships, then feeding to an LLM or template. Store summaries in `AnalysisResult`. (2) In `run_query()`, when the question is global (no specific entity match), retrieve relevant community summaries via HNSW similarity, then aggregate partial answers. (3) For local questions, fall back to the existing node-level search + graph traversal.
  - **Expected impact**: Enables answering global/thematic questions that currently return nothing. Major capability unlock for the `graphify query` UX.
  - **Implementation difficulty**: M (community summaries are straightforward; LLM integration for summary generation is the main effort)
- **Priority**: P0

---

### Paper 2: CausalRAG: Integrating Causal Graphs into Retrieval-Augmented Generation

- **Authors**: Nengbo Wang, Xiaotian Han, Jagdip Singh, Jing Ma, Vipin Chaudhary
- **Year**: 2025 (ACL 2025 Findings)
- **ArXiv/URL**: https://arxiv.org/abs/2503.19878
- **Key Contribution**: Incorporates causal graphs into the RAG retrieval process. Instead of retrieving by semantic similarity alone, traces causal chains through the graph to preserve contextual continuity and improve retrieval precision.
- **Techniques**:
  - Causal graph construction from document chunks
  - Causal chain tracing during retrieval (follow Causes/Enables edges)
  - Contextual continuity preservation across chunks
  - Graph-aware reranking of retrieved passages
- **WeftOS Applicability**:
  - **Which module benefits?** `causal.rs`, `causal_predict.rs`, `graphify query`
  - **What specific function could be improved?** WeftOS already has `CausalGraph` with typed edges (Causes, Enables, EvidenceFor, etc.) and `rank_candidates()` using Fiedler-based evidence ranking. CausalRAG's retrieval-time causal tracing could be integrated into `graphify query` so that when a user asks "why does X fail?", the system follows causal chains in the graph rather than just matching keywords.
  - **Implementation plan**: (1) Add a `causal_trace()` method to `CausalGraph` that performs BFS/DFS along causal edge types (Causes, Enables, EvidenceFor) from seed nodes found by keyword/HNSW match. (2) Return traced subgraphs as context for query responses. (3) Combine with the existing `rank_evidence_by_impact()` to prioritize which causal paths are most structurally significant.
  - **Expected impact**: Dramatically improves query relevance for "why" and "how" questions. Directly leverages the existing causal edge infrastructure.
  - **Implementation difficulty**: S (the causal graph and edge types already exist; this is primarily a new traversal mode)
- **Priority**: P0

---

### Paper 3: Scalable and Adaptive Spectral Embedding for Attributed Graph Clustering (SASE)

- **Authors**: Yunhui Liu, Tieke He, Qing Wu, Tao Zheng, Jianhua Zhao
- **Year**: 2024 (CIKM 2024)
- **ArXiv/URL**: https://arxiv.org/abs/2408.05765
- **Key Contribution**: Parameter-learning-free spectral clustering achieving linear time/space complexity through k-order simple graph convolution and Random Fourier Features (RFF). On ArXiv dataset (169K nodes), achieves 6.9% accuracy improvement and 5.87x speedup over prior methods.
- **Techniques**:
  - k-order simple graph convolution for node feature smoothing
  - Random Fourier Features for approximate spectral embedding
  - Adaptive order selection (no hyperparameter tuning)
  - Linear time/space complexity
- **WeftOS Applicability**:
  - **Which module benefits?** `causal.rs` (spectral_analysis), `graphify cluster`
  - **What specific function could be improved?** The current `spectral_analysis()` in `causal.rs` uses full Lanczos iteration at O(k*m). For large graphs (10K+ nodes), this becomes the bottleneck. SASE's RFF-based spectral embedding could replace or supplement the Lanczos method, especially for the community detection path in `cluster.rs` which currently uses label propagation.
  - **Implementation plan**: (1) Implement RFF-based spectral embedding as an alternative to the Lanczos path in `spectral_analysis()`. (2) Use the resulting embeddings as input to k-means or the existing label propagation, giving better initialization. (3) Add adaptive order selection so the user does not need to tune `max_iterations`. (4) Gate behind a feature flag (`spectral-rff`) for opt-in until benchmarked.
  - **Expected impact**: 3-6x speedup on spectral analysis for graphs with 10K+ nodes. Better clustering quality through attributed spectral embedding.
  - **Implementation difficulty**: M (RFF is well-documented; main work is integrating with the existing Lanczos infrastructure)
- **Priority**: P0

---

### Paper 4: Less is More for RAG: Information Gain Pruning for Generator-Aligned Reranking and Evidence Selection

- **Authors**: Zhipeng Song, Yizhi Zhou, Xiangyu Kong, Jiulong Jiao, Xinrui Bao, Xu You, Xueqing Shi, Yuhang Zhou, Heng Qi
- **Year**: 2026
- **ArXiv/URL**: https://arxiv.org/abs/2601.17532
- **Key Contribution**: Information Gain Pruning (IGP) reranks and prunes evidence passages using a generator-aligned utility signal. Achieves 12-20% relative F1 improvement while reducing input tokens by 76-79%.
- **Techniques**:
  - Generator-aligned utility scoring (not just retrieval relevance)
  - Information gain computation per passage
  - Greedy pruning of redundant/harmful passages
  - Budget-aware evidence selection
- **WeftOS Applicability**:
  - **Which module benefits?** `causal_predict.rs` (evidence ranking), `graphify query`
  - **What specific function could be improved?** The `rank_evidence_by_impact()` function ranks candidates by predicted delta-lambda2 (Fiedler perturbation). IGP's information gain concept is complementary: after ranking by spectral impact, apply information gain pruning to remove redundant evidence edges that would not add new information to the generator. This is directly relevant to the `CausalRankResponse` that feeds the DEMOCRITUS loop.
  - **Implementation plan**: (1) After `rank_evidence_by_impact()` produces sorted candidates, add a second pass that computes marginal information gain of each candidate given the already-selected set. (2) Prune candidates whose marginal gain falls below a threshold. (3) This mirrors IGP but uses the spectral perturbation formula instead of LLM scoring: the "generator-aligned utility" is the Fiedler gap improvement.
  - **Expected impact**: Reduces the number of evidence edges presented to the user/agent by 50-70% while maintaining the same coherence improvement, directly reducing token cost in the DEMOCRITUS loop.
  - **Implementation difficulty**: S (the ranking infrastructure exists; adding marginal gain filtering is ~100 lines)
- **Priority**: P0

---

### Paper 5: RANGER: Repository-Level Agent for Graph-Enhanced Retrieval

- **Authors**: Pratik Shah, Rajat Ghosh, Aryan Singhal, Debojyoti Dutta
- **Year**: 2025
- **ArXiv/URL**: https://arxiv.org/abs/2509.25257
- **Key Contribution**: Builds repository-level knowledge graphs capturing hierarchical and cross-file dependencies down to variable granularity. Uses MCTS-guided graph exploration for natural language queries and fast Cypher lookups for entity queries.
- **Techniques**:
  - AST-based KG construction with node types: Module, Class, Function, Method, Field, GlobalVariable
  - Monte Carlo Tree Search (MCTS) for graph exploration during NL queries
  - Dual-stage retrieval: Cypher for entity queries, MCTS for NL queries
  - Textual descriptions and embeddings enriching graph nodes
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify extract` (AST extraction), `graphify query`, `graphify build`
  - **What specific function could be improved?** The current `build_extractions_from_detection()` in `graphify_cmd.rs` creates only file-level entities with co-location edges when tree-sitter is not available. RANGER's approach of building finer-grained KGs (down to variable level) with cross-file dependency edges directly maps to the existing `extract/lang/` parsers. The MCTS graph exploration technique could replace the current keyword-based `run_query()`.
  - **Implementation plan**: (1) Ensure the AST extractors in `extract/lang/{rust_lang,python,go,javascript}.rs` produce Method/Field/Variable-level entities (some already do). (2) Add cross-file import/call resolution to `extract/cross_file.rs`. (3) Implement MCTS-based graph traversal as an alternative query mode: start from HNSW-matched seed nodes, use MCTS to explore neighboring subgraphs, score paths by relevance. (4) Add per-node text descriptions from docstrings/comments during extraction.
  - **Expected impact**: Transforms graphify from file-level to symbol-level granularity. MCTS exploration enables discovering non-obvious multi-hop relationships that BFS/DFS miss.
  - **Implementation difficulty**: L (cross-file resolution and MCTS are significant engineering)
- **Priority**: P0

---

## P1 -- Implement Soon

### Paper 6: Causal Graphs Meet Thoughts: Enhancing Complex Reasoning in Graph-Augmented LLMs

- **Authors**: Hang Luo, Jian Zhang, Chujun Li
- **Year**: 2025
- **ArXiv/URL**: https://arxiv.org/abs/2501.14892
- **Key Contribution**: Filters knowledge graphs to emphasize cause-effect relationships and aligns retrieval with the LLM's chain-of-thought reasoning steps. Achieves up to 10% absolute improvement on medical QA tasks.
- **Techniques**:
  - Causal-first graph filtering (prioritize Causes/Enables edges over Correlates)
  - Chain-of-thought aligned multi-stage retrieval
  - Iterative path refinement across reasoning stages
- **WeftOS Applicability**:
  - **Which module benefits?** `causal.rs` (edge type filtering), `causal_predict.rs` (evidence ranking)
  - **What specific function could be improved?** `get_edges_by_type()` already supports filtering by `CausalEdgeType`. This paper's insight is to weight the causal edge types during traversal: `Causes > Enables > EvidenceFor > Correlates`. Integrate this weighting into `rank_evidence_by_impact()` so that causal edges get a multiplier on their predicted_delta.
  - **Expected impact**: Better evidence prioritization by combining spectral analysis with causal semantics. Moderate improvement in query answer quality.
  - **Implementation difficulty**: S
- **Priority**: P1

---

### Paper 7: Knowledge Graph Based Repository-Level Code Generation

- **Authors**: Mihir Athale, Vishal Vaddina
- **Year**: 2025
- **ArXiv/URL**: https://arxiv.org/abs/2505.14394
- **Key Contribution**: Represents code repositories as knowledge graphs for context-aware code generation. Uses hybrid retrieval combining full-text semantic search with graph-based queries. Evaluated on EvoCodeBench.
- **Techniques**:
  - tree-sitter AST parsing to extract classes, variables, dependencies
  - Hybrid retrieval: semantic search + graph queries
  - Dependency tracking across files
  - KG-augmented code generation
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify extract`, `graphify build`, `graphify query`
  - **What specific function could be improved?** Validates and extends the approach already taken in `extract/lang/`. The hybrid retrieval (semantic + graph) pattern should be adopted for `graphify query`: first HNSW semantic search on node embeddings, then graph-based expansion along dependency edges.
  - **Expected impact**: Confirms the architecture is on the right track. Specific technique to adopt: the hybrid retrieval pattern.
  - **Implementation difficulty**: M
- **Priority**: P1

---

### Paper 8: Information Gain Propagation: A New Way to Graph Active Learning with Soft Labels

- **Authors**: Wentao Zhang, Yexin Wang, Zhenbang You, Meng Cao, Ping Huang, Jiulong Shan, Zhi Yang, Bin Cui
- **Year**: 2022 (ICLR 2022)
- **ArXiv/URL**: https://arxiv.org/abs/2203.01093
- **Key Contribution**: Proposes information gain propagation criteria for graph active learning. Instead of requiring experts to provide exact labels, uses binary correctness judgments (soft labels). Information gain propagates through the graph structure to identify the most valuable nodes to label.
- **Techniques**:
  - Soft-label graph active learning
  - Information gain propagation through GNN message passing
  - Node selection criteria maximizing propagated information gain
- **WeftOS Applicability**:
  - **Which module benefits?** `causal_predict.rs` (evidence ranking), `graphify analyze` (surprising connections)
  - **What specific function could be improved?** The `rank_evidence_by_impact()` function uses Fiedler perturbation to rank candidates. Information gain propagation offers an alternative ranking criterion: instead of asking "which edge most improves algebraic connectivity?", ask "which node, if labeled/confirmed, would propagate the most information to its neighbors?" This is directly useful for the DEMOCRITUS loop's evidence gathering phase -- it identifies which pieces of evidence to seek next.
  - **Expected impact**: More targeted evidence gathering, especially in sparse graphs where Fiedler perturbation may not discriminate well.
  - **Implementation difficulty**: M (requires implementing the propagation kernel, but the graph infrastructure exists)
- **Priority**: P1

---

### Paper 9: GraphRAG-Causal: A Novel Graph-Augmented Framework for Causal Reasoning in News

- **Authors**: Abdul Haque, Umm e Hani, Ahmad Din, Muhammad Babar, Ali Abbas, Insaf Ullah
- **Year**: 2025
- **ArXiv/URL**: https://arxiv.org/abs/2506.11600
- **Key Contribution**: Three-stage pipeline (annotate -> graph store in Neo4j -> LLM inference with few-shot) for causal classification in news. Achieves 82.1% F1 on causal classification with only 20 few-shot examples.
- **Techniques**:
  - Causal KG construction capturing cause, effect, trigger relationships
  - Neo4j hybrid Cypher queries combining semantic embeddings with structural graph queries
  - Few-shot LLM inference with XML-based prompting
- **WeftOS Applicability**:
  - **Which module benefits?** `causal.rs`, `graphify export` (Cypher export already exists)
  - **What specific function could be improved?** The existing `CausalEdgeType` enum has Causes, Inhibits, Enables but lacks a Trigger type. Adding a `Trigger` edge type and adopting the cause/effect/trigger triple structure from this paper would make the causal graph more expressive. The Cypher export in `export/` could be enhanced to produce Neo4j-compatible causal graphs.
  - **Expected impact**: Richer causal modeling. Enables WeftOS to do causal classification on ingested content.
  - **Implementation difficulty**: S
- **Priority**: P1

---

### Paper 10: A Unified Knowledge Graph to Permit Interoperability of Heterogeneous Digital Evidence

- **Authors**: Ali Alshumrani, Nathan Clarke, Bogdan Ghita
- **Year**: 2024
- **ArXiv/URL**: https://arxiv.org/abs/2402.13746
- **Key Contribution**: Proposes a unified knowledge graph for consolidating heterogeneous digital evidence formats. Supports Link Analysis, Timeline Analysis, Correlation Analysis, and Geolocation Analysis over harmonized evidence.
- **Techniques**:
  - Evidence harmonization across heterogeneous formats
  - Cross-analytic querying (link, timeline, correlation, geolocation)
  - Entity resolution across evidence sources
  - Provenance tracking for evidence chain-of-custody
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify ingest`, `graphify build`, `domain/forensic.rs`
  - **What specific function could be improved?** WeftOS already has a `DomainTag::Forensic` and forensic domain module. This paper provides a concrete schema for unifying digital evidence -- logs, chat transcripts, filesystem artifacts -- into a single knowledge graph with provenance. The entity resolution techniques would improve `build_from_json()` when merging multiple ingestion runs.
  - **Expected impact**: Enables WeftOS to serve as a forensic investigation platform, which aligns with the GTM priority of understanding/documenting/automating client systems.
  - **Implementation difficulty**: M
- **Priority**: P1

---

## P2 -- Research Further

### Paper 11: EA-Agent: A Structured Multi-Step Reasoning Agent for Entity Alignment

- **Authors**: Yixuan Nan, Xixun Lin, Yanmin Shang, Ge Zhang, Zheng Fang, Fang Fang, Yanan Cao
- **Year**: 2026 (ACL 2026)
- **ArXiv/URL**: https://arxiv.org/abs/2604.11686
- **Key Contribution**: Formulates entity alignment (identifying equivalent entities across different knowledge graphs) as a structured multi-step reasoning process using LLM agents. Uses attribute and relation triple selectors to filter redundant information before LLM inference.
- **Techniques**:
  - Multi-step structured reasoning for entity alignment
  - Attribute and relation triple filtering to reduce LLM input
  - LLM-as-agent architecture for cross-KG alignment
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify build` (entity deduplication), `graphify diff`
  - **What specific function could be improved?** When `graphify ingest` runs multiple times or across different repositories, duplicate entities appear (same function in different repos, same concept described differently). EA-Agent's entity alignment approach could power a deduplication pass in the pipeline. The triple filtering technique is useful for reducing token cost when generating graph summaries.
  - **Expected impact**: Cleaner graphs with fewer duplicate entities. Relevant when WeftOS ingests multiple codebases.
  - **Implementation difficulty**: L (requires LLM integration for the reasoning steps)
- **Priority**: P2

---

### Paper 12: From Data to Dialogue: Leveraging the Structure of Knowledge Graphs for Conversational Exploratory Search

- **Authors**: Phillip Schneider, Nils Rehtanz, Kristiina Jokinen, Florian Matthes
- **Year**: 2023 (PACLIC 2023)
- **ArXiv/URL**: https://arxiv.org/abs/2310.05150
- **Key Contribution**: Knowledge-driven dialogue system for exploratory search over news articles. Uses graph structure to navigate between related topics via natural language questions. User study with 54 participants validates the approach.
- **Techniques**:
  - KG-guided dialogue management
  - Graph-based topic navigation
  - Exploratory search via conversational refinement
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify query` (future conversational mode)
  - **What specific function could be improved?** The `run_query()` function is currently single-shot. This paper provides a design pattern for multi-turn exploratory queries where each answer refines the graph traversal context. Relevant for a future `graphify chat` command.
  - **Expected impact**: Enables multi-turn graph exploration. Good UX research reference.
  - **Implementation difficulty**: L
- **Priority**: P2

---

### Paper 13: AGENTiGraph: An Interactive Knowledge Graph Platform for LLM-based Chatbots

- **Authors**: Xinjie Zhao, Moritz Blum, Rui Yang, et al.
- **Year**: 2024 (COLING 2025)
- **ArXiv/URL**: https://arxiv.org/abs/2410.11531
- **Key Contribution**: Multi-agent KG platform with 95.12% intent classification accuracy and 90.45% task execution success. Dual-mode interaction: conversational AI + interactive graph visualization.
- **Techniques**:
  - Multi-agent architecture for KG operations
  - Intent classification for mapping NL to graph operations
  - Dual-mode UI (chat + visualization)
  - Dynamic knowledge graph updates through conversation
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify` CLI overall, future web UI
  - **What specific function could be improved?** Design reference for a future `graphify` web interface. The intent classification approach (mapping NL queries to specific graph operations like "find god nodes", "show community X", "trace causal path") could be adopted for the CLI's query mode.
  - **Expected impact**: UX design guidance. Not directly implementable in current Rust CLI.
  - **Implementation difficulty**: L
- **Priority**: P2

---

### Paper 14: Retrieval-Augmented Generation with Graphs (GraphRAG Survey)

- **Authors**: Haoyu Han, Yu Wang, Harry Shomer, et al. (18 authors)
- **Year**: 2025
- **ArXiv/URL**: https://arxiv.org/abs/2501.00309
- **Key Contribution**: Comprehensive survey defining GraphRAG as a five-component architecture: query processor, retriever, organizer, generator, data source. Reviews domain-specific techniques and identifies research challenges.
- **Techniques**:
  - Taxonomy of GraphRAG components
  - Domain-specific graph retrieval patterns
  - Cross-disciplinary analysis of graph-structured RAG
- **WeftOS Applicability**:
  - **Which module benefits?** Overall architecture planning
  - **What specific function could be improved?** This is a survey, not a specific technique paper. Use as a reference for positioning WeftOS's graphify system within the broader GraphRAG landscape. Helpful for identifying which GraphRAG sub-problems WeftOS already solves and which remain open.
  - **Expected impact**: Architectural clarity. No direct code changes.
  - **Implementation difficulty**: N/A (reference material)
- **Priority**: P2

---

### Paper 15: A Comprehensive Review of Community Detection in Graphs

- **Authors**: (Multiple, Neurocomputing 2024)
- **ArXiv/URL**: https://arxiv.org/abs/2309.11798
- **Key Contribution**: Reviews community detection methods including deep learning approaches, GNN-based methods, and spectral techniques. Covers modularity optimization, label propagation variants, and hierarchical methods.
- **Techniques**:
  - Louvain/Leiden modularity optimization
  - Deep learning community detection (GNN-based)
  - Greedy Recursive Spectral Bisection (GRSBM)
  - Hierarchical divisive community detection
- **WeftOS Applicability**:
  - **Which module benefits?** `graphify cluster`
  - **What specific function could be improved?** The current `cluster()` uses label propagation, which is noted in the code comments as "simpler, deterministic with a fixed seed." This survey provides evidence for when to upgrade to Louvain/Leiden (better quality on larger graphs) or GRSBM (spectral bisection with modularity bounds). The `split_community()` function for oversized communities could benefit from spectral bisection instead of the current heuristic splitting.
  - **Expected impact**: Better community quality on larger graphs. Reference for future clustering upgrades.
  - **Implementation difficulty**: M
- **Priority**: P2

---

## Implementation Roadmap

### Phase 1 (Next Sprint) -- P0 Items

1. **Community Summaries (Paper 1)**: Add `generate_community_summaries()` to the pipeline after `cluster()`. Store in `AnalysisResult`. Wire into `run_query()` for global questions.

2. **Causal Chain Tracing (Paper 2)**: Add `causal_trace()` to `CausalGraph`. Integrate with `graphify query` for "why/how" questions.

3. **Information Gain Pruning for Evidence (Paper 4)**: Add marginal gain filter to `rank_evidence_by_impact()` in `causal_predict.rs`. Threshold-based pruning of redundant evidence edges.

### Phase 2 (Sprint+1) -- P0 Continued

4. **RFF Spectral Embedding (Paper 3)**: Implement `spectral_analysis_rff()` as a feature-gated alternative to Lanczos in `causal.rs`. Benchmark against current implementation on graphs of 1K, 10K, 100K nodes.

5. **MCTS Graph Exploration (Paper 5)**: Implement MCTS traversal for `graphify query`. Requires seed node selection via HNSW, then MCTS exploration with relevance scoring at each expansion.

### Phase 3 (Sprint+2) -- P1 Items

6. **Causal Edge Weighting (Paper 6)**: Add edge-type multipliers to `rank_evidence_by_impact()`.

7. **Hybrid Retrieval (Paper 7)**: Combine HNSW semantic search with graph-based expansion in `run_query()`.

8. **Information Gain Propagation (Paper 8)**: Implement as alternative ranking criterion for evidence gathering.

9. **Trigger Edge Type + Forensic Schema (Papers 9, 10)**: Extend `CausalEdgeType` with `Trigger`. Adopt forensic evidence schema for `domain/forensic.rs`.

---

## Cross-Cutting Themes

### Theme A: Spectral Methods Are Central
Papers 3, 4, 8, 15 all connect to the spectral analysis infrastructure in `causal.rs`. The Lanczos/Fiedler machinery is the foundation. Investments here (RFF speedup, better perturbation predictions) have compound returns across the system.

### Theme B: Graph + RAG Integration
Papers 1, 2, 5, 6, 7, 14 all address how to combine graph structure with retrieval-augmented generation. WeftOS is well-positioned here because it already has both the graph (`KnowledgeGraph`, `CausalGraph`) and the vector search (HNSW). The missing piece is the retrieval-time integration: using graph structure to guide/filter/expand HNSW results.

### Theme C: Evidence Selection Is a First-Class Problem
Papers 4, 8 frame evidence selection as an information-theoretic problem. The existing `rank_evidence_by_impact()` is a strong foundation but operates on spectral criteria alone. Adding information gain (marginal utility of each new evidence piece) would make the DEMOCRITUS loop more efficient.

### Theme D: Causal Reasoning Is Differentiated
Papers 2, 6, 9 focus on causal graph reasoning. WeftOS's `CausalGraph` with typed edges and spectral analysis is more sophisticated than most systems surveyed. The opportunity is to surface this capability through better query interfaces (causal tracing) and richer edge type semantics (Trigger).
