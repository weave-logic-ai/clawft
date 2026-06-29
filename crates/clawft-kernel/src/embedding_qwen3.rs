//! Qwen3-Embedding-0.6B provider (ADR-059) — the preferred ECC embedder.
//!
//! Unlike the BERT [`OnnxEmbeddingProvider`](crate::embedding_onnx::OnnxEmbeddingProvider),
//! Qwen3-Embedding is a **causal decoder-with-KV-cache** ONNX export. This
//! provider therefore:
//!
//! - feeds `input_ids` / `attention_mask` / `position_ids` (`0..seq_len`) **plus
//!   empty, dtype-matched `past_key_values.*` tensors** for every layer (shape
//!   `[1, n_kv_heads, 0, head_dim]`, derived from the graph — never hardcoded);
//! - pools the **last non-pad token** hidden state (not mean/CLS);
//! - applies **MRL**: slice to the first [`QWEN3_DIMS`] dims, *then* L2-renormalize;
//! - uses the HF `tokenizers` crate (Qwen3 BPE from `tokenizer.json`);
//! - exposes [`embed_query`](Qwen3EmbeddingProvider::embed_query) for the
//!   asymmetric query lane (prepends the Qwen3 instruction prefix); the
//!   [`EmbeddingProvider::embed`] trait method is document mode.
//!
//! Without the `onnx-embeddings` feature, the model file, or the tokenizer, the
//! provider degrades gracefully to a [`MockEmbeddingProvider`] so builds and
//! tests never require the ~1.2 GB artifact. The BERT provider stays intact as a
//! lower-priority fallback (see [`select_embedding_provider`](crate::embedding::select_embedding_provider)).

use std::path::{Path, PathBuf};
#[cfg(feature = "onnx-embeddings")]
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::embedding::{EmbeddingError, EmbeddingProvider, MockEmbeddingProvider};

/// MRL target dimensionality — the empirical sweet spot (ADR-059, recall@10 0.983).
pub const QWEN3_DIMS: usize = 512;

/// Canonical model identifier (pins the consistency contract; ADR-059).
pub const QWEN3_MODEL_NAME: &str = "Qwen3-Embedding-0.6B";

/// Qwen3 asymmetric **query** instruction prefix. Documents are embedded raw;
/// queries are prefixed with this (official `get_detailed_instruct` format).
pub const QUERY_INSTRUCTION: &str =
    "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery:";

// ---------------------------------------------------------------------------
// Pure helpers (always compiled + unit-tested; only wired into production
// inference under `onnx-embeddings`, hence the conditional dead-code allow).
// ---------------------------------------------------------------------------

/// In-place L2 normalisation.
#[cfg_attr(not(feature = "onnx-embeddings"), allow(dead_code))]
fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vec.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Last-token pooling: return the hidden state at the **last non-pad position**.
///
/// `hidden` is the flattened `(seq, hidden_dim)` last-hidden-state for a single
/// sequence (batch 1). The last attended position is located via
/// `attention_mask`; if the mask is empty/all-zero the final row is used.
#[cfg_attr(not(feature = "onnx-embeddings"), allow(dead_code))]
fn last_token_pool(
    hidden: &[f32],
    seq: usize,
    hidden_dim: usize,
    attention_mask: &[i64],
) -> Vec<f32> {
    let last = attention_mask
        .iter()
        .take(seq)
        .rposition(|&m| m != 0)
        .unwrap_or(seq.saturating_sub(1));
    let start = last * hidden_dim;
    let end = start + hidden_dim;
    if end <= hidden.len() {
        hidden[start..end].to_vec()
    } else {
        // Defensive: shape mismatch — fall back to a zero vector.
        vec![0.0; hidden_dim]
    }
}

/// MRL: slice to the first `dims` dimensions **then** L2-renormalize (never the
/// reverse). Shorter inputs are zero-padded so the output is always `dims` long.
#[cfg_attr(not(feature = "onnx-embeddings"), allow(dead_code))]
fn mrl_slice_l2norm(full: &[f32], dims: usize) -> Vec<f32> {
    let mut v: Vec<f32> = full.iter().take(dims).copied().collect();
    if v.len() < dims {
        v.resize(dims, 0.0);
    }
    l2_normalize(&mut v);
    v
}

// ---------------------------------------------------------------------------
// KV-cache spec (derived from the ONNX graph, never hardcoded)
// ---------------------------------------------------------------------------

