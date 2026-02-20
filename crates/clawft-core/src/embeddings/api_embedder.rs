//! API-based embedder using an OpenAI-compatible embeddings endpoint.
//!
//! [`ApiEmbedder`] generates embeddings by calling a remote LLM provider's
//! `/embeddings` endpoint. When no API key is configured (or the call fails),
//! it falls back to a deterministic SHA-256-based pseudo-embedding for
//! development and testing.
//!
//! This module is gated behind the `rvf` feature flag.

use std::collections::HashMap;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use super::{Embedder, EmbeddingError};

/// Default embedding dimension when using the hash fallback.
const DEFAULT_DIMENSION: usize = 384;

/// Configuration for the API embedder.
#[derive(Debug, Clone)]
pub struct ApiEmbedderConfig {
    /// Base URL for the embedding endpoint (e.g. `https://api.openai.com/v1`).
    pub base_url: String,
    /// Environment variable holding the API key.
    pub api_key_env: String,
    /// The embedding model name (e.g. `text-embedding-3-small`).
    pub model: String,
    /// Embedding dimension override (if the provider supports it).
    pub dimension: Option<usize>,
    /// Extra HTTP headers for the request.
    pub headers: HashMap<String, String>,
}

impl Default for ApiEmbedderConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key_env: "OPENAI_API_KEY".into(),
            model: "text-embedding-3-small".into(),
            dimension: Some(DEFAULT_DIMENSION),
            headers: HashMap::new(),
        }
    }
}

/// Generates embeddings via a remote API endpoint, with hash fallback.
///
/// # API Mode
///
/// When a valid API key is available, sends requests to the provider's
/// `/embeddings` endpoint in the OpenAI-compatible format.
///
/// # Fallback Mode
///
/// When no API key is set or the API call fails, produces deterministic
/// pseudo-embeddings using SHA-256 hashing. These are NOT semantically
/// meaningful but provide consistent, reproducible vectors for testing
/// and development workflows.
pub struct ApiEmbedder {
    config: ApiEmbedderConfig,
    http: reqwest::Client,
    dim: usize,
}

impl ApiEmbedder {
    /// Create a new `ApiEmbedder` with the given configuration.
    pub fn new(config: ApiEmbedderConfig) -> Self {
        let dim = config.dimension.unwrap_or(DEFAULT_DIMENSION);
        Self {
            config,
            http: reqwest::Client::new(),
            dim,
        }
    }

    /// Create an `ApiEmbedder` with default configuration (OpenAI, 384-dim).
    pub fn with_defaults() -> Self {
        Self::new(ApiEmbedderConfig::default())
    }

    /// Create a fallback-only embedder that never calls an API.
    ///
    /// Useful for testing or environments without API access.
    pub fn hash_only(dimension: usize) -> Self {
        Self::new(ApiEmbedderConfig {
            api_key_env: String::new(), // empty => always falls back
            dimension: Some(dimension),
            ..ApiEmbedderConfig::default()
        })
    }

    /// Resolve the API key from the environment.
    fn resolve_api_key(&self) -> Option<String> {
        if self.config.api_key_env.is_empty() {
            return None;
        }
        std::env::var(&self.config.api_key_env).ok()
    }

    /// Call the remote embeddings API for a single text.
    async fn call_api(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let api_key = self.resolve_api_key().ok_or_else(|| {
            EmbeddingError::Internal(format!("API key not set: {}", self.config.api_key_env))
        })?;

        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{base}/embeddings");

        let mut body = serde_json::json!({
            "input": texts,
            "model": self.config.model,
        });

        if let Some(dim) = self.config.dimension {
            body["dimensions"] = serde_json::json!(dim);
        }

        let mut req = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json");

        for (k, v) in &self.config.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let response = req
            .json(&body)
            .send()
            .await
            .map_err(|e| EmbeddingError::Internal(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Internal(format!(
                "API error HTTP {status}: {body_text}"
            )));
        }

        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EmbeddingError::Internal(format!("failed to parse API response: {e}")))?;

        // Parse OpenAI-format response: { data: [{ embedding: [...] }, ...] }
        let data = resp_json
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| EmbeddingError::Internal("response missing 'data' array".into()))?;

        let mut results = Vec::with_capacity(data.len());
        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| {
                    EmbeddingError::Internal("response item missing 'embedding'".into())
                })?;

            let vec: Vec<f32> = embedding
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();

            results.push(vec);
        }

        Ok(results)
    }

    /// Generate a deterministic pseudo-embedding from text using SHA-256.
    ///
    /// This is NOT semantically meaningful -- it is a fallback for when no
    /// API is available. The output is deterministic: same input always
    /// produces the same vector.
    pub fn hash_fallback(&self, text: &str) -> Vec<f32> {
        let normalized = text.to_lowercase();
        let mut vector = vec![0.0f32; self.dim];

        if normalized.trim().is_empty() {
            return vector;
        }

        // Hash the full text to seed the vector
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let hash = hasher.finalize();

        // Fill the vector by cycling through the hash bytes
        for (i, val) in vector.iter_mut().enumerate() {
            let byte_idx = i % hash.len();
            let bit_idx = (i / hash.len()) % 8;
            let byte = hash[byte_idx];
            if (byte >> bit_idx) & 1 == 1 {
                *val = 1.0;
            } else {
                *val = -1.0;
            }
        }

        // Normalize to unit length
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut vector {
                *val /= norm;
            }
        }

        vector
    }
}

