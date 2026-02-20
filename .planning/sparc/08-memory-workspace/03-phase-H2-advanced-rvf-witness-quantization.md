# SPARC Task: Element 08 -- Phase H2 Advanced (RVF I/O, WITNESS, Quantization)

| Field | Value |
|-------|-------|
| **Element** | 08 -- Memory & Workspace |
| **Phase** | H2 Advanced (H2.3-H2.8) |
| **Timeline** | Week 6-8 |
| **Priority** | H2.3-H2.5 = P1 (MVP), H2.6-H2.8 = P2 (Post-MVP) |
| **Crates** | `clawft-core`, `clawft-cli`, `clawft-types`, `micro-hnsw-wasm` (new, H2.8) |
| **Dependencies** | H2.1 (HNSW store), H2.2 (Embedder trait), RVF 0.2 audit, 04/C1 (MemoryBackend trait) |
| **Blocks** | 10/K4 (ClawHub vector search), export/import for agent migration |
| **Status** | Planning |
| **Branch** | `sprint/phase-5` (stream 5G) |

---

## 1. Overview

This document covers the advanced memory subsystem features for Element 08 Phase H2, items H2.3 through H2.8. These items complete the RVF-based persistence layer, add export/import CLI commands, persist routing policies, and introduce three post-MVP hardening features: tamper-evident WITNESS audit trails, temperature-based vector quantization, and a standalone WASM micro-HNSW module.

**MVP scope** (P1, Week 6-7): H2.3, H2.4, H2.5 -- these must ship before any post-MVP work begins.

**Post-MVP scope** (P2, Week 7-8): H2.6, H2.7, H2.8 -- hardening and optimization features that depend on the MVP foundation.

---

## 2. Current State

### H2.3 -- RVF Segment I/O
- `rvf_stub.rs` exists at `crates/clawft-core/src/embeddings/rvf_stub.rs` with an `RvfStore` that uses brute-force cosine similarity and JSON persistence
- `ProgressiveSearch` at `crates/clawft-core/src/embeddings/progressive.rs` saves as plain JSON via `serde_json::to_string`
- `rvf-runtime` and `rvf-types` are optional workspace dependencies gated behind the `rvf` feature flag
- `memory_bootstrap.rs` uses `RvfStore` for MEMORY.md indexing -- already JSON-based
- No RVF segment format is actually used; all persistence is plain JSON

### H2.4 -- Export/Import CLI
- CLI has `weft memory show`, `weft memory history`, `weft memory search` commands
- No `export` or `import` subcommands exist
- CLI commands are in `crates/clawft-cli/src/commands/`

### H2.5 -- POLICY_KERNEL Persistence
- `IntelligentRouter` at `crates/clawft-core/src/intelligent_router.rs` maintains `policy_store: VectorStore` and `cost_records: Vec<CostRecord>` entirely in memory
- No persistence path; all policies and cost records are lost on restart
- Router uses `Box<dyn Embedder>` for policy matching

### H2.6 -- WITNESS Segments
- Not implemented
- `sha2` crate is already a workspace dependency (`sha2 = "0.10"` in root `Cargo.toml`)
- No hash chain or audit trail infrastructure exists

### H2.7 -- Temperature Quantization
- Not implemented
- All vectors stored as `Vec<f32>` with no tiered storage
- No access frequency tracking exists

### H2.8 -- WASM micro-HNSW
- Not implemented
- No `micro-hnsw-wasm` crate exists
- Main `clawft-wasm` crate has a 300KB budget; micro-HNSW must be separate with 8KB budget

---

## 3. RVF 0.2 Audit Plan (Week 4 Pre-Work)

Before H2.3 implementation begins, the `rvf-runtime` 0.2 public API must be audited. This is a **blocking prerequisite** for H2.3.

### Audit Checklist

| Check | Question | Expected Outcome |
|-------|----------|-----------------|
| Segment read API | Does `rvf-runtime` 0.2 expose a public method to read individual segments from an RVF file? | Method signature and return type documented |
| Segment write API | Does `rvf-runtime` 0.2 expose a public method to write/append segments to an RVF file? | Method signature and input type documented |
| Vector data support | Can segment payloads contain `Vec<f32>` vector data (or equivalent byte arrays)? | Data format confirmed |
| Metadata attachment | Can arbitrary JSON metadata be attached to segments? | Metadata schema confirmed |
| Timestamp fields | Do segments include timestamp fields compatible with `DateTime<Utc>`? | Timestamp format confirmed |
| WITNESS metadata | Can custom hash chain metadata (H2.6) be attached to segments? | Extension mechanism confirmed |
| Batch operations | Does the API support batch read/write for multiple segments? | Performance characteristics documented |

### Fallback Plan

If `rvf-runtime` 0.2 only provides stubs or lacks segment I/O:

1. Implement local serialization using `rvf-types` directly
2. Define a `SegmentFormat` struct that mirrors expected RVF segment structure
3. Serialize using `bincode` or `serde_json` to `.rvf` files
4. Design the implementation so swapping to real `rvf-runtime` I/O is a single-module replacement

### Audit Output

Document findings in `.planning/development_notes/08-memory-workspace/rvf-0.2-audit.md` with:
- API surface inventory (public types and methods)
- Capability matrix (what works, what is stubbed)
- Decision: use `rvf-runtime` or local implementation
- Migration path if starting local

---

## 4. Implementation Tasks

### 4.1 H2.3: RVF Segment I/O (Week 6-7, P1)

**Objective**: Replace plain JSON persistence in `rvf_stub.rs` and `progressive.rs` with RVF segment format.

#### File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-core/src/embeddings/rvf_stub.rs` | Replace | Swap JSON persistence for RVF segment I/O |
| `crates/clawft-core/src/embeddings/progressive.rs` | Modify | Update `save()` and `load()` to use RVF segments |
| `crates/clawft-core/src/memory_bootstrap.rs` | Modify | Update `RvfStore` usage to RVF segment format |
| `crates/clawft-core/Cargo.toml` | Modify | Ensure `rvf-runtime`/`rvf-types` deps are correct |

#### RVF Segment Structure

Each memory entry becomes an RVF segment stored at `~/.clawft/agents/<agentId>/memory/*.rvf`:

```rust
/// A memory entry serialized as an RVF segment.
pub struct MemorySegment {
    /// Segment header
    pub segment_id: Uuid,
    pub segment_type: SegmentType,  // VectorEntry, Metadata, Index
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    /// Payload
    pub entry_id: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,

    /// Optional WITNESS metadata (H2.6, post-MVP)
    pub witness_hash: Option<[u8; 32]>,
}

pub enum SegmentType {
    VectorEntry,
    Metadata,
    IndexManifest,
    WitnessChain,
}
```

#### File Layout

```
~/.clawft/agents/<agentId>/memory/
  vectors.rvf          # All vector entry segments
  metadata.rvf         # Metadata-only segments (tags, namespaces)
  index.rvf            # Index manifest (entry count, dimensions, config)
  witness.rvf          # WITNESS chain (H2.6, post-MVP)
```

#### Implementation Steps

1. Define `MemorySegment` struct with serde support
2. Implement `RvfSegmentWriter` -- writes segments to `.rvf` files
3. Implement `RvfSegmentReader` -- reads segments from `.rvf` files
4. Update `RvfStore::compact()` to write RVF segments instead of JSON
5. Update `RvfStore::open()` to read RVF segments
6. Update `ProgressiveSearch::save()` and `load()` to delegate to RVF I/O
7. Update `bootstrap_memory_index()` to use new persistence format
8. Add migration path: detect old JSON format, convert to RVF on first load

#### Migration Strategy

```rust
/// Detect and migrate legacy JSON stores to RVF format.
pub fn migrate_legacy_store(path: &Path) -> Result<(), RvfError> {
    // Check if file is JSON (legacy) or RVF (new)
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"{") || bytes.starts_with(b"[") {
        // Legacy JSON format -- parse and re-save as RVF
        let legacy: StoreState = serde_json::from_slice(&bytes)?;
        let mut writer = RvfSegmentWriter::new(path)?;
        for entry in legacy.entries {
            writer.write_segment(MemorySegment::from(entry))?;
        }
        writer.finalize()?;
    }
    Ok(())
}
```

---

### 4.2 H2.4: Export/Import CLI (Week 6-7, P1)

**Objective**: Add `weft memory export` and `weft memory import` CLI commands.

#### New CLI Commands

```
weft memory export --agent <id> --output <path> [--format rvf|json] [--include-witness]
weft memory import --agent <id> --input <path> [--validate-witness] [--merge|--replace]
```

#### File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-cli/src/commands/memory.rs` | Modify | Add `export` and `import` subcommands |
| `crates/clawft-core/src/embeddings/export.rs` | New | Export/import logic |

#### Export Behavior

1. Resolve agent workspace: `~/.clawft/agents/<agentId>/memory/`
2. Read all memory segments from the workspace
3. Write to output path as a single portable RVF file
4. Include WITNESS chain if `--include-witness` and H2.6 is available
5. Include manifest with: agent ID, entry count, dimensions, export timestamp, clawft version

```rust
/// Manifest header for exported memory files.
#[derive(Serialize, Deserialize)]
pub struct ExportManifest {
    pub version: String,           // "1.0"
    pub agent_id: String,
    pub entry_count: usize,
    pub dimensions: usize,
    pub exported_at: DateTime<Utc>,
    pub clawft_version: String,
    pub includes_witness: bool,
}
```

#### Import Behavior

