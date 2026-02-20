//! Matrix channel adapter.
//!
//! Feature-gated behind the `matrix` feature flag.
//! Connects to a Matrix homeserver via the client-server API.

pub mod channel;
pub mod types;

pub use channel::MatrixChannelAdapter;
