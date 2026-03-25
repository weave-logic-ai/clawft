//! Configurable cognitive tick loop -- the heartbeat of the ECC cognitive substrate.
//!
//! The [`CognitiveTick`] service drives the kernel's cognitive processing
//! cycle at a configurable interval, with optional adaptive adjustment
//! based on measured compute timings and drift detection.

use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::health::HealthStatus;
use crate::service::{ServiceType, SystemService};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the cognitive tick loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTickConfig {
    /// Target tick interval in milliseconds.
    pub tick_interval_ms: u32,
    /// Fraction of the tick interval budget available for compute (0.0..1.0).
    pub tick_budget_ratio: f32,
    /// Number of ticks used for initial calibration.
    pub calibration_ticks: u32,
    /// Whether to adaptively adjust the tick interval based on load.
    pub adaptive_tick: bool,
    /// Window (in seconds) over which recent timings are averaged.
    pub adaptive_window_s: u32,
}

impl Default for CognitiveTickConfig {
    fn default() -> Self {
        Self {
            tick_interval_ms: 50,
            tick_budget_ratio: 0.3,
            calibration_ticks: 100,
            adaptive_tick: true,
            adaptive_window_s: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// Stats (public snapshot)
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of cognitive tick statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTickStats {
    /// Total number of ticks recorded.
    pub tick_count: u64,
    /// Current (possibly adapted) tick interval in milliseconds.
    pub current_interval_ms: u32,
    /// Running average compute time in microseconds.
    pub avg_compute_us: u64,
    /// Maximum observed compute time in microseconds.
    pub max_compute_us: u64,
    /// Number of ticks that exceeded the compute budget.
    pub drift_count: u64,
    /// Whether the tick loop is currently running.
    pub running: bool,
}

// ---------------------------------------------------------------------------
// Internal mutable state
// ---------------------------------------------------------------------------

struct CognitiveTickState {
    tick_count: u64,
    current_interval_ms: u32,
    running: bool,
    drift_count: u64,
    recent_timings_us: Vec<u64>,
    max_compute_us: u64,
}

// ---------------------------------------------------------------------------
// CognitiveTick
// ---------------------------------------------------------------------------

/// The cognitive tick service.
///
/// Tracks per-tick compute timings, detects budget drift, and optionally
/// adjusts the tick interval to maintain throughput under load.
pub struct CognitiveTick {
    config: CognitiveTickConfig,
    state: Mutex<CognitiveTickState>,
}

impl CognitiveTick {
    /// Create a new cognitive tick service with the given configuration.
    pub fn new(config: CognitiveTickConfig) -> Self {
        let interval = config.tick_interval_ms;
        Self {
            config,
            state: Mutex::new(CognitiveTickState {
                tick_count: 0,
                current_interval_ms: interval,
                running: false,
                drift_count: 0,
                recent_timings_us: Vec::new(),
                max_compute_us: 0,
            }),
        }
    }

    /// Convenience constructor that creates a default config with a custom interval.
    pub fn with_interval(interval_ms: u32) -> Self {
        Self::new(CognitiveTickConfig {
            tick_interval_ms: interval_ms,
            ..CognitiveTickConfig::default()
        })
    }

    /// Return a point-in-time snapshot of statistics.
    pub fn stats(&self) -> CognitiveTickStats {
        let s = self.state.lock().unwrap();
        let avg = if s.recent_timings_us.is_empty() {
            0
        } else {
            let sum: u64 = s.recent_timings_us.iter().sum();
            sum / s.recent_timings_us.len() as u64
        };
        CognitiveTickStats {
            tick_count: s.tick_count,
            current_interval_ms: s.current_interval_ms,
            avg_compute_us: avg,
            max_compute_us: s.max_compute_us,
            drift_count: s.drift_count,
            running: s.running,
        }
    }

    /// Record a tick with the given compute duration in microseconds.
    ///
    /// This method:
    /// 1. Increments the tick counter.
    /// 2. Maintains a sliding window of recent timings.
    /// 3. Updates the maximum observed compute time.
    /// 4. Detects budget drift (compute exceeding the budget).
    /// 5. Adaptively adjusts the tick interval if enabled.
    pub fn record_tick(&self, compute_us: u64) {
        let mut s = self.state.lock().unwrap();

        // 1. Increment tick count.
        s.tick_count += 1;

        // 2. Maintain sliding window.
        let window_size = self.window_capacity(s.current_interval_ms);
        s.recent_timings_us.push(compute_us);
        if s.recent_timings_us.len() > window_size {
            let excess = s.recent_timings_us.len() - window_size;
            s.recent_timings_us.drain(..excess);
        }

        // 3. Update max.
        if compute_us > s.max_compute_us {
            s.max_compute_us = compute_us;
        }

        // 4. Drift detection.
        let budget_us =
            (s.current_interval_ms as f32 * 1000.0 * self.config.tick_budget_ratio) as u64;
        if compute_us > budget_us {
            s.drift_count += 1;
        }

        // 5. Adaptive adjustment.
        if self.config.adaptive_tick && !s.recent_timings_us.is_empty() {
            let avg: u64 =
                s.recent_timings_us.iter().sum::<u64>() / s.recent_timings_us.len() as u64;
            let upper_threshold = (budget_us as f64 * 1.1) as u64;
            let lower_threshold = (budget_us as f64 * 0.5) as u64;

            if avg > upper_threshold {
                // Increase interval by 10%.
                let new_interval = (s.current_interval_ms as f64 * 1.1).round() as u32;
                s.current_interval_ms = new_interval;
            } else if avg < lower_threshold {
                // Decrease interval by 10%, minimum 10ms.
                let new_interval = (s.current_interval_ms as f64 * 0.9).round() as u32;
                s.current_interval_ms = new_interval.max(10);
            }
        }
    }

    /// Whether the tick loop is currently running.
    pub fn is_running(&self) -> bool {
        self.state.lock().unwrap().running
    }

    /// Set the running state.
    pub fn set_running(&self, running: bool) {
        self.state.lock().unwrap().running = running;
    }

    /// Total number of ticks recorded.
    pub fn tick_count(&self) -> u64 {
        self.state.lock().unwrap().tick_count
    }

    /// Current (possibly adapted) tick interval in milliseconds.
    pub fn current_interval_ms(&self) -> u32 {
        self.state.lock().unwrap().current_interval_ms
    }

    /// Number of ticks that exceeded the compute budget.
    pub fn drift_count(&self) -> u64 {
        self.state.lock().unwrap().drift_count
    }

    /// Reset all statistics to initial values (configuration is preserved).
    pub fn reset(&self) {
        let mut s = self.state.lock().unwrap();
        s.tick_count = 0;
        s.current_interval_ms = self.config.tick_interval_ms;
        s.running = false;
        s.drift_count = 0;
        s.recent_timings_us.clear();
        s.max_compute_us = 0;
    }

    // --- private helpers ---

    /// Compute the maximum number of timing samples to retain.
    fn window_capacity(&self, interval_ms: u32) -> usize {
        if interval_ms == 0 {
            return 1;
        }
        let ticks_per_window = (self.config.adaptive_window_s * 1000) / interval_ms;
        (ticks_per_window as usize).max(1)
    }
}

// ---------------------------------------------------------------------------
// SystemService implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl SystemService for CognitiveTick {
    fn name(&self) -> &str {
        "ecc.cognitive_tick"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_running(true);
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.set_running(false);
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.is_running() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded("cognitive tick not running".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = CognitiveTickConfig::default();
        assert_eq!(cfg.tick_interval_ms, 50);
        assert!((cfg.tick_budget_ratio - 0.3).abs() < f32::EPSILON);
        assert_eq!(cfg.calibration_ticks, 100);
        assert!(cfg.adaptive_tick);
        assert_eq!(cfg.adaptive_window_s, 30);
    }

    #[test]
    fn new_with_config() {
        let cfg = CognitiveTickConfig {
            tick_interval_ms: 100,
            tick_budget_ratio: 0.5,
            calibration_ticks: 200,
            adaptive_tick: false,
            adaptive_window_s: 60,
        };
        let ct = CognitiveTick::new(cfg.clone());
        assert_eq!(ct.current_interval_ms(), 100);
        assert!(!ct.is_running());
    }

    #[test]
    fn with_interval() {
        let ct = CognitiveTick::with_interval(75);
        assert_eq!(ct.current_interval_ms(), 75);
        // Other fields should be defaults.
        assert!(ct.config.adaptive_tick);
        assert_eq!(ct.config.calibration_ticks, 100);
    }

    #[test]
    fn stats_initial() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        let s = ct.stats();
        assert_eq!(s.tick_count, 0);
        assert_eq!(s.current_interval_ms, 50);
        assert_eq!(s.avg_compute_us, 0);
        assert_eq!(s.max_compute_us, 0);
        assert_eq!(s.drift_count, 0);
        assert!(!s.running);
    }

    #[test]
    fn record_tick_increments_count() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        ct.record_tick(100);
        ct.record_tick(200);
        ct.record_tick(300);
        assert_eq!(ct.tick_count(), 3);
    }

    #[test]
    fn record_tick_updates_max() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        ct.record_tick(100);
        ct.record_tick(500);
        ct.record_tick(200);
        assert_eq!(ct.stats().max_compute_us, 500);
    }

