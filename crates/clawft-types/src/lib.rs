//! # clawft-types
//!
//! Core type definitions for the clawft AI assistant framework.
//!
//! This crate is the foundation of the dependency graph -- all other
//! clawft crates depend on it. It contains:
//!
//! - **[`error`]** -- [`ClawftError`] and [`ChannelError`] error types
//! - **[`config`]** -- Configuration schema (ported from Python `schema.py`)
//! - **[`event`]** -- Inbound/outbound message events
//! - **[`provider`]** -- LLM response types and the 15-provider registry
//! - **[`session`]** -- Conversation session state
//! - **[`cron`]** -- Scheduled job types

pub mod config;
pub mod cron;
pub mod delegation;
pub mod error;
pub mod event;
pub mod provider;
pub mod routing;
pub mod secret;
pub mod security;
pub mod session;
pub mod skill;
pub mod workspace;

pub use error::{ChannelError, ClawftError, Result};
