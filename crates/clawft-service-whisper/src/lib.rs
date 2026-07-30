//! WeftOS whisper transcription service — HTTP client pipeline.
//!
//! # Phase 2 Track 4 spike — pipeline primitive probe
//!
//! This crate is the first WeftOS sensor-ingestion pipeline built end to
//! end. It is **a probe, not a deliverable**: its job is to answer the
//! questions in `.planning/sensors/PIPELINE-PRIMITIVE-SPIKE.md` by
//! forcing them through a real implementation and journalling the
//! answers in `.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md`.
//!
//! # Shape of the service
//!
//! ```text
//!   substrate/<node>/sensor/mic/pcm_chunk   substrate/_derived/transcript/<src>/mic
//!           (i16 PCM, b64 in JSON)                    (text + timing)
//!                   │                                      ▲
//!                   ▼                                      │
//!              ┌──────────┐   WAV+mpart    ┌──────────┐    │
//!              │ Windower │ ─────────────▶ │ HTTP POST│ ───┘
//!              │  (1–3 s) │                │ /inference│
//!              └──────────┘                └──────────┘
//!                                               │
//!                                               ▼
//!                                      whisper.cpp server
//!                                      (separate process,
//!                                       localhost:8080)
//! ```
//!
//! Path naming follows WEFT-438 (node-scoped canonical). Prefer
//! [`pcm_chunk_input_path`] over the legacy flat constant.
//!
//! Unlike the earlier FFI-linked design (which this crate deliberately
//! does NOT take — see journal §"HTTP-as-stage"), whisper runs as its
//! own HTTP service with its own lifecycle, its own model load, and its
//! own backpressure model (one in-flight request per instance, no 429).
//!
//! # Crate layout
//!
//! - [`wav`]       — minimal RIFF/WAV header writer (16 kHz mono s16le)
//! - [`windower`]  — accumulates PCM chunks into whisper-sized windows
//! - [`client`]    — [`WhisperClient`], the HTTP consumer of `/inference`
//! - [`service`]   — [`WhisperService`], the substrate-connected pipeline
//!
//! The four modules are separable: `wav` and `windower` are pure data,
//! `client` has no substrate knowledge, `service` composes them against
//! an `Arc<SubstrateService>`.

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod audit;
pub mod client;
pub mod manifest;
pub mod service;
pub mod wav;
pub mod windower;

pub use audit::{AUDIT_TARGET, TranscriptAuditEvent};
pub use client::{InferenceResponse, TranscribeError, WhisperClient, WhisperConfig};
pub use manifest::{
    MANIFEST_FILENAME, MANIFEST_SIG_FILENAME, ManifestFile, ModelIntegrityError,
    ModelIntegrityReport, ModelManifest, verify_model_dir, verify_model_dir_soft,
};
pub use service::{WhisperService, WhisperServiceConfig};
pub use windower::{PcmChunk, PcmWindow, Windower};

/// Legacy flat PCM input path (pre-node-identity).
///
/// **Deprecated (WEFT-438):** use [`pcm_chunk_input_path`] with the
/// source node id. Dual-emit may still populate this path during the
/// 0.8.x migration window; it is removed at 0.9.0
/// (`LEGACY_FLAT_REMOVAL_VERSION` in `clawft_substrate::sensor_paths`).
///
/// Payload shape (JSON):
/// ```json
/// { "pcm_b64": "...", "sample_rate": 16000, "channels": 1, "seq": 0, "chunk_ms": 500 }
/// ```
#[deprecated(
    since = "0.8.0",
    note = "use pcm_chunk_input_path(node_id) — WEFT-438 node-scoped layout"
)]
pub const SUBSTRATE_PCM_INPUT_PATH: &str = "substrate/sensor/mic/pcm_chunk";

/// Canonical node-scoped PCM input path for `source_node_id`.
///
/// Builds `substrate/<source_node_id>/sensor/mic/pcm_chunk` per WEFT-438.
/// Mirrors `clawft_substrate::sensor_paths::mic_pcm_chunk_path` without
/// taking a substrate crate dependency.
pub fn pcm_chunk_input_path(source_node_id: &str) -> String {
    format!("substrate/{source_node_id}/sensor/mic/pcm_chunk")
}

/// Mesh-canonical transcript path *prefix* for the whisper pipeline.
///
/// Per `.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md` §R3.2 the
/// fully-resolved publish path is
/// `substrate/_derived/transcript/<source-node-id>/mic` — the source
/// node id is part of the path so subscribers see one stable subtree
/// even if leader handoff swaps which kernel-class node is currently
/// running the pipeline.
///
/// Payload shape (JSON):
/// ```json
/// { "text": "...", "start_ms": 0, "end_ms": 2000, "confidence": null,
///   "lang": "en", "seq": 0 }
/// ```
pub const SUBSTRATE_TRANSCRIPT_OUTPUT_PREFIX: &str = "substrate/_derived/transcript";

/// Environment variable read by [`WhisperConfig::from_env`].
pub const WHISPER_SERVICE_URL_ENV: &str = "WHISPER_SERVICE_URL";

/// Default whisper service URL if the env var is unset.
pub const DEFAULT_WHISPER_SERVICE_URL: &str = "http://127.0.0.1:8080";
