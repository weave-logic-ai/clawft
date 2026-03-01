//! Constitutional governance engine for WeftOS.
//!
//! Implements the three-branch governance model where:
//! - **Legislative** (SOPs, rules, manifests) defines boundaries
//! - **Executive** (agents) acts within defined boundaries
//! - **Judicial** (CGR engine) validates every action
//!
//! No branch can modify another's constraints. Governance violations
//! are type-level impossibilities, not merely audited events.
//!
//! # Design
//!
//! All types compile unconditionally. The CGR validation engine and
//! effect algebra scoring require the `governance` or `ruvector-apps`
//! feature gates. Without them, `GovernanceEngine::evaluate()` returns
//! `GovernanceDecision::Permit` (open governance).

use serde::{Deserialize, Serialize};
use tracing::debug;

/// A governance rule that restricts agent behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRule {
    /// Unique rule identifier.
    pub id: String,

    /// Human-readable rule description.
    pub description: String,

    /// Which branch defined this rule.
    pub branch: GovernanceBranch,

    /// Rule severity (how critical the violation is).
    pub severity: RuleSeverity,

    /// Whether this rule is currently active.
    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

/// Governance branch that owns a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceBranch {
    /// Rules from SOPs, genesis protocol, weftapp.toml.
    Legislative,
    /// Rules from agent execution policies.
    Executive,
    /// Rules from CGR validation engine.
    Judicial,
}

impl std::fmt::Display for GovernanceBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GovernanceBranch::Legislative => write!(f, "legislative"),
            GovernanceBranch::Executive => write!(f, "executive"),
            GovernanceBranch::Judicial => write!(f, "judicial"),
        }
    }
}

/// Rule violation severity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuleSeverity {
    /// Advisory -- logged but not enforced.
    Advisory,
    /// Warning -- logged and flagged, action proceeds.
    Warning,
    /// Blocking -- action is prevented.
    Blocking,
    /// Critical -- action prevented and agent capability may be revoked.
    Critical,
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleSeverity::Advisory => write!(f, "advisory"),
            RuleSeverity::Warning => write!(f, "warning"),
            RuleSeverity::Blocking => write!(f, "blocking"),
            RuleSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// 5-dimensional effect vector for scoring agent actions.
///
/// Each dimension is scored from 0.0 (no impact) to 1.0 (maximum impact).
/// The magnitude of the vector determines whether an action exceeds
/// the environment's governance threshold.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectVector {
    /// Risk score: probability of negative outcome.
    #[serde(default)]
    pub risk: f64,

    /// Fairness score: impact on equitable treatment.
    #[serde(default)]
    pub fairness: f64,

    /// Privacy score: impact on data privacy.
    #[serde(default)]
    pub privacy: f64,

    /// Novelty score: how unprecedented the action is.
    #[serde(default)]
    pub novelty: f64,

    /// Security score: impact on system security.
    #[serde(default)]
    pub security: f64,
}

impl EffectVector {
    /// Compute the magnitude of the effect vector (L2 norm).
    pub fn magnitude(&self) -> f64 {
        (self.risk * self.risk
            + self.fairness * self.fairness
            + self.privacy * self.privacy
            + self.novelty * self.novelty
            + self.security * self.security)
            .sqrt()
    }

    /// Check if any dimension exceeds a threshold.
    pub fn any_exceeds(&self, threshold: f64) -> bool {
        self.risk > threshold
            || self.fairness > threshold
            || self.privacy > threshold
            || self.novelty > threshold
            || self.security > threshold
    }

    /// Get the maximum dimension value.
    pub fn max_dimension(&self) -> f64 {
        [self.risk, self.fairness, self.privacy, self.novelty, self.security]
            .into_iter()
            .fold(0.0_f64, f64::max)
    }
}

/// Governance decision for an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceDecision {
    /// Action is permitted.
    Permit,
    /// Action is permitted with advisory note.
    PermitWithWarning(String),
    /// Action requires human approval before proceeding.
    EscalateToHuman(String),
    /// Action is denied.
    Deny(String),
}

impl std::fmt::Display for GovernanceDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GovernanceDecision::Permit => write!(f, "permit"),
            GovernanceDecision::PermitWithWarning(msg) => write!(f, "permit (warning: {msg})"),
            GovernanceDecision::EscalateToHuman(msg) => write!(f, "escalate ({msg})"),
            GovernanceDecision::Deny(reason) => write!(f, "deny: {reason}"),
        }
    }
}

/// Governance evaluation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRequest {
    /// Agent identifier making the request.
    pub agent_id: String,

    /// Action being requested.
    pub action: String,

    /// Computed effect vector for the action.
    #[serde(default)]
    pub effect: EffectVector,

    /// Additional context for the evaluator.
    #[serde(default)]
    pub context: std::collections::HashMap<String, String>,
}

/// Governance evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceResult {
    /// The decision.
    pub decision: GovernanceDecision,

    /// Rules that were evaluated.
    pub evaluated_rules: Vec<String>,

    /// The effect vector that was scored.
    pub effect: EffectVector,

    /// Whether the effect magnitude exceeded the threshold.
    pub threshold_exceeded: bool,
}

