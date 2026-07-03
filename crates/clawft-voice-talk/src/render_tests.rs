//! Tests for render-from-node (ADR-062 Phase 2.2 / 2.3). Deterministic, mock
//! seams only — no live Hermes, no live TTS.

use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc;

use clawft_channels::voice::policy::VoiceTurnRequest;
use clawft_channels::voice::tts::{TtsChunk, TtsEngine, TtsTier, split_sentences};

// ── Mock seams ────────────────────────────────────────────────────────────

/// A `VoiceLlm` that returns a fixed grounded answer (no endpoint).
struct MockLlm(&'static str);

#[async_trait]
impl VoiceLlm for MockLlm {
    async fn complete(&self, _req: &VoiceTurnRequest) -> Result<String, VoiceError> {
        Ok(self.0.to_string())
    }
}

/// A detector that always reports contradiction (forces the 2.3 repair path).
struct AlwaysContradicts;
impl ContradictionDetector for AlwaysContradicts {
    fn contradicts(&self, _ack: &str, _answer: &str) -> bool {
        true
    }
}

/// One-chunk-per-sentence fake engine that stamps a tier marker so the sink can
/// prove the ack (fast) was spoken before the answer (slow).
struct FakeEngine {
    tier: TtsTier,
    marker: i16,
}

#[async_trait]
impl TtsEngine for FakeEngine {
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        for _ in split_sentences(text) {
            if cancel.is_cancelled() {
                break;
            }
            let chunk = TtsChunk {
                samples: vec![self.marker; 160],
                sample_rate: 16_000,
            };
            if tx.send(chunk).await.is_err() {
                break;
            }
        }
        Ok(())
    }
    fn tier(&self) -> TtsTier {
        self.tier
    }
}

#[derive(Default)]
struct RecordingSink {
    markers: tokio::sync::Mutex<Vec<i16>>,
    flushes: AtomicUsize,
}

#[async_trait]
impl TtsSink for RecordingSink {
    async fn play_chunk(&self, chunk: &TtsChunk) -> Result<(), VoiceError> {
        self.markers.lock().await.push(chunk.samples[0]);
        Ok(())
    }
    async fn flush(&self) {
        self.flushes.fetch_add(1, Ordering::SeqCst);
    }
}

const FAST: i16 = 1;
const SLOW: i16 = 2;

fn dual() -> DualLayerTts {
    let fast = Arc::new(FakeEngine {
        tier: TtsTier::Fast,
        marker: FAST,
    });
    let slow = Arc::new(FakeEngine {
        tier: TtsTier::Slow,
        marker: SLOW,
    });
    DualLayerTts::new(fast, slow).unwrap()
}

