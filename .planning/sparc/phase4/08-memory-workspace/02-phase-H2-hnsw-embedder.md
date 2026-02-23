# SPARC Task: Element 08 Memory & Workspace -- Phase H2 Core (HNSW + Embedder)

| Field | Value |
|-------|-------|
| **Element** | 08 -- Memory & Workspace |
| **Phase** | H2 Core: HNSW-backed VectorStore + Production Embedder |
| **Items** | H2.1 (HNSW VectorStore), H2.2 (Production Embedder), Async Embedding Pipeline |
| **Timeline** | Weeks 5-7 |
| **Priority** | P1 |
| **Crates** | `clawft-core` (primary), `clawft-types` (shared types) |
| **Dependencies** | 03/A2 (stable hash fix -- `DefaultHasher` -> deterministic SipHash), 04/C1 (`MemoryBackend` trait) |
| **Blocks** | 10/K4 (ClawHub vector search), H2.3 (RVF file I/O), H2.6 (WITNESS segments) |
| **Status** | Planning |

---

## 1. Overview

Phase H2 Core replaces the brute-force cosine similarity search with an HNSW-backed vector store and enhances the `Embedder` trait for production use. This phase delivers:

1. **H2.1** -- An `HnswVectorStore` backed by `instant-distance`, providing approximate nearest-neighbor search with <10ms query latency at up to 100K vectors.
2. **H2.2** -- An enhanced `Embedder` trait with `name()` identifier, batch-primary `embed()`, and `dimensions()` rename. Both `HashEmbedder` and `ApiEmbedder` updated to conform.
3. **Async Embedding Pipeline** -- Background embedding via `tokio::spawn` so that `store()` returns immediately and vector search is eventually consistent.

All code lives under `crates/clawft-core/src/embeddings/` and is gated behind the `vector-memory` feature flag (HNSW store) or `rvf` feature flag (API embedder).

---

## 2. Current Code

### 2.1 Embedder Trait (`crates/clawft-core/src/embeddings/mod.rs:46-64`)

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimension(&self) -> usize;
}
```

**Issues**:
- `dimension()` singular -- orchestrator spec calls for `dimensions()`.
- No `name()` method for embedder identification in logs/config.
- Single-text `embed()` is primary; batch is secondary. Should be reversed for efficiency.

### 2.2 HashEmbedder (`crates/clawft-core/src/embeddings/hash_embedder.rs:29-101`)

```rust
pub struct HashEmbedder {
    dimension: usize,
}

impl HashEmbedder {
    pub fn new(dimension: usize) -> Self { Self { dimension } }
    pub fn default_dimension() -> Self { Self::new(384) }

    pub fn compute_embedding(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0f32; self.dimension];
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() { return vector; }
        for word in &words {
            let mut hasher = DefaultHasher::new();   // BUG A2: unstable across Rust versions
            word.to_lowercase().hash(&mut hasher);
            let hash = hasher.finish();
            for (i, val) in vector.iter_mut().enumerate() {
                let mixed = hash ^ (i as u64);
                let bit = (mixed >> (i % 64)) & 1;
                if bit == 1 { *val += 1.0; } else { *val -= 1.0; }
            }
        }
        // normalize to unit length
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 { for val in &mut vector { *val /= norm; } }
        vector
    }
}
```

**BUG A2**: `std::hash::DefaultHasher` is documented as potentially changing between Rust versions. Any persisted embeddings (HNSW index on disk) will become invalid after a Rust toolchain update. This MUST be fixed before H2.1 can persist indices.

### 2.3 ApiEmbedder (`crates/clawft-core/src/embeddings/api_embedder.rs:61-259`)

```rust
pub struct ApiEmbedder {
    config: ApiEmbedderConfig,
    http: reqwest::Client,
    dim: usize,
}
```

Calls OpenAI-compatible `/embeddings` endpoint. Falls back to SHA-256-based pseudo-embedding when no API key is set. Already gated behind `rvf` feature.

### 2.4 ProgressiveSearch (`crates/clawft-core/src/embeddings/progressive.rs:49-132`)

```rust
pub struct ProgressiveSearch {
    entries: Vec<Entry>,
}
```

Brute-force cosine similarity stub with JSON persistence. This is what H2.1 replaces with HNSW.

### 2.5 RvfStore (`crates/clawft-core/src/embeddings/rvf_stub.rs:91-218`)

```rust
pub struct RvfStore {
    entries: Vec<RvfEntry>,
    path: Option<PathBuf>,
}
```

Another brute-force store with RVF-compatible API shape. Also a candidate for HNSW backend integration.

### 2.6 Cargo.toml (`crates/clawft-core/Cargo.toml:13-17`)

```toml
[features]
default = ["full"]
full = []
vector-memory = ["dep:rand"]
rvf = ["vector-memory", "dep:rvf-runtime", "dep:rvf-types", "dep:sha2", "dep:reqwest"]
```

No `instant-distance` dependency exists yet.

---

## 3. Implementation Tasks

### Task H2.1-A: Add `instant-distance` Dependency

**File**: `crates/clawft-core/Cargo.toml`

Add the crate as an optional dependency gated behind `vector-memory`:

```toml
[dependencies]
instant-distance = { version = "0.6", optional = true }

