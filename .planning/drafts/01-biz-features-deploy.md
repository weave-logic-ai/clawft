# Business Requirements: Features, Deployment & Integration (Draft)

> Supplement to [01-business-requirements.md](../01-business-requirements.md).
> Covers Workstreams F, H, K (in-scope: K2-K5), L, M, and J from the
> [Unified Sprint Plan](../improvements.md).

---

## 5d. Software Dev & App Tooling (Workstream F)

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| DT-1 | As a developer, I want my AI agent to create branches, commits, and PRs directly so that I can automate version-control workflows without leaving the conversation | P1 |
| DT-2 | As a developer, I want the agent to run `cargo build`, `cargo test`, and `cargo clippy` and report results inline so I can iterate on code quality within the agent loop | P1 |
| DT-3 | As a developer, I want AST-level code analysis via tree-sitter so the agent can reason about code structure, find usages, and refactor safely | P2 |
| DT-4 | As a user, I want my agent to browse the web, fill forms, take screenshots, and scrape content using headless Chrome so it can interact with web applications autonomously | P2 |
| DT-5 | As a user, I want the agent to manage my calendar (create events, check availability, reschedule) via Google Calendar or Outlook APIs | P3 |
| DT-6 | As a developer, I want a reusable OAuth2 helper plugin so every API integration (email, calendar, Google Chat, Teams) does not reimplement auth flows | P1 |
| DT-7 | As a DevOps engineer, I want the agent to manage Docker/Podman containers (build, run, stop, inspect) from within a conversation | P3 |
| DT-8 | As a developer, I want my agent to function as a VS Code extension backend via MCP so I can use agent-driven edits live in my IDE | P2 |
| DT-9 | As a power user, I want to connect to 1000+ community MCP servers (Google Drive, Slack, databases, enterprise systems) for expanded capabilities without writing custom tools | P1 |

### Feature Summary

| Feature | Description | Plugin Crate / Dep |
|---------|-------------|-------------------|
| Git tool (F1) | Clone, commit, branch, PR, diff, blame via `git2` | `clawft-plugin-git` / `git2` |
| Cargo/build (F2) | Build, test, clippy, publish as skill with tool calls | `clawft-plugin-cargo` |
| Code analysis (F3) | AST parsing + LSP client via `tree-sitter` | `clawft-plugin-treesitter` / `tree-sitter` |
| Browser CDP (F4) | Headless Chrome: screenshot, form fill, scraping | `clawft-plugin-browser` / `chromiumoxide` |
| Calendar (F5) | Google Calendar, Outlook, iCal via APIs | `clawft-plugin-calendar` / `oauth2` |
| OAuth2 helper (F6) | Reusable OAuth2 flow for all API integrations | `clawft-plugin-oauth2` / `oauth2` |
| Container mgmt (F7) | Docker/Podman lifecycle from agent context | `clawft-plugin-containers` |
| MCP IDE integration (F8) | VS Code extension backend via MCP server | `clawft-services` (MCP layer) |
| MCP client (F9) | Connect to external MCP servers as a client | `clawft-services` (MCP layer) |

### Non-Goals (Dev Tools)

- Full IDE replacement (code editing is tool-assisted, not a standalone IDE)
- Direct CI/CD pipeline management (agent can trigger builds, not own the pipeline)
- Git hosting (no built-in Gitea/GitLab -- use external services)
- Browser GUI rendering for the user (CDP is headless, agent-side only)

---