1. Validate the input file format and manifest
2. If `--validate-witness`, verify WITNESS chain integrity (H2.6)
3. Resolve target agent workspace
4. `--merge` (default): add entries that don't exist, skip duplicates by ID
5. `--replace`: clear existing memory, load all from import file
6. Rebuild HNSW index after import (triggers re-index from H2.1)

#### Error Handling

| Error Condition | Behavior |
|----------------|----------|
| Agent workspace does not exist | Create it via `ensure_agent_workspace()` |
| Input file is malformed | Return error with details, import nothing |
| WITNESS chain validation fails | Return error, import nothing (unless `--skip-witness-validation`) |
| Dimension mismatch | Return error with expected vs actual dimensions |
| Partial failure during merge | Rollback all changes (atomic import) |

---

### 4.3 H2.5: POLICY_KERNEL Persistence (Week 6-7, P1)

**Objective**: Persist `IntelligentRouter` routing policies and cost records across restarts.

#### File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-core/src/intelligent_router.rs` | Modify | Add load/save methods |
| `crates/clawft-core/src/policy_persistence.rs` | New | POLICY_KERNEL persistence logic |

#### Storage Location

```
~/.clawft/memory/policy_kernel.json
```

#### Persisted State

```rust
/// Serializable policy kernel state.
#[derive(Serialize, Deserialize)]
pub struct PolicyKernelState {
    /// Schema version for forward compatibility.
    pub version: u32,  // 1
    /// Cached routing policies.
    pub policies: Vec<PolicyEntry>,
    /// Cost/latency history (last N records, configurable).
    pub cost_records: Vec<CostRecordEntry>,
    /// Last save timestamp.
    pub saved_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct PolicyEntry {
    pub id: String,
    pub pattern: String,
    pub embedding: Vec<f32>,
    pub tier: u8,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CostRecordEntry {
    pub model: String,
    pub tokens: u64,
    pub cost: f32,
    pub latency_ms: u64,
    pub timestamp: u64,
}
```

#### Implementation Steps

1. Add `PolicyKernelState` struct with serde support
2. Implement `save_policy_kernel(path: &Path, router: &IntelligentRouter)`
3. Implement `load_policy_kernel(path: &Path) -> PolicyKernelState`
4. Modify `IntelligentRouter::new()` to accept optional path and load on startup
5. Modify `update_policy()` to trigger save after changes
6. Modify `record_cost()` to trigger save after recording (debounced, not every call)
7. Add config for max cost records to retain (default: 1000)

#### Debounced Persistence

To avoid excessive disk I/O, policy saves are debounced:
- Save triggers after `update_policy()` succeeds
- Save triggers after every 10th `record_cost()` call, or on shutdown
- Use a dirty flag to avoid unnecessary writes

```rust
impl IntelligentRouter {
    /// Save policy kernel state to disk if dirty.
    pub fn save_if_dirty(&self) -> Result<(), PolicyPersistError> {
        if !self.dirty {
            return Ok(());
        }
        // ... serialize and write
    }
}
```

---

### 4.4 H2.6: WITNESS Segments (Week 7-8, P2 Post-MVP)

**Objective**: Implement a tamper-evident audit trail using SHA-256 hash chaining over all memory write operations.

#### File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-core/src/embeddings/witness.rs` | New | WITNESS chain implementation |
| `crates/clawft-core/src/embeddings/mod.rs` | Modify | Add `pub mod witness;` |
| `crates/clawft-core/src/embeddings/rvf_stub.rs` | Modify | Integrate WITNESS into write paths |
| `crates/clawft-core/src/embeddings/export.rs` | Modify | Include/validate WITNESS in export/import |

#### WITNESS Segment Structure

Per orchestrator Section 8:

```rust
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use uuid::Uuid;

/// A single entry in the WITNESS hash chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessSegment {
    /// Unique segment identifier.
    pub segment_id: Uuid,
    /// When this segment was created.
    pub timestamp: DateTime<Utc>,
    /// The operation that produced this segment.
    pub operation: WitnessOperation,
    /// SHA-256 hash of the data that was stored/updated/deleted.
    pub data_hash: [u8; 32],
    /// SHA-256 hash of the previous WITNESS segment (chain link).
    pub previous_hash: [u8; 32],
}

/// The type of memory operation being witnessed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WitnessOperation {
    Store,
    Update,
    Delete,
}
```

#### Hash Chain Design

```
  Root Segment           Segment 1              Segment 2
+-----------------+  +-----------------+   +-----------------+
| segment_id: A   |  | segment_id: B   |   | segment_id: C   |
| op: Store       |  | op: Update      |   | op: Delete      |
| data_hash: H(d0)|  | data_hash: H(d1)|   | data_hash: H(d2)|
| prev_hash: 0x00 |  | prev_hash: H(A) |   | prev_hash: H(B) |
+-----------------+  +-----------------+   +-----------------+
        |                    |                      |
        +----chain link------+------chain link------+
```

