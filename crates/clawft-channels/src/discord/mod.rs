//! Discord channel plugin.
//!
//! Provides dual [`Channel`](crate::traits::Channel) +
//! [`ChannelAdapter`](clawft_plugin::traits::ChannelAdapter) implementations
//! (WEFT-170 / C7) that connect to the Discord Gateway via WebSocket.
//! The plugin is registered with the host through [`DiscordChannelFactory`].
//!
//! # Modules
//!
//! - [`events`] -- Discord Gateway payload types and opcodes
//! - [`api`] -- HTTP client wrapper for the Discord REST API
//! - [`channel`] -- `Channel` + `ChannelAdapter` implementations
//! - [`factory`] -- `ChannelFactory` implementation

pub mod api;
pub mod channel;
pub mod chunker;
pub mod events;
pub mod factory;

pub use channel::DiscordChannel;
pub use chunker::{
    ChunkPlan, ChunkerOptions, EmbedField, EmbedPayload, OutboundChunk, chunk_message, plan_chunks,
    resolve_max_message_len,
};
pub use factory::DiscordChannelFactory;

#[cfg(test)]
mod tests;