#[async_trait]
impl Embedder for ApiEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        // Try API first, fall back to hash
        match self.call_api(&[text]).await {
            Ok(mut results) if !results.is_empty() => {
                debug!(model = %self.config.model, "API embedding succeeded");
                Ok(results.remove(0))
            }
            Ok(_) => {
                warn!("API returned empty embeddings, using hash fallback");
                Ok(self.hash_fallback(text))
            }
            Err(e) => {
                debug!(error = %e, "API embedding failed, using hash fallback");
                Ok(self.hash_fallback(text))
            }
        }
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        match self.call_api(&text_refs).await {
            Ok(results) if results.len() == texts.len() => {
                debug!(
                    model = %self.config.model,
                    count = texts.len(),
                    "batch API embedding succeeded"
                );
                Ok(results)
            }
            Ok(_) | Err(_) => {
                debug!("batch API embedding failed, using hash fallback");
                Ok(texts.iter().map(|t| self.hash_fallback(t)).collect())
            }
        }
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_fallback_deterministic() {
        let embedder = ApiEmbedder::hash_only(384);
        let e1 = embedder.hash_fallback("hello world");
        let e2 = embedder.hash_fallback("hello world");
        assert_eq!(e1, e2, "same text must produce identical hash embeddings");
    }

    #[test]
    fn hash_fallback_different_text() {
        let embedder = ApiEmbedder::hash_only(384);
        let e1 = embedder.hash_fallback("hello world");
        let e2 = embedder.hash_fallback("goodbye moon");
        assert_ne!(e1, e2, "different text should produce different embeddings");
    }

    #[test]
    fn hash_fallback_correct_dimension() {
        let embedder = ApiEmbedder::hash_only(128);
        let emb = embedder.hash_fallback("test text");
        assert_eq!(emb.len(), 128);
    }

    #[test]
    fn hash_fallback_unit_norm() {
        let embedder = ApiEmbedder::hash_only(384);
        let emb = embedder.hash_fallback("the quick brown fox");
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "L2 norm should be ~1.0, got {norm}"
        );
    }

    #[test]
    fn hash_fallback_empty_string() {
        let embedder = ApiEmbedder::hash_only(384);
        let emb = embedder.hash_fallback("");
        assert_eq!(emb.len(), 384);
        let sum: f32 = emb.iter().map(|x| x.abs()).sum();
        assert!(
            sum < f32::EPSILON,
            "empty string should produce zero vector, sum={sum}"
        );
    }

    #[test]
    fn hash_fallback_case_insensitive() {
        let embedder = ApiEmbedder::hash_only(384);
        let e1 = embedder.hash_fallback("Hello World");
        let e2 = embedder.hash_fallback("hello world");
        assert_eq!(e1, e2, "hash fallback should be case-insensitive");
    }

    #[test]
    fn default_config() {
        let config = ApiEmbedderConfig::default();
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.api_key_env, "OPENAI_API_KEY");
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.dimension, Some(384));
    }

    #[test]
    fn with_defaults_creates_embedder() {
        let embedder = ApiEmbedder::with_defaults();
        assert_eq!(embedder.dimension(), 384);
    }

    #[test]
    fn hash_only_creates_embedder() {
        let embedder = ApiEmbedder::hash_only(256);
        assert_eq!(embedder.dimension(), 256);
    }

    #[tokio::test]
    async fn embed_falls_back_without_api_key() {
        // No API key set, should fall back to hash
        let embedder = ApiEmbedder::hash_only(64);
        let result = embedder.embed("test text").await;
        assert!(result.is_ok());
        let emb = result.unwrap();
        assert_eq!(emb.len(), 64);
    }

    #[tokio::test]
    async fn embed_batch_falls_back_without_api_key() {
        let embedder = ApiEmbedder::hash_only(64);
        let texts = vec!["hello".to_string(), "world".to_string()];
        let result = embedder.embed_batch(&texts).await;
        assert!(result.is_ok());
        let batch = result.unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].len(), 64);
        assert_eq!(batch[1].len(), 64);
    }

    #[tokio::test]
    async fn embed_batch_consistency_with_single() {
        let embedder = ApiEmbedder::hash_only(64);
        let texts = vec!["hello world".to_string(), "goodbye moon".to_string()];
        let batch = embedder.embed_batch(&texts).await.unwrap();

        let e1 = embedder.embed("hello world").await.unwrap();
        let e2 = embedder.embed("goodbye moon").await.unwrap();

        assert_eq!(batch[0], e1);
        assert_eq!(batch[1], e2);
    }

    #[tokio::test]
    async fn trait_dimension_matches() {
        let embedder = ApiEmbedder::hash_only(128);
        let emb = embedder.embed("test").await.unwrap();
        assert_eq!(emb.len(), embedder.dimension());
    }
}