## 5e. Memory & Workspace (Workstream H)

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| MW-1 | As a multi-agent user, I want each agent to have isolated memory, personality (`SOUL.md`), and session state so that agents do not leak context between different roles | P1 |
| MW-2 | As a user, I want semantic search over my conversation history and memories so I can find relevant past interactions by meaning, not just keywords | P1 |
| MW-3 | As a user, I want the agent to auto-summarize long conversations and persist summaries into the workspace so context is preserved across sessions without re-reading entire logs | P2 |
| MW-4 | As an admin, I want to export and import memory snapshots (`weft memory export` / `weft memory import`) for backup, migration, or sharing between installations | P2 |
| MW-5 | As a security-conscious user, I want tamper-evident audit trails (witness segments) for all agent actions so I can verify that no actions were retroactively modified | P3 |
| MW-6 | As a user running on constrained hardware, I want temperature-based quantization (hot/warm/cold tiers) so that infrequently accessed vectors consume less storage and RAM | P3 |
| MW-7 | As a developer targeting edge/browser deployment, I want vector search to work in WASM via a micro-HNSW engine (< 8 KB) | P3 |
| MW-8 | As a user, I want consistent timestamp formats across all memory, cron, and session data so queries and sort operations work reliably | P1 |

### Feature Summary

| Feature | Description | Sprint Ref |
|---------|-------------|-----------|
| Per-agent workspace | `~/.clawft/agents/<agentId>/` with dedicated SOUL.md, AGENTS.md, USER.md, sessions | H1 |
| Vector memory (HNSW) | HNSW-backed VectorStore replacing brute-force cosine scan | H2.1 |
| Production embedder | LLM embedding API (OpenAI / local ONNX) replacing HashEmbedder | H2.2 |
| RVF file I/O | Real RVF segment read/write for memory persistence | H2.3 |
| Memory export/import | `weft memory export` / `weft memory import` CLI commands | H2.4 |
| Policy persistence | Persist IntelligentRouter routing policies across restarts | H2.5 |
| Witness segments | Tamper-evident audit trail of agent actions | H2.6 |
| Temp-based quantization | Hot (fp16) / warm (PQ) / cold (binary) tiers by access frequency | H2.7 |
| WASM micro-HNSW | `micro-hnsw-wasm` for browser and edge deployments | H2.8 |
| Timestamp unification | Standardize on `DateTime<Utc>` across all types | H3 |

### Non-Goals (Memory & Workspace)

- Cloud-synced memory (all storage is local-first; cloud backup is user responsibility)
- Real-time collaborative memory across multiple running instances
- Natural language memory editing ("forget that I said X") -- search and manual delete only
- Full graph database for memory (vector + JSONL is the storage model)

---

## 5f. Deployment & Community (Workstream K, In-Scope: K2-K5)

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| DC-1 | As a self-hoster, I want official multi-arch Docker images so I can deploy clawft with `docker run` on any platform without compiling from source | P1 |
| DC-2 | As a self-hoster, I want one-click VPS deployment scripts so I can go from zero to running agent in under 5 minutes | P2 |
| DC-3 | As an admin, I want per-agent sandboxing where each agent has independent tool restrictions (e.g., Agent A has shell access, Agent B is read-only) configurable via per-agent config | P1 |
| DC-4 | As an admin, I want WASM + seccomp/landlock sandboxing for plugins so that untrusted skills cannot escape their execution context | P1 |
| DC-5 | As an admin, I want automated security scanning of installed skills (50+ audit checks for prompt injection, data exfiltration, credential leaks) via `weft security scan` | P1 |
| DC-6 | As an admin, I want hardening modules that auto-apply seccomp/landlock profiles and enforce per-skill network allowlists | P2 |
| DC-7 | As an admin, I want background monitors watching for anomalous tool usage, excessive API calls, and unexpected file access patterns | P2 |
| DC-8 | As a community member, I want to discover, install, and rate skills from a central registry (ClawHub) using `weft skill install` and `weft skill search` | P2 |
| DC-9 | As a community member, I want to publish my own skills to ClawHub with versioning, star/comment ratings, and moderation | P2 |
| DC-10 | As a user, I want the agent to automatically search ClawHub when it cannot find a matching local skill, and offer to install relevant community skills | P3 |
| DC-11 | As a developer, I want a feature-parity benchmark suite comparing clawft against OpenClaw on binary size, cold start, memory usage, and throughput | P3 |

### Feature Summary

