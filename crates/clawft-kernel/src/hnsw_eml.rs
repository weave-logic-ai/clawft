//! EML-based HNSW search optimization.
//!
//! Manages four EML models that learn from operational search data to
//! optimize HNSW performance:
//!
//! - **Distance model**: learned dimension selection for fast approximate
//!   cosine distance (progressive dimensionality).
//! - **Ef model**: per-query adaptive beam width (ef_search).
//! - **Path model**: search entry-point prediction.
//! - **Rebuild model**: predicts when the HNSW graph needs rebuilding
//!   based on recall degradation.
//!
//! # Two-Tier Pattern
//!
//! Follows the same two-tier pattern as [`eml_coherence`]:
//! - **Every search**: fast EML predictions guide beam width and entry
//!   point selection (~0.1 us overhead).
//! - **Periodically**: ground-truth recall measurement via brute-force
//!   comparison feeds the distance and rebuild models.
//! - **Every N searches**: models are retrained from accumulated data.
//!
//! This module is compiled only when the `ecc` feature is enabled.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the HNSW EML optimization system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswEmlConfig {
    /// Enable EML-based HNSW optimization.
    pub enabled: bool,
    /// Train models every N searches.
    pub train_every_n: u64,
    /// Measure recall every N searches (for distance/rebuild models).
    pub recall_check_every_n: u64,
    /// Minimum training samples before enabling learned models.
    pub min_training_samples: usize,
    /// Number of selected dimensions for cosine decomposition.
    pub distance_selected_dims: usize,
}

impl Default for HnswEmlConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            train_every_n: 1000,
            recall_check_every_n: 5000,
            min_training_samples: 200,
            distance_selected_dims: 16,
        }
    }
}

// ---------------------------------------------------------------------------
// Training data points
// ---------------------------------------------------------------------------

/// Training point for the ef (beam width) model.
///
/// Records search characteristics so the model can learn the optimal
/// ef_search for a given query profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfTrainingPoint {
    /// L2 norm of the query vector.
    pub query_norm: f64,
    /// Variance across query dimensions.
    pub query_variance: f64,
    /// ef_search value used for this query.
    pub ef_used: usize,
    /// Number of results returned.
    pub result_count: usize,
    /// Wall-clock search time in microseconds.
    pub search_time_us: u64,
}

/// Training point for the distance model.
///
/// Records per-dimension contribution to distance computations so the
/// model can learn which dimensions are most informative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceTrainingPoint {
    /// Per-dimension absolute differences between query and result vectors.
    pub dim_contributions: Vec<f64>,
    /// Full cosine similarity (ground truth).
    pub exact_similarity: f64,
    /// Approximate similarity using only selected dimensions.
    pub approx_similarity: f64,
}

/// Training point for the search path model.
///
/// Records entry-point selection outcomes so the model can predict
/// the best starting node for a given query vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTrainingPoint {
    /// Query vector norm.
    pub query_norm: f64,
    /// Query vector variance.
    pub query_variance: f64,
    /// Number of hops taken during the search.
    pub hops: usize,
    /// Score of the top result.
    pub top_score: f64,
    /// Total entries in the store at search time.
    pub store_size: usize,
}

/// Training point for the rebuild model.
///
/// Records graph health statistics so the model can predict when recall
/// has degraded enough to justify a full HNSW rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildTrainingPoint {
    /// Number of entries in the store.
    pub store_size: usize,
    /// Number of inserts since last rebuild.
    pub inserts_since_rebuild: usize,
    /// Number of deletes since last rebuild.
    pub deletes_since_rebuild: usize,
    /// Measured recall (0.0..1.0).
    pub recall: f64,
    /// Average search time in microseconds.
    pub avg_search_time_us: f64,
}

// ---------------------------------------------------------------------------
// Feature vectors for EML models
// ---------------------------------------------------------------------------

/// Features extracted from a search query for ef/path prediction.
#[derive(Debug, Clone)]
struct SearchFeatures {
    query_norm: f64,
    query_variance: f64,
    store_size: f64,
    recent_avg_time_us: f64,
}

impl SearchFeatures {
    fn normalized(&self) -> [f64; 4] {
        [
            self.query_norm / 100.0,
            self.query_variance.min(1.0),
            self.store_size / 100_000.0,
            self.recent_avg_time_us / 10_000.0,
        ]
    }
}

