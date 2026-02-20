# Phase 5: Sprint (Improvements Integration)

> Integrates codebase fixes from the full 9-crate review with OpenClaw-parity features.
> Source: `.planning/improvements.md` (Workstreams A through M).
> Voice (Workstream G) and UI (K1/K6) are **out of scope** for this sprint -- see
> `voice_development.md` and `ui_development.md` respectively.

---

## 1. Phase Overview

| Stream | Name | Source Workstreams | Duration | Parallel Group |
|--------|------|--------------------|----------|----------------|
| **5A** | Critical Fixes & Type Safety | A, I | Week 1-2 | Group 1 |
| **5B** | Architecture Cleanup | B | Week 2-4 | Group 1-2 |
| **5C** | Plugin & Skill System | C | Week 3-8 | Group 2-3 |
| **5D** | Pipeline & LLM Reliability | D | Week 2-5 | Group 1-2 |
| **5E** | Channel Enhancements | E | Week 4-8 | Group 2-3 |
| **5F** | Dev Tools & Applications | F | Week 5-10 | Group 3 |
| **5G** | Memory & Workspace | H | Week 4-8 | Group 2-3 |
| **5H** | Documentation Sync | J | Week 3-5 | Group 2 |
| **5I** | Multi-Agent Routing | L | Week 5-9 | Group 3 |
| **5J** | Claude Flow Integration | M | Week 3-7 | Group 2-3 |
| **5K** | Deployment & Community | K (K2-K5, K3a) | Week 8-12 | Group 4 |

**Parallel Groups**: Streams within the same group can run concurrently. Groups are loosely sequential:
- **Group 1 (Week 1-4)**: Fixes, cleanup, early pipeline work -- no feature dependencies
- **Group 2 (Week 3-7)**: Plugin foundation, docs, Claude Flow, early channels/memory
- **Group 3 (Week 5-10)**: Feature buildout on top of plugin system
- **Group 4 (Week 8-12)**: Deployment, packaging, benchmarks

---

## 2. Stream 5A: Critical Fixes & Type Safety (Week 1-2)

### Goal
Resolve all blocking bugs and security issues before feature work builds on top. Clear type-safety debt to prevent regressions in later streams.

### Week 1: Critical Bugs & Security

| Week | Task | Item | Output |
|------|------|------|--------|
| 1 | Fix session key round-trip corruption (percent-encoding) | A1 | `session.rs` uses reversible encoding |
| 1 | Replace unstable `DefaultHasher` with `fnv` or `xxhash` (fixed seed) | A2 | Deterministic hash, one-time re-index migration |
| 1 | Fix invalid JSON from error formatting (use `serde_json::json!`) | A3 | Valid JSON tool results in all error paths |
| 1 | Redact plaintext credentials in config structs (env var names only) | A4 | `Debug` impls redact secrets, `#[serde(skip_serializing)]` |
| 1 | Suppress API key echo during onboarding (`rpassword`) | A5 | Terminal echo suppressed |
| 1 | Complete private IP range check in SSRF protection (172.16-31.*) | A6 | Full RFC 1918 coverage |
| 1 | Add HTTP request timeout on LLM provider client (120s) | A7 | `reqwest::ClientBuilder` with timeout |
| 1 | Fix `unsafe std::env::set_var` in parallel tests (`temp_env` crate) | A8 | No UB under parallel test runner |
| 1 | Gate `clawft_services` imports behind `#[cfg(feature = "services")]` | A9 | `--no-default-features` compiles cleanly |

### Week 2: Type Safety & Cleanup

| Week | Task | Item | Output |
|------|------|------|--------|
| 2 | Fix `DelegationTarget` serde to `snake_case` | I1 | Consistent serde across all enums |
| 2 | Convert string-typed policy modes to proper enums | I2 | `CommandPolicyMode`, `RateLimitStrategy` enums |
| 2 | Add `skip_serializing_if` for `ChatMessage::content` | I3 | No `"content": null` in serialized output |
| 2 | Replace job ID generation with `uuid::Uuid::new_v4()` | I4 | No same-second collisions |
| 2 | Fix `camelCase` normalizer consecutive-uppercase handling | I5 | `"HTMLParser"` -> `"html_parser"` |
| 2 | Remove dead code or annotate with `TODO(workstream)` | I6 | Clean `cargo clippy` output |
| 2 | Fix always-true test assertion in transport.rs | I7 | Meaningful assertion on specific outcome |
| 2 | Expose `MockTransport` behind `test-utils` feature flag | I8 | Shared test infrastructure across crates |

**Validation**:
- `cargo build -p clawft-cli --no-default-features` compiles (A9)
- `cargo test --workspace` passes with no UB warnings (A8)
- `cargo clippy --workspace` reports zero warnings (I6)
- Session round-trip test: keys with underscores survive encode/decode (A1)
- Embeddings hash stability test: same input -> same hash across runs (A2)

