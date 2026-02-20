//! Task delegation engine.
//!
//! Routes tasks to the appropriate execution target (Local, Claude, Flow)
//! based on regex rule matching and complexity heuristics.
//!
//! Gated behind the `delegate` feature.

pub mod claude;
pub mod flow;
pub mod schema;

use clawft_types::delegation::{DelegationConfig, DelegationRule, DelegationTarget};
use regex::Regex;
use tracing::debug;

/// Engine that decides where to route a task based on configuration rules
/// and complexity heuristics.
pub struct DelegationEngine {
    config: DelegationConfig,
    compiled_rules: Vec<CompiledRule>,
}

/// A rule with its regex pre-compiled for efficient repeated matching.
struct CompiledRule {
    regex: Regex,
    target: DelegationTarget,
}

/// Complexity keywords that bump the complexity score.
const COMPLEXITY_KEYWORDS: &[&str] = &[
    "deploy",
    "refactor",
    "architect",
    "design",
    "optimize",
    "migrate",
    "security",
    "audit",
    "review",
    "analyze",
    "orchestrate",
    "coordinate",
    "integrate",
    "implement",
    "debug",
    "investigate",
    "comprehensive",
    "distributed",
    "concurrent",
    "parallel",
];

impl DelegationEngine {
    /// Create a new engine from the given configuration.
    ///
    /// Rules with invalid regex patterns are logged and skipped.
    pub fn new(config: DelegationConfig) -> Self {
        let compiled_rules = config
            .rules
            .iter()
            .filter_map(|rule: &DelegationRule| match Regex::new(&rule.pattern) {
                Ok(regex) => Some(CompiledRule {
                    regex,
                    target: rule.target,
                }),
                Err(e) => {
                    debug!(
                        pattern = %rule.pattern,
                        error = %e,
                        "skipping delegation rule with invalid regex"
                    );
                    None
                }
            })
            .collect();

        Self {
            config,
            compiled_rules,
        }
    }

    /// Decide which target should handle the given task.
    ///
    /// Evaluation order:
    /// 1. Walk compiled rules in order; first regex match wins.
    /// 2. If the matched target is `Claude` but `claude_available` is false,
    ///    fall back to `Local`.
    /// 3. If the matched target is `Flow` but `flow_available` is false,
    ///    fall back to `Claude` (if available) or `Local`.
    /// 4. If no rule matches, use `Auto` mode (complexity heuristic).
    pub fn decide(
        &self,
        task: &str,
        claude_available: bool,
        flow_available: bool,
    ) -> DelegationTarget {
        // Try explicit rules first.
        for rule in &self.compiled_rules {
            if rule.regex.is_match(task) {
                let target =
                    self.resolve_availability(rule.target, claude_available, flow_available);
                debug!(
                    task = %task,
                    matched_target = ?rule.target,
                    resolved_target = ?target,
                    "delegation rule matched"
                );
                return target;
            }
        }

        // No rule matched -- use Auto heuristic.
        self.auto_decide(task, claude_available, flow_available)
    }

    /// Estimate task complexity on a 0.0..1.0 scale.
    ///
    /// Uses simple heuristics:
    /// - Normalised text length (longer = more complex)
    /// - Question mark density (questions suggest research)
    /// - Presence of complexity keywords
    pub fn complexity_estimate(task: &str) -> f32 {
        if task.is_empty() {
            return 0.0;
        }

        // Length factor: saturate at 500 characters.
        let len_factor = (task.len() as f32 / 500.0).min(1.0);

        // Question mark density.
        let qmark_count = task.chars().filter(|&c| c == '?').count() as f32;
        let qmark_factor = (qmark_count / 3.0).min(1.0);

        // Keyword hits.
        let lower = task.to_lowercase();
        let keyword_hits = COMPLEXITY_KEYWORDS
            .iter()
            .filter(|kw| lower.contains(*kw))
            .count() as f32;
        let keyword_factor = (keyword_hits / 4.0).min(1.0);

        // Weighted average: length 30%, questions 20%, keywords 50%.
        let score = len_factor * 0.3 + qmark_factor * 0.2 + keyword_factor * 0.5;
        score.min(1.0)
    }

