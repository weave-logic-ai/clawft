//! Voice-specific configuration helpers.
//!
//! Re-exports `VoiceConfig` from `clawft-types` and provides
//! runtime configuration for the voice pipeline.

/// Runtime configuration for the voice pipeline.
/// Wraps the serializable VoiceConfig with runtime state.
#[derive(Debug, Clone)]
pub struct VoicePipelineConfig {
    /// Model cache directory path.
    pub model_cache_dir: std::path::PathBuf,
    /// Whether voice pipeline is active.
    pub active: bool,
}

impl Default for VoicePipelineConfig {
    fn default() -> Self {
        Self {
            model_cache_dir: default_model_cache_dir(),
            active: false,
        }
    }
}

fn default_model_cache_dir() -> std::path::PathBuf {
    // Use ~/.clawft/models/voice/ as default
    if let Some(home) = dirs_fallback() {
        home.join(".clawft").join("models").join("voice")
    } else {
        std::path::PathBuf::from(".clawft/models/voice")
    }
}

fn dirs_fallback() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(std::path::PathBuf::from)
}
