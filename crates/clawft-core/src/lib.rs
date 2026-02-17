//! # clawft-core
//!
//! Core engine for the clawft AI assistant framework.
//!
//! Contains the agent loop, message bus, session management, tool registry,
//! context builder, memory store, and the 6-stage pipeline system.

pub mod bus;
pub mod session;
pub mod agent;
pub mod tools;
pub mod pipeline;
pub mod bootstrap;
pub mod security;

#[cfg(feature = "vector-memory")]
pub mod embeddings;
#[cfg(feature = "vector-memory")]
pub mod vector_store;
#[cfg(feature = "vector-memory")]
pub mod intelligent_router;
#[cfg(feature = "vector-memory")]
pub mod session_indexer;
