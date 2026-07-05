//! The ECC forest substrate + the P2 Talk-Mode loop bound to the
//! `clawft-channels` conversation seams (ADR-062 Phase 6.1).
//!
//! This is the **reconciliation point** between the two turn-taking mechanisms
//! the earlier phases built:
//!
//! - the **P2 [`TalkModeLoop`]** (`clawft-kernel`, the *orchestrator*) — ticks
//!   the [`ImpulseQueue`] on the ADR-047 self-calibrating cadence and is the
//!   **single authority** for the user-turn frontier lifecycle (commit on
//!   `EndOfUtterance`, prune on `TurnClaim`, `Continuer` on `Backchannel`);
//! - the **6.7 [`TalkModeController`]** (`clawft-channels`, now the *render
//!   shell*) — owns capture → endpoint → STT → speaker → render+barge-in, but
//!   does **not** advance node state itself.
//!
//! [`TalkForest`] owns the shared substrate (queue, graph, cross-refs, view,
//! loop). [`KernelImpulseSink`] forwards the P5 capture pipeline's
//! [`VoiceImpulse`]s into the kernel queue (the missing channel→kernel binder).
//! [`LoopObserver`] is the [`ConversationObserver`] the controller emits to: it
//! **dual-writes** each finalized user turn into the forest (ADR-062 §1.1) and
//! drives the loop (`register_turn` + an `EndOfUtterance` impulse) so the loop —
//! not the controller — performs the commit. Agent reply nodes
//! (Speculative→Committed, barge-in→Contradicts) are projected onto the same
//! graph exactly as [`EccConversationObserver`](crate::EccConversationObserver),
//! so there is **one** orchestrator and **one** graph.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use clawft_channels::voice::capture::{ImpulseSink, VoiceImpulse};
use clawft_channels::voice::speaker::SpeakerId;
use clawft_channels::voice::talkmode::{ConversationEvent, ConversationObserver};
use clawft_kernel::causal::NodeId as CausalNodeId;
use clawft_kernel::context_graft::{ChunkMeta, NodeState, SessionView, content_hash};
use clawft_kernel::view_resolver::{SingleViewResolver, ViewResolver};
use clawft_kernel::{
    CausalEdgeType, CausalGraph, CognitiveTick, CognitiveTickConfig, CrossRefStore, ImpulseQueue,
    ImpulseType, StructureTag, TalkModeConfig, TalkModeLoop, UniversalNodeId,
};
use std::collections::HashMap;

/// The shared ECC forest substrate plus the P2 [`TalkModeLoop`] orchestrator.
///
/// One per conversation. Construction is pure (no models, no I/O), so it builds
/// in tests and degrades nowhere. The `view` is a local bookkeeping
/// [`SessionView`] that backs the loop's `mirror_state` commit/prune; the deep
/// L2 graft (real embeddings) lives in `clawft-service-agent` and is out of this
/// assembly's render scope (the render grounds on the LLM directly).
pub struct TalkForest {
    conv_id: String,
    impulses: Arc<ImpulseQueue>,
    causal: Arc<CausalGraph>,
    crossrefs: Arc<CrossRefStore>,
    view: Arc<SessionView>,
    talk_loop: Arc<TalkModeLoop>,
}

impl TalkForest {
    /// Build a fresh forest + loop for `conv_id`. `dims` is the bookkeeping
    /// view's vector width (placeholder vectors only — see the struct note).
    pub fn new(conv_id: impl Into<String>, dims: usize) -> Self {
        let conv_id = conv_id.into();
        let impulses = Arc::new(ImpulseQueue::new());
        let causal = Arc::new(CausalGraph::new());
        let crossrefs = Arc::new(CrossRefStore::new());
        let view = Arc::new(SessionView::new(conv_id.clone(), dims));
        let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
        // Voice is single-conversation: the multiplexed loop resolves its one
        // bookkeeping view for any conv_id (M2 D4 "voice stays green" adapter).
        let views: Arc<dyn ViewResolver> = Arc::new(SingleViewResolver::new(view.clone()));
        let talk_loop = Arc::new(TalkModeLoop::new(
            impulses.clone(),
            causal.clone(),
            crossrefs.clone(),
            views,
            tick,
            TalkModeConfig::default(),
        ));
        Self {
            conv_id,
            impulses,
            causal,
            crossrefs,
            view,
            talk_loop,
        }
    }

    /// The kernel impulse queue (P5 capture binds its sink here).
    pub fn impulses(&self) -> &Arc<ImpulseQueue> {
        &self.impulses
    }

    /// The kernel-global causal graph (the conversation Loom).
    pub fn graph(&self) -> &Arc<CausalGraph> {
        &self.causal
    }

    /// The cross-ref store (speaker / continuer / emotion links).
    pub fn crossrefs(&self) -> &Arc<CrossRefStore> {
        &self.crossrefs
    }

