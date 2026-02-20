# Consensus Review: Business & Technical Requirements Drafts

> **Reviewer**: Senior Technical Review Agent
> **Date**: 2026-02-19
> **Scope**: 6 draft documents covering Workstreams A-M from `improvements.md`
> **Documents Reviewed**:
> 1. `01-biz-stability.md` (Workstreams A, I)
> 2. `01-biz-architecture.md` (Workstreams B, D)
> 3. `01-biz-plugins-channels.md` (Workstreams C, E)
> 4. `01-biz-features-deploy.md` (Workstreams F, H, K, L, M, J)
> 5. `02-tech-core.md` (Workstreams A, B, C, I)
> 6. `02-tech-pipeline.md` (Workstreams D, E, F, H, K, L, M)

---

## 1. Coverage Checklist

Every item from `improvements.md` is checked against all drafts. Status:
- COVERED = adequately represented in at least one draft
- PARTIAL = mentioned but incomplete or shallow
- MISSING = not found in any draft
- DUPLICATE = covered in multiple drafts (may need consolidation)

### Workstream A: Critical Fixes (Week 1-2)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| A1 | Session key round-trip corruption | SF-1 in `01-biz-stability.md` | 12.1 in `02-tech-core.md` | COVERED |
| A2 | Unstable hash function in embeddings | SF-2 in `01-biz-stability.md` | 12.2 in `02-tech-core.md` | COVERED |
| A3 | Invalid JSON from error formatting | SF-3 in `01-biz-stability.md` | 12.3 in `02-tech-core.md` | COVERED |
| A4 | Plaintext credentials in config structs | SF-4 in `01-biz-stability.md` | 12.4 in `02-tech-core.md` | COVERED |
| A5 | API key echoed during onboarding | SF-5 in `01-biz-stability.md` | 12.5 in `02-tech-core.md` | COVERED |
| A6 | Incomplete private IP range in SSRF | SF-6 in `01-biz-stability.md` | 12.6 in `02-tech-core.md` | COVERED |
| A7 | No HTTP request timeout | SF-7 in `01-biz-stability.md` | 12.7 in `02-tech-core.md` | COVERED |
| A8 | Unsafe env::set_var in tests | SF-8 in `01-biz-stability.md` | 12.8 in `02-tech-core.md` | COVERED |
| A9 | --no-default-features fails | SF-9 in `01-biz-stability.md` | 12.9 in `02-tech-core.md` | COVERED |

### Workstream B: Architecture Cleanup (Week 2-4)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| B1 | Unify Usage type across crates | AC-1 in `01-biz-architecture.md` | 13.1 in `02-tech-core.md` | COVERED |
| B2 | Unify duplicate LlmMessage types | AC-2 in `01-biz-architecture.md` | 13.2 in `02-tech-core.md` | COVERED |
| B3 | Split oversized files | AC-3 in `01-biz-architecture.md` | 13.3 in `02-tech-core.md` | COVERED |
| B4 | Unify cron storage formats | AC-4 in `01-biz-architecture.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| B5 | Extract shared tool registry builder | AC-5 in `01-biz-architecture.md` | 13.4 in `02-tech-core.md` | COVERED |
| B6 | Extract shared policy types | AC-6 in `01-biz-architecture.md` | 13.5 in `02-tech-core.md` | COVERED |
| B7 | Deduplicate ProviderConfig naming | AC-7 in `01-biz-architecture.md` | 13.6 in `02-tech-core.md` | COVERED |
| B8 | Consolidate build_messages duplication | AC-8 in `01-biz-architecture.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| B9 | MCP protocol version constant | AC-9 in `01-biz-architecture.md` | Not in tech drafts | PARTIAL - Missing tech spec |

### Workstream C: Plugin & Skill System (Week 3-8)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| C1 | Define clawft-plugin trait crate | PS-1, PS-2 in `01-biz-plugins-channels.md` | 14.1, 14.2 in `02-tech-core.md` | COVERED |
| C2 | WASM plugin host | PS-3, PS-4, PS-15 in `01-biz-plugins-channels.md` | 14.3 in `02-tech-core.md` | COVERED |
| C3 | Skill Loader (OpenClaw-compatible) | PS-5, PS-6 in `01-biz-plugins-channels.md` | 14.4 in `02-tech-core.md` | COVERED |
| C4 | Dynamic skill loading & hot-reload | PS-7, PS-8, PS-9, PS-13 in `01-biz-plugins-channels.md` | 14.5, 14.6 in `02-tech-core.md` | COVERED |
| C4a | Autonomous skill creation | PS-10 in `01-biz-plugins-channels.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| C5 | Wire interactive slash-command framework | PS-11 in `01-biz-plugins-channels.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| C6 | Extend MCP server for loaded skills | PS-12 in `01-biz-plugins-channels.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| C7 | Update PluginHost to unify channels + tools | PS-14 in `01-biz-plugins-channels.md` | Not in tech drafts | PARTIAL - Missing tech spec |

### Workstream D: Pipeline & LLM Reliability (Week 2-5)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| D1 | Parallel tool execution | PR-1 in `01-biz-architecture.md` | 12.1 in `02-tech-pipeline.md` | COVERED |
| D2 | Streaming failover correctness | PR-2 in `01-biz-architecture.md` | 12.2 in `02-tech-pipeline.md` | COVERED |
| D3 | Structured error variants for retry | PR-3 in `01-biz-architecture.md` | 12.3 in `02-tech-pipeline.md` | COVERED |
| D4 | Configurable retry policy | PR-4 in `01-biz-architecture.md` | 12.4 in `02-tech-pipeline.md` | COVERED |
| D5 | Record actual latency | PR-5 in `01-biz-architecture.md` | 12.5 in `02-tech-pipeline.md` | COVERED |
| D6 | Thread sender_id for cost recording | PR-6 in `01-biz-architecture.md` | Not explicitly in tech drafts | PARTIAL - Referenced in biz but missing dedicated tech section |
| D7 | Change StreamCallback to FnMut | PR-7 in `01-biz-architecture.md` | 12.6 in `02-tech-pipeline.md` | COVERED |
| D8 | Bounded message bus channels | PR-8 in `01-biz-architecture.md` | 12.7 in `02-tech-pipeline.md` | COVERED |
| D9 | MCP transport concurrency | PR-9 in `01-biz-architecture.md` | Not explicitly in tech drafts | PARTIAL - Referenced in biz but no dedicated tech section |
| D10 | Cache skill/agent bootstrap files | PR-10 in `01-biz-architecture.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| D11 | Async file I/O in skills loader | PR-11 in `01-biz-architecture.md` | Not in tech drafts | PARTIAL - Missing tech spec |

