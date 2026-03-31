# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Kernel Foundation (K0-K6) — Sprints 6-8

- WeftOS kernel crate (`clawft-kernel`) with 7 layered phases: K0 boot, K1 supervisor, K2 IPC, K3 WASM sandbox, K4 containers, K5 app framework, K6 mesh/agency
- K0: Kernel boot sequence, process table, capability registry, interactive REPL console
- K1: AgentSupervisor with RBAC capability enforcement, end-to-end agent lifecycle, ExoChain integration
- K2: A2A IPC with topic pub/sub, inbox routing, request-response messaging, delegation wiring, IPC tools
- K2.1: SpawnBackend, ServiceEntry, GovernanceGate from symposium implementation; chain-logged lifecycle signals (ipc.recv, ipc.ack, agent.spawn, suspend/resume)
- K2-ext: RVF deep integration with signing, witness chains, governance hooks, lineage tracking
- K3: WASM tool sandbox with Wasmtime execution, fuel metering, and tool registry integration
- K3c: ECC cognitive substrate with 6 kernel modules (causal_graph, hnsw, impulse_queue, embedding, cognitive_tick, cross_ref), boot wiring, and CLI commands
- K4: Container types, config, lifecycle manager, ServiceApi, dual governance gate, chain contracts, shell pipeline
- K5: Application framework with manifest parsing (`weftapp.toml`), CLI commands, lifecycle management, namespace isolation
- K6a: Cluster, environment, and governance types for mesh networking
- K6b: Agent-first architecture with roles, agency model, and autonomous operation
- ML-DSA-65 dual signing: Ed25519 primary + post-quantum ML-KEM-768 placeholder
- Chain-anchored genesis rules for immutable constitutional governance (22 rules)
- AI-SDLC SOPs as chain-anchored constitutional rules
- 19 genesis rule enforcement tests verifying all 22 rules
- K0-K5 full-stack integration tests: native agent + WASM + container + ECC
- K5 app lifecycle integration tests with multi-service apps and namespace isolation
- Standalone `weftos` crate as drop-in AI kernel for any Rust project

#### Weaver Operator CLI — Sprint 7

- `weaver` binary (renamed from `weave`) as WeftOS operator CLI
- Persistent kernel daemon with Unix socket RPC
- Kernel event log with `weaver kernel logs`
- Enriched CLI display: agent inspect, chain detail, resource scoring
- Signal-based stop/restart for systemd compatibility

#### ECC Weaver — Sprints 8-9

- ECC Weaver v2: self-evolving cognitive modeler running as a kernel process
- Codebase analysis pipeline: 12 conversation sources identified and ingested
- Graph ingestion: 408 causal nodes, 1,597 edges from 3 data sources
- ECC graph gap analysis traversing 408 nodes and 1,597 edges
- Cognitive tick integration with live git polling and file watching
- Weaver self-evolution loop: 41/41 TODO items resolved automatically
- Session log ingestion: 29,029 nodes extracted from 88 conversation files
- ONNX embedding backend with sentence-transformer and AST-aware modes
- Spectral analysis: lambda_2 algebraic connectivity for graph health
- Community detection via label propagation
- Predictive change analysis: burst detection and coupling-based prediction

#### OS Gap-Filling (08a/08b/08c) — Sprint 8

- 08a self-healing: restart strategies, process links, reconciliation
- 08b reliable IPC: dead letter handling, delivery guarantees
- 08c content operations: Weaver K3c integration gates
- 12 specialized agent definitions in `agents/` directory
- `weave init` command for project bootstrapping

#### Sprint 9: Test Coverage and Integration Polish

- 1,328 tests passing across 20/20 gate checks
- 45 new boot path tests covering error paths and partial recovery
- 09-weftos-gaps sprint completing 4 sub-sprints with 61 gate checkboxes

#### Sprint 10: Operational Hardening

