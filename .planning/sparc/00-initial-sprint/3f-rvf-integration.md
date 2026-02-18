# SPARC Implementation Plan: Phase 3F - RVF/Ruvector Full Integration

**Timeline**: 7 weeks (Weeks 14-20), including Sprint 0 validation gate
**Owned Crates**: `clawft-core` (RVF modules), new `clawft-rvf-mcp` crate, `clawft-wasm` (microkernel)
**Dependencies**: Phases 1-2 complete; Stream 2B (basic vector-memory) complete; Stream 3A (WASM core) complete
**Branch**: `weft/3f-rvf-full`

---

## 0. Scope and Rationale

Phase 2B delivered a basic in-memory `VectorStore` with brute-force cosine similarity, a `HashEmbedder`, an `IntelligentRouter` with heuristic complexity scoring, and a `SessionIndexer`. These are purely in-memory implementations with no RVF binary format, no persistence, no HNSW, no neural routing, no self-learning, no crypto audit trail, and no MCP bridge.

Phase 3F replaces these scaffolds with the full ruvector intelligence stack:

1. **All 5 ruvector operation levels** (Level 0-4, described in `05-ruvector-crates.md` section 9)
2. **Full RVF binary format** with persistent `.rvf` files (rvf-runtime, rvf-types, rvf-index, rvf-quant)
3. **rvf-agent2b** (AgentDB adapter: `rvf-adapter-agentdb`)
4. **rvf-agentic-flow** (Swarm coordination adapter: `rvf-adapters/agentic-flow`)
5. **rvf-swarm** (ruvector swarm features bundled via feature flag)
6. **rvf-crypto** (WITNESS segments, Ed25519 chain, attestation)
7. **RVF WASM microkernel** (~28 KB, C-ABI exports in clawft-wasm)
8. **RVF ToolProvider plugin** (new `clawft-rvf-mcp` crate implementing `ToolProvider` with 11 MCP tools, registered with shared `McpServerShell` from 3H)
9. **Full intelligence crate integration**: ruvllm, ruvector-sona, ruvector-tiny-dancer-core, ruvector-attention, ruvector-temporal-tensor
10. **Development timeline items** from `04-rvf-integration.md` lines 340-351 (all items)

---

## 1. Current State Assessment

### What exists (from Phase 2B)

| Module | File | State |
|--------|------|-------|
| `VectorStore` | `clawft-core/src/vector_store.rs` | In-memory, brute-force cosine, no persistence |
| `HashEmbedder` | `clawft-core/src/embeddings/hash_embedder.rs` | SimHash-based, deterministic, no API |
| `Embedder` trait | `clawft-core/src/embeddings/mod.rs` | Trait defined, only hash impl |
| `IntelligentRouter` | `clawft-core/src/intelligent_router.rs` | Heuristic complexity scoring, in-memory policy cache |
| `SessionIndexer` | `clawft-core/src/session_indexer.rs` | In-memory, brute-force search |
| Pipeline traits | `clawft-core/src/pipeline/traits.rs` | 6-stage pipeline fully defined |
| `NoopScorer` | `clawft-core/src/pipeline/scorer.rs` | Placeholder |
| `NoopLearner` | `clawft-core/src/pipeline/learner.rs` | Placeholder |
| Feature flag | `vector-memory` | Gates embeddings, vector_store, intelligent_router, session_indexer |

### What must be built

| Component | Target | Notes |
|-----------|--------|-------|
| RVF persistent storage | `.rvf` files via rvf-runtime | Replace in-memory VectorStore |
| Progressive HNSW | 3-tier (Layer A/B/C) | Replace brute-force search |
| Temperature-based quantization | hot/warm/cold tiering | Via ruvector-temporal-tensor |
| AgentDB adapter | PolicyMemoryStore, SessionStateIndex, WitnessLog | Via rvf-adapter-agentdb |
| Agentic-flow adapter | Swarm coordination | Via rvf-adapters/agentic-flow |
| ruvllm integration | TaskComplexityAnalyzer, HnswRouter, QualityScoringEngine | Replace heuristic classifier |
| ruvector-sona integration | MicroLoRA, ReasoningBank, EwcPlusPlus | Replace NoopLearner |
| tiny-dancer integration | FastGRNN, CircuitBreaker, UncertaintyEstimator | Replace StaticRouter |
| ruvector-attention | Flash, MoE, TopologyGated, InfoBottleneck | Replace TokenBudgetAssembler |
| rvf-crypto | WITNESS segments, Ed25519 chain | New audit trail |
| WASM microkernel | rvf-wasm ~28 KB, micro-hnsw-wasm ~12 KB (with wasm-opt) | For clawft-wasm |
| RVF ToolProvider | 11 tools via `ToolProvider` trait, registered with `McpServerShell` | New crate: clawft-rvf-mcp |
| First-startup indexing | Migrate MEMORY.md to .rvf | Bootstrap path |
| API embeddings | Via clawft-llm Provider::embedding() | ApiEmbedder impl |

---

## 2. Crate Structure

### Existing crates modified

```
clawft-core/
  Cargo.toml                          # Add rvf + ruvector deps (feature-gated)
  src/
    lib.rs                            # Add rvf module declarations
    embeddings/
      mod.rs                          # Existing: Embedder trait
      hash_embedder.rs                # Existing: HashEmbedder (keep as fallback)
      api_embedder.rs                 # NEW: ApiEmbedder (LLM provider embeddings)
    rvf/                              # NEW module tree
      mod.rs                          # RVF subsystem entry point
      store.rs                        # RvfVectorStore (wraps rvf-runtime)
      hnsw.rs                         # Progressive HNSW (Layer A/B/C)
      quantizer.rs                    # Temperature-based quantization (ruvector-temporal-tensor)
      witness.rs                      # WitnessLog (rvf-crypto WITNESS segments)
      bootstrap.rs                    # First-startup MEMORY.md indexing
      agentdb.rs                      # AgentDB adapter wrapper
    routing/                          # NEW module tree (replaces intelligent_router.rs)
      mod.rs                          # Router trait re-exports
      static_router.rs               # Keep existing StaticRouter (Level 0)
      complexity.rs                   # ruvllm TaskComplexityAnalyzer wrapper (Level 1)
      neural_router.rs                # tiny-dancer FastGRNN + CircuitBreaker (Level 2)
      policy_store.rs                 # POLICY_KERNEL + COST_CURVE persistence
    learning/                         # NEW module tree
      mod.rs                          # LearningBackend impls
      sona_learner.rs                 # SONA MicroLoRA + EwcPlusPlus wrapper (Level 3)
      reasoning_bank.rs               # SONA ReasoningBank integration
    attention/                        # NEW module tree
      mod.rs                          # ContextAssembler impls (Level 4)
      attention_assembler.rs          # ruvector-attention Flash/MoE/InfoBottleneck
    session/
      indexer.rs                      # Upgrade SessionIndexer to use RVF persistence
  tests/
    rvf_store_tests.rs                # RVF store unit + integration
    routing_rvf_tests.rs              # Intelligent routing with RVF
    witness_tests.rs                  # Audit trail tests
    learning_tests.rs                 # SONA learning tests
    attention_tests.rs                # Attention assembler tests
    bootstrap_tests.rs                # First-startup migration tests
    level_integration_tests.rs        # Full L0-L4 level switching tests

clawft-wasm/
  Cargo.toml                          # Add rvf-wasm, micro-hnsw-wasm deps
  src/
    rvf_microkernel.rs                # NEW: rvf-wasm C-ABI FFI bridge
    vector.rs                         # UPDATE: integrate micro-hnsw-wasm with rvf-wasm
```

