//! Wake word detection for "Hey Weft" trigger phrase.
//!
//! Provides the [`WakeWordDetector`] that processes audio frames and
//! fires a detection event when the wake word is recognized.
//!
//! ## Disposition (WEFT-671 / WEFT-216)
//!
//! This module is the **only supported transitional surface** under
//! `clawft_plugin::voice`. Live caller: CLI `weft voice wake`
//! (`WakeDaemon` / [`WakeWordConfig`]). Product Talk Mode does **not**
//! use this path — see `clawft-voice-talk` and `clawft-channels::voice`.
//!
//! ### Engine status (WEFT-216)
//!
//! | Backend | Status | Feature |
//! |---------|--------|---------|
//! | **Stub** (default) | Shipped. [`WakeWordDetector::process_frame`] always returns `false`. | `voice-wake` (via `voice` umbrella) |
//! | **rustpotter** | **Blocked** — crates.io `rustpotter` 3.0.2 depends on `candle-core` 0.2.2, which fails to compile on the workspace toolchain (rand 0.8/0.9 dual + `half`/`bf16` trait bounds). No dep is pulled. | `voice-wake-rustpotter` (API reserve; fails closed) |
//! | **OpenWakeWord / ONNX KWS** | **Chosen concrete alternative** when a live engine ships. Reuse `clawft-voice-onnx` (`ort`) + a trained ONNX wake model. Not wired yet; tracked as follow-up after model + capture land. | future `voice-wake-onnx` (not declared yet) |
//!
//! There is **no** shipped `models/voice/wake/hey-weft.rpw` (or ONNX
//! equivalent). Training / model integrity is blocked by SC-7 and by the
//! lack of multi-speaker sample set. CPU-budget auto-throttle and
//! recorded-utterance integration tests require a live engine + capture
//! loop and are deferred with the engine.
//!
//! See `docs/plans/wave-0k-WEFT-216-result.md` for the full decision,
//! blockers, and revisit triggers.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::PluginError;

/// Which wake-word engine is active for a detector instance.
///
/// Default (and currently only live) backend is [`WakeWordBackend::Stub`].
/// Additional variants are reserved for when an engine feature compiles
/// and a model is available.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeWordBackend {
    /// No neural / DTW engine. [`WakeWordDetector::process_frame`] never
    /// reports detection. Safe default for CI and for hosts without models.
    Stub,
}

impl WakeWordBackend {
    /// Human-readable name for logs and CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            WakeWordBackend::Stub => "stub",
        }
    }

    /// Whether this backend can ever return `true` from `process_frame`.
    pub fn can_detect(self) -> bool {
        match self {
            WakeWordBackend::Stub => false,
        }
    }
}

impl std::fmt::Display for WakeWordBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Configuration for the wake word detector.
///
/// # WEFT-240 — unify with product [`clawft_types::config::WakeConfig`]
///
/// | Surface | Field | Semantics |
/// |---------|-------|-----------|
/// | Product `WakeConfig` | `sensitivity` | Higher = **more** sensitive |
/// | This detector | `threshold` | Higher = **harder** to trigger |
///
/// Mapping (also on `WakeConfig::to_detector_threshold`):
/// `threshold = (1.0 - sensitivity).clamp(0.0, 1.0)`.
///
/// Prefer [`WakeWordConfig::from_wake_config`] when loading product config.
/// Serde alias `sensitivity` on `threshold` means **detector score threshold**,
/// not product sensitivity (legacy detector JSON only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordConfig {
    /// Path to the wake word model file (`.rpw` for rustpotter, or
    /// future `.onnx` for OpenWakeWord-style KWS).
    ///
    /// Default path is historical (`models/voice/wake/hey-weft.rpw`).
    /// **That file is not shipped** (WEFT-216). Loading a real engine
    /// will fail closed until a model is trained and integrity-checked.
    ///
    /// Type is [`PathBuf`]; product [`clawft_types::config::WakeConfig`]
    /// stores `Option<String>` — bridge via [`WakeWordConfig::from_wake_config`]
    /// (WEFT-234).
    #[serde(default = "default_model_path")]
    pub model_path: PathBuf,

    /// Detection score threshold (0.0–1.0). **Lower = more sensitive.**
    ///
    /// Product configs should set `WakeConfig.sensitivity` and convert with
    /// [`WakeWordConfig::from_wake_config`] (WEFT-240).
    #[serde(default = "default_threshold", alias = "sensitivity")]
    pub threshold: f32,

    /// Minimum gap between detections in frames.
    #[serde(default = "default_min_gap")]
    pub min_gap_frames: usize,

    /// Audio sample rate.
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,

    /// Whether to log detection events.
    #[serde(default = "default_true")]
    pub log_detections: bool,
}

