---
name: clawft
description: General clawft development skill — crate structure, plugin system, providers, channels, and tool creation
version: 0.1.0
category: development
tags:
  - clawft
  - rust
  - plugins
  - channels
  - tools
  - llm
  - providers
author: WeftOS Kernel Team
---

# General clawft Development Skill

This skill covers the overall clawft framework: crate structure, the plugin
system, adding LLM providers, channel development, and tool creation.

## Crate Structure

The workspace contains 18+ crates in a strict dependency hierarchy:

```
                          clawft-cli (weft binary)
                     /    |    |    |    \     \      \
               clawft-  clawft- clawft- clawft- clawft- clawft-
               tools   channels services plugin  llm   security
                 |        |       |       |              |
                 +--------+-------+       |              |
                 |                        |              |
               clawft-core          (plugin crates x8)  |
                 |    \                   |              |
           clawft-    clawft-        clawft-plugin -----+
           platform     llm              |
                 \                        |
                  clawft-types -----------+
```

### Key Crates

| Crate | Purpose |
|-------|---------|
| `clawft-types` | Foundation types (errors, config, messages, session). Zero internal deps. |
| `clawft-platform` | Platform abstraction (fs, env, http, process). Native + WASM targets. |
| `clawft-llm` | Standalone LLM provider abstraction. OpenAI-compatible HTTP API. |
| `clawft-core` | Central engine: agent loop, message bus, 6-stage pipeline, session manager. |
| `clawft-tools` | Built-in tools (11): file ops, shell exec, memory, web fetch/search, spawn. |
| `clawft-channels` | Plugin-based chat channel system (11 adapters). |
| `clawft-services` | Background services: cron, heartbeat, MCP, delegation. |
| `clawft-plugin` | Plugin trait definitions, WASM sandbox, hot-reload, permissions. |
| `clawft-security` | Security primitives, input validation, sandboxing. |
| `clawft-kernel` | WeftOS kernel layer: boot, processes, services, IPC, mesh. |
| `clawft-cli` | `weft` binary. Clap CLI tree, markdown rendering. |
| `clawft-weave` | `weaver` binary. OS/operator CLI for kernel management. |
| `clawft-wasm` | Browser/WASM entrypoint. |

## Pipeline Architecture

Every message flows through a 6-stage pipeline:

```
ChatRequest -> [Classifier] -> [Router] -> [Assembler] -> [Transport] -> [Scorer] -> [Learner] -> LlmResponse
```

| Stage | Default Implementation | Behavior |
|-------|----------------------|----------|
| Classifier | `KeywordClassifier` | Keyword-based TaskType assignment |
| Router | `StaticRouter` | Config-driven model selection |
| Assembler | `TokenBudgetAssembler` | chars/4 heuristic, drops middle messages |
| Transport | `OpenAiCompatTransport` | Stub or live via `ClawftLlmAdapter` |
| Scorer | `NoopScorer` | Returns 1.0 |
| Learner | `NoopLearner` | Discards trajectories |

## Adding an LLM Provider

Providers implement the `Provider` trait in `clawft-llm`:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
}
```

The `ProviderRouter` routes model identifiers (e.g., `"openai/gpt-4o"`) to
provider instances via longest-prefix matching.

### Steps to add a provider

1. Create a new struct implementing `Provider` in `clawft-llm/src/`
2. Add a `ProviderConfig` variant or reuse `OpenAiCompatProvider` if the API is OpenAI-compatible
3. Register in `ProviderRouter` during bootstrap
4. Update config schema in `clawft-types` if needed

## Channel Development

Channels are plugin-based adapters for external messaging platforms. Three core traits:

```rust
pub trait Channel: Send + Sync {
    async fn send(&self, message: OutboundMessage) -> Result<(), ChannelError>;
    async fn receive(&self) -> Result<InboundMessage, ChannelError>;
}

pub trait ChannelHost: Send + Sync {
    async fn start(&self) -> Result<(), ChannelError>;
    async fn stop(&self) -> Result<(), ChannelError>;
}

pub trait ChannelFactory: Send + Sync {
    fn create(&self, config: &Config) -> Result<Box<dyn Channel>, ChannelError>;
}
```

### Steps to add a channel

1. Create a new module in `clawft-channels/src/`
2. Implement `Channel`, `ChannelHost`, and `ChannelFactory`
3. Register with `PluginHost` in the bootstrap sequence
4. Add config fields to `clawft-types` Config if needed

## Tool Creation

Tools are registered in the `ToolRegistry`. Built-in tools live in `clawft-tools`.

### Steps to add a tool

1. Create a tool function in `clawft-tools/src/`
2. Define the tool schema (name, description, parameters as JSON Schema)
3. Register in `ToolRegistry` during bootstrap
4. All file tools MUST enforce workspace path containment
5. Tool results are truncated to 64 KB (`truncate_result()`)

### WASM tools

WASM-based tools run inside the Wasmtime sandbox with:
- Fuel metering (CPU limits)
- Memory limits
- Filesystem scoping (`WasiFsScope`)
- Tool signing for integrity verification

## Plugin System

Six extension-point traits:
- Channel plugins (messaging adapters)
- Tool plugins (custom tool implementations)
- Pipeline stage plugins (custom classifier, router, etc.)
- Skill plugins (loaded from `.claude/skills/`)
- Hook plugins (event-driven automation)
- Slash-command plugins

### Plugin lifecycle

1. Discovery: scan plugin directories
2. Loading: WASM module validation + compilation
3. Registration: register with `PluginHost`
4. Execution: sandboxed execution with permissions
5. Hot-reload: file watcher triggers re-registration

## Build Commands

```bash
# Fast compile check
scripts/build.sh check

# Run all tests
scripts/build.sh test

# Lint
scripts/build.sh clippy

# Full phase gate (before committing)
scripts/build.sh gate

# Release build
scripts/build.sh native

# WASM targets
scripts/build.sh wasi
scripts/build.sh browser

# Build everything
scripts/build.sh all
```

## Configuration

Root config is `Config` in `clawft-types`, deserialized from JSON.
Config file discovery chain (via `ConfigLoader`):
1. Explicit path (CLI `--config` flag)
2. `.clawft.json` in current directory
3. `clawft.json` in current directory
4. Default config

Key config sections:
- `agents`: model, max_tokens, temperature, memory_window, max_tool_iterations
- `channels`: per-channel adapter configuration
- `plugins`: plugin directories and permissions
- `kernel`: WeftOS kernel settings (KernelConfig)

## Security Rules

- Never hardcode API keys or secrets
- Validate all user input at system boundaries (`validate_session_id()`)
- Sanitize content before output (`sanitize_content()`)
- Tool results capped at 64 KB
- File operations enforce workspace containment
- WASM tools run in sandbox with explicit permissions

## Related Files

- **Architecture doc**: `docs/src/content/docs/clawft/architecture.mdx`
- **Types crate**: `crates/clawft-types/`
- **Core crate**: `crates/clawft-core/`
- **Build script**: `scripts/build.sh`
- **CLI crate**: `crates/clawft-cli/`
