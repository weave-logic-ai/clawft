//! `weft talk` assembly (ADR-062 Phase 6.1) — construct the full ECC graph-walk
//! voice conversation with concrete native bindings and run it.
//!
//! [`TalkSession`] is the capstone wiring: it stands up the [`TalkForest`]
//! (substrate + P2 [`TalkModeLoop`] orchestrator), the concrete native
//! components (parakeet STT, smart-turn endpoint, ECAPA speaker, Kokoro+Orpheus
//! [`DualLayerTts`], Hermes [`LocalProviderVoiceLlm`], the AEC barge-in flush),
//! and the 6.7 [`TalkModeController`] **as the render shell** whose
//! [`LoopObserver`] feeds the loop. [`run`](TalkSession::run) spawns the loop
//! (`run_talk_loop`) and the controller concurrently: one orchestrator, one
//! graph.
//!
//! Construction never requires weights or a live endpoint — every native
//! component auto-discovers its model/endpoint and surfaces a `VoiceError`
//! *at use time* (per the component crates' graceful-degradation contract), so
//! `weft talk` builds the whole graph cleanly even when a model or Hermes is
//! absent. The fully-live audio path (cpal mic/speaker) is behind the
//! `live-audio` feature; the deterministic assembly path takes an injected
//! frame channel and mock-able components, so it is exercised without devices.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::policy::VoiceAnswerPolicy;
use clawft_channels::voice::speaker::{SpeakerEmbedder, SpeakerRegistry};
use clawft_channels::voice::stt::SttBackend;
use clawft_channels::voice::talkmode::{
    AudioControl, ConversationEvent, ConversationObserver, TalkModeConfig as ControllerConfig,
    TalkModeController,
};
use clawft_channels::voice::tts::{DualLayerTts, TtsSink};
use clawft_channels::voice::turn::{EndpointModel, SemanticEndpointer};

use crate::forest::{LoopObserver, TalkForest};

/// Configuration for a [`TalkSession`] — conversation identity, the spoken
/// persona, and the turn-taking/audio knobs. Backend *selection* (which STT /
/// TTS / LLM concrete) is done by which builder you call; this carries the
/// scalar parameters those builders share.
#[derive(Debug, Clone)]
pub struct TalkConfig {
    /// Stable conversation id (forest scope + cross-ref namespace).
    pub conv_id: String,
    /// Bookkeeping session-view vector width (placeholder vectors; see
    /// [`TalkForest`]).
    pub dims: usize,
    /// Capture sample rate (Hz).
    pub sample_rate: u32,
    /// VAD energy threshold (dBFS).
    pub vad_threshold_dbfs: f32,
    /// Consecutive voiced frames during playback that count as a barge-in.
    pub barge_in_frames: u32,
    /// Pause (ms) that triggers a semantic end-of-turn check.
    pub short_silence_ms: u32,
    /// Hard silence ceiling (ms) that finalizes a turn regardless of the model.
    pub max_silence_ms: u32,
    /// Endpoint completion-probability threshold to finalize a turn.
    pub endpoint_threshold: f32,
    /// Speaker-embedding cosine threshold for identify-or-enroll.
    pub speaker_threshold: f32,
    /// Default name for a freshly enrolled speaker.
    pub default_speaker_name: String,
    /// Base agent persona (the spoken-answer policy is appended).
    pub base_system: String,
    /// Persistent speaker-registry path (enrollments + spoken self-naming
    /// survive sessions). `None` keeps identities in-memory only.
    pub speaker_store: Option<std::path::PathBuf>,
    /// Listen-only mode (Wave 1 §W1.4): record + classify + store every turn
    /// but skip the brain (no ack / LLM / audio out). Drives `weft voice
    /// listen`. OFF by default.
    pub listen_only: bool,
    /// Native dual-layer **fast** TTS preference (WEFT-613 / david profile).
    /// Default [`FastEnginePreference::Auto`]: Chatterbox clone when
    /// runtime-ready, else Kokoro. Overridden by env `WEFTOS_FAST_TTS` only when
    /// left at Auto and the env is set (see [`native_components`]).
    pub fast_engine: crate::FastEnginePreference,
}

impl Default for TalkConfig {
    fn default() -> Self {
        Self {
            conv_id: "talk".into(),
            dims: 8,
            sample_rate: 16_000,
            vad_threshold_dbfs: -45.0,
            barge_in_frames: 3,
            // Turn-taking knobs match the tuned voicelab david profile
            // (base_silence=0.5 / max_silence=2.5 / turn_threshold=0.5).
            short_silence_ms: 500,
            max_silence_ms: 2_500,
            endpoint_threshold: 0.5,
            // voicelab-tuned cosine floor (same-speaker ~0.6-0.8,
            // different ~0.1-0.2).
            speaker_threshold: 0.45,
            default_speaker_name: "unknown speaker".into(),
            base_system: String::new(),
            speaker_store: std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".weftos/speakers.json")),
            listen_only: false,
            // david-intent: prefer clone path when inference ready (0.8 → Kokoro).
            fast_engine: crate::FastEnginePreference::Auto,
        }
    }
}

