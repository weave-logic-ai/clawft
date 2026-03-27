# ECC Weaver Crate -- SPARC Plan (v2: Self-Evolving Cognitive Modeler)

**Crate**: `crates/ecc-weaver`
**Status**: Design
**Date**: 2026-03-26
**Depends on**: `clawft-kernel` (ECC types, SystemService, A2ARouter, AgentSupervisor)

---

## S -- Specification

### What This Crate Provides

The `ecc-weaver` crate is a kernel-native `SystemService` that iteratively discovers,
refines, and maintains causal models from data. It runs a continuous confidence-driven
**HYPOTHESIZE -> OBSERVE -> EVALUATE -> ADJUST** loop, tracking its own evolution in
a meta-Loom. The Weaver's learned models export as `weave-model.json` for edge deployment.

### Kernel Integration Contract

The Weaver is NOT an external tool. It runs inside WeftOS:

- Implements `SystemService`, registered at boot when `ecc` feature is enabled
- Gets a PID in `ProcessTable`, supervised by `AgentSupervisor`
- Holds `Arc` references to kernel ECC structures (no serialization boundary)
- Consumes `CognitiveTick` events (does not run its own timer loop)
- Communicates via `A2ARouter` IPC (`KernelMessage`)
- Registers at `/kernel/services/weaver` in the resource tree
- Meta-Loom lives in the kernel's own ECC structures (tag `Custom(0x40)`)
- CLI commands (`weaver ecc *`) send messages via daemon Unix socket

### Key Types

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use clawft_kernel::causal::{CausalGraph, CausalEdge, CausalEdgeType, NodeId};
use clawft_kernel::cognitive_tick::{CognitiveTick, CognitiveTickConfig};
use clawft_kernel::crossref::{CrossRefStore, UniversalNodeId, StructureTag, CrossRef, CrossRefType};
use clawft_kernel::hnsw_service::{HnswService, HnswServiceConfig, HnswSearchResult};
use clawft_kernel::impulse::{ImpulseQueue, ImpulseType, Impulse};
use clawft_kernel::chain::ChainManager;
use clawft_kernel::tree_manager::TreeManager;
use clawft_kernel::service::{SystemService, ServiceType};
use clawft_kernel::a2a::A2ARouter;
use clawft_kernel::process::ProcessTable;

// ── The Loom ──────────────────────────────────────────────────────

/// The combined ECC structures for a single domain.
///
/// A Loom holds Arc references to the SHARED kernel structures.
/// Every operation maintains consistency across all structures.
pub struct Loom {
    pub domain: String,
    pub causal: Arc<CausalGraph>,
    pub hnsw: Arc<HnswService>,
    pub crossrefs: Arc<CrossRefStore>,
    pub impulses: Arc<ImpulseQueue>,
    pub chain: Arc<ChainManager>,
    pub tree: Arc<TreeManager>,
}

// ── Modeling Session ──────────────────────────────────────────────

/// The Weaver's core modeling loop state.
///
/// A session tracks the iterative hypothesis-observe-evaluate-adjust
/// loop for one domain. Its meta-Loom entries live in the kernel's
/// CausalGraph tagged with StructureTag::Custom(0x40).
pub struct ModelingSession {
    /// Domain identifier.
    pub domain: String,
    /// User-provided context that shapes model hypotheses.
    pub context: String,
    /// Target confidence the Weaver aims to reach.
    pub target_confidence: f64,
    /// The learned causal model (versioned, exportable).
    pub model: CausalModel,
    /// Latest confidence assessment.
    pub confidence: ConfidenceReport,
    /// Active data sources feeding this session.
    pub sources: Vec<DataSource>,
    /// Version history of the model.
    pub history: Vec<ModelVersion>,
    /// Arc to the Loom this session operates on.
    pub loom: Loom,
    /// Meta-loom root node ID (in the kernel's CausalGraph).
    pub meta_root: NodeId,
    /// Whether auto-adjust is enabled (Act mode = true).
    pub auto_adjust: bool,
}

// ── Causal Model ──────────────────────────────────────────────────

/// A causal model learned from data. This is the Weaver's output.
///
/// Contains the discovered structure: what kinds of nodes exist,
/// what kinds of edges connect them, what patterns recur, and
/// how the cognitive tick should be configured.
pub struct CausalModel {
    /// Monotonically increasing version number.
    pub version: u32,
    /// Discovered node type specifications.
    pub node_types: Vec<NodeTypeSpec>,
    /// Discovered edge type specifications with confidence.
    pub edge_types: Vec<EdgeTypeSpec>,
    /// Recurring patterns discovered in the data.
    pub patterns: Vec<LearnedPattern>,
    /// Cognitive tick configuration (may be adjusted by the Weaver).
    pub tick_config: CognitiveTickConfig,
    /// Data sources the model requires or recommends.
    pub sources_required: Vec<SourceRequirement>,
}

pub struct NodeTypeSpec {
    pub name: String,
    pub embedding_strategy: String,
    pub dimensions: usize,
}

pub struct EdgeTypeSpec {
    pub from: String,
    pub to: String,
    pub edge_type: CausalEdgeType,
    pub confidence: f64,
    pub sample_count: usize,
    pub note: Option<String>,
}

pub struct LearnedPattern {
    pub name: String,
    pub sequence: Vec<String>,
    pub confidence: f64,
    pub instances_found: usize,
}

