//! Prompt mutation strategies + GA population loop (GEPA / ADR-017).
//!
//! - **strategies** — string-level operators (rephrase, add examples,
//!   remove ineffective, emphasize) that do not require an LLM call.
//! - **ga** — deterministic genetic-algorithm population (selection,
//!   crossover, mutation, elitism) and disk persistence (WEFT-38).
//!
//! Production wiring: [`TrajectoryLearner`](crate::pipeline::learner::TrajectoryLearner)
//! sets `evolution_ready` after enough poor trajectories; pipeline stage
//! 3.5 (`apply_prompt_evolution`) calls
//! [`LearningBackend::evolve_prompt`](crate::pipeline::traits::LearningBackend::evolve_prompt),
//! which runs [`ga::evolve`] and returns the champion prompt to transport.

mod ga;
mod strategies;

pub use ga::{
    EvolutionResult, GaConfig, Population, PromptCandidate, evolve, select_evolved_prompt,
};
pub use strategies::{MutationStrategy, TrajectoryHint, auto_select_strategy, mutate_prompt};
