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

/// STT whose `transcribe` blocks on a semaphore — a stand-in for slow native
/// decode, so a test can queue a backlog behind an in-flight turn deterministically.
struct GatedStt {
    text: &'static str,
    gate: Arc<tokio::sync::Semaphore>,
}
#[async_trait]
impl SttBackend for GatedStt {
    async fn warm(&self) -> Result<(), VoiceError> {
        Ok(())
    }
    async fn transcribe(&self, _utt: &Utterance) -> Result<String, VoiceError> {
        // Hold the turn in "decode" until the test releases the gate.
        let _permit = self.gate.acquire().await.unwrap();
        Ok(self.text.to_string())
    }
    fn model(&self) -> SttModel {
        SttModel::ParakeetEnglish
    }
}

/// STT that records the sample-length of the utterance it received — lets a
/// test prove pre-roll prepended the pre-onset audio.
struct LenRecordingStt(Arc<AtomicUsize>);
#[async_trait]
impl SttBackend for LenRecordingStt {
    async fn warm(&self) -> Result<(), VoiceError> {
        Ok(())
    }
    async fn transcribe(&self, utt: &Utterance) -> Result<String, VoiceError> {
        self.0.store(utt.samples.len(), Ordering::SeqCst);
        Ok("recovered word".to_string())
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
    // In-band harmonic tone (fundamental + low harmonics + a formant), loud
    // enough to clear the energy gate and concentrated in 300–3400 Hz so it
    // also clears the spectral voiceness AND-gate. A plain Nyquist tone would
    // pass energy but score ~0 on voiceness (out of the speech band).
    use std::f32::consts::TAU;
    const SR: f32 = 16_000.0;
    let partials = [180.0f32, 360.0, 540.0, 900.0, 1400.0];
    (0..1_600)
        .map(|i| {
            let t = i as f32 / SR;
            let s: f32 = partials.iter().map(|&f| (TAU * f * t).sin()).sum();
            (s / partials.len() as f32 * 20_000.0) as i16
        })
        .collect()
}
fn silent_frame() -> Vec<i16> {
    vec![0i16; 1_600]
}
/// A quiet, below-VAD-gate frame (~-50 dBFS) — sub-threshold word attack that
/// pre-roll should retain and prepend.
fn quiet_frame() -> Vec<i16> {
    (0..1_600)
        .map(|i| if i % 2 == 0 { 100 } else { -100 })
        .collect()
}
/// Room-tone frame (~-55 dBFS) for the noise-floor STARTUP CALIBRATION window.
/// Real capture always opens with room tone before the user speaks; the gate
/// seeds its floor from the first ~500 ms (assumed non-speech), so a test that
/// opens straight into `voiced_frame` would poison the seed with speech level.
/// Kept above `quiet_frame` so the calibrated onset gate still leaves the
/// pre-onset attack sub-threshold.
fn room_tone_frame() -> Vec<i16> {
    (0..1_600)
        .map(|i| if i % 2 == 0 { 58 } else { -58 })
        .collect()
}

/// Feed the ~500 ms noise-floor calibration window (six room-tone frames) so the
/// gate seats its floor before any speech arrives — mirrors real capture.
async fn calibrate(tx: &mpsc::Sender<Vec<i16>>) {
    for _ in 0..6 {
        tx.send(room_tone_frame()).await.unwrap();
    }
}
/// Loud broadband-noise frame (~-14 dBFS) — energy well ABOVE the onset gate but
/// spectrally flat, so the spectral voiceness AND-gate must reject it. This is
/// the round-8 case energy-only VAD false-fires on. Deterministic LCG.
fn noise_frame(seed: u32) -> Vec<i16> {
    let mut state = seed.wrapping_mul(2_654_435_761).max(1);
    (0..1_600)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let u = (state >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
            ((u - 0.5) * 2.0 * 6_000.0) as i16
        })
        .collect()
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
        // Spoken self-ID enrolls a real name (the only enroll path now).
        Arc::new(MockStt("my name is Mathew and hello")),
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

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    // One utterance (300 ms voiced — clears the 250 ms min-turn guard)
    // then silence to cross the ceiling.
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    for _ in 0..9 {
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
    // The live level meter is surfaced (§W1.4 process event) as frames flow.
    assert!(
        events.iter().any(|e| matches!(
            e,
            ConversationEvent::CaptureLevel { rms_dbfs, floor_dbfs }
                if rms_dbfs.is_finite() && floor_dbfs.is_finite()
        )),
        "capture level meter must be surfaced"
    );

    // The endpoint fire is surfaced (§W1.4 process event) just before the turn.
    let ep = events
        .iter()
        .position(|e| matches!(e, ConversationEvent::EndpointFired { .. }));
    assert!(ep.is_some(), "endpoint fire must be surfaced");
    assert!(ep < usr, "endpoint fires before the user turn is committed");
    assert!(events.iter().any(|e| matches!(
        e,
        ConversationEvent::EndpointFired { source, .. } if source == "heuristic"
    )), "endpoint source is the heuristic (no smart-turn model in this test)");

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

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    // Utterance (≥ the 250 ms min-turn guard) + silence to finalize.
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    for _ in 0..9 {
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

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    tx.send(voiced_frame()).await.unwrap();
    for _ in 0..9 {
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

#[tokio::test]
async fn listen_only_cycles_multiple_turns() {
    // Regression (user acting-test bug): listen-only captured the FIRST turn
    // then never re-armed. The controller loop must finalize → record → return
    // → re-arm → finalize again, indefinitely. Drive THREE sequential
    // utterances and assert three UserTurns are observed.
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
        Arc::new(MockStt("hello there")),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("unused in listen-only")),
        tts,
        sink.clone(),
        audio.clone(),
        observer.clone() as Arc<dyn ConversationObserver>,
        TalkModeConfig {
            listen_only: true,
            ..Default::default()
        },
    );

    let (tx, rx) = mpsc::channel::<Vec<i16>>(256);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    let turns = |obs: &RecordingObserver| {
        obs.snapshot()
            .iter()
            .filter(|e| matches!(e, ConversationEvent::UserTurn { .. }))
            .count()
    };

    // Three SEQUENTIAL utterances (real speech has a gap between turns): feed a
    // turn, wait for it to record, then feed the next. Each is a voiced burst
    // (>250 ms min) + a silence tail that crosses the max-silence ceiling. The
    // post-turn drain only discards inter-turn stragglers, so all three record.
    for turn in 1..=3 {
        for _ in 0..3 {
            tx.send(voiced_frame()).await.unwrap();
        }
        for _ in 0..9 {
            tx.send(silent_frame()).await.unwrap();
        }
        wait_until(|| turns(&observer) >= turn).await;
        // Let the post-turn drain settle before the next utterance is fed.
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    assert_eq!(
        turns(&observer),
        3,
        "listen-only must cycle turns continuously (re-arm after each finalize)"
    );
}

#[tokio::test]
async fn listen_only_decodes_both_utterances_through_slow_decode() {
    // Round-3 fix (replaces the old drain): decode runs on a spawned worker so
    // the capture loop never blocks. A full SECOND utterance streams in and
    // finalizes WHILE the first is still in (gated) decode; when decode
    // releases, BOTH must be transcribed completely — no speech dropped. This
    // is RED against the previous drain (which discarded the second utterance).
    let observer = Arc::new(RecordingObserver::default());
    let sink = Arc::new(RecordingSink::default());
    let audio = Arc::new(CountingAudio::default());
    let tts = DualLayerTts::new(
        Arc::new(ImmediateEngine(TtsTier::Fast)),
        Arc::new(ImmediateEngine(TtsTier::Slow)),
    )
    .unwrap();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));

    let mut ctrl = TalkModeController::new(
        endpointer(),
        Arc::new(GatedStt {
            text: "hello there",
            gate: gate.clone(),
        }),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("unused in listen-only")),
        tts,
        sink.clone(),
        audio.clone(),
        observer.clone() as Arc<dyn ConversationObserver>,
        TalkModeConfig {
            listen_only: true,
            ..Default::default()
        },
    );

