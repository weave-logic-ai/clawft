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

    /// SOP reference URL for agents to consult for full procedure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_url: Option<String>,

    /// SOP category tag for filtering rules by domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sop_category: Option<String>,
}

impl GovernanceRule {
    /// Get rules by SOP category from a slice of rules.
    pub fn filter_by_category<'a>(rules: &'a [GovernanceRule], category: &str) -> Vec<&'a GovernanceRule> {
        rules.iter().filter(|r| r.sop_category.as_deref() == Some(category)).collect()
    }
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
        [
            self.risk,
            self.fairness,
            self.privacy,
            self.novelty,
            self.security,
        ]
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
                        blocking_reason = format!(
                            "rule '{}': effect magnitude {magnitude:.2} > threshold {:.2}",
                            rule.id, self.risk_threshold
                        );
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

// ── RVF governance bridge ────────────────────────────────────
//
// Behind `exochain` feature gate: bidirectional mapping between
// WeftOS constitutional governance and RVF witness governance.
//
// WeftOS governance evaluates *whether* an action should proceed
// (effect algebra, risk thresholds, branch-based rules).
// RVF governance records *what happened* during execution
// (witness bundles, tool call traces, cost budgets).

#[cfg(feature = "exochain")]
impl GovernanceDecision {
    /// Map this decision to the equivalent RVF PolicyCheck.
    ///
    /// - `Permit` → `Allowed`
    /// - `PermitWithWarning` / `EscalateToHuman` → `Confirmed`
    /// - `Deny` → `Denied`
    pub fn to_rvf_policy_check(&self) -> rvf_types::witness::PolicyCheck {
        match self {
            GovernanceDecision::Permit => rvf_types::witness::PolicyCheck::Allowed,
            GovernanceDecision::PermitWithWarning(_) => rvf_types::witness::PolicyCheck::Confirmed,
            GovernanceDecision::EscalateToHuman(_) => rvf_types::witness::PolicyCheck::Confirmed,
            GovernanceDecision::Deny(_) => rvf_types::witness::PolicyCheck::Denied,
        }
    }
}

#[cfg(feature = "exochain")]
impl GovernanceEngine {
    /// Derive the equivalent RVF GovernanceMode from this engine's config.
    ///
    /// - `risk_threshold >= 1.0` (open) → `Autonomous`
    /// - `human_approval_required` → `Approved`
    /// - otherwise → `Restricted`
    pub fn to_rvf_mode(&self) -> rvf_types::witness::GovernanceMode {
        if self.risk_threshold >= 1.0 {
            rvf_types::witness::GovernanceMode::Autonomous
        } else if self.human_approval_required {
            rvf_types::witness::GovernanceMode::Approved
        } else {
            rvf_types::witness::GovernanceMode::Restricted
        }
    }

    /// Build an RVF GovernancePolicy from this engine's configuration.
    ///
    /// Uses the default tool lists and cost budgets for each mode.
    /// Callers can customize the returned policy further if needed.
    pub fn to_rvf_policy(&self) -> rvf_runtime::GovernancePolicy {
        match self.to_rvf_mode() {
            rvf_types::witness::GovernanceMode::Restricted => {
                rvf_runtime::GovernancePolicy::restricted()
            }
            rvf_types::witness::GovernanceMode::Approved => {
                rvf_runtime::GovernancePolicy::approved()
            }
            rvf_types::witness::GovernanceMode::Autonomous => {
                rvf_runtime::GovernancePolicy::autonomous()
            }
        }
    }
}