### Workstream E: Channel Enhancements (Week 4-8)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| E1 | Discord Resume (OP 6) | CE-1 in `01-biz-plugins-channels.md` | 13.1 in `02-tech-pipeline.md` | COVERED |
| E2 | Email channel plugin | CE-2, CE-3, CE-14 in `01-biz-plugins-channels.md` | 13.2 in `02-tech-pipeline.md` | COVERED |
| E3 | WhatsApp channel | CE-4, CE-14 in `01-biz-plugins-channels.md` | 13.3 in `02-tech-pipeline.md` | COVERED |
| E4 | Signal / iMessage bridge | CE-5, CE-6, CE-14 in `01-biz-plugins-channels.md` | 13.4 in `02-tech-pipeline.md` | COVERED |
| E5 | Matrix / IRC channels | CE-7, CE-8, CE-14 in `01-biz-plugins-channels.md` | 13.5 in `02-tech-pipeline.md` | COVERED |
| E5a | Google Chat channel | CE-9, CE-10 in `01-biz-plugins-channels.md` | 13.6 in `02-tech-pipeline.md` | COVERED |
| E5b | Microsoft Teams channel | CE-11, CE-12 in `01-biz-plugins-channels.md` | 13.7 in `02-tech-pipeline.md` | COVERED |
| E6 | Enhanced heartbeat / proactive check-in | CE-13 in `01-biz-plugins-channels.md` | Not explicitly in tech drafts | PARTIAL - Missing dedicated tech section |

### Workstream F: Software Dev & App Tooling (Week 5-10)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| F1 | Git tool plugin | DT-1 in `01-biz-features-deploy.md` | 14.1 in `02-tech-pipeline.md` | COVERED |
| F2 | Cargo/build integration | DT-2 in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| F3 | Code analysis via tree-sitter | DT-3 in `01-biz-features-deploy.md` | 14.2 in `02-tech-pipeline.md` | COVERED |
| F4 | Browser CDP automation | DT-4 in `01-biz-features-deploy.md` | 14.3 in `02-tech-pipeline.md` | COVERED |
| F5 | Calendar integration | DT-5 in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| F6 | Generic REST + OAuth2 helper | DT-6 in `01-biz-features-deploy.md` | 14.4 in `02-tech-pipeline.md` | COVERED |
| F7 | Docker/Podman orchestration tool | DT-7 in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| F8 | MCP deep IDE integration | DT-8 in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - Missing tech spec |
| F9 | MCP client for external servers | DT-9 in `01-biz-features-deploy.md` | 14.5 in `02-tech-pipeline.md` | COVERED |

### Workstream G: Voice (OUT OF SCOPE)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| G (forward-compat) | VoiceHandler trait, binary payloads, voice feature flag | Section 6a in `01-biz-features-deploy.md` | 14.1.6 VoiceHandler in `02-tech-core.md`, ChannelAdapter binary support in 14.1.2 | COVERED |

### Workstream H: Memory & Workspace (Week 4-8)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| H1 | Markdown workspace with per-agent isolation | MW-1, MW-3 in `01-biz-features-deploy.md` | 15.1 in `02-tech-pipeline.md` | COVERED |
| H2 | Complete RVF Phase 3 (vector memory) | MW-2, MW-4, MW-5, MW-6, MW-7 in `01-biz-features-deploy.md` | 15.2 in `02-tech-pipeline.md` | COVERED |
| H3 | Standardize timestamp representations | MW-8 in `01-biz-features-deploy.md` | 15.3 in `02-tech-pipeline.md` | COVERED |

### Workstream I: Type Safety & Cleanup (Week 2-6)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| I1 | DelegationTarget serde consistency | TS-1 in `01-biz-stability.md` | 15.1 in `02-tech-core.md` | COVERED |
| I2 | String-typed policy modes to enums | TS-2 in `01-biz-stability.md` | 15.2 in `02-tech-core.md` | COVERED |
| I3 | ChatMessage::content serialization | TS-3 in `01-biz-stability.md` | 15.3 in `02-tech-core.md` | COVERED |
| I4 | Job ID collision fix | TS-4 in `01-biz-stability.md` | 15.4 in `02-tech-core.md` | COVERED |
| I5 | camelCase normalizer acronym handling | TS-5 in `01-biz-stability.md` | 15.5 in `02-tech-core.md` | COVERED |
| I6 | Dead code removal | TS-6 in `01-biz-stability.md` | Not in tech drafts | PARTIAL - Missing tech spec listing specific removals |
| I7 | Fix always-true test assertion | TS-7 in `01-biz-stability.md` | 15.6 in `02-tech-core.md` | COVERED |
| I8 | Share MockTransport across crates | TS-8 in `01-biz-stability.md` | 15.7 in `02-tech-core.md` | COVERED |

