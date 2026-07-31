//! [`TelegramChannel`] -- dual [`Channel`] + [`ChannelAdapter`] for Telegram.
//!
//! Uses long polling via [`TelegramClient`](super::client::TelegramClient)
//! to receive updates. Primary surface is [`ChannelAdapter`] (WEFT-170 / C7);
//! the legacy [`Channel`] trait remains implemented for `PluginHost` /
//! gateway compatibility by bridging through
//! [`ChannelAdapterHostBridge`](crate::plugin_host::ChannelAdapterHostBridge).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use clawft_plugin::error::PluginError;
use clawft_plugin::message::MessagePayload;
use clawft_plugin::traits::{ChannelAdapter, ChannelAdapterHost};
use clawft_types::error::ChannelError;
use clawft_types::event::OutboundMessage;

use crate::plugin_host::ChannelAdapterHostBridge;
use crate::traits::{
    Channel, ChannelFactory, ChannelHost, ChannelMetadata, ChannelStatus, MessageId,
};

use super::client::TelegramClient;

/// Default long-poll timeout in seconds for `getUpdates`.
const DEFAULT_POLL_TIMEOUT_SECS: u64 = 30;

/// Default delay between poll cycles in seconds.
///
/// `getUpdates` already blocks server-side for `DEFAULT_POLL_TIMEOUT_SECS`
/// (30s) before returning, and `tokio::select!` yields cooperatively
/// between iterations. The extra inter-poll sleep adds latency for
/// inbound messages without buying additional fairness, so the default
/// is `0` (no extra sleep). The field remains configurable for operators
/// who want explicit back-pressure on tight error/retry loops; the
/// `if self.poll_interval_secs > 0` guard skips the sleep entirely
/// when set to `0`.
const DEFAULT_POLL_INTERVAL_SECS: u64 = 0;

/// Delay before retrying after an error, in seconds.
const ERROR_RETRY_DELAY_SECS: u64 = 5;

/// Telegram Bot channel implementation.
///
/// Connects to the Telegram Bot API using long polling. Inbound text
/// messages are forwarded to the host pipeline; outbound messages are
/// sent via the `sendMessage` API.
///
/// Implements both:
/// - [`ChannelAdapter`] -- unified plugin path (C7 / WEFT-170)
/// - [`Channel`] -- legacy path used by [`crate::host::PluginHost`]
///
/// # Configuration
///
/// Created via [`TelegramChannelFactory`] from a JSON config object:
///
/// ```json
/// {
///   "token": "123456:ABC-DEF",
///   "allowed_users": ["12345", "67890"]
/// }
/// ```
///
/// When `allowed_users` is empty or absent, all users are permitted.
pub struct TelegramChannel {
    /// HTTP client for the Telegram Bot API.
    client: TelegramClient,
    /// Current lifecycle status.
    status: Arc<RwLock<ChannelStatus>>,
    /// Offset for the next `getUpdates` call (update_id + 1).
    offset: AtomicI64,
    /// Allow-list of user IDs. Empty means everyone is allowed.
    allowed_users: Vec<String>,
    /// Seconds to wait between poll cycles.
    poll_interval_secs: u64,
}

impl TelegramChannel {
    /// Create a new Telegram channel with the given bot token and allow-list.
    pub fn new(token: String, allowed_users: Vec<String>) -> Self {
        Self {
            client: TelegramClient::new(token),
            status: Arc::new(RwLock::new(ChannelStatus::Stopped)),
            offset: AtomicI64::new(0),
            allowed_users,
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
        }
    }

    /// Create a channel with a custom [`TelegramClient`] (for testing).
    #[cfg(test)]
    pub fn with_client(client: TelegramClient, allowed_users: Vec<String>) -> Self {
        Self {
            client,
            status: Arc::new(RwLock::new(ChannelStatus::Stopped)),
            offset: AtomicI64::new(0),
            allowed_users,
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
        }
    }

    /// Set status under the write lock.
    async fn set_status(&self, status: ChannelStatus) {
        *self.status.write().await = status;
    }

