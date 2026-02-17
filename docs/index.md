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

## Architecture

- [Architecture Overview](architecture/overview.md) -- Crate structure, pipeline, message flow

## Reference

- [CLI Reference](reference/cli.md) -- Complete command-line reference for `weft`
- [Tools Reference](reference/tools.md) -- Built-in tools available to the agent
- [API Documentation](https://docs.rs/clawft) -- Generated Rust API docs

## Development

- [Contributing](development/contributing.md) -- Development setup, code style, adding tools and channels
