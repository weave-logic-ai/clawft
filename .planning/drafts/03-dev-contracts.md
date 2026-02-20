# Development Guide Updates: Phase 5 Contracts & Rules

> Draft updates to Sections 5-8 of `03-development-guide.md` covering the unified sprint plan
> from `improvements.md` (Workstreams A through M). These sections should be appended or merged
> into the existing development guide once reviewed.

---

## 5. Concurrent Development Rules (Phase 5 Additions)

### Dependency Management Between Streams

```
Workstream A (Critical Fixes) -----> All feature workstreams (B-M)
                               -----> B (Architecture Cleanup) can start Week 2 in parallel

Workstream B (Architecture Cleanup)
  B3 (File Splits) -----> C1 (Plugin Traits) -- large files must split before trait extraction
  B5 (Tool Registry Builder) -----> L1 (Agent Routing Table)
  B4 (Cron Unification) -----> E6 (Heartbeat Enhancement)

Workstream C (Plugin & Skill System) -- CENTRAL DEPENDENCY HUB
  C1 (Plugin Trait Crate) -----> C2 (WASM Host)
                           -----> C3 (Skill Loader)
                           -----> C7 (PluginHost Unification)
                           -----> E2-E5b (All New Channel Plugins)
                           -----> F1-F7 (All New Tool Plugins)
                           -----> H1 (Markdown Workspace)
                           -----> L1 (Agent Routing Table)
                           -----> K3a (Security Plugin)
  C2 (WASM Host) -----> C4 (Dynamic Loading & Hot-Reload)
                  -----> K3 (Enhanced Sandbox)
  C3 (Skill Loader) -----> C4 (Dynamic Loading)
                     -----> C5 (Slash-Command Framework)
                     -----> C6 (MCP Server for Skills)
  C4 (Dynamic Loading) -----> C4a (Autonomous Skill Creation)
                        -----> K4 (ClawHub Registry)
  C6 (MCP Server for Skills) -----> F8 (Deep IDE Integration)

Workstream D (Pipeline & LLM Reliability)
  D6 (Thread sender_id) -----> L4 (Planning Strategies)
  D9 (MCP Transport) -----> M1 (FlowDelegator)
                      -----> F9 (MCP Client for External Servers)

Workstream E (Channel Enhancements) -- each plugin independent after C1
  E2 (Email) requires A4 (credential fix) + C1
  E3-E5b require C1 only

Workstream F (Dev & App Tooling) -- each plugin independent after C1
  F8 (IDE Integration) requires C6
  F9 (MCP Client) requires D9

Workstream H (Memory & Workspace)
  H1 (Markdown Workspace) requires C1
  H2 (Vector Memory) requires A2 (hash fix)

Workstream I (Type Safety) -- parallel, independent of feature work
  Can start Week 2, no external deps

Workstream J (Documentation) -- follows feature completion
  J7 (Plugin Docs) requires C1-C6

Workstream K (Deployment & Community)
  K3 (Sandbox) requires C2
  K3a (Security Plugin) requires C1 + K3
  K4 (ClawHub) requires C3 + C4 + H2

Workstream L (Multi-Agent Routing)
  L1 (Routing Table) requires B5 + C1
  L2 (Per-Agent Isolation) requires L1 + H1
  L3 (Multi-Agent Swarming) requires L1 + L2
  L4 (Planning Strategies) requires D6

Workstream M (Claude Flow Integration)
  M1 (FlowDelegator) requires D9
  M2 (Wire flow_available) -- standalone fix
  M3 (Enable delegate feature) -- standalone config fix
  M4 (Dynamic MCP Discovery) requires F9
  M5 (Claude Code MCP Bridge) requires M1 + M4
```

**Key dependency observations:**

