# Element 04: Plugin & Skill System -- Execution Tracker

## Execution Schedule

Element 04 has 8 phases across Weeks 3-8:

**Week 3-4 (P0 Foundation)**:
- [x] C1 -- Plugin trait crate (`clawft-plugin` new) -- 6 traits, manifest, permissions (2026-02-19)

**Week 4-5 (P1 WASM)**:
- [ ] C2 -- WASM plugin host -- wasmtime, WIT, 5 host functions, security sandbox, 45 security tests

**Week 5-6 (P1 Skills)**:
- [ ] C3 -- Skill loader -- serde_yaml, local discovery, WASM auto-registration, tokio::fs

**Week 6-7 (P1 Dynamic)**:
- [ ] C4 -- Hot-reload & dynamic loading -- notify watcher, skill precedence, atomic swap, CLI commands

**Week 7 (P2+P1 Integration)**:
- [ ] C5 -- Slash-command framework -- wire SlashCommandRegistry, skill commands, collision detection
- [ ] C6 -- MCP skill exposure -- tools/list, tools/call, hot-reload MCP update

**Week 8 (P2 Unification)**:
- [ ] C7 -- PluginHost unification -- channel migration, concurrent start/stop, SOUL.md injection

**Week 8+ (P2 Stretch)**:
- [ ] C4a -- Autonomous skill creation -- pattern detection, skill generation, WASM compilation, managed install

## Per-Phase Status Table

| Phase | Description | Priority | Week | Crate(s) | Status | Owner | Branch | Key Deliverable |
|-------|-------------|----------|------|----------|--------|-------|--------|-----------------|
| C1 | Plugin trait crate | P0 | 3-4 | clawft-plugin (new) | **Done** | Agent-04 | sprint/phase-5 | 6 traits + manifest schema |
| C2 | WASM plugin host | P1 | 4-5 | clawft-wasm, clawft-core | **In Progress** | Agent-04 | sprint/phase-5 | Sandbox + WIT done; wasmtime pending |
| C3 | Skill loader | P1 | 5-6 | clawft-core | **Done** | Agent-04 | sprint/phase-5 | serde_yaml + WASM auto-reg |
| C4 | Hot-reload | P1 | 6-7 | clawft-core, clawft-cli | **Done** | Agent-04 | sprint/phase-5 | notify watcher + CLI |
| C4a | Autonomous creation | P2 | 8+ | clawft-core | Pending | -- | -- | Pattern detect + auto-gen |
| C5 | Slash commands | P2 | 7 | clawft-cli | **Done** | Agent-04 | sprint/phase-5 | Registry wiring |
| C6 | MCP exposure | P1 | 7-8 | clawft-services | **Done** | Agent-04 | sprint/phase-5 | SkillToolProvider + tools/list + tools/call |
| C7 | PluginHost unification | P2 | 8 | clawft-channels | Pending | -- | -- | Concurrent lifecycle |

## Internal Dependency Graph

```
C1 (trait crate) [NO DEPS]
  |
  +---> C2 (WASM host) -- implements plugin traits
  |       |
  +---> C3 (skill loader) -- also depends on 03-B3 (skills_v2.rs split)
  |       |
  |       +---> C4 (hot-reload) -- requires C2 + C3
  |       |       |
  |       |       +---> C4a (autonomous) -- requires C4
  |       |
  |       +---> C5 (slash commands) -- dynamic commands from loaded skills
  |       |
  |       +---> C6 (MCP exposure) -- MCP exposes loaded skills
  |
  +---> C7 (PluginHost) -- unification targets plugin trait system
```

## Cross-Element Dependencies

| Source (Element 04) | Target (Other Element) | Type |
|---------------------|------------------------|------|
| C1 (plugin traits) | Element 06-10 (all feature work) | Blocks -- traits must exist |
| C2 (WASM host) | Element 03 A6 (SSRF fix) | Uses is_private_ip() |
| C2 (WASM host) | Element 03 A4 (SecretRef) | PluginHost::resolve_secret |
| C2 (WASM host) | Element 03 A9 (feature gates) | wasm-plugins feature pattern |
| C3 (skill loader) | Element 03 B3 (file splits) | skills_v2.rs must be split first |
| C4 (hot-reload) | Element 10 K4 (ClawHub) | Remote skill discovery deferred to K4 |
| C7 (PluginHost) | Element 06 E1-E6 (channel enhancements) | Channel migration foundation |

