//! Google Chat channel adapter.
//!
//! Feature-gated behind the `google-chat` feature flag.
//! Skeleton implementation; full OAuth2 deferred to Workstream F6.

pub mod channel;
pub mod types;

pub use channel::GoogleChatChannelAdapter;
