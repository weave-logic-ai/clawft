# Graphify-RS Architecture: Knowledge Graph Builder for WeftOS

**ADR**: ADR-049 (pending)
**Status**: Design
**Author**: System Architecture
**Date**: 2026-04-04
**Depends on**: ADR-023 (Analyzer Registry), ADR-021 (CLI Kernel), ECC subsystem (CausalGraph, HNSW, CrossRefStore, DEMOCRITUS)

---

## 1. Crate Structure

### Location

`crates/clawft-graphify/` -- follows the existing `clawft-*` naming convention. The crate name `clawft-graphify` keeps it clearly within the WeftOS workspace and avoids confusion with the upstream Python package.

### Workspace Registration

```toml
# Cargo.toml (workspace root) -- add to members list
"crates/clawft-graphify",

# workspace.dependencies -- add
clawft-graphify = { version = "0.5.0", path = "crates/clawft-graphify", default-features = false }
```

### Module Layout

```
crates/clawft-graphify/
  Cargo.toml
  src/
    lib.rs                  # Public API re-exports, feature gates
    model.rs                # Core data model: Entity, Relationship, ExtractionResult, KnowledgeGraph
    entity.rs               # EntityType taxonomy (code + forensic), EntityId (BLAKE3)
    relationship.rs         # RelationType taxonomy, Confidence enum, edge weight mapping
    extract/
      mod.rs                # Extractor trait, ExtractionPipeline
      ast.rs                # tree-sitter structural extraction (rayon-parallel per file)
      semantic.rs           # LLM-based semantic extraction (tokio async, batched Claude calls)
      vision.rs             # Image/diagram extraction via Claude vision (feature-gated)
      detect.rs             # Language/document type detection, file collector
    build.rs                # Graph assembly: merge extractions, deduplicate nodes
    cluster.rs              # Community detection (Leiden via petgraph, no Python dep)
    analyze.rs              # God nodes, surprising connections, gap analysis, suggested questions
    export/
      mod.rs                # ExportFormat enum, dispatch
      json.rs               # JSON serialization (serde)
      graphml.rs            # GraphML export
      cypher.rs             # Neo4j Cypher export
      html.rs               # Interactive HTML visualization
    bridge.rs               # CausalGraph/CrossRefStore/HNSW integration layer
    pipeline.rs             # Full pipeline orchestrator: detect -> extract -> build -> cluster -> analyze -> export
    cache.rs                # BLAKE3 content-hash cache for incremental processing
    domain/
      mod.rs                # Domain trait, domain registry
      code.rs               # Code analysis domain (entity types, edge types, analyzer integration)
      forensic.rs           # Forensic analysis domain (entity types, edge types, gap analysis)
```

### Feature Flags

```toml
[features]
default = ["code-domain", "ast-extract"]
ast-extract = ["dep:tree-sitter"]          # tree-sitter AST extraction
semantic-extract = []                       # LLM-based extraction (requires clawft-llm at runtime)
vision-extract = []                         # Image/diagram extraction via Claude vision API
code-domain = []                            # Code analysis entity/edge types
forensic-domain = []                        # Forensic/document analysis entity/edge types
html-export = []                            # Interactive HTML visualization export
neo4j-export = []                           # Neo4j Cypher export
kernel-bridge = ["dep:clawft-kernel"]       # Integration with CausalGraph, HNSW, CrossRefStore
full = ["ast-extract", "semantic-extract", "vision-extract", "code-domain",
        "forensic-domain", "html-export", "neo4j-export", "kernel-bridge"]

# Language support (each pulls in the corresponding tree-sitter grammar)
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
```

### Dependencies

```toml
[dependencies]
# Workspace deps
serde = { workspace = true }
serde_json = { workspace = true }
blake3 = { workspace = true }
dashmap = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
rayon = "1.10"
petgraph = "0.7"

# Optional
tree-sitter = { version = "0.24", optional = true }
clawft-kernel = { workspace = true, optional = true, features = ["ecc"] }
clawft-llm = { workspace = true, optional = true }
```

### Standalone vs Kernel-Bridged

The crate works in two modes:

1. **Standalone** (default): builds a `KnowledgeGraph` (petgraph-backed) that can be queried and exported independently. No clawft-kernel dependency. Useful for CLI-only workflows and testing.

2. **Kernel-bridged** (`kernel-bridge` feature): provides `GraphifyBridge` that maps `KnowledgeGraph` into `CausalGraph`, indexes entities into `HnswService`, and registers cross-refs in `CrossRefStore`. Used by the DEMOCRITUS loop and `weft assess`.

---

## 2. Data Model

### 2.1 Entity (maps to Graphify node)

```rust
// model.rs

/// Unique identifier for a knowledge graph entity.
/// BLAKE3 hash of (domain_tag, entity_type, canonical_name, source_file).
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub [u8; 32]);

impl EntityId {
    pub fn new(domain: &DomainTag, entity_type: &EntityType, name: &str, source: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[domain.as_u8()]);
        hasher.update(entity_type.discriminant().as_bytes());
        hasher.update(name.as_bytes());
        hasher.update(source.as_bytes());
        Self(*hasher.finalize().as_bytes())
    }

    /// Convert to UniversalNodeId for CrossRefStore integration.
    pub fn to_universal_node_id(&self) -> crossref::UniversalNodeId {
        crossref::UniversalNodeId::from_bytes(self.0)
    }
}

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub entity_type: EntityType,
    pub label: String,
    pub source_file: Option<String>,
    pub source_location: Option<String>,
    pub file_type: FileType,
    pub metadata: serde_json::Value,
}

/// Classification of the source material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Code,
    Document,
    Paper,
    Image,
    Config,
    Unknown,
}
```

