//! Agent routing configuration types.
//!
//! Defines the config schema for the L1 agent routing table which maps
//! channel + user combinations to specific agent instances.
//!
//! # Configuration format
//!
//! ```toml
//! [[agent_routes]]
//! channel = "telegram"
//! match = { user_id = "12345" }
//! agent = "work-agent"
//!
//! [[agent_routes]]
//! channel = "whatsapp"
//! match = { phone = "+1..." }
//! agent = "personal-agent"
//!
//! [agent_routing]
//! catch_all = "default-agent"
//! ```

use serde::{Deserialize, Serialize};

/// A single routing rule that maps channel + match criteria to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoute {
    /// Channel name to match (e.g. "telegram", "slack", "discord").
    pub channel: String,

    /// Criteria to match within the channel.
    #[serde(rename = "match", default)]
    pub match_criteria: MatchCriteria,

    /// Agent ID to route to when this rule matches.
    pub agent: String,
}

/// Criteria used to match an inbound message to a routing rule.
///
/// All fields are optional. A field that is `None` matches any value.
/// Multiple non-`None` fields are AND-ed: all must match.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchCriteria {
    /// Match by sender user ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Match by phone number (for channels like WhatsApp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Match by chat/conversation ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
}

impl MatchCriteria {
    /// Returns true if all specified criteria match the given values.
    ///
    /// Fields that are `None` are treated as wildcards (always match).
    pub fn matches(&self, sender_id: &str, chat_id: &str) -> bool {
        if let Some(ref uid) = self.user_id
            && uid != sender_id
        {
            return false;
        }
        if let Some(ref phone) = self.phone
            && phone != sender_id
            && phone != chat_id
        {
            // Phone can match either sender_id or chat_id.
            return false;
        }
        if let Some(ref cid) = self.chat_id
            && cid != chat_id
        {
            return false;
        }
        true
    }

    /// Returns true if no criteria are specified (matches everything).
    pub fn is_empty(&self) -> bool {
        self.user_id.is_none() && self.phone.is_none() && self.chat_id.is_none()
    }
}

/// Top-level agent routing configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentRoutingConfig {
    /// Ordered routing rules. First match wins.
    #[serde(default)]
    pub routes: Vec<AgentRoute>,

    /// Optional catch-all agent for messages that match no rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catch_all: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_criteria_empty_matches_all() {
        let criteria = MatchCriteria::default();
        assert!(criteria.is_empty());
        assert!(criteria.matches("any_user", "any_chat"));
    }

    #[test]
    fn match_criteria_user_id() {
        let criteria = MatchCriteria {
            user_id: Some("user123".into()),
            ..Default::default()
        };
        assert!(criteria.matches("user123", "chat1"));
        assert!(!criteria.matches("other_user", "chat1"));
    }

    #[test]
    fn match_criteria_phone() {
        let criteria = MatchCriteria {
            phone: Some("+1234567890".into()),
            ..Default::default()
        };
        // Phone matches sender_id.
        assert!(criteria.matches("+1234567890", "chat1"));
        // Phone matches chat_id.
        assert!(criteria.matches("other", "+1234567890"));
        // No match.
        assert!(!criteria.matches("other", "chat1"));
    }

    #[test]
    fn match_criteria_chat_id() {
        let criteria = MatchCriteria {
            chat_id: Some("chat42".into()),
            ..Default::default()
        };
        assert!(criteria.matches("any_user", "chat42"));
        assert!(!criteria.matches("any_user", "other_chat"));
    }

    #[test]
    fn match_criteria_combined_and() {
        let criteria = MatchCriteria {
            user_id: Some("user1".into()),
            chat_id: Some("chat1".into()),
            ..Default::default()
        };
        // Both must match.
        assert!(criteria.matches("user1", "chat1"));
        // Wrong user.
        assert!(!criteria.matches("user2", "chat1"));
        // Wrong chat.
        assert!(!criteria.matches("user1", "chat2"));
    }

    #[test]
    fn agent_route_serde_roundtrip() {
        let route = AgentRoute {
            channel: "telegram".into(),
            match_criteria: MatchCriteria {
                user_id: Some("12345".into()),
                ..Default::default()
            },
            agent: "work-agent".into(),
        };
        let json = serde_json::to_string(&route).unwrap();
        let restored: AgentRoute = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.channel, "telegram");
        assert_eq!(restored.agent, "work-agent");
        assert_eq!(restored.match_criteria.user_id.as_deref(), Some("12345"));
    }

    #[test]
    fn agent_routing_config_defaults() {
        let cfg = AgentRoutingConfig::default();
        assert!(cfg.routes.is_empty());
        assert!(cfg.catch_all.is_none());
    }

    #[test]
    fn agent_routing_config_serde_with_catch_all() {
        let json = r#"{
            "routes": [
                {"channel": "telegram", "match": {"user_id": "123"}, "agent": "bot-a"},
                {"channel": "slack", "agent": "bot-b"}
            ],
            "catch_all": "default-bot"
        }"#;
        let cfg: AgentRoutingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.routes.len(), 2);
        assert_eq!(cfg.routes[0].agent, "bot-a");
        assert_eq!(cfg.routes[1].agent, "bot-b");
        assert_eq!(cfg.catch_all.as_deref(), Some("default-bot"));
    }
}