pub struct SourceRequirement {
    pub source_type: String,
    pub required: bool,
    pub improves: Vec<String>,
}

// ── Confidence ────────────────────────────────────────────────────

/// Confidence assessment with gap analysis.
pub struct ConfidenceReport {
    /// Weighted mean confidence across all edges.
    pub overall: f64,
    /// Per-edge confidence breakdown.
    pub per_edge: HashMap<String, EdgeConfidence>,
    /// Identified gaps with suggestions.
    pub gaps: Vec<ConfidenceGap>,
    /// Actionable suggestions (may be auto-applied in Act mode).
    pub suggestions: Vec<ModelingSuggestion>,
}

pub struct EdgeConfidence {
    pub confidence: f64,
    pub sample_count: usize,
    pub coverage: f64,
    pub consistency: f64,
    pub status: ConfidenceStatus,
}

pub enum ConfidenceStatus {
    Strong,       // >= 0.80
    Developing,   // >= 0.50
    Weak,         // >= 0.30
    Insufficient, // < 0.30
}

pub struct ConfidenceGap {
    pub relationship: String,
    pub confidence: f64,
    pub reason: String,
    pub suggestion: ModelingSuggestion,
}

pub enum ModelingSuggestion {
    /// Request a new data source.
    AddSource {
        source_type: String,
        description: String,
        expected_improvement: f64,
    },
    /// Refine an edge type (e.g., "Causes" should be "Enables").
    RefineEdgeType {
        from: String,
        to: String,
        current: CausalEdgeType,
        suggested: CausalEdgeType,
        reason: String,
    },
    /// Split a coarse node category into finer ones.
    SplitCategory {
        category: String,
        into: Vec<String>,
        reason: String,
    },
    /// Merge categories that turned out to be the same thing.
    MergeCategories {
        categories: Vec<String>,
        into: String,
        reason: String,
    },
    /// Change HNSW dimensions for a node type.
    AdjustDimensions {
        node_type: String,
        current: usize,
        suggested: usize,
        reason: String,
    },
    /// Adjust the cognitive tick interval.
    ChangeTick {
        current_ms: u32,
        suggested_ms: u32,
        reason: String,
    },
}

// ── Data Sources ──────────────────────────────────────────────────

/// Data source that feeds the modeling loop.
pub enum DataSource {
    GitLog {
        path: PathBuf,
        branch: Option<String>,
        watch: bool,
        last_oid: Option<String>,
    },
    FileTree {
        root: PathBuf,
        patterns: Vec<String>,
        watch: bool,
    },
    CiPipeline {
        webhook_url: String,
    },
    IssueTracker {
        api_url: String,
        auth_env_var: Option<String>,
    },
    Documentation {
        root: PathBuf,
    },
    SparcPlan {
        path: PathBuf,
    },
    CustomStream {
        name: String,
        format: StreamFormat,
    },
}

pub enum StreamFormat {
    JsonLines,
    Csv,
    Custom(String),
}

// ── Model Export ──────────────────────────────────────────────────

/// Exportable model configuration for edge deployment.
///
/// Serializes to weave-model.json. Contains the learned model
/// but NOT the data — only the schema for how to model.
#[derive(Serialize, Deserialize)]
pub struct ExportedModel {
    pub version: u32,
    pub domain: String,
    pub created_by: String,
    pub confidence: f64,
    pub model: SerializableCausalModel,
    pub evolution_history: Vec<ModelVersion>,
}

#[derive(Serialize, Deserialize)]
pub struct ModelVersion {
    pub version: u32,
    pub change: String,
    pub confidence_before: f64,
    pub confidence_after: f64,
    pub timestamp: u64,
}

// ── Weaver Knowledge Base ─────────────────────────────────────────

/// Cross-domain learning accumulated from past modeling sessions.
///
/// The knowledge base stores in the kernel's ECC structures
/// (tagged Custom(0x41)). Over time, the Weaver develops domain
/// expertise: a Weaver that has modeled 50 Rust codebases knows
/// that CI output is the best source for commit->test edges.
pub struct WeaverKnowledgeBase {
    /// Root node in the kernel CausalGraph for KB entries.
    pub kb_root: NodeId,
    /// Reference to shared kernel structures.
    pub loom: Loom,
}

/// A learned strategy pattern for a domain type.
pub struct StrategyPattern {
    /// Domain characteristics that trigger this strategy.
    pub domain_characteristics: Vec<String>,
    /// Recommended data sources.
    pub recommended_sources: Vec<String>,
    /// Recommended edge types.
    pub recommended_edge_types: Vec<CausalEdgeType>,
    /// How confident are we in this strategy?
    pub confidence: f64,
    /// Which past sessions contributed to learning this.
    pub learned_from: Vec<String>,
}

// ── The Engine ────────────────────────────────────────────────────