### Workstream J: Documentation & Docs Sync (Week 3-5)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| J1 | Fix provider counts | DS-1 in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - No tech spec needed (doc changes), but adequately specified in biz |
| J2 | Fix assembler truncation description | DS-2 in `01-biz-features-deploy.md` | Not in tech drafts | COVERED (doc-only) |
| J3 | Fix token budget source reference | DS-3 in `01-biz-features-deploy.md` | Not in tech drafts | COVERED (doc-only) |
| J4 | Document identity bootstrap behavior | DS-4 in `01-biz-features-deploy.md` | Not in tech drafts | COVERED (doc-only) |
| J5 | Document rate-limit retry behavior | DS-5 in `01-biz-features-deploy.md` | Not in tech drafts | COVERED (doc-only) |
| J6 | Document CLI log level change | DS-6 in `01-biz-features-deploy.md` | Not in tech drafts | COVERED (doc-only) |
| J7 | Plugin system documentation | DS-7 in `01-biz-features-deploy.md` | Not in tech drafts | COVERED (doc-only) |

### Workstream K: Deployment & Community (Week 8-12)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| K (forward-compat) | UI/dashboard hooks | Section 6b in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - Biz hooks specified, but no tech spec for structured/binary payload support in bus |
| K2 | Docker images | DC-1, DC-2 in `01-biz-features-deploy.md` | 16.1 in `02-tech-pipeline.md` | COVERED |
| K3 | Enhanced sandbox with per-agent isolation | DC-3, DC-4 in `01-biz-features-deploy.md` | 16.2 in `02-tech-pipeline.md` | COVERED |
| K3a | Security plugin system | DC-5, DC-6, DC-7 in `01-biz-features-deploy.md` | 16.3 in `02-tech-pipeline.md` | COVERED |
| K4 | ClawHub skill registry with vector search | DC-8, DC-9, DC-10 in `01-biz-features-deploy.md` | 16.4 in `02-tech-pipeline.md` | COVERED |
| K5 | Benchmarks vs OpenClaw | DC-11 in `01-biz-features-deploy.md` | Not in tech drafts | PARTIAL - Missing tech spec for benchmark harness |

### Workstream L: Multi-Agent Routing & Orchestration (Week 5-9)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| L1 | Agent routing table | MA-1, MA-2 in `01-biz-features-deploy.md` | 17.1 in `02-tech-pipeline.md` | COVERED |
| L2 | Per-agent workspace and session isolation | MA-3 in `01-biz-features-deploy.md` | 17.2 in `02-tech-pipeline.md` | COVERED |
| L3 | Multi-agent swarming | MA-4 in `01-biz-features-deploy.md` | 17.3 in `02-tech-pipeline.md` | COVERED |
| L4 | Planning strategies in Router | MA-5 in `01-biz-features-deploy.md` | 17.4 in `02-tech-pipeline.md` | COVERED |

### Workstream M: Claude Flow / Claude Code Integration (Week 3-7)

| Item | Description | Biz Draft | Tech Draft | Status |
|------|-------------|-----------|------------|--------|
| M1 | Implement FlowDelegator | CF-1 in `01-biz-features-deploy.md` | 18.1 in `02-tech-pipeline.md` | COVERED |
| M2 | Wire flow_available to runtime detection | CF-2 in `01-biz-features-deploy.md` | 18.2 in `02-tech-pipeline.md` | COVERED |
| M3 | Enable delegate feature by default | CF-3 in `01-biz-features-deploy.md` | Feature flags in 16 of `02-tech-core.md` | COVERED |
| M4 | Dynamic MCP server discovery | CF-4, CF-6 in `01-biz-features-deploy.md` | 18.3 in `02-tech-pipeline.md` | COVERED |
| M5 | Claude Code as MCP client transport | CF-5 in `01-biz-features-deploy.md` | 18.4 in `02-tech-pipeline.md` | COVERED |
| M6 | Delegation config documentation | CF-7 in `01-biz-features-deploy.md` | 18.5 referenced in `02-tech-pipeline.md` | COVERED |

### Cross-Cutting Concerns

| Item | Description | Status |
|------|-------------|--------|
| Keep core tiny (< 10 MB base, < 100ms cold start) | Mentioned in `01-biz-features-deploy.md` non-goals/feature summary indirectly via plugin approach | PARTIAL - Not explicitly called out as a requirement in any draft |
| Offline capability | Not mentioned in any draft | MISSING |
| No core forks (all new capabilities as plugins post-C) | Implied in `01-biz-plugins-channels.md` CE-14 and PS-1 | PARTIAL - Should be an explicit architectural constraint |
| Recommended new dependencies | Listed in `02-tech-core.md` (section 18) and `02-tech-pipeline.md` (per feature) | COVERED |

---

## 2. Section Numbering Conflicts

### CRITICAL: All four business requirement drafts propose section "5d"

| Draft | Proposed Section | Content |
|-------|-----------------|---------|
| `01-biz-stability.md` | **5d** | Stability & Security Fixes |
| `01-biz-architecture.md` | **5d** | Architecture Cleanup |
| `01-biz-plugins-channels.md` | **5d** | Plugin & Skill System |
| `01-biz-features-deploy.md` | **5d** | Software Dev & App Tooling |

Additionally, two drafts propose section "5e":

| Draft | Proposed Section | Content |
|-------|-----------------|---------|
| `01-biz-stability.md` | **5e** | Type Safety & Cleanup |
| `01-biz-architecture.md` | **5e** | Pipeline & LLM Reliability |
| `01-biz-plugins-channels.md` | **5e** | Channel Enhancements |
| `01-biz-features-deploy.md` | **5e** | Memory & Workspace |

### Proposed Resolution

The existing `01-business-requirements.md` has sections 5b (Workspace) and 5c (Tiered Routing). The new sections should be numbered sequentially:

| Section | Content | Source Draft |
|---------|---------|-------------|
| 5d | Stability & Security Fixes (A) | `01-biz-stability.md` |
| 5e | Type Safety & Cleanup (I) | `01-biz-stability.md` |
| 5f | Architecture Cleanup (B) | `01-biz-architecture.md` |
| 5g | Pipeline & LLM Reliability (D) | `01-biz-architecture.md` |
| 5h | Plugin & Skill System (C) | `01-biz-plugins-channels.md` |
| 5i | Channel Enhancements (E) | `01-biz-plugins-channels.md` |
| 5j | Software Dev & App Tooling (F) | `01-biz-features-deploy.md` |
| 5k | Memory & Workspace (H) | `01-biz-features-deploy.md` |
| 5l | Deployment & Community (K) | `01-biz-features-deploy.md` |
| 5m | Multi-Agent Routing & Orchestration (L) | `01-biz-features-deploy.md` |
| 5n | Claude Flow / Claude Code Integration (M) | `01-biz-features-deploy.md` |
| 5o | Documentation Sync (J) | `01-biz-features-deploy.md` |

**Rationale**: Order follows the dependency chain (stability first, then architecture, then features that depend on both).

### Technical Requirements Section Numbering Conflicts

Both tech drafts start at section 12, but with different content:

| Draft | Section 12 | Content |
|-------|-----------|---------|
| `02-tech-core.md` | 12 | Workstream A: Critical Fixes |
| `02-tech-pipeline.md` | 12 | Pipeline & LLM Reliability (Workstream D) |

**Proposed Resolution**: `02-tech-core.md` keeps sections 12-19 as-is (A, B, C, I, feature flags, dependency graph, workspace deps, testing). `02-tech-pipeline.md` renumbers to start at section 20:

| Section | Content | Source Draft |
|---------|---------|-------------|
| 12-19 | Workstreams A, B, C, I (as in `02-tech-core.md`) | `02-tech-core.md` |
| 20 | Pipeline & LLM Reliability (D) | `02-tech-pipeline.md` |
| 21 | Channel Enhancements (E) | `02-tech-pipeline.md` |
| 22 | Software Dev & App Tooling (F) | `02-tech-pipeline.md` |
| 23 | Memory & Workspace (H) | `02-tech-pipeline.md` |
| 24 | Deployment (K) | `02-tech-pipeline.md` |
| 25 | Multi-Agent Routing & Orchestration (L) | `02-tech-pipeline.md` |
| 26 | Claude Flow / Claude Code Integration (M) | `02-tech-pipeline.md` |

---

## 3. User Story ID Conflicts

All user story ID prefixes across the four business drafts are **unique**. No conflicts detected:

| Prefix | Domain | Draft |
|--------|--------|-------|
| SF-1 through SF-9 | Stability Fixes | `01-biz-stability.md` |
| TS-1 through TS-8 | Type Safety | `01-biz-stability.md` |
| AC-1 through AC-9 | Architecture Cleanup | `01-biz-architecture.md` |
| PR-1 through PR-11 | Pipeline Reliability | `01-biz-architecture.md` |
| PS-1 through PS-15 | Plugin & Skill System | `01-biz-plugins-channels.md` |
| CE-1 through CE-14 | Channel Enhancements | `01-biz-plugins-channels.md` |
| DT-1 through DT-9 | Dev Tools | `01-biz-features-deploy.md` |
| MW-1 through MW-8 | Memory & Workspace | `01-biz-features-deploy.md` |
| DC-1 through DC-11 | Deployment & Community | `01-biz-features-deploy.md` |
| MA-1 through MA-5 | Multi-Agent | `01-biz-features-deploy.md` |
| CF-1 through CF-7 | Claude Flow | `01-biz-features-deploy.md` |
| DS-1 through DS-7 | Documentation Sync | `01-biz-features-deploy.md` |

These also do not conflict with existing IDs in the base `01-business-requirements.md` (WS-*, TR-*, G*).

**Assessment**: No ID conflicts. The ID scheme is well-designed.

---

## 4. Duplicate Content Requiring Consolidation

### 4.1 Skill Hot-Reload Race Condition Risk

**Duplicate risk register entries across two drafts**:

- `01-biz-plugins-channels.md` risk table: "Skill hot-reload race condition" (Score 4, mitigation: "Atomic swap of skill registry; brief lock during reload; version counter for cache invalidation")
- `01-biz-features-deploy.md` risk table (Section 7b): "Skill hot-reload race condition" (Score 4, mitigation: "Atomic swap of skill registry; drain in-flight tool calls before swap; filesystem watcher debounce")

**Fix**: Consolidate into one entry. The `01-biz-features-deploy.md` version is more complete (adds drain in-flight and debounce). Use that version, merge in the "version counter for cache invalidation" from the other.

### 4.2 WASM Plugin Sandbox Escape Risk

- `01-biz-plugins-channels.md` risk table: "WASM plugin sandbox escape" (Score 5, mitigation references K3)
- `01-biz-features-deploy.md` risk table: "WASM plugin sandbox escape" (Score **8**, mitigation adds seccomp/landlock and audits)

**Inconsistency**: Scores differ (5 vs 8). Since Low likelihood * Critical impact typically yields 5-8, the two drafts disagree on scoring.

**Fix**: Use Score 8 from `01-biz-features-deploy.md` (more conservative). Merge mitigations from both.

### 4.3 Plugin & Skill System Success Criteria

- `01-biz-plugins-channels.md` has detailed success criteria under "Plugin & Skill System (Phase 5: Extend)" with 14 items
- `01-biz-features-deploy.md` has "Plugin & Dev Tools (Phase 5)" with 5 items that partially overlap

