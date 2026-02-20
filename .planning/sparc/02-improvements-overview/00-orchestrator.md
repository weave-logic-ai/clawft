# SPARC Master Orchestration Plan: Improvements Sprint

**Project**: clawft -- Improvements Sprint (Post-Phase 4)
**Source**: `.planning/improvements.md`
**Created**: 2026-02-19
**Status**: COMPLETE -- Sprint executed 2026-02-19 to 2026-02-20
**Execution Model**: 9 feature elements, parallel execution with dependency gates
**Test Baseline**: 2,075+ tests at sprint start
**Test Final**: 2,407 tests at sprint end (net +332)
**Commits**: 6 commits on sprint/phase-5 (07ceb05, 63ebe99, 9d152b5, dfd4066, a67b9e5)
**Phase Standard**: All sprint work is Phase 5. Business requirements use sections 5d-5o.

---

## 1. Executive Summary

This orchestration plan coordinates the improvements sprint identified through a full 9-crate codebase review. The sprint addresses 90+ items across 13 workstreams (A-M), grouped into 8 numbered SPARC feature elements.

**Scope**: Critical fixes, architecture cleanup, plugin system, pipeline reliability, channel enhancements, dev tools, memory/workspace improvements, multi-agent routing, Claude Flow integration, and deployment/community features.

**Out of scope**: Voice (Workstream G) and UI/Dashboard (K1/K6). See `voice_development.md` and `ui_development.md`. Forward-compatibility hooks are in scope.

**B3 scope note**: 9 critical files targeted initially (950-1668 lines each); 39 total files over 500 lines addressed incrementally.

---

## 2. Feature Elements

| Element | SPARC Dir | Workstreams | Weeks | Focus |
|---------|-----------|-------------|-------|-------|
| 03 | `03-critical-fixes-cleanup` | A, B, I, J | 1-5 | Bugs, security, architecture cleanup, type safety, doc sync |
| 04 | `04-plugin-skill-system` | C | 3-8 | Plugin trait crate, WASM host, skill loader, hot-reload |
| 05 | `05-pipeline-reliability` | D | 2-5 | Parallel tools, streaming failover, retry, bounded bus |
| 06 | `06-channel-enhancements` | E | 4-8 | Discord resume, email, WhatsApp, Signal, Matrix, Google Chat, Teams |
| 07 | `07-dev-tools-apps` | F (F9a/F9b split) | 5-10 | Git, tree-sitter, browser CDP, OAuth2, Docker, MCP client (F9a minimal at MVP, F9b full at Week 10) |
| 08 | `08-memory-workspace` | H | 4-8 | Per-agent isolation, RVF Phase 3, vector memory, timestamps |
| 09 | `09-multi-agent-routing` | L, M | 3-9 | Agent routing, swarming, Claude Flow integration, MCP bridge |
| 10 | `10-deployment-community` | K (in-scope) | 8-12 | Docker images, CI/CD, sandboxing, security plugin, ClawHub |

**Element 07 F9 split detail**:
- **F9a** (Week 7-8, MVP): Minimal MCP client -- connect to a single configured MCP server, list tools, invoke tools. No auto-discovery, no connection pooling.
- **F9b** (Week 9-10, post-MVP): Full MCP client -- auto-discovery via `~/.clawft/mcp/`, connection pooling, tool schema caching, health checks, multi-server support.

---

## 3. Dependency Graph

```
03-critical-fixes-cleanup (A, B, I, J) ──┐
                                          ├──> 04-plugin-skill-system (C)
05-pipeline-reliability (D) ──────────────┘         │
                                                    ├──> 06-channel-enhancements (E)
                                                    ├──> 07-dev-tools-apps (F)
                                                    ├──> 08-memory-workspace (H)
                                                    ├──> 09-multi-agent-routing (L, M)
                                                    └──> 10-deployment-community (K)
```

**Critical path**: A (fixes) -> B3 (file splits) -> C1 (plugin traits) -> C2-C4 (plugin infra) -> feature work -> K (deployment)

**Parallel groups**:
- **Group 1 (Week 1-5)**: Elements 03, 05 run in parallel
- **Group 2 (Week 3-8)**: Element 04 starts after C1 prerequisites from 03
- **Group 3 (Week 4-10)**: Elements 06, 07, 08, 09 run in parallel (all depend on 04/C1)
- **Group 4 (Week 8-12)**: Element 10 starts after MVP milestone

**Cross-cutting concern: CI/CD** -- Defined in Element 10 (K2-CI) but benefits all elements from Week 1. PR gates (`cargo clippy`, `cargo test --workspace`, WASM size assertion, binary size check) should be adopted as early as possible. Release pipeline and integration smoke test land with K2 at Week 8-9.

---

## 4. Milestones

**MVP (Week 8)**: Plugin system with skill precedence + hot-reload, email channel, multi-agent routing, 3 ported OpenClaw skills (`coding-agent`, `web-search`, `file-management`), F9a (minimal MCP client for single configured server), Claude Flow integration (extends existing `ClaudeDelegator`, not new `FlowDelegator`) + dynamic MCP + delegate feature enabled by default, all critical/high fixes resolved.

