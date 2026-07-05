//! Forest join for the L2 [`SessionTier`](crate::session_tier::SessionTier)
//! (ADR-062 Phase 1, ADR-046).
//!
//! The live conversation path historically indexed each turn into a per-session
//! HNSW and recalled by cosine alone. This module joins that path to the rest of
//! the **ADR-046 forest**: each indexed turn is *also* written to the
//! kernel-global [`CausalGraph`] (a node + a `Follows` edge to the prior turn)
//! and its speaker is registered as a [`CrossRef`], so recall can walk **lineage
//! + cross-structure links**, not cosine alone.
//!
//! Scope (per ADR-062 §Architecture): the forest's `CausalGraph` is the
//! kernel-global one — we do **not** spin up a per-session graph. Each
//! conversation's turns are scoped within that global graph by the
//! per-conversation [`ConvForest`] lineage map (chain_seq → node), and `Follows`
//! edges are only ever drawn within a conversation, so a forward/reverse walk
//! from one of a conversation's nodes stays inside that conversation.

use std::sync::Mutex;

use dashmap::DashMap;

use clawft_kernel::causal::NodeId as CausalNodeId;
use clawft_kernel::context_graft::{ChunkMeta, GraftContent, GraftedItem, SessionView};
use clawft_kernel::{
    CausalEdgeType, CausalGraph, CrossRef, CrossRefStore, CrossRefType, StructureTag,
    UniversalNodeId,
};

/// Default lineage hop radius when fusing causal neighbours into a recall.
/// One hop pulls in immediate `Follows` parents/children of each cosine hit —
/// enough to surface the turn a recalled turn directly answered.
pub(crate) const DEFAULT_LINEAGE_DEPTH: usize = 2;

/// Hard cap on how many extra (non-cosine) chunks lineage/cross-ref fusion may
/// add to one recall, so a long conversation's same-speaker fan-out can never
/// flood the grafted block.
pub(crate) const MAX_FUSED_EXTRA: usize = 16;

/// Per-conversation lineage state in the kernel-global causal graph.
///
/// Maps this conversation's chain sequences to their causal node ids (and the
/// turn's universal id for cross-ref resolution), and tracks the most-recently
/// indexed turn so each new turn can draw a `Follows` edge from it.
#[derive(Default)]
pub(crate) struct ConvForest {
    /// chain_seq → causal node id for this conversation's turns.
    seq_to_node: DashMap<u64, CausalNodeId>,
    /// chain_seq → the turn node's universal id (cross-ref source key).
    seq_to_uid: DashMap<u64, UniversalNodeId>,
    /// Most-recently indexed turn node — the `Follows` edge source. Guarded so
    /// concurrent cross-conversation indexing can't interleave (within a
    /// conversation, dispatch is already serialised by `AgentService`).
    prev: Mutex<Option<CausalNodeId>>,
    /// Universal id of the most-recently indexed turn (M4 turn-level edge
    /// rooting). Tracked alongside `prev` so the subagent spawner can root a
    /// `TriggeredBy`/`EvidenceFor` edge at the turn that issued the spawn — in
    /// the daemon path that is the assistant tool-call turn, anchored just
    /// before the tool dispatches (the user turn is one `Follows` hop upstream).
    last_uid: Mutex<Option<UniversalNodeId>>,
}

impl ConvForest {
    /// The causal node id a chain sequence was indexed as, if known.
    pub(crate) fn node_for(&self, chain_seq: u64) -> Option<CausalNodeId> {
        self.seq_to_node.get(&chain_seq).map(|e| *e.value())
    }

    /// The turn node's universal id for a chain sequence, if known.
    pub(crate) fn uid_for(&self, chain_seq: u64) -> Option<UniversalNodeId> {
        self.seq_to_uid.get(&chain_seq).map(|e| e.value().clone())
    }

    /// Universal id of the most-recently indexed turn in this conversation, if
    /// any (M4 turn-level edge rooting).
    pub(crate) fn latest_turn_uid(&self) -> Option<UniversalNodeId> {
        self.last_uid.lock().expect("conv forest last_uid lock").clone()
    }
}

/// Stable universal id for a turn node (the cross-ref source key).
pub(crate) fn turn_universal_id(conv_id: &str, chain_seq: u64, text: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        conv_id.as_bytes(),
        chain_seq,
        text.as_bytes(),
        b"turn",
    )
}

/// Stable universal id for a speaker identity within a conversation (the
/// per-speaker cross-ref target). Independent of chain sequence so every turn by
/// the same speaker resolves to the same identity node.
pub(crate) fn speaker_universal_id(conv_id: &str, role: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        conv_id.as_bytes(),
        0,
        role.as_bytes(),
        b"speaker",
    )
}

