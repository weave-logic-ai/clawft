# 08c Content Integrity & Operational Services

**Document ID**: 08c
**Workstream**: W-KERNEL
**Duration**: Weeks 18-22
**Goal**: Complete K3 content-addressed storage, K3c cognitive substrate extensions, K5 operational services, and K6 mesh extensions -- artifact store, WeaverEngine, embedding provider, config/secrets, auth, tree views, artifact exchange, and log aggregation
**Depends on**: K0-K6 (all prior phases), 08a (self-healing), 08b partial (DLQ + metrics needed for K6 items)
**Orchestrator**: `08-orchestrator.md`
**Priority**: P2 (Important)

---

## S -- Specification

### What Changes

K3 delivered the `WasmToolRunner` and WASM module loading but without
content-addressed storage. K3c delivered the ECC cognitive substrate but
without the `WeaverEngine` service or pluggable embedding backends. K5
delivered the `AppManager` and application lifecycle but without config/secrets
management, credential delegation, or per-agent tree views. K6 delivered mesh
transport but without cross-node artifact exchange or log aggregation.

This phase fills all of these higher-level gaps, completing the operational
services layer.

### Gap Summary

| Gap ID | Area | New Lines | Changed Lines | Priority |
|--------|------|:---------:|:------------:|----------|
| K3-G1 | Content-addressed artifact store | ~400 | ~50 | P2 |
| K3c-G1 | WeaverEngine SystemService | ~500 | ~30 | P1 |
| K3c-G2 | EmbeddingProvider trait | ~150 | ~20 | P1 |
| K3c-G3 | Weaver CLI commands (daemon socket) | ~200 | ~30 | P1 |
| K3c-G4 | weave-model.json export + import | ~150 | ~10 | P1 |
| K3c-G5 | Meta-Loom integration | ~100 | ~20 | P1 |
| K5-G1 | Config and secrets service | ~350 | ~30 | P2 |
| K5-G2 | Auth agent (Factotum) | ~300 | ~20 | P2 |
| K5-G3 | Per-agent tree views | ~300 | ~30 | P2 |
| K6-G1 | Artifact exchange protocol | ~200 | ~50 | P2 |
| K6-G2 | Cross-node log aggregation | ~100 | ~20 | P2 |
| **Total** | | **~2,750** | **~330** | |

### Feature Gates

```toml
[features]
os-patterns = ["exochain"]       # Config, auth, tree views, WeaverEngine
os-full = ["os-patterns", "blake3"]  # Adds content-addressed artifact store
ecc = []                         # WeaverEngine, EmbeddingProvider (existing gate)
```

- K3-G1 (artifact store) requires `os-full` (pulls `blake3`)
- K3c-G1/G2/G3/G4/G5 (WeaverEngine, EmbeddingProvider, CLI, export, Meta-Loom) require `ecc` feature
- K5 and K6 gaps require `os-patterns`
- K6-G1 (artifact exchange) requires `os-full` + mesh

---

### K3-G1: Content-Addressed Artifact Store (~400 lines)

**Source**: IPFS CAS (R16)

**Why K3**: The `WasmToolRunner` (`wasm_runner.rs`) and WASM module loading
were introduced in K3. Content-addressed storage for WASM modules ensures
tamper-proof, deduplicated module management.

**Files**: New `crates/clawft-kernel/src/artifact_store.rs`

**Types**:
```rust
/// Content-addressed artifact store using BLAKE3 hashes.
pub struct ArtifactStore {
    artifacts: DashMap<String, StoredArtifact>,
    backend: ArtifactBackend,
    total_size: AtomicU64,
}

pub struct StoredArtifact {
    pub hash: String,
    pub size: u64,
    pub content_type: ArtifactType,
    pub stored_at: DateTime<Utc>,
    pub reference_count: AtomicU32,
}

pub enum ArtifactType {
    WasmModule,
    AppManifest,
    ConfigBundle,
    Generic,
}

pub enum ArtifactBackend {
    Memory(DashMap<String, Vec<u8>>),
    File { base_path: PathBuf },
}
```

**Behavior**:
- `store(content, type)` -- compute BLAKE3 hash, store if new
- `load(hash)` -- retrieve content, verify hash on load
- Deduplication: same content = same hash = one copy
- Reference counting for garbage collection
- Tree registration at `/kernel/artifacts/{hash}`
- `WasmToolRunner::load_module()` modified to load by hash
- File backend: two-level directory sharding (`{hash[0..2]}/{hash}`) avoids
  filesystem limits

**Tests**:
- Store and load roundtrip
- Hash verification on load (tampered content detected)
- Duplicate store returns same hash
- File backend correct path structure
- Reference counting works
- `WasmToolRunner` loads from artifact store

---

### K3c Gaps: ECC Cognitive Substrate (Weaver)

**Operator interface**: The Weaver skill at `agents/weftos-ecc/WEAVER.md`
describes HOW an operator (human or Claude agent) interacts with the
WeaverEngine -- session workflows, CLI commands, confidence interpretation,
and modeling strategy. The implementation below is the kernel `SystemService`
that those interactions drive. The skill is the user-facing guide; the K3c
gaps are the engine underneath.

**Implementation agent**: The `weaver` agent (`agents/weftos/weaver.md`) is
the kernel-native cognitive modeler that runs the HYPOTHESIZE-OBSERVE-
EVALUATE-ADJUST loop. It implements K3c-G1 through K3c-G5.

### K3c-G1: WeaverEngine SystemService (~500 lines)

**Source**: ECC Weaver SPARC Plan (09-ecc-weaver-crate.md)

**Why K3c**: The ECC modules (`causal.rs`, `cognitive_tick.rs`, `crossref.rs`,
`hnsw_service.rs`, `impulse.rs`, `calibration.rs`) were introduced in K3c.
The WeaverEngine is the service that drives ECC to actually process real data.

**Files**: New `crates/clawft-kernel/src/weaver.rs` or `crates/ecc-weaver/`

Register `WeaverEngine` as a `SystemService` at boot (when `ecc` feature enabled):