/// Governance engine.
///
/// Evaluates actions against governance rules and the environment's
/// risk threshold. Without the `governance` feature gate, all
/// evaluations return `Permit`.
pub struct GovernanceEngine {
    rules: Vec<GovernanceRule>,
    risk_threshold: f64,
    human_approval_required: bool,
}

impl GovernanceEngine {
    /// Create a governance engine with the given risk threshold.
    pub fn new(risk_threshold: f64, human_approval_required: bool) -> Self {
        Self {
            rules: Vec::new(),
            risk_threshold,
            human_approval_required,
        }
    }

    /// Create an open governance engine that permits everything.
    pub fn open() -> Self {
        Self {
            rules: Vec::new(),
            risk_threshold: 1.0,
            human_approval_required: false,
        }
    }

    /// Add a governance rule.
    pub fn add_rule(&mut self, rule: GovernanceRule) {
        debug!(rule_id = %rule.id, branch = %rule.branch, "adding governance rule");
        self.rules.push(rule);
    }

    /// Get all active rules.
    pub fn active_rules(&self) -> Vec<&GovernanceRule> {
        self.rules.iter().filter(|r| r.active).collect()
    }

    /// Get rules by branch.
    pub fn rules_by_branch(&self, branch: &GovernanceBranch) -> Vec<&GovernanceRule> {
        self.rules
            .iter()
            .filter(|r| r.active && &r.branch == branch)
            .collect()
    }

    /// Evaluate a governance request.
    ///
    /// Decision logic:
    /// 1. If any blocking/critical rule applies, deny.
    /// 2. If effect magnitude exceeds threshold:
    ///    - If human_approval_required, escalate.
    ///    - Otherwise deny.
    /// 3. If any warning rule applies, permit with warning.
    /// 4. Otherwise permit.
    pub fn evaluate(&self, request: &GovernanceRequest) -> GovernanceResult {
        let magnitude = request.effect.magnitude();
        let threshold_exceeded = magnitude > self.risk_threshold;

        let mut evaluated_rules = Vec::new();
        let mut has_warning = false;
        let mut has_blocking = false;
        let mut blocking_reason = String::new();

        for rule in self.active_rules() {
            evaluated_rules.push(rule.id.clone());

            match rule.severity {
                RuleSeverity::Blocking | RuleSeverity::Critical => {
                    if threshold_exceeded {
                        has_blocking = true;
                        blocking_reason =
                            format!("rule '{}': effect magnitude {magnitude:.2} > threshold {:.2}", rule.id, self.risk_threshold);
                    }
                }
                RuleSeverity::Warning => {
                    if threshold_exceeded {
                        has_warning = true;
                    }
                }
                RuleSeverity::Advisory => {}
            }
        }

        let decision = if has_blocking {
            if self.human_approval_required {
                GovernanceDecision::EscalateToHuman(blocking_reason)
            } else {
                GovernanceDecision::Deny(blocking_reason)
            }
        } else if threshold_exceeded && has_warning {
            GovernanceDecision::PermitWithWarning(format!(
                "effect magnitude {magnitude:.2} approaches threshold {:.2}",
                self.risk_threshold
            ))
        } else {
            GovernanceDecision::Permit
        };

        GovernanceResult {
            decision,
            evaluated_rules,
            effect: request.effect.clone(),
            threshold_exceeded,
        }
    }

    /// Get the configured risk threshold.
    pub fn risk_threshold(&self) -> f64 {
        self.risk_threshold
    }