- Self-healing supervisor: RestartStrategy enum (OneForOne, OneForAll, RestForOne, Escalate, Custom), restart budget with max_restarts/window_secs, exponential backoff (100ms base, 30s cap), ProcessLink and ProcessMonitor
- Persistence: SQLite backend for ExoChain, CausalGraph save/load to disk, HNSW index persistence, recovery-after-restart verification
- Observability: DeadLetterQueue with capture/query/retry, ReliableDelivery with ack tracking, MetricsRegistry, LogService, TimerService, wired into boot sequence
- MeshRuntime: bind/accept/handshake/dispatch, A2A bridge (local KernelMessage to MeshIpcEnvelope and back), mDNS discovery, 2-node LAN communication demonstrated
- DEMOCRITUS continuous cognitive loop: Sense (git/file/IPC events) -> Embed (ONNX vectors) -> Search (HNSW nearest-neighbor) -> Update (causal edges) -> Commit (ExoChain event); CognitiveTick with budget-aware pacing, ImpulseQueue for inter-tick events
- ConfigService and AuthService as kernel services
- 10 new extended tools in `tools_extended.rs`
- Tool signing: ToolSignature with Ed25519, register_signed(), require_signatures mode
- WASM shell execution: ShellCommand/ShellResult types, execute_shell(), shell.exec tool running in Wasmtime sandbox
- K8 GUI prototype: Tauri 2.0 wrapper, Dashboard view, Admin Forms, Knowledge Graph visualization (Cytoscape.js), Component Generator
- Docker packaging with multi-stage build
- E2E integration tests for full kernel boot path
- External codebase analysis validated on ruvector (109 crates analyzed, 16 gaps identified)
- 983 new kernel tests (613 to 1,596 kernel tests; 5,040 total test annotations)

#### clawft Framework — Sprint 6

- Gemini LLM provider
- xAI LLM provider
- Tiered router with multi-provider dispatch
- K4 ClawHub install/publish for skill distribution
- K5 MVP skills: prompt-log, skill-vetting, discord
- IRC channel adapter
- Permission re-prompt on plugin version upgrade
- Pipeline and LLM reliability improvements (D1-D11)
- Workstream A critical fixes (A1-A9)
- `weft tools` CLI subcommand for tool discovery and management
- Unified `scripts/build.sh` build script (native, wasi, browser, gate)

#### Documentation

- Fumadocs Next.js documentation site with 38 MDX pages
- 25 WeftOS kernel subsystem pages (architecture through self-healing)
- 13 clawft framework pages (getting-started through browser)
- SPARC architecture plans: 14 documents covering K0-K6 phases (ADR-028)
- K6 mesh networking SPARC plan with 15 planning documents
- 4 symposium series (K2, K3, K5, ECC) with synthesized decisions
- Comprehensive kernel governance reference (1,057 lines)
- VISION.md documenting the arc from agency to cognitive OS
- Knowledge graph data: 122 commits, 182 modules, 500 edges indexed

### Changed

- Workspace expanded from 9 crates to 22 crates
- Test count grew from 1,029 to 5,040 annotations
- Feature flags grew from initial set to 93 across 20 crates
- `weave` binary renamed to `weaver` to avoid conflict with TeX `weave` tool
- IPC API breaking changes from K2.1 symposium: SpawnBackend enum replaces direct spawn, ServiceEntry wraps service registration, GovernanceGate gates all cross-process calls
- README rewritten to lead with problems solved and deployment patterns
- ruvector distributed crates integrated into WeftOS cluster topology

### Fixed

- DashMap deadlock in A2ARouter under concurrent message dispatch
- Chain persistence defaults not applied on first boot
- Agent send CLI output not displaying response
- SkillToolProvider not wired into MCP server
- Delegation config fallback missing when no config file present
- Anthropic API compatibility: serialize tool-call assistant messages with null content
- Multi-provider LLM routing bug causing incorrect provider selection
- LLM permission handling for provider-specific auth
- 2 clippy warnings in llm_adapter.rs
- Rustdoc warning in topic.rs

### Security

- 6 vulnerabilities from phase-5 security review addressed
- C2 security tests (T30/T42) for input validation and access control
- Tool signing with Ed25519 for tamper detection on registered tools
- WASM sandbox fuel metering to prevent runaway tool execution
- Chain-anchored governance preventing unsigned process mutations

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
