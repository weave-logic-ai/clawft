# ECC Weaver Crate -- SPARC Plan

**Crate**: `crates/ecc-weaver`
**Status**: Design
**Date**: 2026-03-26
**Depends on**: `clawft-kernel` (ECC types), `exo-resource-tree`

---

## S -- Specification

### What This Crate Provides

The `ecc-weaver` crate is the Rust API backing the Weaver skill. It orchestrates
all operations on the ECC forest of trees: initialization, ingestion, stitching,
pruning, and analysis. It treats the combined ECC structures as a single cognitive
fabric called a Loom.

### Key Types

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use clawft_kernel::causal::{CausalGraph, CausalEdge, CausalEdgeType, NodeId};
use clawft_kernel::cognitive_tick::{CognitiveTick, CognitiveTickConfig};
use clawft_kernel::crossref::{CrossRefStore, UniversalNodeId, StructureTag, CrossRef, CrossRefType};
use clawft_kernel::hnsw_service::{HnswService, HnswServiceConfig, HnswSearchResult};
use clawft_kernel::impulse::{ImpulseQueue, ImpulseType, Impulse};
use clawft_kernel::chain::ChainManager;
use clawft_kernel::tree_manager::TreeManager;

/// The combined ECC structures for a single domain.
///
/// A Loom holds all the trees in the forest for one cognitive workspace.
/// Every operation on the Loom maintains consistency across all structures:
/// causal edges have CrossRefs to HNSW entries, chain events record provenance,
/// and the resource tree reflects the domain's namespace organization.
pub struct Loom {
    /// Domain identifier (e.g., "my-project").
    pub domain: String,
    /// The causal reasoning graph.
    pub causal: CausalGraph,
    /// HNSW vector search index.
    pub hnsw: HnswService,
    /// Cross-references linking nodes across structures.
    pub crossrefs: CrossRefStore,
    /// Ephemeral inter-structure event queue.
    pub impulses: ImpulseQueue,
    /// Provenance chain for tamper-evident audit trail.
    pub chain: ChainManager,
    /// Namespace and resource organization.
    pub tree: TreeManager,
    /// Cognitive tick heartbeat.
    pub tick: CognitiveTick,
    /// Parsed configuration from weave.toml.
    pub config: WeaveConfig,
}

/// The top-level orchestrator for all weaving operations.
///
/// Manages multiple Looms (one per domain) and the pattern library.
/// Provides the public API that the Weaver skill and CLI invoke.
pub struct WeaverEngine {
    /// Active looms keyed by domain name.
    looms: HashMap<String, Loom>,
    /// Global pattern library (shared across looms).
    patterns: Vec<Pattern>,
}

/// A recurrent structural pattern (warp thread).
///
/// Patterns define sequences of CausalEdgeTypes that repeat under
/// certain conditions. They are the "shape" of the conversation.
pub struct Pattern {
    /// Human-readable name (e.g., "ci-cd").
    pub name: String,
    /// Description of what this pattern represents.
    pub description: String,
    /// The sequence of edge types that constitutes one occurrence.
    pub edge_sequence: Vec<CausalEdgeType>,
    /// When this pattern recurs.
    pub recurrence: RecurrenceType,
}

/// When a Pattern fires.
pub enum RecurrenceType {
    /// Fire on every new commit event.
    OnCommit,
    /// Fire on every cognitive tick.
    OnTick,
    /// Fire when a named event occurs.
    OnEvent(String),
    /// Fire at a fixed interval.
    Periodic(Duration),
}

// ── Configuration ──────────────────────────────────────────────────

/// Parsed weave.toml.
pub struct WeaveConfig {
    pub domain: DomainConfig,
    pub tick: CognitiveTickConfig,
    pub causal: CausalConfig,
    pub hnsw: HnswConfig,
    pub impulse: ImpulseConfig,
    pub patterns: Vec<PatternConfig>,
    pub sources: Vec<SourceConfig>,
    pub meta: MetaConfig,
}

pub struct DomainConfig {
    pub name: String,
    pub description: String,
    pub mode: OperatingMode,
}

pub enum OperatingMode {
    Act,
    Analyze,
    Generate,
}

