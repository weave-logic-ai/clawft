//! Services for the clawft framework.
//!
//! Provides cron scheduling, heartbeat monitoring, and MCP client
//! functionality. Each service generates [`InboundMessage`](clawft_types::event::InboundMessage)
//! events that feed into the main message bus.

pub mod cron_service;
pub mod error;
pub mod heartbeat;
pub mod mcp;
