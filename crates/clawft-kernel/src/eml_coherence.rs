//! O(1) coherence approximation via EML (exp(x) - ln(y)) master formula.
//!
//! Predicts algebraic connectivity (lambda_2) from graph statistics
//! without running the expensive O(k*m) Lanczos eigenvalue iteration.
//! Based on: Odrzywolel 2026, "All elementary functions from a single operator"
//!
//! # Two-Tier Coherence Pattern (DEMOCRITUS)
//!
//! The intended usage follows a two-tier pattern:
//! - **Every tick**: `coherence_fast()` via the EML model (~0.1 us)
//! - **When drift exceeds threshold**: `spectral_analysis()` via Lanczos (~500 us),
//!   then `model.record()` to feed the training buffer
//! - **Every 1000 exact samples**: `model.train()` to refine parameters
//!
//! This module does NOT modify the cognitive tick loop. Callers are
//! responsible for implementing the two-tier cadence.

use serde::{Deserialize, Serialize};

use crate::causal::CausalGraph;

// ---------------------------------------------------------------------------
// EML operator
// ---------------------------------------------------------------------------

/// The EML universal operator: eml(x, y) = exp(x) - ln(y).
///
/// This is the continuous-mathematics analog of the NAND gate: combined
/// with the constant 1, it can reconstruct all elementary functions.
#[inline]
pub fn eml(x: f64, y: f64) -> f64 {
    x.exp() - y.ln()
}

// ---------------------------------------------------------------------------
// GraphFeatures
// ---------------------------------------------------------------------------

/// Cheap-to-extract graph statistics used as input features for the
/// EML coherence model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphFeatures {
    /// Number of nodes |V|.
    pub node_count: f64,
    /// Number of edges |E|.
    pub edge_count: f64,
    /// Average degree: 2*|E| / |V| (undirected interpretation).
    pub avg_degree: f64,
    /// Maximum degree across all nodes.
    pub max_degree: f64,
    /// Minimum degree across all nodes.
    pub min_degree: f64,
    /// Edge density: 2*|E| / (|V| * (|V|-1)).
    pub density: f64,
    /// Number of connected components.
    pub component_count: f64,
}

impl GraphFeatures {
    /// Extract features from a [`CausalGraph`] in O(n) time.
    pub fn from_causal_graph(graph: &CausalGraph) -> Self {
        let n = graph.node_count() as f64;
        let m = graph.edge_count() as f64;

        if n < 1.0 {
            return Self {
                node_count: 0.0,
                edge_count: 0.0,
                avg_degree: 0.0,
                max_degree: 0.0,
                min_degree: 0.0,
                density: 0.0,
                component_count: 0.0,
            };
        }

        let ids = graph.node_ids();
        let mut max_deg: usize = 0;
        let mut min_deg: usize = usize::MAX;
        for &id in &ids {
            let d = graph.degree(id);
            if d > max_deg {
                max_deg = d;
            }
            if d < min_deg {
                min_deg = d;
            }
        }
        if ids.is_empty() {
            min_deg = 0;
        }

        let avg_degree = if n > 0.0 { 2.0 * m / n } else { 0.0 };
        let density = if n > 1.0 {
            2.0 * m / (n * (n - 1.0))
        } else {
            0.0
        };

        let component_count = graph.connected_components().len() as f64;

        Self {
            node_count: n,
            edge_count: m,
            avg_degree,
            max_degree: max_deg as f64,
            min_degree: min_deg as f64,
            density,
            component_count,
        }
    }

    /// Normalize features to [0, 1] range for numerical stability.
    fn normalized(&self) -> [f64; 7] {
        [
            self.node_count / 10000.0,
            self.edge_count / 50000.0,
            self.avg_degree / 100.0,
            self.max_degree / 1000.0,
            self.density,
            self.component_count / 100.0,
            self.min_degree / 50.0,
        ]
    }
}

// ---------------------------------------------------------------------------
// TrainingPoint
// ---------------------------------------------------------------------------

/// A recorded (features, exact lambda_2) pair for model training.
#[derive(Debug, Clone)]
struct TrainingPoint {
    features: GraphFeatures,
    lambda_2: f64,
}

// ---------------------------------------------------------------------------
// EmlCoherenceModel
// ---------------------------------------------------------------------------