---

## 3. Stream 5B: Architecture Cleanup (Week 2-4)

### Goal
Unify shared types, eliminate duplication, and split oversized files. These structural changes unblock feature work in Streams 5C-5K.

### Week 2: Type Unification

| Week | Task | Item | Output |
|------|------|------|--------|
| 2 | Unify `Usage` type across crates (canonical `u32` in `clawft-types`) | B1 | Single `Usage` type, `clawft-llm` imports it |
| 2 | Unify duplicate `LlmMessage` types | B2 | Single type in `pipeline/traits.rs`, re-exported |
| 2 | Deduplicate `ProviderConfig` naming collision | B7 | `clawft-llm` uses `LlmProviderConfig` |
| 2 | Consolidate `build_messages` duplication | B8 | Shared base with `extra_instructions: Option<String>` |
| 2 | Extract MCP protocol version constant | B9 | `const MCP_PROTOCOL_VERSION` in `mcp/types.rs` |

### Week 3-4: Structural Splits & Deduplication

| Week | Task | Item | Output |
|------|------|------|--------|
| 3 | Split `clawft-types/src/config.rs` (~1400 lines) | B3 | `config/channels.rs`, `config/providers.rs`, `config/policies.rs`, `config/mod.rs` |
| 3 | Split `clawft-core/src/agent/loop_core.rs` (1645 lines) | B3 | Extract tool execution, streaming, message building |
| 3 | Split `clawft-core/src/pipeline/tiered_router.rs` (1646 lines) | B3 | Extract cost tracker, tier selection, classifier |
| 3 | Split `clawft-core/src/pipeline/transport.rs` (1282 lines) | B3 | Extract request building, response parsing |
| 3 | Split `clawft-core/src/tools/registry.rs` (1242 lines) | B3 | Extract individual tool implementations |
| 4 | Split `clawft-core/src/agent/skills_v2.rs` (1159 lines) | B3 | Extract YAML parsing, caching, registry |
| 4 | Split `clawft-core/src/pipeline/llm_adapter.rs` (1127 lines) | B3 | Extract retry logic, config override |
| 4 | Split `clawft-core/src/pipeline/traits.rs` (1107 lines) | B3 | Extract callback types, pipeline stages |
| 4 | Split `clawft-types/src/routing.rs` (~950 lines) | B3 | Extract permissions, delegation |
| 4 | Unify cron storage formats (CLI flat JSON -> JSONL event sourcing) | B4 | Single CronService API for both CLI and gateway |
| 4 | Extract shared tool registry builder | B5 | `build_tool_registry(config, platform) -> ToolRegistry` |
| 4 | Extract shared policy types to `clawft-types` | B6 | Canonical `CommandPolicy`, `UrlPolicy` definitions |

**Validation**:
- `cargo test --workspace` passes after every split (no functional changes)
- All files under 500 lines after B3 splits
- `cargo doc --workspace` generates clean documentation for unified types
- Cron CLI and gateway use same storage format (B4)

---

## 4. Stream 5C: Plugin & Skill System (Week 3-8)

### Goal
Build the plugin infrastructure that all extensibility depends on. After this stream, new capabilities are plugins -- no more modifying `clawft-core` for features.

### Sub-Phase 5C.1: Plugin Trait Crate (Week 3-4)

| Week | Task | Item | Output |
|------|------|------|--------|
| 3 | Define unified plugin traits: `Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler` (placeholder) | C1 | New crate `clawft-plugin` with trait definitions |
| 3 | Define plugin manifest schema (JSON/YAML) with `SKILL.md` compatibility types | C1 | Manifest spec with reserved `voice` capability type |
| 3 | Ensure `ChannelAdapter` trait supports binary (audio) payloads | C1 | Forward-compat for voice and media messages |
| 4 | Wire empty `voice` feature flag in Cargo.toml | C1 | No-op feature, populatable later |
| 4 | Replace hand-rolled YAML parser in `skills_v2.rs` with `serde_yaml::from_str` | C3 (prereq) | Handles nested structures, multi-line, quoted strings |

### Sub-Phase 5C.2: WASM Host & Skill Loader (Week 4-6)

| Week | Task | Item | Output |
|------|------|------|--------|
| 4 | Implement WASM plugin host using `wasmtime` + `wit` component model | C2 | Typed WASM interfaces, size budget (<300KB) |
| 4 | Complete `WasiFileSystem` (currently all stubs) | C2 | Functional filesystem for WASM plugins |
| 5 | Wire `init()` and `process_message()` in `clawft-wasm` | C2 | WASM plugins can process messages |
| 5 | Implement `WasiEnvironment` against `Platform::Environment` trait | C2 | Trait impl, not standalone struct |
| 5 | WASM HTTP client implementation | C2 | Plugins can make HTTP requests |
| 5 | Skill Loader: parse `SKILL.md` (YAML frontmatter -> tool description) | C3 | Auto-register as WASM or native wrapper |
| 6 | `ClawHub` discovery (HTTP index + git clone) | C3 | `weft skill install github.com/openclaw/skills/...` |
| 6 | Update PluginHost to unify channels + tools under plugin traits | C7 | Concurrent `start_all`/`stop_all` |
| 6 | Add `SOUL.md`/`AGENTS.md` personality injection into pipeline | C7 | Learner/Assembler stages support personality |