- `ModelingSession` management (start, resume, stop)
- Confidence-driven modeling loop within CognitiveTick
- Multi-source data ingestion (git, files, docs, CI, issues)
- `weave-model.json` export for edge deployment
- Meta-Loom tracking (Weaver's own evolution as causal events)
- WeaverKnowledgeBase for cross-domain learning
- A2ARouter IPC message handling for `weaver ecc` CLI commands
- Tree registration at `/kernel/services/weaver`

**Estimate**: ~500 lines for core service + ~300 lines for data source ingestion

**Types** (key interfaces):
```rust
/// WeaverEngine: ECC-powered codebase modeling service.
pub struct WeaverEngine {
    session: RwLock<Option<ModelingSession>>,
    knowledge_base: Arc<WeaverKnowledgeBase>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    hnsw: Arc<HnswService>,
    causal_graph: Arc<CausalGraph>,
}

pub struct ModelingSession {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub confidence: f64,
    pub gaps: Vec<ConfidenceGap>,
    pub sources_ingested: Vec<String>,
    pub tick_count: u64,
    pub budget_remaining: Duration,
}

pub struct ConfidenceGap {
    pub domain: String,
    pub current_confidence: f64,
    pub target_confidence: f64,
    pub suggested_sources: Vec<String>,
}
```

**Behavior**:
- Start session: initialize `ModelingSession`, set confidence targets
- CognitiveTick handler: evaluate confidence, identify gaps, ingest next source
- Git log ingestion: parse commits into causal nodes with edges to files
- File tree ingestion: walk directory, create namespace structure in causal graph
- Model export: serialize causal graph + HNSW index to `weave-model.json`
- Meta-Loom: record Weaver's own decisions (which sources to ingest, confidence
  changes) as causal events for self-improvement tracking
- Budget enforcement: respect `CognitiveTick` budget, yield when exhausted

---

### K3c-G2: EmbeddingProvider Trait (~150 lines)

**Why K3c**: Required by WeaverEngine for HNSW vectorization of ingested data.

**Files**: New `crates/clawft-kernel/src/embedding.rs`

**Types**:
```rust
/// Trait for pluggable embedding backends.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text chunk into a vector.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Embed a batch of text chunks (may be more efficient than individual calls).
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Dimensionality of the output vectors.
    fn dimensions(&self) -> usize;

    /// Name of the embedding model (for metadata tracking).
    fn model_name(&self) -> &str;
}

#[derive(Debug)]
pub enum EmbeddingError {
    ModelNotLoaded,
    DimensionMismatch { expected: usize, got: usize },
    BackendError(String),
    RateLimited { retry_after: Duration },
}

/// Mock embedding provider for testing.
pub struct MockEmbeddingProvider {
    pub dimensions: usize,
}

/// ONNX-based local embedding provider (behind feature flag).
#[cfg(feature = "onnx-embeddings")]
pub struct OnnxEmbeddingProvider {
    session: ort::Session,
    dimensions: usize,
    model_name: String,
}
```

**Behavior**:
- `MockEmbeddingProvider`: returns deterministic vectors based on text hash
  (no external dependencies, fast tests)
- `OnnxEmbeddingProvider`: loads ONNX model file, runs inference locally
  (behind `onnx-embeddings` feature flag)
- Batch embedding: `embed_batch` default implementation calls `embed` in loop;
  backends can override for efficiency
- Lazy initialization: model loaded on first `embed()` call

---

### K3c-G3: Weaver CLI Commands (~200 lines)

**Why K3c**: The WeaverEngine is a kernel process, not a CLI tool. Operators
interact with it via `weaver ecc` subcommands that send IPC messages through
the daemon Unix socket to the running Weaver process.

**Files**: Extend CLI in `crates/clawft-cli/src/commands/` (or equivalent), modify `weaver.rs` for IPC handler

**CLI commands**:
```bash
# Session management
weaver ecc session start --domain <name> --git <path> [--context <desc>] [--goal <target>]
weaver ecc session resume --domain <name>
weaver ecc session stop --domain <name>
weaver ecc session watch --domain <name>

# Data sources
weaver ecc source add --domain <name> --type <source_type> [--root <path>] [--watch]
weaver ecc source list --domain <name>

# Confidence
weaver ecc confidence --domain <name> [--edge <from->to>] [--verbose] [--watch]

# Export / Import
weaver ecc export --domain <name> --min-confidence <threshold> --output <path>
weaver ecc import --domain <name> --input <path>

# Meta-loom
weaver ecc meta --domain <name>
weaver ecc meta strategies
weaver ecc meta export-kb --output <path>

# Stitching
weaver ecc stitch --source <domain_a> --target <domain_b> --output <merged_domain>
```

**IPC flow**:
- Each CLI command serializes to a `KernelMessage` with `IpcPayload::WeaverCommand`
- Message sent via daemon Unix socket to the running kernel
- WeaverEngine's IPC handler matches the command variant and executes
- Response returned via `correlation_id` reply pattern

**Types**:
```rust
/// Commands sent from CLI to WeaverEngine via daemon socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeaverCommand {
    SessionStart { domain: String, git_path: Option<PathBuf>, context: Option<String>, goal: Option<String> },
    SessionResume { domain: String },
    SessionStop { domain: String },
    SessionWatch { domain: String },
    SourceAdd { domain: String, source_type: String, root: Option<PathBuf>, watch: bool },
    SourceList { domain: String },
    Confidence { domain: String, edge: Option<String>, verbose: bool },
    Export { domain: String, min_confidence: f64, output: PathBuf },
    Import { domain: String, input: PathBuf },
    MetaStatus { domain: String },
    MetaStrategies,
    MetaExportKb { output: PathBuf },
    Stitch { source: String, target: String, output: String },
}
```

**Tests**:
- CLI `session start` creates a `ModelingSession` in WeaverEngine
- CLI `confidence` returns current confidence report
- CLI `export` produces a valid `weave-model.json` file
- CLI `source add` registers source and triggers ingestion
- Unknown domain returns meaningful error
- Watch mode receives streaming updates via subscription

---

### K3c-G4: weave-model.json Export + Import (~150 lines)

**Why K3c**: Edge devices and offline analysis need a portable, serialized
representation of the Weaver's learned causal model.

**Files**: Extend `weaver.rs` or new `crates/clawft-kernel/src/weaver_export.rs`

