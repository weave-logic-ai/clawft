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
//! - `voice-xai`          — ADR-074 xAI Realtime WebSocket client
//!   ([`xai_realtime`]): connect + `session.update` hello + V1 tool bridge
//!   ([`xai_tool_bridge`] → WindowIntent spawn/focus/summarize; WEFT-690)
//!   + V2 hybrid/metrics/health ([`xai_health`]; WEFT-691).

pub mod analysis;
pub mod capture;
pub mod channel;
pub mod cue;
pub mod edge_reflex;
pub mod paralinguistics;
pub mod personality;
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
pub mod voiceness;
pub mod wav;
// ADR-074 / WEFT-689 + WEFT-690 + WEFT-691: xAI Realtime + tools + health/fallback.
#[cfg(feature = "voice-xai")]
pub mod xai_health;
#[cfg(feature = "voice-xai")]
pub mod xai_realtime;
#[cfg(feature = "voice-xai")]
pub mod xai_tool_bridge;

pub use analysis::{
    AudioAnalysis, Confidence, EmotionAnalysis, EmotionSource, EndpointAnalysis,
    ParalinguisticClass, ParalinguisticsAnalysis, ProsodyAnalysis, SpeakerAction, SpeakerAnalysis,
    SttAnalysis, SttPath, TokenAnalysis, VoiceAnalysis, VOICE_ANALYSIS_VERSION, VOICE_TIER,
};
// SC-2 / WEFT-223: re-export secure PCM helpers used by talk-mode preroll.
pub use clawft_types::config::AudioRetention;
pub use clawft_types::{SecureAudioBuffer, SecureAudioRing, zeroize_samples, zeroize_vec};

pub use capture::{CaptureMetrics, CaptureProcessor, ImpulseSink, VoiceImpulse};
pub use edge_reflex::{EdgeCommand, EdgeReflex, ReflexState};
pub use paralinguistics::{classify_paralinguistics, ParalinguisticInput};
pub use prosody::{
    analyze_prosody, capture_health, emotion_from_prosody, emotion_from_prosody_baseline,
    estimate_f0_track, BaselineDeviation, CaptureHealth, F0Track, ProsodyInput, SessionBaseline,
};
pub use ser::{refine_emotion, DspSer, SerModel, SerPrediction};
pub use channel::{
    AudioSegment, AudioSource, PlaybackSink, VoiceChannelAdapter, VoiceChannelAdapterFactory,
};
pub use policy::{ReasoningHint, VoiceAnswerPolicy, VoiceLlm, VoiceTurnRequest, estimate_tokens};
pub use speaker::{SpeakerEmbedder, SpeakerId, SpeakerMatch, SpeakerNode, SpeakerRegistry, cosine};
pub use stt::{SttBackend, SttModel, SubstrateStt, TranscriptResult, TranscriptToken, Utterance};
pub use cue::{CueKind, cue_chunk};
pub use talkmode::{
    AudioControl, ConversationEvent, ConversationObserver, NoopAudioControl, NoopObserver,
    TalkModeConfig, TalkModeController, contextual_ack,
};
pub use personality::PersonalityTtsDispatch;
pub use tts::{
    DualLayerTts, SubstrateTts, TtsChunk, TtsEngine, TtsSink, TtsTier, TtsVoiceParams, scrub_tags,
    split_sentences,
};
pub use turn::{EndpointModel, HeuristicEndpoint, SemanticEndpointer, TurnDecision};
pub use types::{VoiceAdapterConfig, VoiceError};
pub use vad::{EnergyVad, VadEvent};
// WEFT-230: adaptive silence-timeout types (config + estimator).
pub use clawft_types::config::{AdaptiveSilenceConfig, AdaptiveSilenceTimeout};
pub use voiceness::{SpectralVoiceness, Voiceness};

#[cfg(feature = "voice-xai")]
pub use xai_health::{
    graceful_local_fallback_after_failure, health_probe_endpoint_label, probe_xai_voice_health,
    select_voice_transport, XaiHealthProbeMode,
};
#[cfg(feature = "voice-xai")]
pub use xai_realtime::{
    build_realtime_request, connect_hello, connect_hello_or_fallback, default_conductor_instructions,
    default_endpoint, default_model, load_api_key_from_env, session_update_payload,
    session_update_payload_with_tools, session_update_with_weftos_tools, XaiRealtimeConnectParams,
    XaiRealtimeError, XaiRealtimeHelloResult, XaiSessionStart, DEFAULT_CONNECT_TIMEOUT,
    DEFAULT_HELLO_EVENT_TIMEOUT,
};
#[cfg(feature = "voice-xai")]
pub use xai_tool_bridge::{
    aec_in_front_of_mic_required, aec_pipeline_ready, default_weftos_tools,
    function_call_output_event, handle_function_call_event, map_function_call_to_intent,
    map_to_window_intent, parse_function_call_event, response_create_event, summarize_spoken_safe,
    weftos_realtime_tools, DemoIntentHandler, FunctionCallEvent, IntentHandler, ToolBridgeError,
    ToolCallResult, ToolCatalogOptions, WindowIntent, AEC_IN_FRONT_OF_MIC_NOTE,
    DEFAULT_SUMMARIZE_MAX_CHARS, MAX_SPAWN_COUNT, TOOL_FOCUS_PANE, TOOL_SHELL_INTENT,
    TOOL_SPAWN_AGENT, TOOL_SUMMARIZE,
};
