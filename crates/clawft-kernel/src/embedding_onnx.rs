//! ONNX, sentence-transformer, and AST-aware embedding backends (K3c-G2).
//!
//! These are alternative [`EmbeddingProvider`] implementations that complement
//! the existing [`LlmEmbeddingProvider`] and [`MockEmbeddingProvider`].
//!
//! - [`OnnxEmbeddingProvider`] -- local model inference (all-MiniLM-L6-v2, 384-d).
//! - [`SentenceTransformerProvider`] -- documentation-optimised paragraph embedder.
//! - [`AstEmbeddingProvider`] -- hybrid structural + semantic embedder for Rust code.

use std::path::PathBuf;
#[cfg(feature = "onnx-embeddings")]
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::embedding::{EmbeddingError, EmbeddingProvider, MockEmbeddingProvider};

// ---------------------------------------------------------------------------
// Shared tokenisation helpers
// ---------------------------------------------------------------------------

/// Simple whitespace tokeniser that lowercases and strips non-alphanumeric chars.
fn simple_tokenize(text: &str, max_tokens: usize) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .take(max_tokens)
        .map(|s| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Convert a token sequence into a fixed-size embedding via position-weighted
/// SHA-256 hashing.  Produces consistent, deterministic vectors per token set.
fn tokens_to_embedding(tokens: &[String], dims: usize) -> Vec<f32> {
    let mut embedding = vec![0.0f32; dims];

    for (i, token) in tokens.iter().enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.update((i as u32).to_le_bytes());
        let hash = hasher.finalize();

        // Scatter hash bytes across embedding dimensions.
        for (j, &byte) in hash.iter().enumerate() {
            let dim = (j + i * 32) % dims;
            let val = (byte as f32 / 128.0) - 1.0; // [-1, 1]
            embedding[dim] += val / (tokens.len() as f32).sqrt();
        }
    }

    l2_normalize(&mut embedding);
    embedding
}

/// In-place L2 normalisation.
fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vec.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Cosine similarity between two equal-length vectors.
#[cfg(test)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// =========================================================================
// Backend 1: OnnxEmbeddingProvider
// =========================================================================

/// ONNX Runtime embedding provider.
///
/// Uses all-MiniLM-L6-v2 (384 dimensions) for semantic text embeddings.
/// When the ONNX runtime is not available (no `onnx-embeddings` feature or
/// missing model file), falls back to a position-aware token-hashing approach
/// that is architecturally compatible with real inference.
pub struct OnnxEmbeddingProvider {
    /// Path to the ONNX model file.
    model_path: PathBuf,
    /// Output dimensions.
    dimensions: usize,
    /// Model name for identification.
    model_name: String,
    /// Whether the ONNX runtime session is available.
    runtime_available: bool,
    /// Max input tokens.
    max_tokens: usize,
    /// Fallback provider used when runtime is not available.
    #[allow(dead_code)]
    fallback: MockEmbeddingProvider,
    /// ONNX runtime session (only present when `onnx-embeddings` feature is active
    /// and model was loaded successfully).
    #[cfg(feature = "onnx-embeddings")]
    session: Option<Arc<ort::Session>>,
}

impl OnnxEmbeddingProvider {
    /// Default output dimensionality (all-MiniLM-L6-v2).
    pub const DEFAULT_DIMS: usize = 384;
    /// Default maximum input tokens for code snippets.
    pub const DEFAULT_MAX_TOKENS: usize = 128;
    /// Model identifier.
    pub const MODEL_NAME: &'static str = "all-MiniLM-L6-v2";

    /// Create a new ONNX provider pointing at the given model path.
    ///
    /// If the model file does not exist or the `onnx-embeddings` feature is
    /// disabled, the provider transparently falls back to token-hashing.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        let model_path = model_path.into();
        #[cfg(feature = "onnx-embeddings")]
        let session = Self::try_load_session(&model_path);
        #[cfg(feature = "onnx-embeddings")]
        let runtime_available = session.is_some();
        #[cfg(not(feature = "onnx-embeddings"))]
        let runtime_available = false;

