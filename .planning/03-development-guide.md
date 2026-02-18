# Development Guide: Concurrent Implementation Plan

## 1. Phase Overview

| Phase | Name | Scope | Duration | Parallel Streams |
|-------|------|-------|----------|-----------------|
| **Phase 1** | **Warp** | Foundation + CLI + Telegram plugin | 6-8 weeks | 3 streams |
| **Phase 2** | **Weft** | Slack + Discord plugins + ruvector + services | 4-6 weeks | 4 streams |
| **Phase 3** | **Finish** | WASM + CI/CD + toolchain + workspace + MCP | 6-10 weeks | 9 streams (3A-3I) |

Phases can partially overlap once core types stabilize. The warp (foundation threads) must be set before the weft (cross-threads) can be interlaced.

---

## 2. Phase 1: Warp (Foundation + CLI + Telegram Plugin)

### Goal
A working `weft gateway` that processes Telegram messages via the plugin architecture, and `weft agent -m "hello"` for CLI usage. Must read existing config.json. Channel plugin API is stable.

### Stream 1A: Types + Platform + Plugin API (Week 1-3)

**Owner**: Foundation architect

| Week | Task | Output |
|------|------|--------|
| 1 | Workspace setup: Cargo.toml, rust-toolchain.toml, .cargo/config.toml | Compiling empty workspace |
| 1 | `clawft-types`: Config structs from `schema.py` (all 20+ models) | `Config` deserializes real config.json |
| 1 | `clawft-types`: Event types from `events.py` | `InboundMessage`, `OutboundMessage` |
| 1 | `clawft-types`: Provider types from `base.py` | `LlmResponse`, `ToolCallRequest` |
| 1 | `clawft-types`: Error types | `ClawftError` enum |
| 2 | `clawft-types`: Provider registry from `registry.py` | Static `PROVIDERS` array |
| 2 | `clawft-types`: Session/cron types | `Session`, `CronJob`, `CronSchedule` |
| 2 | `clawft-platform`: Trait definitions (HttpClient, FileSystem, Environment, ProcessSpawner) | Trait API finalized |
| 2 | `clawft-channels`: Channel, ChannelHost, ChannelFactory traits | **Plugin API finalized** |
| 3 | `clawft-platform`: Native implementations (reqwest, std::fs, std::env, tokio::process) | Working PAL |
| 3 | Config loader (JSON file + env var overlay) | `load_config()` reads ~/.clawft/config.json (fallback: ~/.nanobot/) |
| 3 | `clawft-channels`: Plugin host (lifecycle, registry, message routing) | Host manages plugins |

**Validation**:
- `cargo test -p clawft-types` passes with real config.json fixtures
- `cargo test -p clawft-platform` passes with file/HTTP/env tests
- `cargo test -p clawft-channels` passes with mock channel plugin tests

**Blockers for other streams**: Types crate API and Channel trait must stabilize by end of week 2.

### Stream 1B: Core Engine (Week 2-6)

**Owner**: Core engineer

**Depends on**: Stream 1A types API (week 2)

| Week | Task | Output |
|------|------|--------|
| 2 | `clawft-core`: MessageBus (mpsc channels) | `publish_inbound`, `consume_inbound`, `dispatch_outbound` |
| 3 | `clawft-core`: SessionManager (JSONL read/write) | Reads existing Python .jsonl sessions |
| 3 | `clawft-core`: MemoryStore (MEMORY.md + HISTORY.md) | `read_long_term`, `write_long_term`, `append_history` |
| 3 | `clawft-core`: SkillsLoader (list, load, progressive) | Reads existing skill dirs |
| 4 | `clawft-core`: ContextBuilder (system prompt assembly) | `build_system_prompt`, `build_messages` |
| 4 | `clawft-core`: ToolRegistry (register, execute, schema gen) | Dynamic dispatch to tool trait objects |
| 5 | `clawft-core`: AgentLoop (_process_message, _run_agent_loop) | Core iteration: LLM call -> tool exec -> repeat |
| 5 | `clawft-core`: Memory consolidation (LLM-based summarization) | Background consolidation task |
| 6 | `clawft-core`: SubagentManager (background task spawning) | `spawn_subagent` via tokio::spawn |
| 6 | Integration tests: full message flow (inbound -> agent -> outbound) | End-to-end in-process test |

**Validation**:
- Mock LLM provider returns canned responses
- Message flows through bus -> agent -> tool -> response
- Sessions persist across restarts