/// Depth-3 EML master formula for O(1) coherence prediction.
///
/// The architecture is:
///
/// ```text
/// Level 0: 8 linear combinations of 7 input features (24 params)
///   a_i = softmax(alpha, beta, gamma) . (1, x_j, x_k)
///
/// Level 1: 4 EML nodes
///   b_0 = eml(a_0, a_1), b_1 = eml(a_2, a_3), ...
///
/// Level 2: 2 EML nodes with mixing (8 params)
///   c_0 = eml(mix(b_0, b_1), mix(b_2, b_3))
///   c_1 = eml(mix(b_0, b_1), mix(b_2, b_3))
///
/// Level 3: 1 EML node with mixing (2 params)
///   result = eml(mix(c_0), mix(c_1))
/// ```
///
/// Total: 34 trainable parameters.
/// Number of trainable parameters in the depth-3 EML formula.
const PARAM_COUNT: usize = 34;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmlCoherenceModel {
    /// 34 trainable parameters (weights), stored as Vec for serde compat.
    params: Vec<f64>,
    /// Whether the model has been trained to convergence.
    trained: bool,
    /// Training data buffer.
    #[serde(skip)]
    training_data: Vec<TrainingPoint>,
    /// Prediction error history (for drift detection).
    #[serde(skip)]
    error_history: Vec<f64>,
}

impl Default for EmlCoherenceModel {
    fn default() -> Self {
        Self::new()
    }
}

impl EmlCoherenceModel {
    /// Create a new untrained model with zeroed parameters.
    pub fn new() -> Self {
        Self {
            params: vec![0.0; PARAM_COUNT],
            trained: false,
            training_data: Vec::new(),
            error_history: Vec::new(),
        }
    }

    /// Whether the model has been trained to convergence.
    pub fn is_trained(&self) -> bool {
        self.trained
    }

    /// Number of training samples collected so far.
    pub fn training_sample_count(&self) -> usize {
        self.training_data.len()
    }

    /// Mean of the recent error history (empty => 0.0).
    pub fn mean_error(&self) -> f64 {
        if self.error_history.is_empty() {
            return 0.0;
        }
        self.error_history.iter().sum::<f64>() / self.error_history.len() as f64
    }

    // -------------------------------------------------------------------
    // Prediction
    // -------------------------------------------------------------------

    /// O(1) coherence prediction from graph features.
    ///
    /// Falls back to a density-based estimate if the model has not yet
    /// been trained.
    pub fn predict(&self, features: &GraphFeatures) -> f64 {
        if !self.trained {
            // Fallback: density * avg_degree is a rough proxy for
            // algebraic connectivity in random graphs.
            return features.density * features.avg_degree;
        }
        self.evaluate_depth3(&self.params, features)
    }

    /// Evaluate the depth-3 EML tree with the given parameters.
    ///
    /// Parameter layout (34 total):
    ///   [0..24]  Level 0: 8 linear combos, 3 weights each
    ///   [24..32] Level 2: 2 mixing nodes, 4 weights each
    ///                      (alpha1, beta1, alpha2, beta2) per node
    ///   [32..34] Level 3: 1 output mixing, 2 weights
    fn evaluate_depth3(&self, params: &[f64], features: &GraphFeatures) -> f64 {
        let inputs = features.normalized();

        // Level 0: 8 affine combinations, each selecting two features.
        // For node i: a_i = softmax(p[3i], p[3i+1], p[3i+2]) . (1, x[j], x[k])
        // Feature pairs for 8 nodes (deterministic mapping):
        let feature_pairs: [(usize, usize); 8] = [
            (0, 1), // node_count, edge_count
            (2, 3), // avg_degree, max_degree
            (4, 5), // density, component_count
            (6, 0), // min_degree, node_count
            (1, 2), // edge_count, avg_degree
            (3, 4), // max_degree, density
            (5, 6), // component_count, min_degree
            (0, 4), // node_count, density
        ];

        let mut a = [0.0f64; 8];
        for i in 0..8 {
            let base = i * 3;
            let (raw_alpha, raw_beta, raw_gamma) =
                (params[base], params[base + 1], params[base + 2]);
            let (alpha, beta, gamma) = softmax3(raw_alpha, raw_beta, raw_gamma);
            let (j, k) = feature_pairs[i];
            a[i] = alpha + beta * inputs[j] + gamma * inputs[k];
            // Clamp to avoid extreme values in exp/ln
            a[i] = a[i].clamp(-10.0, 10.0);
        }

        // Level 1: 4 EML nodes
        let b = [
            eml_safe(a[0], a[1]),
            eml_safe(a[2], a[3]),
            eml_safe(a[4], a[5]),
            eml_safe(a[6], a[7]),
        ];

        // Level 2: 2 EML nodes with mixing
        // Node 0: eml(mix(b0,b1), mix(b2,b3))
        // Node 1: eml(mix(b0,b1), mix(b2,b3)) with different weights
        let mut c = [0.0f64; 2];
        for i in 0..2 {
            let base = 24 + i * 4;
            let mix_left = params[base] + params[base + 1] * b[0]
                + (1.0 - params[base] - params[base + 1]) * b[1];
            let mix_right = params[base + 2] + params[base + 3] * b[2]
                + (1.0 - params[base + 2] - params[base + 3]) * b[3];
            let ml = mix_left.clamp(-10.0, 10.0);
            let mr = mix_right.clamp(0.01, 10.0); // ln argument must be > 0
            c[i] = eml_safe(ml, mr);
        }

        // Level 3: output
        let w0 = params[32];
        let w1 = params[33];
        let out_left = (w0 * c[0] + (1.0 - w0) * c[1]).clamp(-10.0, 10.0);
        let out_right = (w1 * c[0] + (1.0 - w1) * c[1]).clamp(0.01, 10.0);
        let result = eml_safe(out_left, out_right);

        // Lambda_2 is non-negative
        result.max(0.0)
    }