[features]
vector-memory = ["dep:rand", "dep:instant-distance"]
```

**Verification**: `cargo check --features vector-memory` must succeed.

**Note**: Verify `instant-distance` 0.6 compiles on the project's MSRV. If not, pin to the latest compatible version.

---

### Task H2.1-B: Create `HnswVectorStore`

**New file**: `crates/clawft-core/src/embeddings/hnsw_store.rs`

```rust
//! HNSW-backed vector store using `instant-distance`.
//!
//! Provides approximate nearest-neighbor search with sub-10ms query latency
//! for up to 100K vectors. The index is immutable once built; mutations
//! accumulate in a staging area and trigger periodic re-index.

use std::sync::Arc;

use instant_distance::{Builder, HnswMap, Search};

use super::{Embedder, EmbeddingError};

/// A point wrapper for `instant-distance` that holds a vector.
#[derive(Clone, Debug)]
struct Point(Vec<f32>);

impl instant_distance::Point for Point {
    fn distance(&self, other: &Self) -> f32 {
        // instant-distance expects a distance metric (lower = closer).
        // We use 1.0 - cosine_similarity as the distance.
        let dot: f32 = self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0; // maximum distance
        }
        1.0 - (dot / (norm_a * norm_b))
    }
}

/// Configuration for the HNSW vector store.
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Maximum number of vectors before triggering eviction.
    pub max_vectors: usize,
    /// Number of entries that triggers automatic re-index of the HNSW graph.
    pub reindex_threshold: usize,
    /// HNSW construction parameter: number of neighbors per node.
    /// Higher values = better recall, slower build. Default: 24.
    pub ef_construction: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            max_vectors: 100_000,
            reindex_threshold: 100,
            ef_construction: 24,
        }
    }
}

/// HNSW-backed vector store.
///
/// # Design
///
/// `instant-distance` builds immutable HNSW indices. To support incremental
/// inserts, new entries accumulate in a `staging` buffer. Searches query
/// both the HNSW index and the staging buffer (brute-force for staging),
/// then merge results.
///
/// When `staging.len() >= config.reindex_threshold`, call `rebuild_index()`
/// to fold all staging entries into a new HNSW index.
///
/// # Thread Safety
///
/// This store is NOT internally synchronized. Wrap in `tokio::sync::RwLock`
/// for concurrent access.
pub struct HnswVectorStore {
    /// The built HNSW index (None until first rebuild).
    index: Option<HnswMap<Point, String>>,
    /// All entries (key -> vector), used to rebuild the index.
    entries: Vec<(String, Vec<f32>)>,
    /// Entries added since last index build, searched via brute-force.
    staging: Vec<(String, Vec<f32>)>,
    /// The embedder used to convert text to vectors.
    embedder: Arc<dyn Embedder>,
    /// Configuration.
    config: HnswConfig,
}

impl HnswVectorStore {
    /// Create a new empty HNSW vector store.
    pub fn new(embedder: Arc<dyn Embedder>) -> Self {
        Self::with_config(embedder, HnswConfig::default())
    }