**Types**:
```rust
/// Serialized model for edge deployment or offline analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedModel {
    pub version: String,
    pub domain: String,
    pub exported_at: DateTime<Utc>,
    pub confidence: f64,
    pub node_types: Vec<NodeTypeSpec>,
    pub edge_types: Vec<EdgeTypeSpec>,
    pub causal_nodes: Vec<ExportedCausalNode>,
    pub causal_edges: Vec<ExportedCausalEdge>,
    pub hnsw_snapshot: Option<HnswSnapshot>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct NodeTypeSpec {
    pub name: String,
    pub embedding_strategy: String,
    pub dimensions: usize,
}

pub struct EdgeTypeSpec {
    pub from_type: String,
    pub to_type: String,
    pub edge_type: String,
    pub confidence: f64,
}

pub struct HnswSnapshot {
    pub dimensions: usize,
    pub num_vectors: usize,
    pub serialized: Vec<u8>,
}
```

**Behavior**:
- `export(domain, min_confidence)` filters edges below threshold, serializes
  causal graph + HNSW index into `weave-model.json`
- `import(domain, path)` deserializes exported model, creates new session
  pre-populated with the learned structure
- Import verifies schema version compatibility
- HNSW snapshot optional (can be regenerated from embeddings on import)
- Chain event: `weaver.export` / `weaver.import` with domain and model stats

**Tests**:
- Export produces valid JSON matching `ExportedModel` schema
- Export filters edges below `min_confidence`
- Import creates session with pre-loaded causal graph
- Import + export roundtrip preserves structure
- Schema version mismatch returns clear error
- HNSW snapshot included when available

---

### K3c-G5: Meta-Loom Integration (~100 lines)

**Why K3c**: The Weaver improves at weaving by tracking its own modeling
decisions as causal events in a dedicated Meta-Loom. This is the self-evolution
mechanism: the Weaver's own improvement trajectory IS an ECC conversation.

**Files**: Extend `weaver.rs`

**Types**:
```rust
/// Meta-Loom event: records a Weaver modeling decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaLoomEvent {
    pub session_domain: String,
    pub decision_type: MetaDecisionType,
    pub confidence_before: f64,
    pub confidence_after: Option<f64>,
    pub rationale: String,
    pub timestamp: DateTime<Utc>,
}

pub enum MetaDecisionType {
    SourceAdded { source_type: String },
    EdgeTypeCreated { from: String, to: String, edge_type: String },
    EdgeTypeRemoved { from: String, to: String },
    EmbeddingStrategyChanged { node_type: String, old: String, new: String },
    TickIntervalAdjusted { old_ms: u64, new_ms: u64 },
    HnswDimensionsChanged { node_type: String, old: usize, new: usize },
    ModelVersionBumped { from: u32, to: u32 },
    StrategyLearned { pattern: String },
}
```

**Behavior**:
- Every modeling decision (source addition, edge type creation/removal,
  embedding strategy change, tick interval adjustment) is recorded as a
  `MetaLoomEvent` in the kernel's `CausalGraph`
- Meta-Loom events are in a dedicated namespace: `meta-loom/{domain}`
- `WeaverKnowledgeBase` accumulates successful strategies (patterns that led
  to confidence improvements) across sessions and domains
- `meta strategies` CLI command lists learned strategies from the knowledge base
- `meta export-kb` exports the cross-domain knowledge base for backup or transfer
- Impulses emitted: `ImpulseType::Custom(0x32)` on model version bump,
  `ImpulseType::Custom(0x33)` on source request

**Tests**:
- Modeling decisions recorded in CausalGraph under `meta-loom/` namespace
- Strategy learned when confidence improves after a decision
- `WeaverKnowledgeBase` persists across session restarts
- `meta strategies` returns learned patterns
- `meta export-kb` produces valid JSON
- New session in similar domain applies learned strategies from KB

---

### K5-G1: Config and Secrets Service (~350 lines)

**Source**: Kubernetes ConfigMaps/Secrets (R14)

**Why K5**: Applications need runtime configuration and secrets management.

**Files**: New `crates/clawft-kernel/src/config_service.rs`

**Types**:
```rust
/// Configuration and secrets service backed by the ResourceTree.
pub struct ConfigService {
    tree: Arc<TreeManager>,
    subscribers: DashMap<String, Vec<mpsc::Sender<ConfigChange>>>,
    encryption_key: [u8; 32],
}

pub struct ConfigChange {
    pub namespace: String,
    pub key: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub changed_by: Pid,
    pub timestamp: DateTime<Utc>,
}

pub struct SecretRef {
    pub namespace: String,
    pub key: String,
    pub expires_at: DateTime<Utc>,
    pub scoped_to: Vec<Pid>,
}
```

**Behavior**:
- Config at `/kernel/config/{namespace}/{key}` in ResourceTree
- Secrets at `/kernel/secrets/{namespace}/{key}` -- encrypted at rest
- `subscribe(namespace)` for change notifications
- Secrets encrypted using BLAKE3-derived key from ExoChain genesis
- Config changes logged to ExoChain; secret reads logged for audit

**Tests**:
- Set and get config roundtrip
- Config change notification delivered
- Secret encrypted at rest
- Unauthorized PID cannot read secrets
- Secret expiry works
- Config changes logged to ExoChain

---

### K5-G2: Auth Agent (Factotum) (~300 lines)

**Source**: Plan 9 Factotum (R11)

**Why K5**: Applications need credential management. The Factotum pattern
prevents agents from holding raw credentials.

**Files**: New `crates/clawft-kernel/src/auth_service.rs`

**Types**:
```rust
/// Centralized credential management service (Plan 9 Factotum pattern).
pub struct AuthService {
    credentials: DashMap<String, StoredCredential>,
    active_tokens: DashMap<String, IssuedToken>,
    config_service: Arc<ConfigService>,
}

pub struct StoredCredential {
    pub name: String,
    pub credential_type: CredentialType,
    pub encrypted_value: Vec<u8>,
    pub allowed_agents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

pub enum CredentialType {
    ApiKey,
    BearerToken,
    Certificate,
    Custom(String),
}

pub struct IssuedToken {
    pub token_id: String,
    pub credential_name: String,
    pub issued_to: Pid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub scope: Vec<String>,
}
```

**Behavior**:
- `register_credential(name, type, value, allowed_agents)` -- store credential
- `request_token(credential_name, requester_pid, scope, ttl)` -- issue scoped token
- Agents never receive raw credentials
- Token expiry enforced; `allowed_agents` checked
- Token issuance and credential access logged to ExoChain
- Credentials stored via `ConfigService.set_secret()` (encrypted at rest)

**Tests**:
- Register credential and request token
- Unauthorized agent cannot request token
- Expired token rejected
- Token scope restricts operations
- Raw credential value never exposed
- Credential rotation: update credential, existing tokens continue working

