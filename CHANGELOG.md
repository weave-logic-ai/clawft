# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-17

### Added

#### Core
- 9-crate Rust workspace: types, platform, core, llm, tools, channels, services, cli, wasm
- Agent loop with configurable retry and backoff
- 6-stage LLM pipeline: Classifier, Router, Assembler, Transport, Scorer, Learner
- Platform abstraction layer with traits for HTTP, filesystem, environment, and process
- Native platform implementation for Linux/macOS/Windows
- Tool registry with dynamic registration and dispatch
- Event-driven architecture with typed message passing

#### CLI
- Binary `weft` with subcommand-based interface via clap derive
- `agent` subcommand for running agent sessions
- `gateway` subcommand for HTTP/WebSocket gateway server
- `status` subcommand for system health and diagnostics
- `channels` subcommand for managing channel integrations
- `cron` subcommand for scheduled task management
- `sessions` subcommand for session lifecycle management
- `memory` subcommand for agent memory operations
- `config` subcommand for configuration management

#### Tools
- File operations: read, write, edit, list with path validation
- Shell execution with configurable timeout and working directory
- Agent spawn for sub-agent orchestration
- Memory tool for persistent key-value storage
- Web fetch with HTTP client and response parsing
- Web search with provider abstraction
- Message tool for inter-agent communication

#### Channels
- Telegram channel plugin with bot API integration
- Slack channel plugin with Web API and Events API support
- Discord channel plugin with gateway WebSocket connection

#### Services
- Cron scheduling service with cron expression parsing
- Heartbeat service for liveness monitoring

#### WASM
- Platform stubs for WebAssembly target (HTTP, FS, Env, Process)
- Feature flags (`native-exec`, `channels`, `services`) for conditional compilation
- WASM-compatible build profile with size optimizations

### Security
- `CommandPolicy` with allowlist and denylist for shell command execution
- `UrlPolicy` with SSRF protection (private IP blocking, scheme restrictions)
- Path traversal prevention in file operations

### Infrastructure
- GitHub Actions CI workflow with build matrix (stable, nightly, WASM)
- GitHub Actions release workflow with cross-compilation and asset publishing
- GitHub Actions benchmark workflow for performance regression tracking
- GitHub Actions WASM build workflow for browser/worker targets
- Docker multi-stage build with `FROM scratch` minimal image
- Release profile with LTO, strip, single codegen unit, and abort-on-panic
- 1,029 tests across the workspace

[Unreleased]: https://github.com/clawft/clawft/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/clawft/clawft/releases/tag/v0.1.0
