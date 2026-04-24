# Agent Booster: Technical Architecture

## 🏛️ System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Application                          │
│  (Agentic-flow, CLI, NPM SDK, or Direct API)                    │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                   JavaScript/TypeScript Layer                    │
│                                                                   │
│  ┌──────────────────┐         ┌──────────────────┐             │
│  │  Auto-Detection  │────────▶│  Native Addon    │ (Node.js)   │
│  │     Loader       │         │   (via napi-rs)  │             │
│  └──────────────────┘         └──────────────────┘             │
│           │                            │                         │
│           │                            │                         │
│           ▼                            ▼                         │
│  ┌──────────────────┐         ┌──────────────────┐             │
│  │   WASM Module    │         │   Core Rust API  │             │
│  │ (wasm-bindgen)   │────────▶│                  │             │
│  └──────────────────┘         └──────────────────┘             │
└───────────────────────────────────┬─────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Rust Core Library                           │
│                   (agent-booster crate)                          │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                      Parser Module                          │ │
│  │  ┌──────────────────┐  ┌──────────────────┐               │ │
│  │  │  Tree-sitter     │  │  Language         │               │ │
│  │  │  Integration     │  │  Grammars         │               │ │
│  │  └──────────────────┘  └──────────────────┘               │ │
│  │           │                      │                          │ │
│  │           └──────────┬───────────┘                          │ │
│  │                      ▼                                       │ │
│  │           ┌─────────────────────┐                           │ │
│  │           │   AST Extraction    │                           │ │
│  │           │   & Chunking        │                           │ │
│  │           └─────────────────────┘                           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   Embeddings Module                         │ │
│  │  ┌──────────────────┐  ┌──────────────────┐               │ │
│  │  │  ONNX Runtime    │  │  Model Manager   │               │ │
│  │  │  Inference       │  │  (Download/Cache)│               │ │
│  │  └──────────────────┘  └──────────────────┘               │ │
│  │           │                      │                          │ │
│  │           └──────────┬───────────┘                          │ │
│  │                      ▼                                       │ │
│  │           ┌─────────────────────┐                           │ │
│  │           │   Tokenization      │                           │ │
│  │           │   & Embedding       │                           │ │
│  │           └─────────────────────┘                           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                  Vector Search Module                       │ │
│  │  ┌──────────────────┐  ┌──────────────────┐               │ │
│  │  │   HNSW Index     │  │  Similarity       │               │ │
│  │  │   (in-memory)    │  │  Metrics          │               │ │
│  │  └──────────────────┘  └──────────────────┘               │ │
│  │           │                      │                          │ │
│  │           └──────────┬───────────┘                          │ │
│  │                      ▼                                       │ │
│  │           ┌─────────────────────┐                           │ │
│  │           │  K-NN Search        │                           │ │
│  │           │  & Ranking          │                           │ │
│  │           └─────────────────────┘                           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Merge Module                             │ │
│  │  ┌──────────────────┐  ┌──────────────────┐               │ │
│  │  │  Strategy        │  │  Confidence       │               │ │
│  │  │  Selector        │  │  Scorer           │               │ │
│  │  └──────────────────┘  └──────────────────┘               │ │
│  │           │                      │                          │ │
│  │           └──────────┬───────────┘                          │ │
│  │                      ▼                                       │ │
│  │           ┌─────────────────────┐                           │ │
│  │           │  Code Application   │                           │ │
│  │           │  & Validation       │                           │ │
│  │           └─────────────────────┘                           │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## 📦 Crate Structure

### Core Library (`agent-booster`)

```rust
// Public API
pub struct AgentBooster {
    parser: Parser,
    embedder: Embedder,
    config: Config,
}

impl AgentBooster {
    pub fn new(config: Config) -> Result<Self>;
    pub fn apply_edit(&mut self, request: EditRequest) -> Result<EditResult>;
    pub fn batch_apply(&mut self, requests: Vec<EditRequest>) -> Result<Vec<EditResult>>;
}

// Request/Response types
pub struct EditRequest {
    pub original_code: String,
    pub edit_snippet: String,
    pub language: Language,
    pub options: EditOptions,
}

pub struct EditResult {
    pub merged_code: String,
    pub confidence: f32,
    pub strategy: MergeStrategy,
    pub metadata: EditMetadata,
}

// Configuration
pub struct Config {
    pub model: EmbeddingModel,
    pub confidence_threshold: f32,
    pub max_chunks: usize,
    pub cache_embeddings: bool,
}
```

### Native Addon (`agent-booster-native`)