---

### K5-G3: Per-Agent Tree Views (~300 lines)

**Source**: Plan 9 namespaces (R9)

**Why K5**: Application agents need isolated views of the ResourceTree.
`IpcScope` from K1 provides the permission model; tree views apply it at
the storage layer.

**Files**: Extend `crates/clawft-kernel/src/tree_manager.rs`

**Types**:
```rust
/// Filtered view of the ResourceTree scoped to an agent's capabilities.
pub struct AgentTreeView {
    pub agent_id: String,
    pub pid: Pid,
    pub allowed_paths: Vec<String>,
    pub tree: Arc<TreeManager>,
}
```

**Behavior**:
- `IpcScope::Restricted`: sees only `/agents/{agent_id}/**` and `/kernel/config/**` (read-only)
- `IpcScope::Full`: sees entire tree
- `IpcScope::NamespaceOnly(ns)`: sees `/agents/{agent_id}/**` and `/namespaces/{ns}/**`
- All tree operations pass through view filter
- Unauthorized access returns `KernelError::PermissionDenied`

**Tests**:
- Restricted agent sees only own subtree
- Restricted agent cannot read `/kernel/secrets/`
- Full agent sees entire tree
- Namespace-scoped agent sees correct paths
- Write to unauthorized path returns `PermissionDenied`

---

### K6-G1: Artifact Exchange Protocol (~200 lines)

**Source**: IPFS Bitswap (R17)

**Why K6**: Artifact exchange requires the mesh transport (K6) to transfer
content-addressed blobs between nodes.

**Files**: Modify `crates/clawft-kernel/src/mesh_framing.rs`, extend `artifact_store.rs`

**Changes**:
- Add `FrameType::ArtifactRequest { hash: String }` to mesh framing
- Add `FrameType::ArtifactResponse { hash: String, data: Vec<u8> }` to mesh framing
- Nodes that have an artifact serve it on request
- `artifact_store.request_remote(hash, node_id)` asks mesh peer for artifact
- BLAKE3 hash verified on receipt
- New artifact announcements gossiped via mesh topic

**Tests**:
- Request artifact from remote node
- Hash verified on receipt (mismatch rejected)
- Non-existent artifact returns error
- Artifact gossip announces new hashes

---

### K6-G2: Cross-Node Log Aggregation (~100 lines)

**Why K6**: With mesh networking, logs from multiple nodes should be
queryable from a single point.

**Files**: Extend `crates/clawft-kernel/src/log_service.rs`

**Behavior**:
- Log entries include `node_id` field when received from remote nodes
- `LogQuery` gains `node_id: Option<String>` filter
- Remote log query: `mesh.request()` to peer's `LogService` via `ServiceApi`
- Aggregated results merged by timestamp

**Tests**:
- Remote log entries carry source `node_id`
- Log query filters by `node_id`
- Aggregated query merges results from multiple nodes

---

## P -- Pseudocode

### 1. Artifact Store and Load (K3-G1)

```
fn artifact_store(store: &ArtifactStore, content: &[u8], content_type: ArtifactType) -> String:
    let hash = blake3::hash(content).to_hex().to_string()

    // Deduplication: if hash already exists, increment reference count
    if let Some(existing) = store.artifacts.get(&hash):
        existing.reference_count.fetch_add(1, Ordering::Relaxed)
        return hash

    // Store content in backend
    match &store.backend:
        Memory(map) -> map.insert(hash.clone(), content.to_vec())
        File { base_path } ->
            let dir = base_path.join(&hash[0..2])
            fs::create_dir_all(&dir)
            fs::write(dir.join(&hash), content)

    // Register metadata
    store.artifacts.insert(hash.clone(), StoredArtifact {
        hash: hash.clone(),
        size: content.len() as u64,
        content_type,
        stored_at: Utc::now(),
        reference_count: AtomicU32::new(1),
    })
    store.total_size.fetch_add(content.len() as u64, Ordering::Relaxed)

    // Register in tree
    tree.insert(format!("/kernel/artifacts/{}", hash), metadata)

    hash

fn artifact_load(store: &ArtifactStore, hash: &str) -> Result<Vec<u8>>:
    if !store.artifacts.contains_key(hash):
        return Err(KernelError::NotFound)

    let content = match &store.backend:
        Memory(map) -> map.get(hash).map(|v| v.clone())
        File { base_path } -> fs::read(base_path.join(&hash[0..2]).join(hash)).ok()

    let content = content.ok_or(KernelError::NotFound)?

    // Verify integrity
    let actual_hash = blake3::hash(&content).to_hex().to_string()
    if actual_hash != hash:
        return Err(KernelError::IntegrityError {
            expected: hash.to_string(),
            actual: actual_hash,
        })

    Ok(content)
```

### 2. WeaverEngine CognitiveTick Handler (K3c-G1)

```
fn weaver_tick(engine: &WeaverEngine, tick: &CognitiveTick) -> TickResult:
    let session = engine.session.read()
    let session = match session.as_ref():
        Some(s) => s
        None => return TickResult::Idle  // no active session

    let budget_start = Instant::now()

    // Phase 1: Evaluate current confidence
    let confidence = evaluate_confidence(&engine.knowledge_base, &engine.causal_graph)
    session.confidence = confidence.overall

    // Phase 2: Identify gaps
    let gaps = confidence.domains.iter()
        .filter(|d| d.confidence < d.target)
        .map(|d| ConfidenceGap {
            domain: d.name.clone(),
            current_confidence: d.confidence,
            target_confidence: d.target,
            suggested_sources: suggest_sources(d),
        })
        .collect()
    session.gaps = gaps

    if budget_start.elapsed() > tick.budget:
        return TickResult::BudgetExhausted

    // Phase 3: Ingest next source (if budget remains)
    if let Some(gap) = session.gaps.first():
        if let Some(source) = gap.suggested_sources.first():
            let nodes = ingest_source(source, &engine.causal_graph, &engine.embedding_provider)
            session.sources_ingested.push(source.clone())

            // Meta-Loom: record own decision
            engine.causal_graph.add_event(CausalEvent {
                source: "weaver.meta-loom",
                action: "ingest",
                target: source,
                confidence_before: gap.current_confidence,
            })

    session.tick_count += 1
    TickResult::Progress { confidence: session.confidence, gaps_remaining: session.gaps.len() }

fn ingest_source(source: &str, graph: &CausalGraph, embeddings: &dyn EmbeddingProvider):
    match source:
        s if s.starts_with("git:") ->
            // Parse git log into causal nodes
            let commits = parse_git_log(s.trim_start_matches("git:"))
            for commit in commits:
                let embedding = embeddings.embed(&commit.message).await
                graph.add_node(CausalNode::from_commit(commit, embedding))

        s if s.starts_with("file:") ->
            // Walk file tree into namespace structure
            let files = walk_directory(s.trim_start_matches("file:"))
            let texts: Vec<&str> = files.iter().map(|f| f.content.as_str()).collect()
            let embeddings = embeddings.embed_batch(&texts).await
            for (file, emb) in files.iter().zip(embeddings):
                graph.add_node(CausalNode::from_file(file, emb))

        _ -> log::warn!("Unknown source type: {}", source)
```

