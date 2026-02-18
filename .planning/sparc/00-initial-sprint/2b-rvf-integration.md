# SPARC Implementation Plan: Stream 2B - RVF Integration

**Timeline**: Week 7-11
**Owned Crates**: `clawft-core` (ruvector integration, vector memory, intelligent routing)
**Dependencies**: Phase 1 complete, rvf-runtime + ruvector crates available

---

## 1. Agent Instructions

### Critical Planning Documents (MUST READ FIRST)
```
.planning/04-rvf-integration.md           # FULL RVF specification (50+ pages)
.planning/05-ruvector-crates.md           # FULL ruvector crate map + APIs
.planning/02-technical-requirements.md    # Pipeline traits, feature flags
.planning/03-development-guide.md         # Stream 2B timeline
```

**CRITICAL**: ADR-026 in `04-rvf-integration.md` specifies 3-tier model routing with Agent Booster (WASM). This MUST be implemented.

### Python Source Files (Reference Only - No Direct Port)
```
# Python nanobot does NOT have RVF/ruvector integration
# These files show existing memory/routing patterns to replace:
repos/nanobot/nanobot/memory/store.py                  # File-based memory (replace with RVF)
repos/nanobot/nanobot/agent/router.py                  # Static routing (replace with intelligent)
```

### Module Structure
```
clawft-core/
├── Cargo.toml                    # Add rvf + ruvector dependencies with feature flags
├── src/
│   ├── memory/
│   │   ├── mod.rs                # MemoryStore trait + file backend
│   │   ├── file_store.rs         # Existing MEMORY.md backend (no RVF)
│   │   ├── rvf_store.rs          # NEW: RvfVectorStore (feature = "rvf")
│   │   └── hybrid_store.rs       # NEW: Hybrid file + RVF (feature = "rvf")
│   ├── routing/
│   │   ├── mod.rs                # Router trait
│   │   ├── static_router.rs      # Existing static routing (no RVF)
│   │   └── intelligent_router.rs # NEW: Learned routing (feature = "rvf", "intelligent-routing")
│   ├── session/
│   │   ├── mod.rs                # Session trait
│   │   ├── manager.rs            # Existing session manager
│   │   └── indexer.rs            # NEW: RVF session indexing (feature = "rvf")
│   └── embeddings/
│       ├── mod.rs                # NEW: Embedding trait
│       ├── api_embeddings.rs     # NEW: LLM provider embeddings via clawft-llm
│       └── hash_embeddings.rs    # NEW: HashEmbedding fallback
├── tests/
│   ├── memory_rvf_tests.rs       # RVF memory tests
│   ├── routing_tests.rs          # Intelligent routing tests
│   └── session_indexing_tests.rs # Session indexing tests
```

---

## 2. Specification

### 2.1 Week 7: Dependencies & Feature Flags

#### Cargo.toml Dependencies (Feature-Gated)
```toml
[dependencies]
# Core dependencies (always enabled)
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.42", features = ["full"] }
tracing = "0.1"
anyhow = "1.0"

# RVF dependencies (feature-gated)
rvf-runtime = { version = "0.3", optional = true }
rvf-types = { version = "0.3", optional = true }
rvf-index = { version = "0.3", optional = true }
rvf-adapters-agentdb = { version = "0.3", optional = true }

# ruvector dependencies (feature-gated)
ruvector-core = { version = "0.1", optional = true }
ruvector-attention = { version = "0.1", optional = true }
ruvector-temporal-tensor = { version = "0.1", optional = true }

# Optional enhanced features
sona = { version = "0.1", optional = true }
ruvllm = { version = "0.1", optional = true }
tiny-dancer = { version = "0.1", optional = true }

[features]
default = []
rvf = ["dep:rvf-runtime", "dep:rvf-types", "dep:rvf-index"]
rvf-agentdb = ["rvf", "dep:rvf-adapters-agentdb", "dep:ruvector-core"]
intelligent-routing = ["rvf", "ruvllm", "tiny-dancer"]
sona = ["dep:sona"]
attention = ["dep:ruvector-attention"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]
```

#### Feature Flag Strategy
- **No RVF**: File-based memory (MEMORY.md), static routing
- **rvf**: Progressive HNSW indexing, vector search
- **rvf-agentdb**: AgentDB backend for rvf (production-ready)
- **intelligent-routing**: Learned routing via POLICY_KERNEL + COST_CURVE
- **sona**: SONA embeddings (experimental)
- **attention**: Attention mechanism overlays (experimental)
- **temporal-tensor**: Time-series indexing (experimental)

### 2.2 Week 8: Memory + Router

#### RvfVectorStore (clawft-core/src/memory/rvf_store.rs)
**Purpose**: Semantic search over MEMORY.md using progressive HNSW indexing

**RVF Segment Types**:
```
MEMORY segment:
  - Key: "memory/{uuid}"
  - Embedding: 384-dim vector (from HashEmbedding or LLM provider)
  - Metadata: { timestamp, tags, source: "user" | "agent" }
  - Content: Original markdown text

HNSW segment (progressive):
  - Layer: 0-15 (adaptive based on dataset size)
  - Neighbors: Max 16 per node
  - Distance: Cosine similarity
  - Updates: Incremental on new memory writes
```

**API**:
```rust
pub struct RvfVectorStore {
    runtime: RvfRuntime,
    index: HnswIndex,
    embedder: Box<dyn Embedder>,
}

impl RvfVectorStore {
    pub async fn new(data_dir: PathBuf, embedder: Box<dyn Embedder>) -> Result<Self>;

    pub async fn add_memory(&mut self, text: &str, tags: &[String]) -> Result<String>;

    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<MemoryMatch>>;

    pub async fn progressive_index(&mut self) -> Result<IndexStats>;
}

pub struct MemoryMatch {
    pub id: String,
    pub text: String,
    pub score: f32,
    pub tags: Vec<String>,
    pub timestamp: u64,
}
```

