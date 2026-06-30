//! Render-FROM-node: the missing node→render path (ADR-062 Phase 2.2 / 2.3).
//!
//! ADR-062 D3: a voice turn is **one actor running two self-branches**:
//!
//! - a **Speculative node** — a *cheap graph-local read* off the frontier (a
//!   contextual ack derived from the in-flight turn node, **not** a second LLM
//!   call) — rendered immediately by the fast TTS layer to cover latency; then
//! - a **Committed node** — the deep walk (graft → ground on the local LLM) —
//!   that **supersedes** the speculative one (an `Enables` edge + the
//!   speculative node aged to `stale`, the IU `Retained` outcome).
//!
//! When the committed answer **contradicts** what the spoken speculative ack
//! already said, we do **not** silently abandon it: the speculative node is
//! `pruned` and an **overt-repair node** ("sorry — I started to say X …") is
//! emitted and spoken (ADR-062 D3, the IU survey's strongest warning).
//!
//! This is the concrete binding the kernel stays free of: it wires the generic
//! `clawft-channels` voice seams ([`VoiceLlm`], [`DualLayerTts`]/[`TtsSink`]) to
//! the kernel ECC [`CausalGraph`]. Reply-node lifecycle lives in
//! `metadata.state ∈ {speculative, committed, stale, pruned}` and the supersede
//! relationships are real [`CausalEdge`](clawft_kernel::causal::CausalEdge)s,
//! exactly as the existing [`EccConversationObserver`](crate::ecc) does — no
//! parallel mechanism.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;

use clawft_channels::voice::policy::{VoiceAnswerPolicy, VoiceLlm};
use clawft_channels::voice::talkmode::contextual_ack;
use clawft_channels::voice::tts::{DualLayerTts, TtsSink};
use clawft_channels::voice::types::VoiceError;
use clawft_kernel::{CausalEdgeType, CausalGraph};

type NodeId = u64;

// ---------------------------------------------------------------------------
// Contradiction detector (the 2.3 seam)
// ---------------------------------------------------------------------------

/// Decides whether a grounded committed answer **contradicts** the speculative
/// ack that was already spoken (ADR-062 D3). The production detector (an
/// LLM-judge / entailment check) is future work; the renderer is generic over
/// this seam so the overt-repair path is unit-testable without a model.
pub trait ContradictionDetector: Send + Sync {
    /// `true` when `committed_answer` contradicts what `spoken_ack` implied.
    fn contradicts(&self, spoken_ack: &str, committed_answer: &str) -> bool;
}

/// Default v0.1 detector: a cheap ack ("one sec.") asserts nothing about the
/// answer, so it can never contradict it. Real semantic contradiction detection
/// is deferred (Weaver / LLM-judge).
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverContradicts;

impl ContradictionDetector for NeverContradicts {
    fn contradicts(&self, _spoken_ack: &str, _committed_answer: &str) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Render outcome
// ---------------------------------------------------------------------------

/// What a turn render produced (for provenance + tests).
#[derive(Debug, Clone)]
pub struct RenderOutcome {
    /// The speculative ack node (rendered first, fast layer).
    pub speculative_node: NodeId,
    /// The committed grounded-answer node (the content of record).
    pub committed_node: NodeId,
    /// The overt-repair node, emitted only when the committed answer
    /// contradicted the spoken ack (ADR-062 D3 / Phase 2.3).
    pub repair_node: Option<NodeId>,
    /// How the committed node superseded the speculative one: [`Enables`]
    /// (consistent) or [`Contradicts`] (a repair was needed).
    ///
    /// [`Enables`]: CausalEdgeType::Enables
    /// [`Contradicts`]: CausalEdgeType::Contradicts
    pub superseded_via: CausalEdgeType,
    /// The ack text that was spoken on the fast layer.
    pub spoken_ack: String,
    /// The text spoken on the slow layer (the answer, or the repair).
    pub spoken_answer: String,
}

// ---------------------------------------------------------------------------
// NodeRenderer
// ---------------------------------------------------------------------------

/// Renders responses FROM nodes for one conversation (ADR-062 Phase 2.2 / 2.3).
///
/// Generic over the [`ContradictionDetector`] seam; binds the
/// `clawft-channels` voice seams to the kernel ECC graph.
pub struct NodeRenderer<D: ContradictionDetector = NeverContradicts> {
    graph: Arc<CausalGraph>,
    llm: Arc<dyn VoiceLlm>,
    tts: DualLayerTts,
    sink: Arc<dyn TtsSink>,
    policy: VoiceAnswerPolicy,
    detector: D,
    base_system: String,
    /// Monotone HLC stand-in for edge timestamps (the daemon supplies real HLC
    /// later; structure + state are what matter here, as in [`crate::ecc`]).
    clock: AtomicU64,
}

impl NodeRenderer<NeverContradicts> {
    /// Build a renderer with the default (never-contradicts) detector.
    pub fn new(
        graph: Arc<CausalGraph>,
        llm: Arc<dyn VoiceLlm>,
        tts: DualLayerTts,
        sink: Arc<dyn TtsSink>,
    ) -> Self {
        Self::with_detector(graph, llm, tts, sink, NeverContradicts)
    }
}

impl<D: ContradictionDetector> NodeRenderer<D> {
    /// Build a renderer with an explicit contradiction detector (the 2.3 seam).
    pub fn with_detector(
        graph: Arc<CausalGraph>,
        llm: Arc<dyn VoiceLlm>,
        tts: DualLayerTts,
        sink: Arc<dyn TtsSink>,
        detector: D,
    ) -> Self {
        Self {
            graph,
            llm,
            tts,
            sink,
            policy: VoiceAnswerPolicy::default(),
            detector,
            base_system: String::new(),
            clock: AtomicU64::new(1),
        }
    }