### 2.2 EntityType Taxonomy

```rust
// entity.rs

/// Domain tag for entity ID generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainTag {
    Code = 0x20,
    Forensic = 0x21,
    Custom(u8),
}

/// Unified entity type taxonomy.
/// Discriminant strings are used in BLAKE3 hashing for stable IDs.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    // --- Code domain (0x20-0x2F) ---
    Module,
    Class,
    Function,
    Import,
    Config,
    Service,
    Endpoint,
    Interface,
    Struct,
    Enum,
    Constant,
    Package,

    // --- Forensic domain (0x30-0x3F) ---
    Person,
    Event,
    Evidence,
    Location,
    Timeline,
    Document,
    Hypothesis,
    Organization,
    PhysicalObject,
    DigitalArtifact,
    FinancialRecord,
    Communication,

    // --- Shared ---
    File,
    Concept,
    Custom(String),
}

impl EntityType {
    /// Stable string discriminant for BLAKE3 ID hashing.
    pub fn discriminant(&self) -> &str {
        match self {
            Self::Module => "module",
            Self::Class => "class",
            Self::Function => "function",
            Self::Import => "import",
            Self::Config => "config",
            Self::Service => "service",
            Self::Endpoint => "endpoint",
            Self::Interface => "interface",
            Self::Struct => "struct_",
            Self::Enum => "enum_",
            Self::Constant => "constant",
            Self::Package => "package",
            Self::Person => "person",
            Self::Event => "event",
            Self::Evidence => "evidence",
            Self::Location => "location",
            Self::Timeline => "timeline",
            Self::Document => "document",
            Self::Hypothesis => "hypothesis",
            Self::Organization => "organization",
            Self::PhysicalObject => "physical_object",
            Self::DigitalArtifact => "digital_artifact",
            Self::FinancialRecord => "financial_record",
            Self::Communication => "communication",
            Self::File => "file",
            Self::Concept => "concept",
            Self::Custom(s) => s.as_str(),
        }
    }
}
```

### 2.3 Relationship (maps to Graphify edge)

```rust
// relationship.rs

/// Confidence level for an extracted relationship.
/// Maps directly from Graphify's three-tier confidence model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Deterministically extracted from AST/structure.
    Extracted,
    /// Inferred by LLM or heuristic reasoning.
    Inferred,
    /// Ambiguous -- multiple interpretations possible.
    Ambiguous,
}

impl Confidence {
    /// Map to edge weight for CausalGraph integration.
    ///   EXTRACTED -> 1.0
    ///   INFERRED  -> 0.7
    ///   AMBIGUOUS -> 0.4
    pub fn to_weight(&self) -> f32 {
        match self {
            Self::Extracted => 1.0,
            Self::Inferred => 0.7,
            Self::Ambiguous => 0.4,
        }
    }
}

/// Relationship type taxonomy.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    // --- Code domain ---
    Calls,
    Imports,
    ImportsFrom,
    DependsOn,
    Contains,
    Implements,
    Configures,
    Extends,
    MethodOf,
    Instantiates,

    // --- Forensic domain ---
    WitnessedBy,
    FoundAt,
    Contradicts,
    Corroborates,
    AlibiedBy,
    Precedes,
    DocumentedIn,
    OwnedBy,
    ContactedBy,
    LocatedAt,
    SemanticallySimilarTo,

    // --- Shared ---
    RelatedTo,
    CaseOf,
    Custom(String),
}

/// A directed, typed relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub source: EntityId,
    pub target: EntityId,
    pub relation_type: RelationType,
    pub confidence: Confidence,
    pub weight: f32,
    pub source_file: Option<String>,
    pub source_location: Option<String>,
    pub metadata: serde_json::Value,
}
```

### 2.4 Mapping to CausalGraph

| RelationType | CausalEdgeType | Notes |
|---|---|---|
| Calls, Imports, ImportsFrom, DependsOn | `Causes` | Causal dependency chain |
| Contains, MethodOf | `Enables` | Structural containment |
| Implements, Extends | `Enables` | Type hierarchy |
| Configures | `Enables` | Configuration dependency |
| Contradicts | `Contradicts` | Direct mapping |
| Corroborates, WitnessedBy | `EvidenceFor` | Supporting evidence |
| Precedes | `Follows` | Temporal ordering |
| SemanticallySimilarTo, RelatedTo | `Correlates` | Semantic similarity |
| AlibiedBy | `Inhibits` | Negation of suspicion |
| FoundAt, LocatedAt, DocumentedIn | `Correlates` | Spatial/documentary co-occurrence |

New CausalEdgeType variants are NOT added. The existing 8 types are sufficient. Domain-specific semantics are preserved in the `metadata` field on CausalEdge and in CrossRef entries that carry the original `RelationType` as a `CrossRefType::Custom(0x20+)`.

### 2.5 KnowledgeGraph