    /// Whether `sender_id` is permitted (empty allow-list = everyone).
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        self.allowed_users.is_empty() || self.allowed_users.iter().any(|id| id == sender_id)
    }

    /// Current lifecycle status (non-blocking read).
    pub fn status(&self) -> ChannelStatus {
        self.status
            .try_read()
            .map(|s| s.clone())
            .unwrap_or(ChannelStatus::Stopped)
    }

    /// Process a single update, delivering any text message via
    /// [`ChannelAdapterHost`].
    pub(crate) async fn process_update(
        &self,
        update: &super::types::Update,
        host: &Arc<dyn ChannelAdapterHost>,
    ) -> Result<(), PluginError> {
        let Some(ref msg) = update.message else {
            debug!(update_id = update.update_id, "skipping non-message update");
            return Ok(());
        };

        let Some(ref text) = msg.text else {
            debug!(
                update_id = update.update_id,
                "skipping message without text"
            );
            return Ok(());
        };

        let sender_id = msg
            .from
            .as_ref()
            .map(|u| u.id.to_string())
            .unwrap_or_default();

        if !self.is_allowed(&sender_id) {
            warn!(
                sender_id = %sender_id,
                chat_id = msg.chat.id,
                "message from disallowed user, ignoring"
            );
            return Ok(());
        }

        let chat_id = msg.chat.id.to_string();

        let mut metadata = HashMap::new();
        metadata.insert(
            "message_id".into(),
            serde_json::Value::Number(msg.message_id.into()),
        );
        if let Some(ref from) = msg.from {
            metadata.insert("first_name".into(), from.first_name.clone().into());
            if let Some(ref username) = from.username {
                metadata.insert("username".into(), username.clone().into());
            }
        }
        metadata.insert("chat_type".into(), msg.chat.chat_type.clone().into());

        // WEFT-162: signal that the sender passed an explicit allow_from
        // check so the permission resolver can promote them from
        // zero-trust to user level. Mirrors the Discord channel; only
        // emitted when `allowed_users` is non-empty AND contains the
        // sender (i.e. a real positive match, not "everyone allowed
        // because the list is empty").
        if !self.allowed_users.is_empty() && self.allowed_users.iter().any(|id| id == &sender_id) {
            metadata.insert("allow_from_match".into(), serde_json::Value::Bool(true));
        }

        host.deliver_inbound(
            "telegram",
            &sender_id,
            &chat_id,
            MessagePayload::text(text.clone()),
            metadata,
        )
        .await
    }

    /// Shared long-poll loop used by both trait surfaces.
    async fn run_poll_loop(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        self.set_status(ChannelStatus::Starting).await;

        // Verify the bot token.
        let me = self.client.get_me().await.map_err(|e| {
            error!(error = %e, "failed to verify Telegram bot token");
            channel_err_to_plugin(e)
        })?;

        info!(
            bot_id = me.id,
            bot_name = %me.first_name,
            "Telegram bot authenticated"
        );

        self.set_status(ChannelStatus::Running).await;

        // Long-polling loop.
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("Telegram channel received cancellation");
                    break;
                }
                result = self.client.get_updates(
                    Some(self.offset.load(Ordering::SeqCst)),
                    DEFAULT_POLL_TIMEOUT_SECS,
                ) => {
                    match result {
                        Ok(updates) => {
                            for update in &updates {
                                if let Err(e) = self.process_update(update, &host).await {
                                    error!(
                                        update_id = update.update_id,
                                        error = %e,
                                        "failed to process update"
                                    );
                                }
                                // Advance offset past this update regardless
                                // of whether processing succeeded.
                                self.offset.store(
                                    update.update_id + 1,
                                    Ordering::SeqCst,
                                );
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "getUpdates failed");
                            self.set_status(ChannelStatus::Error(e.to_string())).await;

                            // Wait before retrying, but still respect cancellation.
                            tokio::select! {
                                _ = cancel.cancelled() => {
                                    info!("Telegram channel cancelled during error backoff");
                                    break;
                                }
                                _ = tokio::time::sleep(
                                    std::time::Duration::from_secs(ERROR_RETRY_DELAY_SECS)
                                ) => {}
                            }

                            self.set_status(ChannelStatus::Running).await;
                        }
                    }
                }
            }

            // Brief yield between polls so other tasks can run.
            if self.poll_interval_secs > 0 {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(
                        std::time::Duration::from_secs(self.poll_interval_secs)
                    ) => {}
                }
            }
        }

        self.set_status(ChannelStatus::Stopped).await;
        info!("Telegram channel stopped");
        Ok(())
    }

    /// Send text (and optional reply_to) via the Bot API.
    async fn send_text(
        &self,
        chat_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<String, ChannelError> {
        let chat_id_i64: i64 = chat_id.parse().map_err(|_| {
            ChannelError::SendFailed(format!("invalid chat_id '{chat_id}': expected i64"))
        })?;

        let reply_to_i64: Option<i64> = reply_to
            .map(|id| {
                id.parse::<i64>().map_err(|_| {
                    ChannelError::SendFailed(format!("invalid reply_to '{id}': expected i64"))
                })
            })
            .transpose()?;

        let sent = self
            .client
            .send_message(chat_id_i64, content, reply_to_i64)
            .await?;

        Ok(sent.message_id.to_string())
    }
}

