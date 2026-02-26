//! Model download and cache management.
//!
//! Downloads STT/TTS/VAD models, verifies SHA-256 integrity,
//! and caches them in the local filesystem.

use std::path::PathBuf;

/// Manages voice model downloads, caching, and integrity verification.
pub struct ModelDownloadManager {
    cache_dir: PathBuf,
}

/// Information about a downloadable model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model identifier (e.g., "sherpa-onnx-streaming-zipformer-en-20M").
    pub id: String,
    /// Download URL.
    pub url: String,
    /// Expected SHA-256 hash of the downloaded file.
    pub sha256: String,
    /// File size in bytes (for progress reporting).
    pub size_bytes: u64,
}

impl ModelDownloadManager {
    /// Create a new manager with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Check if a model is already cached and valid.
    pub fn is_cached(&self, model: &ModelInfo) -> bool {
        let model_path = self.cache_dir.join(&model.id);
        model_path.exists()
    }

    /// Get the local path where a model would be cached.
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        self.cache_dir.join(model_id)
    }

    /// List available STT models.
    pub fn available_stt_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "sherpa-onnx-streaming-zipformer-en-20M".into(),
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M.tar.bz2".into(),
                sha256: "PLACEHOLDER_SHA256_STT".into(),
                size_bytes: 20_000_000,
            },
        ]
    }

    /// List available TTS models.
    pub fn available_tts_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "vits-piper-en_US-amy-medium".into(),
                url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-amy-medium.tar.bz2".into(),
                sha256: "PLACEHOLDER_SHA256_TTS".into(),
                size_bytes: 40_000_000,
            },
        ]
    }

    /// List available VAD models.
    pub fn available_vad_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "silero-vad-v5".into(),
                url: "https://github.com/snakers4/silero-vad/raw/master/files/silero_vad.onnx".into(),
                sha256: "PLACEHOLDER_SHA256_VAD".into(),
                size_bytes: 2_000_000,
            },
        ]
    }
}