```rust
// model.rs

/// In-memory knowledge graph backed by petgraph.
/// This is the standalone representation before bridging to CausalGraph.
pub struct KnowledgeGraph {
    graph: petgraph::Graph<Entity, Relationship, petgraph::Directed>,
    entity_index: HashMap<EntityId, petgraph::graph::NodeIndex>,
    /// Community assignments after clustering.
    communities: Option<HashMap<usize, Vec<EntityId>>>,
    /// Community labels (auto-generated or user-provided).
    community_labels: Option<HashMap<usize, String>>,
    /// Extraction statistics.
    stats: ExtractionStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub files_processed: usize,
    pub files_skipped: usize,
    pub entities_extracted: usize,
    pub relationships_extracted: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_hits: usize,
    pub extraction_duration_ms: u64,
}

impl KnowledgeGraph {
    pub fn new() -> Self;
    pub fn add_entity(&mut self, entity: Entity) -> petgraph::graph::NodeIndex;
    pub fn add_relationship(&mut self, rel: Relationship) -> Option<petgraph::graph::EdgeIndex>;
    pub fn entity(&self, id: &EntityId) -> Option<&Entity>;
    pub fn entity_count(&self) -> usize;
    pub fn relationship_count(&self) -> usize;
    pub fn neighbors(&self, id: &EntityId) -> Vec<&Entity>;
    pub fn subgraph(&self, ids: &[EntityId]) -> KnowledgeGraph;

    // Analysis (delegated to analyze module)
    pub fn god_nodes(&self, top_n: usize) -> Vec<GodNode>;
    pub fn surprising_connections(&self, top_n: usize) -> Vec<SurprisingConnection>;
    pub fn suggest_questions(&self, top_n: usize) -> Vec<SuggestedQuestion>;
    pub fn diff(&self, other: &KnowledgeGraph) -> GraphDiff;
}
```

---

## 3. Integration Points

### 3.1 CausalGraph Integration (`bridge.rs`)

```rust
// bridge.rs (compiled only with `kernel-bridge` feature)

use clawft_kernel::causal::{CausalGraph, CausalEdgeType, NodeId as CausalNodeId};
use clawft_kernel::crossref::{CrossRefStore, CrossRef, CrossRefType, StructureTag, UniversalNodeId};
use clawft_kernel::hnsw_service::HnswService;

/// Bridges a KnowledgeGraph into the ECC subsystems.
pub struct GraphifyBridge {
    causal_graph: Arc<CausalGraph>,
    hnsw: Arc<HnswService>,
    crossref_store: Arc<CrossRefStore>,
    /// Maps EntityId -> CausalNodeId for reverse lookup.
    entity_to_causal: DashMap<EntityId, CausalNodeId>,
}

impl GraphifyBridge {
    pub fn new(
        causal_graph: Arc<CausalGraph>,
        hnsw: Arc<HnswService>,
        crossref_store: Arc<CrossRefStore>,
    ) -> Self;

    /// Ingest an entire KnowledgeGraph into the ECC subsystems.
    /// Returns the number of (nodes, edges, crossrefs) created.
    pub async fn ingest(
        &self,
        kg: &KnowledgeGraph,
        embedding_provider: &dyn EmbeddingProvider,
        hlc_timestamp: u64,
        chain_seq: u64,
    ) -> Result<IngestResult, GraphifyError>;

    /// Ingest a single entity: add to CausalGraph, embed into HNSW, register CrossRef.
    pub async fn ingest_entity(
        &self,
        entity: &Entity,
        embedding_provider: &dyn EmbeddingProvider,
        hlc_timestamp: u64,
        chain_seq: u64,
    ) -> Result<CausalNodeId, GraphifyError>;

    /// Ingest a single relationship: add CausalEdge + CrossRef.
    pub fn ingest_relationship(
        &self,
        rel: &Relationship,
        hlc_timestamp: u64,
        chain_seq: u64,
    ) -> Result<(), GraphifyError>;

    /// Look up which CausalNodeId corresponds to an EntityId.
    pub fn causal_node_for(&self, entity_id: &EntityId) -> Option<CausalNodeId>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub nodes_created: usize,
    pub edges_created: usize,
    pub crossrefs_created: usize,
    pub embeddings_indexed: usize,
    pub duration_ms: u64,
}
```

### 3.2 HNSW Integration

When `ingest_entity` is called:

1. Serialize entity label + metadata to a text representation.
2. Call `embedding_provider.embed(&text).await` to get a `Vec<f32>`.
3. Call `hnsw.insert(entity_id_hex, embedding, metadata_json)`.
4. The HNSW metadata JSON includes `{"entity_type": "...", "domain": "...", "source_file": "...", "label": "..."}` for post-search filtering.

This enables `weaver graphify query --semantic "authentication flow"` to find relevant entities via cosine similarity.

### 3.3 CrossRefStore Integration

For every entity ingested, a CrossRef is created linking the graphify entity to the ExoChain provenance event:

```rust
CrossRef {
    source: entity_id.to_universal_node_id(),
    source_structure: StructureTag::Custom(0x20),  // 0x20 = Graphify KnowledgeGraph
    target: exochain_event_universal_id,
    target_structure: StructureTag::ExoChain,
    ref_type: CrossRefType::EvidenceFor,
    created_at: hlc_timestamp,
    chain_seq,
}
```

For inter-entity relationships, a CrossRef captures the original `RelationType`:

```rust
CrossRef {
    source: source_entity_id.to_universal_node_id(),
    source_structure: StructureTag::Custom(0x20),
    target: target_entity_id.to_universal_node_id(),
    target_structure: StructureTag::Custom(0x20),
    ref_type: CrossRefType::Custom(relation_type.as_crossref_discriminant()),
    created_at: hlc_timestamp,
    chain_seq,
}
```

The `Custom(0x20)` StructureTag is registered as the Graphify namespace. Relation-type discriminants use the `0x20..0x3F` range.

### 3.4 DEMOCRITUS Loop Integration

The DEMOCRITUS loop processes Impulses. A new `ImpulseType` variant enables graphify:

```rust
// In clawft-kernel impulse.rs (requires adding variant):
pub enum ImpulseType {
    // ... existing variants ...
    /// A document or source file was added/changed -- trigger graphify extraction.
    GraphifyFileChanged { path: PathBuf },
    /// Batch graphify extraction completed -- results ready for ingestion.
    GraphifyExtractionComplete { result_key: String },
}
```

The DEMOCRITUS tick processes these impulses:

1. `GraphifyFileChanged`: queues the file for extraction in the next batch.
2. `GraphifyExtractionComplete`: calls `GraphifyBridge::ingest()` with the extraction result.

This allows continuous, incremental knowledge graph updates as files change (via `notify` file watcher).

### 3.5 Assessment Analyzer Integration

A new analyzer is registered in the `AnalyzerRegistry`:

```rust
// In clawft-kernel assessment/analyzers/ -- new file: graphify.rs

pub struct GraphifyAnalyzer {
    bridge: Arc<GraphifyBridge>,
}

impl Analyzer for GraphifyAnalyzer {
    fn id(&self) -> &str { "graphify" }
    fn name(&self) -> &str { "Knowledge Graph" }
    fn categories(&self) -> &[&str] {
        &["architecture", "dependencies", "complexity", "knowledge-gaps"]
    }
    fn analyze(&self, project: &Path, files: &[PathBuf], context: &AnalysisContext) -> Vec<Finding> {
        // 1. Run extraction pipeline on scoped files
        // 2. Build KnowledgeGraph
        // 3. Run god_nodes -> generate "high coupling" findings
        // 4. Run surprising_connections -> generate "unexpected dependency" findings
        // 5. Run gap_analysis -> generate "missing test/doc" findings
        // 6. Ingest into CausalGraph via bridge
    }
}
```

This makes graphify extraction automatic during `weft assess` runs. The 9th analyzer.

---

## 4. Pipeline Architecture

### 4.1 Full Pipeline

```
DETECT -> EXTRACT -> BUILD -> CLUSTER -> ANALYZE -> EXPORT
  |          |          |        |          |          |
  |    [parallel per    |   [petgraph    [god nodes   [JSON/HTML/
  |     file, rayon     |    Leiden]     surprises     GraphML/
  |     for AST,        |                questions     Cypher]
  |     tokio for       |                gap analysis]
  |     semantic]       |
  |                     |
  v                     v
collect_files()    merge extractions,
detect language    deduplicate by EntityId
```

### 4.2 Pipeline Orchestrator

```rust
// pipeline.rs

pub struct PipelineConfig {
    /// Which extraction modes to run.
    pub extractors: Vec<ExtractorKind>,
    /// Maximum concurrent AST extractions (rayon thread count).
    pub ast_parallelism: usize,
    /// Maximum concurrent LLM API calls.
    pub semantic_concurrency: usize,
    /// Whether to run clustering.
    pub cluster: bool,
    /// Whether to run analysis.
    pub analyze: bool,
    /// Export formats to produce.
    pub exports: Vec<ExportFormat>,
    /// Cache directory for incremental processing.
    pub cache_dir: Option<PathBuf>,
    /// Domain configuration.
    pub domain: DomainTag,
}

#[derive(Debug, Clone)]
pub enum ExtractorKind {
    Ast,
    Semantic,
    Vision,
}

pub struct Pipeline {
    config: PipelineConfig,
    cache: Option<ContentCache>,
}

impl Pipeline {
    pub fn new(config: PipelineConfig) -> Self;

    /// Run the full pipeline: detect -> extract -> build -> cluster -> analyze.
    /// Returns the complete KnowledgeGraph with analysis results.
    pub async fn run(&self, paths: &[PathBuf]) -> Result<PipelineResult, GraphifyError>;

    /// Run extraction only (no clustering/analysis). Used for incremental updates.
    pub async fn extract_only(&self, paths: &[PathBuf]) -> Result<Vec<ExtractionResult>, GraphifyError>;
}

pub struct PipelineResult {
    pub graph: KnowledgeGraph,
    pub analysis: Option<AnalysisResult>,
    pub stats: ExtractionStats,
}

pub struct AnalysisResult {
    pub god_nodes: Vec<GodNode>,
    pub surprising_connections: Vec<SurprisingConnection>,
    pub questions: Vec<SuggestedQuestion>,
    pub communities: HashMap<usize, Vec<EntityId>>,
    pub community_labels: HashMap<usize, String>,
    pub cohesion_scores: HashMap<usize, f64>,
}
```

### 4.3 Parallelism Model

```
                    +------------------+
                    |  Pipeline::run() |
                    +--------+---------+
                             |
              +--------------+--------------+
              |                             |
    +----+----+----+              +---------+---------+
    | rayon par_iter|              | tokio::spawn batch|
    | AST per file  |              | semantic per file |
    | (CPU-bound)   |              | (IO-bound, Claude)|
    +----+----+----+              +---------+---------+
              |                             |
              +--------------+--------------+
                             |
                    +--------v---------+
                    |  build::merge()  |
                    |  (single thread) |
                    +--------+---------+
                             |
                    +--------v---------+
                    | cluster::leiden()|
                    | (CPU, petgraph)  |
                    +--------+---------+
                             |
                    +--------v---------+
                    | analyze::all()   |
                    | (single thread)  |
                    +------------------+
```

