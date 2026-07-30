//! Matrix channel adapter.
//!
//! Feature-gated behind the `matrix` feature flag.
//! Connects to a Matrix homeserver via the Client-Server API (`/sync`
//! long-poll, room join, `m.room.message` send).

pub mod channel;
pub mod client;
pub mod types;

pub use channel::MatrixChannelAdapter;
pub use client::MatrixClient;
pub use types::MatrixAdapterConfig;