/// One `past_key_values.*` graph input: its name and the concrete head geometry.
#[cfg(feature = "onnx-embeddings")]
#[derive(Debug, Clone)]
struct KvInput {
    name: String,
    n_kv_heads: i64,
    head_dim: i64,
    fp16: bool,
}

// ---------------------------------------------------------------------------
// Qwen3EmbeddingProvider
// ---------------------------------------------------------------------------

/// Qwen3-Embedding-0.6B provider — decoder-ONNX, last-token pooling, MRL-512.
pub struct Qwen3EmbeddingProvider {
    /// Path to `model_fp16.onnx`.
    model_path: PathBuf,
    /// Path to `tokenizer.json`.
    tokenizer_path: PathBuf,
    /// Output dimensionality (always [`QWEN3_DIMS`]).
    dimensions: usize,
    /// Model name (real name when live, `*-mock-fallback` when degraded).
    model_name: String,
    /// Whether real decoder-ONNX inference is active (model + tokenizer loaded).
    runtime_available: bool,
    /// Degraded fallback (graceful Mock when no model/tokenizer/feature).
    fallback: MockEmbeddingProvider,
    /// Qwen3 BPE tokenizer loaded from `tokenizer.json`.
    #[cfg(feature = "onnx-embeddings")]
    tokenizer: Option<tokenizers::Tokenizer>,
    /// Decoder-ONNX session (`run` needs `&mut`, hence the mutex).
    #[cfg(feature = "onnx-embeddings")]
    session: Option<Arc<Mutex<ort::session::Session>>>,
    /// Empty-KV-cache spec derived from the graph inputs.
    #[cfg(feature = "onnx-embeddings")]
    kv_spec: Vec<KvInput>,
}

impl Qwen3EmbeddingProvider {
    /// Create a provider pointing at `model_fp16.onnx` + `tokenizer.json`.
    ///
    /// Degrades to Mock when the feature is off, files are missing, or loading
    /// fails — never panics, never requires the model to construct.
    pub fn new(model_path: impl Into<PathBuf>, tokenizer_path: impl Into<PathBuf>) -> Self {
        let model_path = model_path.into();
        let tokenizer_path = tokenizer_path.into();

        #[cfg(feature = "onnx-embeddings")]
        let tokenizer = Self::try_load_tokenizer(&tokenizer_path);
        #[cfg(feature = "onnx-embeddings")]
        let session = Self::try_load_session(&model_path);
        #[cfg(feature = "onnx-embeddings")]
        let kv_spec = session
            .as_ref()
            .and_then(|s| s.lock().ok().map(|g| Self::derive_kv_spec(&g)))
            .unwrap_or_default();

        #[cfg(feature = "onnx-embeddings")]
        let runtime_available = tokenizer.is_some() && session.is_some();
        #[cfg(not(feature = "onnx-embeddings"))]
        let runtime_available = false;

        Self {
            model_name: if runtime_available {
                QWEN3_MODEL_NAME.to_string()
            } else {
                format!("{QWEN3_MODEL_NAME}-mock-fallback")
            },
            model_path,
            tokenizer_path,
            dimensions: QWEN3_DIMS,
            runtime_available,
            fallback: MockEmbeddingProvider::new(QWEN3_DIMS),
            #[cfg(feature = "onnx-embeddings")]
            tokenizer,
            #[cfg(feature = "onnx-embeddings")]
            session,
            #[cfg(feature = "onnx-embeddings")]
            kv_spec,
        }
    }

    /// Whether real decoder-ONNX inference is active (vs. Mock fallback).
    pub fn is_runtime_available(&self) -> bool {
        self.runtime_available
    }

    /// Path to the configured ONNX model file.
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Path to the configured tokenizer file.
    pub fn tokenizer_path(&self) -> &Path {
        &self.tokenizer_path
    }

    /// Embed a **query** (asymmetric lane): prepends the Qwen3 instruction prefix.
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>, EmbeddingError> {
        #[cfg(feature = "onnx-embeddings")]
        if self.runtime_available {
            let prompted = format!("{QUERY_INSTRUCTION}{query}");
            return self.qwen3_embed(&prompted);
        }
        self.fallback.embed(query).await
    }

