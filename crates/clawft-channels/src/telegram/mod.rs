//! Telegram channel plugin.
//!
//! Provides dual [`Channel`](crate::traits::Channel) +
//! [`ChannelAdapter`](clawft_plugin::traits::ChannelAdapter) implementations
//! (WEFT-170 / C7) that connect to the Telegram Bot API via long polling.
//! The plugin is registered with the host through [`TelegramChannelFactory`].
//!
//! # Modules
//!
//! - [`types`] -- Telegram Bot API request/response types
//! - [`client`] -- HTTP client wrapper for the Telegram Bot API
//! - [`channel`] -- `Channel` + `ChannelAdapter` implementations and factory

pub mod channel;
pub mod client;
pub mod types;

pub use channel::{TelegramChannel, TelegramChannelFactory};
pub use client::TelegramClient;

#[cfg(test)]
mod tests;