fn default_model_path() -> PathBuf {
    PathBuf::from("models/voice/wake/hey-weft.rpw")
}
fn default_threshold() -> f32 {
    0.5
}
fn default_min_gap() -> usize {
    30
}
fn default_sample_rate() -> u32 {
    16000
}
fn default_true() -> bool {
    true
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            model_path: default_model_path(),
            threshold: default_threshold(),
            min_gap_frames: default_min_gap(),
            sample_rate: default_sample_rate(),
            log_detections: default_true(),
        }
    }
}

impl WakeWordConfig {
    /// Build detector config from product [`clawft_types::config::WakeConfig`]
    /// (WEFT-240 sensitivity↔threshold + WEFT-234 model_path bridge).
    ///
    /// - `threshold = wake.to_detector_threshold()` (`1.0 - sensitivity`)
    /// - `model_path` from `wake.model_path` (`Option<String>` → `PathBuf`),
    ///   or the historical default when unset
    pub fn from_wake_config(wake: &clawft_types::config::WakeConfig) -> Self {
        Self {
            model_path: wake
                .model_path_buf()
                .unwrap_or_else(default_model_path),
            threshold: wake.to_detector_threshold(),
            min_gap_frames: default_min_gap(),
            sample_rate: default_sample_rate(),
            log_detections: default_true(),
        }
    }

    /// Product sensitivity corresponding to this detector threshold.
    pub fn to_product_sensitivity(&self) -> f32 {
        clawft_types::config::WakeConfig::sensitivity_from_detector_threshold(self.threshold)
    }
}

/// Events emitted by the wake word detector.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WakeWordEvent {
    /// Wake word was detected with the given confidence.
    Detected {
        /// Detection confidence score (0.0-1.0).
        confidence: f32,
    },
    /// Detector started listening.
    Started,
    /// Detector stopped listening.
    Stopped,
    /// An error occurred during detection.
    Error {
        /// Error description.
        message: String,
    },
}

/// Why a non-stub engine is unavailable at compile or runtime time.
///
/// Stable string for docs, tests, and CLI. Do not rephrase casually —
/// tests match substrings.
pub const RUSTPOTTER_BLOCKED_REASON: &str = "rustpotter blocked (WEFT-216): crates.io rustpotter \
3.0.2 depends on candle-core 0.2.2 which fails to compile on this workspace toolchain \
(rand 0.8/0.9 dual + half/bf16 SampleUniform). No rustpotter dep is linked. \
Concrete alternative: OpenWakeWord-style ONNX KWS via clawft-voice-onnx (follow-up). \
See docs/plans/wave-0k-WEFT-216-result.md";

/// Compile-time: is the experimental `voice-wake-rustpotter` feature enabled?
///
/// Even when `true`, the engine is **not live** — see
/// [`RUSTPOTTER_BLOCKED_REASON`]. This only means the reserved fail-closed
/// API is compiled in.
pub const fn rustpotter_feature_enabled() -> bool {
    cfg!(feature = "voice-wake-rustpotter")
}

/// Wake word detector.
///
/// Default construction yields a **stub** backend ([`WakeWordBackend::Stub`]):
/// `process_frame` always returns `false`. That is intentional and honest —
/// not a silent half-integration.
///
/// Real detection requires an unblocked engine feature **and** a trained
/// model under `model_path` (neither is available in-tree today).
pub struct WakeWordDetector {
    config: WakeWordConfig,
    backend: WakeWordBackend,
    running: bool,
    /// Frames processed since the last reported detection (or start).
    /// Reserved for min-gap enforcement once a live engine exists.
    frames_since_detection: usize,
}