### Sub-Phase 5C.3: Dynamic Loading & Hot-Reload (Week 6-8)

| Week | Task | Item | Output |
|------|------|------|--------|
| 6 | Runtime loading with sandbox isolation | C4 | Plugins load at runtime |
| 6 | Skill precedence layering: workspace > managed/local > bundled | C4 | Matches OpenClaw resolution order |
| 7 | Hot-reload watcher (`notify` crate) on skill directories | C4 | Changes take effect mid-session |
| 7 | Plugin-shipped skills: manifest declares skill directories | C4 | Plugin skills participate in precedence |
| 7 | Wire interactive slash-command framework through registry | C5 | Dynamic skill-contributed commands |
| 8 | Auto-expose loaded skills through MCP server | C6 | VS Code/Copilot/Claude Desktop integration |
| 8 | Autonomous skill creation: detect repeated patterns, generate skills | C4a | Self-improving agent capability |

**Validation**:
- `clawft-plugin` crate compiles with trait definitions and manifest schema
- WASM plugin loads, calls `init()` and `process_message()` successfully
- `SKILL.md` parsing handles nested YAML, multi-line values, quoted strings
- Skill precedence: workspace skill overrides managed skill of same name
- Hot-reload: modify skill file -> agent picks up change without restart
- MCP server `tools/list` includes dynamically loaded skills
- `cargo build -p clawft-cli --no-default-features` still compiles (plugin is feature-gated)

---

## 5. Stream 5D: Pipeline & LLM Reliability (Week 2-5)

### Goal
Improve agent loop performance, LLM transport reliability, and routing pipeline correctness.

### Week 2-3: Core Pipeline Fixes

| Week | Task | Item | Output |
|------|------|------|--------|
| 2 | Parallel tool execution (`futures::future::join_all`) | D1 | Concurrent multi-tool calls |
| 2 | Fix streaming failover (reset stream, discard partial) | D2 | Clean failover without data corruption |
| 2 | Structured error variants for retry (`ServerError { status }`) | D3 | No string prefix matching |
| 3 | Configurable retry policy via `ClawftLlmConfig` | D4 | Retry count, backoff, status codes configurable |
| 3 | Record actual wall-clock latency around provider calls | D5 | `ResponseOutcome.latency_ms` is real data |
| 3 | Thread `sender_id` for cost recording in TieredRouter | D6 | `CostTracker` integration functional |
| 3 | Change `StreamCallback` from `Fn` to `FnMut` | D7 | Token accumulators work as callbacks |

### Week 4-5: Performance & Transport

| Week | Task | Item | Output |
|------|------|------|--------|
| 4 | Bounded message bus channels with configurable buffer | D8 | Backpressure prevents unbounded memory growth |
| 4 | MCP transport concurrency: request-ID multiplexer for stdio | D9 | Concurrent MCP calls over single transport |
| 4 | MCP HTTP transport: accept `Arc<reqwest::Client>` | D9 | Shared client, redirect child stderr to log |
| 5 | Cache skill/agent bootstrap files with mtime checking | D10 | No disk read on every LLM call |
| 5 | Async file I/O in skills loader (`tokio::fs`) | D11 | No blocking of Tokio executor |

**Validation**:
- Multi-tool benchmark: 3 tools execute in ~1x time (not 3x) (D1)
- Failover test: mid-stream failure produces clean output from second provider (D2)
- Retry test: `ServerError { status: 503 }` triggers retry, `status: 400` does not (D3)
- Latency recorded in routing feedback and audit output (D5)
- Cost tracking accumulates per `sender_id` (D6)
- Message bus: fast producer blocks when buffer full (D8)

---

## 6. Stream 5E: Channel Enhancements (Week 4-8)

### Goal
Improve existing channels and add new channel plugins. All new channels implemented as `clawft-plugin` `ChannelAdapter` instances.

**Depends on**: Stream 5C Sub-Phase 5C.1 (plugin trait crate), Stream 5A item A4 (credential redaction for E2 OAuth)