- **C1 (Plugin Trait Crate) is the single most critical blocker.** It gates all new channel plugins (E2-E5b), all new tool plugins (F1-F7), workspace features (H1), agent routing (L1), security plugins (K3a), and the PluginHost unification (C7). C1 must be treated as highest priority after Workstream A fixes.
- **Workstream A (Critical Fixes) should complete before feature work builds on top.** Specifically: A2 (hash fix) blocks H2 (vector memory), A4 (credential fix) blocks E2 (email channel), and A1-A9 collectively ensure the foundation is stable.
- **Workstream B3 (File Splits) must precede C1.** The 1400-line `config.rs` and 1100+ line trait files need to be split before plugin traits can be cleanly extracted alongside existing types.
- **Workstreams I, J, and parts of D are independent** and can run in parallel with the C-gated feature work.
- **Workstream M (Claude Flow) has a separate dependency chain** through D9 and F9 that does not block the plugin system. M2 and M3 are standalone fixes that can land immediately.

### Interface Contracts

Before coding begins, these interfaces must be agreed upon (extends existing contracts 1-12):

13. **Plugin traits** (`clawft-plugin`): Unified extensibility interfaces
    - `Tool`: `name()`, `description()`, `parameters() -> JsonSchema`, `execute(input: Value) -> Result<Value>`, `permissions() -> ToolPermissions`
    - `ChannelAdapter`: `name()`, `start()`, `stop()`, `send(msg: OutboundMessage)`, `is_running()`, `supports_binary() -> bool`
    - `PipelineStage`: `name()`, `process(ctx: &mut PipelineContext) -> Result<()>`, `priority() -> i32`
    - `Skill`: `name()`, `description()`, `triggers() -> Vec<TriggerPattern>`, `execute(ctx: SkillContext) -> Result<SkillOutput>`, `source() -> SkillSource`
    - `MemoryBackend`: `store(key, value)`, `retrieve(key)`, `search(query, limit)`, `delete(key)`, `list(namespace)`
    - `VoiceHandler` (placeholder): `transcribe(audio: &[u8]) -> Result<String>`, `synthesize(text: &str) -> Result<Vec<u8>>`, `is_available() -> bool`

14. **Plugin manifest schema** (`clawft.plugin.json` / `clawft.plugin.yaml`):
    ```json
    {
      "name": "plugin-name",
      "version": "0.1.0",
      "description": "...",
      "capabilities": ["tool", "channel", "pipeline_stage", "skill", "memory", "voice"],
      "wasm": { "file": "plugin.wasm", "max_size_kb": 300 },
      "skills_dir": "skills/",
      "permissions": { "network": ["api.example.com"], "filesystem": ["read"], "shell": false },
      "config_schema": { ... }
    }
    ```

15. **SKILL.md format spec**: YAML frontmatter with execution hints
    ```yaml
    ---
    name: skill-name
    description: One-line description for tool schema
    version: 0.1.0
    triggers:
      - pattern: "regex or keyword"
        priority: 10
    parameters:
      - name: input
        type: string
        required: true
        description: "..."
    source: wasm | native | prompt
    wasm_file: skill.wasm  # if source: wasm
    ---
    # Skill prompt / instructions below (used when source: prompt)
    ```

