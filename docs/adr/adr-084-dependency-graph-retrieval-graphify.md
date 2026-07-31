# ADR-084: Dependency-Graph Retrieval in Graphify (SGKR)

- **Status**: Proposed (Candidate)
- **Closes / tracks**: WEFT-372 (candidate write-up); implementation not yet scheduled
- **Date**: 2026-07-31
- **Deciders**: Pending (graphify / ECC maintainers)
- **Historical alias**: Phase-2 paper survey listed this as **“ADR-050”**. Numbers
  **050–053 were already claimed** (`adr-050-escalation-security-final-review.md`,
  plugin crates fate, ToolRegistry split, voice STT). Next free indices start at
  **084** (after ADR-083 browser WASM). This document is the canonical candidate
  for survey item 050.
- **Related**: ADR-011 (raw HNSW; not a similarity replacement), ADR-023
  (Analyzer registry), ADR-046 (forest of trees), ADR-062 (ECC graph-walk),
  ADR-082 (graphify Rust port)
- **Source**:
  - `.planning/development_notes/knowledge-graph-paper-survey-phase2.md` (Paper 4: SGKR)
  - ArXiv 2604.10516 — *SGKR: Structure-Grounded Knowledge Retrieval via Code Dependencies*
  - `crates/clawft-graphify/` (`extract/ast.rs`, `extract/cross_file.rs`, `bridge.rs`)

## Context

`clawft-graphify` already builds a directed knowledge graph from AST extraction
(tree-sitter; Rust / Python / JS / Go) and cross-file import resolution. The graph
supports visualization, god-node / surprise scoring, and (with `kernel-bridge`)
mapping into `CausalGraph` + HNSW.

What it does **not** do today is treat the call/dependency graph as a
**retrieval index for multi-step data flow**. Embedding-similarity retrieval
(HNSW) answers “what is textually / vector-similar”; it does not answer “which
functions form a pipeline from semantic input tag *X* to output tag *Y*.”

SGKR (ArXiv 2604.10516) replaces embedding-only retrieval for multi-step code
reasoning with:

1. AST-derived caller→callee dependency edges,
2. Semantic I/O tag mapping from query keywords to graph nodes,
3. BFS path finding between input and output tags,
4. Subgraph assembly (union of paths) as LLM / agent context.

Phase-2 survey priority: **P0** — directly advances system understanding /
automation GTM; ~80% of infrastructure already exists in graphify. Effort: **M**.

Cross-file resolution is still Python-heavy for import patterns (GRAPH-011
class); extractors exist for four languages but linking is incomplete.

## Decision (Proposed)

### 1. Adopt structure-grounded pipeline retrieval on `KnowledgeGraph`

**Propose** a first-class retrieval API on `clawft-graphify`’s `KnowledgeGraph`
(or a thin facade) of the form:

```text
retrieve_pipeline(input_tag, output_tag) -> Subgraph
```

Semantics:

- Resolve `input_tag` / `output_tag` to sets of entity nodes (functions, types,
  data entities) via existing metadata + optional light semantic map.
- Run **BFS (or multi-source BFS)** over **call / dependency edges** only
  (caller→callee, import→definition), not over every relationship type.
- Return the **union of paths** as a petgraph subgraph suitable for export,
  question generation, and agent context packing.

This is **retrieval**, not a new graph store. It composes with HNSW (ADR-011):
HNSW finds seed entities; dependency BFS expands structural neighborhood.

### 2. Extend cross-file resolution beyond Python

Treat multi-language cross-file linking as a prerequisite for high-quality
pipeline retrieval:

| Language | AST extract today | Cross-file link target |
|----------|-------------------|------------------------|
| Python   | Yes               | Improve + keep as reference |
| Rust     | Yes               | `use` / `mod` / path resolution |
| JS/TS    | Yes               | import/export graphs |
| Go       | Yes               | package / import graphs |

No change to EntityId / taxonomy stability (ADR-082).

### 3. Semantic I/O tags

Bootstrap tags from existing entity metadata (names, types, doc comments) before
any LLM step. Optional LLM enrichment stays behind the existing semantic extract
path (`semantic_extract.rs`) and must not be required for the BFS core.

### 4. Integration points

| Consumer | Use |
|----------|-----|
| `weaver graphify …` | CLI subcommand / flag for pipeline query |
| `GraphifyBridge` | Optional: surface pipeline subgraphs into ECC / CrossRef |
| DEMOCRITUS SEARCH | Optional secondary structural expansion after HNSW hit |
| Agent / assessment | Context pack for multi-step “how does data flow” questions |

### 5. Explicit non-goals (v0)

- Not a full program dependence graph (PDG) with data-flow SSA.
- Not replacing HNSW or FrankenSearch debates (ADR-011 stands).
- Not requiring LLM narration per hop (see TRACE survey note — too slow for
  15 ms cognitive tick; optional offline only).

## Consequences

### Positive

- Closes the gap between “map of what exists” and “executable understanding of
  how *X* becomes *Y*.”
- High reuse of petgraph + existing extractors; survey estimates medium effort.
- Improves GTM story for client-system automation and documentation.

### Negative / risks

- Incomplete cross-file linking yields truncated pipelines (false negatives).
- Over-broad edge types can pollute BFS; need a clear dependency edge filter.
- Semantic tag quality may need iteration; bad tags → empty or noisy subgraphs.

### Follow-ups

- Implement `retrieve_pipeline` + unit tests on fixture multi-file crates.
- Language-by-language cross-file resolution milestones.
- Benchmark retrieval quality vs embedding-only baseline on internal codebases.
- Optional later: TRACE-style exploration priors (survey P1; not this ADR).

## Alternatives considered

| Alternative | Why not (for now) |
|-------------|-------------------|
| Embedding-only RAG over code chunks | Misses structural multi-step pipelines (SGKR thesis) |
| Full PDG / LLVM-level DFG | Far heavier than AST call graph; out of graphify scope |
| External code-intel LSP only | Not in-process; breaks lockstep / Analyzer story (ADR-082) |

## Open questions

1. Tag vocabulary: free-form strings vs closed ontology per domain pack?
2. Should pipeline retrieval register as a kernel Analyzer method or stay
   graphify-only until bridge demand is proven?
3. Max path length / beam width defaults for large monorepos?
