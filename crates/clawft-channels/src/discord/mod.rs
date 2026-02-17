//! Discord channel plugin.
//!
//! Provides a [`Channel`](crate::traits::Channel) implementation that
//! connects to the Discord Gateway via WebSocket. The plugin is registered
//! with the host through [`DiscordChannelFactory`].
//!
//! # Modules
//!
//! - [`events`] -- Discord Gateway payload types and opcodes
//! - [`api`] -- HTTP client wrapper for the Discord REST API
//! - [`channel`] -- `Channel` trait implementation
//! - [`factory`] -- `ChannelFactory` implementation

pub mod api;
pub mod channel;
pub mod events;
pub mod factory;

pub use channel::DiscordChannel;
pub use factory::DiscordChannelFactory;

#[cfg(test)]
mod tests;