/// Dual-write a turn into the kernel-global forest (ADR-062 §1.1).
///
/// Adds a [`CausalGraph`] node for the turn (carrying `chain_seq` in its
/// metadata so a later walk can map nodes back to witness sequences), links
/// `prev → new` with [`CausalEdgeType::Follows`] (`weight = coherence`), and
/// registers the speaker (plus optional emotion/goal) as [`CrossRef`]s. Returns
/// the new node id.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dual_write_turn(
    causal: &CausalGraph,
    crossrefs: &CrossRefStore,
    forest: &ConvForest,
    conv_id: &str,
    chain_seq: u64,
    role: &str,
    text: &str,
    coherence: f32,
    emotion: Option<&str>,
    goal: Option<&str>,
) -> CausalNodeId {
    let label = format!("turn:{conv_id}:{chain_seq}");
    let metadata = serde_json::json!({
        "conv_id": conv_id,
        "chain_seq": chain_seq,
        "role": role,
        "state": "frontier",
    });
    let node = causal.add_node(label, metadata);
    forest.seq_to_node.insert(chain_seq, node);

    // Follows edge from the prior turn in this conversation. The edge weight is
    // the turn's coherence (clamped) so the lineage carries the soft-coherence
    // signal (ADR-062 D6) rather than a flat 1.0.
    {
        let mut prev = forest.prev.lock().expect("conv forest prev lock");
        if let Some(prev_node) = *prev {
            causal.link(
                prev_node,
                node,
                CausalEdgeType::Follows,
                coherence.clamp(0.0, 1.0),
                0,
                chain_seq,
            );
        }
        *prev = Some(node);
    }

    // Speaker cross-ref: turn → speaker identity node (per-speaker recall).
    let turn_uid = turn_universal_id(conv_id, chain_seq, text);
    forest.seq_to_uid.insert(chain_seq, turn_uid.clone());
    // Track the latest turn's uid so the subagent spawner can root spawn edges
    // at the actual spawning turn (M4 turn-level edge rooting).
    *forest.last_uid.lock().expect("conv forest last_uid lock") = Some(turn_uid.clone());
    let speaker_uid = speaker_universal_id(conv_id, role);
    crossrefs.insert(CrossRef {
        source: turn_uid.clone(),
        source_structure: StructureTag::CausalGraph,
        target: speaker_uid,
        target_structure: StructureTag::CausalGraph,
        ref_type: CrossRefType::Speaker,
        created_at: 0,
        chain_seq,
    });

    // Optional emotion / goal cross-refs. Phase 1 has no VAD/goal extractor in
    // the data-plane path, so these are wired but only fire when a caller
    // supplies a label — never fabricated.
    if let Some(emotion) = emotion {
        let emo_uid = UniversalNodeId::new(
            &StructureTag::CausalGraph,
            conv_id.as_bytes(),
            0,
            emotion.as_bytes(),
            b"emotion",
        );
        crossrefs.insert(CrossRef {
            source: turn_uid.clone(),
            source_structure: StructureTag::CausalGraph,
            target: emo_uid,
            target_structure: StructureTag::CausalGraph,
            ref_type: CrossRefType::EmotionCause,
            created_at: 0,
            chain_seq,
        });
    }
    if let Some(goal) = goal {
        let goal_uid = UniversalNodeId::new(
            &StructureTag::CausalGraph,
            conv_id.as_bytes(),
            0,
            goal.as_bytes(),
            b"goal",
        );
        crossrefs.insert(CrossRef {
            source: turn_uid,
            source_structure: StructureTag::CausalGraph,
            target: goal_uid,
            target_structure: StructureTag::CausalGraph,
            ref_type: CrossRefType::GoalMotivation,
            created_at: 0,
            chain_seq,
        });
    }

    node
}

/// Draw a cross-conversation [`CrossRef`] link (M4 A.4).
///
/// Spawn causality between two conversations — `C.goal --TriggeredBy--> T_user@P`
/// on spawn, `R_c@C --EvidenceFor--> T_user@P` on completion (design D3) — rides
/// the shared [`CrossRefStore`], **not** a [`CausalGraph`] edge. That is
/// deliberate: the causal-graph BFS ([`lineage_fuse`]) walks `Follows` edges and
/// would cross any node→node causal edge, so a cross-conv causal edge would leak
/// a parent's turns into a child's recall. A `CrossRef` carries the same
/// `TriggeredBy` / `EvidenceFor` semantics for the ADR-067 graph view while
/// keeping each conversation's lineage walk contained.
///
/// No enum changes: [`CrossRefType::TriggeredBy`] and
/// [`CrossRefType::EvidenceFor`] already exist.
pub(crate) fn link_cross_conv(
    crossrefs: &CrossRefStore,
    from_uid: UniversalNodeId,
    to_uid: UniversalNodeId,
    ref_type: CrossRefType,
    chain_seq: u64,
) {
    crossrefs.insert(CrossRef {
        source: from_uid,
        source_structure: StructureTag::CausalGraph,
        target: to_uid,
        target_structure: StructureTag::CausalGraph,
        ref_type,
        created_at: 0,
        chain_seq,
    });
}