```rust
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub struct AgentBoosterNative {
    inner: agent_booster::AgentBooster,
}

#[napi]
impl AgentBoosterNative {
    #[napi(constructor)]
    pub fn new(config: Option<JsConfig>) -> Result<Self>;

    #[napi]
    pub async fn apply_edit(&mut self, request: JsEditRequest) -> Result<JsEditResult>;

    #[napi]
    pub async fn batch_apply(&mut self, requests: Vec<JsEditRequest>) -> Result<Vec<JsEditResult>>;
}

// JavaScript-compatible types
#[napi(object)]
pub struct JsConfig {
    pub model: Option<String>,
    pub confidence_threshold: Option<f32>,
}

#[napi(object)]
pub struct JsEditRequest {
    pub original_code: String,
    pub edit_snippet: String,
    pub language: String,
}

#[napi(object)]
pub struct JsEditResult {
    pub merged_code: String,
    pub confidence: f32,
    pub strategy: String,
}
```

### WASM Bindings (`agent-booster-wasm`)

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct AgentBoosterWasm {
    inner: agent_booster::AgentBooster,
}

#[wasm_bindgen]
impl AgentBoosterWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<AgentBoosterWasm, JsValue>;

    #[wasm_bindgen(js_name = applyEdit)]
    pub async fn apply_edit(&mut self, request: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = batchApply)]
    pub async fn batch_apply(&mut self, requests: JsValue) -> Result<JsValue, JsValue>;
}
```

## 🧩 Module Breakdown

### 1. Parser Module (`parser.rs`)

**Responsibilities:**
- Initialize tree-sitter parsers for supported languages
- Parse source code into AST
- Extract semantic chunks (functions, classes, methods, etc.)
- Handle incremental parsing for performance

**Dependencies:**
- `tree-sitter`
- `tree-sitter-{language}` for each supported language

**Key Functions:**
```rust
pub struct Parser {
    parsers: HashMap<Language, TreeSitterParser>,
}

impl Parser {
    pub fn new() -> Self;
    pub fn parse(&mut self, code: &str, language: Language) -> Result<Tree>;
    pub fn extract_chunks(&self, tree: &Tree, code: &str) -> Vec<CodeChunk>;
    pub fn incremental_parse(&mut self, old_tree: &Tree, edits: &[InputEdit]) -> Result<Tree>;
}

pub struct CodeChunk {
    pub code: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub node_type: String,
    pub parent_type: Option<String>,
    pub complexity: u32,
}
```

**Supported Languages (Priority Order):**
1. JavaScript (MVP)
2. TypeScript (MVP)
3. Python
4. Rust
5. Go
6. Java
7. C/C++
8. Ruby
9. PHP
10. C#

### 2. Embeddings Module (`embeddings.rs`)

**Responsibilities:**
- Download and cache embedding models
- Tokenize code into model inputs
- Generate embeddings via ONNX Runtime
- Manage multiple model variants

**Dependencies:**
- `ort` (ONNX Runtime for Rust)
- `tokenizers` (HuggingFace tokenizers)
- `reqwest` (for model downloads)

**Key Functions:**
```rust
pub struct Embedder {
    session: ort::Session,
    tokenizer: Tokenizer,
    model_info: ModelInfo,
}

impl Embedder {
    pub fn new(model: EmbeddingModel) -> Result<Self>;
    pub fn embed(&self, text: &str) -> Result<Vec<f32>>;
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    pub fn dimension(&self) -> usize;
}

pub enum EmbeddingModel {
    JinaCodeV2,        // 768 dim, best accuracy
    AllMiniLML6V2,     // 384 dim, faster
    CodeBERT,          // 768 dim, alternative
    Custom(PathBuf),   // User-provided model
}

pub struct ModelInfo {
    pub name: String,
    pub dimension: usize,
    pub max_tokens: usize,
    pub download_url: String,
}
```

**Model Management:**
```rust
pub struct ModelManager;

impl ModelManager {
    pub fn download_model(model: &EmbeddingModel) -> Result<PathBuf>;
    pub fn get_cached_model(model: &EmbeddingModel) -> Option<PathBuf>;
    pub fn cache_dir() -> PathBuf;  // ~/.cache/agent-booster/models/
}
```

### 3. Vector Search Module (`vector.rs`)

**Responsibilities:**
- Build HNSW index from embeddings
- Perform K-NN similarity search
- Calculate similarity scores (cosine, dot product)
- Rank results by confidence

**Dependencies:**
- `hnsw` or custom HNSW implementation
- `ndarray` for vector operations

**Key Functions:**
```rust
pub struct VectorIndex {
    index: Hnsw<f32>,
    metadata: Vec<ChunkMetadata>,
    dimension: usize,
}

