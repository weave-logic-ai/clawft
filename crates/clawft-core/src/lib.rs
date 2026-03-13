//! # clawft-core
//!
//! Core engine for the clawft AI assistant framework.
//!
//! Contains the agent loop, message bus, session management, tool registry,
//! context builder, memory store, and the 6-stage pipeline system.

pub mod agent;
#[cfg(feature = "native")]
pub mod agent_bus;
pub mod agent_routing;
pub mod bootstrap;
pub mod bus;
pub mod clawft_md;
pub mod config_merge;
pub mod json_repair;
pub mod pipeline;
pub mod planning;
pub mod routing_validation;
pub mod runtime;
pub mod security;
pub mod session;
pub mod tools;
pub mod workspace;

#[cfg(feature = "vector-memory")]
pub mod embeddings;
#[cfg(feature = "vector-memory")]
pub mod intelligent_router;
#[cfg(feature = "vector-memory")]
pub mod policy_kernel;
#[cfg(feature = "vector-memory")]
pub mod session_indexer;
#[cfg(feature = "vector-memory")]
pub mod vector_store;

#[cfg(feature = "rvf")]
pub mod complexity;
#[cfg(feature = "rvf")]
pub mod memory_bootstrap;
#[cfg(feature = "rvf")]
pub mod scoring;