/// Resolve a chunk's content into a COW reference (blob hash > inline > bare ref).
/// Mirrors the kernel's private `graft_content` so a lineage-sourced item carries
/// the same content reference shape as a cosine hit.
fn chunk_content(meta: &ChunkMeta) -> GraftContent {
    if let Some(ref h) = meta.blob_hash {
        GraftContent::Blob(h.clone())
    } else if let Some(ref t) = meta.inline {
        GraftContent::Inline(t.clone())
    } else {
        GraftContent::Reference
    }
}

/// The `chain_seq` a causal node was created for (read from its metadata).
fn node_chain_seq(causal: &CausalGraph, node: CausalNodeId) -> Option<u64> {
    causal.get_node(node)?.metadata.get("chain_seq")?.as_u64()
}

/// Fuse causal-lineage + cross-structure recall onto a cosine recall
/// (ADR-062 §1.2).
///
/// Starts from `cosine` (the HNSW result, highest relevance first) and expands
/// it by walking the forest from each hit:
/// - **lineage** — `traverse_reverse`/`traverse_forward` over `Follows` edges
///   pulls in the turns a recalled turn answered / led to;
/// - **cross-structure** — same-speaker [`CrossRef`]s pull in that speaker's
///   other turns.
///
/// Every surfaced chunk is fetched from `view` (so provenance — `chain_seq` +
/// `content_hash` — stays intact), deduplicated by content hash against the
/// cosine set and each other, and appended after the cosine hits. The expansion
/// is capped at [`MAX_FUSED_EXTRA`] so it can never flood the block.
pub(crate) fn lineage_fuse(
    view: &SessionView,
    causal: &CausalGraph,
    crossrefs: &CrossRefStore,
    forest: &ConvForest,
    cosine: Vec<GraftedItem>,
    depth: usize,
) -> Vec<GraftedItem> {
    let mut seen: std::collections::HashSet<String> =
        cosine.iter().map(|g| g.content_hash.clone()).collect();
    let cosine_seqs: std::collections::HashSet<u64> = cosine.iter().map(|g| g.chain_seq).collect();
    let mut out = cosine;

    // Gather candidate chain sequences reachable from the cosine hits, dedup'd
    // and excluding the cosine set itself.
    let mut candidates: Vec<u64> = Vec::new();
    let mut candidate_seen: std::collections::HashSet<u64> = cosine_seqs.clone();
    let mut push_candidate = |seq: u64, candidates: &mut Vec<u64>| {
        if candidate_seen.insert(seq) {
            candidates.push(seq);
        }
    };

    for seq in &cosine_seqs {
        // Lineage: forward + reverse Follows neighbours of this turn's node.
        if let Some(node) = forest.node_for(*seq) {
            for nid in causal
                .traverse_reverse(node, depth)
                .into_iter()
                .chain(causal.traverse_forward(node, depth))
            {
                if let Some(linked_seq) = node_chain_seq(causal, nid) {
                    push_candidate(linked_seq, &mut candidates);
                }
            }
        }
        // Cross-structure: same-speaker turns via the Speaker cross-ref.
        if let Some(uid) = forest.uid_for(*seq) {
            for speaker_cr in crossrefs.by_type(&uid, &CrossRefType::Speaker) {
                for sib in crossrefs.get_reverse(&speaker_cr.target) {
                    push_candidate(sib.chain_seq, &mut candidates);
                }
            }
        }
    }

    // Materialise candidates from the view, preserving provenance + dedup'ing by
    // content hash, capped so fusion stays a focused expansion.
    for seq in candidates {
        if out.len().saturating_sub(cosine_seqs.len()) >= MAX_FUSED_EXTRA {
            break;
        }
        let Some(meta) = view.chunk(seq) else {
            continue; // indexed in the graph but not (yet) in this view
        };
        if !seen.insert(meta.content_hash.clone()) {
            continue; // duplicate content already grafted
        }
        out.push(GraftedItem {
            chain_seq: meta.chain_seq,
            content_hash: meta.content_hash.clone(),
            kind: meta.kind.clone(),
            state: meta.state,
            // Lineage-sourced: not a cosine score. 0.0 marks "linked, not
            // directly similar"; provenance (chain_seq) is the load-bearing field.
            score: 0.0,
            content: chunk_content(&meta),
        });
    }

    out
}