    /// The bookkeeping session view (frontier lifecycle source of truth).
    pub fn view(&self) -> &Arc<SessionView> {
        &self.view
    }

    /// The P2 Talk-Mode loop (the turn-taking orchestrator / SystemService).
    pub fn talk_loop(&self) -> &Arc<TalkModeLoop> {
        &self.talk_loop
    }

    /// This conversation's id.
    pub fn conv_id(&self) -> &str {
        &self.conv_id
    }
}

// ---------------------------------------------------------------------------
// KernelImpulseSink — the P5 channel → kernel queue binder
// ---------------------------------------------------------------------------

/// Forwards the capture pipeline's [`VoiceImpulse`]s into the kernel
/// [`ImpulseQueue`] (ADR-062 Phase 5 / D5). This is the binder
/// `clawft-channels::voice::capture` documents but cannot provide without a
/// kernel dependency.
///
/// Capture emits *floor signals only*; the loop arbitrates. An onset
/// (`SpeechStart`) becomes a `TurnClaim` (floor claim, code `0x50`); an endpoint
/// (`EndOfUtterance`) becomes an `EndOfUtterance` impulse (code `0x51`). These
/// carry no `chain_seq`, so the loop treats them as floor pressure — the
/// authoritative commit fires from the [`LoopObserver`]'s registered-turn EOU.
pub struct KernelImpulseSink {
    impulses: Arc<ImpulseQueue>,
    source: [u8; 32],
}

impl KernelImpulseSink {
    /// Bind a sink to a forest's queue, attributing impulses to `source_node`.
    pub fn new(impulses: Arc<ImpulseQueue>, source_node: [u8; 32]) -> Self {
        Self {
            impulses,
            source: source_node,
        }
    }
}

impl ImpulseSink for KernelImpulseSink {
    fn emit(&self, impulse: VoiceImpulse, hlc: u64) {
        let kind = match impulse {
            VoiceImpulse::SpeechStart => ImpulseType::TurnClaim,
            VoiceImpulse::EndOfUtterance => ImpulseType::EndOfUtterance,
        };
        let tag = StructureTag::CausalGraph.as_u8();
        self.impulses
            .emit(tag, self.source, tag, kind, serde_json::json!({}), hlc);
    }
}

// ---------------------------------------------------------------------------
// LoopObserver — drives the P2 loop from the controller's lifecycle
// ---------------------------------------------------------------------------

#[derive(Default)]
struct LoopState {
    /// chain_seq → causal turn node (so a barge-in / follow can find it).
    last_turn: Option<CausalNodeId>,
    /// Most recent speculative ack node (taken when the committed answer lands).
    last_speculative: Option<CausalNodeId>,
    /// Most recent reply node (speculative or committed) for barge-in pruning.
    last_reply: Option<CausalNodeId>,
    /// speaker id → per-speaker node.
    speakers: HashMap<SpeakerId, CausalNodeId>,
}

/// The [`ConversationObserver`] that makes the P2 [`TalkModeLoop`] the single
/// turn-taking orchestrator while the controller stays the render shell.
///
/// On `UserTurn` it dual-writes the turn (ADR-062 §1.1) into the causal graph +
/// the session view (as `Frontier`), registers it with the loop, and emits an
/// `EndOfUtterance` impulse — so the loop's next tick performs the
/// Frontier→Committed commit across both substrates. Reply nodes and barge-in
/// pruning are written onto the same graph (matching
/// [`EccConversationObserver`](crate::EccConversationObserver)).
pub struct LoopObserver {
    forest: Arc<TalkForest>,
    state: Mutex<LoopState>,
    /// Monotone chain-sequence allocator for this conversation's turns.
    next_seq: AtomicU64,
    /// Monotone HLC stand-in for impulse + edge timestamps.
    clock: AtomicU64,
}

