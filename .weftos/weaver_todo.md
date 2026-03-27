# Weaver TODO

Items discovered during the initial codebase analysis that need implementation.
Last updated: 2026-03-26 (after graph ingestion)

## EmbeddingProvider Backends

- [x] EmbeddingProvider trait defined (`crates/clawft-kernel/src/embedding.rs`)
- [x] MockEmbeddingProvider (deterministic SHA-256 hash-based, for testing)
- [x] ONNX backend (all-MiniLM-L6-v2, 384 dimensions) — behind `onnx-embeddings` feature; hash-fallback until `ort` available
- [x] LLM API backend (call clawft-llm provider for embeddings) — `LlmEmbeddingProvider` with fallback to mock
- [x] Sentence-transformer backend for documentation chunks — `SentenceTransformerProvider` with markdown preprocessing + mean pooling
- [x] AST-aware backend for Rust code (function signatures, type definitions) — `AstEmbeddingProvider` with hybrid structural+text embedding
- [x] Batch embedding pipeline (process multiple chunks in one call) — `embed_batch` + `sync_embed` for ingestion

## Data Ingestion

- [x] Git history ingestion — 102 nodes, 965 edges (Follows/Enables/Correlates) `.weftos/graph/git-history.json`
- [x] Module dependency ingestion — 214 nodes, 473 edges (Uses/Enables/EvidenceFor) `.weftos/graph/module-deps.json`
- [x] Test-to-source mapping — included in module-deps as EvidenceFor edges (162 edges)
- [x] SPARC plan ingestion — 11 phases with exit criteria counts `.weftos/graph/decisions.json`
- [x] Symposium decision ingestion — 59 decisions, 22 commitments `.weftos/graph/decisions.json`

## Cognitive Tick Integration

- [x] Register Weaver with CognitiveTick as a tick consumer — `on_tick()` method on WeaverEngine
- [x] Budget-aware tick processing (respect tick_budget_ratio) — `on_tick(budget_ms)` checks elapsed vs budget at each phase
- [x] Incremental git polling (detect new commits since last tick) — `GitPoller` struct, `enable_git_polling()`
- [x] File watcher integration (detect source file changes) — `FileWatcher` struct (mtime-based), `enable_file_watching()`

## Confidence Improvement

- [x] Initial confidence assessment (0.62 → 0.78 after ingestion) — see `docs/weftos/weaver-analysis-clawft.md`
- [x] Confidence scoring based on edge coverage (% of modules with causal edges) — `compute_confidence()`
- [x] Gap detection: modules with no incoming/outgoing causal edges — orphan detection in `compute_confidence()`
- [x] Suggestion engine: recommend data sources based on confidence gaps — integrated into `ConfidenceReport`
- [x] Confidence history tracking in meta-Loom

## Export / Import

- [x] ExportedModel type defined (`crates/clawft-kernel/src/weaver.rs`)
- [x] weave-model.json export CLI command (`weaver ecc export`) — `export_model_to_file()` + causal data population
- [x] weave-model.json import on edge devices — `import_model_from_file()` roundtrip verified
- [x] Model diff (compare two exported models) — `diff_models()` + `ModelDiff` struct
- [x] Model merge (stitch models from different analysis sessions) — `merge_models()` + `MergeResult`/`MergeConflict`/`MergeStats`

## Self-Improvement

- [x] MetaLoomEvent + MetaDecisionType types defined (`weaver.rs`)
- [x] WeaverKnowledgeBase type defined (`weaver.rs`)
- [x] Track which analysis strategies improved confidence (meta-Loom)
- [x] Cross-domain pattern library persistence — `save_to_file()`/`load_from_file()`/`learn_pattern()`/`find_patterns()` + `SerializableKB`
- [x] Recommend tick interval adjustments based on change frequency

## Completed Infrastructure

- [x] WeaverEngine SystemService (`crates/clawft-kernel/src/weaver.rs`, 1,371 lines)
- [x] ModelingSession, CausalModel, ConfidenceReport types
- [x] WeaverCommand/WeaverResponse IPC protocol (13 command variants)
- [x] DataSource enum (7 variants: GitLog, FileTree, CiPipeline, etc.)
- [x] Codebase analysis report (`docs/weftos/weaver-analysis-clawft.md`, 911 lines)
- [x] WEAVER.md operator skill (`agents/weftos-ecc/WEAVER.md`, 692 lines)
- [x] SPARC crate plan (`09-ecc-weaver-crate.md`, 1,238 lines)
- [x] `weftos init` creates `.weftos/` + `weave.toml`
- [x] 12 WeftOS agents defined (`agents/weftos/`)
- [x] `weave init` installs agents + skills to `.claude/`