    let (tx, rx) = mpsc::channel::<Vec<i16>>(256);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    let turns = |obs: &RecordingObserver| {
        obs.snapshot()
            .iter()
            .filter(|e| matches!(e, ConversationEvent::UserTurn { .. }))
            .count()
    };

    // Turn 1 (finalizes) immediately followed by a full second utterance — both
    // queued before decode is released; the second finalizes while the first is
    // still in the worker's (gated) decode.
    for _ in 0..3 {
        tx.send(voiced_frame()).await.unwrap();
    }
    for _ in 0..9 {
        tx.send(silent_frame()).await.unwrap();
    }
    for _ in 0..3 {
        tx.send(voiced_frame()).await.unwrap();
    }
    for _ in 0..9 {
        tx.send(silent_frame()).await.unwrap();
    }

    // Let the loop finalize both turns (worker parked in gated decode on #1,
    // #2 queued behind it).
    tokio::time::sleep(Duration::from_millis(80)).await;
    // Release decode — both queued utterances now decode in order.
    gate.add_permits(100);
    wait_until(|| turns(&observer) >= 2).await;

    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    assert_eq!(
        turns(&observer),
        2,
        "both utterances must decode — the non-blocking worker drops no speech \
         during decode (RED against the old drain)"
    );
}

#[tokio::test]
async fn listen_only_marks_unknown_without_polluting_registry() {
    // Speaker-match-against-registry regression: on a non-match, listen-only
    // must NOT auto-enroll a placeholder "unknown speaker" (which pollutes the
    // persistent registry). It marks the turn Unknown and enrolls nothing.
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
        Arc::new(MockStt("this is a test")),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45), // empty — no profile to match
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("unused")),
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

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    for _ in 0..3 {
        tx.send(voiced_frame()).await.unwrap();
    }
    for _ in 0..9 {
        tx.send(silent_frame()).await.unwrap();
    }
    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::UserTurn { .. }))).await;
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    // No enrollment event fired — the registry was not polluted.
    assert!(
        !observer.has(|e| matches!(e, ConversationEvent::SpeakerEnrolled { .. })),
        "listen-only must not auto-enroll on a non-match"
    );
    let va = observer
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            ConversationEvent::UserTurn {
                voice_analysis: Some(va),
                ..
            } => Some(va),
            _ => None,
        })
        .expect("user turn recorded");
    assert_eq!(
        va.speaker.action,
        SpeakerAction::Unknown,
        "non-match in listen-only is Unknown, not Enrolled"
    );
    assert!(va.speaker.id.is_none(), "no placeholder id assigned");
}