### 3. Weaver CLI IPC Handler (K3c-G3)

```
fn handle_weaver_command(engine: &WeaverEngine, cmd: WeaverCommand) -> Result<WeaverResponse>:
    match cmd:
        SessionStart { domain, git_path, context, goal } ->
            // Create new ModelingSession
            let session = ModelingSession {
                id: uuid::new_v4().to_string(),
                domain: domain.clone(),
                started_at: Utc::now(),
                confidence: 0.0,
                gaps: vec![],
                sources_ingested: vec![],
                tick_count: 0,
                budget_remaining: Duration::from_secs(300),  // 5 min default
            }

            // If KB has similar domains, apply learned strategies
            if let Some(strategies) = engine.knowledge_base.strategies_for(&domain):
                session.apply_strategies(strategies)

            // Auto-add git source if provided
            if let Some(path) = git_path:
                engine.add_source(&domain, "git_log", Some(path), false).await?

            engine.sessions.write().insert(domain.clone(), session)
            tree.insert(format!("/kernel/services/weaver/sessions/{}", domain), metadata)
            chain.append("weaver", "weaver.session.start", { domain, context, goal })

            // Meta-Loom: record session creation
            meta_loom_record(engine, &domain, MetaDecisionType::ModelVersionBumped { from: 0, to: 1 },
                "Session initialized", 0.0)

            Ok(WeaverResponse::SessionStarted { domain, session_id: session.id })

        Confidence { domain, edge, verbose } ->
            let session = engine.sessions.read().get(&domain)
                .ok_or(KernelError::NotFound)?
            let report = evaluate_confidence(&engine.knowledge_base, &engine.causal_graph, &domain)
            if let Some(edge_filter) = edge:
                report = report.filter_edge(&edge_filter)
            Ok(WeaverResponse::ConfidenceReport(report))

        Export { domain, min_confidence, output } ->
            let model = engine.export_model(&domain, min_confidence)?
            fs::write(&output, serde_json::to_string_pretty(&model)?)?
            chain.append("weaver", "weaver.export", { domain, path: output, edges: model.causal_edges.len() })
            Ok(WeaverResponse::Exported { path: output, edges: model.causal_edges.len() })

        Import { domain, input } ->
            let content = fs::read_to_string(&input)?
            let model: ExportedModel = serde_json::from_str(&content)?
            engine.import_model(&domain, model)?
            chain.append("weaver", "weaver.import", { domain, path: input })
            Ok(WeaverResponse::Imported { domain })

        MetaStrategies ->
            let strategies = engine.knowledge_base.list_strategies()
            Ok(WeaverResponse::Strategies(strategies))

        MetaExportKb { output } ->
            let kb = engine.knowledge_base.export()?
            fs::write(&output, serde_json::to_string_pretty(&kb)?)?
            Ok(WeaverResponse::KbExported { path: output })

        // ... other command variants
```

### 4. Model Export (K3c-G4)

```
fn export_model(engine: &WeaverEngine, domain: &str, min_confidence: f64) -> Result<ExportedModel>:
    let session = engine.sessions.read().get(domain)
        .ok_or(KernelError::NotFound)?

    // Collect node types from session schema
    let node_types = session.schema.node_types.clone()
    let edge_types = session.schema.edge_types.iter()
        .filter(|e| e.confidence >= min_confidence)
        .cloned()
        .collect()

    // Serialize causal nodes for this domain
    let causal_nodes = engine.causal_graph.nodes_for_domain(domain)
        .map(|n| ExportedCausalNode::from(n))
        .collect()

    // Serialize causal edges above confidence threshold
    let causal_edges = engine.causal_graph.edges_for_domain(domain)
        .filter(|e| e.confidence >= min_confidence)
        .map(|e| ExportedCausalEdge::from(e))
        .collect()

    // Optional HNSW snapshot
    let hnsw_snapshot = engine.hnsw.snapshot_for_domain(domain)
        .map(|snap| HnswSnapshot {
            dimensions: snap.dimensions,
            num_vectors: snap.num_vectors,
            serialized: snap.serialize(),
        })

    Ok(ExportedModel {
        version: "1.0".to_string(),
        domain: domain.to_string(),
        exported_at: Utc::now(),
        confidence: session.confidence,
        node_types,
        edge_types,
        causal_nodes,
        causal_edges,
        hnsw_snapshot,
        metadata: session.metadata.clone(),
    })

fn import_model(engine: &WeaverEngine, domain: &str, model: ExportedModel) -> Result<()>:
    // Verify schema version
    if !is_compatible_version(&model.version):
        return Err(KernelError::IncompatibleVersion { expected: "1.x", got: model.version })

    // Create session pre-populated with learned structure
    let session = ModelingSession::from_export(&model)
    engine.sessions.write().insert(domain.to_string(), session)

    // Load causal nodes and edges into graph
    for node in model.causal_nodes:
        engine.causal_graph.add_node(CausalNode::from_export(node, domain))
    for edge in model.causal_edges:
        engine.causal_graph.add_edge(CausalEdge::from_export(edge, domain))

    // Restore HNSW if snapshot provided
    if let Some(snap) = model.hnsw_snapshot:
        engine.hnsw.restore_snapshot(domain, snap.serialized)?

    // Meta-Loom: record import
    meta_loom_record(engine, domain, MetaDecisionType::ModelVersionBumped { from: 0, to: 1 },
        "Imported from exported model", model.confidence)

    Ok(())
```

### 5. Meta-Loom Recording (K3c-G5)