- Chain starts with `previous_hash = [0u8; 32]` (zero hash)
- Each segment's hash is computed as: `SHA-256(segment_id || timestamp || operation || data_hash || previous_hash)`
- Verification: sequential scan from root, recompute each hash and verify it matches `previous_hash` of the next segment

#### WITNESS Chain Manager

```rust
/// Manages the WITNESS hash chain for a memory store.
pub struct WitnessChain {
    /// All segments in chain order.
    segments: Vec<WitnessSegment>,
    /// Hash of the most recent segment (tail of chain).
    head_hash: [u8; 32],
    /// File path for persistence.
    path: Option<PathBuf>,
}

impl WitnessChain {
    /// Create a new empty chain.
    pub fn new(path: Option<&Path>) -> Self { ... }

    /// Load an existing chain from disk.
    pub fn load(path: &Path) -> Result<Self, WitnessError> { ... }

    /// Append a new witness segment for a memory operation.
    pub fn witness(
        &mut self,
        operation: WitnessOperation,
        data: &[u8],
    ) -> WitnessSegment { ... }

    /// Verify the entire chain from root to head.
    /// Returns Ok(segment_count) or Err with the index of the first invalid segment.
    pub fn verify(&self) -> Result<usize, WitnessVerifyError> { ... }

    /// Persist the chain to disk.
    pub fn save(&self) -> Result<(), WitnessError> { ... }

    /// Return the number of segments in the chain.
    pub fn len(&self) -> usize { ... }

    /// Return the hash of the most recent segment.
    pub fn head_hash(&self) -> &[u8; 32] { ... }
}
```

#### Hash Computation

```rust
/// Compute the canonical hash of a WITNESS segment.
fn compute_segment_hash(segment: &WitnessSegment) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(segment.segment_id.as_bytes());
    hasher.update(segment.timestamp.to_rfc3339().as_bytes());
    hasher.update(&[segment.operation as u8]);
    hasher.update(&segment.data_hash);
    hasher.update(&segment.previous_hash);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Compute SHA-256 hash of arbitrary data.
fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
```

#### Verification Errors

```rust
#[derive(Debug)]
pub enum WitnessVerifyError {
    /// Chain is empty (no segments to verify).
    EmptyChain,
    /// Root segment has non-zero previous_hash.
    InvalidRoot,
    /// Segment at index N has a mismatched previous_hash.
    ChainBreak {
        index: usize,
        expected_hash: [u8; 32],
        actual_hash: [u8; 32],
    },
}
```

#### Integration with Memory Write Paths

All memory write operations in H2.1-H2.5 optionally produce WITNESS segments:

```rust
impl RvfStore {
    /// Ingest with optional WITNESS recording.
    pub fn ingest_witnessed(
        &mut self,
        id: String,
        embedding: Vec<f32>,
        metadata: serde_json::Value,
        witness_chain: Option<&mut WitnessChain>,
    ) {
        // Serialize data for hashing
        let data = serde_json::to_vec(&(&id, &embedding, &metadata)).unwrap_or_default();

        // Record in WITNESS chain
        if let Some(chain) = witness_chain {
            chain.witness(WitnessOperation::Store, &data);
        }

        // Proceed with normal ingest
        self.ingest(id, embedding, metadata);
    }
}
```

#### Export/Import WITNESS Integration

- `weft memory export --include-witness` serializes the full WITNESS chain as a trailer after all vector segments
- `weft memory import --validate-witness` deserializes the WITNESS chain and calls `verify()` before importing any data
- If verification fails, import is rejected with details about which segment broke the chain

---

### 4.5 H2.7: Temperature-Based Quantization (Week 7-8, P2 Post-MVP)

**Objective**: Implement tiered vector storage to reduce disk and memory usage for infrequently accessed vectors.

#### File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-core/src/embeddings/quantization.rs` | New | Temperature tier logic |
| `crates/clawft-core/src/embeddings/mod.rs` | Modify | Add `pub mod quantization;` |
| `crates/clawft-core/Cargo.toml` | Modify | Add `half` crate for fp16 support |

#### Temperature Tiers

Per orchestrator Section 6:

| Tier | Name | Storage Format | Memory Footprint | Access Pattern | Decompression |
|------|------|---------------|-----------------|----------------|---------------|
| Hot | Full precision | `Vec<f32>` in memory + disk | Full (4 bytes/dim) | Frequently accessed | None |
| Warm | Half precision | fp16 on disk | 0 (disk only, cached on access) | Moderate access | fp16 -> f32 |
| Cold | Product-quantized | PQ on disk | 0 (disk only, cached on access) | Rarely accessed | PQ -> f32 |

#### Key Design Constraint

The HNSW index **always** uses full-precision `Vec<f32>` pointers. Quantization applies **only** to the storage layer. No index rebuild is required when vectors move between tiers.