| Feature | Description | Sprint Ref |
|---------|-------------|-----------|
| Docker images | Multi-arch images (linux/amd64, linux/arm64); one-click VPS scripts | K2 |
| Per-agent sandbox | Independent tool restrictions per agent via agent config | K3 |
| WASM + OS sandbox | seccomp/landlock + WASM isolation for untrusted plugins | K3 |
| Security plugin | 50+ audit checks, hardening modules, background monitors, CLI integration | K3a |
| ClawHub registry | Central skill registry with HTTP index, git clone, vector search discovery | K4 |
| ClawHub social | Star/comment system, moderation hooks, versioning (semver) | K4 |
| Agent auto-search | Automatic ClawHub query when no local skill matches | K4 |
| Benchmark suite | Feature parity + performance comparison vs OpenClaw | K5 |

### Non-Goals (Deployment & Community)

- Kubernetes operator or Helm chart (Docker images are the deployment unit; orchestration is user-managed)
- Hosted/SaaS ClawHub (the registry is a self-hostable HTTP index + git backend)
- Automated skill monetization or payment processing on ClawHub
- Web dashboard for deployment management (deferred to UI workstream)

---

## 5g. Multi-Agent Routing & Orchestration (Workstream L)

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| MA-1 | As a multi-user gateway operator, I want different users (identified by channel-specific IDs like WhatsApp number or Telegram user ID) routed to different agents with fully isolated "brains" | P1 |
| MA-2 | As a user with multiple roles, I want to define agent routing rules in `clawft.toml` mapping channel/account/peer identifiers to specific agent IDs | P1 |
| MA-3 | As an agent operator, I want each routed agent to have its own workspace directory (`~/.clawft/agents/<agentId>/`), session store, skill overrides, and personality files | P1 |
| MA-4 | As a developer, I want agents to delegate subtasks to other agents via the message bus, enabling coordinator/worker patterns for complex tasks | P2 |
| MA-5 | As a developer, I want the pipeline Router to support ReAct (Reason+Act) and Plan-and-Execute strategies so the agent can decompose complex requests into multi-step plans before execution | P3 |

### Feature Summary

| Feature | Description | Sprint Ref |
|---------|-------------|-----------|
| Agent routing table | Config-driven mapping of channel identifiers to agent IDs | L1 |
| Per-agent isolation | Dedicated agentDir, sessions, skills, SOUL.md per routed agent | L2 |
| Multi-agent swarming | Agent delegation via message bus; coordinator/worker pattern | L3 |
| Planning strategies | ReAct and Plan-and-Execute in pipeline Router | L4 |

### Configuration Example

```toml
[[agent_routes]]
channel = "telegram"
match = { user_id = "12345" }
agent = "work-agent"

[[agent_routes]]
channel = "whatsapp"
match = { phone = "+1..." }
agent = "personal-agent"

[[agent_routes]]
channel = "slack"
match = { workspace_id = "T01ABC" }
agent = "team-agent"
```

### Non-Goals (Multi-Agent)

- Cross-agent memory sharing by default (isolation is the default; sharing is opt-in via explicit config)
- Automated agent creation (routing rules are manually configured)
- Load balancing across duplicate agent instances (single-instance per agent ID)
- Agent-to-agent authentication (all agents share the same trust domain within a single deployment)

---

## 5h. Claude Flow / Claude Code Integration (Workstream M)

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| CF-1 | As a Claude Code user, I want clawft to delegate complex tasks to Claude Code by spawning `claude` as a subprocess so that I get Claude Code's full tool-use capabilities from within clawft | P1 |
| CF-2 | As a developer, I want `flow_available` to be dynamically detected at runtime (check `claude` on PATH, probe health endpoint, cache result) instead of hardcoded `false` | P1 |
| CF-3 | As a developer, I want the `delegate` feature enabled by default so delegation works out of the box without manual feature flag configuration | P1 |
| CF-4 | As a power user, I want to manage MCP server connections at runtime (`weft mcp add`, `weft mcp list`, `weft mcp remove`) without restarting the agent | P2 |
| CF-5 | As a developer, I want clawft and Claude Code to share tools bidirectionally -- clawft exposes tools to Claude Code as an MCP server, and consumes Claude Code's tools as an MCP client | P2 |
| CF-6 | As a developer, I want MCP connections to auto-reconnect with exponential backoff and health-check pings so stale connections are detected and recovered | P2 |
| CF-7 | As a user, I want the delegation system documented end-to-end: how to enable it, write routing rules, configure excluded tools, set up Claude Code integration, and troubleshoot failures | P1 |

