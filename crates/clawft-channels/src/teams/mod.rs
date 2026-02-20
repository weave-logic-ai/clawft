//! Microsoft Teams channel adapter.
//!
//! Feature-gated behind the `teams` feature flag.
//! Uses Bot Framework and Graph API with Azure AD auth.

pub mod channel;
pub mod types;

pub use channel::TeamsChannelAdapter;
