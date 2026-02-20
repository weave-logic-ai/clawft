# Development Assignment: Element 08 -- Memory & Workspace

**Element**: 08
**Workstream**: H (Memory & Workspace)
**Timeline**: Weeks 4-8
**Dependencies**: 04/C1 (MemoryBackend trait), 03/A2 (stable hash for vector memory)
**Blocks**: 10/K4 (ClawHub needs vector search from H2)
**Branch**: `sprint/phase-5` (stream 5G)

---

## Overview

Per-agent workspace isolation, HNSW-backed vector memory, RVF Phase 3 completion, and timestamp standardization. This element builds the foundation for agent-scoped storage, semantic search, and tamper-evident memory audit trails.

---

## Unit 1: H1 Per-Agent Workspace + H3 Timestamps (Week 4-6)

### Objective

Extend the existing `WorkspaceManager` in `clawft-core/src/workspace.rs` to support per-agent workspace isolation under `~/.clawft/agents/<agentId>/`, and standardize all timestamp types to `DateTime<Utc>`.

### Current State

The `WorkspaceManager` already manages global workspaces with create/list/load/status/delete operations. It creates `.clawft/` subdirectories including an `agents/` directory, but does not create per-agent directories within it.

**Existing code** (`crates/clawft-core/src/workspace.rs:73-82`):

```rust
const WORKSPACE_SUBDIRS: &[&str] = &["sessions", "memory", "skills", "agents", "hooks"];

pub struct WorkspaceManager {
    registry_path: PathBuf,
    registry: WorkspaceRegistry,
}
```

The `WorkspaceEntry` type uses `Option<String>` for timestamps (`crates/clawft-types/src/workspace.rs:12-26`):

```rust
pub struct WorkspaceEntry {
    pub name: String,
    pub path: PathBuf,
    pub last_accessed: Option<String>,  // H3: change to DateTime<Utc>
    pub created_at: Option<String>,     // H3: change to DateTime<Utc>
}
```

Other timestamp inconsistencies:
- `CronJob.created_at_ms`: `i64` (milliseconds) -- needs `DateTime<Utc>`
- `InboundMessage.timestamp`: Already `DateTime<Utc>` (correct)

### Deliverables

#### 1.1 Per-Agent Workspace Methods

Add to `WorkspaceManager` in `crates/clawft-core/src/workspace.rs`:

```rust
impl WorkspaceManager {
    /// Create or ensure a per-agent workspace exists.
    /// Idempotent: returns path if workspace already exists.
    pub fn ensure_agent_workspace(&self, agent_id: &str) -> Result<PathBuf> {
        // ~/.clawft/agents/<agent_id>/
        // Creates: SOUL.md, AGENTS.md, USER.md, config.toml
        // Plus subdirs: sessions/, memory/, skills/
    }

    /// Create a per-agent workspace from a template.
    pub fn create_agent_workspace(
        &self,
        agent_id: &str,
        template: Option<&Path>,
    ) -> Result<PathBuf> {
        // If template is None, use ~/.clawft/agents/default/ if it exists
        // Otherwise create bare minimum
    }

    /// Delete a per-agent workspace by agent ID.
    pub fn delete_agent_workspace(&self, agent_id: &str) -> Result<()> { ... }

    /// List all agent workspaces.
    pub fn list_agent_workspaces(&self) -> Result<Vec<String>> { ... }
}
```

Per-agent directory structure:
```
~/.clawft/agents/<agentId>/
  SOUL.md          # Agent personality
  AGENTS.md        # Agent capabilities
  USER.md          # User preferences for this agent
  config.toml      # Per-agent config overrides
  sessions/        # Per-agent session store
  memory/          # Per-agent memory namespace
  skills/          # Per-agent skill overrides
  tool_state/      # Per-plugin state (contract 3.1)
```

#### 1.2 Cross-Agent Shared Memory Protocol

Symlink-based sharing with read-only default:

```toml
# In ~/.clawft/agents/agent-a/config.toml
shared_namespaces = ["project-context"]

# In ~/.clawft/agents/agent-b/config.toml
[[import_namespaces]]
agent = "agent-a"
namespace = "project-context"
read_write = false  # default
```

Implementation: create symlinks from agent-b's memory directory to agent-a's exported namespace. Filesystem-level locking for write access (use `fs2::FileExt` or `file-lock`).

#### 1.3 H3 Timestamp Standardization

Update `WorkspaceEntry` in `crates/clawft-types/src/workspace.rs`:

```rust
use chrono::{DateTime, Utc};

pub struct WorkspaceEntry {
    pub name: String,
    pub path: PathBuf,
    pub last_accessed: Option<DateTime<Utc>>,  // was Option<String>
    pub created_at: Option<DateTime<Utc>>,      // was Option<String>
}
```