    #[cfg(feature = "onnx-embeddings")]
    fn try_load_tokenizer(path: &Path) -> Option<tokenizers::Tokenizer> {
        if !path.exists() {
            tracing::debug!("Qwen3 tokenizer.json not found at {}", path.display());
            return None;
        }
        match tokenizers::Tokenizer::from_file(path) {
            Ok(t) => {
                tracing::info!("Qwen3 tokenizer loaded from {}", path.display());
                Some(t)
            }
            Err(e) => {
                tracing::warn!("Failed to load Qwen3 tokenizer: {e}");
                None
            }
        }
    }

    #[cfg(feature = "onnx-embeddings")]
    fn try_load_session(model_path: &Path) -> Option<Arc<Mutex<ort::session::Session>>> {
        if !model_path.exists() {
            tracing::debug!("Qwen3 ONNX model not found at {}", model_path.display());
            return None;
        }
        match ort::session::Session::builder()
            .and_then(|mut builder| builder.commit_from_file(model_path))
        {
            Ok(session) => {
                tracing::info!("Qwen3 ONNX session loaded from {}", model_path.display());
                Some(Arc::new(Mutex::new(session)))
            }
            Err(e) => {
                tracing::warn!("Failed to load Qwen3 ONNX session: {e}");
                None
            }
        }
    }

    /// Derive the empty-KV-cache spec from the graph's `past_key_values.*`
    /// inputs — layer count, kv-heads, head-dim, and dtype are all read from the
    /// model, never hardcoded.
    #[cfg(feature = "onnx-embeddings")]
    fn derive_kv_spec(session: &ort::session::Session) -> Vec<KvInput> {
        let mut out = Vec::new();
        for input in session.inputs() {
            let name = input.name();
            if !name.starts_with("past_key_values") && !name.starts_with("past_key") {
                continue;
            }
            if let ort::value::ValueType::Tensor { ty, shape, .. } = input.dtype() {
                // [batch, n_kv_heads, past_len, head_dim]
                if shape.len() == 4 {
                    out.push(KvInput {
                        name: name.to_string(),
                        n_kv_heads: shape[1],
                        head_dim: shape[3],
                        fp16: matches!(ty, ort::value::TensorElementType::Float16),
                    });
                }
            }
        }
        out
    }

    /// Run real decoder-ONNX inference: tokenize → feed ids/mask/position_ids +
    /// empty past_key_values → last-token pool → MRL-512 slice + L2-norm.
    #[cfg(feature = "onnx-embeddings")]
    fn qwen3_embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        use std::borrow::Cow;

        use ort::session::SessionInputValue;
        use ort::value::Tensor;

