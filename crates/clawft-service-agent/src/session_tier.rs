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

use std::sync::Arc;

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
        }
    }

    /// Override the per-turn graft fan-out (default [`DEFAULT_GRAFT_TOP_K`]).
    pub fn with_graft_top_k(mut self, k: usize) -> Self {
        self.graft_top_k = k;
        self
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
    pub async fn index_turn(&self, conv_id: &str, chain_seq: u64, kind: &str, text: &str) {
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
        match view
            .graft_text(&*self.embedder, query, self.graft_top_k)
            .await
        {
            Ok(items) => Self::render_block(&items),
            Err(e) => {
                warn!(conv_id, error = %e, "session_tier: graft query failed; no graft");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
#[path = "session_tier_tests.rs"]
mod tests;