Also update `CronJob` types to use `DateTime<Utc>` instead of `i64` milliseconds.

### Cross-Element Dependencies

- **Contract 3.3 (Workspace <-> Routing)**: H1 owns `WorkspaceManager`. L2 (Element 09, Unit 2) calls `ensure_agent_workspace(agent_id)` when routing to a new agent. Workspace deletion is administrative only.
- **Contract 3.1 (Tool Plugin <-> Memory)**: Per-agent workspace includes `tool_state/<plugin_name>/` directories for plugin state persistence via `KeyValueStore` trait.

### Security Criteria

- [ ] Agent workspace directories created with `0700` permissions
- [ ] Symlink targets validated (no escape to arbitrary paths)
- [ ] `read_write = true` requires explicit opt-in with fs-level locking

### Acceptance Criteria

- [ ] `ensure_agent_workspace("test-agent")` creates full directory structure
- [ ] `ensure_agent_workspace` is idempotent (second call returns same path)
- [ ] `delete_agent_workspace` removes the directory
- [ ] `list_agent_workspaces` lists all agent IDs
- [ ] Cross-agent symlink sharing works for read-only access
- [ ] `WorkspaceEntry.last_accessed` and `created_at` use `DateTime<Utc>`
- [ ] All existing workspace tests pass after timestamp migration

### Test Requirements

- Unit tests for each new `WorkspaceManager` method
- Test idempotency of `ensure_agent_workspace`
- Test symlink creation and read-only enforcement
- Migration test: old `String` timestamps deserialize into `DateTime<Utc>`

---

## Unit 2: H2.1 HNSW + H2.2 Production Embedder (Week 5-7)

### Objective

Replace the brute-force cosine scan with HNSW-backed vector search using `instant-distance`, and define the `Embedder` trait with `HashEmbedder` + `ApiEmbedder` implementations.

### Current State

The `Embedder` trait already exists in `crates/clawft-core/src/embeddings/mod.rs`:

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> { ... }
    fn dimension(&self) -> usize;
}
```

`HashEmbedder` exists (`crates/clawft-core/src/embeddings/hash_embedder.rs`) but uses `std::hash::DefaultHasher` -- **unstable across Rust versions** (bug A2). Must be fixed before vector memory work.

`ApiEmbedder` exists (`crates/clawft-core/src/embeddings/api_embedder.rs`) with SHA-256 fallback when no API key is set. Already calls OpenAI-compatible `/embeddings` endpoint.

No HNSW index exists. Current vector search is brute-force.

### Deliverables

#### 2.1 HNSW Integration

**Dependency**: A2 must be fixed first (stable hash function).

Add `instant-distance` to `clawft-core/Cargo.toml`:

```toml
[dependencies]
instant-distance = { version = "0.6", optional = true }

[features]
vector-memory = ["dep:rand", "dep:instant-distance"]
```

Create `crates/clawft-core/src/embeddings/hnsw_store.rs`:

```rust
use instant_distance::{Builder, Search, HnswMap};
use super::{Embedder, EmbeddingError};

pub struct HnswVectorStore {
    /// Immutable HNSW index (rebuilt on mutation).
    index: Option<HnswMap<Point, String>>,
    /// Raw vectors + keys for rebuild.
    entries: Vec<(String, Vec<f32>)>,
    /// Embedder for new insertions.
    embedder: Arc<dyn Embedder>,
}

impl HnswVectorStore {
    pub fn new(embedder: Arc<dyn Embedder>) -> Self { ... }
    pub async fn insert(&mut self, key: String, text: &str) -> Result<(), EmbeddingError> { ... }
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> { ... }
    pub fn rebuild_index(&mut self) { ... }  // Periodic full re-index
}
```

Key design notes:
- `instant-distance` builds immutable indices -- mitigate with periodic full re-index
- Acceptable for <100K vectors at <10ms search target
- Background re-index with read-continue semantics

#### 2.2 Embedder Trait Enhancement

The trait is already defined. Enhance per orchestrator spec (Section 5):

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    fn dimensions(&self) -> usize;  // rename from dimension()
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn name(&self) -> &str;  // new: embedder identifier
}
```

The orchestrator specifies `embed(&self, texts: &[String])` (batch-only). The current trait has both `embed` (single) and `embed_batch`. Decision: keep both for ergonomics, but the trait signature should match the orchestrator spec for the primary method.

#### 2.3 Async Embedding Pipeline

Per orchestrator Section 9:

