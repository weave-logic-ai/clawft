//! Pluggable IMAP/SMTP backends for the email channel.
//!
//! Production builds wire [`super::imap_client::RealImapBackend`] and
//! [`super::smtp_client::RealSmtpBackend`]. Unit tests inject
//! [`MockImapBackend`] / [`MockSmtpBackend`] so the poll loop and
//! `send()` path exercise real adapter logic without network I/O.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use clawft_plugin::error::PluginError;

use super::types::ParsedEmail;

/// Fetches unseen messages from an IMAP mailbox.
#[async_trait]
pub trait ImapBackend: Send + Sync {
    /// Connect, authenticate, search UNSEEN, parse RFC822, and return
    /// new messages. Implementations may mark messages `\Seen` after a
    /// successful fetch (or leave that to a follow-up `mark_seen`).
    async fn fetch_unseen(&self) -> Result<Vec<ParsedEmail>, PluginError>;
}

/// Submits outbound MIME messages via SMTP.
#[async_trait]
pub trait SmtpBackend: Send + Sync {
    /// Build a MIME message and submit it. Returns the Message-ID
    /// header value that was placed on the wire (SMTP does not echo
    /// Message-ID in the server response; the client generates it).
    async fn send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<&str>,
    ) -> Result<String, PluginError>;
}

// ---------------------------------------------------------------------------
// Test / harness mocks
// ---------------------------------------------------------------------------

/// In-memory IMAP backend for unit tests.
pub struct MockImapBackend {
    /// Messages returned by the next `fetch_unseen` calls (FIFO batches).
    pub batches: Mutex<Vec<Vec<ParsedEmail>>>,
    /// Force the next N calls to fail with this error message.
    pub fail_with: Mutex<Option<String>>,
    /// Count of `fetch_unseen` invocations.
    pub calls: Mutex<usize>,
}

impl MockImapBackend {
    pub fn new() -> Self {
        Self {
            batches: Mutex::new(Vec::new()),
            fail_with: Mutex::new(None),
            calls: Mutex::new(0),
        }
    }

    pub fn with_batch(emails: Vec<ParsedEmail>) -> Self {
        let mock = Self::new();
        // seed synchronously — callers construct before spawning async
        mock.batches
            .try_lock()
            .expect("mock lock")
            .push(emails);
        mock
    }

    pub fn push_batch(&self, emails: Vec<ParsedEmail>) {
        if let Ok(mut g) = self.batches.try_lock() {
            g.push(emails);
        }
    }
}

impl Default for MockImapBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ImapBackend for MockImapBackend {
    async fn fetch_unseen(&self) -> Result<Vec<ParsedEmail>, PluginError> {
        *self.calls.lock().await += 1;
        if let Some(msg) = self.fail_with.lock().await.take() {
            return Err(PluginError::ExecutionFailed(msg));
        }
        let mut batches = self.batches.lock().await;
        if batches.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(batches.remove(0))
        }
    }
}

/// In-memory SMTP backend for unit tests.
pub struct MockSmtpBackend {
    pub sent: Mutex<Vec<MockSentMail>>,
    pub fail_with: Mutex<Option<String>>,
}

/// A record of a message submitted through [`MockSmtpBackend`].
#[derive(Debug, Clone)]
pub struct MockSentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to: Option<String>,
    pub message_id: String,
}

impl MockSmtpBackend {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            fail_with: Mutex::new(None),
        }
    }
}

impl Default for MockSmtpBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SmtpBackend for MockSmtpBackend {
    async fn send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<&str>,
    ) -> Result<String, PluginError> {
        if let Some(msg) = self.fail_with.lock().await.take() {
            return Err(PluginError::ExecutionFailed(msg));
        }
        let message_id = format!(
            "<mock-{}@test.local>",
            chrono::Utc::now().timestamp_millis()
        );
        self.sent.lock().await.push(MockSentMail {
            to: to.into(),
            subject: subject.into(),
            body: body.into(),
            in_reply_to: in_reply_to.map(str::to_string),
            message_id: message_id.clone(),
        });
        Ok(message_id)
    }
}

/// Shared backend handles used by the adapter.
pub type SharedImap = Arc<dyn ImapBackend>;
pub type SharedSmtp = Arc<dyn SmtpBackend>;