pub struct CausalConfig {
    pub edge_types: Vec<CausalEdgeType>,
    pub decay_rate: f32,
    pub decay_threshold: f32,
    pub max_edges: usize,
    pub max_nodes: usize,
}

pub struct HnswConfig {
    pub dimensions: usize,
    pub ef_search: usize,
    pub ef_construction: usize,
    pub max_entries: usize,
}

pub struct ImpulseConfig {
    pub ttl_ticks: u32,
    pub max_queue_depth: usize,
}

pub struct PatternConfig {
    pub name: String,
    pub description: String,
    pub edge_sequence: Vec<String>,
    pub recurrence: String,
}

pub struct MetaConfig {
    pub enabled: bool,
    pub conversations: Vec<MetaConversationConfig>,
}

pub struct MetaConversationConfig {
    pub name: String,
    pub relates_to: Vec<String>,
    pub pattern: String,
}

// ── Source configuration ───────────────────────────────────────────

pub enum SourceConfig {
    GitLog { path: PathBuf, branch: String },
    FileTree { root: PathBuf, patterns: Vec<String> },
    Documentation { root: PathBuf },
    SparcPlan { path: PathBuf },
    Api { url: String, auth: Option<String> },
}

// ── Analysis types ─────────────────────────────────────────────────

/// Codebase analyzer that produces initialization plans.
pub struct Analyzer {
    pub domain: String,
}

impl Analyzer {
    pub fn analyze_codebase(path: &Path) -> AnalysisPlan { todo!() }
    pub fn analyze_documentation(path: &Path) -> AnalysisPlan { todo!() }
    pub fn analyze_git_history(path: &Path) -> AnalysisPlan { todo!() }
    pub fn suggest_patterns(plan: &AnalysisPlan) -> Vec<Pattern> { todo!() }
    pub fn identify_meta_conversations(plan: &AnalysisPlan) -> Vec<MetaConversation> { todo!() }
}

/// The output of analysis: a plan for what to create in the Loom.
pub struct AnalysisPlan {
    pub nodes: Vec<PlannedNode>,
    pub edges: Vec<PlannedEdge>,
    pub embeddings: Vec<PlannedEmbedding>,
    pub crossrefs: Vec<PlannedCrossRef>,
    pub estimated_vectors: usize,
    pub estimated_edges: usize,
    pub recommended_dimensions: usize,
    pub meta_conversations: Vec<MetaConversation>,
}

pub struct PlannedNode {
    pub label: String,
    pub node_type: String,
    pub metadata: serde_json::Value,
}

pub struct PlannedEdge {
    pub source_label: String,
    pub target_label: String,
    pub edge_type: CausalEdgeType,
    pub weight: f32,
}

pub struct PlannedEmbedding {
    pub node_label: String,
    pub content_chunk: String,
    pub estimated_dimensions: usize,
}

pub struct PlannedCrossRef {
    pub source_structure: StructureTag,
    pub target_structure: StructureTag,
    pub ref_type: CrossRefType,
    pub description: String,
}

pub struct MetaConversation {
    pub name: String,
    pub description: String,
    pub relates_to: Vec<String>,
    pub pattern: String,
}

// ── Merging ────────────────────────────────────────────────────────

/// Forest stitching operations.
pub struct Merger;

impl Merger {
    pub fn stitch(source: &Loom, target: &Loom) -> StitchResult { todo!() }
    pub fn resolve_conflicts(a: &CausalEdge, b: &CausalEdge) -> ConflictResolution { todo!() }
}

pub struct StitchResult {
    pub cross_refs_created: usize,
    pub conflicts_resolved: usize,
    pub novel_connections: Vec<NovelConnection>,
}

pub struct NovelConnection {
    pub source_domain: String,
    pub target_domain: String,
    pub similarity: f32,
    pub description: String,
}

pub enum ConflictResolution {
    PreferHigherWeight,
    PreferMoreRecent,
    KeepBoth,
}

// ── Pruning ────────────────────────────────────────────────────────

/// Trimming and garbage collection.
pub struct Pruner;

