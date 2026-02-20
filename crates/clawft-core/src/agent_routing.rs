//! Agent routing engine.
//!
//! Routes inbound messages to the appropriate agent based on channel
//! and user match criteria. Supports first-match-wins semantics,
//! catch-all fallback, and automatic workspace creation.
//!
//! # Routing algorithm
//!
//! 1. Walk routes in order; first match wins.
//! 2. If no route matches and a catch-all is configured, use it.
//! 3. If no route matches and no catch-all: reject with error.
//! 4. Anonymous messages (empty `sender_id`) route to catch-all or
//!    a dedicated "anonymous" agent with reduced permissions.

use tracing::warn;

use clawft_types::agent_routing::{AgentRoute, AgentRoutingConfig};
use clawft_types::event::InboundMessage;

/// Result of routing an inbound message to an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingResult {
    /// Message routed to a specific agent by rule match.
    Agent(String),
    /// Message routed to the catch-all agent.
    CatchAll(String),
    /// No matching route and no catch-all configured.
    NoMatch,
}

/// Engine that routes inbound messages to agent instances.
///
/// Created from an [`AgentRoutingConfig`] loaded from `clawft.toml`.
/// Routes are evaluated in order; first match wins.
pub struct AgentRouter {
    routes: Vec<AgentRoute>,
    catch_all: Option<String>,
}

impl AgentRouter {
    /// Create a new router from configuration.
    pub fn new(config: AgentRoutingConfig) -> Self {
        Self {
            routes: config.routes,
            catch_all: config.catch_all,
        }
    }

    /// Create an empty router with no rules.
    pub fn empty() -> Self {
        Self {
            routes: Vec::new(),
            catch_all: None,
        }
    }

    /// Create a router with only a catch-all agent (no rules).
    pub fn with_catch_all(agent: impl Into<String>) -> Self {
        Self {
            routes: Vec::new(),
            catch_all: Some(agent.into()),
        }
    }

    /// Route a message to the appropriate agent ID.
    ///
    /// # Routing semantics
    ///
    /// - First matching rule wins.
    /// - Anonymous messages (empty `sender_id`) are routed to the
    ///   catch-all agent (if configured).
    /// - No match + no catch-all: returns [`RoutingResult::NoMatch`]
    ///   and logs a warning.
    pub fn route(&self, msg: &InboundMessage) -> RoutingResult {
        // Anonymous messages go straight to catch-all.
        if msg.sender_id.is_empty() {
            return self.route_anonymous(msg);
        }

        // Walk rules in order; first match wins.
        for rule in &self.routes {
            if rule.channel == msg.channel
                && rule.match_criteria.matches(&msg.sender_id, &msg.chat_id)
            {
                return RoutingResult::Agent(rule.agent.clone());
            }
        }

        // No rule matched.
        if let Some(ref catch_all) = self.catch_all {
            RoutingResult::CatchAll(catch_all.clone())
        } else {
            warn!(
                channel = %msg.channel,
                sender_id = %msg.sender_id,
                chat_id = %msg.chat_id,
                "no agent configured for this channel/user"
            );
            RoutingResult::NoMatch
        }
    }

    /// Route an anonymous message (empty sender_id).
    fn route_anonymous(&self, msg: &InboundMessage) -> RoutingResult {
        if let Some(ref catch_all) = self.catch_all {
            RoutingResult::CatchAll(catch_all.clone())
        } else {
            warn!(
                channel = %msg.channel,
                chat_id = %msg.chat_id,
                "anonymous message with no catch-all configured"
            );
            RoutingResult::NoMatch
        }
    }

