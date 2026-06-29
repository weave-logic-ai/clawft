//! Native-Rust ONNX voice models for WeftOS.
//!
//! This crate hosts the cross-platform, backend-agnostic voice models that run
//! locally via [`ort`](https://docs.rs/ort) (ONNX Runtime), keeping
//! `clawft-channels` free of model/runtime weight. It is the audio sibling of
//! the text embedders in `clawft-kernel` (`embedding_qwen3` / `embedding_onnx`)
//! and follows the same idiom: a feature flag (`onnx`) gates `ort`, the front-end
//! ("input prep") is pure Rust, and inference degrades gracefully when weights
//! are absent so default builds and unit tests need no model files.
//!
//! # Models
//!
//! - [`EcapaEmbedder`] — ECAPA-TDNN speaker embedding (192-d d-vectors),
//!   satisfying [`clawft_channels::voice::speaker::SpeakerEmbedder`]. Front-end:
//!   [`Fbank`] (SpeechBrain-parity 80-dim log-mel).
//!
//! Parakeet STT and smart-turn endpointing extend this same crate (Phase 2).
//!
//! # Feature flags
//!
//! - `onnx` — pull in `ort` for real inference. Without it the front-end still
//!   compiles and is unit-tested; embedders return a clear "model unavailable"
//!   [`clawft_channels::voice::types::VoiceError`].

pub mod ecapa;
pub mod fbank;

pub use ecapa::{EMBED_DIM, EcapaEmbedder};
pub use fbank::{Fbank, N_MELS, SAMPLE_RATE};
