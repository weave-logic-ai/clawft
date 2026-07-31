# ADR-091: LightRAG dual-level keyword retrieval in graphify

- **Status**: Accepted
- **Closes / tracks**: WEFT-517
- **Date**: 2026-07-31
- **Related**: ADR-082 (graphify port), ADR-084 (SGKR), LightRAG P4/P5 (WEFT-376/375)
- **Source**: Guo et al. 2410.05779 — *LightRAG: Simple and Fast Retrieval-Augmented Generation*

## Context

GraphRAG-style community summarization is token-heavy (~250 tokens per
community summary on a representative graphify workload). LightRAG
proposes dual-level keyword retrieval:

1. **Low-level** — entity / local keywords
2. **High-level** — theme / community keywords

Combined with prior graphify work (graph re-rank P4, edge embeddings P5)
this yields a retrieval path suitable for `suggest_questions()` and
conversation without dumping full community prose into the LLM context.

## Decision

Ship `crates/clawft-graphify/src/lightrag.rs` with:

- `tokenize_keywords` / `split_levels`
- `dual_level_retrieve(kg, query, community_of, community_labels, top_k)`
- `TokenCostEstimate` comparing dual-level vs GraphRAG-ish dump

Always compiled (no Cargo feature flag) — call sites opt in by calling
the dual-level API. Token-cost comparison is unit-tested with a
synthetic 2-community graph; savings ratio is typically ≫ 10×.

## Consequences

- Dual-level is available as a library primitive for conversation /
  suggest_questions follow-ups.
- Full LightRAG graph construction (entity+relation extraction pipeline
  from the paper) is **not** reimplemented; we reuse clawft-graphify's
  existing AST extraction.
- GraphRAG community-summary path remains available for audits that
  explicitly want long-form community context.