16. **Agent routing table config format** (`clawft.toml`):
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
    channel = "*"
    match = {}
    agent = "default-agent"
    ```
    Match semantics: exact string match on specified fields. Wildcard `"*"` for channel catches unmatched routes. First match wins. Missing `agent_routes` section defaults to single shared agent (backward compatible).

17. **FlowDelegator protocol** (`clawft-services/src/delegation/flow.rs`):
    - Spawn: `tokio::process::Command::new("claude")` with `--print` or `--json` flag
    - Input: task description as stdin or `--message` argument
    - Output: streaming stdout parsed as JSON lines or plain text
    - MCP callback: clawft registers as MCP server so Claude Code can invoke clawft tools during delegation
    - Fallback: if `claude` binary not on `$PATH`, fall back to `ClaudeDelegator` (direct Anthropic API)
    - Health check: `claude --version` with 5s timeout, result cached for 5 minutes

18. **Dynamic MCP discovery protocol**:
    - `weft mcp add <name> <command|url>` registers a server entry in `clawft.toml` under `[tools.mcp_servers]`
    - `weft mcp list` enumerates registered servers with connection status
    - `weft mcp remove <name>` removes entry
    - Runtime reconnection: exponential backoff (1s, 2s, 4s, ... 60s max) on connection failure
    - Health check: periodic `ping` JSON-RPC method, mark unhealthy after 3 consecutive failures
    - Hot-reload: `notify` file watcher on `clawft.toml` triggers MCP server list refresh without restart

19. **Security plugin audit check interface** (`clawft-plugin`):
    - `AuditCheck` trait: `name()`, `severity() -> AuditSeverity`, `check(plugin: &PluginManifest, source: &[u8]) -> Result<Vec<AuditFinding>>`
    - `AuditSeverity`: `Critical`, `High`, `Medium`, `Low`, `Info`
    - `AuditFinding`: `{ check_name, severity, message, location: Option<SourceLocation>, remediation: Option<String> }`
    - Built-in checks (50+): prompt injection patterns, data exfiltration URLs, unsafe shell commands, credential literals, excessive permissions
    - CLI: `weft security scan [--path <dir>]`, `weft security audit [--plugin <name>]`

### File Ownership

To avoid merge conflicts, each stream owns specific crates (extends existing ownership table):

| Stream | Owned Crates / Files |
|--------|---------------------|
| **Phase 1-4** | *(unchanged -- see existing table above)* |
| **A (Critical Fixes)** | Targeted fixes across all crates (single-file patches, no structural changes) |
| **B (Architecture Cleanup)** | clawft-types (unification, splits), clawft-core (dedup, splits), clawft-services (cron unification) |
| **C1 (Plugin Trait Crate)** | clawft-plugin (NEW CRATE -- trait definitions, manifest schema, SKILL.md types) |
| **C2 (WASM Plugin Host)** | clawft-wasm (WASM host implementation, WIT bindings, size budget enforcement) |
| **C3 (Skill Loader)** | clawft-core/src/agent/skills_v2.rs (SKILL.md parser, ClawHub discovery) |
| **C4 (Dynamic Loading)** | clawft-plugin (hot-reload watcher), clawft-core (skill precedence layering) |
| **C4a (Autonomous Creation)** | clawft-core/src/agent/ (skill generation logic) |
| **C5 (Slash Commands)** | clawft-cli/src/interactive/ (wire builtins + registry) |
| **C6 (MCP Skill Exposure)** | clawft-services/src/mcp/ (auto-expose loaded skills) |
| **C7 (PluginHost Unification)** | clawft-channels/src/host.rs (unify under plugin traits) |
| **D (Pipeline Reliability)** | clawft-core/src/pipeline/, clawft-llm/src/failover.rs, clawft-llm/src/retry.rs |
| **E1 (Discord Resume)** | clawft-channels/src/discord/ |
| **E2-E5b (New Channel Plugins)** | New plugin crates or clawft-channels/ subdirectories (email, whatsapp, signal, matrix, irc, google-chat, teams) |
| **F1-F7 (New Tool Plugins)** | New plugin crates (git, cargo, tree-sitter, browser, calendar, oauth2, docker) |
| **F8 (IDE Integration)** | clawft-services/src/mcp/ (VS Code extension backend) |
| **F9 (MCP Client)** | clawft-services/src/mcp/ (client-side connections to external servers) |
| **H1 (Workspace)** | clawft-core/src/workspace.rs, new workspace module |
| **H2 (Vector Memory)** | clawft-core/src/embeddings/, clawft-core/src/memory/ |
| **I (Type Safety)** | Targeted fixes across crates (single-field patches) |
| **J (Documentation)** | docs/ |
| **K2 (Docker)** | Dockerfile, scripts/, CI workflows |
| **K3 (Sandbox)** | clawft-wasm (seccomp/landlock), clawft-core (per-agent sandbox config) |
| **K3a (Security Plugin)** | clawft-plugin (audit check trait + built-in checks), clawft-cli (`weft security`) |
| **K4 (ClawHub)** | clawft-services (registry API client), clawft-cli (`weft skill publish/install`) |
| **L (Multi-Agent Routing)** | clawft-core/src/agent/ (routing table), clawft-types/src/routing.rs (route config) |
| **M1 (FlowDelegator)** | clawft-services/src/delegation/flow.rs (NEW FILE) |
| **M2 (Wire flow_available)** | clawft-tools/src/delegate_tool.rs |
| **M3 (Enable delegate feature)** | clawft-cli/Cargo.toml, clawft-services/Cargo.toml, clawft-tools/Cargo.toml |
| **M4 (Dynamic MCP Discovery)** | clawft-cli/src/mcp_tools.rs, clawft-cli/src/commands/ (new `mcp` subcommand) |
| **M5 (MCP Bridge)** | clawft-services/src/mcp/ (bidirectional bridge) |
| **M6 (Delegation Docs)** | docs/guides/configuration.md, docs/guides/tool-calls.md |

**Conflict zones** (require coordination between streams):
- `clawft-types/src/config.rs` -- touched by A4, B3, B7, I2. B3 file split should land first.
- `clawft-core/src/agent/loop_core.rs` -- touched by A3, B3, D1. B3 file split should land first.
- `clawft-services/src/mcp/` -- touched by C6, F8, F9, M4, M5. Coordinate via module-level ownership (each stream owns distinct files within the directory).
- `clawft-cli/Cargo.toml` -- touched by M3 and several plugin streams adding feature flags. Use additive-only changes; review for conflicts on merge.

---

## 6. Critical Path (Phase 5)

The critical path through the unified sprint plan:

```
A (Critical Fixes, W1-2)
  -> B3 (File Splits, W2-3) -- unblocks clean trait extraction
    -> C1 (Plugin Trait Crate, W3-4) -- MOST CRITICAL GATE
      -> C2 (WASM Host, W4-6) + C3 (Skill Loader, W4-5) -- parallel
        -> C4 (Dynamic Loading + Hot-Reload, W6-7)
          -> Feature Streams using plugins:
             E2-E5b (Channel Plugins, W5-8) -- can start once C1 lands
             F1-F7 (Tool Plugins, W6-10) -- can start once C1 lands
             H1 (Workspace, W5-7) -- needs C1
             L1-L3 (Multi-Agent Routing, W6-9) -- needs C1 + B5
             M1-M5 (Claude Flow, W4-7) -- parallel chain via D9
          -> K (Deployment, W8-12) -- last mile
