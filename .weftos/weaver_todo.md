# Weaver TODO

Items discovered during the initial codebase analysis that need implementation.

## EmbeddingProvider Backends

- [ ] ONNX backend (all-MiniLM-L6-v2, 384 dimensions) — behind `onnx-embeddings` feature
- [ ] LLM API backend (call clawft-llm provider for embeddings) — uses existing provider trait
- [ ] Sentence-transformer backend for documentation chunks
- [ ] AST-aware backend for Rust code (function signatures, type definitions)
- [ ] Batch embedding pipeline (process multiple chunks in one call)

## Data Ingestion

- [ ] Git history ingestion — parse commits into causal nodes with Follows/Enables/TriggeredBy edges
- [ ] Module dependency ingestion — parse `use` statements into Contains/Enables edges
- [ ] Test-to-source mapping — link test functions to the code they test (EvidenceFor edges)
- [ ] SPARC plan ingestion — parse plan files into causal decision chains
- [ ] Symposium decision ingestion — extract decisions/commitments as goal nodes

## Cognitive Tick Integration

- [ ] Register Weaver with CognitiveTick as a tick consumer
- [ ] Budget-aware tick processing (respect tick_budget_ratio)
- [ ] Incremental git polling (detect new commits since last tick)
- [ ] File watcher integration (detect source file changes)

## Confidence Improvement

- [ ] Confidence scoring based on edge coverage (% of modules with causal edges)
- [ ] Gap detection: modules with no incoming/outgoing causal edges
- [ ] Suggestion engine: recommend data sources based on confidence gaps
- [ ] Confidence history tracking in meta-Loom

## Export / Import

- [ ] weave-model.json export with full node/edge type specs
- [ ] weave-model.json import on edge devices
- [ ] Model diff (compare two exported models)
- [ ] Model merge (stitch models from different analysis sessions)

## Self-Improvement

- [ ] Track which analysis strategies improved confidence (meta-Loom)
- [ ] Cross-domain pattern library (WeaverKnowledgeBase)
- [ ] Recommend tick interval adjustments based on change frequency