    /// Create a new HNSW vector store with custom configuration.
    pub fn with_config(embedder: Arc<dyn Embedder>, config: HnswConfig) -> Self {
        Self {
            index: None,
            entries: Vec::new(),
            staging: Vec::new(),
            embedder,
            config,
        }
    }

    /// Insert a text entry. Embeds the text and adds to the staging buffer.
    ///
    /// Returns the generated embedding vector for the caller's use.
    pub async fn insert(&mut self, key: String, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embedding = self.embedder.embed(text).await?;
        self.insert_vector(key, embedding.clone());
        Ok(embedding)
    }

    /// Insert a pre-computed vector directly (for async pipeline use).
    pub fn insert_vector(&mut self, key: String, vector: Vec<f32>) {
        // Remove existing entry with same key (upsert semantics).
        self.entries.retain(|(k, _)| k != &key);
        self.staging.retain(|(k, _)| k != &key);

        self.entries.push((key.clone(), vector.clone()));
        self.staging.push((key, vector));
    }

    /// Search for the `k` nearest entries to a query vector.
    ///
    /// Searches both the HNSW index (if built) and the staging buffer,
    /// then merges and deduplicates results by key.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() || k == 0 {
            return Vec::new();
        }

        let mut results: Vec<(String, f32)> = Vec::new();

        // Search the HNSW index if it exists.
        if let Some(ref index) = self.index {
            let query_point = Point(query.to_vec());
            let mut search = Search::default();
            for result in index.search(&query_point, &mut search).take(k) {
                let key = result.value.clone();
                let similarity = 1.0 - result.distance;
                results.push((key, similarity));
            }
        }

        // Brute-force search the staging buffer.
        for (key, vector) in &self.staging {
            let similarity = cosine_similarity(query, vector);
            results.push((key.clone(), similarity));
        }

        // Deduplicate by key, keeping the highest similarity score.
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut seen = std::collections::HashSet::new();
        results.retain(|(key, _)| seen.insert(key.clone()));
        results.truncate(k);

        results
    }

    /// Search using a text query (embeds the query first).
    pub async fn search_text(&self, query: &str, k: usize) -> Result<Vec<(String, f32)>, EmbeddingError> {
        let query_vec = self.embedder.embed(query).await?;
        Ok(self.search(&query_vec, k))
    }

    /// Rebuild the HNSW index from all entries.
    ///
    /// This is an O(n log n) operation. Call periodically or when
    /// `needs_reindex()` returns true.
    pub fn rebuild_index(&mut self) {
        if self.entries.is_empty() {
            self.index = None;
            self.staging.clear();
            return;
        }

        let points: Vec<Point> = self.entries.iter().map(|(_, v)| Point(v.clone())).collect();
        let values: Vec<String> = self.entries.iter().map(|(k, _)| k.clone()).collect();

        let hnsw = Builder::default().build(points, values);
        self.index = Some(hnsw);
        self.staging.clear();
    }

    /// Returns true if the staging buffer has reached the reindex threshold.
    pub fn needs_reindex(&self) -> bool {
        self.staging.len() >= self.config.reindex_threshold
    }

    /// Return the total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if the store has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the number of entries in the staging buffer (not yet in HNSW).
    pub fn staging_len(&self) -> usize {
        self.staging.len()
    }

    /// Delete an entry by key. Returns true if an entry was removed.
    ///
    /// Note: The HNSW index is not updated until the next `rebuild_index()`.
    /// Deleted entries are filtered from search results.
    pub fn delete(&mut self, key: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(k, _)| k != key);
        self.staging.retain(|(k, _)| k != key);
        self.entries.len() < before
    }

    /// Get the embedder reference.
    pub fn embedder(&self) -> &Arc<dyn Embedder> {
        &self.embedder
    }
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
```

**Key design decisions**:

1. **Dual-layer search**: HNSW index for committed entries + brute-force for staging. This avoids rebuilding the index on every insert.
2. **Immutable index**: `instant-distance` produces immutable graphs. We rebuild when `staging.len() >= reindex_threshold`.
3. **Distance metric**: `1.0 - cosine_similarity` converts similarity to distance for `instant-distance`.
4. **Deduplication**: When entries exist in both HNSW and staging (after upsert), we keep the highest-scoring match.

---

### Task H2.1-C: Register `hnsw_store` Module

**File**: `crates/clawft-core/src/embeddings/mod.rs`

Add module declaration under the `vector-memory` feature gate:

```rust
#[cfg(feature = "vector-memory")]
pub mod hash_embedder;

