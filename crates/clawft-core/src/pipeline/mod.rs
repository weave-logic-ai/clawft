//! 6-stage pluggable pipeline system.
//!
//! Stages: Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner

pub mod traits;
pub mod classifier;
pub mod router;
pub mod assembler;
pub mod transport;
pub mod scorer;
pub mod learner;
pub mod llm_adapter;