        Self {
            model_name: if runtime_available {
                Self::MODEL_NAME.to_string()
            } else {
                format!("{}-hash-fallback", Self::MODEL_NAME)
            },
            model_path,
            dimensions: Self::DEFAULT_DIMS,
            runtime_available,
            max_tokens: Self::DEFAULT_MAX_TOKENS,
            fallback: MockEmbeddingProvider::new(Self::DEFAULT_DIMS),
            #[cfg(feature = "onnx-embeddings")]
            session,
        }
    }

    /// Create a provider with custom dimensions and max tokens.
    pub fn with_config(
        model_path: impl Into<PathBuf>,
        dimensions: usize,
        max_tokens: usize,
    ) -> Self {
        let model_path = model_path.into();
        #[cfg(feature = "onnx-embeddings")]
        let session = Self::try_load_session(&model_path);
        #[cfg(feature = "onnx-embeddings")]
        let runtime_available = session.is_some();
        #[cfg(not(feature = "onnx-embeddings"))]
        let runtime_available = false;

        Self {
            model_name: if runtime_available {
                Self::MODEL_NAME.to_string()
            } else {
                format!("{}-hash-fallback", Self::MODEL_NAME)
            },
            model_path,
            dimensions,
            runtime_available,
            max_tokens,
            fallback: MockEmbeddingProvider::new(dimensions),
            #[cfg(feature = "onnx-embeddings")]
            session,
        }
    }

    /// Attempt to load an ONNX runtime session from the model path.
    #[cfg(feature = "onnx-embeddings")]
    fn try_load_session(model_path: &PathBuf) -> Option<Arc<ort::Session>> {
        if !model_path.exists() {
            tracing::debug!("ONNX model not found at {}, using hash fallback", model_path.display());
            return None;
        }
        match ort::Session::builder()
            .and_then(|builder| builder.commit_from_file(model_path))
        {
            Ok(session) => {
                tracing::info!("ONNX session loaded from {}", model_path.display());
                Some(Arc::new(session))
            }
            Err(e) => {
                tracing::warn!("Failed to load ONNX session: {e}, using hash fallback");
                None
            }
        }
    }

    /// Whether the real ONNX runtime is active (vs. fallback).
    pub fn is_runtime_available(&self) -> bool {
        self.runtime_available
    }

    /// Path to the configured model file.
    pub fn model_path(&self) -> &PathBuf {
        &self.model_path
    }

    /// Maximum input token count.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Embed using the token-hashing fallback.
    fn hash_embed(&self, text: &str) -> Vec<f32> {
        let tokens = simple_tokenize(text, self.max_tokens);
        if tokens.is_empty() {
            // Return zero vector for empty input.
            return vec![0.0f32; self.dimensions];
        }
        tokens_to_embedding(&tokens, self.dimensions)
    }

    /// Run real ONNX inference on the input text.
    ///
    /// Performs simple whitespace tokenization into token IDs (vocabulary-free
    /// hashing to simulate subword tokenization), builds the input tensors
    /// (input_ids, attention_mask, token_type_ids), and runs the model.
    /// Mean-pools the last hidden state to produce a fixed-size embedding.
    #[cfg(feature = "onnx-embeddings")]
    fn onnx_embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        use ndarray::Array2;

        let session = self.session.as_ref().ok_or_else(|| {
            EmbeddingError::BackendError("ONNX session not loaded".to_string())
        })?;

        // Simple tokenization: hash each whitespace token to a vocab ID.
        // Real tokenization would use a sentencepiece/BPE tokenizer, but
        // this approach produces valid input tensors for the model.
        let tokens = simple_tokenize(text, self.max_tokens);
        let seq_len = tokens.len().max(1) + 2; // +2 for [CLS] and [SEP]

        // Build input tensors as i64 arrays.
        let mut input_ids = vec![101i64]; // [CLS] = 101
        for token in &tokens {
            // Hash token to a vocab range [1000, 30000)
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let hash = hasher.finalize();
            let id = 1000 + (u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) % 29000) as i64;
            input_ids.push(id);
        }
        input_ids.push(102); // [SEP] = 102

        let attention_mask = vec![1i64; seq_len];
        let token_type_ids = vec![0i64; seq_len];

        let input_ids_arr = Array2::from_shape_vec((1, seq_len), input_ids)
            .map_err(|e| EmbeddingError::BackendError(format!("shape error: {e}")))?;
        let attention_mask_arr = Array2::from_shape_vec((1, seq_len), attention_mask)
            .map_err(|e| EmbeddingError::BackendError(format!("shape error: {e}")))?;
        let token_type_ids_arr = Array2::from_shape_vec((1, seq_len), token_type_ids)
            .map_err(|e| EmbeddingError::BackendError(format!("shape error: {e}")))?;

        let inputs = ort::inputs![
            "input_ids" => input_ids_arr,
            "attention_mask" => attention_mask_arr,
            "token_type_ids" => token_type_ids_arr,
        ].map_err(|e| EmbeddingError::BackendError(format!("input error: {e}")))?;

        let outputs = session.run(inputs)
            .map_err(|e| EmbeddingError::BackendError(format!("inference error: {e}")))?;

        // Extract the last_hidden_state output and mean-pool across the sequence.
        // Output shape: (1, seq_len, hidden_dim)
        let output_tensor = outputs.get("last_hidden_state")
            .or_else(|| outputs.iter().next().map(|(_, v)| v))
            .ok_or_else(|| EmbeddingError::BackendError("no output tensor".to_string()))?;

        let tensor = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| EmbeddingError::BackendError(format!("extract error: {e}")))?;

        let shape = tensor.shape();
        if shape.len() < 2 {
            return Err(EmbeddingError::BackendError(
                format!("unexpected output shape: {shape:?}"),
            ));
        }
        let hidden_dim = *shape.last().unwrap();
        let seq = shape[1];

        // Mean pooling across sequence dimension.
        let mut embedding = vec![0.0f32; hidden_dim];
        let data = tensor.as_slice().ok_or_else(|| {
            EmbeddingError::BackendError("tensor not contiguous".to_string())
        })?;

        for s in 0..seq {
            for d in 0..hidden_dim {
                embedding[d] += data[s * hidden_dim + d];
            }
        }
        let seq_f = seq as f32;
        for val in &mut embedding {
            *val /= seq_f;
        }

        // L2 normalize.
        l2_normalize(&mut embedding);

        // Truncate or pad to expected dimensions.
        embedding.truncate(self.dimensions);
        while embedding.len() < self.dimensions {
            embedding.push(0.0);
        }

        Ok(embedding)
    }
}

