//! Slack Web API client.
//!
//! [`SlackApiClient`] provides typed methods for the subset of the
//! Slack Web API used by the channel plugin: `apps.connections.open`,
//! `chat.postMessage`, and `chat.update`.

use reqwest::Client;
use tracing::debug;

use clawft_types::error::ChannelError;

use super::events::{ChatPostMessageResponse, ChatUpdateResponse, ConnectionsOpenResponse};

/// Base URL for the Slack Web API.
const SLACK_API_BASE: &str = "https://slack.com/api";

/// HTTP client for the Slack Web API.
///
/// Wraps a [`reqwest::Client`] and the bot token to provide typed
/// request methods. The base URL can be overridden for testing.
pub struct SlackApiClient {
    /// Shared HTTP client.
    http: Client,
    /// Bot token for API authorization.
    bot_token: String,
    /// Base URL for API calls.
    base_url: String,
}

impl SlackApiClient {
    /// Create a new client with the given bot token.
    pub fn new(bot_token: String) -> Self {
        Self {
            http: Client::new(),
            bot_token,
            base_url: SLACK_API_BASE.to_owned(),
        }
    }

    /// Create a client pointing at a custom base URL (for testing).
    #[cfg(test)]
    pub fn with_base_url(bot_token: String, base_url: String) -> Self {
        Self {
            http: Client::new(),
            bot_token,
            base_url,
        }
    }

    /// Return the base URL used for API requests.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Call `apps.connections.open` to obtain a Socket Mode WebSocket URL.
    ///
    /// This endpoint requires the **app-level token** (`xapp-...`), not
    /// the bot token. The caller must supply the app token explicitly.
    pub async fn apps_connections_open(
        &self,
        app_token: &str,
    ) -> Result<String, ChannelError> {
        let url = format!("{}/apps.connections.open", self.base_url);

        debug!("calling apps.connections.open");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {app_token}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(e.to_string()))?;

        let body: ConnectionsOpenResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(e.to_string()))?;

        if !body.ok {
            let err_msg = body.error.unwrap_or_else(|| "unknown error".into());
            return Err(ChannelError::AuthFailed(format!(
                "apps.connections.open failed: {err_msg}"
            )));
        }

        body.url.ok_or_else(|| {
            ChannelError::ConnectionFailed(
                "apps.connections.open returned ok but no URL".into(),
            )
        })
    }

    /// Post a message to a Slack channel.
    ///
    /// Returns the message timestamp (`ts`) on success.
    pub async fn chat_post_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> Result<String, ChannelError> {
        let url = format!("{}/chat.postMessage", self.base_url);

        let mut body = serde_json::json!({
            "channel": channel,
            "text": text,
        });

        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_owned());
        }

        debug!(channel = %channel, "posting message");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        let result: ChatPostMessageResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        if !result.ok {
            let err_msg = result.error.unwrap_or_else(|| "unknown error".into());
            return Err(ChannelError::SendFailed(format!(
                "chat.postMessage failed: {err_msg}"
            )));
        }

        result.ts.ok_or_else(|| {
            ChannelError::SendFailed("chat.postMessage returned ok but no ts".into())
        })
    }

    /// Update an existing message.
    pub async fn chat_update(
        &self,
        channel: &str,
        ts: &str,
        text: &str,
    ) -> Result<(), ChannelError> {
        let url = format!("{}/chat.update", self.base_url);

        let body = serde_json::json!({
            "channel": channel,
            "ts": ts,
            "text": text,
        });

        debug!(channel = %channel, ts = %ts, "updating message");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        let result: ChatUpdateResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        if !result.ok {
            let err_msg = result.error.unwrap_or_else(|| "unknown error".into());
            return Err(ChannelError::SendFailed(format!(
                "chat.update failed: {err_msg}"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_url() {
        let client = SlackApiClient::new("xoxb-test".into());
        assert_eq!(client.base_url(), "https://slack.com/api");
    }

    #[test]
    fn custom_base_url() {
        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "http://localhost:9999".into());
        assert_eq!(client.base_url(), "http://localhost:9999");
    }
}