**Progressive Indexing**:
- Initial write: Store MEMORY segment only (no HNSW)
- Background task: Build HNSW incrementally (10 items/batch)
- Query: Fall back to brute-force if HNSW incomplete
- Checkpoint: Save HNSW state every 100 additions

#### IntelligentRouter (clawft-core/src/routing/intelligent_router.rs)
**Purpose**: Learned routing using POLICY_KERNEL and COST_CURVE segments

**RVF Segment Types**:
```
POLICY_KERNEL segment:
  - Key: "policy/{pattern_hash}"
  - Embedding: Query pattern embedding
  - Metadata: { model_tier: 1|2|3, success_rate, avg_latency, last_used }
  - Content: JSON policy { "pattern": "...", "tier": 3, "reason": "..." }

COST_CURVE segment:
  - Key: "cost/{model_name}/{timestamp}"
  - Metadata: { model: "haiku" | "sonnet" | "opus", latency_ms, tokens, cost_usd }
  - Content: JSON { "input_tokens": 1234, "output_tokens": 567, ... }
```

**API**:
```rust
pub struct IntelligentRouter {
    policy_store: RvfVectorStore,
    pattern_index: RvfVectorStore,
    providers: HashMap<String, OpenAiCompatProvider>,
    cost_tracker: CostTracker,
}

impl IntelligentRouter {
    pub async fn route_request(&self, prompt: &str, context: &Context) -> Result<RoutingDecision>;

    pub async fn record_cost(&mut self, model: &str, usage: Usage) -> Result<()>;

    pub async fn update_policy(&mut self, pattern: &str, tier: u8, feedback: Feedback) -> Result<()>;

    pub async fn get_cost_stats(&self, timeframe: Duration) -> Result<CostStats>;
}

pub struct RoutingDecision {
    pub tier: u8,  // 1 = Agent Booster (WASM), 2 = Haiku, 3 = Sonnet/Opus
    pub model: String,
    pub reason: String,
    pub complexity_score: f32,
}
```

**Routing Algorithm** (ADR-026):
```
1. Check for [AGENT_BOOSTER_AVAILABLE] tag from clawft-llm
   - If present: Use Tier 1 (WASM, <1ms, $0)

2. Else: Compute complexity score
   - Pattern match: Check POLICY_KERNEL for similar queries
   - Heuristics: Token count, code blocks, reasoning keywords
   - Complexity score: 0.0 (simple) to 1.0 (complex)

3. Route based on score:
   - <0.30: Tier 2 (Haiku, ~500ms, $0.0002)
   - ≥0.30: Tier 3 (Sonnet/Opus, 2-5s, $0.003-0.015)

4. Record in COST_CURVE
5. Update POLICY_KERNEL with success/failure feedback
```

### 2.3 Week 9: Session Indexing

#### Session Indexer (clawft-core/src/session/indexer.rs)
**Purpose**: Index conversation turns for semantic retrieval

**RVF Segment Types**:
```
SESSION segment:
  - Key: "session/{session_id}/turn/{turn_id}"
  - Embedding: Turn content embedding (user + assistant)
  - Metadata: { timestamp, session_id, role: "user" | "assistant", model }
  - Content: JSON { "user": "...", "assistant": "...", "tools": [...] }

HNSW segment (progressive):
  - Index session turns for semantic similarity
  - Used for context retrieval in long conversations
```

**API**:
```rust
pub struct SessionIndexer {
    runtime: RvfRuntime,
    index: HnswIndex,
    embedder: Box<dyn Embedder>,
}

impl SessionIndexer {
    pub async fn index_turn(&mut self, session_id: &str, turn: &ConversationTurn) -> Result<()>;

    pub async fn search_turns(&self, query: &str, session_id: Option<&str>, top_k: usize) -> Result<Vec<TurnMatch>>;

    pub async fn get_session_context(&self, session_id: &str, current_turn: usize, max_turns: usize) -> Result<Vec<ConversationTurn>>;
}

pub struct TurnMatch {
    pub session_id: String,
    pub turn_id: usize,
    pub user_message: String,
    pub assistant_message: String,
    pub score: f32,
    pub timestamp: u64,
}
```

**Indexing Strategy**:
- Index on session save (after each turn)
- Progressive HNSW build (background task)
- TTL: 30 days (configurable)

### 2.4 Week 10: Learned Routing

#### COST_CURVE Integration
**Purpose**: Track model costs + performance over time

**API**:
```rust
pub struct CostTracker {
    runtime: RvfRuntime,
}

impl CostTracker {
    pub async fn record_usage(&mut self, model: &str, usage: Usage, latency: Duration) -> Result<()>;

    pub async fn get_model_stats(&self, model: &str, timeframe: Duration) -> Result<ModelStats>;

    pub async fn optimize_routing(&self, budget: f32, latency_target: Duration) -> Result<RoutingPolicy>;
}

pub struct ModelStats {
    pub total_calls: u64,
    pub total_cost: f32,
    pub avg_latency: Duration,
    pub p95_latency: Duration,
    pub success_rate: f32,
}
```

**Feedback Loop**:
```
User feedback → update_policy()
  - Success: Increase tier confidence for pattern
  - Failure: Decrease tier confidence, try higher tier next time
  - Store in POLICY_KERNEL as CRUD update
```

### 2.5 Week 11: Audit + Migration

#### Witness Log (clawft-core/src/audit/witness.rs)
**Purpose**: Cryptographic audit trail via WITNESS segments

