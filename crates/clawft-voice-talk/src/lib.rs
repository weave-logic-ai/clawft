//! Talk-Mode bridge (ADR-061 §7) — concrete bindings for the conversational
//! voice loop.
//!
//! `clawft-channels::voice::talkmode` is deliberately generic: the controller
//! depends only on traits. This crate provides the concrete, dependency-heavy
//! implementations the daemon wires in, keeping clawft-channels free of
//! clawft-llm / clawft-kernel / cpal:
//!
//! - [`LocalProviderVoiceLlm`] — [`VoiceLlm`](clawft_channels::voice::policy::VoiceLlm)
//!   over `clawft_llm::LocalProvider` (the live local Hermes serving recipe,
//!   ADR-060), reasoning off for concise spoken answers (ADR-061 §3).
//! - [`AecAudioControl`] — the barge-in
//!   [`AudioControl`](clawft_channels::voice::talkmode::AudioControl) over
//!   `clawft_voice_aec::AecProcessor` (flush the render reference on interrupt).
//! - [`EccConversationObserver`] — the
//!   [`ConversationObserver`](clawft_channels::voice::talkmode::ConversationObserver)
//!   that writes the speculative→committed handoff onto the kernel ECC
//!   `CausalGraph` (NodeState lifecycle in node metadata; no parallel
//!   mechanism).
//! - [`native_dual_layer`] — binds the native TTS node-renderers (Kokoro fast +
//!   Orpheus/SNAC slow, from `clawft-voice-tts`) into the
//!   [`DualLayerTts`](clawft_channels::voice::tts::DualLayerTts) that
//!   [`NodeRenderer`] drives (ADR-062 Phase 4).
//!
//! The fully-live end-to-end path (real mic/speaker + STT/TTS/ECAPA weights +
//! the Hermes endpoint) is exercised by `#[ignore]`d tests — this environment
//! has no audio devices or model weights.

mod audio;
mod ecc;
mod forest;
#[cfg(feature = "live-audio")]
pub mod live;
mod llm;
mod native;
mod render;
mod session;
mod tts;

pub use audio::AecAudioControl;
// Re-exported so `weft voice talk` can implement an extra observer
// (turn recording via `agent.turn.record`) without a direct
// clawft-channels dependency.
pub use clawft_channels::voice::talkmode::{ConversationEvent, ConversationObserver};
pub use ecc::EccConversationObserver;
pub use forest::{KernelImpulseSink, LoopObserver, TalkForest};
pub use llm::LocalProviderVoiceLlm;
pub use native::native_components;
pub use render::{ContradictionDetector, NeverContradicts, NodeRenderer, RenderOutcome};
pub use session::{TalkComponents, TalkConfig, TalkSession};
pub use tts::{native_dual_layer, native_dual_layer_with_ollama};

// Re-export the concrete native TTS node-renderers so callers can compose a
// custom DualLayerTts (e.g. swap the fast backend) without a second dependency.
pub use clawft_voice_tts::{KokoroTts, OrpheusTts, SnacDecode, SnacOnnxDecoder};
