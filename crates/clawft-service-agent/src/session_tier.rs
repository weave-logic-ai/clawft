//! Per-conversation L2 context tier (ADR-058 Phase 5.1).
//!
//! The bridge between clawft-core's [`ContextGraftProvider`] seam and
//! clawft-kernel's session view / promotion. This crate depends on both layers,
//! so it is where the L2 tier is actually assembled:
//!
//! - **index** ([`index_turn`]) — when a turn is chain-appended, embed it and
//!   index it into the conversation's `SessionView` keyed by chain sequence
//!   (ADR-058 Phase 3.3).
//! - **graft** (the [`ContextGraftProvider`] impl) — per turn, the loop calls
//!   [`graft_block`]; we run a scoped semantic query and return the recalled
//!   chunks as a system message that cites each item's ExoChain sequence
//!   (witness chain) (Phase 3.2).
//! - **prune** ([`prune_to_recent`]) — bound the live window by evicting the
//!   oldest live chunks to `Stale`; they stay in the index, re-graftable
//!   (Phase 4.1).
//! - **promote** ([`promote_and_drop`]) — at conversation end, run the
//!   postmortem and emit a `memory.promote` chain event, then drop the view
//!   (Phase 4.2/4.3).
//!
//! [`index_turn`]: SessionTier::index_turn
//! [`graft_block`]: SessionTier::graft_block
//! [`prune_to_recent`]: SessionTier::prune_to_recent
//! [`promote_and_drop`]: SessionTier::promote_and_drop

use std::sync::{Arc, OnceLock, Weak};

use async_trait::async_trait;
use dashmap::DashMap;
use tracing::warn;

use clawft_core::agent::context::LlmMessage;
use clawft_core::agent::graft::ContextGraftProvider;
use clawft_kernel::artifact_store::ArtifactStore;
use clawft_kernel::chain::ChainManager;
use clawft_kernel::context_graft::{GraftContent, GraftedItem, SessionView};
use clawft_kernel::context_promote::{PromotionSignals, postmortem, promote_to_chain};
use clawft_kernel::embedding::EmbeddingProvider;
use clawft_kernel::{
    CausalGraph, CrossRefStore, ImpulseType, StructureTag, TalkModeLoop, UniversalNodeId,
    ViewResolver,
};

use crate::session_forest::{self, ConvForest, DEFAULT_LINEAGE_DEPTH};
use crate::turn_classifier::TurnClassifier;

/// Coherence weight stamped on a turn's `Follows` edge in the data-plane join
/// (ADR-062 §1.1). Phase 1 has no live per-turn coherence read on the index
/// path; the Talk-Mode tick (Phase 2) supplies the scored value. Until then a
/// neutral 1.0 keeps the lineage edge present and walkable.
const DEFAULT_TURN_COHERENCE: f32 = 1.0;

/// Default byte threshold above which chunk text is externalized to a
/// content-addressed blob instead of being held inline.
const DEFAULT_INLINE_MAX: usize = 4096;

/// Default number of chunks grafted per turn.
const DEFAULT_GRAFT_TOP_K: usize = 5;

