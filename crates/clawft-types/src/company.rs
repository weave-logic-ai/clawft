//! Company and org-chart types for the Paperclip Patterns integration.
//!
//! Models organizational structure: companies, agent roles within an
//! org chart, and the hierarchical reporting relationships between agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A company or organisation that owns agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    /// Unique identifier (UUID or slug).
    pub id: String,
    /// Human-readable company name.
    pub name: String,
    /// Optional description of the company's purpose.
    #[serde(default)]
    pub description: String,
    /// When the company record was created.
    pub created_at: DateTime<Utc>,
}

impl Company {
    /// Create a new company with the current timestamp.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            created_at: Utc::now(),
        }
    }
}

/// Role an agent holds within an organisational chart.
///
/// This is distinct from [`clawft_kernel::AgentRole`] which describes
/// the OS-level role (root, supervisor, worker). `OrgRole` models
/// business-level hierarchy inspired by the Paperclip Patterns.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    /// Chief executive -- top-level strategic agent.
    Ceo,
    /// Middle management -- coordinates workers.
    Manager,
    /// Executes tasks assigned by managers.
    Worker,
    /// A custom organisational role.
    Custom(String),
}

impl std::fmt::Display for OrgRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrgRole::Ceo => write!(f, "ceo"),
            OrgRole::Manager => write!(f, "manager"),
            OrgRole::Worker => write!(f, "worker"),
            OrgRole::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

/// A node in an organisational chart, representing one agent's position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgNode {
    /// The agent occupying this position.
    pub agent_id: String,
    /// Organisational role.
    pub role: OrgRole,
    /// Who this agent reports to (`None` for the CEO / root).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reports_to: Option<String>,
    /// Budget allocated to this node, in cents (avoids float precision).
    #[serde(default)]
    pub budget_cents: u64,
    /// High-level goals assigned to this node.
    #[serde(default)]
    pub goals: Vec<String>,
}

/// An organisational chart linking a company to its agent hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgChart {
    /// The company this chart belongs to.
    pub company_id: String,
    /// Ordered list of org nodes (first node is typically the CEO).
    pub nodes: Vec<OrgNode>,
}

impl OrgChart {
    /// Create an empty org chart for a company.
    pub fn new(company_id: impl Into<String>) -> Self {
        Self {
            company_id: company_id.into(),
            nodes: Vec::new(),
        }
    }

    /// Return all direct reports of the given agent.
    pub fn direct_reports(&self, agent_id: &str) -> Vec<&OrgNode> {
        self.nodes
            .iter()
            .filter(|n| n.reports_to.as_deref() == Some(agent_id))
            .collect()
    }

    /// Find a node by agent ID.
    pub fn find_agent(&self, agent_id: &str) -> Option<&OrgNode> {
        self.nodes.iter().find(|n| n.agent_id == agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_new() {
        let c = Company::new("acme-1", "Acme Corp");
        assert_eq!(c.id, "acme-1");
        assert_eq!(c.name, "Acme Corp");
        assert!(c.description.is_empty());
    }

    #[test]
    fn company_serde_roundtrip() {
        let c = Company {
            id: "co-1".into(),
            name: "Test Inc".into(),
            description: "A test company".into(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&c).unwrap();
        let restored: Company = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "co-1");
        assert_eq!(restored.name, "Test Inc");
        assert_eq!(restored.description, "A test company");
    }

    #[test]
    fn org_role_serde() {
        let roles = [
            (OrgRole::Ceo, "\"ceo\""),
            (OrgRole::Manager, "\"manager\""),
            (OrgRole::Worker, "\"worker\""),
        ];
        for (role, expected) in &roles {
            let json = serde_json::to_string(role).unwrap();
            assert_eq!(&json, expected);
            let restored: OrgRole = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, role);
        }
    }

    #[test]
    fn org_role_custom_serde() {
        let role = OrgRole::Custom("intern".into());
        let json = serde_json::to_string(&role).unwrap();
        let restored: OrgRole = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, OrgRole::Custom("intern".into()));
    }