| Week | Task | Item | Output |
|------|------|------|--------|
| 4 | Discord Gateway Resume (OP 6): use stored `session_id`/`resume_url` | E1 | Reconnect without full re-identify |
| 5 | Email channel plugin: IMAP + SMTP via `lettre` + `imap` | E2 | Full read/reply/attach, Gmail OAuth2 |
| 5 | Email proactive inbox triage via cron | E2 | Scheduled inbox processing |
| 6 | WhatsApp channel via Cloud API wrapper | E3 | Plugin-based WhatsApp integration |
| 6 | Signal / iMessage bridge (via `signal-cli` subprocess) | E4 | Plugin-based messaging bridge |
| 7 | Matrix / IRC channel adapters | E5 | Generic protocol adapters as plugins |
| 7 | Google Chat channel (Google Workspace API, OAuth2) | E5a | DMs and Spaces support |
| 7 | Microsoft Teams channel (Bot Framework / Graph API) | E5b | Channels and 1:1 chat support |
| 8 | Enhanced heartbeat / proactive check-in mode | E6 | CronService "check-in" for proactive behavior |

**Validation**:
- Discord reconnect uses resume instead of re-identify when `session_id` exists (E1)
- Email channel sends and receives via IMAP/SMTP in integration test (E2)
- WhatsApp plugin processes inbound webhook and sends reply (E3)
- All new channels register through `clawft-plugin` `ChannelAdapter` trait
- `weft channels status` shows all plugin channels with health

---

## 7. Stream 5F: Dev Tools & Applications (Week 5-10)

### Goal
Developer tools and application integrations, all implemented as plugins via the `clawft-plugin` trait system.

**Depends on**: Stream 5C Sub-Phase 5C.1 (plugin traits), Stream 5C item C6 (MCP skill exposure)

| Week | Task | Item | Output |
|------|------|------|--------|
| 5 | Git tool plugin (`git2` crate): clone, commit, branch, PR, diff, blame | F1 | MCP-exposed git operations |
| 6 | Cargo/build integration plugin: build, test, clippy, publish | F2 | Skill with tool calls |
| 6 | Code analysis via `tree-sitter`: AST parsing, LSP client | F3 | IDE-like code intelligence |
| 7 | Browser CDP automation (`chromiumoxide`): screenshot, form fill, scraping | F4 | Sandboxed headless browser control |
| 7 | Calendar integration: Google Calendar / Outlook / iCal | F5 | OAuth2-based calendar ops |
| 8 | Generic REST + OAuth2 helper plugin | F6 | Reusable OAuth2 flow for all integrations |
| 8 | Docker/Podman orchestration tool plugin | F7 | Container lifecycle from agent context |
| 9 | MCP deep IDE integration: VS Code extension backend | F8 | Agent edits code live in IDE |
| 10 | MCP client for external servers (1000+ community MCP servers) | F9 | `mcp_servers` in `clawft.toml`, auto-discovery, pooling |

**Validation**:
- Git plugin: clone repo, create branch, commit, diff in integration test (F1)
- Tree-sitter: parse Rust file, extract function signatures (F3)
- Browser: headless screenshot of URL (F4)
- OAuth2 helper: complete auth flow with test provider (F6)
- MCP client: connect to external MCP server, list tools, invoke tool (F9)
- All plugins behind feature flags, base binary unaffected

---

## 8. Stream 5G: Memory & Workspace (Week 4-8)

### Goal
Per-agent workspace isolation and completion of the RVF vector memory roadmap.

**Depends on**: Stream 5A item A2 (stable hash fix), Stream 5C Sub-Phase 5C.1 (plugin traits)

| Week | Task | Item | Output |
|------|------|------|--------|
| 4 | Markdown workspace: `~/.clawft/workspace` with `SKILL.md`, `SOUL.md`, `USER.md` | H1 | Workspace coexists with JSONL/vector memory |
| 4 | Per-agent isolation: `~/.clawft/agents/<agentId>/` with dedicated files | H1 | Agent-specific personality and memory isolation |
| 5 | Auto-summarization of long conversations | H1 | Background summarization task |
| 5 | HNSW-backed VectorStore (replace brute-force cosine scan) | H2.1 | `instant-distance` or `hnsw` crate integration |
| 5 | Production embedder: wire LLM embedding API (`api_embedder.rs`) | H2.2 | Replace `HashEmbedder` with real embeddings |
| 6 | RVF file I/O: real segment read/write for memory persistence | H2.3 | Replace plain JSON in `ProgressiveSearch` |
| 6 | `weft memory export` / `weft memory import` CLI commands | H2.4 | Portable data transfer |
| 7 | POLICY_KERNEL storage: persist routing policies across restarts | H2.5 | Durable `IntelligentRouter` state |
| 7 | WITNESS segments: tamper-evident audit trail | H2.6 | Cryptographic action audit |
| 7 | Temperature-based quantization: hot/warm/cold tiers | H2.7 | Reduced storage for cold vectors |
| 8 | WASM compatibility: `micro-hnsw-wasm` for browser/edge | H2.8 | Vector search in WASM deployments |
| 8 | Standardize timestamp representations to `DateTime<Utc>` | H3 | Consistent timestamps across all types |

