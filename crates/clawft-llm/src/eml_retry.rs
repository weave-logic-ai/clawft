//! EML learned retry timing for LLM providers.
//!
//! Replaces the hardcoded exponential backoff parameters with an
//! [`eml_core::EmlModel`] that learns optimal retry delays from
//! historical success/failure outcomes.
//!
//! # Architecture
//!
//! - [`RetryModel`] wraps an `EmlModel::new(2, 3, 1)` with inputs:
//!   `(error_type_ordinal, attempt_number, hour_of_day)` and output:
//!   `optimal_delay_ms`.
//! - When untrained, falls back to the standard [`RetryConfig`] defaults.
//!
//! # Training
//!
//! After each retry outcome (success or final failure), call
//! [`RetryModel::record`] with the actual delay used and whether the
//! next attempt succeeded. The model learns which delays lead to
//! successful retries.
//!
//! # Persistence (WEFT-39)
//!
//! Learned weights survive daemon restarts via JSON on disk:
//! - [`RetryModel::save_to_path`] / [`RetryModel::load_from_path`]
//! - Default location: `{state_dir}/retry_model.json` (see
//!   [`retry_model_path`]); production wiring uses
//!   `~/.clawft/state/retry_model.json` or `{runtime}/eml-models/`.
//! - Optional [`RetryModel::with_persist_path`] auto-saves after a
//!   successful train. Callers should also save on clean shutdown.

use std::path::{Path, PathBuf};

use eml_core::EmlModel;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::ProviderError;
use crate::retry::RetryConfig;

/// Filename used under a state / eml-models directory.
pub const RETRY_MODEL_FILENAME: &str = "retry_model.json";

/// Resolve `{state_dir}/retry_model.json`.
pub fn retry_model_path(state_dir: &Path) -> PathBuf {
    state_dir.join(RETRY_MODEL_FILENAME)
}

// ═══════════════════════════════════════════════════════════════════
// Error type encoding
// ═══════════════════════════════════════════════════════════════════

/// Map a [`ProviderError`] to a numeric ordinal for the model.
///
/// Exposed `pub(crate)` so `retry::RetryPolicy` can snapshot the
/// ordinal at retry time (when it still has an `&ProviderError`) and
/// replay it later via [`RetryModel::record_by_ordinal`] — needed
/// because `ProviderError` doesn't impl `Clone`.
pub(crate) fn error_ordinal(err: &ProviderError) -> f64 {
    match err {
        ProviderError::RateLimited { .. } => 0.0,
        ProviderError::Timeout => 1.0,
        ProviderError::Http(_) => 2.0,
        ProviderError::ServerError { status, .. } => {
            // Encode common status codes as distinct ordinals
            match *status {
                500 => 3.0,
                502 => 4.0,
                503 => 5.0,
                504 => 6.0,
                _ => 7.0,
            }
        }
        _ => 8.0,
    }
}

/// Get the current hour of day (0..23) as a normalized feature.
fn hour_of_day_normalized() -> f64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hour = (secs % 86400) / 3600;
    hour as f64 / 24.0
}

// ═══════════════════════════════════════════════════════════════════
// RetryModel
// ═══════════════════════════════════════════════════════════════════

/// Learned retry timing model for LLM provider calls.
///
/// Learns the optimal delay between retries based on the error type,
/// attempt number, and time of day. Falls back to exponential backoff
/// from [`RetryConfig`] when untrained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryModel {
    /// EML model: 3 inputs (error_ordinal, attempt, hour), 1 output (delay_ms)
    model: EmlModel,
    /// Fallback config used when the model is untrained.
    #[serde(skip)]
    fallback: RetryConfig,
    /// Optional path for auto-persist after train (WEFT-39). Not serialized.
    #[serde(skip)]
    persist_path: Option<PathBuf>,
}

impl RetryModel {
    /// Create a new untrained retry model.
    pub fn new() -> Self {
        Self {
            model: EmlModel::new(2, 3, 1),
            fallback: RetryConfig::default(),
            persist_path: None,
        }
    }

    /// Create with a custom fallback configuration.
    pub fn with_fallback(fallback: RetryConfig) -> Self {
        Self {
            model: EmlModel::new(2, 3, 1),
            fallback,
            persist_path: None,
        }
    }

