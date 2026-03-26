//! WeaverEngine: ECC-powered codebase modeling service (K3c-G1).
//!
//! The WeaverEngine is a [`SystemService`] that drives the ECC cognitive
//! substrate to model real-world data sources (git logs, file trees, CI
//! pipelines, documentation). It manages [`ModelingSession`]s, evaluates
//! confidence via the causal graph, and records its own decisions in the
//! Meta-Loom for self-improvement tracking.
//!
//! This module requires the `ecc` feature.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::causal::{CausalEdgeType, CausalGraph};
use crate::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use crate::health::HealthStatus;
use crate::hnsw_service::HnswService;
use crate::impulse::{ImpulseQueue, ImpulseType};
use crate::service::{ServiceType, SystemService};

// ---------------------------------------------------------------------------
// WeaverCommand (IPC messages from CLI / agents)
// ---------------------------------------------------------------------------

/// Commands sent to the WeaverEngine via IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeaverCommand {
    /// Start a new modeling session.
    SessionStart {
        domain: String,
        git_path: Option<PathBuf>,
        context: Option<String>,
        goal: Option<String>,
    },
    /// Resume an existing session.
    SessionResume { domain: String },
    /// Stop a session.
    SessionStop { domain: String },
    /// Watch session progress (streaming).
    SessionWatch { domain: String },
    /// Add a data source to a session.
    SourceAdd {
        domain: String,
        source_type: String,
        root: Option<PathBuf>,
        watch: bool,
    },
    /// List sources for a session.
    SourceList { domain: String },
    /// Query confidence.
    Confidence {
        domain: String,
        edge: Option<String>,
        verbose: bool,
    },
    /// Export model.
    Export {
        domain: String,
        min_confidence: f64,
        output: PathBuf,
    },
    /// Import model.
    Import {
        domain: String,
        input: PathBuf,
    },
    /// Query meta-loom status.
    MetaStatus { domain: String },
    /// List learned strategies.
    MetaStrategies,
    /// Export knowledge base.
    MetaExportKb { output: PathBuf },
    /// Stitch two domains.
    Stitch {
        source: String,
        target: String,
        output: String,
    },
}

// ---------------------------------------------------------------------------
// WeaverResponse
// ---------------------------------------------------------------------------

/// Responses from the WeaverEngine to CLI / agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeaverResponse {
    /// Session started successfully.
    SessionStarted { domain: String, session_id: String },
    /// Session stopped.
    SessionStopped { domain: String },
    /// Session resumed.
    SessionResumed { domain: String },
    /// Confidence report.
    ConfidenceReport(ConfidenceReport),
    /// Source added.
    SourceAdded { domain: String, source_type: String },
    /// Sources listed.
    Sources(Vec<String>),
    /// Model exported.
    Exported { path: PathBuf, edges: usize },
    /// Model imported.
    Imported { domain: String },
    /// Learned strategies.
    Strategies(Vec<StrategyPattern>),
    /// Knowledge base exported.
    KbExported { path: PathBuf },
    /// Error.
    Error(String),
}

// ---------------------------------------------------------------------------
// DataSource
// ---------------------------------------------------------------------------

/// A data source that can be ingested by the WeaverEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    /// Git commit history.
    GitLog { path: PathBuf },
    /// File system tree.
    FileTree { root: PathBuf },
    /// CI pipeline events.
    CiPipeline { url: String },
    /// Issue tracker feed.
    IssueTracker { url: String },
    /// Documentation corpus.
    Documentation { root: PathBuf },
    /// User-defined stream.
    CustomStream { name: String },
}

impl DataSource {
    /// Human-readable type name.
    pub fn type_name(&self) -> &str {
        match self {
            Self::GitLog { .. } => "git_log",
            Self::FileTree { .. } => "file_tree",
            Self::CiPipeline { .. } => "ci_pipeline",
            Self::IssueTracker { .. } => "issue_tracker",
            Self::Documentation { .. } => "documentation",
            Self::CustomStream { .. } => "custom_stream",
        }
    }
}

// ---------------------------------------------------------------------------
// ModelingSession
// ---------------------------------------------------------------------------

/// An active or suspended modeling session for a single domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelingSession {
    /// Unique session identifier.
    pub id: String,
    /// Domain name (e.g., project name).
    pub domain: String,
    /// When the session was started.
    pub started_at: DateTime<Utc>,
    /// Current overall confidence (0.0 .. 1.0).
    pub confidence: f64,
    /// Identified confidence gaps.
    pub gaps: Vec<ConfidenceGap>,
    /// Data sources that have been ingested.
    pub sources_ingested: Vec<String>,
    /// Number of cognitive ticks processed.
    pub tick_count: u64,
    /// Remaining budget for this session.
    pub budget_remaining_ms: u64,
    /// Whether the session is currently active.
    pub active: bool,
    /// Arbitrary metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// ConfidenceGap / ConfidenceReport
// ---------------------------------------------------------------------------

/// A gap in the model's confidence for a specific domain area.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceGap {
    /// Area name.
    pub domain: String,
    /// Current confidence level.
    pub current_confidence: f64,
    /// Target confidence level.
    pub target_confidence: f64,
    /// Suggested sources to improve confidence.
    pub suggested_sources: Vec<String>,
}

/// Full confidence report for a modeling session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceReport {
    /// Overall confidence.
    pub overall: f64,
    /// Per-domain gap analysis.
    pub gaps: Vec<ConfidenceGap>,
    /// Modeling suggestions.
    pub suggestions: Vec<ModelingSuggestion>,
}

/// Suggestions for improving model quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelingSuggestion {
    /// Add a new data source.
    AddSource { source_type: String, reason: String },
    /// Refine an edge type relationship.
    RefineEdgeType { from: String, to: String },
    /// Split a category into subcategories.
    SplitCategory { category: String },
    /// Increase observation window.
    ExtendObservation { domain: String },
}

// ---------------------------------------------------------------------------
// ExportedModel (K3c-G4)
// ---------------------------------------------------------------------------

/// Serialized model for edge deployment or offline analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedModel {
    /// Schema version.
    pub version: String,
    /// Domain this model was built for.
    pub domain: String,
    /// When the export was created.
    pub exported_at: DateTime<Utc>,
    /// Overall confidence at export time.
    pub confidence: f64,
    /// Node type specifications.
    pub node_types: Vec<NodeTypeSpec>,
    /// Edge type specifications.
    pub edge_types: Vec<EdgeTypeSpec>,
    /// Exported causal nodes.
    pub causal_nodes: Vec<ExportedCausalNode>,
    /// Exported causal edges.
    pub causal_edges: Vec<ExportedCausalEdge>,
    /// Arbitrary metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Node type specification in an exported model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeSpec {
    /// Type name.
    pub name: String,
    /// Embedding strategy identifier.
    pub embedding_strategy: String,
    /// Vector dimensions.
    pub dimensions: usize,
}

/// Edge type specification in an exported model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTypeSpec {
    /// Source node type.
    pub from_type: String,
    /// Target node type.
    pub to_type: String,
    /// Edge type name.
    pub edge_type: String,
    /// Confidence for this edge type.
    pub confidence: f64,
}

/// Exported causal node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedCausalNode {
    /// Node label.
    pub label: String,
    /// Node metadata.
    pub metadata: serde_json::Value,
}

/// Exported causal edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedCausalEdge {
    /// Source node label.
    pub source_label: String,
    /// Target node label.
    pub target_label: String,
    /// Edge type.
    pub edge_type: String,
    /// Edge weight.
    pub weight: f32,
}

// ---------------------------------------------------------------------------
// MetaLoomEvent (K3c-G5)
// ---------------------------------------------------------------------------

/// Records a Weaver modeling decision in the Meta-Loom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaLoomEvent {
    /// Domain of the active session.
    pub session_domain: String,
    /// Type of modeling decision.
    pub decision_type: MetaDecisionType,
    /// Confidence before the decision.
    pub confidence_before: f64,
    /// Confidence after (filled in by next tick).
    pub confidence_after: Option<f64>,
    /// Human-readable rationale.
    pub rationale: String,
    /// When the decision was made.
    pub timestamp: DateTime<Utc>,
}

/// Classification of meta-loom decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetaDecisionType {
    /// A new data source was added.
    SourceAdded { source_type: String },
    /// A new edge type relationship was created.
    EdgeTypeCreated {
        from: String,
        to: String,
        edge_type: String,
    },
    /// An edge type was removed.
    EdgeTypeRemoved { from: String, to: String },
    /// Embedding strategy changed for a node type.
    EmbeddingStrategyChanged {
        node_type: String,
        old: String,
        new: String,
    },
    /// Tick interval was adjusted.
    TickIntervalAdjusted { old_ms: u64, new_ms: u64 },
    /// Model version was bumped.
    ModelVersionBumped { from: u32, to: u32 },
    /// A new strategy was learned.
    StrategyLearned { pattern: String },
}

// ---------------------------------------------------------------------------
// StrategyPattern / WeaverKnowledgeBase
// ---------------------------------------------------------------------------

/// A learned modeling strategy from cross-domain experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPattern {
    /// Decision type that led to improvement.
    pub decision_type: String,
    /// Domain context where it was learned.
    pub context: String,
    /// Confidence improvement observed.
    pub improvement: f64,
    /// When the strategy was learned.
    pub timestamp: DateTime<Utc>,
}

/// Cross-domain knowledge base that accumulates successful strategies.
pub struct WeaverKnowledgeBase {
    /// Learned strategies.
    strategies: RwLock<Vec<StrategyPattern>>,
    /// Strategy count.
    strategy_count: AtomicU64,
}

impl Default for WeaverKnowledgeBase {
    fn default() -> Self {
        Self {
            strategies: RwLock::new(Vec::new()),
            strategy_count: AtomicU64::new(0),
        }
    }
}