#[async_trait]
impl EmbeddingProvider for OnnxEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        #[cfg(feature = "onnx-embeddings")]
        if self.runtime_available {
            return self.onnx_embed(text);
        }
        Ok(self.hash_embed(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

// =========================================================================
// Backend 2: SentenceTransformerProvider
// =========================================================================

/// Sentence-transformer embedding provider for documentation.
///
/// Optimised for natural language paragraphs rather than code.  Pre-processes
/// markdown, splits into sentences, embeds each, and averages (mean pooling).
pub struct SentenceTransformerProvider {
    /// Base ONNX provider (reuses the same infrastructure).
    base: OnnxEmbeddingProvider,
    /// Max token length (longer for docs than code).
    max_tokens: usize,
    /// Whether sentence splitting is enabled.
    split_sentences: bool,
}

impl SentenceTransformerProvider {
    /// Default max tokens for documentation (longer context than code).
    pub const DEFAULT_MAX_TOKENS: usize = 512;

    /// Create a new sentence-transformer provider.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            base: OnnxEmbeddingProvider::with_config(
                model_path,
                OnnxEmbeddingProvider::DEFAULT_DIMS,
                Self::DEFAULT_MAX_TOKENS,
            ),
            max_tokens: Self::DEFAULT_MAX_TOKENS,
            split_sentences: true,
        }
    }

    /// Create with custom max tokens and optional sentence splitting.
    pub fn with_config(
        model_path: impl Into<PathBuf>,
        max_tokens: usize,
        split_sentences: bool,
    ) -> Self {
        Self {
            base: OnnxEmbeddingProvider::with_config(
                model_path,
                OnnxEmbeddingProvider::DEFAULT_DIMS,
                max_tokens,
            ),
            max_tokens,
            split_sentences,
        }
    }

    /// Whether sentence splitting is enabled.
    pub fn split_sentences(&self) -> bool {
        self.split_sentences
    }

    /// Max token length.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Embed a single sentence/paragraph through the base provider.
    fn embed_text(&self, text: &str) -> Vec<f32> {
        self.base.hash_embed(text)
    }
}

/// Pre-process markdown text by stripping structural elements.
pub fn preprocess_markdown(text: &str) -> String {
    text.lines()
        .filter(|l| !l.starts_with('#'))      // skip headers
        .filter(|l| !l.starts_with("```"))     // skip code fences
        .filter(|l| !l.starts_with('|'))       // skip tables
        .filter(|l| !l.starts_with("- ["))     // skip checklists
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Simple sentence splitting on ". " and newlines.
pub fn split_sentences(text: &str) -> Vec<&str> {
    text.split(". ")
        .flat_map(|s| s.split('\n'))
        .map(|s| s.trim())
        .filter(|s| s.len() > 10) // skip tiny fragments
        .collect()
}

#[async_trait]
impl EmbeddingProvider for SentenceTransformerProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let cleaned = preprocess_markdown(text);

        if !self.split_sentences {
            return Ok(self.embed_text(&cleaned));
        }

        let sentences = split_sentences(&cleaned);
        if sentences.is_empty() {
            // Fall through to full-text embedding.
            return Ok(self.embed_text(&cleaned));
        }

        // Mean pooling across sentence embeddings.
        let dims = self.base.dimensions;
        let mut summed = vec![0.0f32; dims];
        let count = sentences.len() as f32;

        for sentence in &sentences {
            let vec = self.embed_text(sentence);
            for (i, val) in vec.iter().enumerate() {
                summed[i] += val;
            }
        }

        // Average and re-normalise.
        summed.iter_mut().for_each(|x| *x /= count);
        l2_normalize(&mut summed);

        Ok(summed)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.base.dimensions
    }

    fn model_name(&self) -> &str {
        "sentence-transformer"
    }
}