    /// Override the base agent persona prepended to the spoken-answer policy.
    pub fn with_base_system(mut self, system: impl Into<String>) -> Self {
        self.base_system = system.into();
        self
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::SeqCst)
    }

    fn link(&self, source: NodeId, target: NodeId, edge: CausalEdgeType) {
        // chain_seq 0: provenance HLC/chain wiring is the daemon's; the edge
        // semantics + graph structure are what matter here (cf. crate::ecc).
        self.graph.link(source, target, edge, 1.0, self.tick(), 0);
    }

    /// The **cheap graph-local read** (ADR-062 D3): derive a contextual ack from
    /// the in-flight turn node — a graph read of the frontier head, **never** a
    /// second LLM call. Falls back to the supplied transcript if the node has no
    /// readable label.
    fn speculative_read(&self, turn_node: NodeId, transcript: &str) -> String {
        let frontier_text = self
            .graph
            .get_node(turn_node)
            .map(|n| n.label)
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| transcript.to_string());
        contextual_ack(&frontier_text)
    }

    /// Render a turn FROM its frontier node: speculative ack (fast) → committed
    /// grounded answer (slow) that supersedes it; an overt repair when the
    /// committed answer contradicts the spoken ack.
    ///
    /// `turn_node` is the user-utterance node (from the Phase-1 dual-write +
    /// the Phase-2.1 loop); `transcript` is its text; `speaker_context` is
    /// private (fed to the LLM, never spoken). The deep walk's graft is the
    /// caller's (Phase 1 `lineage_fuse`); here the LLM is the grounded brain.
    pub async fn render_turn(
        &self,
        turn_node: NodeId,
        transcript: &str,
        speaker_context: Option<String>,
        cancel: CancellationToken,
    ) -> Result<RenderOutcome, VoiceError> {
        // ── Speculative branch: a cheap graph-local read, NOT an LLM call. ──
        let spoken_ack = self.speculative_read(turn_node, transcript);
        let speculative_node = self.graph.add_node(
            spoken_ack.clone(),
            serde_json::json!({ "kind": "reply", "state": "speculative" }),
        );
        self.link(speculative_node, turn_node, CausalEdgeType::EvidenceFor);

        // ── Committed branch: the deep walk — ground on the local LLM. ──
        let answer = self
            .policy
            .answer(&*self.llm, &self.base_system, transcript, speaker_context)
            .await?;
        let committed_node = self.graph.add_node(
            answer.clone(),
            serde_json::json!({ "kind": "reply", "state": "committed" }),
        );
        self.link(committed_node, turn_node, CausalEdgeType::EvidenceFor);

        // ── Supersede (consistent) or repair (contradiction). ──
        let contradicts = self.detector.contradicts(&spoken_ack, &answer);
        let (superseded_via, repair_node, spoken_answer) = if contradicts {
            // Overt repair: prune the contradicted ack, emit + speak a repair
            // node rather than silently abandoning the ack (ADR-062 D3).
            self.link(
                committed_node,
                speculative_node,
                CausalEdgeType::Contradicts,
            );
            self.graph.set_node_state(speculative_node, "pruned");
            let repair = repair_text(&spoken_ack, &answer);
            let repair_node = self.graph.add_node(
                repair.clone(),
                serde_json::json!({ "kind": "repair", "state": "committed" }),
            );
            // The repair contradicts the spoken ack and is evidenced by the turn.
            self.link(repair_node, speculative_node, CausalEdgeType::Contradicts);
            self.link(repair_node, turn_node, CausalEdgeType::EvidenceFor);
            (CausalEdgeType::Contradicts, Some(repair_node), repair)
        } else {
            // Consistent supersession: the committed answer Enables (extends)
            // the ack, which is Retained but aged to stale (not authoritative).
            self.link(committed_node, speculative_node, CausalEdgeType::Enables);
            self.graph.set_node_state(speculative_node, "stale");
            (CausalEdgeType::Enables, None, answer.clone())
        };

        // ── Render: fast ack FIRST, then the slow answer/repair supersedes. ──
        // DualLayerTts guarantees ack chunks land before answer chunks, so the
        // Speculative render is audibly first and the Committed render follows.
        self.tts
            .speak(Some(&spoken_ack), &spoken_answer, self.sink.clone(), cancel)
            .await?;

        Ok(RenderOutcome {
            speculative_node,
            committed_node,
            repair_node,
            superseded_via,
            spoken_ack,
            spoken_answer,
        })
    }
}

/// Build the overt-repair utterance (ADR-062 D3). Announces the correction
/// rather than abandoning the spoken ack silently.
fn repair_text(spoken_ack: &str, committed_answer: &str) -> String {
    format!("Sorry — I started to say \"{spoken_ack}\". Actually, {committed_answer}")
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