// ---------------------------------------------------------------------------
// ChannelAdapter (primary C7 surface)
// ---------------------------------------------------------------------------

#[async_trait]
impl ChannelAdapter for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    fn display_name(&self) -> &str {
        "Telegram Bot"
    }

    fn supports_threads(&self) -> bool {
        false
    }

    fn supports_media(&self) -> bool {
        true
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        self.run_poll_loop(host, cancel).await
    }

    async fn send(&self, target: &str, payload: &MessagePayload) -> Result<String, PluginError> {
        let content = payload.as_text().ok_or_else(|| {
            PluginError::ExecutionFailed("telegram: only text payloads supported".into())
        })?;
        self.send_text(target, content, None)
            .await
            .map_err(channel_err_to_plugin)
    }
}

// ---------------------------------------------------------------------------
// Channel (legacy PluginHost / gateway surface)
// ---------------------------------------------------------------------------

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    fn metadata(&self) -> ChannelMetadata {
        ChannelMetadata {
            name: "telegram".into(),
            display_name: "Telegram Bot".into(),
            supports_threads: false,
            supports_media: true,
        }
    }

    fn status(&self) -> ChannelStatus {
        TelegramChannel::status(self)
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        TelegramChannel::is_allowed(self, sender_id)
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelHost>,
        cancel: CancellationToken,
    ) -> Result<(), ChannelError> {
        // Bridge legacy ChannelHost → ChannelAdapterHost so the shared
        // poll loop (and MessagePayload inbound path) is the single source
        // of truth.
        let adapter_host: Arc<dyn ChannelAdapterHost> =
            Arc::new(ChannelAdapterHostBridge::new(host));
        self.run_poll_loop(adapter_host, cancel)
            .await
            .map_err(plugin_err_to_channel)
    }

    async fn send(&self, msg: &OutboundMessage) -> Result<MessageId, ChannelError> {
        let id = self
            .send_text(&msg.chat_id, &msg.content, msg.reply_to.as_deref())
            .await?;
        Ok(MessageId(id))
    }
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

fn channel_err_to_plugin(e: ChannelError) -> PluginError {
    PluginError::ExecutionFailed(e.to_string())
}

fn plugin_err_to_channel(e: PluginError) -> ChannelError {
    ChannelError::Other(e.to_string())
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Factory for creating [`TelegramChannel`] instances from JSON config.
///
/// Expected config shape:
///
/// ```json
/// {
///   "token": "123456:ABC-DEF",
///   "allowed_users": ["12345", "67890"]
/// }
/// ```
pub struct TelegramChannelFactory;

impl ChannelFactory for TelegramChannelFactory {
    fn channel_name(&self) -> &str {
        "telegram"
    }

    fn build(&self, config: &serde_json::Value) -> Result<Arc<dyn Channel>, ChannelError> {
        let mut token = config
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        // Resolve token: explicit value > token_env env var > error
        if token.is_empty() {
            if let Some(env_var) = config.get("token_env").and_then(|v| v.as_str()) {
                match std::env::var(env_var) {
                    Ok(val) if !val.is_empty() => token = val,
                    _ => {
                        return Err(ChannelError::Other(format!(
                            "telegram token_env '{env_var}' is not set or empty"
                        )));
                    }
                }
            } else {
                return Err(ChannelError::Other(
                    "missing 'token' (or 'token_env') in telegram config".into(),
                ));
            }
        }

        let allowed_users: Vec<String> = config
            .get("allowed_users")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok(Arc::new(TelegramChannel::new(token, allowed_users)))
    }
}