### Stream 1C: Provider + Tools + CLI + Telegram (Week 3-8)

**Owner**: Application engineer

**Depends on**: Stream 1A platform API + Channel trait (week 3), Stream 1B tool registry (week 4)

| Week | Task | Output |
|------|------|--------|
| 3 | Extract `clawft-llm` from barni-providers as standalone crate | Standalone library compiles independently |
| 3 | `clawft-llm`: Internalize CircuitBreaker, strip tenant/compliance | No barni deps remaining |
| 3 | `clawft-llm`: Add config-driven OpenAI-compatible endpoint support | Any base_url works |
| 4 | `clawft-llm`: Prefix-based model routing + model aliasing | `"anthropic/claude-3-sonnet"` auto-routes |
| 4 | `clawft-llm`: JSON repair (malformed LLM output handling) | Fallback parsing for broken JSON |
| 5 | `clawft-tools`: File tools (read, write, edit, list_dir) | With path validation |
| 5 | `clawft-tools`: Shell exec tool | With timeout, workspace restriction |
| 6 | `clawft-tools`: Web search (Brave API) + web fetch (readability) | HTTP-based tools |
| 6 | `clawft-tools`: Message tool + spawn tool | Bus integration |
| 7 | `clawft-channels/telegram`: Telegram plugin (long-polling, send, voice) | **First channel plugin works** |
| 7 | `clawft-cli`: `agent` command (interactive + single-message) | `weft agent -m "hello"` |
| 8 | `clawft-cli`: `gateway` command (plugin host + agent + dispatch) | `weft gateway` runs |
| 8 | `clawft-cli`: `onboard`, `status` commands | Setup and diagnostics |

**Validation**:
- Real Telegram bot receives and responds to messages via plugin
- CLI interactive mode with history and markdown rendering
- `weft status` shows correct provider configuration

### Phase 1 Milestone Checklist

- [ ] `cargo build --release` produces single `weft` binary
- [x] Binary reads existing `~/.nanobot/config.json` (and `~/.clawft/config.json`)
- [ ] `weft agent -m "What is 2+2?"` returns LLM response
- [x] `weft gateway` starts Telegram bot via plugin architecture
- [ ] Telegram bot receives messages and responds
- [x] File tools work (tested via Telegram: "read the file at ...")
- [ ] Shell exec works (tested via Telegram: "run ls -la")
- [x] Sessions persist (restart gateway, history preserved)
- [x] Memory consolidation runs when window exceeded
- [x] Channel plugin API is stable and documented

---

## 3. Phase 2: Weft (Remaining Plugins + ruvector + Services)

### Goal
Slack and Discord channel plugins interlaced into the fabric. ruvector-powered model routing and vector memory woven in. Cron/heartbeat/MCP services.

### Stream 2A: Channel Plugins (Week 7-10)

| Week | Task | Notes |
|------|------|-------|
| 7-8 | Slack plugin (Socket Mode WS + Web API) | WebSocket + 3 REST endpoints, HMAC verification |
| 9-10 | Discord plugin (Gateway WS + REST) | WebSocket, Ed25519 verification, rate limiting |

### Stream 2B: RVF Integration (Week 7-11)

See [04-rvf-integration.md](04-rvf-integration.md) for full specification.

| Week | Task | Notes |
|------|------|-------|
| 7 | Add rvf-runtime + rvf-types as workspace deps | Feature-gated behind `ruvector` flag |
| 8 | `clawft-core/memory`: RvfVectorStore for semantic memory search | Progressive HNSW over MEMORY.md content |
| 8 | `clawft-core`: IntelligentRouter with POLICY_KERNEL via clawft-llm transport | Learned routing to optimal provider/model |
| 9 | `clawft-core/session`: Index session turns in rvf | Semantic session retrieval |
| 10 | `clawft-core`: Learned routing via COST_CURVE segments + clawft-llm transport | Update policies from response quality |
| 11 | Witness log via rvf WITNESS segments | Cryptographic audit trail |
| 11 | First-startup indexing from existing MEMORY.md | Migration from flat file to rvf |

### Stream 2C: Services (Week 7-11)

