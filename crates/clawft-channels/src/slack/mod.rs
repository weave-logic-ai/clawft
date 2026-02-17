//! Slack channel plugin.
//!
//! Provides a [`Channel`](crate::traits::Channel) implementation that
//! connects to Slack via Socket Mode WebSocket. The plugin is registered
//! with the host through [`SlackChannelFactory`].
//!
//! # Modules
//!
//! - [`events`] -- Slack Socket Mode and Events API types
//! - [`api`] -- HTTP client wrapper for the Slack Web API
//! - [`signature`] -- HMAC-SHA256 request signature verification
//! - [`channel`] -- `Channel` trait implementation
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