### New crate

```
clawft-rvf-mcp/                       # NEW CRATE: RVF ToolProvider (pluggable MCP)
  Cargo.toml
  src/
    lib.rs                            # Constructs RvfToolProvider, registers with McpServerShell
    provider.rs                       # Implements ToolProvider trait for RVF namespace
    tools/
      mod.rs                          # Tool registry (maps tool names to handler fns)
      create_store.rs                 # rvf_create_store tool
      open_store.rs                   # rvf_open_store tool
      ingest.rs                       # rvf_ingest tool
      query.rs                        # rvf_query tool
      status.rs                       # rvf_status tool
      delete.rs                       # rvf_delete tool
      compact.rs                      # rvf_compact tool
      delete_filter.rs                # rvf_delete_filter tool
      list_stores.rs                  # rvf_list_stores tool
      route.rs                        # rvf_route tool (expose intelligent routing)
      witness.rs                      # rvf_witness tool (audit trail query)
  tests/
    provider_tests.rs                 # ToolProvider trait conformance tests
    tool_integration_tests.rs         # Tool execution tests
```

### Workspace Cargo.toml additions

```toml
[workspace]
members = [
    # ... existing members ...
    "crates/clawft-rvf-mcp",
]

[workspace.dependencies]
# RVF core (feature-gated in clawft-core)
rvf-runtime = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
rvf-types = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
rvf-index = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
rvf-quant = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
rvf-adapter-agentdb = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
rvf-crypto = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
rvf-wasm = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }

# ruvector intelligence (feature-gated in clawft-core)
ruvector-core = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
ruvllm = { git = "https://github.com/ruvnet/ruvector", branch = "main", default-features = false, features = ["minimal"], optional = true }
ruvector-sona = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
ruvector-tiny-dancer-core = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
ruvector-attention = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
ruvector-temporal-tensor = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }

# WASM-only
micro-hnsw-wasm = { git = "https://github.com/ruvnet/ruvector", branch = "main", optional = true }
```

---

## 3. Feature Flag Design

### clawft-core/Cargo.toml

```toml
[features]
default = ["full"]
full = []
vector-memory = ["dep:rand"]

# RVF persistence layer
rvf = ["dep:rvf-runtime", "dep:rvf-types", "dep:rvf-index", "dep:rvf-quant"]
rvf-agentdb = ["rvf", "dep:rvf-adapter-agentdb", "dep:ruvector-core"]
rvf-crypto = ["rvf", "dep:rvf-crypto"]

# Intelligence crates (each independently toggleable)
ruvllm = ["dep:ruvllm"]
tiny-dancer = ["dep:ruvector-tiny-dancer-core"]
intelligent-routing = ["ruvllm", "tiny-dancer"]
sona = ["dep:ruvector-sona"]
attention = ["dep:ruvector-attention"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]

# Convenience bundles
ruvector = ["rvf-agentdb", "intelligent-routing", "sona", "attention", "temporal-tensor"]
ruvector-full = ["ruvector", "rvf-crypto"]
```

### clawft-cli/Cargo.toml

```toml
[features]
# ... existing features ...
ruvector = [
    "clawft-core/ruvector",
]
ruvector-full = [
    "ruvector",
    "clawft-core/rvf-crypto",
]
rvf-mcp = ["dep:clawft-rvf-mcp"]
```

### clawft-wasm/Cargo.toml

```toml
[features]
default = ["micro-hnsw", "temporal-tensor", "sona-wasm"]
micro-hnsw = ["dep:micro-hnsw-wasm"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]
sona-wasm = ["dep:ruvector-sona"]
rvf-wasm = ["dep:rvf-wasm"]
coherence = ["dep:cognitum-gate-kernel"]
```

### Build profiles

| Profile | Features | Estimated Binary | Use Case |
|---------|----------|-----------------|----------|
| `weft` (default) | cli + telegram + all-tools | ~5 MB | Minimal self-hosted bot |
| `weft --features ruvector` | + all Tier 1 intelligence | ~8-10 MB | Smart routing + learning |
| `weft --features ruvector-full` | + crypto audit trail | ~10-12 MB | Full intelligence + audit |
| `weft --features rvf-mcp` | + MCP server | ~11-13 MB | MCP bridge for external LLMs |
| `clawft-wasm` (default) | micro-hnsw + temporal-tensor + ruvector-sona | < 300 KB | Edge/WASM deployment |

---

## 4. Ruvector 5 Operation Levels

From `05-ruvector-crates.md` section 9, mapped to clawft implementations:

### Level 0: Static Registry (no ruvector)

- **TaskClassifier**: `KeywordClassifier` (regex/keyword match) -- EXISTING
- **ModelRouter**: `StaticRouter` (config.json ProviderSpec registry) -- EXISTING
- **ContextAssembler**: `TokenBudgetAssembler` (truncate by token count) -- EXISTING
- **LlmTransport**: `OpenAiCompatTransport` (clawft-llm) -- EXISTING
- **QualityScorer**: `NoopScorer` -- EXISTING
- **LearningBackend**: `NoopLearner` -- EXISTING
- **Memory**: Substring search on MEMORY.md -- EXISTING
- **Feature gate**: No `ruvector` feature, no additional deps