/// The top-level SystemService.
///
/// Manages multiple ModelingSessions (one per domain) and the
/// shared WeaverKnowledgeBase. Processes cognitive tick events
/// by advancing each active session's modeling loop.
pub struct WeaverEngine {
    /// Active sessions keyed by domain name.
    sessions: HashMap<String, ModelingSession>,
    /// Cross-domain knowledge base.
    knowledge_base: WeaverKnowledgeBase,
    /// Arc references to kernel structures.
    causal: Arc<CausalGraph>,
    hnsw: Arc<HnswService>,
    crossrefs: Arc<CrossRefStore>,
    impulses: Arc<ImpulseQueue>,
    chain: Arc<ChainManager>,
    tree: Arc<TreeManager>,
    router: Arc<A2ARouter>,
}
```

### Crate Feature Flags

```toml
[features]
default = ["git", "embeddings"]
git = ["gix"]               # Git history ingestion via gitoxide
embeddings = []              # Embedding generation (trait-based, pluggable)
watch = ["notify"]           # Filesystem watching for Act mode
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum WeaverError {
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("domain already has active session: {0}")]
    SessionExists(String),
    #[error("configuration error: {0}")]
    ConfigError(String),
    #[error("ingestion failed: {source}")]
    IngestError { source: Box<dyn std::error::Error + Send + Sync> },
    #[error("confidence below threshold: {current:.2} < {required:.2}")]
    ConfidenceBelowThreshold { current: f64, required: f64 },
    #[error("stitch conflict: {0}")]
    StitchConflict(String),
    #[error("export error: {0}")]
    ExportError(String),
    #[error("source error: {0}")]
    SourceError(String),
    #[error("kernel service unavailable: {0}")]
    KernelUnavailable(String),
    #[error("io error: {source}")]
    Io { #[from] source: std::io::Error },
    #[error("json error: {source}")]
    Json { #[from] source: serde_json::Error },
}
```

---

## P -- Pseudocode

### 1. SystemService Implementation

```
impl SystemService for WeaverEngine:
    fn name() -> &str:
        "ecc.weaver"

    fn service_type() -> ServiceType:
        ServiceType::Extension

    async fn start():
        # Register in resource tree
        tree.insert("/kernel/services/weaver", ResourceKind::Service)

        # Initialize WeaverKnowledgeBase from persisted KB entries
        knowledge_base = WeaverKnowledgeBase::load_from(causal, hnsw, crossrefs)

        # Subscribe to CognitiveTick events
        tick_rx = subscribe_to_tick_events()

        # Subscribe to A2ARouter for CLI commands
        router.register("ecc.weaver", handle_message)

        # Record boot in chain
        chain.append("weaver.boot", json!({
            "kb_strategies": knowledge_base.strategy_count(),
            "persisted_sessions": 0,
        }))

    async fn stop():
        # Checkpoint all active sessions
        for session in sessions.values():
            checkpoint_session(session)
        # Flush impulse queue
        impulses.flush()

    async fn health_check() -> HealthStatus:
        for session in sessions.values():
            if session.confidence.overall < 0.1:
                return HealthStatus::Degraded("session {session.domain} has very low confidence")
        HealthStatus::Healthy
```

### 2. Cognitive Tick Handler

The Weaver does NOT run its own timer. It processes tick events from the kernel.

```
FUNCTION on_tick(tick_id: u64, budget_ms: f32):
    # Divide budget across active sessions
    per_session_budget = budget_ms / sessions.len()

    FOR session IN sessions.values_mut():
        start = Instant::now()

        # Phase 1: Ingest new data from sources
        for source in session.sources WHERE source.has_pending():
            events = source.poll(budget: per_session_budget * 0.4)
            for event in events:
                ingest_event(session, event)

        # Phase 2: Evaluate confidence (lightweight, runs every tick)
        IF tick_id % session.model.tick_config.evaluation_interval == 0:
            new_report = evaluate_confidence(session)

            IF new_report.overall != session.confidence.overall:
                # Record in meta-loom
                record_meta_event(session, "confidence_evaluation", json!({
                    "tick": tick_id,
                    "before": session.confidence.overall,
                    "after": new_report.overall,
                    "gaps": new_report.gaps.len(),
                }))
                session.confidence = new_report

        # Phase 3: Auto-adjust if enabled and confidence is below target
        IF session.auto_adjust AND session.confidence.overall < session.target_confidence:
            for suggestion in session.confidence.suggestions:
                MATCH suggestion:
                    RefineEdgeType(change):
                        apply_edge_refinement(session, change)
                        record_meta_event(session, "auto_adjust:refine_edge", json!(change))

                    SplitCategory(spec):
                        apply_category_split(session, spec)
                        record_meta_event(session, "auto_adjust:split_category", json!(spec))

                    MergeCategories(spec):
                        apply_category_merge(session, spec)
                        record_meta_event(session, "auto_adjust:merge_categories", json!(spec))

                    AdjustDimensions(spec):
                        apply_dimension_adjustment(session, spec)
                        record_meta_event(session, "auto_adjust:adjust_dimensions", json!(spec))

                    ChangeTick(spec):
                        apply_tick_change(session, spec)
                        record_meta_event(session, "auto_adjust:change_tick", json!(spec))

                    AddSource(spec):
                        # Cannot auto-add — emit impulse for operator
                        impulses.emit(
                            source_structure: StructureTag::Custom(0x40).as_u8(),
                            source_node: session.meta_root_universal_id(),
                            target_structure: StructureTag::CausalGraph.as_u8(),
                            impulse_type: ImpulseType::Custom(0x33),
                            payload: json!({"action": "source_request", "spec": spec}),
                            hlc_timestamp: now_hlc(),
                        )
                        record_meta_event(session, "source_request_emitted", json!(spec))

            # If any adjustments were applied, bump model version
            IF adjustments_applied > 0:
                old_version = session.model.version
                session.model.version += 1
                session.history.push(ModelVersion {
                    version: session.model.version,
                    change: describe_changes(applied_adjustments),
                    confidence_before: previous_confidence,
                    confidence_after: session.confidence.overall,
                    timestamp: now_hlc(),
                })

                # Record version bump in chain
                chain.append("weaver.model.version_bump", json!({
                    "domain": session.domain,
                    "old_version": old_version,
                    "new_version": session.model.version,
                    "confidence": session.confidence.overall,
                }))

                # Emit model-changed impulse
                impulses.emit(
                    source_structure: StructureTag::Custom(0x40).as_u8(),
                    source_node: session.meta_root_universal_id(),
                    target_structure: StructureTag::CausalGraph.as_u8(),
                    impulse_type: ImpulseType::Custom(0x32),
                    payload: json!({
                        "version": session.model.version,
                        "confidence": session.confidence.overall,
                    }),
                    hlc_timestamp: now_hlc(),
                )

                # Update WeaverKnowledgeBase with what we learned
                update_knowledge_base(session)

        # Budget enforcement
        IF start.elapsed() > per_session_budget:
            break  # yield remaining budget to next session
```

### 3. Event Ingestion

```
FUNCTION ingest_event(session: &mut ModelingSession, event: SourceEvent):
    loom = &session.loom
    model = &session.model

    # 1. Determine node type from event
    node_type = classify_event(event, model.node_types)

    # 2. Create causal graph node
    node_id = loom.causal.add_node(
        label: format!("{}:{}", node_type.name, event.id),
        metadata: json!({
            "type": node_type.name,
            "source": event.source_name,
            "timestamp": event.timestamp,
            "data": event.payload,
        })
    )

    # 3. Generate embedding using the node type's strategy
    embedding = generate_embedding(event.content, node_type.embedding_strategy)
    hnsw_id = format!("{}:{}", session.domain, event.id)
    loom.hnsw.insert(hnsw_id, embedding, json!({"node_id": node_id}))

    # 4. Infer edges using the model's edge types
    for edge_spec in model.edge_types WHERE edge_spec.from == node_type.name:
        # Search HNSW for candidate targets of this edge type
        candidates = loom.hnsw.search(
            &embedding,
            top_k: 10,
            filter: |meta| meta["type"] == edge_spec.to,
        )

        for candidate in candidates WHERE candidate.score > edge_spec.threshold():
            target_node_id = candidate.metadata["node_id"]
            loom.causal.link(
                node_id,
                target_node_id,
                edge_spec.edge_type.clone(),
                candidate.score,  # weight = similarity score
                now_hlc(),
                loom.chain.next_seq(),
            )

    # 5. CrossRef: HNSW entry <-> CausalGraph node
    loom.crossrefs.insert(CrossRef {
        source: hnsw_universal_id(hnsw_id),
        source_structure: StructureTag::HnswIndex,
        target: causal_node_universal_id(node_id),
        target_structure: StructureTag::CausalGraph,
        ref_type: CrossRefType::MemoryEncoded,
        created_at: now_hlc(),
        chain_seq: loom.chain.next_seq(),
    })

    # 6. Record in chain
    loom.chain.append("weaver.ingest.event", json!({
        "domain": session.domain,
        "event_id": event.id,
        "node_type": node_type.name,
        "node_id": node_id,
    }))
```

### 4. Confidence Evaluation

```
FUNCTION evaluate_confidence(session: &ModelingSession) -> ConfidenceReport:
    report = ConfidenceReport::empty()
    model = &session.model
    loom = &session.loom

    # For each edge type in the model, measure confidence
    for edge_spec in model.edge_types:
        key = format!("{}->{}:{}", edge_spec.from, edge_spec.to, edge_spec.edge_type)

        # Count: how many source nodes of this type have at least one edge?
        source_nodes = count_nodes_of_type(loom.causal, edge_spec.from)
        nodes_with_edge = count_nodes_with_outgoing_edge(
            loom.causal, edge_spec.from, edge_spec.edge_type
        )

        coverage = IF source_nodes > 0 THEN nodes_with_edge / source_nodes ELSE 0.0

        # Consistency: do edges of this type have consistent weights?
        weights = collect_edge_weights(loom.causal, edge_spec.edge_type)
        consistency = 1.0 - standard_deviation(weights)  # low variance = high consistency

        # Sample count
        sample_count = weights.len()

        # Composite confidence
        confidence = weighted_mean([
            (coverage, 0.4),
            (consistency, 0.3),
            (sample_size_factor(sample_count), 0.3),
        ])

        status = MATCH confidence:
            >= 0.80 => Strong
            >= 0.50 => Developing
            >= 0.30 => Weak
            _       => Insufficient

        report.per_edge.insert(key, EdgeConfidence {
            confidence, sample_count, coverage, consistency, status
        })

        # Identify gaps
        IF confidence < session.target_confidence:
            gap = identify_gap(edge_spec, coverage, sample_count, consistency)
            suggestion = generate_suggestion(gap, session, &knowledge_base)
            report.gaps.push(ConfidenceGap {
                relationship: key,
                confidence,
                reason: gap.reason,
                suggestion,
            })
            report.suggestions.push(suggestion)

    # Overall confidence: weighted mean of per-edge, weighted by goal relevance
    report.overall = weighted_mean_by_goal_relevance(
        report.per_edge,
        session.context,
    )

    RETURN report

FUNCTION generate_suggestion(gap, session, kb) -> ModelingSuggestion:
    # Check if the WeaverKnowledgeBase has a strategy for this gap
    strategy = kb.find_strategy(session.domain_characteristics(), gap.relationship)
    IF strategy.is_some():
        RETURN strategy.as_suggestion()

    # Otherwise, reason from the gap
    MATCH gap.type:
        LowCoverage:
            # Not enough source nodes have this edge
            # Likely need a new data source
            source_type = infer_needed_source(gap.edge_spec)
            RETURN AddSource {
                source_type,
                description: format!("{} data would establish {} causation", source_type, gap.relationship),
                expected_improvement: estimate_improvement(gap, source_type),
            }
        LowConsistency:
            # Edges exist but weights vary wildly
            # Likely the edge type is too coarse
            RETURN RefineEdgeType {
                from: gap.edge_spec.from,
                to: gap.edge_spec.to,
                current: gap.edge_spec.edge_type,
                suggested: infer_better_edge_type(gap),
                reason: "edge weights are inconsistent, suggesting the relationship is more nuanced",
            }
        LowSampleCount:
            # Not enough data points
            RETURN AddSource { ... }
```

### 5. Model Export

```
FUNCTION export_model(session: &ModelingSession) -> ExportedModel:
    # Serialize the learned model (NOT the data)
    exported = ExportedModel {
        version: session.model.version,
        domain: session.domain.clone(),
        created_by: "weaver-engine".to_string(),
        confidence: session.confidence.overall,
        model: session.model.to_serializable(),
        evolution_history: session.history.clone(),
    }

    # Record export in chain
    session.loom.chain.append("weaver.export", json!({
        "domain": session.domain,
        "version": session.model.version,
        "confidence": session.confidence.overall,
    }))

    # Record in meta-loom
    record_meta_event(session, "model_exported", json!({
        "version": session.model.version,
        "confidence": session.confidence.overall,
    }))

    RETURN exported
```

### 6. Forest Stitching

```
FUNCTION stitch_sessions(source: &ModelingSession, target: &ModelingSession) -> StitchResult:
    result = StitchResult::empty()

    # Phase 1: Cross-domain HNSW similarity search
    source_entries = source.loom.hnsw.all_entries()
    FOR entry IN source_entries:
        matches = target.loom.hnsw.search(&entry.embedding, top_k: 5)
        FOR match IN matches WHERE match.score > threshold:
            result.connections.push(CrossDomainConnection {
                source_domain: source.domain,
                target_domain: target.domain,
                source_node: entry.metadata["node_id"],
                target_node: match.metadata["node_id"],
                similarity: match.score,
            })

            # Create cross-forest CrossRef
            target.loom.crossrefs.insert(CrossRef {
                source: entry.universal_id,
                source_structure: StructureTag::HnswIndex,
                target: match.universal_id,
                target_structure: StructureTag::HnswIndex,
                ref_type: CrossRefType::Elaborates,
                created_at: now_hlc(),
                chain_seq: target.loom.chain.next_seq(),
            })

    # Phase 2: Resolve causal conflicts
    FOR connection IN result.connections:
        source_edges = source.loom.causal.get_forward_edges(connection.source_node)
        target_edges = target.loom.causal.get_forward_edges(connection.target_node)
        conflicts = find_contradictions(source_edges, target_edges)
        FOR conflict IN conflicts:
            resolution = resolve_by_confidence(conflict)
            apply_resolution(target.loom.causal, resolution)
            result.conflicts_resolved += 1

    # Phase 3: Chain provenance
    target.loom.chain.append("weaver.stitch", json!({
        "source": source.domain,
        "target": target.domain,
        "connections": result.connections.len(),
        "conflicts": result.conflicts_resolved,
    }))

    # Phase 4: Novelty impulses
    FOR connection IN result.connections:
        target.loom.impulses.emit(
            source_structure: StructureTag::HnswIndex.as_u8(),
            source_node: connection.source_universal_id,
            target_structure: StructureTag::CausalGraph.as_u8(),
            impulse_type: ImpulseType::NoveltyDetected,
            payload: json!({"stitch": true, "similarity": connection.similarity}),
            hlc_timestamp: now_hlc(),
        )

    RETURN result
```

### 7. Meta-Loom Recording

```
FUNCTION record_meta_event(session: &mut ModelingSession, event_type: &str, payload: Value):
    loom = &session.loom

    # Create a meta-loom causal node
    node_id = loom.causal.add_node(
        label: format!("meta:{}:{}:{}", session.domain, event_type, session.model.version),
        metadata: json!({
            "type": "weaver_meta",
            "structure_tag": 0x40,
            "domain": session.domain,
            "event_type": event_type,
            "model_version": session.model.version,
            "payload": payload,
            "timestamp": now_hlc(),
        })
    )

    # Link to previous meta event (Follows edge)
    IF let Some(prev_meta_node) = session.last_meta_node:
        loom.causal.link(
            prev_meta_node,
            node_id,
            CausalEdgeType::Follows,
            1.0,
            now_hlc(),
            loom.chain.next_seq(),
        )

    # Link to meta root (Enables edge — session enables this event)
    loom.causal.link(
        session.meta_root,
        node_id,
        CausalEdgeType::Enables,
        1.0,
        now_hlc(),
        loom.chain.next_seq(),
    )

    session.last_meta_node = Some(node_id)

    # Embed the meta event for cross-domain similarity
    embedding = generate_embedding(
        format!("{}: {}", event_type, serde_json::to_string(&payload).unwrap()),
        "meta_event",
    )
    loom.hnsw.insert(
        format!("meta:{}:{}", session.domain, node_id),
        embedding,
        json!({"node_id": node_id, "type": "weaver_meta"}),
    )
```

### 8. WeaverKnowledgeBase Update

```
FUNCTION update_knowledge_base(session: &ModelingSession):
    kb = &mut knowledge_base

    # Extract domain characteristics from session context + sources
    characteristics = extract_characteristics(session)
    # e.g., ["rust", "cargo", "github-actions", "ci_pipeline"]

    # Check if a similar strategy already exists
    existing = kb.find_similar_strategy(characteristics)

    IF existing.is_some():
        # Update existing strategy with new evidence
        strategy = existing.unwrap()
        strategy.confidence = weighted_update(
            strategy.confidence,
            session.confidence.overall,
        )
        strategy.learned_from.push(session.domain.clone())

        # Record in meta-loom
        record_meta_event(session, "kb_strategy_updated", json!({
            "characteristics": characteristics,
            "confidence": strategy.confidence,
        }))
    ELSE:
        # Create new strategy pattern
        strategy = StrategyPattern {
            domain_characteristics: characteristics,
            recommended_sources: session.sources.iter().map(|s| s.type_name()).collect(),
            recommended_edge_types: session.model.edge_types.iter()
                .filter(|e| e.confidence > 0.6)
                .map(|e| e.edge_type.clone())
                .collect(),
            confidence: session.confidence.overall,
            learned_from: vec![session.domain.clone()],
        }

        # Store in kernel CausalGraph with KB tag
        kb_node = loom.causal.add_node(
            label: format!("kb:strategy:{}", characteristics.join("+")),
            metadata: json!({
                "type": "weaver_kb_strategy",
                "structure_tag": 0x41,
                "strategy": strategy,
            })
        )

        # Link to KB root
        loom.causal.link(kb.kb_root, kb_node, CausalEdgeType::Enables, 1.0, now_hlc(), chain_seq)

        # Embed for similarity search
        embedding = generate_embedding(
            format!("domain strategy: {}", characteristics.join(", ")),
            "kb_strategy",
        )
        loom.hnsw.insert(format!("kb:strategy:{}", kb_node), embedding, json!({"node_id": kb_node}))

        record_meta_event(session, "kb_strategy_created", json!({
            "characteristics": characteristics,
            "confidence": strategy.confidence,
        }))
```

### 9. CLI Message Handler

```
FUNCTION handle_message(msg: KernelMessage) -> KernelMessage:
    MATCH msg.command:
        "session.start" =>
            domain = msg.payload["domain"]
            context = msg.payload["context"]
            goal = msg.payload["goal"]

            # Check if session exists
            IF sessions.contains_key(domain):
                RETURN error("session already exists, use session.resume")

            # Create Loom from shared kernel structures
            loom = Loom {
                domain, causal, hnsw, crossrefs, impulses, chain, tree,
            }

            # Query knowledge base for initial hypothesis
            initial_model = knowledge_base.hypothesize(context, domain_characteristics)

            # Create session
            session = ModelingSession::new(domain, context, goal, loom, initial_model)
            sessions.insert(domain, session)

            # Ingest initial sources
            for source in msg.payload["sources"]:
                add_source_to_session(session, source)

            RETURN ok(json!({ "domain": domain, "model_version": 1, "confidence": session.confidence.overall }))

        "session.resume" =>
            domain = msg.payload["domain"]
            session = sessions.get(domain)?
            RETURN ok(json!({ "domain": domain, "model_version": session.model.version, "confidence": session.confidence.overall }))

        "source.add" =>
            domain = msg.payload["domain"]
            session = sessions.get_mut(domain)?
            source = parse_source(msg.payload)
            add_source_to_session(session, source)
            RETURN ok(json!({ "source_added": true, "confidence": session.confidence.overall }))

        "confidence" =>
            domain = msg.payload["domain"]
            session = sessions.get(domain)?
            RETURN ok(serde_json::to_value(&session.confidence))

        "export" =>
            domain = msg.payload["domain"]
            session = sessions.get(domain)?
            min_confidence = msg.payload.get("min_confidence").unwrap_or(0.0)
            IF session.confidence.overall < min_confidence:
                RETURN error(format!("confidence {:.2} below threshold {:.2}", ...))
            exported = export_model(session)
            RETURN ok(serde_json::to_value(&exported))

        "stitch" =>
            source_domain = msg.payload["source"]
            target_domain = msg.payload["target"]
            source = sessions.get(source_domain)?
            target = sessions.get(target_domain)?
            result = stitch_sessions(source, target)
            RETURN ok(serde_json::to_value(&result))

        "meta" =>
            domain = msg.payload["domain"]
            session = sessions.get(domain)?
            trajectory = collect_meta_trajectory(session)
            RETURN ok(serde_json::to_value(&trajectory))

        _ => RETURN error("unknown command")
```

---

## A -- Architecture

### Crate Hierarchy

```
crates/
  clawft-kernel/           # ECC services, SystemService trait, A2ARouter
  exo-resource-tree/       # Resource tree data structure
  rvf-crypto/              # BLAKE3, Ed25519, SHAKE-256
  ecc-weaver/              # THIS CRATE
    src/
      lib.rs               # Re-exports, WeaverEngine SystemService impl
      engine.rs            # WeaverEngine: session management, tick handler
      session.rs           # ModelingSession: the modeling loop state
      model.rs             # CausalModel, NodeTypeSpec, EdgeTypeSpec, LearnedPattern
      confidence.rs        # ConfidenceReport, evaluation logic, gap analysis
      suggestion.rs        # ModelingSuggestion, auto-adjustment application
      export.rs            # ExportedModel, weave-model.json serialization
      source/
        mod.rs             # DataSource enum, SourceEvent trait
        git.rs             # Git history ingestion via gitoxide
        file_tree.rs       # Source tree ingestion
        ci.rs              # CI pipeline webhook receiver
        issue.rs           # Issue tracker API client
        docs.rs            # Documentation ingestion
        sparc.rs           # SPARC plan ingestion
        custom.rs          # Custom stream ingestion
      stitch.rs            # Forest stitching, cross-domain merge
      meta_loom.rs         # Meta-Loom recording, trajectory collection
      knowledge_base.rs    # WeaverKnowledgeBase, StrategyPattern, cross-domain learning
      embedding.rs         # EmbeddingProvider trait (pluggable backends)
      classify.rs          # Event classification, node type inference
      error.rs             # WeaverError
    Cargo.toml
```

### Dependency Graph

```
ecc-weaver
  |-- clawft-kernel (Arc<CausalGraph>, Arc<HnswService>, Arc<CrossRefStore>,
  |                   Arc<ImpulseQueue>, Arc<ChainManager>, Arc<TreeManager>,
  |                   SystemService, A2ARouter, AgentSupervisor, CognitiveTick)
  |-- exo-resource-tree (ResourceKind)
  |-- rvf-crypto (BLAKE3 for UniversalNodeId)
  |-- serde + serde_json (serialization)
  |-- thiserror (error types)
  |-- tracing (structured logging)
  |-- gix (optional: git history via gitoxide)
  |-- notify (optional: filesystem watching for Act mode)
  |-- glob (file pattern matching)
```

### Data Flow

```
Source Data (git, files, CI, issues, docs)
    |
    v
DataSource.poll() yields SourceEvents
    |
    v
ingest_event():
    |
    |--creates-->  CausalGraph nodes + edges (via Arc<CausalGraph>)
    |--generates-> Embeddings --> Arc<HnswService>
    |--creates-->  CrossRefs linking HNSW <-> Causal <-> Chain
    |--records-->  Arc<ChainManager> (provenance events)
    |
    v
on_tick() (driven by kernel CognitiveTick):
    |
    |--evaluates--> ConfidenceReport (gap analysis)
    |--adjusts----> CausalModel (if auto_adjust=true)
    |--records----> Meta-Loom (in kernel CausalGraph, tag 0x40)
    |--updates----> WeaverKnowledgeBase (in kernel CausalGraph, tag 0x41)
    |--emits------> Impulses (via Arc<ImpulseQueue>)
    |
    v
export_model():
    |
    v
weave-model.json (deployable config for edge devices)

CLI (weaver ecc *) --unix-socket--> A2ARouter --KernelMessage--> WeaverEngine
```

### SystemService Registration

```rust
#[async_trait]
impl SystemService for WeaverEngine {
    fn name(&self) -> &str { "ecc.weaver" }
    fn service_type(&self) -> ServiceType { ServiceType::Extension }

    async fn start(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Register in resource tree at /kernel/services/weaver
        self.tree.insert("/kernel/services/weaver", ResourceKind::Service)?;
        // Load KB from persisted state in CausalGraph
        self.knowledge_base.load()?;
        // Subscribe to tick events
        self.subscribe_ticks()?;
        // Register A2A handler for CLI commands
        self.router.register("ecc.weaver", Self::handle_message)?;
        // Record boot in chain
        self.chain.append("weaver.boot", json!({
            "kb_strategies": self.knowledge_base.strategy_count(),
        }))?;
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Checkpoint all sessions, flush impulses
        for session in self.sessions.values() {
            self.checkpoint_session(session)?;
        }
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        let degraded = self.sessions.values()
            .any(|s| s.confidence.overall < 0.1 && s.sources.len() > 0);
        if degraded { HealthStatus::Degraded } else { HealthStatus::Healthy }
    }
}
```

### Embedding Trait

```rust
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding for the given text using the named strategy.
    fn embed(&self, text: &str, strategy: &str) -> Result<Vec<f32>, WeaverError>;
    /// Return the output dimensions for a given strategy.
    fn dimensions(&self, strategy: &str) -> usize;
}
```

---

## R -- Refinement

### Performance Considerations

1. **Incremental ingestion**: When re-ingesting git history, only process commits
   newer than `last_oid` stored in the DataSource. Never rebuild the full history.

2. **Tick budget enforcement**: The Weaver must yield to the kernel's tick budget.
   If a session's ingestion or evaluation exceeds its allocation, it defers work
   to the next tick. This prevents the Weaver from starving other services.

3. **Batch HNSW inserts**: Group embedding insertions into batches of 100 to
   reduce lock contention on `Arc<HnswService>`.

4. **Lazy embedding**: If no embedding provider is configured, nodes are stored
   without embeddings and marked for later processing. The next tick fills them in.

5. **Confidence evaluation frequency**: Full evaluation runs every N ticks
   (configurable, default 10). Per-tick work is limited to ingestion + incremental
   edge updates.

6. **Meta-Loom pruning**: The meta-Loom itself grows over time. Apply decay to
   meta-Loom edges, pruning old modeling decisions that are no longer relevant.
   Keep model version boundaries as anchor nodes (never pruned).

7. **HNSW iteration for stitching**: Requires `HnswService` to expose an iteration
   API. If not available, maintain a manifest of entry IDs in the session.

### Security Considerations

1. **No secrets in embeddings**: Strip secrets, API keys, and credentials before
   embedding. Apply `.gitignore` rules plus common secret patterns.

2. **Chain provenance**: Every model version bump is chain-recorded. Tamper-evident
   audit trail of all modeling decisions.

3. **Source authentication**: External API sources (issue_tracker, ci_pipeline) use
   environment variable references for auth, never stored in config.

4. **CrossRef integrity**: Verify referenced nodes exist before creating CrossRefs.

5. **Export sanitization**: Exported `weave-model.json` contains no data, only
   structural schema. Verify no PII leaks into model descriptions.

### Testing Strategy

1. **Unit tests per module**:
   - `confidence.rs`: Scoring calculation, gap identification, status thresholds
   - `suggestion.rs`: Suggestion generation for each gap type
   - `model.rs`: Model versioning, serialization roundtrip
   - `export.rs`: ExportedModel JSON schema compliance
   - `source/git.rs`: Git history parsing on a test repository
   - `source/file_tree.rs`: File tree scanning on fixture directories
   - `stitch.rs`: Stitch two small sessions, verify CrossRef creation
   - `meta_loom.rs`: Meta event recording, trajectory collection
   - `knowledge_base.rs`: Strategy pattern storage, retrieval, similarity

2. **Integration tests**:
   - Full modeling loop: start session -> ingest -> evaluate -> adjust -> export
   - Cross-domain stitch on two test sessions
   - Tick handler: verify impulses emitted, meta-loom populated, budget respected
   - Knowledge base: model two similar domains, verify strategy reuse on third

3. **Property tests** (proptest):
   - Confidence is monotonically non-decreasing as data is added (ceteris paribus)
   - Model version is strictly increasing
   - Meta-loom node count >= model version count (every version has meta events)
   - Export confidence matches session confidence at time of export
   - Stitch is commutative (connections A<->B same regardless of direction)

4. **Benchmark tests**:
   - Ingestion throughput: events/second for git history
   - Confidence evaluation latency on 10K-node graph
   - Stitch latency for two 5K-entry HNSW indices
   - Tick budget compliance: Weaver never exceeds allocated budget

---

## C -- Completion

### Exit Criteria Checklist

- [ ] `WeaverEngine` implements `SystemService` and registers at kernel boot
- [ ] `WeaverEngine` registered in resource tree at `/kernel/services/weaver`
- [ ] `WeaverEngine` processes cognitive tick events (no independent timer)
- [ ] `WeaverEngine` handles A2ARouter messages for all CLI commands
- [ ] `ModelingSession` creates a Loom from shared kernel Arc references
- [ ] `ModelingSession` tracks a versioned `CausalModel` with evolution history
- [ ] `CausalModel` exports to `weave-model.json` via `ExportedModel`
- [ ] Confidence evaluation produces per-edge scores, gaps, and suggestions
- [ ] `ModelingSuggestion::AddSource` emits impulse for operator action
- [ ] Auto-adjustment applies `RefineEdgeType`, `SplitCategory`, `MergeCategories`, `AdjustDimensions`, `ChangeTick` automatically in Act mode
- [ ] Model version bumps recorded in chain with full provenance
- [ ] Meta-Loom records every modeling decision in kernel CausalGraph (tag 0x40)
- [ ] Meta-Loom trajectory can be collected and displayed
- [ ] `WeaverKnowledgeBase` stores strategy patterns in kernel CausalGraph (tag 0x41)
- [ ] `WeaverKnowledgeBase` provides initial hypotheses for new sessions based on domain similarity
- [ ] Cross-domain learning: strategy confidence updates when reused
- [ ] Forest stitching creates cross-forest CrossRefs for semantically similar nodes
- [ ] Stitching resolves causal conflicts by confidence
- [ ] All data sources (`GitLog`, `FileTree`, `CiPipeline`, `IssueTracker`, `Documentation`, `SparcPlan`, `CustomStream`) have working ingestion
- [ ] Watch mode: sources with `watch: true` yield events on each tick
- [ ] Impulses emitted: `BeliefUpdate`, `CoherenceAlert`, `NoveltyDetected`, `EdgeConfirmed`, `Custom(0x32)` model bump, `Custom(0x33)` source request
- [ ] Tick budget enforcement: Weaver never exceeds allocated budget
- [ ] `EmbeddingProvider` trait is pluggable, no hardcoded provider
- [ ] `scripts/build.sh check` passes with no warnings
- [ ] `scripts/build.sh clippy` passes
- [ ] `scripts/build.sh test` passes all unit, integration, and property tests
- [ ] Benchmark tests establish baseline throughput numbers
- [ ] The WEAVER.md skill file CLI commands map to working A2ARouter handlers