impl WakeWordDetector {
    /// Create a new wake word detector with the given configuration.
    ///
    /// Always constructs the **stub** backend. Does not attempt to load
    /// rustpotter or any model file (model is not shipped; engine blocked).
    pub fn new(config: WakeWordConfig) -> Result<Self, PluginError> {
        info!(
            model = %config.model_path.display(),
            threshold = config.threshold,
            backend = %WakeWordBackend::Stub,
            "wake word detector created (stub; WEFT-216 engine deferred)"
        );
        Ok(Self {
            config,
            backend: WakeWordBackend::Stub,
            running: false,
            frames_since_detection: 0,
        })
    }

    /// Attempt to construct a detector with the rustpotter engine.
    ///
    /// **Always fails closed** today (WEFT-216): rustpotter does not compile
    /// on this workspace. The `voice-wake-rustpotter` feature only reserves
    /// this API surface so callers can feature-gate against a future unlock.
    ///
    /// Without the feature, returns [`PluginError::NotImplemented`] naming
    /// the missing feature. With the feature, returns the same error class
    /// with [`RUSTPOTTER_BLOCKED_REASON`].
    ///
    /// Callers that need a [`WakeWordEvent::Error`] for UI/daemon broadcast
    /// should use [`WakeWordDetector::error_event_for_rustpotter_block`].
    pub fn try_with_rustpotter(_config: WakeWordConfig) -> Result<Self, PluginError> {
        #[cfg(feature = "voice-wake-rustpotter")]
        {
            tracing::warn!(reason = RUSTPOTTER_BLOCKED_REASON, "try_with_rustpotter");
            Err(PluginError::NotImplemented(RUSTPOTTER_BLOCKED_REASON.into()))
        }
        #[cfg(not(feature = "voice-wake-rustpotter"))]
        {
            Err(PluginError::NotImplemented(
                "voice-wake-rustpotter feature not enabled; engine still blocked (WEFT-216). \
                 Enable the feature only to exercise the fail-closed path; see \
                 docs/plans/wave-0k-WEFT-216-result.md"
                    .into(),
            ))
        }
    }

    /// [`WakeWordEvent::Error`] describing the rustpotter block (WEFT-234).
    ///
    /// Emitted by the wake daemon when a non-stub engine is requested but
    /// unavailable, so the Error variant is reached on a live code path.
    pub fn error_event_for_rustpotter_block() -> WakeWordEvent {
        WakeWordEvent::Error {
            message: RUSTPOTTER_BLOCKED_REASON.into(),
        }
    }

    /// Build an [`WakeWordEvent::Error`] with an arbitrary message.
    pub fn error_event(message: impl Into<String>) -> WakeWordEvent {
        WakeWordEvent::Error {
            message: message.into(),
        }
    }

    /// Active backend for this instance.
    pub fn backend(&self) -> WakeWordBackend {
        self.backend
    }

    /// `true` when this detector cannot detect (current production path).
    pub fn is_stub(&self) -> bool {
        !self.backend.can_detect()
    }

    /// Process a single audio frame. Returns `true` if wake word detected.
    ///
    /// **Stub:** always returns `false`. Counts frames while running so a
    /// future engine can enforce `min_gap_frames` without API changes.
    /// Samples are ignored.
    pub fn process_frame(&mut self, samples: &[i16]) -> bool {
        if !self.running {
            debug!("wake word: process_frame while stopped (ignored)");
            return false;
        }
        self.frames_since_detection = self.frames_since_detection.saturating_add(1);
        match self.backend {
            WakeWordBackend::Stub => {
                debug!(
                    samples = samples.len(),
                    frames_since = self.frames_since_detection,
                    "wake word: processing frame (stub backend, no detection)"
                );
                false
            }
        }
    }