**RVF Segment Types**:
```
WITNESS segment:
  - Key: "witness/{event_id}"
  - Metadata: { event_type, timestamp, actor }
  - Content: JSON event data
  - Signature: Ed25519 signature over content
  - Chain: Links to previous WITNESS (Merkle chain)
```

**API**:
```rust
pub struct WitnessLog {
    runtime: RvfRuntime,
    signing_key: SigningKey,
}

impl WitnessLog {
    pub async fn record_event(&mut self, event: AuditEvent) -> Result<String>;

    pub async fn verify_chain(&self) -> Result<bool>;

    pub async fn query_events(&self, filter: EventFilter) -> Result<Vec<AuditEvent>>;
}

pub enum AuditEvent {
    MemoryWrite { key: String, value: String },
    PolicyUpdate { pattern: String, tier: u8 },
    SessionTurn { session_id: String, turn_id: usize },
    CostRecord { model: String, cost: f32 },
}
```

#### First-Startup Indexing
**Purpose**: Index existing MEMORY.md on first run

**Algorithm**:
```
1. Check for .rvf/indexed marker file
2. If not exists:
   a. Read MEMORY.md line-by-line
   b. Parse markdown entries
   c. Generate embeddings (batch of 10)
   d. Write MEMORY segments to RVF
   e. Build HNSW index (progressive)
   f. Write .rvf/indexed marker
3. Else: Skip indexing
```

**Performance**:
- Target: Index 1000 memories in <30 seconds
- Batching: 10 embeddings per API call (if using LLM provider)
- Fallback: HashEmbedding if provider unavailable

### 2.6 Embedding Strategy

#### Embedder Trait (clawft-core/src/embeddings/mod.rs)
```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
}
```

#### API-Based Embeddings (Option A - MVP)
```rust
pub struct ApiEmbedder {
    provider: Arc<dyn Provider>, // From clawft-llm
}

impl Embedder for ApiEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.provider.embedding(text).await
    }

    fn dimension(&self) -> usize {
        384 // Provider-specific
    }
}
```

**Provider Endpoint**:
- OpenAI: POST /v1/embeddings (model: text-embedding-3-small)
- Anthropic: No native embeddings (fallback to HashEmbedding)
- Custom: Provider-specific embedding endpoint

#### HashEmbedding Fallback (clawft-core/src/embeddings/hash_embeddings.rs)
```rust
pub struct HashEmbedder {
    dimension: usize,
}

impl Embedder for HashEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // SimHash or MinHash algorithm
        // Fast, deterministic, no API call
        Ok(simhash(text, self.dimension))
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}
```

**When to Use**:
- Offline testing
- Provider doesn't support embeddings
- Low-latency requirement (<5ms)

---

## 3. Pseudocode

### 3.1 RvfVectorStore Implementation
```rust
pub struct RvfVectorStore {
    runtime: RvfRuntime,
    index: HnswIndex,
    embedder: Box<dyn Embedder>,
    indexing_task: Option<JoinHandle<()>>,
}

impl RvfVectorStore {
    pub async fn new(data_dir: PathBuf, embedder: Box<dyn Embedder>) -> Result<Self> {
        let runtime = RvfRuntime::open(&data_dir.join(".rvf"))?;
        let index = HnswIndex::new(embedder.dimension(), 16, CosineSimilarity);

        // Start progressive indexing background task
        let indexing_task = Some(tokio::spawn(progressive_indexing(runtime.clone(), index.clone())));

        Ok(Self { runtime, index, embedder, indexing_task })
    }

    pub async fn add_memory(&mut self, text: &str, tags: &[String]) -> Result<String> {
        // 1. Generate embedding
        let embedding = self.embedder.embed(text).await?;

        // 2. Create MEMORY segment
        let id = Uuid::new_v4().to_string();
        let segment = Segment::new(
            format!("memory/{}", id),
            Some(embedding.clone()),
            json!({
                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                "tags": tags,
                "source": "user"
            }),
            text.to_string()
        );

        // 3. Write to RVF
        self.runtime.write_segment(segment).await?;

        // 4. Add to HNSW index (progressive)
        self.index.insert_deferred(id.clone(), embedding);

        Ok(id)
    }

    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<MemoryMatch>> {
        // 1. Generate query embedding
        let query_embedding = self.embedder.embed(query).await?;

        // 2. Search HNSW index
        let neighbors = if self.index.is_complete() {
            self.index.search(&query_embedding, top_k)?
        } else {
            // Fallback: Brute-force search over MEMORY segments
            self.brute_force_search(&query_embedding, top_k).await?
        };

        // 3. Fetch MEMORY segments
        let mut matches = Vec::new();
        for (id, score) in neighbors {
            let segment = self.runtime.read_segment(&format!("memory/{}", id)).await?;
            matches.push(MemoryMatch {
                id,
                text: segment.content,
                score,
                tags: segment.metadata["tags"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect(),
                timestamp: segment.metadata["timestamp"].as_u64().unwrap(),
            });
        }

        Ok(matches)
    }

    async fn brute_force_search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<(String, f32)>> {
        // Iterate all MEMORY segments, compute cosine similarity
        let segments = self.runtime.query_segments("memory/*").await?;
        let mut scored: Vec<(String, f32)> = segments
            .into_iter()
            .filter_map(|seg| {
                seg.embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(query_embedding, emb);
                    (seg.key.strip_prefix("memory/").unwrap().to_string(), score)
                })
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(top_k);
        Ok(scored)
    }

    pub async fn progressive_index(&mut self) -> Result<IndexStats> {
        // Called by background task
        let pending = self.index.pending_insertions();
        if pending.is_empty() {
            return Ok(IndexStats { indexed: self.index.len(), pending: 0 });
        }

        // Process 10 items per batch
        let batch_size = 10;
        for batch in pending.chunks(batch_size) {
            for (id, embedding) in batch {
                self.index.insert(id.clone(), embedding.clone())?;
            }
            self.runtime.checkpoint().await?; // Save HNSW state
        }

        Ok(IndexStats { indexed: self.index.len(), pending: 0 })
    }
}

async fn progressive_indexing(runtime: RvfRuntime, index: HnswIndex) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        if let Err(e) = index.process_pending().await {
            tracing::warn!("Progressive indexing error: {}", e);
        }
    }
}
```