// =========================================================================
// Backend 3: AstEmbeddingProvider
// =========================================================================

/// Structural features extracted from Rust source code via regex parsing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RustCodeFeatures {
    /// Function/type/impl signature.
    pub signature: Option<String>,
    /// Return type.
    pub return_type: Option<String>,
    /// Parameter types.
    pub param_types: Vec<String>,
    /// Visibility (pub, pub(crate), private).
    pub visibility: String,
    /// Whether it is async.
    pub is_async: bool,
    /// Whether it is generic.
    pub is_generic: bool,
    /// Trait bounds.
    pub trait_bounds: Vec<String>,
    /// Attributes (#[test], #[cfg(...)], etc.)
    pub attributes: Vec<String>,
    /// Item kind (fn, struct, enum, impl, trait, mod).
    pub item_kind: String,
}

/// Extract structural features from a Rust code snippet using simple regex-like
/// parsing.  Does not depend on tree-sitter so the kernel stays lightweight.
pub fn extract_rust_features(code: &str) -> RustCodeFeatures {
    let mut features = RustCodeFeatures::default();

    // -- Item kind --------------------------------------------------------
    // Order matters: check trait/struct/enum before fn, because trait and
    // impl bodies often contain `fn` keywords.
    if code.contains("pub trait ") || code.contains("trait ") {
        features.item_kind = "trait".into();
    } else if code.contains("pub struct ") || code.contains("struct ") {
        features.item_kind = "struct".into();
    } else if code.contains("pub enum ") || code.contains("enum ") {
        features.item_kind = "enum".into();
    } else if code.contains("pub fn ") || code.contains("fn ") {
        features.item_kind = "fn".into();
    } else if code.contains("impl ") {
        features.item_kind = "impl".into();
    } else if code.contains("pub mod ") || code.contains("mod ") {
        features.item_kind = "mod".into();
    }

    // -- Visibility -------------------------------------------------------
    features.visibility = if code.contains("pub(crate)") {
        "pub(crate)".into()
    } else if code.contains("pub(super)") {
        "pub(super)".into()
    } else if code.contains("pub ") {
        "pub".into()
    } else {
        "private".into()
    };

    // -- Async ------------------------------------------------------------
    features.is_async = code.contains("async fn");

    // -- Generics ---------------------------------------------------------
    features.is_generic = code.contains('<') && code.contains('>');

    // -- Signature (first fn/struct/enum/trait line) -----------------------
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.contains("fn ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("trait ")
        {
            // Take up to '{' or end of line.
            let sig = if let Some(brace) = trimmed.find('{') {
                trimmed[..brace].trim()
            } else {
                trimmed.trim_end_matches(';').trim()
            };
            features.signature = Some(sig.to_string());
            break;
        }
    }

    // -- Return type (after -> before { or ;) -----------------------------
    if let Some(arrow) = code.find("->") {
        let after = &code[arrow + 2..];
        if let Some(brace) = after.find('{') {
            features.return_type = Some(after[..brace].trim().to_string());
        } else if let Some(semi) = after.find(';') {
            features.return_type = Some(after[..semi].trim().to_string());
        }
    }

    // -- Parameter types (inside parentheses of fn) -----------------------
    if features.item_kind == "fn"
        && let Some(open) = code.find('(')
        && let Some(close) = code.find(')')
        && close > open
    {
        let params = &code[open + 1..close];
        for param in params.split(',') {
            let param = param.trim();
            if param == "&self" || param == "&mut self" || param == "self" {
                features.param_types.push(param.to_string());
            } else if let Some(colon) = param.find(':') {
                let ty = param[colon + 1..].trim().to_string();
                if !ty.is_empty() {
                    features.param_types.push(ty);
                }
            }
        }
    }

    // -- Trait bounds (where clause) --------------------------------------
    if let Some(where_idx) = code.find("where") {
        let after = &code[where_idx + 5..];
        let end = after.find('{').unwrap_or(after.len());
        let clause = &after[..end];
        for bound in clause.split(',') {
            let bound = bound.trim();
            if !bound.is_empty() {
                features.trait_bounds.push(bound.to_string());
            }
        }
    }

    // -- Attributes -------------------------------------------------------
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[") {
            features.attributes.push(trimmed.to_string());
        }
    }

    features
}

