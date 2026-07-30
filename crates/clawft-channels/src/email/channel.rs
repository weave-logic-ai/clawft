//! Email channel adapter implementation.
//!
//! Real IMAP poll + SMTP send behind the `email` feature (WEFT-154).
//!
//! # Runtime
//!
//! - `start()` validates config, then enters an interval-based poll loop.
//!   Each tick calls [`ImapBackend::fetch_unseen`], delivers each message
//!   via [`ChannelAdapterHost::deliver_inbound`], and dedupes by
//!   Message-ID. Transport errors use exponential backoff (1s → 60s);
//!   cancellation is always respected via `tokio::select!`.
//! - `send()` builds a MIME message and submits it through
//!   [`SmtpBackend`]. The returned id is the Message-ID header placed
//!   on the wire (SMTP does not echo Message-ID in the server reply).
//!
//! # Auth / TLS
//!
//! Password auth supports inline [`SecretString`] or `password_env`.
//! IMAP: implicit TLS (`imap_use_tls`, default port 993) or STARTTLS.
//! SMTP: STARTTLS (default port 587) or implicit TLS (port 465).
//!
//! # Test injection
//!
//! [`EmailChannelAdapter::with_backends`] injects mock IMAP/SMTP so unit
//! tests exercise the poll loop and send path without network I/O.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use clawft_plugin::error::PluginError;
use clawft_plugin::message::MessagePayload;
use clawft_plugin::traits::{ChannelAdapter, ChannelAdapterHost};

use super::imap_client::RealImapBackend;
use super::smtp_client::RealSmtpBackend;
use super::transport::{SharedImap, SharedSmtp};
use super::types::{EmailAdapterConfig, ParsedEmail};

/// Initial reconnect backoff after a transport error.
const BACKOFF_INITIAL: Duration = Duration::from_secs(1);
/// Cap for exponential reconnect backoff.
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Email channel adapter.
///
/// Connects to an IMAP server to receive email messages and uses SMTP
/// to send replies. Implements the [`ChannelAdapter`] plugin trait.
pub struct EmailChannelAdapter {
    config: EmailAdapterConfig,
    imap: SharedImap,
    smtp: SharedSmtp,
    /// In-process Message-ID dedupe set (guards redelivery within a run).
    seen_ids: Mutex<HashSet<String>>,
}

impl EmailChannelAdapter {
    /// Create a new email channel adapter with real IMAP + SMTP backends.
    pub fn new(config: EmailAdapterConfig) -> Self {
        let imap: SharedImap = Arc::new(RealImapBackend::new(config.clone()));
        let smtp: SharedSmtp = Arc::new(RealSmtpBackend::new(config.clone()));
        Self::with_backends(config, imap, smtp)
    }

    /// Create an adapter with caller-supplied backends (tests / harness).
    pub fn with_backends(config: EmailAdapterConfig, imap: SharedImap, smtp: SharedSmtp) -> Self {
        Self {
            config,
            imap,
            smtp,
            seen_ids: Mutex::new(HashSet::new()),
        }
    }

    /// Borrow the adapter config.
    pub fn config(&self) -> &EmailAdapterConfig {
        &self.config
    }

    /// Check if a sender email is in the allow list.
    ///
    /// Returns `true` when the allow list is empty (everyone allowed)
    /// or when `sender` appears in the list (case-insensitive).
    pub fn is_sender_allowed(&self, sender: &str) -> bool {
        if self.config.allowed_senders.is_empty() {
            return true;
        }
        let sender_lower = sender.to_lowercase();
        self.config
            .allowed_senders
            .iter()
            .any(|s| s.to_lowercase() == sender_lower)
    }