    /// Number of configured routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Whether a catch-all agent is configured.
    pub fn has_catch_all(&self) -> bool {
        self.catch_all.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use clawft_types::agent_routing::{AgentRoute, AgentRoutingConfig, MatchCriteria};
    use std::collections::HashMap;

    fn make_msg(channel: &str, sender_id: &str, chat_id: &str) -> InboundMessage {
        InboundMessage {
            channel: channel.into(),
            sender_id: sender_id.into(),
            chat_id: chat_id.into(),
            content: "test".into(),
            timestamp: Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        }
    }

    fn make_router() -> AgentRouter {
        AgentRouter::new(AgentRoutingConfig {
            routes: vec![
                AgentRoute {
                    channel: "telegram".into(),
                    match_criteria: MatchCriteria {
                        user_id: Some("user123".into()),
                        ..Default::default()
                    },
                    agent: "work-agent".into(),
                },
                AgentRoute {
                    channel: "whatsapp".into(),
                    match_criteria: MatchCriteria {
                        phone: Some("+15551234".into()),
                        ..Default::default()
                    },
                    agent: "personal-agent".into(),
                },
                AgentRoute {
                    channel: "slack".into(),
                    match_criteria: MatchCriteria::default(),
                    agent: "slack-agent".into(),
                },
            ],
            catch_all: Some("default-agent".into()),
        })
    }

    #[test]
    fn route_first_match_wins() {
        let router = make_router();
        let msg = make_msg("telegram", "user123", "chat1");
        assert_eq!(
            router.route(&msg),
            RoutingResult::Agent("work-agent".into())
        );
    }

    #[test]
    fn route_whatsapp_by_phone() {
        let router = make_router();
        let msg = make_msg("whatsapp", "+15551234", "chat2");
        assert_eq!(
            router.route(&msg),
            RoutingResult::Agent("personal-agent".into())
        );
    }

    #[test]
    fn route_wildcard_match() {
        let router = make_router();
        // Slack rule has empty match criteria (wildcard).
        let msg = make_msg("slack", "any_user", "any_chat");
        assert_eq!(
            router.route(&msg),
            RoutingResult::Agent("slack-agent".into())
        );
    }

    #[test]
    fn route_no_match_falls_to_catch_all() {
        let router = make_router();
        let msg = make_msg("discord", "user456", "chat3");
        assert_eq!(
            router.route(&msg),
            RoutingResult::CatchAll("default-agent".into())
        );
    }

    #[test]
    fn route_no_match_no_catch_all() {
        let router = AgentRouter::new(AgentRoutingConfig {
            routes: vec![AgentRoute {
                channel: "telegram".into(),
                match_criteria: MatchCriteria {
                    user_id: Some("user123".into()),
                    ..Default::default()
                },
                agent: "work-agent".into(),
            }],
            catch_all: None,
        });
        let msg = make_msg("discord", "user456", "chat3");
        assert_eq!(router.route(&msg), RoutingResult::NoMatch);
    }

    #[test]
    fn route_anonymous_to_catch_all() {
        let router = make_router();
        let msg = make_msg("telegram", "", "chat1");
        assert_eq!(
            router.route(&msg),
            RoutingResult::CatchAll("default-agent".into())
        );
    }

    #[test]
    fn route_anonymous_no_catch_all() {
        let router = AgentRouter::empty();
        let msg = make_msg("telegram", "", "chat1");
        assert_eq!(router.route(&msg), RoutingResult::NoMatch);
    }

    #[test]
    fn route_wrong_channel_no_match() {
        let router = make_router();
        // user123 exists but on telegram, not discord.
        let msg = make_msg("discord", "user123", "chat1");
        assert_eq!(
            router.route(&msg),
            RoutingResult::CatchAll("default-agent".into())
        );
    }

    #[test]
    fn empty_router() {
        let router = AgentRouter::empty();
        assert_eq!(router.route_count(), 0);
        assert!(!router.has_catch_all());
    }

    #[test]
    fn with_catch_all_only() {
        let router = AgentRouter::with_catch_all("fallback");
        let msg = make_msg("any", "user", "chat");
        assert_eq!(
            router.route(&msg),
            RoutingResult::CatchAll("fallback".into())
        );
    }
}