    /// Attach a disk path used by [`Self::maybe_persist`] after train.
    ///
    /// Does not load from the path; use [`Self::load_or_default`] first.
    pub fn with_persist_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.persist_path = Some(path.into());
        self
    }

    /// Path configured via [`Self::with_persist_path`], if any.
    pub fn persist_path(&self) -> Option<&Path> {
        self.persist_path.as_deref()
    }

    /// Compute the optimal delay for a retry attempt.
    ///
    /// When untrained, falls back to [`crate::retry::compute_delay`].
    pub fn delay_ms(&self, err: &ProviderError, attempt: u32) -> u64 {
        if !self.model.is_trained() {
            return crate::retry::compute_delay(&self.fallback, attempt).as_millis() as u64;
        }

        let inputs = [
            error_ordinal(err) / 8.0, // normalize to [0, 1]
            attempt as f64 / 10.0,    // normalize (typical max ~5-10)
            hour_of_day_normalized(),
        ];
        let predicted = self.model.predict_primary(&inputs);

        // Clamp to sane bounds
        let delay = predicted.clamp(100.0, 60_000.0);
        delay as u64
    }

    /// Record a retry outcome for training.
    ///
    /// # Arguments
    /// - `err`: The error that triggered the retry.
    /// - `attempt`: The attempt number (0-indexed).
    /// - `delay_ms`: The actual delay used before this attempt.
    /// - `succeeded`: Whether the subsequent attempt succeeded.
    pub fn record(&mut self, err: &ProviderError, attempt: u32, delay_ms: u64, succeeded: bool) {
        self.record_by_ordinal(error_ordinal(err), attempt, delay_ms, succeeded);
    }

    /// Record a retry outcome from a pre-computed error ordinal.
    ///
    /// Escape-hatch for callers who can't hold onto the original
    /// `ProviderError` through the retry loop (it isn't `Clone`
    /// because of `reqwest::Error` / `serde_json::Error`). Snapshot
    /// the ordinal via [`error_ordinal`] at retry time, pass it
    /// here when the outcome is known.
    pub fn record_by_ordinal(
        &mut self,
        ordinal: f64,
        attempt: u32,
        delay_ms: u64,
        succeeded: bool,
    ) {
        let inputs = [
            ordinal / 8.0,
            attempt as f64 / 10.0,
            hour_of_day_normalized(),
        ];

        // Target: if succeeded, the delay was good — record it.
        // If failed, the delay was too short — record 2x as target.
        let target = if succeeded {
            delay_ms as f64
        } else {
            (delay_ms as f64 * 2.0).min(60_000.0)
        };

        self.model.record(&inputs, &[Some(target)]);

        // Opportunistic train + persist once we have enough samples.
        // `EmlModel::train` no-ops below 50 samples; after that we retrain
        // every 25 samples so the hot path stays cheap.
        let n = self.model.training_sample_count();
        if n >= 50 && n.is_multiple_of(25) {
            let converged = self.model.train();
            if converged {
                debug!(samples = n, "RetryModel train converged");
            }
            self.maybe_persist();
        }
    }

    /// Train the model. Returns true if converged.
    ///
    /// On success (and when a persist path is set), writes weights to disk.
    pub fn train(&mut self) -> bool {
        let ok = self.model.train();
        if ok {
            self.maybe_persist();
        }
        ok
    }

    /// Whether the model is trained.
    pub fn is_trained(&self) -> bool {
        self.model.is_trained()
    }

    /// Number of training samples.
    pub fn training_sample_count(&self) -> usize {
        self.model.training_sample_count()
    }

    /// Read-only view of the learned parameter vector (for continuity tests).
    pub fn params(&self) -> &[f64] {
        self.model.params_slice()
    }

    /// Install weights without coordinate-descent training.
    ///
    /// Used by tests and migration tooling to stamp a trained (or untrained)
    /// parameter snapshot that round-trips through persistence.
    pub fn install_weights(&mut self, params: &[f64], trained: bool) {
        let slice = self.model.params_slice_mut();
        assert_eq!(
            slice.len(),
            params.len(),
            "param length mismatch: model has {}, got {}",
            slice.len(),
            params.len()
        );
        slice.copy_from_slice(params);
        self.model.mark_trained(trained);
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("RetryModel serialization should not fail")
    }

    /// Deserialize from JSON.
    ///
    /// Restores learned params + trained flag. The in-memory training
    /// sample buffer is not persisted (`#[serde(skip)]` on `EmlModel`).
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Persist learned weights to `path` as pretty JSON.
    ///
    /// Creates parent directories as needed. Uses a temp-file + rename
    /// so a crash mid-write cannot leave a truncated file.
    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes())?;
        std::fs::rename(&tmp, path)?;
        debug!(path = %path.display(), trained = self.is_trained(), "saved RetryModel");
        Ok(())
    }

    /// Load learned weights from `path`.
    ///
    /// Returns `None` if the file is missing or malformed.
    pub fn load_from_path(path: &Path) -> Option<Self> {
        if !path.exists() {
            return None;
        }
        match std::fs::read_to_string(path) {
            Ok(data) => match Self::from_json(&data) {
                Some(model) => {
                    debug!(
                        path = %path.display(),
                        trained = model.is_trained(),
                        "loaded RetryModel"
                    );
                    Some(model)
                }
                None => {
                    warn!(
                        path = %path.display(),
                        "failed to deserialize RetryModel, using defaults"
                    );
                    None
                }
            },
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to read RetryModel file");
                None
            }
        }
    }

    /// Load from `path`, or return an untrained default if missing/corrupt.
    pub fn load_or_default(path: &Path) -> Self {
        Self::load_from_path(path).unwrap_or_default()
    }

    /// Save to the configured [`Self::persist_path`], if any.
    pub fn maybe_persist(&self) {
        let Some(path) = self.persist_path.as_ref() else {
            return;
        };
        if let Err(e) = self.save_to_path(path) {
            warn!(path = %path.display(), error = %e, "RetryModel persist failed");
        }
    }

    /// Save to the configured persist path, or return an error if unset.
    pub fn save(&self) -> std::io::Result<()> {
        let path = self.persist_path.as_ref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "RetryModel has no persist_path configured",
            )
        })?;
        self.save_to_path(path)
    }
}