/// Per-conversation L2 context tier shared between the conversation sink (which
/// indexes turns) and the agent loop (which grafts). One ephemeral
/// [`SessionView`] per conversation; the chain is the durable source of truth.
pub struct SessionTier {
    embedder: Arc<dyn EmbeddingProvider>,
    chain: Arc<ChainManager>,
    store: Option<Arc<ArtifactStore>>,
    views: DashMap<String, Arc<SessionView>>,
    graft_top_k: usize,
    inline_max: usize,
    /// ADR-062 §1.1 forest join — the kernel-global causal graph + cross-ref
    /// store. When present, each indexed turn is dual-written into the forest
    /// (causal node + `Follows` lineage + speaker cross-ref) and recall fuses
    /// lineage/cross-structure links with cosine (ADR-062 §1.2). `None` keeps
    /// the legacy cosine-only L2 behaviour.
    forest: Option<ForestHandles>,
    /// Per-conversation lineage state in the global causal graph.
    forests: DashMap<String, Arc<ConvForest>>,
    /// Turn classifier (ADR-067 P2, design §D2). When set, [`index_turn`] runs
    /// it synchronously before the forest dual-write so every committed turn
    /// node carries a non-null 4-axis `classification` blob (and its `text`).
    /// `None` — the daemon's default when `[kernel.agent.classification]
    /// mode = off` — keeps the legacy unclassified node metadata.
    ///
    /// [`index_turn`]: Self::index_turn
    classifier: Option<Arc<dyn TurnClassifier>>,
    /// The daemon-hosted multiplexed [`TalkModeLoop`] (M2 D2). When set,
    /// [`index_turn`](Self::index_turn) registers each dual-written turn with
    /// the loop and emits an `EndOfUtterance` so the loop commits it
    /// Frontier→Committed on the shared forest. Empty keeps the legacy
    /// index-only behaviour (no commit actor — turns stay `Frontier`).
    ///
    /// A write-once [`OnceLock`] rather than a plain field so the daemon can
    /// attach the loop through [`set_talk_loop`](Self::set_talk_loop) *after*
    /// the tier is already `Arc`-wrapped — the loop needs the finished
    /// `Arc<SessionTier>` as its [`ViewResolver`], so the tier can't hold the
    /// loop before it exists. Interior mutability breaks that construction
    /// cycle (see [`weak_view_resolver`](Self::weak_view_resolver)).
    talk_loop: OnceLock<Arc<TalkModeLoop>>,
}

/// The kernel-global forest handles the L2 tier dual-writes into.
struct ForestHandles {
    causal: Arc<CausalGraph>,
    crossrefs: Arc<CrossRefStore>,
}

impl SessionTier {
    /// Build a tier over the given embedder (ADR-059 Qwen3, or the Mock
    /// fallback when weights are absent), witness chain, and optional artifact
    /// store for large-payload externalization.
    pub fn new(
        embedder: Arc<dyn EmbeddingProvider>,
        chain: Arc<ChainManager>,
        store: Option<Arc<ArtifactStore>>,
    ) -> Self {
        Self {
            embedder,
            chain,
            store,
            views: DashMap::new(),
            graft_top_k: DEFAULT_GRAFT_TOP_K,
            inline_max: DEFAULT_INLINE_MAX,
            forest: None,
            forests: DashMap::new(),
            classifier: None,
            talk_loop: OnceLock::new(),
        }
    }

    /// Attach a turn classifier (ADR-067 P2). Once set, [`index_turn`] derives a
    /// 4-axis `classification` blob per turn and threads it (plus the turn text,
    /// design §6) into the forest dual-write. The daemon calls this at
    /// agent-service boot when `[kernel.agent.classification] mode != off`.
    ///
    /// [`index_turn`]: Self::index_turn
    pub fn with_classifier(mut self, classifier: Arc<dyn TurnClassifier>) -> Self {
        self.classifier = Some(classifier);
        self
    }

    /// Join this tier to the **kernel-global forest** (ADR-062 §1.1 / ADR-046):
    /// the global [`CausalGraph`] and [`CrossRefStore`] (the daemon's
    /// `k.ecc_causal()` / `k.ecc_crossrefs()`). Once joined, [`index_turn`]
    /// dual-writes each turn as a causal node + `Follows` lineage edge + speaker
    /// cross-ref, and [`graft_block`] fuses lineage/cross-structure recall with
    /// cosine. Without it the tier stays cosine-only (legacy L2).
    ///
    /// [`index_turn`]: Self::index_turn
    /// [`graft_block`]: ContextGraftProvider::graft_block
    pub fn with_forest(mut self, causal: Arc<CausalGraph>, crossrefs: Arc<CrossRefStore>) -> Self {
        self.forest = Some(ForestHandles { causal, crossrefs });
        self
    }

    /// Override the per-turn graft fan-out (default [`DEFAULT_GRAFT_TOP_K`]).
    pub fn with_graft_top_k(mut self, k: usize) -> Self {
        self.graft_top_k = k;
        self
    }