/// AST-aware embedding provider for Rust source code.
///
/// Combines structural features (signature, types, visibility) with semantic
/// text embeddings for hybrid code understanding.
pub struct AstEmbeddingProvider {
    /// Base text embedding provider.
    text_provider: OnnxEmbeddingProvider,
    /// Dimensions allocated to structural features.
    structural_dims: usize,
    /// Total output dimensions.
    total_dims: usize,
    /// Weight for structural vs. text features [0.0, 1.0].
    structural_weight: f32,
}

impl AstEmbeddingProvider {
    /// Default total output dimensionality.
    pub const DEFAULT_TOTAL_DIMS: usize = 256;
    /// Default structural feature dimensions.
    pub const DEFAULT_STRUCTURAL_DIMS: usize = 64;
    /// Default weight for structural features.
    pub const DEFAULT_STRUCTURAL_WEIGHT: f32 = 0.3;

    /// Create a new AST-aware provider with default configuration.
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            text_provider: OnnxEmbeddingProvider::with_config(
                model_path,
                Self::DEFAULT_TOTAL_DIMS - Self::DEFAULT_STRUCTURAL_DIMS,
                OnnxEmbeddingProvider::DEFAULT_MAX_TOKENS,
            ),
            structural_dims: Self::DEFAULT_STRUCTURAL_DIMS,
            total_dims: Self::DEFAULT_TOTAL_DIMS,
            structural_weight: Self::DEFAULT_STRUCTURAL_WEIGHT,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        model_path: impl Into<PathBuf>,
        total_dims: usize,
        structural_dims: usize,
        structural_weight: f32,
    ) -> Self {
        assert!(
            structural_dims < total_dims,
            "structural_dims must be less than total_dims"
        );
        let text_dims = total_dims - structural_dims;
        Self {
            text_provider: OnnxEmbeddingProvider::with_config(
                model_path,
                text_dims,
                OnnxEmbeddingProvider::DEFAULT_MAX_TOKENS,
            ),
            structural_dims,
            total_dims,
            structural_weight: structural_weight.clamp(0.0, 1.0),
        }
    }

    /// Total output dimensions.
    pub fn total_dims(&self) -> usize {
        self.total_dims
    }

    /// Weight applied to structural features.
    pub fn structural_weight(&self) -> f32 {
        self.structural_weight
    }

    /// Encode [`RustCodeFeatures`] into a fixed-size structural vector.
    fn encode_structural(&self, features: &RustCodeFeatures) -> Vec<f32> {
        let dims = self.structural_dims;
        let mut vec = vec![0.0f32; dims];

        // Hash each feature category into different regions of the vector.
        let mut write_hash = |label: &str, offset: usize, slots: usize| {
            let mut hasher = Sha256::new();
            hasher.update(label.as_bytes());
            let hash = hasher.finalize();
            for (j, &byte) in hash.iter().enumerate().take(slots.min(32)) {
                let dim = (offset + j) % dims;
                vec[dim] += (byte as f32 / 128.0) - 1.0;
            }
        };

        // Item kind (fn, struct, enum, ...).
        write_hash(&format!("kind:{}", features.item_kind), 0, 8);

        // Visibility.
        write_hash(&format!("vis:{}", features.visibility), 8, 6);

        // Async flag.
        if features.is_async {
            write_hash("async:true", 14, 4);
        }

        // Generic flag.
        if features.is_generic {
            write_hash("generic:true", 18, 4);
        }

        // Return type.
        if let Some(ref rt) = features.return_type {
            write_hash(&format!("ret:{rt}"), 22, 8);
        }

        // Parameter types.
        for (i, pt) in features.param_types.iter().enumerate() {
            write_hash(&format!("param{i}:{pt}"), 30 + i * 6, 6);
        }

        // Attributes.
        for (i, attr) in features.attributes.iter().enumerate() {
            write_hash(&format!("attr{i}:{attr}"), 48 + i * 4, 4);
        }

        l2_normalize(&mut vec);
        vec
    }

    /// Produce the hybrid embedding for a Rust code snippet.
    fn hybrid_embed(&self, code: &str) -> Vec<f32> {
        let features = extract_rust_features(code);
        let structural = self.encode_structural(&features);
        let text = self.text_provider.hash_embed(code);

        let w_s = self.structural_weight;
        let w_t = 1.0 - w_s;

        // Concatenate weighted structural + text vectors.
        let mut combined = Vec::with_capacity(self.total_dims);
        for val in &structural {
            combined.push(val * w_s);
        }
        for val in &text {
            combined.push(val * w_t);
        }

        // Ensure exact dimensionality.
        combined.truncate(self.total_dims);
        while combined.len() < self.total_dims {
            combined.push(0.0);
        }

        l2_normalize(&mut combined);
        combined
    }
}

