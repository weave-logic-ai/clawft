//! WeftOS whisper transcription service — HTTP client pipeline (scaffold).
//!
//! This commit lays the crate foundation: WAV-wrapping + PCM-chunk
//! windowing. The HTTP client and the substrate-connected service are
//! added in follow-up commits on the same `phase2-whisper-spike`
//! branch. See `.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md` for
//! the full shape (rolls in once the pipeline lands end-to-end).

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod wav;
pub mod windower;

pub use windower::{PcmChunk, PcmWindow, Windower};

/// Substrate path the service subscribes to for inbound PCM chunks.
pub const SUBSTRATE_PCM_INPUT_PATH: &str = "substrate/sensor/mic/pcm_chunk";

/// Substrate path the service publishes transcripts to.
pub const SUBSTRATE_TRANSCRIPT_OUTPUT_PATH: &str = "substrate/derived/transcript/mic";

/// Environment variable overriding the whisper service URL.
pub const WHISPER_SERVICE_URL_ENV: &str = "WHISPER_SERVICE_URL";

/// Default whisper service URL when the env var is unset.
pub const DEFAULT_WHISPER_SERVICE_URL: &str = "http://127.0.0.1:8080";