| Week | Task | Notes |
|------|------|-------|
| 7-8 | Cron service (scheduling + persistence) | Port from `cron/service.py` + `cron/types.py` |
| 8 | Cron CLI commands (list, add, remove, enable, run) | 5 subcommands |
| 9 | Heartbeat service | Timer-based LLM prompt |
| 10 | MCP client (stdio + HTTP transports) | JSON-RPC over stdio/HTTP |
| 11 | Codex OAuth provider (SSE streaming) | Special-case provider |
| 11 | (Optional) litellm-rs sidecar integration | `SidecarTransport` for exotic providers, ~1.5 days LOE |

### Stream 2D: CLI Completion (Week 7-11)

| Week | Task | Notes |
|------|------|-------|
| 7 | `channels status` command | Table display of all channel plugin configs |
| 8 | `cron` subcommand group | All cron CLI commands |
| 9 | `provider login` command | Codex OAuth flow |
| 10-11 | Markdown-to-platform conversion | Telegram HTML, Slack mrkdwn, Discord markdown |

### Phase 2 Milestone Checklist

- [x] Telegram, Slack, Discord channels all work as plugins
- [ ] RVF-backed model routing selects optimal provider/model
- [ ] RVF vector memory search returns relevant context (progressive HNSW)
- [x] Cron jobs persist and execute on schedule
- [x] Heartbeat runs every 30 minutes
- [ ] MCP client connects to external MCP servers and exposes their tools to the agent (Mode 2)
- [ ] `weft channels status` shows all plugin status
- [ ] `weft cron list` shows scheduled jobs
- [ ] Codex OAuth login works

---

## 4. Phase 3: Finish (WASM Target + Optimization + Release)

### Goal
Extract `clawft-core` to compile as wasm32-wasip2. Optimize binary sizes. CI/CD pipeline. Rust toolchain upgrade. Security hardening. Workspace/project management. MCP server delegation. Tighten the weave.

### Stream 3A: WASM Core (Week 11-14)

| Week | Task | Notes |
|------|------|-------|
| 11 | `clawft-wasm` crate: entrypoint, WASI HTTP client | Basic WASM binary that makes LLM calls |
| 11 | WASI filesystem integration | Config loading, session persistence |
| 12 | Platform trait implementations for WASI | WasiHttpClient, WasiFileSystem, WasiEnvironment |
| 12 | Strip exec tool, channel plugins, CLI from WASM build | Feature flag gating |
| 13 | `talc` allocator + rvf-wasm microkernel integration | Vector search in WASM (< 8 KB) |
| 14 | Test in WAMR and Wasmtime | Validate on target runtimes |

### Stream 3B: Polish + CI/CD (Week 11-14)

| Week | Task | Notes |
|------|------|-------|
| 11 | Binary size profiling with `twiggy` | Identify top contributors |
| 12 | GitHub Actions: build matrix (linux, macos, windows) | Cross-compile with `cross` |
| 12 | GitHub Actions: WASM build + size assertions | Fail if over threshold |
| 13 | Release workflow: GitHub Releases with binaries | Tag-based release |
| 13 | Container image: `FROM scratch` with static binary | Minimal Docker image |
| 14 | Performance benchmarks: Rust vs Python | Startup, memory, throughput |

### Stream 3C: Rust Toolchain Upgrade (Week 15)

| Week | Task | Notes |
|------|------|-------|
| 15 | Upgrade `rust-toolchain.toml` from 1.85 to 1.93+ | Edition 2024, wasip2 Tier 2 support |
| 15 | Fix new clippy lints (`let_and_return`, `derivable_impls`, `collapsible_if`) | Clean build required after toolchain change |
| 15 | Update WASM CI to target `wasm32-wasip2` primary | Retain `wasip1` as fallback |

### Stream 3D: Config Hierarchy & Project Workspaces (Week 15-16)

| Week | Task | Notes |
|------|------|-------|
| 15 | `clawft-platform`: Project workspace discovery (walk cwd upward for `.clawft/`) | New `discover_project_root()` function |
| 15 | `clawft-platform`: Config deep-merge (global + project layers) | `ConfigLoader::load()` returns merged config |
| 16 | `clawft-types`: Add `project_root: Option<PathBuf>` to `Config` | Tracks active project context |
| 16 | `clawft-cli`: `weft init` command scaffolds `.clawft/` project workspace | Creates config.json, skills/, memory/, agents/ |
| 16 | Unit + integration tests for config merge semantics | Scalar override, map merge, array replace, null removal |

### Stream 3E: Skills & Agent Discovery Chain (Week 16-17)