```

**Phase 5 critical path (sequential gates):**

```
A (W1-2) -> B3 (W2-3) -> C1 (W3-4) -> C2+C3 (W4-6) -> C4 (W6-7) -> K (W8-12)
```

**Total critical path: 12 weeks** to full vision.

**Parallel acceleration opportunities:**
- D (Pipeline Reliability) runs entirely in parallel with B and C.
- I (Type Safety) runs independently from Week 2.
- M2 and M3 (standalone fixes) can land in Week 1 with zero dependencies.
- E and F plugin streams can start prototyping against C1 trait drafts before C1 merges, then finalize once traits are stable.

**MVP milestone (Week 8):**
- All A (critical fixes) resolved
- B3 file splits complete
- C1-C4 plugin system functional with skill precedence and hot-reload
- E2 email channel working as plugin
- L1 agent routing table functional
- M1-M3 Claude Flow integration operational (FlowDelegator + delegate feature enabled)
- F9 MCP client connecting to external servers
- 3 ported OpenClaw skills running

**Full vision milestone (Week 12):**
- F4 browser automation via CDP
- F1-F3 dev tool suite (git, cargo, tree-sitter)
- K3a security plugin with 50+ audit checks
- K4 ClawHub registry with vector search
- K3 per-agent sandboxing
- K2 Docker images published
- K5 benchmarks vs OpenClaw complete
- All forward-compat hooks for voice and UI in place

---

## 7. Risk Mitigation During Development (Phase 5 Additions)

Extends the existing risk table with Phase 5 specific risks:

| Risk | Detection | Response |
|------|-----------|----------|
| **Plugin API design too complex** | Plugin authors struggle; trait has >10 required methods for simple tools | Start with minimal trait (name + execute). Use default method implementations for optional capabilities. Review after first 3 plugins are implemented. Refactor before Week 6. |
| **Plugin API design too simple** | Cannot express channel-specific features (e.g., Discord embeds, Telegram keyboards) | Provide `metadata: Value` escape hatch on core traits. Channel-specific extension traits for rich features. |
| **WASM plugin size exceeds budget (<300KB)** | CI size assertion on `wasm32-wasip2` build | Profile with `twiggy`. Strip debug info. Use `wasm-opt -Oz`. Move heavy deps to host-provided imports. Budget: <300KB uncompressed, <120KB gzipped. |
| **Multi-agent routing config complexity** | Users misconfigure routing table; wrong agent handles messages | Provide `weft routes check` validation command. Default catch-all route to single agent. Log route match decisions at debug level. Include routing config examples in quickstart. |
| **Claude Flow integration (external dep on claude CLI)** | `claude` binary not installed or wrong version; subprocess hangs | Runtime detection with `claude --version` (5s timeout). Graceful fallback to direct Anthropic API via `ClaudeDelegator`. Cache detection result for 5 minutes. Clear error message: "Claude Code CLI not found; using direct API". |
| **Hot-reload race conditions** | Skill directory change mid-execution; partial file reads | Use atomic file operations: write to `.tmp`, rename into place. Reload on next skill invocation, not mid-execution. File watcher debounce (500ms). Lock skill registry during swap. |
| **Skill precedence confusion for users** | User cannot determine why a skill is overridden or which version runs | `weft skill list --verbose` shows source directory and precedence layer (workspace > managed > bundled). `weft skill which <name>` shows resolution path. Warn on name collision at startup (debug level). |
| **Security plugin false positives** | Legitimate skills blocked by audit checks; users bypass security entirely | Severity-based enforcement: only `Critical` blocks installation by default. `High`/`Medium`/`Low` produce warnings. Per-check allowlist in config: `security.allow = ["check-name"]`. `weft security scan --explain` shows reasoning for each finding. |
| **Plugin trait crate API churn during early development** | Downstream plugin crates break on every C1 change | Freeze trait signatures at end of Week 4 (C1 milestone). Use `#[non_exhaustive]` on enums. Semantic versioning from 0.1.0. Breaking changes require RFC in `.planning/`. |
| **WASM sandbox escape** | Plugin accesses host resources outside allowed permissions | WASI capability-based security: plugins only get explicitly granted capabilities. No ambient authority. Fuzz test WASM host with malicious inputs. Security audit before K3 milestone. |
| **Cross-agent session leakage** | Agent A reads Agent B's session data | Per-agent directory isolation (L2). Session paths include agent ID. Integration test: two agents with overlapping channel, verify no cross-reads. |
| **ClawHub supply chain attack** | Malicious skill published to registry | Mandatory security scan on publish. Signed manifests. `weft skill install` runs `weft security scan` before activation. Community flagging system. |