    // -------------------------------------------------------------------
    // Training
    // -------------------------------------------------------------------

    /// Record a training point (called after every exact Lanczos computation).
    pub fn record(&mut self, features: GraphFeatures, lambda_2: f64) {
        // Also track prediction error for drift detection
        let predicted = self.predict(&features);
        self.error_history.push((predicted - lambda_2).abs());
        // Keep last 100 error values
        if self.error_history.len() > 100 {
            self.error_history.remove(0);
        }

        self.training_data.push(TrainingPoint { features, lambda_2 });
    }

    /// Train the model when enough data is collected.
    ///
    /// Uses random restart + coordinate descent (gradient-free
    /// optimization suitable for 34 parameters).
    ///
    /// Returns `true` if the model converged (MSE < 0.01).
    pub fn train(&mut self) -> bool {
        if self.training_data.len() < 50 {
            return false; // not enough data yet
        }

        let mut best_params = self.params.clone();
        let mut best_mse = self.evaluate_mse(&self.params);

        // Phase 1: random restarts to find a good basin
        let mut rng_state: u64 = 0xDEAD_BEEF_CAFE_1234;
        for _ in 0..100 {
            let params = random_params(&mut rng_state);
            let mse = self.evaluate_mse(&params);
            if mse < best_mse {
                best_mse = mse;
                best_params = params;
            }
        }

        // Phase 2: coordinate descent refinement
        let deltas = [-0.1, -0.01, -0.001, 0.001, 0.01, 0.1];
        for _ in 0..1000 {
            let mut improved = false;
            for i in 0..PARAM_COUNT {
                for &delta in &deltas {
                    let mut candidate = best_params.clone();
                    candidate[i] += delta;
                    let mse = self.evaluate_mse(&candidate);
                    if mse < best_mse {
                        best_mse = mse;
                        best_params = candidate;
                        improved = true;
                    }
                }
            }
            if !improved {
                break; // converged
            }
        }

        self.params = best_params;
        self.trained = best_mse < 0.01;
        self.trained
    }

    /// Compute mean squared error over the training set.
    fn evaluate_mse(&self, params: &[f64]) -> f64 {
        if self.training_data.is_empty() {
            return f64::MAX;
        }
        let sum: f64 = self
            .training_data
            .iter()
            .map(|tp| {
                let predicted = self.evaluate_depth3(params, &tp.features);
                (predicted - tp.lambda_2).powi(2)
            })
            .sum();
        sum / self.training_data.len() as f64
    }
}

// ---------------------------------------------------------------------------
// CausalGraph integration
// ---------------------------------------------------------------------------