### Level 1: Complexity-Aware Routing (ruvllm)

- **TaskClassifier**: `ComplexityClassifier` wrapping `ruvllm::TaskComplexityAnalyzer`
  - 7-factor weighted scoring (token count, reasoning depth, domain specificity, context dependency, output structure, tool usage, ambiguity)
- **ModelRouter**: `HnswModelRouter` wrapping `ruvllm::HnswRouter`
  - 150x faster routing via HNSW pattern matching against known task types
- **QualityScorer**: `RuvllmQualityScorer` wrapping `ruvllm::QualityScoringEngine`
  - 5-dimension scoring (accuracy, relevance, completeness, coherence, safety)
- **Memory**: `RvfVectorStore` with rvf-runtime + progressive HNSW
- **Session**: `SessionIndexer` upgraded to rvf persistence
- **Feature gate**: `ruvllm` + `rvf`

### Level 2: Neural Routing with Resilience (+ tiny-dancer)

- **ModelRouter**: `NeuralRouter` wrapping `ruvector-tiny-dancer-core::FastGRNN`
  - Sub-millisecond neural inference for provider selection
  - `CircuitBreaker` detects provider failures, auto-routes around them
  - `UncertaintyEstimator` prevents low-confidence routing
- **Feature gate**: `intelligent-routing` (implies `ruvllm` + `tiny-dancer`)

### Level 3: Self-Learning Routing (+ ruvector-sona)

- **LearningBackend**: `SonaLearner` wrapping `ruvector_sona::SonaEngine`
  - `MicroLoRA` (rank-2): Instant per-request adaptation
  - `Base LoRA` (rank-8): Background hourly consolidation
  - `EwcPlusPlus`: Prevents catastrophic forgetting
  - `ReasoningBank`: Stores task-outcome trajectories (Reasoning, CodeGen, Conversational, Analysis, Creative)
- Three-tier learning loop: Instant (per-request) -> Background (hourly) -> Deep (weekly)
- **Feature gate**: `sona`

### Level 4: Intelligent Context Management (+ attention + temporal-tensor)

- **ContextAssembler**: `AttentionAssembler` wrapping `ruvector-attention`
  - `FlashAttention`: Efficient memory retrieval from large context windows
  - `LocalGlobalAttention`: Prioritize recent messages while maintaining long-term context
  - `MoEAttention`: Route context types through specialized attention heads
  - `InformationBottleneck`: Compress context to most relevant information
- **Quantization**: `ruvector-temporal-tensor` auto-tiered quantization
  - Hot tier: 8-bit (4x compression vs f32)
  - Warm tier: 7-bit (4.57x) or 5-bit (6.4x)
  - Cold tier: 3-bit (10.67x compression)
- **Feature gate**: `attention` + `temporal-tensor`

---

## 5. MCP Server Bridge Protocol Design

### Architecture (Pluggable ToolProvider)

The `clawft-rvf-mcp` crate implements the `ToolProvider` trait from clawft-services.
It does **not** own transport or framing -- that is handled by `McpServerShell` (delivered by Phase 3H Session 2).

```
LLM (Claude, etc.)
    |
    | JSON-RPC over stdio (newline-delimited JSON, per MCP 2025-06-18)
    v
[McpServerShell]  (clawft-services — from 3H)
    |  dispatches via ToolProvider trait
    |  applies middleware: SecurityGuard, ResultGuard
    v
[RvfToolProvider]  (clawft-rvf-mcp — namespace: "rvf")
    |
    | In-process Rust calls
    v
[clawft-core RVF subsystem]
    |
    v
[rvf-runtime] -> .rvf files on disk
```

> **Note on framing**: MCP 2025-06-18 stdio transport uses newline-delimited JSON (`\n`-separated messages),
> **not** `Content-Length` header framing (which is an LSP convention). The `McpServerShell` handles this.

### Tool Definitions (MCP Protocol)

```json
{
  "tools": [
    {
      "name": "rvf_create_store",
      "description": "Create a new RVF vector store",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Store name (used as filename)" },
          "dimension": { "type": "integer", "description": "Vector dimension (e.g. 384)" },
          "distance_metric": { "type": "string", "enum": ["cosine", "euclidean", "dot"], "default": "cosine" }
        },
        "required": ["name", "dimension"]
      }
    },
    {
      "name": "rvf_open_store",
      "description": "Open an existing RVF vector store",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string", "description": "Path to .rvf file" }
        },
        "required": ["path"]
      }
    },
    {
      "name": "rvf_ingest",
      "description": "Add vectors to an open store",
      "inputSchema": {
        "type": "object",
        "properties": {
          "store": { "type": "string", "description": "Store name or path" },
          "entries": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "text": { "type": "string" },
                "embedding": { "type": "array", "items": { "type": "number" } },
                "tags": { "type": "array", "items": { "type": "string" } },
                "metadata": { "type": "object" }
              },
              "required": ["id", "text"]
            }
          },
          "auto_embed": { "type": "boolean", "default": true, "description": "Generate embeddings automatically (requires configured embedder)" }
        },
        "required": ["store", "entries"]
      }
    },
    {
      "name": "rvf_query",
      "description": "Search vectors by embedding similarity",
      "inputSchema": {
        "type": "object",
        "properties": {
          "store": { "type": "string" },
          "query": { "type": "string", "description": "Natural language query (auto-embedded)" },
          "query_embedding": { "type": "array", "items": { "type": "number" }, "description": "Pre-computed query embedding" },
          "top_k": { "type": "integer", "default": 10 },
          "filter_tags": { "type": "array", "items": { "type": "string" } },
          "min_score": { "type": "number", "default": 0.0 }
        },
        "required": ["store"]
      }
    },
    {
      "name": "rvf_status",
      "description": "Get store metrics (entry count, index state, size)",
      "inputSchema": {
        "type": "object",
        "properties": { "store": { "type": "string" } },
        "required": ["store"]
      }
    },
    {
      "name": "rvf_delete",
      "description": "Remove vectors by ID",
      "inputSchema": {
        "type": "object",
        "properties": {
          "store": { "type": "string" },
          "ids": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["store", "ids"]
      }
    },
    {
      "name": "rvf_compact",
      "description": "Reclaim space by removing tombstoned entries",
      "inputSchema": {
        "type": "object",
        "properties": { "store": { "type": "string" } },
        "required": ["store"]
      }
    },
    {
      "name": "rvf_delete_filter",
      "description": "Delete entries matching metadata filter",
      "inputSchema": {
        "type": "object",
        "properties": {
          "store": { "type": "string" },
          "filter": { "type": "object", "description": "Key-value filter on metadata" }
        },
        "required": ["store", "filter"]
      }
    },
    {
      "name": "rvf_list_stores",
      "description": "List all open RVF stores",
      "inputSchema": { "type": "object", "properties": {} }
    },
    {
      "name": "rvf_route",
      "description": "Get intelligent routing decision for a prompt",
      "inputSchema": {
        "type": "object",
        "properties": {
          "prompt": { "type": "string" },
          "context_tags": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["prompt"]
      }
    },
    {
      "name": "rvf_witness",
      "description": "Query the cryptographic audit trail",
      "inputSchema": {
        "type": "object",
        "properties": {
          "event_type": { "type": "string", "enum": ["memory_write", "policy_update", "session_turn", "cost_record"] },
          "since_timestamp": { "type": "integer" },
          "limit": { "type": "integer", "default": 50 },
          "verify_chain": { "type": "boolean", "default": false }
        }
      }
    }
  ]
}
```