    /// Build an inbound metadata map from a parsed email.
    pub fn build_metadata(email: &ParsedEmail) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();
        metadata.insert("subject".to_string(), serde_json::json!(email.subject));
        metadata.insert(
            "message_id".to_string(),
            serde_json::json!(email.message_id),
        );
        metadata.insert("to".to_string(), serde_json::json!(email.to));
        if let Some(ref reply_to) = email.in_reply_to {
            metadata.insert("in_reply_to".to_string(), serde_json::json!(reply_to));
        }
        metadata
    }

    /// Deliver a parsed email to the host pipeline.
    pub async fn deliver_email(
        &self,
        email: &ParsedEmail,
        host: &Arc<dyn ChannelAdapterHost>,
    ) -> Result<(), PluginError> {
        if !self.is_sender_allowed(&email.from) {
            warn!(
                sender = %email.from,
                "email from disallowed sender, ignoring"
            );
            return Ok(());
        }

        // Truncate body to max_body_chars (backends may already truncate).
        let body = if email.body.chars().count() > self.config.max_body_chars {
            let truncated: String = email
                .body
                .chars()
                .take(self.config.max_body_chars)
                .collect();
            format!("{truncated}\n\n[truncated]")
        } else {
            email.body.clone()
        };

        let content = format!("Subject: {}\n\n{}", email.subject, body);
        let metadata = Self::build_metadata(email);

        host.deliver_inbound(
            "email",
            &email.from,
            &email.from,
            MessagePayload::text(content),
            metadata,
        )
        .await
    }

    /// One poll cycle: fetch unseen, dedupe, deliver.
    async fn poll_once(&self, host: &Arc<dyn ChannelAdapterHost>) -> Result<usize, PluginError> {
        let emails = self.imap.fetch_unseen().await?;
        let mut delivered = 0usize;
        for email in emails {
            {
                let mut seen = self.seen_ids.lock().await;
                if !seen.insert(email.message_id.clone()) {
                    debug!(
                        message_id = %email.message_id,
                        "email: skipping already-seen Message-ID"
                    );
                    continue;
                }
            }
            if let Err(e) = self.deliver_email(&email, host).await {
                error!(
                    message_id = %email.message_id,
                    error = %e,
                    "email: deliver_inbound failed"
                );
                // Allow retry on next poll if delivery failed.
                self.seen_ids.lock().await.remove(&email.message_id);
            } else {
                delivered += 1;
            }
        }
        Ok(delivered)
    }

    fn validate_start(&self) -> Result<(), PluginError> {
        if self.config.imap_host.is_empty() {
            return Err(PluginError::LoadFailed(
                "email adapter: imap_host is required".into(),
            ));
        }
        if self.config.email_address.is_empty() {
            return Err(PluginError::LoadFailed(
                "email adapter: email_address is required".into(),
            ));
        }
        Ok(())
    }

    /// Derive a subject for outbound replies.
    fn outbound_subject(target: &str) -> String {
        // Prefer threading via In-Reply-To when available; for plain
        // outbound we use a generic subject tagged with the target.
        let _ = target;
        "Message from clawft".to_string()
    }
}

#[async_trait]
impl ChannelAdapter for EmailChannelAdapter {
    fn name(&self) -> &str {
        "email"
    }

    fn display_name(&self) -> &str {
        "Email (IMAP/SMTP)"
    }

    fn supports_threads(&self) -> bool {
        true // Email threading via In-Reply-To / References headers
    }

    fn supports_media(&self) -> bool {
        false // Attachment support is a future enhancement
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        info!(
            imap_host = %self.config.imap_host,
            mailbox = %self.config.mailbox,
            poll_interval_secs = self.config.poll_interval_secs,
            "email channel adapter starting (IMAP poll + SMTP send)"
        );
        self.validate_start()?;

        let poll_interval = Duration::from_secs(self.config.poll_interval_secs.max(1));
        let mut interval = tokio::time::interval(poll_interval);
        // Skip the immediate first tick so we don't hammer the server
        // on start; still poll once after a short grace if cancelled
        // mid-wait.
        interval.tick().await;

        let mut backoff = BACKOFF_INITIAL;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("email channel adapter shutting down");
                    return Ok(());
                }
                _ = interval.tick() => {
                    match self.poll_once(&host).await {
                        Ok(n) => {
                            if n > 0 {
                                info!(delivered = n, "email poll delivered messages");
                            } else {
                                debug!(
                                    mailbox = %self.config.mailbox,
                                    "email poll: no new messages"
                                );
                            }
                            backoff = BACKOFF_INITIAL;
                        }
                        Err(e) => {
                            error!(
                                error = %e,
                                backoff_secs = backoff.as_secs(),
                                "email poll failed; reconnecting with backoff"
                            );
                            tokio::select! {
                                _ = cancel.cancelled() => {
                                    info!("email channel adapter shutting down during backoff");
                                    return Ok(());
                                }
                                _ = tokio::time::sleep(backoff) => {}
                            }
                            backoff = (backoff * 2).min(BACKOFF_MAX);
                        }
                    }
                }
            }
        }
    }

    async fn send(&self, target: &str, payload: &MessagePayload) -> Result<String, PluginError> {
        let content = match payload.as_text() {
            Some(text) => text,
            None => {
                return Err(PluginError::ExecutionFailed(
                    "email adapter only supports text payloads".into(),
                ));
            }
        };

        if self.config.smtp_host.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "email adapter: smtp_host is not configured".into(),
            ));
        }

        // Optional "Subject: …\n\nbody" convention for structured replies.
        let (subject, body) = split_subject_body(content);
        let subject = subject.unwrap_or_else(|| Self::outbound_subject(target));

        info!(
            to = %target,
            smtp_host = %self.config.smtp_host,
            "sending email via SMTP"
        );
        debug!(content_len = body.len(), "email content");

        self.smtp
            .send(target, &subject, body, /* in_reply_to */ None)
            .await
    }
}