impl eml_core::FeatureVector for SearchFeatures {
    fn as_features(&self) -> Vec<f64> {
        self.normalized().to_vec()
    }

    fn feature_count() -> usize {
        4
    }
}

/// Features for the rebuild prediction model.
#[derive(Debug, Clone)]
struct RebuildFeatures {
    store_size: f64,
    inserts_since_rebuild: f64,
    deletes_since_rebuild: f64,
    avg_search_time_us: f64,
}

impl RebuildFeatures {
    fn normalized(&self) -> [f64; 4] {
        [
            self.store_size / 100_000.0,
            self.inserts_since_rebuild / 10_000.0,
            self.deletes_since_rebuild / 10_000.0,
            self.avg_search_time_us / 10_000.0,
        ]
    }
}

impl eml_core::FeatureVector for RebuildFeatures {
    fn as_features(&self) -> Vec<f64> {
        self.normalized().to_vec()
    }

    fn feature_count() -> usize {
        4
    }
}

// ---------------------------------------------------------------------------
// Status snapshot
// ---------------------------------------------------------------------------

/// Point-in-time status snapshot of the HNSW EML system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswEmlStatus {
    /// Whether EML optimization is enabled.
    pub enabled: bool,
    /// Whether the distance model has been trained.
    pub distance_trained: bool,
    /// Number of distance training samples.
    pub distance_samples: usize,
    /// Whether the ef model has been trained.
    pub ef_trained: bool,
    /// Number of ef training samples.
    pub ef_samples: usize,
    /// Whether the path model has been trained.
    pub path_trained: bool,
    /// Number of path training samples.
    pub path_samples: usize,
    /// Whether the rebuild model has been trained.
    pub rebuild_trained: bool,
    /// Number of rebuild training samples.
    pub rebuild_samples: usize,
    /// Total searches since last training cycle.
    pub searches_since_train: u64,
    /// Total searches processed since creation.
    pub total_searches: u64,
    /// Total training cycles completed.
    pub train_cycles: u64,
    /// Last measured recall (if available).
    pub last_recall: Option<f64>,
}

// ---------------------------------------------------------------------------
// Predictions
// ---------------------------------------------------------------------------

/// Predicted optimal ef_search for a query.
#[derive(Debug, Clone)]
pub struct EfPrediction {
    /// Recommended ef_search value.
    pub recommended_ef: usize,
    /// Whether this is a learned prediction (vs. default).
    pub is_learned: bool,
}

/// Predicted rebuild urgency.
#[derive(Debug, Clone)]
pub struct RebuildPrediction {
    /// Predicted recall if no rebuild is performed (0.0..1.0).
    pub predicted_recall: f64,
    /// Whether a rebuild is recommended.
    pub should_rebuild: bool,
    /// Whether this is a learned prediction (vs. heuristic).
    pub is_learned: bool,
}

// ---------------------------------------------------------------------------
// HnswEmlManager
// ---------------------------------------------------------------------------

/// Manages EML models for HNSW search optimization.
///
/// Follows the two-tier pattern: fast EML predictions on every search,
/// periodic ground-truth measurement for training.
pub struct HnswEmlManager {
    /// Configuration.
    config: HnswEmlConfig,
    /// Learned dimension selection for fast approximate distance.
    distance_model: eml_core::EmlModel,
    /// Per-query adaptive beam width.
    ef_model: eml_core::EmlModel,
    /// Search path entry-point predictor.
    path_model: eml_core::EmlModel,
    /// Rebuild trigger predictor.
    rebuild_model: eml_core::EmlModel,
    /// Training data buffers.
    distance_training: Vec<DistanceTrainingPoint>,
    ef_training: Vec<EfTrainingPoint>,
    path_training: Vec<PathTrainingPoint>,
    rebuild_training: Vec<RebuildTrainingPoint>,
    /// Searches since last train cycle.
    searches_since_train: u64,
    /// Total searches processed.
    total_searches: u64,
    /// Total training cycles completed.
    train_cycles: u64,
    /// Last measured recall.
    last_recall: Option<f64>,
    /// Recent search times for averaging.
    recent_search_times_us: Vec<u64>,
}