### Hooks Integration

The MCP server bridge is how hooks interact with RVF. Because `RvfToolProvider` registers
with the shared `McpServerShell`, hooks connect to a single MCP server process (not an
RVF-specific subprocess):

```
Hook (pre-task, post-edit, post-task, etc.)
    |
    | MCP tool call (e.g. rvf_ingest, rvf_query)
    v
[McpServerShell] (single shared MCP server — from 3H)
    |  routes "rvf_*" tools to RvfToolProvider
    v
[RvfToolProvider] -> [clawft-core RVF subsystem]
```

Hooks use the MCP protocol to:
- **pre-task**: `rvf_query` to retrieve relevant context
- **post-edit**: `rvf_ingest` to index changed files
- **post-task**: `rvf_witness` to record audit event
- **session-restore**: `rvf_query` to find related sessions

---

## 6. WASM Microkernel Architecture

### rvf-wasm Integration (~28 KB)

The rvf-wasm microkernel provides vector operations in WASM linear memory via 14 C-ABI exports:

```rust
// clawft-wasm/src/rvf_microkernel.rs

/// Initialize the rvf-wasm microkernel with a configuration block.
///
/// The config block is a packed struct in WASM linear memory:
///   - u32: vector dimension
///   - u32: max vectors
///   - u32: distance metric (0=cosine, 1=euclidean, 2=dot)
extern "C" {
    fn rvf_init(config_ptr: i32) -> i32;
    fn rvf_load_query(query_ptr: i32, dim: i32) -> i32;
    fn rvf_load_block(block_ptr: i32, count: i32, dtype: i32) -> i32;
    fn rvf_distances(metric: i32, result_ptr: i32) -> i32;
    fn rvf_topk_merge(dist_ptr: i32, id_ptr: i32, count: i32, k: i32) -> i32;
    fn rvf_topk_read(out_ptr: i32) -> i32;
    fn rvf_quantize(input_ptr: i32, count: i32, bits: i32, output_ptr: i32) -> i32;
    fn rvf_dequantize(input_ptr: i32, count: i32, bits: i32, output_ptr: i32) -> i32;
}

/// Safe Rust wrapper around the rvf-wasm C-ABI.
pub struct RvfWasmKernel {
    dimension: u32,
    max_vectors: u32,
    initialized: bool,
}

impl RvfWasmKernel {
    pub fn new(dimension: u32, max_vectors: u32) -> Result<Self, &'static str> { ... }
    pub fn search(&self, query: &[f32], vectors: &[f32], count: u32, k: u32) -> Vec<(u32, f32)> { ... }
    pub fn quantize(&self, input: &[f32], bits: u8) -> Vec<u8> { ... }
    pub fn dequantize(&self, input: &[u8], bits: u8) -> Vec<f32> { ... }
}
```

### micro-hnsw-wasm Integration (11.8 KB)

```rust
// clawft-wasm/src/vector.rs (upgraded)

#[cfg(feature = "micro-hnsw")]
use micro_hnsw_wasm::Hnsw;

#[cfg(feature = "rvf-wasm")]
use crate::rvf_microkernel::RvfWasmKernel;

/// Unified WASM vector store combining micro-hnsw-wasm index with
/// rvf-wasm distance computation and quantization.
pub struct WasmVectorStore {
    #[cfg(feature = "micro-hnsw")]
    hnsw: Hnsw,

    #[cfg(feature = "rvf-wasm")]
    kernel: RvfWasmKernel,

    dimension: usize,
}
```

### WASM Size Budget Impact

| Component | Size | Status |
|-----------|------|--------|
| clawft-core (WASM subset) | ~100 KB | Existing |
| micro-hnsw-wasm | 11.8 KB | Existing |
| rvf-wasm microkernel | ~28 KB | NEW (validated Sprint 0) |
| ruvector-temporal-tensor (FFI) | < 10 KB | Existing |
| ruvector-sona (WASM subset) | ~30 KB | Existing |
| rvf-types | ~30 KB | NEW |
| WASI HTTP client | ~50 KB | Existing |
| **Total** | **~240-250 KB** | Under 300 KB budget |

---

## 7. Weekly Sprint Plan

### Sprint 0 (Week 14): Dependency Validation Gate

**Goal**: Validate all ruvector crate dependencies compile and APIs match plan pseudocode. Hard go/no-go gate.

**Tasks**:
1. Compile all 11 ruvector crates against Rust 1.93+
2. Validate public API surfaces match plan pseudocode for all 8 integration points
3. Confirm ruvllm `minimal` feature excludes candle/tokenizers/hf-hub
4. Measure binary size delta for each crate
5. Document any API mismatches requiring plan corrections

**LOE**: 2 engineer-days

**Exit Criteria**:
- [ ] All 11 ruvector crates compile on Rust 1.93+
- [ ] All 8 API surfaces match plan pseudocode (or corrections documented)
- [ ] Candle bloat confirmed excluded with `minimal` feature
- [ ] Binary size impact measured and within budget projections
- [ ] Go/no-go decision recorded

---

### Sprint 1 (Week 15): RVF Runtime Foundation + ApiEmbedder + Bootstrap

