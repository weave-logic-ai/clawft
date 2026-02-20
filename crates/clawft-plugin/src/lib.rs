//! Plugin trait definitions for clawft.
//!
//! This crate defines the unified plugin architecture for the clawft AI
//! assistant framework. It provides trait definitions for tools, channels,
//! pipeline stages, skills, memory backends, and voice handlers -- the
//! six core extension points that all downstream feature work depends on.
//!
//! # Trait Overview
//!
//! | Trait | Purpose |
//! |-------|---------|
//! | [`Tool`] | Tool execution interface for agent capabilities |
//! | [`ChannelAdapter`] | Channel message handling for external platforms |
//! | [`PipelineStage`] | Processing stage in the agent pipeline |
//! | [`Skill`] | High-level agent capability with tools and instructions |
//! | [`MemoryBackend`] | Pluggable memory storage backend |
//! | [`VoiceHandler`] | Voice/audio processing (placeholder for Workstream G) |
//!
//! # Supporting Traits
//!
//! | Trait | Purpose |
//! |-------|---------|
//! | [`KeyValueStore`] | Key-value storage exposed to plugins via `ToolContext` |
//! | [`ToolContext`] | Execution context passed to tool/skill invocations |
//! | [`ChannelAdapterHost`] | Host services for channel adapters |
//!
//! # Plugin Manifest
//!
//! Plugins declare their capabilities, permissions, and resource limits
//! through a [`PluginManifest`], typically parsed from a JSON file
//! (`clawft.plugin.json`).
//!
//! # Feature Flags
//!
//! - `voice` -- Reserved for Workstream G. Currently a no-op.

pub mod error;
pub mod manifest;
pub mod message;
pub mod sandbox;
pub mod traits;

// Re-export core types at crate root for convenience.
pub use error::PluginError;
pub use manifest::{
    PluginCapability, PluginManifest, PluginPermissions, PluginResourceConfig,
};
pub use message::MessagePayload;
pub use sandbox::{
    SandboxAuditEntry, SandboxPolicy, SandboxType,
    NetworkPolicy, FilesystemPolicy, ProcessPolicy, EnvPolicy,
};
pub use traits::{
    ChannelAdapter, ChannelAdapterHost, KeyValueStore, MemoryBackend,
    PipelineStage, PipelineStageType, Skill, Tool, ToolContext, VoiceHandler,
};