**Fix**: Keep the detailed 14-item list from `01-biz-plugins-channels.md` as the primary. Move the dev-tool-specific items from `01-biz-features-deploy.md` into a separate "Dev Tools" subsection.

### 4.4 Skill Precedence Layering

Described in both:
- `01-biz-plugins-channels.md` PS-9 and feature summary
- `02-tech-core.md` Section 14.5

The descriptions are consistent. The tech spec adds a 4th layer (plugin-shipped skills). The biz draft mentions plugin-shipped skills in PS-13. No action needed -- just verify consistency when integrating.

---

## 5. Gaps and Missing Items

### 5.1 Missing Technical Specifications

The following items have business requirement coverage but **lack corresponding technical specifications**:

| Item | Missing From | Severity |
|------|-------------|----------|
| B4 (Cron storage unification) | `02-tech-core.md` or `02-tech-pipeline.md` | Medium -- needs migration plan and API specification |
| B8 (Consolidate build_messages) | `02-tech-core.md` | Low -- straightforward refactor, can be spec'd inline |
| B9 (MCP protocol version constant) | `02-tech-core.md` | Low -- trivial change |
| C4a (Autonomous skill creation) | Either tech draft | Medium -- needs detection heuristic and WASM compilation pipeline |
| C5 (Slash-command registry wiring) | Either tech draft | Low -- wiring existing dead code |
| C6 (MCP server skill exposure) | Either tech draft | Medium -- needs tool schema generation from skills |
| C7 (PluginHost unification) | Either tech draft | Medium -- needs migration path from old Channel trait |
| D6 (Thread sender_id for cost recording) | `02-tech-pipeline.md` | Medium -- critical for per-user billing |
| D9 (MCP transport concurrency) | `02-tech-pipeline.md` | Medium -- needs multiplexer design |
| D10 (Cache skill/agent bootstrap files) | `02-tech-pipeline.md` | Low -- standard caching pattern |
| D11 (Async file I/O in skills loader) | `02-tech-pipeline.md` | Low -- `tokio::fs` replacement |
| E6 (Enhanced heartbeat) | `02-tech-pipeline.md` | Low -- extends existing CronService |
| F2 (Cargo/build integration) | `02-tech-pipeline.md` | Low -- skill/tool pattern already established |
| F5 (Calendar integration) | `02-tech-pipeline.md` | Low -- follows OAuth2 pattern from F6 |
| F7 (Docker/Podman tool) | `02-tech-pipeline.md` | Low -- standard tool pattern |
| F8 (MCP IDE integration) | `02-tech-pipeline.md` | Medium -- needs VS Code extension architecture |
| I6 (Dead code removal) | `02-tech-core.md` | Low -- needs enumeration of specific items |
| K5 (Benchmark suite) | `02-tech-pipeline.md` | Low -- testing infrastructure |

### 5.2 Missing Business Requirements

| Item | Description | Severity |
|------|-------------|----------|
| Cross-cutting: "Offline capability" | `improvements.md` explicitly lists "All local-first where possible. Cloud is always a fallback, never required" but no draft captures this as a business requirement | Medium |
| Cross-cutting: "No core forks" | `improvements.md` says "After Workstream C, all new capabilities are plugins" -- this is an architectural policy that should be an explicit Non-Goal or Constraint addition | Medium |
| Cross-cutting: "Keep core tiny" | Binary size targets (< 10 MB base, < 100ms cold start) from `improvements.md` are not restated in any draft | Low (already in base doc G2/G3) |

---

## 6. Factual Errors and Inconsistencies

### 6.1 `01-biz-stability.md` Non-Goals vs Source

The non-goal "Supporting custom timeout values per-provider (global default is adequate for this phase; per-provider config is Workstream D)" is slightly misleading. The tech spec in `02-tech-core.md` Section 12.7 actually adds `timeout_secs` to `ProviderConfig`, making per-provider timeout possible within the A7 fix itself. The biz non-goal should be revised to clarify that the A7 fix provides per-provider config, while D4 addresses retry-specific configuration.

**File**: `/home/aepod/dev/clawft/.planning/drafts/01-biz-stability.md`
**Change**: Revise the third non-goal bullet to: "Advanced timeout strategies (circuit breaker patterns, adaptive timeouts) -- a simple configurable per-provider timeout is included in A7; adaptive behavior is Workstream D"

### 6.2 `02-tech-core.md` Section 12.4 - FeishuConfig and QQConfig

The `SecretRef` migration table lists `FeishuConfig`, `DingTalkConfig`, `MochatConfig`, and `QQConfig`. However, the existing `01-business-requirements.md` Section 2 Non-Goals states: "Feishu, DingTalk, Mochat, QQ (Chinese platform channels). These can be added later as community-contributed plugins." If these channels are out of scope, their config structs may not exist or may be dead code.

**Recommendation**: Verify whether these config structs actually exist in the codebase (`clawft-types/src/config.rs`). If they exist as legacy compatibility with Python nanobot config, the migration is still appropriate. If they do not exist, remove them from the table.

### 6.3 `02-tech-core.md` Section 13.2 - LlmMessage Dedup Claim

The draft states `context.rs` is "missing `tool_calls` field." This should be verified against the actual codebase, since `improvements.md` says "Two separate structs with identical fields" and the TODO comment acknowledges this. The tech spec should accurately reflect the current state.

**File**: `/home/aepod/dev/clawft/.planning/drafts/02-tech-core.md`
**Change**: Verify the field difference claim against the actual code in `crates/clawft-core/src/agent/context.rs` and `crates/clawft-core/src/pipeline/traits.rs` before merging.

### 6.4 `02-tech-pipeline.md` Section 18.1 - FlowDelegator uses --print

