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
