//! Skill rerank seam for HybridRouter v2.5 (WEFT-46).
//!
//! After the primary [`ContextRouter`](super::ContextRouter) returns a
//! non-empty skill list (typically EmbeddingRouter's top-K), HybridRouter
//! optionally reorders that list through a [`SkillReranker`].
//!
//! The trait itself has **no** heavyweight deps so tests and default
//! builds can exercise the plumbing with a stub. The sona-backed
//! production adapter lives behind the `hybrid-rerank` feature
//! (`sona_rerank` module) so a slow upstream cannot block builds.

use std::sync::Arc;

/// One skill candidate from the primary router's top-K.
///
/// `score` is a prior from the primary (cosine similarity, reciprocal
/// rank, etc.). Higher is better. The reranker may rewrite it.
#[derive(Debug, Clone, PartialEq)]
pub struct RerankCandidate {
    /// Skill id / registry name.
    pub id: String,
    /// Prior score from the primary (higher = better).
    pub score: f32,
}

impl RerankCandidate {
    /// Construct a candidate.
    pub fn new(id: impl Into<String>, score: f32) -> Self {
        Self {
            id: id.into(),
            score,
        }
    }
}

/// Reorders a primary top-K skill list.
///
/// Contract:
/// - Input order is the primary's ranking (best first).
/// - Output **must** contain the same set of ids (no drop, no invent).
/// - Prefer stable ordering on equal scores (primary order wins ties).
/// - Fail-open: on internal error return the input unchanged.
pub trait SkillReranker: Send + Sync + 'static {
    /// Rerank `candidates` for `query`. See trait docs for the contract.
    fn rerank(&self, query: &str, candidates: &[RerankCandidate]) -> Vec<RerankCandidate>;
}

/// No-op reranker: preserves primary order and scores.
///
/// Useful as an explicit "rerank-off" arm in benchmarks and as the
/// default when no adapter is attached.
#[derive(Debug, Default, Clone, Copy)]
pub struct IdentityReranker;

impl SkillReranker for IdentityReranker {
    fn rerank(&self, _query: &str, candidates: &[RerankCandidate]) -> Vec<RerankCandidate> {
        candidates.to_vec()
    }
}

/// Assign reciprocal-rank priors `1 / (rank + 1)` to an ordered skill list.
///
/// Used by [`HybridRouter`](super::HybridRouter) when the primary only
/// returns skill names (no cosine scores). Rank 0 (best) gets 1.0.
pub fn rank_priors(skills: &[String]) -> Vec<RerankCandidate> {
    skills
        .iter()
        .enumerate()
        .map(|(rank, id)| RerankCandidate::new(id.clone(), 1.0 / (rank as f32 + 1.0)))
        .collect()
}

/// Apply a reranker and return skill ids in the new order.
///
/// Fail-open guarantees:
/// - empty input → empty output
/// - reranker drops/adds ids → fall back to original order
/// - duplicate ids in output → fall back to original order
pub fn apply_rerank(
    reranker: &dyn SkillReranker,
    query: &str,
    skills: &[String],
) -> Vec<String> {
    if skills.len() <= 1 {
        return skills.to_vec();
    }

    let candidates = rank_priors(skills);
    let reranked = reranker.rerank(query, &candidates);

    if !same_id_set(skills, &reranked) {
        tracing::warn!(
            target: "context_router.hybrid_rerank",
            "SkillReranker returned a different id set; preserving primary order"
        );
        return skills.to_vec();
    }

    reranked.into_iter().map(|c| c.id).collect()
}

fn same_id_set(skills: &[String], reranked: &[RerankCandidate]) -> bool {
    if skills.len() != reranked.len() {
        return false;
    }
    let mut a: Vec<&str> = skills.iter().map(String::as_str).collect();
    let mut b: Vec<&str> = reranked.iter().map(|c| c.id.as_str()).collect();
    a.sort_unstable();
    b.sort_unstable();
    // Reject duplicates in either list (primary should be unique; if
    // the reranker invents dups the sort-equal check alone is not enough).
    if a.windows(2).any(|w| w[0] == w[1]) || b.windows(2).any(|w| w[0] == w[1]) {
        return false;
    }
    a == b
}

/// Object-safe helper so HybridRouter can store `Option<Arc<dyn SkillReranker>>`.
pub type SharedReranker = Arc<dyn SkillReranker>;

#[cfg(test)]
mod tests {
    use super::*;

    struct ReverseReranker;

    impl SkillReranker for ReverseReranker {
        fn rerank(&self, _query: &str, candidates: &[RerankCandidate]) -> Vec<RerankCandidate> {
            let mut out = candidates.to_vec();
            out.reverse();
            // Keep scores consistent with the new order so callers
            // inspecting scores see a valid ranking.
            for (i, c) in out.iter_mut().enumerate() {
                c.score = 1.0 / (i as f32 + 1.0);
            }
            out
        }
    }

    struct DroppingReranker;

    impl SkillReranker for DroppingReranker {
        fn rerank(&self, _query: &str, candidates: &[RerankCandidate]) -> Vec<RerankCandidate> {
            candidates.iter().skip(1).cloned().collect()
        }
    }

    #[test]
    fn identity_preserves_order() {
        let skills = vec!["a".into(), "b".into(), "c".into()];
        let out = apply_rerank(&IdentityReranker, "q", &skills);
        assert_eq!(out, skills);
    }

    #[test]
    fn reverse_reranker_reorders() {
        let skills = vec!["a".into(), "b".into(), "c".into()];
        let out = apply_rerank(&ReverseReranker, "q", &skills);
        assert_eq!(out, vec!["c".to_string(), "b".to_string(), "a".to_string()]);
    }

    #[test]
    fn bad_reranker_falls_open() {
        let skills = vec!["a".into(), "b".into(), "c".into()];
        let out = apply_rerank(&DroppingReranker, "q", &skills);
        assert_eq!(out, skills, "must preserve primary order on contract breach");
    }

    #[test]
    fn single_skill_skips_rerank() {
        let skills = vec!["only".into()];
        let out = apply_rerank(&ReverseReranker, "q", &skills);
        assert_eq!(out, skills);
    }

    #[test]
    fn rank_priors_are_descending() {
        let skills = vec!["a".into(), "b".into(), "c".into()];
        let p = rank_priors(&skills);
        assert!((p[0].score - 1.0).abs() < 1e-6);
        assert!((p[1].score - 0.5).abs() < 1e-6);
        assert!((p[2].score - 1.0 / 3.0).abs() < 1e-6);
    }
}
