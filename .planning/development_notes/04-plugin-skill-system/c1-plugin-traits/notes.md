# Phase C1: Plugin Traits -- Notes

**Items**: C1 (clawft-plugin crate with 6 traits, manifest, permissions)
**Week**: 3-4

---

## Completed: 2026-02-19

### C1: Plugin Trait Crate (`clawft-plugin`)
- New workspace crate `crates/clawft-plugin/` created
- 6 core traits defined in `traits.rs`:
  - `Tool` -- tool execution interface (name, description, parameters_schema, execute)
  - `ChannelAdapter` -- channel message handling with `CancellationToken` lifecycle
  - `PipelineStage` -- pipeline processing with `PipelineStageType` enum (PreProcess, PostProcess, Transform)
  - `Skill` -- skill definition with tool list and system instructions
  - `MemoryBackend` -- pluggable memory storage (store, retrieve, search, delete)
  - `VoiceHandler` -- placeholder trait for Workstream G forward-compat
- 3 supporting traits: `KeyValueStore`, `ToolContext`, `ChannelAdapterHost`
- All traits are `Send + Sync` with `#[async_trait]`

### Plugin Manifest (`manifest.rs`)
- `PluginManifest` struct with serde JSON deserialization
- `PluginCapability` enum for declaring trait capabilities
- `PluginPermissions` struct: `filesystem`, `network`, `env_vars`, `shell`
- `PluginResourceConfig`: fuel metering limits, memory limits

### Message Types (`message.rs`)
- `MessagePayload` enum: `Text(String)`, `Structured(Value)`, `Binary { mime_type, data }`
- Forward-compat for future UI canvas and voice payloads

### Error Types (`error.rs`)
- `PluginError` enum for unified plugin error handling

### Module Structure
```
crates/clawft-plugin/
  src/
    lib.rs        -- re-exports all public types
    traits.rs     -- 6 core + 3 supporting traits
    manifest.rs   -- plugin manifest schema
    message.rs    -- MessagePayload enum
    error.rs      -- PluginError types
```

### Feature Flag
- `voice` feature reserved as no-op in Cargo.toml
