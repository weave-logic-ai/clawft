//! Voice channel adapter.
//!
//! Implements [`ChannelAdapter`](clawft_plugin::traits::ChannelAdapter) for
//! a microphone-driven voice loop. Capture pulls 16 kHz mono PCM, an
//! energy-based VAD slices it into utterances on speech-end, and each
//! utterance is POSTed as a multipart WAV to a substrate STT endpoint
//! (whisper.cpp `/inference` style, configurable). The transcript is
//! delivered to the agent pipeline via `ChannelAdapterHost::deliver_inbound`,
//! and outbound TTS plays through cpal output (or the trait-injected sink
//! used in tests).
//!
//! # Why substrate (Option A) and not in-process sherpa-rs
//!
//! The canonical-path ADR for voice STT (WEFT-205) is in M5 and
//! undecided. Option A talks to the already-shipped
//! [`clawft-service-whisper`] HTTP daemon (and to a TTS daemon with the
//! same shape) so the voice channel becomes a thin client. If the M5 ADR
//! ratifies substrate-only, no rework. If it picks in-process sherpa-rs,
//! this surface remains as the substrate-fallback path. See
//! `.planning/reviews/0.7.0-release-gate/10-voice.md` open question §1.
//!
//! # Modules
//!
//! - [`types`]   — [`VoiceAdapterConfig`], [`VoiceError`].
//! - [`capture`] — [`CaptureProcessor`] (frames → VAD/endpoint →
//!   [`VoiceImpulse`]s + frame forwarding, ADR-062 Phase 5).
//! - [`channel`] — [`VoiceChannelAdapter`] + audio-source / playback-sink
//!   traits + the wiremock-backed test path.
//! - [`vad`]     — energy-RMS voice activity detector (utterance segmenter).
//! - [`wav`]     — minimal 16-kHz mono `s16le` RIFF/WAV header writer.
//!
//! # Feature flags
//!
//! - `voice`              — module + trait-injected audio path. Builds
//!   without alsa / CoreAudio / WASAPI dev headers; CI runs tests on it.
//! - `voice-real-audio`   — also pulls in `cpal` for real I/O.
//! - `real-audio-test`    — turns on cpal-touching tests (skipped on CI).

pub mod analysis;
pub mod capture;
pub mod channel;
pub mod edge_reflex;
pub mod paralinguistics;
pub mod policy;
pub mod prosody;
pub mod ser;
pub mod speaker;
pub mod stt;
pub mod talkmode;
pub mod tts;
pub mod turn;
pub mod types;
pub mod vad;
pub mod wav;

pub use analysis::{
    AudioAnalysis, Confidence, EmotionAnalysis, EmotionSource, EndpointAnalysis,
    ParalinguisticClass, ParalinguisticsAnalysis, ProsodyAnalysis, SpeakerAction, SpeakerAnalysis,
    SttAnalysis, SttPath, TokenAnalysis, VoiceAnalysis, VOICE_ANALYSIS_VERSION, VOICE_TIER,
};
pub use capture::{CaptureProcessor, ImpulseSink, VoiceImpulse};
pub use edge_reflex::{EdgeCommand, EdgeReflex, ReflexState};
pub use paralinguistics::{classify_paralinguistics, ParalinguisticInput};
pub use prosody::{
    analyze_prosody, capture_health, emotion_from_prosody, estimate_f0_track, CaptureHealth,
    F0Track, ProsodyInput,
};
pub use ser::{refine_emotion, DspSer, SerModel, SerPrediction};
pub use channel::{
    AudioSegment, AudioSource, PlaybackSink, VoiceChannelAdapter, VoiceChannelAdapterFactory,
};
pub use policy::{ReasoningHint, VoiceAnswerPolicy, VoiceLlm, VoiceTurnRequest, estimate_tokens};
pub use speaker::{SpeakerEmbedder, SpeakerId, SpeakerMatch, SpeakerNode, SpeakerRegistry, cosine};
pub use stt::{SttBackend, SttModel, SubstrateStt, TranscriptResult, TranscriptToken, Utterance};
pub use talkmode::{
    AudioControl, ConversationEvent, ConversationObserver, NoopAudioControl, NoopObserver,
    TalkModeConfig, TalkModeController, contextual_ack,
};
pub use tts::{
    DualLayerTts, SubstrateTts, TtsChunk, TtsEngine, TtsSink, TtsTier, scrub_tags, split_sentences,
};
pub use turn::{EndpointModel, HeuristicEndpoint, SemanticEndpointer, TurnDecision};
pub use types::{VoiceAdapterConfig, VoiceError};
pub use vad::{EnergyVad, VadEvent};
