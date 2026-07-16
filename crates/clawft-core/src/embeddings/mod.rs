//! Embedding trait definitions and implementations.
//!
//! Provides the [`Embedder`] trait for generating vector embeddings from text,
//! plus a [`hash_embedder::HashEmbedder`] that uses SimHash for local,
//! deterministic embeddings with no API calls required.
//!
//! All types in this module are gated behind the `vector-memory` feature flag.

#[cfg(feature = "vector-memory")]
pub mod hash_embedder;
#[cfg(feature = "vector-memory")]
pub mod hnsw_store;
#[cfg(feature = "vector-memory")]
pub mod micro_hnsw;
// `rvf_stub` has no dependency on the real `rvf-runtime` crate (just serde +
// tracing, both unconditional deps), so it is gated on the broader
// `vector-memory` feature rather than `rvf`. This keeps it available as the
// vector-memory backend for builds that can't link `rvf-runtime` (wasm,
// browser, no-default) -- see `rvf_backend` below for how the two
// implementations are selected.
#[cfg(feature = "vector-memory")]
pub mod rvf_stub;

#[cfg(feature = "rvf")]
pub mod api_embedder;
#[cfg(feature = "rvf")]
pub mod progressive;
#[cfg(feature = "rvf")]
pub mod quantization;
#[cfg(feature = "rvf")]
pub mod rvf_real;
#[cfg(feature = "rvf")]
pub mod witness;

// Link-time fix for an rvf-runtime 0.2.0 portability bug on macOS -- see
// the module docs. Not part of the public surface.
#[cfg(all(feature = "rvf", target_os = "macos"))]
// macOS `__errno_location` shim: DELIBERATELY NOT DEFINED HERE. The `rvf`
// feature depends on `clawft-cow-memory`, whose `macos_errno_shim` provides
// the one `#[no_mangle]` definition for the whole binary — defining a second
// copy here breaks fat-LTO release links ("Linking globals named
// '__errno_location': symbol multiply defined!", found live at WEFT-615
// install). See clawft-cow-memory/src/macos_errno_shim.rs and WEFT-662.

/// Backend-selecting alias: real `rvf-runtime`-backed store when the `rvf`
/// feature is enabled (native builds that can link the crate), the in-memory
/// stub otherwise. Consumers should import [`RvfStore`]/[`RvfError`] from
/// here rather than reaching into `rvf_real`/`rvf_stub` directly, so they
/// automatically follow whichever backend the active feature set selects.
///
/// See `crates/clawft-core/src/embeddings/rvf_real.rs` module docs for the
/// on-disk layout and the real-vs-stub semantic gaps this bridges.
pub mod rvf_backend {
    #[cfg(feature = "rvf")]
    pub use super::rvf_real::{RvfEntry, RvfError, RvfQueryResult, RvfStore};
    #[cfg(all(feature = "vector-memory", not(feature = "rvf")))]
    pub use super::rvf_stub::{RvfEntry, RvfError, RvfQueryResult, RvfStore};
}

use async_trait::async_trait;
use std::fmt;

/// Errors that can occur during embedding generation.
#[non_exhaustive]
#[derive(Debug)]
pub enum EmbeddingError {
    /// The input text could not be processed.
    InvalidInput(String),
    /// An internal error occurred in the embedder.
    Internal(String),
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbeddingError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            EmbeddingError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for EmbeddingError {}

/// Trait for generating vector embeddings from text.
///
/// Implementations can be local (e.g. SimHash) or remote (e.g. OpenAI embeddings API).
/// All implementations must be `Send + Sync` for use across async tasks.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate a vector embedding for the given text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Batch embed multiple texts.
    ///
    /// Default implementation calls [`embed`](Embedder::embed) for each text sequentially.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Return the dimensionality of embeddings produced by this embedder.
    fn dimension(&self) -> usize;

    /// Return the name/identifier of this embedder (e.g. "hash", "openai-text-embedding-3-small").
    ///
    /// Used for logging and configuration. Default returns "unknown".
    fn name(&self) -> &str {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_error_display_invalid_input() {
        let err = EmbeddingError::InvalidInput("bad text".into());
        assert_eq!(format!("{err}"), "invalid input: bad text");
    }

    #[test]
    fn embedding_error_display_internal() {
        let err = EmbeddingError::Internal("something broke".into());
        assert_eq!(format!("{err}"), "internal error: something broke");
    }

    #[test]
    fn embedding_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(EmbeddingError::Internal("test".into()));
        assert!(err.to_string().contains("test"));
    }
}