#### Access Tracking

```rust
/// Tracks access patterns for temperature tier decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTracker {
    /// Number of times this entry has been accessed.
    pub access_count: u64,
    /// Timestamp of last access.
    pub last_accessed: DateTime<Utc>,
    /// Current temperature tier.
    pub tier: TemperatureTier,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemperatureTier {
    Hot,
    Warm,
    Cold,
}
```

#### Tier Transition Rules

```rust
/// Configuration for temperature tier transitions.
pub struct TierConfig {
    /// Entries with access_count >= hot_threshold stay hot.
    pub hot_threshold: u64,           // default: 10
    /// Entries accessed within warm_window stay warm (at minimum).
    pub warm_window: Duration,        // default: 7 days
    /// Entries not accessed within cold_window become cold.
    pub cold_window: Duration,        // default: 30 days
    /// Maximum number of hot entries (LRU eviction to warm).
    pub max_hot_entries: usize,       // default: 10_000
}

impl TierConfig {
    pub fn classify(&self, tracker: &AccessTracker, now: DateTime<Utc>) -> TemperatureTier {
        let age = now - tracker.last_accessed;

        if tracker.access_count >= self.hot_threshold && age < self.warm_window {
            TemperatureTier::Hot
        } else if age < self.cold_window {
            TemperatureTier::Warm
        } else {
            TemperatureTier::Cold
        }
    }
}
```

#### fp16 Compression (Warm Tier)

```rust
use half::f16;

/// Compress a full-precision vector to fp16 for warm storage.
pub fn compress_to_fp16(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|&v| f16::from_f32(v).to_le_bytes())
        .collect()
}

/// Decompress fp16 bytes back to full-precision vector.
pub fn decompress_from_fp16(bytes: &[u8], dimensions: usize) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .take(dimensions)
        .map(|chunk| {
            let val = f16::from_le_bytes([chunk[0], chunk[1]]);
            val.to_f32()
        })
        .collect()
}
```

#### Product Quantization (Cold Tier)

Product quantization (PQ) splits vectors into sub-vectors and encodes each with a codebook:

```rust
/// Product quantizer for cold-tier vector compression.
pub struct ProductQuantizer {
    /// Number of sub-vector segments.
    pub num_segments: usize,  // default: 8
    /// Bits per sub-vector code.
    pub bits_per_code: usize, // default: 8 (256 centroids per segment)
    /// Codebooks: [segment][centroid][dimension]
    pub codebooks: Vec<Vec<Vec<f32>>>,
}

impl ProductQuantizer {
    /// Train codebooks from a set of vectors (k-means per segment).
    pub fn train(vectors: &[Vec<f32>], num_segments: usize, bits_per_code: usize) -> Self { ... }

    /// Encode a full-precision vector to PQ codes.
    pub fn encode(&self, vector: &[f32]) -> Vec<u8> { ... }

    /// Decode PQ codes back to approximate full-precision vector.
    pub fn decode(&self, codes: &[u8]) -> Vec<f32> { ... }
}
```

#### Storage Layout

```
~/.clawft/agents/<agentId>/memory/
  vectors.rvf            # Hot tier: full-precision segments
  vectors-warm.rvf       # Warm tier: fp16 compressed segments
  vectors-cold.rvf       # Cold tier: PQ compressed segments
  access_tracker.json    # Access frequency/recency data
  pq_codebooks.bin       # Product quantizer codebooks (trained)
```

#### Tier Transition Process

1. Background task runs periodically (configurable, default: every hour)
2. For each entry, compute new tier based on `AccessTracker` and `TierConfig`
3. If tier changed:
   a. Read vector from current tier storage
   b. Compress/decompress as needed
   c. Write to new tier storage
   d. Remove from old tier storage
   e. Update `AccessTracker`
4. HNSW index is NOT modified (pointers remain valid)

---

### 4.6 H2.8: WASM micro-HNSW (Week 7-8, P2 Post-MVP)

**Objective**: Build a standalone WASM module for vector search with an 8KB size budget, separate from the main `clawft-wasm` crate.

#### File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/micro-hnsw-wasm/Cargo.toml` | New | Standalone WASM crate |
| `crates/micro-hnsw-wasm/src/lib.rs` | New | Minimal HNSW for WASM |
| `.cargo/config.toml` | Modify | Add WASM build target config |

#### Crate Structure

```
crates/micro-hnsw-wasm/
  Cargo.toml
  src/
    lib.rs          # Public API (insert, search, serialize)
    hnsw.rs         # Minimal HNSW graph implementation
    distance.rs     # Cosine similarity (no_std compatible)
```

#### Size Budget Constraints

The 8KB budget requires aggressive optimization:

| Strategy | Savings |
|----------|---------|
| `#![no_std]` with `alloc` only | Eliminates std overhead |
| No serde dependency | Use manual binary serialization |
| No `uuid` dependency | Use u32 IDs |
| Fixed-dimension vectors only | Configurable at compile time via const generic |
| No float formatting | Binary-only I/O |
| `wasm-opt -Oz` post-processing | 30-50% size reduction |
| `lto = true`, `opt-level = "z"` | Maximum size optimization |

#### Cargo.toml

```toml
[package]
name = "micro-hnsw-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# No dependencies -- everything is hand-rolled for size budget

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

#### Public API (WASM Exports)

```rust
#![no_std]
extern crate alloc;
use alloc::vec::Vec;

/// Create a new HNSW index with the given dimensionality.
#[no_mangle]
pub extern "C" fn hnsw_create(dimensions: u32, max_elements: u32) -> *mut HnswIndex { ... }

/// Insert a vector into the index. Returns the assigned ID.
#[no_mangle]
pub extern "C" fn hnsw_insert(
    index: *mut HnswIndex,
    vector_ptr: *const f32,
    vector_len: u32,
) -> u32 { ... }

/// Search for k nearest neighbors. Results written to output buffer.
#[no_mangle]
pub extern "C" fn hnsw_search(
    index: *const HnswIndex,
    query_ptr: *const f32,
    query_len: u32,
    k: u32,
    results_ptr: *mut SearchResult,
) -> u32 { ... }

/// Serialize the index to a byte buffer for persistence.
#[no_mangle]
pub extern "C" fn hnsw_serialize(
    index: *const HnswIndex,
    buf_ptr: *mut u8,
    buf_len: u32,
) -> u32 { ... }

/// Deserialize an index from a byte buffer.
#[no_mangle]
pub extern "C" fn hnsw_deserialize(
    buf_ptr: *const u8,
    buf_len: u32,
) -> *mut HnswIndex { ... }

/// Free an index.
#[no_mangle]
pub extern "C" fn hnsw_free(index: *mut HnswIndex) { ... }
```

#### Communication with Main WASM Agent

The micro-HNSW module communicates with the main `clawft-wasm` agent via **message passing**, not direct function calls:

```
Main WASM Agent (clawft-wasm, 300KB)
        |
        | postMessage({ type: "search", query: [...], k: 5 })
        v
  micro-HNSW Worker (micro-hnsw-wasm, 8KB)
        |
        | postMessage({ type: "results", matches: [...] })
        v
Main WASM Agent
```

This keeps vector search **optional** -- the main agent functions without micro-HNSW loaded.

#### CI Size Assertion

```yaml
# In CI pipeline
- name: Check micro-HNSW WASM size
  run: |
    wasm-pack build crates/micro-hnsw-wasm --release
    SIZE=$(stat -f%z crates/micro-hnsw-wasm/pkg/micro_hnsw_wasm_bg.wasm 2>/dev/null || stat --printf="%s" crates/micro-hnsw-wasm/pkg/micro_hnsw_wasm_bg.wasm)
    if [ "$SIZE" -gt 8192 ]; then
      echo "FAIL: micro-HNSW WASM is ${SIZE} bytes, exceeds 8KB budget"
      exit 1
    fi
```

---

## 5. MVP vs Post-MVP Scope

Per cross-element integration spec Section 4:

### MVP (P1, Week 6-7) -- MUST ship

| Item | Description | Depends On |
|------|-------------|------------|
| H2.3 | RVF segment I/O replacing JSON persistence | RVF 0.2 audit, H2.1 |
| H2.4 | `weft memory export/import` CLI commands | H2.3 |
| H2.5 | POLICY_KERNEL persistence for IntelligentRouter | None (standalone) |

### Post-MVP (P2, Week 7-8) -- Hardening and optimization

| Item | Description | Depends On |
|------|-------------|------------|
| H2.6 | WITNESS hash chain audit trail | H2.3, sha2 crate |
| H2.7 | Temperature-based vector quantization | H2.1 (HNSW store), half crate |
| H2.8 | WASM micro-HNSW standalone module | H2.1 (HNSW algorithm) |

### Dependency Graph

```
RVF 0.2 Audit ──> H2.3 (RVF I/O) ──> H2.4 (Export/Import) ──> H2.6 (WITNESS export)
                       |
                       v
                  H2.5 (POLICY_KERNEL) [independent]

H2.1 (HNSW) ──> H2.7 (Quantization)
             ──> H2.8 (WASM micro-HNSW)