    #[test]
    fn is_running_default_false() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        assert!(!ct.is_running());
    }

    #[test]
    fn set_running() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        ct.set_running(true);
        assert!(ct.is_running());
        ct.set_running(false);
        assert!(!ct.is_running());
    }

    #[test]
    fn tick_count() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        assert_eq!(ct.tick_count(), 0);
        ct.record_tick(10);
        assert_eq!(ct.tick_count(), 1);
    }

    #[test]
    fn current_interval_ms() {
        let ct = CognitiveTick::with_interval(42);
        assert_eq!(ct.current_interval_ms(), 42);
    }

    #[test]
    fn drift_detection() {
        // Default: interval=50ms, budget_ratio=0.3 => budget = 50*1000*0.3 = 15000us
        let mut cfg = CognitiveTickConfig::default();
        cfg.adaptive_tick = false; // disable adaptive so interval stays constant
        let ct = CognitiveTick::new(cfg);

        // Under budget: no drift.
        ct.record_tick(10_000);
        assert_eq!(ct.drift_count(), 0);

        // Exactly at budget boundary (15000): not exceeding, no drift.
        ct.record_tick(15_000);
        assert_eq!(ct.drift_count(), 0);

        // Over budget.
        ct.record_tick(16_000);
        assert_eq!(ct.drift_count(), 1);

        // Another over budget.
        ct.record_tick(20_000);
        assert_eq!(ct.drift_count(), 2);
    }

    #[test]
    fn adaptive_increase() {
        // Set up a config where budget is small so we can easily exceed 1.1x.
        // interval=50ms, ratio=0.3 => budget = 15000us, upper = 16500us
        let cfg = CognitiveTickConfig {
            tick_interval_ms: 50,
            tick_budget_ratio: 0.3,
            calibration_ticks: 100,
            adaptive_tick: true,
            adaptive_window_s: 30,
        };
        let ct = CognitiveTick::new(cfg);

        // Record many ticks with compute well above the upper threshold (16500us).
        for _ in 0..20 {
            ct.record_tick(20_000);
        }

        // Interval should have increased from 50.
        assert!(
            ct.current_interval_ms() > 50,
            "expected interval > 50, got {}",
            ct.current_interval_ms()
        );
    }

    #[test]
    fn adaptive_decrease() {
        // interval=100ms, ratio=0.3 => budget = 30000us, lower = 15000us
        let cfg = CognitiveTickConfig {
            tick_interval_ms: 100,
            tick_budget_ratio: 0.3,
            calibration_ticks: 100,
            adaptive_tick: true,
            adaptive_window_s: 30,
        };
        let ct = CognitiveTick::new(cfg);

        // Record many ticks with compute well below the lower threshold.
        for _ in 0..20 {
            ct.record_tick(1_000);
        }

        // Interval should have decreased from 100.
        assert!(
            ct.current_interval_ms() < 100,
            "expected interval < 100, got {}",
            ct.current_interval_ms()
        );
    }

    #[test]
    fn adaptive_min_interval() {
        // Start with a small interval so it can shrink toward the minimum.
        let cfg = CognitiveTickConfig {
            tick_interval_ms: 12,
            tick_budget_ratio: 0.3,
            calibration_ticks: 100,
            adaptive_tick: true,
            adaptive_window_s: 30,
        };
        let ct = CognitiveTick::new(cfg);

        // Record very fast ticks to push the interval down.
        for _ in 0..200 {
            ct.record_tick(1);
        }

        // Interval must never go below 10ms.
        assert!(
            ct.current_interval_ms() >= 10,
            "expected interval >= 10, got {}",
            ct.current_interval_ms()
        );
    }

    #[test]
    fn reset_clears_stats() {
        let ct = CognitiveTick::with_interval(80);
        ct.set_running(true);
        ct.record_tick(5_000);
        ct.record_tick(50_000);

        // Verify non-zero state.
        assert!(ct.tick_count() > 0);
        assert!(ct.stats().max_compute_us > 0);
        assert!(ct.is_running());

        ct.reset();

        assert_eq!(ct.tick_count(), 0);
        assert_eq!(ct.stats().max_compute_us, 0);
        assert_eq!(ct.stats().avg_compute_us, 0);
        assert_eq!(ct.drift_count(), 0);
        assert!(!ct.is_running());
        // Interval should be reset to config value.
        assert_eq!(ct.current_interval_ms(), 80);
    }

    #[tokio::test]
    async fn service_name_and_type() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        assert_eq!(ct.name(), "ecc.cognitive_tick");
        assert_eq!(ct.service_type(), ServiceType::Core);
    }

    #[tokio::test]
    async fn service_start_stop() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        assert!(!ct.is_running());

        ct.start().await.unwrap();
        assert!(ct.is_running());

        ct.stop().await.unwrap();
        assert!(!ct.is_running());
    }

    #[tokio::test]
    async fn health_check_reflects_running() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        assert_eq!(
            ct.health_check().await,
            HealthStatus::Degraded("cognitive tick not running".into())
        );

        ct.start().await.unwrap();
        assert_eq!(ct.health_check().await, HealthStatus::Healthy);
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = CognitiveTickConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: CognitiveTickConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tick_interval_ms, cfg.tick_interval_ms);
        assert!((restored.tick_budget_ratio - cfg.tick_budget_ratio).abs() < f32::EPSILON);
    }

    #[test]
    fn stats_serde_roundtrip() {
        let ct = CognitiveTick::new(CognitiveTickConfig::default());
        ct.record_tick(1234);
        let stats = ct.stats();
        let json = serde_json::to_string(&stats).unwrap();
        let restored: CognitiveTickStats = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tick_count, 1);
        assert_eq!(restored.avg_compute_us, 1234);
    }
}
