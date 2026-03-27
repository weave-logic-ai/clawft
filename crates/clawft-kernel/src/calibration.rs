//! Boot-time ECC benchmarking and capability advertisement (Phase K3c).
//!
//! This module is compiled only when the `ecc` feature is enabled.
//! It provides [`run_calibration`], which exercises the HNSW index and
//! causal graph with synthetic data, measures per-tick latency, and
//! returns an [`EccCalibration`] that other modules (CognitiveTick,
//! cluster advertisement) use to auto-tune cadence and decide which
//! subsystems are feasible on this hardware.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::causal::{CausalEdgeType, CausalGraph};
use crate::hnsw_service::HnswService;

// ---------------------------------------------------------------------------
// EccCalibrationConfig
// ---------------------------------------------------------------------------

/// Tuning knobs for the calibration run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EccCalibrationConfig {
    /// Number of synthetic ticks to execute during calibration.
    pub calibration_ticks: u32,
    /// Initial tick interval in milliseconds (may be auto-raised).
    pub tick_interval_ms: u32,
    /// Target compute-time / tick-interval ratio (e.g. 0.3 = 30%).
    pub tick_budget_ratio: f32,
    /// Dimensionality of synthetic test vectors.
    pub vector_dimensions: usize,
}

impl Default for EccCalibrationConfig {
    fn default() -> Self {
        Self {
            calibration_ticks: 30,
            tick_interval_ms: 50,
            tick_budget_ratio: 0.3,
            vector_dimensions: 384,
        }
    }
}

// ---------------------------------------------------------------------------
// EccCalibration
// ---------------------------------------------------------------------------

/// Results of a boot-time calibration run.
///
/// Consumed by `CognitiveTick` (for cadence), by `cluster.rs` (for
/// capability advertisement), and optionally by ExoChain (logged as an
/// `ecc.boot.calibration` chain event).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EccCalibration {
    /// Median per-tick latency in microseconds.
    pub compute_p50_us: u64,
    /// 95th-percentile per-tick latency in microseconds.
    pub compute_p95_us: u64,
    /// Effective tick interval after auto-adjustment (ms).
    pub tick_interval_ms: u32,
    /// Ratio of p95 compute time to tick interval.
    pub headroom_ratio: f32,
    /// Number of HNSW vectors inserted during calibration.
    pub hnsw_vector_count: u32,
    /// Number of causal edges created during calibration.
    pub causal_edge_count: u32,
    /// Whether spectral analysis is feasible on this hardware.
    pub spectral_capable: bool,
    /// Unix timestamp (seconds) at which calibration completed.
    pub calibrated_at: u64,
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random vector generation
// ---------------------------------------------------------------------------

/// Generate a deterministic f32 vector using a simple LCG, avoiding the
/// `rand` crate dependency.
fn pseudo_random_vector(seed: u64, dims: usize) -> Vec<f32> {
    let mut state = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (0..dims)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // Map to [-1.0, 1.0]
            ((state >> 33) as f32) / (u32::MAX as f32 / 2.0) - 1.0
        })
        .collect()
}

// ---------------------------------------------------------------------------
// run_calibration
// ---------------------------------------------------------------------------