impl VectorIndex {
    pub fn new(dimension: usize) -> Self;
    pub fn insert(&mut self, id: usize, vector: &[f32], metadata: ChunkMetadata);
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult>;
    pub fn clear(&mut self);
}

pub struct SearchResult {
    pub chunk_id: usize,
    pub similarity: f32,
    pub metadata: ChunkMetadata,
}

pub struct ChunkMetadata {
    pub chunk: CodeChunk,
    pub embedding_cache: Option<Vec<f32>>,
}

// Similarity metrics
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32;
pub fn dot_product(a: &[f32], b: &[f32]) -> f32;
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32;
```

### 4. Merge Module (`merge.rs`)

**Responsibilities:**
- Select merge strategy based on confidence
- Apply edits to code
- Validate syntax after merge
- Generate confidence scores

**Dependencies:**
- Tree-sitter (for validation)

**Key Functions:**
```rust
pub struct Merger {
    validator: SyntaxValidator,
}

impl Merger {
    pub fn new() -> Self;
    pub fn merge(&self, request: MergeRequest) -> Result<MergeResult>;
}

pub struct MergeRequest {
    pub original_code: String,
    pub edit_snippet: String,
    pub best_match: SearchResult,
    pub language: Language,
}

pub struct MergeResult {
    pub code: String,
    pub strategy: MergeStrategy,
    pub confidence: f32,
}

pub enum MergeStrategy {
    ExactReplace,      // Replace entire matched chunk
    InsertBefore,      // Insert before matched chunk
    InsertAfter,       // Insert after matched chunk
    Append,            // Append to end of file
    FuzzyReplace,      // GumTree-based fuzzy merge
}

// Strategy selection
pub fn select_strategy(similarity: f32, context: &MergeContext) -> MergeStrategy;

// Confidence calculation
pub fn calculate_confidence(
    similarity: f32,
    strategy: MergeStrategy,
    syntax_valid: bool,
) -> f32;
```

**Merge Strategy Decision Tree:**
```rust
fn select_strategy(similarity: f32, chunk: &CodeChunk) -> MergeStrategy {
    match similarity {
        s if s >= 0.95 => MergeStrategy::ExactReplace,
        s if s >= 0.85 => {
            if chunk.node_type == "function_declaration" {
                MergeStrategy::ExactReplace
            } else {
                MergeStrategy::InsertAfter
            }
        },
        s if s >= 0.65 => MergeStrategy::InsertAfter,
        s if s >= 0.50 => MergeStrategy::FuzzyReplace,
        _ => MergeStrategy::Append,
    }
}
```

## 🚀 Performance Optimizations

### 1. Lazy Initialization
```rust
pub struct AgentBooster {
    parser: OnceCell<Parser>,
    embedder: OnceCell<Embedder>,
    config: Config,
}

impl AgentBooster {
    pub fn new(config: Config) -> Self {
        // Don't initialize parser/embedder until first use
        Self {
            parser: OnceCell::new(),
            embedder: OnceCell::new(),
            config,
        }
    }
}
```

### 2. Embedding Caching
```rust
pub struct EmbeddingCache {
    cache: HashMap<u64, Vec<f32>>,  // hash(code) -> embedding
    max_size: usize,
}

impl EmbeddingCache {
    pub fn get(&self, code: &str) -> Option<&Vec<f32>>;
    pub fn insert(&mut self, code: &str, embedding: Vec<f32>);
    pub fn clear(&mut self);
}
```

### 3. Parallel Processing
```rust
use rayon::prelude::*;

pub fn batch_apply(
    &mut self,
    requests: Vec<EditRequest>
) -> Result<Vec<EditResult>> {
    requests
        .par_iter()
        .map(|req| self.apply_edit(req.clone()))
        .collect()
}
```

### 4. Incremental Parsing
```rust
pub struct IncrementalParser {
    last_tree: Option<Tree>,
    last_code: String,
}

impl IncrementalParser {
    pub fn parse(&mut self, code: &str) -> Result<Tree> {
        if let Some(old_tree) = &self.last_tree {
            let edits = compute_edits(&self.last_code, code);
            self.parser.parse_incremental(code, old_tree, &edits)
        } else {
            self.parser.parse(code)
        }
    }
}
```

## 🧪 Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_exact_match() {
        let booster = AgentBooster::new(Config::default()).unwrap();
        let result = booster.apply_edit(EditRequest {
            original_code: "function foo() { return 1; }",
            edit_snippet: "function foo() { return 2; }",
            language: Language::JavaScript,
            ..Default::default()
        }).unwrap();

        assert_eq!(result.strategy, MergeStrategy::ExactReplace);
        assert!(result.confidence > 0.95);
    }
}
```