## Exit Criteria

### Core (C1-C4):
- [x] clawft-plugin crate compiles with all trait definitions (2026-02-19)
- [x] At least one plugin implements each of the 6 traits (mock impls in tests, 2026-02-19)
- [ ] WASM plugin host loads and runs a test plugin
- [ ] weft skill install <path> works for local skills
- [ ] Hot-reload detects file changes within 2 seconds
- [ ] Skill precedence (workspace > managed > bundled) verified
- [x] VoiceHandler trait placeholder exists (2026-02-19)

### Security (C2):
- [ ] Every WIT host function validates against PluginPermissions
- [ ] WASM fuel metering enabled (configurable, default 1B units)
- [ ] WASM memory limits via StoreLimits (default 16MB)
- [x] read-file/write-file canonicalize paths, reject external symlinks (2026-02-19)
- [x] http-request applies SSRF check + network allowlist (2026-02-19)
- [x] get-env returns None for non-permitted vars (2026-02-19)
- [x] Rate limiting on http-request and log (2026-02-19)
- [ ] Audit logging for all host function calls
- [ ] All 45 security tests (T01-T45) pass -- 30/45 done (T01-T27, T33-T36, T44)

### Integration (C5-C7):
- [ ] Agent commands routed through registry, not inline match
- [ ] MCP tools/list includes loaded skill tools with JSON Schema
- [ ] MCP tools/call routes through skill.execute_tool()
- [ ] Hot-reload updates MCP tool listing without restart
- [ ] Existing channels work through unified PluginHost
- [ ] start_all()/stop_all() execute concurrently
- [ ] SOUL.md injected into Assembler pipeline stage

### Stretch (C4a):
- [ ] Pattern detection threshold configurable (default: 3)
- [ ] Generated SKILL.md passes same validation
- [ ] User prompted for approval before install
- [ ] Disabled by default, opt-in only
- [ ] Minimal permissions enforced

## Review Gates

| Gate | Scope | Requirement |
|------|-------|-------------|
| C1 API Review | Trait definitions | All trait designs reviewed before C2+ begins |
| C2 Security Review | WASM sandbox | Security-focused review; all 45 tests verified |
| C3 Regression | Skill parser | All existing 559+ skill tests still pass |
| C4 Integration | Hot-reload | End-to-end test: add skill file -> watcher detects -> registry updates -> MCP exposes |
| C7 Channel Migration | PluginHost | All existing channel tests pass without modification |

## Risk Register

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| WASM sandbox escape via host functions | Low | Critical | 8 | Enforce permission allowlists. Fuel metering. Memory limits. Audit all host-function impls. |
| Skill hot-reload race condition | Medium | Low | 4 | Atomic swap with drain. Debounce rapid changes. |
| Autonomous skill creation safety (C4a) | Low | Critical | 6 | Pending state. User approval. Minimal permissions. Disabled by default. |
| Week 7-8 schedule overload | Medium | Medium | 6 | C4a deferred to stretch. C5->C6 sequenced. C7 in Week 8. |
| wasmtime adds significant binary size | Medium | Low | 4 | Feature-gate behind wasm-plugins. Split builds. |
| skills_v2.rs not split (B3 late) | Medium | Medium | 6 | C3 can work on unsplit file; B3 split preferred. |

## Progress Summary

| Phase | Status | % Done |
|-------|--------|--------|
| C1 (Plugin Traits) | **Done** | 100% |
| C2 (WASM Host) | **In Progress** | 60% |
| C3 (Skill Loader) | **Done** | 100% |
| C4 (Hot-Reload) | **Done** | 100% |
| C4a (Autonomous) | Pending | 0% |
| C5 (Slash Commands) | **Done** | 100% |
| C6 (MCP Exposure) | **Done** | 100% |
| C7 (PluginHost) | Pending | 0% |
| **Total** | **In Progress** | **75%** |