#[cfg(feature = "exochain")]
impl GovernanceResult {
    /// Map the decision to an RVF TaskOutcome.
    ///
    /// This is a convenience for recording the governance result in a
    /// witness bundle. The caller should override based on actual execution.
    pub fn to_rvf_task_outcome(&self) -> rvf_types::witness::TaskOutcome {
        match &self.decision {
            GovernanceDecision::Permit | GovernanceDecision::PermitWithWarning(_) => {
                rvf_types::witness::TaskOutcome::Solved
            }
            GovernanceDecision::EscalateToHuman(_) => rvf_types::witness::TaskOutcome::Skipped,
            GovernanceDecision::Deny(_) => rvf_types::witness::TaskOutcome::Failed,
        }
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
            reference_url: None,
            sop_category: None,
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
        assert!(
            GovernanceDecision::Deny("too risky".into())
                .to_string()
                .contains("too risky")
        );
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
        engine.add_rule(make_rule(
            "r1",
            RuleSeverity::Warning,
            GovernanceBranch::Legislative,
        ));
        engine.add_rule(make_rule(
            "r2",
            RuleSeverity::Blocking,
            GovernanceBranch::Judicial,
        ));
        engine.add_rule(make_rule(
            "r3",
            RuleSeverity::Advisory,
            GovernanceBranch::Judicial,
        ));

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
            reference_url: None,
            sop_category: None,
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

    #[test]
    fn filter_rules_by_sop_category() {
        let rules = vec![
            GovernanceRule {
                id: "SOP-L001".into(),
                description: "test".into(),
                branch: GovernanceBranch::Legislative,
                severity: RuleSeverity::Blocking,
                active: true,
                reference_url: Some("https://example.com".into()),
                sop_category: Some("governance".into()),
            },
            GovernanceRule {
                id: "SOP-J001".into(),
                description: "test".into(),
                branch: GovernanceBranch::Judicial,
                severity: RuleSeverity::Blocking,
                active: true,
                reference_url: Some("https://example.com".into()),
                sop_category: Some("ethics".into()),
            },
            GovernanceRule {
                id: "GOV-001".into(),
                description: "test".into(),
                branch: GovernanceBranch::Judicial,
                severity: RuleSeverity::Blocking,
                active: true,
                reference_url: None,
                sop_category: None,
            },
        ];
        let ethics = GovernanceRule::filter_by_category(&rules, "ethics");
        assert_eq!(ethics.len(), 1);
        assert_eq!(ethics[0].id, "SOP-J001");

        let governance = GovernanceRule::filter_by_category(&rules, "governance");
        assert_eq!(governance.len(), 1);

        let none = GovernanceRule::filter_by_category(&rules, "nonexistent");
        assert!(none.is_empty());
    }

    #[test]
    fn governance_rule_with_sop_serde() {
        let rule = GovernanceRule {
            id: "SOP-L001".into(),
            description: "test".into(),
            branch: GovernanceBranch::Legislative,
            severity: RuleSeverity::Blocking,
            active: true,
            reference_url: Some("https://example.com/sop".into()),
            sop_category: Some("governance".into()),
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("reference_url"));
        assert!(json.contains("sop_category"));
        let restored: GovernanceRule = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.reference_url, Some("https://example.com/sop".into()));
    }

    #[test]
    fn governance_rule_without_sop_backward_compat() {
        // Old-format JSON without new fields should deserialize fine
        let json = r#"{"id":"GOV-001","description":"test","branch":"Judicial","severity":"Blocking","active":true}"#;
        let rule: GovernanceRule = serde_json::from_str(json).unwrap();
        assert!(rule.reference_url.is_none());
        assert!(rule.sop_category.is_none());
    }

    // ── Genesis rule enforcement tests ──────────────────────────────

    /// Helper to create a GovernanceRule with optional SOP category,
    /// matching the shape used in boot.rs genesis rules.
    fn make_sop_rule(
        id: &str,
        severity: RuleSeverity,
        branch: GovernanceBranch,
        category: Option<&str>,
    ) -> GovernanceRule {
        GovernanceRule {
            id: id.into(),
            description: format!("Genesis rule {id}"),
            branch,
            severity,
            active: true,
            reference_url: None,
            sop_category: category.map(|c| c.into()),
        }
    }