### 3.2 IntelligentRouter Implementation
```rust
pub struct IntelligentRouter {
    policy_store: RvfVectorStore,
    pattern_index: RvfVectorStore,
    providers: HashMap<String, OpenAiCompatProvider>,
    cost_tracker: CostTracker,
}

impl IntelligentRouter {
    pub async fn route_request(&self, prompt: &str, context: &Context) -> Result<RoutingDecision> {
        // 1. Check for [AGENT_BOOSTER_AVAILABLE] tag
        if context.tags.contains("AGENT_BOOSTER_AVAILABLE") {
            return Ok(RoutingDecision {
                tier: 1,
                model: "agent-booster-wasm".to_string(),
                reason: "Simple transform detected by clawft-llm".to_string(),
                complexity_score: 0.0,
            });
        }

        // 2. Search POLICY_KERNEL for similar patterns
        let similar_policies = self.policy_store.search(prompt, 5).await?;
        if let Some(best_match) = similar_policies.first() {
            if best_match.score > 0.85 {
                let policy: PolicyEntry = serde_json::from_str(&best_match.text)?;
                return Ok(RoutingDecision {
                    tier: policy.tier,
                    model: policy.model,
                    reason: format!("Matched policy: {}", policy.reason),
                    complexity_score: policy.complexity,
                });
            }
        }

        // 3. Compute complexity score via heuristics
        let complexity = compute_complexity(prompt);

        // 4. Route based on complexity
        let (tier, model) = if complexity < 0.30 {
            (2, "claude-haiku-3.5".to_string())
        } else {
            (3, "claude-sonnet-4.5".to_string())
        };

        Ok(RoutingDecision {
            tier,
            model: model.clone(),
            reason: format!("Complexity score: {:.2}", complexity),
            complexity_score: complexity,
        })
    }

    pub async fn record_cost(&mut self, model: &str, usage: Usage) -> Result<()> {
        let segment = Segment::new(
            format!("cost/{}/{}", model, Uuid::new_v4()),
            None,
            json!({
                "model": model,
                "latency_ms": usage.latency.as_millis(),
                "tokens": usage.total_tokens,
                "cost_usd": usage.cost,
            }),
            serde_json::to_string(&usage)?,
        );

        self.cost_tracker.runtime.write_segment(segment).await?;
        Ok(())
    }

    pub async fn update_policy(&mut self, pattern: &str, tier: u8, feedback: Feedback) -> Result<()> {
        let embedding = self.policy_store.embedder.embed(pattern).await?;

        // Search for existing policy
        let existing = self.policy_store.search(pattern, 1).await?;
        if let Some(match_) = existing.first() {
            if match_.score > 0.95 {
                // Update existing policy
                let mut policy: PolicyEntry = serde_json::from_str(&match_.text)?;
                policy.success_rate = (policy.success_rate * policy.usage_count as f32 + if feedback.success { 1.0 } else { 0.0 }) / (policy.usage_count + 1) as f32;
                policy.usage_count += 1;

                // CRUD update via rvf
                self.policy_store.runtime.update_segment(&match_.id, serde_json::to_string(&policy)?).await?;
                return Ok(());
            }
        }

        // Create new policy
        let policy = PolicyEntry {
            pattern: pattern.to_string(),
            tier,
            model: if tier == 2 { "claude-haiku-3.5" } else { "claude-sonnet-4.5" }.to_string(),
            complexity: compute_complexity(pattern),
            success_rate: if feedback.success { 1.0 } else { 0.0 },
            usage_count: 1,
            last_used: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            reason: feedback.reason,
        };

        self.policy_store.add_memory(&serde_json::to_string(&policy)?, &["policy"]).await?;
        Ok(())
    }
}

fn compute_complexity(prompt: &str) -> f32 {
    let mut score = 0.0;

    // Token count heuristic
    let tokens = prompt.split_whitespace().count();
    score += (tokens as f32 / 1000.0).min(0.3);

    // Code blocks
    if prompt.contains("```") {
        score += 0.2;
    }

    // Reasoning keywords
    let reasoning_keywords = ["explain", "why", "analyze", "design", "architect", "refactor"];
    for keyword in reasoning_keywords {
        if prompt.to_lowercase().contains(keyword) {
            score += 0.1;
            break;
        }
    }

    // Multi-step instructions
    if prompt.matches("then").count() > 2 || prompt.matches(char::is_numeric).count() > 3 {
        score += 0.2;
    }

    score.min(1.0)
}
```

### 3.3 Session Indexing
```rust
impl SessionIndexer {
    pub async fn index_turn(&mut self, session_id: &str, turn: &ConversationTurn) -> Result<()> {
        // Combine user + assistant messages for embedding
        let combined = format!("User: {}\nAssistant: {}", turn.user_message, turn.assistant_message);
        let embedding = self.embedder.embed(&combined).await?;

        let segment = Segment::new(
            format!("session/{}/turn/{}", session_id, turn.turn_id),
            Some(embedding.clone()),
            json!({
                "timestamp": turn.timestamp,
                "session_id": session_id,
                "role": "conversation",
                "model": turn.model,
            }),
            serde_json::to_string(&turn)?,
        );

        self.runtime.write_segment(segment).await?;
        self.index.insert_deferred(format!("{}/{}", session_id, turn.turn_id), embedding);

        Ok(())
    }

