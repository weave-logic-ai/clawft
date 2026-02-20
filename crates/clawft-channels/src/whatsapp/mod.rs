//! WhatsApp channel adapter (Cloud API).
//!
//! Feature-gated behind the `whatsapp` feature flag.

pub mod channel;
pub mod types;

pub use channel::WhatsAppChannelAdapter;
