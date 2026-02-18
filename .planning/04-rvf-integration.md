# RVF Integration Specification

## 1. Overview

[RVF (RuVector Format)](https://github.com/ruvnet/ruvector/blob/main/crates/rvf/README.md) is the foundational technology for clawft's intelligence layer. It is a universal binary substrate that merges vector database, model routing, progressive indexing, and WASM runtime into a single format.

A single `.rvf` file stores agent memory embeddings, session state, routing policies, and can run queries via a 5.5 KB WASM microkernel. This replaces litellm and provides capabilities far beyond simple model routing.

## 2. Why RVF for clawft

| Requirement | RVF Solution |
|-------------|-------------|
| Lightweight memory (<10 MB RSS) | Auto-tiered quantization: hot=fp16, warm=PQ, cold=binary |
| Fast semantic search | Three-tier progressive HNSW: 70% recall in microseconds |
| WASM compatibility | `rvf-wasm` microkernel: < 8 KB, `no_std`, 14 C-ABI exports |
| Single-file deployment | All data in one `.rvf` file -- no external DB needed |
| Model routing | POLICY_KERNEL + COST_CURVE segments for learned routing |
| Audit trail | WITNESS segments with cryptographic chain |
| No external deps | Pure Rust, crash-safe without WAL |

## 3. RVF Architecture

### Segment Types Used by clawft

| Segment | Code | clawft Use |
|---------|------|------------|
| VEC | `0x01` | Memory embeddings, session embeddings |
| INDEX | `0x02` | HNSW adjacency for fast search |
| META | `0x07` | Key-value metadata (agent config, skill metadata) |
| HOT | `0x08` | Frequently-accessed memory entries |
| SKETCH | `0x09` | Access frequency tracking (auto-tiering) |
| WITNESS | `0x0A` | Audit trail of agent actions |
| QUANT | `0x06` | Quantization dictionaries |
| MANIFEST | `0x05` | Segment directory |
| POLICY_KERNEL | `0x31` | Model routing policy parameters |
| COST_CURVE | `0x32` | Provider cost/latency/quality curves |

### Progressive Indexing (Three-Tier HNSW)

clawft benefits from immediate query capability:

```
Layer A (Coarse Routing)    -> Loads in microseconds, ~70% recall
  | background loading
Layer B (Hot Region)        -> Loads based on temperature, ~85% recall
  | background loading
Layer C (Full Graph)        -> Complete HNSW, ~95% recall
```

On cold start, clawft can answer memory queries with Layer A while Layers B/C load. Sub-500ms to first useful memory retrieval.

### Temperature-Based Quantization

RVF automatically manages memory efficiency:

| Tier | Access Pattern | Storage | Memory per 384-dim vector |
|------|---------------|---------|--------------------------|
| Hot | Frequent | fp16 | 768 bytes |
| Warm | Moderate | Product quantization | ~48 bytes (16x compression) |
| Cold | Rare | Binary | 48 bytes (32x compression) |

For 10,000 memory entries: Hot=7.5 MB, Warm=470 KB, Cold=470 KB. With typical 20% hot / 30% warm / 50% cold distribution: ~2.0 MB total.

## 4. RVF Crate Map

### Core crates (always included)

| Crate | Purpose | Size Impact |
|-------|---------|-------------|
| `rvf-runtime` | RvfStore: create, open, ingest, query, delete, compact | ~150 KB |
| `rvf-types` | Segment headers, error types, metadata types | ~30 KB |
| `rvf-index` | Progressive HNSW (Layer A/B/C) | ~80 KB |
| `rvf-quant` | Scalar, product, binary quantization | ~40 KB |

### Optional crates

| Crate | Feature Flag | Purpose | Size Impact |
|-------|-------------|---------|-------------|
| `rvf-adapters/agentdb` | `rvf-agentdb` | AgentDB vector store interface | ~20 KB |
| `rvf-adapters/agentic-flow` | `rvf-swarm` | Swarm coordination (future) | ~25 KB |
| `rvf-crypto` | `rvf-crypto` | Witness chain, attestation | ~50 KB |
| `rvf-wasm` | WASM target only | < 8 KB microkernel | < 8 KB |

### Total for clawft (estimated)

- **Minimal** (rvf-runtime + rvf-types + rvf-index): ~260 KB addition to binary
- **Full** (+ agentdb + crypto): ~330 KB addition
- **WASM microkernel**: < 8 KB standalone

## 5. Integration Points

### 5.1 Memory Store (clawft-core)

Replace flat-file MEMORY.md search with rvf vector search:

```rust
use rvf_adapters_agentdb::RvfVectorStore;

pub struct MemoryStore {
    /// File-based memory (backward compatible)
    memory_path: PathBuf,   // ~/.clawft/workspace/memory/MEMORY.md
    history_path: PathBuf,  // ~/.clawft/workspace/memory/HISTORY.md

    /// RVF-backed semantic search (optional feature)
    #[cfg(feature = "rvf")]
    vector_store: Option<RvfVectorStore>,
}

impl MemoryStore {
    /// Search memory semantically using rvf vector search.
    /// Falls back to substring matching if rvf is disabled.
    #[cfg(feature = "rvf")]
    pub fn search_semantic(&self, query_embedding: &[f32], k: usize) -> Vec<MemoryEntry> {
        if let Some(store) = &self.vector_store {
            let results = store.search(query_embedding, k, None).unwrap_or_default();
            results.into_iter().map(|r| MemoryEntry::from_rvf(r)).collect()
        } else {
            self.search_substring(/* fallback */)
        }
    }
}
```

**Storage layout**: `~/.clawft/workspace/memory/memory.rvf`

### 5.2 Session Manager (clawft-core)

Index session summaries in rvf for semantic session retrieval:

```rust
pub struct SessionManager {
    /// JSONL file-based sessions (backward compatible)
    sessions_dir: PathBuf,

    /// RVF index of session summaries
    #[cfg(feature = "rvf")]
    session_index: Option<RvfVectorStore>,
}

impl SessionManager {
    /// Find relevant past sessions by semantic similarity.
    #[cfg(feature = "rvf")]
    pub fn find_related_sessions(&self, context_embedding: &[f32], k: usize) -> Vec<Session> {
        // Search rvf index, then load full JSONL sessions by ID
    }

    /// After each session turn, index the summary embedding.
    #[cfg(feature = "rvf")]
    pub fn index_session_turn(&mut self, session_id: &str, summary: &str, embedding: &[f32]) {
        // Add to rvf index with session_id as metadata
    }
}
```

**Storage layout**: `~/.clawft/sessions/index.rvf`

### 5.3 Model Routing (clawft-core, using clawft-llm transport)

Use rvf POLICY_KERNEL and COST_CURVE segments for learned routing:

```rust
#[cfg(feature = "rvf")]
pub struct IntelligentRouter {
    /// rvf store for routing policies and patterns
    policy_store: RvfVectorStore,
    /// Task pattern embeddings for complexity estimation
    pattern_index: RvfVectorStore,
    /// Underlying HTTP providers
    providers: HashMap<String, OpenAiCompatProvider>,
}

impl IntelligentRouter {
    /// Route a request to the optimal provider/model.
    pub async fn route(&self, request: &ChatRequest) -> RoutingDecision {
        // 1. Embed the request context
        // 2. Search pattern_index for similar past requests
        // 3. Estimate complexity from matched patterns
        // 4. Look up policy_store for learned thresholds
        // 5. Select provider/model based on complexity + cost + latency
    }

    /// After a response, update routing policies based on quality.
    pub fn update_policy(&mut self, decision: &RoutingDecision, quality: f32) {
        // Update pattern embeddings and policy weights in rvf
    }
}
```

**Storage layout**: `~/.clawft/routing/policies.rvf`

### 5.4 Witness Log (clawft-core)

Use rvf WITNESS segments for audit trail:

```rust
#[cfg(feature = "rvf-crypto")]
pub struct WitnessLog {
    store: RvfStore,
}

impl WitnessLog {
    /// Record an agent action (tool use, LLM call, etc.)
    pub fn record(&mut self, action: &str, details: &serde_json::Value) {
        // Append WITNESS segment with cryptographic hash chain
    }

    /// Search audit trail semantically
    pub fn search(&self, query_embedding: &[f32], k: usize) -> Vec<WitnessEntry> {
        // Query WITNESS segments by embedding similarity
    }
}
```

### 5.5 WASM Core (clawft-wasm)

The rvf-wasm microkernel (< 8 KB) provides vector operations directly in WASM:

```rust
// In clawft-wasm: use rvf-wasm's C-ABI exports for vector search
extern "C" {
    fn rvf_init(config_ptr: i32) -> i32;
    fn rvf_load_query(query_ptr: i32, dim: i32) -> i32;
    fn rvf_load_block(block_ptr: i32, count: i32, dtype: i32) -> i32;
    fn rvf_distances(metric: i32, result_ptr: i32) -> i32;
    fn rvf_topk_merge(dist_ptr: i32, id_ptr: i32, count: i32, k: i32) -> i32;
    fn rvf_topk_read(out_ptr: i32) -> i32;
}
```

This means the WASM clawft binary gets vector search without bringing in the full rvf-runtime. The < 8 KB microkernel adds negligible overhead to the < 300 KB WASM target.

## 6. Embedding Strategy

RVF stores vector embeddings but does not generate them. clawft needs an embedding source:

### Option A: LLM-generated embeddings (recommended for MVP)
- Use the configured LLM provider's embedding endpoint (e.g., `text-embedding-3-small`)
- One HTTP call per embedding, ~100ms latency
- No additional binary size

### Option B: Local ONNX model (future)
- Embed `all-MiniLM-L6-v2` via ONNX Runtime
- Zero-latency embeddings, works offline
- Adds ~20 MB to binary (model weights)
- Feature-gated behind `local-embeddings`

### Option C: Hash-based fallback
- ruvector includes `HashEmbedding` for development/testing
- NOT suitable for production (no semantic similarity)
- Zero additional deps

### Recommendation
Start with Option A for MVP (API-based), with a `HashEmbedding` fallback for offline/testing. Add Option B as a future feature.

### Embedding Ownership

Embedding generation is owned by **clawft-core** (not clawft-llm). clawft-core calls the configured provider's embedding endpoint via clawft-llm's `Provider::embedding()` method. This keeps clawft-llm focused on HTTP transport while clawft-core decides when and what to embed.

## 7. Data Migration

### From Python nanobot to clawft

No migration needed -- rvf is additive:

1. Existing `MEMORY.md` and `HISTORY.md` files remain readable
2. On first clawft startup, optionally index existing memory content into `.rvf`
3. New memories are written to both flat file (backward compat) and rvf index
4. JSONL sessions remain as-is; rvf only adds a search index alongside
5. clawft checks `~/.clawft/` first, falls back to `~/.nanobot/` for seamless migration

### Storage Footprint

| Component | Files | Estimated Size |
|-----------|-------|---------------|
| Memory index | `memory.rvf` | ~2-5 MB (10K entries) |
| Session index | `index.rvf` | ~1-3 MB (1K sessions) |
| Routing policies | `policies.rvf` | ~100 KB |
| Witness log | `witness.rvf` | ~500 KB - 5 MB (grows over time) |

Total: ~4-14 MB additional storage for full intelligence layer.

## 8. Feature Flag Design

```toml
# In clawft-core/Cargo.toml
[features]
default = []
rvf = ["dep:rvf-runtime", "dep:rvf-types", "dep:rvf-index"]
rvf-agentdb = ["rvf", "dep:rvf-adapters-agentdb", "dep:ruvector-core"]
rvf-crypto = ["rvf", "dep:rvf-crypto"]

# Intelligence crates (see 05-ruvector-crates.md for details)
ruvllm = ["dep:ruvllm"]
tiny-dancer = ["dep:ruvector-tiny-dancer-core"]
intelligent-routing = ["ruvllm", "tiny-dancer"]
sona = ["dep:sona"]
attention = ["dep:ruvector-attention"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]

# Note: clawft-llm is a standalone library -- it has NO ruvector features.
# RVF intelligence lives in clawft-core, which uses clawft-llm for transport.

# In clawft-cli/Cargo.toml
[features]
ruvector = [
    "clawft-core/rvf-agentdb",
    "clawft-core/intelligent-routing",
    "clawft-core/sona",
    "clawft-core/attention",
    "clawft-core/temporal-tensor",
]
ruvector-full = ["ruvector", "clawft-core/rvf-crypto"]
```

Without the `ruvector` feature, clawft falls back to:
- Static `ProviderSpec` registry matching (same as Python nanobot)
- Substring-based memory search
- No session index
- No witness log
- No intelligent routing

## 9. RVF MCP Integration (Mode 1, In-Process ToolProvider)

RVF vector operations are exposed to MCP clients via an in-process `RvfToolProvider` registered with clawft's `McpServerShell`. This is NOT a standalone subprocess -- the RVF tools run inside clawft's process as a `ToolProvider` implementation, routed through `CompositeToolProvider` alongside `BuiltinToolProvider` and other providers.

The `RvfToolProvider` exposes 11 tools under the `rvf__` namespace:

| Tool | Description |
|------|-------------|
| `rvf__create_store` | Create new vector store |
| `rvf__open_store` | Open existing store |
| `rvf__ingest` | Add vectors |
| `rvf__query` | Search by embedding |
| `rvf__status` | Get store metrics |
| `rvf__delete` | Remove vectors |
| `rvf__compact` | Reclaim space |
| `rvf__delete_filter` | Delete by metadata filter |
| `rvf__list_stores` | List open stores |

When `weft mcp-server` is running, external MCP clients (Claude Code, claude-flow) can invoke these tools directly, allowing LLMs to query and manage the vector store through the standard MCP `tools/call` interface.

## 10. Development Timeline

| Week | Task | Stream |
|------|------|--------|
| 7 | Add rvf-runtime + rvf-types as workspace deps (feature-gated) | 2B |
| 8 | MemoryStore: integrate RvfVectorStore for semantic search | 2B |
| 8 | IntelligentRouter: basic pattern-matched routing | 2B |
| 9 | SessionManager: index session turns in rvf | 2B |
| 10 | IntelligentRouter: learned routing policies | 2B |
| 11 | WitnessLog: audit trail via rvf WITNESS segments | 2B |
| 11 | First-startup memory indexing from existing MEMORY.md | 2B |
| 13 | WASM: integrate rvf-wasm microkernel for vector ops | 3A |
