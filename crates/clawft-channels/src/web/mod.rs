//! Web channel — delivers outbound messages via a broadcaster callback.
//!
//! Unlike Telegram/Slack/Discord which connect to external APIs, the web
//! channel delivers responses to browser clients through a topic-based
//! broadcaster (WebSocket/SSE). Inbound messages arrive via the REST API
//! (not through `Channel::start`), so `start()` is a no-op that waits
//! for cancellation.

pub mod channel;

pub use channel::{WebChannel, WebChannelFactory, WebPublisher};