    pub async fn search_turns(&self, query: &str, session_id: Option<&str>, top_k: usize) -> Result<Vec<TurnMatch>> {
        let query_embedding = self.embedder.embed(query).await?;

        // Filter by session_id if provided
        let filter = session_id.map(|sid| format!("session/{}/turn/*", sid));

        let neighbors = self.index.search(&query_embedding, top_k)?;

        let mut matches = Vec::new();
        for (id, score) in neighbors {
            let segment = self.runtime.read_segment(&format!("session/{}", id)).await?;
            let turn: ConversationTurn = serde_json::from_str(&segment.content)?;

            if let Some(filter) = &filter {
                if !segment.key.starts_with(filter) {
                    continue;
                }
            }

            matches.push(TurnMatch {
                session_id: turn.session_id,
                turn_id: turn.turn_id,
                user_message: turn.user_message,
                assistant_message: turn.assistant_message,
                score,
                timestamp: turn.timestamp,
            });
        }

        Ok(matches)
    }
}
```

### 3.4 First-Startup Indexing
```rust
pub async fn index_existing_memory(memory_path: &Path, store: &mut RvfVectorStore) -> Result<()> {
    let marker_path = memory_path.parent().unwrap().join(".rvf/indexed");
    if marker_path.exists() {
        tracing::info!("Memory already indexed, skipping");
        return Ok(());
    }

    tracing::info!("First startup: Indexing existing MEMORY.md");

    let content = tokio::fs::read_to_string(memory_path).await?;
    let entries = parse_memory_markdown(&content)?;

    tracing::info!("Found {} memory entries to index", entries.len());

    // Batch embeddings (10 per call)
    for batch in entries.chunks(10) {
        let texts: Vec<String> = batch.iter().map(|e| e.text.clone()).collect();
        let embeddings = store.embedder.embed_batch(&texts).await?;

        for (entry, embedding) in batch.iter().zip(embeddings) {
            let segment = Segment::new(
                format!("memory/{}", entry.id),
                Some(embedding),
                json!({
                    "timestamp": entry.timestamp,
                    "tags": entry.tags,
                    "source": "migration"
                }),
                entry.text.clone()
            );

            store.runtime.write_segment(segment).await?;
        }
    }

    // Mark as indexed
    tokio::fs::create_dir_all(marker_path.parent().unwrap()).await?;
    tokio::fs::write(marker_path, b"indexed").await?;

    tracing::info!("Indexing complete");
    Ok(())
}
```

---

## 4. Architecture

### 4.1 RVF Integration Architecture
```
clawft-core
├── memory/
│   ├── MemoryStore trait (abstraction)
│   ├── FileStore (MEMORY.md backend, no RVF)
│   └── RvfVectorStore (MEMORY segments + HNSW, feature = "rvf")
│
├── routing/
│   ├── Router trait (abstraction)
│   ├── StaticRouter (config-based, no RVF)
│   └── IntelligentRouter (POLICY_KERNEL + COST_CURVE, feature = "rvf")
│
├── session/
│   ├── SessionManager (existing)
│   └── SessionIndexer (SESSION segments + HNSW, feature = "rvf")
│
├── embeddings/
│   ├── Embedder trait (abstraction)
│   ├── ApiEmbedder (clawft-llm Provider::embedding())
│   └── HashEmbedder (SimHash/MinHash, fallback)
│
└── audit/
    └── WitnessLog (WITNESS segments, cryptographic audit trail)
```

### 4.2 RVF Segment Schema
```
MEMORY segments:
  Key: "memory/{uuid}"
  Embedding: Vec<f32> (384-dim)
  Metadata: { timestamp, tags, source }
  Content: Original markdown text

HNSW segments (progressive):
  Key: "hnsw/{layer}/{node_id}"
  Metadata: { layer, neighbors }
  Content: Neighbor list + distances

POLICY_KERNEL segments:
  Key: "policy/{pattern_hash}"
  Embedding: Vec<f32> (384-dim)
  Metadata: { model_tier, success_rate, avg_latency, last_used }
  Content: JSON policy

COST_CURVE segments:
  Key: "cost/{model}/{timestamp}"
  Metadata: { model, latency_ms, tokens, cost_usd }
  Content: JSON usage stats

SESSION segments:
  Key: "session/{session_id}/turn/{turn_id}"
  Embedding: Vec<f32> (384-dim)
  Metadata: { timestamp, session_id, role, model }
  Content: JSON ConversationTurn

WITNESS segments:
  Key: "witness/{event_id}"
  Metadata: { event_type, timestamp, actor }
  Content: JSON AuditEvent
  Signature: Ed25519 signature
  Chain: Link to previous WITNESS
```

### 4.3 Embedding Generation Flow
```
User writes memory
  ↓
clawft-core/memory/rvf_store.rs
  ↓
Embedder trait implementation
  ↓
  ├─ ApiEmbedder → clawft-llm Provider::embedding()
  │   ↓
  │   OpenAiCompatProvider::embedding()
  │   ↓
  │   POST https://api.openai.com/v1/embeddings
  │   ↓
  │   Return Vec<f32> (384-dim)
  │
  └─ HashEmbedder → SimHash(text, 384)
      ↓
      Return Vec<f32> (384-dim, deterministic)
  ↓
RvfRuntime::write_segment(MEMORY)
  ↓
HnswIndex::insert_deferred(id, embedding)
  ↓