### Feature Summary

| Feature | Description | Sprint Ref |
|---------|-------------|-----------|
| FlowDelegator | Spawn `claude` CLI as subprocess; stream results back; fallback to direct API | M1 |
| Runtime flow detection | Replace hardcoded `false` with PATH check + config flag + health probe | M2 |
| Delegate feature default | Add `delegate` to default features; `claude_enabled` defaults to `true` | M3 |
| Dynamic MCP management | `weft mcp add/list/remove`; reconnection with backoff; config hot-reload | M4 |
| Bidirectional MCP bridge | clawft as MCP server for Claude Code + clawft as MCP client to Claude Code | M5 |
| Delegation documentation | End-to-end guide for delegation config, routing rules, Claude Code setup | M6 |

### Configuration Example

```toml
[delegation]
claude_enabled = true
claude_flow_enabled = true

[delegation.routing_rules]
# Regex patterns map to delegation targets
"^(refactor|architect).*" = "flow"
"^(translate|summarize).*" = "local"

[tools.mcp_servers.claude-code]
command = "claude"
args = ["mcp", "serve"]
```

### Non-Goals (Claude Flow Integration)

- Running Claude Flow as a persistent daemon managed by clawft (Claude Code manages its own lifecycle)
- Automatic Claude Code installation (user must install `claude` CLI separately)
- Proxying Claude Code's billing or usage tracking through clawft
- Supporting non-Anthropic external code agents as delegation targets (Claude Code/Flow only in this workstream)

---

## 5i. Documentation Sync (Workstream J)

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| DS-1 | As a new user reading docs, I want provider counts to match the actual codebase so I am not confused about supported providers | P1 |
| DS-2 | As a developer, I want the assembler truncation behavior documented accurately so I understand how token budgets are enforced | P1 |
| DS-3 | As a developer, I want the token budget source (`max_context_tokens` in routing tiers) documented correctly so I configure routing without guessing | P1 |
| DS-4 | As a user, I want to know that placing `SOUL.md` or `IDENTITY.md` in workspace root or `.clawft/` overrides the default agent preamble, so I can customize agent personality | P1 |
| DS-5 | As an operator, I want rate-limit retry behavior (3 retries, 500ms minimum wait) documented so I can tune it or account for it in provider budgets | P2 |
| DS-6 | As a developer, I want to know the default CLI log level is `warn` (not `info`) so I set `--verbose` when debugging | P2 |
| DS-7 | As a plugin developer, I want full documentation for the plugin/skill system (architecture, creating plugins, SKILL.md format, ClawHub, WASM compilation) | P2 |

### Feature Summary

| Fix / Addition | Files Affected | Sprint Ref |
|---------------|----------------|-----------|
| Provider count correction | `docs/architecture/overview.md`, `docs/guides/providers.md`, `docs/getting-started/quickstart.md`, `docs/reference/config.md`, `clawft-types/src/lib.rs` | J1 |
| Assembler truncation accuracy | `docs/architecture/overview.md` | J2 |
| Token budget source reference | `docs/guides/routing.md` | J3 |
| Identity bootstrap behavior | `docs/guides/skills-and-agents.md` or `docs/guides/configuration.md` | J4 |
| Rate-limit retry behavior | `docs/guides/providers.md` | J5 |
| CLI log level change | `docs/reference/cli.md` | J6 |
| Plugin system documentation | New: `docs/guides/plugins.md`, `docs/guides/skill-format.md`, `docs/guides/clawhub.md` | J7 |

### Non-Goals (Documentation)

- Auto-generated API reference from rustdoc (valuable but separate from content accuracy fixes)
- Translated documentation (English only for now)
- Video tutorials or interactive guides

---

