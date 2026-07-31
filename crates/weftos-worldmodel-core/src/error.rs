//! Portable error kinds for world-model core traits.

use core::fmt;

use crate::latent::LATENT_DIM;

/// Result alias for world-model operations.
pub type WorldModelResult<T> = Result<T, WorldModelError>;

/// Errors that core traits may surface. Concrete impls may wrap these or
/// map into richer host errors at the facade boundary.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WorldModelError {
    /// Observation or payload `latent_dim` does not match local contract.
    LatentDimMismatch {
        /// Expected width (usually [`LATENT_DIM`]).
        expected: u16,
        /// Width present on the frame / tensor.
        got: u16,
    },
    /// Service / component unavailable (callers fall back to local ECC).
    Unavailable,
    /// Invalid observation bytes or framing.
    InvalidObservation,
    /// Planner could not produce a feasible plan.
    PlanFailed,
    /// Predictor step failed (model / numerical).
    PredictFailed,
    /// Encoder step failed.
    EncodeFailed,
    /// Recall / index miss or backend error.
    RecallFailed,
    /// Subscription limit or unknown handle.
    SubscriptionFailed,
    /// SIGReg health gate rejected the update.
    SigRegUnhealthy {
        /// Last reported health score in `[0, 1]`.
        health: f32,
    },
    /// Training surface could not run (empty buffer, insufficient samples).
    TrainingFailed,
    /// Streaming-merge checkpoint blocked by the four-condition AND gate.
    CheckpointRejected,
    /// Hot-swap staging / commit / rollback failed (unknown slot, empty).
    HotSwapFailed,
    /// RVF / model segment codec error (truncated, unknown type, bad digest).
    SegmentCodec,
    /// Generic message for stub / host mapping (alloc string optional later).
    Other(&'static str),
}

impl WorldModelError {
    /// Helper: mismatch against the v1 192-dim contract.
    pub const fn dim_mismatch(got: u16) -> Self {
        Self::LatentDimMismatch {
            expected: LATENT_DIM as u16,
            got,
        }
    }
}

impl fmt::Display for WorldModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LatentDimMismatch { expected, got } => {
                write!(f, "latent dim mismatch: expected {expected}, got {got}")
            }
            Self::Unavailable => write!(f, "world model unavailable"),
            Self::InvalidObservation => write!(f, "invalid observation"),
            Self::PlanFailed => write!(f, "plan failed"),
            Self::PredictFailed => write!(f, "predict failed"),
            Self::EncodeFailed => write!(f, "encode failed"),
            Self::RecallFailed => write!(f, "recall failed"),
            Self::SubscriptionFailed => write!(f, "subscription failed"),
            Self::SigRegUnhealthy { health } => {
                write!(f, "sigreg unhealthy: health={health}")
            }
            Self::TrainingFailed => write!(f, "training failed"),
            Self::CheckpointRejected => write!(f, "checkpoint rejected by AND gate"),
            Self::HotSwapFailed => write!(f, "model hot-swap failed"),
            Self::SegmentCodec => write!(f, "model segment codec error"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl core::error::Error for WorldModelError {}