    /// Start listening for the wake word.
    pub fn start(&mut self) -> WakeWordEvent {
        self.running = true;
        self.frames_since_detection = 0;
        info!(backend = %self.backend, "wake word detector started");
        WakeWordEvent::Started
    }

    /// Stop listening for the wake word.
    pub fn stop(&mut self) -> WakeWordEvent {
        self.running = false;
        info!(backend = %self.backend, "wake word detector stopped");
        WakeWordEvent::Stopped
    }

    /// Check if the detector is currently running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get the current configuration.
    pub fn config(&self) -> &WakeWordConfig {
        &self.config
    }

    /// Frames observed since last detection / start (stub accounting).
    pub fn frames_since_detection(&self) -> usize {
        self.frames_since_detection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_word_config_defaults() {
        let config = WakeWordConfig::default();
        assert_eq!(
            config.model_path,
            PathBuf::from("models/voice/wake/hey-weft.rpw")
        );
        assert!((config.threshold - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.min_gap_frames, 30);
        assert_eq!(config.sample_rate, 16000);
        assert!(config.log_detections);
    }

    #[test]
    fn wake_word_config_serde_roundtrip() {
        let config = WakeWordConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: WakeWordConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.model_path, restored.model_path);
        assert!((config.threshold - restored.threshold).abs() < f32::EPSILON);
        assert_eq!(config.min_gap_frames, restored.min_gap_frames);
        assert_eq!(config.sample_rate, restored.sample_rate);
        assert_eq!(config.log_detections, restored.log_detections);
    }

    #[test]
    fn wake_word_config_custom_values() {
        let json = r#"{
            "model_path": "/custom/model.rpw",
            "threshold": 0.8,
            "min_gap_frames": 50,
            "sample_rate": 48000,
            "log_detections": false
        }"#;
        let config: WakeWordConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.model_path, PathBuf::from("/custom/model.rpw"));
        assert!((config.threshold - 0.8).abs() < f32::EPSILON);
        assert_eq!(config.min_gap_frames, 50);
        assert_eq!(config.sample_rate, 48000);
        assert!(!config.log_detections);
    }

    #[test]
    fn wake_word_detector_create_is_stub() {
        let config = WakeWordConfig::default();
        let detector = WakeWordDetector::new(config).unwrap();
        assert!(!detector.is_running());
        assert!(detector.is_stub());
        assert_eq!(detector.backend(), WakeWordBackend::Stub);
        assert!(!detector.backend().can_detect());
        assert_eq!(detector.backend().as_str(), "stub");
    }

    #[test]
    fn wake_word_detector_start_stop_lifecycle() {
        let config = WakeWordConfig::default();
        let mut detector = WakeWordDetector::new(config).unwrap();

        // Initially not running.
        assert!(!detector.is_running());

        // Start.
        let event = detector.start();
        assert!(matches!(event, WakeWordEvent::Started));
        assert!(detector.is_running());

        // Stop.
        let event = detector.stop();
        assert!(matches!(event, WakeWordEvent::Stopped));
        assert!(!detector.is_running());
    }

    #[test]
    fn wake_word_detector_process_frame_returns_false() {
        let config = WakeWordConfig::default();
        let mut detector = WakeWordDetector::new(config).unwrap();
        detector.start();

        let samples = vec![0i16; 512];
        assert!(!detector.process_frame(&samples));
        assert_eq!(detector.frames_since_detection(), 1);
        assert!(!detector.process_frame(&samples));
        assert_eq!(detector.frames_since_detection(), 2);
    }

    #[test]
    fn wake_word_detector_process_frame_while_stopped_is_noop() {
        let config = WakeWordConfig::default();
        let mut detector = WakeWordDetector::new(config).unwrap();
        let samples = vec![0i16; 64];
        assert!(!detector.process_frame(&samples));
        assert_eq!(detector.frames_since_detection(), 0);
    }

    #[test]
    fn wake_word_detector_config_accessor() {
        let config = WakeWordConfig {
            threshold: 0.75,
            ..Default::default()
        };
        let detector = WakeWordDetector::new(config).unwrap();
        assert!((detector.config().threshold - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn wake_word_event_serde_detected() {
        let event = WakeWordEvent::Detected { confidence: 0.95 };
        let json = serde_json::to_string(&event).unwrap();
        let restored: WakeWordEvent = serde_json::from_str(&json).unwrap();
        match restored {
            WakeWordEvent::Detected { confidence } => {
                assert!((confidence - 0.95).abs() < f32::EPSILON);
            }
            _ => panic!("expected Detected variant"),
        }
    }

    #[test]
    fn wake_word_event_serde_started() {
        let event = WakeWordEvent::Started;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"started\""));
        let restored: WakeWordEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, WakeWordEvent::Started));
    }

    #[test]
    fn wake_word_event_serde_stopped() {
        let event = WakeWordEvent::Stopped;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"stopped\""));
        let restored: WakeWordEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, WakeWordEvent::Stopped));
    }

    #[test]
    fn wake_word_event_serde_error() {
        let event = WakeWordEvent::Error {
            message: "model not found".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: WakeWordEvent = serde_json::from_str(&json).unwrap();
        match restored {
            WakeWordEvent::Error { message } => {
                assert_eq!(message, "model not found");
            }
            _ => panic!("expected Error variant"),
        }
    }

    #[test]
    fn wake_word_event_all_variants_serialize() {
        let events = vec![
            WakeWordEvent::Detected { confidence: 0.5 },
            WakeWordEvent::Started,
            WakeWordEvent::Stopped,
            WakeWordEvent::Error {
                message: "test".into(),
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let _: WakeWordEvent = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn try_with_rustpotter_fails_closed() {
        match WakeWordDetector::try_with_rustpotter(WakeWordConfig::default()) {
            Ok(_) => panic!("expected NotImplemented, got Ok"),
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("not implemented")
                        || msg.contains("WEFT-216")
                        || msg.contains("voice-wake-rustpotter")
                        || msg.contains("rustpotter"),
                    "unexpected error: {msg}"
                );
            }
        }
    }

    #[test]
    fn from_wake_config_maps_sensitivity_and_model_path() {
        let product = clawft_types::config::WakeConfig {
            enabled: true,
            phrase: "hey weft".into(),
            sensitivity: 0.8,
            model_path: Some("models/custom/wake.onnx".into()),
        };
        let cfg = WakeWordConfig::from_wake_config(&product);
        assert!((cfg.threshold - 0.2).abs() < f32::EPSILON);
        assert!((cfg.to_product_sensitivity() - 0.8).abs() < f32::EPSILON);
        assert_eq!(
            cfg.model_path,
            PathBuf::from("models/custom/wake.onnx")
        );
    }

    #[test]
    fn from_wake_config_default_model_when_unset() {
        let product = clawft_types::config::WakeConfig::default();
        let cfg = WakeWordConfig::from_wake_config(&product);
        assert_eq!(cfg.model_path, default_model_path());
        assert!((cfg.threshold - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn error_event_for_rustpotter_block_is_error_variant() {
        match WakeWordDetector::error_event_for_rustpotter_block() {
            WakeWordEvent::Error { message } => {
                assert!(message.contains("WEFT-216"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn rustpotter_blocked_reason_is_stable() {
        assert!(RUSTPOTTER_BLOCKED_REASON.contains("WEFT-216"));
        assert!(RUSTPOTTER_BLOCKED_REASON.contains("candle-core"));
        assert!(RUSTPOTTER_BLOCKED_REASON.contains("OpenWakeWord"));
    }

    #[test]
    fn backend_display_and_serde() {
        let b = WakeWordBackend::Stub;
        assert_eq!(b.to_string(), "stub");
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("stub"));
        let restored: WakeWordBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, WakeWordBackend::Stub);
    }

    #[test]
    fn rustpotter_feature_flag_matches_cfg() {
        // Document compile-time gate; both branches are valid.
        let enabled = rustpotter_feature_enabled();
        assert_eq!(enabled, cfg!(feature = "voice-wake-rustpotter"));
    }
}
