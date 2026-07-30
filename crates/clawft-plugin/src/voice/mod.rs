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
//!   `weft voice wake` (`clawft-cli::commands::voice`). Still a stub
//!   detector (always silent) but the API is the transitional home for
//!   wake until a follow-up migrates it into a `clawft-voice-*` crate.
//!
//! ### What is deprecated scaffold
//!
//! Everything else under this directory (capture, playback, stt, tts,
//! channel, talk_mode, echo, noise, cloud_*, fallback, commands, …) is
//! **feature-gated legacy scaffold** retained for compile coverage under
//! `--features voice` (WEFT-212 umbrella) and for historical tests. Do
//! **not** implement the 0.7 audit-era tickets against these files unless
//! the work is re-homed onto the live stack above.
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
pub mod fallback;
pub mod transcript_log;

// Re-export key types.
//
// Wake types are the supported transitional surface (CLI `weft voice wake`).
// All other re-exports are legacy scaffold — prefer clawft-channels /
// clawft-voice-* for new work (WEFT-671).

pub use channel::{VoiceChannel, VoiceStatus};
pub use config::{VoiceAudioConfig, VoiceCaptureSpec, VoicePipelineConfig, VoicePlaybackSpec};
pub use echo::{EchoCanceller, EchoCancellerConfig};
pub use events::VoiceWsEvent;
pub use models::ModelDownloadManager;
pub use noise::{NoiseSuppressor, NoiseSuppressorConfig};
pub use quality::{AudioMetrics, analyze_frame};
pub use talk_mode::TalkModeController;

#[cfg(feature = "voice-wake")]
pub use wake::{WakeWordConfig, WakeWordDetector, WakeWordEvent};
#[cfg(feature = "voice-wake")]
pub use wake_daemon::WakeDaemon;