impl WeaverKnowledgeBase {
    /// Create a new, empty knowledge base.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful strategy.
    pub fn record_strategy(&self, pattern: StrategyPattern) {
        if let Ok(mut strategies) = self.strategies.write() {
            strategies.push(pattern);
            self.strategy_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// List all learned strategies.
    pub fn list_strategies(&self) -> Vec<StrategyPattern> {
        self.strategies
            .read()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Find strategies relevant to a given domain (simple substring match).
    pub fn strategies_for(&self, domain: &str) -> Vec<StrategyPattern> {
        self.strategies
            .read()
            .map(|all| {
                all.iter()
                    .filter(|s| {
                        s.context.contains(domain)
                            || domain.contains(&s.context)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Export the full knowledge base as JSON.
    pub fn export(&self) -> serde_json::Value {
        serde_json::to_value(self.list_strategies()).unwrap_or_default()
    }

    /// Total number of learned strategies.
    pub fn count(&self) -> u64 {
        self.strategy_count.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// TickResult
// ---------------------------------------------------------------------------

/// Outcome of a single cognitive tick for the WeaverEngine.
#[derive(Debug)]
pub enum TickResult {
    /// No active session; engine is idle.
    Idle,
    /// Budget exhausted before work could be done.
    BudgetExhausted,
    /// Progress was made.
    Progress {
        /// Current overall confidence.
        confidence: f64,
        /// Number of remaining gaps.
        gaps_remaining: usize,
    },
}

// ---------------------------------------------------------------------------
// WeaverError
// ---------------------------------------------------------------------------

/// Errors produced by the WeaverEngine.
#[derive(Debug)]
pub enum WeaverError {
    /// I/O error reading a file.
    Io(std::io::Error),
    /// JSON parsing error.
    Json(serde_json::Error),
    /// Domain logic error.
    Domain(String),
}

impl fmt::Display for WeaverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "weaver I/O error: {e}"),
            Self::Json(e) => write!(f, "weaver JSON error: {e}"),
            Self::Domain(msg) => write!(f, "weaver error: {msg}"),
        }
    }
}

impl std::error::Error for WeaverError {}

impl From<std::io::Error> for WeaverError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for WeaverError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

// ---------------------------------------------------------------------------
// IngestResult
// ---------------------------------------------------------------------------

/// Statistics from ingesting a graph file into the WeaverEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    /// Number of causal graph nodes created.
    pub nodes_added: usize,
    /// Number of causal graph edges created.
    pub edges_added: usize,
    /// Number of HNSW embeddings created.
    pub embeddings_created: usize,
    /// Source identifier (e.g., "git-history", "module-deps").
    pub source: String,
}

// ---------------------------------------------------------------------------
// GitPoller (incremental git change detection)
// ---------------------------------------------------------------------------

/// Incremental git polling state — detects new commits since last check.
pub struct GitPoller {
    /// Repository path.
    repo_path: PathBuf,
    /// Last known commit hash.
    last_known_hash: Option<String>,
    /// Branch to poll.
    branch: String,
    /// Polling enabled flag.
    enabled: bool,
}

impl GitPoller {
    /// Create a new poller for the given repository path and branch.
    pub fn new(repo_path: PathBuf, branch: String) -> Self {
        Self {
            repo_path,
            last_known_hash: None,
            branch,
            enabled: true,
        }
    }

    /// Check for new commits since last poll.
    /// Returns the number of new commits found (0 if none or on error).
    pub fn poll(&mut self) -> usize {
        if !self.enabled {
            return 0;
        }

        let repo_str = self.repo_path.to_str().unwrap_or(".");
        let output = std::process::Command::new("git")
            .args(["-C", repo_str, "rev-parse", "HEAD"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let current_hash = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if self.last_known_hash.as_deref() == Some(&current_hash) {
                    return 0;
                }

                let count = if let Some(ref last) = self.last_known_hash {
                    let count_output = std::process::Command::new("git")
                        .args([
                            "-C",
                            repo_str,
                            "rev-list",
                            "--count",
                            &format!("{}..{}", last, current_hash),
                        ])
                        .output();
                    match count_output {
                        Ok(o) if o.status.success() => {
                            String::from_utf8_lossy(&o.stdout)
                                .trim()
                                .parse()
                                .unwrap_or(1)
                        }
                        _ => 1,
                    }
                } else {
                    1 // First poll — at least 1 commit exists
                };

                self.last_known_hash = Some(current_hash);
                count
            }
            _ => 0,
        }
    }

    /// Get the last known commit hash.
    pub fn last_hash(&self) -> Option<&str> {
        self.last_known_hash.as_deref()
    }

    /// Get the branch being polled.
    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// Whether polling is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable polling.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

// ---------------------------------------------------------------------------
// FileWatcher (mtime-based change detection)
// ---------------------------------------------------------------------------

/// Simple file change detector using modification timestamps.
///
/// Avoids the `notify` crate dependency by comparing cached mtimes on each
/// poll call. Only watches files that have been explicitly registered.
pub struct FileWatcher {
    /// Watched paths with their last known mtime.
    watched: HashMap<PathBuf, SystemTime>,
    /// Root directory to scan for initial registration.
    root: PathBuf,
    /// File patterns to match (e.g., `"*.rs"`, `"Cargo.toml"`).
    patterns: Vec<String>,
    /// Enabled flag.
    enabled: bool,
}

impl FileWatcher {
    /// Create a new file watcher for the given root and patterns.
    pub fn new(root: PathBuf, patterns: Vec<String>) -> Self {
        Self {
            watched: HashMap::new(),
            root,
            patterns,
            enabled: true,
        }
    }

    /// Scan watched files for mtime changes since last check.
    /// Returns paths of files that changed or were deleted.
    pub fn poll_changes(&mut self) -> Vec<PathBuf> {
        if !self.enabled {
            return vec![];
        }

        let mut changed = Vec::new();
        let entries: Vec<PathBuf> = self.watched.keys().cloned().collect();

        for path in entries {
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Ok(mtime) = metadata.modified() {
                    if let Some(last_mtime) = self.watched.get(&path) {
                        if mtime > *last_mtime {
                            changed.push(path.clone());
                            self.watched.insert(path, mtime);
                        }
                    }
                }
            } else {
                // File deleted
                changed.push(path.clone());
                self.watched.remove(&path);
            }
        }

        changed
    }

    /// Register a single file to watch.
    pub fn watch(&mut self, path: PathBuf) {
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(mtime) = metadata.modified() {
                self.watched.insert(path, mtime);
            }
        }
    }

    /// Register all files matching patterns in the root directory (non-recursive).
    pub fn watch_directory(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let matches = self.patterns.iter().any(|p| {
                        if p.starts_with("*.") {
                            name.ends_with(&p[1..])
                        } else {
                            name == p
                        }
                    });
                    if matches {
                        self.watch(path);
                    }
                }
            }
        }
    }

    /// Number of watched files.
    pub fn watched_count(&self) -> usize {
        self.watched.len()
    }

    /// Whether file watching is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable file watching.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

// ---------------------------------------------------------------------------
// CognitiveTickResult
// ---------------------------------------------------------------------------

/// Detailed outcome of a single cognitive tick processed by the WeaverEngine.
///
/// Complements the existing [`TickResult`] enum with per-tick metrics for the
/// CognitiveTick integration (git polling, file watching, ingestion progress).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitiveTickResult {
    /// Which tick number this result corresponds to.
    pub tick_number: u64,
    /// Actual wall-clock time consumed by this tick (ms).
    pub elapsed_ms: u32,
    /// Budget allocated for this tick (ms).
    pub budget_ms: u32,
    /// Number of new git commits detected during this tick.
    pub git_commits_found: usize,
    /// Number of source files that changed since last tick.
    pub files_changed: usize,
    /// Number of pending nodes processed during ingestion phase.
    pub nodes_processed: usize,
    /// Whether the confidence report was recomputed this tick.
    pub confidence_updated: bool,
    /// Whether the tick completed within its budget.
    pub within_budget: bool,
}

// ---------------------------------------------------------------------------
// WeaverEngine
// ---------------------------------------------------------------------------

/// ECC-powered codebase modeling service.
///
/// Manages modeling sessions, drives confidence evaluation via the
/// causal graph, and records modeling decisions in the Meta-Loom.
pub struct WeaverEngine {
    /// Active modeling sessions keyed by domain.
    sessions: RwLock<HashMap<String, ModelingSession>>,
    /// Cross-domain knowledge base.
    knowledge_base: Arc<WeaverKnowledgeBase>,
    /// Embedding provider for vectorization.
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// Causal graph reference.
    causal_graph: Arc<CausalGraph>,
    /// HNSW service reference.
    #[allow(dead_code)]
    hnsw: Arc<HnswService>,
    /// Impulse queue for emitting meta-loom signals.
    impulse_queue: Option<Arc<ImpulseQueue>>,
    /// Meta-loom event history per domain.
    meta_loom_events: RwLock<HashMap<String, Vec<MetaLoomEvent>>>,
    /// Total ticks processed across all sessions.
    tick_count: AtomicU64,
    /// Git poller for incremental commit detection.
    git_poller: Option<GitPoller>,
    /// File watcher for source file change detection.
    file_watcher: Option<FileWatcher>,
    /// Ticks since the last confidence recomputation.
    ticks_since_confidence_update: u64,
    /// Last computed confidence report (cached).
    last_confidence: Option<ConfidenceReport>,
}

impl WeaverEngine {
    /// Create a new WeaverEngine with the given dependencies.
    pub fn new(
        causal_graph: Arc<CausalGraph>,
        hnsw: Arc<HnswService>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            knowledge_base: Arc::new(WeaverKnowledgeBase::new()),
            embedding_provider,
            causal_graph,
            hnsw,
            impulse_queue: None,
            meta_loom_events: RwLock::new(HashMap::new()),
            tick_count: AtomicU64::new(0),
            git_poller: None,
            file_watcher: None,
            ticks_since_confidence_update: 0,
            last_confidence: None,
        }
    }

    /// Create a WeaverEngine with a mock embedding provider (for tests).
    pub fn new_with_mock(
        causal_graph: Arc<CausalGraph>,
        hnsw: Arc<HnswService>,
    ) -> Self {
        Self::new(
            causal_graph,
            hnsw,
            Arc::new(MockEmbeddingProvider::new(64)),
        )
    }

    /// Set the impulse queue for emitting meta-loom signals.
    pub fn set_impulse_queue(&mut self, queue: Arc<ImpulseQueue>) {
        self.impulse_queue = Some(queue);
    }

    /// Get a reference to the knowledge base.
    pub fn knowledge_base(&self) -> &Arc<WeaverKnowledgeBase> {
        &self.knowledge_base
    }