**Validation**:
- Agent A and Agent B have fully isolated workspaces with independent `SOUL.md` (H1)
- Vector search returns semantically relevant results (not random) (H2.1-2)
- Memory export/import round-trips: export, delete, import, verify identical (H2.4)
- WITNESS segments: tamper detection catches modified audit entries (H2.6)
- All timestamps deserialize consistently regardless of source (H3)

---

## 9. Stream 5H: Documentation Sync (Week 3-5)

### Goal
Bring documentation in sync with actual codebase behavior. Fix incorrect provider counts, wrong descriptions, and document undocumented features.

| Week | Task | Item | Output |
|------|------|------|--------|
| 3 | Fix provider counts across docs (9 actual, not 7-8) | J1 | Consistent provider references |
| 3 | Fix assembler truncation description (Level 0 does truncate) | J2 | Accurate overview.md |
| 3 | Fix token budget source reference (uses `max_context_tokens`) | J3 | Accurate routing.md |
| 4 | Document identity bootstrap (`SOUL.md`/`IDENTITY.md` override) | J4 | New section in skills-and-agents.md or configuration.md |
| 4 | Document rate-limit retry behavior (3-retry, 500ms wait) | J5 | Documented in providers.md |
| 4 | Document CLI log level change (default `info` -> `warn`) | J6 | Documented in cli.md |
| 5 | Plugin system documentation: architecture, creating plugins, SKILL.md format | J7 | Full plugin/skill docs (depends on C1-C6) |

**Validation**:
- All provider count references show 9 (or correct current number)
- New users can follow plugin docs to create a working plugin (J7)
- `docs/` directory has no stale references to Python codebase

---

## 10. Stream 5I: Multi-Agent Routing (Week 5-9)

### Goal
Route inbound channels to isolated agents. Different users sharing one gateway get fully isolated AI "brains".

**Depends on**: Stream 5B item B5 (shared tool registry), Stream 5C Sub-Phase 5C.1 (plugin traits), Stream 5G item H1 (per-agent workspace)

| Week | Task | Item | Output |
|------|------|------|--------|
| 5 | Agent routing table: map channel identifiers to agent IDs | L1 | `[[agent_routes]]` config section |
| 6 | Per-agent workspace and session isolation | L2 | Dedicated `agentDir`, session store, skill overrides |
| 7 | Cross-agent shared memory namespace (opt-in) | L2 | Explicit sharing, no default cross-talk |
| 7 | Multi-agent swarming via `.swarm/` directory | L3 | Subtask delegation, coordinator pattern |
| 8 | Message bus integration for agent-to-agent communication | L3 | Results shared through bus |
| 9 | Planning strategies in Router: ReAct and Plan-and-Execute | L4 | Multi-step plan decomposition before execution |

**Validation**:
- Two Telegram users routed to different agents with isolated sessions (L1-L2)
- Agent A cannot access Agent B memory/workspace unless explicitly configured (L2)
- Coordinator agent spawns worker agents, collects results (L3)
- ReAct strategy: agent reasons, acts, observes, repeats (L4)

---

## 11. Stream 5J: Claude Flow Integration (Week 3-7)

### Goal
Make the delegation system's `DelegationTarget::Flow` path functional. Connect ClawFT to Claude Code and Claude Flow for multi-agent orchestration.

**Depends on**: Stream 5D item D9 (MCP transport concurrency)

### Week 3-4: Core Integration

| Week | Task | Item | Output |
|------|------|------|--------|
| 3 | Implement `FlowDelegator`: spawn `claude` CLI as subprocess | M1 | `claude --print` / `claude --json` transport |
| 3 | Stream results back through agent loop | M1 | Claude Code results integrated into conversation |
| 3 | Fallback to `ClaudeDelegator` (direct API) if `claude` binary unavailable | M1 | Graceful degradation |
| 4 | Wire `flow_available` to runtime detection (check `$PATH`, config, health) | M2 | Replace `let flow_available = false;` hardcode |
| 4 | Cache runtime detection result | M2 | No re-probe on every delegation decision |
| 4 | Enable `delegate` feature by default in `clawft-cli/Cargo.toml` | M3 | `register_delegation()` no longer a no-op stub |
| 4 | Set `claude_enabled` default to `true` (graceful degradation) | M3 | Works out of box if API key present |

### Week 5-7: Advanced Features