```
fn meta_loom_record(
    engine: &WeaverEngine,
    domain: &str,
    decision: MetaDecisionType,
    rationale: &str,
    confidence_before: f64,
):
    let event = MetaLoomEvent {
        session_domain: domain.to_string(),
        decision_type: decision.clone(),
        confidence_before,
        confidence_after: None,  // filled in by next confidence eval
        rationale: rationale.to_string(),
        timestamp: Utc::now(),
    }

    // Record in CausalGraph under meta-loom namespace
    let node_id = engine.causal_graph.add_node(CausalNode {
        namespace: format!("meta-loom/{}", domain),
        node_type: "meta_decision".to_string(),
        payload: serde_json::to_value(&event).unwrap(),
        timestamp: Utc::now(),
    })

    // Link to previous meta-loom event for this domain
    if let Some(prev) = engine.causal_graph.latest_node_in("meta-loom/{}", domain):
        engine.causal_graph.add_edge(CausalEdge {
            from: prev.id,
            to: node_id,
            edge_type: "FollowedBy".to_string(),
            confidence: 1.0,
        })

    // Emit impulse for significant decisions
    match &decision:
        MetaDecisionType::ModelVersionBumped { .. } ->
            engine.impulse_bus.emit(ImpulseType::Custom(0x32))
        MetaDecisionType::SourceAdded { .. } ->
            engine.impulse_bus.emit(ImpulseType::Custom(0x33))
        _ -> {}

    // Check if this decision improved confidence (for KB learning)
    // Deferred: confidence_after filled in by next tick, then:
    //   if confidence_after > confidence_before:
    //     engine.knowledge_base.record_successful_strategy(decision, domain)

fn knowledge_base_learn(kb: &WeaverKnowledgeBase, event: &MetaLoomEvent):
    if let Some(after) = event.confidence_after:
        if after > event.confidence_before:
            let improvement = after - event.confidence_before
            kb.record_strategy(StrategyPattern {
                decision_type: event.decision_type.variant_name(),
                context: event.session_domain.clone(),
                improvement,
                timestamp: event.timestamp,
            })
```

### 6. Config Change Notification (K5-G1)

```
fn config_set(service: &ConfigService, namespace: &str, key: &str, value: Value, pid: Pid):
    let path = format!("/kernel/config/{}/{}", namespace, key)

    // Get old value for change notification
    let old_value = service.tree.get(&path).map(|node| node.payload.clone())

    // Write to tree
    service.tree.insert(&path, serde_json::to_vec(&value)?)

    // Log to ExoChain
    chain.append("config", "config.changed", {
        namespace, key, changed_by: pid,
    })

    // Notify subscribers
    let change = ConfigChange {
        namespace: namespace.to_string(),
        key: key.to_string(),
        old_value,
        new_value: Some(value),
        changed_by: pid,
        timestamp: Utc::now(),
    }

    if let Some(subs) = service.subscribers.get(namespace):
        let mut dead = vec![]
        for (i, sender) in subs.iter().enumerate():
            if sender.try_send(change.clone()).is_err():
                dead.push(i)
        // Remove dead subscribers (reverse order to preserve indices)
        for i in dead.into_iter().rev():
            subs.remove(i)

fn secret_set(service: &ConfigService, namespace: &str, key: &str, value: &[u8], scoped_to: Vec<Pid>):
    let path = format!("/kernel/secrets/{}/{}", namespace, key)

    // Encrypt at rest
    let encrypted = encrypt_aead(value, &service.encryption_key)

    // Store encrypted value
    service.tree.insert(&path, encrypted)

    // Store access control metadata
    let ref_data = SecretRef {
        namespace: namespace.to_string(),
        key: key.to_string(),
        expires_at: Utc::now() + Duration::from_secs(86400),  // 24h default
        scoped_to,
    }
    service.tree.insert(&format!("{}/_meta", path), serde_json::to_vec(&ref_data)?)

    // Audit log
    chain.append("config", "secret.stored", { namespace, key })
```

### 7. Auth Token Scoping (K5-G2)

```
fn request_token(
    auth: &AuthService,
    credential_name: &str,
    requester_pid: Pid,
    scope: Vec<String>,
    ttl: Duration,
) -> Result<IssuedToken>:
    // Look up credential
    let cred = auth.credentials.get(credential_name)
        .ok_or(KernelError::NotFound)?

    // Check authorization
    let requester_agent_id = process_table.get(requester_pid)?.agent_id
    if !cred.allowed_agents.contains(&requester_agent_id):
        chain.append("auth", "auth.denied", {
            credential: credential_name,
            requester: requester_agent_id,
        })
        return Err(KernelError::PermissionDenied)

    // Issue scoped token (never expose raw credential)
    let token = IssuedToken {
        token_id: uuid::new_v4().to_string(),
        credential_name: credential_name.to_string(),
        issued_to: requester_pid,
        issued_at: Utc::now(),
        expires_at: Utc::now() + ttl,
        scope,
    }

    auth.active_tokens.insert(token.token_id.clone(), token.clone())

    // Audit log
    chain.append("auth", "auth.token.issued", {
        token_id: token.token_id,
        credential: credential_name,
        issued_to: requester_agent_id,
        scope: token.scope,
        ttl_secs: ttl.as_secs(),
    })

    Ok(token)
```

---

## A -- Architecture

### Component Integration Diagram

