//! [`ConversationObserver`] binding that writes the conversation onto the
//! kernel ECC `CausalGraph` (ADR-061 §7 / ADR-056).
//!
//! The speculative→committed handoff IS the kernel `NodeState` lifecycle — we
//! do not build a parallel mechanism. Since `CausalNode` carries free-form JSON
//! metadata (there is no dedicated `NodeState` enum in the kernel), the state
//! lives in `metadata.state` ∈ {speculative, committed, pruned} and the
//! lifecycle relationships are real `CausalEdge`s:
//!
//! - user utterance → a committed node, `Follows` the previous turn;
//! - speaker → a per-speaker node; the turn is attributed `Causes` (speaker
//!   produced turn) — the multiplayer foundation;
//! - fast ack → a `speculative` node, `EvidenceFor` the turn;
//! - grounded answer → a `committed` node, `EvidenceFor` the turn and `Enables`
//!   (supersedes/extends) the speculative ack;
//! - barge-in → a `pruned` node that `Contradicts` the in-flight reply.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use clawft_channels::voice::speaker::SpeakerId;
use clawft_channels::voice::talkmode::{ConversationEvent, ConversationObserver};
use clawft_kernel::{CausalEdgeType, CausalGraph};

type NodeId = u64;

#[derive(Default)]
struct EccState {
    /// Most recent committed user turn (for `Follows` chaining + attribution).
    prev_turn: Option<NodeId>,
    last_turn: Option<NodeId>,
    /// In-flight speculative ack node, taken when the committed answer lands.
    last_speculative: Option<NodeId>,
    /// Most recent reply node (speculative or committed) for barge-in pruning.
    last_reply: Option<NodeId>,
    /// speaker id → per-speaker node id.
    speakers: HashMap<SpeakerId, NodeId>,
}

/// Writes conversation lifecycle events onto a shared [`CausalGraph`].
pub struct EccConversationObserver {
    graph: Arc<CausalGraph>,
    state: Mutex<EccState>,
    clock: AtomicU64,
}

impl EccConversationObserver {
    /// Build over a shared causal graph (the conversation Loom).
    pub fn new(graph: Arc<CausalGraph>) -> Self {
        Self {
            graph,
            state: Mutex::new(EccState::default()),
            clock: AtomicU64::new(1),
        }
    }

    /// The graph being written to (for inspection / persistence).
    pub fn graph(&self) -> &Arc<CausalGraph> {
        &self.graph
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::SeqCst)
    }

    fn link(&self, source: NodeId, target: NodeId, edge: CausalEdgeType) {
        let ts = self.tick();
        // chain_seq 0: provenance HLC/chain wiring is the daemon's to supply;
        // the edge semantics + graph structure are what matter here.
        self.graph.link(source, target, edge, 1.0, ts, 0);
    }
}