    #[test]
    fn org_role_display() {
        assert_eq!(OrgRole::Ceo.to_string(), "ceo");
        assert_eq!(OrgRole::Manager.to_string(), "manager");
        assert_eq!(OrgRole::Worker.to_string(), "worker");
        assert_eq!(OrgRole::Custom("vp".into()).to_string(), "custom(vp)");
    }

    #[test]
    fn org_node_serde_roundtrip() {
        let node = OrgNode {
            agent_id: "agent-a".into(),
            role: OrgRole::Manager,
            reports_to: Some("agent-ceo".into()),
            budget_cents: 500_000,
            goals: vec!["ship v2".into(), "reduce churn".into()],
        };
        let json = serde_json::to_string(&node).unwrap();
        let restored: OrgNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agent_id, "agent-a");
        assert_eq!(restored.role, OrgRole::Manager);
        assert_eq!(restored.reports_to.as_deref(), Some("agent-ceo"));
        assert_eq!(restored.budget_cents, 500_000);
        assert_eq!(restored.goals.len(), 2);
    }

    #[test]
    fn org_node_omits_none_reports_to() {
        let node = OrgNode {
            agent_id: "ceo".into(),
            role: OrgRole::Ceo,
            reports_to: None,
            budget_cents: 0,
            goals: vec![],
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(!json.contains("reports_to"));
    }

    #[test]
    fn org_chart_new_is_empty() {
        let chart = OrgChart::new("co-1");
        assert_eq!(chart.company_id, "co-1");
        assert!(chart.nodes.is_empty());
    }

    #[test]
    fn org_chart_direct_reports() {
        let chart = OrgChart {
            company_id: "co-1".into(),
            nodes: vec![
                OrgNode {
                    agent_id: "ceo".into(),
                    role: OrgRole::Ceo,
                    reports_to: None,
                    budget_cents: 1_000_000,
                    goals: vec!["grow".into()],
                },
                OrgNode {
                    agent_id: "mgr-1".into(),
                    role: OrgRole::Manager,
                    reports_to: Some("ceo".into()),
                    budget_cents: 300_000,
                    goals: vec![],
                },
                OrgNode {
                    agent_id: "mgr-2".into(),
                    role: OrgRole::Manager,
                    reports_to: Some("ceo".into()),
                    budget_cents: 200_000,
                    goals: vec![],
                },
                OrgNode {
                    agent_id: "wk-1".into(),
                    role: OrgRole::Worker,
                    reports_to: Some("mgr-1".into()),
                    budget_cents: 0,
                    goals: vec![],
                },
            ],
        };
        let ceo_reports = chart.direct_reports("ceo");
        assert_eq!(ceo_reports.len(), 2);
        assert!(ceo_reports.iter().any(|n| n.agent_id == "mgr-1"));
        assert!(ceo_reports.iter().any(|n| n.agent_id == "mgr-2"));

        let mgr1_reports = chart.direct_reports("mgr-1");
        assert_eq!(mgr1_reports.len(), 1);
        assert_eq!(mgr1_reports[0].agent_id, "wk-1");

        let wk_reports = chart.direct_reports("wk-1");
        assert!(wk_reports.is_empty());
    }

    #[test]
    fn org_chart_find_agent() {
        let chart = OrgChart {
            company_id: "co-1".into(),
            nodes: vec![OrgNode {
                agent_id: "a1".into(),
                role: OrgRole::Worker,
                reports_to: None,
                budget_cents: 0,
                goals: vec![],
            }],
        };
        assert!(chart.find_agent("a1").is_some());
        assert!(chart.find_agent("nonexistent").is_none());
    }

    #[test]
    fn org_chart_serde_roundtrip() {
        let chart = OrgChart {
            company_id: "co-1".into(),
            nodes: vec![
                OrgNode {
                    agent_id: "ceo".into(),
                    role: OrgRole::Ceo,
                    reports_to: None,
                    budget_cents: 1_000_000,
                    goals: vec!["profit".into()],
                },
                OrgNode {
                    agent_id: "w1".into(),
                    role: OrgRole::Worker,
                    reports_to: Some("ceo".into()),
                    budget_cents: 0,
                    goals: vec![],
                },
            ],
        };
        let json = serde_json::to_string(&chart).unwrap();
        let restored: OrgChart = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.company_id, "co-1");
        assert_eq!(restored.nodes.len(), 2);
    }
}
