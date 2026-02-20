//! Signal channel adapter.
//!
//! Feature-gated behind the `signal` feature flag.
//! Uses `signal-cli` subprocess for message send/receive.

pub mod channel;
pub mod types;

pub use channel::SignalChannelAdapter;
