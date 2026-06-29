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
//!
//! The fully-live end-to-end path (real mic/speaker + STT/TTS/ECAPA weights +
//! the Hermes endpoint) is exercised by `#[ignore]`d tests — this environment
//! has no audio devices or model weights.

mod audio;
mod ecc;
mod llm;

pub use audio::AecAudioControl;
pub use ecc::EccConversationObserver;
pub use llm::LocalProviderVoiceLlm;