---

## 8. Definition of Done (Phase 5 Additions)

A task is done when (extends existing criteria 1-8):

1. Code compiles without warnings (`cargo clippy`)
2. All tests pass (`cargo test`)
3. No new `unsafe` blocks without justification
4. Public API has doc comments
5. Feature-gated code has appropriate `cfg` attributes
6. No hardcoded secrets or paths
7. Binary size regression checked (for release builds)
8. Channel plugins pass mock ChannelHost integration tests
9. **Plugin trait compliance**: If the task implements a plugin trait (`Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`), all required trait methods are implemented and pass the trait compliance test suite (`cargo test -p clawft-plugin --test trait_compliance`)
10. **Security audit check**: No prompt injection vectors in skill definitions, no credential leaks in plugin manifests or debug output, no unbounded resource consumption. Run `weft security scan` on any new plugin or skill code. For security-sensitive changes (A4, A5, A6, K3, K3a), require peer review.
11. **Backward compatibility verified**: Existing `config.json` files still deserialize correctly after type changes. Run `cargo test -p clawft-types --test config_compat` against fixture files from pre-change configs. Missing new fields must have `#[serde(default)]`. Renamed fields must have `#[serde(alias = "old_name")]`.
12. **WASM size budget**: If the task modifies `clawft-wasm` or `clawft-plugin`, verify the WASM binary stays under 300KB uncompressed and 120KB gzipped. CI enforces this via size assertion.
13. **Plugin isolation**: Plugins must not access host state outside their declared permissions. WASM plugins are sandboxed by default. Native plugins behind `unsafe` feature flag require justification.

