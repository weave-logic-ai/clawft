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
//! - [`voice_bridge`] -- WEFT-232 Discord VC → STT → agent → TTS → VC
//!   (`discord-voice` feature; mock transport for tests)

pub mod api;
pub mod channel;
pub mod chunker;
pub mod events;
pub mod factory;
/// Discord voice-channel bridge (WEFT-232). Requires `discord-voice`.
#[cfg(feature = "discord-voice")]
pub mod voice_bridge;

pub use channel::DiscordChannel;
pub use chunker::{
    ChunkPlan, ChunkerOptions, EmbedField, EmbedPayload, OutboundChunk, chunk_message, plan_chunks,
    resolve_max_message_len,
};
pub use factory::DiscordChannelFactory;

#[cfg(feature = "discord-voice")]
pub use voice_bridge::{
    real_transport_status, DiscordVcPcmFrame, DiscordVcTransport, DiscordVoiceBridge,
    DiscordVoiceBridgeConfig, DiscordVoiceTurn, MockDiscordVcTransport,
    SONGBIRD_TRANSPORT_IMPLEMENTED,
};

#[cfg(test)]
mod tests;
