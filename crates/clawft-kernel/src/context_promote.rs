//! Postmortem promotion + `memory.promote` chain event (ADR-058 Phase 4.2/4.3).
//!
//! Because the session view ([`SessionView`]) is discarded per run, a
//! session-end rollup **must promote durable facts to the trunk** before the
//! view is dropped — else "fresh each run" becomes "amnesia each run". This is
//! the load-bearing L2→L3 boundary:
//!
//! - **4.2 [`postmortem`]** — a mandatory post-task review that nominates chunks
//!   for promotion using multiple signals (explicit "important" marks, recall /
//!   reference count, task-completion score). Not a single heuristic.
//! - **4.3 [`MemoryPromote`] / `promote_to_chain`** — promotion is itself a
//!   `memory.promote` [`ChainLoggable`](crate::chain::ChainLoggable) event
//!   (ADR-020/022) recording which chain sequences were distilled, giving
//!   auditable lineage of long-term memory. (Requires the `exochain` feature.)

use crate::context_graft::{NodeState, SessionView};

/// Why a chunk was nominated for promotion (auditable, multi-signal).
#[derive(Debug, Clone, PartialEq)]
pub enum PromotionReason {
    /// Explicitly marked "important to remember" (e.g. a `memory_store` call).
    Important,
    /// Recalled/grafted this many times during the session.
    ReferenceCount(u32),
    /// The task completed with this score (a boosting signal).
    TaskScore(f32),
}

/// A chunk nominated for L2→L3 promotion by the postmortem.
#[derive(Debug, Clone)]
pub struct PromotionCandidate {
    /// Source ExoChain sequence (lineage key).
    pub chain_seq: u64,
    /// Content-addressed identity (provenance).
    pub content_hash: String,
    /// The signals that fired for this candidate.
    pub reasons: Vec<PromotionReason>,
}

/// Multi-signal thresholds for the postmortem nominator.
#[derive(Debug, Clone)]
pub struct PromotionSignals {
    /// Task-completion score in `[0,1]`.
    pub task_score: f32,
    /// At/above this, the task counts as successful: a single recall then
    /// suffices to nominate, and a `TaskScore` reason is attached.
    pub task_threshold: f32,
    /// Minimum recall count to nominate on reference-count alone.
    pub min_refs: u32,
}

impl Default for PromotionSignals {
    fn default() -> Self {
        Self {
            task_score: 0.0,
            task_threshold: 0.7,
            min_refs: 2,
        }
    }
}

/// Postmortem promotion stage (ADR-058 Phase 4.2).
///
/// Examines the finished session and nominates chunks for durable promotion.
/// A chunk is nominated if it was marked important **or** recalled at least
/// `min_refs` times (relaxed to a single recall on a successful task).
/// `Pruned`/contradicted chunks are never promoted. The reasons list records
/// exactly which signals fired, for auditability.
pub fn postmortem(view: &SessionView, signals: &PromotionSignals) -> Vec<PromotionCandidate> {
    let task_ok = signals.task_score >= signals.task_threshold;
    let effective_min_refs = if task_ok { 1 } else { signals.min_refs };

    let mut out = Vec::new();
    for seq in view.chain_seqs() {
        let Some(meta) = view.chunk(seq) else {
            continue;
        };
        if meta.state == NodeState::Pruned {
            continue;
        }

        let mut reasons = Vec::new();
        if view.is_important(seq) {
            reasons.push(PromotionReason::Important);
        }
        let rc = view.ref_count(seq);
        if rc > 0 && rc >= effective_min_refs {
            reasons.push(PromotionReason::ReferenceCount(rc));
        }
        if reasons.is_empty() {
            continue; // no firing signal — not durable-worthy
        }
        if task_ok {
            reasons.push(PromotionReason::TaskScore(signals.task_score));
        }
        out.push(PromotionCandidate {
            chain_seq: seq,
            content_hash: meta.content_hash,
            reasons,
        });
    }
    out
}

// ── 4.3 promotion as a chain event (requires the chain) ───────────────

/// A `memory.promote` chain event (ADR-058 Phase 4.3 / ADR-020 ChainLoggable).
///
/// Records which source chain sequences were distilled into a durable fact, so
/// long-term memory carries auditable lineage back to its witnesses.
#[cfg(feature = "exochain")]
pub struct MemoryPromote {
    /// Source ExoChain sequences distilled into the durable fact.
    pub source_chain_seqs: Vec<u64>,
    /// Their content-addressed identities (provenance).
    pub content_hashes: Vec<String>,
    /// The distilled durable fact promoted onto the trunk (L3).
    pub durable_fact: String,
}

#[cfg(feature = "exochain")]
impl crate::chain::ChainLoggable for MemoryPromote {
    fn chain_event_source(&self) -> &str {
        "memory"
    }

    fn chain_event_kind(&self) -> &str {
        "memory.promote"
    }

    fn chain_event_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "source_chain_seqs": self.source_chain_seqs,
            "content_hashes": self.content_hashes,
            "durable_fact": self.durable_fact,
        })
    }
}