**Goal**: Replace in-memory VectorStore with persistent rvf-runtime backend. Include first-startup MEMORY.md indexing (moved from old Sprint 5 per consensus -- users need existing memory indexed immediately).

**Tasks**:
1. Add rvf-runtime, rvf-types, rvf-index, rvf-quant as workspace dependencies
2. Create `clawft-core/src/rvf/mod.rs` and `clawft-core/src/rvf/store.rs`
3. Implement `RvfVectorStore` wrapping `rvf-runtime::RvfStore`
   - `new(data_dir: PathBuf, embedder: Box<dyn Embedder>)` -- opens or creates `.rvf` file
   - `add(text, tags, metadata)` -> writes VEC + META segments
   - `search(query_embedding, top_k)` -> returns sorted results
   - `remove(id)` -> tombstones entry
   - `compact()` -> reclaims tombstoned space
4. Implement `ApiEmbedder` in `clawft-core/src/embeddings/api_embedder.rs`
   - Calls clawft-llm `Provider::embedding()` endpoint
   - Supports OpenAI `text-embedding-3-small` (384-dim)
   - Falls back to `HashEmbedder` if provider unavailable
5. Wire `RvfVectorStore` into `MemoryStore` behind `#[cfg(feature = "rvf")]`
6. Implement first-startup migration in `clawft-core/src/rvf/bootstrap.rs`
   - Check for `.rvf/indexed` marker
   - Parse existing `MEMORY.md` entries
   - Batch embed (10 per call) using configured embedder
   - Write VEC + META segments
   - Build progressive HNSW
   - Write marker file
7. Write tests: store creation, add/search/remove, persistence across restarts, bootstrap from MEMORY.md

**LOE**: 6 engineer-days

**Exit Criteria**:
- [ ] `cargo build --features rvf` compiles
- [ ] `cargo build` (no rvf feature) still compiles and passes all existing tests
- [ ] RvfVectorStore persists data to `~/.clawft/workspace/memory/memory.rvf`
- [ ] Search returns relevant results with cosine > 0.5 for matching entries
- [ ] ApiEmbedder generates real embeddings via OpenAI endpoint (integration test with mock)
- [ ] First-startup indexing migrates 1000 memories in < 30 seconds
- [ ] Migration is idempotent (second run is no-op)
- [ ] Tests: >= 15 unit tests, >= 3 integration tests, >= 3 bootstrap tests

---

### Sprint 2 (Week 16): Progressive HNSW + MemoryStore Integration

**Goal**: Implement 3-tier progressive HNSW and integrate with MemoryStore search.

**Tasks**:
1. Implement `clawft-core/src/rvf/hnsw.rs` -- progressive HNSW wrapper
   - Layer A (coarse routing): loads in microseconds, ~70% recall
   - Layer B (hot region): loads based on temperature, ~85% recall
   - Layer C (full graph): complete HNSW, ~95% recall
   - Background task for progressive index building
2. Add progressive indexing to `RvfVectorStore`
   - `progressive_index()` method for background task
   - Fallback to brute-force when HNSW incomplete
   - Checkpoint every 100 insertions
3. Upgrade `IntelligentRouter` policy store to use `RvfVectorStore`
   - POLICY_KERNEL segments for cached routing decisions
   - COST_CURVE segments for model usage tracking
4. Upgrade `SessionIndexer` to use `RvfVectorStore` for persistent session index
   - Storage layout: `~/.clawft/sessions/index.rvf`
5. Write tests: progressive indexing correctness, Layer A/B/C recall, background task

**LOE**: 5 engineer-days

**Exit Criteria**:
- [ ] HNSW Layer A available within 100ms of cold start
- [ ] HNSW builds progressively in background (10 items/batch)
- [ ] Search latency < 50ms for 1000 entries with HNSW Layer C
- [ ] SessionIndexer persists index across process restarts
- [ ] IntelligentRouter policies persist in `.rvf` file
- [ ] Tests: >= 12 unit tests, >= 3 integration tests

---

### Sprint 3 (Week 17): ruvllm (Level 1) + QualityScorer

**Goal**: Replace heuristic routing with ruvllm complexity analysis. Level 2 (tiny-dancer neural routing) deferred to Phase 4 per consensus.

**Tasks**:
1. Add ruvllm (minimal feature) as workspace dep
2. Implement `clawft-core/src/routing/complexity.rs`
   - `ComplexityClassifier` implementing `TaskClassifier` trait
   - Wraps `ruvllm::TaskComplexityAnalyzer` (7-factor scoring)
   - Extends `ruvllm` tier output to map to clawft-llm's full provider registry
3. Implement `clawft-core/src/routing/policy_store.rs`
   - Persistent POLICY_KERNEL and COST_CURVE via RvfVectorStore
   - Policy update after each routing decision
4. Implement `RuvllmQualityScorer` implementing `QualityScorer` trait
   - Wraps `ruvllm::QualityScoringEngine` (5-dimension scoring)
5. Wire Level 1 into `build_default_pipeline()` behind feature gates
6. Write tests: complexity scoring accuracy, quality scoring dimensions, policy persistence

**LOE**: 4 engineer-days

**Exit Criteria**:
- [ ] ComplexityClassifier produces complexity scores within 10% of ruvllm reference
- [ ] QualityScorer produces 5-dimension scores for any response
- [ ] Policy store persists POLICY_KERNEL and COST_CURVE across restarts
- [ ] Tests: >= 12 unit tests, >= 3 integration tests

---

### Sprint 4 (Week 18): RvfToolProvider + MCP Registration

**Goal**: Build `clawft-rvf-mcp` as a `ToolProvider` plugin, registering with the shared `McpServerShell` from Phase 3H. Moved earlier per consensus (was Sprint 6) to align with 3H Session 2 delivery in Week 18.

**Gates on**: Phase 3H Session 2 delivering `ToolProvider` trait, `McpServerShell`, and middleware pipeline (`SecurityGuard`, `ResultGuard`).

**Tasks**:
1. Create `clawft-rvf-mcp` crate with Cargo.toml (depends on clawft-services for `ToolProvider` trait)
2. Implement `RvfToolProvider` in `provider.rs`
   - `impl ToolProvider for RvfToolProvider`
   - `fn namespace(&self) -> &str` returns `"rvf"`
   - `async fn list_tools(&self) -> Vec<ToolDefinition>` returns all 11 tool definitions
   - `async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError>` dispatches to tool handlers