The FlowDelegator implementation uses `cmd.arg("--print")` which runs Claude Code in non-interactive print mode. However, `improvements.md` M1 says: "Passes tasks via `claude --print` (non-interactive mode) or `claude --json`". The tech spec hardcodes `--print` and `--output-format json` but the flag is actually `--output-format` not `--json` as a standalone flag. This appears correct in the code but the comment in improvements.md is slightly different. No action needed -- the tech spec correctly uses `--output-format json`.

### 6.5 `01-biz-architecture.md` Success Criteria Phase Labeling

- Architecture Cleanup is labeled "Phase 3G+"
- Pipeline & LLM Reliability is labeled "Phase 4"

The base document uses Phase 1 (Warp), Phase 2 (Weft), Phase 3 (Finish), Phase 3G+ (Workspace), Phase 4 (Tiered Routing). These phase labels need updating since the sprint plan introduces new milestones (Week 1-2, 2-4, etc.) that do not cleanly map to the existing phase numbering.

**Recommendation**: Either (a) create new phase names for the sprint plan milestones, or (b) use week ranges instead of phase numbers in success criteria.

### 6.6 `01-biz-features-deploy.md` - WhatsApp Non-Goal Removal

Section 7a proposes removing the Non-Goal "WhatsApp channel in initial release (punted to future phase)" because WhatsApp is now in scope. This is correct per `improvements.md` E3. However, the Non-Goal also says "punted to future phase" -- and E3 is indeed in a future phase from the original MVP perspective. The removal is appropriate but should add a note about which phase WhatsApp is targeted for.

---

## 7. Consistency Between Business and Technical Requirements

### 7.1 Alignment Check

| Workstream | Biz-Tech Alignment | Notes |
|-----------|-------------------|-------|
| A (Critical Fixes) | Strong | All 9 items have detailed biz stories AND tech specs. Migration paths documented. |
| B (Architecture) | Good | 3 of 9 items missing tech specs (B4, B8, B9) -- all are straightforward |
| C (Plugin System) | Good | C1-C4 have excellent tech coverage. C4a, C5, C6, C7 need tech specs added |
| D (Pipeline) | Moderate | D1-D5, D7, D8 well covered. D6, D9, D10, D11 need tech specs |
| E (Channels) | Strong | All items covered in both layers with good architectural detail |
| F (Dev Tools) | Moderate | F1, F3, F4, F6, F9 have tech specs. F2, F5, F7, F8 need them |
| H (Memory) | Strong | All items well covered with code-level specs |
| I (Type Safety) | Strong | 7 of 8 items have tech specs. I6 needs specific enumeration |
| J (Docs) | N/A | Documentation-only; no tech specs needed beyond file paths |
| K (Deployment) | Good | K2, K3, K3a, K4 covered. K5 needs benchmark harness spec |
| L (Multi-Agent) | Strong | All items have detailed tech architecture |
| M (Claude Flow) | Strong | All items well covered with implementation code |

### 7.2 Contradictions Found

None found. The business and technical requirements are consistent with each other.

---

## 8. Forward Compatibility Assessment

### 8.1 Voice (Workstream G)

Requirements from `improvements.md`:
1. VoiceHandler trait placeholder in C1 -- **COVERED** in `02-tech-core.md` Section 14.1.6
2. Plugin manifest reserves `voice` capability -- **COVERED** in `02-tech-core.md` Section 14.1.1 (`PluginCapability::Voice`)
3. ChannelAdapter supports binary payloads -- **COVERED** in `02-tech-core.md` Section 14.1.2 (`MessagePayload::Binary`)
4. Feature flag `voice` wired as no-op -- **COVERED** in `02-tech-core.md` Section 16 (`voice = []`)

**Assessment**: All four voice forward-compatibility requirements are properly addressed.

### 8.2 UI (Workstream K1/K6)

Requirements from `improvements.md`:
1. Bus supports structured/binary payloads -- **COVERED** in `01-biz-features-deploy.md` Section 6b but **MISSING** a specific tech spec showing the bus type changes
2. MCP server tool schemas stable -- **PARTIALLY COVERED** -- referenced in biz, no specific stability guarantees in tech spec
3. Config/session APIs readable without agent loop -- **PARTIALLY COVERED** -- referenced in biz, no tech spec

**Assessment**: The biz requirements correctly list the hooks but the tech drafts do not include concrete implementation guidance for the bus payload changes or direct-access APIs. These should be added to `02-tech-pipeline.md`.

---

## 9. Risk Register Consistency

### 9.1 Duplicate Risks (Requiring Consolidation)

| Risk | Draft 1 | Draft 2 | Resolution |
|------|---------|---------|------------|
| Skill hot-reload race condition | `01-biz-plugins-channels.md` (Score 4) | `01-biz-features-deploy.md` (Score 4) | Merge into one entry; use more complete mitigation from features-deploy draft |
| WASM plugin sandbox escape | `01-biz-plugins-channels.md` (Score 5) | `01-biz-features-deploy.md` (Score 8) | Use Score 8 (more conservative); merge mitigations |

### 9.2 Risk Coverage Assessment

All workstreams have appropriate risk entries. The risk register additions across drafts are comprehensive and cover:
- Security risks (sandbox escape, SSRF, credential leaks, supply-chain)
- Reliability risks (race conditions, message drops, token refresh failures)
- Migration risks (session keys, cron storage, serde changes, hash migration)
- Operational risks (API rate limits, CLI availability, protocol drift)
- Performance risks (bounded channels, infinite planning loops)

### 9.3 Missing Risks