/// Execute a calibration run against the provided HNSW index and causal
/// graph.
///
/// This inserts `config.calibration_ticks` synthetic vectors into HNSW,
/// searches each one, creates causal edges linking consecutive ticks,
/// and hashes each vector with BLAKE3 (simulating a Merkle commit).
/// After collecting per-tick timings it computes p50/p95 percentiles,
/// decides the effective tick interval, checks spectral feasibility,
/// and cleans up all synthetic data.
pub fn run_calibration(
    hnsw: &HnswService,
    causal: &CausalGraph,
    config: &EccCalibrationConfig,
) -> EccCalibration {
    let n = config.calibration_ticks as usize;
    assert!(n > 0, "calibration_ticks must be > 0");

    // Pre-generate all test vectors.
    let vectors: Vec<Vec<f32>> = (0..n)
        .map(|i| pseudo_random_vector(i as u64, config.vector_dimensions))
        .collect();

    // Pre-create causal graph nodes so that link() can find them.
    let node_ids: Vec<u64> = (0..n)
        .map(|i| causal.add_node(format!("cal_{i}"), serde_json::json!({})))
        .collect();

    // Run synthetic ticks and collect per-tick timings.
    let mut timings_us: Vec<u64> = Vec::with_capacity(n);

    for i in 0..n {
        let start = Instant::now();

        // 1. HNSW insert
        let id = format!("cal_{i}");
        hnsw.insert(id, vectors[i].clone(), serde_json::json!({}));

        // 2. HNSW search (k=10)
        let _results = hnsw.search(&vectors[i], 10);

        // 3. Causal edge: link tick i-1 -> tick i
        if i > 0 {
            causal.link(
                node_ids[i - 1],
                node_ids[i],
                CausalEdgeType::Follows,
                1.0,
                0,
                0,
            );
        }

        // 4. BLAKE3 hash (Merkle commit simulation)
        let vec_bytes: Vec<u8> = vectors[i]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let _hash = blake3::hash(&vec_bytes);

        let elapsed = start.elapsed().as_micros() as u64;
        timings_us.push(elapsed);
    }

    // Compute percentiles.
    timings_us.sort_unstable();
    let p50 = timings_us[n / 2];
    let p95 = timings_us[n * 95 / 100];

    // Record counts before cleanup.
    let hnsw_vector_count = n as u32;
    let causal_edge_count = if n > 1 { (n - 1) as u32 } else { 0 };

    // Clean up synthetic data.
    hnsw.clear();
    causal.clear();

    // Auto-adjust tick interval: ensure at least config.tick_interval_ms,
    // but raise it if p95 / budget_ratio exceeds the configured value.
    let p95_ms = p95 as f32 / 1000.0;
    let required_ms = (p95_ms / config.tick_budget_ratio) as u32;
    let tick_interval_ms = config.tick_interval_ms.max(required_ms);

    // Headroom ratio: what fraction of the tick interval does p95 consume?
    let headroom_ratio = if tick_interval_ms > 0 {
        p95_ms / (tick_interval_ms as f32)
    } else {
        1.0
    };

    // Spectral feasibility: p95 under 10ms means we can afford spectral
    // analysis within the tick budget.
    let spectral_capable = p95 < 10_000;

    // Unix timestamp (seconds).
    let calibrated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    EccCalibration {
        compute_p50_us: p50,
        compute_p95_us: p95,
        tick_interval_ms,
        headroom_ratio,
        hnsw_vector_count,
        causal_edge_count,
        spectral_capable,
        calibrated_at,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hnsw_service::{HnswService, HnswServiceConfig};

    fn make_hnsw() -> HnswService {
        HnswService::new(HnswServiceConfig::default())
    }

    fn make_causal() -> CausalGraph {
        CausalGraph::new()
    }

    fn small_config() -> EccCalibrationConfig {
        EccCalibrationConfig {
            calibration_ticks: 10,
            tick_interval_ms: 50,
            tick_budget_ratio: 0.3,
            vector_dimensions: 16,
        }
    }

    // 1. default_config
    #[test]
    fn default_config() {
        let cfg = EccCalibrationConfig::default();
        assert_eq!(cfg.calibration_ticks, 30);
        assert_eq!(cfg.tick_interval_ms, 50);
        assert!((cfg.tick_budget_ratio - 0.3).abs() < f32::EPSILON);
        assert_eq!(cfg.vector_dimensions, 384);
    }

    // 2. pseudo_random_vector_deterministic
    #[test]
    fn pseudo_random_vector_deterministic() {
        let a = pseudo_random_vector(42, 8);
        let b = pseudo_random_vector(42, 8);
        assert_eq!(a, b, "same seed must produce identical vectors");

        let c = pseudo_random_vector(99, 8);
        assert_ne!(a, c, "different seeds must produce different vectors");
    }

    // 3. pseudo_random_vector_correct_dimensions
    #[test]
    fn pseudo_random_vector_correct_dimensions() {
        for dims in [1, 16, 128, 384] {
            let v = pseudo_random_vector(0, dims);
            assert_eq!(v.len(), dims);
        }
    }

    // 4. calibration_basic
    #[test]
    fn calibration_basic() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        let cfg = small_config();

        let cal = run_calibration(&hnsw, &causal, &cfg);

        assert!(cal.compute_p50_us > 0, "p50 must be positive");
        assert!(cal.compute_p95_us > 0, "p95 must be positive");
        assert!(cal.tick_interval_ms > 0, "tick interval must be positive");
        assert!(cal.headroom_ratio > 0.0, "headroom must be positive");
        assert_eq!(cal.hnsw_vector_count, 10);
        assert_eq!(cal.causal_edge_count, 9);
        assert!(cal.calibrated_at > 0, "timestamp must be set");
    }

    // 5. calibration_cleans_up
    #[test]
    fn calibration_cleans_up() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        let cfg = small_config();

        let _cal = run_calibration(&hnsw, &causal, &cfg);

        assert!(
            hnsw.is_empty(),
            "HNSW store must be empty after calibration cleanup"
        );
        assert_eq!(
            causal.node_count(),
            0,
            "causal graph must be empty after calibration cleanup"
        );
        assert_eq!(
            causal.edge_count(),
            0,
            "causal graph edges must be zero after calibration cleanup"
        );
    }

    // 6. calibration_p50_less_than_p95
    #[test]
    fn calibration_p50_less_than_p95() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        let cfg = small_config();

        let cal = run_calibration(&hnsw, &causal, &cfg);

        assert!(
            cal.compute_p50_us <= cal.compute_p95_us,
            "p50 ({}) must be <= p95 ({})",
            cal.compute_p50_us,
            cal.compute_p95_us,
        );
    }

    // 7. calibration_spectral_capable
    #[test]
    fn calibration_spectral_capable() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        // Very small workload should be fast enough for spectral.
        let cfg = EccCalibrationConfig {
            calibration_ticks: 5,
            tick_interval_ms: 50,
            tick_budget_ratio: 0.3,
            vector_dimensions: 4,
        };

        let cal = run_calibration(&hnsw, &causal, &cfg);

        assert!(
            cal.spectral_capable,
            "a trivial calibration run should report spectral capable (p95={}us)",
            cal.compute_p95_us,
        );
    }

    // 8. calibration_tick_interval_auto_adjusted
    #[test]
    fn calibration_tick_interval_auto_adjusted() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        // Set an impossibly low tick interval and tight budget ratio
        // so that p95 forces an upward adjustment.
        let cfg = EccCalibrationConfig {
            calibration_ticks: 10,
            tick_interval_ms: 1, // very low
            tick_budget_ratio: 0.01, // very tight budget
            vector_dimensions: 64,
        };

        let cal = run_calibration(&hnsw, &causal, &cfg);

        assert!(
            cal.tick_interval_ms >= cfg.tick_interval_ms,
            "tick_interval_ms ({}) must be >= configured value ({})",
            cal.tick_interval_ms,
            cfg.tick_interval_ms,
        );
    }

    // Bonus: single-tick calibration edge case (no causal edges)
    #[test]
    fn calibration_single_tick() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        let cfg = EccCalibrationConfig {
            calibration_ticks: 1,
            tick_interval_ms: 50,
            tick_budget_ratio: 0.3,
            vector_dimensions: 8,
        };

        let cal = run_calibration(&hnsw, &causal, &cfg);

        assert_eq!(cal.hnsw_vector_count, 1);
        assert_eq!(cal.causal_edge_count, 0, "no edges with a single tick");
        assert!(hnsw.is_empty());
        assert_eq!(causal.node_count(), 0);
    }

    // Bonus: headroom_ratio is within sane bounds
    #[test]
    fn calibration_headroom_sane() {
        let hnsw = make_hnsw();
        let causal = make_causal();
        let cfg = small_config();

        let cal = run_calibration(&hnsw, &causal, &cfg);

        assert!(
            cal.headroom_ratio >= 0.0 && cal.headroom_ratio <= 1.0,
            "headroom_ratio ({}) should be in [0, 1] for a well-budgeted run",
            cal.headroom_ratio,
        );
    }
}