/// If the payload starts with `Subject: …\n\n`, peel it off.
fn split_subject_body(content: &str) -> (Option<String>, &str) {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("Subject:") {
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        if let Some((subj, body)) = rest.split_once("\n\n") {
            return (Some(subj.trim().to_string()), body);
        }
        if let Some((subj, body)) = rest.split_once('\n') {
            return (Some(subj.trim().to_string()), body);
        }
    }
    (None, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use clawft_plugin::traits::ChannelAdapterHost;
    use tokio::sync::Mutex;

    use super::super::transport::{MockImapBackend, MockSmtpBackend};
    use clawft_types::secret::SecretString;
    use super::super::types::EmailAuth;

    // -- Mock host --

    struct MockAdapterHost {
        messages: Mutex<Vec<(String, String, String, MessagePayload)>>,
    }

    impl MockAdapterHost {
        fn new() -> Self {
            Self {
                messages: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl ChannelAdapterHost for MockAdapterHost {
        async fn deliver_inbound(
            &self,
            channel: &str,
            sender_id: &str,
            chat_id: &str,
            payload: MessagePayload,
            _metadata: HashMap<String, serde_json::Value>,
        ) -> Result<(), PluginError> {
            self.messages.lock().await.push((
                channel.into(),
                sender_id.into(),
                chat_id.into(),
                payload,
            ));
            Ok(())
        }
    }

    fn make_config() -> EmailAdapterConfig {
        EmailAdapterConfig {
            imap_host: "imap.test.com".into(),
            smtp_host: "smtp.test.com".into(),
            email_address: "bot@test.com".into(),
            auth: EmailAuth::Password {
                username: "bot@test.com".into(),
                password: SecretString::new("test-password"),
                password_env: None,
            },
            poll_interval_secs: 1,
            ..Default::default()
        }
    }

    fn mock_adapter(
        config: EmailAdapterConfig,
    ) -> (
        EmailChannelAdapter,
        Arc<MockImapBackend>,
        Arc<MockSmtpBackend>,
    ) {
        let imap = Arc::new(MockImapBackend::new());
        let smtp = Arc::new(MockSmtpBackend::new());
        let adapter = EmailChannelAdapter::with_backends(
            config,
            imap.clone() as SharedImap,
            smtp.clone() as SharedSmtp,
        );
        (adapter, imap, smtp)
    }

    // -- Trait method tests --

    #[test]
    fn name_is_email() {
        let (adapter, _, _) = mock_adapter(make_config());
        assert_eq!(adapter.name(), "email");
    }

    #[test]
    fn display_name() {
        let (adapter, _, _) = mock_adapter(make_config());
        assert_eq!(adapter.display_name(), "Email (IMAP/SMTP)");
    }

    #[test]
    fn supports_threads() {
        let (adapter, _, _) = mock_adapter(make_config());
        assert!(adapter.supports_threads());
    }

    #[test]
    fn supports_media_false() {
        let (adapter, _, _) = mock_adapter(make_config());
        assert!(!adapter.supports_media());
    }

    // -- Sender filtering --

    #[test]
    fn empty_allow_list_allows_everyone() {
        let (adapter, _, _) = mock_adapter(make_config());
        assert!(adapter.is_sender_allowed("anyone@example.com"));
        assert!(adapter.is_sender_allowed(""));
    }

    #[test]
    fn allow_list_filters_senders() {
        let mut config = make_config();
        config.allowed_senders = vec!["boss@company.com".into(), "Admin@Corp.com".into()];
        let (adapter, _, _) = mock_adapter(config);

        assert!(adapter.is_sender_allowed("boss@company.com"));
        assert!(adapter.is_sender_allowed("BOSS@COMPANY.COM")); // case insensitive
        assert!(adapter.is_sender_allowed("admin@corp.com"));
        assert!(!adapter.is_sender_allowed("stranger@evil.com"));
    }

    // -- Email delivery --

    #[tokio::test]
    async fn deliver_email_sends_to_host() {
        let (adapter, _, _) = mock_adapter(make_config());
        let mock_host = Arc::new(MockAdapterHost::new());
        let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

        let email = ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "Help request".into(),
            body: "I need help with my account".into(),
            message_id: "<msg-001@example.com>".into(),
            in_reply_to: None,
        };

        adapter.deliver_email(&email, &host).await.unwrap();

        let msgs = mock_host.messages.lock().await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].0, "email");
        assert_eq!(msgs[0].1, "alice@example.com");
        let text = msgs[0].3.as_text().unwrap();
        assert!(text.contains("Help request"));
        assert!(text.contains("I need help with my account"));
    }

    #[tokio::test]
    async fn deliver_email_skips_disallowed_sender() {
        let mut config = make_config();
        config.allowed_senders = vec!["boss@company.com".into()];
        let (adapter, _, _) = mock_adapter(config);
        let mock_host = Arc::new(MockAdapterHost::new());
        let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

        let email = ParsedEmail {
            from: "stranger@evil.com".into(),
            to: "bot@test.com".into(),
            subject: "Spam".into(),
            body: "Buy now!".into(),
            message_id: "<spam@evil.com>".into(),
            in_reply_to: None,
        };

        adapter.deliver_email(&email, &host).await.unwrap();
        assert!(mock_host.messages.lock().await.is_empty());
    }

    #[tokio::test]
    async fn deliver_email_truncates_long_body() {
        let mut config = make_config();
        config.max_body_chars = 20;
        let (adapter, _, _) = mock_adapter(config);
        let mock_host = Arc::new(MockAdapterHost::new());
        let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();

        let email = ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "Long email".into(),
            body: "A".repeat(100),
            message_id: "<long@example.com>".into(),
            in_reply_to: None,
        };

        adapter.deliver_email(&email, &host).await.unwrap();

        let msgs = mock_host.messages.lock().await;
        let text = msgs[0].3.as_text().unwrap();
        assert!(text.contains("[truncated]"));
    }

    // -- Metadata --

    #[test]
    fn build_metadata_includes_subject_and_message_id() {
        let email = ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "Test subject".into(),
            body: "body text".into(),
            message_id: "<msg-001@test.com>".into(),
            in_reply_to: None,
        };

        let metadata = EmailChannelAdapter::build_metadata(&email);
        assert_eq!(metadata["subject"], serde_json::json!("Test subject"));
        assert_eq!(
            metadata["message_id"],
            serde_json::json!("<msg-001@test.com>")
        );
        assert_eq!(metadata["to"], serde_json::json!("bot@test.com"));
        assert!(!metadata.contains_key("in_reply_to"));
    }

    #[test]
    fn build_metadata_includes_in_reply_to() {
        let email = ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "Re: Test".into(),
            body: "reply".into(),
            message_id: "<msg-002@test.com>".into(),
            in_reply_to: Some("<msg-001@test.com>".into()),
        };

        let metadata = EmailChannelAdapter::build_metadata(&email);
        assert_eq!(
            metadata["in_reply_to"],
            serde_json::json!("<msg-001@test.com>")
        );
    }

    // -- Send via mock SMTP --

    #[tokio::test]
    async fn send_text_payload_via_smtp_backend() {
        let (adapter, _, smtp) = mock_adapter(make_config());
        let payload = MessagePayload::text("Hello from the bot!");
        let result = adapter.send("user@example.com", &payload).await;
        assert!(result.is_ok());
        let msg_id = result.unwrap();
        assert!(msg_id.starts_with('<'));
        assert!(msg_id.ends_with('>'));

        let sent = smtp.sent.lock().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "user@example.com");
        assert_eq!(sent[0].body, "Hello from the bot!");
        assert_eq!(sent[0].message_id, msg_id);
    }

    #[tokio::test]
    async fn send_with_subject_prefix() {
        let (adapter, _, smtp) = mock_adapter(make_config());
        let payload = MessagePayload::text("Subject: Re: Help\n\nHere is the fix.");
        let msg_id = adapter.send("user@example.com", &payload).await.unwrap();
        let sent = smtp.sent.lock().await;
        assert_eq!(sent[0].subject, "Re: Help");
        assert_eq!(sent[0].body, "Here is the fix.");
        assert_eq!(sent[0].message_id, msg_id);
    }

    #[tokio::test]
    async fn send_non_text_payload_fails() {
        let (adapter, _, _) = mock_adapter(make_config());
        let payload = MessagePayload::structured(serde_json::json!({"key": "value"}));
        let result = adapter.send("user@example.com", &payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_without_smtp_host_fails() {
        let mut config = make_config();
        config.smtp_host = String::new();
        let (adapter, _, _) = mock_adapter(config);
        let payload = MessagePayload::text("test");
        let result = adapter.send("user@example.com", &payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_smtp_error_propagates() {
        let (adapter, _, smtp) = mock_adapter(make_config());
        *smtp.fail_with.lock().await = Some("smtp down".into());
        let err = adapter
            .send("user@example.com", &MessagePayload::text("x"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("smtp down"));
    }

    // -- Start validation --

    #[tokio::test]
    async fn start_without_imap_host_fails() {
        let mut config = make_config();
        config.imap_host = String::new();
        let (adapter, _, _) = mock_adapter(config);
        let host = Arc::new(MockAdapterHost::new());
        let cancel = CancellationToken::new();

        let result = adapter.start(host, cancel).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("imap_host"));
    }

    #[tokio::test]
    async fn start_without_email_address_fails() {
        let mut config = make_config();
        config.email_address = String::new();
        let (adapter, _, _) = mock_adapter(config);
        let host = Arc::new(MockAdapterHost::new());
        let cancel = CancellationToken::new();

        let result = adapter.start(host, cancel).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("email_address"));
    }

    #[tokio::test]
    async fn start_shuts_down_on_cancel() {
        let mut config = make_config();
        config.poll_interval_secs = 60;
        let (adapter, _, _) = mock_adapter(config);
        let host = Arc::new(MockAdapterHost::new());
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { adapter.start(host, cancel_clone).await });

        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // -- Poll loop integration against mock IMAP --

    #[tokio::test]
    async fn poll_loop_delivers_inbound_from_mock_imap() {
        let mut config = make_config();
        config.poll_interval_secs = 1;
        let (adapter, imap, _) = mock_adapter(config);

        imap.push_batch(vec![ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "Inbound".into(),
            body: "please help".into(),
            message_id: "<in-1@example.com>".into(),
            in_reply_to: None,
        }]);

        let mock_host = Arc::new(MockAdapterHost::new());
        let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { adapter.start(host, cancel_clone).await });

        // Wait for at least one poll interval tick.
        tokio::time::sleep(Duration::from_millis(1200)).await;
        cancel.cancel();
        let result = handle.await.unwrap();
        assert!(result.is_ok());

        let msgs = mock_host.messages.lock().await;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].1, "alice@example.com");
        assert!(msgs[0].3.as_text().unwrap().contains("please help"));
    }

    #[tokio::test]
    async fn poll_loop_dedupes_message_ids() {
        let mut config = make_config();
        config.poll_interval_secs = 1;
        let (adapter, imap, _) = mock_adapter(config);

        let email = ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "Dup".into(),
            body: "once".into(),
            message_id: "<dup@example.com>".into(),
            in_reply_to: None,
        };
        // Same message in two successive batches.
        imap.push_batch(vec![email.clone()]);
        imap.push_batch(vec![email]);

        let mock_host = Arc::new(MockAdapterHost::new());
        let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { adapter.start(host, cancel_clone).await });
        tokio::time::sleep(Duration::from_millis(2500)).await;
        cancel.cancel();
        handle.await.unwrap().unwrap();

        let msgs = mock_host.messages.lock().await;
        assert_eq!(msgs.len(), 1, "duplicate Message-ID must not redeliver");
    }

    #[tokio::test]
    async fn poll_loop_backoff_on_imap_error_then_recovers() {
        let mut config = make_config();
        config.poll_interval_secs = 1;
        let (adapter, imap, _) = mock_adapter(config);

        *imap.fail_with.lock().await = Some("connection reset".into());
        imap.push_batch(vec![ParsedEmail {
            from: "alice@example.com".into(),
            to: "bot@test.com".into(),
            subject: "After error".into(),
            body: "recovered".into(),
            message_id: "<rec@example.com>".into(),
            in_reply_to: None,
        }]);

        let mock_host = Arc::new(MockAdapterHost::new());
        let host: Arc<dyn ChannelAdapterHost> = mock_host.clone();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let handle = tokio::spawn(async move { adapter.start(host, cancel_clone).await });
        // First tick fails (backoff 1s), second tick should deliver.
        tokio::time::sleep(Duration::from_millis(3500)).await;
        cancel.cancel();
        handle.await.unwrap().unwrap();

        let msgs = mock_host.messages.lock().await;
        assert!(
            !msgs.is_empty(),
            "adapter should recover after IMAP error and deliver"
        );
        assert!(msgs[0].3.as_text().unwrap().contains("recovered"));
    }

    #[test]
    fn split_subject_body_parses_prefix() {
        let (s, b) = split_subject_body("Subject: Hello\n\nWorld");
        assert_eq!(s.as_deref(), Some("Hello"));
        assert_eq!(b, "World");
    }

    #[test]
    fn split_subject_body_no_prefix() {
        let (s, b) = split_subject_body("just body");
        assert!(s.is_none());
        assert_eq!(b, "just body");
    }
}