| Week | Task | Item | Output |
|------|------|------|--------|
| 5 | `weft mcp add <name> <command|url>` CLI command | M4 | Runtime MCP server registration |
| 5 | `weft mcp list` and `weft mcp remove` commands | M4 | MCP server management |
| 5 | Reconnection with exponential backoff for failed MCP connections | M4 | Resilient connections |
| 6 | Hot-reload: watch `clawft.toml` for `mcp_servers` changes | M4 | Live config updates |
| 6 | Bidirectional MCP bridge: clawft as MCP server for Claude Code | M5 | Claude Code connects to clawft tools |
| 7 | Bidirectional MCP bridge: clawft as MCP client to Claude Code | M5 | clawft uses Claude Code's tool ecosystem |
| 7 | Delegation config documentation in `docs/guides/` | M6 | End-user docs for delegation setup |

**Validation**:
- `FlowDelegator` spawns `claude` and returns streamed results (M1)
- `flow_available` returns `true` when `claude` is on `$PATH` and config enabled (M2)
- `delegate` feature compiles by default, degrades gracefully without API key (M3)
- `weft mcp add test-server -- npx test-server` registers and connects (M4)
- Claude Code can invoke clawft tools via MCP (M5 outbound)
- clawft can invoke Claude Code tools via MCP (M5 inbound)

---

## 12. Stream 5K: Deployment & Community (Week 8-12)

### Goal
Docker images, security hardening, ClawHub skill registry, and feature-parity benchmarks.

**Depends on**: Stream 5C (plugin system complete), Stream 5G item H2 (vector store for ClawHub search)

**Note**: K1 (Web Dashboard + Live Canvas) and K6 (Native Shells) are **out of scope** -- see `ui_development.md`.

### Forward-Compatibility Requirements (in scope)
- Agent loop and bus must support structured/binary payloads (future canvas rendering)
- MCP server tool schemas must be stable for future dashboard introspection
- Config and session APIs must be read-accessible without going through the agent loop

| Week | Task | Item | Output |
|------|------|------|--------|
| 8 | Multi-arch Docker images | K2 | `linux/amd64`, `linux/arm64` |
| 8 | One-click VPS deployment scripts | K2 | Docker Compose + provisioning |
| 9 | Enhanced sandbox with per-agent isolation (WASM + seccomp/landlock) | K3 | Per-skill permission system |
| 9 | Per-agent sandboxing: independent tool restrictions per agent | K3 | Agent A has shell, Agent B read-only |
| 10 | Security plugin: 50+ audit checks (prompt injection, exfiltration, credential leaks) | K3a | `weft security scan`, `weft security audit` |
| 10 | Security plugin: hardening modules (seccomp profiles, network restrict, domain allowlist) | K3a | `weft security harden` |
| 10 | Security plugin: background monitors (anomalous usage, excessive calls) | K3a | Real-time threat detection |
| 11 | ClawHub skill registry: publishing/installing skills | K4 | `weft skill publish`, `weft skill install` |
| 11 | ClawHub vector search for skill discovery | K4 | Semantic skill matching via H2 vector store |
| 11 | ClawHub agent auto-search: query when no local skill matches | K4 | Automatic skill discovery |
| 11 | ClawHub versioning (semver) + star/comment system | K4 | `weft skill update`, quality control |
| 12 | Feature parity test suite vs OpenClaw | K5 | Comprehensive comparison |
| 12 | Performance benchmarks: binary size, cold start, memory, throughput | K5 | Quantified advantages |

**Validation**:
- Docker image builds and runs on both amd64 and arm64 (K2)
- Sandbox blocks disallowed tool calls per-agent (K3)
- Security scan detects known prompt injection patterns (K3a)
- `weft skill install <name>` fetches from ClawHub and registers (K4)
- Vector search: `weft skill search "git operations"` returns git-related skills (K4)
- Benchmark report generated with comparison data (K5)

---

## 13. Phase 5 Milestone Checklist

### MVP Milestone (Week 8)

- [ ] All critical bugs fixed (A1-A9)
- [ ] All security issues resolved (A4-A6)
- [ ] All type safety improvements applied (I1-I8)
- [ ] Unified types across crates (B1, B2, B7)
- [ ] All files under 500 lines (B3 splits)
- [ ] `clawft-plugin` crate with trait definitions and manifest schema (C1)
- [ ] WASM plugin host functional (C2)
- [ ] Skill loader parses `SKILL.md` and auto-registers (C3)
- [ ] Skill precedence layering working (C4)
- [ ] Hot-reload watcher functional (C4)
- [ ] Email channel plugin working (E2)
- [ ] Parallel tool execution (D1)
- [ ] Streaming failover correctness (D2)
- [ ] Configurable retry policy (D4)
- [ ] Actual latency recording (D5)
- [ ] Per-agent workspace isolation (H1)
- [ ] Multi-agent routing table (L1)
- [ ] `FlowDelegator` functional (M1)
- [ ] `delegate` feature enabled by default (M3)
- [ ] Dynamic MCP server management (M4)
- [ ] Documentation sync complete (J1-J6)
- [ ] 3+ ported OpenClaw skills working