impl Pruner {
    pub fn apply_decay(loom: &Loom, decay_rate: f32) -> DecayReport { todo!() }
    pub fn prune(loom: &Loom, threshold: f32) -> PruneReport { todo!() }
    pub fn gc_crossrefs(loom: &Loom) -> usize { todo!() }
    pub fn archive_subtree(loom: &Loom, root_node: NodeId) -> Vec<u8> { todo!() }
}

pub struct DecayReport {
    pub edges_decayed: usize,
    pub edges_below_threshold: usize,
}

pub struct PruneReport {
    pub edges_removed: usize,
    pub nodes_removed: usize,
    pub hnsw_entries_removed: usize,
    pub crossrefs_removed: usize,
    pub bytes_archived: usize,
}

// ── Ingestion ──────────────────────────────────────────────────────

/// Data source ingestion into ECC structures.
pub struct Ingester;

impl Ingester {
    pub fn ingest_git_log(loom: &mut Loom, config: &SourceConfig) -> IngestReport { todo!() }
    pub fn ingest_file_tree(loom: &mut Loom, config: &SourceConfig) -> IngestReport { todo!() }
    pub fn ingest_documentation(loom: &mut Loom, config: &SourceConfig) -> IngestReport { todo!() }
    pub fn ingest_sparc_plan(loom: &mut Loom, config: &SourceConfig) -> IngestReport { todo!() }
}

