//! SIGReg latent vector contract (WEFT-543).

/// Latent dimensionality for `mesh.sensor.v1` / SIGReg manifold.
///
/// Isotropic Gaussian `N(0, I_192)`. Changing this constant without a wire
/// major-version bump is forbidden — receivers must reject mismatched
/// `latent_dim` fields (see decision batch WEFT-543).
pub const LATENT_DIM: usize = 192;

/// Wire-friendly form of [`LATENT_DIM`] (`u16` on the bus).
pub const LATENT_DIM_U16: u16 = 192;

/// Schema major version that pins [`LATENT_DIM`] = 192.
pub const LATENT_SCHEMA_MAJOR_V1: u16 = 1;

/// Fixed-width latent code `z ∈ ℝ¹⁹²`.
pub type Latent = [f32; LATENT_DIM];

/// Version tag for latent payloads (ExoChain / mesh framing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LatentVersion {
    /// Schema major (1 ⇒ 192-dim SIGReg).
    pub major: u16,
    /// Schema minor (additive fields only).
    pub minor: u16,
}

impl LatentVersion {
    /// v1.0 — 192-dim SIGReg contract.
    pub const V1: Self = Self {
        major: LATENT_SCHEMA_MAJOR_V1,
        minor: 0,
    };

    /// Whether this version expects [`LATENT_DIM`] width.
    pub const fn expects_v1_dim(self) -> bool {
        self.major == LATENT_SCHEMA_MAJOR_V1
    }
}

/// All-zero latent (prior mean of `N(0, I)`).
#[inline]
pub const fn zero_latent() -> Latent {
    [0.0; LATENT_DIM]
}

/// True when `dim` matches the v1 SIGReg width.
#[inline]
pub const fn latent_dim_matches_v1(dim: u16) -> bool {
    dim == LATENT_DIM_U16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_v1_flags() {
        assert!(LatentVersion::V1.expects_v1_dim());
        assert!(!LatentVersion {
            major: 2,
            minor: 0
        }
        .expects_v1_dim());
    }
}