/// The concrete component bindings a [`TalkSession`] is assembled from.
///
/// Generic over the [`EndpointModel`] `M` so the native smart-turn model and a
/// mock both fit (the deterministic assembly test injects a mock). The other
/// seams are trait objects (already dynamic in the controller).
pub struct TalkComponents<M: EndpointModel> {
    /// Semantic end-of-turn detector (smart-turn ONNX, or heuristic/mock).
    pub endpointer: SemanticEndpointer<M>,
    /// Speech-to-text backend (parakeet ONNX, or a mock).
    pub stt: Arc<dyn SttBackend>,
    /// Optional speaker embedder (ECAPA ONNX) for attribution.
    pub embedder: Option<Arc<dyn SpeakerEmbedder>>,
    /// The forest-grounded brain (LocalProvider → Hermes, or a mock).
    pub llm: Arc<dyn clawft_channels::voice::policy::VoiceLlm>,
    /// Dual-layer TTS (fast ack + slow expressive answer).
    pub tts: DualLayerTts,
    /// Playback sink (cpal AEC sink, or a recording mock).
    pub sink: Arc<dyn TtsSink>,
    /// Barge-in audio control (AEC render-reference flush, or no-op).
    pub audio: Arc<dyn AudioControl>,
}

/// The assembled `weft talk` graph: the forest + the controller shell driving
/// the loop through a [`LoopObserver`].
pub struct TalkSession<M: EndpointModel> {
    forest: Arc<TalkForest>,
    controller: TalkModeController<M>,
}

impl<M: EndpointModel + 'static> TalkSession<M> {
    /// Assemble a session from explicit components (the seam the deterministic
    /// test and the native builder share).
    pub fn assemble(config: TalkConfig, components: TalkComponents<M>) -> Self {
        Self::assemble_observed(config, components, None)
    }

    /// [`Self::assemble`] with an optional extra [`ConversationObserver`]
    /// fanned out alongside the loop's own [`LoopObserver`] — the seam for
    /// mirroring committed turns to an external recorder (e.g. the daemon's
    /// `agent.turn.record` RPC so voice exchanges anchor to the witness
    /// chain). The extra observer sees every event the loop observer sees;
    /// it must be non-blocking (fan-out is synchronous on the turn path).
    pub fn assemble_observed(
        config: TalkConfig,
        components: TalkComponents<M>,
        extra_observer: Option<Arc<dyn ConversationObserver>>,
    ) -> Self {
        let forest = Arc::new(TalkForest::new(config.conv_id.clone(), config.dims));
        let loop_observer: Arc<dyn ConversationObserver> =
            Arc::new(LoopObserver::new(forest.clone()));
        let observer: Arc<dyn ConversationObserver> = match extra_observer {
            Some(extra) => Arc::new(FanoutObserver {
                primary: loop_observer,
                extra,
            }),
            None => loop_observer,
        };
        // Load persisted speaker identities when a store is configured;
        // fresh registry otherwise (or on first run / unreadable file).
        let registry = config
            .speaker_store
            .as_ref()
            .filter(|p| p.exists())
            .and_then(|p| SpeakerRegistry::load(p).ok())
            .unwrap_or_else(|| SpeakerRegistry::new(config.speaker_threshold));
        // WEFT-644: prefer Silero neural VAD when silero_vad.onnx is staged;
        // else SpectralVoiceness. Injected via the Voiceness seam — energy
        // floor / watchdog paths are untouched.
        let controller = TalkModeController::new(
            components.endpointer,
            components.stt,
            components.embedder,
            registry,
            VoiceAnswerPolicy::default(),
            components.llm,
            components.tts,
            components.sink,
            components.audio,
            observer,
            ControllerConfig {
                sample_rate: config.sample_rate,
                vad_threshold_dbfs: config.vad_threshold_dbfs,
                barge_in_frames: config.barge_in_frames,
                default_speaker_name: config.default_speaker_name,
                base_system: config.base_system,
                speaker_store: config.speaker_store,
                listen_only: config.listen_only,
                ..ControllerConfig::default()
            },
        )
        .with_voiceness(clawft_voice_onnx::preferred_voiceness());
        Self { forest, controller }
    }

    /// The forest (graph / loop) for inspection, persistence, or tests.
    pub fn forest(&self) -> &Arc<TalkForest> {
        &self.forest
    }

    /// Shared capture-path metrics — the live path passes these to the
    /// `CaptureProcessor` so its dropped-frame / high-water counts surface in
    /// the `VoiceAnalysis` record (§round-5 instrumentation).
    pub fn capture_metrics(&self) -> Arc<clawft_channels::voice::CaptureMetrics> {
        self.controller.capture_metrics()
    }

    /// Run the conversation: drive the P2 loop (orchestrator) on its
    /// self-calibrating tick **and** the controller (render shell) on the frame
    /// stream, concurrently, until `cancel` fires or `frames` closes.
    ///
    /// `frames` are 16 kHz mono `i16` echo-cancelled mic frames. In the live
    /// path they come from `clawft_voice_aec::run_capture`; the deterministic
    /// test feeds a scripted channel.
    pub async fn run(mut self, frames: mpsc::Receiver<Vec<i16>>, cancel: CancellationToken) {
        let talk_loop = self.forest.talk_loop().clone();
        let loop_cancel = cancel.clone();
        let loop_handle = tokio::spawn(async move {
            clawft_kernel::talk_loop::run_talk_loop(talk_loop, loop_cancel).await
        });
        self.controller.run(frames, cancel).await;
        loop_handle.abort();
        let _ = loop_handle.await;
    }
}

/// Fans each [`ConversationEvent`] out to the loop's own observer first
/// (turn-taking is load-bearing) and then to an extra recorder. Both see
/// every event; the extra observer must not block the turn path.
struct FanoutObserver {
    primary: Arc<dyn ConversationObserver>,
    extra: Arc<dyn ConversationObserver>,
}

impl ConversationObserver for FanoutObserver {
    fn observe(&self, event: ConversationEvent) {
        self.primary.observe(event.clone());
        self.extra.observe(event);
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