- **AST extraction**: `rayon::par_iter` over files. Each file gets its own tree-sitter parser. No shared state. Results collected into `Vec<ExtractionResult>`.
- **Semantic extraction**: `tokio::spawn` per file (or per batch of files, depending on token budget). Concurrent Claude API calls bounded by `semantic_concurrency` (default 5) using a `tokio::sync::Semaphore`.
- **Build phase**: single-threaded merge. Entities deduplicated by `EntityId`. Last-write-wins for metadata (semantic overwrites AST, matching Python Graphify behavior).
- **Cluster phase**: Leiden community detection on petgraph. CPU-bound, single-threaded (petgraph's Leiden is not parallelized).
- **Analyze phase**: god nodes, surprising connections, questions. All read-only graph traversals.

### 4.4 Caching Strategy

```rust
// cache.rs

/// Content-addressed cache using BLAKE3 file hashes.
pub struct ContentCache {
    cache_dir: PathBuf,
    /// Maps BLAKE3(file_content) -> cached ExtractionResult.
    index: DashMap<[u8; 32], CacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    content_hash: [u8; 32],
    extraction: ExtractionResult,
    extracted_at: u64,
    extractor_version: u32,
}

impl ContentCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self, std::io::Error>;
    /// Check if a file has a cached extraction result.
    /// Returns None if file content has changed (BLAKE3 mismatch).
    pub fn get(&self, path: &Path) -> Option<ExtractionResult>;
    /// Store extraction result keyed by file content hash.
    pub fn put(&self, path: &Path, result: &ExtractionResult) -> Result<(), std::io::Error>;
    /// Remove stale entries for files that no longer exist.
    pub fn gc(&self, live_paths: &[PathBuf]) -> usize;
}
```

Cache key: `BLAKE3(file_content_bytes)`. This means:
- Renamed files with identical content reuse the cache.
- Files with whitespace-only changes are NOT cached (content hash changes). This is intentional -- AST may differ.
- Cache is invalidated when the extractor version changes (stored in `CacheEntry`).

Cache location: `.weftos/graphify-cache/` inside the project root.

### 4.5 Error Handling

```rust
// lib.rs

#[derive(Debug, thiserror::Error)]
pub enum GraphifyError {
    #[error("extraction failed for {path}: {reason}")]
    ExtractionFailed { path: PathBuf, reason: String },
    #[error("tree-sitter grammar not available for {language}")]
    GrammarNotAvailable { language: String },
    #[error("LLM API error: {0}")]
    LlmError(String),
    #[error("cache I/O error: {0}")]
    CacheError(#[from] std::io::Error),
    #[error("graph build error: {0}")]
    BuildError(String),
    #[error("kernel bridge error: {0}")]
    BridgeError(String),
}
```

Partial results are always returned. If extraction fails for 3 of 100 files, the pipeline proceeds with 97 files and reports the 3 failures in `ExtractionStats::files_skipped` and in structured error metadata.

---

## 5. CLI Integration (weaver)

### 5.1 New Weaver Commands

Added to `crates/clawft-cli/src/commands/` as a new `graphify.rs` subcommand module.

```
weaver graphify ingest <PATH>...
    --mode <fast|deep>           # fast = AST only, deep = AST + semantic
    --domain <code|forensic>     # entity type domain
    --cache / --no-cache         # incremental processing (default: --cache)
    --concurrency <N>            # max concurrent LLM calls (default: 5)
    --output <PATH>              # export result to file
    --format <json|html|graphml|cypher>  # export format (default: json)

weaver graphify query <QUERY>
    --semantic                   # use HNSW vector search
    --type <entity_type>         # filter by entity type
    --limit <N>                  # max results (default: 20)
    --depth <N>                  # neighborhood traversal depth (default: 1)

weaver graphify analyze
    --god-nodes <N>              # show top N hub entities (default: 10)
    --surprises <N>              # show top N surprising connections (default: 5)
    --questions <N>              # show top N suggested questions (default: 7)
    --gaps                       # run gap analysis (forensic domain)
    --diff <PATH>                # compare against previous graph snapshot

weaver graphify export <FORMAT>
    --output <PATH>              # output file path
    --filter <EXPR>              # entity type / community filter

weaver graphify watch <PATH>
    --interval <SECONDS>         # re-scan interval (default: 30)
    --daemon                     # run as background daemon
```

### 5.2 Integration with `weft assess`

When `clawft-graphify` is compiled with `kernel-bridge`:

```
weft assess --analyzers graphify    # run only the graphify analyzer
weft assess                          # runs all 9 analyzers including graphify
weft assess --full                   # runs graphify in deep mode (AST + semantic)
```

The graphify analyzer appears in the assessment report as:

```
## Knowledge Graph Analysis

Entities: 847 (412 code, 435 inferred)
Relationships: 1,293
Communities: 12

### Hub Entities (God Nodes)
1. AuthService (47 connections) -- consider decomposing
2. DatabasePool (31 connections) -- shared resource bottleneck

### Unexpected Dependencies
1. PaymentController -> LoggingMiddleware (INFERRED, crosses auth/logging boundary)

### Knowledge Gaps
1. No tests found for `CacheManager` (12 downstream dependents)
2. `UserService.delete()` has no documentation
```

### 5.3 Interactive Mode

`weaver graphify ingest --interactive` opens a REPL:

```
graphify> :entities Person
  [P-001] John Doe (3 connections)
  [P-002] Jane Smith (7 connections)

graphify> :neighbors P-002
  -> witnessed_by -> [E-003] Crime Scene Photo (EXTRACTED)
  -> alibied_by -> [P-001] John Doe (INFERRED)
  <- corroborates -> [D-005] Phone Records (EXTRACTED)

graphify> :gaps
  Missing: No witness statements for Event[E-007]
  Missing: Location for Evidence[EV-012]
  Low confidence: 4 AMBIGUOUS relationships need verification
```

---

## 6. Two Application Domains

### 6.1 Code Analysis Domain (`domain/code.rs`)

#### Entity Types

| EntityType | Description | AST Source |
|---|---|---|
| `Module` | Top-level file/module | File node |
| `Class` | Class/struct/interface declaration | `class_definition`, `class_declaration` etc. |
| `Function` | Function/method definition | `function_definition`, `method_definition` etc. |
| `Import` | External dependency reference | `import_statement`, `use_declaration` etc. |
| `Config` | Configuration file/entry | `.toml`, `.yaml`, `.json` config files |
| `Service` | Service/server/daemon definition | Inferred from class names, annotations |
| `Endpoint` | API endpoint (HTTP route, RPC) | Inferred from decorators, route macros |
| `Interface` | Trait/interface/protocol | `trait_item`, `interface_declaration` |
| `Struct` | Data structure (no behavior) | `struct_item`, type aliases |
| `Enum` | Enumeration type | `enum_item`, `enum_declaration` |

#### Edge Types

| RelationType | Extraction Method | Weight |
|---|---|---|
| `Calls` | AST call expression analysis | 1.0 (EXTRACTED) |
| `Imports` / `ImportsFrom` | AST import statement | 1.0 (EXTRACTED) |
| `Contains` | AST parent-child (class -> method) | 1.0 (EXTRACTED) |
| `DependsOn` | Semantic inference (cross-file) | 0.7 (INFERRED) |
| `Implements` | AST superclass/trait impl | 1.0 (EXTRACTED) |
| `Configures` | Semantic inference (config -> service) | 0.7 (INFERRED) |
| `Extends` | AST inheritance/generics | 1.0 (EXTRACTED) |
| `Instantiates` | AST constructor calls | 1.0 (EXTRACTED) |

#### Integration with Existing 8 Analyzers

The graphify analyzer enhances the existing ones:

- **ComplexityAnalyzer**: graphify's god_nodes identifies high-coupling entities that complexity metrics miss (cyclomatic complexity is per-function; graphify measures cross-function dependency fan-out).
- **DependencyAnalyzer**: graphify provides actual call-graph edges, not just import declarations. It detects runtime dependencies that static import analysis cannot.
- **SecurityAnalyzer**: entities labeled `Endpoint` with INFERRED edges to unsanitized inputs surface attack vectors.
- **TopologyAnalyzer**: graphify communities map directly to architectural modules. Community cohesion scores validate or challenge the discovered topology.
- **DataSourceAnalyzer**: graphify edges from `Config` entities to `Service` entities reveal which services depend on which data sources, completing the data source map.

#### Assessment Report Enhancement

Before graphify, the assessment report has: file counts, complexity scores, dependency lists, security findings, topology hints.

After graphify, the assessment report gains:
- **Architectural map**: visual graph of module dependencies with community boundaries.
- **Coupling hotspots**: god nodes with quantified fan-in/fan-out.
- **Hidden dependencies**: INFERRED cross-module edges not visible in import statements.
- **Testability gaps**: entities with high fan-out but no test file containing their name.
- **Documentation gaps**: public interfaces with no doc comments and high usage count.

### 6.2 Forensic Analysis Domain (`domain/forensic.rs`)

#### Entity Types

| EntityType | Description | Extraction Source |
|---|---|---|
| `Person` | Named individual | NER in documents, witness lists |
| `Event` | Discrete occurrence with time/place | Temporal expressions, incident reports |
| `Evidence` | Physical or digital evidence item | Evidence logs, chain-of-custody docs |
| `Location` | Geographic place or address | NER, address patterns |
| `Timeline` | Ordered sequence of events | Inferred from temporal edges |
| `Document` | Source document (report, transcript) | File metadata |
| `Hypothesis` | Investigator theory or conclusion | Inferred from reasoning patterns |
| `Organization` | Company, agency, institution | NER |
| `PhysicalObject` | Non-evidence physical item | Context extraction |
| `DigitalArtifact` | Digital record (email, log, photo) | File type detection |
| `FinancialRecord` | Transaction, account, financial doc | Pattern matching |
| `Communication` | Phone call, message, email exchange | Communication metadata |

#### Edge Types

| RelationType | Description | Typical Confidence |
|---|---|---|
| `WitnessedBy` | Person observed an event | EXTRACTED (from statements) |
| `FoundAt` | Evidence located at a place | EXTRACTED |
| `Contradicts` | Two statements/evidence disagree | INFERRED |
| `Corroborates` | Two sources support same conclusion | INFERRED |
| `AlibiedBy` | Person provides alibi for another | EXTRACTED |
| `Precedes` | Temporal ordering between events | EXTRACTED/INFERRED |
| `DocumentedIn` | Entity mentioned in a document | EXTRACTED |
| `OwnedBy` | Object/account belongs to person | EXTRACTED/INFERRED |
| `ContactedBy` | Communication between persons | EXTRACTED |
| `LocatedAt` | Person/object at a location at time | EXTRACTED/INFERRED |
| `SemanticallySimilarTo` | Conceptual similarity (embedding) | INFERRED |

#### Gap Analysis

```rust
// domain/forensic.rs

pub struct GapAnalysis {
    /// Events with no witnesses.
    pub unwitnessed_events: Vec<EntityId>,
    /// Evidence not linked to any location.
    pub unlocated_evidence: Vec<EntityId>,
    /// Persons with no alibi for key events.
    pub unalibiied_persons: Vec<(EntityId, Vec<EntityId>)>,  // (person, events)
    /// Timeline gaps: consecutive events with large temporal distance.
    pub timeline_gaps: Vec<TimelineGap>,
    /// Contradictions: pairs of entities with Contradicts edges.
    pub contradictions: Vec<(EntityId, EntityId, String)>,  // (a, b, reason)
    /// Low-confidence critical paths: AMBIGUOUS edges on shortest paths between key entities.
    pub weak_links: Vec<WeakLink>,
    /// Completeness score: 0.0 (many gaps) to 1.0 (comprehensive).
    pub completeness_score: f64,
}

pub struct TimelineGap {
    pub before: EntityId,
    pub after: EntityId,
    pub gap_description: String,
}

pub struct WeakLink {
    pub path: Vec<EntityId>,
    pub weakest_edge: (EntityId, EntityId),
    pub confidence: Confidence,
    pub impact: String,
}

impl ForensicDomain {
    /// Analyze the knowledge graph for investigative gaps.
    pub fn gap_analysis(&self, kg: &KnowledgeGraph) -> GapAnalysis;

    /// Score how complete the case documentation is.
    /// Factors: entity coverage, edge density, confidence distribution,
    /// timeline continuity, witness coverage.
    pub fn coherence_score(&self, kg: &KnowledgeGraph) -> f64;
}
```

#### Coherence Scoring

The coherence score (0.0 to 1.0) is computed as a weighted average of:

| Factor | Weight | Description |
|---|---|---|
| Entity coverage | 0.20 | Ratio of entities with >= 2 relationships |
| Edge confidence | 0.25 | Weighted average of edge confidence values |
| Timeline continuity | 0.20 | Fraction of events with temporal ordering |
| Witness coverage | 0.15 | Fraction of events with >= 1 WitnessedBy edge |
| Evidence linkage | 0.10 | Fraction of evidence linked to both location and person |
| Contradiction resolution | 0.10 | Fraction of Contradicts edges with associated Hypothesis |

---

## 7. Performance Requirements

### 7.1 Targets

| Workload | Target | Bottleneck | Strategy |
|---|---|---|---|
| 10K code files, AST only | < 2 minutes | CPU (tree-sitter parsing) | rayon par_iter, 8 threads |
| 10K code files, AST + semantic | < 5 minutes | IO (Claude API latency) | tokio, 10 concurrent calls |
| 1K documents, semantic only | < 10 minutes | IO (Claude API latency) | tokio, 5 concurrent calls |
| 50K entity graph in memory | < 500 MB | petgraph node/edge storage | ~10 KB per entity avg |
| HNSW search (384-dim) | < 10 ms per query | Mutex acquisition | batch search API |
| Incremental re-scan (100 changed files of 10K) | < 30 seconds | Cache lookup + delta extraction | BLAKE3 content hash |

### 7.2 Memory Budget

```
Entity: ~200 bytes (id: 32, type: 8, label: ~40, source_file: ~60, metadata: ~60)
Relationship: ~160 bytes (src: 32, tgt: 32, type: 8, confidence: 1, weight: 4, metadata: ~80)
petgraph overhead: ~64 bytes per node, ~48 bytes per edge

50K entities: 50,000 * (200 + 64) = ~13 MB
150K relationships: 150,000 * (160 + 48) = ~31 MB
Total graph: ~44 MB

HNSW index (384-dim, 50K vectors): 50,000 * 384 * 4 = ~75 MB
Metadata per vector: ~200 bytes * 50,000 = ~10 MB
Total HNSW: ~85 MB

Cache index (in memory): ~32 bytes per entry * 10,000 files = ~320 KB

Total working set: ~130 MB (well under 500 MB target)
```

### 7.3 Concurrency Model

```
rayon thread pool (AST extraction)
  |-- 8 threads (configurable, default = num_cpus)
  |-- Each thread: parse file -> extract nodes/edges -> return Vec<ExtractionResult>
  |-- No shared mutable state during extraction

tokio runtime (semantic extraction + HNSW + LLM)
  |-- Semaphore(N) for concurrent LLM API calls
  |-- Each task: read file content -> call Claude API -> parse response -> return ExtractionResult
  |-- Backpressure: if semaphore is full, new tasks wait

Single-threaded phases:
  |-- build::merge() -- sequential for deterministic deduplication order
  |-- cluster::leiden() -- petgraph Leiden is single-threaded
  |-- analyze::* -- read-only graph traversals
```

---

## 8. Implementation Phases

### Phase 1: Core Data Model + AST Extraction (Sprint 17, Week 1-2)

**Deliverables:**
- `model.rs`, `entity.rs`, `relationship.rs` -- all types defined and tested
- `extract/detect.rs` -- file collection and language detection
- `extract/ast.rs` -- tree-sitter extraction for Python, JavaScript, TypeScript, Rust (4 languages)
- `build.rs` -- merge extractions, deduplicate entities
- `cache.rs` -- BLAKE3 content-hash cache
- `export/json.rs` -- JSON export

**Milestone:** `weaver graphify ingest ./src --mode fast --format json` produces a correct JSON knowledge graph for a Rust + Python project.

**Tests:** Unit tests for each extractor (insta snapshots), integration test with a multi-file fixture.

### Phase 2: Analysis + Clustering + CLI (Sprint 17, Week 3-4)

**Deliverables:**
- `cluster.rs` -- Leiden community detection on petgraph
- `analyze.rs` -- god_nodes, surprising_connections, suggest_questions, graph_diff
- `domain/code.rs` -- code domain entity/edge type mappings
- `pipeline.rs` -- full pipeline orchestrator
- CLI commands: `weaver graphify ingest`, `weaver graphify analyze`, `weaver graphify export`
- `export/html.rs` -- interactive HTML visualization (port from Python)

**Milestone:** `weaver graphify ingest ./src && weaver graphify analyze` produces god nodes, surprises, and questions. HTML export renders an interactive graph.

**Tests:** Snapshot tests for analysis output, HTML export golden file test.

### Phase 3: Kernel Bridge + DEMOCRITUS Integration (Sprint 18, Week 1-2)

**Deliverables:**
- `bridge.rs` -- CausalGraph, HNSW, CrossRefStore integration
- New `ImpulseType::GraphifyFileChanged` and `GraphifyExtractionComplete` variants
- DEMOCRITUS loop handler for graphify impulses
- `GraphifyAnalyzer` registered in `AnalyzerRegistry`
- `weaver graphify query --semantic` via HNSW

**Milestone:** `weft assess` includes graphify analysis. Changed files trigger incremental extraction via DEMOCRITUS. Semantic search finds entities by meaning.

**Tests:** Integration tests with mock EmbeddingProvider, CausalGraph bridge round-trip test.

### Phase 4: Semantic Extraction + Forensic Domain (Sprint 18, Week 3-4)

**Deliverables:**
- `extract/semantic.rs` -- LLM-based semantic extraction via `clawft-llm`
- `domain/forensic.rs` -- forensic entity/edge types, gap analysis, coherence scoring
- `extract/vision.rs` -- image/diagram extraction (feature-gated)
- `weaver graphify ingest --mode deep --domain forensic`
- `weaver graphify analyze --gaps`

**Milestone:** Full forensic pipeline works end-to-end. Gap analysis surfaces missing witnesses, unlinked evidence, timeline discontinuities. Coherence score quantifies case completeness.

**Tests:** Forensic domain fixture (synthetic cold case), gap analysis assertions, coherence score calibration tests.

### Phase 5: Polish + Performance + Additional Languages (Sprint 19, Week 1-2)

**Deliverables:**
- Additional language support: Go, Java, C, C++, Ruby, C# (6 more languages, total 10)
- `export/graphml.rs`, `export/cypher.rs` -- additional export formats
- `weaver graphify watch --daemon` -- continuous file watching
- Performance optimization: benchmark suite, memory profiling
- `weaver graphify ingest --interactive` REPL mode

**Milestone:** All 10 languages extract correctly. Performance targets met (10K files < 5 min). Watch mode runs as daemon.

**Tests:** Per-language extraction snapshots, performance benchmark (fail if regression > 10%), watch mode integration test.

---

## Appendix A: Extractor Trait

```rust
// extract/mod.rs

/// Result of extracting knowledge from a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub source_file: PathBuf,
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub hyperedges: Vec<Hyperedge>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub errors: Vec<String>,
}

/// A hyperedge connecting multiple entities (e.g., a function calling multiple services).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperedge {
    pub label: String,
    pub entity_ids: Vec<EntityId>,
    pub metadata: serde_json::Value,
}

/// Trait for knowledge extractors.
#[async_trait::async_trait]
pub trait Extractor: Send + Sync {
    /// Unique identifier for this extractor.
    fn id(&self) -> &str;
    /// Extract knowledge from a single file.
    async fn extract(&self, path: &Path, content: &[u8]) -> Result<ExtractionResult, GraphifyError>;
    /// Whether this extractor can handle the given file type.
    fn supports(&self, path: &Path) -> bool;
}
```

## Appendix B: StructureTag and CrossRefType Extensions

```rust
// New StructureTag value (registered in crossref.rs):
StructureTag::Custom(0x20)  // Graphify KnowledgeGraph

// New CrossRefType values for graphify relationships:
// 0x20..0x2F: Code domain relations
CrossRefType::Custom(0x20)  // Calls
CrossRefType::Custom(0x21)  // Imports
CrossRefType::Custom(0x22)  // DependsOn
CrossRefType::Custom(0x23)  // Contains
CrossRefType::Custom(0x24)  // Implements
CrossRefType::Custom(0x25)  // Configures

// 0x30..0x3F: Forensic domain relations
CrossRefType::Custom(0x30)  // WitnessedBy
CrossRefType::Custom(0x31)  // FoundAt
CrossRefType::Custom(0x32)  // Contradicts
CrossRefType::Custom(0x33)  // Corroborates
CrossRefType::Custom(0x34)  // AlibiedBy
CrossRefType::Custom(0x35)  // Precedes
```

## Appendix C: Dependency Graph

```
clawft-graphify
  |-- serde, serde_json (workspace)
  |-- blake3 (workspace)
  |-- dashmap (workspace)
  |-- tokio (workspace)
  |-- tracing (workspace)
  |-- thiserror (workspace)
  |-- rayon (new, 1.10)
  |-- petgraph (new, 0.7)
  |-- tree-sitter (optional, 0.24)
  |-- tree-sitter-{python,javascript,...} (optional, per language)
  |-- clawft-kernel (optional, kernel-bridge feature)
  |-- clawft-llm (optional, semantic-extract feature)
```

No circular dependencies. `clawft-graphify` depends on `clawft-kernel` and `clawft-llm` only optionally. The kernel depends on graphify only through the runtime `AnalyzerRegistry::register()` call, not a compile-time crate dependency (the `GraphifyAnalyzer` lives inside `clawft-graphify` and is registered at startup by the CLI binary).
