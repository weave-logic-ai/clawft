//! Full Talk-Mode assembly — the live end-to-end path (ADR-061 §7).
//!
//! This wires every CONCRETE binding into the generic controller:
//!   AEC barge-in (clawft-voice-aec) · substrate STT (6.2) · substrate dual TTS
//!   (6.4) · semantic endpoint (6.5) · speaker registry (6.6) · spoken-answer
//!   policy (6.3) · LocalProvider/Hermes (VoiceLlm) · kernel ECC observer.
//!
//! It is `#[ignore]`d: it needs the live local stack — Hermes at
//! `http://127.0.0.1:8090/v1` (ADR-060), substrate whisper + TTS daemons, and a
//! 16 kHz mono WAV at `$WEFT_TALK_WAV` to drive as captured audio. No audio is
//! faked; without those services the test does not run. Speaker ID is left off
//! here (ECAPA weights are out-of-band); the real deployment plugs a
//! `SpeakerEmbedder` in the same slot.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::VoiceAnswerPolicy;
use clawft_channels::voice::speaker::SpeakerRegistry;
use clawft_channels::voice::stt::{SttModel, SubstrateStt};
use clawft_channels::voice::talkmode::{
    ConversationEvent, ConversationObserver, TalkModeConfig, TalkModeController,
};
use clawft_channels::voice::tts::{DualLayerTts, SubstrateTts, TtsChunk, TtsSink, TtsTier};
use clawft_channels::voice::turn::{HeuristicEndpoint, SemanticEndpointer};
use clawft_channels::voice::types::VoiceError;
use clawft_channels::voice::wav::wav_to_pcm_s16le;

use clawft_kernel::CausalGraph;
use clawft_voice_talk::{AecAudioControl, EccConversationObserver, LocalProviderVoiceLlm};

/// Discarding sink — a live deployment swaps in a cpal streaming sink; for the
/// harness we only verify the pipeline produces chunks, not speaker output.
struct DiscardSink;
#[async_trait]
impl TtsSink for DiscardSink {
    async fn play_chunk(&self, _chunk: &TtsChunk) -> Result<(), VoiceError> {
        Ok(())
    }
    async fn flush(&self) {}
}