```
+------------------------------------------------------------------+
|                      EXISTING KERNEL (K0-K6)                      |
|                                                                   |
|  +----------+  +----------+  +-------------+  +----------------+ |
|  | Wasm     |  | Tree     |  | ChainManager|  | Mesh Transport | |
|  | Runner   |  | Manager  |  | (ExoChain)  |  | (K6)           | |
|  | (K3)     |  | (K1)     |  |             |  |                | |
|  +----+-----+  +----+-----+  +------+------+  +-------+--------+ |
|       |              |               |                 |          |
+-------|--------------|---------------|-----------------|----------+
        |              |               |                 |
+-------|--------------|---------------|-----------------|----------+
| 08c   |              |               |                 |          |
| FILLS |              |               |                 |          |
|  +----v---------+    |               |          +------v--------+ |
|  | Artifact     |    |               |          | Artifact      | |
|  | Store        |    |               |          | Exchange      | |
|  | (K3-G1)      |    |               |          | Protocol      | |
|  | BLAKE3 hash  |    |               |          | (K6-G1)       | |
|  +--------------+    |               |          +---------------+ |
|                      |               |                            |
|  +-------------------v--+  +---------v--------+                   |
|  | Config Service       |  | Auth Service     |                   |
|  | (K5-G1)              |  | (K5-G2)          |                   |
|  | Tree-backed config   |  | Factotum pattern |                   |
|  | Encrypted secrets    |  | Scoped tokens    |                   |
|  +----------------------+  +------------------+                   |
|                                                                   |
|  +----------------------+  +------------------+                   |
|  | Agent Tree Views     |  | Log Aggregation  |                   |
|  | (K5-G3)              |  | (K6-G2)          |                   |
|  | IpcScope filtering   |  | Cross-node merge |                   |
|  +----------------------+  +------------------+                   |
|                                                                   |
|  +----------------------+  +------------------+                   |
|  | WeaverEngine         |  | EmbeddingProvider|                   |
|  | (K3c-G1)             |  | (K3c-G2)         |                   |
|  | Modeling sessions    |  | Mock + ONNX      |                   |
|  | CognitiveTick loop   |  | backends         |                   |
|  | Multi-source ingest  |  |                  |                   |
|  +----------+-----------+  +------------------+                   |
|             |                                                     |
|  +----------v-----------+  +------------------+                   |
|  | Weaver CLI Commands  |  | Meta-Loom        |                   |
|  | (K3c-G3)             |  | (K3c-G5)         |                   |
|  | Daemon socket IPC    |  | Self-evolution    |                   |
|  | session/confidence/  |  | tracking in       |                   |
|  | export/source/meta   |  | CausalGraph       |                   |
|  +----------------------+  +------------------+                   |
|                                                                   |
|  +----------------------+                                         |
|  | weave-model.json     |                                         |
|  | Export + Import       |                                         |
|  | (K3c-G4)             |                                         |
|  | Edge deployment      |                                         |
|  +----------------------+                                         |
+------------------------------------------------------------------+
```

### Component Integration Map

| Gap ID | New Component | Files | Depends On | Registered As | Tree Path |
|--------|--------------|-------|-----------|---------------|-----------|
| K3-G1 | `ArtifactStore` | new `artifact_store.rs` | `blake3`, `TreeManager` | `SystemService::Core` | `/kernel/artifacts/{hash}` |
| K3c-G1 | `WeaverEngine` | new `weaver.rs` or `crates/ecc-weaver/` | `CausalGraph`, `HnswService`, `EmbeddingProvider` | `SystemService::Core` | `/kernel/services/weaver` |
| K3c-G2 | `EmbeddingProvider` | new `embedding.rs` | -- (trait only) | -- | -- |
| K3c-G3 | `WeaverCommand` IPC handler | CLI commands dir, `weaver.rs` | `WeaverEngine`, daemon socket | CLI extension | -- |
| K3c-G4 | `ExportedModel` + import | `weaver_export.rs` or `weaver.rs` | `WeaverEngine`, `CausalGraph`, `HnswService` | -- | -- |
| K3c-G5 | `MetaLoomEvent` tracking | `weaver.rs` | `CausalGraph`, `WeaverKnowledgeBase` | -- | `meta-loom/{domain}` |
| K5-G1 | `ConfigService` | new `config_service.rs` | `TreeManager`, `ChainManager` | `SystemService::Core` | `/kernel/config/{ns}/{key}` |
| K5-G2 | `AuthService` | new `auth_service.rs` | `ConfigService`, `ChainManager` | `SystemService::Core` | `/kernel/services/auth` |
| K5-G3 | `AgentTreeView` | extend `tree_manager.rs` | `TreeManager`, `AgentCapabilities` | Part of `TreeManager` | -- |
| K6-G1 | Artifact exchange frames | `mesh_framing.rs`, `artifact_store.rs` | `ArtifactStore`, mesh transport | `FrameType` extension | -- |
| K6-G2 | Cross-node log aggregation | extend `log_service.rs` | `LogService` (08b), mesh `ServiceApi` | Extension of `LogService` | -- |

### Boot Sequence (08c additions)

```
Existing boot (K0-K6):
  1-12. [unchanged]

08b additions (positions 13-17, 19):
  [see 08b-reliable-ipc.md]

08c additions (when os-patterns enabled):
  20. ConfigService          (K5-G1, needs TreeManager)
  21. AuthService            (K5-G2, needs ConfigService)
  22. ArtifactStore          (K3-G1, needs TreeManager, os-full only)
  [WeaverEngine registers separately when ecc feature enabled]
```

---

## R -- Refinement

### Performance Considerations

- `ArtifactStore` file backend: two-level directory sharding
  (`{hash[0..2]}/{hash}`) avoids filesystem limits on directory entries
- `ArtifactStore` deduplication: hash check is O(1) via `DashMap`
- `WeaverEngine` CognitiveTick: budget-aware, yields when budget exhausted
  so it never blocks the kernel tick cycle
- `WeaverEngine` embedding: lazy batch embedding via `embed_batch()` reduces
  round-trips to embedding backend
- `ConfigService` change notifications: non-blocking `try_send()` to
  subscribers, dead subscribers cleaned up automatically
- `AuthService` token lookup: `DashMap` for O(1) validation
- `AgentTreeView`: filter applied at query time, not materialized -- no
  storage overhead

### Security Considerations

- `ConfigService` encrypts secrets using BLAKE3-derived key from ExoChain
  genesis block -- key rotation requires ExoChain migration
- `AuthService` never exposes raw credentials: agents receive scoped,
  time-limited tokens only
- `AuthService` logs all credential access and token issuance to ExoChain
  for tamper-evident audit
- `AgentTreeView` enforces capability-based access at storage layer --
  defense in depth (IPC scope + tree view)
- `ArtifactStore` verifies BLAKE3 hash on every load -- tampered content
  detected immediately
- Artifact exchange verifies hash on receipt -- man-in-the-middle detected
- Config changes logged to ExoChain -- unauthorized changes detectable

### Migration Path

- `WasmToolRunner::load_module()` gains optional hash-based loading; existing
  path-based loading preserved for backward compatibility
- `LogQuery` gains `node_id` with default `None` -- backward compatible
- `FrameType` gains new variants -- exhaustive matches need updating
- `TreeManager` API unchanged; `AgentTreeView` wraps it

### Testing Strategy