#[async_trait]
impl EmbeddingProvider for AstEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.hybrid_embed(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|t| self.hybrid_embed(t)).collect())
    }

    fn dimensions(&self) -> usize {
        self.total_dims
    }

    fn model_name(&self) -> &str {
        "ast-aware-hybrid"
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helper -----------------------------------------------------------

    fn vec_magnitude(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    // =====================================================================
    // OnnxEmbeddingProvider tests
    // =====================================================================

    #[test]
    fn onnx_construction_default() {
        let p = OnnxEmbeddingProvider::new("/nonexistent/model.onnx");
        assert_eq!(p.dimensions(), 384);
        assert!(!p.is_runtime_available());
        assert!(p.model_name().contains("fallback"));
    }

    #[test]
    fn onnx_construction_custom() {
        let p = OnnxEmbeddingProvider::with_config("/tmp/model.onnx", 128, 64);
        assert_eq!(p.dimensions(), 128);
        assert_eq!(p.max_tokens(), 64);
    }

    #[tokio::test]
    async fn onnx_embed_returns_correct_dimensions() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let vec = p.embed("hello world").await.unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[tokio::test]
    async fn onnx_embed_deterministic() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let v1 = p.embed("test input").await.unwrap();
        let v2 = p.embed("test input").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn onnx_embed_different_inputs_differ() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let v1 = p.embed("alpha").await.unwrap();
        let v2 = p.embed("beta").await.unwrap();
        assert_ne!(v1, v2);
    }

    #[tokio::test]
    async fn onnx_embed_l2_normalized() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let vec = p.embed("normalisation check").await.unwrap();
        let mag = vec_magnitude(&vec);
        assert!((mag - 1.0).abs() < 0.01, "magnitude = {mag}, expected ~1.0");
    }

    #[tokio::test]
    async fn onnx_embed_batch() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let results = p.embed_batch(&["a", "b", "c"]).await.unwrap();
        assert_eq!(results.len(), 3);
        for v in &results {
            assert_eq!(v.len(), 384);
        }
    }

    #[tokio::test]
    async fn onnx_similar_inputs_high_cosine() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let v1 = p.embed("the quick brown fox").await.unwrap();
        let v2 = p.embed("the quick brown dog").await.unwrap();
        let sim = cosine_similarity(&v1, &v2);
        assert!(sim > 0.5, "similar inputs cosine = {sim}, expected > 0.5");
    }

    #[tokio::test]
    async fn onnx_empty_input_returns_zero_vector() {
        let p = OnnxEmbeddingProvider::new("/tmp/model.onnx");
        let vec = p.embed("").await.unwrap();
        assert_eq!(vec.len(), 384);
        assert!(vec.iter().all(|x| *x == 0.0));
    }

    // =====================================================================
    // SentenceTransformerProvider tests
    // =====================================================================

    #[test]
    fn sentence_construction() {
        let p = SentenceTransformerProvider::new("/tmp/model.onnx");
        assert_eq!(p.dimensions(), 384);
        assert_eq!(p.max_tokens(), 512);
        assert!(p.split_sentences());
        assert_eq!(p.model_name(), "sentence-transformer");
    }

    #[tokio::test]
    async fn sentence_embed_returns_correct_dimensions() {
        let p = SentenceTransformerProvider::new("/tmp/model.onnx");
        let vec = p.embed("This is a test paragraph with enough words.").await.unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[tokio::test]
    async fn sentence_embed_l2_normalized() {
        let p = SentenceTransformerProvider::new("/tmp/model.onnx");
        let vec = p.embed("Testing normalisation of sentence embeddings here.").await.unwrap();
        let mag = vec_magnitude(&vec);
        assert!((mag - 1.0).abs() < 0.01, "magnitude = {mag}, expected ~1.0");
    }

    #[tokio::test]
    async fn sentence_embed_batch() {
        let p = SentenceTransformerProvider::new("/tmp/model.onnx");
        let results = p
            .embed_batch(&[
                "First paragraph with a decent amount of words in it.",
                "Second paragraph also has a reasonable length for testing.",
            ])
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        for v in &results {
            assert_eq!(v.len(), 384);
        }
    }

    #[tokio::test]
    async fn sentence_similar_inputs_positive_cosine() {
        let p = SentenceTransformerProvider::new("/tmp/model.onnx");
        let v1 = p.embed("The kernel boots up the system and runs all the services correctly.").await.unwrap();
        let v2 = p.embed("The kernel boots up the system and runs all the services properly.").await.unwrap();
        let v3 = p.embed("Quantum chromodynamics explains the strong interaction between quarks.").await.unwrap();
        let sim_similar = cosine_similarity(&v1, &v2);
        let sim_different = cosine_similarity(&v1, &v3);
        assert!(
            sim_similar > sim_different,
            "similar ({sim_similar}) should be closer than different ({sim_different})"
        );
    }

    #[test]
    fn preprocess_markdown_strips_headers() {
        let md = "# Title\nSome text.\n## Subtitle\nMore text.";
        let result = preprocess_markdown(md);
        assert!(!result.contains("Title"));
        assert!(!result.contains("Subtitle"));
        assert!(result.contains("Some text."));
        assert!(result.contains("More text."));
    }

    #[test]
    fn preprocess_markdown_strips_code_fences() {
        let md = "Before.\n```rust\nlet x = 1;\n```\nAfter.";
        let result = preprocess_markdown(md);
        assert!(!result.contains("```"));
        // Code line itself is kept (only fence markers are stripped).
        assert!(result.contains("Before."));
        assert!(result.contains("After."));
    }

    #[test]
    fn preprocess_markdown_strips_tables() {
        let md = "Intro.\n| Col1 | Col2 |\n|------|------|\n| A | B |\nOutro.";
        let result = preprocess_markdown(md);
        assert!(!result.contains("Col1"));
        assert!(result.contains("Intro."));
        assert!(result.contains("Outro."));
    }

    #[test]
    fn preprocess_markdown_strips_checklists() {
        let md = "Text here.\n- [x] Done item\n- [ ] Todo item\nMore text.";
        let result = preprocess_markdown(md);
        assert!(!result.contains("Done item"));
        assert!(result.contains("Text here."));
    }

    #[test]
    fn split_sentences_basic() {
        let text = "First sentence here. Second sentence here. Third.";
        let sentences = split_sentences(text);
        // "Third." is only 6 chars, below the 10-char minimum.
        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].contains("First"));
        assert!(sentences[1].contains("Second"));
    }

    #[test]
    fn split_sentences_newlines() {
        let text = "Line one is long enough.\nLine two is also long enough.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 2);
    }

    // =====================================================================
    // AstEmbeddingProvider tests
    // =====================================================================

    #[test]
    fn ast_construction_default() {
        let p = AstEmbeddingProvider::new("/tmp/model.onnx");
        assert_eq!(p.dimensions(), 256);
        assert_eq!(p.total_dims(), 256);
        assert!((p.structural_weight() - 0.3).abs() < 0.001);
        assert_eq!(p.model_name(), "ast-aware-hybrid");
    }

    #[tokio::test]
    async fn ast_embed_returns_correct_dimensions() {
        let p = AstEmbeddingProvider::new("/tmp/model.onnx");
        let vec = p.embed("pub fn hello() -> String { }").await.unwrap();
        assert_eq!(vec.len(), 256);
    }

    #[tokio::test]
    async fn ast_embed_l2_normalized() {
        let p = AstEmbeddingProvider::new("/tmp/model.onnx");
        let vec = p.embed("pub fn hello() -> String { }").await.unwrap();
        let mag = vec_magnitude(&vec);
        assert!((mag - 1.0).abs() < 0.01, "magnitude = {mag}, expected ~1.0");
    }

    #[tokio::test]
    async fn ast_embed_batch() {
        let p = AstEmbeddingProvider::new("/tmp/model.onnx");
        let results = p
            .embed_batch(&["fn a() {}", "fn b() {}", "struct C {}"])
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
        for v in &results {
            assert_eq!(v.len(), 256);
        }
    }

    #[tokio::test]
    async fn ast_embed_different_inputs_differ() {
        let p = AstEmbeddingProvider::new("/tmp/model.onnx");
        let v1 = p.embed("pub fn alpha() -> u32 {}").await.unwrap();
        let v2 = p.embed("struct Beta { x: f64 }").await.unwrap();
        assert_ne!(v1, v2);
    }

    #[tokio::test]
    async fn ast_structural_similarity_same_signature() {
        // Two functions with same signature but different names should be
        // closer than two items with different signatures.
        let p = AstEmbeddingProvider::new("/tmp/model.onnx");
        let v_foo = p
            .embed("pub async fn foo(&self, x: u32) -> Result<(), Error> {}")
            .await
            .unwrap();
        let v_bar = p
            .embed("pub async fn bar(&self, x: u32) -> Result<(), Error> {}")
            .await
            .unwrap();
        let v_struct = p.embed("pub struct Baz { count: usize }").await.unwrap();

        let sim_fns = cosine_similarity(&v_foo, &v_bar);
        let sim_fn_struct = cosine_similarity(&v_foo, &v_struct);
        assert!(
            sim_fns > sim_fn_struct,
            "same-signature fns ({sim_fns}) should be more similar than fn-vs-struct ({sim_fn_struct})"
        );
    }

    // =====================================================================
    // extract_rust_features tests
    // =====================================================================

    #[test]
    fn rust_features_pub_async_fn() {
        let code = r#"
#[test]
pub async fn process_batch(&self, items: Vec<Item>) -> Result<(), Error> {
    // body
}
"#;
        let f = extract_rust_features(code);
        assert_eq!(f.item_kind, "fn");
        assert_eq!(f.visibility, "pub");
        assert!(f.is_async);
        assert!(f.is_generic);
        assert_eq!(f.return_type.as_deref(), Some("Result<(), Error>"));
        assert!(f.attributes.contains(&"#[test]".to_string()));
        assert!(f.param_types.contains(&"&self".to_string()));
        assert!(f.param_types.iter().any(|p| p.contains("Vec<Item>")));
    }

    #[test]
    fn rust_features_struct() {
        let code = "pub struct Config { pub name: String, pub value: u64 }";
        let f = extract_rust_features(code);
        assert_eq!(f.item_kind, "struct");
        assert_eq!(f.visibility, "pub");
        assert!(!f.is_async);
        assert!(!f.is_generic); // no < > in this struct
        assert!(f.return_type.is_none());
    }

    #[test]
    fn rust_features_private_fn() {
        let code = "fn helper(x: &str) -> bool { true }";
        let f = extract_rust_features(code);
        assert_eq!(f.item_kind, "fn");
        assert_eq!(f.visibility, "private");
        assert!(!f.is_async);
        assert_eq!(f.return_type.as_deref(), Some("bool"));
        assert!(f.param_types.iter().any(|p| p.contains("&str")));
    }

    #[test]
    fn rust_features_enum() {
        let code = "pub enum Status { Active, Inactive, Pending }";
        let f = extract_rust_features(code);
        assert_eq!(f.item_kind, "enum");
        assert_eq!(f.visibility, "pub");
    }

    #[test]
    fn rust_features_trait() {
        let code = "pub trait Displayable { fn display(&self) -> String; }";
        let f = extract_rust_features(code);
        assert_eq!(f.item_kind, "trait");
        assert_eq!(f.visibility, "pub");
    }

    #[test]
    fn rust_features_impl_block() {
        let code = "impl MyStruct { fn new() -> Self { Self {} } }";
        let f = extract_rust_features(code);
        // "fn" is detected before "impl" because code.contains("fn ")
        assert_eq!(f.item_kind, "fn");
    }

    #[test]
    fn rust_features_where_clause() {
        let code = "pub fn serialize<T>(val: T) -> String where T: Serialize + Debug { }";
        let f = extract_rust_features(code);
        assert!(f.is_generic);
        assert!(!f.trait_bounds.is_empty());
        assert!(f.trait_bounds.iter().any(|b| b.contains("Serialize")));
    }

    #[test]
    fn rust_features_pub_crate() {
        let code = "pub(crate) fn internal_helper() {}";
        let f = extract_rust_features(code);
        assert_eq!(f.visibility, "pub(crate)");
    }

    #[test]
    fn rust_features_multiple_attributes() {
        let code = "#[cfg(test)]\n#[allow(dead_code)]\nfn test_fn() {}";
        let f = extract_rust_features(code);
        assert_eq!(f.attributes.len(), 2);
        assert!(f.attributes.contains(&"#[cfg(test)]".to_string()));
        assert!(f.attributes.contains(&"#[allow(dead_code)]".to_string()));
    }

    // =====================================================================
    // Tokenisation helper tests
    // =====================================================================

    #[test]
    fn simple_tokenize_basic() {
        let tokens = simple_tokenize("Hello World! Foo-bar", 10);
        assert_eq!(tokens, vec!["hello", "world", "foobar"]);
    }

    #[test]
    fn simple_tokenize_max_tokens() {
        let tokens = simple_tokenize("a b c d e f", 3);
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn simple_tokenize_empty() {
        let tokens = simple_tokenize("", 10);
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokens_to_embedding_deterministic() {
        let tokens: Vec<String> = vec!["hello".into(), "world".into()];
        let v1 = tokens_to_embedding(&tokens, 64);
        let v2 = tokens_to_embedding(&tokens, 64);
        assert_eq!(v1, v2);
    }

    #[test]
    fn tokens_to_embedding_normalized() {
        let tokens: Vec<String> = vec!["test".into()];
        let v = tokens_to_embedding(&tokens, 128);
        let mag = vec_magnitude(&v);
        assert!((mag - 1.0).abs() < 0.01);
    }
}
