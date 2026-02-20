//! Email channel adapter (IMAP + SMTP).
//!
//! Provides a [`ChannelAdapter`](clawft_plugin::traits::ChannelAdapter)
//! implementation for email communication via IMAP (inbound) and SMTP
//! (outbound). Feature-gated behind the `email` feature flag.
//!
//! # Architecture
//!
//! - Polls an IMAP mailbox for new messages at a configurable interval
//! - Delivers parsed emails to the agent pipeline via `ChannelAdapterHost`
//! - Sends outbound replies via SMTP
//! - Supports password and OAuth2 (Gmail) authentication
//! - All credentials use [`SecretString`](clawft_types::secret::SecretString)
//!
//! # Modules
//!
//! - [`types`] -- Configuration and message types
//! - [`channel`] -- `ChannelAdapter` trait implementation
//! - [`factory`] -- Factory for creating adapters from JSON config

pub mod channel;
pub mod factory;
pub mod types;

pub use channel::EmailChannelAdapter;
pub use factory::EmailChannelAdapterFactory;