impl ConversationObserver for EccConversationObserver {
    fn observe(&self, event: ConversationEvent) {
        let mut st = self.state.lock().expect("ecc observer state poisoned");
        match event {
            ConversationEvent::SpeakerEnrolled { id, name } => {
                let node = self.graph.add_node(
                    format!("speaker:{name}"),
                    serde_json::json!({ "kind": "speaker", "speaker_id": id, "name": name }),
                );
                st.speakers.insert(id, node);
            }
            ConversationEvent::UserTurn {
                text,
                speaker,
                speaker_name,
                ..
            } => {
                let t = self.graph.add_node(
                    text,
                    serde_json::json!({
                        "kind": "user_turn",
                        "state": "committed",
                        "speaker": speaker,
                        "speaker_name": speaker_name,
                    }),
                );
                if let Some(prev) = st.prev_turn {
                    self.link(t, prev, CausalEdgeType::Follows);
                }
                // Attribute the turn to its speaker node (speaker produced turn).
                if let Some(id) = &speaker
                    && let Some(&spk) = st.speakers.get(id)
                {
                    self.link(spk, t, CausalEdgeType::Causes);
                }
                st.prev_turn = Some(t);
                st.last_turn = Some(t);
                st.last_speculative = None;
                st.last_reply = None;
            }
            ConversationEvent::SpeculativeReply { ack } => {
                let s = self.graph.add_node(
                    ack,
                    serde_json::json!({ "kind": "reply", "state": "speculative" }),
                );
                if let Some(t) = st.last_turn {
                    self.link(s, t, CausalEdgeType::EvidenceFor);
                }
                st.last_speculative = Some(s);
                st.last_reply = Some(s);
            }
            ConversationEvent::CommittedReply { answer } => {
                let c = self.graph.add_node(
                    answer,
                    serde_json::json!({ "kind": "reply", "state": "committed" }),
                );
                if let Some(t) = st.last_turn {
                    self.link(c, t, CausalEdgeType::EvidenceFor);
                }
                if let Some(s) = st.last_speculative.take() {
                    // Committed answer supersedes / extends the speculative ack.
                    self.link(c, s, CausalEdgeType::Enables);
                }
                st.last_reply = Some(c);
            }
            // §W1.4 process events are surface-only — not committed graph
            // nodes. SpeculativeFiller (WEFT-658) joins this group: it's an
            // ephemeral "thinking" line, not a reply the ECC forest tracks.
            // TurnGated (WEFT-659) joins it too: the gated turn's `UserTurn`
            // already recorded the decomposition; this is just the "why no
            // reply followed" explanation for the surface.
            ConversationEvent::EndpointFired { .. }
            | ConversationEvent::PartialTranscript { .. }
            | ConversationEvent::CaptureLevel { .. }
            | ConversationEvent::SpeculativeFiller { .. }
            | ConversationEvent::CueTone { .. }
            | ConversationEvent::TtsRendered { .. }
            | ConversationEvent::TurnGated { .. } => {}
            ConversationEvent::Interrupted => {
                let i = self.graph.add_node(
                    "interrupted".to_string(),
                    serde_json::json!({ "kind": "interrupt", "state": "pruned" }),
                );
                if let Some(r) = st.last_reply.take() {
                    self.link(i, r, CausalEdgeType::Contradicts);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_speculative_then_committed_handoff() {
        let graph = Arc::new(CausalGraph::new());
        let obs = EccConversationObserver::new(graph.clone());

        obs.observe(ConversationEvent::SpeakerEnrolled {
            id: "spk-0".into(),
            name: "Alice".into(),
        });
        obs.observe(ConversationEvent::UserTurn {
            text: "what's the capital of France".into(),
            speaker: Some("spk-0".into()),
            speaker_name: Some("Alice".into()),
            voice_analysis: None,
        });
        obs.observe(ConversationEvent::SpeculativeReply {
            ack: "one sec.".into(),
        });
        obs.observe(ConversationEvent::CommittedReply {
            answer: "Paris.".into(),
        });

        // 4 nodes: speaker, turn, speculative, committed.
        assert_eq!(graph.node_count(), 4);
        // Edges: turn→speaker(Causes), spec→turn(EvidenceFor),
        // committed→turn(EvidenceFor), committed→spec(Enables) = 4.
        assert_eq!(graph.edge_count(), 4);
    }

    #[test]
    fn second_turn_follows_first_and_barge_in_contradicts() {
        let graph = Arc::new(CausalGraph::new());
        let obs = EccConversationObserver::new(graph.clone());

        obs.observe(ConversationEvent::UserTurn {
            text: "turn one".into(),
            speaker: None,
            speaker_name: None,
            voice_analysis: None,
        });
        obs.observe(ConversationEvent::SpeculativeReply { ack: "ok".into() });
        // Barge-in prunes the in-flight reply.
        obs.observe(ConversationEvent::Interrupted);
        obs.observe(ConversationEvent::UserTurn {
            text: "turn two".into(),
            speaker: None,
            speaker_name: None,
            voice_analysis: None,
        });

        // Nodes: turn1, spec, interrupt, turn2 = 4.
        assert_eq!(graph.node_count(), 4);
        // Edges: spec→turn1(EvidenceFor), interrupt→spec(Contradicts),
        // turn2→turn1(Follows) = 3.
        assert_eq!(graph.edge_count(), 3);
    }
}
