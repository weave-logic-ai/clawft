//! Pluggable embedding backends for ECC vector operations (K3c-G2).
//!
//! Provides the [`EmbeddingProvider`] trait that the [`WeaverEngine`] uses
//! to convert text into vector embeddings for HNSW storage and similarity
//! search. Ships with [`MockEmbeddingProvider`] for deterministic testing.

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;

// ---------------------------------------------------------------------------
// EmbeddingError
// ---------------------------------------------------------------------------

/// Errors that embedding backends may produce.
#[derive(Debug)]
pub enum EmbeddingError {
    /// The underlying model has not been loaded yet.
    ModelNotLoaded,
    /// Vector dimensionality does not match the expected value.
    DimensionMismatch {
        /// Expected dimensionality.
        expected: usize,
        /// Actual dimensionality returned.
        got: usize,
    },
    /// Generic backend failure.
    BackendError(String),
    /// Rate-limited; caller should retry after the given duration.
    RateLimited {
        /// How long to wait before retrying.
        retry_after: Duration,
    },
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotLoaded => write!(f, "embedding model not loaded"),
            Self::DimensionMismatch { expected, got } => {
                write!(f, "dimension mismatch: expected {expected}, got {got}")
            }
            Self::BackendError(msg) => write!(f, "embedding backend error: {msg}"),
            Self::RateLimited { retry_after } => {
                write!(f, "rate limited, retry after {}ms", retry_after.as_millis())
            }
        }
    }
}

impl std::error::Error for EmbeddingError {}

// ---------------------------------------------------------------------------
// EmbeddingProvider trait
// ---------------------------------------------------------------------------

/// Trait for pluggable embedding backends.
///
/// Implementations convert text into fixed-dimensionality float vectors
/// suitable for HNSW indexing and cosine similarity search.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text chunk into a vector.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Embed a batch of text chunks.
    ///
    /// The default implementation calls [`embed`](Self::embed) in a loop.
    /// Backends that support native batching should override this for
    /// efficiency.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Dimensionality of the output vectors.
    fn dimensions(&self) -> usize;

    /// Name of the embedding model (for metadata tracking).
    fn model_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// MockEmbeddingProvider
// ---------------------------------------------------------------------------

/// Deterministic embedding provider for testing.
///
/// Produces vectors derived from a SHA-256 hash of the input text,
/// ensuring reproducible results without any external model dependency.
pub struct MockEmbeddingProvider {
    /// Output vector dimensionality.
    pub dims: usize,
}

impl MockEmbeddingProvider {
    /// Create a mock provider with the given output dimensionality.
    pub fn new(dims: usize) -> Self {
        Self { dims }
    }

    /// Deterministic hash-based embedding generation.
    fn hash_embed(&self, text: &str) -> Vec<f32> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();

        let mut vec = Vec::with_capacity(self.dims);
        for i in 0..self.dims {
            // Cycle through hash bytes, normalise to [-1, 1]
            let byte = hash[i % 32];
            vec.push((byte as f32 / 128.0) - 1.0);
        }
        vec
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.hash_embed(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|t| self.hash_embed(t)).collect())
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn model_name(&self) -> &str {
        "mock-sha256"
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_embed_returns_correct_dimensions() {
        let provider = MockEmbeddingProvider::new(64);
        let vec = provider.embed("hello world").await.unwrap();
        assert_eq!(vec.len(), 64);
    }

    #[tokio::test]
    async fn mock_embed_deterministic() {
        let provider = MockEmbeddingProvider::new(32);
        let v1 = provider.embed("test input").await.unwrap();
        let v2 = provider.embed("test input").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn mock_embed_different_inputs_differ() {
        let provider = MockEmbeddingProvider::new(32);
        let v1 = provider.embed("alpha").await.unwrap();
        let v2 = provider.embed("beta").await.unwrap();
        assert_ne!(v1, v2);
    }

    #[tokio::test]
    async fn mock_embed_batch() {
        let provider = MockEmbeddingProvider::new(16);
        let results = provider.embed_batch(&["a", "b", "c"]).await.unwrap();
        assert_eq!(results.len(), 3);
        for v in &results {
            assert_eq!(v.len(), 16);
        }
    }

    #[tokio::test]
    async fn mock_embed_batch_matches_individual() {
        let provider = MockEmbeddingProvider::new(8);
        let batch = provider.embed_batch(&["x", "y"]).await.unwrap();
        let x = provider.embed("x").await.unwrap();
        let y = provider.embed("y").await.unwrap();
        assert_eq!(batch[0], x);
        assert_eq!(batch[1], y);
    }

    #[test]
    fn mock_model_name() {
        let provider = MockEmbeddingProvider::new(16);
        assert_eq!(provider.model_name(), "mock-sha256");
    }

    #[test]
    fn mock_dimensions() {
        let provider = MockEmbeddingProvider::new(128);
        assert_eq!(provider.dimensions(), 128);
    }

    #[test]
    fn embedding_error_display() {
        let err = EmbeddingError::DimensionMismatch {
            expected: 384,
            got: 256,
        };
        assert!(err.to_string().contains("384"));
        assert!(err.to_string().contains("256"));

        let err2 = EmbeddingError::ModelNotLoaded;
        assert!(err2.to_string().contains("not loaded"));
    }
}
