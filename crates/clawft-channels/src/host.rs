//! [`PluginHost`] -- manages channel plugin lifecycle.
//!
//! The plugin host is responsible for:
//!
//! - Registering [`ChannelFactory`] instances
//! - Creating channel instances from configuration
//! - Starting and stopping channels (each in its own tokio task)
//! - Routing outbound messages to the correct channel

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::traits::*;
use clawft_types::error::ChannelError;
use clawft_types::event::OutboundMessage;

/// Manages channel plugins: registration, lifecycle, and message routing.
///
/// Channels are created from [`ChannelFactory`] instances, started in
/// individual tokio tasks with their own [`CancellationToken`], and stopped
/// gracefully on shutdown.
pub struct PluginHost {
    /// Registered channel factories, keyed by channel name.
    factories: RwLock<HashMap<String, Arc<dyn ChannelFactory>>>,
    /// Active channel instances, keyed by channel name.
    channels: RwLock<HashMap<String, Arc<dyn Channel>>>,
    /// Cancellation tokens for running channel tasks.
    cancel_tokens: RwLock<HashMap<String, CancellationToken>>,
    /// Join handles for running channel tasks.
    task_handles: RwLock<HashMap<String, JoinHandle<()>>>,
    /// The host interface provided to channel plugins.
    host_impl: Arc<dyn ChannelHost>,
}