Background task: progressive_indexing()
  ↓
HnswIndex::insert(id, embedding) [batch 10]
  ↓
RvfRuntime::checkpoint() [save HNSW]
```

### 4.4 Intelligent Routing Flow (ADR-026)
```
User sends prompt
  ↓
clawft-core/routing/intelligent_router.rs
  ↓
1. Check [AGENT_BOOSTER_AVAILABLE] tag
   ├─ Yes → Return Tier 1 (WASM, <1ms, $0)
   └─ No → Continue
  ↓
2. Search POLICY_KERNEL for similar patterns
   ├─ Match found (score > 0.85) → Return cached tier
   └─ No match → Continue
  ↓
3. Compute complexity score (heuristics)
   ├─ < 0.30 → Tier 2 (Haiku, ~500ms, $0.0002)
   └─ ≥ 0.30 → Tier 3 (Sonnet/Opus, 2-5s, $0.003-0.015)
  ↓
4. Return RoutingDecision
  ↓
clawft-llm executes request with selected provider
  ↓
5. Record COST_CURVE segment (usage stats)
  ↓
6. Collect user feedback (success/failure)
  ↓
7. Update POLICY_KERNEL (CRUD update or new entry)
```

### 4.5 Dependency Graph
```
clawft-core (feature = "rvf")
├── rvf-runtime (core runtime)
├── rvf-types (segment types)
├── rvf-index (HNSW index)
├── rvf-adapters-agentdb (optional backend)
├── ruvector-core (vector ops)
├── ruvector-attention (optional overlays)
├── ruvector-temporal-tensor (optional time-series)
├── clawft-llm (Provider::embedding() for ApiEmbedder)
└── sona (optional SONA embeddings)
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 RvfVectorStore Tests

#### Unit Tests
```rust
// tests/memory_rvf_tests.rs
#[tokio::test]
async fn test_rvf_store_add_memory() {
    let tmpdir = TempDir::new().unwrap();
    let embedder = Box::new(HashEmbedder::new(384));
    let mut store = RvfVectorStore::new(tmpdir.path().to_path_buf(), embedder).await.unwrap();

    let id = store.add_memory("test memory", &["tag1", "tag2"]).await.unwrap();
    assert!(!id.is_empty());

    // Verify segment exists
    let segment = store.runtime.read_segment(&format!("memory/{}", id)).await.unwrap();
    assert_eq!(segment.content, "test memory");
    assert_eq!(segment.metadata["tags"], json!(["tag1", "tag2"]));
}

#[tokio::test]
async fn test_rvf_store_search() {
    let tmpdir = TempDir::new().unwrap();
    let embedder = Box::new(HashEmbedder::new(384));
    let mut store = RvfVectorStore::new(tmpdir.path().to_path_buf(), embedder).await.unwrap();

    store.add_memory("authentication using JWT", &["auth"]).await.unwrap();
    store.add_memory("database schema design", &["database"]).await.unwrap();
    store.add_memory("JWT token refresh", &["auth"]).await.unwrap();

    // Wait for progressive indexing
    tokio::time::sleep(Duration::from_secs(2)).await;

    let results = store.search("JWT authentication", 2).await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].text.contains("JWT"));
    assert!(results[0].score > 0.5);
}

#[tokio::test]
async fn test_rvf_store_progressive_indexing() {
    let tmpdir = TempDir::new().unwrap();
    let embedder = Box::new(HashEmbedder::new(384));
    let mut store = RvfVectorStore::new(tmpdir.path().to_path_buf(), embedder).await.unwrap();

    // Add 100 memories
    for i in 0..100 {
        store.add_memory(&format!("memory {}", i), &[]).await.unwrap();
    }

    // Initial state: HNSW incomplete
    assert!(!store.index.is_complete());

    // Trigger progressive indexing
    store.progressive_index().await.unwrap();

    // After indexing: HNSW complete
    assert!(store.index.is_complete());
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_rvf_memory_with_api_embeddings() {
    let provider = Arc::new(OpenAiCompatProvider::new("test-key", "https://api.openai.com/v1"));
    let embedder = Box::new(ApiEmbedder::new(provider));
    let mut store = RvfVectorStore::new(TempDir::new().unwrap().path().to_path_buf(), embedder).await.unwrap();

    store.add_memory("OpenAI GPT-4 usage", &["llm"]).await.unwrap();

    // Should use API embeddings, not HashEmbedding
    let results = store.search("GPT-4", 1).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_first_startup_indexing() {
    let tmpdir = TempDir::new().unwrap();
    let memory_path = tmpdir.path().join("MEMORY.md");
    tokio::fs::write(&memory_path, "## Memory 1\nTest memory\n## Memory 2\nAnother memory").await.unwrap();

    let embedder = Box::new(HashEmbedder::new(384));
    let mut store = RvfVectorStore::new(tmpdir.path().to_path_buf(), embedder).await.unwrap();

    index_existing_memory(&memory_path, &mut store).await.unwrap();

    // Verify indexed
    assert!(tmpdir.path().join(".rvf/indexed").exists());

    // Verify searchable
    let results = store.search("Test", 1).await.unwrap();
    assert_eq!(results.len(), 1);
}
```

### 5.2 IntelligentRouter Tests

