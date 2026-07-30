//! v2.5 [`ContextRouter`] composition: primary + optional sona rerank + fallback.
//!
//! Phase E3 of `docs/plans/agent-core-v1.md`, completed by **WEFT-46**.
//!
//! Pipeline:
//!
//! 1. Run the primary router (typically v2
//!    [`EmbeddingRouter`](super::EmbeddingRouter)).
//! 2. If the decision is structurally empty → fall through to the
//!    cheaper fallback (typically v1
//!    [`LlmClassifierRouter`](super::LlmClassifierRouter)) and mark
//!    `fallback_used`.
//! 3. Otherwise, if a [`SkillReranker`](super::rerank::SkillReranker) is
//!    attached, reorder the primary's skill top-K (sona-backed when the
//!    `hybrid-rerank` feature is enabled and a
//!    [`SonaSkillReranker`](super::sona_rerank::SonaSkillReranker) is
//!    supplied). Archetype follows the new top-1 when the primary had
//!    set one from its previous top-1.
//!
//! ## Hard contract (do not break)
//!
//! Per `docs/research/rvf-context-router.md`: the chain NEVER picks a
//! model and NEVER escalates a tier. Every member of the chain emits a
//! [`ContextDecision`] in the same shape; [`HybridRouter`] just selects
//! between them (and optionally reorders skills). The B1
//! [`clamp_complexity`](super::clamp_complexity) invariant on
//! `complexity_hint ∈ [-0.3, +0.3]` is preserved at every boundary —
//! `HybridRouter` never reads or rewrites the hint, so the producing
//! router's clamp survives intact.
//!
//! ## Why "empty decision" instead of a numeric threshold
//!
//! Without comparable confidences across heterogeneous primaries (an
//! EmbeddingRouter cosine and an LlmClassifierRouter archetype label
//! aren't on the same axis), structural emptiness is the cheap-correct
//! seam: the v2 router already collapses to
//! [`ContextDecision::default()`] when top-1 cosine falls below
//! `confidence_threshold`.
//!
//! ## Feature gate
//!
//! The rerank *plumbing* (optional [`SkillReranker`]) is always compiled.
//! The **sona** adapter crate is feature-gated (`hybrid-rerank`) so a
//! slow upstream never blocks default builds.

use std::sync::Arc;

use async_trait::async_trait;

use super::rerank::{SharedReranker, SkillReranker, apply_rerank};
use super::{ContextDecision, ContextRequest, ContextRouter};

// TODO(agent-core-v1 phase E3+): wire MicroLoraRouter (v3) once
// ruvllm-wasm lifts the 11-pattern HNSW cap
// (docs/research/rvf-context-router.md:118-128). The 35+-skill clawft
// catalog overruns ruvllm-wasm v2.0.1's documented per-index ceiling,
// which is why v3 is held until upstream lands a larger cap.

/// Chains two [`ContextRouter`]s with an optional skill-list rerank.
///
/// Primary runs first; empty decision → fallback. Non-empty decision →
/// optional [`SkillReranker`] reorders `skills` (WEFT-46).
pub struct HybridRouter {
    primary: Arc<dyn ContextRouter>,
    fallback: Arc<dyn ContextRouter>,
    /// Optional v2.5 skill rerank (sona-backed when feature + adapter).
    reranker: Option<SharedReranker>,
}

impl HybridRouter {
    /// Compose a primary + fallback chain without a reranker.
    ///
    /// Both arms are read-only on the route hot path; the wrapper is
    /// `Arc`-friendly to match the `Arc<dyn ContextRouter>` shape the
    /// daemon hands to
    /// [`build_daemon_agent_loop`](crate::bootstrap::build_daemon_agent_loop).
    pub fn new(primary: Arc<dyn ContextRouter>, fallback: Arc<dyn ContextRouter>) -> Self {
        Self {
            primary,
            fallback,
            reranker: None,
        }
    }

    /// Attach a skill reranker (e.g. sona-backed) after the primary.
    ///
    /// No-op identity behaviour when the reranker preserves order
    /// (day-0 SONA, or [`IdentityReranker`](super::rerank::IdentityReranker)).
    #[must_use]
    pub fn with_reranker(mut self, reranker: SharedReranker) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Convenience: attach any [`SkillReranker`] by value.
    #[must_use]
    pub fn with_skill_reranker(self, reranker: impl SkillReranker) -> Self {
        self.with_reranker(Arc::new(reranker))
    }

    /// Whether a reranker is currently attached.
    pub fn has_reranker(&self) -> bool {
        self.reranker.is_some()
    }
}