### Full Sprint Milestone (Week 12)

- [ ] Autonomous skill creation (C4a)
- [ ] MCP server exposes loaded skills (C6)
- [ ] Discord resume working (E1)
- [ ] WhatsApp, Signal, Matrix channels as plugins (E3-E5)
- [ ] Google Chat and Microsoft Teams channels (E5a, E5b)
- [ ] Git, Cargo, tree-sitter, browser plugins (F1-F4)
- [ ] Calendar and OAuth2 helper plugins (F5-F6)
- [ ] Docker/Podman orchestration plugin (F7)
- [ ] MCP client for external servers (F9)
- [ ] HNSW-backed vector store (H2.1)
- [ ] Production embedder (H2.2)
- [ ] RVF file I/O (H2.3)
- [ ] Memory export/import (H2.4)
- [ ] WITNESS segments (H2.6)
- [ ] Multi-agent swarming (L3)
- [ ] Planning strategies (ReAct, Plan-and-Execute) (L4)
- [ ] Bidirectional MCP bridge with Claude Code (M5)
- [ ] Delegation documentation complete (M6)
- [ ] Plugin system documentation (J7)
- [ ] Multi-arch Docker images (K2)
- [ ] Per-agent sandboxing (K3)
- [ ] Security plugin with 50+ audit checks (K3a)
- [ ] ClawHub with vector search (K4)
- [ ] Feature parity benchmarks vs OpenClaw (K5)

### Out of Scope (Separate Tracks)

- [ ] Voice (Workstream G) -- see `voice_development.md`
- [ ] Web Dashboard + Live Canvas (K1) -- see `ui_development.md`
- [ ] Native Shells (K6) -- see `ui_development.md`

---

## 14. Stream Dependency Diagram

```
Stream 5A (Critical Fixes+Types) -----> Stream 5B (Architecture) -----> Stream 5C (Plugin System)
  Week 1-2                                Week 2-4          |             Week 3-8
     |                                       |              |                |
     | A2 (hash fix)                         | B5 (tool     |                | C1 (traits)
     |                                       |  registry)   |                |
     v                                       v              |                v
Stream 5G (Memory)                   Stream 5I (Multi-Agent) |         Stream 5E (Channels)
  Week 4-8                             Week 5-9             |           Week 4-8
                                                            |                |
Stream 5D (Pipeline) -----> Stream 5J (Claude Flow)         |                | C1 (traits)
  Week 2-5       |           Week 3-7                       |                |
     |           |              |                           |                v
     | D9        |              | M4 (dynamic MCP)          |         Stream 5F (Dev Tools)
     |           |              v                           |           Week 5-10
     |           +-------> Stream 5F item F9                |
     |                     (MCP client)                     |
     |                                                      |
     v                                                      v
Stream 5H (Docs)                                     Stream 5K (Deployment)
  Week 3-5                                             Week 8-12
  (follows feature completion)                           |
                                                         | C2, H2, K3
                                                         v
                                                    K4 (ClawHub)
                                                    K5 (Benchmarks)
```

### Key Dependency Chains

```
A4 (cred redaction) -----> E2 (email OAuth)
A2 (stable hash) --------> H2 (vector memory)
B4 (cron unify) ---------> E6 (heartbeat)
B5 (tool registry) ------> L1 (agent routing)
C1 (plugin traits) ------> C2, C3, C7, E2-E5b, F1-F7, H1, L1
C2 (WASM host) ----------> C4 (dynamic loading) -----> C4a (autonomous creation)
C3 (skill loader) -------> C4, C5, C6, K4
C4 (dynamic load) -------> C4a (auto-create)
C6 (MCP skill expose) ---> F8 (IDE integration)
D6 (sender_id) ----------> L4 (planning strategies)
D9 (MCP transport) ------> M1 (FlowDelegator), F9 (MCP client)
H1 (workspace) ----------> L2 (per-agent isolation)
H2 (vector store) -------> K4 (ClawHub vector search)
K3 (sandbox) ------------> K3a (security plugin)
L1 (routing) ------------> L2 (isolation) -----> L3 (swarming)
M1 (FlowDelegator) ------> M5 (bidirectional bridge)
M4 (dynamic MCP) --------> M5 (bidirectional bridge)
```

---

## 15. Cross-Cutting Concerns

These apply across all Phase 5 streams:

1. **Keep core tiny** -- Heavy deps (`wasmtime`, `chromiumoxide`, `git2`) go in optional plugins behind feature flags. Target: <10 MB base binary, sub-100ms cold start.
2. **Offline capability** -- All local-first where possible. Cloud is always a fallback, never required.
3. **No core forks** -- After Stream 5C, all new capabilities are plugins. No more modifying `clawft-core` for features.
4. **Forward compatibility** -- Voice and UI hooks built into the plugin system (C1) and channel adapter traits so they can be added without breaking changes.