#[tokio::test]
#[ignore = "requires live Hermes (8090) + substrate STT/TTS daemons + $WEFT_TALK_WAV"]
async fn live_full_talk_mode_pipeline() {
    let wav_path = std::env::var("WEFT_TALK_WAV").expect("set WEFT_TALK_WAV to a 16k mono WAV");
    let stt_url = std::env::var("WEFT_WHISPER_URL")
        .unwrap_or_else(|_| "http://localhost:8112/inference".into());
    let tts_url =
        std::env::var("WEFT_TTS_URL").unwrap_or_else(|_| "http://localhost:8113/synthesize".into());

    // ── Concrete bindings ────────────────────────────────────────────────
    let stt = Arc::new(SubstrateStt::new(stt_url, SttModel::ParakeetEnglish, "en", 30).unwrap());
    let fast = Arc::new(SubstrateTts::new(tts_url.clone(), TtsTier::Fast, true, 30).unwrap());
    let slow = Arc::new(SubstrateTts::new(tts_url, TtsTier::Slow, false, 30).unwrap());
    let tts = DualLayerTts::new(fast, slow).unwrap();

    let llm = Arc::new(LocalProviderVoiceLlm::hermes());
    let audio = Arc::new(AecAudioControl::from_default());

    let graph = Arc::new(CausalGraph::new());
    let observer = Arc::new(EccConversationObserver::new(graph.clone()));
    let observer_dyn: Arc<dyn ConversationObserver> = observer.clone();

    let mut ctrl = TalkModeController::new(
        SemanticEndpointer::new(HeuristicEndpoint, 16_000, 250, 1_500, 0.5),
        stt,
        None, // speaker ID: plug a SpeakerEmbedder here when ECAPA weights exist
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        llm,
        tts,
        Arc::new(DiscardSink),
        audio,
        observer_dyn,
        TalkModeConfig {
            base_system: "You are a terse voice assistant.".into(),
            ..Default::default()
        },
    );

    // Drive the WAV as captured audio (100 ms frames) + trailing silence.
    let (pcm, sr) = wav_to_pcm_s16le(&std::fs::read(&wav_path).unwrap()).unwrap();
    assert_eq!(sr, 16_000, "WEFT_TALK_WAV must be 16 kHz mono");
    let (tx, rx) = mpsc::channel::<Vec<i16>>(256);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    for frame in pcm.chunks(1_600) {
        tx.send(frame.to_vec()).await.unwrap();
    }
    for _ in 0..20 {
        tx.send(vec![0i16; 1_600]).await.unwrap();
    }

    // Wait for the grounded answer to commit (real STT + Hermes + TTS).
    for _ in 0..400 {
        if observer.graph().node_count() > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;

    // The conversation produced real ECC nodes (user turn + replies).
    assert!(
        observer.graph().node_count() >= 2,
        "expected committed user turn + reply nodes on the CausalGraph"
    );
    let _ = ConversationEvent::Interrupted; // (barge-in path covered in unit tests)
}

/// The ALL-NATIVE live path (ADR-062 Phase 6.2): the real `weft talk` assembly
/// — parakeet STT, smart-turn endpoint, ECAPA speaker, Kokoro+Orpheus TTS, the
/// P2 Talk-Mode loop as orchestrator, and Hermes as the brain — driven by a WAV
/// instead of a mic. No audio is faked; needs the staged ONNX models
/// (`.weftos/models/…`, `--features onnx`) + live Hermes at `:8090`. The
/// playback sink is discarded (the agent runtime has no speaker); a mic+speaker
/// deployment uses `clawft_voice_talk::live` (the `live-audio` feature).
///
/// Run it (for the human/operator):
/// ```text
/// WEFT_TALK_WAV=/path/to/16k_mono.wav \
///   cargo test -p clawft-voice-talk --features onnx --test assembly \
///   live_native_talk_session -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "requires staged ONNX models (--features onnx) + live Hermes (8090) + $WEFT_TALK_WAV"]
async fn live_native_talk_session() {
    use clawft_channels::voice::talkmode::NoopAudioControl;
    use clawft_voice_talk::{TalkConfig, TalkSession, native_components};

    let wav_path = std::env::var("WEFT_TALK_WAV").expect("set WEFT_TALK_WAV to a 16k mono WAV");
    let config = TalkConfig {
        conv_id: "live-native".into(),
        base_system: "You are a terse voice assistant.".into(),
        ..TalkConfig::default()
    };
    // The all-native component stack (discarding sink: no speaker in-runtime).
    let components = native_components(&config, Arc::new(DiscardSink), Arc::new(NoopAudioControl))
        .expect("native components assemble");
    let session = TalkSession::assemble(config, components);
    let forest = session.forest().clone();

    let (pcm, sr) = wav_to_pcm_s16le(&std::fs::read(&wav_path).unwrap()).unwrap();
    assert_eq!(sr, 16_000, "WEFT_TALK_WAV must be 16 kHz mono");

    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let (tx, rx) = mpsc::channel::<Vec<i16>>(256);
    // `run` drives the P2 loop (orchestrator) + controller (render shell) live.
    let handle = tokio::spawn(async move { session.run(rx, run_cancel).await });

    for frame in pcm.chunks(1_600) {
        tx.send(frame.to_vec()).await.unwrap();
    }
    for _ in 0..20 {
        tx.send(vec![0i16; 1_600]).await.unwrap();
    }

    // Wait for the loom to fill AND for the loop tick to commit the user
    // turn: `LoopObserver` writes the turn as Frontier and the
    // Frontier→Committed promotion happens on the loop's *next* tick, so
    // waiting on node_count alone races the commit.
    for _ in 0..400 {
        if forest.graph().node_count() >= 2
            && forest.view().state(1)
                == Some(clawft_kernel::context_graft::NodeState::Committed)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;

    assert!(
        forest.graph().node_count() >= 2,
        "native spoken turn produced user-turn + reply nodes on the forest"
    );
    assert_eq!(
        forest.view().state(1),
        Some(clawft_kernel::context_graft::NodeState::Committed),
        "the P2 loop committed the user turn"
    );
}

/// Wave 1 §W1.5 exit — the **complete `VoiceAnalysis` decomposition off real
/// audio**. Drives a WAV (drive an *acted* / high-arousal clip to watch arousal
/// move) through the all-native in-process session and captures the record the
/// controller assembles at the observer boundary
/// (`ConversationEvent::UserTurn { voice_analysis }`) — the exact record
/// `weft voice talk` serializes onto `agent.turn.record`. Asserts every section
/// is present and honest: native STT per-token data, a real SNR, a voiced pitch
/// track, an in-range arousal off the DSP floor, `tier:"voice"`.
///
/// This is the audio→record half of the exit path; the record→`index_turn`→
/// `conversation.graph` half is pinned device-free by
/// `clawft-service-agent`'s `voice_analysis_wire.rs` (store + emotion flip) and
/// `clawft-weave`'s `conversation_graph_serves_voice_analysis_verbatim` (served
/// verbatim), bound to this half by the frozen §W1.2 schema. The acoustic
/// arousal *response* (acted > calm) is proven device-free in
/// `voice_analysis_dsp.rs`.
///
/// Needs only the staged ONNX models (`--features onnx`) — the record is
/// emitted *before* the LLM reply, so no live Hermes is required. Run it:
/// ```text
/// WEFT_TALK_WAV=/path/to/16k_mono_acted.wav \
///   cargo test -p clawft-voice-talk --features onnx --test assembly \
///   live_native_voice_analysis_decomposition -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "requires staged ONNX models (--features onnx) + $WEFT_TALK_WAV (16k mono; drive an acted clip)"]
async fn live_native_voice_analysis_decomposition() {
    use std::sync::Mutex;

    use clawft_channels::voice::talkmode::NoopAudioControl;
    use clawft_channels::voice::{EmotionSource, SttPath, VoiceAnalysis};
    use clawft_voice_talk::{TalkConfig, TalkSession, native_components};

    /// Captures the `VoiceAnalysis` the controller emits on the user turn.
    #[derive(Default)]
    struct Capture {
        va: Mutex<Option<VoiceAnalysis>>,
    }
    impl ConversationObserver for Capture {
        fn observe(&self, event: ConversationEvent) {
            if let ConversationEvent::UserTurn {
                voice_analysis: Some(va),
                ..
            } = event
            {
                *self.va.lock().unwrap() = Some(*va);
            }
        }
    }

    let wav_path = std::env::var("WEFT_TALK_WAV").expect("set WEFT_TALK_WAV to a 16k mono WAV");
    let config = TalkConfig {
        conv_id: "voice-analysis-exit".into(),
        base_system: "You are a terse voice assistant.".into(),
        ..TalkConfig::default()
    };
    let capture = Arc::new(Capture::default());
    let capture_obs: Arc<dyn ConversationObserver> = capture.clone();
    let components = native_components(&config, Arc::new(DiscardSink), Arc::new(NoopAudioControl))
        .expect("native components assemble");
    let session = TalkSession::assemble_observed(config, components, Some(capture_obs));
    let forest = session.forest().clone();

    let (pcm, sr) = wav_to_pcm_s16le(&std::fs::read(&wav_path).unwrap()).unwrap();
    assert_eq!(sr, 16_000, "WEFT_TALK_WAV must be 16 kHz mono");

    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let (tx, rx) = mpsc::channel::<Vec<i16>>(256);
    let handle = tokio::spawn(async move { session.run(rx, run_cancel).await });

    for frame in pcm.chunks(1_600) {
        tx.send(frame.to_vec()).await.unwrap();
    }
    for _ in 0..20 {
        tx.send(vec![0i16; 1_600]).await.unwrap();
    }

    // Wait for the finalized user turn to produce its record (emitted before
    // the reply, so this does not wait on the LLM).
    for _ in 0..400 {
        if capture.va.lock().unwrap().is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;

    let va = capture
        .va
        .lock()
        .unwrap()
        .clone()
        .expect("the finalized user turn produced a VoiceAnalysis record");

    // §W1.2 completeness + honesty, off real audio.
    assert_eq!(va.v, 1, "record versioned");
    assert_eq!(va.tier, "voice", "tier discriminator");
    // STT decode enrichment (§W1.3): native path with real per-token data.
    assert_eq!(va.stt.path, SttPath::Native, "native TDT path");
    assert!(!va.stt.model.is_empty(), "STT model wire name present");
    assert!(!va.stt.tokens.is_empty(), "native decode surfaced per-token timing/conf");
    assert!(
        va.stt.token_conf_mean > 0.0 && va.stt.token_conf_mean <= 1.0,
        "per-token softmax confidence in (0,1]: {}",
        va.stt.token_conf_mean
    );
    let first = &va.stt.tokens[0];
    assert!(first.dur_ms > 0, "token carries a duration");
    // Capture health → SNR from RMS − floor.
    assert!(va.audio.duration_ms > 0, "utterance duration measured");
    assert!(va.audio.snr_db.is_finite(), "SNR = RMS mean − noise floor");
    // Prosody: a real voiced pitch track (needs voiced speech in the clip).
    assert!(va.prosody.f0_mean_hz > 0.0, "voiced pitch estimated: {}", va.prosody.f0_mean_hz);
    // Emotion axis off the DSP floor (no SER staged), in range, honest source.
    assert!(
        (0.0..=1.0).contains(&va.emotion.arousal),
        "arousal in [0,1]: {}",
        va.emotion.arousal
    );
    assert_eq!(va.emotion.source, EmotionSource::ProsodyDsp, "DSP floor with no SER staged");
    assert_eq!(va.emotion.dominance, 0.0, "dominance 0.0 without a SER model");

    // The turn also committed on the in-process forest.
    assert!(
        forest.graph().node_count() >= 1,
        "the finalized user turn committed on the forest"
    );
}