impl LoopObserver {
    /// Build over a shared forest.
    pub fn new(forest: Arc<TalkForest>) -> Self {
        Self {
            forest,
            state: Mutex::new(LoopState::default()),
            next_seq: AtomicU64::new(1),
            clock: AtomicU64::new(1),
        }
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::SeqCst)
    }

    fn link(&self, source: CausalNodeId, target: CausalNodeId, edge: CausalEdgeType) {
        self.forest
            .causal
            .link(source, target, edge, 1.0, self.tick(), 0);
    }

    /// Dual-write a finalized user turn into the forest and register it with the
    /// loop. Returns the assigned `chain_seq` so the caller can emit the EOU.
    fn dual_write_turn(&self, text: &str, role: &str) -> (u64, CausalNodeId) {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        // Session-view frontier chunk (placeholder vector — bookkeeping only).
        let meta = ChunkMeta {
            chain_seq: seq,
            content_hash: content_hash(text),
            state: NodeState::Frontier,
            kind: "agent.chat.turn".into(),
            blob_hash: None,
            inline: Some(text.to_string()),
        };
        let dims = self.forest.view.dims();
        self.forest.view.insert_vector(vec![0.0; dims], meta);
        // Causal turn node (chain_seq in metadata so the loop maps node→seq).
        let node = self.forest.causal.add_node(
            format!("turn:{}:{seq}", self.forest.conv_id),
            serde_json::json!({
                "conv_id": self.forest.conv_id,
                "chain_seq": seq,
                "role": role,
                "kind": "user_turn",
                "state": "frontier",
            }),
        );
        let uid = UniversalNodeId::new(
            &StructureTag::CausalGraph,
            self.forest.conv_id.as_bytes(),
            seq,
            text.as_bytes(),
            b"turn",
        );
        self.forest
            .talk_loop
            .register_turn(seq, node, uid, self.forest.conv_id());
        (seq, node)
    }

    /// Emit an `EndOfUtterance` impulse for `seq` so the loop commits it.
    fn emit_eou(&self, seq: u64) {
        let tag = StructureTag::CausalGraph.as_u8();
        self.forest.impulses.emit(
            tag,
            [0u8; 32],
            tag,
            ImpulseType::EndOfUtterance,
            serde_json::json!({ "chain_seq": seq }),
            self.tick(),
        );
    }

    /// Emit a `TurnClaim` (barge-in) impulse so the loop prunes the in-flight
    /// user-frontier turn (if one is still open).
    fn emit_claim(&self) {
        let tag = StructureTag::CausalGraph.as_u8();
        self.forest.impulses.emit(
            tag,
            [0u8; 32],
            tag,
            ImpulseType::TurnClaim,
            serde_json::json!({}),
            self.tick(),
        );
    }
}

impl ConversationObserver for LoopObserver {
    fn observe(&self, event: ConversationEvent) {
        match event {
            ConversationEvent::SpeakerEnrolled { id, name } => {
                let node = self.forest.causal.add_node(
                    format!("speaker:{name}"),
                    serde_json::json!({ "kind": "speaker", "speaker_id": id, "name": name }),
                );
                self.state
                    .lock()
                    .expect("loop observer state poisoned")
                    .speakers
                    .insert(id, node);
            }
            ConversationEvent::UserTurn {
                text,
                speaker,
                speaker_name,
                ..
            } => {
                let role = speaker_name.as_deref().unwrap_or("user");
                let (seq, node) = self.dual_write_turn(&text, role);
                let mut st = self.state.lock().expect("loop observer state poisoned");
                // Attribute the turn to its speaker node (speaker → turn).
                if let Some(id) = &speaker
                    && let Some(&spk) = st.speakers.get(id)
                {
                    drop(st);
                    self.link(spk, node, CausalEdgeType::Causes);
                    st = self.state.lock().expect("loop observer state poisoned");
                }
                st.last_turn = Some(node);
                st.last_speculative = None;
                st.last_reply = None;
                drop(st);
                // Hand the commit to the loop (orchestration), not the observer.
                self.emit_eou(seq);
            }
            ConversationEvent::SpeculativeReply { ack } => {
                let s = self.forest.causal.add_node(
                    ack,
                    serde_json::json!({ "kind": "reply", "state": "speculative" }),
                );
                let mut st = self.state.lock().expect("loop observer state poisoned");
                let turn = st.last_turn;
                st.last_speculative = Some(s);
                st.last_reply = Some(s);
                drop(st);
                if let Some(t) = turn {
                    self.link(s, t, CausalEdgeType::EvidenceFor);
                }
            }
            ConversationEvent::CommittedReply { answer } => {
                let c = self.forest.causal.add_node(
                    answer,
                    serde_json::json!({ "kind": "reply", "state": "committed" }),
                );
                let mut st = self.state.lock().expect("loop observer state poisoned");
                let turn = st.last_turn;
                let spec = st.last_speculative.take();
                st.last_reply = Some(c);
                drop(st);
                if let Some(t) = turn {
                    self.link(c, t, CausalEdgeType::EvidenceFor);
                }
                if let Some(s) = spec {
                    self.link(c, s, CausalEdgeType::Enables);
                }
            }
            ConversationEvent::Interrupted => {
                let i = self.forest.causal.add_node(
                    "interrupted".to_string(),
                    serde_json::json!({ "kind": "interrupt", "state": "pruned" }),
                );
                let reply = self
                    .state
                    .lock()
                    .expect("loop observer state poisoned")
                    .last_reply
                    .take();
                if let Some(r) = reply {
                    self.link(i, r, CausalEdgeType::Contradicts);
                }
                // Drive the floor: the barge-in claims back the floor.
                self.emit_claim();
            }
        }
    }
}

#[cfg(test)]
#[path = "forest_tests.rs"]
mod tests;
