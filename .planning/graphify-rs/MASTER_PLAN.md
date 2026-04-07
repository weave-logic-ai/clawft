# Graphify Rust Port -- Master Implementation Plan

**Crate:** `clawft-graphify`
**ADR:** ADR-049 (pending)
**Source:** Graphify Python (11K LOC, 20 modules, 126 tests)
**Target:** WeftOS workspace crate at `crates/clawft-graphify/`
**Date:** 2026-04-04

---

## Overview

### What We Are Building

A Rust knowledge graph builder that extracts entities and relationships from source code (via tree-sitter AST) and documents (via LLM semantic extraction), clusters them into communities, analyzes structural patterns, and exports interactive visualizations. It serves two domains:

1. **Code Assessment:** Automated codebase understanding -- maps modules, classes, functions, imports, call graphs, and inferred dependencies into a queryable knowledge graph. Integrates as the 9th analyzer in the WeftOS assessment pipeline.

2. **Forensic Analysis:** Document/evidence analysis for investigative workflows -- extracts persons, events, evidence, locations, timelines and their relationships. Identifies gaps, contradictions, and weak links in case documentation.

### Why Rust

- **Performance:** 10K files in < 2 minutes (vs ~10 minutes in Python). Rayon parallelism for CPU-bound AST extraction, tokio for IO-bound LLM calls.
- **Memory:** 50K entity graph in < 500 MB (petgraph vs NetworkX overhead).
- **Integration:** Native access to CausalGraph, HNSW, CrossRefStore, ExoChain provenance, and DEMOCRITUS loop -- no FFI bridge.
- **Distribution:** Ships as part of the `weaver` CLI binary. No Python runtime dependency.

### Source Inventory

