//! Deterministic Talk-Mode orchestration test (ADR-061 §7) — proves the full
//! capture → endpoint → STT → speaker → grounded agent → ack → answer pipeline
//! AND barge-in-flush AND the speculative→committed ECC handoff, all with mock
//! STT / TTS / LLM / embedder, green without any live model or audio device.
#![cfg(feature = "voice")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::VoiceAnswerPolicy;
use clawft_channels::voice::analysis::{EmotionSource, SpeakerAction};
use clawft_channels::voice::policy::{VoiceLlm, VoiceTurnRequest};
use clawft_channels::voice::speaker::{SpeakerEmbedder, SpeakerRegistry};
use clawft_channels::voice::stt::{SttBackend, SttModel, Utterance};
use clawft_channels::voice::talkmode::{
    AudioControl, ConversationEvent, ConversationObserver, TalkModeConfig, TalkModeController,
};
use clawft_channels::voice::tts::{DualLayerTts, TtsChunk, TtsEngine, TtsSink, TtsTier};
use clawft_channels::voice::turn::{HeuristicEndpoint, SemanticEndpointer};
use clawft_channels::voice::types::VoiceError;

// ── Mocks ───────────────────────────────────────────────────────────────

struct MockStt(&'static str);
#[async_trait]
impl SttBackend for MockStt {
    async fn warm(&self) -> Result<(), VoiceError> {
        Ok(())
    }
    async fn transcribe(&self, _utt: &Utterance) -> Result<String, VoiceError> {
        Ok(self.0.to_string())
    }
    fn model(&self) -> SttModel {
        SttModel::ParakeetEnglish
    }
}

struct MockLlm(&'static str);
#[async_trait]
impl VoiceLlm for MockLlm {
    async fn complete(&self, _req: &VoiceTurnRequest) -> Result<String, VoiceError> {
        Ok(self.0.to_string())
    }
}

struct MockEmbedder;
#[async_trait]
impl SpeakerEmbedder for MockEmbedder {
    async fn embed(&self, _audio: &[i16], _sr: u32) -> Result<Vec<f32>, VoiceError> {
        Ok(vec![1.0, 0.0, 0.0])
    }
    fn dim(&self) -> usize {
        3
    }
}

/// Fast engine: emits one chunk immediately per sentence.
struct ImmediateEngine(TtsTier);
#[async_trait]
impl TtsEngine for ImmediateEngine {
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        for _ in text.split('.').filter(|s| !s.trim().is_empty()) {
            if cancel.is_cancelled() {
                break;
            }
            let _ = tx
                .send(TtsChunk {
                    samples: vec![1i16; 160],
                    sample_rate: 16_000,
                })
                .await;
        }
        Ok(())
    }
    fn tier(&self) -> TtsTier {
        self.0
    }
}

/// Slow engine: drips many chunks with sleeps so a barge-in can interject.
struct DripEngine;
#[async_trait]
impl TtsEngine for DripEngine {
    async fn synthesize_stream(
        &self,
        _text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        for _ in 0..100 {
            if cancel.is_cancelled() {
                break;
            }
            if tx
                .send(TtsChunk {
                    samples: vec![2i16; 160],
                    sample_rate: 16_000,
                })
                .await
                .is_err()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        Ok(())
    }
    fn tier(&self) -> TtsTier {
        TtsTier::Slow
    }
}

#[derive(Default)]
struct RecordingSink {
    chunks: AtomicUsize,
    flushes: AtomicUsize,
}
#[async_trait]
impl TtsSink for RecordingSink {
    async fn play_chunk(&self, _chunk: &TtsChunk) -> Result<(), VoiceError> {
        self.chunks.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn flush(&self) {
        self.flushes.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct CountingAudio(AtomicUsize);
impl AudioControl for CountingAudio {
    fn flush(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct RecordingObserver {
    events: std::sync::Mutex<Vec<ConversationEvent>>,
}
impl ConversationObserver for RecordingObserver {
    fn observe(&self, event: ConversationEvent) {
        self.events.lock().unwrap().push(event);
    }
}
impl RecordingObserver {
    fn snapshot(&self) -> Vec<ConversationEvent> {
        self.events.lock().unwrap().clone()
    }
    fn has<F: Fn(&ConversationEvent) -> bool>(&self, f: F) -> bool {
        self.events.lock().unwrap().iter().any(f)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn voiced_frame() -> Vec<i16> {
    (0..1_600)
        .map(|i| if i % 2 == 0 { 8_000 } else { -8_000 })
        .collect()
}
fn silent_frame() -> Vec<i16> {
    vec![0i16; 1_600]
}

fn endpointer() -> SemanticEndpointer<HeuristicEndpoint> {
    // short=100ms, max=300ms, threshold 0.5. With no streaming partial the
    // semantic check abstains (empty text → 0.3) so the max-silence ceiling
    // finalizes — deterministic.
    SemanticEndpointer::new(HeuristicEndpoint, 16_000, 100, 300, 0.5)
}

async fn wait_until<F: Fn() -> bool>(f: F) {
    for _ in 0..200 {
        if f() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("condition not met within timeout");
}

// ── Tests ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn full_pipeline_speculative_then_committed() {
    let observer = Arc::new(RecordingObserver::default());
    let sink = Arc::new(RecordingSink::default());
    let audio = Arc::new(CountingAudio::default());

    let tts = DualLayerTts::new(
        Arc::new(ImmediateEngine(TtsTier::Fast)),
        Arc::new(ImmediateEngine(TtsTier::Slow)),
    )
    .unwrap();

    let mut ctrl = TalkModeController::new(
        endpointer(),
        Arc::new(MockStt("tell me about Paris today")),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("Paris is the capital of France.")),
        tts,
        sink.clone(),
        audio.clone(),
        observer.clone() as Arc<dyn ConversationObserver>,
        TalkModeConfig::default(),
    );

    let (tx, rx) = mpsc::channel::<Vec<i16>>(64);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    // One utterance (300 ms voiced — clears the 250 ms min-turn guard)
    // then silence to cross the ceiling.
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    for _ in 0..4 {
        tx.send(silent_frame()).await.unwrap();
    }
    // Keep feeding silence so the speak loop's frame monitor sees no barge-in.
    let feeder = {
        let tx = tx.clone();
        tokio::spawn(async move {
            for _ in 0..50 {
                if tx.send(silent_frame()).await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
    };

    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::CommittedReply { .. }))).await;
    cancel.cancel();
    feeder.abort();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    let events = observer.snapshot();
    // Order: SpeakerEnrolled → UserTurn → SpeculativeReply → CommittedReply.
    let spk = events
        .iter()
        .position(|e| matches!(e, ConversationEvent::SpeakerEnrolled { .. }));
    let usr = events
        .iter()
        .position(|e| matches!(e, ConversationEvent::UserTurn { .. }));
    let spec = events
        .iter()
        .position(|e| matches!(e, ConversationEvent::SpeculativeReply { .. }));
    let comm = events
        .iter()
        .position(|e| matches!(e, ConversationEvent::CommittedReply { .. }));
    assert!(spk.is_some() && usr.is_some() && spec.is_some() && comm.is_some());
    assert!(spk < usr, "speaker enrolled before user turn");
    assert!(usr < spec, "user turn before speculative ack");
    assert!(
        spec < comm,
        "speculative ack must precede committed answer (the ECC handoff)"
    );

    // The user turn is speaker-attributed (named, never spoken).
    let attributed = events.iter().any(|e| {
        matches!(
            e,
            ConversationEvent::UserTurn {
                speaker: Some(_),
                speaker_name: Some(_),
                ..
            }
        )
    });
    assert!(attributed, "turn must be attributed to a speaker node");

    // The user turn carries the complete VoiceAnalysis record (§W1.2),
    // produced client-side and handed off at the observer boundary.
    let va = events
        .iter()
        .find_map(|e| match e {
            ConversationEvent::UserTurn {
                voice_analysis: Some(va),
                ..
            } => Some(va.clone()),
            _ => None,
        })
        .expect("user turn must carry a voice_analysis record");
    assert_eq!(va.v, 1, "record is versioned");
    assert_eq!(va.tier, "voice", "tier discriminator");
    assert_eq!(va.stt.model, "parakeet-tdt-0.6b", "STT model wire name");
    assert_eq!(va.speaker.embedding_dim, 3, "ECAPA-mock dim threaded onto record");
    assert!(
        matches!(
            va.speaker.action,
            SpeakerAction::Identified | SpeakerAction::Enrolled
        ),
        "speaker attributed"
    );
    assert!(va.audio.snr_db.is_finite(), "SNR computed from RMS − floor");
    assert_eq!(
        va.emotion.source,
        EmotionSource::ProsodyDsp,
        "DSP emotion floor with no SER model staged"
    );

    // Ack + answer actually reached the sink; no barge-in happened.
    assert!(
        sink.chunks.load(Ordering::SeqCst) >= 2,
        "ack + answer chunks played"
    );
    assert_eq!(
        audio.0.load(Ordering::SeqCst),
        0,
        "no AEC flush without barge-in"
    );
    assert!(!observer.has(|e| matches!(e, ConversationEvent::Interrupted)));
}

#[tokio::test]
async fn barge_in_flushes_and_emits_interrupted() {
    let observer = Arc::new(RecordingObserver::default());
    let sink = Arc::new(RecordingSink::default());
    let audio = Arc::new(CountingAudio::default());

    // Slow answer drips so the barge-in lands mid-stream.
    let tts = DualLayerTts::new(
        Arc::new(ImmediateEngine(TtsTier::Fast)),
        Arc::new(DripEngine),
    )
    .unwrap();

    let mut ctrl = TalkModeController::new(
        endpointer(),
        Arc::new(MockStt("read me the whole document please")),
        None, // no speaker ID for this test
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("Here is a long answer that will be interrupted.")),
        tts,
        sink.clone(),
        audio.clone(),
        observer.clone() as Arc<dyn ConversationObserver>,
        TalkModeConfig {
            barge_in_frames: 3,
            // Barge-in is opt-in (default off until AEC residual is tuned);
            // this test IS the barge-in path, so enable it and disable the
            // AEC-convergence grace (scripted frames land instantly).
            barge_in_enabled: true,
            barge_in_grace_ms: 0,
            ..Default::default()
        },
    );

    let (tx, rx) = mpsc::channel::<Vec<i16>>(64);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    // Utterance (≥ the 250 ms min-turn guard) + silence to finalize.
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    for _ in 0..4 {
        tx.send(silent_frame()).await.unwrap();
    }
    // Wait until the committed answer starts streaming, then BARGE IN.
    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::CommittedReply { .. }))).await;
    for _ in 0..6 {
        tx.send(voiced_frame()).await.unwrap();
    }

    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::Interrupted))).await;
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    assert!(
        observer.has(|e| matches!(e, ConversationEvent::Interrupted)),
        "barge-in must emit Interrupted (Contradicts/prune)"
    );
    assert!(
        audio.0.load(Ordering::SeqCst) >= 1,
        "barge-in must flush the AEC render reference"
    );
    assert!(
        sink.flushes.load(Ordering::SeqCst) >= 1,
        "barge-in must flush the TTS sink"
    );
}

#[tokio::test]
async fn listen_only_records_the_turn_but_skips_the_brain() {
    // §W1.4 listen-only: the turn is recorded + decomposed (UserTurn with the
    // full VoiceAnalysis fires so the recorder/observer carries it), but the
    // brain is OFF — no ack, no committed answer, no audio out.
    let observer = Arc::new(RecordingObserver::default());
    let sink = Arc::new(RecordingSink::default());
    let audio = Arc::new(CountingAudio::default());
    let tts = DualLayerTts::new(
        Arc::new(ImmediateEngine(TtsTier::Fast)),
        Arc::new(ImmediateEngine(TtsTier::Slow)),
    )
    .unwrap();

    let mut ctrl = TalkModeController::new(
        endpointer(),
        Arc::new(MockStt("what is the capital of France")),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("THIS ANSWER MUST NOT BE PRODUCED")),
        tts,
        sink.clone(),
        audio.clone(),
        observer.clone() as Arc<dyn ConversationObserver>,
        TalkModeConfig {
            listen_only: true,
            ..Default::default()
        },
    );

    let (tx, rx) = mpsc::channel::<Vec<i16>>(64);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    for _ in 0..6 {
        tx.send(silent_frame()).await.unwrap();
    }

    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::UserTurn { .. }))).await;
    // Give the loop a beat to (not) produce a reply, then stop.
    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    // The turn is recorded WITH the full decomposition...
    assert!(
        observer.has(|e| matches!(
            e,
            ConversationEvent::UserTurn {
                voice_analysis: Some(_),
                ..
            }
        )),
        "listen-only still records the turn + VoiceAnalysis"
    );
    // ...but the brain never runs: no ack, no committed reply, no audio out.
    assert!(
        !observer.has(|e| matches!(e, ConversationEvent::SpeculativeReply { .. })),
        "listen-only must not ack"
    );
    assert!(
        !observer.has(|e| matches!(e, ConversationEvent::CommittedReply { .. })),
        "listen-only must not answer"
    );
    assert_eq!(
        sink.chunks.load(Ordering::SeqCst),
        0,
        "listen-only plays no audio"
    );
}