pub struct IngestReport {
    pub nodes_created: usize,
    pub edges_created: usize,
    pub embeddings_inserted: usize,
    pub crossrefs_created: usize,
    pub duration_ms: u64,
}
```

### Crate Feature Flags

```toml
[features]
default = ["git", "embeddings"]
git = ["gix"]               # Git history ingestion via gitoxide
embeddings = []              # Embedding generation (trait-based, runtime pluggable)
watch = ["notify"]           # Filesystem watching for act mode
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum WeaverError {
    #[error("loom not found: {0}")]
    LoomNotFound(String),
    #[error("configuration error: {0}")]
    ConfigError(String),
    #[error("ingestion failed: {source}")]
    IngestError { source: Box<dyn std::error::Error + Send + Sync> },
    #[error("stitch conflict: {0}")]
    StitchConflict(String),
    #[error("pruning error: {0}")]
    PruneError(String),
    #[error("analysis error: {0}")]
    AnalysisError(String),
    #[error("io error: {source}")]
    Io { #[from] source: std::io::Error },
    #[error("toml parse error: {source}")]
    TomlParse { #[from] source: toml::de::Error },
}
```

---

## P -- Pseudocode

### 1. Loom Initialization from WeaveConfig

```
FUNCTION init_loom(config: WeaveConfig) -> Result<Loom>:
    # 1. Create empty ECC structures
    causal = CausalGraph::new()
    hnsw = HnswService::new(HnswServiceConfig {
        ef_search: config.hnsw.ef_search,
        ef_construction: config.hnsw.ef_construction,
        default_dimensions: config.hnsw.dimensions,
    })
    crossrefs = CrossRefStore::new()
    impulses = ImpulseQueue::new()
    chain = ChainManager::new(config.domain.name)
    tree = TreeManager::new(chain)
    tick = CognitiveTick::new(config.tick.clone())

    # 2. Run calibration
    calibration = run_calibration(&hnsw, &causal, &default_calibration_config())
    IF calibration.tick_interval_ms > config.tick.tick_interval_ms:
        log_warn("Hardware requires slower tick: {}ms", calibration.tick_interval_ms)

    # 3. Create domain root node in causal graph
    root_id = causal.add_node(
        label: format!("domain:{}", config.domain.name),
        metadata: json!({
            "type": "domain_root",
            "mode": config.domain.mode,
            "created_at": now_hlc()
        })
    )

    # 4. Create namespace nodes from sources
    FOR source IN config.sources:
        namespace_label = source.namespace_label()
        ns_id = causal.add_node(namespace_label, json!({"type": "namespace"}))
        causal.link(root_id, ns_id, Enables, 1.0, now_hlc(), chain.next_seq())

        # Create ResourceTree entry
        tree.insert(namespace_label, ResourceKind::Namespace)

    # 5. Record genesis in ExoChain
    chain.append("weaver.genesis", json!({
        "domain": config.domain.name,
        "namespaces": config.sources.len(),
        "calibration": calibration,
    }))

    # 6. Create CrossRefs linking chain genesis to causal root
    crossrefs.insert(CrossRef {
        source: chain.genesis_universal_id(),
        source_structure: StructureTag::ExoChain,
        target: causal_node_universal_id(root_id),
        target_structure: StructureTag::CausalGraph,
        ref_type: CrossRefType::TriggeredBy,
        created_at: now_hlc(),
        chain_seq: chain.current_seq(),
    })

    # 7. Start cognitive tick
    tick.start()

    RETURN Ok(Loom {
        domain: config.domain.name,
        causal, hnsw, crossrefs, impulses,
        chain, tree, tick, config
    })
```

### 2. Codebase Analysis into AnalysisPlan

```
FUNCTION analyze_codebase(path: Path) -> AnalysisPlan:
    plan = AnalysisPlan::empty()

    # Phase 1: File tree discovery
    files = walk_directory(path, skip: [".git", "target", "node_modules"])
    FOR file IN files:
        plan.nodes.push(PlannedNode {
            label: file.relative_path,
            node_type: classify_file(file),  # "source", "test", "config", "doc"
            metadata: json!({ "size": file.size, "ext": file.extension }),
        })

    # Phase 2: Module/dependency analysis
    modules = extract_modules(files)  # parse Cargo.toml, package.json, mod.rs, etc.
    FOR (mod_a, mod_b) IN module_dependencies(modules):
        plan.edges.push(PlannedEdge {
            source: mod_a.name,
            target: mod_b.name,
            edge_type: Causes,  # mod_a depends on mod_b
            weight: 0.8,
        })

    # Phase 3: Test-to-source mapping
    FOR (test_file, source_file) IN test_source_pairs(files):
        plan.edges.push(PlannedEdge {
            source: test_file,
            target: source_file,
            edge_type: EvidenceFor,
            weight: 0.9,
        })

    # Phase 4: Embedding plan
    FOR file IN files WHERE file.is_source_or_doc():
        chunks = chunk_file(file, max_tokens: 512)
        FOR chunk IN chunks:
            plan.embeddings.push(PlannedEmbedding {
                node_label: file.relative_path,
                content_chunk: chunk.text,
                estimated_dimensions: 384,
            })

    # Phase 5: CrossRef plan
    FOR embedding IN plan.embeddings:
        plan.crossrefs.push(PlannedCrossRef {
            source_structure: StructureTag::HnswIndex,
            target_structure: StructureTag::CausalGraph,
            ref_type: CrossRefType::MemoryEncoded,
            description: format!("embedding for {}", embedding.node_label),
        })

    # Phase 6: Meta-conversation identification
    plan.meta_conversations = identify_meta_conversations(files, modules)

    # Phase 7: Compute estimates
    plan.estimated_vectors = plan.embeddings.len()
    plan.estimated_edges = plan.edges.len()
    plan.recommended_dimensions = recommend_dimensions(plan.estimated_vectors)

    RETURN plan
```

### 3. Pattern Weaving (SDLC as Conversation)

```
FUNCTION weave_git_history(loom: &mut Loom, repo_path: Path, branch: String) -> IngestReport:
    report = IngestReport::zero()
    repo = open_git_repo(repo_path)
    commits = repo.log(branch, limit: loom.config.sources.git_depth)

    # Create nodes for all commits
    commit_nodes: HashMap<OID, NodeId> = {}
    FOR commit IN commits:
        node_id = loom.causal.add_node(
            label: format!("commit:{}", commit.short_oid),
            metadata: json!({
                "oid": commit.oid,
                "author": commit.author,
                "message": commit.message,
                "timestamp": commit.timestamp,
                "files_changed": commit.files_changed,
            })
        )
        commit_nodes.insert(commit.oid, node_id)
        report.nodes_created += 1

    # Create edges for commit relationships
    FOR commit IN commits:
        node = commit_nodes[commit.oid]

        # Parent -> child = Follows (sequential conversation flow)
        FOR parent_oid IN commit.parents:
            IF parent_node = commit_nodes.get(parent_oid):
                loom.causal.link(parent_node, node, Follows, 1.0, commit.hlc, chain_seq)
                report.edges_created += 1

        # Merge commits: both parents Enable the merge
        IF commit.parents.len() > 1:
            FOR parent_oid IN commit.parents:
                IF parent_node = commit_nodes.get(parent_oid):
                    loom.causal.link(parent_node, node, Enables, 0.9, commit.hlc, chain_seq)

        # Issue references: commit TriggeredBy issue
        FOR issue_ref IN extract_issue_refs(commit.message):
            issue_node = get_or_create_issue_node(loom, issue_ref)
            loom.causal.link(issue_node, node, TriggeredBy, 0.8, commit.hlc, chain_seq)
            report.edges_created += 1

    # Create embeddings for commit messages + diffs
    FOR (oid, node_id) IN commit_nodes:
        commit = repo.get_commit(oid)
        content = format!("{}\n{}", commit.message, commit.diff_summary)
        embedding = generate_embedding(content)  # via pluggable trait
        hnsw_id = format!("commit:{}", oid)
        loom.hnsw.insert(hnsw_id, embedding, json!({"node_id": node_id}))
        report.embeddings_inserted += 1

        # CrossRef: HNSW entry -> CausalGraph node
        loom.crossrefs.insert(CrossRef {
            source: hnsw_universal_id(hnsw_id),
            source_structure: StructureTag::HnswIndex,
            target: causal_node_universal_id(node_id),
            target_structure: StructureTag::CausalGraph,
            ref_type: CrossRefType::MemoryEncoded,
            created_at: now_hlc(),
            chain_seq: loom.chain.next_seq(),
        })
        report.crossrefs_created += 1

    # Record in ExoChain
    loom.chain.append("weaver.ingest.git", json!({
        "branch": branch,
        "commits": commits.len(),
        "report": report,
    }))

    # Detect patterns in the ingested history
    apply_patterns(loom, &commit_nodes)

    RETURN report

FUNCTION apply_patterns(loom: &mut Loom, nodes: &HashMap<OID, NodeId>):
    FOR pattern IN loom.config.patterns:
        # Scan for sequences of edges matching the pattern's edge_sequence
        matches = find_pattern_matches(loom.causal, pattern.edge_sequence)
        FOR match IN matches:
            # Reinforce matched edges by refreshing their weight
            FOR edge IN match.edges:
                edge.weight = (edge.weight + 0.1).min(1.0)
            # Emit novelty impulse if this is a new pattern instance
            IF match.is_new:
                loom.impulses.emit(
                    source_structure: StructureTag::CausalGraph.as_u8(),
                    source_node: match.root_universal_id,
                    target_structure: StructureTag::HnswIndex.as_u8(),
                    impulse_type: ImpulseType::NoveltyDetected,
                    payload: json!({"pattern": pattern.name, "match": match.id}),
                    hlc_timestamp: now_hlc(),
                )
```

### 4. Forest Stitching (Merge Two Looms)

```
FUNCTION stitch(source: &Loom, target: &Loom) -> StitchResult:
    result = StitchResult::empty()

    # Phase 1: Find semantic matches via HNSW cross-search
    # For each entry in source HNSW, search target HNSW for similar vectors
    source_entries = source.hnsw.all_entries()  # would need iteration API
    FOR entry IN source_entries:
        matches = target.hnsw.search(&entry.embedding, top_k: 5)
        FOR match IN matches WHERE match.score > 0.8:  # similarity threshold
            # Found a cross-domain semantic connection
            result.novel_connections.push(NovelConnection {
                source_domain: source.domain,
                target_domain: target.domain,
                similarity: match.score,
                description: format!("{} ~ {}", entry.id, match.id),
            })

            # Create cross-forest CrossRef
            target.crossrefs.insert(CrossRef {
                source: entry.universal_id,
                source_structure: StructureTag::HnswIndex,
                target: match.universal_id,
                target_structure: StructureTag::HnswIndex,
                ref_type: CrossRefType::Elaborates,
                created_at: now_hlc(),
                chain_seq: target.chain.next_seq(),
            })
            result.cross_refs_created += 1

    # Phase 2: Resolve causal graph conflicts
    # For each pair of matched nodes, check if their causal neighborhoods conflict
    FOR connection IN result.novel_connections:
        source_node = lookup_causal_node(source, connection.source_hnsw_id)
        target_node = lookup_causal_node(target, connection.target_hnsw_id)
        IF source_node AND target_node:
            source_edges = source.causal.get_forward_edges(source_node)
            target_edges = target.causal.get_forward_edges(target_node)
            conflicts = find_contradictions(source_edges, target_edges)
            FOR conflict IN conflicts:
                resolution = resolve_conflict(conflict)
                MATCH resolution:
                    PreferHigherWeight => keep_edge_with_higher_weight(conflict)
                    PreferMoreRecent => keep_edge_with_later_timestamp(conflict)
                    KeepBoth => keep_both_with_provenance_metadata(conflict)
                result.conflicts_resolved += 1

    # Phase 3: Create ExoChain bridge events
    target.chain.append("weaver.stitch.bridge", json!({
        "source_domain": source.domain,
        "target_domain": target.domain,
        "connections": result.novel_connections.len(),
        "conflicts": result.conflicts_resolved,
    }))

    # Phase 4: Emit novelty impulses
    FOR connection IN result.novel_connections:
        target.impulses.emit(
            source_structure: StructureTag::HnswIndex.as_u8(),
            source_node: connection.source_universal_id,
            target_structure: StructureTag::CausalGraph.as_u8(),
            impulse_type: ImpulseType::NoveltyDetected,
            payload: json!({"stitch": true, "similarity": connection.similarity}),
            hlc_timestamp: now_hlc(),
        )

    RETURN result
```

### 5. Pruning with Decay

```
FUNCTION prune_loom(loom: &mut Loom) -> PruneReport:
    report = PruneReport::zero()
    config = &loom.config.causal

    # Phase 1: Apply decay to all edges
    all_node_ids = loom.causal.all_node_ids()
    FOR node_id IN all_node_ids:
        edges = loom.causal.get_forward_edges(node_id)
        FOR edge IN edges:
            # Decay: new_weight = weight * (1.0 - decay_rate)
            new_weight = edge.weight * (1.0 - config.decay_rate)
            IF new_weight < config.decay_threshold:
                # Mark for removal
                edges_to_remove.push((edge.source, edge.target))
            ELSE:
                # Update weight (requires mutable edge access or re-link)
                update_edge_weight(loom.causal, edge, new_weight)

    # Phase 2: Remove decayed edges
    FOR (source, target) IN edges_to_remove:
        removed = loom.causal.unlink(source, target)
        report.edges_removed += removed

    # Phase 3: Remove orphaned nodes (no edges in or out)
    FOR node_id IN all_node_ids:
        forward = loom.causal.get_forward_edges(node_id)
        reverse = loom.causal.get_reverse_edges(node_id)
        IF forward.is_empty() AND reverse.is_empty():
            # Check if node is a domain root (never prune roots)
            node = loom.causal.get_node(node_id)
            IF node.metadata["type"] != "domain_root":
                loom.causal.remove_node(node_id)
                report.nodes_removed += 1

    # Phase 4: Clean up HNSW entries for removed nodes
    FOR removed_node_id IN removed_nodes:
        # Find HNSW entries linked to this node via CrossRefs
        universal_id = causal_node_universal_id(removed_node_id)
        refs = loom.crossrefs.get_reverse(&universal_id)
        FOR ref IN refs WHERE ref.source_structure == StructureTag::HnswIndex:
            # Remove the HNSW entry (requires HnswService.remove API)
            remove_hnsw_entry(loom.hnsw, ref.source)
            report.hnsw_entries_removed += 1

    # Phase 5: Garbage collect unreferenced CrossRefs
    report.crossrefs_removed = gc_crossrefs(loom)

    # Phase 6: Record in ExoChain
    loom.chain.append("weaver.prune", json!({
        "edges_removed": report.edges_removed,
        "nodes_removed": report.nodes_removed,
        "hnsw_removed": report.hnsw_entries_removed,
        "crossrefs_removed": report.crossrefs_removed,
    }))

    # Phase 7: Emit completion impulse
    loom.impulses.emit(
        source_structure: StructureTag::CausalGraph.as_u8(),
        source_node: [0u8; 32],
        target_structure: StructureTag::CausalGraph.as_u8(),
        impulse_type: ImpulseType::Custom(0x30),  # Pruning complete
        payload: json!(report),
        hlc_timestamp: now_hlc(),
    )

    RETURN report
```

---

## A -- Architecture

### Crate Hierarchy

```
crates/
  clawft-core/           # Core types, embeddings, HNSW store
  clawft-kernel/         # ECC services: CausalGraph, HnswService, CrossRefStore, etc.
  exo-resource-tree/     # Resource tree data structure
  rvf-crypto/            # BLAKE3, Ed25519, SHAKE-256 hashing
  ecc-weaver/            # THIS CRATE
    src/
      lib.rs             # Re-exports, WeaverEngine
      loom.rs            # Loom struct and lifecycle
      config.rs          # WeaveConfig, TOML parsing
      analyzer.rs        # Codebase analysis -> AnalysisPlan
      ingester/
        mod.rs           # Ingester trait and dispatch
        git.rs           # Git history ingestion
        file_tree.rs     # Source tree ingestion
        docs.rs          # Documentation ingestion
        sparc.rs         # SPARC plan ingestion
      merger.rs          # Forest stitching
      pruner.rs          # Decay, pruning, GC
      patterns.rs        # Pattern matching and detection
      meta.rs            # Meta-conversation tracking
      embedding.rs       # Embedding trait (pluggable providers)
      error.rs           # WeaverError
    Cargo.toml
```

### Dependency Graph

```
ecc-weaver
  |-- clawft-kernel  (CausalGraph, HnswService, CrossRefStore, ImpulseQueue,
  |                    CognitiveTick, ChainManager, TreeManager, calibration)
  |-- exo-resource-tree  (ResourceTree, MutationLog, ResourceKind)
  |-- rvf-crypto  (BLAKE3 hashing for UniversalNodeId generation)
  |-- toml  (weave.toml parsing)
  |-- serde + serde_json  (serialization)
  |-- thiserror  (error types)
  |-- tracing  (structured logging)
  |-- gix  (optional: git history via gitoxide)
  |-- notify  (optional: filesystem watching for act mode)
  |-- glob  (file pattern matching)
```

### Integration Points

**Kernel registration**: The WeaverEngine registers as a `SystemService` at
kernel boot when the `ecc` feature is enabled. It listens for impulses and
processes them on the cognitive tick.

```rust
#[async_trait]
impl SystemService for WeaverEngine {
    fn name(&self) -> &str { "ecc.weaver" }
    fn service_type(&self) -> ServiceType { ServiceType::Extension }

    async fn start(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Load all weave.toml configs from known locations
        // Initialize looms for each domain
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Checkpoint all looms
        // Flush impulse queues
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        // Check all looms are healthy
        // Report any with high drift counts
        HealthStatus::Healthy
    }
}
```

**CLI integration**: The `weaver ecc` subcommand dispatches to `WeaverEngine`
methods. This lives in the CLI crate, not in `ecc-weaver` itself.

**Embedding trait**: Embeddings are generated through a pluggable trait, allowing
different backends (local model, API call, pre-computed):

```rust
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, WeaverError>;
    fn dimensions(&self) -> usize;
}
```

### Data Flow

```
Source Data (git, files, docs)
    |
    v
Ingester  --creates-->  CausalGraph nodes + edges
    |                        |
    |--generates-->  Embeddings --> HnswService
    |                        |          |
    |--creates-->  CrossRefs linking HNSW <-> Causal <-> Chain
    |                                           |
    |--records-->  ChainManager (ExoChain events for provenance)
    |
    v
CognitiveTick processes impulses each tick
    |
    v
Pruner applies decay, removes stale structure
    |
    v
Analyzer reads the loom and produces reports
```

---

## R -- Refinement

### Performance Considerations

1. **Incremental ingestion**: When re-weaving git history, only process commits
   newer than the last ingested commit (stored in ExoChain metadata). Do not
   rebuild the entire history.

2. **Batch HNSW inserts**: Group embedding insertions into batches of 100 to
   reduce lock contention on the `Mutex<HnswStore>`.

3. **Lazy embedding generation**: If no embedding provider is configured, store
   nodes without embeddings and mark them for later processing. The cognitive
   tick can embed in background ticks.

4. **Pruning budget**: Pruning should respect the tick budget. If the graph is
   large, prune in chunks across multiple ticks rather than all at once.

5. **HNSW iteration**: The current `HnswService` API does not expose iteration
   over all entries. Forest stitching requires this. Either add an `iter()`
   method to `HnswService` or maintain a separate manifest of entry IDs in
   the Loom.

### Security Considerations

1. **No secrets in embeddings**: The ingester must strip secrets, API keys, and
   credentials from content before embedding. Apply the same rules as `.gitignore`
   plus additional patterns for common secret formats.

2. **Chain provenance**: Every weaving operation is recorded in ExoChain. This
   provides a tamper-evident audit trail of all modifications to the loom.

3. **Source validation**: When ingesting from external APIs, validate TLS
   certificates and authenticate requests. Never store raw API credentials
   in `weave.toml` -- use environment variable references.

4. **CrossRef integrity**: CrossRefs use `UniversalNodeId` (BLAKE3 hashes).
   Verify that referenced nodes actually exist before creating CrossRefs.

### Testing Strategy

1. **Unit tests per module**:
   - `config.rs`: TOML parsing roundtrip, invalid config rejection
   - `analyzer.rs`: Codebase analysis on fixture directories
   - `ingester/git.rs`: Git history parsing on a test repository
   - `ingester/file_tree.rs`: File tree scanning on fixture directories
   - `merger.rs`: Stitch two small looms, verify CrossRef creation
   - `pruner.rs`: Decay application, orphan removal, threshold correctness
   - `patterns.rs`: Pattern matching on known edge sequences

2. **Integration tests**:
   - Full pipeline: analyze -> init -> weave -> prune on a real git repo
   - Cross-domain stitch on two test looms
   - Cognitive tick interaction: verify impulses are emitted correctly

3. **Property tests** (proptest):
   - Pruning never removes domain root nodes
   - Decay is monotonically decreasing
   - CrossRef count after GC <= CrossRef count before GC
   - Stitch is commutative (stitch(A,B) produces same connections as stitch(B,A))

4. **Benchmark tests**:
   - Ingestion throughput: commits/second for git history weaving
   - Pruning latency: time to prune a 10K-node graph
   - Stitch latency: time to stitch two 5K-entry HNSW indices

---

## C -- Completion

### Exit Criteria Checklist

- [ ] `WeaveConfig` parses from TOML and round-trips through serde
- [ ] `Loom::init()` creates all ECC structures and records genesis in ExoChain
- [ ] `Analyzer::analyze_codebase()` produces an `AnalysisPlan` for a test directory
- [ ] `Analyzer::suggest_patterns()` identifies at least one pattern in a test plan
- [ ] `Ingester::ingest_git_log()` creates causal nodes/edges for a test git repo
- [ ] `Ingester::ingest_file_tree()` maps files to causal nodes with correct edge types
- [ ] Embeddings are inserted into HNSW with CrossRefs back to causal nodes
- [ ] `Merger::stitch()` creates cross-forest CrossRefs for semantically similar nodes
- [ ] `Merger::resolve_conflicts()` handles contradictory edges correctly
- [ ] `Pruner::apply_decay()` reduces edge weights by the configured rate
- [ ] `Pruner::prune()` removes edges below threshold and cleans orphaned nodes
- [ ] `Pruner::gc_crossrefs()` removes CrossRefs pointing to deleted nodes
- [ ] Pattern detection finds known patterns in a seeded causal graph
- [ ] Meta-conversations are identified and tracked as separate causal subgraphs
- [ ] All operations record provenance events in ExoChain
- [ ] Impulses are emitted for novelty detection, coherence alerts, and pruning
- [ ] `WeaverEngine` implements `SystemService` and registers at kernel boot
- [ ] `scripts/build.sh check` passes with no warnings
- [ ] `scripts/build.sh clippy` passes
- [ ] `scripts/build.sh test` passes all unit, integration, and property tests
- [ ] Benchmark tests establish baseline throughput numbers
- [ ] The WEAVER.md skill file commands map to working `WeaverEngine` methods
