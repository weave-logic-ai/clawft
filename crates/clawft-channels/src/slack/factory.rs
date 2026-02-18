//! [`SlackChannelFactory`] -- creates Slack channels from JSON config.

use std::sync::Arc;

use clawft_types::config::SlackConfig;
use clawft_types::error::ChannelError;

use crate::traits::{Channel, ChannelFactory};

use super::channel::SlackChannel;

/// Factory for creating [`SlackChannel`] instances from JSON configuration.
///
/// Expected config shape matches [`SlackConfig`]:
///
/// ```json
/// {
///   "enabled": true,
///   "mode": "socket",
///   "bot_token": "xoxb-...",
///   "app_token": "xapp-...",
///   "group_policy": "mention",
///   "dm": { "enabled": true, "policy": "open" }
/// }
/// ```
pub struct SlackChannelFactory;

impl ChannelFactory for SlackChannelFactory {
    fn channel_name(&self) -> &str {
        "slack"
    }

    fn build(&self, config: &serde_json::Value) -> Result<Arc<dyn Channel>, ChannelError> {
        let slack_config: SlackConfig = serde_json::from_value(config.clone())
            .map_err(|e| ChannelError::Other(format!("invalid slack config: {e}")))?;

        if slack_config.bot_token.is_empty() {
            return Err(ChannelError::Other(
                "missing 'bot_token' in slack config".into(),
            ));
        }

        if slack_config.app_token.is_empty() {
            return Err(ChannelError::Other(
                "missing 'app_token' in slack config".into(),
            ));
        }

        Ok(Arc::new(SlackChannel::new(slack_config)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_channel_name() {
        let factory = SlackChannelFactory;
        assert_eq!(factory.channel_name(), "slack");
    }

    #[test]
    fn factory_build_success() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({
            "enabled": true,
            "bot_token": "xoxb-test-token",
            "app_token": "xapp-test-token",
            "mode": "socket",
            "group_policy": "mention"
        });
        let channel = factory.build(&config);
        assert!(channel.is_ok());
        let ch = channel.unwrap();
        assert_eq!(ch.name(), "slack");
    }

    #[test]
    fn factory_build_missing_bot_token_errors() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({
            "enabled": true,
            "app_token": "xapp-test"
        });
        let result = factory.build(&config);
        match result {
            Err(ChannelError::Other(msg)) => {
                assert!(
                    msg.contains("bot_token"),
                    "error should mention bot_token: {msg}"
                );
            }
            Err(other) => panic!("expected ChannelError::Other, got: {other:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn factory_build_missing_app_token_errors() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({
            "enabled": true,
            "bot_token": "xoxb-test"
        });
        let result = factory.build(&config);
        match result {
            Err(ChannelError::Other(msg)) => {
                assert!(
                    msg.contains("app_token"),
                    "error should mention app_token: {msg}"
                );
            }
            Err(other) => panic!("expected ChannelError::Other, got: {other:?}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    #[test]
    fn factory_build_empty_config_errors() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({});
        let result = factory.build(&config);
        assert!(result.is_err());
    }

    #[test]
    fn factory_build_with_dm_config() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({
            "bot_token": "xoxb-test",
            "app_token": "xapp-test",
            "dm": {
                "enabled": true,
                "policy": "allowlist",
                "allow_from": ["U123", "U456"]
            }
        });
        let channel = factory.build(&config);
        assert!(channel.is_ok());
    }

    #[test]
    fn factory_build_with_group_allowlist() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({
            "bot_token": "xoxb-test",
            "app_token": "xapp-test",
            "group_policy": "allowlist",
            "group_allow_from": ["C123", "C456"]
        });
        let channel = factory.build(&config);
        assert!(channel.is_ok());
    }

    #[test]
    fn factory_build_camel_case_aliases() {
        let factory = SlackChannelFactory;
        let config = serde_json::json!({
            "botToken": "xoxb-test",
            "appToken": "xapp-test",
            "groupPolicy": "open"
        });
        let channel = factory.build(&config);
        assert!(channel.is_ok());
    }
}
