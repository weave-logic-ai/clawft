//! Slack channel plugin.
//!
//! Provides dual [`Channel`](crate::traits::Channel) +
//! [`ChannelAdapter`](clawft_plugin::traits::ChannelAdapter) implementations
//! (WEFT-170 / C7) that connect to Slack via Socket Mode WebSocket.
//! The plugin is registered with the host through [`SlackChannelFactory`].
//!
//! # Modules
//!
//! - [`events`] -- Slack Socket Mode and Events API types
//! - [`api`] -- HTTP client wrapper for the Slack Web API
//! - [`signature`] -- HMAC-SHA256 request signature verification
//! - [`channel`] -- `Channel` + `ChannelAdapter` implementations
//! - [`factory`] -- `ChannelFactory` implementation

pub mod api;
pub mod channel;
pub mod events;
pub mod factory;
pub mod signature;

pub use channel::SlackChannel;
pub use factory::SlackChannelFactory;

#[cfg(test)]
mod tests;
