# Plugin System Architecture

> **Status**: This document describes the plugin framework introduced in
> Element 04. The `clawft-plugin` crate provides trait definitions; runtime
> loading and sandboxing are planned for a future sprint.

## Overview

clawft's plugin system provides six core extension points through the
`clawft-plugin` crate. Each extension point is defined by a trait that
plugins implement to add new capabilities.

| Extension Point | Trait | Purpose |
|----------------|-------|---------|
| Tools | `Tool` | Agent tool execution (e.g. web search, file I/O) |
| Channels | `ChannelAdapter` | External platform message handling (Telegram, Slack, etc.) |
| Pipeline Stages | `PipelineStage` | Custom processing stages in the agent pipeline |
| Skills | `Skill` | High-level agent capabilities with tools and instructions |
| Memory Backends | `MemoryBackend` | Pluggable memory storage (vector, KV, graph) |
| Voice Handlers | `VoiceHandler` | Voice/audio processing (reserved for Workstream G) |

## Plugin Manifest

Every plugin declares its capabilities, permissions, and resource limits
through a `clawft.plugin.json` manifest file:

```json
{
  "name": "my-plugin",
  "version": "0.1.0",
  "capabilities": ["tool"],
  "permissions": {
    "network": false,
    "filesystem": false,
    "env_vars": []
  },
  "resources": {
    "max_memory_mb": 64,
    "max_cpu_seconds": 10
  }
}
```

The `PluginManifest` struct in `clawft-plugin` validates and parses this file.

## Plugin Traits

### Tool

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &dyn ToolContext,
    ) -> Result<serde_json::Value, PluginError>;
}
```

### ChannelAdapter

```rust
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&mut self, host: Arc<dyn ChannelAdapterHost>) -> Result<(), PluginError>;
    async fn stop(&mut self) -> Result<(), PluginError>;
    async fn send(&self, payload: MessagePayload) -> Result<(), PluginError>;
}
```

### PipelineStage

```rust
#[async_trait]
pub trait PipelineStage: Send + Sync {
    fn stage_type(&self) -> PipelineStageType;
    fn name(&self) -> &str;
    async fn process(
        &self,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError>;
}
```

### Skill

```rust
#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn tools(&self) -> Vec<Arc<dyn Tool>>;
    fn system_prompt(&self) -> Option<String>;
}
```

### MemoryBackend

```rust
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn store(&self, key: &str, value: &[u8]) -> Result<(), PluginError>;
    async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>, PluginError>;
    async fn delete(&self, key: &str) -> Result<(), PluginError>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<String>, PluginError>;
}
```

## Supporting Types

- **`ToolContext`** -- Execution context passed to tool invocations, providing
  access to key-value storage and configuration.
- **`ChannelAdapterHost`** -- Host services available to channel adapters for
  dispatching inbound messages and querying state.
- **`PluginError`** -- Unified error type for plugin operations.
- **`MessagePayload`** -- Structured message envelope for channel communication.

## Planned Features

The following features are planned for future sprints:

- **Dynamic loading**: Load plugins from shared libraries or WASM modules at runtime.
- **Sandboxing**: Resource limits and capability-based permissions enforcement.
- **Plugin registry**: Discovery and management of installed plugins.
- **Hot reload**: Update plugins without restarting the host process.