/// Promote the nominated candidates to the durable trunk (ADR-058 Phase 4.3).
///
/// Emits a single `memory.promote` chain event recording the source chain
/// sequences (lineage) and marks the promoted chunks [`NodeState::Committed`].
/// The session view should be dropped **only after** this returns. No-op
/// (`None`) when there are no candidates.
#[cfg(feature = "exochain")]
pub fn promote_to_chain(
    view: &SessionView,
    candidates: &[PromotionCandidate],
    durable_fact: impl Into<String>,
    chain: &crate::chain::ChainManager,
) -> Option<crate::chain::ChainEvent> {
    if candidates.is_empty() {
        return None;
    }
    let source_chain_seqs: Vec<u64> = candidates.iter().map(|c| c.chain_seq).collect();
    let content_hashes: Vec<String> = candidates.iter().map(|c| c.content_hash.clone()).collect();
    let event = MemoryPromote {
        source_chain_seqs: source_chain_seqs.clone(),
        content_hashes,
        durable_fact: durable_fact.into(),
    };
    let logged = chain.append_loggable(&event);
    for seq in source_chain_seqs {
        view.set_state(seq, NodeState::Committed);
    }
    Some(logged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_graft::{ChunkMeta, SessionView};

    fn axis_vec(dims: usize, axis: usize) -> Vec<f32> {
        let mut v = vec![0.01_f32; dims];
        if axis < dims {
            v[axis] = 1.0;
        }
        v
    }

    fn insert(view: &SessionView, seq: u64, axis: usize, text: &str) {
        let meta = ChunkMeta {
            chain_seq: seq,
            content_hash: crate::context_graft::content_hash(text),
            state: NodeState::Frontier,
            kind: "agent.chat.turn".into(),
            blob_hash: None,
            inline: Some(text.into()),
        };
        assert!(view.insert_vector(axis_vec(8, axis), meta));
    }

    #[test]
    fn postmortem_nominates_important_and_recalled() {
        let view = SessionView::new("s1", 8);
        insert(&view, 1, 0, "important fact");
        insert(&view, 2, 1, "recalled fact");
        insert(&view, 3, 2, "ignored fact");

        // Signal A: explicit important mark on seq 1.
        view.mark_important(1);
        // Signal B: recall seq 2 twice (ref_count = 2 = default min_refs).
        let _ = view.graft(&axis_vec(8, 1), 1);
        let _ = view.graft(&axis_vec(8, 1), 1);
        assert_eq!(view.ref_count(2), 2);

        let cands = postmortem(&view, &PromotionSignals::default());
        let seqs: Vec<u64> = cands.iter().map(|c| c.chain_seq).collect();
        assert!(seqs.contains(&1), "important chunk nominated");
        assert!(seqs.contains(&2), "recalled chunk nominated");
        assert!(!seqs.contains(&3), "untouched chunk not nominated");

        let c1 = cands.iter().find(|c| c.chain_seq == 1).unwrap();
        assert!(c1.reasons.contains(&PromotionReason::Important));
        let c2 = cands.iter().find(|c| c.chain_seq == 2).unwrap();
        assert!(c2.reasons.contains(&PromotionReason::ReferenceCount(2)));
    }

    #[test]
    fn successful_task_lowers_recall_threshold() {
        let view = SessionView::new("s1", 8);
        insert(&view, 5, 0, "recalled once");
        let _ = view.graft(&axis_vec(8, 0), 1); // ref_count = 1 (< default 2)

        // Without task success: not nominated (1 < min_refs 2).
        assert!(postmortem(&view, &PromotionSignals::default()).is_empty());

        // With task success: single recall suffices, TaskScore reason attached.
        let signals = PromotionSignals {
            task_score: 0.9,
            task_threshold: 0.7,
            min_refs: 2,
        };
        let cands = postmortem(&view, &signals);
        assert_eq!(cands.len(), 1);
        assert!(
            cands[0]
                .reasons
                .iter()
                .any(|r| matches!(r, PromotionReason::TaskScore(_)))
        );
    }

    #[test]
    fn pruned_chunks_are_not_promoted() {
        let view = SessionView::new("s1", 8);
        insert(&view, 1, 0, "fact");
        view.mark_important(1);
        view.prune(1); // Stale is still promotable; Pruned is not.
        view.set_state(1, NodeState::Pruned);
        assert!(postmortem(&view, &PromotionSignals::default()).is_empty());
    }

    /// 4.3: promotion emits a `memory.promote` chain event with lineage and
    /// commits the chunks. Requires the `exochain` feature.
    #[cfg(feature = "exochain")]
    #[test]
    fn promote_emits_memory_promote_event_with_lineage() {
        use crate::chain::ChainManager;

        let view = SessionView::new("s1", 8);
        insert(&view, 11, 0, "durable A");
        insert(&view, 12, 1, "durable B");
        view.mark_important(11);
        view.mark_important(12);

        let cands = postmortem(&view, &PromotionSignals::default());
        assert_eq!(cands.len(), 2);

        let chain = ChainManager::new(0, 1000);
        let before = chain.head_sequence();
        let event = promote_to_chain(&view, &cands, "A and B are durable", &chain)
            .expect("promotion event emitted");

        assert_eq!(event.kind, "memory.promote");
        assert!(event.sequence > before);
        let payload = event.payload.expect("payload present");
        let seqs = payload["source_chain_seqs"].as_array().unwrap();
        let seqs: Vec<u64> = seqs.iter().map(|v| v.as_u64().unwrap()).collect();
        assert!(seqs.contains(&11) && seqs.contains(&12));
        assert_eq!(payload["durable_fact"], "A and B are durable");

        // Promoted chunks are now Committed (and dropped from the live window).
        assert_eq!(view.chunk(11).unwrap().state, NodeState::Committed);
        assert_eq!(view.chunk(12).unwrap().state, NodeState::Committed);
    }

    #[cfg(feature = "exochain")]
    #[test]
    fn promote_empty_candidates_is_noop() {
        use crate::chain::ChainManager;
        let view = SessionView::new("s1", 8);
        let chain = ChainManager::new(0, 1000);
        assert!(promote_to_chain(&view, &[], "nothing", &chain).is_none());
    }
}