3. Implement all 11 tool handlers (unchanged from prior plan):
   - `rvf_create_store`, `rvf_open_store`, `rvf_ingest`, `rvf_query`
   - `rvf_status`, `rvf_delete`, `rvf_compact`, `rvf_delete_filter`
   - `rvf_list_stores`, `rvf_route`, `rvf_witness`
4. Implement store lifecycle management
   - Stores opened/created on demand
   - LRU eviction for idle stores (configurable, default 10)
   - Graceful shutdown flushes all stores
5. Register `RvfToolProvider` with `McpServerShell` in `lib.rs`:
   ```rust
   pub fn register(shell: &mut McpServerShell) {
       let provider = RvfToolProvider::new(rvf_config);
       shell.register_provider(Box::new(provider));
   }
   ```
6. **No transport code** -- `McpServerShell` owns stdio framing (newline-delimited JSON per MCP 2025-06-18)
7. Write tests: `ToolProvider` trait conformance, each tool handler, error handling

**LOE**: 6 engineer-days

**Exit Criteria**:
- [ ] `RvfToolProvider` implements `ToolProvider` trait from clawft-services
- [ ] `list_tools()` returns all 11 tool definitions with correct schemas
- [ ] `call_tool()` dispatches to correct handler for each of the 11 tools
- [ ] `rvf_create_store` + `rvf_ingest` + `rvf_query` round-trip works via `call_tool()`
- [ ] `rvf_route` returns a routing decision with tier + model + reason
- [ ] `rvf_witness` returns audit events (when rvf-crypto enabled)
- [ ] `RvfToolProvider` registers successfully with `McpServerShell`
- [ ] Error results conform to `ToolError` variants (not raw JSON-RPC errors)
- [ ] Tests: >= 18 unit tests, >= 5 integration tests

---

### Sprint 5 (Week 19): rvf-crypto + AgentDB

**Goal**: Implement cryptographic audit trail and AgentDB adapter. Agentic-flow integration deferred to Phase 4 per consensus. Bootstrap moved to Sprint 1.

**Tasks**:
1. Implement `clawft-core/src/rvf/witness.rs`
   - `WitnessLog` using rvf-crypto WITNESS segments
   - Ed25519 signatures over event content
   - Merkle chain linking each WITNESS to previous
   - `record_event(event)` -> returns event ID
   - `verify_chain()` -> validates entire chain integrity
   - `query_events(filter)` -> returns matching events
   - Supports semantic search over audit trail (embedding + HNSW)
2. Implement `clawft-core/src/rvf/agentdb.rs`
   - Wraps `rvf-adapter-agentdb::RvfVectorStore` into clawft abstractions
   - `PolicyMemoryStore`, `SessionStateIndex` via AgentDB
   - Configures `api-embeddings` feature for real semantic search
3. Write tests: chain integrity verification, tamper detection, AgentDB adapter conformance

**LOE**: 4 engineer-days

**Exit Criteria**:
- [ ] WitnessLog appends events with valid Ed25519 signatures
- [ ] `verify_chain()` returns true for unmodified chain, false for tampered chain
- [ ] AgentDB adapter passes all existing VectorStore tests
- [ ] Tests: >= 15 unit tests, >= 4 integration tests

---

### Sprint 6 (Week 20): Polish, Benchmarks, CLI, Level Integration Tests

**Goal**: Performance validation, size budget finalization, full test coverage, CLI integration. Consolidates old Sprints 7-8 (WASM microkernel deferred to Phase 4 per consensus).

**Tasks**:
1. Performance benchmarks (native):
   - Memory search latency for 1K/10K/100K entries
   - Progressive HNSW build time for 1K/10K entries
   - Routing decision latency (Level 0-1)
   - Embedding generation latency (API vs Hash)
2. Binary size profiling:
   - `twiggy top -n 30` for native binary with ruvector-full
   - Identify any oversized dependencies
3. Security audit:
   - WitnessLog signature validation
   - Embedding API key handling (env vars only, never in .rvf files)
   - RVF file permissions (0700)
   - Input validation on MCP tool calls
4. Level integration tests in `clawft-core/tests/level_integration_tests.rs`
   - Test Level 0 -> 1 upgrade path
   - Test graceful degradation when features disabled
5. Test coverage gap analysis:
   - Ensure >= 80% coverage on all new code
   - 100% coverage on critical paths (routing decisions, witness chain, bootstrap)
6. CLI integration:
   - `weft memory search "<query>"` -- semantic search via RVF
   - `weft memory stats` -- show RVF index status (entries, HNSW state, size)
   - `weft routing stats` -- show tier usage, costs
   - `weft session search "<query>"` -- semantic session search

**LOE**: 5 engineer-days

**Exit Criteria**:
- [ ] Memory search < 100ms for 10K entries (HNSW Layer C)
- [ ] Embedding generation < 200ms (API), < 5ms (Hash)
- [ ] Native binary (ruvector-full) < 12 MB
- [ ] Test coverage >= 80% for all RVF modules
- [ ] Level integration tests pass for L0-L1 upgrade and graceful degradation
- [ ] Security audit passes (no hardcoded keys, no path traversal, chain tamper detection)

---

## 8. Test Strategy

### Test Categories

| Category | Location | Count Target | Coverage |
|----------|----------|-------------|----------|
| RVF store unit tests | `clawft-core/tests/rvf_store_tests.rs` | >= 20 | Store CRUD, persistence, search |
| HNSW tests | `clawft-core/tests/rvf_store_tests.rs` | >= 8 | Progressive build, Layer A/B/C recall |
| Routing tests | `clawft-core/tests/routing_rvf_tests.rs` | >= 18 | All 3 levels, policy cache, circuit breaker |
| Learning tests | `clawft-core/tests/learning_tests.rs` | >= 12 | SONA, EwcPlusPlus, ReasoningBank |
| Attention tests | `clawft-core/tests/attention_tests.rs` | >= 8 | Flash, MoE, InfoBottleneck |
| Witness tests | `clawft-core/tests/witness_tests.rs` | >= 10 | Signing, chain validation, tamper detection |
| Bootstrap tests | `clawft-core/tests/bootstrap_tests.rs` | >= 6 | Migration, idempotency |
| ToolProvider tests | `clawft-rvf-mcp/tests/provider_tests.rs` | >= 15 | ToolProvider trait conformance, dispatch, errors |
| MCP tool tests | `clawft-rvf-mcp/tests/tool_integration_tests.rs` | >= 11 | One per tool via call_tool() |
| WASM tests | `clawft-wasm/tests/rvf_microkernel_tests.rs` | >= 8 | FFI, search, quantize |
| Level integration | `clawft-core/tests/level_integration_tests.rs` | >= 10 | L0-L4, upgrade, downgrade |
| **Total** | | **>= 126** | **>= 80%** |