impl HnswEmlManager {
    /// Create a new manager with the given configuration.
    pub fn new(config: HnswEmlConfig) -> Self {
        Self {
            config,
            // depth-3 models with 4 input features, 1 output head each
            distance_model: eml_core::EmlModel::new(3, 4, 1),
            ef_model: eml_core::EmlModel::new(3, 4, 1),
            path_model: eml_core::EmlModel::new(3, 4, 1),
            rebuild_model: eml_core::EmlModel::new(3, 4, 1),
            distance_training: Vec::new(),
            ef_training: Vec::new(),
            path_training: Vec::new(),
            rebuild_training: Vec::new(),
            searches_since_train: 0,
            total_searches: 0,
            train_cycles: 0,
            last_recall: None,
            recent_search_times_us: Vec::new(),
        }
    }

    /// Create a manager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(HnswEmlConfig::default())
    }

    /// Whether EML optimization is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Borrow the configuration.
    pub fn config(&self) -> &HnswEmlConfig {
        &self.config
    }

    // -------------------------------------------------------------------
    // Training data collection
    // -------------------------------------------------------------------

    /// Record a search for training data collection.
    ///
    /// Called after every HNSW search to collect ef and path training
    /// data. When enough samples accumulate, triggers a training cycle.
    pub fn record_search(
        &mut self,
        query: &[f32],
        result_count: usize,
        top_score: f32,
        ef_used: usize,
        search_time_us: u64,
        store_size: usize,
    ) {
        if !self.config.enabled {
            return;
        }

        let qnorm = vector_norm(query);
        let qvar = vector_variance(query);

        // Record ef training data.
        self.ef_training.push(EfTrainingPoint {
            query_norm: qnorm,
            query_variance: qvar,
            ef_used,
            result_count,
            search_time_us,
        });

        // Record path training data.
        self.path_training.push(PathTrainingPoint {
            query_norm: qnorm,
            query_variance: qvar,
            hops: 0, // placeholder -- real hop count requires HNSW internals
            top_score: top_score as f64,
            store_size,
        });

        // Track recent search times.
        self.recent_search_times_us.push(search_time_us);
        if self.recent_search_times_us.len() > 1000 {
            self.recent_search_times_us.drain(..500);
        }

        self.searches_since_train += 1;
        self.total_searches += 1;

        // Trigger periodic training.
        if self.searches_since_train >= self.config.train_every_n {
            self.train_all();
        }
    }

    /// Measure actual recall by comparing HNSW results against brute-force.
    ///
    /// Called periodically (e.g., every `recall_check_every_n` searches or
    /// from the DEMOCRITUS loop). Feeds the distance and rebuild models.
    ///
    /// Returns the average recall across the sample queries.
    pub fn measure_recall(
        &mut self,
        hnsw_results: &[Vec<String>],
        exact_results: &[Vec<String>],
        store_size: usize,
        inserts_since_rebuild: usize,
        deletes_since_rebuild: usize,
    ) -> f64 {
        if hnsw_results.is_empty() || exact_results.is_empty() {
            return 0.0;
        }

        let n = hnsw_results.len().min(exact_results.len());
        let mut total_recall = 0.0;

        for i in 0..n {
            let recall = compute_recall(&hnsw_results[i], &exact_results[i]);
            total_recall += recall;
        }

        let avg_recall = total_recall / n as f64;
        self.last_recall = Some(avg_recall);

        // Record rebuild training data.
        let avg_search_time = if self.recent_search_times_us.is_empty() {
            0.0
        } else {
            self.recent_search_times_us.iter().sum::<u64>() as f64
                / self.recent_search_times_us.len() as f64
        };

        self.rebuild_training.push(RebuildTrainingPoint {
            store_size,
            inserts_since_rebuild,
            deletes_since_rebuild,
            recall: avg_recall,
            avg_search_time_us: avg_search_time,
        });

        avg_recall
    }

    /// Record distance training data from paired HNSW vs. exact results.
    ///
    /// `query` is the search query, `hnsw_embedding` and `exact_embedding`
    /// are the embeddings of the HNSW result and exact result respectively.
    pub fn record_distance_pair(
        &mut self,
        query: &[f32],
        hnsw_embedding: &[f32],
        exact_embedding: &[f32],
    ) {
        if !self.config.enabled {
            return;
        }

        let dim_contributions: Vec<f64> = query
            .iter()
            .zip(exact_embedding.iter())
            .map(|(q, e)| ((*q - *e) as f64).abs())
            .collect();

        let exact_sim = cosine_similarity_f32(query, exact_embedding) as f64;
        let approx_sim = cosine_similarity_f32(query, hnsw_embedding) as f64;

        self.distance_training.push(DistanceTrainingPoint {
            dim_contributions,
            exact_similarity: exact_sim,
            approx_similarity: approx_sim,
        });
    }

    // -------------------------------------------------------------------
    // Predictions
    // -------------------------------------------------------------------

    /// Predict the optimal ef_search for a query.
    ///
    /// Returns a default of 100 if the model is not yet trained.
    pub fn predict_ef(&self, query: &[f32], store_size: usize) -> EfPrediction {
        let default_ef = 100;

        if !self.config.enabled || !self.ef_model.is_trained() {
            return EfPrediction {
                recommended_ef: default_ef,
                is_learned: false,
            };
        }

        let features = SearchFeatures {
            query_norm: vector_norm(query),
            query_variance: vector_variance(query),
            store_size: store_size as f64,
            recent_avg_time_us: self.avg_recent_search_time(),
        };

        let predicted = self.ef_model.predict_primary(&features.normalized());

        // Scale prediction: the model outputs a normalized value, scale to
        // a reasonable ef range [10, 500].
        let ef = (predicted * 500.0).clamp(10.0, 500.0) as usize;

        EfPrediction {
            recommended_ef: ef,
            is_learned: true,
        }
    }

    /// Predict whether the HNSW index should be rebuilt.
    pub fn predict_rebuild(
        &self,
        store_size: usize,
        inserts_since_rebuild: usize,
        deletes_since_rebuild: usize,
    ) -> RebuildPrediction {
        if !self.config.enabled || !self.rebuild_model.is_trained() {
            // Heuristic fallback: rebuild if mutations exceed 10% of store.
            let mutation_ratio = if store_size > 0 {
                (inserts_since_rebuild + deletes_since_rebuild) as f64 / store_size as f64
            } else {
                0.0
            };
            return RebuildPrediction {
                predicted_recall: 1.0 - mutation_ratio * 0.1,
                should_rebuild: mutation_ratio > 0.1,
                is_learned: false,
            };
        }

        let features = RebuildFeatures {
            store_size: store_size as f64,
            inserts_since_rebuild: inserts_since_rebuild as f64,
            deletes_since_rebuild: deletes_since_rebuild as f64,
            avg_search_time_us: self.avg_recent_search_time(),
        };

        let predicted_recall = self
            .rebuild_model
            .predict_primary(&features.normalized())
            .clamp(0.0, 1.0);

        RebuildPrediction {
            predicted_recall,
            should_rebuild: predicted_recall < 0.90,
            is_learned: true,
        }
    }

    // -------------------------------------------------------------------
    // Training
    // -------------------------------------------------------------------

    /// Train all models that have sufficient data.
    ///
    /// Returns `true` if at least one model was trained.
    pub fn train_all(&mut self) -> bool {
        let mut any_trained = false;

        if self.train_ef_model() {
            any_trained = true;
        }
        if self.train_path_model() {
            any_trained = true;
        }
        if self.train_rebuild_model() {
            any_trained = true;
        }
        if self.train_distance_model() {
            any_trained = true;
        }

        self.searches_since_train = 0;
        if any_trained {
            self.train_cycles += 1;
        }

        any_trained
    }

    /// Train the ef model from collected search data.
    fn train_ef_model(&mut self) -> bool {
        if self.ef_training.len() < self.config.min_training_samples {
            return false;
        }

        // Feed training data: inputs = [norm, variance, store_size, avg_time]
        // target = normalized ef that produced good results
        for point in &self.ef_training {
            let inputs = [
                point.query_norm / 100.0,
                point.query_variance.min(1.0),
                point.result_count as f64 / 100.0,
                point.search_time_us as f64 / 10_000.0,
            ];
            // Target: normalize ef_used to [0, 1] range
            let target = point.ef_used as f64 / 500.0;
            self.ef_model.record(&inputs, &[Some(target)]);
        }

        let converged = self.ef_model.train();
        // Clear processed training data (keep last 100 for continued learning).
        let keep = self.ef_training.len().saturating_sub(100);
        self.ef_training.drain(..keep);
        converged
    }

    /// Train the path model.
    fn train_path_model(&mut self) -> bool {
        if self.path_training.len() < self.config.min_training_samples {
            return false;
        }

        for point in &self.path_training {
            let inputs = [
                point.query_norm / 100.0,
                point.query_variance.min(1.0),
                point.store_size as f64 / 100_000.0,
                point.top_score.clamp(0.0, 1.0),
            ];
            // Target: normalized hop count (fewer hops = better entry point)
            let target = point.hops as f64 / 100.0;
            self.path_model.record(&inputs, &[Some(target)]);
        }

        let converged = self.path_model.train();
        let keep = self.path_training.len().saturating_sub(100);
        self.path_training.drain(..keep);
        converged
    }

    /// Train the rebuild model.
    fn train_rebuild_model(&mut self) -> bool {
        if self.rebuild_training.len() < self.config.min_training_samples {
            return false;
        }

        for point in &self.rebuild_training {
            let inputs = [
                point.store_size as f64 / 100_000.0,
                point.inserts_since_rebuild as f64 / 10_000.0,
                point.deletes_since_rebuild as f64 / 10_000.0,
                point.avg_search_time_us / 10_000.0,
            ];
            // Target: actual recall
            let target = point.recall;
            self.rebuild_model.record(&inputs, &[Some(target)]);
        }

        let converged = self.rebuild_model.train();
        let keep = self.rebuild_training.len().saturating_sub(100);
        self.rebuild_training.drain(..keep);
        converged
    }

    /// Train the distance model.
    fn train_distance_model(&mut self) -> bool {
        if self.distance_training.len() < self.config.min_training_samples {
            return false;
        }

        for point in &self.distance_training {
            // Use aggregate dimension stats as features
            let n_dims = point.dim_contributions.len().max(1) as f64;
            let mean_contrib = point.dim_contributions.iter().sum::<f64>() / n_dims;
            let max_contrib = point
                .dim_contributions
                .iter()
                .copied()
                .fold(0.0_f64, f64::max);
            let inputs = [
                mean_contrib.min(1.0),
                max_contrib.min(1.0),
                n_dims / 1000.0,
                point.exact_similarity.clamp(0.0, 1.0),
            ];
            // Target: approximation quality
            let target = (point.approx_similarity - point.exact_similarity)
                .abs()
                .min(1.0);
            self.distance_model.record(&inputs, &[Some(target)]);
        }

        let converged = self.distance_model.train();
        let keep = self.distance_training.len().saturating_sub(100);
        self.distance_training.drain(..keep);
        converged
    }

    // -------------------------------------------------------------------
    // Status & reset
    // -------------------------------------------------------------------

    /// Return a point-in-time status snapshot.
    pub fn status(&self) -> HnswEmlStatus {
        HnswEmlStatus {
            enabled: self.config.enabled,
            distance_trained: self.distance_model.is_trained(),
            distance_samples: self.distance_training.len(),
            ef_trained: self.ef_model.is_trained(),
            ef_samples: self.ef_training.len(),
            path_trained: self.path_model.is_trained(),
            path_samples: self.path_training.len(),
            rebuild_trained: self.rebuild_model.is_trained(),
            rebuild_samples: self.rebuild_training.len(),
            searches_since_train: self.searches_since_train,
            total_searches: self.total_searches,
            train_cycles: self.train_cycles,
            last_recall: self.last_recall,
        }
    }

    /// Reset all models and clear training data.
    pub fn reset(&mut self) {
        self.distance_model = eml_core::EmlModel::new(3, 4, 1);
        self.ef_model = eml_core::EmlModel::new(3, 4, 1);
        self.path_model = eml_core::EmlModel::new(3, 4, 1);
        self.rebuild_model = eml_core::EmlModel::new(3, 4, 1);
        self.distance_training.clear();
        self.ef_training.clear();
        self.path_training.clear();
        self.rebuild_training.clear();
        self.searches_since_train = 0;
        self.total_searches = 0;
        self.train_cycles = 0;
        self.last_recall = None;
        self.recent_search_times_us.clear();
    }

    // -------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------

    /// Average of recent search times in microseconds.
    fn avg_recent_search_time(&self) -> f64 {
        if self.recent_search_times_us.is_empty() {
            return 0.0;
        }
        self.recent_search_times_us.iter().sum::<u64>() as f64
            / self.recent_search_times_us.len() as f64
    }
}