| Python Module | LOC | Rust Module | Priority |
|---------------|-----|-------------|----------|
| extract.py | 1,734 | extract/ast.rs | P0 |
| build.py | 71 | build.rs | P0 |
| detect.py | 280 | extract/detect.rs | P0 |
| cluster.py | 130 | cluster.rs | P0 |
| analyze.py | 290 | analyze.rs | P0 |
| export.py | 680 | export/*.rs | P0 (JSON), P1 (others) |
| cache.py | 95 | cache.rs | P0 |
| validate.py | 55 | (serde validation) | P0 |
| report.py | 190 | report.rs | P1 |
| security.py | 95 | (clawft-security) | P1 |
| serve.py | 240 | (MCP server) | P2 |
| watch.py | 85 | watch.rs | P1 |
| hooks.py | 110 | hooks.rs | P2 |
| wiki.py | 160 | export/wiki.rs | P2 |
| benchmark.py | 75 | bench/ | P2 |
| ingest.py | 292 | ingest.rs | P1 |
| manifest.py | 5 | (merged into detect) | P0 |

---

## Crate Layout

```
crates/clawft-graphify/
  Cargo.toml
  src/
    lib.rs                    # Public API re-exports, feature gates, GraphifyError
    model.rs                  # Entity, Relationship, KnowledgeGraph, ExtractionResult, ExtractionStats
    entity.rs                 # EntityId (BLAKE3), EntityType taxonomy, DomainTag
    relationship.rs           # RelationType taxonomy, Confidence enum
    extract/
      mod.rs                  # Extractor trait, ExtractionPipeline, language dispatch
      ast.rs                  # Generic LanguageConfig-driven tree-sitter extraction
      lang/
        mod.rs                # LanguageId enum, extension -> language mapping
        python.rs             # Python config + rationale post-pass
        javascript.rs         # JS/TS config + arrow function handling
        java.rs               # Java config + scoped import handler
        c.rs                  # C config + declarator unwrapping
        cpp.rs                # C++ config + qualified identifier handling
        ruby.rs               # Ruby config
        csharp.rs             # C# config + namespace walk
        kotlin.rs             # Kotlin config
        scala.rs              # Scala config
        php.rs                # PHP config
        lua.rs                # Lua config + require() regex
        swift.rs              # Swift config + enum case + conformance
        go.rs                 # Go custom extractor (not config-driven)
        rust_lang.rs          # Rust custom extractor (not config-driven)
      semantic.rs             # LLM-based semantic extraction (feature-gated)
      vision.rs               # Image/diagram extraction via Claude vision (feature-gated)
      detect.rs               # File discovery, classification, sensitive file filtering, manifest
      cross_file.rs           # Cross-file import resolution (currently Python-only, extensible)
    build.rs                  # Graph assembly: merge extractions, deduplicate entities
    cluster.rs                # Community detection (Leiden, label propagation fallback)
    analyze.rs                # God nodes, surprising connections, questions, graph diff
    export/
      mod.rs                  # ExportFormat enum, dispatch
      json.rs                 # JSON serialization (node_link_data compatible)
      graphml.rs              # GraphML export
      cypher.rs               # Neo4j Cypher text export
      html.rs                 # Interactive HTML/vis.js visualization
      obsidian.rs             # Obsidian vault + canvas export
      wiki.rs                 # Wikipedia-style article generation
      svg.rs                  # SVG graph rendering
    bridge.rs                 # CausalGraph/CrossRefStore/HNSW integration (feature-gated)
    pipeline.rs               # Full pipeline orchestrator
    cache.rs                  # BLAKE3 content-hash cache
    report.rs                 # GRAPH_REPORT.md generation
    ingest.rs                 # URL fetching + query result storage
    watch.rs                  # File watcher (notify crate)
    hooks.rs                  # Git hook installation/management
    domain/
      mod.rs                  # Domain trait, domain registry
      code.rs                 # Code analysis domain config
      forensic.rs             # Forensic domain: gap analysis, coherence scoring
  tests/
    fixtures/
      extraction.json         # Canonical test graph (ported from Python)
      python_sample/          # Multi-file Python project for extraction tests
      rust_sample/            # Multi-file Rust project
      go_sample/              # Multi-file Go project
    test_model.rs             # Entity, Relationship, KnowledgeGraph unit tests
    test_extract_python.rs    # Python AST extraction snapshot tests
    test_extract_js.rs        # JS/TS extraction snapshot tests
    test_extract_rust.rs      # Rust extraction snapshot tests
    test_extract_go.rs        # Go extraction snapshot tests
    test_build.rs             # Graph assembly + deduplication tests
    test_detect.rs            # File discovery + classification tests
    test_cluster.rs           # Community detection tests
    test_analyze.rs           # God nodes, surprises, questions, diff tests
    test_export_json.rs       # JSON export schema conformance
    test_export_html.rs       # HTML export structure tests
    test_cache.rs             # Cache roundtrip + invalidation tests
    test_pipeline.rs          # End-to-end pipeline integration tests
    test_bridge.rs            # CausalGraph bridge tests (feature-gated)
  benches/
    extraction.rs             # Benchmark: 1K files extraction throughput
    graph_ops.rs              # Benchmark: 50K node graph operations
```

### Cargo.toml

```toml
[package]
name = "clawft-graphify"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[features]
default = ["code-domain", "ast-extract", "lang-python", "lang-javascript",
           "lang-typescript", "lang-rust", "lang-go"]
ast-extract = ["dep:tree-sitter"]
semantic-extract = []
vision-extract = []
code-domain = []
forensic-domain = []
html-export = ["dep:html-escape"]
neo4j-export = []
kernel-bridge = ["dep:clawft-kernel"]
full = ["ast-extract", "semantic-extract", "vision-extract", "code-domain",
        "forensic-domain", "html-export", "neo4j-export", "kernel-bridge",
        "lang-all"]

lang-python = ["ast-extract", "dep:tree-sitter-python"]
lang-javascript = ["ast-extract", "dep:tree-sitter-javascript"]
lang-typescript = ["ast-extract", "dep:tree-sitter-typescript"]
lang-rust = ["ast-extract", "dep:tree-sitter-rust"]
lang-go = ["ast-extract", "dep:tree-sitter-go"]
lang-java = ["ast-extract", "dep:tree-sitter-java"]
lang-c = ["ast-extract", "dep:tree-sitter-c"]
lang-cpp = ["ast-extract", "dep:tree-sitter-cpp"]
lang-ruby = ["ast-extract", "dep:tree-sitter-ruby"]
lang-csharp = ["ast-extract", "dep:tree-sitter-c-sharp"]
lang-all = ["lang-python", "lang-javascript", "lang-typescript", "lang-rust",
            "lang-go", "lang-java", "lang-c", "lang-cpp", "lang-ruby", "lang-csharp"]

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
blake3 = { workspace = true }
dashmap = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
regex = { workspace = true }
rayon = "1.10"
petgraph = "0.7"
walkdir = "2"

# Optional
tree-sitter = { version = "0.24", optional = true }
tree-sitter-python = { version = "0.23", optional = true }
tree-sitter-javascript = { version = "0.23", optional = true }
tree-sitter-typescript = { version = "0.23", optional = true }
tree-sitter-rust = { version = "0.23", optional = true }
tree-sitter-go = { version = "0.23", optional = true }
tree-sitter-java = { version = "0.23", optional = true }
tree-sitter-c = { version = "0.23", optional = true }
tree-sitter-cpp = { version = "0.23", optional = true }
tree-sitter-ruby = { version = "0.23", optional = true }
tree-sitter-c-sharp = { version = "0.23", optional = true }
html-escape = { version = "0.2", optional = true }
clawft-kernel = { workspace = true, optional = true }
clawft-llm = { workspace = true, optional = true }
notify = { workspace = true, optional = true }

[dev-dependencies]
insta = { workspace = true }
tempfile = { workspace = true }
```

---

## Implementation Tasks

### Phase 1: Core Model + AST Extraction

#### GRAPH-001: Core Data Model Types
- **Module:** `src/model.rs`, `src/entity.rs`, `src/relationship.rs`
- **Implement:**
  - `EntityId` struct with BLAKE3 constructor (`new(domain, entity_type, name, source)`)
  - `EntityId::from_legacy_string(s: &str)` for Python-compatible string IDs
  - `EntityId` Display (hex), Debug, Hash, Eq, Serialize/Deserialize
  - `DomainTag` enum (Code=0x20, Forensic=0x21, Custom(u8))
  - `EntityType` enum (12 code + 12 forensic + File, Concept, Custom(String))
  - `EntityType::discriminant() -> &str` for stable ID hashing
  - `FileType` enum (Code, Document, Paper, Image, Config, Unknown)
  - `Entity` struct (id, entity_type, label, source_file, source_location, file_type, metadata, legacy_id: Option<String>)
  - `Confidence` enum (Extracted, Inferred, Ambiguous) with `to_weight()` and `to_score()` methods
  - `RelationType` enum (10 code + 11 forensic + RelatedTo, CaseOf, Custom(String))
  - `Relationship` struct (source, target, relation_type, confidence, weight, source_file, source_location, metadata)
  - `ExtractionResult` struct (source_file, entities, relationships, hyperedges, input_tokens, output_tokens, errors)
  - `Hyperedge` struct (label, entity_ids, metadata)
  - `ExtractionStats` struct (files_processed, files_skipped, entities_extracted, relationships_extracted, input_tokens, output_tokens, cache_hits, extraction_duration_ms)
- **Dependencies:** None
- **Estimated complexity:** M
- **Acceptance criteria:**
  - `EntityId::new()` produces deterministic 32-byte hashes
  - `EntityId` round-trips through serde_json
  - `Confidence::to_weight()` returns 1.0/0.7/0.4
  - All enum variants serialize to lowercase strings matching Python output

#### GRAPH-002: KnowledgeGraph (petgraph wrapper)
- **Module:** `src/model.rs`
- **Implement:**
  - `KnowledgeGraph` struct wrapping `petgraph::Graph<Entity, Relationship, Directed>`
  - `entity_index: HashMap<EntityId, NodeIndex>` for O(1) lookup
  - `communities: Option<HashMap<usize, Vec<EntityId>>>`
  - `community_labels: Option<HashMap<usize, String>>`
  - `stats: ExtractionStats`
  - Methods: `new()`, `add_entity()`, `add_relationship()`, `entity()`, `entity_count()`, `relationship_count()`, `neighbors()`, `degree()`, `edges()`, `subgraph()`, `node_ids()`
  - `add_entity` must be idempotent: if entity ID exists, overwrite attributes (last-write-wins matching Python NetworkX behavior)
  - `add_relationship` must skip if source or target not in graph
- **Dependencies:** GRAPH-001
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Add 1000 entities and 3000 edges, verify counts
  - Duplicate entity add overwrites attributes
  - `neighbors()` returns correct set
  - `subgraph()` preserves edges between included nodes

#### GRAPH-003: Error Types
- **Module:** `src/lib.rs`
- **Implement:**
  - `GraphifyError` enum with variants: ExtractionFailed, GrammarNotAvailable, LlmError, CacheError, BuildError, BridgeError, DetectionError, ExportError, ValidationError
  - `thiserror` derive for all variants
  - `impl From<std::io::Error>` for CacheError
- **Dependencies:** None
- **Estimated complexity:** S
- **Acceptance criteria:**
  - All error types display meaningful messages
  - `?` operator works for common conversions

#### GRAPH-004: File Detection + Classification
- **Module:** `src/extract/detect.rs`
- **Implement:**
  - Extension sets: `CODE_EXTENSIONS`, `DOC_EXTENSIONS`, `PAPER_EXTENSIONS`, `IMAGE_EXTENSIONS` (matching Python exactly)
  - `fn classify_file(path: &Path) -> Option<FileType>`
  - `fn is_sensitive(path: &Path) -> bool` with all 6 regex patterns from Python
  - `fn looks_like_paper(path: &Path) -> bool` with 13 paper signal regexes, threshold 3
  - `fn detect(root: &Path) -> Result<DetectionResult, GraphifyError>` using `walkdir` with noise directory filtering
  - `DetectionResult` struct matching Python output
  - `SKIP_DIRS` set (37+ patterns)
  - `fn count_words(path: &Path) -> usize`
  - Corpus size warnings (50K/500K word thresholds, 200 file threshold)
  - Memory directory inclusion (`graphify-out/memory/`)
  - `fn detect_incremental(root: &Path, manifest: &Manifest) -> IncrementalDetection`
  - `Manifest` struct with load/save (JSON file of path -> mtime)
- **Dependencies:** GRAPH-001 (FileType enum)
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Classifies `.py` as Code, `.pdf` as Paper, `.md` with 3+ paper signals as Paper
  - Skips `.env`, `id_rsa`, `credentials.json`
  - Prunes `node_modules/`, `.git/`, `__pycache__/`
  - Incremental detection identifies new/changed/deleted files

#### GRAPH-005: Language ID + Extension Dispatch
- **Module:** `src/extract/lang/mod.rs`
- **Implement:**
  - `LanguageId` enum: Python, JavaScript, TypeScript, Rust, Go, Java, C, Cpp, Ruby, CSharp, Kotlin, Scala, Php, Lua, Swift
  - `fn language_for_extension(ext: &str) -> Option<LanguageId>` -- maps all 22 extensions
  - `fn collect_files(target: &Path) -> Vec<PathBuf>` -- glob for supported extensions, filter hidden dirs, sort
- **Dependencies:** None
- **Estimated complexity:** S
- **Acceptance criteria:**
  - `.tsx` maps to TypeScript, `.cc` maps to Cpp, `.kts` maps to Kotlin, `.toc` maps to Lua
  - `collect_files` returns sorted paths, excludes hidden directories

#### GRAPH-006: LanguageConfig Framework
- **Module:** `src/extract/ast.rs`
- **Implement:**
  - `LanguageConfig` struct with all fields from Python dataclass (class_types, function_types, import_types, call_types, name_field, body_field, call_function_field, call_accessor_node_types, function_boundary_types)
  - Node type sets as `&'static [&'static str]`
  - `fn make_legacy_id(parts: &[&str]) -> String` -- exact Python `_make_id` algorithm
  - `fn read_text(node: &Node, source: &[u8]) -> String`
  - `fn resolve_name(node: &Node, source: &[u8], config: &LanguageConfig) -> Option<String>`
  - `fn find_body(node: &Node, config: &LanguageConfig) -> Option<Node>`
  - Generic `fn extract_ast(path: &Path, lang: LanguageId) -> Result<ExtractionResult, GraphifyError>` implementing the two-pass extraction algorithm
  - Stack-based iterative walk (not recursive) to avoid stack overflow
  - Call graph pass with `label_to_nid` HashMap and `seen_call_pairs` HashSet
  - Edge cleaning (filter invalid sources, allow external import targets)
- **Dependencies:** GRAPH-001, GRAPH-005
- **Estimated complexity:** L
- **Acceptance criteria:**
  - `make_legacy_id(["auth", "AuthService"])` produces `"auth_authservice"`
  - Generic extraction produces correct nodes/edges for a Python file
  - Call graph pass emits INFERRED edges with weight 0.8
  - Edge cleaning removes edges with invalid sources

#### GRAPH-007: Python Language Support
- **Module:** `src/extract/lang/python.rs`
- **Implement:**
  - `PYTHON_CONFIG` constant with class_types={class_definition}, function_types={function_definition}, import_types={import_statement, import_from_statement}, etc.
  - `fn import_python(node, source, file_nid, stem, edges, str_path)` -- handles `import X` and `from X import Y`
  - Inheritance extraction from `superclasses` field (Python-specific in walk)
  - `fn extract_python_rationale(path, result)` post-pass: module/class/function docstrings + rationale comments (NOTE, IMPORTANT, HACK, WHY, RATIONALE, TODO, FIXME prefixes)
  - `fn extract_python(path) -> ExtractionResult` -- generic extraction + rationale post-pass
- **Dependencies:** GRAPH-006, `tree-sitter-python` feature
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Extracts classes, functions, imports, inheritance from a Python file
  - Rationale comments create `rationale` nodes with `rationale_for` edges
  - Snapshot test matches Python extractor output for a reference file

#### GRAPH-008: JavaScript/TypeScript Language Support
- **Module:** `src/extract/lang/javascript.rs`
- **Implement:**
  - `JS_CONFIG` and `TS_CONFIG` constants
  - `fn import_js(...)` -- string child extraction, strip `./`, take last path segment
  - Arrow function handling: detect `lexical_declaration` -> `variable_declarator` -> `arrow_function`, extract name, create entity
  - `fn extract_js(path) -> ExtractionResult` -- dispatches JS vs TS by extension
- **Dependencies:** GRAPH-006, `tree-sitter-javascript`/`tree-sitter-typescript` features
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Extracts classes, functions, arrow functions, imports
  - Arrow functions get `name()` label format
  - TS uses `language_typescript` function name

#### GRAPH-009: Rust Language Support (custom extractor)
- **Module:** `src/extract/lang/rust_lang.rs`
- **Implement:**
  - Custom walker (not LanguageConfig-driven, matching Python `extract_rust`)
  - Handles: `function_item`, `struct_item`, `enum_item`, `trait_item`, `impl_item`, `use_declaration`
  - `impl_item` extracts type name, recurses with parent_impl_nid context
  - `use_declaration` parses argument, splits on `{`, strips `::*`, takes last `::` segment
  - Call graph: handles `identifier`, `field_expression.field`, `scoped_identifier.name`
- **Dependencies:** GRAPH-006, `tree-sitter-rust` feature
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Extracts structs, enums, traits, impl methods, use declarations, function calls
  - impl methods get `.method_name()` label format
  - Snapshot test against a reference Rust file

#### GRAPH-010: Go Language Support (custom extractor)
- **Module:** `src/extract/lang/go.rs`
- **Implement:**
  - Custom walker matching Python `extract_go`
  - Handles: `function_declaration`, `method_declaration` (with receiver type extraction), `type_declaration`, `import_declaration`
  - Method receiver: extract type from `parameter_declaration.type`, strip leading `*`
  - Import: strip quotes, take last `/` segment
  - Call graph: `identifier` and `selector_expression.field`
- **Dependencies:** GRAPH-006, `tree-sitter-go` feature
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Extracts functions, methods with receiver types, type declarations, imports
  - Methods get `.method_name()` label under the receiver type node
  - Snapshot test against a reference Go file

#### GRAPH-011: Cross-File Import Resolution
- **Module:** `src/extract/cross_file.rs`
- **Implement:**
  - `fn resolve_cross_file_imports(per_file: &[ExtractionResult], paths: &[PathBuf]) -> Vec<Relationship>`
  - Pass 1: build `stem -> {ClassName: EntityId}` global map from all extraction results
  - Pass 2: for each Python file with `from .module import Name`, resolve Name in global map, emit INFERRED "uses" edges from local classes to imported entities
  - Currently Python-only (uses tree-sitter-python to re-parse imports)
  - Extensible: add TypeScript/Go/Rust import resolution in future
- **Dependencies:** GRAPH-006, GRAPH-007
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Given `auth.py` importing from `models.py`, produces class-level "uses" edges
  - Does not produce edges for unresolvable imports
  - Handles relative imports (`from .models import X`)

#### GRAPH-012: Graph Assembly (build)
- **Module:** `src/build.rs`
- **Implement:**
  - `fn build(extractions: &[ExtractionResult]) -> KnowledgeGraph`
  - Concatenates entities, relationships, hyperedges from all extractions
  - Deduplicates entities by ID (last-write-wins for attributes)
  - Skips relationships where source or target is not in entity set (expected for external/stdlib imports)
  - Preserves original edge direction in metadata
  - Accumulates token counts
  - Stores hyperedges in graph metadata
  - `fn build_from_json(extraction: &serde_json::Value) -> Result<KnowledgeGraph, GraphifyError>` for loading from JSON
- **Dependencies:** GRAPH-002
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Merging two extractions with overlapping entity IDs keeps last attributes
  - External import edges are silently dropped (not errors)
  - Round-trip: extract -> build -> export JSON -> build_from_json produces equivalent graph

#### GRAPH-013: Content-Hash Cache
- **Module:** `src/cache.rs`
- **Implement:**
  - `ContentCache` struct with `cache_dir: PathBuf` and `DashMap<[u8; 32], CacheEntry>` index
  - `CacheEntry`: content_hash, extraction result, extracted_at timestamp, extractor_version
  - Cache key: `BLAKE3(file_content_bytes)` (NOT SHA256 as in Python -- use BLAKE3 for consistency with the rest of WeftOS)
  - `fn get(&self, path: &Path) -> Option<ExtractionResult>` -- reads file, hashes, looks up
  - `fn put(&self, path: &Path, result: &ExtractionResult) -> Result<(), io::Error>` -- atomic write (write .tmp, rename)
  - `fn gc(&self, live_paths: &[PathBuf]) -> usize` -- remove entries for deleted files
  - `fn clear(&self) -> Result<(), io::Error>` -- remove all cache entries
  - Cache location: `.weftos/graphify-cache/` (NOT `graphify-out/cache/` as in Python)
  - Invalidation: content change (hash mismatch) or extractor version change
- **Dependencies:** GRAPH-001
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Cache hit returns correct ExtractionResult
  - Modifying file content causes cache miss
  - Atomic writes: crash during write does not corrupt cache
  - GC removes stale entries, returns count

#### GRAPH-014: Schema Validation
- **Module:** Integrated into serde derives + custom validation in `build.rs`
- **Implement:**
  - `fn validate_extraction(data: &serde_json::Value) -> Vec<String>` matching Python's validate.py checks:
    1. Top-level is object
    2. `nodes` key exists and is array
    3. Each node has required fields (id, label, file_type, source_file)
    4. `file_type` is valid enum value
    5. `edges` key exists and is array
    6. Each edge has required fields (source, target, relation, confidence, source_file)
    7. `confidence` is valid enum value
    8. Edge source/target reference existing node IDs (warn, don't error -- external refs expected)
- **Dependencies:** GRAPH-001
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Valid extraction passes with no errors
  - Missing `nodes` key returns error
  - Invalid `file_type` returns error
  - Dangling edge reference returns warning (not error)

#### GRAPH-015: JSON Export
- **Module:** `src/export/json.rs`
- **Implement:**
  - `fn to_json(kg: &KnowledgeGraph, output: &Path) -> Result<(), GraphifyError>`
  - Output schema must match Python's `node_link_data` format exactly:
    ```json
    {
      "directed": false,
      "multigraph": false,
      "graph": {},
      "nodes": [{"id": "...", "label": "...", "file_type": "...", "source_file": "...", "source_location": "...", "community": int}],
      "links": [{"source": "...", "target": "...", "relation": "...", "confidence": "...", "confidence_score": float, "weight": float, "_src": "...", "_tgt": "..."}],
      "hyperedges": [...]
    }
    ```
  - Add `community` attribute to nodes from KnowledgeGraph communities
  - Add default `confidence_score` (EXTRACTED=1.0, INFERRED=0.5, AMBIGUOUS=0.2) if not present
  - Use legacy string IDs (not BLAKE3 hex) for backward compatibility
- **Dependencies:** GRAPH-002
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Output JSON matches Python schema exactly (field names, types, ordering)
  - Round-trip: export -> import produces equivalent graph
  - Community assignments appear on node objects

### Phase 2: Graph Analysis + Export

#### GRAPH-016: Community Detection (Leiden)
- **Module:** `src/cluster.rs`
- **Implement:**
  - `fn cluster(kg: &KnowledgeGraph) -> HashMap<usize, Vec<EntityId>>`
  - Leiden algorithm implementation on petgraph (or label propagation fallback)
  - Handle edge cases: empty graph -> {}, edgeless graph -> each node is own community
  - Separate isolates from connected nodes, run Leiden on connected subgraph
  - Assign isolates to individual communities
  - **Split oversized communities:** if community > 25% of graph AND >= 10 nodes, run second Leiden pass
  - Re-index communities by size descending (0 = largest)
  - `fn cohesion_score(kg: &KnowledgeGraph, community_nodes: &[EntityId]) -> f64` -- edge density metric
  - `fn score_all(kg: &KnowledgeGraph, communities: &HashMap<usize, Vec<EntityId>>) -> HashMap<usize, f64>`
  - `fn auto_label(kg: &KnowledgeGraph, community: &[EntityId]) -> String` -- most common source file stem or highest-degree node label
  - Constants: `MAX_COMMUNITY_FRACTION = 0.25`, `MIN_SPLIT_SIZE = 10`
- **Dependencies:** GRAPH-002
- **Estimated complexity:** L
- **Acceptance criteria:**
  - Cluster returns dict covering all nodes
  - Cohesion score = 1.0 for complete subgraph, 0.0 for disconnected
  - Oversized communities get split
  - Communities indexed by size descending

#### GRAPH-017: God Nodes Analysis
- **Module:** `src/analyze.rs`
- **Implement:**
  - `fn god_nodes(kg: &KnowledgeGraph, top_n: usize) -> Vec<GodNode>`
  - `GodNode` struct: `{id: EntityId, label: String, edges: usize}`
  - Sort by degree descending, exclude file nodes and concept nodes
  - `fn is_file_node(kg: &KnowledgeGraph, id: &EntityId) -> bool` -- 3 conditions: label matches filename, label starts with `.` and ends with `()`, label ends with `()` and degree <= 1
  - `fn is_concept_node(kg: &KnowledgeGraph, id: &EntityId) -> bool` -- empty source_file or no file extension
- **Dependencies:** GRAPH-002
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Returns top N entities by degree, excluding file/concept nodes
  - Output matches Python god_nodes for reference fixture

#### GRAPH-018: Surprising Connections
- **Module:** `src/analyze.rs`
- **Implement:**
  - `fn surprising_connections(kg: &KnowledgeGraph, top_n: usize) -> Vec<SurprisingConnection>`
  - Strategy selector: multi-file -> `cross_file_surprises()`, single-file -> `cross_community_surprises()`
  - **Surprise scoring (5 factors):**
    1. Confidence weight: AMBIGUOUS=3, INFERRED=2, EXTRACTED=1
    2. Cross file-type bonus (+2): code<->paper, code<->image
    3. Cross-repo bonus (+2): different top-level directory
    4. Cross-community bonus (+1)
    5. Peripheral->hub bonus (+1): low-degree node reaching high-degree node
    6. Semantic similarity multiplier (1.5x)
  - Returns: `[{source, target, source_files, confidence, relation, why: Vec<String>}]`
  - `cross_file_surprises`: exclude structural edges (imports, contains, method), concept nodes, file nodes, same-file edges
  - `cross_community_surprises`: fallback uses edge betweenness centrality; with communities uses cross-boundary edges; deduplicates by community pair
- **Dependencies:** GRAPH-002, GRAPH-016
- **Estimated complexity:** L
- **Acceptance criteria:**
  - AMBIGUOUS edges score higher than EXTRACTED
  - Cross-file-type edges score higher than same-type
  - Returns human-readable `why` reasons
  - Deduplicates by community pair

#### GRAPH-019: Question Generation
- **Module:** `src/analyze.rs`
- **Implement:**
  - `fn suggest_questions(kg: &KnowledgeGraph, top_n: usize) -> Vec<SuggestedQuestion>`
  - `SuggestedQuestion` struct: `{question_type: QuestionType, question: String, why: String}`
  - **5 strategies:**
    1. AMBIGUOUS edges: "What is the exact relationship between X and Y?"
    2. Bridge nodes (betweenness centrality top-3): "Why does X connect A to B?"
    3. God nodes with INFERRED edges: "Are the N inferred relationships involving X correct?"
    4. Isolated/weakly-connected nodes (degree <= 1): "What connects X, Y, Z to the rest?"
    5. Low-cohesion communities (score < 0.15, size >= 5): "Should X be split?"
  - Fallback: `{type: NoSignal}` if no questions generated
- **Dependencies:** GRAPH-002, GRAPH-016
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Generates at least one question for a non-trivial graph
  - Bridge node questions reference actual shortest paths
  - Isolated node questions batch multiple nodes

#### GRAPH-020: Graph Diff
- **Module:** `src/analyze.rs`
- **Implement:**
  - `fn graph_diff(old: &KnowledgeGraph, new: &KnowledgeGraph) -> GraphDiff`
  - `GraphDiff` struct: `{new_nodes, removed_nodes, new_edges, removed_edges, summary: String}`
  - Edge identity: `(min(u,v), max(u,v), relation)` -- undirected edge key
  - Compares entity sets and edge sets
- **Dependencies:** GRAPH-002
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Adding a node appears in `new_nodes`
  - Removing an edge appears in `removed_edges`
  - Empty diff when comparing graph to itself

#### GRAPH-021: Pipeline Orchestrator
- **Module:** `src/pipeline.rs`
- **Implement:**
  - `PipelineConfig` struct: extractors, ast_parallelism, semantic_concurrency, cluster, analyze, exports, cache_dir, domain
  - `Pipeline` struct with `new(config)` and `async fn run(paths) -> Result<PipelineResult, GraphifyError>`
  - `PipelineResult` struct: graph, analysis, stats
  - `AnalysisResult` struct: god_nodes, surprising_connections, questions, communities, community_labels, cohesion_scores
  - Orchestration: detect -> extract (rayon parallel) -> build -> cluster -> analyze
  - Partial failure handling: skip failed files, report in stats
  - `fn extract_only(paths) -> Result<Vec<ExtractionResult>, GraphifyError>` for incremental updates
- **Dependencies:** GRAPH-004, GRAPH-006, GRAPH-012, GRAPH-013, GRAPH-016, GRAPH-017, GRAPH-018, GRAPH-019
- **Estimated complexity:** L
- **Acceptance criteria:**
  - Full pipeline runs on a multi-file project
  - Cache is used for unchanged files
  - Partial failures don't abort the pipeline
  - Stats report correct counts

#### GRAPH-022: HTML/vis.js Export
- **Module:** `src/export/html.rs`
- **Implement:**
  - `fn to_html(kg: &KnowledgeGraph, output: &Path) -> Result<(), GraphifyError>`
  - Max 5,000 nodes (error above this)
  - vis.js forceAtlas2Based physics with exact parameters from Python
  - Node sizing: `10 + 30 * (degree / max_degree)`
  - Node labels: shown only for degree >= 15% of max_degree
  - Edge styling: EXTRACTED = solid width-2 opacity-0.7; non-EXTRACTED = dashed width-1 opacity-0.35
  - 10 community colors matching Python palette
  - Interactive features: click-to-inspect sidebar, search with autocomplete (top 20), community legend with toggle, neighbor navigation, stats footer
  - Hyperedge rendering: convex hull polygons
  - XSS prevention: `sanitize_label()` on all labels
  - Self-contained HTML file (vis.js inlined or CDN link)
- **Dependencies:** GRAPH-002, GRAPH-016, `html-export` feature
- **Estimated complexity:** L
- **Acceptance criteria:**
  - Output is valid HTML that opens in a browser
  - Contains vis.js library reference
  - Search box works with autocomplete
  - Community colors match Python palette
  - Labels sanitized (no XSS)

#### GRAPH-023: GraphML Export
- **Module:** `src/export/graphml.rs`
- **Implement:**
  - `fn to_graphml(kg: &KnowledgeGraph, output: &Path) -> Result<(), GraphifyError>`
  - Standard GraphML XML format
  - Adds `community` attribute to nodes
  - Opens correctly in Gephi and yEd
- **Dependencies:** GRAPH-002, GRAPH-016
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Valid XML output
  - Gephi can import the file

#### GRAPH-024: Cypher Export
- **Module:** `src/export/cypher.rs`
- **Implement:**
  - `fn to_cypher(kg: &KnowledgeGraph, output: &Path) -> Result<(), GraphifyError>`
  - MERGE-based statements for idempotent import
  - Node labels from file_type (capitalized, alphanumeric only, fallback "Entity")
  - Edge relation types: uppercase, non-alphanumeric replaced with `_`
  - Cypher injection protection: escape backslash and single quote
- **Dependencies:** GRAPH-002
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Output is valid Cypher syntax
  - MERGE statements are safe to re-run
  - Special characters escaped

#### GRAPH-025: Report Generation
- **Module:** `src/report.rs`
- **Implement:**
  - `fn generate(kg: &KnowledgeGraph, analysis: &AnalysisResult, root: &str) -> String`
  - 10 sections matching Python output: Header, Corpus Check, Summary, God Nodes, Surprising Connections, Hyperedges, Communities, Ambiguous Edges, Knowledge Gaps, Suggested Questions
  - Confidence breakdown percentages
  - Community cohesion scores
  - Knowledge gap detection: isolated nodes, thin communities, high ambiguity warning
- **Dependencies:** GRAPH-002, GRAPH-016, GRAPH-017, GRAPH-018, GRAPH-019
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Report contains all 10 sections
  - Confidence percentages sum to ~100%
  - Community section shows top 8 real nodes per community

### Phase 3: WeftOS Bridge

#### GRAPH-026: CausalGraph Bridge
- **Module:** `src/bridge.rs` (feature-gated: `kernel-bridge`)
- **Implement:**
  - `GraphifyBridge` struct holding `Arc<CausalGraph>`, `Arc<HnswService>`, `Arc<CrossRefStore>`
  - `entity_to_causal: DashMap<EntityId, CausalNodeId>` for reverse lookup
  - `fn ingest(kg: &KnowledgeGraph, ...) -> Result<IngestResult, GraphifyError>`
  - `fn ingest_entity(entity, embedding_provider, hlc_timestamp, chain_seq) -> Result<CausalNodeId, GraphifyError>`
  - `fn ingest_relationship(rel, hlc_timestamp, chain_seq) -> Result<(), GraphifyError>`
  - RelationType -> CausalEdgeType mapping (Calls/Imports/DependsOn -> Causes; Contains/MethodOf -> Enables; Contradicts -> Contradicts; Corroborates/WitnessedBy -> EvidenceFor; Precedes -> Follows; SemanticallySimilarTo/RelatedTo -> Correlates; AlibiedBy -> Inhibits)
  - `fn causal_node_for(entity_id: &EntityId) -> Option<CausalNodeId>`
  - `IngestResult` struct: nodes_created, edges_created, crossrefs_created, embeddings_indexed, duration_ms
- **Dependencies:** GRAPH-001, GRAPH-002, clawft-kernel
- **Estimated complexity:** L
- **Acceptance criteria:**
  - Entity ingest creates CausalNode + HNSW entry + CrossRef
  - Relationship ingest creates CausalEdge + CrossRef
  - Round-trip: bridge entity, look up by EntityId, get correct CausalNodeId

#### GRAPH-027: HNSW Semantic Indexing
- **Module:** `src/bridge.rs` (part of bridge)
- **Implement:**
  - On `ingest_entity`: serialize label + metadata to text, call `embedding_provider.embed()`, insert into HNSW
  - HNSW metadata JSON: `{"entity_type": "...", "domain": "...", "source_file": "...", "label": "..."}`
  - `fn semantic_search(query: &str, limit: usize) -> Vec<(EntityId, f32)>` via HNSW cosine similarity
- **Dependencies:** GRAPH-026, clawft-kernel HNSW
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Semantic search for "authentication" returns entities with "auth" in label
  - Top results have higher cosine similarity scores

#### GRAPH-028: CrossRefStore Integration
- **Module:** `src/bridge.rs` (part of bridge)
- **Implement:**
  - Per entity: CrossRef from EntityId (StructureTag::Custom(0x20)) to ExoChain event
  - Per relationship: CrossRef from source to target, both StructureTag::Custom(0x20), with CrossRefType::Custom(relation discriminant)
  - Code domain relations: 0x20-0x2F range
  - Forensic domain relations: 0x30-0x3F range
- **Dependencies:** GRAPH-026, clawft-kernel CrossRefStore
- **Estimated complexity:** M
- **Acceptance criteria:**
  - CrossRefs created for all entities and relationships
  - Discriminant ranges don't overlap with existing CrossRefTypes

#### GRAPH-029: Assessment Analyzer Registration
- **Module:** `src/bridge.rs` or separate `analyzer.rs` (feature-gated)
- **Implement:**
  - `GraphifyAnalyzer` struct implementing `Analyzer` trait
  - Categories: `["architecture", "dependencies", "complexity", "knowledge-gaps"]`
  - `fn analyze()`: run extraction pipeline -> build graph -> generate findings from god_nodes, surprising_connections, gap_analysis
  - Register as 9th analyzer in `AnalyzerRegistry`
  - Findings: "high coupling" from god nodes, "unexpected dependency" from surprises, "missing test/doc" from gaps
- **Dependencies:** GRAPH-021, GRAPH-026, clawft-kernel Analyzer trait
- **Estimated complexity:** M
- **Acceptance criteria:**
  - `weft assess --analyzers graphify` produces findings
  - God nodes generate "high coupling" findings with edge counts
  - Assessment report includes "Knowledge Graph Analysis" section

#### GRAPH-030: DEMOCRITUS Impulse Integration
- **Module:** clawft-kernel modifications + bridge handler
- **Implement:**
  - New `ImpulseType` variants: `GraphifyFileChanged { path: PathBuf }`, `GraphifyExtractionComplete { result_key: String }`
  - DEMOCRITUS tick handler: queue files for extraction, ingest completed results
  - File watcher -> impulse generation
- **Dependencies:** GRAPH-026, GRAPH-029, clawft-kernel DEMOCRITUS
- **Estimated complexity:** M
- **Acceptance criteria:**
  - File change triggers impulse
  - Impulse triggers extraction
  - Extraction result triggers bridge ingestion

### Phase 4: Semantic Extraction + Forensic Domain

#### GRAPH-031: LLM Semantic Extraction
- **Module:** `src/extract/semantic.rs` (feature-gated: `semantic-extract`)
- **Implement:**
  - `SemanticExtractor` implementing `Extractor` trait
  - Uses `clawft-llm` for Claude API calls
  - Prompt template for entity/relationship extraction from text
  - Structured output parsing (JSON response)
  - Token budget management
  - Concurrency control via `tokio::sync::Semaphore` (default 5 concurrent calls)
  - Handles: documents, papers, config files, markdown
  - Produces: INFERRED and AMBIGUOUS confidence relationships
- **Dependencies:** GRAPH-001, clawft-llm
- **Estimated complexity:** L
- **Acceptance criteria:**
  - Extracts entities from a markdown document
  - Respects concurrency limit
  - Produces valid ExtractionResult with correct confidence levels

#### GRAPH-032: Vision Extraction
- **Module:** `src/extract/vision.rs` (feature-gated: `vision-extract`)
- **Implement:**
  - `VisionExtractor` implementing `Extractor` trait
  - Uses Claude vision API for image/diagram analysis
  - Extracts entities and relationships from architecture diagrams, screenshots
  - Image format support: PNG, JPG, WebP, SVG
- **Dependencies:** GRAPH-001, clawft-llm
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Extracts entities from an architecture diagram image
  - Returns structured ExtractionResult

#### GRAPH-033: Forensic Domain Types
- **Module:** `src/domain/forensic.rs` (feature-gated: `forensic-domain`)
- **Implement:**
  - Forensic entity types: Person, Event, Evidence, Location, Timeline, Document, Hypothesis, Organization, PhysicalObject, DigitalArtifact, FinancialRecord, Communication
  - Forensic edge types: WitnessedBy, FoundAt, Contradicts, Corroborates, AlibiedBy, Precedes, DocumentedIn, OwnedBy, ContactedBy, LocatedAt, SemanticallySimilarTo
  - Domain-specific validation rules
  - Prompt templates for forensic extraction
- **Dependencies:** GRAPH-001
- **Estimated complexity:** M
- **Acceptance criteria:**
  - All 12 entity types and 11 edge types defined
  - Serialization matches expected format

#### GRAPH-034: Gap Analysis Engine
- **Module:** `src/domain/forensic.rs`
- **Implement:**
  - `GapAnalysis` struct: unwitnessed_events, unlocated_evidence, unalibiied_persons, timeline_gaps, contradictions, weak_links, completeness_score
  - `fn gap_analysis(kg: &KnowledgeGraph) -> GapAnalysis`
  - Identifies: events with no WitnessedBy edge, evidence with no FoundAt, persons with no AlibiedBy for key events, temporal gaps, Contradicts edges, AMBIGUOUS edges on shortest paths
  - `TimelineGap` struct, `WeakLink` struct
- **Dependencies:** GRAPH-002, GRAPH-033
- **Estimated complexity:** L
- **Acceptance criteria:**
  - Detects unwitnessed events in a synthetic case
  - Timeline gaps identified for non-consecutive events
  - Contradictions listed

#### GRAPH-035: Coherence Scoring
- **Module:** `src/domain/forensic.rs`
- **Implement:**
  - `fn coherence_score(kg: &KnowledgeGraph) -> f64`
  - Weighted average of 6 factors:
    1. Entity coverage (0.20): ratio with >= 2 relationships
    2. Edge confidence (0.25): weighted average of confidence values
    3. Timeline continuity (0.20): fraction of events with temporal ordering
    4. Witness coverage (0.15): fraction of events with WitnessedBy
    5. Evidence linkage (0.10): fraction linked to both location and person
    6. Contradiction resolution (0.10): fraction of Contradicts with Hypothesis
  - Range: 0.0 (many gaps) to 1.0 (comprehensive)
- **Dependencies:** GRAPH-002, GRAPH-033
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Complete synthetic case scores > 0.8
  - Incomplete case scores < 0.4
  - Score is between 0.0 and 1.0

### Phase 5: CLI + Polish

#### GRAPH-036: CLI Commands
- **Module:** `crates/clawft-cli/src/commands/graphify.rs`
- **Implement:**
  - `weaver graphify ingest <PATH>... [--mode fast|deep] [--domain code|forensic] [--cache] [--concurrency N] [--output PATH] [--format json|html|graphml|cypher]`
  - `weaver graphify query <QUERY> [--semantic] [--type entity_type] [--limit N] [--depth N]`
  - `weaver graphify analyze [--god-nodes N] [--surprises N] [--questions N] [--gaps] [--diff PATH]`
  - `weaver graphify export <FORMAT> [--output PATH] [--filter EXPR]`
  - `weaver graphify watch <PATH> [--interval SECONDS] [--daemon]`
  - Clap derive-based argument parsing
  - Progress reporting (tracing events)
- **Dependencies:** GRAPH-021, clawft-cli
- **Estimated complexity:** M
- **Acceptance criteria:**
  - `weaver graphify ingest ./src --mode fast --format json` produces correct output
  - All subcommands have --help output
  - Progress messages show file count and timing

#### GRAPH-037: File Watcher
- **Module:** `src/watch.rs` (feature-gated)
- **Implement:**
  - Uses `notify` crate for file system events
  - Debounced event handling (default 3s)
  - Watched extensions: code + docs + images (matching Python's 30+ extensions)
  - Code-only changes: full local pipeline rebuild (no LLM)
  - Non-code changes: flag file + notification for manual `--update`
  - Filters: hidden directories, `graphify-out/`, `__pycache__/`
- **Dependencies:** GRAPH-021, `notify` crate
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Detects new/modified Python file, triggers re-extraction
  - Debounces rapid changes
  - Non-code changes write flag file

#### GRAPH-038: Git Hooks
- **Module:** `src/hooks.rs`
- **Implement:**
  - `fn install(repo_path: &Path) -> Result<String, GraphifyError>` -- installs post-commit and post-checkout hooks
  - `fn uninstall(repo_path: &Path) -> Result<String, GraphifyError>` -- removes graphify sections using markers
  - `fn status(repo_path: &Path) -> Result<String, GraphifyError>`
  - Idempotent: checks markers before installing
  - Appends to existing hooks
  - Marker-delimited sections for clean removal
- **Dependencies:** None
- **Estimated complexity:** S
- **Acceptance criteria:**
  - Install creates executable hook files
  - Uninstall removes only graphify sections
  - Status reports correct installation state

#### GRAPH-039: Additional Language Support (6 languages)
- **Modules:** `src/extract/lang/{java,c,cpp,ruby,csharp,kotlin,scala,php,lua,swift}.rs`
- **Implement:**
  - Java: class/interface, method/constructor, import (scoped_identifier walk)
  - C: function (declarator unwrapping), preproc_include
  - C++: class_specifier, function (declarator unwrapping), preproc_include
  - Ruby: class, method/singleton_method, call (method field)
  - C#: class/interface, method, using_directive, namespace walk
  - Kotlin: class/object, function, import_header
  - Scala: class/object, function, import_declaration
  - PHP: class, function/method, namespace_use_clause, dual call types
  - Lua: function, require() regex import
  - Swift: class/protocol, function/init/deinit/subscript, import, enum case, conformance
- **Dependencies:** GRAPH-006, respective tree-sitter-* features
- **Estimated complexity:** L (aggregate of 10 languages)
- **Acceptance criteria:**
  - Each language extracts classes, functions, imports from a reference file
  - Snapshot tests for each language

#### GRAPH-040: Obsidian Export
- **Module:** `src/export/obsidian.rs`
- **Implement:**
  - Per-node `.md` files with YAML frontmatter + wikilinks
  - Per-community overview notes with cohesion, members, cross-community links, bridge nodes
  - `.obsidian/graph.json` for community colors
  - Filename deduplication (numeric suffix)
  - Canvas export (`.canvas` JSON, grid layout)
- **Dependencies:** GRAPH-002, GRAPH-016
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Opens correctly in Obsidian
  - Wikilinks resolve between notes
  - Community notes include Dataview query blocks

#### GRAPH-041: Performance Benchmarks
- **Module:** `benches/extraction.rs`, `benches/graph_ops.rs`
- **Implement:**
  - Extraction throughput: 1K files/second target for AST
  - Graph operations: 50K node graph queries
  - Memory profiling assertions
  - Token reduction measurement (corpus tokens vs graph-query tokens)
  - Regression detection: fail if > 10% slower than baseline
- **Dependencies:** GRAPH-021
- **Estimated complexity:** M
- **Acceptance criteria:**
  - 10K files extracted in < 2 minutes
  - 50K entity graph in < 500 MB RAM
  - Benchmarks run in CI

#### GRAPH-042: URL Ingest
- **Module:** `src/ingest.rs`
- **Implement:**
  - `fn ingest(url: &str, target_dir: &Path, author: Option<&str>) -> Result<PathBuf, GraphifyError>`
  - URL type detection (tweet, arxiv, github, youtube, pdf, image, webpage)
  - Fetchers for each type with YAML frontmatter markdown output
  - `fn save_query_result(question, answer, memory_dir) -> Result<PathBuf, GraphifyError>`
  - SSRF protection via clawft-security
  - Filename deduplication
- **Dependencies:** reqwest, clawft-security
- **Estimated complexity:** M
- **Acceptance criteria:**
  - Fetches a webpage and saves as markdown with frontmatter
  - Query results saved in memory directory
  - SSRF protection blocks private IPs

---

## Task Dependency Graph

```
GRAPH-001 (types) ──────────────┬──────────────────────────────────────┐
  |                              |                                      |
GRAPH-002 (KnowledgeGraph) ─────+── GRAPH-012 (build) ──────┐          |
  |         |         |         |                             |          |
  |    GRAPH-003      |    GRAPH-014 (validation)             |          |
  |    (errors)       |                                       |          |
  |                   |                                       |          |
GRAPH-004 ──┐    GRAPH-005 ──── GRAPH-006 (LanguageConfig) ──+── GRAPH-013 (cache)
(detect)    |    (lang ID)        |       |       |          |
            |                     |       |       |          |
            |              GRAPH-007  GRAPH-008  GRAPH-009  GRAPH-010
            |              (Python)   (JS/TS)    (Rust)     (Go)
            |                 |                              |
            |              GRAPH-011 (cross-file) ──────────┘
            |                 |
            |    ┌────────────+────────────────────────────────┐
            |    |            |                                 |
         GRAPH-015   GRAPH-016 (cluster)                       |
         (JSON)         |         |                            |
                   GRAPH-017  GRAPH-018  GRAPH-019  GRAPH-020  |
                   (god)      (surprises) (questions) (diff)   |
                        |         |         |                  |
                   GRAPH-022  GRAPH-023  GRAPH-024  GRAPH-025  |
                   (HTML)     (GraphML)  (Cypher)   (Report)   |
                        |                                      |
                   GRAPH-021 (pipeline) ───────────────────────┘
                        |
            ┌───────────+───────────────┐
            |           |               |
       GRAPH-026   GRAPH-031       GRAPH-036
       (bridge)    (semantic)      (CLI)
         |   |          |              |
    GRAPH-027 GRAPH-028 GRAPH-032  GRAPH-037
    (HNSW)   (CrossRef) (vision)   (watcher)
         |                             |
    GRAPH-029 (analyzer)          GRAPH-038
         |                        (hooks)
    GRAPH-030 (DEMOCRITUS)
                                  GRAPH-039 (languages)
    GRAPH-033 (forensic types)    GRAPH-040 (Obsidian)
         |       |                GRAPH-041 (benchmarks)
    GRAPH-034 GRAPH-035           GRAPH-042 (ingest)
    (gaps)    (coherence)
```

### Critical Path

```
GRAPH-001 -> GRAPH-002 -> GRAPH-006 -> GRAPH-007 -> GRAPH-012 -> GRAPH-016 -> GRAPH-021 -> GRAPH-026 -> GRAPH-029
```

This is the minimum sequence to get from zero to "assessment analyzer works." Each task on this path blocks all downstream work.

---

## Developer Assignment Guide

### Workstream A: Core Types + AST Extraction (can start immediately)

**Tasks:** GRAPH-001, GRAPH-002, GRAPH-003, GRAPH-005, GRAPH-006, GRAPH-007, GRAPH-008, GRAPH-009, GRAPH-010, GRAPH-011

**Developer profile:** Strong Rust, familiar with tree-sitter, comfortable with AST traversal.

**Week 1:** GRAPH-001, GRAPH-002, GRAPH-003, GRAPH-005 (foundation types)
**Week 2:** GRAPH-006, GRAPH-007 (generic framework + Python)
**Week 3:** GRAPH-008, GRAPH-009, GRAPH-010, GRAPH-011 (JS/TS, Rust, Go, cross-file)

**Output:** `extract()` function that takes a directory of Python/JS/TS/Rust/Go files and produces an `ExtractionResult` with correct entities, relationships, and call graph edges.

### Workstream B: Detection + Build + Cache (can start after GRAPH-001)

**Tasks:** GRAPH-004, GRAPH-012, GRAPH-013, GRAPH-014, GRAPH-015

**Developer profile:** Systems Rust, file I/O, serialization.

**Week 1:** GRAPH-004 (file detection), GRAPH-014 (validation)
**Week 2:** GRAPH-012 (build), GRAPH-013 (cache), GRAPH-015 (JSON export)

**Output:** File discovery pipeline, graph assembly with deduplication, content-hash cache, JSON export.

### Workstream C: Analysis + Visualization (starts after GRAPH-002 available)

**Tasks:** GRAPH-016, GRAPH-017, GRAPH-018, GRAPH-019, GRAPH-020, GRAPH-022, GRAPH-023, GRAPH-024, GRAPH-025

**Developer profile:** Graph algorithms, data visualization, HTML/CSS/JS for the vis.js export.

**Week 1:** GRAPH-016 (Leiden clustering), GRAPH-017 (god nodes), GRAPH-020 (diff)
**Week 2:** GRAPH-018 (surprises), GRAPH-019 (questions), GRAPH-025 (report)
**Week 3:** GRAPH-022 (HTML export), GRAPH-023 (GraphML), GRAPH-024 (Cypher)

**Output:** Full analysis pipeline producing god nodes, surprising connections, questions, graph diff, and exports in JSON/HTML/GraphML/Cypher.

### Workstream D: WeftOS Bridge (starts after Phase 1 core types)

**Tasks:** GRAPH-026, GRAPH-027, GRAPH-028, GRAPH-029, GRAPH-030

**Developer profile:** Deep WeftOS/clawft-kernel knowledge, CausalGraph, HNSW, CrossRefStore, DEMOCRITUS.

**Week 1:** GRAPH-026 (bridge skeleton), GRAPH-027 (HNSW)
**Week 2:** GRAPH-028 (CrossRef), GRAPH-029 (analyzer registration)
**Week 3:** GRAPH-030 (DEMOCRITUS impulses)

**Output:** `weft assess --analyzers graphify` works. File changes trigger incremental extraction via DEMOCRITUS. Semantic search works.

### Workstream E: Semantic + Forensic (starts after bridge works)

**Tasks:** GRAPH-031, GRAPH-032, GRAPH-033, GRAPH-034, GRAPH-035

**Developer profile:** LLM integration, prompt engineering, domain modeling.

**Week 1:** GRAPH-031 (semantic extraction), GRAPH-033 (forensic types)
**Week 2:** GRAPH-032 (vision), GRAPH-034 (gap analysis)
**Week 3:** GRAPH-035 (coherence scoring)

**Output:** Full forensic pipeline: document extraction, gap analysis, coherence scoring.

### Workstream F: CLI + Polish (starts after pipeline works)

**Tasks:** GRAPH-036, GRAPH-037, GRAPH-038, GRAPH-039, GRAPH-040, GRAPH-041, GRAPH-042

**Developer profile:** CLI design, system integration, performance tuning.

**Week 1:** GRAPH-036 (CLI commands), GRAPH-038 (git hooks)
**Week 2:** GRAPH-037 (file watcher), GRAPH-039 (additional languages -- parallel with Workstream A if needed)
**Week 3:** GRAPH-040 (Obsidian), GRAPH-041 (benchmarks), GRAPH-042 (URL ingest)

**Output:** Complete CLI with all subcommands, file watcher, git hooks, 15 language support, benchmarks.

### Parallelism Summary

```
Week 1:  [A: types]      [B: detect]     [C: --]         [D: --]
Week 2:  [A: framework]  [B: build]      [C: cluster]    [D: --]
Week 3:  [A: languages]  [B: cache]      [C: analysis]   [D: bridge]
Week 4:  [A: cross-file] [--]            [C: exports]    [D: HNSW/CrossRef]
Week 5:  [--]            [--]            [C: HTML]       [D: analyzer/DEMOCRITUS]
Week 6:  [E: semantic]   [F: CLI]        [--]            [--]
Week 7:  [E: forensic]   [F: watcher]    [F: languages]  [--]
Week 8:  [E: coherence]  [F: polish]     [F: benchmarks] [--]
```

Maximum parallelism: 4 developers (Weeks 3-5). Minimum: 2 developers (Weeks 1-2).

---

## Success Criteria

### Functional

1. All 126 Python tests have Rust equivalents passing
2. Python extraction output matches Rust output for identical input files (snapshot tests)
3. Full round-trip works: source files -> extraction -> KnowledgeGraph -> CausalGraph bridge -> gap analysis -> HTML report
4. `weft assess` includes graphify as the 9th analyzer
5. Incremental extraction correctly handles file add/modify/delete
6. All 15 languages extract classes, functions, imports, and call graphs
7. Cross-file import resolution produces correct INFERRED edges
8. Community detection assigns every node to a community
9. Surprise scoring ranks AMBIGUOUS cross-file-type edges highest
10. JSON export schema matches Python output exactly (backward compatible)

### Performance

1. 10K code files extracted (AST only) in < 2 minutes on 8-core machine
2. 50K entity graph fits in < 500 MB RAM
3. HNSW search returns in < 10 ms per query
4. Incremental re-scan of 100 changed files out of 10K completes in < 30 seconds
5. Cache lookup: < 1 ms per file (BLAKE3 hash + DashMap lookup)
6. HTML export for 5K node graph generates in < 5 seconds

### Quality

1. Zero `unsafe` blocks in the crate (all unsafe is in tree-sitter FFI, handled by tree-sitter crate)
2. All public APIs documented with rustdoc
3. No clippy warnings at `#![deny(clippy::all)]`
4. Feature-gated optional dependencies: default build has minimal deps
5. All error paths return structured `GraphifyError` (no panics in library code)

---

## Appendix: Extension -> Language -> Tree-Sitter Crate Mapping

| Extension(s) | LanguageId | tree-sitter crate | Feature flag |
|-------------|-----------|-------------------|-------------|
| `.py` | Python | `tree-sitter-python` | `lang-python` |
| `.js` | JavaScript | `tree-sitter-javascript` | `lang-javascript` |
| `.ts`, `.tsx` | TypeScript | `tree-sitter-typescript` | `lang-typescript` |
| `.rs` | Rust | `tree-sitter-rust` | `lang-rust` |
| `.go` | Go | `tree-sitter-go` | `lang-go` |
| `.java` | Java | `tree-sitter-java` | `lang-java` |
| `.c`, `.h` | C | `tree-sitter-c` | `lang-c` |
| `.cpp`, `.cc`, `.cxx`, `.hpp` | Cpp | `tree-sitter-cpp` | `lang-cpp` |
| `.rb` | Ruby | `tree-sitter-ruby` | `lang-ruby` |
| `.cs` | CSharp | `tree-sitter-c-sharp` | `lang-csharp` |
| `.kt`, `.kts` | Kotlin | (not in default, add to Cargo.toml) | (not yet) |
| `.scala` | Scala | (not in default, add to Cargo.toml) | (not yet) |
| `.php` | Php | (not in default, add to Cargo.toml) | (not yet) |
| `.lua`, `.toc` | Lua | (not in default, add to Cargo.toml) | (not yet) |
| `.swift` | Swift | (not in default, add to Cargo.toml) | (not yet) |

Note: Kotlin, Scala, PHP, Lua, and Swift tree-sitter grammars may not have stable Rust crate versions on crates.io. If not available, these languages can be added as P2 items or handled via tree-sitter's dynamic grammar loading.