        let session = self
            .session
            .as_ref()
            .ok_or_else(|| EmbeddingError::BackendError("Qwen3 session not loaded".to_string()))?;
        let tokenizer = self.tokenizer.as_ref().ok_or_else(|| {
            EmbeddingError::BackendError("Qwen3 tokenizer not loaded".to_string())
        })?;

        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| EmbeddingError::BackendError(format!("tokenize error: {e}")))?;
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&u| u as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&u| u as i64)
            .collect();
        let seq_len = input_ids.len();
        if seq_len == 0 {
            return Ok(vec![0.0; self.dimensions]);
        }
        let position_ids: Vec<i64> = (0..seq_len as i64).collect();
        let shape = vec![1_i64, seq_len as i64];

        let ids_t = Tensor::from_array((shape.clone(), input_ids))
            .map_err(|e| EmbeddingError::BackendError(format!("input_ids tensor: {e}")))?;
        let mask_t = Tensor::from_array((shape.clone(), attention_mask.clone()))
            .map_err(|e| EmbeddingError::BackendError(format!("attention_mask tensor: {e}")))?;
        let pos_t = Tensor::from_array((shape, position_ids))
            .map_err(|e| EmbeddingError::BackendError(format!("position_ids tensor: {e}")))?;

        let mut inputs = ort::inputs![
            "input_ids" => ids_t,
            "attention_mask" => mask_t,
            "position_ids" => pos_t,
        ];

        // Empty, dtype-matched KV cache for every layer (past_len = 0).
        for kv in &self.kv_spec {
            let kv_shape = vec![1_i64, kv.n_kv_heads, 0_i64, kv.head_dim];
            let value: SessionInputValue<'_> = if kv.fp16 {
                Tensor::from_array((kv_shape, Vec::<half::f16>::new()))
                    .map_err(|e| EmbeddingError::BackendError(format!("kv tensor: {e}")))?
                    .into()
            } else {
                Tensor::from_array((kv_shape, Vec::<f32>::new()))
                    .map_err(|e| EmbeddingError::BackendError(format!("kv tensor: {e}")))?
                    .into()
            };
            inputs.push((Cow::from(kv.name.clone()), value));
        }

        let mut guard = session.lock().map_err(|_| {
            EmbeddingError::BackendError("Qwen3 session mutex poisoned".to_string())
        })?;
        let outputs = guard
            .run(inputs)
            .map_err(|e| EmbeddingError::BackendError(format!("inference error: {e}")))?;

        let output = match outputs.get("last_hidden_state") {
            Some(v) => v,
            None if outputs.len() > 0 => &outputs[0],
            None => {
                return Err(EmbeddingError::BackendError("no output tensor".to_string()));
            }
        };

        // fp16 model → output is likely fp16; try f32 first, then f16→f32.
        let (out_shape, hidden): (Vec<i64>, Vec<f32>) = match output.try_extract_tensor::<f32>() {
            Ok((s, d)) => (s.to_vec(), d.to_vec()),
            Err(_) => {
                let (s, d) = output
                    .try_extract_tensor::<half::f16>()
                    .map_err(|e| EmbeddingError::BackendError(format!("extract error: {e}")))?;
                (s.to_vec(), d.iter().map(|h| h.to_f32()).collect())
            }
        };

        if out_shape.len() < 3 {
            return Err(EmbeddingError::BackendError(format!(
                "unexpected output shape: {out_shape:?}"
            )));
        }
        let seq = out_shape[out_shape.len() - 2] as usize;
        let hidden_dim = out_shape[out_shape.len() - 1] as usize;

        let pooled = last_token_pool(&hidden, seq, hidden_dim, &attention_mask);
        Ok(mrl_slice_l2norm(&pooled, self.dimensions))
    }
}

#[async_trait]
impl EmbeddingProvider for Qwen3EmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        #[cfg(feature = "onnx-embeddings")]
        if self.runtime_available {
            return self.qwen3_embed(text);
        }
        self.fallback.embed(text).await
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