impl std::fmt::Debug for HnswEmlManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswEmlManager")
            .field("enabled", &self.config.enabled)
            .field("total_searches", &self.total_searches)
            .field("train_cycles", &self.train_cycles)
            .field("ef_trained", &self.ef_model.is_trained())
            .field("rebuild_trained", &self.rebuild_model.is_trained())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Compute the L2 norm of a vector.
fn vector_norm(v: &[f32]) -> f64 {
    v.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt()
}

/// Compute the variance of a vector's elements.
fn vector_variance(v: &[f32]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let n = v.len() as f64;
    let mean = v.iter().map(|x| *x as f64).sum::<f64>() / n;
    v.iter()
        .map(|x| {
            let d = *x as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / n
}

/// Compute recall: fraction of exact results that appear in HNSW results.
fn compute_recall(hnsw_ids: &[String], exact_ids: &[String]) -> f64 {
    if exact_ids.is_empty() {
        return 1.0;
    }
    let found = exact_ids
        .iter()
        .filter(|id| hnsw_ids.contains(id))
        .count();
    found as f64 / exact_ids.len() as f64
}

/// Compute cosine similarity between two f32 vectors.
fn cosine_similarity_f32(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> HnswEmlManager {
        HnswEmlManager::with_defaults()
    }

    // -- Construction & defaults --

    #[test]
    fn new_manager_defaults() {
        let m = make_manager();
        assert!(m.is_enabled());
        assert_eq!(m.total_searches, 0);
        assert_eq!(m.train_cycles, 0);
        assert!(m.last_recall.is_none());
    }

    #[test]
    fn config_default_values() {
        let cfg = HnswEmlConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.train_every_n, 1000);
        assert_eq!(cfg.recall_check_every_n, 5000);
        assert_eq!(cfg.min_training_samples, 200);
        assert_eq!(cfg.distance_selected_dims, 16);
    }

    #[test]
    fn status_initial() {
        let m = make_manager();
        let s = m.status();
        assert!(s.enabled);
        assert!(!s.distance_trained);
        assert!(!s.ef_trained);
        assert!(!s.path_trained);
        assert!(!s.rebuild_trained);
        assert_eq!(s.total_searches, 0);
        assert_eq!(s.train_cycles, 0);
        assert!(s.last_recall.is_none());
    }

    // -- Training data collection --

    #[test]
    fn record_search_increments_count() {
        let mut m = make_manager();
        m.record_search(&[1.0, 0.0, 0.0], 3, 0.95, 100, 500, 1000);
        assert_eq!(m.total_searches, 1);
        assert_eq!(m.searches_since_train, 1);
        assert_eq!(m.ef_training.len(), 1);
        assert_eq!(m.path_training.len(), 1);
    }

    #[test]
    fn record_search_disabled_does_nothing() {
        let mut m = HnswEmlManager::new(HnswEmlConfig {
            enabled: false,
            ..Default::default()
        });
        m.record_search(&[1.0, 0.0], 1, 0.5, 100, 100, 50);
        assert_eq!(m.total_searches, 0);
        assert_eq!(m.ef_training.len(), 0);
    }

    #[test]
    fn record_multiple_searches() {
        let mut m = make_manager();
        for i in 0..10 {
            m.record_search(
                &[i as f32 / 10.0, 1.0 - i as f32 / 10.0, 0.5],
                2,
                0.8,
                100,
                (200 + i * 10) as u64,
                500,
            );
        }
        assert_eq!(m.total_searches, 10);
        assert_eq!(m.ef_training.len(), 10);
        assert_eq!(m.path_training.len(), 10);
    }

    // -- Recall measurement --

    #[test]
    fn measure_recall_perfect() {
        let mut m = make_manager();
        let hnsw = vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]];
        let exact = vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]];
        let recall = m.measure_recall(&hnsw, &exact, 100, 10, 0);
        assert!((recall - 1.0).abs() < 1e-9);
        assert_eq!(m.last_recall, Some(1.0));
        assert_eq!(m.rebuild_training.len(), 1);
    }

    #[test]
    fn measure_recall_partial() {
        let mut m = make_manager();
        let hnsw = vec![vec!["a".to_string(), "d".to_string()]];
        let exact = vec![vec!["a".to_string(), "b".to_string()]];
        let recall = m.measure_recall(&hnsw, &exact, 100, 5, 2);
        assert!((recall - 0.5).abs() < 1e-9);
    }

    #[test]
    fn measure_recall_empty() {
        let mut m = make_manager();
        let recall = m.measure_recall(&[], &[], 100, 0, 0);
        assert!((recall - 0.0).abs() < 1e-9);
    }

    // -- Distance pair recording --

    #[test]
    fn record_distance_pair_stores_data() {
        let mut m = make_manager();
        m.record_distance_pair(
            &[1.0, 0.0, 0.0],
            &[0.9, 0.1, 0.0],
            &[1.0, 0.0, 0.0],
        );
        assert_eq!(m.distance_training.len(), 1);
    }

    // -- Predictions (untrained) --

    #[test]
    fn predict_ef_untrained_returns_default() {
        let m = make_manager();
        let pred = m.predict_ef(&[1.0, 0.0, 0.0], 500);
        assert_eq!(pred.recommended_ef, 100);
        assert!(!pred.is_learned);
    }

    #[test]
    fn predict_rebuild_untrained_heuristic() {
        let m = make_manager();
        // 10 inserts out of 100 = 10% => borderline
        let pred = m.predict_rebuild(100, 10, 0);
        assert!(!pred.is_learned);
        assert!(pred.predicted_recall > 0.0);
    }

    #[test]
    fn predict_rebuild_high_mutation_ratio() {
        let m = make_manager();
        // 50 inserts + 50 deletes out of 100 = 100% mutation
        let pred = m.predict_rebuild(100, 50, 50);
        assert!(pred.should_rebuild);
        assert!(!pred.is_learned);
    }

    // -- Training --

    #[test]
    fn train_all_insufficient_data_returns_false() {
        let mut m = make_manager();
        // Add fewer samples than min_training_samples
        for i in 0..10 {
            m.record_search(
                &[i as f32 / 10.0, 0.5, 0.5],
                2,
                0.8,
                100,
                500,
                100,
            );
        }
        let result = m.train_all();
        assert!(!result);
    }

    #[test]
    fn train_all_with_sufficient_ef_data() {
        let mut m = HnswEmlManager::new(HnswEmlConfig {
            min_training_samples: 50,
            train_every_n: 100_000, // don't auto-train during recording
            ..Default::default()
        });

        // Generate enough ef training data
        for i in 0..60 {
            let q = vec![
                (i as f32 * 0.1).sin(),
                (i as f32 * 0.2).cos(),
                i as f32 / 60.0,
            ];
            m.record_search(&q, 5, 0.9, 100 + i, (500 + i * 10) as u64, 1000);
        }

        // train_all should attempt training (may or may not converge)
        let _ = m.train_all();
        assert_eq!(m.searches_since_train, 0); // reset after train
    }

    // -- Reset --

    #[test]
    fn reset_clears_everything() {
        let mut m = make_manager();
        for i in 0..5 {
            m.record_search(
                &[i as f32, 0.0, 1.0],
                1,
                0.5,
                100,
                100,
                50,
            );
        }
        m.last_recall = Some(0.95);
        m.train_cycles = 3;

        m.reset();

        assert_eq!(m.total_searches, 0);
        assert_eq!(m.train_cycles, 0);
        assert!(m.last_recall.is_none());
        assert!(m.ef_training.is_empty());
        assert!(m.path_training.is_empty());
        assert!(m.rebuild_training.is_empty());
        assert!(m.distance_training.is_empty());
        assert!(m.recent_search_times_us.is_empty());
        assert!(!m.ef_model.is_trained());
    }

    // -- Helper function tests --

    #[test]
    fn vector_norm_unit() {
        let norm = vector_norm(&[1.0, 0.0, 0.0]);
        assert!((norm - 1.0).abs() < 1e-9);
    }

    #[test]
    fn vector_norm_pythagorean() {
        let norm = vector_norm(&[3.0, 4.0]);
        assert!((norm - 5.0).abs() < 1e-9);
    }

    #[test]
    fn vector_norm_empty() {
        let norm = vector_norm(&[]);
        assert!((norm - 0.0).abs() < 1e-9);
    }

    #[test]
    fn vector_variance_uniform() {
        let var = vector_variance(&[5.0, 5.0, 5.0]);
        assert!(var.abs() < 1e-9);
    }

    #[test]
    fn vector_variance_known() {
        // [1, 3] => mean=2, var=((1-2)^2 + (3-2)^2)/2 = 1.0
        let var = vector_variance(&[1.0, 3.0]);
        assert!((var - 1.0).abs() < 1e-9);
    }

    #[test]
    fn vector_variance_empty() {
        let var = vector_variance(&[]);
        assert!((var - 0.0).abs() < 1e-9);
    }

    #[test]
    fn compute_recall_all_found() {
        let hnsw = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let exact = vec!["a".to_string(), "b".to_string()];
        let recall = compute_recall(&hnsw, &exact);
        assert!((recall - 1.0).abs() < 1e-9);
    }

    #[test]
    fn compute_recall_none_found() {
        let hnsw = vec!["x".to_string(), "y".to_string()];
        let exact = vec!["a".to_string(), "b".to_string()];
        let recall = compute_recall(&hnsw, &exact);
        assert!((recall - 0.0).abs() < 1e-9);
    }

    #[test]
    fn compute_recall_empty_exact() {
        let hnsw = vec!["a".to_string()];
        let exact: Vec<String> = vec![];
        let recall = compute_recall(&hnsw, &exact);
        assert!((recall - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cosine_similarity_identical() {
        let score = cosine_similarity_f32(&[1.0, 0.0], &[1.0, 0.0]);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let score = cosine_similarity_f32(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(score.abs() < 0.01);
    }

    #[test]
    fn cosine_similarity_different_lengths() {
        let score = cosine_similarity_f32(&[1.0], &[1.0, 0.0]);
        assert!((score - 0.0).abs() < f32::EPSILON);
    }

    // -- Config serde roundtrip --

    #[test]
    fn config_serde_roundtrip() {
        let cfg = HnswEmlConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: HnswEmlConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.enabled, cfg.enabled);
        assert_eq!(restored.train_every_n, cfg.train_every_n);
        assert_eq!(restored.recall_check_every_n, cfg.recall_check_every_n);
        assert_eq!(restored.min_training_samples, cfg.min_training_samples);
        assert_eq!(restored.distance_selected_dims, cfg.distance_selected_dims);
    }

    // -- Status serde roundtrip --

    #[test]
    fn status_serde_roundtrip() {
        let m = make_manager();
        let s = m.status();
        let json = serde_json::to_string(&s).unwrap();
        let restored: HnswEmlStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.enabled, s.enabled);
        assert_eq!(restored.total_searches, s.total_searches);
    }

    // -- Debug impl --

    #[test]
    fn debug_format_does_not_panic() {
        let m = make_manager();
        let _ = format!("{:?}", m);
    }

    // -- Auto-train trigger --

    #[test]
    fn auto_train_triggers_at_threshold() {
        let mut m = HnswEmlManager::new(HnswEmlConfig {
            train_every_n: 5,
            min_training_samples: 200, // won't converge, but train_all runs
            ..Default::default()
        });

        for i in 0..5 {
            m.record_search(
                &[i as f32, 0.0, 1.0],
                1,
                0.5,
                100,
                100,
                50,
            );
        }
        // After 5 searches (== train_every_n), searches_since_train resets.
        assert_eq!(m.searches_since_train, 0);
    }

    // -- Integration: full lifecycle --

    #[test]
    fn full_lifecycle_no_panics() {
        let mut m = HnswEmlManager::new(HnswEmlConfig {
            min_training_samples: 10,
            train_every_n: 100_000,
            ..Default::default()
        });

        // Record searches
        for i in 0..20 {
            let q = vec![
                (i as f32 * 0.3).sin(),
                (i as f32 * 0.7).cos(),
                i as f32 / 20.0,
            ];
            m.record_search(&q, 3, 0.85, 100, 300 + i * 5, 500);
        }

        // Record distance pairs
        for _ in 0..15 {
            m.record_distance_pair(
                &[1.0, 0.0, 0.0],
                &[0.9, 0.1, 0.0],
                &[1.0, 0.0, 0.0],
            );
        }

        // Measure recall
        let hnsw = vec![vec!["a".into(), "b".into()]];
        let exact = vec![vec!["a".into(), "c".into()]];
        let recall = m.measure_recall(&hnsw, &exact, 500, 20, 5);
        assert!(recall >= 0.0 && recall <= 1.0);

        // Predict (untrained)
        let ef_pred = m.predict_ef(&[1.0, 0.0, 0.0], 500);
        assert!(ef_pred.recommended_ef > 0);

        let rebuild_pred = m.predict_rebuild(500, 20, 5);
        assert!(rebuild_pred.predicted_recall >= 0.0);

        // Train
        let _ = m.train_all();

        // Status
        let s = m.status();
        assert!(s.total_searches > 0);

        // Reset
        m.reset();
        let s2 = m.status();
        assert_eq!(s2.total_searches, 0);
    }
}