| Risk | Workstream | Recommended Score | Mitigation |
|------|-----------|------------------|------------|
| Plugin trait API instability causing churn | C | Medium likelihood, Medium impact = 4 | Stabilize trait API before community adoption; version the trait crate; provide migration guide |
| `serde_yaml` crate maintenance status | C3 | Low likelihood, Low impact = 2 | `serde_yaml` is actively maintained; if abandoned, YAML parsing can fall back to `serde_yml` fork |
| `instant-distance` HNSW crate maturity | H2 | Medium likelihood, Medium impact = 4 | Benchmark against `hnsw` and `hnswlib` crates; wrap behind trait for swappability |

---

## 10. Success Criteria Assessment

### 10.1 Specificity and Testability

| Draft | Criteria Quality | Issues |
|-------|-----------------|--------|
| `01-biz-stability.md` | **Excellent** (9/10) | Every criterion is specific and testable. Clear pass/fail conditions. |
| `01-biz-architecture.md` | **Very Good** (8/10) | Most criteria are testable. The "all source files under 500 lines" criterion should exclude test files and generated code (mentioned but worth emphasizing). |
| `01-biz-plugins-channels.md` | **Very Good** (8/10) | Good specificity. "Hot-reload detects file changes...within 2 seconds" is admirably precise. "Autonomous skill creation generates valid SKILL.md" needs a more specific validation definition. |
| `01-biz-features-deploy.md` | **Good** (7/10) | Some criteria are too vague: "At least 3 dev tool plugins...functional" -- what constitutes "functional"? "HNSW vector search returns results in < 10ms for 100K vectors" is excellent. |
| `02-tech-core.md` | **Excellent** (9/10) | Testing requirements per section are specific with clear unit test descriptions. Property testing for A1 is a nice touch. |
| `02-tech-pipeline.md` | **Very Good** (8/10) | Good code-level specificity. Some `todo!()` placeholders need implementation guidance. |

### 10.2 Criteria Gaps

The following workstream items lack explicit success criteria:

- D6 (sender_id threading) -- biz draft mentions it but no success criterion
- D9 (MCP transport concurrency) -- mentioned in PR-9 story but no specific testable criterion
- D10, D11 -- mentioned in stories but success criteria are combined under a generic "skills loader uses tokio::fs" criterion
- F2 (Cargo integration) -- no success criterion
- F5 (Calendar) -- no success criterion
- F7 (Docker tool) -- no success criterion
- F8 (MCP IDE) -- no success criterion

---

## 11. Style Consistency

### 11.1 Match with Existing Document Style

The existing `01-business-requirements.md` uses:
- Markdown tables for user stories with ID/Story/Priority columns
- Feature Summary tables with Feature/Description/Config Layer columns
- Non-Goals sections per feature area
- Risk Register as a table with Risk/Likelihood/Impact/Score/Mitigation columns
- Success Criteria as checkbox lists

**Assessment**: All four business drafts follow this style consistently. The `01-biz-plugins-channels.md` and `01-biz-features-deploy.md` drafts add a "Dependencies" table that is not in the original but is a valuable addition.

The existing `02-technical-requirements.md` uses:
- Section-per-crate organization with code blocks
- Rust code examples with comments
- Dependency tables
- Feature flag TOML blocks

**Assessment**: Both tech drafts match this style well. `02-tech-core.md` is slightly more verbose with full implementations while `02-tech-pipeline.md` uses more `todo!()` stubs. The `02-tech-core.md` style (more complete) is preferred.

### 11.2 Minor Style Issues

1. `01-biz-stability.md` uses pipe-escaped `|` in the TS-7 assertion example (`result.is_err() \|\| result.is_ok()`). This is correct for table rendering but looks odd.

2. `01-biz-features-deploy.md` Section 7a proposes modifying the Non-Goals section of the existing document. This is the only draft that modifies existing content rather than adding new sections. This is appropriate but should be clearly flagged as a MODIFICATION rather than an ADDITION during integration.

3. `02-tech-pipeline.md` uses `default_true()` function but never defines it in the same file (it is assumed to exist). The definition should be included or referenced.

---

## 12. Overall Quality Assessment

| Draft | Score | Strengths | Weaknesses |
|-------|-------|-----------|------------|
| `01-biz-stability.md` | **9/10** | Excellent coverage of A and I. Clear user stories, specific success criteria, thoughtful migration notes, well-scoped non-goals. | Minor: non-goal about per-provider timeout is slightly misleading (see 6.1). |
| `01-biz-architecture.md` | **8/10** | Good coverage of B and D. Strong user stories for both workstreams. Clear dependency mapping. | Missing tech specs for B4, B8, B9. Phase labeling does not match existing scheme. |
| `01-biz-plugins-channels.md` | **9/10** | Comprehensive coverage of C and E. Excellent channel capability matrix. Good dependency table. Thorough risk analysis. | Duplicate risk entry with features-deploy draft. Minor overlap in skill precedence description. |
| `01-biz-features-deploy.md` | **8/10** | Ambitious scope covering 6 workstreams (F, H, K, L, M, J). Good forward-compat section. Helpful story ID reference appendix. | Some success criteria too vague. Covers too many workstreams in one draft -- harder to review. Duplicate risks with plugins-channels draft. |
| `02-tech-core.md` | **9/10** | Excellent technical depth for A, B, C, I. Complete code examples. Good testing requirements. New crate specification is thorough. | Missing tech specs for B4, B8, B9, C4a, C5, C6, C7. |
| `02-tech-pipeline.md` | **8/10** | Strong coverage of D, E, F, H, K, L, M. Good architectural diagrams. Complete config examples. | Missing tech specs for several D items (D6, D9-D11), F items (F2, F5, F7, F8). Some `todo!()` stubs need completion. Section numbering conflicts with `02-tech-core.md`. |

---

## 13. Specific Suggested Fixes

### Fix 1: Resolve section numbering in all four biz drafts