What the Graph Reveals Now

  Growth: Kernel grew 14.5% since mid-session analysis. The Weaver subsystem (6,571 lines) is now larger than 7 of the 22 workspace crates — it's
   become a major subsystem in its own right.

  The Bridge: process.rs (degree 26) is the single module connecting mesh, exochain, and os-patterns into a coherent kernel. If it breaks, three
  feature domains disconnect.

  Stale Feedback Loop: 12 decisions show as "pending" in the graph even though sprint 09b resolved them. The ingestion pipeline reads the
  original symposium markdown, not the updated resolution docs. This is the Weaver's first self-identified bug — it can see its own data is
  stale.

  Where the Conversation Goes

  1. Fix the Weaver's own feedback loop

  The Weaver discovered it has stale data. This is the meta-Loom in action — it should update its own decision graph when resolutions are
  committed. This is a live test of the self-improvement cycle.

  2. Community detection on the graph

  Can we automatically discover the 12 conversations from graph structure alone? If spectral clustering on the causal graph produces clusters
  that match the manually-identified conversations, that validates the ECC model. If it doesn't, the divergence is itself interesting.

  4. Predictive analysis

  The git history graph has temporal patterns. Can we predict which modules are likely to change next based on commit burst rhythm and coupling?

  5. Cross-project test

  Run weftos init on the mentra project (the glasses firmware) and see what the Weaver discovers about a completely different domain. The
  WeaverKnowledgeBase should then carry "Rust project" patterns learned from clawft into the mentra analysis.

  6. Technical debt as a graph metric

  Define: debt_score = (fan_out * lines) / (tests * recency). High fan-out, large, poorly tested, old modules have the highest debt. The graph
  can compute this across every module.

  7. Automated SPARC plan generation

  The Weaver has seen the pattern: identify gap → write SPARC plan → implement → test → review. Could it generate a SPARC plan from a gap report
  automatically?


  ● The causal graph has BFS traversal and path finding but no community detection or spectral analysis yet. These are mentioned in VISION.md as
  capabilities but haven't been implemented in the Rust code. The gap analysis scripts do community detection in Python though. Let me check the
  gap analysis more closely to see if it has those features:

  Searched for 1 pattern (ctrl+o to expand)

● The community detection and predictive features exist conceptually but aren't implemented in the Rust kernel yet — they're mentioned as future
  work items in the weaver_todo. The gap analysis Python scripts do structural analysis but not spectral community detection.

## Sprint 10 Deliverables (2026-03-27)

### Kernel Hardening
- [x] RestartStrategy (OneForOne, OneForAll, RestForOne, Permanent, Transient) + budget + backoff
- [x] ProcessLink/ProcessMonitor integration with supervisor
- [x] CausalGraph persistence (JSON save/load)
- [x] HNSW index persistence
- [x] PersistenceConfig coordinator
- [x] DemocritusLoop continuous cognitive cycle (ecc:D5 resolved)
- [x] DeadLetterQueue wired into A2A router
- [x] MetricsRegistry wired into boot with kernel gauges
- [x] LogService + TimerService wired into boot
- [x] ConfigService with typed values + change notifications
- [x] AuthService with SHA-256 hashed credentials + scoped tokens
- [x] MeshRuntime with A2A bridge (RemoteNode → peer delivery)
- [x] Mesh discovery (Kademlia + heartbeat + peer address registry)
- [x] Chain sync request/response stubs
- [x] 10 new WASM tools (fs.analyze, git.log, doc.parse, etc.)

### K8 GUI
- [x] React 19 + TypeScript + Vite + Tailwind scaffold
- [x] Dashboard view (metrics, processes, chain events, health)
- [x] Admin Forms view (5 CRUD forms: spawn/stop/config/chain/service)
- [x] Knowledge Graph view (Cytoscape.js, community coloring, search, lambda_2)
- [x] NodeDetail panel (edges, metadata, community)
- [x] WebSocket hook with mock kernel data

### Testing
- [x] 20+ A2A router tests
- [x] 16+ boot path tests
- [x] 18+ supervisor restart tests
- [x] 12+ persistence roundtrip tests
- [x] 12+ DEMOCRITUS loop tests
- [x] 10+ mesh runtime tests
- [x] 16+ extended tools tests
- [x] 8+ config/auth service tests
- [x] 8 E2E integration tests
- [x] Total: 1,580+ tests (up from 613)

### Documentation + Deployment
- [x] INSTALL.md
- [x] FEATURE_GATES.md
- [x] CONFIGURATION.md
- [x] external-analysis-results.md (ruvector: 109 crates validated)
- [x] docker-compose.yml (2-node mesh)
- [x] .dockerignore
- [x] Sprint 10 plan + product ROADMAP.md
- [x] GTM synthesis document