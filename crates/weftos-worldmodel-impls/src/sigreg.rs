//! SIGReg monitor stubs (online Welford stats land in WEFT-528).

use weftos_worldmodel_core::{Latent, SigRegHealth, SigRegMonitor, WorldModelResult};

/// Stub monitor: always reports perfect health.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSigRegMonitor {
    /// Version tag reported in health snapshots (ExoChain attestation).
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
    use weftos_worldmodel_core::zero_latent;

    #[test]
    fn perfect() {
        let mon = NullSigRegMonitor { version_tag: 7 };
        assert_eq!(mon.health().version_tag, 7);
        assert!(mon.health().is_healthy());
        let mut mon = mon;
        assert!(mon.update(&zero_latent()).unwrap().is_healthy());
    }
}