    /// Attach the daemon-hosted multiplexed [`TalkModeLoop`] (M2 D2) as an
    /// owned-value builder. Once attached, [`index_turn`](Self::index_turn)
    /// registers each dual-written turn node with the loop and emits an
    /// `EndOfUtterance` impulse, so the loop's next tick commits the turn
    /// Frontier→Committed on the same forest
    /// [`graft_block`](ContextGraftProvider::graft_block) reads from.
    ///
    /// The impulse queue handle is derived from the loop itself
    /// ([`TalkModeLoop::impulses`]) at emit time — one source of truth, no
    /// separately-threaded queue (plan §4 open-item 1).
    ///
    /// Use this when the tier is built and consumed by value (tests, voice's
    /// single-view assembly). The daemon builds a `TalkModeLoop` whose
    /// resolver *is* this tier — a construction cycle the by-value builder
    /// can't express — so the daemon uses [`set_talk_loop`](Self::set_talk_loop)
    /// after `Arc`-wrapping instead.
    pub fn with_talk_loop(self, talk_loop: Arc<TalkModeLoop>) -> Self {
        // First write wins; wiring attaches the loop exactly once.
        let _ = self.talk_loop.set(talk_loop);
        self
    }

    /// Attach the [`TalkModeLoop`] through interior mutability (M2 D2), after
    /// the tier is already `Arc`-wrapped. This is the daemon's wiring path: the
    /// loop's [`ViewResolver`] must be this very tier, so the tier can't hold
    /// the loop until the loop (and thus the tier `Arc`) exists. The daemon:
    ///
    /// ```ignore
    /// let tier = Arc::new(SessionTier::new(..).with_forest(..));
    /// let resolver = SessionTier::weak_view_resolver(&tier); // loop → tier (Weak)
    /// let talk_loop = Arc::new(TalkModeLoop::new(.., resolver, ..));
    /// tier.set_talk_loop(talk_loop.clone());                 // tier → loop (Arc)
    /// ```
    ///
    /// The loop holds a [`Weak`] back-reference (via `weak_view_resolver`), so
    /// there is **no strong reference cycle** — the tier owns the loop, the loop
    /// only borrows the tier and gracefully no-ops its commits if the tier is
    /// ever dropped. First write wins.
    pub fn set_talk_loop(&self, talk_loop: Arc<TalkModeLoop>) {
        let _ = self.talk_loop.set(talk_loop);
    }

    /// Build the [`ViewResolver`] the daemon hands to `TalkModeLoop::new` — a
    /// `Weak` handle to this tier so the loop resolves the tier's live
    /// per-conversation views without forming a strong `tier ↔ loop` cycle
    /// (see [`set_talk_loop`](Self::set_talk_loop)). Resolves to `None` (a
    /// logged no-op in the loop) once the tier is dropped.
    pub fn weak_view_resolver(tier: &Arc<Self>) -> Arc<dyn ViewResolver> {
        Arc::new(WeakTierResolver(Arc::downgrade(tier)))
    }

    /// Get or create the per-conversation forest lineage state.
    fn conv_forest(&self, conv_id: &str) -> Arc<ConvForest> {
        self.forests.entry(conv_id.to_string()).or_default().clone()
    }

    /// Universal id of a conversation's most-recently anchored turn (M4
    /// turn-level edge rooting). Returns `None` for a conversation with no
    /// indexed turns yet (or when the forest join is disabled). The subagent
    /// spawner uses this to root `TriggeredBy`/`EvidenceFor` edges at the turn
    /// that issued the spawn — in the daemon path the assistant tool-call turn,
    /// anchored just before the tool dispatches (the user turn is one `Follows`
    /// hop upstream) — instead of a synthetic conversation anchor.
    pub fn latest_turn_uid(&self, conv_id: &str) -> Option<UniversalNodeId> {
        self.forests.get(conv_id).and_then(|f| f.latest_turn_uid())
    }

