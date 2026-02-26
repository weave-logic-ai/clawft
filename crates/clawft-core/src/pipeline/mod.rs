//! 6-stage pluggable pipeline system.
//!
//! Stages: Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner

pub mod assembler;
pub mod classifier;
pub mod cost_tracker;
pub mod learner;
#[cfg(feature = "native")]
pub mod llm_adapter;
pub mod permissions;
pub mod rate_limiter;
pub mod router;
pub mod scorer;
pub mod tiered_router;
pub mod traits;
pub mod transport;