1. `store()` writes raw data immediately (keyword-searchable)
2. `store()` spawns background `tokio::spawn` for embedding
3. Background task calls `Embedder.embed()` and inserts into HNSW
4. `pending_embeddings` queue tracks in-flight items
5. Fallback: if embedding API unavailable, items stay keyword-only (exponential backoff, max 3 retries)

### Cross-Element Dependencies

- **A2 (Element 03)**: Must fix unstable hash function in `HashEmbedder` before any persisted embeddings
- **C1 (Element 04)**: `MemoryBackend` plugin trait accepts `Arc<dyn Embedder>` at construction

### Security Criteria

- [ ] API keys resolved from environment variables only (never hardcoded)
- [ ] Embedding API calls use HTTPS only
- [ ] Failed API calls logged at `warn` level without leaking request content

### Acceptance Criteria

- [ ] HNSW search returns relevant results for cosine similarity queries
- [ ] `instant-distance` integrated as the HNSW backend
- [ ] `HashEmbedder` uses stable deterministic hash (post-A2 fix)
- [ ] `ApiEmbedder` calls OpenAI-compatible endpoint successfully
- [ ] Async pipeline: `store()` returns immediately, embedding runs in background
- [ ] `pending_embeddings` queue tracks in-flight items
- [ ] Fallback to keyword-only when embedding API unavailable

### Test Requirements

- Property-based tests for HNSW correctness (insert N vectors, query returns nearest)
- Test periodic re-index produces same search results
- Test async pipeline: verify item is keyword-searchable before embedding completes
- Test fallback behavior when API is unavailable

---

## Unit 3: H2.3-H2.5 RVF I/O + Export/Import + Policy Kernel (Week 6-7)

### Objective

Implement real RVF segment I/O for memory persistence, add `weft memory export/import` CLI commands, and persist `POLICY_KERNEL` routing policies.

### Current State

- RVF 0.2 integration exists as stubs (`crates/clawft-core/src/embeddings/rvf_stub.rs`)
- `ProgressiveSearch` saves as plain JSON, not RVF format
- CLI has `weft memory show/history/search` but no `export/import`
- `IntelligentRouter` routing policies not persisted across restarts

### Deliverables

#### 3.1 RVF 0.2 Audit (Week 4 pre-work)

Before implementation, audit `rvf-runtime` 0.2 API:
- Verify segment read/write operations exist in public API
- Verify segment types support vector data, metadata, timestamps
- Verify WITNESS metadata can be attached

**If `rvf-runtime` 0.2 only provides stubs**: implement local serialization using `rvf-types` directly.

#### 3.2 RVF Segment I/O

Replace plain JSON persistence with RVF segment format:
- File location: `~/.clawft/agents/<agentId>/memory/*.rvf`
- Each memory entry is an RVF segment with vector data + metadata

#### 3.3 Export/Import CLI

New CLI commands in `crates/clawft-cli/src/commands/`:

```
weft memory export --agent <id> --output <path>
weft memory import --agent <id> --input <path>
```

Export produces a portable RVF file. Import validates and loads it.

#### 3.4 POLICY_KERNEL Persistence

Persist `IntelligentRouter` routing policies to disk:
- Location: `~/.clawft/memory/policy_kernel.json`
- Load on startup, save on policy changes
- Format: JSON (simple, debuggable)

### Cross-Element Dependencies

- **RVF 0.2 audit**: Must complete before H2.3 implementation begins
- **H2.1 (Unit 2)**: HNSW store must be available for vector segment I/O

### Security Criteria

- [ ] RVF files validated before import (no arbitrary code execution)
- [ ] File permissions on memory directories are `0700`

### Acceptance Criteria

- [ ] RVF 0.2 API audit completed with findings documented
- [ ] Segment I/O path confirmed (rvf-runtime or local implementation)
- [ ] `weft memory export` produces valid RVF file
- [ ] `weft memory import` loads and validates RVF file
- [ ] `POLICY_KERNEL` persisted and restored across restarts

### Test Requirements

- Roundtrip test: export then import, verify data integrity
- Test import rejects malformed files
- Test POLICY_KERNEL persistence across simulated restarts

---

## Unit 4: H2.6-H2.8 WITNESS + Quantization + WASM HNSW (Week 7-8)

### Objective

Advanced memory features: tamper-evident audit trails, temperature-based storage tiers, and a separate WASM HNSW module.

### Current State

None of these features exist. The `sha2` crate is already a workspace dependency.

### Deliverables

#### 4.1 WITNESS Segments (H2.6)

Per orchestrator Section 8, SHA-256 hash chain:

```rust
pub struct WitnessSegment {
    pub segment_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub operation: WitnessOperation,  // Store, Update, Delete
    pub data_hash: [u8; 32],          // SHA-256 of stored data
    pub previous_hash: [u8; 32],      // SHA-256 of previous segment
}
```

