# clawft Documentation

clawft is a modular Rust framework for building AI assistants with multi-channel
messaging and tool integration. The CLI binary `weft` provides an interactive
agent, a multi-channel gateway, and management commands.

---

## Getting Started

- [Quick Start](getting-started/quickstart.md) -- Install, configure, and run your first agent

## Guides

- [Configuration](guides/configuration.md) -- Config file format, environment variables, feature flags
- [Channels](guides/channels.md) -- Setting up Telegram, Slack, and Discord channels
- [Providers](guides/providers.md) -- LLM provider configuration, built-in providers, custom providers
- [Routing and Pipeline](guides/routing.md) -- 6-stage pipeline, intelligent routing, scoring and learning
- [Tool Calls](guides/tool-calls.md) -- Tool execution lifecycle, MCP integration
- [RVF Integration](guides/rvf.md) -- Vector memory, RVF format, planned features
- [Skills and Agents](guides/skills-and-agents.md) -- Custom skills, agent definitions, slash commands
- [Workspaces](guides/workspaces.md) -- Project workspaces, CLAWFT.md, scoped configuration
- [Voice](guides/voice.md) -- Voice pipeline, TTS providers (OpenAI, ElevenLabs), talk mode
- [Build](guides/build.md) -- Build system, feature flags, native and WASM targets

## Deployment

- [Docker](deployment/docker.md) -- Docker image, Compose, health checks, security
- [WASM](deployment/wasm.md) -- WebAssembly build, Wasmtime/WAMR, platform limitations
- [Releases](deployment/release.md) -- Version numbering, downloading binaries, building from source

## Architecture

- [Architecture Overview](architecture/overview.md) -- Crate structure, pipeline, message flow

## Web Dashboard

- [Developer Guide](ui/developer-guide.md) -- React + Vite UI setup, project structure, development workflow
- [API Reference](ui/api-reference.md) -- REST, WebSocket, and SSE endpoint documentation
- [Architecture](ui/architecture.md) -- API layer architecture, bridge pattern, real-time events
- [Browser Mode](ui/browser-mode.md) -- WASM-based in-browser operation
- [Deployment](ui/deployment.md) -- Building and deploying the web dashboard

## Reference

- [CLI Reference](reference/cli.md) -- Complete command-line reference for `weft`
- [Tools Reference](reference/tools.md) -- Built-in tools available to the agent
- [Skill Index](skills/index.md) -- API-style registry of all built-in skills
- [API Documentation](https://docs.rs/clawft) -- Generated Rust API docs
- [Security](reference/security.md) -- Security model, path traversal protection, prompt injection guards

## Development

- [Contributing](development/contributing.md) -- Development setup, code style, adding tools and channels