    /// Build a GovernanceEngine with all 22 genesis rules matching boot.rs.
    fn genesis_engine() -> GovernanceEngine {
        let mut engine = GovernanceEngine::new(0.7, false);

        // ── Core constitutional rules (GOV-001 .. GOV-007) ──────
        // Judicial blocking
        engine.add_rule(make_sop_rule("GOV-001", RuleSeverity::Blocking, GovernanceBranch::Judicial, None));
        engine.add_rule(make_sop_rule("GOV-002", RuleSeverity::Blocking, GovernanceBranch::Judicial, None));
        // Legislative warning
        engine.add_rule(make_sop_rule("GOV-003", RuleSeverity::Warning, GovernanceBranch::Legislative, None));
        // Executive advisory
        engine.add_rule(make_sop_rule("GOV-004", RuleSeverity::Advisory, GovernanceBranch::Executive, None));
        // Legislative warning
        engine.add_rule(make_sop_rule("GOV-005", RuleSeverity::Warning, GovernanceBranch::Legislative, None));
        // Executive blocking
        engine.add_rule(make_sop_rule("GOV-006", RuleSeverity::Blocking, GovernanceBranch::Executive, None));
        // Judicial advisory
        engine.add_rule(make_sop_rule("GOV-007", RuleSeverity::Advisory, GovernanceBranch::Judicial, None));

        // ── AI-SDLC SOP rules: Legislative (6) ──────────────────
        engine.add_rule(make_sop_rule("SOP-L001", RuleSeverity::Blocking, GovernanceBranch::Legislative, Some("governance")));
        engine.add_rule(make_sop_rule("SOP-L002", RuleSeverity::Warning, GovernanceBranch::Legislative, Some("governance")));
        engine.add_rule(make_sop_rule("SOP-L003", RuleSeverity::Warning, GovernanceBranch::Legislative, Some("engineering")));
        engine.add_rule(make_sop_rule("SOP-L004", RuleSeverity::Advisory, GovernanceBranch::Legislative, Some("lifecycle")));
        engine.add_rule(make_sop_rule("SOP-L005", RuleSeverity::Blocking, GovernanceBranch::Legislative, Some("ethics")));
        engine.add_rule(make_sop_rule("SOP-L006", RuleSeverity::Warning, GovernanceBranch::Legislative, Some("governance")));

        // ── AI-SDLC SOP rules: Executive (5) ────────────────────
        engine.add_rule(make_sop_rule("SOP-E001", RuleSeverity::Warning, GovernanceBranch::Executive, Some("engineering")));
        engine.add_rule(make_sop_rule("SOP-E002", RuleSeverity::Blocking, GovernanceBranch::Executive, Some("lifecycle")));
        engine.add_rule(make_sop_rule("SOP-E003", RuleSeverity::Warning, GovernanceBranch::Executive, Some("security")));
        engine.add_rule(make_sop_rule("SOP-E004", RuleSeverity::Advisory, GovernanceBranch::Executive, Some("lifecycle")));
        engine.add_rule(make_sop_rule("SOP-E005", RuleSeverity::Advisory, GovernanceBranch::Executive, Some("governance")));

        // ── AI-SDLC SOP rules: Judicial (4) ─────────────────────
        engine.add_rule(make_sop_rule("SOP-J001", RuleSeverity::Blocking, GovernanceBranch::Judicial, Some("ethics")));
        engine.add_rule(make_sop_rule("SOP-J002", RuleSeverity::Warning, GovernanceBranch::Judicial, Some("ethics")));
        engine.add_rule(make_sop_rule("SOP-J003", RuleSeverity::Warning, GovernanceBranch::Judicial, Some("lifecycle")));
        engine.add_rule(make_sop_rule("SOP-J004", RuleSeverity::Advisory, GovernanceBranch::Judicial, Some("quality")));

        engine
    }

    #[test]
    fn genesis_has_22_rules() {
        let engine = genesis_engine();
        assert_eq!(engine.rule_count(), 22);
    }