- Chain starts with `previous_hash = [0u8; 32]`
- Verification: sequential scan from root, recompute each hash
- All memory write paths (H2.1-H2.5) optionally include WITNESS metadata
- `weft memory export` includes WITNESS chain
- `weft memory import` validates chain before importing

The `sha2` crate is already available in the workspace.

#### 4.2 Temperature-Based Quantization (H2.7)

Per orchestrator Section 6, storage-layer only (single HNSW index with full-precision):

| Tier | Storage | Access Pattern |
|------|---------|---------------|
| Hot | `Vec<f32>` full precision | Frequently accessed |
| Warm | fp16 on disk, decompressed to f32 | Moderate access |
| Cold | Product-quantized (PQ) on disk | Rarely accessed |

- HNSW index always uses full-precision pointers
- No index rebuild on tier transition
- Transitions driven by access frequency and recency

#### 4.3 WASM micro-HNSW (H2.8)

Separate WASM module (`micro-hnsw-wasm`) with **8KB size budget**:
- NOT bundled in main `clawft-wasm` crate (300KB budget)
- Communicates with main WASM agent via message passing
- Keeps vector search optional in the browser/edge agent

### Cross-Element Dependencies

- **sha2 crate**: Already in workspace deps
- **H2.1 (Unit 2)**: HNSW store provides the index for quantization
- **H2.3 (Unit 3)**: WITNESS integrates with RVF I/O for export/import

### Security Criteria

- [ ] WITNESS hash chain uses SHA-256 (cryptographic, not hash-map hash)
- [ ] Chain verification detects any segment tampering
- [ ] WASM module sandboxed, no filesystem access

### Acceptance Criteria

- [ ] WITNESS segments use SHA-256 hash chaining
- [ ] Sequential verification from root detects tampering
- [ ] `weft memory export` includes WITNESS chain
- [ ] `weft memory import` validates WITNESS chain
- [ ] Temperature tiers work: hot in memory, warm/cold on disk
- [ ] Tier transitions transparent to search (no index rebuild)
- [ ] WASM micro-HNSW compiled size < 8KB
- [ ] WASM module communicates via message passing

### Test Requirements

- WITNESS chain integrity test: insert segments, verify chain, tamper with one, detect
- Quantization roundtrip: store as hot, transition to warm, verify search still works
- WASM module size assertion in CI

### Note on MVP Scope

H2.6 (WITNESS), H2.7 (quantization), and H2.8 (WASM micro-HNSW) are **NOT in MVP** (per cross-element integration spec Section 4). They are post-Week 8 hardening and optimization features. Prioritize Units 1-3.

---

## File Map

| File | Unit | Action |
|------|------|--------|
| `crates/clawft-core/src/workspace.rs` | 1 | Extend with per-agent methods |
| `crates/clawft-types/src/workspace.rs` | 1 | Update timestamps to `DateTime<Utc>` |
| `crates/clawft-core/src/embeddings/mod.rs` | 2 | Enhance `Embedder` trait, add `name()` |
| `crates/clawft-core/src/embeddings/hash_embedder.rs` | 2 | Fix after A2 (stable hash) |
| `crates/clawft-core/src/embeddings/api_embedder.rs` | 2 | Already functional, wire to HNSW |
| `crates/clawft-core/src/embeddings/hnsw_store.rs` | 2 | NEW: HNSW vector store |
| `crates/clawft-core/src/embeddings/rvf_stub.rs` | 3 | Replace with real RVF I/O |
| `crates/clawft-core/src/embeddings/progressive.rs` | 3 | Update to use HNSW + RVF |
| `crates/clawft-core/Cargo.toml` | 2 | Add `instant-distance` dependency |
| `crates/clawft-core/src/embeddings/witness.rs` | 4 | NEW: WITNESS hash chain |
| `crates/clawft-core/src/embeddings/quantization.rs` | 4 | NEW: temperature tiers |

---

## Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| `instant-distance` crate maturity | Medium | Medium | **4** | Pure Rust, no unsafe; property-based tests; vendoring as fallback |
| Embedding API availability/latency | Medium | Medium | **4** | Async pipeline with pending queue; keyword-only fallback |
| Cross-agent shared memory consistency | Low | High | **4** | Read-only default; write requires opt-in with fs locking |
| RVF 0.2 lacks segment I/O | Medium | Medium | **4** | Week 4 audit; local implementation using `rvf-types` as fallback |
| HNSW memory consumption (100K x 1536 dims ~ 600MB) | Medium | High | **6** | Support 256-dim Matryoshka reduction; `max_vectors` config |
