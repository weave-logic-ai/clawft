//! Optional candle ML skeleton (`feature = "candle"`).
//!
//! # Honest status (WEFT-521)
//!
//! This module defines configuration types and trait impl shells for:
//!
//! - **ViT-tiny** sensor encoder → `LATENT_DIM` (192) latent
//! - **AdaLN-modulated** action-conditioned predictor `pred_φ`
//!
//! Without loaded weights, both return [`WorldModelError::Unavailable`].
//! Full transformer blocks, checkpoint I/O, and training are **not**
//! implemented here — document remaining work in the crate README and
//! WEFT-529.
//!
//! Enabling this feature pulls `candle-core` / `candle-nn` 0.8. Historical
//! candle-core 0.2.x does not build on this workspace (WEFT-216). Default CI
//! must keep `candle` off until a pin is proven green.

use candle_core::Device;
use weftos_worldmodel_core::{
    Action, Encoder, Latent, Predictor, WorldModelError, WorldModelResult, LATENT_DIM,
};

/// Prove the `candle` feature links a toolchain-compatible candle-core.
///
/// Real tensor graphs replace this once ViT-tiny / AdaLN weights land.
#[inline]
pub fn candle_cpu_device() -> Device {
    Device::Cpu
}

/// ViT-tiny hyper-parameters aligned with the SIGReg 192-d contract.
///
/// Values are intentional placeholders for an edge-scale vision encoder;
/// bake-off may adjust depth / width without changing [`LATENT_DIM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VitTinyConfig {
    /// Input image side length (square RGB assumed).
    pub image_size: usize,
    /// Patch size (image_size must be divisible by patch_size).
    pub patch_size: usize,
    /// Transformer hidden width.
    pub embed_dim: usize,
    /// Number of transformer blocks.
    pub depth: usize,
    /// Attention heads.
    pub num_heads: usize,
    /// Output latent width (must equal [`LATENT_DIM`] for v1).
    pub latent_dim: usize,
}

impl Default for VitTinyConfig {
    fn default() -> Self {
        Self {
            image_size: 64,
            patch_size: 8,
            embed_dim: 192,
            depth: 4,
            num_heads: 3,
            latent_dim: LATENT_DIM,
        }
    }
}

impl VitTinyConfig {
    /// Validate geometric constraints (no weights required).
    pub fn validate(&self) -> WorldModelResult<()> {
        if self.latent_dim != LATENT_DIM {
            return Err(WorldModelError::dim_mismatch(self.latent_dim as u16));
        }
        if self.patch_size == 0 || self.image_size % self.patch_size != 0 {
            return Err(WorldModelError::Other("invalid vit patch geometry"));
        }
        if self.num_heads == 0 || self.embed_dim % self.num_heads != 0 {
            return Err(WorldModelError::Other("invalid vit head geometry"));
        }
        Ok(())
    }
}

/// Candle ViT-tiny encoder shell.
///
/// Holds config only until weight loading is implemented. [`Encoder::encode`]
/// returns [`WorldModelError::Unavailable`] so callers fall back to local ECC
/// (ADR-090).
#[derive(Debug, Clone)]
pub struct CandleVitEncoder {
    /// Architecture hyper-parameters.
    pub config: VitTinyConfig,
    /// Optional path hint for future safetensors / candle load.
    pub weights_path: Option<alloc::string::String>,
}

impl CandleVitEncoder {
    /// Construct with default ViT-tiny config and no weights.
    pub fn unloaded() -> Self {
        Self {
            config: VitTinyConfig::default(),
            weights_path: None,
        }
    }

    /// Construct with config; still unloaded until a load API lands.
    pub fn with_config(config: VitTinyConfig) -> WorldModelResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            weights_path: None,
        })
    }

    /// Whether trained weights are available for inference.
    pub fn is_loaded(&self) -> bool {
        // Weight loading not implemented (WEFT-521 skeleton).
        false
    }
}

impl Encoder for CandleVitEncoder {
    fn encode(&self, _observation: &[u8]) -> WorldModelResult<Latent> {
        self.config.validate()?;
        if !self.is_loaded() {
            return Err(WorldModelError::Unavailable);
        }
        // Future: patch embed → transformer → project to LATENT_DIM.
        Err(WorldModelError::EncodeFailed)
    }

    fn latent_dim(&self) -> usize {
        self.config.latent_dim
    }
}

/// AdaLN-modulated predictor shell (`pred_φ`: z_t, a_t → ẑ_{t+1}`).
///
/// Conditioning path: action code → scale/shift for latent residual blocks.
/// Unloaded state returns [`WorldModelError::Unavailable`].
#[derive(Debug, Clone, Default)]
pub struct AdaLnPredictor {
    /// Optional path hint for future checkpoint load.
    pub weights_path: Option<alloc::string::String>,
    /// Hidden width for AdaLN MLP (placeholder).
    pub hidden_dim: usize,
}

impl AdaLnPredictor {
    /// Unloaded predictor with default hidden width.
    pub fn unloaded() -> Self {
        Self {
            weights_path: None,
            hidden_dim: 256,
        }
    }

    /// Whether trained weights are available.
    pub fn is_loaded(&self) -> bool {
        false
    }
}

impl Predictor for AdaLnPredictor {
    fn predict(&self, _z_t: &Latent, _action: &Action) -> WorldModelResult<Latent> {
        if !self.is_loaded() {
            return Err(WorldModelError::Unavailable);
        }
        Err(WorldModelError::PredictFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_valid() {
        VitTinyConfig::default().validate().unwrap();
        // Links candle-core on this feature set.
        let _ = candle_cpu_device();
    }

    #[test]
    fn unloaded_encoder_unavailable() {
        let enc = CandleVitEncoder::unloaded();
        assert!(!enc.is_loaded());
        assert!(matches!(
            enc.encode(b"rgb"),
            Err(WorldModelError::Unavailable)
        ));
        assert_eq!(enc.latent_dim(), LATENT_DIM);
    }

    #[test]
    fn unloaded_adaln_unavailable() {
        let pred = AdaLnPredictor::unloaded();
        assert!(matches!(
            pred.predict(&weftos_worldmodel_core::zero_latent(), &Action::null()),
            Err(WorldModelError::Unavailable)
        ));
    }

    #[test]
    fn bad_latent_dim_rejected() {
        let mut cfg = VitTinyConfig::default();
        cfg.latent_dim = 64;
        assert!(matches!(
            cfg.validate(),
            Err(WorldModelError::LatentDimMismatch { got: 64, .. })
        ));
    }
}