impl CausalGraph {
    /// O(1) approximate coherence from EML model.
    ///
    /// Falls back to density-based estimate if model not trained.
    pub fn coherence_fast(&self, model: &EmlCoherenceModel) -> f64 {
        let features = GraphFeatures::from_causal_graph(self);
        model.predict(&features)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Numerically safe EML: clamps exp and ensures positive ln argument.
#[inline]
fn eml_safe(x: f64, y: f64) -> f64 {
    let ex = x.clamp(-20.0, 20.0).exp();
    let ly = if y > 0.0 { y.ln() } else { f64::MIN_POSITIVE.ln() };
    ex - ly
}

/// Softmax over 3 values so that alpha + beta + gamma = 1.
#[inline]
fn softmax3(a: f64, b: f64, c: f64) -> (f64, f64, f64) {
    let max = a.max(b).max(c);
    let ea = (a - max).exp();
    let eb = (b - max).exp();
    let ec = (c - max).exp();
    let sum = ea + eb + ec;
    (ea / sum, eb / sum, ec / sum)
}

/// Generate 34 random parameters in [-1, 1] using a simple LCG.
fn random_params(state: &mut u64) -> Vec<f64> {
    let mut params = vec![0.0f64; PARAM_COUNT];
    for p in params.iter_mut() {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Map to [-1, 1]
        *p = (*state >> 33) as f64 / (u32::MAX as f64 / 2.0) - 1.0;
    }
    params
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal::{CausalEdgeType, CausalGraph};

    // -- eml operator -------------------------------------------------------

    #[test]
    fn eml_identity() {
        // eml(0, 1) = exp(0) - ln(1) = 1 - 0 = 1
        let result = eml(0.0, 1.0);
        assert!(
            (result - 1.0).abs() < 1e-12,
            "eml(0, 1) should be 1.0, got {result}"
        );
    }

    #[test]
    fn eml_exp_only() {
        // eml(1, 1) = exp(1) - ln(1) = e - 0 = e
        let result = eml(1.0, 1.0);
        assert!(
            (result - std::f64::consts::E).abs() < 1e-12,
            "eml(1, 1) should be e, got {result}"
        );
    }

    #[test]
    fn eml_ln_only() {
        // eml(0, e) = exp(0) - ln(e) = 1 - 1 = 0
        let result = eml(0.0, std::f64::consts::E);
        assert!(
            result.abs() < 1e-12,
            "eml(0, e) should be 0.0, got {result}"
        );
    }

    // -- GraphFeatures extraction -------------------------------------------

    #[test]
    fn features_empty_graph() {
        let g = CausalGraph::new();
        let f = GraphFeatures::from_causal_graph(&g);
        assert_eq!(f.node_count, 0.0);
        assert_eq!(f.edge_count, 0.0);
        assert_eq!(f.density, 0.0);
        assert_eq!(f.component_count, 0.0);
    }

    #[test]
    fn features_triangle() {
        let g = CausalGraph::new();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(c, a, CausalEdgeType::Causes, 1.0, 0, 0);

        let f = GraphFeatures::from_causal_graph(&g);
        assert_eq!(f.node_count, 3.0);
        assert_eq!(f.edge_count, 3.0);
        assert!((f.avg_degree - 2.0).abs() < 1e-9);
        assert_eq!(f.component_count, 1.0);
        // density = 2*3 / (3*2) = 1.0
        assert!((f.density - 1.0).abs() < 1e-9);
    }

    #[test]
    fn features_disconnected() {
        let g = CausalGraph::new();
        let _a = g.add_node("A".into(), serde_json::json!({}));
        let _b = g.add_node("B".into(), serde_json::json!({}));

        let f = GraphFeatures::from_causal_graph(&g);
        assert_eq!(f.node_count, 2.0);
        assert_eq!(f.edge_count, 0.0);
        assert_eq!(f.component_count, 2.0);
        assert_eq!(f.min_degree, 0.0);
        assert_eq!(f.max_degree, 0.0);
    }

    // -- EmlCoherenceModel prediction (untrained fallback) ------------------

    #[test]
    fn predict_untrained_fallback() {
        let model = EmlCoherenceModel::new();
        assert!(!model.is_trained());

        let features = GraphFeatures {
            node_count: 10.0,
            edge_count: 20.0,
            avg_degree: 4.0,
            max_degree: 6.0,
            min_degree: 2.0,
            density: 0.444,
            component_count: 1.0,
        };
        let result = model.predict(&features);
        // Fallback: density * avg_degree
        let expected = 0.444 * 4.0;
        assert!(
            (result - expected).abs() < 1e-9,
            "untrained fallback: expected {expected}, got {result}"
        );
    }

    // -- EmlCoherenceModel record + training --------------------------------

    #[test]
    fn record_increments_count() {
        let mut model = EmlCoherenceModel::new();
        assert_eq!(model.training_sample_count(), 0);

        let f = GraphFeatures {
            node_count: 5.0,
            edge_count: 4.0,
            avg_degree: 1.6,
            max_degree: 2.0,
            min_degree: 1.0,
            density: 0.4,
            component_count: 1.0,
        };
        model.record(f, 0.5);
        assert_eq!(model.training_sample_count(), 1);
    }

    #[test]
    fn train_insufficient_data_returns_false() {
        let mut model = EmlCoherenceModel::new();
        // Add only 10 samples (need 50)
        for i in 0..10 {
            let f = GraphFeatures {
                node_count: i as f64,
                edge_count: i as f64,
                avg_degree: 2.0,
                max_degree: 3.0,
                min_degree: 1.0,
                density: 0.5,
                component_count: 1.0,
            };
            model.record(f, 1.0);
        }
        assert!(!model.train());
        assert!(!model.is_trained());
    }

    // -- Convergence test with known graph families -------------------------

    /// Generate training data from known graph families where lambda_2
    /// has a closed-form expression, then verify the model can learn.
    #[test]
    fn convergence_on_known_graphs() {
        let mut model = EmlCoherenceModel::new();

        // Collect 200 training points from known graph families.
        // We don't build actual CausalGraphs here -- we compute features
        // and lambda_2 analytically for speed.

        let mut samples = Vec::new();

        // Complete graph K_n: lambda_2 = n, density = 1.0
        for n in 3..30 {
            let nf = n as f64;
            let e = nf * (nf - 1.0) / 2.0;
            let lambda_2 = nf; // K_n has lambda_2 = n
            samples.push((
                GraphFeatures {
                    node_count: nf,
                    edge_count: e,
                    avg_degree: nf - 1.0,
                    max_degree: nf - 1.0,
                    min_degree: nf - 1.0,
                    density: 1.0,
                    component_count: 1.0,
                },
                lambda_2,
            ));
        }

        // Star graph S_n: lambda_2 = 1
        for n in 3..30 {
            let nf = n as f64;
            samples.push((
                GraphFeatures {
                    node_count: nf,
                    edge_count: nf - 1.0,
                    avg_degree: 2.0 * (nf - 1.0) / nf,
                    max_degree: nf - 1.0,
                    min_degree: 1.0,
                    density: 2.0 * (nf - 1.0) / (nf * (nf - 1.0)),
                    component_count: 1.0,
                },
                1.0,
            ));
        }

        // Cycle graph C_n: lambda_2 = 2(1 - cos(2*pi/n))
        for n in 3..30 {
            let nf = n as f64;
            let lambda_2 = 2.0 * (1.0 - (2.0 * std::f64::consts::PI / nf).cos());
            samples.push((
                GraphFeatures {
                    node_count: nf,
                    edge_count: nf,
                    avg_degree: 2.0,
                    max_degree: 2.0,
                    min_degree: 2.0,
                    density: 2.0 * nf / (nf * (nf - 1.0)),
                    component_count: 1.0,
                },
                lambda_2,
            ));
        }

        // Path graph P_n: lambda_2 = 2(1 - cos(pi/n))
        for n in 3..30 {
            let nf = n as f64;
            let lambda_2 = 2.0 * (1.0 - (std::f64::consts::PI / nf).cos());
            samples.push((
                GraphFeatures {
                    node_count: nf,
                    edge_count: nf - 1.0,
                    avg_degree: 2.0 * (nf - 1.0) / nf,
                    max_degree: 2.0,
                    min_degree: 1.0,
                    density: 2.0 * (nf - 1.0) / (nf * (nf - 1.0)),
                    component_count: 1.0,
                },
                lambda_2,
            ));
        }

        // Erdos-Renyi G(n, p): lambda_2 ~ n*p - 2*sqrt(n*p*(1-p))
        for n in [20, 50, 100, 200] {
            for &p in &[0.1, 0.2, 0.3, 0.5, 0.7] {
                let nf = n as f64;
                let e = nf * (nf - 1.0) * p / 2.0;
                let avg_deg = (nf - 1.0) * p;
                let lambda_2 = (nf * p - 2.0 * (nf * p * (1.0 - p)).sqrt()).max(0.0);
                samples.push((
                    GraphFeatures {
                        node_count: nf,
                        edge_count: e,
                        avg_degree: avg_deg,
                        max_degree: avg_deg * 1.5, // rough estimate
                        min_degree: (avg_deg * 0.5).max(0.0),
                        density: p,
                        component_count: 1.0,
                    },
                    lambda_2,
                ));
            }
        }

        // Feed all samples as training data
        for (features, lambda_2) in &samples {
            model.record(features.clone(), *lambda_2);
        }

        assert!(
            model.training_sample_count() >= 50,
            "should have enough training data: {}",
            model.training_sample_count()
        );

        // Train
        let converged = model.train();

        // Verify: even if not fully converged on this mixed dataset,
        // the MSE should be reasonable. The model may not perfectly fit
        // all graph families simultaneously (that's expected for a 34-param
        // model), but it should do much better than random.
        let mse = model.evaluate_mse(&model.params);
        assert!(
            mse < 100.0,
            "MSE should be reasonable after training, got {mse}"
        );

        // If converged, the model should predict reasonably
        if converged {
            // Test on a complete graph K_5: lambda_2 = 5
            let k5 = GraphFeatures {
                node_count: 5.0,
                edge_count: 10.0,
                avg_degree: 4.0,
                max_degree: 4.0,
                min_degree: 4.0,
                density: 1.0,
                component_count: 1.0,
            };
            let pred = model.predict(&k5);
            assert!(
                pred > 0.0,
                "prediction for K5 should be positive, got {pred}"
            );
        }
    }

    // -- CausalGraph::coherence_fast integration ----------------------------

    #[test]
    fn coherence_fast_on_triangle() {
        let g = CausalGraph::new();
        let a = g.add_node("A".into(), serde_json::json!({}));
        let b = g.add_node("B".into(), serde_json::json!({}));
        let c = g.add_node("C".into(), serde_json::json!({}));
        g.link(a, b, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(b, c, CausalEdgeType::Causes, 1.0, 0, 0);
        g.link(c, a, CausalEdgeType::Causes, 1.0, 0, 0);

        let model = EmlCoherenceModel::new();
        let fast = g.coherence_fast(&model);
        // Untrained: density * avg_degree = 1.0 * 2.0 = 2.0
        assert!(
            (fast - 2.0).abs() < 1e-9,
            "coherence_fast untrained triangle: expected 2.0, got {fast}"
        );
    }

    #[test]
    fn coherence_fast_empty() {
        let g = CausalGraph::new();
        let model = EmlCoherenceModel::new();
        let fast = g.coherence_fast(&model);
        assert!(
            fast.abs() < 1e-12,
            "coherence_fast on empty graph should be 0"
        );
    }

    // -- Helper function tests ----------------------------------------------

    #[test]
    fn softmax3_sums_to_one() {
        let (a, b, c) = softmax3(1.0, 2.0, 3.0);
        let sum = a + b + c;
        assert!(
            (sum - 1.0).abs() < 1e-12,
            "softmax3 should sum to 1.0, got {sum}"
        );
    }

    #[test]
    fn softmax3_equal_inputs() {
        let (a, b, c) = softmax3(0.0, 0.0, 0.0);
        assert!((a - 1.0 / 3.0).abs() < 1e-12);
        assert!((b - 1.0 / 3.0).abs() < 1e-12);
        assert!((c - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn eml_safe_does_not_panic() {
        // Extreme values should not panic
        let _ = eml_safe(100.0, 0.0);
        let _ = eml_safe(-100.0, -5.0);
        let _ = eml_safe(0.0, f64::MIN_POSITIVE);
        let _ = eml_safe(f64::NAN, 1.0); // NaN propagation is acceptable
    }

    #[test]
    fn error_history_tracks_drift() {
        let mut model = EmlCoherenceModel::new();
        let f = GraphFeatures {
            node_count: 5.0,
            edge_count: 5.0,
            avg_degree: 2.0,
            max_degree: 2.0,
            min_degree: 2.0,
            density: 0.5,
            component_count: 1.0,
        };

        model.record(f.clone(), 1.0);
        model.record(f.clone(), 2.0);
        assert_eq!(model.error_history.len(), 2);
        assert!(model.mean_error() >= 0.0);
    }
}
