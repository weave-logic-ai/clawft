//! 6-stage pluggable pipeline system.
//!
//! Stages: Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner

pub mod assembler;
pub mod classifier;
pub mod learner;
pub mod llm_adapter;
pub mod router;
pub mod scorer;
pub mod traits;
pub mod transport;