- Each gap is independently testable
- Integration test flows:
  - Store WASM module -> load by hash -> verify integrity
  - Start Weaver session -> tick -> ingest source -> export model -> import on new domain
  - CLI `weaver ecc session start` -> `confidence` -> `source add` -> `export` -> `import`
  - Meta-Loom: session start -> source added -> confidence improved -> strategy learned in KB
  - WeaverKnowledgeBase: learn pattern in domain A -> start domain B -> KB strategy applied
  - Set config -> subscriber receives notification -> ExoChain event
  - Register credential -> request token -> use token -> expire
  - Restricted agent tries unauthorized tree path -> PermissionDenied
  - Store artifact -> gossip -> remote node requests -> exchange -> verify
- `ArtifactStore` tested with both Memory and File backends
- `WeaverEngine` tested with `MockEmbeddingProvider`
- K6 gaps use `InMemoryTransport` from K6
- All tests independent of `mesh` feature (single-node) except K6 gaps

### Dependency Minimization

- `blake3` is the only new external dependency (artifact store, `os-full` only)
- `ort` (ONNX Runtime) is optional, behind `onnx-embeddings` feature
- All other code uses workspace-existing crates: `dashmap`, `tokio`, `chrono`,
  `serde`, `uuid`
- `os-patterns` feature does not pull `blake3` or `ort`
- `os-full` adds only `blake3`

---

## C -- Completion

### Exit Criteria

#### K3 WASM Sandbox Gate (P2)
- [ ] Artifacts stored by BLAKE3 hash
- [ ] Artifact retrieval verifies hash on load
- [ ] Duplicate artifacts deduplicated automatically
- [ ] WASM modules loaded from artifact store by hash
- [ ] File backend writes to correct path structure
- [ ] Reference counting and garbage collection work

#### K3c ECC Cognitive Substrate Gate (P1)
- [ ] WeaverEngine registers as SystemService at boot (ecc feature)
- [ ] Modeling session start/stop via `weaver ecc` CLI
- [ ] Confidence evaluation produces gap analysis with suggestions
- [ ] Model export produces valid weave-model.json
- [ ] Git log ingestion creates causal nodes and edges
- [ ] File tree ingestion creates namespace structure
- [ ] Meta-Loom records Weaver's own modeling decisions as causal events
- [ ] CognitiveTick handler respects budget allocation
- [ ] EmbeddingProvider trait defined with mock implementation
- [ ] Cross-domain WeaverKnowledgeBase persists strategy patterns
- [ ] ONNX embedding backend (behind `onnx-embeddings` feature flag)
- [ ] CLI `weaver ecc session start` creates ModelingSession in WeaverEngine
- [ ] CLI `weaver ecc confidence` returns current confidence report
- [ ] CLI `weaver ecc export` produces valid weave-model.json file
- [ ] CLI `weaver ecc source add` registers source and triggers ingestion
- [ ] CLI `weaver ecc import` restores session from exported model
- [ ] CLI `weaver ecc meta strategies` returns learned strategies from KB
- [ ] CLI `weaver ecc meta export-kb` exports cross-domain knowledge base
- [ ] Export + import roundtrip preserves causal graph structure
- [ ] Export filters edges below `min_confidence` threshold
- [ ] Meta-Loom events stored under `meta-loom/{domain}` namespace in CausalGraph
- [ ] Successful strategies (confidence improvements) recorded in WeaverKnowledgeBase
- [ ] New sessions in similar domains apply learned strategies from KB
- [ ] ImpulseType::Custom(0x32) emitted on model version bump
- [ ] ImpulseType::Custom(0x33) emitted on source request

#### K5 App Framework Gate (P2)
- [ ] Config service stores and retrieves configuration
- [ ] Config change notifications delivered to subscribers
- [ ] Config changes logged to ExoChain
- [ ] Secret service encrypts at rest
- [ ] Secret service delivers scoped, time-limited access
- [ ] Unauthorized PID cannot read secrets
- [ ] Auth service registers and manages credentials
- [ ] Auth service issues scoped, time-limited tokens
- [ ] Agents never receive raw credentials
- [ ] Credential access logged to ExoChain
- [ ] Per-agent tree views filter by capabilities
- [ ] Restricted agents cannot see unauthorized tree paths
- [ ] Namespace-scoped agents see correct paths
- [ ] Write to unauthorized tree path returns `PermissionDenied`

#### K6 Mesh Networking Gate (P2)
- [ ] Artifact exchange protocol transfers between mesh nodes
- [ ] Hash mismatch on receipt is rejected
- [ ] Artifact gossip announces new hashes
- [ ] Remote log entries carry source `node_id`
- [ ] Log query filters by `node_id`
- [ ] Aggregated query merges from multiple nodes

#### Phase Gate
- [ ] All K3 exit criteria pass
- [ ] All K3c exit criteria pass
- [ ] All K5 exit criteria pass
- [ ] All K6 exit criteria pass
- [ ] All existing tests pass (843+ baseline)
- [ ] New tests: target 80+ for this phase
- [ ] Clippy clean for all new code (`scripts/build.sh clippy`)
- [ ] Feature gated behind `os-patterns` / `os-full` / `ecc`
- [ ] `blake3` only required when `os-full` enabled
- [ ] `ort` only required when `onnx-embeddings` enabled

### Testing Verification Commands

```bash
# Build with OS patterns feature
scripts/build.sh native --features os-patterns

# Build with full OS features (includes blake3)
scripts/build.sh native --features os-full

# Build with ECC features
scripts/build.sh native --features ecc

# Run content + ops tests
scripts/build.sh test -- --features os-full,ecc -p clawft-kernel

# Verify base build unchanged
scripts/build.sh check

# Clippy
scripts/build.sh clippy
```

### Implementation Order

```
Week 18 (parallel with 08a/08b):
  K3-G1: Content-addressed artifact store (no IPC dependency)
  K3c-G2: EmbeddingProvider trait (no dependencies)

Week 19:
  K3c-G1: WeaverEngine SystemService (needs EmbeddingProvider)
  K3c-G5: Meta-Loom integration (part of WeaverEngine, built together)

Week 20:
  K3c-G3: Weaver CLI commands wired through daemon socket
  K3c-G4: weave-model.json export + import
  K5-G1: Config and secrets service

Week 21:
  K5-G2: Auth agent (Factotum) (needs ConfigService)
  K5-G3: Per-agent tree views
  K6-G1: Artifact exchange protocol (needs ArtifactStore + mesh)

Week 22:
  K6-G2: Cross-node log aggregation (needs LogService from 08b)
  Integration testing across all 08c gaps (including Weaver end-to-end)
  Review and polish
```