```

---

## 6. Tests Required

### Unit Tests

| Test | Item | Priority | Description |
|------|------|----------|-------------|
| `rvf_segment_roundtrip` | H2.3 | P1 | Write RVF segment, read back, verify all fields match |
| `rvf_migration_from_json` | H2.3 | P1 | Load legacy JSON store, verify auto-migration to RVF |
| `rvf_segment_invalid_format` | H2.3 | P1 | Attempt to load corrupted RVF file, verify error |
| `rvf_multiple_segments` | H2.3 | P1 | Write 100 segments, read back, verify ordering and data |
| `export_produces_valid_rvf` | H2.4 | P1 | Export agent memory, verify output file is valid RVF |
| `import_loads_exported_data` | H2.4 | P1 | Export then import, verify all entries restored |
| `import_merge_mode` | H2.4 | P1 | Import with `--merge`, verify existing entries preserved |
| `import_replace_mode` | H2.4 | P1 | Import with `--replace`, verify old entries removed |
| `import_rejects_malformed` | H2.4 | P1 | Import corrupt file, verify error and no data change |
| `import_dimension_mismatch` | H2.4 | P1 | Import file with wrong dimensions, verify rejection |
| `policy_kernel_save_load` | H2.5 | P1 | Save policies, restart, verify policies restored |
| `policy_kernel_cost_records` | H2.5 | P1 | Record costs, save, reload, verify stats match |
| `policy_kernel_debounce` | H2.5 | P1 | Verify save not triggered on every record_cost() |
| `policy_kernel_empty_load` | H2.5 | P1 | Load from non-existent path, verify empty state |
| `witness_chain_append` | H2.6 | P2 | Append 3 segments, verify chain links |
| `witness_chain_verify_valid` | H2.6 | P2 | Build chain, verify returns Ok(count) |
| `witness_chain_detect_tamper` | H2.6 | P2 | Build chain, tamper with one segment, verify detects break |
| `witness_chain_empty` | H2.6 | P2 | Verify empty chain returns EmptyChain error |
| `witness_chain_invalid_root` | H2.6 | P2 | Set non-zero previous_hash on root, verify InvalidRoot |
| `witness_chain_persistence` | H2.6 | P2 | Save chain, reload, verify integrity |
| `witness_export_import` | H2.6 | P2 | Export with WITNESS, import with validation, verify |
| `fp16_roundtrip` | H2.7 | P2 | Compress f32 to fp16, decompress, verify < 0.01 error |
| `pq_encode_decode` | H2.7 | P2 | Train PQ, encode vector, decode, verify approximate match |
| `tier_transition_hot_to_warm` | H2.7 | P2 | Access entry, wait, verify transition and data integrity |
| `tier_transition_search_works` | H2.7 | P2 | Move entry to cold, verify search still returns it |
| `wasm_hnsw_insert_search` | H2.8 | P2 | Insert vectors, search, verify correct nearest neighbor |
| `wasm_hnsw_serialize_roundtrip` | H2.8 | P2 | Serialize index, deserialize, verify search works |
| `wasm_module_size_budget` | H2.8 | P2 | Compile WASM, assert size < 8192 bytes |

### Integration Tests

| Test | Items | Priority | Description |
|------|-------|----------|-------------|
| `memory_full_lifecycle` | H2.3-H2.5 | P1 | Create agent, store memories, export, import to new agent, verify |
| `policy_survives_restart` | H2.5 | P1 | Route request, save, simulate restart, route same request, verify cached |
| `witness_full_pipeline` | H2.3+H2.6 | P2 | Store/update/delete with WITNESS, export, import with validation |
| `quantization_transparent` | H2.1+H2.7 | P2 | Store vectors, trigger tier transitions, verify search results unchanged |

---

## 7. Acceptance Criteria

### H2.3 RVF Segment I/O (P1)
- [ ] `RvfStore` persists data in RVF segment format (not plain JSON)
- [ ] File location follows convention: `~/.clawft/agents/<agentId>/memory/*.rvf`
- [ ] Legacy JSON stores auto-migrate to RVF on first load
- [ ] `ProgressiveSearch::save()` and `load()` use RVF format
- [ ] `memory_bootstrap.rs` creates RVF-format stores
- [ ] Roundtrip test passes: write segments, read back, all fields match

### H2.4 Export/Import CLI (P1)
- [ ] `weft memory export --agent <id> --output <path>` produces valid RVF file
- [ ] `weft memory import --agent <id> --input <path>` loads and restores data
- [ ] Export includes manifest with agent ID, entry count, dimensions, timestamp
- [ ] Import with `--merge` preserves existing entries
- [ ] Import with `--replace` clears existing entries
- [ ] Import rejects malformed files with descriptive error
- [ ] Import rejects dimension mismatches

### H2.5 POLICY_KERNEL Persistence (P1)
- [ ] Routing policies persisted at `~/.clawft/memory/policy_kernel.json`
- [ ] Policies loaded on `IntelligentRouter` startup
- [ ] Policies saved after `update_policy()` succeeds
- [ ] Cost records saved with debouncing (not every call)
- [ ] Router functions correctly with no existing policy file (fresh start)
- [ ] Persisted policies survive simulated restart

### H2.6 WITNESS Segments (P2)
- [ ] `WitnessSegment` struct uses SHA-256 for `data_hash` and `previous_hash`
- [ ] Chain starts with `previous_hash = [0u8; 32]`
- [ ] `WitnessChain::verify()` detects tampering at any segment
- [ ] All memory write paths optionally produce WITNESS segments
- [ ] `weft memory export --include-witness` includes full chain
- [ ] `weft memory import --validate-witness` verifies chain before importing
- [ ] Chain persists to disk and reloads correctly

### H2.7 Temperature Quantization (P2)
- [ ] Hot vectors: `Vec<f32>` full precision in memory and on disk
- [ ] Warm vectors: fp16 on disk, decompressed to f32 on access
- [ ] Cold vectors: product-quantized on disk, decompressed on access
- [ ] HNSW index NOT rebuilt on tier transition
- [ ] Search results are the same regardless of vector tier
- [ ] Tier transitions driven by access frequency and recency
- [ ] Background task handles tier transitions without blocking search

### H2.8 WASM micro-HNSW (P2)
- [ ] Separate crate: `crates/micro-hnsw-wasm/`
- [ ] NOT bundled in main `clawft-wasm` crate
- [ ] Compiled WASM size < 8KB
- [ ] Supports insert and k-NN search via C ABI exports
- [ ] Communicates with main WASM agent via message passing
- [ ] `no_std` compatible with `alloc` only
- [ ] CI assertion on WASM module size

---

## 8. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| RVF 0.2 lacks segment I/O, forcing local implementation | Medium | Medium | **4** | Week 4 audit determines path; local `rvf-types` implementation as fallback. Design for swappability. |
| Migration from JSON to RVF corrupts existing data | Low | High | **4** | Backup before migration. Atomic migration (write new, rename on success). JSON detection by file header. |
| WITNESS chain grows unbounded over time | Medium | Medium | **4** | Configurable chain length limit with archival of old segments. Periodic chain checkpointing (compress N segments into a single checkpoint). |
| fp16 quantization introduces unacceptable precision loss | Low | Medium | **3** | fp16 error is bounded (< 0.1% for normalized embeddings). Verify with roundtrip tests. Keep HNSW index at full precision. |
| Product quantization codebook training is slow for large datasets | Medium | Low | **3** | Train on a sample (max 10K vectors). Codebook retraining is infrequent (only on cold tier population changes). |
| 8KB WASM budget is too tight for useful HNSW | High | Medium | **6** | Start with brute-force cosine in WASM, add HNSW layers only if budget allows. Fixed dimensions reduce code size. Benchmark with `wasm-opt -Oz`. |
| POLICY_KERNEL file corruption on crash during write | Low | Medium | **3** | Atomic write: write to temp file, `fsync`, rename. JSON format is human-recoverable. |
| Export/import dimension mismatch between different embedder configs | Medium | Medium | **4** | Manifest includes dimensions. Import validates before loading. Clear error message with expected vs actual. |
| Cross-platform path differences for `.rvf` file locations | Low | Low | **2** | Use `dirs` crate or `std::env::home_dir()` for home directory. All paths use `PathBuf`. |

---

## 9. Open Questions

1. **RVF 0.2 audit timeline**: Is the audit already scheduled for Week 4, or does it need to be added to the sprint?
2. **WITNESS chain length limit**: What is the maximum acceptable chain length before archival/checkpointing? Suggested: 10,000 segments with periodic checkpoints.
3. **PQ codebook training data**: Should codebooks be trained on the agent's own vectors or on a shared global set? Agent-specific is more accurate but requires per-agent training.
4. **Export format versioning**: Should we use a version header in RVF exports to handle future format changes? Suggested: yes, `ExportManifest.version = "1.0"`.
5. **WASM micro-HNSW dimensions**: Should the WASM module use const-generic dimensions (e.g., `HnswIndex<64>`) for size optimization, or runtime-configurable dimensions for flexibility?

---

## 10. Cross-Element Dependencies

| Dependency | Element | Item | Direction | Notes |
|-----------|---------|------|-----------|-------|
| MemoryBackend trait | 04/C1 | Plugin system | Incoming | H2.3 implements trait for RVF storage |
| Stable hash (A2) | 03/A2 | Type safety | Incoming | Must be fixed before persisted embeddings |
| HNSW store | 08/H2.1 | Vector memory | Internal | H2.3, H2.7, H2.8 all depend on HNSW availability |
| Embedder trait | 08/H2.2 | Embeddings | Internal | H2.3 needs embedder for re-indexing on import |
| ClawHub vector search | 10/K4 | Community | Outgoing | ClawHub requires vector search from H2 |
| sha2 crate | Workspace | Build | Available | Already in workspace deps (`sha2 = "0.10"`) |
| Agent workspace | 08/H1 | Workspace | Internal | H2.3 file paths use per-agent workspace dirs |
