# ADR-082: Graphify Rust Port — `clawft-graphify` knowledge-graph crate

- **Status**: Accepted (2026-07-31)
- **Closes**: WEFT-371
- **Date**: Design 2026-04-04; accepted retrospectively 2026-07-31
- **Deciders**: Workstream 12 (knowledge-graph) maintainers; clawft-graphify
  implementers (Phase 1A–5 + Sprint 17)
- **Historical alias**: Planning docs (`.planning/graphify-rs/MASTER_PLAN.md`,
  `architecture.md`, release-gate `12-knowledge-graph-graphify.md`) referred
  to this decision as **“ADR-049 (pending)”**. Number **049 was claimed** by
  the WeftOS kernel overview when `docs/architecture/adr-028-weftos-kernel.md`
  was renumbered/relocated to `docs/adr/adr-049-weftos-kernel.md` (WEFT-140,
  2026-04-28). This document is the canonical graphify-port ADR under the
  next free index number (**082**).
- **Related**: ADR-009 (sparse Lanczos), ADR-011 (no FrankenSearch / raw HNSW),
  ADR-021 (CLI → kernel), ADR-023 (assessment / Analyzer registry), ADR-046
  (forest of trees), ADR-047 (cognitive tick), ADR-049 (kernel overview —
  *different* ADR), ADR-056 (BVH spatial index), ADR-062 (ECC graph-walk
  conversation), ADR-067 (conversation graph view); WEFT-368 (Reqwest
  HttpClient), WEFT-383 (drop dead `clawft-llm` optional dep)
- **Source**:
  - `.planning/graphify-rs/MASTER_PLAN.md`
  - `.planning/graphify-rs/architecture.md`
  - `.planning/graphify-rs/phase1a-notes.md`, `phase3-notes.md`,
    `phase45-notes.md`
  - `.planning/reviews/0.7.0-release-gate/12-knowledge-graph-graphify.md`
  - `.planning/development_notes/knowledge-graph-paper-survey.md` (+ phase2)
  - `crates/clawft-graphify/`

## Context

WeftOS needed a first-party **knowledge-graph builder** that:

1. Extracts entities and relationships from **source code** (AST) and
   **documents** (LLM / vision),
2. Clusters and analyzes structure (communities, god nodes, gaps, surprises),
3. Exports queryable / visual artifacts, and
4. Bridges into the ECC substrate (`CausalGraph`, HNSW, `CrossRefStore`) for
   assessment and agent retrieval.

The upstream inspiration was the **Python Graphify** stack (~11 K LOC, ~20
modules, 126 tests): NetworkX graphs, tree-sitter extraction, LLM semantic
extraction, community detection, and multi-format export. Running that stack
in-process against WeftOS was a non-starter:

| Constraint | Python Graphify | WeftOS requirement |
|------------|-----------------|--------------------|
| Runtime | CPython + pip deps | Ship inside `weaver` / daemon binary |
| Graph store | NetworkX | Native petgraph + optional ECC bridge |
| Vectors | External / ad-hoc | `HnswService` / `VectorBackend` (ADR-011) |
| Audit / identity | None | ExoChain, BLAKE3 entity IDs, Analyzer registry |
| Performance | ~10 min / 10 K files (order-of-magnitude) | Target &lt; 2 min / 10 K files (Rayon + cache) |

Without a Rust port, knowledge-graph work would remain a sidecar process, break
lockstep versioning (ADR-001), and could not be the 9th kernel `Analyzer` or
feed DEMOCRITUS / assess paths.

Planning (2026-04-04) specified crate `clawft-graphify`, phases 1A–6, dual
domains (code + forensic), and marked **ADR-049 pending**. Implementation
shipped through Phases 1–5 and most of Sprint 17 (KG-001..KG-010, KG-014,
KG-016; OG-1) **before** this ADR was written. WEFT-371 closes that governance
gap: record the port decision as **Accepted** against the architecture that
actually landed.

## Decision

### 1. Port Graphify into workspace crate `clawft-graphify`

