# SPARC Feature Element 04: Plugin & Skill System

**Workstream**: C (Plugin & Skill System)
**Timeline**: Weeks 3-8
**Status**: Done (C2 at 98% -- T41 deferred)
**Dependencies**: 03 (B3 file splits must precede C1), 03-A4 (SecretRef pattern used by PluginHost::resolve_secret), 03-A9 (feature gates pattern for MCP)
**Blocks**: 06, 07, 08, 09, 10 (all feature work uses plugin traits)

---

## 1. Summary

Define a unified plugin architecture (`clawft-plugin` crate) with traits for tools, channels, pipeline stages, skills, memory backends, and voice handlers. Implement WASM plugin host, skill loader with SKILL.md parsing, dynamic loading with hot-reload, and autonomous skill creation.

---

## 2. Phases

### Phase C1: Plugin Trait Crate (Week 3-4) -- P0

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| `clawft-plugin` crate | New crate with trait definitions | clawft-plugin (new) |
| Unified traits | `Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler` | clawft-plugin |
| Plugin manifest | JSON/YAML schema for plugin metadata | clawft-plugin |
| Forward-compat | `VoiceHandler` as placeholder, `voice` capability reserved | clawft-plugin |

### Phase C2: WASM Plugin Host (Week 4-5) -- P1

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| wasmtime integration | `wasmtime` + `wit` component model for typed interfaces | clawft-wasm, clawft-core |
| WasiFileSystem | Complete implementation (currently all stubs) | clawft-wasm |
| WASM HTTP client | Implement for plugin network access | clawft-wasm |
| Size enforcement | < 300KB uncompressed, < 120KB gzipped per plugin | clawft-wasm |

### Phase C3: Skill Loader (Week 5-6) -- P1

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| SKILL.md parser | Replace hand-rolled YAML parser with `serde_yaml` | clawft-core/src/agent/skills_v2.rs (refactored in-place) |
| Local skill discovery | Scan workspace, managed (`~/.clawft/skills`), and bundled directories | clawft-core/src/agent/skills_v2.rs |
| Auto-registration | WASM or native wrapper from SKILL.md | clawft-core/src/agent/skills_v2.rs |
| OpenClaw compatibility | Match OpenClaw skill format | clawft-core/src/agent/skills_v2.rs |

**Note**: ClawHub remote discovery (HTTP index + git clone) is deferred to K4. C3 covers local skill loading only.

### Phase C4: Dynamic Loading & Hot-Reload (Week 6-7) -- P1

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Skill precedence | Workspace > managed/local > bundled | clawft-core/src/agent/skill_watcher.rs |
| Hot-reload watcher | `notify` crate for file-system watching | clawft-core/src/agent/skill_watcher.rs |
| Plugin-shipped skills | Skills declared in plugin manifest | clawft-core/src/agent/skill_watcher.rs |
| `weft skill install` | CLI command for skill management | clawft-cli |

### Phase C4a: Autonomous Skill Creation (Week 8+, stretch) -- P2

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Pattern detection | Agent detects repeated task patterns (configurable threshold, default: 3 repetitions) | clawft-core |
| Skill generation | Auto-write SKILL.md + implementation | clawft-core |
| WASM compilation | Compile generated skills to WASM | clawft-wasm |
| Managed install | Install into `~/.clawft/skills` (pending state, requires user approval) | clawft-core |

### Phase C5: Slash-Command Framework (Week 7) -- P2

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Registry wiring | Wire interactive slash-command framework; route agent commands through registry instead of inline match blocks | clawft-cli/src/interactive/ |
| Skill-contributed commands | Skills can contribute commands that appear in `/help` output | clawft-cli/src/interactive/ |
| Command collision handling | Clear error on command name collisions between skills | clawft-cli/src/interactive/ |

### Phase C6: MCP Skill Exposure (Week 7-8) -- P1

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| MCP tools/list | Loaded skills appear as callable tools in MCP server tool listing with JSON Schema parameter definitions | clawft-services/src/mcp/server.rs |
| MCP tools/call routing | `tools/call` for a skill-provided tool routes through the skill's `execute_tool()` method | clawft-services/src/mcp/server.rs |
| Hot-reload MCP update | Adding/removing a skill via hot-reload updates the MCP tool listing without server restart | clawft-services/src/mcp/server.rs |

### Phase C7: PluginHost Unification (Week 8) -- P2

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Channel migration | Existing Telegram, Discord, and Slack channels work through the unified PluginHost without behavior changes | clawft-channels/src/host.rs |
| Concurrent lifecycle | `PluginHost.start_all()` and `stop_all()` execute concurrently, not sequentially | clawft-channels/src/host.rs |
| SOUL.md injection | SOUL.md personality content injected into the Assembler pipeline stage system prompt | clawft-channels/src/host.rs |

---

## 2.5 Internal Dependency Graph

```
C1 (trait crate)
  |
  +---> C2 (WASM host) -- implements plugin traits
  |       |
  +---> C3 (skill loader) -- skills register as trait implementations
  |       |     also depends on: 03-B3 (skills_v2.rs YAML parser must be replaced first)
  |       |
  |       +---> C4 (hot-reload) -- requires both WASM host and skill loader
  |       |       |
  |       |       +---> C4a (autonomous creation) -- requires full hot-reload pipeline
  |       |
  |       +---> C5 (slash-commands) -- dynamic commands come from loaded skills
  |       |
  |       +---> C6 (MCP exposure) -- MCP exposes loaded skills
  |
  +---> C7 (unified PluginHost) -- unification targets the plugin trait system
```

### Week 7-8 Sequencing

To reduce overload in weeks 7-8, the schedule is:

- **Week 7**: C5 (slash-commands), then C6 (MCP exposure) -- sequenced within the week
- **Week 8**: C7 (PluginHost unification)
- **Week 8+ (stretch)**: C4a (autonomous skill creation) -- P2, only if C5-C7 are complete

---

## 3. Exit Criteria

### Core Criteria

- [x] `clawft-plugin` crate compiles with all trait definitions (2026-02-19)
- [x] At least one plugin implements each trait (all six: Tool, ChannelAdapter, PipelineStage, Skill, MemoryBackend, VoiceHandler) (2026-02-19)
- [x] WASM plugin host loads and runs a test plugin (2026-02-20)
- [x] `weft skill install <path>` works for local skills (2026-02-20)
- [x] Hot-reload detects file changes within 2 seconds (2026-02-20)
- [x] Skill precedence (workspace > managed > bundled) verified (2026-02-20)
- [x] VoiceHandler trait placeholder exists (forward-compat) (2026-02-19)
- [x] All existing tests pass (2026-02-20, 2,407 tests)
- [x] J7 plugin system documentation complete (final deliverable from Element 03 J7) (2026-02-19, skeleton)

### C4a Exit Criteria (Autonomous Skill Creation)

- [x] Pattern detection threshold is configurable (default: 3 repetitions) (2026-02-20)
- [x] Generated SKILL.md passes the same validation as manually authored skills (2026-02-20)
- [x] User is prompted for approval before auto-generated skills are installed (2026-02-20)
- [x] Autonomous skill creation is disabled by default and must be opted into (2026-02-20)
- [x] Auto-generated skills have minimal permissions (no shell, no network, filesystem limited to workspace) (2026-02-20)

### C5 Exit Criteria (Slash-Command Framework)

- [x] Agent commands routed through the registry, not inline match blocks (2026-02-20)
- [x] Skills can contribute commands that appear in `/help` output (2026-02-20)
- [x] Command name collisions between skills produce a clear error (2026-02-20)

### C6 Exit Criteria (MCP Skill Exposure)

- [x] MCP `tools/list` response includes all loaded skill tools with JSON Schema parameter definitions (2026-02-20)
- [x] MCP `tools/call` for a skill-provided tool routes through the skill's `execute_tool()` method (2026-02-20)
- [x] Adding/removing a skill via hot-reload updates the MCP tool listing without server restart (2026-02-20)

### C7 Exit Criteria (PluginHost Unification)

- [x] Existing Telegram, Discord, and Slack channels work through the unified PluginHost without behavior changes (2026-02-20)
- [x] `PluginHost.start_all()` and `stop_all()` execute concurrently, not sequentially (2026-02-20)
- [x] SOUL.md content is injected into the Assembler pipeline stage system prompt (2026-02-20)

---

## 4. WASM Security Requirements (CRITICAL)

The plugin system introduces the most significant new attack surface in the sprint. Every host function exposed to WASM plugins is a potential escape path from the sandbox.

### 4.1 Host-Function Permission Enforcement

Every WIT host function MUST validate against the plugin's `PluginPermissions` before executing:

| Host Function | Permission Check |
|---------------|-----------------|
| `http-request` | Validate URL against `permissions.network` allowlist + apply SSRF check (reuse A6 `is_private_ip()`) |
| `read-file` | Canonicalize path, verify it falls within `permissions.filesystem` paths, reject symlinks pointing outside allowed directories |
| `write-file` | Same as `read-file` -- canonicalize, verify, reject external symlinks |
| `get-env` | Only return values for env vars explicitly listed in `permissions.env_vars` |
| `log` | Rate-limit to prevent log flooding |

### 4.2 Resource Limits

| Resource | Default Limit | Configurable? |
|----------|--------------|---------------|
| WASM fuel metering | 1,000,000,000 units (~1s CPU) | Yes, via plugin manifest |
| WASM memory | 16 MB per plugin | Yes, via plugin manifest |
| Plugin binary size | < 300 KB uncompressed, < 120 KB gzipped | Yes, via global config |

### 4.3 Security Exit Criteria

- [x] Every WIT host function validates against `PluginPermissions` before executing (2026-02-20)
- [x] WASM plugins have fuel metering enabled (configurable, default: 1B units) (2026-02-20)
- [x] WASM plugins have memory limits (configurable, default: 16MB) (2026-02-20)
- [x] `read-file` / `write-file` host functions canonicalize paths and reject symlinks outside allowed directories (2026-02-19)
- [x] `http-request` host function applies SSRF check + network allowlist (2026-02-19)
- [x] `get-env` host function only returns values for explicitly permitted env vars (2026-02-19)
- [x] Auto-generated skills (C4a) require user approval before activation (2026-02-20)
- [ ] Shell-execution skills require explicit user approval on install (deferred -- part of T39 lifecycle tests)

---

## 5. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| WASM sandbox escape via host functions | Low | Critical | **8** | Enforce permission allowlists on all WIT host function calls. Fuel metering and memory limits prevent DoS. Audit all host-function implementations before merge. |
| Skill hot-reload race condition (in-flight tool call interrupted) | Medium | Low | **4** | Atomic swap with drain: new skill version loads alongside old; in-flight calls complete on old version before swap. File watcher debounces rapid changes. |
| Autonomous skill creation safety (C4a) | Low | Critical | **6** | Skills created autonomously install in "pending" state. User approval required before activation. Minimal permission set enforced (no shell, no network, workspace-only filesystem). Disabled by default. |
| Week 7-8 schedule overload (C5+C6+C7+C4a) | Medium | Medium | **6** | C4a moved to P2/stretch (Week 8+). C5 -> C6 sequenced within Week 7. C7 in Week 8. If behind schedule, C4a and C7 defer to a follow-up element. |
