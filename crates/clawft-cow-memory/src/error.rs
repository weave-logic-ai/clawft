//! Error type for `clawft-cow-memory`.
//!
//! `rvf_types::RvfError` does not implement `std::error::Error` (the crate
//! is `no_std`-compatible), so we cannot use thiserror's `#[from]`/`#[source]`
//! chaining on it directly -- that attribute requires the wrapped field to
//! implement the standard `Error` trait so `.source()` can be generated.
//! We wrap it with a plain variant + a hand-written `From` impl instead,
//! which only needs `Display` (which `RvfError` does implement).

use crate::node::CheckpointId;

/// Errors produced by `clawft-cow-memory`.
#[derive(Debug, thiserror::Error)]
pub enum CowMemoryError {
    /// A lower-level `rvf-runtime` operation failed.
    #[error("rvf-runtime error: {0}")]
    Rvf(rvf_types::RvfError),

    /// A filesystem operation (create dir, remove file, ...) failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A query or ingest vector did not match the store's configured
    /// dimension.
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    /// `rollback`/`fork`/`diff` referenced a `CheckpointId` that is not on
    /// this lineage's ancestor chain (already rolled back, promoted away,
    /// or from a different `BranchableMemory`).
    #[error("checkpoint not found in this lineage: {0:?}")]
    CheckpointNotFound(CheckpointId),

    /// `BranchableMemory::create` was called with `dimension == 0`.
    #[error("vector dimension must be non-zero")]
    InvalidDimension,

    /// `ingest` was called with mismatched parallel-array lengths.
    #[error("ingest item count mismatch: {vectors} vectors vs {ids} ids")]
    IngestArityMismatch { vectors: usize, ids: usize },
}

impl From<rvf_types::RvfError> for CowMemoryError {
    fn from(e: rvf_types::RvfError) -> Self {
        CowMemoryError::Rvf(e)
    }
}

pub type Result<T> = std::result::Result<T, CowMemoryError>;