**Accept** a first-party Rust crate at `crates/clawft-graphify/` as the
canonical knowledge-graph extraction, analysis, query, and export engine for
WeftOS. Do **not** vendor or shell out to the Python package in production
paths. CLI surface is `weaver graphify …` in `clawft-weave`
(`graphify_cmd.rs`).

### 2. Dual-domain, single substrate

One `KnowledgeGraph` model serves two product domains:

| Domain | Entity focus | Notable APIs |
|--------|--------------|--------------|
| **Code assessment** | Module, Class, Function, Import, Service, Endpoint, … | AST extract, cross-file resolve, god nodes, dependency BFS |
| **Forensic analysis** | Person, Event, Evidence, Location, Timeline, Hypothesis, … | `gap_analysis()`, `coherence_score()`, `counterfactual_delta()` |

Domain packs live under `domain/{code,forensic}.rs` behind `code-domain` /
`forensic-domain` features. Shared types: `Entity`, `Relationship`,
`EntityId`, taxonomies in `entity.rs` / `relationship.rs`.

### 3. Core data model and identity

- **`EntityId`**: BLAKE3 of `(domain_tag_byte, type_discriminant, name,
  source_file)` — 32-byte stable ID; `from_legacy_string()` preserves Python
  JSON import compatibility.
- **`KnowledgeGraph`**: `petgraph::Graph<Entity, Relationship, Directed>` plus
  `HashMap<EntityId, NodeIndex>` for O(1) lookup; idempotent insert;
  subgraph extraction.
- **Taxonomies**: ~26 entity types and ~23 relation types with **frozen
  discriminants** (e.g. `Struct` → `"struct_"`, `Enum` → `"enum_"`) so IDs
  remain stable across releases.
- **Confidence**: UPPERCASE serde matching Python; separate graph weight vs
  export score mappings.
- **Cache**: BLAKE3 content-hash under `.weftos/graphify-cache/` with
  `EXTRACTOR_VERSION` invalidation (not Python’s SHA-256 path).

### 4. Standalone vs kernel-bridged

| Mode | Feature | Behavior |
|------|---------|----------|
| **Standalone** (default) | no `kernel-bridge` | Build / query / export `KnowledgeGraph` with no `clawft-kernel` dep |
| **Kernel-bridged** | `kernel-bridge` | `GraphifyBridge` maps into `CausalGraph`, indexes entities in HNSW, registers cross-refs; `GraphifyAnalyzer` is the 9th kernel `Analyzer` (ADR-023) |

Kernel imports the crate; the crate does not pull kernel unless opted in
(same dependency direction as ADR-056’s `clawft-bvh` pattern).

### 5. Feature gates and language coverage

As shipped in `crates/clawft-graphify/Cargo.toml`:

```text
default        = ["code-domain"]
ast-extract    → tree-sitter
lang-{python,javascript,typescript,rust,go,java,c,cpp,ruby,csharp}
lang-all       → all lang-*
semantic-extract / vision-extract  → callback-based LLM (no clawft-llm dep)
html-export, neo4j-export, kernel-bridge, http-client, full
```

