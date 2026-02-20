//! IRC channel adapter.
//!
//! Feature-gated behind the `irc` feature flag.
//! Connects to an IRC server for text-based message send/receive.

pub mod channel;
pub mod types;

pub use channel::IrcChannelAdapter;
