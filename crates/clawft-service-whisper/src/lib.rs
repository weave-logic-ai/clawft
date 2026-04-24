//! WeftOS whisper transcription service — HTTP client pipeline (client layer).
//!
//! This commit adds the HTTP client that POSTs WAV-wrapped PCM to
//! whisper.cpp's `/inference` endpoint. Still missing: the substrate
//! subscribe/publish wiring that ties the client into WeftOS pub/sub —
//! lands in the next commit on this branch.
//!
//! See `.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md` for the full
//! shape + the FFI-vs-HTTP rationale (§1).

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod client;
pub mod wav;
pub mod windower;

pub use client::{InferenceResponse, TranscribeError, WhisperClient, WhisperConfig};
pub use windower::{PcmChunk, PcmWindow, Windower};

/// Substrate path the service subscribes to for inbound PCM chunks.
pub const SUBSTRATE_PCM_INPUT_PATH: &str = "substrate/sensor/mic/pcm_chunk";

/// Substrate path the service publishes transcripts to.
pub const SUBSTRATE_TRANSCRIPT_OUTPUT_PATH: &str = "substrate/derived/transcript/mic";

/// Environment variable overriding the whisper service URL.
pub const WHISPER_SERVICE_URL_ENV: &str = "WHISPER_SERVICE_URL";

/// Default whisper service URL when the env var is unset.
pub const DEFAULT_WHISPER_SERVICE_URL: &str = "http://127.0.0.1:8080";
