//! Mid-stream gating analysis hooks for the 50 ms [`CognitiveTick`] path (WEFT-617).
//!
//! Pure **analysis → optional impulse emit**. Never owns control flow: floor
//! commit / barge-in cancel stay with [`TalkModeLoop`] / Talk-Mode. See
//! [`docs/research/weft-617-midstream-eval.md`](../../../../docs/research/weft-617-midstream-eval.md)
//! and [`.planning/ruv/integration/midstream-integration-plan.md`](../../../../.planning/ruv/integration/midstream-integration-plan.md).
//!
//! ## Phase map
//!
//! | Phase | What this module provides now | Future |
//! |-------|-------------------------------|--------|
//! | A (IU partials) | [`PrefixMidstreamAnalyzer`] common-prefix scaffold | Vendored DTW/edit-distance when it beats prefix |
//! | B (stall) | [`MidstreamImpulseBridge`] + [`StallSignal`] contract | Bounded-window pattern / `find_similar` |
//! | C/D | window caps only | VAP prior / Weaver offline miners |
//!
//! [`CognitiveTick`]: clawft_kernel::CognitiveTick
//! [`TalkModeLoop`]: clawft_kernel::TalkModeLoop

use std::sync::Arc;

use clawft_kernel::{ImpulseQueue, ImpulseType, StructureTag};

// ---------------------------------------------------------------------------
// Tick budget constraints (must stay inside CognitiveTick headroom)
// ---------------------------------------------------------------------------

/// Hard cap on token / frame windows fed to tick-hot analysis.
///
/// MidStream DTW/LCS is O(n·m); at n,m ≤ 64 cost is single-digit microseconds
/// and fits the default 50 ms tick (~15 ms soft compute budget). Larger windows
/// and O(n³) pattern miners are **forbidden** on this path (Weaver offline only).
pub const TICK_WINDOW_CAP: usize = 64;

/// Maximum haystack length for sliding-window similarity (Phase B/C).
pub const TICK_HAYSTACK_CAP: usize = 128;

// ---------------------------------------------------------------------------
// Partial / IU restart-and-revise (Phase A)
// ---------------------------------------------------------------------------

/// Outcome of comparing a new STT partial against the last *stable* prefix.
///
/// Maps to speculative node lifecycle (synthesis §C.7): extend the in-flight
/// node, revise (revoke + rewrite), or mark ready to commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialDiffVerdict {
    /// Partial matches the stable prefix exactly (no graph write).
    Unchanged,
    /// Partial is a strict extension of the stable prefix (append / update).
    Extend,
    /// Partial diverges before the end of the stable prefix (restart-and-revise).
    Revise,
    /// Partial is empty or caller marks it final — ready for Frontier→Committed.
    Commit,
}

/// Stall / loop signal from a bounded recent-token window (Phase B).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StallSignal {
    /// Short reason for logs / impulse payload (`"repeat"`, `"empty_loop"`, …).
    pub reason: &'static str,
    /// How many consecutive identical tokens (or empty ticks) triggered it.
    pub run_length: usize,
}

/// Mid-stream analysis seam used by STT partial paths and tick-side analyzers.
///
/// Implementations must be pure and allocation-light on the hot path. They
/// **must not** call into TTS, the Talk-Mode controller, or mutate the causal
/// graph — only return verdicts / signals for the caller (or
/// [`MidstreamImpulseBridge`]) to act on.
pub trait MidstreamAnalyzer: Send + Sync {
    /// Diff a new streaming partial against the last stable prefix.
    fn diff_partial(&self, stable_prefix: &str, new_partial: &str) -> PartialDiffVerdict;

    /// Look for stall / repetition over a **capped** recent-token window.
    ///
    /// Callers must pass at most [`TICK_WINDOW_CAP`] tokens; implementations may
    /// ignore the rest or return `None` if the window is empty.
    fn detect_stall(&self, recent_tokens: &[String]) -> Option<StallSignal>;
}