### Test Infrastructure

```rust
// Shared test utilities
mod test_utils {
    /// Create a temporary directory with an RVF store for testing.
    pub fn temp_rvf_store(dim: usize) -> (TempDir, RvfVectorStore) { ... }

    /// Create a mock embedder that returns deterministic embeddings.
    pub fn mock_embedder(dim: usize) -> Box<dyn Embedder> {
        Box::new(HashEmbedder::new(dim))
    }

    /// Create a mock LLM provider that returns canned responses.
    pub fn mock_provider() -> Arc<dyn Provider> { ... }

    /// Create test MEMORY.md content with N entries.
    pub fn test_memory_content(n: usize) -> String { ... }
}
```

### Feature Flag Test Matrix

All tests must pass for each feature combination:

| Build | Command | Must Pass |
|-------|---------|-----------|
| No features | `cargo test -p clawft-core --no-default-features` | All non-RVF tests |
| vector-memory only | `cargo test -p clawft-core --features vector-memory` | + embeddings, vector_store, intelligent_router, session_indexer |
| rvf only | `cargo test -p clawft-core --features rvf` | + RVF store, progressive HNSW |
| ruvllm only | `cargo test -p clawft-core --features ruvllm` | + complexity classifier, quality scorer |
| intelligent-routing | `cargo test -p clawft-core --features intelligent-routing` | + neural router, circuit breaker |
| sona | `cargo test -p clawft-core --features sona` | + ruvector-sona learner, reasoning bank |
| ruvector (all) | `cargo test -p clawft-core --features ruvector` | All Level 0-4 tests |
| ruvector-full | `cargo test -p clawft-core --features ruvector-full` | + witness log, crypto |
| MCP crate | `cargo test -p clawft-rvf-mcp` | All ToolProvider + tool handler tests |
| WASM | `cargo test -p clawft-wasm` | WASM-specific tests |

---

## 9. Size Budget Impact

### Native Binary Impact

| Feature | Additional Size | Cumulative |
|---------|----------------|------------|
| Base (no ruvector) | ~5 MB | 5 MB |
| + rvf (runtime + types + index + quant) | ~300 KB | 5.3 MB |
| + ruvector-core (AgentDB) | ~300 KB | 5.6 MB |
| + ruvllm (minimal) | ~2 MB | 7.6 MB |
| + ruvector-sona | ~100 KB | 7.7 MB |
| + ruvector-tiny-dancer-core | ~700 KB | 8.4 MB |
| + ruvector-attention | ~80 KB | 8.5 MB |
| + ruvector-temporal-tensor | ~40 KB | 8.5 MB |
| + rvf-crypto | ~50 KB | 8.6 MB |
| + clawft-rvf-mcp | ~200 KB | 8.8 MB |
| **Total (ruvector-full + MCP)** | **~3.8 MB** | **~8.8 MB** |

Target: < 12 MB. Estimated: ~8.8 MB. Headroom: ~3.2 MB.

### WASM Binary Impact

| Component | Size | Notes |
|-----------|------|-------|
| Existing WASM core | ~242 KB | micro-hnsw, temporal-tensor, sona, etc. |
| + rvf-wasm microkernel | ~28 KB | C-ABI exports, no wasm-bindgen (Sprint 0 validated) |
| + rvf-types | ~30 KB | Segment parsing (already counted in existing budget) |
| **Total** | **~250 KB** | Under 300 KB budget |

---

## 10. Development Timeline Items (04-rvf-integration.md lines 340-351)

Cross-reference with sprint allocation:

| Week (orig) | Task | Sprint | Status |
|-------------|------|--------|--------|
| 7 | Add rvf-runtime + rvf-types as workspace deps (feature-gated) | Sprint 1 | Covered |
| 8 | MemoryStore: integrate RvfVectorStore for semantic search | Sprint 1-2 | Covered |
| 8 | IntelligentRouter: basic pattern-matched routing | Sprint 2-3 | Covered |
| 9 | SessionManager: index session turns in rvf | Sprint 2 | Covered |
| 10 | IntelligentRouter: learned routing policies | Sprint 3 | Covered |
| 11 | WitnessLog: audit trail via rvf WITNESS segments | Sprint 5 | Covered |
| 11 | First-startup memory indexing from existing MEMORY.md | Sprint 1 | Covered (moved from Sprint 5 per consensus) |
| 13 | WASM: integrate rvf-wasm microkernel for vector ops | -- | Deferred to Phase 4 per consensus |

7 of 8 development timeline items are covered. WASM microkernel deferred to Phase 4.

---

## 11. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| ruvllm `minimal` feature still pulls candle (5-15 MB bloat) | Medium | High | Pin exact version; verify binary delta before/after; fallback to cherry-picking modules |
| tiny-dancer pulls rusqlite + redb (WASM-incompatible) | Known | Medium | Native only; use ruvllm-wasm for WASM routing; document the native-only constraint |
| ruvector crates pre-1.0 API instability | High | Medium | Pin exact git commits in Cargo.toml; wrap behind clawft's own trait abstractions |
| RVF file corruption (truncated writes, power loss) | Low | High | rvf-runtime has crash-safe design (no WAL needed); add integrity check on open |
| SONA learning quality unproven at clawft scale | Medium | Low | Start with MicroLoRA only; add base LoRA after validation; measure quality improvement |
| ToolProvider dispatch adds latency to hook calls | Low | Low | In-process Rust calls (no IPC); keep stores in-memory once opened; LRU cache; benchmark < 1ms per call_tool() |
| 3H Session 2 delays block Sprint 4 | Medium | High | Sprint 4 can be re-ordered after Sprint 5 (rvf-crypto) if 3H is late; tool handler code is independent of ToolProvider trait |
| ruvector-core AgentDB uses hash embeddings by default | Known | Medium | Must configure `api-embeddings` feature or provide ApiEmbedder integration |
| WASM binary size exceeds 300 KB with rvf-wasm | Low | High | rvf-wasm is ~28 KB (Sprint 0 validated); use C-ABI (not wasm-bindgen); profile with twiggy |

