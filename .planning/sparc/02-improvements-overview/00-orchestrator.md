# SPARC Master Orchestration Plan: Improvements Sprint

**Project**: clawft -- Improvements Sprint (Post-Phase 4)
**Source**: `.planning/improvements.md`
**Created**: 2026-02-19
**Status**: Planning in progress
**Execution Model**: 9 feature elements, parallel execution with dependency gates
**Test Baseline**: 2,075+ tests at sprint start
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

## 6. Source Documents

- `.planning/improvements.md` -- Sprint plan (source of truth)
- `.planning/01-business-requirements.md` -- Business requirements
- `.planning/02-technical-requirements.md` -- Technical requirements
- `.planning/03-development-guide.md` -- Development guide
- `.planning/voice_development.md` -- Voice feature plan (deferred)
- `.planning/ui_development.md` -- UI feature plan (deferred)