| Week | Task | Notes |
|------|------|-------|
| 16 | `clawft-core/skills`: Multi-directory discovery (project > global > legacy) | `SkillsLoader::new()` accepts ordered search paths |
| 17 | `clawft-core/skills`: Name-collision resolution (project skills win) | Union with precedence |
| 17 | `clawft-core/agent`: Agent definition chain (`.clawft/agents/` > global) | AGENTS.md merge |
| 17 | Tests: skill override, union, missing dirs, empty project skills | Discovery chain validation |

### Stream 3F: MCP Server Mode & Tool Delegation (Week 17-18)

| Week | Task | Notes |
|------|------|-------|
| 17 | `clawft-services/src/mcp/`: McpServerShell + ToolProvider trait + CompositeToolProvider | Newline-delimited JSON-RPC over stdio |
| 17 | `ToolRegistry::to_mcp_tools()`: Advertise tools via `tools/list` | Schema conversion |
| 18 | `tools/call` dispatch through CompositeToolProvider with middleware pipeline | Delegated tool execution with SecurityGuard, PermissionFilter, AuditLog |
| 18 | `clawft-cli`: `weft mcp-server` command | Starts clawft as MCP server |
| 18 | Session isolation for delegated calls | Caller-specified or dedicated session |

### Stream 3G: Project-Scoped MCP Client (Week 18-19)

| Week | Task | Notes |
|------|------|-------|
| 18 | `.clawft/mcp/servers.json` loading in `ConfigLoader` | Project-local MCP server configs |
| 19 | Merge project + global `mcp_servers` maps (deep-merge) | Project servers override global by name |
| 19 | MCP client connects to project-scoped servers on startup | Project context auto-detected |
| 19 | Integration test: project MCP server + global MCP server coexist | Merged tool namespace |

### Stream 3H: Claude Code / claude-flow Integration (Week 19-20)

| Week | Task | Notes |
|------|------|-------|
| 19 | Document MCP server protocol for Claude Code integration | Stdio transport, tool schemas |
| 19 | `weft mcp-server` inherits project workspace from cwd | Project-aware delegation |
| 20 | Test: Claude Code invokes clawft tools via MCP | End-to-end validation |
| 20 | Test: claude-flow orchestrates clawft agent via MCP | Swarm coordination scenario |

### Stream 3I: Documentation & Polish (Week 20)

| Week | Task | Notes |
|------|------|-------|
| 20 | Workspace setup guide (`docs/workspace.md`) | Project init, config hierarchy, skills chain |
| 20 | MCP integration guide (`docs/mcp-integration.md`) | Server mode, Claude Code setup, claude-flow setup |
| 20 | Update `weft --help` and command documentation | New commands: `init`, `mcp-server` |
| 20 | CHANGELOG entries for Phase 3D-3H features | Keep a Changelog format |

### Phase 3 Milestone Checklist

**WASM & CI/CD (3A-3C)**:
- [ ] `clawft-wasm` compiles to wasm32-wasip2
- [ ] Agent loop processes messages in Wasmtime
- [ ] File tools work via WASI filesystem
- [ ] rvf-wasm microkernel provides vector search in WASM (< 8 KB)
- [ ] WASM binary < 300 KB uncompressed
- [ ] WASM binary < 120 KB gzipped
- [ ] Runs in WAMR interpreter mode
- [ ] CI builds for linux-x86_64, linux-aarch64, macos-arm64, windows-x86_64
- [ ] GitHub Releases workflow publishes binaries
- [x] Rust toolchain upgraded to 1.93+ (edition 2024)
- [x] Zero clippy warnings on new toolchain

**Workspace & Project Management (3D-3E)**:
- [ ] `weft init` scaffolds `.clawft/` project workspace
- [ ] Project config deep-merges over global config
- [ ] Project skills discovered with precedence over global
- [ ] Agent definitions resolved via project > global chain
- [ ] Config merge tests pass (override, merge, replace, null removal)

**MCP & Integration (3F-3H)**:
- [ ] `weft mcp-server` exposes tools via McpServerShell (newline-delimited JSON stdio)
- [ ] External clients can invoke tools via `tools/call` through CompositeToolProvider
- [ ] Project-scoped MCP servers loaded and connected
- [ ] Claude Code can use clawft as MCP tool provider
- [ ] claude-flow can orchestrate clawft agent via MCP
- [ ] Session isolation for delegated calls works

**Documentation (3I)**:
- [ ] Workspace setup guide complete
- [ ] MCP integration guide complete
- [ ] CLI help updated for new commands

