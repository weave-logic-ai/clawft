# Business Requirements: clawft

## 1. Problem Statement

nanobot is a ~10,000-line Python personal AI assistant framework supporting 9 chat channels, 14 LLM providers, a tool system, memory/session management, cron scheduling, and MCP integration. **clawft** is its Rust rewrite -- a single binary that weaves together channels, providers, and tools into a lightweight, portable fabric.

The current Python implementation has inherent limitations:

- **Deployment weight**: Python runtime + 24 pip dependencies + Node.js bridge = 200+ MB installed footprint
- **Startup latency**: 2-5 seconds cold start due to import chain (litellm alone pulls 50+ transitive deps)
- **Portability**: Requires Python 3.11+, pip, and optional npm -- difficult to deploy on constrained devices
- **Single binary**: Not achievable in Python without PyInstaller/Nuitka (fragile, 80-150 MB output)
- **Resource consumption**: ~50-80 MB RSS idle, problematic for IoT/edge devices

## 2. Goals

### Primary Goals

| ID | Goal | Success Metric |
|----|------|----------------|
| G1 | Single static binary for all platforms | `cargo build --release` produces one `weft` executable, no runtime deps |
| G2 | Run on constrained devices | Idle RSS < 10 MB on ARM64, < 5 MB WASM |
| G3 | Sub-second cold start | < 500ms to first message processing |
| G4 | Feature-gated compilation | Binary includes only enabled channels/providers |
| G5 | Config compatibility | Reads existing `~/.nanobot/config.json` without migration |
| G6 | Pluggable channel architecture | All channels implemented as plugins behind a common trait |
| G7 | RVF-powered intelligence | Model routing + vector memory via RVF (RuVector Format). See [04-rvf-integration.md](04-rvf-integration.md) |
| G12 | Tiered model routing with permission control | TieredRouter selects model based on task complexity + user permissions; 3 permission levels enforced |
| G13 | Granular permission system | Per-level control over model access, tool access, rate limits, context window, streaming, escalation |
| G14 | Cost-aware routing | Daily/monthly spend caps per permission level; automatic fallback to cheaper models when budget exhausted |

### Secondary Goals

| ID | Goal | Success Metric |
|----|------|----------------|
| G8 | WASM core extraction | Core agent loop compiles to wasm32-wasip2, < 300 KB gzipped |
| G9 | Cross-compilation | CI builds for linux-x86_64, linux-aarch64, macos-arm64, windows-x86_64 |
| G10 | Embeddable library | `clawft-core` usable as a Rust library crate |
| G11 | Drop-in replacement | `weft` CLI has same commands and flags as Python `nanobot` |
| G15 | Per-user permission overrides | Individual users can have custom permission levels beyond their default tier |
| G16 | Permission audit trail | All routing and permission decisions logged for compliance and debugging |

### Non-Goals

- GUI or web dashboard
- Multi-tenancy or user management
- Dynamic plugin loading (`.so`/`.dll` at runtime) -- compile-time feature flags only
- WhatsApp channel in initial release (punted to future phase)
- Chinese platform channels (Feishu, DingTalk, Mochat, QQ) -- community-contributed later

## 3. Constraints

### Hard Constraints

1. **Standalone within `repos/nanobot/`**: The Rust workspace lives alongside the Python code, not integrated into the barni Cargo workspace at `barni/src/`
2. **Config compatibility**: Must read the existing `config.json` schema (Python pydantic model) without requiring users to migrate. Reads from `~/.clawft/` with automatic fallback to `~/.nanobot/`
3. **Session compatibility**: Must read existing `~/.nanobot/sessions/*.jsonl` files
4. **Workspace compatibility**: Must read existing `~/.nanobot/workspace/` layout (AGENTS.md, SOUL.md, USER.md, memory/, skills/)
5. **No new external services**: No databases, no Redis, no Docker required for basic operation
6. **MIT license**: All dependencies must be compatible
7. **Plugin architecture**: Every channel MUST implement the `Channel` trait; no channel logic in core

### Soft Constraints