    /// Get total rule count.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(id: &str, severity: RuleSeverity, branch: GovernanceBranch) -> GovernanceRule {
        GovernanceRule {
            id: id.into(),
            description: format!("Test rule {id}"),
            branch,
            severity,
            active: true,
        }
    }

    #[test]
    fn effect_vector_magnitude() {
        let v = EffectVector {
            risk: 0.3,
            fairness: 0.4,
            privacy: 0.0,
            novelty: 0.0,
            security: 0.0,
        };
        assert!((v.magnitude() - 0.5).abs() < 0.001);
    }

    #[test]
    fn effect_vector_zero() {
        let v = EffectVector::default();
        assert!((v.magnitude() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn effect_any_exceeds() {
        let v = EffectVector {
            risk: 0.8,
            ..Default::default()
        };
        assert!(v.any_exceeds(0.5));
        assert!(!v.any_exceeds(0.9));
    }

    #[test]
    fn effect_max_dimension() {
        let v = EffectVector {
            risk: 0.2,
            fairness: 0.5,
            privacy: 0.3,
            novelty: 0.1,
            security: 0.4,
        };
        assert!((v.max_dimension() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn governance_branch_display() {
        assert_eq!(GovernanceBranch::Legislative.to_string(), "legislative");
        assert_eq!(GovernanceBranch::Executive.to_string(), "executive");
        assert_eq!(GovernanceBranch::Judicial.to_string(), "judicial");
    }

    #[test]
    fn rule_severity_ordering() {
        assert!(RuleSeverity::Advisory < RuleSeverity::Warning);
        assert!(RuleSeverity::Warning < RuleSeverity::Blocking);
        assert!(RuleSeverity::Blocking < RuleSeverity::Critical);
    }

    #[test]
    fn governance_decision_display() {
        assert_eq!(GovernanceDecision::Permit.to_string(), "permit");
        assert!(GovernanceDecision::Deny("too risky".into())
            .to_string()
            .contains("too risky"));
    }

    #[test]
    fn open_engine_permits_everything() {
        let engine = GovernanceEngine::open();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "deploy".into(),
            effect: EffectVector {
                risk: 0.9,
                security: 0.9,
                ..Default::default()
            },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert_eq!(result.decision, GovernanceDecision::Permit);
    }

    #[test]
    fn blocking_rule_denies() {
        let mut engine = GovernanceEngine::new(0.5, false);
        engine.add_rule(make_rule(
            "security-check",
            RuleSeverity::Blocking,
            GovernanceBranch::Judicial,
        ));

        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "deploy".into(),
            effect: EffectVector {
                risk: 0.6,
                ..Default::default()
            },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert!(matches!(result.decision, GovernanceDecision::Deny(_)));
        assert!(result.threshold_exceeded);
    }

    #[test]
    fn blocking_with_human_approval_escalates() {
        let mut engine = GovernanceEngine::new(0.5, true);
        engine.add_rule(make_rule(
            "security-check",
            RuleSeverity::Blocking,
            GovernanceBranch::Judicial,
        ));

        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "deploy".into(),
            effect: EffectVector {
                risk: 0.6,
                ..Default::default()
            },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert!(matches!(
            result.decision,
            GovernanceDecision::EscalateToHuman(_)
        ));
    }

    #[test]
    fn warning_rule_permits_with_warning() {
        let mut engine = GovernanceEngine::new(0.5, false);
        engine.add_rule(make_rule(
            "risk-check",
            RuleSeverity::Warning,
            GovernanceBranch::Executive,
        ));

        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "deploy".into(),
            effect: EffectVector {
                risk: 0.6,
                ..Default::default()
            },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert!(matches!(
            result.decision,
            GovernanceDecision::PermitWithWarning(_)
        ));
    }

    #[test]
    fn below_threshold_permits() {
        let mut engine = GovernanceEngine::new(0.5, false);
        engine.add_rule(make_rule(
            "security-check",
            RuleSeverity::Blocking,
            GovernanceBranch::Judicial,
        ));

        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "read".into(),
            effect: EffectVector {
                risk: 0.1,
                ..Default::default()
            },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert_eq!(result.decision, GovernanceDecision::Permit);
        assert!(!result.threshold_exceeded);
    }

    #[test]
    fn rules_by_branch() {
        let mut engine = GovernanceEngine::new(0.5, false);
        engine.add_rule(make_rule("r1", RuleSeverity::Warning, GovernanceBranch::Legislative));
        engine.add_rule(make_rule("r2", RuleSeverity::Blocking, GovernanceBranch::Judicial));
        engine.add_rule(make_rule("r3", RuleSeverity::Advisory, GovernanceBranch::Judicial));

        let judicial = engine.rules_by_branch(&GovernanceBranch::Judicial);
        assert_eq!(judicial.len(), 2);
        let legislative = engine.rules_by_branch(&GovernanceBranch::Legislative);
        assert_eq!(legislative.len(), 1);
    }

    #[test]
    fn inactive_rules_excluded() {
        let mut engine = GovernanceEngine::new(0.5, false);
        engine.add_rule(GovernanceRule {
            id: "disabled".into(),
            description: "Disabled rule".into(),
            branch: GovernanceBranch::Judicial,
            severity: RuleSeverity::Blocking,
            active: false,
        });
        assert_eq!(engine.active_rules().len(), 0);
    }

    #[test]
    fn governance_rule_serde_roundtrip() {
        let rule = make_rule("sec-1", RuleSeverity::Critical, GovernanceBranch::Judicial);
        let json = serde_json::to_string(&rule).unwrap();
        let restored: GovernanceRule = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "sec-1");
        assert!(restored.active);
    }

    #[test]
    fn governance_request_serde_roundtrip() {
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "deploy".into(),
            effect: EffectVector {
                risk: 0.5,
                privacy: 0.3,
                ..Default::default()
            },
            context: std::collections::HashMap::from([("env".into(), "prod".into())]),
        };
        let json = serde_json::to_string(&request).unwrap();
        let restored: GovernanceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agent_id, "agent-1");
        assert!((restored.effect.risk - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn effect_vector_serde_roundtrip() {
        let v = EffectVector {
            risk: 0.1,
            fairness: 0.2,
            privacy: 0.3,
            novelty: 0.4,
            security: 0.5,
        };
        let json = serde_json::to_string(&v).unwrap();
        let restored: EffectVector = serde_json::from_str(&json).unwrap();
        assert!((restored.security - 0.5).abs() < f64::EPSILON);
    }
}
