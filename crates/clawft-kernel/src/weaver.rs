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
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::causal::CausalGraph;
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
            causal_nodes: Vec::new(), // Simplified: full graph export would enumerate nodes
            causal_edges: Vec::new(),
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
}