    /// Pre-warm the embedder (ADR-058 Phase 5.3). Runs one throwaway embed so
    /// the model graph + runtime are hot before the first real graft — the
    /// daemon calls this at startup (mirrors the 6.2 STT pre-warm) so turn 1 of
    /// the first conversation doesn't pay the embedder's first-inference cost.
    /// Best-effort; safe to call on the Mock fallback (a cheap no-cost embed).
    pub async fn warm(&self) {
        self.embedder.warm().await;
    }

    /// Get or create the session view for `conv_id`, sized to the embedder.
    fn view(&self, conv_id: &str) -> Arc<SessionView> {
        self.views
            .entry(conv_id.to_string())
            .or_insert_with(|| Arc::new(SessionView::for_embedder(conv_id, &*self.embedder)))
            .clone()
    }

    /// Read the existing view without creating one (query/promote side).
    fn existing_view(&self, conv_id: &str) -> Option<Arc<SessionView>> {
        self.views.get(conv_id).map(|e| e.value().clone())
    }

    /// Index a chain-appended turn into the conversation's session view
    /// (ADR-058 Phase 3.3). `chain_seq` is the sequence returned when the turn
    /// was appended to the chain — the universal key + witness. Non-fatal:
    /// indexing failure is logged, never propagated (the turn already landed on
    /// the chain).
    pub async fn index_turn(
        &self,
        conv_id: &str,
        chain_seq: u64,
        kind: &str,
        role: &str,
        text: &str,
    ) {
        if text.trim().is_empty() {
            return;
        }
        let view = self.view(conv_id);
        if let Err(e) = view
            .index_chunk(
                &*self.embedder,
                self.store.as_deref(),
                chain_seq,
                kind,
                text,
                self.inline_max,
            )
            .await
        {
            warn!(conv_id, chain_seq, error = %e, "session_tier: failed to index turn");
        }

        // ADR-062 §1.1: dual-write the turn into the kernel-global forest —
        // a causal node + a `Follows` lineage edge to the prior turn + the
        // speaker cross-ref. Best-effort: the turn already landed on the chain
        // and in the cosine index regardless.
        if let Some(forest) = &self.forest {
            let conv_forest = self.conv_forest(conv_id);

            // ADR-067 P2: when a classifier is attached (mode != off), derive
            // the 4-axis `classification` blob synchronously here (design §D1 —
            // µs of CPU, safe on the witness path). `prev_topic` threads the
            // conversation's last topic for the continuity carry (design §D2);
            // the derived `emotion.label` / `goal` feed the dual-write's
            // EmotionCause / GoalMotivation cross-ref params. With no classifier
            // the blob is `None`, so no `classification`/`text` keys are written
            // (legacy behaviour, and text-at-rest stays off — design §6).
            let classification = self.classifier.as_ref().map(|c| {
                let prev_topic = conv_forest.last_topic();
                c.classify(role, text, prev_topic.as_deref())
            });
            let blob = classification.as_ref().map(|cv| cv.to_metadata_value());
            let emotion_label = classification.as_ref().map(|cv| cv.emotion.label.clone());
            let goal = classification.as_ref().and_then(|cv| cv.goal.clone());

            let node = session_forest::dual_write_turn(
                &forest.causal,
                &forest.crossrefs,
                &conv_forest,
                conv_id,
                chain_seq,
                role,
                text,
                DEFAULT_TURN_COHERENCE,
                blob.as_ref(),
                emotion_label.as_deref(),
                goal.as_deref(),
            );

            // Carry this turn's topic forward for the next turn's continuity
            // heuristic (design §D2). Only when a classifier ran.
            if let Some(cv) = &classification {
                conv_forest.set_last_topic(cv.topic.clone());
            }

            // M2 D2 — text ImpulseSource, at the anchor convergence seam.
            // `index_turn` is reached by BOTH `agent.chat` and
            // `agent.turn.record` (through `KernelTurnAnchor`) and NEVER by the
            // in-process CLI (`NoopTurnAnchor`), so emitting here fires an
            // impulse for exactly the daemon turns that mint a `chain_seq`.
            // Register the just-written turn node with the multiplexed loop,
            // then emit an `EndOfUtterance` so the loop's next tick commits it
            // Frontier→Committed on the shared forest. `register_turn` MUST
            // precede the emit: the loop routes the commit by the turn's
            // registered `conv_id`.
            if let Some(talk_loop) = self.talk_loop.get() {
                let uid = session_forest::turn_universal_id(conv_id, chain_seq, text);
                talk_loop.register_turn(chain_seq, node, uid, conv_id);

                // HLC stamp: `chain_seq` itself — the kernel-global monotone
                // sequence minted by `ChainManager::append` (one counter across
                // every conversation). Reusing it as the impulse HLC makes the
                // queue's drain-order match witness-chain append-order across
                // modalities without inventing a new clock (plan §4 item 3).
                let tag = StructureTag::CausalGraph.as_u8();
                talk_loop.impulses().emit(
                    tag,
                    [0u8; 32],
                    tag,
                    ImpulseType::EndOfUtterance,
                    serde_json::json!({ "chain_seq": chain_seq, "conv_id": conv_id }),
                    chain_seq,
                );
            }
        }
    }

