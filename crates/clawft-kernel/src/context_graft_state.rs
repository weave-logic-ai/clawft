//! Causal-model node lifecycle ([`NodeState`]) and cross-substrate mirroring
//! for the session graft layer (ADR-058 / ADR-062 P0.2).
//!
//! Split out of `context_graft.rs` to keep that file under the 500-line cap.

use super::SessionView;

/// Node lifecycle state on the causal model (ADR-058 / ADR-056).
///
/// A grafted branch that coheres **collapses (commits)** to the trunk; one that
/// does not is held off `main_line` and pruned — the same branch/merge/prune
/// semantics as git ("causal collapse modeled on the git tree").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// A candidate branch not yet grafted into the working set.
    Speculative,
    /// Grafted and live in the working window this run.
    Frontier,
    /// Promoted (collapsed) onto the durable trunk (L3).
    Committed,
    /// Aged out of the live window; origin retained on the chain, re-graftable.
    Stale,
    /// Contradicted/revised; dropped off `main_line`.
    Pruned,
}

impl NodeState {
    /// Short stable tag for metadata/serialization.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeState::Speculative => "speculative",
            NodeState::Frontier => "frontier",
            NodeState::Committed => "committed",
            NodeState::Stale => "stale",
            NodeState::Pruned => "pruned",
        }
    }

    /// Whether `self → next` is a legal per-turn lifecycle transition
    /// (ADR-062 D2). The spine is `Speculative → Frontier → Committed`;
    /// soft/hard rebases drop to `Stale`/`Pruned`. `Pruned` is terminal.
    ///
    /// `Committed → Stale` models aging a committed node out of the live window
    /// (it stays re-graftable on the chain); `Stale → Frontier` is the re-graft.
    pub fn can_transition_to(self, next: NodeState) -> bool {
        use NodeState::*;
        matches!(
            (self, next),
            (Speculative, Frontier)
                | (Speculative, Stale)
                | (Speculative, Pruned)
                | (Frontier, Committed)
                | (Frontier, Stale)
                | (Frontier, Pruned)
                | (Committed, Stale)
                | (Stale, Frontier)
                | (Stale, Pruned)
        )
    }
}

/// Mirror a per-turn node-state transition across **both** substrates
/// (ADR-062 P0.2): the authoritative L2 [`SessionView`] chunk and the durable
/// [`CausalNode`](crate::causal::CausalNode) `metadata.state`.
///
/// The session view is the source of truth and gates the transition
/// ([`SessionView::transition`]); only if that legal step lands is the state
/// mirrored onto the causal node. Returns `true` only when **both** sides were
/// updated; if the causal node is unknown the view has still advanced (the
/// boolean reports the mirror was incomplete, so callers can re-sync).
pub fn mirror_state(
    view: &SessionView,
    chain_seq: u64,
    graph: &crate::causal::CausalGraph,
    causal_node_id: crate::causal::NodeId,
    next: NodeState,
) -> bool {
    if !view.transition(chain_seq, next) {
        return false;
    }
    graph.set_node_state(causal_node_id, next.as_str())
}
