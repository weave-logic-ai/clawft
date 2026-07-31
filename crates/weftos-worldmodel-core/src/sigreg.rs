//! SIGReg health monitoring contract (Welford stats land in WEFT-528).

use crate::error::WorldModelResult;
use crate::latent::Latent;

/// Auto-rollback when `sigreg_health` stays below this for the window.
pub const SIGREG_HEALTH_ROLLBACK_THRESHOLD: f32 = 0.85;

/// Sustained unhealthy window before rollback (seconds).
pub const SIGREG_HEALTH_WINDOW_SECS: u32 = 30;

/// Snapshot of SIGReg manifold health.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SigRegHealth {
    /// Scalar health in `[0, 1]` (1 = ideal isotropic Gaussian).
    pub score: f32,
    /// Consecutive seconds below threshold (for rollback gate).
    pub seconds_below_threshold: u32,
    /// Model / encoder version tag (ExoChain attestation).
    pub version_tag: u64,
}

impl SigRegHealth {
    /// Perfect health snapshot.
    pub const fn perfect(version_tag: u64) -> Self {
        Self {
            score: 1.0,
            seconds_below_threshold: 0,
            version_tag,
        }
    }

    /// Whether score is at or above the rollback threshold.
    pub fn is_healthy(self) -> bool {
        self.score >= SIGREG_HEALTH_ROLLBACK_THRESHOLD
    }

    /// Whether the sustained-unhealthy rollback gate would fire.
    pub fn should_rollback(self) -> bool {
        !self.is_healthy() && self.seconds_below_threshold >= SIGREG_HEALTH_WINDOW_SECS
    }
}

/// Running SIGReg monitor (Epps-Pulley / Welford in impls).
///
/// Production: Welford online stats + auto-rollback (WEFT-528).
pub trait SigRegMonitor {
    /// Ingest a latent sample; return updated health.
    fn update(&mut self, z: &Latent) -> WorldModelResult<SigRegHealth>;

    /// Current health without updating.
    fn health(&self) -> SigRegHealth;
}

/// Stub monitor: always perfect health.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSigRegMonitor {
    /// Version tag reported in health snapshots.
    pub version_tag: u64,
}

impl SigRegMonitor for NullSigRegMonitor {
    fn update(&mut self, _z: &Latent) -> WorldModelResult<SigRegHealth> {
        Ok(self.health())
    }

    fn health(&self) -> SigRegHealth {
        SigRegHealth::perfect(self.version_tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_gate() {
        let mut h = SigRegHealth {
            score: 0.5,
            seconds_below_threshold: 29,
            version_tag: 1,
        };
        assert!(!h.should_rollback());
        h.seconds_below_threshold = 30;
        assert!(h.should_rollback());
        h.score = 0.9;
        assert!(!h.should_rollback());
    }
}