## 6. Forward Compatibility (Out-of-Scope Features Requiring Hooks)

The following features are out of scope for the current sprint but require forward-compatible hooks in the architecture so they can be added later without breaking changes.

### 6a. Voice (Workstream G -- Deferred)

Full plan in `voice_development.md`. The following hooks MUST be built during the current sprint:

| Requirement | Where | Rationale |
|-------------|-------|-----------|
| `VoiceHandler` trait placeholder in `clawft-plugin` trait crate (C1) | `clawft-plugin/src/traits.rs` | Plugin system must account for voice from day one so the trait surface is stable when voice is implemented |
| Plugin manifest schema reserves `voice` capability type | `clawft-plugin` manifest schema | Manifest parsing must not reject voice-typed plugins; it just ignores them until the voice feature is enabled |
| `ChannelAdapter` trait supports binary (audio) payloads | `clawft-plugin/src/traits.rs` | Voice and media messages require binary payloads; text-only adapter trait would force a breaking change later |
| Feature flag `voice` wired as empty/no-op in Cargo.toml | Workspace `Cargo.toml` files | Feature flag exists in the matrix so downstream `cfg` guards compile, even though the feature does nothing yet |

### 6b. UI / Web Dashboard / Native Shells (Workstream K -- K1, K6 Deferred)

Full plan in `ui_development.md`. The following hooks MUST be built during the current sprint:

| Requirement | Where | Rationale |
|-------------|-------|-----------|
| Agent loop and bus support structured/binary payloads (not text-only) | `clawft-core` bus and agent loop | Future canvas rendering needs structured payloads; text-only assumptions would force a rewrite |
| MCP server tool schemas stable enough for dashboard introspection | `clawft-services/src/mcp/` | A future dashboard will discover tools via MCP; schema churn breaks the integration |
| Config and session APIs readable without going through the agent loop | Config/session modules | Future dashboard needs direct read access to config and sessions for display, not agent-mediated access |

---

## 7. Updates to Existing Sections

### 7a. Non-Goals Updates

The following items should be REMOVED or REVISED in the Non-Goals section of `01-business-requirements.md`:

| Current Non-Goal | Action | Reason |
|-----------------|--------|--------|
| "GUI or web dashboard" | **Revise** to: "GUI or web dashboard (deferred to post-sprint; see `ui_development.md`)" | UI is planned but deferred, not permanently out of scope |
| "Dynamic plugin loading (`.so`/`.dll` at runtime) -- compile-time feature flags only" | **Revise** to: "Dynamic native plugin loading (`.so`/`.dll`); WASM plugin loading IS in scope (Workstream C2)" | WASM plugins (`.wasm` at runtime) are now a core feature; native dynamic loading remains out of scope |
| "WhatsApp channel in initial release (punted to future phase)" | **Remove** | WhatsApp channel is now in scope as Workstream E3 |

### 7b. Risk Register Additions

The following risks should be ADDED to the Risk Register in `01-business-requirements.md`:

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| WASM plugin sandbox escape | Low | Critical | 8 | Defense-in-depth: WASM isolation + seccomp/landlock + per-skill permission allowlists (K3, K3a); regular security audits |
| ClawHub supply-chain attack (malicious skill) | Medium | High | 6 | Mandatory security scan on install (K3a); star/review system for community vetting; signed manifests for verified publishers |
| Claude Code CLI unavailable or API-breaking changes | Medium | Medium | 4 | FlowDelegator falls back to direct Anthropic API (ClaudeDelegator); version pinning for `claude` CLI; integration tests in CI |
| MCP protocol version drift between clawft and external servers | Medium | Medium | 4 | Pin to stable MCP protocol version (B9); version negotiation handshake on connect; graceful degradation for unknown capabilities |
| Multi-agent routing misconfiguration leaks context | Medium | High | 6 | Strict workspace isolation by default (L2); no cross-agent memory unless explicitly configured; audit logging of all routing decisions |
| Browser CDP plugin security exposure | Medium | High | 6 | Sandboxed in separate process; domain allowlist required; no persistent browser state by default; explicit user opt-in for form fill |
| Vector memory corruption during HNSW index rebuild | Low | High | 5 | Write-ahead log before index mutation; periodic checkpoints; `weft memory export` as backup mechanism |
| Skill hot-reload race condition | Medium | Medium | 4 | Atomic swap of skill registry; drain in-flight tool calls before swap; filesystem watcher debounce |
| OAuth2 token storage security | Medium | High | 6 | Tokens stored encrypted at rest; OS keyring integration where available; memory-only fallback with short TTL |
| Planning strategy (ReAct/Plan-and-Execute) infinite loops | Medium | Medium | 4 | Configurable max planning depth; cost budget acts as hard stop; timeout per planning step |