---

## Testing Contracts (Phase 5 Additions)

Extends existing testing contracts (unit, integration, plugin, fixture, CI) with Phase 5 requirements:

### Plugin System Tests (Workstream C)

| Test Category | Scope | Owner |
|--------------|-------|-------|
| **Trait compliance tests** | Each plugin trait has a test harness that verifies: all required methods callable, return types correct, error handling works, default implementations behave as documented | C1 |
| **WASM sandbox tests** | Plugin cannot: access filesystem outside grant, make network calls outside allowlist, exceed memory limit, run longer than timeout. Test with intentionally malicious WASM modules. | C2 |
| **Hot-reload tests** | Skill directory change triggers reload on next invocation. Concurrent skill execution during reload does not panic. Partial file write does not corrupt skill registry. Reload debounce prevents thrashing. | C4 |
| **Skill precedence tests** | Workspace skill overrides managed skill of same name. Managed skill overrides bundled. Plugin-shipped skills participate in normal precedence. Collision logged at debug level. | C4 |
| **WASM size regression** | CI asserts `wasm32-wasip2` binary < 300KB. Fails build if exceeded. | C2 |
| **Plugin manifest validation** | Invalid manifests (missing required fields, unknown capability types, oversized WASM reference) produce clear error messages. | C1 |

### Multi-Agent Tests (Workstream L)

| Test Category | Scope | Owner |
|--------------|-------|-------|
| **Routing table tests** | Correct agent selected for each channel + match combination. First-match-wins semantics. Wildcard catch-all works. Missing routing table defaults to single agent. Invalid config produces startup error. | L1 |
| **Session isolation tests** | Agent A session writes are invisible to Agent B. Agent A skill overrides do not affect Agent B. Shared memory namespace opt-in works. Cross-agent message delivery through bus works when enabled. | L2 |
| **Cross-agent communication tests** | Agent A delegates subtask to Agent B via message bus. Result flows back. Coordinator pattern: lead decomposes, workers execute, results aggregate. Timeout handling for unresponsive agents. | L3 |
| **Route validation CLI test** | `weft routes check` detects: overlapping routes, missing default route, invalid agent references, unreachable routes. | L1 |

### Claude Flow Integration Tests (Workstream M)

| Test Category | Scope | Owner |
|--------------|-------|-------|
| **FlowDelegator subprocess tests** | `FlowDelegator` spawns `claude --print`, passes task, captures output. Timeout after configurable duration. Process cleanup on cancellation. Stderr captured to logs. | M1 |
| **MCP bridge tests** | clawft registers as MCP server, Claude Code connects and invokes a tool. clawft connects to external MCP server as client and lists tools. Bidirectional: both directions work in same session. | M5 |
| **Fallback tests** | `claude` binary not on PATH: falls back to `ClaudeDelegator`. `claude` binary times out: falls back. API key missing: delegation disabled with clear log message. | M1 |
| **Runtime detection tests** | `flow_available` returns `true` when `claude` on PATH + config enabled. Returns `false` when binary missing. Returns `false` when config disabled. Caches result. | M2 |
| **Dynamic MCP discovery tests** | `weft mcp add` registers server. `weft mcp list` shows it. Server connects on next gateway restart. Failed connection retries with exponential backoff. `weft mcp remove` disconnects and removes. | M4 |

### Test Infrastructure Additions

- **Mock `claude` binary**: Shell script that mimics `claude --print` and `claude --version` for CI environments without Claude Code installed. Located at `tests/fixtures/mock-claude`.
- **WASM test plugins**: Pre-compiled `.wasm` test fixtures for sandbox and compliance testing. Located at `tests/fixtures/wasm/`.
- **Multi-agent test harness**: Spins up N agents with isolated workspaces, configurable routing table, and mock channel for controlled message injection. Located at `tests/integration/multi_agent.rs`.