    /// Mark a chunk "important to remember" (explicit promotion signal, e.g.
    /// from a `memory_store` tool call). Returns `false` if unknown.
    pub fn mark_important(&self, conv_id: &str, chain_seq: u64) -> bool {
        self.existing_view(conv_id)
            .map(|v| v.mark_important(chain_seq))
            .unwrap_or(false)
    }

    /// Prune the conversation's live window down to its `keep` most-recent live
    /// chunks (ADR-058 Phase 4.1). Evicted chunks become `Stale` but remain in
    /// the index, re-graftable via retrieval. Returns the number pruned.
    pub fn prune_to_recent(&self, conv_id: &str, keep: usize) -> usize {
        let Some(view) = self.existing_view(conv_id) else {
            return 0;
        };
        let live = view.live_seqs(); // ascending (oldest first)
        if live.len() <= keep {
            return 0;
        }
        let drop_n = live.len() - keep;
        let mut pruned = 0;
        for seq in live.into_iter().take(drop_n) {
            if view.prune(seq) {
                pruned += 1;
            }
        }
        pruned
    }

    /// Conversation ids with a live session view (ADR-058 Phase 5, deferred
    /// step 4). Lets the daemon enumerate active conversations — e.g. an
    /// idle-conversation reaper, or a shutdown sweep that promotes each before
    /// the views are dropped.
    pub fn active_conversations(&self) -> Vec<String> {
        self.views.iter().map(|e| e.key().clone()).collect()
    }

    /// Concatenate this conversation's retained (inline) chunk text, oldest
    /// first, capped at `max_bytes`, as the postmortem prompt source. Returns
    /// `None` for an unknown conversation and `Some("")` when no chunk text was
    /// retained inline (large chunks live as content-addressed blobs). The
    /// daemon feeds this to the LLM postmortem that produces the durable fact.
    pub fn conversation_digest(&self, conv_id: &str, max_bytes: usize) -> Option<String> {
        let view = self.existing_view(conv_id)?;
        let mut digest = String::new();
        for seq in view.chain_seqs() {
            let Some(meta) = view.chunk(seq) else {
                continue;
            };
            let Some(text) = meta.inline else { continue };
            if digest.len() + text.len() + 1 > max_bytes {
                break;
            }
            digest.push_str(&text);
            digest.push('\n');
        }
        Some(digest)
    }

    /// Drop a conversation's view without promoting (conversation ended with
    /// nothing durable to keep). Returns `true` if a view was removed. The chain
    /// remains the source of truth — the ephemeral L2 view is simply discarded.
    pub fn drop_view(&self, conv_id: &str) -> bool {
        self.views.remove(conv_id).is_some()
    }