    /// Auto-decide based on complexity.
    ///
    /// - Low complexity (< 0.3): Local
    /// - Medium complexity (0.3..0.7): Claude (if available), else Local
    /// - High complexity (>= 0.7): Flow (if available), else Claude, else Local
    fn auto_decide(
        &self,
        task: &str,
        claude_available: bool,
        flow_available: bool,
    ) -> DelegationTarget {
        let complexity = Self::complexity_estimate(task);

        let target = if complexity < 0.3 {
            DelegationTarget::Local
        } else if complexity < 0.7 {
            if claude_available && self.config.claude_enabled {
                DelegationTarget::Claude
            } else {
                DelegationTarget::Local
            }
        } else if flow_available && self.config.claude_flow_enabled {
            DelegationTarget::Flow
        } else if claude_available && self.config.claude_enabled {
            DelegationTarget::Claude
        } else {
            DelegationTarget::Local
        };

        debug!(
            task = %task,
            complexity = complexity,
            target = ?target,
            "auto delegation decision"
        );

        target
    }

    /// Resolve a target given current availability.
    fn resolve_availability(
        &self,
        target: DelegationTarget,
        claude_available: bool,
        flow_available: bool,
    ) -> DelegationTarget {
        match target {
            DelegationTarget::Claude if !claude_available || !self.config.claude_enabled => {
                DelegationTarget::Local
            }
            DelegationTarget::Flow if !flow_available || !self.config.claude_flow_enabled => {
                if claude_available && self.config.claude_enabled {
                    DelegationTarget::Claude
                } else {
                    DelegationTarget::Local
                }
            }
            DelegationTarget::Auto => {
                // Should not normally appear in rules, but handle gracefully.
                DelegationTarget::Auto
            }
            other => other,
        }
    }

    /// Get a reference to the underlying config.
    pub fn config(&self) -> &DelegationConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::delegation::{DelegationConfig, DelegationRule, DelegationTarget};

    fn make_engine(rules: Vec<DelegationRule>) -> DelegationEngine {
        DelegationEngine::new(DelegationConfig {
            claude_enabled: true,
            claude_flow_enabled: true,
            rules,
            ..Default::default()
        })
    }

    #[test]
    fn rule_matching_dispatches_correctly() {
        let engine = make_engine(vec![
            DelegationRule {
                pattern: r"(?i)deploy".into(),
                target: DelegationTarget::Flow,
            },
            DelegationRule {
                pattern: r"(?i)^list\b".into(),
                target: DelegationTarget::Local,
            },
            DelegationRule {
                pattern: r"(?i)research|analyze".into(),
                target: DelegationTarget::Claude,
            },
        ]);

        assert_eq!(
            engine.decide("deploy to production", true, true),
            DelegationTarget::Flow
        );
        assert_eq!(
            engine.decide("list all files", true, true),
            DelegationTarget::Local
        );
        assert_eq!(
            engine.decide("analyze the codebase", true, true),
            DelegationTarget::Claude
        );
        assert_eq!(
            engine.decide("research best practices", true, true),
            DelegationTarget::Claude
        );
    }

    #[test]
    fn fallback_when_claude_unavailable() {
        let engine = make_engine(vec![DelegationRule {
            pattern: r"(?i)research".into(),
            target: DelegationTarget::Claude,
        }]);

        // Claude unavailable: should fall back to Local.
        assert_eq!(
            engine.decide("research AI patterns", false, true),
            DelegationTarget::Local
        );
    }

    #[test]
    fn fallback_when_flow_unavailable() {
        let engine = make_engine(vec![DelegationRule {
            pattern: r"(?i)deploy".into(),
            target: DelegationTarget::Flow,
        }]);

        // Flow unavailable, Claude available: fall back to Claude.
        assert_eq!(
            engine.decide("deploy to staging", true, false),
            DelegationTarget::Claude
        );

        // Both unavailable: fall back to Local.
        assert_eq!(
            engine.decide("deploy to staging", false, false),
            DelegationTarget::Local
        );
    }