/// Phase-A scaffold: common-prefix compare (no DTW yet).
///
/// Kill criterion (WEFT-617): if vendored DTW does not measurably reduce partial
/// churn / improve commit timing vs this helper, **keep this** and drop DTW.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrefixMidstreamAnalyzer {
    /// Consecutive identical tokens before a stall signal (default 4).
    pub stall_run: usize,
}

impl PrefixMidstreamAnalyzer {
    /// Default stall run length of 4 identical tokens.
    pub fn new() -> Self {
        Self { stall_run: 4 }
    }

    /// Custom stall sensitivity.
    pub fn with_stall_run(stall_run: usize) -> Self {
        Self {
            stall_run: stall_run.max(2),
        }
    }
}

impl MidstreamAnalyzer for PrefixMidstreamAnalyzer {
    fn diff_partial(&self, stable_prefix: &str, new_partial: &str) -> PartialDiffVerdict {
        let stable = stable_prefix.trim();
        let partial = new_partial.trim();

        if partial.is_empty() {
            return if stable.is_empty() {
                PartialDiffVerdict::Unchanged
            } else {
                // Empty partial after content often means endpoint / revoke path.
                PartialDiffVerdict::Commit
            };
        }
        if stable.is_empty() {
            return PartialDiffVerdict::Extend;
        }
        if partial == stable {
            return PartialDiffVerdict::Unchanged;
        }
        if let Some(rest) = partial.strip_prefix(stable) {
            // Extension only when new material appears (ignore pure whitespace grow).
            return if rest.trim().is_empty() {
                PartialDiffVerdict::Unchanged
            } else {
                PartialDiffVerdict::Extend
            };
        }
        // Diverged: common prefix shorter than stable → restart-and-revise.
        PartialDiffVerdict::Revise
    }