1. **Minimum Supported Rust Version (MSRV)**: Rust 1.85+ (edition 2024, matching barni workspace)
2. **Compilation time**: Full release build < 5 minutes on CI
3. **Binary size**: Native CLI < 15 MB stripped, < 5 MB with minimal features

## 4. User Personas

### Persona 1: Self-Hoster (Primary)
- Runs clawft on a VPS or home server
- Connects 1-3 channels (typically Telegram + Slack or Discord)
- Wants: `curl -L ... | tar xz && ./weft gateway` -- single binary, zero deps
- Cares about: ease of deployment, stability, low resource usage

### Persona 2: IoT/Edge Developer
- Runs clawft on Raspberry Pi, ESP32, or similar
- Uses 1 channel, 1 provider
- Wants: minimal binary that fits in flash, low RAM
- Cares about: binary size, memory footprint, WASM compatibility

### Persona 3: Developer/Contributor
- Extends clawft with custom skills, tools, or channel plugins
- Uses the CLI for testing
- Wants: clean Rust API, good error messages, fast iteration
- Cares about: code quality, documentation, plugin API simplicity

## 5. Feature Matrix

### Channels (pluggable, each behind a Cargo feature flag)

| Channel | Python Lines | Priority | Feature Flag | Notes |
|---------|-------------|----------|--------------|-------|
| CLI/Interactive | (in commands.py) | P0 | `cli` | Built-in, NOT a plugin (core component) |
| Telegram | 390 | P0 | `channel-telegram` | Plugin. Long-polling via REST |
| Slack | 235 | P1 | `channel-slack` | Plugin. Socket Mode (WebSocket) + Web API |
| Discord | 311 | P1 | `channel-discord` | Plugin. Gateway (WebSocket) + REST |
| Email | 350 | P2 | `channel-email` | Plugin. IMAP + SMTP (future phase) |
| WhatsApp | 169 | P3 | `channel-whatsapp` | Plugin. Punted (future phase) |

**Removed from scope**: Feishu, DingTalk, Mochat, QQ (Chinese platform channels). These can be added later as community-contributed plugins using the same `Channel` trait.

### Tools

| Tool | Python Lines | Priority | WASM-Compatible |
|------|-------------|----------|-----------------|
| read_file | 54 | P0 | WASIp2 only |
| write_file | 43 | P0 | WASIp2 only |
| edit_file | 108 | P0 | WASIp2 only |
| list_dir | 30 | P0 | WASIp2 only |
| exec (shell) | 82 | P0 (native only) | No |
| web_search | 64 | P1 | Yes (HTTP) |
| web_fetch | 79 | P1 | Yes (HTTP) |
| message | 65 | P0 | Yes |
| spawn | 75 | P1 | No (needs process) |
| cron | 63 | P2 | Partial |
| MCP client | 81 | P2 | Partial |

### Providers (via clawft-llm + RVF)

LLM provider transport is handled by **clawft-llm**, a standalone library extracted from barni-providers. Model routing intelligence is provided by RVF (RuVector Format). See [04-rvf-integration.md](04-rvf-integration.md) and [06-provider-layer-options.md](06-provider-layer-options.md).

- **clawft-llm**: Standalone library with 4 native providers (Anthropic, OpenAI, Bedrock, Gemini) + config-driven OpenAI-compatible endpoint support for any provider
- **OpenAI-compatible**: Groq, DeepSeek, Mistral, OpenRouter, Together, Fireworks, Perplexity, xAI, Ollama, Azure -- all via configurable `base_url`
- **Optional sidecar**: litellm-rs for exotic providers with non-OpenAI wire formats (Cohere, HuggingFace)
- **Intelligent routing**: RVF POLICY_KERNEL + COST_CURVE segments for learned provider/model selection
- **Progressive HNSW**: Three-tier indexing (70% recall in microseconds, 95% after full load)
- **Vector memory**: RVF-backed semantic search for session/memory retrieval (150x faster)
- **Auto-tiering**: Hot=fp16, Warm=PQ, Cold=binary quantization by access frequency
- **Witness chain**: Cryptographic audit trail of all routing decisions
- **WASM microkernel**: < 8 KB vector search engine for edge deployment

