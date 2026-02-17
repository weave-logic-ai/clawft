//! Telegram channel plugin.
//!
//! Provides a [`Channel`](crate::traits::Channel) implementation that
//! connects to the Telegram Bot API via long polling. The plugin is
//! registered with the host through [`TelegramChannelFactory`].
//!
//! # Modules
//!
//! - [`types`] -- Telegram Bot API request/response types
//! - [`client`] -- HTTP client wrapper for the Telegram Bot API
//! - [`channel`] -- `Channel` trait implementation and factory

pub mod channel;
pub mod client;
pub mod types;

pub use channel::{TelegramChannel, TelegramChannelFactory};
pub use client::TelegramClient;

#[cfg(test)]
mod tests;
