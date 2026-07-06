//! Deterministic assembly test (ADR-062 Phase 6.2) — the GATE.
//!
//! Proves the **wired** `weft talk` pipeline runs a full turn end-to-end with
//! mock STT / TTS / LLM / endpoint (no audio device, no model, no Hermes): a
//! scripted frame stream drives capture → endpoint → STT → grounded answer →
//! Speculative ack → Committed answer, and the P2 loop (the orchestrator)
//! commits the user turn onto the forest. This exercises the real assembly
//! (`TalkSession::assemble`), the real controller, and the real
//! [`LoopObserver`]→loop reconciliation — only the leaf I/O is mocked.

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::policy::{VoiceLlm, VoiceTurnRequest};
use clawft_channels::voice::stt::{SttBackend, SttModel, Utterance};
use clawft_channels::voice::talkmode::NoopAudioControl;
use clawft_channels::voice::tts::{DualLayerTts, TtsChunk, TtsEngine, TtsSink, TtsTier};
use clawft_channels::voice::turn::{EndpointModel, SemanticEndpointer};
use clawft_channels::voice::types::VoiceError;
use clawft_kernel::context_graft::NodeState;

use super::*;

// ── Mocks (leaf I/O only) ────────────────────────────────────────────────────

struct AlwaysComplete;
#[async_trait]
impl EndpointModel for AlwaysComplete {
    async fn completion_prob(&self, _audio: &[i16], _partial: &str) -> f32 {
        1.0
    }
}

struct ScriptedStt(String);
#[async_trait]
impl SttBackend for ScriptedStt {
    async fn warm(&self) -> Result<(), VoiceError> {
        Ok(())
    }
    async fn transcribe(&self, _utt: &Utterance) -> Result<String, VoiceError> {
        Ok(self.0.clone())
    }
    fn model(&self) -> SttModel {
        SttModel::ParakeetEnglish
    }
}

struct ScriptedLlm(String);
#[async_trait]
impl VoiceLlm for ScriptedLlm {
    async fn complete(&self, _req: &VoiceTurnRequest) -> Result<String, VoiceError> {
        Ok(self.0.clone())
    }
}

/// One-chunk mock engine for a given tier (proves the dual-layer wiring).
struct MockEngine(TtsTier);
#[async_trait]
impl TtsEngine for MockEngine {
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        _cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        if !text.trim().is_empty() {
            let _ = tx
                .send(TtsChunk {
                    samples: vec![0i16; 8],
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

#[derive(Default)]
struct RecordingSink {
    spoken: Mutex<usize>,
}
#[async_trait]
impl TtsSink for RecordingSink {
    async fn play_chunk(&self, _chunk: &TtsChunk) -> Result<(), VoiceError> {
        *self.spoken.lock().unwrap() += 1;
        Ok(())
    }
    async fn flush(&self) {}
}

fn voiced_frame(len: usize) -> Vec<i16> {
    // In-band harmonic tone (fundamental + low harmonics + a formant) so the
    // frame clears BOTH stages of the round-8 gate: loud enough for the energy
    // gate and concentrated in 300–3400 Hz for the spectral voiceness AND-gate.
    // (A ±amplitude square alternation is a Nyquist tone — energy passes but
    // voiceness scores ~0.)
    use std::f32::consts::TAU;
    const SR: f32 = 16_000.0;
    let partials = [180.0f32, 360.0, 540.0, 900.0, 1400.0];
    (0..len)
        .map(|i| {
            let t = i as f32 / SR;
            let s: f32 = partials.iter().map(|&f| (TAU * f * t).sin()).sum();
            (s / partials.len() as f32 * 20_000.0) as i16
        })
        .collect()
}

/// Room-tone frame (~-55 dBFS) for the noise-floor STARTUP CALIBRATION window.
/// The gate seeds its floor from the first ~500 ms (assumed non-speech), so a
/// test that opens straight into `voiced_frame` would have its speech eaten as
/// calibration and poison the floor seed (same idiom as
/// `clawft-channels/tests/talkmode.rs`).
fn room_tone_frame(len: usize) -> Vec<i16> {
    (0..len)
        .map(|i| if i % 2 == 0 { 58 } else { -58 })
        .collect()
}

/// THE gate: a scripted turn drives the assembled pipeline; the loop commits.
#[tokio::test]
async fn assembled_pipeline_runs_a_full_turn_end_to_end() {
    let config = TalkConfig {
        conv_id: "assembly".into(),
        ..TalkConfig::default()
    };
    let tts = DualLayerTts::new(
        Arc::new(MockEngine(TtsTier::Fast)),
        Arc::new(MockEngine(TtsTier::Slow)),
    )
    .expect("fast+slow tiers");
    let sink = Arc::new(RecordingSink::default());
    let components = TalkComponents {
        endpointer: SemanticEndpointer::new(AlwaysComplete, config.sample_rate, 250, 2_000, 0.5),
        stt: Arc::new(ScriptedStt("what is the capital of France".into())),
        embedder: None,
        llm: Arc::new(ScriptedLlm("Paris.".into())),
        tts,
        sink: sink.clone(),
        audio: Arc::new(NoopAudioControl),
    };

    // The real assembly under test.
    let session = TalkSession::assemble(config, components);
    let forest = session.forest().clone();
    let TalkSession { mut controller, .. } = session;

    // Scripted capture, mirroring what the round-8 gate needs from real audio:
    // ~600 ms room tone (seats the startup-calibration noise floor — real
    // capture always opens with room tone), then ~600 ms voiced (builds the
    // utterance; clears the 400 ms min-voiced endpoint gate AND the 500 ms
    // short-audio discount window so AlwaysComplete's prob isn't halved), then
    // ~800 ms silence — the first 400 ms is eaten by the VAD hangover tail
    // (still reported voiced), the rest crosses the 250 ms endpoint threshold →
    // AlwaysComplete finalizes. `tx` stays open for the whole turn so the
    // barge-in monitor never sees a closed channel mid-render (a live mic
    // streams continuously).
    let (tx, rx) = mpsc::channel::<Vec<i16>>(64);
    let frame = 1_600usize; // 100 ms @ 16 kHz
    for _ in 0..6 {
        tx.send(room_tone_frame(frame)).await.unwrap();
    }
    for _ in 0..6 {
        tx.send(voiced_frame(frame)).await.unwrap();
    }
    for _ in 0..8 {
        tx.send(vec![0i16; frame]).await.unwrap();
    }

    // Run the render shell as a task; stop it once the turn has been spoken.
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let handle = tokio::spawn(async move { controller.run(rx, run_cancel).await });
    for _ in 0..500 {
        if *sink.spoken.lock().unwrap() >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    cancel.cancel();
    drop(tx);
    tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("controller stopped")
        .expect("controller task did not panic");

    // The render shell spoke (ack + answer chunks reached the sink).
    assert!(
        *sink.spoken.lock().unwrap() >= 2,
        "ack + answer were spoken"
    );

    // The user turn was dual-written + registered (Frontier) — the observer's
    // job — and the loop (orchestrator) is what commits it. Drive one tick.
    assert_eq!(forest.view().state(1), Some(NodeState::Frontier));
    let r = forest.talk_loop().tick();
    assert_eq!(r.commits, 1, "the P2 loop commits the user turn");
    assert_eq!(forest.view().state(1), Some(NodeState::Committed));

    // The conversation landed on the graph: user turn + speculative + committed
    // reply nodes (≥ 3), proving the full Speculative→Committed handoff wrote.
    assert!(
        forest.graph().node_count() >= 3,
        "turn + speculative + committed nodes on the CausalGraph"
    );
}
