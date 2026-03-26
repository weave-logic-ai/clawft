# Weaver TODO

Items discovered during the initial codebase analysis that need implementation.
Last updated: 2026-03-26 (after graph ingestion)

## EmbeddingProvider Backends

- [x] EmbeddingProvider trait defined (`crates/clawft-kernel/src/embedding.rs`)
- [x] MockEmbeddingProvider (deterministic SHA-256 hash-based, for testing)
- [ ] ONNX backend (all-MiniLM-L6-v2, 384 dimensions) — behind `onnx-embeddings` feature
- [ ] LLM API backend (call clawft-llm provider for embeddings) — uses existing provider trait
- [ ] Sentence-transformer backend for documentation chunks
- [ ] AST-aware backend for Rust code (function signatures, type definitions)
- [ ] Batch embedding pipeline (process multiple chunks in one call)

## Data Ingestion

- [x] Git history ingestion — 102 nodes, 965 edges (Follows/Enables/Correlates) `.weftos/graph/git-history.json`
- [x] Module dependency ingestion — 214 nodes, 473 edges (Uses/Enables/EvidenceFor) `.weftos/graph/module-deps.json`
- [x] Test-to-source mapping — included in module-deps as EvidenceFor edges (162 edges)
- [x] SPARC plan ingestion — 11 phases with exit criteria counts `.weftos/graph/decisions.json`
- [x] Symposium decision ingestion — 59 decisions, 22 commitments `.weftos/graph/decisions.json`

## Cognitive Tick Integration

- [ ] Register Weaver with CognitiveTick as a tick consumer
- [ ] Budget-aware tick processing (respect tick_budget_ratio)
- [ ] Incremental git polling (detect new commits since last tick)
- [ ] File watcher integration (detect source file changes)

## Confidence Improvement

- [x] Initial confidence assessment (0.62 → 0.78 after ingestion) — see `docs/weftos/weaver-analysis-clawft.md`
- [ ] Confidence scoring based on edge coverage (% of modules with causal edges)
- [ ] Gap detection: modules with no incoming/outgoing causal edges
- [ ] Suggestion engine: recommend data sources based on confidence gaps
- [ ] Confidence history tracking in meta-Loom

## Export / Import

- [x] ExportedModel type defined (`crates/clawft-kernel/src/weaver.rs`)
- [ ] weave-model.json export CLI command (`weaver ecc export`)
- [ ] weave-model.json import on edge devices
- [ ] Model diff (compare two exported models)
- [ ] Model merge (stitch models from different analysis sessions)

## Self-Improvement

- [x] MetaLoomEvent + MetaDecisionType types defined (`weaver.rs`)
- [x] WeaverKnowledgeBase type defined (`weaver.rs`)
- [ ] Track which analysis strategies improved confidence (meta-Loom)
- [ ] Cross-domain pattern library persistence
- [ ] Recommend tick interval adjustments based on change frequency

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