#[async_trait]
impl ContextRouter for HybridRouter {
    async fn route(&self, request: &ContextRequest) -> ContextDecision {
        let primary_decision = self.primary.route(request).await;
        if is_empty_decision(&primary_decision) {
            tracing::info!(
                target: "context_router.hybrid_fallback",
                "HybridRouter: primary returned empty; falling through to fallback"
            );
            let mut fallback_decision = self.fallback.route(request).await;
            // WEFT-335: mark the decision so the router-decision log
            // can seed the v2→v2.5 fallback-rate promotion gate.
            fallback_decision.fallback_used = true;
            fallback_decision
        } else {
            self.maybe_rerank(request, primary_decision)
        }
    }
}

impl HybridRouter {
    /// WEFT-46: reorder primary skills when a reranker is attached.
    fn maybe_rerank(&self, request: &ContextRequest, mut decision: ContextDecision) -> ContextDecision {
        let Some(reranker) = self.reranker.as_ref() else {
            return decision;
        };
        if decision.skills.len() <= 1 {
            return decision;
        }

        let prev_top = decision.skills.first().cloned();
        let reordered = apply_rerank(reranker.as_ref(), &request.content, &decision.skills);
        if reordered == decision.skills {
            return decision;
        }

        // If the primary stamped archetype from its top-1 skill and
        // rerank changed top-1, clear archetype rather than leave a
        // stale label. Downstream still has the reordered skills.
        if decision.archetype.is_some()
            && prev_top.as_ref() != reordered.first()
        {
            tracing::debug!(
                target: "context_router.hybrid_rerank",
                prev_top = prev_top.as_deref().unwrap_or(""),
                new_top = reordered.first().map(String::as_str).unwrap_or(""),
                "HybridRouter: top-1 changed under rerank; clearing archetype"
            );
            decision.archetype = None;
        }

        tracing::debug!(
            target: "context_router.hybrid_rerank",
            skills = ?reordered,
            "HybridRouter: applied skill rerank"
        );
        decision.skills = reordered;
        decision
    }
}