---

## 12. Cross-Stream Dependencies

### Inputs (what 3F depends on)

| Dependency | Source | What We Need | Blocks Sprint |
|------------|--------|-------------|---------------|
| clawft-core pipeline traits | Phase 1B | `TaskClassifier`, `ModelRouter`, `ContextAssembler`, `QualityScorer`, `LearningBackend` (stable) | 1 |
| clawft-core bootstrap.rs | Phase 1B | `AppContext`, `build_default_pipeline()` | 1 |
| clawft-llm Provider trait | Phase 1C | `Provider::embedding()` method for ApiEmbedder | 1 |
| clawft-wasm platform | Phase 3A | WASI HTTP, FS, Env implementations | Deferred (Phase 4) |
| vector-memory feature | Phase 2B | VectorStore, HashEmbedder, Embedder trait, IntelligentRouter, SessionIndexer | 1 |
| **`ToolProvider` trait** | **Phase 3H Session 2** | **`ToolProvider` trait definition (`namespace`, `list_tools`, `call_tool`)** | **4** |
| **`McpServerShell`** | **Phase 3H Session 2** | **Shared MCP server shell: stdio transport, newline-delimited JSON framing, provider registry** | **4** |
| **Middleware pipeline** | **Phase 3H Session 2** | **`SecurityGuard` (input validation), `ResultGuard` (output sanitization) middleware** | **4** |

### Outputs (what 3F provides to others)

| Consumer | What We Provide |
|----------|----------------|
| clawft-cli | `weft memory search`, `weft routing stats`, `weft session search`, `weft rvf-mcp` commands |
| clawft-wasm | rvf-wasm microkernel, upgraded vector store |
| Hooks (via McpServerShell) | `RvfToolProvider` with 11 tools, registered with shared `McpServerShell` |
| Future: clawft-channels | Semantic message search for conversation context |

---

## 13. Sprint Summary

| Sprint | Week | Focus | LOE | Cumulative Tests |
|--------|------|-------|-----|-----------------|
| 0 | 14 | Dependency validation (go/no-go gate) | 2d | -- |
| 1 | 15 | RVF Runtime Foundation + ApiEmbedder + Bootstrap | 6d | ~21 |
| 2 | 16 | Progressive HNSW + Session/Router persistence | 5d | ~36 |
| 3 | 17 | ruvllm (L1 only) + QualityScorer | 4d | ~51 |
| 4 | 18 | RvfToolProvider + MCP Registration (gates on 3H S2) | 6d | ~74 |
| 5 | 19 | rvf-crypto + AgentDB | 4d | ~93 |
| 6 | 20 | Polish, Benchmarks, CLI, Level integration tests | 5d | ~103+ |
| **Total** | **7 weeks** | | **32d** | **>= 100 tests** |

---

## 14. Phase 3F Exit Criteria

### MUST HAVE (blocks release)

1. Ruvector operation Levels 0-1 implemented and tested (L2-L4 deferred to Phase 4 per consensus)
2. RVF persistence: `.rvf` files survive process restart
3. Progressive HNSW: Layer A available in < 100ms cold start
4. ruvllm complexity analysis integrated as TaskClassifier
5. rvf-crypto WITNESS log with Ed25519 chain verification
6. `RvfToolProvider` implements `ToolProvider` trait with all 11 tools, registered with `McpServerShell`
7. First-startup MEMORY.md migration working
8. All feature flags tested independently and in combination
9. Test coverage >= 80% on all new code
10. Native binary with ruvector-full < 12 MB
11. All existing tests still pass with no ruvector features

### SHOULD HAVE (important but not blocking)

12. `RvfToolProvider::call_tool()` latency < 1ms per dispatch (excluding store I/O)
13. CLI commands `weft memory search`, `weft routing stats`, `weft session search`

### NICE TO HAVE (deferred to Phase 4)

14. tiny-dancer neural routing (Level 2)
15. SONA self-learning (Level 3) + temporal-tensor quantization
16. Attention-based context assembly (Level 4)
17. WASM microkernel (rvf-wasm) integration
18. ruvector-graph knowledge graph integration (Level 5)
19. ruvector-domain-expansion cross-domain transfer
20. rvlite browser-compatible WASM vector DB
21. Performance parity between MCP bridge and direct API calls

---

## 15. Notes for Implementation Agent

1. **READ 04-rvf-integration.md and 05-ruvector-crates.md FIRST** -- these are the source of truth for all RVF/ruvector APIs
2. **Pin exact git commits** for all ruvector dependencies -- these are pre-1.0 crates
3. **Feature flags are mandatory** -- every RVF module must compile-out cleanly
4. **Use TDD London School** -- mock ruvector interfaces, write failing tests, implement
5. **Existing code stays working** -- `cargo build` with no features must always pass
6. **ruvllm MUST use `minimal` feature** -- do NOT pull in candle/tokenizers/hf-hub
7. **tiny-dancer is native only** -- do NOT attempt WASM compilation
8. **ApiEmbedder is the production path** -- HashEmbedder is fallback/testing only
9. **RvfToolProvider is the MCP integration path** -- hooks call RVF via MCP tool calls routed through McpServerShell to RvfToolProvider; no standalone MCP server binary needed
10. **WASM microkernel uses C-ABI** -- no wasm-bindgen, no Rust struct passing across FFI
11. **Error handling: graceful degradation** -- if rvf-runtime fails to open, fall back to in-memory VectorStore
12. **Logging**: use `tracing` with structured fields for all RVF operations
13. **Never hardcode paths** -- use `Platform::home_dir()` + config for all .rvf file locations
14. **Performance targets are real** -- benchmark each sprint and track regressions
15. **The 5 levels are opt-in** -- each level adds intelligence without requiring the next
16. **No transport code in clawft-rvf-mcp** -- `McpServerShell` from 3H owns stdio framing (newline-delimited JSON per MCP 2025-06-18); `RvfToolProvider` only implements `namespace()`, `list_tools()`, `call_tool()`
17. **Sprint 4 gates on 3H Session 2** -- if 3H is delayed, implement tool handlers first (they are pure Rust functions), wire ToolProvider last; Sprint 5 (rvf-crypto) can run in parallel