/// 2.2: a turn emits a Speculative render first, then a Committed render that
/// supersedes it — state transitions + supersede edge asserted (mock seams).
#[tokio::test]
async fn speculative_then_committed_supersedes() {
    let graph = Arc::new(CausalGraph::new());
    // The user-utterance (frontier) node — produced upstream by P1 + P2.1.
    let turn = graph.add_node(
        "tell me about Paris please now".to_string(),
        serde_json::json!({ "kind": "user_turn", "state": "committed" }),
    );

    let sink = Arc::new(RecordingSink::default());
    let renderer = NodeRenderer::new(
        graph.clone(),
        Arc::new(MockLlm("Paris is the capital of France.")),
        dual(),
        sink.clone(),
    );

    let outcome = renderer
        .render_turn(
            turn,
            "tell me about Paris please now",
            None,
            CancellationToken::new(),
        )
        .await
        .unwrap();

    // Render ordering: the fast ack chunk is spoken BEFORE any slow answer chunk.
    let order = sink.markers.lock().await.clone();
    assert!(!order.is_empty());
    assert_eq!(order[0], FAST, "speculative ack must render first");
    assert!(order.contains(&SLOW), "committed answer must render");
    let first_slow = order.iter().position(|&m| m == SLOW).unwrap();
    assert!(
        order[..first_slow].iter().all(|&m| m == FAST),
        "all fast (speculative) chunks precede the first slow (committed) chunk"
    );

    // The ack is a cheap fixed phrase from the closed cacheable set (so the
    // slow tier pre-renders it in the answer's voice), NOT the LLM answer.
    // Subject echo ("Paris — one sec.") returns with WEFT-613.
    assert_eq!(
        outcome.spoken_ack,
        clawft_channels::voice::talkmode::ACK_LONG
    );
    assert_ne!(outcome.spoken_ack, outcome.spoken_answer);

    // State transitions: the committed reply is committed; the speculative ack
    // is Retained → aged to stale (superseded, not authoritative).
    assert_eq!(
        graph.get_node(outcome.committed_node).unwrap().metadata["state"],
        "committed"
    );
    assert_eq!(
        graph.get_node(outcome.speculative_node).unwrap().metadata["state"],
        "stale"
    );

    // Supersede edge: committed --Enables--> speculative.
    assert_eq!(outcome.superseded_via, CausalEdgeType::Enables);
    let enables = graph.get_edges_by_type(outcome.committed_node, &CausalEdgeType::Enables);
    assert_eq!(enables.len(), 1);
    assert_eq!(enables[0].target, outcome.speculative_node);

    // No repair on the consistent path.
    assert!(outcome.repair_node.is_none());
}

/// 2.3: when the committed answer contradicts the spoken speculative ack, an
/// overt repair node is emitted and rendered — never a silent abandon.
#[tokio::test]
async fn contradiction_emits_and_renders_overt_repair() {
    let graph = Arc::new(CausalGraph::new());
    let turn = graph.add_node(
        "did the Eagles win last night".to_string(),
        serde_json::json!({ "kind": "user_turn", "state": "committed" }),
    );

    let sink = Arc::new(RecordingSink::default());
    let renderer = NodeRenderer::with_detector(
        graph.clone(),
        Arc::new(MockLlm("No — they lost in overtime.")),
        dual(),
        sink.clone(),
        AlwaysContradicts,
    );

    let outcome = renderer
        .render_turn(
            turn,
            "did the Eagles win last night",
            None,
            CancellationToken::new(),
        )
        .await
        .unwrap();

    // A repair node was emitted (not a silent abandon).
    let repair_node = outcome.repair_node.expect("repair node must be emitted");
    assert_eq!(
        graph.get_node(repair_node).unwrap().metadata["kind"],
        "repair"
    );

    // The contradicted speculative ack is hard-pruned.
    assert_eq!(
        graph.get_node(outcome.speculative_node).unwrap().metadata["state"],
        "pruned"
    );

    // The committed answer Contradicts the speculative ack (the supersede edge).
    assert_eq!(outcome.superseded_via, CausalEdgeType::Contradicts);
    let committed_contradicts =
        graph.get_edges_by_type(outcome.committed_node, &CausalEdgeType::Contradicts);
    assert_eq!(committed_contradicts.len(), 1);
    assert_eq!(committed_contradicts[0].target, outcome.speculative_node);

    // The repair node also Contradicts the spoken ack it is correcting.
    let repair_contradicts = graph.get_edges_by_type(repair_node, &CausalEdgeType::Contradicts);
    assert_eq!(repair_contradicts.len(), 1);
    assert_eq!(repair_contradicts[0].target, outcome.speculative_node);

    // What is SPOKEN on the slow layer is the overt repair, and it was rendered.
    assert!(outcome.spoken_answer.starts_with("Sorry —"));
    assert!(outcome.spoken_answer.contains("lost in overtime"));
    let order = sink.markers.lock().await.clone();
    assert_eq!(order[0], FAST, "the ack is still spoken first");
    assert!(
        order.contains(&SLOW),
        "the overt repair is rendered on the slow layer"
    );
}