    #[test]
    fn genesis_high_risk_operation_blocked() {
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "deploy-to-prod".into(),
            effect: EffectVector { risk: 0.9, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert!(
            matches!(result.decision, GovernanceDecision::Deny(_)),
            "high-risk operation should be denied, got {:?}", result.decision,
        );
        assert!(result.threshold_exceeded);
    }

    #[test]
    fn genesis_low_risk_operation_permitted() {
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "read-file".into(),
            effect: EffectVector { risk: 0.1, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert_eq!(result.decision, GovernanceDecision::Permit);
        assert!(!result.threshold_exceeded);
    }

    #[test]
    fn genesis_privacy_violation_triggers_enforcement() {
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "access-user-data".into(),
            effect: EffectVector { privacy: 0.8, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        // magnitude = 0.8 > 0.7 threshold; blocking rules exist -> Deny
        assert!(
            matches!(result.decision, GovernanceDecision::Deny(_) | GovernanceDecision::PermitWithWarning(_)),
            "privacy violation should trigger warning or deny, got {:?}", result.decision,
        );
        assert!(result.threshold_exceeded);
    }

    #[test]
    fn genesis_security_sensitive_blocked() {
        // GOV-002: security-sensitive actions blocked when threshold exceeded
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "modify-firewall".into(),
            effect: EffectVector { security: 0.9, risk: 0.5, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        // magnitude = sqrt(0.81 + 0.25) ~ 1.03 > 0.7
        assert!(matches!(result.decision, GovernanceDecision::Deny(_)));
    }

    #[test]
    fn genesis_fairness_bias_blocked() {
        // SOP-J001: bias/fairness evaluation blocking
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "ml-agent".into(),
            action: "evaluate-candidate".into(),
            effect: EffectVector { fairness: 0.9, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert!(
            matches!(result.decision, GovernanceDecision::Deny(_)),
            "fairness violation should be blocked by SOP-J001, got {:?}", result.decision,
        );
    }

    #[test]
    fn genesis_agent_spawn_blocked_when_risky() {
        // GOV-006 + SOP-E002: agent spawn with high risk denied
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "orchestrator".into(),
            action: "agent.spawn".into(),
            effect: EffectVector { risk: 0.8, novelty: 0.5, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        // magnitude = sqrt(0.64 + 0.25) ~ 0.94 > 0.7
        assert!(matches!(result.decision, GovernanceDecision::Deny(_)));
    }

    #[test]
    fn genesis_agent_spawn_permitted_when_safe() {
        // GOV-006: spawn with low effect should be permitted
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "orchestrator".into(),
            action: "agent.spawn".into(),
            effect: EffectVector { risk: 0.1, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert_eq!(result.decision, GovernanceDecision::Permit);
    }

    #[test]
    fn genesis_data_protection_blocks_high_privacy() {
        // SOP-L005: data protection blocking
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "data-agent".into(),
            action: "export-pii".into(),
            effect: EffectVector { privacy: 0.9, risk: 0.3, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        // magnitude = sqrt(0.81 + 0.09) ~ 0.95 > 0.7
        assert!(matches!(result.decision, GovernanceDecision::Deny(_)));
    }

    #[test]
    fn genesis_with_human_approval_escalates() {
        // Same rules but with human_approval_required = true
        let mut engine = GovernanceEngine::new(0.7, true);
        engine.add_rule(make_sop_rule("GOV-001", RuleSeverity::Blocking, GovernanceBranch::Judicial, None));

        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "high-risk-op".into(),
            effect: EffectVector { risk: 0.9, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert!(
            matches!(result.decision, GovernanceDecision::EscalateToHuman(_)),
            "with human_approval, blocking should escalate, got {:?}", result.decision,
        );
    }

    #[test]
    fn genesis_all_branches_represented() {
        let engine = genesis_engine();
        let legislative = engine.rules_by_branch(&GovernanceBranch::Legislative);
        let executive = engine.rules_by_branch(&GovernanceBranch::Executive);
        let judicial = engine.rules_by_branch(&GovernanceBranch::Judicial);

        // Legislative: GOV-003, GOV-005, SOP-L001..L006 = 8
        assert_eq!(legislative.len(), 8, "legislative should have 8 rules, got {}", legislative.len());
        // Executive: GOV-004, GOV-006, SOP-E001..E005 = 7
        assert_eq!(executive.len(), 7, "executive should have 7 rules, got {}", executive.len());
        // Judicial: GOV-001, GOV-002, GOV-007, SOP-J001..J004 = 7
        assert_eq!(judicial.len(), 7, "judicial should have 7 rules, got {}", judicial.len());
    }

    #[test]
    fn genesis_blocking_rules_count() {
        let engine = genesis_engine();
        let blocking_count = engine.active_rules().iter()
            .filter(|r| matches!(r.severity, RuleSeverity::Blocking | RuleSeverity::Critical))
            .count();
        // GOV-001, GOV-002, GOV-006, SOP-L001, SOP-L005, SOP-E002, SOP-J001 = 7
        assert_eq!(blocking_count, 7, "should have exactly 7 blocking rules");
    }

    #[test]
    fn genesis_warning_rules_count() {
        let engine = genesis_engine();
        let warning_count = engine.active_rules().iter()
            .filter(|r| matches!(r.severity, RuleSeverity::Warning))
            .count();
        // GOV-003, GOV-005, SOP-L002, SOP-L003, SOP-L006,
        // SOP-E001, SOP-E003, SOP-J002, SOP-J003 = 9
        assert_eq!(warning_count, 9, "should have exactly 9 warning rules");
    }

    #[test]
    fn genesis_advisory_rules_count() {
        let engine = genesis_engine();
        let advisory_count = engine.active_rules().iter()
            .filter(|r| matches!(r.severity, RuleSeverity::Advisory))
            .count();
        // GOV-004, GOV-007, SOP-L004, SOP-E004, SOP-E005, SOP-J004 = 6
        assert_eq!(advisory_count, 6, "should have exactly 6 advisory rules");
    }

    #[test]
    fn genesis_moderate_risk_with_only_warnings_permits_with_warning() {
        // Only warning-severity rules, no blocking: should PermitWithWarning
        let mut engine = GovernanceEngine::new(0.7, false);
        engine.add_rule(make_sop_rule("SOP-L002", RuleSeverity::Warning, GovernanceBranch::Legislative, Some("governance")));
        engine.add_rule(make_sop_rule("SOP-E001", RuleSeverity::Warning, GovernanceBranch::Executive, Some("engineering")));

        let request = GovernanceRequest {
            agent_id: "dev-agent".into(),
            action: "write-code".into(),
            effect: EffectVector { risk: 0.5, novelty: 0.5, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        // magnitude ~ 0.71 > 0.7, only warnings -> PermitWithWarning
        assert!(
            matches!(result.decision, GovernanceDecision::PermitWithWarning(_)),
            "warning-only rules above threshold should PermitWithWarning, got {:?}", result.decision,
        );
    }

    #[test]
    fn genesis_advisory_only_permits_above_threshold() {
        // Advisory rules alone never block or warn -- action is permitted
        let mut engine = GovernanceEngine::new(0.7, false);
        engine.add_rule(make_sop_rule("GOV-004", RuleSeverity::Advisory, GovernanceBranch::Executive, None));
        engine.add_rule(make_sop_rule("GOV-007", RuleSeverity::Advisory, GovernanceBranch::Judicial, None));

        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "novel-action".into(),
            effect: EffectVector { novelty: 0.9, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert_eq!(result.decision, GovernanceDecision::Permit,
            "advisory-only rules should still permit, got {:?}", result.decision);
        assert!(result.threshold_exceeded);
    }

    #[test]
    fn genesis_evaluates_all_22_rules() {
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "any-action".into(),
            effect: EffectVector { risk: 0.9, ..Default::default() },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        assert_eq!(
            result.evaluated_rules.len(), 22,
            "all 22 rules should be evaluated, got {}", result.evaluated_rules.len(),
        );
    }

    #[test]
    fn genesis_sop_categories_present() {
        let engine = genesis_engine();
        let all_rules = engine.active_rules();
        let categorized: Vec<_> = all_rules.iter()
            .filter(|r| r.sop_category.is_some())
            .collect();
        // 15 SOP rules have categories, 7 GOV rules do not
        assert_eq!(categorized.len(), 15, "15 SOP rules should have categories");

        let categories: std::collections::HashSet<_> = categorized.iter()
            .map(|r| r.sop_category.as_deref().unwrap())
            .collect();
        assert!(categories.contains("governance"));
        assert!(categories.contains("ethics"));
        assert!(categories.contains("engineering"));
        assert!(categories.contains("lifecycle"));
        assert!(categories.contains("security"));
        assert!(categories.contains("quality"));
    }

    #[test]
    fn genesis_multi_dimension_high_effect_denied() {
        // Multiple dimensions contributing to high magnitude
        let engine = genesis_engine();
        let request = GovernanceRequest {
            agent_id: "agent-1".into(),
            action: "risky-novel-private".into(),
            effect: EffectVector {
                risk: 0.4,
                privacy: 0.4,
                novelty: 0.4,
                security: 0.4,
                fairness: 0.0,
            },
            context: Default::default(),
        };
        let result = engine.evaluate(&request);
        // magnitude = sqrt(4 * 0.16) = sqrt(0.64) = 0.8 > 0.7
        assert!(
            matches!(result.decision, GovernanceDecision::Deny(_)),
            "combined multi-dimension effect should exceed threshold and deny, got {:?}",
            result.decision,
        );
        assert!(result.threshold_exceeded);
    }

    #[cfg(feature = "exochain")]
    mod rvf_bridge_tests {
        use super::*;

        #[test]
        fn decision_to_policy_check() {
            use rvf_types::witness::PolicyCheck;

            assert_eq!(
                GovernanceDecision::Permit.to_rvf_policy_check(),
                PolicyCheck::Allowed,
            );
            assert_eq!(
                GovernanceDecision::PermitWithWarning("low risk".into()).to_rvf_policy_check(),
                PolicyCheck::Confirmed,
            );
            assert_eq!(
                GovernanceDecision::EscalateToHuman("needs review".into()).to_rvf_policy_check(),
                PolicyCheck::Confirmed,
            );
            assert_eq!(
                GovernanceDecision::Deny("blocked".into()).to_rvf_policy_check(),
                PolicyCheck::Denied,
            );
        }

        #[test]
        fn open_engine_maps_to_autonomous() {
            use rvf_types::witness::GovernanceMode;

            let engine = GovernanceEngine::open();
            assert_eq!(engine.to_rvf_mode(), GovernanceMode::Autonomous);
        }

        #[test]
        fn strict_engine_maps_to_restricted() {
            use rvf_types::witness::GovernanceMode;

            let engine = GovernanceEngine::new(0.5, false);
            assert_eq!(engine.to_rvf_mode(), GovernanceMode::Restricted);
        }

        #[test]
        fn human_approval_maps_to_approved() {
            use rvf_types::witness::GovernanceMode;

            let engine = GovernanceEngine::new(0.5, true);
            assert_eq!(engine.to_rvf_mode(), GovernanceMode::Approved);
        }

        #[test]
        fn to_rvf_policy_mode_matches() {
            use rvf_types::witness::GovernanceMode;

            let open = GovernanceEngine::open();
            let policy = open.to_rvf_policy();
            assert_eq!(policy.mode, GovernanceMode::Autonomous);

            let strict = GovernanceEngine::new(0.3, false);
            let policy = strict.to_rvf_policy();
            assert_eq!(policy.mode, GovernanceMode::Restricted);

            let human = GovernanceEngine::new(0.3, true);
            let policy = human.to_rvf_policy();
            assert_eq!(policy.mode, GovernanceMode::Approved);
        }

        #[test]
        fn governance_result_to_task_outcome() {
            use rvf_types::witness::TaskOutcome;

            let permit_result = GovernanceResult {
                decision: GovernanceDecision::Permit,
                evaluated_rules: vec![],
                effect: EffectVector::default(),
                threshold_exceeded: false,
            };
            assert_eq!(permit_result.to_rvf_task_outcome() as u8, TaskOutcome::Solved as u8);

            let deny_result = GovernanceResult {
                decision: GovernanceDecision::Deny("blocked".into()),
                evaluated_rules: vec![],
                effect: EffectVector::default(),
                threshold_exceeded: true,
            };
            assert_eq!(deny_result.to_rvf_task_outcome() as u8, TaskOutcome::Failed as u8);

            let escalate_result = GovernanceResult {
                decision: GovernanceDecision::EscalateToHuman("review".into()),
                evaluated_rules: vec![],
                effect: EffectVector::default(),
                threshold_exceeded: true,
            };
            assert_eq!(escalate_result.to_rvf_task_outcome() as u8, TaskOutcome::Skipped as u8);
        }
    }
}