impl PluginHost {
    /// Create a new `PluginHost` with the given [`ChannelHost`] implementation.
    pub fn new(host: Arc<dyn ChannelHost>) -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            cancel_tokens: RwLock::new(HashMap::new()),
            task_handles: RwLock::new(HashMap::new()),
            host_impl: host,
        }
    }

    /// Register a channel factory.
    ///
    /// If a factory with the same channel name is already registered,
    /// it will be replaced.
    pub async fn register_factory(&self, factory: Arc<dyn ChannelFactory>) {
        let name = factory.channel_name().to_owned();
        info!(channel = %name, "registering channel factory");
        self.factories.write().await.insert(name, factory);
    }

    /// Initialize a channel from its JSON config section.
    ///
    /// Looks up the factory by `name`, invokes [`ChannelFactory::build`],
    /// and stores the resulting channel instance. The channel is **not**
    /// started -- call [`start_channel`](PluginHost::start_channel) or
    /// [`start_all`](PluginHost::start_all) afterwards.
    pub async fn init_channel(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<(), ChannelError> {
        let factories = self.factories.read().await;
        let factory = factories
            .get(name)
            .ok_or_else(|| ChannelError::NotFound(name.to_owned()))?;

        let channel = factory.build(config)?;
        info!(channel = %name, "channel initialized");
        self.channels.write().await.insert(name.to_owned(), channel);
        Ok(())
    }

    /// Start all initialized channels.
    ///
    /// Each channel is started in its own tokio task. Returns a list of
    /// `(channel_name, result)` pairs indicating success or failure for
    /// each channel's initial setup.
    pub async fn start_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
        let channels = self.channels.read().await;
        let names: Vec<String> = channels.keys().cloned().collect();
        drop(channels);

        let mut results = Vec::with_capacity(names.len());
        for name in names {
            let result = self.start_channel(&name).await;
            results.push((name, result));
        }
        results
    }

    /// Stop all running channels gracefully.
    ///
    /// Cancels each channel's token and awaits task completion. Returns
    /// a list of `(channel_name, result)` pairs.
    pub async fn stop_all(&self) -> Vec<(String, Result<(), ChannelError>)> {
        let tokens = self.cancel_tokens.read().await;
        let names: Vec<String> = tokens.keys().cloned().collect();
        drop(tokens);

        let mut results = Vec::with_capacity(names.len());
        for name in names {
            let result = self.stop_channel(&name).await;
            results.push((name, result));
        }
        results
    }

    /// Start a specific channel by name.
    ///
    /// Spawns a tokio task that calls [`Channel::start`] with the host
    /// interface and a cancellation token.
    pub async fn start_channel(&self, name: &str) -> Result<(), ChannelError> {
        let channels = self.channels.read().await;
        let channel = channels
            .get(name)
            .ok_or_else(|| ChannelError::NotFound(name.to_owned()))?
            .clone();
        drop(channels);

        let cancel = CancellationToken::new();
        let host = self.host_impl.clone();
        let channel_name = name.to_owned();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move {
            info!(channel = %channel_name, "starting channel");
            if let Err(e) = channel.start(host, cancel_clone).await {
                error!(channel = %channel_name, error = %e, "channel exited with error");
            } else {
                info!(channel = %channel_name, "channel stopped");
            }
        });

        self.cancel_tokens
            .write()
            .await
            .insert(name.to_owned(), cancel);
        self.task_handles
            .write()
            .await
            .insert(name.to_owned(), handle);

        Ok(())
    }

    /// Stop a specific channel by name.
    ///
    /// Cancels the channel's token and waits for the task to finish.
    pub async fn stop_channel(&self, name: &str) -> Result<(), ChannelError> {
        let token = self
            .cancel_tokens
            .write()
            .await
            .remove(name)
            .ok_or_else(|| ChannelError::NotFound(name.to_owned()))?;

        info!(channel = %name, "stopping channel");
        token.cancel();

        if let Some(handle) = self.task_handles.write().await.remove(name) {
            if let Err(e) = handle.await {
                warn!(channel = %name, error = %e, "channel task panicked");
            }
        }

        Ok(())
    }

    /// Route an outbound message to the appropriate channel.
    pub async fn send_to_channel(
        &self,
        msg: &OutboundMessage,
    ) -> Result<MessageId, ChannelError> {
        let channels = self.channels.read().await;
        let channel = channels
            .get(&msg.channel)
            .ok_or_else(|| ChannelError::NotFound(msg.channel.clone()))?
            .clone();
        drop(channels);

        channel.send(msg).await
    }

    /// Get status of all initialized channels.
    pub async fn get_status(&self) -> HashMap<String, ChannelStatus> {
        let channels = self.channels.read().await;
        channels
            .iter()
            .map(|(name, ch)| (name.clone(), ch.status()))
            .collect()
    }

    /// Get the list of registered factory names.
    pub async fn registered_channels(&self) -> Vec<String> {
        self.factories.read().await.keys().cloned().collect()
    }

    /// Get the list of active (initialized) channel names.
    pub async fn active_channels(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use clawft_types::event::InboundMessage;
    use std::sync::atomic::{AtomicU8, Ordering};

    // ── Mock implementations ─────────────────────────────────────────

    /// Status codes for [`MockChannel`].
    const STATUS_STOPPED: u8 = 0;
    const STATUS_RUNNING: u8 = 1;

    /// A minimal channel that tracks its lifecycle via an atomic status byte.
    struct MockChannel {
        channel_name: String,
        status_byte: AtomicU8,
    }

    impl MockChannel {
        fn new(name: &str) -> Self {
            Self {
                channel_name: name.to_owned(),
                status_byte: AtomicU8::new(STATUS_STOPPED),
            }
        }
    }

    #[async_trait]
    impl Channel for MockChannel {
        fn name(&self) -> &str {
            &self.channel_name
        }

        fn metadata(&self) -> ChannelMetadata {
            ChannelMetadata {
                name: self.channel_name.clone(),
                display_name: format!("Mock {}", self.channel_name),
                supports_threads: false,
                supports_media: false,
            }
        }

        fn status(&self) -> ChannelStatus {
            match self.status_byte.load(Ordering::SeqCst) {
                STATUS_RUNNING => ChannelStatus::Running,
                _ => ChannelStatus::Stopped,
            }
        }

        fn is_allowed(&self, _sender_id: &str) -> bool {
            true
        }

        async fn start(
            &self,
            _host: Arc<dyn ChannelHost>,
            cancel: CancellationToken,
        ) -> Result<(), ChannelError> {
            self.status_byte.store(STATUS_RUNNING, Ordering::SeqCst);
            cancel.cancelled().await;
            self.status_byte.store(STATUS_STOPPED, Ordering::SeqCst);
            Ok(())
        }

        async fn send(&self, _msg: &OutboundMessage) -> Result<MessageId, ChannelError> {
            if self.status_byte.load(Ordering::SeqCst) != STATUS_RUNNING {
                return Err(ChannelError::NotConnected);
            }
            Ok(MessageId("mock-msg-001".into()))
        }
    }

    /// A factory that produces [`MockChannel`] instances.
    struct MockChannelFactory {
        name: String,
    }

    impl MockChannelFactory {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_owned(),
            }
        }
    }

    impl ChannelFactory for MockChannelFactory {
        fn channel_name(&self) -> &str {
            &self.name
        }

        fn build(
            &self,
            _config: &serde_json::Value,
        ) -> Result<Arc<dyn Channel>, ChannelError> {
            Ok(Arc::new(MockChannel::new(&self.name)))
        }
    }

    /// A mock host that collects delivered inbound messages.
    struct MockChannelHost {
        messages: tokio::sync::Mutex<Vec<InboundMessage>>,
    }

    impl MockChannelHost {
        fn new() -> Self {
            Self {
                messages: tokio::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ChannelHost for MockChannelHost {
        async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError> {
            self.messages.lock().await.push(msg);
            Ok(())
        }

        async fn register_command(&self, _cmd: Command) -> Result<(), ChannelError> {
            Ok(())
        }

        async fn publish_inbound(
            &self,
            channel: &str,
            sender_id: &str,
            chat_id: &str,
            content: &str,
            media: Vec<String>,
            metadata: HashMap<String, serde_json::Value>,
        ) -> Result<(), ChannelError> {
            let msg = InboundMessage {
                channel: channel.to_owned(),
                sender_id: sender_id.to_owned(),
                chat_id: chat_id.to_owned(),
                content: content.to_owned(),
                timestamp: chrono::Utc::now(),
                media,
                metadata,
            };
            self.deliver_inbound(msg).await
        }
    }

    // ── Tests ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn register_factory() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        let factory = Arc::new(MockChannelFactory::new("telegram"));
        plugin_host.register_factory(factory).await;

        let registered = plugin_host.registered_channels().await;
        assert_eq!(registered, vec!["telegram"]);
    }

    #[tokio::test]
    async fn register_multiple_factories() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        plugin_host
            .register_factory(Arc::new(MockChannelFactory::new("telegram")))
            .await;
        plugin_host
            .register_factory(Arc::new(MockChannelFactory::new("slack")))
            .await;

        let mut registered = plugin_host.registered_channels().await;
        registered.sort();
        assert_eq!(registered, vec!["slack", "telegram"]);
    }

    #[tokio::test]
    async fn init_channel_with_registered_factory() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        plugin_host
            .register_factory(Arc::new(MockChannelFactory::new("telegram")))
            .await;

        let config = serde_json::json!({"token": "test-token"});
        let result = plugin_host.init_channel("telegram", &config).await;
        assert!(result.is_ok());

        let active = plugin_host.active_channels().await;
        assert_eq!(active, vec!["telegram"]);
    }

    #[tokio::test]
    async fn init_channel_unknown_factory_errors() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        let config = serde_json::json!({});
        let result = plugin_host.init_channel("nonexistent", &config).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, ChannelError::NotFound(_)),
            "expected NotFound, got: {err:?}",
        );
    }

    #[tokio::test]
    async fn start_and_stop_channel() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        plugin_host
            .register_factory(Arc::new(MockChannelFactory::new("mock")))
            .await;

        let config = serde_json::json!({});
        plugin_host.init_channel("mock", &config).await.unwrap();
        plugin_host.start_channel("mock").await.unwrap();

        // Give the spawned task a moment to set status.
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let statuses = plugin_host.get_status().await;
        assert_eq!(statuses.get("mock"), Some(&ChannelStatus::Running));

        plugin_host.stop_channel("mock").await.unwrap();

        // After stop, the task has been awaited so status should revert.
        let statuses = plugin_host.get_status().await;
        assert_eq!(statuses.get("mock"), Some(&ChannelStatus::Stopped));
    }

    #[tokio::test]
    async fn start_all_stop_all() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        for name in ["alpha", "beta"] {
            plugin_host
                .register_factory(Arc::new(MockChannelFactory::new(name)))
                .await;
            plugin_host
                .init_channel(name, &serde_json::json!({}))
                .await
                .unwrap();
        }

        let start_results = plugin_host.start_all().await;
        assert!(start_results.iter().all(|(_, r)| r.is_ok()));

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let statuses = plugin_host.get_status().await;
        assert_eq!(statuses.get("alpha"), Some(&ChannelStatus::Running));
        assert_eq!(statuses.get("beta"), Some(&ChannelStatus::Running));

        let stop_results = plugin_host.stop_all().await;
        assert!(stop_results.iter().all(|(_, r)| r.is_ok()));

        let statuses = plugin_host.get_status().await;
        assert_eq!(statuses.get("alpha"), Some(&ChannelStatus::Stopped));
        assert_eq!(statuses.get("beta"), Some(&ChannelStatus::Stopped));
    }

    #[tokio::test]
    async fn send_to_unknown_channel_errors() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        let msg = OutboundMessage {
            channel: "nonexistent".into(),
            chat_id: "c1".into(),
            content: "hello".into(),
            reply_to: None,
            media: vec![],
            metadata: HashMap::new(),
        };

        let result = plugin_host.send_to_channel(&msg).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChannelError::NotFound(_)));
    }

    #[tokio::test]
    async fn send_to_active_channel() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        plugin_host
            .register_factory(Arc::new(MockChannelFactory::new("mock")))
            .await;
        plugin_host
            .init_channel("mock", &serde_json::json!({}))
            .await
            .unwrap();
        plugin_host.start_channel("mock").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let msg = OutboundMessage {
            channel: "mock".into(),
            chat_id: "c1".into(),
            content: "hello".into(),
            reply_to: None,
            media: vec![],
            metadata: HashMap::new(),
        };

        let result = plugin_host.send_to_channel(&msg).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MessageId("mock-msg-001".into()));

        plugin_host.stop_all().await;
    }

    #[tokio::test]
    async fn get_status_empty_host() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        let statuses = plugin_host.get_status().await;
        assert!(statuses.is_empty());
    }

    #[tokio::test]
    async fn start_nonexistent_channel_errors() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        let result = plugin_host.start_channel("ghost").await;
        assert!(matches!(result.unwrap_err(), ChannelError::NotFound(_)));
    }

    #[tokio::test]
    async fn stop_nonexistent_channel_errors() {
        let host = Arc::new(MockChannelHost::new());
        let plugin_host = PluginHost::new(host);

        let result = plugin_host.stop_channel("ghost").await;
        assert!(matches!(result.unwrap_err(), ChannelError::NotFound(_)));
    }
}