#### Unit Tests
```rust
// tests/routing_tests.rs
#[tokio::test]
async fn test_intelligent_router_tier1_agent_booster() {
    let router = IntelligentRouter::new(...).await.unwrap();
    let context = Context { tags: vec!["AGENT_BOOSTER_AVAILABLE".to_string()], ..Default::default() };

    let decision = router.route_request("var x = 1", &context).await.unwrap();

    assert_eq!(decision.tier, 1);
    assert_eq!(decision.model, "agent-booster-wasm");
}

#[tokio::test]
async fn test_intelligent_router_tier2_simple() {
    let router = IntelligentRouter::new(...).await.unwrap();
    let context = Context::default();

    let decision = router.route_request("What is 2+2?", &context).await.unwrap();

    assert_eq!(decision.tier, 2);
    assert_eq!(decision.model, "claude-haiku-3.5");
    assert!(decision.complexity_score < 0.30);
}

#[tokio::test]
async fn test_intelligent_router_tier3_complex() {
    let router = IntelligentRouter::new(...).await.unwrap();
    let context = Context::default();

    let decision = router.route_request("Design a distributed database architecture with CRDT conflict resolution", &context).await.unwrap();

    assert_eq!(decision.tier, 3);
    assert_eq!(decision.model, "claude-sonnet-4.5");
    assert!(decision.complexity_score >= 0.30);
}

#[tokio::test]
async fn test_intelligent_router_policy_cache() {
    let mut router = IntelligentRouter::new(...).await.unwrap();

    // First request: No cached policy
    let decision1 = router.route_request("implement authentication", &Context::default()).await.unwrap();

    // Record policy
    router.update_policy("implement authentication", decision1.tier, Feedback { success: true, reason: "test" }).await.unwrap();

    // Second request: Should use cached policy
    let decision2 = router.route_request("implement authentication", &Context::default()).await.unwrap();

    assert_eq!(decision1.tier, decision2.tier);
    assert!(decision2.reason.contains("Matched policy"));
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_cost_tracking() {
    let mut router = IntelligentRouter::new(...).await.unwrap();

    let usage = Usage {
        total_tokens: 1000,
        cost: 0.003,
        latency: Duration::from_secs(2),
    };

    router.record_cost("claude-sonnet-4.5", usage).await.unwrap();

    // Verify COST_CURVE segment
    let stats = router.cost_tracker.get_model_stats("claude-sonnet-4.5", Duration::from_secs(3600)).await.unwrap();
    assert_eq!(stats.total_calls, 1);
    assert_eq!(stats.total_cost, 0.003);
}

#[tokio::test]
async fn test_feedback_loop() {
    let mut router = IntelligentRouter::new(...).await.unwrap();

    // Initial routing
    let decision = router.route_request("test pattern", &Context::default()).await.unwrap();

    // Positive feedback
    router.update_policy("test pattern", decision.tier, Feedback { success: true, reason: "worked well" }).await.unwrap();

    // Second routing: Should increase confidence
    let decision2 = router.route_request("test pattern", &Context::default()).await.unwrap();
    assert_eq!(decision.tier, decision2.tier);
}
```

### 5.3 Session Indexing Tests

```rust
// tests/session_indexing_tests.rs
#[tokio::test]
async fn test_session_indexer_index_turn() {
    let tmpdir = TempDir::new().unwrap();
    let embedder = Box::new(HashEmbedder::new(384));
    let mut indexer = SessionIndexer::new(tmpdir.path().to_path_buf(), embedder).await.unwrap();

    let turn = ConversationTurn {
        session_id: "session1".to_string(),
        turn_id: 1,
        user_message: "How do I use JWT?".to_string(),
        assistant_message: "JWT is used for authentication...".to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        model: "claude-sonnet-4.5".to_string(),
    };

    indexer.index_turn("session1", &turn).await.unwrap();

    // Verify segment
    let segment = indexer.runtime.read_segment("session/session1/turn/1").await.unwrap();
    assert!(segment.content.contains("JWT"));
}

#[tokio::test]
async fn test_session_indexer_search() {
    let tmpdir = TempDir::new().unwrap();
    let embedder = Box::new(HashEmbedder::new(384));
    let mut indexer = SessionIndexer::new(tmpdir.path().to_path_buf(), embedder).await.unwrap();

    // Index multiple turns
    indexer.index_turn("session1", &ConversationTurn { /* JWT turn */ }).await.unwrap();
    indexer.index_turn("session1", &ConversationTurn { /* database turn */ }).await.unwrap();
    indexer.index_turn("session2", &ConversationTurn { /* JWT turn */ }).await.unwrap();

    // Search across all sessions
    let results = indexer.search_turns("JWT authentication", None, 2).await.unwrap();
    assert_eq!(results.len(), 2);

    // Search within specific session
    let results = indexer.search_turns("JWT", Some("session1"), 2).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session_id, "session1");
}
```

### 5.4 Test Coverage Requirements
- **Unit test coverage**: >90% for RVF integration code
- **Integration test coverage**: >75% for end-to-end flows
- **Critical paths**: 100% coverage for POLICY_KERNEL updates, COST_CURVE recording, embedding generation

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [x] All unit tests passing (>90% coverage)
- [ ] All integration tests passing (>75% coverage)
- [x] RvfVectorStore tested with both ApiEmbedder and HashEmbedder
- [x] IntelligentRouter tested with all 3 tiers (Agent Booster, Haiku, Sonnet)
- [ ] Progressive HNSW indexing tested with 1000+ memories
- [ ] First-startup indexing tested with existing MEMORY.md
- [ ] POLICY_KERNEL feedback loop validated
- [ ] COST_CURVE tracking validated

### 6.2 Feature Flag Configuration
- [x] `rvf` feature flag tested in clawft-core/Cargo.toml
- [ ] `rvf-agentdb` feature flag tested (AgentDB backend)
- [x] `intelligent-routing` feature flag tested
- [ ] `sona`, `attention`, `temporal-tensor` feature flags stubbed (future work)
- [ ] Build matrix: no features, rvf only, rvf+intelligent-routing, all features