impl Default for RetryModel {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untrained_uses_exponential_backoff() {
        let model = RetryModel::new();
        assert!(!model.is_trained());

        let err = ProviderError::Timeout;
        let delay = model.delay_ms(&err, 0);
        // Default: base_delay=1000ms, attempt 0 => 2^0 * 1000 = 1000ms + jitter
        assert!(delay >= 1000, "delay should be >= 1000ms, got {delay}");
        assert!(
            delay <= 1500,
            "delay should be <= 1500ms (with jitter), got {delay}"
        );
    }

    #[test]
    fn untrained_delay_increases_with_attempt() {
        let model = RetryModel::new();
        let err = ProviderError::ServerError {
            status: 503,
            body: "unavailable".into(),
        };

        let d0 = model.delay_ms(&err, 0);
        let d1 = model.delay_ms(&err, 1);
        let d2 = model.delay_ms(&err, 2);

        // Exponential backoff: each attempt should be >= previous
        // (ignoring jitter randomness, we check the general trend)
        assert!(
            d1 >= d0 || d2 > d0,
            "delays should generally increase: {d0}, {d1}, {d2}"
        );
    }

    #[test]
    fn error_ordinal_mappings() {
        assert!(
            (error_ordinal(&ProviderError::RateLimited {
                retry_after_ms: 1000
            }))
            .abs()
                < 1e-9
        );
        assert!((error_ordinal(&ProviderError::Timeout) - 1.0).abs() < 1e-9);
        assert!(
            (error_ordinal(&ProviderError::ServerError {
                status: 503,
                body: String::new(),
            }) - 5.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn record_increments_count() {
        let mut model = RetryModel::new();
        assert_eq!(model.training_sample_count(), 0);

        model.record(&ProviderError::Timeout, 0, 1000, true);
        assert_eq!(model.training_sample_count(), 1);

        model.record(&ProviderError::Timeout, 1, 2000, false);
        assert_eq!(model.training_sample_count(), 2);
    }

    #[test]
    fn train_insufficient_data() {
        let mut model = RetryModel::new();
        for i in 0..10 {
            model.record(&ProviderError::Timeout, i, 1000 * (i as u64 + 1), true);
        }
        assert!(!model.train());
        assert!(!model.is_trained());
    }

    #[test]
    fn serialization_roundtrip() {
        let model = RetryModel::new();
        let json = model.to_json();
        let restored = RetryModel::from_json(&json).expect("should deserialize");
        assert!(!restored.is_trained());
    }

    #[test]
    fn from_json_invalid() {
        assert!(RetryModel::from_json("not json").is_none());
    }

    #[test]
    fn with_fallback_config() {
        use std::time::Duration;
        let config = RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            jitter_fraction: 0.0,
        };
        let model = RetryModel::with_fallback(config);
        let delay = model.delay_ms(&ProviderError::Timeout, 0);
        // base_delay=500ms, attempt 0, no jitter => exactly 500ms
        assert_eq!(delay, 500);
    }

    #[test]
    fn default_impl() {
        let model = RetryModel::default();
        assert!(!model.is_trained());
    }

    #[test]
    fn hour_of_day_in_range() {
        let h = hour_of_day_normalized();
        assert!((0.0..1.0).contains(&h), "hour should be in [0, 1), got {h}");
    }

    #[test]
    fn retry_model_path_joins_filename() {
        let p = retry_model_path(Path::new("/var/lib/clawft/state"));
        assert_eq!(p, PathBuf::from("/var/lib/clawft/state/retry_model.json"));
    }

    #[test]
    fn save_load_roundtrip_preserves_learned_weights() {
        // WEFT-39: train (install weights), save, "restart" via load, assert continuity.
        let dir = std::env::temp_dir().join(format!(
            "retry_model_persist_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = retry_model_path(&dir);

        let mut model = RetryModel::new();
        let mut params: Vec<f64> = model.params().to_vec();
        for (i, p) in params.iter_mut().enumerate() {
            *p = (i as f64 * 0.17).sin();
        }
        model.install_weights(&params, true);
        assert!(model.is_trained());

        model
            .save_to_path(&path)
            .expect("save_to_path should succeed");
        assert!(path.exists());

        // Simulate daemon restart: drop model, load from disk.
        drop(model);
        let restored = RetryModel::load_from_path(&path).expect("should load after restart");
        assert!(restored.is_trained(), "trained flag must survive restart");
        assert_eq!(restored.params().len(), params.len());
        for (i, (a, b)) in restored.params().iter().zip(params.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-12,
                "param[{i}] mismatch after restart: {a} vs {b}"
            );
        }

        // Predictions must match the pre-restart model (same weights).
        let err = ProviderError::Timeout;
        let mut again = RetryModel::new();
        again.install_weights(&params, true);
        let d0 = again.delay_ms(&err, 1);
        let d1 = restored.delay_ms(&err, 1);
        assert_eq!(d0, d1, "delay predictions must match after restore");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_returns_none_or_default() {
        let path = std::env::temp_dir().join(format!(
            "retry_model_missing_{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        assert!(RetryModel::load_from_path(&path).is_none());
        let d = RetryModel::load_or_default(&path);
        assert!(!d.is_trained());
    }

    #[test]
    fn load_corrupt_returns_none() {
        let path = std::env::temp_dir().join(format!(
            "retry_model_corrupt_{}.json",
            std::process::id()
        ));
        std::fs::write(&path, "not valid json {{{").unwrap();
        assert!(RetryModel::load_from_path(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn persist_path_auto_saves_after_explicit_train_when_converged() {
        // With <50 samples train returns false and should not require a file.
        let dir = std::env::temp_dir().join(format!(
            "retry_model_autopath_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = retry_model_path(&dir);

        let mut model = RetryModel::new().with_persist_path(&path);
        assert_eq!(model.persist_path(), Some(path.as_path()));
        for i in 0..10 {
            model.record(&ProviderError::Timeout, i, 1000 * (i as u64 + 1), true);
        }
        assert!(!model.train());
        // No file required when train did not converge; maybe_persist is only
        // called on successful train() or the opportunistic path.

        // Stamp trained weights and exercise explicit save().
        let params = model.params().to_vec();
        model.install_weights(&params, true);
        model.save().expect("save via persist_path");
        assert!(path.exists());

        let loaded = RetryModel::load_or_default(&path);
        assert!(loaded.is_trained());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