**MVP clarification**: F9a (minimal MCP client) at MVP; F9b (full auto-discovery) at Week 10. M1 extends existing `ClaudeDelegator`, not a new `FlowDelegator`.

**Full Vision (Week 12)**: Browser automation, dev tool suite, F9b (full MCP client with auto-discovery), ClawHub with vector search, per-agent sandboxing, security plugin, Docker images, CI/CD pipeline. All forward-compat hooks for voice and UI in place.

**Post-Sprint**: Voice (see `voice_development.md`), UI/Dashboard/Native Shells (see `ui_development.md`).

---

## 5. Forward Compatibility

Even though Voice and UI are out of scope, the following hooks must be built:

**Voice (Workstream G)**:
- C1 defines `VoiceHandler` trait placeholder
- Plugin manifest reserves `voice` capability type
- `ChannelAdapter` trait supports binary/audio payloads
- Feature flag `voice` wired as no-op in Cargo.toml

**UI (Workstream K1/K6)**:
- Agent loop and bus support structured/binary payloads
- MCP server tool schemas are stable for future dashboard introspection
- Config and session APIs are read-accessible without going through agent loop

**Forward-compat verification**: Compile-time tests for `VoiceHandler` instantiation and binary payload `ChannelAdapter`. These tests verify the trait placeholders and capability hooks remain functional as the codebase evolves, catching breakage before voice/UI implementation begins.

---

## 6. Sprint Completion Report

**Executed**: 2026-02-19 to 2026-02-20 (4 waves, 6 commits)
**Branch**: `sprint/phase-5`

### Element Completion

| Element | Focus | % Done | Items Done | Deferred/Blocked |
|---------|-------|--------|------------|-----------------|
| 03 | Critical Fixes & Cleanup | 100% | 33/33 | -- |
| 04 | Plugin & Skill System | 99% | 8/8 phases | C2: 3 minor security tests (T30/T41/T42) |
| 05 | Pipeline & LLM Reliability | 100% | 11/11 | -- |
| 06 | Channel Enhancements | 89% | 8/9 | IRC channel deferred |
| 07 | Dev Tools & Apps | 100% | 10/10 | -- |
| 08 | Memory & Workspace | 100% | 10/10 | -- |
| 09 | Multi-Agent Routing | 93% | 13/14 | M6 docs deferred |
| 10 | Deployment & Community | 90% | K2-K5 core | Skill install/publish, MVP skills blocked on C3/C4 runtime |

### Wave Execution

| Wave | Scope | Commit | Files | Tests |
|------|-------|--------|-------|-------|
| 1 | Elements 03-A, 05 | `07ceb05`, `63ebe99` | 59 | 1903 |
| 2 | Element 03-B/I/J, 04-C1 | `9d152b5` | 77 | 1968 |
| 3 | Elements 04/06/07/08/09 | `dfd4066` | 93 | 2204 |
| 4 | Elements 04/07/08/10 | `a67b9e5` | 68 | 2407 |

### New Crates Added (8)

- `clawft-plugin` -- 6 core plugin traits + manifest
- `clawft-plugin-git` -- 7 git tools via git2
- `clawft-plugin-cargo` -- 5 cargo subcommands
- `clawft-plugin-oauth2` -- OAuth2 + REST tools
- `clawft-plugin-treesitter` -- AST analysis tools
- `clawft-plugin-browser` -- CDP browser automation
- `clawft-plugin-calendar` -- Calendar CRUD tools
- `clawft-plugin-containers` -- Docker/Podman tools
- `clawft-security` -- 57 audit checks across 10 categories

### New Channel Adapters (7)

Email, WhatsApp, Signal, Matrix, Google Chat, Microsoft Teams, Discord Resume

### Key Architecture Deliverables

- WASM sandbox: wasmtime 29, fuel metering, memory limits, 5 host functions, 45 security tests
- HNSW vector memory: instant-distance, brute-force fallback, temperature quantization
- Agent routing: first-match-wins router, AgentBus with per-agent inboxes, SwarmCoordinator
- MCP ecosystem: discovery, bridge, IDE integration, client pool, skill exposure
- Plugin lifecycle: hot-reload, autonomous skill creation, slash-command framework
- Per-agent workspace isolation: SOUL.md injection, 3-level config merge
- SHA-256 WITNESS audit chains with tamper detection
- SandboxPolicy with OS sandbox (seccomp/landlock) + WASM fallback
- CI/CD: PR gates, multi-arch Docker release pipeline

### Remaining Items

1. **C2 T30/T41/T42** -- 3 WASM security tests (wall-clock timeout, version re-prompt, lifecycle)
2. **IRC channel** -- deferred (low priority)
3. **M6 docs** -- deferred to documentation sprint
4. **K4 skill install/publish** -- blocked on ClawHub server availability
5. **K5 3 MVP skills** -- blocked on C3/C4 skill loader runtime integration

---

## 7. Source Documents

- `.planning/improvements.md` -- Sprint plan (source of truth)
- `.planning/01-business-requirements.md` -- Business requirements
- `.planning/02-technical-requirements.md` -- Technical requirements
- `.planning/03-development-guide.md` -- Development guide
- `.planning/voice_development.md` -- Voice feature plan (deferred)
- `.planning/ui_development.md` -- UI feature plan (deferred)