    #[test]
    fn auto_mode_low_complexity_is_local() {
        let engine = make_engine(vec![]);
        // Short, simple task with no complexity keywords.
        assert_eq!(engine.decide("hi", true, true), DelegationTarget::Local);
    }

    #[test]
    fn auto_mode_high_complexity_routes_to_flow() {
        let engine = make_engine(vec![]);
        // Many keywords + long text + question marks to push score >= 0.7.
        let task = "Please architect and design a comprehensive distributed \
                    system with concurrent processing, then deploy, optimize, \
                    migrate, refactor the security audit and review the \
                    integration. Can you also investigate the debug logs and \
                    coordinate the orchestration???";
        let score = DelegationEngine::complexity_estimate(task);
        assert!(score >= 0.7, "expected >= 0.7, got {score}");
        assert_eq!(engine.decide(task, true, true), DelegationTarget::Flow);
    }

    #[test]
    fn auto_mode_medium_complexity_routes_to_claude() {
        let engine = make_engine(vec![]);
        // Enough keywords and length to land in 0.3..0.7 range.
        let task = "Please review this function, analyze the performance \
                    characteristics, and investigate potential optimizations \
                    for the codebase";
        let score = DelegationEngine::complexity_estimate(task);
        assert!(
            score >= 0.3 && score < 0.7,
            "expected 0.3..0.7, got {score}"
        );
        assert_eq!(engine.decide(task, true, true), DelegationTarget::Claude);
    }

    #[test]
    fn auto_mode_falls_back_to_local_when_services_disabled() {
        let engine = DelegationEngine::new(DelegationConfig {
            claude_enabled: false,
            claude_flow_enabled: false,
            ..Default::default()
        });
        let task = "architect and design a comprehensive distributed system \
                    with security audit and deploy orchestration??";
        assert_eq!(engine.decide(task, true, true), DelegationTarget::Local);
    }

    #[test]
    fn complexity_estimate_empty_is_zero() {
        assert_eq!(DelegationEngine::complexity_estimate(""), 0.0);
    }

    #[test]
    fn complexity_estimate_scales_with_keywords() {
        let low = DelegationEngine::complexity_estimate("hello world");
        let high = DelegationEngine::complexity_estimate(
            "architect and design a comprehensive distributed system \
             with security audit",
        );
        assert!(low < high, "low={low}, high={high}");
    }

    #[test]
    fn complexity_estimate_capped_at_one() {
        let very_complex = "deploy refactor architect design optimize migrate \
                           security audit review analyze orchestrate coordinate \
                           integrate implement debug investigate comprehensive \
                           distributed concurrent parallel????";
        let score = DelegationEngine::complexity_estimate(very_complex);
        assert!(score <= 1.0, "score={score}");
        assert!(score > 0.8, "should be high complexity, got {score}");
    }

    #[test]
    fn invalid_regex_skipped() {
        let engine = make_engine(vec![
            DelegationRule {
                pattern: r"[invalid".into(), // broken regex
                target: DelegationTarget::Claude,
            },
            DelegationRule {
                pattern: r"(?i)hello".into(),
                target: DelegationTarget::Local,
            },
        ]);
        // The broken rule is skipped; "hello" still matches.
        assert_eq!(
            engine.decide("hello world", true, true),
            DelegationTarget::Local
        );
    }

    #[test]
    fn first_rule_wins() {
        let engine = make_engine(vec![
            DelegationRule {
                pattern: r"(?i)deploy".into(),
                target: DelegationTarget::Flow,
            },
            DelegationRule {
                pattern: r"(?i)deploy".into(),
                target: DelegationTarget::Local,
            },
        ]);
        assert_eq!(
            engine.decide("deploy now", true, true),
            DelegationTarget::Flow
        );
    }
}