**Languages**: ten tree-sitter grammars (Python, JS/TS, Rust, Go, Java, C,
C++, Ruby, C#) plus generic / tree-calculus shape detection
(`extract/treecalc.rs`, shared `clawft-treecalc`).

**HTTP**: `HttpClient` trait + `StubHttpClient` for tests; production
`ReqwestHttpClient` behind `http-client` (WEFT-368). Weaver enables
`http-client`.

**LLM**: `semantic-extract` / `vision-extract` take
`FnOnce(String) -> Future` callbacks rather than depending on `clawft-llm`
(testability; dead optional dep removed in WEFT-383).

### 6. Pipeline architecture

```text
detect → extract (AST | semantic | vision) → build → cluster → analyze
       → (optional) summary / topology / bridge → export / report
```

Orchestrated by `pipeline.rs`. Supporting surfaces:

- **Ingest** — URL fetch with SSRF denylist (`file://`, localhost, RFC1918)
- **Watch** — polling default; optional `notify`
- **Hooks** — git post-commit / post-checkout → rebuild
- **Vault** — Obsidian frontmatter / wikilinks (v0.6.11+)
- **Conversation** — multi-turn graph exploration (`conversation.rs`, KG-016)
- **EML models** — surprise scorer, cluster threshold, query fusion (KG-001),
  community summary hooks

CLI: `ingest | query | export | diff | rebuild | watch | hooks`.

### 7. Sprint 17 / research layer (accepted as in-tree direction)

Paper-survey algorithms (GraphRAG, CausalRAG, SASE, SGKR, RANGER, CodaRAG,
RoMem, TRACE, TransFIR, …) land **on top of** the port substrate, not as a
separate product:

| ID | Theme | Status (as of WEFT-371 close) |
|----|-------|-------------------------------|
| KG-001 | EML query score fusion | Done |
| KG-002 | Community summaries (GraphRAG) | Done |
| KG-003 | Causal chain tracing | Done (kernel) |
| KG-004 | RFF spectral embedding (SASE) | Done in tree; size-threshold vs Lanczos open |
| KG-005 | Information-gain pruning | Done (kernel) |
| KG-006 | BFS dependency retrieval (SGKR) | Done |
| KG-007 | MCTS exploration (RANGER) | Done |
| KG-008 | Entity dedup via HNSW (CodaRAG) | Done |
| KG-009 | Geometric shadowing (RoMem) | Done (kernel) |
| KG-010 | Multi-hop beam (TRACE) | Done |
| KG-011 / 012 | LogQuantized / SIMD distance | Stub; blocked on ruvector upstream |
| KG-013 | K-STEMIT spatio-temporal GNN | Not started |
| KG-014 | Codebook cold-start (TransFIR) | Done (kernel) |
| KG-015 | EA-Agent multi-repo alignment | Not started |
| KG-016 | Conversational graph exploration | Done |
| KG-017 / 018 | Edge EML distill / Newman modularity | Not started |
| OG-1 | VOWL export | Done |
| OG-2..4 | OWL ingest, full force/SVG VOWL | Open / partial |

Retrospective ADRs for individual algorithms (historically sketched as
ADR-050..053 in the phase-2 survey) are **out of scope for this document**;
they remain WEFT-372 (and must also renumber if those slots are occupied).

### 8. Explicit non-goals (port decision)

- **Not** a replacement for ECC causal / HNSW / BVH indexes — graphify
  *feeds and queries* them via the bridge.
- **Not** a general graph database product (Neo4j/Cypher export is optional
  interchange, not the system of record).
- **Not** FrankenSearch or a new similarity index (ADR-011 stands).
- **Not** mandatory Python interop at runtime after the port.

## Phase status (port timeline)

| Phase | Scope | Status |
|-------|--------|--------|
| **1A** | Core model, build, cache, validation, JSON export | Shipped 2026-04-04 (`phase1a-notes.md`) |
| **1B / 2** | AST extract, cluster, analyze, report | Shipped |
| **3** | Kernel bridge, code + forensic domains | Shipped (`phase3-notes.md`) |
| **4–5** | Semantic/vision extract, ingest, watch, hooks, extra exports, CLI | Shipped (`phase45-notes.md`) |
| **Sprint 16** | Vector / hybrid HNSW substrate (kernel; consumed by WS12) | Shipped |
| **Sprint 17** | KG-NNN + OG pipeline | Mostly landed; residue in deferred table |
| **6 (plan)** | MCP server, dedicated benches | Still open |
| **This ADR** | Governance record of the port | **Accepted** (WEFT-371) |

Approximate in-tree size: ~20 K+ LOC across ~50 Rust modules under
`crates/clawft-graphify/src/` (grew past the original ~12 K port estimate as
Sprint 17 and vault/layout/topology landed).

## Alternatives considered

| Option | Why rejected |
|--------|--------------|
| **Keep Python Graphify as sidecar** | Extra runtime, version skew, no native CausalGraph/HNSW/ExoChain, weak distribution story for `weaver` |
| **FFI / PyO3 bridge only** | Same dep surface; harder CI; partial type safety; still NetworkX memory profile |
| **Thin wrapper over Neo4j / external KG SaaS** | Offline-first and lockstep release break; audit chain stays outside WeftOS |
| **Fold entirely into `clawft-kernel`** | Couples tree-sitter/export weight to every kernel consumer; loses standalone CLI/test mode |
| **Greenfield design ignoring Python** | Would discard proven extract/cluster/export UX and JSON interchange used by existing tooling |

## Deferred / open items (not blocking this ADR)

Documented so status is honest; tracked under workstream 12 / related WEFTs:

1. Standalone `export/cypher.rs` and `export/svg.rs` (enum variants exist;
   dispatcher / modules incomplete vs MASTER_PLAN names).
2. Sugiyama layered layout (`layout/mod.rs` falls back to tree).
3. Full rebuild ↔ extraction pipeline integration polish.
4. MCP graphify server and criterion benches named in Phase 6.
5. KG-011/012/013/015/017/018 and OG-2..4 (table above).
6. Incremental pipeline (`run_incremental`), multi-key / edge embeddings,
   graph-aware HNSW re-rank (paper-survey open questions).
7. Schema-driven edge validation beyond JSON shape checks.
8. Vision extract E2E fixtures under default CI.

## Consequences

### Positive

- Single Rust crate owns knowledge-graph capability; ships with WeftOS
  binaries without a Python runtime.
- Stable BLAKE3 entity identity and petgraph model integrate cleanly with
  ECC / HNSW / CrossRef when `kernel-bridge` is on.
- Feature gates keep default builds lean (`code-domain` only) while
  `full` / `lang-all` support assessment and research workloads.
- Dual domains share one pipeline; forensic and code do not fork the crate.
- Sprint 17 algorithms have a clear substrate and CLI (`weaver graphify
  query` fusion, community summaries, conversation mode).
- Governance: MASTER_PLAN “ADR pending” debt closed (WEFT-371).

### Negative

- Large crate surface (~20 K+ LOC) increases review and clippy burden;
  must stay modular and feature-gated.
- Doc/numbering drift: “ADR-049” in older planning docs means *this*
  decision, not the kernel overview — readers must follow the historical
  alias note above.
- Export and OG pipeline incomplete relative to the original master plan;
  enum variants can imply formats that the top-level `export()` still
  rejects.

### Neutral

- Python JSON `node_link_data` interchange remains a compatibility target
  for import/export tests, not a runtime dependency.
- Kernel-side KG pieces (causal trace, RFF, RoMem, TransFIR stubs) live in
  `clawft-kernel` but are conceptually workstream-12 features.
- Further algorithm-specific ADRs (WEFT-372 and successors) refine *how*
  retrieval works; they do not reopen *whether* the Rust port exists.

## Implications

- **Code**: no functional change required to accept this ADR — the crate is
  already the system of record. Future graphify work lands under
  `crates/clawft-graphify/` (+ weave CLI + kernel bridge consumers).
- **Docs**: this file is the canonical decision record; planning docs that
  say “ADR-049 (pending)” should be read as **ADR-082 Accepted**.
- **Index**: listed in `docs/adr/README.md` as ADR-082.
- **Follow-ups**: WEFT-372 (algorithm ADRs), remaining KG/OG backlog from
  release-gate workstream 12.

## References

1. `.planning/graphify-rs/MASTER_PLAN.md` — original port plan
2. `.planning/graphify-rs/architecture.md` — crate layout & feature design
3. `.planning/graphify-rs/phase1a-notes.md`, `phase3-notes.md`,
   `phase45-notes.md` — implementation decisions
4. `.planning/reviews/0.7.0-release-gate/12-knowledge-graph-graphify.md` —
   audit inventory, phase status, KG/OG task tables
5. `.planning/development_notes/knowledge-graph-paper-survey.md` (+ phase2)
6. `crates/clawft-graphify/` — implementation
7. `crates/clawft-weave/src/commands/graphify_cmd.rs` — CLI
8. ADR-049 (`adr-049-weftos-kernel.md`) — number collision explanation only
)