### Integration Tests
```rust
#[test]
fn test_real_world_edit() {
    let original = include_str!("../../fixtures/complex.ts");
    let edit = include_str!("../../fixtures/complex.edit.ts");
    let expected = include_str!("../../fixtures/complex.expected.ts");

    let result = apply_edit(original, edit, Language::TypeScript).unwrap();
    assert_eq!(result.code, expected);
}
```

### Benchmark Tests
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_apply_edit(c: &mut Criterion) {
    let booster = AgentBooster::new(Config::default()).unwrap();

    c.bench_function("apply_simple_edit", |b| {
        b.iter(|| {
            booster.apply_edit(black_box(simple_request()))
        })
    });
}

criterion_group!(benches, benchmark_apply_edit);
criterion_main!(benches);
```

## 🔒 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentBoosterError {
    #[error("Failed to parse code: {0}")]
    ParseError(String),

    #[error("Failed to generate embedding: {0}")]
    EmbeddingError(String),

    #[error("No suitable merge location found (confidence: {0:.2})")]
    LowConfidence(f32),

    #[error("Merge resulted in invalid syntax")]
    InvalidSyntax,

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AgentBoosterError>;
```

## 📊 Memory Management

### Memory Budget
```
Total memory target: < 500MB

Breakdown:
- ONNX model:        150-200MB (loaded once)
- Tokenizer:         20-30MB
- Tree-sitter:       10-20MB
- HNSW index:        50-100MB (per file, ephemeral)
- Working memory:    100-150MB
- Overhead:          20-50MB
```

### Cleanup Strategy
```rust
impl Drop for AgentBooster {
    fn drop(&mut self) {
        // Clear caches
        self.embedding_cache.clear();
        // Free ONNX session
        // (handled by ort automatically)
    }
}
```

## 🔄 Data Flow

```
Input: { original_code, edit_snippet, language }
  │
  ▼
┌─────────────────────────────────┐
│  1. Parse Original Code         │
│     - Tree-sitter               │
│     - Extract chunks (5-50)     │
│     - Cache AST                 │
└─────────────────────────────────┘
  │ CodeChunk[]
  ▼
┌─────────────────────────────────┐
│  2. Generate Embeddings         │
│     - Tokenize chunks           │
│     - ONNX inference (batch)    │
│     - Cache embeddings          │
└─────────────────────────────────┘
  │ Vec<Vec<f32>>
  ▼
┌─────────────────────────────────┐
│  3. Build Vector Index          │
│     - HNSW construction         │
│     - In-memory only            │
└─────────────────────────────────┘
  │ VectorIndex
  ▼
┌─────────────────────────────────┐
│  4. Embed Edit Snippet          │
│     - Tokenize                  │
│     - ONNX inference            │
└─────────────────────────────────┘
  │ Vec<f32>
  ▼
┌─────────────────────────────────┐
│  5. Vector Similarity Search    │
│     - K-NN (k=5)               │
│     - Cosine similarity         │
│     - Rank by score             │
└─────────────────────────────────┘
  │ SearchResult[]
  ▼
┌─────────────────────────────────┐
│  6. Select Merge Strategy       │
│     - Analyze confidence        │
│     - Consider context          │
│     - Choose strategy           │
└─────────────────────────────────┘
  │ MergeStrategy
  ▼
┌─────────────────────────────────┐
│  7. Apply Merge                 │
│     - String manipulation       │
│     - Preserve formatting       │
│     - Handle edge cases         │
└─────────────────────────────────┘
  │ String
  ▼
┌─────────────────────────────────┐
│  8. Validate Syntax             │
│     - Parse merged code         │
│     - Check for errors          │
│     - Calculate final confidence│
└─────────────────────────────────┘
  │
  ▼
Output: { merged_code, confidence, strategy, metadata }
```

## 🎯 Future Enhancements

### Phase 2: Advanced Features
- [ ] Multi-file refactoring support
- [ ] Custom model fine-tuning pipeline
- [ ] LSP integration for better context
- [ ] Conflict resolution for concurrent edits

### Phase 3: Enterprise Features
- [ ] Team collaboration features
- [ ] Audit logging
- [ ] Custom security policies
- [ ] On-premise deployment

### Phase 4: AI Enhancements
- [ ] Learn from corrections
- [ ] Adaptive confidence thresholds
- [ ] Context-aware embeddings
- [ ] Cross-language refactoring
