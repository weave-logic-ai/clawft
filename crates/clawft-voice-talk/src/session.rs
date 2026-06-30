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
    AudioControl, ConversationObserver, TalkModeConfig as ControllerConfig, TalkModeController,
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
}

impl Default for TalkConfig {
    fn default() -> Self {
        Self {
            conv_id: "talk".into(),
            dims: 8,
            sample_rate: 16_000,
            vad_threshold_dbfs: -45.0,
            barge_in_frames: 3,
            short_silence_ms: 250,
            max_silence_ms: 2_000,
            endpoint_threshold: 0.5,
            speaker_threshold: 0.6,
            default_speaker_name: "unknown speaker".into(),
            base_system: String::new(),
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
        let forest = Arc::new(TalkForest::new(config.conv_id.clone(), config.dims));
        let observer: Arc<dyn ConversationObserver> = Arc::new(LoopObserver::new(forest.clone()));
        let registry = SpeakerRegistry::new(config.speaker_threshold);
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
            },
        );
        Self { forest, controller }
    }

    /// The forest (graph / loop) for inspection, persistence, or tests.
    pub fn forest(&self) -> &Arc<TalkForest> {
        &self.forest
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