    /// Run the session-end postmortem and promote durable facts to the trunk
    /// (ADR-058 Phase 4.2/4.3), then drop the view. Returns the
    /// `memory.promote` chain sequence if anything was promoted.
    pub fn promote_and_drop(&self, conv_id: &str, durable_fact: &str) -> Option<u64> {
        let view = self.existing_view(conv_id)?;
        let candidates = postmortem(&view, &PromotionSignals::default());
        let seq =
            promote_to_chain(&view, &candidates, durable_fact, &self.chain).map(|e| e.sequence);
        self.views.remove(conv_id);
        seq
    }

    /// Render grafted items into a single system message, citing each item's
    /// chain sequence so the model (and audit) can trace the witness chain.
    fn render_block(items: &[GraftedItem]) -> Vec<LlmMessage> {
        if items.is_empty() {
            return Vec::new();
        }
        let mut body = String::from(
            "# Recalled context (L2 graft)\n\n\
             Earlier context retrieved for this turn. Each item cites its \
             ExoChain sequence (witness):\n\n",
        );
        for it in items {
            let text = match &it.content {
                GraftContent::Inline(t) => t.clone(),
                GraftContent::Blob(h) => format!("[content-addressed blob {h}]"),
                GraftContent::Reference => "[recoverable from chain]".to_string(),
            };
            body.push_str(&format!("- [chain_seq {}] {}\n", it.chain_seq, text));
        }
        vec![LlmMessage {
            role: "system".into(),
            content: body,
            tool_call_id: None,
            tool_calls: None,
        }]
    }
}

#[async_trait]
impl ContextGraftProvider for SessionTier {
    async fn graft_block(&self, conv_id: &str, query: &str) -> Vec<LlmMessage> {
        let Some(view) = self.existing_view(conv_id) else {
            return Vec::new();
        };
        // Cosine recall (HNSW) — the historical L2 graft.
        let cosine = match view
            .graft_text(&*self.embedder, query, self.graft_top_k)
            .await
        {
            Ok(items) => items,
            Err(e) => {
                warn!(conv_id, error = %e, "session_tier: graft query failed; no graft");
                return Vec::new();
            }
        };
        // ADR-062 §1.2: when joined to the forest, fuse causal-lineage +
        // cross-structure recall onto the cosine hits so the walk follows
        // lineage, not cosine alone. Provenance (chain_seq) stays intact.
        let items = match (&self.forest, self.forests.get(conv_id)) {
            (Some(forest), Some(conv_forest)) => session_forest::lineage_fuse(
                &view,
                &forest.causal,
                &forest.crossrefs,
                &conv_forest,
                cosine,
                DEFAULT_LINEAGE_DEPTH,
            ),
            _ => cosine,
        };
        Self::render_block(&items)
    }
}

/// M2 D4 — SessionView frontier unification. The multiplexed
/// [`TalkModeLoop`] resolves each conversation's [`SessionView`] through this
/// seam, so it commits the *same* view [`graft_block`] reads from — one view,
/// not a second lifecycle copy. Delegates to the tier's existing per-conversation
/// view map; `None` when the view has been reaped (the loop treats that as a
/// logged no-op).
///
/// [`graft_block`]: ContextGraftProvider::graft_block
impl ViewResolver for SessionTier {
    fn view_for(&self, conv_id: &str) -> Option<Arc<SessionView>> {
        self.existing_view(conv_id)
    }
}

/// A [`Weak`] adapter that lets the multiplexed [`TalkModeLoop`] resolve a
/// [`SessionTier`]'s views without a strong `tier ↔ loop` reference cycle
/// (built by [`SessionTier::weak_view_resolver`]). The tier owns the loop; the
/// loop only borrows the tier. When the tier has been dropped the upgrade
/// fails and view resolution returns `None` — which the loop treats as a
/// logged no-op, exactly the reaped-conversation contract of D4.
struct WeakTierResolver(Weak<SessionTier>);

impl ViewResolver for WeakTierResolver {
    fn view_for(&self, conv_id: &str) -> Option<Arc<SessionView>> {
        self.0.upgrade()?.view_for(conv_id)
    }
}

#[cfg(test)]
#[path = "session_tier_tests.rs"]
mod tests;