#[cfg(feature = "vector-memory")]
pub mod hnsw_store;        // <-- NEW
```

---

### Task H2.2-A: Enhance Embedder Trait

**File**: `crates/clawft-core/src/embeddings/mod.rs`

Update the `Embedder` trait per orchestrator Section 5:

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Return the dimensionality of embeddings produced by this embedder.
    fn dimensions(&self) -> usize;

    /// Generate a vector embedding for a single text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Batch embed multiple texts.
    ///
    /// Default implementation calls `embed()` for each text sequentially.
    /// Implementations SHOULD override this for efficiency (e.g., batched API calls).
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Return the name/identifier of this embedder.
    ///
    /// Used for logging, configuration, and selecting the correct embedder.
    /// Examples: `"hash-simhash-384"`, `"openai-text-embedding-3-small"`.
    fn name(&self) -> &str;
}
```

**Changes from current**:
1. `dimension()` -> `dimensions()` (rename for consistency with orchestrator spec)
2. New `name()` method
3. Keep both single-text `embed()` and batch `embed_batch()` for ergonomics (deviating slightly from orchestrator's batch-only `embed(&[String])` to preserve backward compatibility)

**Migration note**: The `dimension()` -> `dimensions()` rename is a breaking change. All `Embedder` implementors and callers must be updated.

---

### Task H2.2-B: Update HashEmbedder

**File**: `crates/clawft-core/src/embeddings/hash_embedder.rs`

Changes:
1. Implement `dimensions()` (replacing `dimension()`)
2. Implement `name()` returning `"hash-simhash-{dim}"`

```rust
#[async_trait]
impl Embedder for HashEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.compute_embedding(text))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let results: Vec<Vec<f32>> = texts.iter().map(|t| self.compute_embedding(t)).collect();
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimension
    }

    fn name(&self) -> &str {
        // Note: Cannot dynamically format here since we return &str.
        // Use a fixed name; dimension is available via dimensions().
        "hash-simhash"
    }
}
```

**A2 dependency note**: The `DefaultHasher` bug (A2) must be fixed before any HNSW index is persisted to disk. The fix is to replace `DefaultHasher::new()` with a deterministic `SipHasher` using a fixed key:

```rust
// Replace in compute_embedding():
// OLD:
use std::hash::{DefaultHasher, Hash, Hasher};
let mut hasher = DefaultHasher::new();

// NEW (after A2 fix):
use siphasher::SipHasher;
const HASH_KEY_0: u64 = 0x_CLAWFT_SEED_0;
const HASH_KEY_1: u64 = 0x_CLAWFT_SEED_1;
let mut hasher = SipHasher::new_with_keys(HASH_KEY_0, HASH_KEY_1);
```

This fix is tracked in 03/A2 and must land before or alongside H2.1.

---

### Task H2.2-C: Update ApiEmbedder

**File**: `crates/clawft-core/src/embeddings/api_embedder.rs`

Changes:
1. Implement `dimensions()` (replacing `dimension()`)
2. Implement `name()` returning the model name from config

```rust
#[async_trait]
impl Embedder for ApiEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // ... existing implementation unchanged ...
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        // ... existing implementation unchanged ...
    }

    fn dimensions(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &str {
        &self.config.model
    }
}
```

---

### Task H2.2-D: Update Callers of `dimension()`

Search for all callers of `.dimension()` on `Embedder` trait objects and update to `.dimensions()`. Known locations:

- `crates/clawft-core/src/embeddings/hash_embedder.rs` tests (e.g., `embedder.dimension()`)
- Any `MemoryBackend` implementation that reads embedder dimensionality
- The new `HnswVectorStore` if it references `embedder.dimension()`

---

## 4. Async Embedding Pipeline

### 4.1 Architecture

Per orchestrator Section 9, the embedding pipeline decouples data storage from vector indexing:

```
store(key, text)
  |
  +-- [sync] Write raw data to keyword-searchable storage
  |         (immediately available for keyword search)
  |
  +-- [async] tokio::spawn embedding task
                |
                +-- Embedder.embed(text)
                |
                +-- HnswVectorStore.insert_vector(key, embedding)
                |
                +-- Remove from pending_embeddings queue
```

### 4.2 `PendingEmbeddings` Queue

**New file**: `crates/clawft-core/src/embeddings/pipeline.rs`

```rust
//! Async embedding pipeline for eventually-consistent vector search.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

use super::{Embedder, EmbeddingError};
use super::hnsw_store::HnswVectorStore;

/// Status of a pending embedding task.
#[derive(Debug, Clone)]
pub enum EmbeddingStatus {
    /// Queued, waiting for processing.
    Queued,
    /// Currently being embedded.
    InProgress,
    /// Failed with retry count.
    Failed { retries: u32, last_error: String },
}

/// An item in the embedding queue.
#[derive(Debug, Clone)]
struct PendingItem {
    key: String,
    text: String,
    status: EmbeddingStatus,
    retries: u32,
    queued_at: Instant,
}

/// Configuration for the embedding pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum concurrent embedding tasks.
    pub max_concurrent: usize,
    /// Maximum retry attempts for failed embeddings.
    pub max_retries: u32,
    /// Base delay for exponential backoff (doubles each retry).
    pub backoff_base: Duration,
    /// Channel buffer size for the embedding queue.
    pub queue_capacity: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            max_retries: 3,
            backoff_base: Duration::from_millis(500),
            queue_capacity: 1024,
        }
    }
}

/// Message sent to the background embedding worker.
enum PipelineMsg {
    /// Embed and index this item.
    Embed { key: String, text: String },
    /// Shut down the pipeline.
    Shutdown,
}

/// Handle for submitting items to the async embedding pipeline.
///
/// The pipeline processes embeddings in the background. Items are
/// immediately available for keyword search; vector search availability
/// is eventually consistent.
pub struct EmbeddingPipeline {
    tx: mpsc::Sender<PipelineMsg>,
    pending: Arc<RwLock<HashMap<String, EmbeddingStatus>>>,
}

impl EmbeddingPipeline {
    /// Create and start a new embedding pipeline.
    ///
    /// Spawns a background task that processes embedding requests.
    pub fn start(
        embedder: Arc<dyn Embedder>,
        store: Arc<RwLock<HnswVectorStore>>,
        config: PipelineConfig,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<PipelineMsg>(config.queue_capacity);
        let pending: Arc<RwLock<HashMap<String, EmbeddingStatus>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let pending_clone = pending.clone();
        let max_retries = config.max_retries;
        let backoff_base = config.backoff_base;

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    PipelineMsg::Embed { key, text } => {
                        // Mark as in-progress
                        {
                            let mut p = pending_clone.write().await;
                            p.insert(key.clone(), EmbeddingStatus::InProgress);
                        }

                        let mut retries = 0u32;
                        let mut success = false;

                        while retries <= max_retries {
                            match embedder.embed(&text).await {
                                Ok(embedding) => {
                                    let mut s = store.write().await;
                                    s.insert_vector(key.clone(), embedding);
                                    if s.needs_reindex() {
                                        s.rebuild_index();
                                    }
                                    success = true;
                                    break;
                                }
                                Err(e) => {
                                    retries += 1;
                                    if retries > max_retries {
                                        warn!(
                                            key = %key,
                                            error = %e,
                                            retries = retries,
                                            "embedding failed after max retries, item stays keyword-only"
                                        );
                                        let mut p = pending_clone.write().await;
                                        p.insert(key.clone(), EmbeddingStatus::Failed {
                                            retries,
                                            last_error: e.to_string(),
                                        });
                                    } else {
                                        let delay = backoff_base * 2u32.pow(retries - 1);
                                        debug!(
                                            key = %key,
                                            error = %e,
                                            retry = retries,
                                            delay_ms = delay.as_millis(),
                                            "embedding failed, retrying"
                                        );
                                        tokio::time::sleep(delay).await;
                                    }
                                }
                            }
                        }

                        if success {
                            let mut p = pending_clone.write().await;
                            p.remove(&key);
                            debug!(key = %key, "embedding completed successfully");
                        }
                    }
                    PipelineMsg::Shutdown => {
                        debug!("embedding pipeline shutting down");
                        break;
                    }
                }
            }
        });

        Self { tx, pending }
    }

    /// Submit a text for background embedding.
    ///
    /// Returns immediately. The embedding will be processed asynchronously.
    pub async fn submit(&self, key: String, text: String) -> Result<(), EmbeddingError> {
        {
            let mut p = self.pending.write().await;
            p.insert(key.clone(), EmbeddingStatus::Queued);
        }

        self.tx
            .send(PipelineMsg::Embed { key: key.clone(), text })
            .await
            .map_err(|_| {
                EmbeddingError::Internal("embedding pipeline channel closed".into())
            })
    }

    /// Check the status of a pending embedding.
    pub async fn status(&self, key: &str) -> Option<EmbeddingStatus> {
        let p = self.pending.read().await;
        p.get(key).cloned()
    }

    /// Return the number of items currently pending.
    pub async fn pending_count(&self) -> usize {
        let p = self.pending.read().await;
        p.len()
    }

    /// Shut down the pipeline gracefully.
    pub async fn shutdown(&self) {
        let _ = self.tx.send(PipelineMsg::Shutdown).await;
    }
}
```

### 4.3 Integration with `store()` Path

The `MemoryBackend` implementation (from 04/C1) will integrate the pipeline as follows:

```rust
// In MemoryBackend::store() implementation:
pub async fn store(&self, key: &str, value: &str, metadata: serde_json::Value) -> Result<()> {
    // 1. Write raw data immediately (keyword-searchable)
    self.keyword_store.insert(key, value, metadata).await?;

    // 2. Submit for background embedding (eventually consistent)
    self.embedding_pipeline.submit(key.to_string(), value.to_string()).await?;

    Ok(())
}
```

### 4.4 Consistency Model

- **Keyword search**: Immediately consistent. `store()` writes data synchronously.
- **Vector search**: Eventually consistent. New items appear in vector search results only after the background embedding task completes (typically <1s for local embedders, 1-5s for API embedders).
- **Query behavior**: If a vector search returns no results but keyword search does, the UI/caller can indicate that vector indexing is in progress.

---

## 5. Module Registration

### 5.1 `embeddings/mod.rs` Final Structure

After H2.1 + H2.2, the module should export:

```rust
#[cfg(feature = "vector-memory")]
pub mod hash_embedder;

#[cfg(feature = "vector-memory")]
pub mod hnsw_store;

#[cfg(feature = "vector-memory")]
pub mod pipeline;

#[cfg(feature = "rvf")]
pub mod api_embedder;

#[cfg(feature = "rvf")]
pub mod progressive;   // retained for backward compat, deprecated

#[cfg(feature = "rvf")]
pub mod rvf_stub;      // retained for backward compat, deprecated
```

### 5.2 Feature Flag Dependencies

```toml
[features]
vector-memory = ["dep:rand", "dep:instant-distance"]
rvf = ["vector-memory", "dep:rvf-runtime", "dep:rvf-types", "dep:sha2", "dep:reqwest"]
```

The `pipeline` module requires `tokio` (already a workspace dep) but no new optional deps.

---

## 6. Tests Required

### Unit Tests

| Test | Module | Description |
|------|--------|-------------|
| `hnsw_insert_and_search` | `hnsw_store` | Insert 3 entries, search returns correct top-k ordered by similarity |
| `hnsw_search_empty_store` | `hnsw_store` | Search on empty store returns empty vec |
| `hnsw_search_k_zero` | `hnsw_store` | Search with k=0 returns empty vec |
| `hnsw_upsert_semantics` | `hnsw_store` | Insert same key twice, store contains only one entry |
| `hnsw_delete_entry` | `hnsw_store` | Delete removes entry from both entries and staging |
| `hnsw_rebuild_index` | `hnsw_store` | After rebuild, staging is cleared and search still works |
| `hnsw_staging_brute_force` | `hnsw_store` | Before rebuild, entries in staging are found via brute-force |
| `hnsw_needs_reindex` | `hnsw_store` | Returns true when staging exceeds threshold |
| `hnsw_search_dedup` | `hnsw_store` | Entries in both HNSW and staging are not duplicated in results |
| `hnsw_search_text` | `hnsw_store` | `search_text()` embeds query and returns results |
| `embedder_trait_name` | `hash_embedder` | `HashEmbedder.name()` returns `"hash-simhash"` |
| `embedder_trait_dimensions` | `hash_embedder` | `HashEmbedder.dimensions()` returns correct value |
| `api_embedder_name` | `api_embedder` | `ApiEmbedder.name()` returns the model name |
| `api_embedder_dimensions` | `api_embedder` | `ApiEmbedder.dimensions()` returns correct value |
| `pipeline_submit_and_complete` | `pipeline` | Submit item, wait, verify it appears in HNSW store |
| `pipeline_pending_count` | `pipeline` | Pending count increments on submit, decrements on completion |
| `pipeline_status_transitions` | `pipeline` | Status transitions: Queued -> InProgress -> removed (on success) |
| `pipeline_retry_on_failure` | `pipeline` | With a failing embedder, retries up to max_retries |
| `pipeline_shutdown` | `pipeline` | Shutdown stops processing cleanly |

### Integration Tests

| Test | Description |
|------|-------------|
| `hnsw_100_entries_search_quality` | Insert 100 entries with HashEmbedder, verify top-5 recall against brute-force |
| `hnsw_rebuild_preserves_results` | Insert entries, rebuild, verify search results are identical to pre-rebuild |
| `pipeline_end_to_end` | Submit 10 items via pipeline, wait for completion, verify all 10 are searchable |
| `pipeline_api_fallback` | With no API key, ApiEmbedder falls back to hash; pipeline still indexes items |

### Property-Based Tests (if `proptest` available)

| Test | Description |
|------|-------------|
| `hnsw_search_top1_is_exact_match` | For any inserted vector, searching with the same vector returns that entry as top-1 |
| `hnsw_cosine_distance_symmetry` | `distance(a, b) == distance(b, a)` for all points |

---

## 7. Acceptance Criteria

- [ ] `instant-distance` added to `Cargo.toml` under `vector-memory` feature
- [ ] `HnswVectorStore` compiles and passes all unit tests
- [ ] `HnswVectorStore.search()` returns correct top-k results for 100+ entries
- [ ] `HnswVectorStore.rebuild_index()` produces a valid index from staging
- [ ] `Embedder` trait has `dimensions()` and `name()` methods
- [ ] `HashEmbedder` implements updated `Embedder` trait
- [ ] `ApiEmbedder` implements updated `Embedder` trait
- [ ] All callers of `.dimension()` updated to `.dimensions()`
- [ ] `EmbeddingPipeline` processes items asynchronously via `tokio::spawn`
- [ ] Pipeline retries failed embeddings with exponential backoff (max 3 retries)
- [ ] Pipeline `pending_count()` accurately tracks in-flight items
- [ ] `store()` returns immediately; vector search is eventually consistent
- [ ] All existing tests in `hash_embedder`, `api_embedder`, `progressive`, `rvf_stub` still pass
- [ ] `cargo check --features vector-memory` succeeds
- [ ] `cargo check --features rvf` succeeds
- [ ] `cargo test --features vector-memory` passes all new and existing tests
- [ ] `cargo test --features rvf` passes all new and existing tests
- [ ] No new `unsafe` code introduced
- [ ] HNSW search latency <10ms for 10K vectors (benchmark)

---

## 8. Risk Notes

### R1: `instant-distance` Immutable Index Overhead

**Risk**: Rebuilding the index is O(n log n). For 100K vectors at 384 dimensions, a rebuild takes ~200ms. This is a latency spike during reindex.

**Mitigation**:
- Set `reindex_threshold` conservatively (e.g., 100 entries) so rebuilds happen frequently but cheaply.
- Execute `rebuild_index()` in the background pipeline, not on the hot `search()` path.
- The staging buffer provides brute-force search for <100 unindexed entries with negligible latency.

### R2: Memory Consumption at Scale

**Risk**: 100K vectors x 1536 dims x 4 bytes = ~600MB. With HNSW graph overhead, total memory could reach 800MB+.

**Mitigation**:
- Default to 384-dimension embeddings (HashEmbedder and ApiEmbedder default).
- `HnswConfig.max_vectors` enforces an upper bound with eviction.
- Phase H2.7 (temperature-based quantization) will reduce storage for cold vectors.
- Support Matryoshka dimensionality reduction (256-dim truncation) as a config option.

### R3: BUG A2 Blocks Persisted Indices

**Risk**: If `DefaultHasher` output changes across Rust versions, any HNSW index persisted with `HashEmbedder` vectors becomes invalid after a toolchain update.

**Mitigation**:
- **Hard dependency**: 03/A2 (stable hash fix) MUST land before or alongside H2.1.
- Do NOT implement HNSW persistence (H2.3) until A2 is resolved.
- In-memory HNSW is safe since the index is rebuilt on startup anyway.

### R4: Async Pipeline Ordering

**Risk**: Background embedding tasks may complete out of order. If item A is submitted before item B, B may appear in vector search before A.

**Mitigation**:
- This is acceptable for the eventually-consistent model.
- The `pending_embeddings` queue provides visibility into what is still being processed.
- Callers that need strict ordering should await `pipeline.status(key)` before querying.

### R5: `instant-distance` API Stability

**Risk**: The `instant-distance` crate is a smaller project with limited maintainers. Breaking API changes could require migration effort.

**Mitigation**:
- Pin to `0.6.x` in `Cargo.toml`.
- The `HnswVectorStore` wraps `instant-distance` behind our own API, isolating the rest of the codebase.
- If the crate is abandoned, the wrapper makes it straightforward to swap in an alternative (e.g., `hnsw_rs` or a vendored copy).

### R6: Embedding API Availability

**Risk**: If the embedding API (OpenAI, etc.) is down or slow, the pipeline queue grows unboundedly.

**Mitigation**:
- `PipelineConfig.queue_capacity` bounds the channel size (default 1024).
- Exponential backoff with max 3 retries prevents infinite loops.
- After max retries, the item is marked `Failed` and stays keyword-only.
- Monitoring via `pipeline.pending_count()` and `pipeline.status()`.

---

## 9. File Summary

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-core/Cargo.toml` | EDIT | Add `instant-distance` optional dep, update `vector-memory` feature |
| `crates/clawft-core/src/embeddings/mod.rs` | EDIT | Add `hnsw_store` and `pipeline` modules; update `Embedder` trait |
| `crates/clawft-core/src/embeddings/hnsw_store.rs` | NEW | HNSW-backed vector store |
| `crates/clawft-core/src/embeddings/pipeline.rs` | NEW | Async embedding pipeline |
| `crates/clawft-core/src/embeddings/hash_embedder.rs` | EDIT | Implement `dimensions()` and `name()` on `HashEmbedder` |
| `crates/clawft-core/src/embeddings/api_embedder.rs` | EDIT | Implement `dimensions()` and `name()` on `ApiEmbedder` |

---

## 10. Dependency Graph

```
03/A2 (stable hash) ──┐
                       ├──> H2.1 (HNSW VectorStore) ──┐
04/C1 (MemoryBackend) ┘                               ├──> H2.3 (RVF I/O)
                                                       ├──> H2.6 (WITNESS)
H2.2 (Embedder trait) ──> H2.1 (HNSW uses Embedder)   ├──> 10/K4 (ClawHub)
                                                       │
H2.1 + H2.2 ──> Async Pipeline ──> MemoryBackend integration
```