### 6.3 Embedding Integration
- [ ] ApiEmbedder integrated with clawft-llm Provider::embedding()
- [ ] OpenAI embedding endpoint tested (POST /v1/embeddings)
- [x] HashEmbedder fallback tested for offline/no-provider scenarios
- [x] Embedding dimension validation (384-dim)
- [ ] Batch embedding tested (10 per call)

### 6.4 RVF Runtime Integration
- [ ] RvfRuntime initialized in clawft-core data directory (.rvf/)
- [ ] MEMORY segments written/read successfully
- [ ] HNSW segments written/read successfully
- [ ] POLICY_KERNEL segments CRUD tested
- [ ] COST_CURVE segments append-only tested
- [ ] SESSION segments written/read successfully
- [ ] WITNESS segments cryptographic chain validated
- [ ] Checkpoint/restore tested

### 6.5 Routing Integration
- [ ] IntelligentRouter integrated with clawft-llm request flow
- [x] [AGENT_BOOSTER_AVAILABLE] tag detection tested
- [x] Tier 1/2/3 routing decisions logged
- [ ] POLICY_KERNEL search latency <50ms
- [x] Complexity scoring heuristics validated
- [ ] Fallback to StaticRouter when RVF disabled

### 6.6 CLI Integration (Stream 2D dependency)
- [ ] `weft memory search "<query>"` command added
- [ ] `weft memory stats` shows RVF index status
- [ ] `weft routing stats` shows tier usage + costs
- [ ] `weft session search "<query>"` searches indexed turns

### 6.7 Documentation
- [ ] README.md for RVF integration in clawft-core
- [ ] ADR-026 documented (3-tier model routing)
- [ ] Feature flag guide (when to use rvf, intelligent-routing, etc.)
- [ ] Migration guide from file-based memory to RVF
- [ ] Embedding strategy documentation (API vs Hash)
- [ ] POLICY_KERNEL schema documented
- [ ] COST_CURVE schema documented
- [ ] WITNESS log schema documented

### 6.8 Performance Benchmarks
- [ ] Memory search latency <100ms for 1000+ memories
- [ ] Progressive indexing <30s for 1000 memories
- [ ] Routing decision latency <50ms (with POLICY_KERNEL cache)
- [ ] Embedding generation <200ms per call (API), <5ms (Hash)
- [ ] RVF write throughput >100 segments/sec
- [ ] HNSW search accuracy >95% recall@10

### 6.9 Security Audit
- [ ] WITNESS log signatures validated
- [ ] POLICY_KERNEL access control (future: multi-user)
- [ ] Embedding API key security (environment variables only)
- [ ] RVF data directory permissions (0700)

### 6.10 Final Review
- [ ] Code review by at least 2 reviewers
- [ ] Security audit of WITNESS log cryptography
- [ ] Performance profiling (flamegraph)
- [ ] Memory leak testing (valgrind/miri)
- [ ] Changelog updated with RVF features
- [ ] Version bumped (clawft-core 0.2.0)

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockFileSystem, MockEnvironment};`
- **Import mocks from 1B**: `use clawft_core::test_utils::{MockLlmTransport, MockMessageBus};`
- **Use shared fixtures**: Load `tests/fixtures/config.json` for config loading tests
- **Use shared fixtures**: Load `tests/fixtures/session.jsonl` for session indexing tests
- **Use shared fixtures**: Load `tests/fixtures/MEMORY.md` for memory indexing tests

### Security Tests (Required)
- Vector search injection prevention (malicious query strings)
- WITNESS log tamper detection (invalid hashes, chain breaks)
- RVF file corruption handling (truncated files, invalid segments)

### Coverage Target
- Unit test coverage: >= 80% (measured via `cargo-tarpaulin`)
- Critical paths (semantic search, routing decisions): 100%

### Integration with 1B SessionManager
```rust
#[cfg(feature = "rvf")]
#[tokio::test]
async fn test_index_existing_python_sessions() {
    // Load Python-generated .jsonl files from tests/fixtures/
    let session_mgr = SessionManager::new(fixtures_dir.join("sessions"));
    let sessions = session_mgr.list_sessions().await.unwrap();
    // Index all into RVF
    for session in &sessions {
        let embedding = mock_embedder.embed(&session.summary).await;
        session_mgr.index_session_turn(&session.id, &session.summary, &embedding).await;
    }
    // Verify searchable
    let results = session_mgr.find_related_sessions(&query_embedding, 5).await;
    assert!(!results.is_empty());
}
```

---

## Notes for Implementation Agent

1. **READ 04-rvf-integration.md FIRST** - This is the source of truth for RVF spec (50+ pages)
2. **READ 05-ruvector-crates.md SECOND** - Full ruvector crate API reference
3. **ADR-026 is critical** - 3-tier model routing with Agent Booster must be implemented exactly as specified
4. **Use TDD London School**: Mock RvfRuntime, write failing tests, then implement
5. **Parallel file operations**: Create all test files + implementation files in single message
6. **Feature flags are mandatory**: Code must work with/without RVF enabled
7. **Embedding strategy**: Default to ApiEmbedder (clawft-llm), fallback to HashEmbedder
8. **Progressive indexing**: HNSW build must be incremental (background task)
9. **POLICY_KERNEL**: Store as MEMORY segments with embeddings for semantic search
10. **COST_CURVE**: Append-only segments for time-series analysis
11. **WITNESS log**: Ed25519 signatures + Merkle chain for audit trail
12. **First-startup indexing**: Parse existing MEMORY.md, batch embed, write segments
13. **Error handling**: Graceful degradation when RVF unavailable
14. **Logging**: Use tracing for debug logs, structured fields for RVF operations
15. **Performance**: Target <100ms search latency, <30s indexing for 1000 memories
