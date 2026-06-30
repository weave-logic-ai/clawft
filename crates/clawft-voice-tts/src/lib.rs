//! Native-Rust TTS node-renderers for WeftOS — **no Python** (ADR-062 Phase 4).
//!
//! ADR-062 D3 maps a voice turn to two self-branches: a **Speculative** node
//! rendered immediately by a *fast* engine (latency cover), superseded by a
//! **Committed** node rendered by an expressive *slow* engine. This crate hosts
//! the two concrete engines behind the generic
//! [`clawft_channels::voice::tts::TtsEngine`] seam, plus the SNAC audio-codec
//! decoder the slow engine streams through. It is the TTS sibling of the STT /
//! speaker / endpoint models in `clawft-voice-onnx` and follows the same idiom:
//! a feature flag (`onnx`) gates `ort`, the token/frame math is pure Rust and
//! unit-tested, and inference degrades gracefully when weights or the Ollama
//! runtime are absent so default builds and unit tests need no model files.
//!
//! # Engines
//!
//! - [`OrpheusTts`] — **slow** layer (`TtsTier::Slow`). Drives Orpheus token
//!   generation over the **Ollama** HTTP streaming API
//!   (`POST /api/generate`, `raw`+`stream`), parses the `<custom_token_N>`
//!   stream, and decodes it to gap-free PCM through a [`SnacDecode`]. Performs
//!   paralinguistic tags (`<laugh>`) literally — text is never scrubbed.
//! - [`KokoroTts`] — **fast** layer (`TtsTier::Fast`). A native ONNX TTS for the
//!   Speculative ack (<1 s TTFA). Chatterbox stays a future backend behind the
//!   same [`TtsEngine`] when an ONNX export exists.
//!
//! # SNAC
//!
//! [`SnacDecode`] is the seam the slow engine renders through; [`SnacOnnxDecoder`]
//! is the concrete SNAC-24kHz `ort` decoder. The Orpheus→SNAC token math
//! ([`parse_custom_token`], [`SnacDecode::decode_frames`]'s frame regroup) is
//! pure Rust and unit-tested without any model.
//!
//! # Feature flags
//!
//! - `onnx` — pull in `ort` for real SNAC + Kokoro inference. Without it the
//!   token/frame front-end still compiles and is unit-tested; the decoders
//!   return a clear [`clawft_channels::voice::types::VoiceError`].

pub mod kokoro;
pub mod orpheus;
pub mod snac;

pub use kokoro::KokoroTts;
pub use orpheus::{OrpheusTts, parse_custom_token};
pub use snac::{SnacDecode, SnacOnnxDecoder};