#[tokio::test]
async fn preroll_prepends_pre_onset_audio_to_utterance() {
    // Onset-clipping regression: sub-threshold word attack (quiet frames below
    // the VAD gate) precedes the voiced frames. Pre-roll must prepend that
    // pre-onset audio, so the utterance handed to STT is longer than the
    // voiced-only slice — the leading word is recovered, not clipped.
    let recorded = Arc::new(AtomicUsize::new(0));
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
        Arc::new(LenRecordingStt(recorded.clone())),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("unused")),
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

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    // 2 quiet (pre-onset attack) → 3 voiced → 4 silence (finalize).
    for _ in 0..2 {
        tx.send(quiet_frame()).await.unwrap();
    }
    for _ in 0..3 {
        tx.send(voiced_frame()).await.unwrap();
    }
    for _ in 0..9 {
        tx.send(silent_frame()).await.unwrap();
    }
    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::UserTurn { .. }))).await;
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    let voiced_only = 3 * 1_600;
    let got = recorded.load(Ordering::SeqCst);
    assert!(
        got > voiced_only,
        "STT utterance ({got}) must exceed the voiced-only slice ({voiced_only}) — \
         pre-roll should prepend the pre-onset attack"
    );
}

#[tokio::test]
async fn talk_mode_no_self_id_is_unknown_without_pollution() {
    // Round-5: talk mode must NOT auto-enroll a placeholder "unknown speaker"
    // on a non-match — that was the registry-pollution leak. Without a spoken
    // self-ID the turn is Unknown and nothing is written to the registry.
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
        Arc::new(MockStt("what time is the meeting")), // no "my name is"
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45), // empty
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("At three.")),
        tts,
        sink.clone(),
        audio.clone(),
        observer.clone() as Arc<dyn ConversationObserver>,
        TalkModeConfig::default(), // talk mode (listen_only = false)
    );

    let (tx, rx) = mpsc::channel::<Vec<i16>>(64);
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { ctrl.run(rx, run_cancel).await });

    // Seat the noise-floor from room tone before any speech (startup calibration).
    calibrate(&tx).await;

    for _ in 0..3 {
        tx.send(voiced_frame()).await.unwrap();
    }
    for _ in 0..9 {
        tx.send(silent_frame()).await.unwrap();
    }
    wait_until(|| observer.has(|e| matches!(e, ConversationEvent::UserTurn { .. }))).await;
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    assert!(
        !observer.has(|e| matches!(e, ConversationEvent::SpeakerEnrolled { .. })),
        "talk mode must NOT enroll a placeholder without a spoken self-ID"
    );
    let va = observer
        .snapshot()
        .into_iter()
        .find_map(|e| match e {
            ConversationEvent::UserTurn {
                voice_analysis: Some(va),
                ..
            } => Some(va),
            _ => None,
        })
        .expect("user turn recorded");
    assert_eq!(va.speaker.action, SpeakerAction::Unknown);
    assert!(va.speaker.id.is_none());
}

#[tokio::test]
async fn loud_broadband_noise_produces_no_turn() {
    // Round-8 end-to-end: broadband room tone LOUDER than the onset gate (energy
    // VAD would false-fire and finalize junk turns) must produce NO utterance —
    // the spectral voiceness AND-gate rejects it because it isn't voice-shaped.
    let recorded = Arc::new(AtomicUsize::new(0));
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
        Arc::new(LenRecordingStt(recorded.clone())),
        Some(Arc::new(MockEmbedder)),
        SpeakerRegistry::new(0.45),
        VoiceAnswerPolicy::default(),
        Arc::new(MockLlm("unused")),
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

    calibrate(&tx).await;
    // A second of loud broadband noise, then silence — well above the energy
    // gate the whole time.
    for i in 0..10 {
        tx.send(noise_frame(i + 1)).await.unwrap();
    }
    for _ in 0..9 {
        tx.send(silent_frame()).await.unwrap();
    }
    // Give the loop time to process; there is nothing to wait FOR (the point is
    // that no turn ever fires), so drain deterministically then assert.
    cancel.cancel();
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;

    assert_eq!(
        recorded.load(Ordering::SeqCst),
        0,
        "broadband noise above the energy gate must not reach STT"
    );
    assert!(
        !observer.has(|e| matches!(e, ConversationEvent::UserTurn { .. })),
        "broadband noise must not finalize a user turn"
    );
}