    /// Get a reference to the embedding provider.
    pub fn embedding_provider(&self) -> &Arc<dyn EmbeddingProvider> {
        &self.embedding_provider
    }

    /// Get a reference to the causal graph.
    pub fn causal_graph(&self) -> &Arc<CausalGraph> {
        &self.causal_graph
    }

    /// Get a reference to the HNSW service.
    pub fn hnsw(&self) -> &Arc<HnswService> {
        &self.hnsw
    }

    // ── Graph file ingestion ──────────────────────────────────────

    /// Ingest a graph JSON file (git-history, module-deps, or decisions).
    ///
    /// Reads a `.weftos/graph/*.json` file, creates causal graph nodes for
    /// each entry, creates edges between related nodes, and inserts
    /// embeddings into the HNSW index for each node's text representation.
    pub fn ingest_graph_file(&self, path: &Path) -> Result<IngestResult, WeaverError> {
        let data = std::fs::read_to_string(path)?;
        let graph: serde_json::Value = serde_json::from_str(&data)?;

        let source = graph["source"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let empty_vec = vec![];
        let nodes = graph["nodes"].as_array().unwrap_or(&empty_vec);
        let edges = graph["edges"].as_array().unwrap_or(&empty_vec);

        let mut nodes_added = 0usize;
        let mut edges_added = 0usize;
        let mut embeddings_created = 0usize;

        // Map from JSON node id (string) to causal graph NodeId.
        let mut id_map: HashMap<String, u64> = HashMap::new();

        // Phase 1: Create nodes.
        for node in nodes {
            let node_id_str = node["id"].as_str().unwrap_or("").to_string();
            if node_id_str.is_empty() {
                continue;
            }

            let label = if let Some(title) = node["title"].as_str() {
                format!("{source}/{node_id_str}: {title}")
            } else if let Some(subject) = node["subject"].as_str() {
                format!("{source}/{node_id_str}: {subject}")
            } else {
                format!("{source}/{node_id_str}")
            };

            let causal_id = self.causal_graph.add_node(
                label.clone(),
                node.clone(),
            );
            id_map.insert(node_id_str.clone(), causal_id);
            nodes_added += 1;

            // Create an HNSW embedding for the node's text.
            let embed_text = Self::node_to_embed_text(node, &source);
            if !embed_text.is_empty() {
                // Use synchronous hash-embed for ingestion (avoiding async).
                let embed_vec = self.sync_embed(&embed_text);
                self.hnsw.insert(
                    format!("{source}/{}", node_id_str),
                    embed_vec,
                    node.clone(),
                );
                embeddings_created += 1;
            }
        }

        // Phase 2: Create edges.
        for edge in edges {
            let from_str = edge["from"].as_str().unwrap_or("");
            let to_str = edge["to"].as_str().unwrap_or("");
            let edge_type_str = edge["type"].as_str().unwrap_or("Correlates");
            let weight = edge["weight"].as_f64().unwrap_or(1.0) as f32;

            let from_id = id_map.get(from_str).copied();
            let to_id = id_map.get(to_str).copied();

            if let (Some(src), Some(tgt)) = (from_id, to_id) {
                let edge_type = Self::parse_edge_type(edge_type_str);
                let linked = self.causal_graph.link(
                    src, tgt, edge_type, weight, 0, 0,
                );
                if linked {
                    edges_added += 1;
                }
            }
        }

        info!(
            source = %source,
            nodes_added,
            edges_added,
            embeddings_created,
            "graph file ingested"
        );

        Ok(IngestResult {
            nodes_added,
            edges_added,
            embeddings_created,
            source,
        })
    }

    /// Convert a graph node's fields into embeddable text.
    fn node_to_embed_text(node: &serde_json::Value, source: &str) -> String {
        match source {
            "git-history" => {
                let subject = node["subject"].as_str().unwrap_or("");
                let author = node["author"].as_str().unwrap_or("");
                let files = node["files"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                format!("commit by {author}: {subject} files: {files}")
            }
            "module-dependencies" => {
                let id = node["id"].as_str().unwrap_or("");
                let deps = node["dependencies"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                let lines = node["lines"].as_u64().unwrap_or(0);
                format!("module {id} ({lines} lines) depends on: {deps}")
            }
            "decisions-and-phases" => {
                let title = node["title"].as_str().unwrap_or("");
                let rationale = node["rationale"].as_str().unwrap_or("");
                let panel = node["panel"].as_str().unwrap_or("");
                format!("decision ({panel}): {title} — {rationale}")
            }
            _ => {
                // Fallback: serialize the whole node.
                serde_json::to_string(node).unwrap_or_default()
            }
        }
    }

    /// Parse an edge type string to a CausalEdgeType.
    fn parse_edge_type(s: &str) -> CausalEdgeType {
        match s {
            "Causes" => CausalEdgeType::Causes,
            "Inhibits" => CausalEdgeType::Inhibits,
            "Correlates" => CausalEdgeType::Correlates,
            "Enables" => CausalEdgeType::Enables,
            "Follows" => CausalEdgeType::Follows,
            "Contradicts" => CausalEdgeType::Contradicts,
            "TriggeredBy" => CausalEdgeType::TriggeredBy,
            "EvidenceFor" => CausalEdgeType::EvidenceFor,
            _ => CausalEdgeType::Correlates,
        }
    }

    /// Synchronous embedding using the mock fallback (for ingestion loops).
    ///
    /// This avoids the need for async in the ingestion path. The mock
    /// provider's `hash_embed` is deterministic and instant.
    fn sync_embed(&self, text: &str) -> Vec<f32> {
        use sha2::{Digest, Sha256};
        let dims = self.embedding_provider.dimensions();
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();
        let mut vec = Vec::with_capacity(dims);
        for i in 0..dims {
            let byte = hash[i % 32];
            vec.push((byte as f32 / 128.0) - 1.0);
        }
        vec
    }

    // ── Confidence scoring from graph data ────────────────────────

    /// Compute confidence based on graph coverage.
    ///
    /// Examines the causal graph to determine what fraction of nodes have
    /// edges (both incoming and outgoing), the edge density, and identifies
    /// orphan nodes that lack causal connections.
    pub fn compute_confidence(&self) -> ConfidenceReport {
        let node_count = self.causal_graph.node_count() as usize;
        let edge_count = self.causal_graph.edge_count() as usize;

        if node_count == 0 {
            return ConfidenceReport {
                overall: 0.0,
                gaps: vec![ConfidenceGap {
                    domain: "graph".to_string(),
                    current_confidence: 0.0,
                    target_confidence: 0.8,
                    suggested_sources: vec![
                        "git_log".into(),
                        "module_deps".into(),
                        "decisions".into(),
                    ],
                }],
                suggestions: vec![ModelingSuggestion::AddSource {
                    source_type: "git_log".to_string(),
                    reason: "No graph data ingested yet".to_string(),
                }],
            };
        }

        // Edge density: ratio of actual edges to maximum possible.
        let max_edges = if node_count > 1 {
            node_count * (node_count - 1)
        } else {
            1
        };
        let edge_density = (edge_count as f64 / max_edges as f64).min(1.0);

        // Node connectivity: fraction of nodes that have at least one edge.
        // We sample by checking forward + reverse edges for each node id up to
        // the known count (sequential IDs starting from 1).
        let mut connected_nodes = 0usize;
        let mut orphan_labels: Vec<String> = Vec::new();
        let next_id = self.causal_graph.node_count() + 1;
        // Iterate over plausible node IDs. The CausalGraph allocates IDs
        // sequentially starting at 1 so scanning 1..next_id covers all.
        for nid in 1..next_id {
            if self.causal_graph.get_node(nid).is_some() {
                let fwd = self.causal_graph.get_forward_edges(nid);
                let rev = self.causal_graph.get_reverse_edges(nid);
                if !fwd.is_empty() || !rev.is_empty() {
                    connected_nodes += 1;
                } else if let Some(node) = self.causal_graph.get_node(nid) {
                    orphan_labels.push(node.label.clone());
                }
            }
        }

        let connectivity = if node_count > 0 {
            connected_nodes as f64 / node_count as f64
        } else {
            0.0
        };

        // Composite confidence: weighted average of components.
        // - Connectivity (40%): nodes with edges / total nodes
        // - Edge density (20%): capped contribution from edge density
        // - Node volume (20%): diminishing returns above 100 nodes
        // - Source diversity (20%): number of distinct source prefixes
        let volume_score = (node_count as f64 / 100.0).min(1.0);
        let density_capped = (edge_density * 50.0).min(1.0); // amplify sparse graphs

        let overall = (connectivity * 0.40
            + density_capped * 0.20
            + volume_score * 0.20
            + self.source_diversity_score() * 0.20)
            .min(1.0);

        // Build gaps.
        let mut gaps = Vec::new();
        if connectivity < 0.7 {
            gaps.push(ConfidenceGap {
                domain: "node_connectivity".to_string(),
                current_confidence: connectivity,
                target_confidence: 0.7,
                suggested_sources: vec!["module_deps".into(), "git_log".into()],
            });
        }
        if volume_score < 0.5 {
            gaps.push(ConfidenceGap {
                domain: "data_volume".to_string(),
                current_confidence: volume_score,
                target_confidence: 0.5,
                suggested_sources: vec!["git_log".into(), "file_tree".into()],
            });
        }

        // Suggestions from orphan nodes.
        let mut suggestions = Vec::new();
        if !orphan_labels.is_empty() {
            let sample: Vec<_> = orphan_labels.iter().take(5).cloned().collect();
            suggestions.push(ModelingSuggestion::AddSource {
                source_type: "causal_edges".to_string(),
                reason: format!(
                    "{} orphan nodes without edges (e.g., {})",
                    orphan_labels.len(),
                    sample.join(", ")
                ),
            });
        }

        ConfidenceReport {
            overall,
            gaps,
            suggestions,
        }
    }

    /// Count distinct source prefixes in node labels to gauge diversity.
    fn source_diversity_score(&self) -> f64 {
        let sessions = match self.sessions.read() {
            Ok(s) => s,
            Err(_) => return 0.0,
        };
        let total_sources: usize = sessions.values().map(|s| s.sources_ingested.len()).sum();
        // Diminishing returns: 3 sources = 1.0.
        (total_sources as f64 / 3.0).min(1.0)
    }

    // ── Model export to file ──────────────────────────────────────

    /// Export the current model state to a JSON file at the given path.
    ///
    /// Produces a `weave-model.json` that includes the causal graph nodes,
    /// edges, confidence report, and metadata.
    pub fn export_model_to_file(
        &self,
        domain: &str,
        min_confidence: f64,
        path: &Path,
    ) -> Result<ExportedModel, WeaverError> {
        let model = self.export_model(domain, min_confidence)
            .map_err(WeaverError::Domain)?;
        let json = serde_json::to_string_pretty(&model)?;
        std::fs::write(path, json)?;
        info!(domain, ?path, "model exported to file");
        Ok(model)
    }

    /// Import a model from a JSON file.
    pub fn import_model_from_file(
        &self,
        domain: &str,
        path: &Path,
    ) -> Result<(), WeaverError> {
        let data = std::fs::read_to_string(path)?;
        let model: ExportedModel = serde_json::from_str(&data)?;
        self.import_model(domain, model)
            .map_err(WeaverError::Domain)?;
        info!(domain, ?path, "model imported from file");
        Ok(())
    }

    // ── Session management ────────────────────────────────────────

    /// Start a new modeling session.
    pub fn start_session(
        &self,
        domain: &str,
        context: Option<&str>,
        _goal: Option<&str>,
    ) -> Result<String, String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = ModelingSession {
            id: session_id.clone(),
            domain: domain.to_string(),
            started_at: Utc::now(),
            confidence: 0.0,
            gaps: Vec::new(),
            sources_ingested: Vec::new(),
            tick_count: 0,
            budget_remaining_ms: 300_000, // 5 min default
            active: true,
            metadata: {
                let mut m = HashMap::new();
                if let Some(ctx) = context {
                    m.insert("context".to_string(), serde_json::Value::String(ctx.to_string()));
                }
                m
            },
        };

        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;
        if sessions.contains_key(domain) {
            return Err(format!("session already exists for domain: {domain}"));
        }
        sessions.insert(domain.to_string(), session);

        // Record meta-loom event.
        self.record_meta_loom(
            domain,
            MetaDecisionType::ModelVersionBumped { from: 0, to: 1 },
            "Session initialized",
            0.0,
        );

        // Emit impulse.
        self.emit_impulse(ImpulseType::Custom(0x32));

        info!(domain, session_id = %session_id, "weaver session started");
        Ok(session_id)
    }

    /// Stop a modeling session.
    pub fn stop_session(&self, domain: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;
        let session = sessions
            .get_mut(domain)
            .ok_or_else(|| format!("no session for domain: {domain}"))?;
        session.active = false;
        info!(domain, "weaver session stopped");
        Ok(())
    }

    /// Resume a stopped session.
    pub fn resume_session(&self, domain: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;
        let session = sessions
            .get_mut(domain)
            .ok_or_else(|| format!("no session for domain: {domain}"))?;
        session.active = true;
        info!(domain, "weaver session resumed");
        Ok(())
    }

    /// Get a snapshot of a session.
    pub fn get_session(&self, domain: &str) -> Option<ModelingSession> {
        self.sessions
            .read()
            .ok()?
            .get(domain)
            .cloned()
    }

    /// List all session domains.
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions
            .read()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }

    // ── Source management ─────────────────────────────────────────

    /// Add a data source to a session.
    pub fn add_source(
        &self,
        domain: &str,
        source_type: &str,
        _root: Option<&PathBuf>,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;
        let session = sessions
            .get_mut(domain)
            .ok_or_else(|| format!("no session for domain: {domain}"))?;
        session.sources_ingested.push(source_type.to_string());

        // Record meta-loom event.
        drop(sessions); // release lock before recording
        self.record_meta_loom(
            domain,
            MetaDecisionType::SourceAdded {
                source_type: source_type.to_string(),
            },
            &format!("Added source: {source_type}"),
            self.get_session(domain).map(|s| s.confidence).unwrap_or(0.0),
        );

        // Emit impulse for source request.
        self.emit_impulse(ImpulseType::Custom(0x33));

        Ok(())
    }

    // ── Confidence evaluation ─────────────────────────────────────

    /// Evaluate confidence for a session domain.
    pub fn evaluate_confidence(&self, domain: &str) -> Result<ConfidenceReport, String> {
        let sessions = self.sessions.read().map_err(|e| e.to_string())?;
        let session = sessions
            .get(domain)
            .ok_or_else(|| format!("no session for domain: {domain}"))?;

        // Simple confidence model based on source count and graph size.
        let source_count = session.sources_ingested.len() as f64;
        let node_count = self.causal_graph.node_count() as f64;

        // Confidence grows with data, capped at 1.0.
        let base_confidence = (source_count * 0.15 + node_count * 0.01).min(1.0);

        let mut gaps = Vec::new();
        if source_count < 3.0 {
            gaps.push(ConfidenceGap {
                domain: domain.to_string(),
                current_confidence: base_confidence,
                target_confidence: 0.8,
                suggested_sources: vec!["git_log".into(), "file_tree".into()],
            });
        }

        let suggestions = if source_count < 2.0 {
            vec![ModelingSuggestion::AddSource {
                source_type: "git_log".into(),
                reason: "No git history ingested yet".into(),
            }]
        } else {
            Vec::new()
        };

        Ok(ConfidenceReport {
            overall: base_confidence,
            gaps,
            suggestions,
        })
    }

    // ── Cognitive tick handler ─────────────────────────────────────

    /// Process a single cognitive tick.
    ///
    /// Called by the CognitiveTick service during each tick cycle.
    /// Budget-aware: yields if budget is exhausted.
    pub fn tick(&self, budget: Duration) -> TickResult {
        let budget_start = Instant::now();

        let mut sessions = match self.sessions.write() {
            Ok(s) => s,
            Err(_) => return TickResult::Idle,
        };

        // Find an active session.
        let active_domain = sessions
            .iter()
            .find(|(_, s)| s.active)
            .map(|(d, _)| d.clone());

        let domain = match active_domain {
            Some(d) => d,
            None => return TickResult::Idle,
        };

        let session = match sessions.get_mut(&domain) {
            Some(s) => s,
            None => return TickResult::Idle,
        };

        // Phase 1: Evaluate confidence.
        let source_count = session.sources_ingested.len() as f64;
        let node_count = self.causal_graph.node_count() as f64;
        let confidence = (source_count * 0.15 + node_count * 0.01).min(1.0);
        session.confidence = confidence;

        // Phase 2: Identify gaps.
        let mut gaps = Vec::new();
        if confidence < 0.8 {
            gaps.push(ConfidenceGap {
                domain: domain.clone(),
                current_confidence: confidence,
                target_confidence: 0.8,
                suggested_sources: vec!["git_log".into(), "file_tree".into()],
            });
        }
        session.gaps = gaps.clone();

        if budget_start.elapsed() > budget {
            return TickResult::BudgetExhausted;
        }

        // Phase 3: Create a causal node to record the tick.
        let tick_label = format!("weaver.tick.{}.{}", domain, session.tick_count);
        self.causal_graph.add_node(
            tick_label,
            serde_json::json!({
                "domain": domain,
                "confidence": confidence,
                "tick": session.tick_count,
            }),
        );

        session.tick_count += 1;
        self.tick_count.fetch_add(1, Ordering::Relaxed);

        TickResult::Progress {
            confidence,
            gaps_remaining: gaps.len(),
        }
    }

    // ── Export / Import (K3c-G4) ──────────────────────────────────

    /// Export the model for a domain.
    pub fn export_model(
        &self,
        domain: &str,
        min_confidence: f64,
    ) -> Result<ExportedModel, String> {
        let sessions = self.sessions.read().map_err(|e| e.to_string())?;
        let session = sessions
            .get(domain)
            .ok_or_else(|| format!("no session for domain: {domain}"))?;

        // Collect all edges and filter by confidence.
        let edge_types: Vec<EdgeTypeSpec> = session
            .sources_ingested
            .iter()
            .enumerate()
            .map(|(i, src)| EdgeTypeSpec {
                from_type: "source".into(),
                to_type: "domain".into(),
                edge_type: format!("ingested_{src}"),
                confidence: (i as f64 + 1.0) * 0.2,
            })
            .filter(|e| e.confidence >= min_confidence)
            .collect();

        // Collect causal nodes from the graph.
        let mut causal_nodes = Vec::new();
        let mut causal_edges_out = Vec::new();
        let next_id = self.causal_graph.node_count() + 1;
        for nid in 1..next_id {
            if let Some(node) = self.causal_graph.get_node(nid) {
                causal_nodes.push(ExportedCausalNode {
                    label: node.label.clone(),
                    metadata: node.metadata.clone(),
                });
                // Collect forward edges from this node.
                for edge in self.causal_graph.get_forward_edges(nid) {
                    if let Some(target_node) = self.causal_graph.get_node(edge.target) {
                        causal_edges_out.push(ExportedCausalEdge {
                            source_label: node.label.clone(),
                            target_label: target_node.label.clone(),
                            edge_type: format!("{}", edge.edge_type),
                            weight: edge.weight,
                        });
                    }
                }
            }
        }

        Ok(ExportedModel {
            version: "1.0".to_string(),
            domain: domain.to_string(),
            exported_at: Utc::now(),
            confidence: session.confidence,
            node_types: vec![NodeTypeSpec {
                name: "default".into(),
                embedding_strategy: self.embedding_provider.model_name().to_string(),
                dimensions: self.embedding_provider.dimensions(),
            }],
            edge_types,
            causal_nodes,
            causal_edges: causal_edges_out,
            metadata: session.metadata.clone(),
        })
    }

    /// Import a previously exported model.
    pub fn import_model(
        &self,
        domain: &str,
        model: ExportedModel,
    ) -> Result<(), String> {
        // Version check.
        if !model.version.starts_with("1.") {
            return Err(format!(
                "incompatible model version: expected 1.x, got {}",
                model.version
            ));
        }

        let session = ModelingSession {
            id: uuid::Uuid::new_v4().to_string(),
            domain: domain.to_string(),
            started_at: Utc::now(),
            confidence: model.confidence,
            gaps: Vec::new(),
            sources_ingested: model
                .edge_types
                .iter()
                .map(|e| e.edge_type.clone())
                .collect(),
            tick_count: 0,
            budget_remaining_ms: 300_000,
            active: true,
            metadata: model.metadata,
        };

        let mut sessions = self.sessions.write().map_err(|e| e.to_string())?;
        sessions.insert(domain.to_string(), session);

        // Record meta-loom.
        drop(sessions);
        self.record_meta_loom(
            domain,
            MetaDecisionType::ModelVersionBumped { from: 0, to: 1 },
            "Imported from exported model",
            model.confidence,
        );

        info!(domain, "weaver model imported");
        Ok(())
    }

    // ── Command handler (IPC) ─────────────────────────────────────

    /// Handle a WeaverCommand received via IPC.
    pub fn handle_command(&self, cmd: WeaverCommand) -> WeaverResponse {
        match cmd {
            WeaverCommand::SessionStart {
                domain,
                context,
                goal,
                ..
            } => match self.start_session(
                &domain,
                context.as_deref(),
                goal.as_deref(),
            ) {
                Ok(session_id) => WeaverResponse::SessionStarted { domain, session_id },
                Err(e) => WeaverResponse::Error(e),
            },
            WeaverCommand::SessionStop { domain } => match self.stop_session(&domain) {
                Ok(()) => WeaverResponse::SessionStopped { domain },
                Err(e) => WeaverResponse::Error(e),
            },
            WeaverCommand::SessionResume { domain } => match self.resume_session(&domain) {
                Ok(()) => WeaverResponse::SessionResumed { domain },
                Err(e) => WeaverResponse::Error(e),
            },
            WeaverCommand::SourceAdd {
                domain,
                source_type,
                root,
                ..
            } => match self.add_source(&domain, &source_type, root.as_ref()) {
                Ok(()) => WeaverResponse::SourceAdded {
                    domain,
                    source_type,
                },
                Err(e) => WeaverResponse::Error(e),
            },
            WeaverCommand::SourceList { domain } => {
                match self.get_session(&domain) {
                    Some(s) => WeaverResponse::Sources(s.sources_ingested),
                    None => WeaverResponse::Error(format!("no session for domain: {domain}")),
                }
            }
            WeaverCommand::Confidence { domain, .. } => {
                match self.evaluate_confidence(&domain) {
                    Ok(report) => WeaverResponse::ConfidenceReport(report),
                    Err(e) => WeaverResponse::Error(e),
                }
            }
            WeaverCommand::Export {
                domain,
                min_confidence,
                output,
            } => match self.export_model(&domain, min_confidence) {
                Ok(model) => {
                    let edges = model.edge_types.len();
                    // In a real implementation, this would write to disk.
                    debug!(?output, edges, "model exported");
                    WeaverResponse::Exported { path: output, edges }
                }
                Err(e) => WeaverResponse::Error(e),
            },
            WeaverCommand::Import { domain, input } => {
                // In a real implementation, this would read from disk.
                debug!(?input, "model import requested");
                WeaverResponse::Imported { domain }
            }
            WeaverCommand::MetaStrategies => {
                let strategies = self.knowledge_base.list_strategies();
                WeaverResponse::Strategies(strategies)
            }
            WeaverCommand::MetaExportKb { output } => {
                debug!(?output, "KB export requested");
                WeaverResponse::KbExported { path: output }
            }
            _ => WeaverResponse::Error("command not implemented".into()),
        }
    }

    // ── Meta-Loom (K3c-G5) ───────────────────────────────────────

    /// Record a meta-loom event.
    fn record_meta_loom(
        &self,
        domain: &str,
        decision: MetaDecisionType,
        rationale: &str,
        confidence_before: f64,
    ) {
        let event = MetaLoomEvent {
            session_domain: domain.to_string(),
            decision_type: decision,
            confidence_before,
            confidence_after: None,
            rationale: rationale.to_string(),
            timestamp: Utc::now(),
        };

        // Record in causal graph under meta-loom namespace.
        let label = format!("meta-loom/{}", domain);
        self.causal_graph.add_node(
            label,
            serde_json::to_value(&event).unwrap_or_default(),
        );

        // Store in local event history.
        if let Ok(mut events) = self.meta_loom_events.write() {
            events
                .entry(domain.to_string())
                .or_default()
                .push(event);
        }
    }

    /// Get meta-loom events for a domain.
    pub fn meta_loom_events(&self, domain: &str) -> Vec<MetaLoomEvent> {
        self.meta_loom_events
            .read()
            .ok()
            .and_then(|m| m.get(domain).cloned())
            .unwrap_or_default()
    }

    /// Emit an impulse if the queue is configured.
    fn emit_impulse(&self, impulse_type: ImpulseType) {
        if let Some(queue) = &self.impulse_queue {
            queue.emit(
                0x03, // CausalGraph
                [0u8; 32],
                0x03, // self-referential
                impulse_type,
                serde_json::Value::Null,
                0,
            );
        }
    }

    // ── CognitiveTick integration ──────────────────────────────────

    /// Handle a cognitive tick — process pending work within budget.
    ///
    /// Called by the CognitiveTick service each cycle. Performs git polling,
    /// file change detection, pending ingestion, and periodic confidence
    /// recomputation, all within the supplied time budget.
    pub fn on_tick(&mut self, budget_ms: u32) -> CognitiveTickResult {
        let start = Instant::now();
        let budget = Duration::from_millis(budget_ms as u64);
        let mut result = CognitiveTickResult {
            tick_number: self.tick_count.load(Ordering::Relaxed),
            budget_ms,
            ..Default::default()
        };

        // 1. Check for new git commits (if git polling enabled).
        if start.elapsed() < budget {
            if let Some(new_commits) = self.poll_git() {
                result.git_commits_found = new_commits;
            }
        }

        // 2. Check for file changes (if file watcher enabled).
        if start.elapsed() < budget {
            if let Some(changed_files) = self.poll_file_changes() {
                result.files_changed = changed_files;
            }
        }

        // 3. Process pending ingestion queue (delegate to existing tick()).
        if start.elapsed() < budget {
            let remaining = budget.saturating_sub(start.elapsed());
            let tick_result = self.tick(remaining);
            if let TickResult::Progress { .. } = tick_result {
                result.nodes_processed = 1;
            }
        }

        // 4. Recompute confidence every 100 ticks.
        if start.elapsed() < budget && self.ticks_since_confidence_update > 100 {
            self.last_confidence = Some(self.compute_confidence());
            self.ticks_since_confidence_update = 0;
            result.confidence_updated = true;
        }

        result.elapsed_ms = start.elapsed().as_millis() as u32;
        result.within_budget = start.elapsed() <= budget;
        self.tick_count.fetch_add(1, Ordering::Relaxed);
        self.ticks_since_confidence_update += 1;

        result
    }

    /// Enable git polling for a repository path and branch.
    pub fn enable_git_polling(&mut self, repo_path: PathBuf, branch: String) {
        self.git_poller = Some(GitPoller::new(repo_path, branch));
    }

    /// Enable file watching for source files under a root directory.
    pub fn enable_file_watching(&mut self, root: PathBuf, patterns: Vec<String>) {
        let mut watcher = FileWatcher::new(root, patterns);
        watcher.watch_directory();
        self.file_watcher = Some(watcher);
    }

    /// Poll git for new commits (internal helper for on_tick).
    fn poll_git(&mut self) -> Option<usize> {
        self.git_poller.as_mut().map(|p| p.poll())
    }

    /// Poll file watcher for changed files (internal helper for on_tick).
    fn poll_file_changes(&mut self) -> Option<usize> {
        self.file_watcher.as_mut().map(|w| w.poll_changes().len())
    }

    /// Get the cached confidence report from the last recomputation.
    pub fn cached_confidence(&self) -> Option<&ConfidenceReport> {
        self.last_confidence.as_ref()
    }

    /// Get a reference to the git poller, if enabled.
    pub fn git_poller(&self) -> Option<&GitPoller> {
        self.git_poller.as_ref()
    }

    /// Get a reference to the file watcher, if enabled.
    pub fn file_watcher(&self) -> Option<&FileWatcher> {
        self.file_watcher.as_ref()
    }

    /// Total ticks processed.
    pub fn total_ticks(&self) -> u64 {
        self.tick_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl SystemService for WeaverEngine {
    fn name(&self) -> &str {
        "weaver"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("weaver engine started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Stop all active sessions.
        if let Ok(mut sessions) = self.sessions.write() {
            for (_, session) in sessions.iter_mut() {
                session.active = false;
            }
        }
        info!(
            total_ticks = self.total_ticks(),
            kb_strategies = self.knowledge_base.count(),
            "weaver engine stopped"
        );
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        // Always healthy: both active and idle states are normal.
        HealthStatus::Healthy
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hnsw_service::HnswServiceConfig;

    fn make_engine() -> WeaverEngine {
        let graph = Arc::new(CausalGraph::new());
        let hnsw = Arc::new(HnswService::new(HnswServiceConfig::default()));
        WeaverEngine::new_with_mock(graph, hnsw)
    }

    #[test]
    fn start_session_creates_session() {
        let engine = make_engine();
        let sid = engine
            .start_session("test-domain", Some("test context"), None)
            .unwrap();
        assert!(!sid.is_empty());
        let session = engine.get_session("test-domain").unwrap();
        assert_eq!(session.domain, "test-domain");
        assert!(session.active);
        assert_eq!(session.confidence, 0.0);
    }

    #[test]
    fn duplicate_session_start_fails() {
        let engine = make_engine();
        engine.start_session("dup", None, None).unwrap();
        let result = engine.start_session("dup", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn stop_and_resume_session() {
        let engine = make_engine();
        engine.start_session("lifecycle", None, None).unwrap();
        engine.stop_session("lifecycle").unwrap();
        assert!(!engine.get_session("lifecycle").unwrap().active);
        engine.resume_session("lifecycle").unwrap();
        assert!(engine.get_session("lifecycle").unwrap().active);
    }

    #[test]
    fn add_source_records_ingestion() {
        let engine = make_engine();
        engine.start_session("src-test", None, None).unwrap();
        engine.add_source("src-test", "git_log", None).unwrap();
        let session = engine.get_session("src-test").unwrap();
        assert_eq!(session.sources_ingested, vec!["git_log"]);
    }

    #[test]
    fn add_source_nonexistent_domain_fails() {
        let engine = make_engine();
        let result = engine.add_source("nope", "git_log", None);
        assert!(result.is_err());
    }

    #[test]
    fn evaluate_confidence_basic() {
        let engine = make_engine();
        engine.start_session("conf", None, None).unwrap();
        let report = engine.evaluate_confidence("conf").unwrap();
        assert!(report.overall >= 0.0 && report.overall <= 1.0);
        // With no sources, should have gaps.
        assert!(!report.gaps.is_empty());
    }

    #[test]
    fn evaluate_confidence_improves_with_sources() {
        let engine = make_engine();
        engine.start_session("improve", None, None).unwrap();
        let r1 = engine.evaluate_confidence("improve").unwrap();
        engine.add_source("improve", "git_log", None).unwrap();
        engine.add_source("improve", "file_tree", None).unwrap();
        let r2 = engine.evaluate_confidence("improve").unwrap();
        assert!(r2.overall >= r1.overall);
    }

    #[test]
    fn tick_with_no_session_returns_idle() {
        let engine = make_engine();
        let result = engine.tick(Duration::from_secs(1));
        assert!(matches!(result, TickResult::Idle));
    }

    #[test]
    fn tick_processes_active_session() {
        let engine = make_engine();
        engine.start_session("tick-test", None, None).unwrap();
        let result = engine.tick(Duration::from_secs(5));
        assert!(matches!(result, TickResult::Progress { .. }));
        let session = engine.get_session("tick-test").unwrap();
        assert_eq!(session.tick_count, 1);
    }

    #[test]
    fn tick_increments_total_count() {
        let engine = make_engine();
        engine.start_session("ticks", None, None).unwrap();
        engine.tick(Duration::from_secs(5));
        engine.tick(Duration::from_secs(5));
        assert_eq!(engine.total_ticks(), 2);
    }

    #[test]
    fn tick_skips_stopped_session() {
        let engine = make_engine();
        engine.start_session("stopped", None, None).unwrap();
        engine.stop_session("stopped").unwrap();
        let result = engine.tick(Duration::from_secs(5));
        assert!(matches!(result, TickResult::Idle));
    }

    #[test]
    fn export_model_basic() {
        let engine = make_engine();
        engine.start_session("export", None, None).unwrap();
        engine.add_source("export", "git_log", None).unwrap();
        let model = engine.export_model("export", 0.0).unwrap();
        assert_eq!(model.domain, "export");
        assert_eq!(model.version, "1.0");
        assert!(!model.node_types.is_empty());
    }

    #[test]
    fn export_model_filters_by_confidence() {
        let engine = make_engine();
        engine.start_session("filter", None, None).unwrap();
        engine.add_source("filter", "git", None).unwrap();
        engine.add_source("filter", "file", None).unwrap();
        let model_all = engine.export_model("filter", 0.0).unwrap();
        let model_high = engine.export_model("filter", 0.5).unwrap();
        assert!(model_all.edge_types.len() >= model_high.edge_types.len());
    }

    #[test]
    fn import_model_creates_session() {
        let engine = make_engine();
        let model = ExportedModel {
            version: "1.0".into(),
            domain: "imported".into(),
            exported_at: Utc::now(),
            confidence: 0.75,
            node_types: vec![],
            edge_types: vec![],
            causal_nodes: vec![],
            causal_edges: vec![],
            metadata: HashMap::new(),
        };
        engine.import_model("imported", model).unwrap();
        let session = engine.get_session("imported").unwrap();
        assert_eq!(session.confidence, 0.75);
        assert!(session.active);
    }

    #[test]
    fn import_model_version_check() {
        let engine = make_engine();
        let model = ExportedModel {
            version: "2.0".into(),
            domain: "bad".into(),
            exported_at: Utc::now(),
            confidence: 0.5,
            node_types: vec![],
            edge_types: vec![],
            causal_nodes: vec![],
            causal_edges: vec![],
            metadata: HashMap::new(),
        };
        let result = engine.import_model("bad", model);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("incompatible"));
    }

    #[test]
    fn meta_loom_events_recorded_on_session_start() {
        let engine = make_engine();
        engine.start_session("meta", None, None).unwrap();
        let events = engine.meta_loom_events("meta");
        assert!(!events.is_empty());
        assert!(matches!(
            events[0].decision_type,
            MetaDecisionType::ModelVersionBumped { .. }
        ));
    }

    #[test]
    fn meta_loom_events_recorded_on_source_add() {
        let engine = make_engine();
        engine.start_session("meta-src", None, None).unwrap();
        engine.add_source("meta-src", "git_log", None).unwrap();
        let events = engine.meta_loom_events("meta-src");
        assert!(events.len() >= 2); // session start + source add
        assert!(matches!(
            events.last().unwrap().decision_type,
            MetaDecisionType::SourceAdded { .. }
        ));
    }

    #[test]
    fn knowledge_base_record_and_list() {
        let kb = WeaverKnowledgeBase::new();
        kb.record_strategy(StrategyPattern {
            decision_type: "SourceAdded".into(),
            context: "rust-project".into(),
            improvement: 0.15,
            timestamp: Utc::now(),
        });
        let strategies = kb.list_strategies();
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].improvement, 0.15);
    }

    #[test]
    fn knowledge_base_strategies_for_domain() {
        let kb = WeaverKnowledgeBase::new();
        kb.record_strategy(StrategyPattern {
            decision_type: "SourceAdded".into(),
            context: "rust".into(),
            improvement: 0.1,
            timestamp: Utc::now(),
        });
        kb.record_strategy(StrategyPattern {
            decision_type: "EdgeType".into(),
            context: "python".into(),
            improvement: 0.2,
            timestamp: Utc::now(),
        });
        assert_eq!(kb.strategies_for("rust").len(), 1);
        assert_eq!(kb.strategies_for("python").len(), 1);
        assert_eq!(kb.strategies_for("go").len(), 0);
    }

    #[test]
    fn knowledge_base_export() {
        let kb = WeaverKnowledgeBase::new();
        kb.record_strategy(StrategyPattern {
            decision_type: "test".into(),
            context: "test".into(),
            improvement: 0.5,
            timestamp: Utc::now(),
        });
        let exported = kb.export();
        assert!(exported.is_array());
    }

    #[test]
    fn handle_command_session_start() {
        let engine = make_engine();
        let resp = engine.handle_command(WeaverCommand::SessionStart {
            domain: "cmd-test".into(),
            git_path: None,
            context: None,
            goal: None,
        });
        assert!(matches!(resp, WeaverResponse::SessionStarted { .. }));
    }

    #[test]
    fn handle_command_confidence() {
        let engine = make_engine();
        engine.start_session("cmd-conf", None, None).unwrap();
        let resp = engine.handle_command(WeaverCommand::Confidence {
            domain: "cmd-conf".into(),
            edge: None,
            verbose: false,
        });
        assert!(matches!(resp, WeaverResponse::ConfidenceReport(_)));
    }

    #[test]
    fn handle_command_source_list() {
        let engine = make_engine();
        engine.start_session("cmd-src", None, None).unwrap();
        engine.add_source("cmd-src", "git_log", None).unwrap();
        let resp = engine.handle_command(WeaverCommand::SourceList {
            domain: "cmd-src".into(),
        });
        match resp {
            WeaverResponse::Sources(s) => assert_eq!(s, vec!["git_log"]),
            other => panic!("expected Sources, got {other:?}"),
        }
    }

    #[test]
    fn handle_command_unknown_domain() {
        let engine = make_engine();
        let resp = engine.handle_command(WeaverCommand::Confidence {
            domain: "missing".into(),
            edge: None,
            verbose: false,
        });
        assert!(matches!(resp, WeaverResponse::Error(_)));
    }

    #[test]
    fn list_sessions() {
        let engine = make_engine();
        engine.start_session("a", None, None).unwrap();
        engine.start_session("b", None, None).unwrap();
        let mut sessions = engine.list_sessions();
        sessions.sort();
        assert_eq!(sessions, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn system_service_impl() {
        let engine = make_engine();
        assert_eq!(engine.name(), "weaver");
        assert_eq!(engine.service_type(), ServiceType::Core);
        engine.start().await.unwrap();
        let health = engine.health_check().await;
        assert_eq!(health, HealthStatus::Healthy);
        engine.stop().await.unwrap();
    }

    #[test]
    fn impulse_emitted_on_session_start() {
        let queue = Arc::new(ImpulseQueue::new());
        let graph = Arc::new(CausalGraph::new());
        let hnsw = Arc::new(HnswService::new(HnswServiceConfig::default()));
        let mut engine = WeaverEngine::new_with_mock(graph, hnsw);
        engine.set_impulse_queue(queue.clone());
        engine.start_session("impulse-test", None, None).unwrap();
        let impulses = queue.drain_ready();
        assert!(!impulses.is_empty());
        assert!(impulses.iter().any(|i| i.impulse_type == ImpulseType::Custom(0x32)));
    }

    #[test]
    fn impulse_emitted_on_source_add() {
        let queue = Arc::new(ImpulseQueue::new());
        let graph = Arc::new(CausalGraph::new());
        let hnsw = Arc::new(HnswService::new(HnswServiceConfig::default()));
        let mut engine = WeaverEngine::new_with_mock(graph, hnsw);
        engine.set_impulse_queue(queue.clone());
        engine.start_session("impulse-src", None, None).unwrap();
        let _ = queue.drain_ready(); // clear session-start impulse
        engine.add_source("impulse-src", "git", None).unwrap();
        let impulses = queue.drain_ready();
        assert!(impulses.iter().any(|i| i.impulse_type == ImpulseType::Custom(0x33)));
    }

    // ── Graph ingestion tests ────────────────────────────────────

    #[test]
    fn ingest_graph_file_git_history() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let graph_path = PathBuf::from(&manifest)
            .join("../../.weftos/graph/git-history.json");
        if !graph_path.exists() {
            // Skip if running outside the project tree.
            return;
        }
        let engine = make_engine();
        let result = engine.ingest_graph_file(&graph_path).unwrap();
        assert!(result.nodes_added > 0, "should ingest at least one node");
        assert_eq!(result.source, "git-history");
        assert!(result.embeddings_created > 0, "should create embeddings");
        // Verify causal graph was populated.
        assert!(engine.causal_graph().node_count() > 0);
    }

    #[test]
    fn ingest_graph_file_module_deps() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let graph_path = PathBuf::from(&manifest)
            .join("../../.weftos/graph/module-deps.json");
        if !graph_path.exists() {
            return;
        }
        let engine = make_engine();
        let result = engine.ingest_graph_file(&graph_path).unwrap();
        assert!(result.nodes_added > 0);
        assert_eq!(result.source, "module-dependencies");
    }

    #[test]
    fn ingest_graph_file_decisions() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let graph_path = PathBuf::from(&manifest)
            .join("../../.weftos/graph/decisions.json");
        if !graph_path.exists() {
            return;
        }
        let engine = make_engine();
        let result = engine.ingest_graph_file(&graph_path).unwrap();
        assert!(result.nodes_added > 0);
        assert_eq!(result.source, "decisions-and-phases");
    }

    #[test]
    fn ingest_graph_creates_edges() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let graph_path = PathBuf::from(&manifest)
            .join("../../.weftos/graph/git-history.json");
        if !graph_path.exists() {
            return;
        }
        let engine = make_engine();
        let result = engine.ingest_graph_file(&graph_path).unwrap();
        assert!(result.edges_added > 0, "git-history graph should have edges");
        assert!(engine.causal_graph().edge_count() > 0);
    }

    #[test]
    fn ingest_graph_populates_hnsw() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let graph_path = PathBuf::from(&manifest)
            .join("../../.weftos/graph/module-deps.json");
        if !graph_path.exists() {
            return;
        }
        let engine = make_engine();
        let result = engine.ingest_graph_file(&graph_path).unwrap();
        assert!(result.embeddings_created > 0);
        assert!(engine.hnsw().insert_count() > 0);
    }

    #[test]
    fn ingest_nonexistent_file_returns_error() {
        let engine = make_engine();
        let result = engine.ingest_graph_file(Path::new("/nonexistent/graph.json"));
        assert!(result.is_err());
    }

    #[test]
    fn ingest_invalid_json_returns_error() {
        let dir = std::env::temp_dir().join("weaver_test_invalid");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("bad.json");
        std::fs::write(&path, "not valid json {{{").unwrap();
        let engine = make_engine();
        let result = engine.ingest_graph_file(&path);
        assert!(result.is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ingest_empty_graph_returns_zero_counts() {
        let dir = std::env::temp_dir().join("weaver_test_empty");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("empty.json");
        std::fs::write(
            &path,
            r#"{"source":"test","nodes":[],"edges":[]}"#,
        )
        .unwrap();
        let engine = make_engine();
        let result = engine.ingest_graph_file(&path).unwrap();
        assert_eq!(result.nodes_added, 0);
        assert_eq!(result.edges_added, 0);
        assert_eq!(result.embeddings_created, 0);
        assert_eq!(result.source, "test");
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── Confidence scoring tests ─────────────────────────────────

    #[test]
    fn compute_confidence_empty_graph() {
        let engine = make_engine();
        let report = engine.compute_confidence();
        assert_eq!(report.overall, 0.0);
        assert!(!report.gaps.is_empty(), "should have gaps with empty graph");
        assert!(
            !report.suggestions.is_empty(),
            "should have suggestions with empty graph"
        );
    }

    #[test]
    fn compute_confidence_with_nodes_only() {
        let engine = make_engine();
        // Add nodes without edges -- should be partially confident.
        for i in 0..10 {
            engine.causal_graph().add_node(
                format!("node-{i}"),
                serde_json::json!({"test": true}),
            );
        }
        let report = engine.compute_confidence();
        // All orphans: connectivity = 0, but volume > 0.
        assert!(report.overall > 0.0, "should have some confidence from volume");
        assert!(report.overall < 0.5, "should be low without edges");
    }

    #[test]
    fn compute_confidence_with_connected_graph() {
        let engine = make_engine();
        // Create a small connected graph.
        let n1 = engine.causal_graph().add_node(
            "module-a".into(),
            serde_json::json!({}),
        );
        let n2 = engine.causal_graph().add_node(
            "module-b".into(),
            serde_json::json!({}),
        );
        let n3 = engine.causal_graph().add_node(
            "module-c".into(),
            serde_json::json!({}),
        );
        engine.causal_graph().link(n1, n2, CausalEdgeType::Enables, 1.0, 0, 0);
        engine.causal_graph().link(n2, n3, CausalEdgeType::Causes, 0.8, 0, 0);

        let report = engine.compute_confidence();
        // At least 2 of 3 nodes have edges, so connectivity should be decent.
        assert!(report.overall > 0.0);
    }

    #[test]
    fn compute_confidence_detects_orphans() {
        let engine = make_engine();
        let n1 = engine.causal_graph().add_node(
            "connected-a".into(),
            serde_json::json!({}),
        );
        let n2 = engine.causal_graph().add_node(
            "connected-b".into(),
            serde_json::json!({}),
        );
        engine.causal_graph().add_node(
            "orphan-x".into(),
            serde_json::json!({}),
        );
        engine.causal_graph().link(n1, n2, CausalEdgeType::Follows, 1.0, 0, 0);

        let report = engine.compute_confidence();
        // The suggestion should mention orphan nodes.
        let has_orphan_suggestion = report.suggestions.iter().any(|s| match s {
            ModelingSuggestion::AddSource { reason, .. } => reason.contains("orphan"),
            _ => false,
        });
        assert!(has_orphan_suggestion, "should detect the orphan node");
    }

    #[test]
    fn compute_confidence_improves_with_more_data() {
        let engine = make_engine();
        // Small graph.
        let n1 = engine.causal_graph().add_node("a".into(), serde_json::json!({}));
        let n2 = engine.causal_graph().add_node("b".into(), serde_json::json!({}));
        engine.causal_graph().link(n1, n2, CausalEdgeType::Enables, 1.0, 0, 0);
        let c1 = engine.compute_confidence().overall;

        // Add more connected nodes.
        for i in 0..50 {
            let na = engine.causal_graph().add_node(format!("extra-{i}"), serde_json::json!({}));
            engine.causal_graph().link(n1, na, CausalEdgeType::Correlates, 0.5, 0, 0);
        }
        let c2 = engine.compute_confidence().overall;
        assert!(c2 > c1, "confidence should increase with more data ({c2} > {c1})");
    }

    // ── Model export/import roundtrip tests ──────────────────────

    #[test]
    fn export_model_includes_causal_data() {
        let engine = make_engine();
        engine.start_session("exp-causal", None, None).unwrap();
        // Add some nodes and edges.
        let n1 = engine.causal_graph().add_node("node-a".into(), serde_json::json!({}));
        let n2 = engine.causal_graph().add_node("node-b".into(), serde_json::json!({}));
        engine.causal_graph().link(n1, n2, CausalEdgeType::Causes, 0.9, 0, 0);

        let model = engine.export_model("exp-causal", 0.0).unwrap();
        assert!(!model.causal_nodes.is_empty(), "exported model should have nodes");
        assert!(!model.causal_edges.is_empty(), "exported model should have edges");
    }

    #[test]
    fn export_model_to_file_roundtrip() {
        let dir = std::env::temp_dir().join("weaver_test_export");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test-model.json");

        let engine = make_engine();
        engine.start_session("roundtrip", None, None).unwrap();
        engine.add_source("roundtrip", "git_log", None).unwrap();

        // Add some graph data.
        let n1 = engine.causal_graph().add_node("rt-a".into(), serde_json::json!({}));
        let n2 = engine.causal_graph().add_node("rt-b".into(), serde_json::json!({}));
        engine.causal_graph().link(n1, n2, CausalEdgeType::Enables, 1.0, 0, 0);

        // Export.
        let exported = engine.export_model_to_file("roundtrip", 0.0, &path).unwrap();
        assert!(path.exists(), "export file should exist");

        // Read back and verify JSON is valid.
        let data = std::fs::read_to_string(&path).unwrap();
        let reimported: ExportedModel = serde_json::from_str(&data).unwrap();
        assert_eq!(reimported.domain, "roundtrip");
        assert_eq!(reimported.version, exported.version);
        assert_eq!(reimported.causal_nodes.len(), exported.causal_nodes.len());
        assert_eq!(reimported.causal_edges.len(), exported.causal_edges.len());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_model_from_file_works() {
        let dir = std::env::temp_dir().join("weaver_test_import");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("import-model.json");

        // Write a model file.
        let model = ExportedModel {
            version: "1.0".into(),
            domain: "file-import".into(),
            exported_at: Utc::now(),
            confidence: 0.82,
            node_types: vec![],
            edge_types: vec![],
            causal_nodes: vec![ExportedCausalNode {
                label: "test-node".into(),
                metadata: serde_json::json!({}),
            }],
            causal_edges: vec![],
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string_pretty(&model).unwrap();
        std::fs::write(&path, json).unwrap();

        let engine = make_engine();
        engine.import_model_from_file("file-import", &path).unwrap();
        let session = engine.get_session("file-import").unwrap();
        assert_eq!(session.confidence, 0.82);
        assert!(session.active);

        std::fs::remove_dir_all(&dir).ok();
    }

    // ── Edge type parsing tests ──────────────────────────────────

    #[test]
    fn parse_edge_type_known_types() {
        assert_eq!(WeaverEngine::parse_edge_type("Causes"), CausalEdgeType::Causes);
        assert_eq!(WeaverEngine::parse_edge_type("Enables"), CausalEdgeType::Enables);
        assert_eq!(WeaverEngine::parse_edge_type("Follows"), CausalEdgeType::Follows);
        assert_eq!(WeaverEngine::parse_edge_type("Correlates"), CausalEdgeType::Correlates);
        assert_eq!(WeaverEngine::parse_edge_type("EvidenceFor"), CausalEdgeType::EvidenceFor);
        assert_eq!(WeaverEngine::parse_edge_type("Inhibits"), CausalEdgeType::Inhibits);
        assert_eq!(WeaverEngine::parse_edge_type("Contradicts"), CausalEdgeType::Contradicts);
        assert_eq!(WeaverEngine::parse_edge_type("TriggeredBy"), CausalEdgeType::TriggeredBy);
    }

    #[test]
    fn parse_edge_type_unknown_defaults_to_correlates() {
        assert_eq!(WeaverEngine::parse_edge_type("FooBar"), CausalEdgeType::Correlates);
        assert_eq!(WeaverEngine::parse_edge_type(""), CausalEdgeType::Correlates);
    }

    // ── WeaverError tests ────────────────────────────────────────

    #[test]
    fn weaver_error_display() {
        let io_err = WeaverError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        assert!(io_err.to_string().contains("I/O"));

        let domain_err = WeaverError::Domain("test failure".to_string());
        assert!(domain_err.to_string().contains("test failure"));
    }

    // ── CognitiveTick integration tests ─────────────────────────

    fn make_engine_mut() -> WeaverEngine {
        let graph = Arc::new(CausalGraph::new());
        let hnsw = Arc::new(HnswService::new(HnswServiceConfig::default()));
        WeaverEngine::new_with_mock(graph, hnsw)
    }

    #[test]
    fn on_tick_respects_budget() {
        let mut engine = make_engine_mut();
        engine.start_session("tick-budget", None, None).unwrap();
        let result = engine.on_tick(500); // 500ms budget
        assert!(
            result.within_budget,
            "tick should complete within 500ms budget"
        );
        assert!(result.elapsed_ms <= 500, "elapsed should be within budget");
    }

    #[test]
    fn on_tick_returns_correct_fields() {
        let mut engine = make_engine_mut();
        engine.start_session("tick-fields", None, None).unwrap();
        let result = engine.on_tick(100);
        assert_eq!(result.budget_ms, 100);
        assert_eq!(result.tick_number, 0); // first tick
        // No git poller or file watcher configured, so these should be 0.
        assert_eq!(result.git_commits_found, 0);
        assert_eq!(result.files_changed, 0);
    }

    #[test]
    fn on_tick_increments_tick_count() {
        let mut engine = make_engine_mut();
        engine.start_session("tick-count", None, None).unwrap();
        engine.on_tick(100);
        engine.on_tick(100);
        engine.on_tick(100);
        // on_tick calls tick() internally which also increments, plus on_tick itself.
        // The on_tick method does fetch_add(1) each call.
        assert!(engine.total_ticks() >= 3, "should have at least 3 ticks");
    }

    #[test]
    fn on_tick_confidence_update_after_100_ticks() {
        let mut engine = make_engine_mut();
        engine.start_session("conf-update", None, None).unwrap();
        // Simulate 101 ticks to trigger confidence update.
        engine.ticks_since_confidence_update = 101;
        let result = engine.on_tick(1000);
        assert!(
            result.confidence_updated,
            "confidence should be updated after 100+ ticks"
        );
        assert!(
            engine.cached_confidence().is_some(),
            "cached confidence should be set"
        );
        assert_eq!(
            engine.ticks_since_confidence_update, 1,
            "counter should reset to 1 (incremented after reset)"
        );
    }

    #[test]
    fn on_tick_no_confidence_update_before_100_ticks() {
        let mut engine = make_engine_mut();
        engine.start_session("no-conf-update", None, None).unwrap();
        let result = engine.on_tick(100);
        assert!(
            !result.confidence_updated,
            "should not update confidence on first tick"
        );
    }

    #[test]
    fn on_tick_git_and_file_watcher_disabled_by_default() {
        let engine = make_engine_mut();
        assert!(
            engine.git_poller().is_none(),
            "git poller should be None by default"
        );
        assert!(
            engine.file_watcher().is_none(),
            "file watcher should be None by default"
        );
    }

    // ── GitPoller tests ─────────────────────────────────────────

    #[test]
    fn git_poller_poll_detects_commits_in_real_repo() {
        // Use the actual project repo for this test.
        let manifest =
            std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let repo_path = PathBuf::from(&manifest).join("../..");
        if !repo_path.join(".git").exists() {
            return; // skip if not in a git repo
        }
        let mut poller = GitPoller::new(repo_path, "HEAD".to_string());
        // First poll should detect at least 1 commit.
        let count = poller.poll();
        assert!(count >= 1, "first poll should find at least 1 commit");
        assert!(poller.last_hash().is_some(), "last hash should be set");
    }

    #[test]
    fn git_poller_poll_returns_zero_on_no_changes() {
        let manifest =
            std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let repo_path = PathBuf::from(&manifest).join("../..");
        if !repo_path.join(".git").exists() {
            return;
        }
        let mut poller = GitPoller::new(repo_path, "HEAD".to_string());
        poller.poll(); // first poll sets the baseline
        let count = poller.poll(); // second poll, no new commits
        assert_eq!(count, 0, "second poll should find 0 new commits");
    }

    #[test]
    fn git_poller_disabled_returns_zero() {
        let mut poller = GitPoller::new(PathBuf::from("/tmp"), "main".to_string());
        poller.set_enabled(false);
        assert_eq!(poller.poll(), 0);
        assert!(!poller.is_enabled());
    }

    #[test]
    fn git_poller_nonexistent_repo_returns_zero() {
        let mut poller = GitPoller::new(
            PathBuf::from("/nonexistent/path/to/repo"),
            "main".to_string(),
        );
        let count = poller.poll();
        assert_eq!(count, 0, "should return 0 for nonexistent repo");
        assert!(poller.last_hash().is_none());
    }

    #[test]
    fn git_poller_branch_accessor() {
        let poller = GitPoller::new(PathBuf::from("/tmp"), "develop".to_string());
        assert_eq!(poller.branch(), "develop");
    }

    // ── FileWatcher tests ───────────────────────────────────────

    #[test]
    fn file_watcher_watch_and_poll_detects_mtime_change() {
        let dir = std::env::temp_dir().join("weaver_fw_test_mtime");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test.rs");
        std::fs::write(&path, "fn main() {}").unwrap();

        let mut watcher = FileWatcher::new(dir.clone(), vec!["*.rs".to_string()]);
        watcher.watch(path.clone());
        assert_eq!(watcher.watched_count(), 1);

        // First poll: no changes (mtime matches).
        let changed = watcher.poll_changes();
        assert!(changed.is_empty(), "no changes on first poll");

        // Simulate mtime change by sleeping briefly and rewriting.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&path, "fn main() { println!(\"changed\"); }").unwrap();

        let changed = watcher.poll_changes();
        assert_eq!(changed.len(), 1, "should detect the changed file");
        assert_eq!(changed[0], path);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_watcher_watch_directory_registers_files() {
        let dir = std::env::temp_dir().join("weaver_fw_test_dir");
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(dir.join("lib.rs"), "// lib").unwrap();
        std::fs::write(dir.join("main.rs"), "// main").unwrap();
        std::fs::write(dir.join("readme.md"), "# readme").unwrap();

        let mut watcher = FileWatcher::new(dir.clone(), vec!["*.rs".to_string()]);
        watcher.watch_directory();
        assert_eq!(watcher.watched_count(), 2, "should only watch .rs files");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_watcher_detects_deleted_file() {
        let dir = std::env::temp_dir().join("weaver_fw_test_delete");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("temp.rs");
        std::fs::write(&path, "// temp").unwrap();

        let mut watcher = FileWatcher::new(dir.clone(), vec!["*.rs".to_string()]);
        watcher.watch(path.clone());

        // Delete the file.
        std::fs::remove_file(&path).unwrap();
        let changed = watcher.poll_changes();
        assert_eq!(changed.len(), 1, "should detect deleted file");
        assert_eq!(watcher.watched_count(), 0, "deleted file should be unregistered");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_watcher_disabled_returns_empty() {
        let mut watcher = FileWatcher::new(
            PathBuf::from("/tmp"),
            vec!["*.rs".to_string()],
        );
        watcher.set_enabled(false);
        assert!(watcher.poll_changes().is_empty());
        assert!(!watcher.is_enabled());
    }

    // ── WeaverEngine git/file integration tests ─────────────────

    #[test]
    fn enable_git_polling_sets_poller() {
        let mut engine = make_engine_mut();
        assert!(engine.git_poller().is_none());
        engine.enable_git_polling(PathBuf::from("/tmp"), "main".to_string());
        assert!(engine.git_poller().is_some());
        assert_eq!(engine.git_poller().unwrap().branch(), "main");
    }

    #[test]
    fn enable_file_watching_sets_watcher() {
        let dir = std::env::temp_dir().join("weaver_fw_test_enable");
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(dir.join("test.rs"), "// test").unwrap();

        let mut engine = make_engine_mut();
        assert!(engine.file_watcher().is_none());
        engine.enable_file_watching(dir.clone(), vec!["*.rs".to_string()]);
        assert!(engine.file_watcher().is_some());
        assert_eq!(engine.file_watcher().unwrap().watched_count(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn on_tick_with_git_polling_enabled() {
        let manifest =
            std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let repo_path = PathBuf::from(&manifest).join("../..");
        if !repo_path.join(".git").exists() {
            return;
        }
        let mut engine = make_engine_mut();
        engine.start_session("git-tick", None, None).unwrap();
        engine.enable_git_polling(repo_path, "HEAD".to_string());
        let result = engine.on_tick(500);
        // First tick should detect at least 1 commit (initial baseline).
        assert!(
            result.git_commits_found >= 1,
            "first on_tick with git polling should find commits"
        );
    }

    #[test]
    fn cognitive_tick_result_default() {
        let result = CognitiveTickResult::default();
        assert_eq!(result.tick_number, 0);
        assert_eq!(result.elapsed_ms, 0);
        assert_eq!(result.budget_ms, 0);
        assert_eq!(result.git_commits_found, 0);
        assert_eq!(result.files_changed, 0);
        assert_eq!(result.nodes_processed, 0);
        assert!(!result.confidence_updated);
        assert!(!result.within_budget);
    }

    #[test]
    fn cognitive_tick_result_serde_roundtrip() {
        let result = CognitiveTickResult {
            tick_number: 42,
            elapsed_ms: 15,
            budget_ms: 50,
            git_commits_found: 3,
            files_changed: 2,
            nodes_processed: 10,
            confidence_updated: true,
            within_budget: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: CognitiveTickResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tick_number, 42);
        assert_eq!(restored.git_commits_found, 3);
        assert!(restored.confidence_updated);
    }
}