---

## 4b. Phase 4: Tiered Routing & Permission System

### Goal
Replace the Level 0 StaticRouter with a configurable TieredRouter that selects models based on task complexity and user permissions. Implement a 3-level permission system (zero-trust → user → admin) with granular capability controls. Cost-aware routing with budget enforcement.

See [08-tiered-router.md](08-tiered-router.md) for the full feature specification.

### Stream 4A: Permission Types & Config (Week 1-2)

**Owner**: Types architect

**Depends on**: Phase 3 complete (existing pipeline types stable)

| Week | Task | Output |
|------|------|--------|
| 1 | `clawft-types`: Add `PermissionLevel` enum (ZeroTrust, User, Admin) | Serializable permission levels |
| 1 | `clawft-types`: Add `PermissionCapabilities` struct (model_access, tool_access, rate_limit, etc.) | Granular capability definition |
| 1 | `clawft-types`: Add `ModelTier` struct (name, models, complexity_range, cost) | Tier configuration |
| 1 | `clawft-types`: Add `RoutingConfig` (mode, tiers, permissions, budgets, escalation) | Top-level routing config |
| 1 | `clawft-types`: Add `routing: Option<RoutingConfig>` to `Config` struct | Config integration |
| 2 | Config deserialization tests with fixture JSON | Validate full config round-trips |
| 2 | Default permission configs (sensible built-in defaults for each level) | Zero-config works |

**Validation**:
- `cargo test -p clawft-types` passes with routing config fixtures
- Default `RoutingConfig` serializes to expected JSON
- Backward compatible: missing `routing` field defaults to `None` (StaticRouter behavior)

### Stream 4B: TieredRouter Implementation (Week 2-4)

**Owner**: Pipeline engineer

**Depends on**: Stream 4A types (week 1)

| Week | Task | Output |
|------|------|--------|
| 2 | `clawft-core/pipeline`: `TieredRouter` struct implementing `ModelRouter` trait | Core routing logic |
| 2 | Tier selection algorithm: complexity → matching tier → model selection | Deterministic model selection |
| 3 | Permission filtering: filter available tiers by user's `allowed_tiers` | Permission enforcement |
| 3 | Escalation logic: promote to higher tier when complexity > threshold (if allowed) | Complexity-triggered upgrade |
| 3 | Fallback chain: if preferred tier model unavailable, try next in tier, then lower tier | Graceful degradation |
| 3 | `CostTracker`: track estimated spend per user/level (daily/monthly) | Budget tracking |
| 4 | Budget enforcement: auto-fallback to cheaper tier when budget exhausted | Cost cap enforcement |
| 4 | `RateLimiter`: token-bucket rate limiter per user/level | Rate limiting |
| 4 | Comprehensive unit tests (20+ test cases for routing decisions) | Full coverage |

**Validation**:
- TieredRouter selects correct tier for complexity values at boundaries
- Permission filtering blocks access to disallowed tiers
- Escalation triggers correctly at threshold
- Budget exhaustion triggers fallback
- Rate limiter rejects over-limit requests
- Fallback chain works when models are unavailable

### Stream 4C: Auth Context & Integration (Week 3-5)

**Owner**: Integration engineer

**Depends on**: Stream 4B router (week 3)

| Week | Task | Output |
|------|------|--------|
| 3 | `clawft-core/agent`: Resolve `PermissionLevel` from `InboundMessage.sender_id` + config | User → permission mapping |
| 3 | Thread permission context through to `ChatRequest` metadata or extended `TaskProfile` | Permission available at routing time |
| 4 | `clawft-core/pipeline`: `PipelineRegistry` selects TieredRouter when `routing.mode == "tiered"` | Automatic router selection |
| 4 | Tool access enforcement: `ToolRegistry` filters available tools by permission capabilities | Tool restriction |
| 4 | `clawft-channels`: Pass sender identity from Discord/Slack/Telegram to `InboundMessage` | Channel identity threading |
| 5 | Integration tests: end-to-end message → classification → routing → tool filtering | Full pipeline validation |
| 5 | Backward compatibility: `routing.mode == "static"` or missing routing config uses StaticRouter | Zero-breaking-change |

**Validation**:
- Zero-trust user gets free-tier model, no tools
- User-level gets standard models, basic tools
- Admin gets all models, all tools
- Missing routing config defaults to StaticRouter (existing behavior)
- Per-user overrides work

