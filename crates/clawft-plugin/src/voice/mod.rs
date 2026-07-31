//! # Voice pipeline scaffold (legacy / transitional) — WEFT-671
//!
//! All types are gated behind the `voice` feature flag.
//! Sub-features (`voice-stt`, `voice-tts`, `voice-vad`, `voice-wake`)
//! control which pipeline components are compiled.
//!
//! ## Disposition (WEFT-671 / docs/plans/wave-0a-WEFT-671-decision.md)
//!
//! **This module is NOT the product voice stack.** Canonical Talk Mode,
//! STT, TTS, VAD, barge-in, and AEC live in:
//!
//! | Surface | Crate / path |
//! |---------|----------------|
//! | Talk Mode / edge loop | `clawft-voice-talk`, `clawft-channels::voice` |
//! | TTS engines | `clawft-voice-tts` |
//! | ONNX helpers | `clawft-voice-onnx` |
//! | Echo cancel / NS | `clawft-voice-aec` |
//! | Substrate STT (canonical) | `clawft-service-whisper` (ADR-053) |
//! | Full-duplex floor | ADR-068 (`DuplexChannel` / `VoiceEdge`) |
//!
//! ### What remains supported here
//!
//! - **`wake` / `wake_daemon`** (`WakeWordConfig`, `WakeDaemon`,
//!   `WakeWordDetector`) — sole **live external caller**: CLI
//!   `weft voice wake` (`clawft-cli::commands::voice`). **Stub backend**
//!   (WEFT-216): rustpotter blocked on candle-core; concrete alternative
//!   is OpenWakeWord-style ONNX KWS via `clawft-voice-onnx` (not wired).
//!   Feature `voice-wake-rustpotter` reserves a fail-closed API. See
//!   `docs/plans/wave-0k-WEFT-216-result.md`.
//!
//! ### What is deprecated scaffold
//!
//! Everything else under this directory (capture, playback, stt, tts,
//! channel, talk_mode, echo, noise, cloud_*, fallback, commands, …) is
//! **feature-gated legacy scaffold** retained for compile coverage under
//! `--features voice` (WEFT-212 umbrella) and for historical tests.
//! Product Talk Mode / STT / TTS work stays on the live stack above.
//!
//! **WEFT-217 exception:** `echo` and `noise` implement real pure-Rust DSP
//! (NLMS AEC + spectral subtraction) so they are not deceptive passthroughs.
//! Production full-duplex still prefers `clawft-voice-aec` (WebRTC AEC3/NS).
//!
//! Follow-up (see decision note): migrate wake into a canonical crate,
//! then delete or archive the remaining scaffold.
//!
//! Related: ADR-053, ADR-061, ADR-068; audit
//! `.planning/reviews/0.7.0-release-gate/10-voice.md`.

pub mod config;

#[cfg(feature = "voice-vad")]
pub mod capture;
#[cfg(feature = "voice-vad")]
pub mod playback;
#[cfg(feature = "voice-vad")]
pub mod privacy_indicator;
#[cfg(feature = "voice-vad")]
pub mod vad;

// WEFT-230: re-export adaptive silence types at the voice surface so
// callers do not need to dig into clawft-types directly.
pub use clawft_types::config::{AdaptiveSilenceConfig, AdaptiveSilenceTimeout};

#[cfg(feature = "voice-stt")]
pub mod stt;

#[cfg(feature = "voice-tts")]
pub mod tts;

pub mod models;

#[cfg(feature = "voice-wake")]
pub mod wake;
#[cfg(feature = "voice-wake")]
pub mod wake_daemon;

pub mod channel;
pub mod echo;
pub mod events;
pub mod noise;
pub mod quality;
pub mod talk_mode;

pub mod cloud_stt;
pub mod cloud_tts;
pub mod commands;
/// SC-6 / WEFT-225: anti-replay nonce + transcription-echo confirmation.
pub mod confirm;
pub mod fallback;
/// SC-8 / WEFT-226: command/wake token buckets + post-fail cooldown.
pub mod rate_limit;
pub mod transcript_log;

// Re-export key types.
//
// Wake types are the supported transitional surface (CLI `weft voice wake`).
// All other re-exports are legacy scaffold — prefer clawft-channels /
// clawft-voice-* for new work (WEFT-671).

// SC-2 / WEFT-223: re-export retention + secure buffer types for callers.
pub use clawft_types::config::AudioRetention;
pub use clawft_types::{SecureAudioBuffer, SecureAudioRing};

pub use channel::{VoiceChannel, VoiceStatus};
pub use commands::{
    CommandDispatch, VoiceCommand, VoiceCommandRegistry, VoiceCommandSession, levenshtein_distance,
};
pub use config::{VoiceAudioConfig, VoiceCaptureSpec, VoicePipelineConfig, VoicePlaybackSpec};
pub use confirm::{
    CONFIRM_TARGET, ConfirmResult, ConfirmationGate, EntropyNonceSource, FixedNonceSource,
    NonceSource, PendingConfirmation, build_confirm_prompt, command_requires_confirm,
    matches_confirm_nonce,
};
pub use echo::{EchoCanceller, EchoCancellerConfig};
pub use events::VoiceWsEvent;
pub use rate_limit::{
    RATE_LIMIT_TARGET, VoiceRateDecision, VoiceRateKind, VoiceRateLimiter,
};
pub use models::{
    EnsureOutcome, ModelDownloadManager, ModelInfo, finish_stderr_progress, is_placeholder_hash,
    sha256_hex_bytes, sha256_hex_file, stderr_progress_line,
};
pub use noise::{NoiseSuppressor, NoiseSuppressorConfig};
pub use quality::{AudioMetrics, analyze_frame};
pub use talk_mode::TalkModeController;

#[cfg(feature = "voice-wake")]
pub use wake::{
    RUSTPOTTER_BLOCKED_REASON, WakeWordBackend, WakeWordConfig, WakeWordDetector, WakeWordEvent,
    rustpotter_feature_enabled,
};
#[cfg(feature = "voice-wake")]
pub use wake_daemon::WakeDaemon;

#[cfg(feature = "voice-vad")]
pub use vad::{VadEvent, VoiceActivityDetector};