---

## 8. Success Criteria Additions

### Plugin & Dev Tools (Phase 5)

- [ ] At least 3 dev tool plugins (git, cargo, tree-sitter) functional and MCP-exposed
- [ ] Browser CDP plugin can screenshot a URL and return image data to the agent
- [ ] OAuth2 helper plugin used by at least 2 other plugins (email + calendar)
- [ ] MCP client connects to at least 3 external MCP servers and exposes their tools to the agent
- [ ] `weft mcp add <name> <command|url>` dynamically adds MCP server without restart

### Memory & Workspace (Phase 5)

- [ ] HNSW vector search returns results in < 10ms for 100K vectors
- [ ] Per-agent workspace isolation verified: Agent A's memory is inaccessible from Agent B's context
- [ ] `weft memory export` / `weft memory import` round-trips without data loss
- [ ] All timestamp fields use `DateTime<Utc>` (no mixed i64/String representations)
- [ ] HashEmbedder replaced with production LLM embedding API

### Deployment & Community (Phase 6)

- [ ] Docker image published for linux/amd64 and linux/arm64
- [ ] `docker run clawft/weft gateway` starts a functional agent
- [ ] `weft security scan` detects at least 5 known-bad patterns in a test skill corpus
- [ ] Per-agent sandbox enforced: Agent with `tools.allow = ["read_file"]` cannot call `exec`
- [ ] ClawHub prototype serves skill index over HTTP; `weft skill install <name>` works end-to-end
- [ ] Benchmark suite produces comparison table: clawft vs OpenClaw on binary size, cold start, RSS, throughput

### Multi-Agent Routing (Phase 5)

- [ ] Agent routing table correctly isolates 3+ agents on a single gateway instance
- [ ] Telegram user A and Telegram user B on the same gateway get independent sessions and memory
- [ ] Agent delegation via message bus: coordinator agent dispatches subtask and receives result

### Claude Flow Integration (Phase 5)

- [ ] FlowDelegator spawns `claude` subprocess and returns streamed result to agent loop
- [ ] `flow_available` dynamically detects `claude` on PATH (no hardcoded `false`)
- [ ] `delegate` feature enabled by default in `clawft-cli` release builds
- [ ] Bidirectional MCP bridge: clawft tool callable from Claude Code AND Claude Code tool callable from clawft
- [ ] Delegation config documented in `docs/guides/configuration.md`

### Documentation Sync (Phase 4)

- [ ] All provider counts in docs match actual `PROVIDERS` array length
- [ ] Assembler truncation behavior accurately described in architecture docs
- [ ] Token budget source references `max_context_tokens` (not `agents.defaults.max_tokens`)
- [ ] `SOUL.md` / `IDENTITY.md` override behavior documented
- [ ] Plugin system has dedicated documentation covering architecture, SKILL.md format, and ClawHub

---

## Appendix: Story ID Reference

| Prefix | Domain | Workstream |
|--------|--------|------------|
| DT-* | Dev Tools | F |
| MW-* | Memory & Workspace | H |
| DC-* | Deployment & Community | K (K2-K5) |
| MA-* | Multi-Agent Routing | L |
| CF-* | Claude Flow Integration | M |
| DS-* | Documentation Sync | J |
| WS-* | Workspace (existing) | -- |
| TR-* | Tiered Routing (existing) | -- |
| G* | Goals (existing) | -- |
