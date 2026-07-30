//! Email channel adapter (IMAP + SMTP).
//!
//! Provides a [`ChannelAdapter`](clawft_plugin::traits::ChannelAdapter)
//! implementation for email communication via IMAP (inbound) and SMTP
//! (outbound). Feature-gated behind the `email` feature flag.
//!
//! # Architecture
//!
//! - Polls an IMAP mailbox for new (UNSEEN) messages at a configurable
//!   interval via the `imap` crate (sync client under `spawn_blocking`)
//! - Parses RFC822 with `mailparse` and delivers to the agent pipeline
//!   via `ChannelAdapterHost`
//! - Sends outbound replies via `lettre` (async SMTP, STARTTLS or
//!   implicit TLS)
//! - Supports password + `password_env` authentication; OAuth2/XOAUTH2
//!   is typed in config but not yet wired to IMAP/SMTP SASL
//! - All credentials use [`SecretString`](clawft_types::secret::SecretString)
//!
//! # Modules
//!
//! - [`types`] -- Configuration and message types
//! - [`transport`] -- `ImapBackend` / `SmtpBackend` traits + mocks
//! - [`imap_client`] -- Real IMAP backend
//! - [`smtp_client`] -- Real SMTP backend
//! - [`channel`] -- `ChannelAdapter` trait implementation
//! - [`factory`] -- Factory for creating adapters from JSON config

pub mod channel;
pub mod factory;
pub mod imap_client;
pub mod smtp_client;
pub mod transport;
pub mod types;

pub use channel::EmailChannelAdapter;
pub use factory::EmailChannelAdapterFactory;
pub use imap_client::RealImapBackend;
pub use smtp_client::RealSmtpBackend;
pub use transport::{ImapBackend, MockImapBackend, MockSmtpBackend, SmtpBackend};
pub use types::{EmailAdapterConfig, EmailAuth, ParsedEmail, ResolvedCredentials};