**Files**: All four `/home/aepod/dev/clawft/.planning/drafts/01-biz-*.md`
**Change**: Renumber sections per the scheme in Section 2 of this review.

### Fix 2: Resolve section numbering in tech pipeline draft

**File**: `/home/aepod/dev/clawft/.planning/drafts/02-tech-pipeline.md`
**Change**: Renumber sections 12-18 to 20-26 to avoid conflict with `02-tech-core.md`.

### Fix 3: Consolidate duplicate risk entries

**File**: `/home/aepod/dev/clawft/.planning/drafts/01-biz-plugins-channels.md`
**Change**: Remove the "Skill hot-reload race condition" and "WASM plugin sandbox escape" risk entries. These will be kept in `01-biz-features-deploy.md` with merged mitigations.

**File**: `/home/aepod/dev/clawft/.planning/drafts/01-biz-features-deploy.md`
**Change**: Update "WASM plugin sandbox escape" mitigation to include "wasmtime's security model is well-tested" from the plugins-channels draft. Update "Skill hot-reload race condition" mitigation to include "version counter for cache invalidation" from the plugins-channels draft.

### Fix 4: Add missing tech specs for B4, B8, B9

**File**: `/home/aepod/dev/clawft/.planning/drafts/02-tech-core.md`
**Change**: Add sections 13.7 (B4: Cron storage unification -- specify JSONL format, migration CLI, CronService API), 13.8 (B8: build_messages consolidation -- show the shared function signature), 13.9 (B9: MCP protocol version constant -- trivial, just show the const definition).

### Fix 5: Add missing tech spec for D6 (sender_id threading)

**File**: `/home/aepod/dev/clawft/.planning/drafts/02-tech-pipeline.md`
**Change**: Add section showing how `sender_id` flows from `InboundMessage` -> `ChatRequest` -> `RoutingDecision` -> `CostTracker.update()`. This is a pipeline wiring change that needs explicit documentation.

### Fix 6: Add cross-cutting concerns as explicit biz requirements

**File**: `/home/aepod/dev/clawft/.planning/drafts/01-biz-features-deploy.md` (or a new short section)
**Change**: Add a "Cross-Cutting Requirements" section capturing:
- "All local-first where possible. Cloud is always a fallback, never required." (from improvements.md)
- "After Workstream C, all new capabilities are plugins. No more modifying clawft-core for features." (architectural constraint)
- Binary size and cold-start targets (already in base doc but worth re-affirming)

### Fix 7: Fix non-goal wording in stability draft

**File**: `/home/aepod/dev/clawft/.planning/drafts/01-biz-stability.md`
**Change**: Revise the third Non-Goal bullet under "Stability Fixes" from "Supporting custom timeout values per-provider" to "Advanced timeout strategies (circuit breaker, adaptive timeouts) -- a simple per-provider timeout is part of A7; adaptive behavior is in Workstream D".

### Fix 8: Add forward-compat tech specs for UI hooks

**File**: `/home/aepod/dev/clawft/.planning/drafts/02-tech-pipeline.md`
**Change**: Add a brief section under D8 (bounded bus) or as a new section showing:
- `InboundMessage` and `OutboundMessage` should carry an optional `payload: Option<MessagePayload>` field (reusing the type from `clawft-plugin`)
- Config and session modules should expose read-only accessor functions that do not require an `AgentLoop` reference

### Fix 9: Standardize phase labels in success criteria

**Files**: `/home/aepod/dev/clawft/.planning/drafts/01-biz-architecture.md` and `/home/aepod/dev/clawft/.planning/drafts/01-biz-features-deploy.md`
**Change**: Replace "Phase 3G+", "Phase 4", "Phase 5", "Phase 6" labels with week-based milestones matching the sprint plan: "Sprint Week 2-4", "Sprint Week 4-8", "Sprint Week 8-12", etc. Or define a clear phase mapping in an integration preamble.

### Fix 10: Verify Chinese platform config structs exist

**File**: `/home/aepod/dev/clawft/.planning/drafts/02-tech-core.md`
**Change**: Before merging, verify whether `FeishuConfig`, `DingTalkConfig`, `MochatConfig`, `QQConfig` actually exist in `crates/clawft-types/src/config.rs`. If they do, the SecretRef migration is correct. If they do not, remove them from the table in Section 12.4.

---

## 14. Summary

### What is Done Well

- **Comprehensive coverage**: 95%+ of `improvements.md` items are represented in at least the business layer
- **User stories are well-written**: Each follows the "As a [persona], I want [capability] so that [value]" format consistently
- **Technical depth is excellent**: The tech drafts provide implementation-ready code blocks, dependency specifications, and testing requirements
- **Forward compatibility**: Voice and UI hooks are properly addressed
- **Risk management**: Thorough risk entries with actionable mitigations
- **Backward compatibility**: Migration paths and serde aliases are thoughtfully documented

### What Needs Work

1. **Section numbering conflicts** (Critical -- must fix before integration)
2. **Duplicate risk entries** (Medium -- consolidate the two instances)
3. **Missing tech specs** for ~15 items (Medium -- most are low-complexity items that can be brief)
4. **Cross-cutting concerns** not captured as explicit requirements (Medium)
5. **Phase labeling inconsistency** across drafts (Low -- cosmetic but confusing)
6. **Forward-compat UI hooks** need tech-level specifics (Low-Medium)

### Recommendation

The drafts are in good shape overall. Address the critical section numbering issue, consolidate the duplicate risks, and add the highest-priority missing tech specs (B4, D6, D9) before integration. The remaining missing tech specs (C4a-C7, D10-D11, F2/F5/F7/F8, I6, K5) can be added as stub sections with `TODO` markers during integration and fleshed out when those workstreams begin.

**Overall readiness for integration**: 85%. With the fixes above, readiness would reach 95%.
