# ruvector Crates: Utilization Plan for clawft

## 1. Overview

The [ruvector](https://github.com/ruvnet/ruvector) repository contains **80+ crates** spanning vector search, neural routing, self-learning, graph operations, quantum computing, distributed consensus, and WASM microkernels. This document maps which crates clawft should weave into its fabric, at which phase, and how they address the litellm replacement and beyond.

### The Fundamental Insight

ruvector provides **routing intelligence** -- the brains that decide *which* model to use, *how* to learn from outcomes, and *where* to store knowledge. It does **not** provide HTTP transport to external LLM APIs. clawft must build its own `OpenAiCompatProvider` HTTP layer (using reqwest) that the ruvector intelligence layer steers. Think of it as: ruvector is the navigator, clawft is the ship.

---

## 2. Crate Relevance Tiers

### Tier 1: Directly Applicable (Phase 1-2 integration)

| Crate | Purpose | Binary Impact | WASM |
|-------|---------|--------------|------|
| `ruvllm` | Task complexity analysis, model tier selection, session management | ~2 MB (`minimal`) | Via ruvllm-wasm |
| `sona` | Self-Optimizing Neural Architecture -- two-tier LoRA, EWC++, ReasoningBank | ~100 KB | Excellent |
| `rvf` + `rvf-types` | Binary segment format (24 types), self-bootstrapping WASM microkernel | ~50 KB (types) | no_std |
| `ruvector-core` | AgenticDB: PolicyMemoryStore, SessionStateIndex, WitnessLog, HNSW search | ~200-500 KB (`memory-only`) | memory-only feature |
| `ruvector-attention` | 40+ attention mechanisms (Flash, MoE, TopologyGated, Hyperbolic) | ~80 KB | Excellent |
| `micro-hnsw-wasm` | Zero-dep 11.8 KB WASM HNSW search | 11.8 KB | Perfect |
| `ruvector-tiny-dancer-core` | Neural routing: FastGRNN, CircuitBreaker, UncertaintyEstimator | ~500 KB-1 MB | Native only (rusqlite, redb) |
| `ruvector-filter` | Metadata filtering for vector queries | ~30 KB | Native only |
| `ruvector-metrics` | Performance metrics collection | ~20 KB | Good |

### Tier 2: Valuable Extensions (Phase 2-3)

| Crate | Purpose | Binary Impact | WASM |
|-------|---------|--------------|------|
| `ruvector-graph` | Property graph + Cypher parser + RAG engine + multi-hop reasoning | ~500 KB | graph-wasm variant |
| `ruvector-temporal-tensor` | Tiered quantization (8/7/5/3 bit), zero deps | < 10 KB | Perfect |
| `cognitum-gate-kernel` | no_std coherence gate tile (256-tile fabric) | < 10 KB | Perfect |
| `ruvector-domain-expansion` | Cross-domain transfer learning | < 50 KB | Excellent |
| `ruvector-nervous-system-wasm` | Bio-inspired BTSP, HDC, WTA, GlobalWorkspace | < 100 KB | Excellent (standalone) |
| `ruvector-delta-core` | Delta vector change tracking, streaming, compression | ~100 KB | Good |
| `ruvector-snapshot` | Point-in-time persistence | ~50 KB | Native only |
| `ruvector-collections` | Typed vector collections | ~40 KB | Native only |
| `rvlite` | WASM vector DB with SQL/Cypher/SPARQL + IndexedDB | ~200-500 KB | Excellent (browser) |
| `ruvllm-wasm` | Standalone WASM routing + chat templates (no candle) | ~50-200 KB | Excellent (standalone reimpl) |

### Tier 3: Future / Distributed (Phase 3+)

| Crate | Purpose | Binary Impact | WASM |
|-------|---------|--------------|------|
| `ruvector-raft` | Raft consensus for multi-instance | ~200 KB | No |
| `ruvector-cluster` | Consistent hashing, node management | ~300 KB | No |
| `ruvector-replication` | Vector clock conflict resolution | ~150 KB | No |
| `mcp-gate` | MCP server for coherence gate (JSON-RPC stdio) | ~200 KB | No (server) |
| `prime-radiant` | Universal coherence engine (hub crate) | Huge | Limited |
| `ruvector-gnn` | GNN on HNSW topology, EWC, replay buffers | ~300 KB | Partial |

### Tier 4: Specialized / Experimental

| Crate | Purpose | Binary Impact | WASM |
|-------|---------|--------------|------|
| `ruqu-core` + algorithms | Quantum state-vector simulation | < 50 KB | Excellent |
| `ruvector-math` | Optimal Transport, Wasserstein, Info Geometry | ~200 KB | Good (nalgebra) |
| `ruvector-hyperbolic-hnsw` | Poincare ball embeddings for hierarchy-aware search | ~400 KB | Good |
| `ruvector-sparse-inference` | PowerInfer-style edge inference, GGUF loading | ~300 KB | Problematic |
| `ruvector-fpga-transformer` | FPGA transformer acceleration | Large | No |

---

## 3. Addressing the litellm Replacement

### What litellm does (Python nanobot)

1. **HTTP proxy**: Sends requests to 14+ provider endpoints (OpenAI, Anthropic, Groq, etc.)
2. **API translation**: Normalizes different API formats to OpenAI chat/completions schema
3. **Key management**: Routes API keys per provider
4. **Retry/fallback**: Retries failed requests, falls back to alternate providers
5. **Rate limiting**: Respects provider rate limits
6. **Streaming**: Handles SSE streaming responses
7. **Model aliasing**: Maps friendly names to provider-specific model IDs

### What ruvector crates provide (intelligence layer)

| Capability | Crate | How It Works |
|-----------|-------|-------------|
| **Task complexity analysis** | `ruvllm` | 7-factor weighted scoring (token count, reasoning depth, domain specificity, context dependency, output structure, tool usage, ambiguity) |
| **Model tier selection** | `ruvllm` | Maps complexity score to tier (Haiku < 0.3, Sonnet 0.3-0.7, Opus > 0.7) with override support |
| **Semantic pattern matching** | `ruvllm` HnswRouter | 150x faster routing by matching against known task patterns |
| **Quality scoring** | `ruvllm` QualityScoringEngine | 5-dimension scoring (accuracy, relevance, completeness, coherence, safety) |
| **Neural routing** | `ruvector-tiny-dancer-core` | FastGRNN sub-ms inference, CircuitBreaker for failing providers, UncertaintyEstimator |
| **Self-learning** | `sona` | Two-tier LoRA (micro rank-2 instant, base rank-8 background), EWC++ anti-forgetting |
| **Pattern storage** | `sona` ReasoningBank | Stores task patterns (Reasoning, CodeGen, Conversational, etc.) for future routing |
| **Policy persistence** | `rvf` POLICY_KERNEL + COST_CURVE segments | Learned routing policies survive restarts |
| **Audit trail** | `ruvector-core` WitnessLog / `rvf` WITNESS segments | Cryptographic chain of routing decisions |
| **Session memory** | `ruvector-core` SessionStateIndex | Semantic session retrieval via HNSW |

### What clawft must build (transport layer)

| Component | Description | Crate |
|-----------|-------------|-------|
| **OpenAiCompatProvider** | HTTP client (reqwest) for chat/completions endpoint | `clawft-llm` (standalone library) |
| **Request builder** | Construct provider-specific request payloads | `clawft-llm` (standalone library) |
| **Response parser** | Parse SSE streams and JSON responses | `clawft-llm` (standalone library) |
| **Provider registry** | Static registry mapping model names to API specs | `clawft-llm` (standalone library) |
| **API key resolver** | Read keys from config, env vars | `clawft-platform` |
| **Retry logic** | Exponential backoff with jitter | `clawft-llm` (standalone library) |
| **Rate limiter** | Token bucket per provider | `clawft-llm` (standalone library) |
| **JSON repair** | Fix malformed LLM JSON output | `clawft-core` |

### The Architecture: ruvector Navigates, clawft Ships

```
User Message
    |
    v
[clawft-core: AgentLoop]
    |
    v
[ruvllm: TaskComplexityAnalyzer]  -- "This needs Opus-tier reasoning"
    |
    v
[ruvector-tiny-dancer-core: NeuralRouter]  -- "Anthropic has best quality for this pattern"
    |                                          "OpenAI is down (CircuitBreaker open)"
    v
[sona: ReasoningBank]  -- "Similar task last week got 0.92 quality from Anthropic"
    |
    v
[clawft-llm: OpenAiCompatProvider]  -- HTTP POST to api.anthropic.com/v1/messages
    |
    v
[Response]
    |
    v
[ruvllm: QualityScoringEngine]  -- Score response quality
    |
    v
[sona: update LoRA weights]  -- Learn from this outcome
[rvf: persist policy update]  -- Save for next restart
```

---

## 4. Key Crate Deep Dives

### 4.1 ruvllm -- Model Routing Intelligence

**Location**: `crates/ruvllm/`

**Key modules**:
- `claude_flow::model_router` -- `ComplexityFactors`, `ComplexityWeights`, `ModelSelector`
- `HnswRouter` -- HNSW-indexed pattern matching for routing decisions (150x faster than linear scan)
- `QualityScoringEngine` -- 5-dimension response quality scoring
- `SessionManager` -- Conversation state tracking
- `WitnessLog` -- Audit trail of routing decisions

**Feature flags**: ruvllm is a large crate with optional candle-based local inference. clawft should use the `minimal` feature (async runtime only, no inference backend) to avoid pulling in candle, tokenizers, hf-hub, and GPU backends (metal, cuda). The `minimal` build is ~2-3 MB; with `candle` it balloons to 5-15 MB.

**Current limitation**: Only routes among Claude tiers (Haiku/Sonnet/Opus). Must be extended to route across 14+ providers.

**Additional modules available** (cherry-pick, don't import all):
- `IntelligentContextManager`, `EpisodicMemory`, `WorkingMemory` -- context management
- `ReflectiveAgent`, `ErrorPatternLearner`, `ConfidenceChecker` -- self-correction
- `ClaudeFlowAgent`, `AgentRouter`, `AgentCoordinator` -- agent orchestration
- `SpeculativeDecoder` -- speculative decoding (not needed for API-only usage)

**clawft integration**:
```rust
// In clawft-core/src/routing/intelligent.rs
use ruvllm::model_router::{TaskComplexityAnalyzer, ComplexityFactors};
use ruvllm::hnsw_router::HnswRouter;

pub struct IntelligentRouter {
    complexity: TaskComplexityAnalyzer,
    pattern_router: HnswRouter,
    providers: HashMap<String, OpenAiCompatProvider>,  // clawft's HTTP layer
}

impl IntelligentRouter {
    pub async fn route(&self, request: &ChatRequest) -> RoutingDecision {
        let factors = self.complexity.analyze(&request);
        let tier = factors.select_tier();
        let pattern = self.pattern_router.find_best_match(&request);
        // Combine tier + pattern + provider health to select
        self.select_provider(tier, pattern)
    }
}
```

**Feature flag**: `clawft-core/ruvllm`

**WASM note**: `ruvllm-wasm` is a standalone reimplementation (does NOT depend on the main `ruvllm` crate). It provides `HnswRouterWasm`, `SonaInstantWasm`, chat template formatting (Llama3, Mistral, Qwen, Phi, Gemma), and `ParallelInference` via Web Workers. Estimated 50-200 KB WASM. Useful if clawft-wasm needs routing intelligence without the full native ruvllm stack.

### 4.2 sona -- Self-Optimizing Neural Architecture

**Location**: `crates/sona/`

**Key capabilities**:
- **MicroLoRA** (rank-2): Instant per-request adaptation. Updates after every LLM response.
- **Base LoRA** (rank-8): Background hourly consolidation of micro-LoRA updates.
- **EWC++** (Elastic Weight Consolidation): Prevents catastrophic forgetting when learning new patterns.
- **ReasoningBank**: Stores task-outcome trajectories with pattern types (Reasoning, CodeGen, Conversational, Analysis, Creative).
- **Three-tier learning loop**: Instant (per-request) -> Background (hourly) -> Deep (weekly).

**Dependencies**: Only 5 direct deps (parking_lot, crossbeam, rand, serde, thiserror). WASM-ready.

**clawft integration**: SONA maps directly to clawft's agent loop feedback:
```
query -> LLM call -> tool execution -> outcome
  |                                       |
  +--- sona records trajectory -----------+
  |                                       |
  +--- micro-LoRA updates weights --------+
```

Every agent interaction becomes a training signal. Over time, clawft learns:
- Which provider/model works best for which task types
- Which tool sequences are most effective
- How to estimate complexity more accurately
- User-specific preferences and patterns

**Feature flag**: `clawft-core/sona`

### 4.3 ruvector-attention -- Context Window Management

**Location**: `crates/ruvector-attention/`

**40+ attention mechanisms**, most relevant for clawft:

| Mechanism | clawft Use |
|-----------|-----------|
| `FlashAttention` | Efficient memory retrieval from large context windows |
| `LocalGlobalAttention` | Prioritize recent messages while maintaining long-term context |
| `TopologyGatedAttention` | Weight context by conversation topology (main thread vs subagent) |
| `InformationBottleneck` | Compress context to most relevant information before LLM call |
| `MoEAttention` (Mixture of Experts) | Route different context types through specialized attention heads |
| `HyperbolicAttention` | Hierarchy-aware attention for skill/tool documentation |

**Builder API**: `AttentionBuilder::new().flash().local_global(window=512).build()`

**Presets**: Bert, Gpt, Longformer patterns available out of the box.

**clawft integration**: The ContextBuilder (clawft-core) can use attention mechanisms to intelligently select which memories, session history, and skill documentation to include in the system prompt, rather than truncating by token count.

**Feature flag**: `clawft-core/attention`

### 4.4 ruvector-tiny-dancer-core -- Neural Provider Routing

**Location**: `crates/ruvector-tiny-dancer-core/`

**Key types**:
- `Candidate` -- Provider/model with embeddings, success rate, latency, metadata
- `FastGRNN` -- Sub-millisecond neural inference for routing decisions
- `CircuitBreaker` -- Automatic provider failover (open/half-open/closed states)
- `UncertaintyEstimator` -- Confidence intervals on routing decisions
- `Trainer` -- Knowledge distillation from large models to routing network

**clawft integration**: Wraps the provider pool with neural selection:
```
Request -> FastGRNN inference (<1ms) -> Select provider
                                     -> CircuitBreaker check
                                     -> Uncertainty threshold
                                     -> Fallback if uncertain
```

When a provider fails, CircuitBreaker opens and traffic automatically routes elsewhere. When it recovers, half-open state tests with limited traffic before full restoration.

**Feature flag**: `clawft-core/tiny-dancer`

### 4.5 micro-hnsw-wasm -- WASM Vector Search

**Location**: `crates/micro-hnsw-wasm/`

**Zero dependencies. 11.8 KB WASM binary.**

Provides HNSW nearest-neighbor search in pure WASM with no allocator dependencies. This is the ideal candidate for clawft-wasm's vector search needs -- far smaller than the full rvf-runtime.

**clawft integration**: In the WASM build target, micro-hnsw-wasm replaces the full HNSW index for memory search, keeping the WASM binary well under the 300 KB target.

### 4.6 ruvector-temporal-tensor -- Tiered Quantization

**Location**: `crates/ruvector-temporal-tensor/`

**Zero external dependencies. < 10 KB WASM.**

Implements ADR-017 groupwise symmetric quantization:
- **Hot tier**: 8-bit (4x compression vs f32)
- **Warm tier**: 7-bit (4.57x) or 5-bit (6.4x)
- **Cold tier**: 3-bit (10.67x compression)

Temperature-based auto-tiering by access frequency. This is how clawft keeps memory footprint low: frequently-accessed memories stay at full precision while old memories compress dramatically.

**Feature flag**: `clawft-core/temporal-tensor`

### 4.7 ruvector-core -- AgenticDB Foundation

**Location**: `crates/ruvector-core/`

**Key components**:
- `AgenticDB` -- Vector database with HNSW indexing (~2,500 queries/sec on 10K vectors)
- `PolicyMemoryStore` -- Stores learned routing policies
- `SessionStateIndex` -- Indexes session summaries for semantic retrieval
- `WitnessLog` -- Cryptographic audit trail
- `memory-only` feature -- Strips file I/O for WASM compatibility

This is the storage backbone that rvf segments are built on. clawft's MemoryStore and SessionManager both use AgenticDB under the hood.

**Caveat**: AgenticDB uses placeholder hash-based embeddings by default -- NOT real semantic embeddings. clawft must integrate a real embedding source (API-based via `api-embeddings` feature, or local ONNX model in the future) for production-quality semantic search.

**Feature flag**: `clawft-core/agenticdb` (implies ruvector-core with memory-only for WASM)

### 4.8 rvlite -- WASM Vector Database with Query Languages

**Location**: `crates/rvlite/`

Standalone WASM vector database with SQL (pgvector-compatible), SPARQL, and Cypher query languages. Uses IndexedDB for browser persistence.

**Key API**: `RvLite` -- constructor, insert, search, sql, cypher, sparql. Supports `save()`, `load()`, `export_json()`, `import_json()` via IndexedDB.

**Binary size**: ~200-500 KB WASM (opt-level "z", LTO, strip).

**clawft integration**: If clawft ever runs in-browser (not just WASM edge), rvlite provides a full query-language-capable vector database. The SQL interface (`SELECT ... ORDER BY distance`) is particularly developer-friendly. Lower priority than micro-hnsw-wasm for the MVP WASM target due to size.

### 4.9 micro-hnsw-wasm -- Ultra-Light WASM Vector Search (Expanded)

**Location**: `crates/micro-hnsw-wasm/`

Beyond basic HNSW, this crate includes neuromorphic features:
- **Spiking Neural Networks**: LIF (Leaky Integrate-and-Fire) neurons, STDP (Spike-Timing-Dependent Plasticity) learning
- **Winner-Take-All circuits**: Competitive neural selection
- **Dendritic computation**: Multi-compartment processing
- **Multi-core sharding**: 256 cores x 32 vectors = 8,192 total capacity
- **Typed graph nodes**: 16 node types with edge weights (enables Cypher-style typed queries)

**Compile-time limits**: MAX_VECTORS=32 per core, MAX_DIMS=16, MAX_NEIGHBORS=6, BEAM_WIDTH=3.

**clawft integration**: The 16-dimension / 8K vector cap makes this ideal for routing tables and skill indexes, not general-purpose RAG. The neuromorphic features enable online learning at the edge -- the HNSW index itself adapts via STDP without any external training loop.

---

## 5. WASM Size Budget

The clawft-wasm target must stay under 300 KB uncompressed. Here's the budget with ruvector crates:

| Component | Estimated Size | Notes |
|-----------|---------------|-------|
| clawft-core (agent loop, context, tools) | ~100 KB | No exec tool, no channel plugins |
| micro-hnsw-wasm | 11.8 KB | Vector search |
| ruvector-temporal-tensor (ffi) | < 10 KB | Quantization, zero deps |
| cognitum-gate-kernel | < 10 KB | Coherence gate, no_std |
| sona (wasm subset) | ~30 KB | Learning, minimal deps |
| rvf-types | ~30 KB | Segment parsing, no_std |
| reqwest (wasm) or manual HTTP | ~50 KB | LLM API calls |
| **Total** | **~242 KB** | Under 300 KB budget |

Crates that must NOT be in WASM: ruvector-graph (500 KB), ruvector-gnn (300 KB), prime-radiant (enormous), ruvector-sparse-inference (memmap2), anything with tokio/rayon.

---

## 6. Architectural Patterns from ruvector

### 6.1 Dual-Crate Pattern

Many ruvector systems ship a heavy native crate and a separate lightweight WASM crate:

| Native Crate | WASM Crate | Pattern |
|-------------|-----------|---------|
| `ruvector-nervous-system` | `ruvector-nervous-system-wasm` | Standalone reimplementation (< 100 KB vs ~500 KB) |
| `ruvector-dag` | `ruvector-dag-wasm` | Standalone reimplementation (< 20 KB vs ~300 KB) |
| `ruvector-delta-core` | `ruvector-delta-wasm` | Thin wrapper (same size) |
| `ruvector-math` | `ruvector-math-wasm` | Thin wrapper (nalgebra weight) |

**Lesson for clawft**: The standalone reimplementation pattern produces dramatically smaller WASM. clawft-wasm should follow this pattern -- a purpose-built minimal core, not a feature-flagged subset of the native crate.

### 6.2 Zero-Dep Champions

| Crate | Deps | WASM Size | Technique |
|-------|------|-----------|-----------|
| `ruvector-temporal-tensor` | 0 | < 10 KB | C-ABI FFI exports, no wasm-bindgen |
| `cognitum-gate-kernel` | 1 (libm) | < 10 KB | no_std, bump allocator, 64 KB memory budget |
| `ruqu-core` | 2 (rand, thiserror) | < 30 KB | Pure Rust state-vector sim |

**Lesson for clawft**: For the WASM microkernel, avoid wasm-bindgen overhead. Use raw C-ABI exports (like ruvector-temporal-tensor) for the tightest possible binary.

### 6.3 Coherence Gate Architecture

The cognitum-gate system cleanly separates:
- **cognitum-gate-kernel** (WASM tiles) -- runs in browser/edge, < 10 KB
- **cognitum-gate-tilezero** (arbiter) -- runs server-side, coordinates tiles
- **mcp-gate** (protocol) -- exposes coherence decisions via MCP

**Lesson for clawft**: This pattern could be used for clawft's own safety/coherence layer. The kernel runs inside the WASM agent, the arbiter runs on the gateway, and decisions are exposed via MCP tools.

---

## 7. Feature Flag Design

```toml
# clawft-core/Cargo.toml
[features]
default = []

# Tier 1: Core intelligence
agenticdb = ["dep:ruvector-core"]
sona = ["dep:sona"]
attention = ["dep:ruvector-attention"]

# Tier 1: RVF persistence
rvf = ["dep:rvf-runtime", "dep:rvf-types", "dep:rvf-index"]
rvf-agentdb = ["rvf", "agenticdb", "dep:rvf-adapters-agentdb"]
rvf-crypto = ["rvf", "dep:rvf-crypto"]

# Tier 2: Quantization
temporal-tensor = ["dep:ruvector-temporal-tensor"]

# Tier 2: Graph knowledge
graph = ["dep:ruvector-graph"]

# NOTE: clawft-llm is a standalone library with NO ruvector features.
# It provides pure HTTP transport. Intelligence wrapping lives in clawft-core.

# clawft-core/Cargo.toml (continued - intelligence wrapping ruvector crates)
ruvllm = ["dep:ruvllm"]
tiny-dancer = ["dep:ruvector-tiny-dancer-core"]
intelligent-routing = ["ruvllm", "tiny-dancer"]

# clawft-cli/Cargo.toml
[features]
default = ["channel-telegram", "all-tools"]

# Bundle: all ruvector intelligence
ruvector = [
    "clawft-core/rvf-agentdb",
    "clawft-core/intelligent-routing",
    "clawft-core/sona",
    "clawft-core/attention",
    "clawft-core/temporal-tensor",
]

# Bundle: full ruvector + crypto audit + graph
ruvector-full = [
    "ruvector",
    "clawft-core/rvf-crypto",
    "clawft-core/graph",
]

# clawft-wasm/Cargo.toml
[features]
default = ["micro-hnsw", "temporal-tensor", "sona-wasm"]
micro-hnsw = ["dep:micro-hnsw-wasm"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]
sona-wasm = ["dep:sona"]
coherence = ["dep:cognitum-gate-kernel"]
```

### Build Profiles

| Profile | Features | Binary Size | Use Case |
|---------|----------|------------|----------|
| `weft` (default) | cli + telegram | ~5 MB | Minimal self-hosted bot |
| `weft` (ruvector) | + all Tier 1 crates | ~8 MB | Intelligent routing + learning |
| `weft` (ruvector-full) | + crypto + graph | ~10 MB | Full intelligence suite |
| `clawft-wasm` (default) | micro-hnsw + temporal-tensor + sona | < 300 KB | Edge/WASM deployment |

---

## 8. Integration Timeline

### Phase 1: Warp (Weeks 1-8) -- No ruvector deps

The foundation phase uses no ruvector crates. clawft-llm implements `OpenAiCompatProvider` with static `ProviderSpec` registry matching (same as Python nanobot). This ensures the HTTP transport layer works before adding intelligence.

### Phase 2: Weft (Weeks 7-11) -- Tier 1 integration

| Week | Crate | Integration |
|------|-------|-------------|
| 7 | `ruvector-core` (agenticdb) | Add as workspace dep, feature-gated |
| 7 | `rvf` + `rvf-types` | Binary format for persistence |
| 8 | `sona` | Wire into agent loop feedback (micro-LoRA per response) |
| 8 | `ruvllm` | TaskComplexityAnalyzer + HnswRouter in provider selection |
| 9 | `ruvector-attention` | ContextBuilder uses attention for prompt assembly |
| 9 | `ruvector-core` SessionStateIndex | Semantic session retrieval |
| 10 | `ruvector-tiny-dancer-core` | Neural provider routing + CircuitBreaker |
| 10 | `ruvector-temporal-tensor` | Auto-tiered memory quantization |
| 11 | `rvf-crypto` | WITNESS segment audit trail |
| 11 | First-startup indexing | Migrate existing MEMORY.md into rvf + sona |

### Phase 3: Finish (Weeks 11-14) -- WASM + Tier 2

| Week | Crate | Integration |
|------|-------|-------------|
| 11 | `micro-hnsw-wasm` | WASM vector search (replaces full HNSW) |
| 12 | `ruvector-temporal-tensor` (ffi) | WASM quantization |
| 12 | `cognitum-gate-kernel` | WASM coherence gate |
| 13 | `sona` (wasm subset) | WASM learning |
| 13 | `ruvector-domain-expansion` | Cross-domain transfer |
| 14 | Benchmark: ruvector ON vs OFF | Measure routing quality, latency, learning |

### Future (Post-Phase 3)

| Crate | Use Case |
|-------|----------|
| `ruvector-graph` | Knowledge graph for skill relationships, multi-hop reasoning |
| `ruvector-raft` + cluster | Multi-instance clawft deployment |
| `ruvector-gnn` | Learned HNSW topology optimization |
| `ruvector-nervous-system-wasm` | Bio-inspired BTSP learning for edge |
| `mcp-gate` | Expose coherence gate as MCP tool |
| `ruvector-sparse-inference` | Local model inference (future offline mode) |
| `prime-radiant` | Universal coherence/safety layer (server-side only) |
| `ruqu-core` | Quantum-inspired optimization algorithms |

---

## 9. The Progressive Story

clawft becomes progressively more intelligent as ruvector features are enabled:

**Level 0 (no ruvector)**: Static registry routing. Same as Python nanobot.
- Model X -> Provider Y (hardcoded mapping)
- Substring memory search
- Token-count context truncation

**Level 1 (ruvllm)**: Complexity-aware routing.
- Analyzes task complexity (7 factors)
- Routes simple tasks to cheaper models, complex tasks to capable ones
- Pattern matching against known task types (150x faster via HNSW)

**Level 2 (+ tiny-dancer)**: Neural routing with resilience.
- Sub-millisecond neural inference for provider selection
- CircuitBreaker detects provider failures, auto-routes around them
- Uncertainty estimation prevents low-confidence routing

**Level 3 (+ sona)**: Self-learning routing.
- Every response trains micro-LoRA weights (instant adaptation)
- ReasoningBank stores task-outcome trajectories
- EWC++ prevents forgetting while learning new patterns
- Over days/weeks, routing quality continuously improves

**Level 4 (+ attention + temporal-tensor)**: Intelligent context management.
- Attention mechanisms select optimal context for each LLM call
- Flash attention for large memory retrieval
- Information bottleneck compresses irrelevant context
- Auto-tiered quantization keeps memory footprint low (hot=fp16, cold=3-bit)

**Level 5 (+ graph + domain-expansion)**: Knowledge fabric.
- Property graph of skills, tools, and their relationships
- Multi-hop reasoning across knowledge base
- Cross-domain transfer learning (patterns learned in one domain apply to others)
- The full weave: every thread of intelligence interlaced

Each level is opt-in via feature flags. A bare `weft` binary at Level 0 is ~5 MB. A fully-woven `weft` at Level 5 is ~10 MB. Both read the same config.json, run the same agent loop, and serve the same channels.

---

## 10. litellm-rs Assessment

### What litellm-rs Is

[litellm-rs](https://github.com/majiayu000/litellm-rs) (v0.3.2, MIT) is a **full AI gateway server** -- not a lightweight library. It's modeled after Python litellm and includes:

- **33 wired providers** (OpenAI, Anthropic, Groq, DeepSeek, Mistral, Bedrock, Vertex AI, etc.) with 123 provider modules total (many likely stubs)
- **Clean library API**: `completion(model, messages, options)`, `completion_stream()`, `embedding()`
- **Full SSE streaming** with per-provider stream handling
- **Tool/function calling** with JSON Schema parameters
- **7 routing strategies**: SimpleShuffle, LeastBusy, UsageBased, LatencyBased, CostBased, RateLimitAware, RoundRobin
- **Retry/fallback**: Configurable retries (default 3), exponential backoff, fallback chain (max 5), cooldown tracking
- **Rate limiting**: Sliding window, token bucket, fixed window strategies
- **Model aliasing**: `"gpt4"` -> `"gpt-4"`, prefix routing (`"anthropic/claude-3-opus"`)
- **Cost tracking**: Per-provider cost calculators with pricing database
- **SDK module**: `LLMClient` with load balancing, provider stats, health checks

### Why NOT a Direct Dependency

| Issue | Detail |
|-------|--------|
| **Too heavy** | Even "lite" mode pulls actix-web, actix-cors, actix-multipart, tokio-full |
| **Not WASM-compatible** | Hard deps on actix-web, tokio-full, reqwest-rustls, sea-orm |
| **Rust edition 2024** | Requires 1.87+; clawft targets 1.85+ (edition 2024), compatible |
| **Very young** | 7 months, 17 stars, 3 contributors, no tagged releases |
| **reqwest 0.11** | One major version behind (0.12 current) |
| **Build issues** | Open issues about feature flag build failures |
| **Provider stubs** | 123 modules but only 33 in ProviderType enum |

### What to Learn From It

litellm-rs validates the API surface we need. clawft should adopt the same patterns:

1. **`completion(model, messages, options)` API** -- clean, Python-litellm-compatible
2. **Prefix-based model routing** -- `"anthropic/claude-3-opus"` automatically routes to Anthropic
3. **Retry with cooldown tracking** -- deployments go into cooldown after N failures (same concept as tiny-dancer's CircuitBreaker)
4. **7 routing strategies** -- especially LatencyBased, CostBased, and RateLimitAware are directly useful
5. **Model pricing database** -- JSON config for cost-aware routing

### The Path Forward: Trait-Based Pipeline

Rather than depending on litellm-rs, clawft builds the same capabilities as traits. litellm-rs (or any future Rust LLM library) can be plugged in as an optional backend if/when it matures.

---

## 11. Pluggable Provider Architecture

### The Problem

Different tasks need different capabilities:

| Task Type | Routing Need | Context Need | Quality Need |
|-----------|-------------|-------------|-------------|
| **Research** | Broad model access, cost-efficient | Large context windows, many sources | Accuracy, citation quality |
| **Development** | Fast, low-latency | Code context, file trees | Correctness, tool-call reliability |
| **Creative** | High-quality models | Conversation history, style context | Coherence, voice consistency |
| **Triage** | Cheapest possible | Minimal context | Speed over quality |
| **Analysis** | Reasoning-capable | Structured data, tables | Logical soundness |

A single routing strategy doesn't serve all these. clawft needs a **pluggable pipeline** where each stage can be swapped based on task type.

### Trait Pipeline Design

```rust
/// Stage 1: Classify the task to determine which pipeline to use.
pub trait TaskClassifier: Send + Sync {
    fn classify(&self, request: &ChatRequest) -> TaskProfile;
}

/// Stage 2: Select the best provider/model for this task profile.
pub trait ModelRouter: Send + Sync {
    async fn route(&self, request: &ChatRequest, profile: &TaskProfile) -> RoutingDecision;
    fn update(&self, decision: &RoutingDecision, outcome: &ResponseOutcome);
}

/// Stage 3: Build context (system prompt, memories, skills) for the task.
pub trait ContextAssembler: Send + Sync {
    async fn assemble(&self, request: &ChatRequest, profile: &TaskProfile) -> AssembledContext;
}

/// Stage 4: Execute the LLM call via HTTP.
pub trait LlmTransport: Send + Sync {
    async fn complete(&self, request: &TransportRequest) -> Result<LlmResponse>;
    async fn complete_stream(&self, request: &TransportRequest) -> Result<ResponseStream>;
}

/// Stage 5: Score the response quality.
pub trait QualityScorer: Send + Sync {
    fn score(&self, request: &ChatRequest, response: &LlmResponse) -> QualityScore;
}

/// Stage 6: Learn from the interaction.
pub trait LearningBackend: Send + Sync {
    fn record(&self, trajectory: &Trajectory);
    fn adapt(&self, signal: &LearningSignal);
}
```

### Implementations Per Stage

| Stage | Level 0 (no ruvector) | Level 1+ (ruvector) | Future (litellm-rs) |
|-------|----------------------|--------------------|--------------------|
| **TaskClassifier** | `KeywordClassifier` (regex/keyword match) | `ruvllm::TaskComplexityAnalyzer` (7-factor) | -- |
| **ModelRouter** | `StaticRouter` (config.json mapping) | `ruvllm::HnswRouter` + `tiny-dancer::FastGRNN` | `litellm_rs::Router` (7 strategies) |
| **ContextAssembler** | `TokenBudgetAssembler` (truncate by count) | `ruvector-attention` (Flash, MoE, InfoBottleneck) | -- |
| **LlmTransport** | `OpenAiCompatTransport` (clawft-llm) | Same (transport is always clawft-llm's) | `SidecarTransport` (optional litellm-rs) |
| **QualityScorer** | `NoopScorer` | `ruvllm::QualityScoringEngine` (5-dimension) | -- |
| **LearningBackend** | `NoopLearner` | `sona::SonaEngine` (micro-LoRA + EWC++) | -- |

### Task Profiles and Pipeline Selection

```rust
pub struct TaskProfile {
    pub task_type: TaskType,          // Research, Development, Creative, Triage, Analysis
    pub complexity: f32,              // 0.0 - 1.0
    pub urgency: Urgency,            // Low, Normal, High
    pub budget_class: BudgetClass,   // Minimal, Standard, Premium
    pub required_capabilities: Vec<Capability>,  // ToolCalling, LargeContext, Streaming, Vision
}

pub enum TaskType {
    Research,     // Broad access, accuracy-focused
    Development,  // Fast, tool-call reliable
    Creative,     // High quality, voice consistent
    Triage,       // Cheap and fast
    Analysis,     // Reasoning-capable
    Custom(String),
}
```

The `TaskProfile` determines which implementations to use at each pipeline stage. A `PipelineRegistry` maps profiles to configured pipelines:

```rust
pub struct PipelineRegistry {
    pipelines: HashMap<TaskType, Pipeline>,
    default: Pipeline,
}

pub struct Pipeline {
    classifier: Arc<dyn TaskClassifier>,
    router: Arc<dyn ModelRouter>,
    context: Arc<dyn ContextAssembler>,
    transport: Arc<dyn LlmTransport>,
    scorer: Arc<dyn QualityScorer>,
    learner: Arc<dyn LearningBackend>,
}
```

### Configuration Example

```json
{
  "pipelines": {
    "research": {
      "router": "cost-based",
      "preferred_models": ["anthropic/claude-sonnet", "openai/gpt-4o"],
      "context": { "max_tokens": 100000, "include_web_results": true },
      "quality": { "min_score": 0.8, "retry_if_below": true }
    },
    "development": {
      "router": "latency-based",
      "preferred_models": ["anthropic/claude-sonnet", "groq/llama-3.1-70b"],
      "context": { "max_tokens": 32000, "include_file_tree": true },
      "quality": { "min_tool_call_accuracy": 0.95 }
    },
    "triage": {
      "router": "cost-based",
      "preferred_models": ["groq/llama-3.1-8b", "anthropic/claude-haiku"],
      "context": { "max_tokens": 4000 },
      "quality": { "min_score": 0.5 }
    }
  }
}
```

### How ruvector Crates Plug In

Each ruvector crate implements one or more pipeline traits:

| Crate | Implements | What It Adds |
|-------|-----------|-------------|
| `ruvllm` (minimal) | `TaskClassifier`, `ModelRouter`, `QualityScorer` | Complexity analysis, HNSW routing, quality scoring |
| `ruvector-tiny-dancer-core` | `ModelRouter` | Neural routing (FastGRNN), CircuitBreaker, uncertainty |
| `sona` | `LearningBackend` | Micro-LoRA adaptation, EWC++, ReasoningBank |
| `ruvector-attention` | `ContextAssembler` | Flash attention, MoE, InformationBottleneck |
| `ruvector-core` | Storage for all stages | AgenticDB backing for policies, sessions, witness |
| `rvf` | Persistence for all stages | Binary format for routing policies, memory, audit |

And litellm-rs could plug in as:

| Crate | Implements | What It Adds |
|-------|-----------|-------------|
| `litellm-rs` (sidecar) | `LlmTransport` (via `SidecarTransport`) | 33 providers for exotic wire formats (Cohere, HuggingFace) |

### Progressive Adoption Path

```
Phase 1 (Warp):
  TaskClassifier  = KeywordClassifier
  ModelRouter     = StaticRouter (config.json ProviderSpec registry)
  ContextAssembler = TokenBudgetAssembler
  LlmTransport   = clawft-llm providers (Anthropic, OpenAI, Bedrock, Gemini, OpenAI-compat)
  QualityScorer   = NoopScorer
  LearningBackend = NoopLearner

Phase 2 (Weft), progressive:
  + ruvllm:         TaskClassifier = ComplexityAnalyzer, QualityScorer = QualityEngine
  + tiny-dancer:    ModelRouter = NeuralRouter (wraps FastGRNN + CircuitBreaker)
  + sona:           LearningBackend = SonaEngine
  + attention:      ContextAssembler = AttentionAssembler
  + ruvector-core:  All stages persist to AgenticDB
  + litellm-rs:     SidecarTransport (optional, for Cohere/HuggingFace/exotic providers)
```

Each upgrade replaces a Noop/Static implementation with an intelligent one. The pipeline traits never change. The config.json gains new fields but old configs still work (missing fields use defaults = Level 0 implementations).

---

## 12. Key Gaps and Risks

| Gap | Impact | Mitigation |
|-----|--------|------------|
| ruvllm only routes Claude tiers (Haiku/Sonnet/Opus) | Must extend to 14+ providers | clawft-core wraps ruvllm's tier output and maps to clawft-llm's full provider registry |
| No HTTP transport in any ruvector crate | clawft must build the entire HTTP layer | Use reqwest + tokio; well-understood pattern |
| ruvector-sparse-inference has WASM-hostile deps (memmap2, rayon) | Cannot use for WASM offline inference | Defer local inference to post-Phase 3; use API-based only in WASM |
| prime-radiant pulls in nearly everything | Accidental full-ecosystem dependency | Never use prime-radiant directly; pick individual crates |
| sona's learning quality unproven at clawft's scale | May need tuning | Start with micro-LoRA only; add base LoRA after validation |
| WASM binary size with many crates | May exceed 300 KB target | Use zero-dep champions (temporal-tensor, cognitum-gate-kernel, micro-hnsw-wasm) |
| tiny-dancer-core pulls rusqlite + redb + simsimd | ~500 KB-1 MB binary impact; not WASM-compatible | Native only; use ruvllm-wasm for WASM routing |
| ruvector-core AgenticDB uses hash embeddings by default | Not real semantic search without embedding integration | Must configure `api-embeddings` feature or provide custom EmbeddingProvider |
| ruvllm default features include candle (local inference) | 5-15 MB binary bloat if not careful | Always use `minimal` feature for clawft; cherry-pick modules |
| ruvector crates are pre-1.0 | API instability | Pin exact versions; wrap behind clawft's own trait abstractions |