## 5b. Workspace & Project Management

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| WS-1 | As a developer, I want to create isolated workspaces for different projects so that configuration, skills, and memory don't collide | P1 |
| WS-2 | As a user, I want project-specific configuration that overrides global defaults so I can tune model selection, tool policies, and system prompts per project | P1 |
| WS-3 | As a user, I want `weft init` in a directory to scaffold a `.clawft/` project workspace that is recognized automatically when I run `weft` from that directory | P1 |
| WS-4 | As a developer, I want project-local skills to be discovered alongside global skills, with project skills taking precedence | P2 |
| WS-5 | As a developer, I want project-scoped MCP server configurations so that tools available to the agent vary by project context | P2 |
| WS-6 | As a user, I want to integrate clawft with Claude Code and claude-flow so that MCP tools can be delegated to the agent from external orchestrators | P2 |

### Feature Summary

| Feature | Description | Config Layer |
|---------|-------------|-------------|
| Global workspace | `~/.clawft/` -- user-wide config, skills, sessions, memory | Global |
| Project workspace | `.clawft/` in project root -- project-specific overrides | Project |
| Config hierarchy | Project config merges over global config (deep merge, project wins) | Both |
| Skills discovery chain | Project skills dir > global skills dir (union with project precedence) | Both |
| MCP server (Mode 1) | `weft mcp-server` exposes clawft's built-in tools to external MCP clients (Claude Code, claude-flow) | Project |
| MCP client (Mode 2) | clawft connects to external MCP servers and makes their tools available to the agent | Both |
| `weft init` command | Scaffolds `.clawft/config.json`, `.clawft/skills/`, `.clawft/memory/` in cwd | Project |

### Non-Goals (Workspace)

- Multi-user workspaces (single-user only)
- Remote/cloud workspace sync
- Workspace templates marketplace

## 5c. Tiered Routing & Permission System

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| TR-1 | As an admin, I want to configure model tiers mapping complexity ranges to specific models so that simple tasks use cheap models and complex tasks get premium models | P1 |
| TR-2 | As an admin, I want to define 3 permission levels (zero-trust, user, admin) that control which models, tools, and features each level can access | P1 |
| TR-3 | As a user with "user" permission level, I want my requests to automatically route to the appropriate model tier for the task complexity, within my permission boundaries | P1 |
| TR-4 | As an admin, I want to set daily and monthly cost budgets so spending is automatically capped with fallback to cheaper models | P1 |
| TR-5 | As a zero-trust user (unauthenticated), I should only be able to use free-tier models with no tool access and strict rate limits | P1 |
| TR-6 | As an admin, I want to grant individual users elevated permissions or custom model access without changing the global permission levels | P2 |
| TR-7 | As a developer, I want the permission system to be extensible via a generic capabilities map so new permission dimensions can be added without schema changes | P2 |
| TR-8 | As an admin, I want escalation rules that allow a user-tier request to be promoted to a premium model when task complexity exceeds a configurable threshold | P2 |
| TR-9 | As a channel plugin developer, I want to pass user identity from the channel (Discord user ID, Slack user ID) through to the permission system for per-user routing | P2 |
| TR-10 | As an admin, I want an audit log of all routing decisions showing which model was selected, why, and what permissions were applied | P3 |

### Feature Summary

| Feature | Description | Config Layer |
|---------|-------------|-------------|
| Model tiers | Complexity-to-model mapping with fallback chains | Global + Project |
| Permission levels | 3 built-in levels (zero-trust, user, admin) with granular capabilities | Global |
| Per-user overrides | Individual user permission customization | Global |
| Cost budgets | Daily/monthly spend caps with auto-fallback | Global + Project |
| Escalation rules | Complexity-triggered model tier promotion | Global + Project |
| Tool access control | Per-level allowlist/denylist of available tools | Global + Project |
| Rate limiting | Per-level request rate caps | Global |
| Audit logging | Permission and routing decision trail | Global |

### Non-Goals (Tiered Routing)

- OAuth/OIDC authentication (channels provide identity)
- Billing/payment integration (cost tracking only, no charging)
- Multi-tenant isolation (single-user deployment model)
- Real-time permission changes (config reload required)