### Stream 4D: Cost Tracking & Audit (Week 4-5)

**Owner**: Observability engineer

**Depends on**: Stream 4B CostTracker (week 3)

| Week | Task | Output |
|------|------|--------|
| 4 | `CostTracker` persistence (daily/monthly tallies to disk or memory) | Persistent cost state |
| 4 | Cost estimation from token counts + tier cost_per_1k_tokens | Accurate cost tracking |
| 5 | Audit log: record routing decisions (model, reason, permissions, cost) to structured log | Tracing-based audit |
| 5 | `weft status --routing` command: show current tier config, budget usage, active limits | CLI observability |

**Validation**:
- Cost tracking accumulates correctly across requests
- Budget reset on day/month boundary
- Audit log captures all routing decisions
- `weft status --routing` shows useful info

### Phase 4 Milestone Checklist

- [ ] `RoutingConfig` types added to `clawft-types` with full serde support
- [ ] `TieredRouter` implements `ModelRouter` trait
- [ ] 3 permission levels (zero-trust, user, admin) enforced
- [ ] Granular tool access control per permission level
- [ ] Rate limiting per permission level
- [ ] Cost budget tracking with auto-fallback
- [ ] Complexity-based escalation works
- [ ] Model fallback chains work
- [ ] Auth context threads from channel → router
- [ ] Per-user permission overrides work
- [ ] Backward compatible: missing routing config = StaticRouter
- [ ] Config format documented and tested
- [ ] Audit logging of routing decisions
- [ ] `weft status --routing` shows tier and budget info
- [ ] 50+ unit tests covering routing edge cases

---

## 5. Concurrent Development Rules

### Dependency Management Between Streams

```
Stream 1A (Types+Platform+Plugin API) -----> Stream 1B (Core) -----> Stream 2A-D
                                       -----> Stream 1C (Provider+Tools+CLI+Telegram)

Stream 3A (WASM) -------\
Stream 3B (CI/CD) ------+---> parallel, independent
Stream 3C (Toolchain) --/

Stream 3D (Config Hierarchy) -----> Stream 3E (Skills Chain) -----> Stream 3G (Project MCP)
                                                               -----> Stream 3H (Integration)
Stream 3F (MCP Server) -----> Stream 3H (Integration)
Stream 3I (Docs) -- follows all other Phase 3 streams

Stream 4A (Permission Types) -----> Stream 4B (TieredRouter) -----> Stream 4C (Auth Integration)
                                                              -----> Stream 4D (Cost & Audit)
```

- **Stream 1A is the warp.** No other stream can start coding until types API is defined (end of week 1), platform traits defined (end of week 2), and Channel plugin trait defined (end of week 2).
- **Streams 1B and 1C can run in parallel** once types and traits are stable.
- **Phase 2 streams are fully parallel** because each channel plugin, ruvector integration, and service is independent.
- **Streams 3A, 3B, 3C are parallel** (WASM, CI/CD, and toolchain are independent).
- **Stream 3D (config hierarchy) gates 3E, 3G, and 3H** because skills chain and project MCP depend on project workspace discovery.
- **Stream 3F (MCP server) gates 3H** because integration testing requires the server to exist.
- **Stream 3I (docs) runs last**, after all features are implemented.

### Interface Contracts

Before coding begins, these interfaces must be agreed upon:

1. **Config JSON format**: Test fixtures from real Python-generated config.json files
2. **Tool trait**: `name()`, `description()`, `parameters()`, `execute()` signatures
3. **Channel trait**: `name()`, `start()`, `send()`, `is_running()` signatures
4. **ChannelHost trait**: `deliver_inbound()`, `config()`, `http()` signatures
5. **ChannelFactory trait**: `channel_name()`, `build()` signatures
6. **Provider trait** (from clawft-llm): `name()`, `models()`, `complete()`, `complete_stream()`, `status()` signatures
7. **Platform traits**: HttpClient, FileSystem, Environment, ProcessSpawner signatures
8. **MessageBus API**: publish/consume/subscribe signatures
9. **Error types**: Which errors are recoverable vs fatal
10. **Config merge**: Deep-merge semantics (scalar override, map merge, array replace, null removal)
11. **Skills discovery**: Multi-directory search order and name-collision resolution
12. **MCP server**: `tools/list` and `tools/call` JSON-RPC message formats

### File Ownership

To avoid merge conflicts, each stream owns specific crates:

| Stream | Owned Crates |
|--------|-------------|
| 1A | clawft-types, clawft-platform, clawft-channels (host + trait) |
| 1B | clawft-core |
| 1C | clawft-llm, clawft-tools, clawft-cli, clawft-channels/telegram |
| 2A | clawft-channels/slack, clawft-channels/discord |
| 2B | clawft-core (ruvector integration, vector memory, intelligent routing) |
| 2C | clawft-services |
| 2D | clawft-cli (additional commands) |
| 3A | clawft-wasm |
| 3B | CI/CD workflows, scripts, Dockerfile, benchmarks |
| 3C | Workspace-wide toolchain config, clippy fixes across all crates |
| 3D | clawft-platform (config merge), clawft-types (project_root), clawft-cli (`init`) |
| 3E | clawft-core (skills chain, agent discovery) |
| 3F | clawft-services (mcp_server), clawft-cli (`mcp-server`) |
| 3G | clawft-platform (project MCP config), clawft-services (MCP client merge) |
| 3H | Integration tests, end-to-end validation |
| 3I | docs/, CHANGELOG, CLI help text |
| 4A | clawft-types (routing config, permission types) |
| 4B | clawft-core/pipeline (TieredRouter, CostTracker, RateLimiter) |
| 4C | clawft-core/agent (auth context), clawft-channels (identity threading) |
| 4D | clawft-core (audit), clawft-cli (`weft status --routing`) |

### Branching Strategy

- `main` -- stable, always compiles
- `weft/phase-1` -- integration branch for Phase 1 (Warp)
- `weft/types` -- Stream 1A work
- `weft/core` -- Stream 1B work
- `weft/provider-tools-cli` -- Stream 1C work
- Merge to `weft/phase-1` when stream milestones pass
- Merge `weft/phase-1` to `main` when Phase 1 milestone passes

### Testing Contracts

Each stream must provide:
1. **Unit tests** for all public functions
2. **Integration tests** that use mock implementations of platform traits
3. **Plugin tests** using mock ChannelHost for channel plugins
4. **Fixture files** for config.json, session.jsonl, MEMORY.md examples
5. **CI passing** before merge to integration branch

---

## 6. Critical Path

The critical path through the project is:

```
Types API (W1) -> Platform Traits (W2) -> Channel Plugin Trait (W2) ->
Core MessageBus (W2) -> Core AgentLoop (W5) ->
Telegram Plugin (W7) -> Gateway Command (W8)
```

Total critical path: **8 weeks** to MVP.

Phase 4 Critical Path:
```
Permission Types (W1) -> TieredRouter (W2-3) -> Auth Integration (W3-4) ->
Cost Tracking (W4-5) -> End-to-end Tests (W5)
```

Total Phase 4 critical path: **5 weeks**.

---

## 7. Risk Mitigation During Development

| Risk | Detection | Response |
|------|-----------|----------|
| Types API changes after streams start | Breaking `cargo build --workspace` | Immediate fix in types + notify all streams |
| Plugin API too restrictive for Slack/Discord | Integration test failure during Phase 2 | Extend Channel trait with optional methods or metadata escape hatch |
| RVF crate size too large for minimal builds | CI size check | Ensure RVF is fully feature-gated; minimal build excludes it (~260 KB) |
| Provider HTTP format mismatch | Integration test failure | Add provider-specific test fixtures from real API captures |
| Session format incompatibility | Load test with real Python sessions | Fix serde annotations, add migration layer if needed |
| Binary size exceeds target | CI size check | Profile with twiggy, optimize hot paths |
| WASM compilation failure | CI WASM build | Feature-flag problematic deps, use cfg attributes |
| Config merge edge cases | Deep-merge unit tests | Comprehensive fixtures for null, empty, nested override scenarios |
| Skills name collision across dirs | Discovery integration tests | Document and test precedence rules; warn on collision in debug mode |
| MCP server stdio conflicts with CLI | Manual test of `weft mcp-server` | Separate stdio streams; MCP uses stdin/stdout, logs use stderr |

---

## 8. Definition of Done

A task is done when:
1. Code compiles without warnings (`cargo clippy`)
2. All tests pass (`cargo test`)
3. No new `unsafe` blocks without justification
4. Public API has doc comments
5. Feature-gated code has appropriate `cfg` attributes
6. No hardcoded secrets or paths
7. Binary size regression checked (for release builds)
8. Channel plugins pass mock ChannelHost integration tests
