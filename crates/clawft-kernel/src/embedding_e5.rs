//! e5-small-v2 ONNX embedding provider (WEFT-640).
//!
//! Thin asymmetric wrapper over the BERT-family
//! [`OnnxEmbeddingProvider`](crate::embedding_onnx::OnnxEmbeddingProvider):
//! same WordPiece tokenizer, masked-mean pool, and L2 norm — the *only*
//! behavioural difference is the e5 training convention of prepending
//! `passage: ` on documents and `query: ` on queries.
//!
//! - **Dim**: 384 (matches RVF/HNSW default and `weave.toml` target).
//! - **License**: MIT weights (`intfloat/e5-small-v2`); community ONNX exports.
//! - **Staging**: `.weftos/models/e5-small-v2/model.onnx` (+ sibling `vocab.txt`).
//! - **CI**: without the `onnx-embeddings` feature or staged artifacts the
//!   provider degrades to the ONNX hash fallback (deterministic, no download).
//!
//! Selection: [`select_embedding_provider`](crate::embedding::select_embedding_provider)
//! promotes e5 when its ONNX file is present and the runtime loads, **below**
//! project-canonical Qwen3 (ADR-059) and **above** legacy MiniLM.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::embedding::{EmbeddingError, EmbeddingProvider};
use crate::embedding_onnx::OnnxEmbeddingProvider;

/// Canonical model identifier (embedder_id / consistency contract).
pub const E5_MODEL_NAME: &str = "e5-small-v2";

/// Native output dimensionality (matches RVF / HNSW default).
pub const E5_DIMS: usize = 384;

/// Max tokens for e5-small-v2 (BERT 512 context; leave room for specials).
pub const E5_MAX_TOKENS: usize = 512;

/// Document / passage prefix required by e5 training.
pub const PASSAGE_PREFIX: &str = "passage: ";

/// Query prefix required by e5 training.
pub const QUERY_PREFIX: &str = "query: ";

/// e5-small-v2 provider — 384-d BERT encoder with asymmetric query/passage prefixes.
pub struct E5EmbeddingProvider {
    inner: OnnxEmbeddingProvider,
    /// Stable name exposed via [`EmbeddingProvider::model_name`].
    model_name: String,
}

impl E5EmbeddingProvider {
    /// Create a provider pointing at an e5 ONNX model file.
    ///
    /// Degrades to the ONNX hash fallback when the feature is off, the file is
    /// missing, or session load fails — never panics, never requires the model
    /// to construct (CI-safe).
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        let model_path = model_path.into();
        let inner =
            OnnxEmbeddingProvider::with_config(model_path, E5_DIMS, E5_MAX_TOKENS);
        let model_name = if inner.is_runtime_available() {
            E5_MODEL_NAME.to_string()
        } else {
            format!("{E5_MODEL_NAME}-hash-fallback")
        };
        Self { inner, model_name }
    }

    /// Whether real ONNX inference is active (vs. hash fallback).
    pub fn is_runtime_available(&self) -> bool {
        self.inner.is_runtime_available()
    }

    /// Path to the configured ONNX model file.
    pub fn model_path(&self) -> &Path {
        self.inner.model_path()
    }

    /// Prefix a string for the **passage** (document) lane.
    pub fn prefix_passage(text: &str) -> String {
        if text.starts_with(PASSAGE_PREFIX) || text.starts_with(QUERY_PREFIX) {
            text.to_string()
        } else {
            format!("{PASSAGE_PREFIX}{text}")
        }
    }

    /// Prefix a string for the **query** lane.
    pub fn prefix_query(text: &str) -> String {
        if text.starts_with(QUERY_PREFIX) || text.starts_with(PASSAGE_PREFIX) {
            text.to_string()
        } else {
            format!("{QUERY_PREFIX}{text}")
        }
    }
}

#[async_trait]
impl EmbeddingProvider for E5EmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // Document / passage lane.
        self.inner.embed(&Self::prefix_passage(text)).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    async fn embed_query(&self, query: &str) -> Result<Vec<f32>, EmbeddingError> {
        // Asymmetric query lane — required for e5 retrieval quality.
        self.inner.embed(&Self::prefix_query(query)).await
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_passage_is_idempotent() {
        let once = E5EmbeddingProvider::prefix_passage("hello");
        assert_eq!(once, "passage: hello");
        assert_eq!(E5EmbeddingProvider::prefix_passage(&once), once);
    }

    #[test]
    fn prefix_query_is_idempotent() {
        let once = E5EmbeddingProvider::prefix_query("find deploys");
        assert_eq!(once, "query: find deploys");
        assert_eq!(E5EmbeddingProvider::prefix_query(&once), once);
    }

    #[test]
    fn degraded_provider_reports_hash_fallback_name() {
        // No real model file — hash fallback, CI-safe.
        let p = E5EmbeddingProvider::new("/nonexistent/e5-small-v2/model.onnx");
        assert!(!p.is_runtime_available());
        assert_eq!(p.model_name(), "e5-small-v2-hash-fallback");
        assert_eq!(p.dimensions(), E5_DIMS);
    }

    #[tokio::test]
    async fn embed_and_embed_query_differ_under_hash_fallback() {
        // Prefixes change the hash input → different vectors. Proves the
        // asymmetric lane is wired even without ONNX weights.
        let p = E5EmbeddingProvider::new("/nonexistent/e5-small-v2/model.onnx");
        let doc = p.embed("the deploy OOM'd on model load").await.unwrap();
        let qry = p.embed_query("the deploy OOM'd on model load").await.unwrap();
        assert_eq!(doc.len(), E5_DIMS);
        assert_eq!(qry.len(), E5_DIMS);
        assert_ne!(
            doc, qry,
            "query: vs passage: prefixes must change the hash-fallback vector"
        );
    }

    #[tokio::test]
    async fn embed_is_deterministic() {
        let p = E5EmbeddingProvider::new("/nonexistent/e5-small-v2/model.onnx");
        let a = p.embed("stable input").await.unwrap();
        let b = p.embed("stable input").await.unwrap();
        assert_eq!(a, b);
    }
}