    fn detect_stall(&self, recent_tokens: &[String]) -> Option<StallSignal> {
        if recent_tokens.is_empty() {
            return None;
        }
        let window = if recent_tokens.len() > TICK_WINDOW_CAP {
            &recent_tokens[recent_tokens.len() - TICK_WINDOW_CAP..]
        } else {
            recent_tokens
        };
        let last = window.last()?.as_str();
        if last.is_empty() {
            return None;
        }
        let mut run = 0usize;
        for t in window.iter().rev() {
            if t.as_str() == last {
                run += 1;
            } else {
                break;
            }
        }
        if run >= self.stall_run {
            Some(StallSignal {
                reason: "repeat",
                run_length: run,
            })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Impulse bridge (Phase B hook) — analysis result → ImpulseQueue only
// ---------------------------------------------------------------------------

/// Emits [`ImpulseType::CoherenceAlert`] when a [`MidstreamAnalyzer`] reports a
/// stall. Does not interpret floor policy; the loop reads the impulse on the
/// next CognitiveTick drain.
pub struct MidstreamImpulseBridge<A: MidstreamAnalyzer> {
    analyzer: A,
    impulses: Arc<ImpulseQueue>,
    source: [u8; 32],
}

impl<A: MidstreamAnalyzer> MidstreamImpulseBridge<A> {
    /// Bind an analyzer to a forest / session impulse queue.
    pub fn new(analyzer: A, impulses: Arc<ImpulseQueue>, source_node: [u8; 32]) -> Self {
        Self {
            analyzer,
            impulses,
            source: source_node,
        }
    }

    /// Shared analyzer (for partial-diff without emitting).
    pub fn analyzer(&self) -> &A {
        &self.analyzer
    }

    /// Diff helpers pass-through (no impulse).
    pub fn diff_partial(&self, stable_prefix: &str, new_partial: &str) -> PartialDiffVerdict {
        self.analyzer.diff_partial(stable_prefix, new_partial)
    }

    /// Run stall detection; on hit, emit `CoherenceAlert` and return the signal.
    ///
    /// `hlc` is the hybrid-logical timestamp for queue ordering (caller supplies
    /// monotone clock, same pattern as [`crate::KernelImpulseSink`]).
    pub fn check_stall_and_emit(
        &self,
        recent_tokens: &[String],
        hlc: u64,
    ) -> Option<StallSignal> {
        let signal = self.analyzer.detect_stall(recent_tokens)?;
        let tag = StructureTag::CausalGraph.as_u8();
        let payload = serde_json::json!({
            "source": "midstream",
            "reason": signal.reason,
            "run_length": signal.run_length,
            "weft": "WEFT-617",
        });
        self.impulses.emit(
            tag,
            self.source,
            tag,
            ImpulseType::CoherenceAlert,
            payload,
            hlc,
        );
        Some(signal)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_diff_extend_revise_unchanged_commit() {
        let a = PrefixMidstreamAnalyzer::new();
        assert_eq!(
            a.diff_partial("", "hello"),
            PartialDiffVerdict::Extend
        );
        assert_eq!(
            a.diff_partial("hello", "hello"),
            PartialDiffVerdict::Unchanged
        );
        assert_eq!(
            a.diff_partial("hello", "hello world"),
            PartialDiffVerdict::Extend
        );
        assert_eq!(
            a.diff_partial("hello world", "hello there"),
            PartialDiffVerdict::Revise
        );
        assert_eq!(
            a.diff_partial("hello", ""),
            PartialDiffVerdict::Commit
        );
        assert_eq!(
            a.diff_partial("", ""),
            PartialDiffVerdict::Unchanged
        );
    }

    #[test]
    fn stall_detects_repeated_tokens_within_cap() {
        let a = PrefixMidstreamAnalyzer::with_stall_run(3);
        let tokens: Vec<String> = ["a", "b", "x", "x", "x"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let s = a.detect_stall(&tokens).expect("stall");
        assert_eq!(s.reason, "repeat");
        assert_eq!(s.run_length, 3);
        assert!(a.detect_stall(&["a".into(), "b".into()]).is_none());
    }

    #[test]
    fn stall_respects_window_cap_tail() {
        let a = PrefixMidstreamAnalyzer::with_stall_run(3);
        let mut tokens: Vec<String> = (0..TICK_WINDOW_CAP + 10)
            .map(|i| format!("t{i}"))
            .collect();
        // Only the tail matters; three identical at end.
        let n = tokens.len();
        tokens[n - 3] = "loop".into();
        tokens[n - 2] = "loop".into();
        tokens[n - 1] = "loop".into();
        assert!(a.detect_stall(&tokens).is_some());
    }

    #[test]
    fn bridge_emits_coherence_alert_on_stall() {
        let q = Arc::new(ImpulseQueue::new());
        let bridge = MidstreamImpulseBridge::new(
            PrefixMidstreamAnalyzer::with_stall_run(3),
            q.clone(),
            [7u8; 32],
        );
        let tokens: Vec<String> = ["x", "x", "x"].into_iter().map(str::to_string).collect();
        let hit = bridge.check_stall_and_emit(&tokens, 42);
        assert!(hit.is_some());
        let drained = q.drain_ready();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].impulse_type, ImpulseType::CoherenceAlert);
        assert_eq!(drained[0].hlc_timestamp, 42);
        assert_eq!(drained[0].payload["source"], "midstream");
        assert_eq!(drained[0].payload["reason"], "repeat");
    }

    #[test]
    fn bridge_silent_when_no_stall() {
        let q = Arc::new(ImpulseQueue::new());
        let bridge =
            MidstreamImpulseBridge::new(PrefixMidstreamAnalyzer::new(), q.clone(), [0u8; 32]);
        assert!(bridge
            .check_stall_and_emit(&["one".into(), "two".into()], 1)
            .is_none());
        assert!(q.drain_ready().is_empty());
    }

    #[test]
    fn window_caps_are_tick_safe_constants() {
        // Documented contract: small-n DP fits 50 ms tick; keep caps tight.
        assert!(TICK_WINDOW_CAP <= 64);
        assert!(TICK_HAYSTACK_CAP <= 256);
        assert!(TICK_WINDOW_CAP * TICK_WINDOW_CAP < 10_000);
    }
}