// ── Tests (no model required) ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_magnitude(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    // -- last-token pooling math ------------------------------------------

    #[test]
    fn last_token_pool_picks_last_attended_row() {
        // 3 rows × 2 dims; mask marks row index 1 as the last attended one.
        let hidden = vec![1.0, 1.0, 2.0, 2.0, 9.0, 9.0];
        let mask = vec![1, 1, 0];
        let pooled = last_token_pool(&hidden, 3, 2, &mask);
        assert_eq!(pooled, vec![2.0, 2.0]);
    }

    #[test]
    fn last_token_pool_all_attended_picks_final_row() {
        let hidden = vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0];
        let mask = vec![1, 1, 1];
        let pooled = last_token_pool(&hidden, 3, 2, &mask);
        assert_eq!(pooled, vec![3.0, 3.0]);
    }

    #[test]
    fn last_token_pool_empty_mask_falls_back_to_final_row() {
        let hidden = vec![1.0, 1.0, 7.0, 7.0];
        let pooled = last_token_pool(&hidden, 2, 2, &[]);
        assert_eq!(pooled, vec![7.0, 7.0]);
    }

    // -- MRL slice + L2-norm ----------------------------------------------

    #[test]
    fn mrl_slices_to_512_and_normalizes() {
        let full: Vec<f32> = (0..1024).map(|i| (i as f32) + 1.0).collect();
        let out = mrl_slice_l2norm(&full, QWEN3_DIMS);
        assert_eq!(out.len(), QWEN3_DIMS, "must slice to 512 dims");
        let mag = vec_magnitude(&out);
        assert!((mag - 1.0).abs() < 1e-4, "magnitude = {mag}, expected ~1.0");
        // Slice-then-normalize: it must reflect the first 512 dims only.
        assert!(out[0] > 0.0 && out[511] > 0.0);
    }

    #[test]
    fn mrl_zero_pads_short_input() {
        let short = vec![3.0, 4.0]; // |[3,4]| = 5
        let out = mrl_slice_l2norm(&short, QWEN3_DIMS);
        assert_eq!(out.len(), QWEN3_DIMS);
        assert!((out[0] - 0.6).abs() < 1e-4);
        assert!((out[1] - 0.8).abs() < 1e-4);
        assert!(out[2..].iter().all(|&x| x == 0.0));
    }

    // -- tokenizer/model-absent fallback ----------------------------------

    #[tokio::test]
    async fn absent_model_degrades_to_mock() {
        let p =
            Qwen3EmbeddingProvider::new("/nonexistent/model_fp16.onnx", "/nonexistent/tok.json");
        assert!(!p.is_runtime_available());
        assert_eq!(p.dimensions(), QWEN3_DIMS);
        assert!(p.model_name().starts_with(QWEN3_MODEL_NAME));
        assert!(p.model_name().contains("fallback"));

        let v = p.embed("hello world").await.unwrap();
        assert_eq!(v.len(), QWEN3_DIMS);
        // Deterministic via the Mock SHA-256 backend.
        let v2 = p.embed("hello world").await.unwrap();
        assert_eq!(v, v2);
    }

    #[tokio::test]
    async fn fallback_query_embeds_without_model() {
        let p = Qwen3EmbeddingProvider::new("/nope.onnx", "/nope.json");
        let q = p
            .embed_query("what is the kernel boot order")
            .await
            .unwrap();
        assert_eq!(q.len(), QWEN3_DIMS);
    }

    #[tokio::test]
    async fn fallback_batch_matches_individual() {
        let p = Qwen3EmbeddingProvider::new("/nope.onnx", "/nope.json");
        let batch = p.embed_batch(&["a", "b"]).await.unwrap();
        let a = p.embed("a").await.unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0], a);
    }

    #[test]
    fn query_instruction_format_is_official() {
        assert!(QUERY_INSTRUCTION.starts_with("Instruct: "));
        assert!(QUERY_INSTRUCTION.ends_with("\nQuery:"));
    }

    // -- Live decoder-ONNX test (task 1.4 — needs the model) --------------

    #[cfg(feature = "onnx-embeddings")]
    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na = vec_magnitude(a);
        let nb = vec_magnitude(b);
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }

    /// Live decoder-ONNX inference (task 1.4). Requires `--features
    /// onnx-embeddings` **and** the model bundle at
    /// `.weftos/models/Qwen3-Embedding-0.6B/` (`model_fp16.onnx` +
    /// `tokenizer.json`, ~1.2 GB — ADR-059 follow-up 1). Run with:
    ///
    /// ```text
    /// cargo test -p clawft-kernel --features onnx-embeddings -- --ignored qwen3_live
    /// ```
    ///
    /// Full lab cosine-parity (≥ 0.99 vs MLX) is validated against the probe set
    /// in `~/llm/docs/models/results/embedlab.json` once the model is fetched.
    #[cfg(feature = "onnx-embeddings")]
    #[tokio::test]
    #[ignore = "requires Qwen3 model bundle (~1.2 GB) — ADR-059 follow-up 1"]
    async fn qwen3_live_inference_sanity() {
        let provider = Qwen3EmbeddingProvider::new(
            ".weftos/models/Qwen3-Embedding-0.6B/model_fp16.onnx",
            ".weftos/models/Qwen3-Embedding-0.6B/tokenizer.json",
        );
        assert!(
            provider.is_runtime_available(),
            "model bundle not loaded; fetch onnx-community/Qwen3-Embedding-0.6B-ONNX first"
        );

        // Document mode: 512-d, L2-normalized.
        let doc = provider
            .embed("The kernel boots the process table.")
            .await
            .unwrap();
        assert_eq!(doc.len(), QWEN3_DIMS);
        let mag = vec_magnitude(&doc);
        assert!((mag - 1.0).abs() < 1e-2, "doc not L2-normalized: {mag}");

        // Query mode (instruction-prefixed): 512-d.
        let q = provider
            .embed_query("how does the kernel boot")
            .await
            .unwrap();
        assert_eq!(q.len(), QWEN3_DIMS);

        // Relatedness sanity: related pair scores higher than an unrelated one.
        let a = provider.embed("process table allocation").await.unwrap();
        let b = provider
            .embed("allocating PIDs in the process table")
            .await
            .unwrap();
        let c = provider
            .embed("ocean acoustic sonar propagation")
            .await
            .unwrap();
        assert!(
            cosine(&a, &b) > cosine(&a, &c),
            "related cosine {} should exceed unrelated cosine {}",
            cosine(&a, &b),
            cosine(&a, &c)
        );
    }
}