/// Structural emptiness check — does this decision carry any signal at
/// all? Mirrors the four user-visible fields on [`ContextDecision`]:
/// `skills`, `archetype`, `tool_subset`, `complexity_hint`. If none of
/// them carry a value, the decision is indistinguishable from
/// [`ContextDecision::default()`] and the chain falls through.
fn is_empty_decision(d: &ContextDecision) -> bool {
    d.skills.is_empty()
        && d.archetype.is_none()
        && d.tool_subset.is_none()
        && d.complexity_hint == 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::agent::context_router::rerank::{IdentityReranker, RerankCandidate};

    /// Counting stub: records how many times `route` was called and
    /// returns the same canned [`ContextDecision`] each time. This is
    /// the same pattern B1 / E1 used to keep the router test surface
    /// narrow without spinning up real backends.
    struct StubRouter {
        decision: ContextDecision,
        calls: Mutex<usize>,
    }

    impl StubRouter {
        fn new(decision: ContextDecision) -> Self {
            Self {
                decision,
                calls: Mutex::new(0),
            }
        }

        fn calls(&self) -> usize {
            *self.calls.lock().unwrap()
        }
    }

    #[async_trait]
    impl ContextRouter for StubRouter {
        async fn route(&self, _request: &ContextRequest) -> ContextDecision {
            *self.calls.lock().unwrap() += 1;
            self.decision.clone()
        }
    }

    /// Test-only reverse reranker (stable contract).
    struct ReverseReranker;

    impl SkillReranker for ReverseReranker {
        fn rerank(&self, _query: &str, candidates: &[RerankCandidate]) -> Vec<RerankCandidate> {
            let mut out = candidates.to_vec();
            out.reverse();
            for (i, c) in out.iter_mut().enumerate() {
                c.score = 1.0 / (i as f32 + 1.0);
            }
            out
        }
    }

    fn req() -> ContextRequest {
        ContextRequest {
            content: "hello".into(),
            channel: "panel".into(),
            chat_id: "c1".into(),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn hybrid_returns_primary_when_non_empty() {
        // Primary has a real signal (skills + non-zero hint) — the
        // chain returns it verbatim and never touches the fallback.
        let primary_decision = ContextDecision::new(vec!["x".into()], None, 0.1);
        let primary = Arc::new(StubRouter::new(primary_decision));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));

        let router = HybridRouter::new(
            primary.clone() as Arc<dyn ContextRouter>,
            fallback.clone() as Arc<dyn ContextRouter>,
        );
        let decision = router.route(&req()).await;

        assert_eq!(decision.skills, vec!["x".to_string()]);
        assert!((decision.complexity_hint - 0.1).abs() < 1e-5);
        assert_eq!(primary.calls(), 1);
        assert_eq!(
            fallback.calls(),
            0,
            "fallback must not run when primary is non-empty"
        );
    }

    #[tokio::test]
    async fn hybrid_falls_back_on_empty_primary() {
        // Primary returns the empty decision; fallback wins. The
        // fallback's archetype-bearing decision must surface unchanged.
        let primary = Arc::new(StubRouter::new(ContextDecision::default()));
        let mut fb = ContextDecision::new(Vec::new(), None, 0.2);
        fb.archetype = Some("X".into());
        let fallback = Arc::new(StubRouter::new(fb));

        let router = HybridRouter::new(
            primary.clone() as Arc<dyn ContextRouter>,
            fallback.clone() as Arc<dyn ContextRouter>,
        );
        let decision = router.route(&req()).await;

        assert_eq!(decision.archetype.as_deref(), Some("X"));
        assert!((decision.complexity_hint - 0.2).abs() < 1e-5);
        assert!(decision.skills.is_empty());
        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 1);
    }

    #[tokio::test]
    async fn hybrid_returns_default_when_both_empty() {
        // Both arms are empty — the chain returns the fallback's
        // (empty) decision. Both arms are called exactly once.
        let primary = Arc::new(StubRouter::new(ContextDecision::default()));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));

        let router = HybridRouter::new(
            primary.clone() as Arc<dyn ContextRouter>,
            fallback.clone() as Arc<dyn ContextRouter>,
        );
        let decision = router.route(&req()).await;

        assert!(decision.skills.is_empty());
        assert!(decision.archetype.is_none());
        assert!(decision.tool_subset.is_none());
        assert_eq!(decision.complexity_hint, 0.0);
        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 1);
    }

    #[tokio::test]
    async fn hybrid_does_not_call_fallback_when_primary_has_skills() {
        // Skills-only primary decision counts as non-empty.
        let primary_decision = ContextDecision::new(vec!["alpha".into(), "beta".into()], None, 0.0);
        let primary = Arc::new(StubRouter::new(primary_decision));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));

        let router = HybridRouter::new(
            primary.clone() as Arc<dyn ContextRouter>,
            fallback.clone() as Arc<dyn ContextRouter>,
        );
        let decision = router.route(&req()).await;

        assert_eq!(
            decision.skills,
            vec!["alpha".to_string(), "beta".to_string()]
        );
        assert_eq!(decision.complexity_hint, 0.0);
        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 0);
    }

    #[tokio::test]
    async fn hybrid_does_not_call_fallback_when_primary_has_archetype() {
        // Archetype-only primary decision counts as non-empty even
        // though `skills` is empty and `complexity_hint == 0.0`.
        let mut d = ContextDecision::new(Vec::new(), None, 0.0);
        d.archetype = Some("Reasoning".into());
        let primary = Arc::new(StubRouter::new(d));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));

        let router = HybridRouter::new(
            primary.clone() as Arc<dyn ContextRouter>,
            fallback.clone() as Arc<dyn ContextRouter>,
        );
        let decision = router.route(&req()).await;

        assert_eq!(decision.archetype.as_deref(), Some("Reasoning"));
        assert!(decision.skills.is_empty());
        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 0);
    }

    #[tokio::test]
    async fn hybrid_does_not_call_fallback_when_primary_has_complexity() {
        // Non-zero complexity_hint counts as non-empty even with no
        // skills + no archetype + no tool_subset. This guards the v1
        // classifier's "complexity-only" outputs (it can return just
        // `{archetype: ..., complexity: -0.2}` without skills).
        let primary_decision = ContextDecision::new(Vec::new(), None, -0.2);
        let primary = Arc::new(StubRouter::new(primary_decision));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));

        let router = HybridRouter::new(
            primary.clone() as Arc<dyn ContextRouter>,
            fallback.clone() as Arc<dyn ContextRouter>,
        );
        let decision = router.route(&req()).await;

        assert!((decision.complexity_hint + 0.2).abs() < 1e-5);
        assert!(decision.skills.is_empty());
        assert!(decision.archetype.is_none());
        assert_eq!(primary.calls(), 1);
        assert_eq!(fallback.calls(), 0);
    }

    #[test]
    fn is_empty_decision_default_is_empty() {
        // Sanity-check the structural predicate against
        // ContextDecision::default(): the chain *must* fall through on
        // it, otherwise the v2.5 plumbing degenerates into "always
        // returns the primary".
        let d = ContextDecision::default();
        assert!(is_empty_decision(&d));
    }

    #[test]
    fn is_empty_decision_tool_subset_some_empty_is_non_empty() {
        // `Some(vec![])` is "no tools at all" — a real, intentional
        // signal (per the trait docs), not the absence of one. Treat
        // it as non-empty so the chain doesn't override an explicit
        // pure-chat skill restriction.
        let d = ContextDecision {
            skills: Vec::new(),
            tool_subset: Some(Vec::new()),
            complexity_hint: 0.0,
            archetype: None,
            fallback_used: false,
        };
        assert!(!is_empty_decision(&d));
    }

    #[tokio::test]
    async fn hybrid_marks_fallback_used_on_fall_through() {
        let primary = Arc::new(StubRouter {
            decision: ContextDecision::default(),
            calls: Mutex::new(0),
        });
        let fallback = Arc::new(StubRouter {
            decision: ContextDecision::new(vec!["rescued".into()], None, 0.0),
            calls: Mutex::new(0),
        });
        let hybrid = HybridRouter::new(primary, fallback);
        let req = ContextRequest {
            content: "x".into(),
            channel: "panel".into(),
            chat_id: "c".into(),
            metadata: Default::default(),
        };
        let d = hybrid.route(&req).await;
        assert!(d.fallback_used, "fall-through must set fallback_used");
        assert_eq!(d.skills, vec!["rescued".to_string()]);
    }

    // ── WEFT-46: rerank step ──────────────────────────────────────────

    #[tokio::test]
    async fn hybrid_reranks_primary_skills_when_attached() {
        let primary = Arc::new(StubRouter::new(ContextDecision::new(
            vec!["a".into(), "b".into(), "c".into()],
            None,
            0.1,
        )));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));
        let router = HybridRouter::new(primary, fallback).with_skill_reranker(ReverseReranker);

        let d = router.route(&req()).await;
        assert_eq!(
            d.skills,
            vec!["c".to_string(), "b".to_string(), "a".to_string()]
        );
        assert!((d.complexity_hint - 0.1).abs() < 1e-5);
        assert!(!d.fallback_used);
    }

    #[tokio::test]
    async fn hybrid_identity_reranker_preserves_order() {
        let primary = Arc::new(StubRouter::new(ContextDecision::new(
            vec!["a".into(), "b".into()],
            None,
            0.0,
        )));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));
        let router = HybridRouter::new(primary, fallback).with_skill_reranker(IdentityReranker);

        let d = router.route(&req()).await;
        assert_eq!(d.skills, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn hybrid_does_not_rerank_on_fallback_path() {
        // Empty primary → fallback; reranker must not touch fallback skills.
        let primary = Arc::new(StubRouter::new(ContextDecision::default()));
        let fallback = Arc::new(StubRouter::new(ContextDecision::new(
            vec!["fb-a".into(), "fb-b".into()],
            None,
            0.0,
        )));
        let router = HybridRouter::new(primary, fallback).with_skill_reranker(ReverseReranker);

        let d = router.route(&req()).await;
        assert!(d.fallback_used);
        assert_eq!(
            d.skills,
            vec!["fb-a".to_string(), "fb-b".to_string()],
            "fallback path must not apply primary rerank"
        );
    }

    #[tokio::test]
    async fn hybrid_clears_stale_archetype_when_top1_changes() {
        let mut primary_d = ContextDecision::new(vec!["a".into(), "b".into()], None, 0.0);
        primary_d.archetype = Some("FromA".into());
        let primary = Arc::new(StubRouter::new(primary_d));
        let fallback = Arc::new(StubRouter::new(ContextDecision::default()));
        let router = HybridRouter::new(primary, fallback).with_skill_reranker(ReverseReranker);

        let d = router.route(&req()).await;
        assert_eq!(d.skills, vec!["b".to_string(), "a".to_string()]);
        assert!(
            d.archetype.is_none(),
            "archetype from previous top-1 must clear when top-1 flips"
        );
    }

    #[test]
    fn hybrid_reports_has_reranker() {
        let primary = Arc::new(StubRouter::new(ContextDecision::default())) as Arc<dyn ContextRouter>;
        let fallback = Arc::new(StubRouter::new(ContextDecision::default())) as Arc<dyn ContextRouter>;
        assert!(!HybridRouter::new(primary.clone(), fallback.clone()).has_reranker());
        assert!(
            HybridRouter::new(primary, fallback)
                .with_skill_reranker(IdentityReranker)
                .has_reranker()
        );
    }
}
