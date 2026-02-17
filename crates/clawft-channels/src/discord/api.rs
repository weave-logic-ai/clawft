//! Discord REST API client.
//!
//! [`DiscordApiClient`] provides typed methods for the subset of the
//! Discord REST API used by the channel plugin: sending and editing
//! messages.

use reqwest::Client;
use tracing::{debug, warn};

use clawft_types::error::ChannelError;

use super::events::RateLimitInfo;

/// Base URL for the Discord REST API v10.
const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

/// Response from creating or editing a message.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiscordMessage {
    /// Unique message ID.
    pub id: String,

    /// Channel where the message lives.
    pub channel_id: String,

    /// Message content.
    pub content: String,
}

/// HTTP client for the Discord REST API.
///
/// Wraps a [`reqwest::Client`] with Bot token authentication and
/// basic rate limit tracking.
pub struct DiscordApiClient {
    /// Shared HTTP client.
    http: Client,
    /// Bot token for API authorization.
    token: String,
    /// Base URL for API calls.
    base_url: String,
}

impl DiscordApiClient {
    /// Create a new client with the given bot token.
    pub fn new(token: String) -> Self {
        Self {
            http: Client::new(),
            token,
            base_url: DISCORD_API_BASE.to_owned(),
        }
    }

    /// Create a client pointing at a custom base URL (for testing).
    #[cfg(test)]
    pub fn with_base_url(token: String, base_url: String) -> Self {
        Self {
            http: Client::new(),
            token,
            base_url,
        }
    }

    /// Return the base URL used for API requests.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Send a message to a channel.
    ///
    /// Returns the message ID on success.
    pub async fn create_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<String, ChannelError> {
        let url = format!("{}/channels/{channel_id}/messages", self.base_url);

        let body = serde_json::json!({
            "content": content,
        });

        debug!(channel_id = %channel_id, "creating message");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        // Check rate limit headers.
        let rate_limit = RateLimitInfo::from_headers(resp.headers());
        if rate_limit.is_limited() {
            let wait_ms = rate_limit.retry_after_ms().unwrap_or(1000);
            warn!(
                wait_ms = wait_ms,
                "Discord rate limit reached, waiting before retry"
            );
            tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
        }

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(ChannelError::SendFailed(format!(
                "Discord API returned {status}: {err_body}"
            )));
        }

        let msg: DiscordMessage = resp
            .json()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        Ok(msg.id)
    }

    /// Edit an existing message.
    pub async fn edit_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<(), ChannelError> {
        let url = format!(
            "{}/channels/{channel_id}/messages/{message_id}",
            self.base_url
        );

        let body = serde_json::json!({
            "content": content,
        });

        debug!(
            channel_id = %channel_id,
            message_id = %message_id,
            "editing message"
        );

        let resp = self
            .http
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(e.to_string()))?;

        // Check rate limit headers.
        let rate_limit = RateLimitInfo::from_headers(resp.headers());
        if rate_limit.is_limited() {
            let wait_ms = rate_limit.retry_after_ms().unwrap_or(1000);
            warn!(
                wait_ms = wait_ms,
                "Discord rate limit reached, waiting before retry"
            );
        }

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(ChannelError::SendFailed(format!(
                "Discord API returned {status}: {err_body}"
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
        let client = DiscordApiClient::new("test-token".into());
        assert_eq!(client.base_url(), "https://discord.com/api/v10");
    }

    #[test]
    fn custom_base_url() {
        let client =
            DiscordApiClient::with_base_url("test-token".into(), "http://localhost:9999".into());
        assert_eq!(client.base_url(), "http://localhost:9999");
    }
}