### Recommended New Dependencies (minimal, Rust-native)

| Crate | Stream | Purpose |
|-------|--------|---------|
| `wasmtime` + `wit-bindgen` | 5C | Plugin host |
| `lettre`, `imap` | 5E | Email channel |
| `chromiumoxide` | 5F | Browser automation |
| `git2` | 5F | Git operations |
| `oauth2` | 5E, 5F | Auth flows |
| `tree-sitter` | 5F | Code analysis |
| `notify` | 5C | File-system watcher for hot-reload |
| `fnv` or `xxhash` | 5A | Stable deterministic hash |
| `rpassword` | 5A | Secure terminal input |
| `temp_env` | 5A | Safe env var mutation in tests |
| `instant-distance` or `hnsw` | 5G | HNSW vector index |

---

## 16. Risk Mitigation

| Risk | Detection | Response |
|------|-----------|----------|
| Plugin trait API changes after channels start | Breaking `cargo build` in 5E/5F | Immediate fix in `clawft-plugin` + notify all streams |
| WASM plugin size exceeds 300KB budget | CI size check | Profile with `twiggy`, split plugin, lazy-load components |
| Skill precedence conflicts across layers | Discovery integration tests | Warn on collision in debug mode, document override rules |
| `FlowDelegator` subprocess hangs | Timeout in integration test | Kill subprocess after configurable timeout, log stderr |
| ClawHub registry latency impacts agent responsiveness | Benchmark skill install time | Cache index locally, async background fetch |
| Security plugin false positives block legitimate skills | Audit check integration tests | Configurable severity thresholds, allowlist for trusted skills |
| Vector store memory usage at scale | Memory profiling under load | Temperature-based quantization (H2.7), configurable index size |

---

## 17. File Ownership

To avoid merge conflicts, each stream owns specific areas:

| Stream | Owned Areas |
|--------|-------------|
| 5A | Bug fixes across all crates (targeted patches), `clawft-types` serde fixes |
| 5B | `clawft-types` (type unification, splits), `clawft-core` (structural splits), `clawft-services/src/cron_service/` |
| 5C | New `clawft-plugin` crate, `clawft-wasm` updates, `clawft-core/src/agent/skills_v2.rs`, `clawft-channels/src/host.rs`, `clawft-cli/src/interactive/` |
| 5D | `clawft-core/src/agent/loop_core.rs` (perf), `clawft-core/src/pipeline/` (reliability), `clawft-core/src/bus.rs`, `clawft-llm/src/` |
| 5E | `clawft-channels/` (existing + new plugins), `clawft-services/src/heartbeat/` |
| 5F | New plugin crates (git, cargo, browser, calendar, etc.), `clawft-services/src/mcp/` (client) |
| 5G | `clawft-core/src/embeddings/`, `clawft-core/src/workspace.rs`, `clawft-types` (timestamp unification) |
| 5H | `docs/` directory exclusively |
| 5I | `clawft-core/src/agent/` (routing), `clawft-types/src/routing.rs` (agent routes) |
| 5J | `clawft-services/src/delegation/`, `clawft-tools/src/delegate_tool.rs`, `clawft-cli/Cargo.toml` (feature flags) |
| 5K | `Dockerfile`, deployment scripts, `clawft-services/src/mcp/middleware.rs` (sandbox), new security plugin crate |

### Branching Strategy

- `master` -- stable, always compiles (do not commit directly)
- `sprint/phase-5` -- integration branch for all Phase 5 work
- `sprint/5a-fixes` -- Stream 5A work
- `sprint/5b-cleanup` -- Stream 5B work
- `sprint/5c-plugins` -- Stream 5C work
- `sprint/5d-pipeline` -- Stream 5D work
- `sprint/5e-channels` -- Stream 5E work
- `sprint/5f-devtools` -- Stream 5F work
- `sprint/5g-memory` -- Stream 5G work
- `sprint/5h-docs` -- Stream 5H work
- `sprint/5i-routing` -- Stream 5I work
- `sprint/5j-claude-flow` -- Stream 5J work
- `sprint/5k-deploy` -- Stream 5K work

Merge to `sprint/phase-5` when stream milestones pass. Merge `sprint/phase-5` to `master` when MVP or Full Sprint milestone passes.

---

## 18. Definition of Done (Phase 5)

A task is done when:
1. Code compiles without warnings (`cargo clippy --workspace`)
2. All tests pass (`cargo test --workspace`)
3. No new `unsafe` blocks without justification
4. Public API has doc comments
5. Feature-gated code has appropriate `cfg` attributes
6. No hardcoded secrets or paths
7. Binary size regression checked (for release builds)
8. New plugins implement `clawft-plugin` traits (not core modifications)
9. New channels register through `ChannelAdapter` trait
10. Plugin dependencies are behind feature flags