## 6. Risk Register

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| ExecTool impossible in WASM | Certain | High | 10 | Dual-target: native primary, WASM core without exec |
| WebSocket crates too heavy for WASM | High | Medium | 6 | Channel plugins are native-only; WASM core uses HTTP only |
| ruvector integration complexity | Medium | Medium | 4 | ruvector already WASM-aware; incremental integration |
| Config schema drift between Python/Rust | Medium | Medium | 4 | Shared JSON schema validation in CI; test with real configs |
| Plugin API too restrictive | Medium | Medium | 4 | Design Channel trait with escape hatch (`metadata: Value`) |
| Compilation time too slow for iteration | Medium | Low | 3 | Workspace split, feature flags, incremental builds |
| Contributor adoption barrier (Rust) | High | Low | 3 | Good docs, clear plugin API, Python version stays maintained |
| Complexity classifier accuracy | Medium | Medium | 4 | Keyword classifier may misclassify; escalation rules provide safety net for under-classified tasks |
| Permission bypass via MCP delegation | Medium | High | 6 | MCP server mode must inherit caller's permission level, not admin default |
| Cost tracking drift from actual billing | Medium | Low | 3 | Use conservative estimates; provide config to adjust cost multipliers |
| Model tier configuration complexity | Medium | Medium | 4 | Provide sensible defaults; `weft init` generates starter tier config |

## 7. Success Criteria

### MVP (Phase 1: Warp)

- [x] `cargo build --release` produces single `weft` binary (4.9 MB)
- [x] `weft gateway` starts and processes channel messages (validated with Discord)
- [x] `weft agent -m "hello"` works in CLI
- [x] Reads existing `config.json` without changes
- [x] All file tools work (read, write, edit, list_dir)
- [x] Shell exec tool works
- [x] Web search and fetch tools work
- [x] Session persistence (JSONL) works
- [x] Memory consolidation works
- [x] Channel plugin API documented and stable
- [x] Binary size < 10 MB (linux-x86_64, stripped, default features) -- actual: 4.9 MB
- [ ] RSS idle < 15 MB

### Full Parity (Phase 2: Weft)

- [x] Telegram, Slack, Discord channels all functional as plugins
- [x] All tools functional including MCP (MCP server + client operational)
- [x] Cron scheduling works
- [x] Heartbeat service works
- [ ] ruvector model routing integrated (feature-gated, Phase 3 dependency)
- [x] Vector-based memory search operational
- [x] `weft channels status` shows all channels
- [x] `weft cron list/add/remove/enable/run` works

### WASM Target (Phase 3: Finish)

- [ ] `clawft-core` compiles to `wasm32-wasip2`
- [ ] Agent loop processes messages via HTTP-only LLM calls
- [ ] File tools work via WASI filesystem
- [ ] WASM binary < 300 KB gzipped
- [ ] Runs in WAMR and Wasmtime

### Workspace & Project Management (Phase 3G+)

- [x] `weft workspace create` scaffolds project workspace (`weft init` alias pending)
- [x] Project `config.json` merges over global `~/.clawft/config.json` (`config_merge.rs`)
- [x] Project skills discovered and loaded with precedence over global (`skills list` with source annotation)
- [x] Project-scoped MCP server configurations work
- [x] MCP tool delegation from Claude Code / claude-flow works (`weft mcp-server` operational)

### Tiered Routing & Permissions (Phase 4)

- [x] TieredRouter selects model based on complexity + permissions (`tiered_router.rs`, 1646 lines)
- [x] 3 permission levels enforced (zero-trust, user, admin) (`PermissionLevelConfig` in `routing.rs`)
- [x] Tool access restricted by permission level (`permissions.rs`)
- [x] Rate limiting enforced per permission level (`rate_limiter.rs`)
- [x] Cost budgets with automatic fallback work (`cost_tracker.rs`)
- [x] Escalation from user → premium tier when complexity > threshold
- [x] Existing StaticRouter continues to work as default (backward compatible)
- [x] Config format documented and validated (`docs/guides/configuration.md`)
