//! Scoring harness shell.
//!
//! ECC produces a [`ScenarioPrediction`]; the truth manifest produces a
//! [`CounterfactualTruth`]. [`score`] combines them into a [`ScenarioScore`].
//! Phase 0 wires directional accuracy, magnitude error, and CI coverage only;
//! later phases will layer edge recall/precision on top.

use crate::truth::CounterfactualTruth;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPrediction {
    pub scenario_id: String,
    pub predicted_delta: f64,
    pub predicted_delta_pct: f64,
    pub ci_80: [f64; 2],
    pub ci_95: [f64; 2],
    #[serde(default)]
    pub predicted_edges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioScore {
    pub scenario_id: String,
    pub actual_delta: f64,
    pub predicted_delta: f64,
    pub directional_accuracy: bool,
    pub magnitude_error: f64,
    pub within_ci_80: bool,
    pub within_ci_95: bool,
    pub passes_tier_gate: bool,
}

/// A prediction "passes" if direction matches AND the actual value falls
/// within the 80% CI. Magnitude-error thresholds come later.
pub fn score(prediction: &ScenarioPrediction, truth: &CounterfactualTruth) -> ScenarioScore {
    let actual = truth.delta;
    let predicted = prediction.predicted_delta;

    let directional_accuracy = if actual.abs() < 1e-6 && predicted.abs() < 1e-6 {
        true
    } else {
        actual.signum() == predicted.signum()
    };

    let magnitude_error = if actual.abs() > 1e-6 {
        (predicted - actual).abs() / actual.abs()
    } else {
        (predicted - actual).abs()
    };

    let within_ci_80 = actual >= prediction.ci_80[0] && actual <= prediction.ci_80[1];
    let within_ci_95 = actual >= prediction.ci_95[0] && actual <= prediction.ci_95[1];
    let passes_tier_gate = directional_accuracy && within_ci_80;

    ScenarioScore {
        scenario_id: prediction.scenario_id.clone(),
        actual_delta: actual,
        predicted_delta: predicted,
        directional_accuracy,
        magnitude_error,
        within_ci_80,
        within_ci_95,
        passes_tier_gate,
    }
}
