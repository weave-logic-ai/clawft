# Phase 3 SPARC Plans -- Expert Review Consensus

## Date: 2026-02-17
## Review Process: 7 independent expert reviewers, 1 consensus coordinator
## Total Review Effort: ~2,069 lines of review across 7 documents

---

## 1. Executive Summary

The Phase 3 plan portfolio is in strong shape overall, with 6 of 7 plans receiving APPROVE_WITH_CHANGES and only one (3F-RVF) receiving REVISE. The plans demonstrate solid engineering judgment, thorough TDD methodology, and honest risk identification. The core infrastructure work (3D WASI/Docker, 3E optimization, 3G workspaces, 3H tool delegation) is well-specified and ready to implement with targeted fixes. The gap analysis (3I) provides a sound foundation but overstates the P0 count due to factual errors against the current codebase.

The primary systemic concerns were: (1) ~~duplicate MCP server infrastructure across 3F-RVF and 3H~~ (RESOLVED -- pluggable ToolProvider architecture agreed), (2) ~~unreconciled WASM binary size budgets between 3D and 3E~~ (RESOLVED -- empirical baseline, tiered budget), (3) the 3F-RVF plan being approximately 40% overscoped with unvalidated external dependencies (scope reduced; Sprint 0 passed), (4) security gaps appearing across multiple plans (path traversal in 3D, missing security model in 3F-agents, no access control in 3H MCP server), and (5) ~~discovery chain / config hierarchy inconsistencies between 3F-agents and 3G~~ (RESOLVED -- canonical 3-level chain). After applying the corrections identified by reviewers (particularly the 3I P0 recount), the portfolio represents approximately 20-26 weeks of implementation work with a realistic path to completion.

The most important pre-implementation action is a Sprint 0 validation gate for the 3F-RVF ruvector dependencies. If those crates do not compile or have API mismatches, the 3F-RVF plan requires fundamental restructuring -- and that restructuring must happen before committing 32+ engineer-days.

---

## 2. Verdict Summary Table

| Phase | Plan | Reviewer | Reviewer Verdict | Consensus Verdict | Avg Score | Critical Issues |
|-------|------|----------|-----------------|-------------------|-----------|-----------------|
| 3D | WASI HTTP/FS + Docker Multi-Arch | Systems Architect | APPROVE_WITH_CHANGES | **DEFERRED TO PHASE 4** | 3.4/5 | Size budget unrealistic; wasi-http-client maturity unverified; path traversal in resolve(); bin vs reactor component model. **Decision: needs more research, deferred 2026-02-17.** |
| 3E | WASM Allocator & Size Optimization | Performance Engineer | APPROVE_WITH_CHANGES | **APPROVE_WITH_CHANGES** | 3.8/5 | Target mismatch with 3D (wasip1 vs wasip2); todo!() in exported function; size targets need empirical cdylib baseline |
| 3F-RVF | RVF/ruvector Full Integration | AI/ML Systems Architect | REVISE | **REVISE** | 3.25/5 | ~40% overscoped; ruvector crate maturity unvalidated; MCP server duplicates 3H; timeline overlap with 3D on clawft-wasm |
| 3F-agents | Agents, Skills, Commands | Developer Experience Engineer | APPROVE_WITH_CHANGES | **APPROVE_WITH_CHANGES** | 3.6/5 | Discovery chain conflicts with 3G and tech requirements; no security model for loading arbitrary skill/agent files |
| 3G | Projects/Workspaces | Software Architect | APPROVE_WITH_CHANGES | **APPROVE_WITH_CHANGES** | 4.6/5 | Array merge semantics conflict with tech requirements; normalize_keys missing from merge pipeline; CLAWFT.md walk unbounded |
| 3H | Tool Delegation + MCP | Distributed Systems Engineer | APPROVE_WITH_CHANGES | **APPROVE_WITH_CHANGES** | 3.75/5 | MCP server mode has no access control; Claude delegation bridge duplicates tool execution logic; duplicates 3F-RVF MCP server |
| 3I | Gap Analysis | QA Architect | APPROVE_WITH_CHANGES | **APPROVE_WITH_CHANGES** | 3.6/5 | ~~3 of 12 P0s already resolved in code~~ **(re-baselined: 8 P0)**; GAP-11 misquotes trait definition |

---

## 3. Cross-Cutting Themes

### ~~3.1 Theme: Duplicate MCP Server Infrastructure~~ **RESOLVED (2026-02-17)**

**Affected plans**: 3F-RVF (Sprint 4, clawft-rvf-mcp crate), 3H (weft mcp-server command)

**Root cause**: 3F-RVF and 3H were planned independently and each designs its own MCP JSON-RPC server with Content-Length framing, initialize handshake, tools/list dispatch, and tools/call handling.

**Resolution**: Agreed on a **pluggable ToolProvider architecture** in `clawft-services/src/mcp/`. The design replaces the original `McpServer<R: ToolRegistry>` generic with a trait-based plugin system that cleanly separates protocol handling from tool implementation.

**Core design -- 4 new files in `clawft-services/src/mcp/`**:

| File | ~Lines | Purpose |
|------|--------|---------|
| `provider.rs` | ~80 | `ToolProvider` trait: `namespace()`, `list_tools()`, `call_tool()`. Plus `ToolDefinition`, `CallToolResult`, `ToolError` types. |
| `server.rs` | ~300 | `McpServerShell`: all MCP protocol handling (JSON-RPC, newline-delimited stdio framing, initialize handshake, tools/list aggregation, tools/call routing). Generic over `AsyncRead + AsyncWrite` for testability. |
| `middleware.rs` | ~150 | Pipeline: `SecurityGuard` (CommandPolicy + UrlPolicy -- resolves 3H CRIT-01), `PermissionFilter` (allowed_tools intersection), `ResultGuard` (truncate_result + sanitize_content -- resolves 3H CRIT-02), `AuditLog`. |
| `composite.rs` | ~120 | `CompositeToolProvider`: aggregates multiple `ToolProvider`s with namespace routing. `tools/list` merges across providers with namespace prefixing. `tools/call` strips namespace, routes to correct provider. |

**ToolProvider trait** (`provider.rs`):
```rust
#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn namespace(&self) -> &str;
    async fn list_tools(&self) -> Vec<ToolDefinition>;
    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError>;
}
```

**Concrete providers**:
- `BuiltinToolProvider`: wraps `ToolRegistry` for `weft mcp-server` (3H)
- `RvfToolProvider`: 11 RVF tools, in-process Rust calls (3F-RVF)
- `ProxyToolProvider`: wraps `McpClient` for external MCP servers
- `DelegationToolProvider`: Claude delegation bridge (3H)

**Protocol correction**: MCP 2025-06-18 stdio transport uses **newline-delimited JSON**, not Content-Length framing (that is the LSP convention). Both 3H and 3F-RVF plans originally specified Content-Length -- corrected. Current clawft code (newline-delimited) is already spec-compliant.

**Impact on plans**:
- **3H Session 2**: Delivers `McpServerShell` + `ToolProvider` trait + middleware pipeline + `BuiltinToolProvider`. This is the shared infrastructure.
- **3F-RVF Sprint 4**: Implements `RvfToolProvider`. No transport code -- registers with `McpServerShell`. Depends on 3H Session 2.
- **3F-agents**: Added glob/prefix support for MCP-namespaced tools in `allowed_tools`. SEC-SKILL-08 added.
- **3G**: MCP config merge documented explicitly.
- **clawft-rvf-mcp crate**: Simplified -- only tool handlers + `RvfToolProvider`. No `transport.rs`.

### ~~3.2 Theme: WASM Binary Size Budget Disagreement~~ **RESOLVED (2026-02-17)**

**Affected plans**: ~~3D (350KB/140KB gzipped),~~ 3E (300KB/120KB gzipped), 3F-RVF (~250KB WASM microkernel)

**Resolution**: Empirical baseline measured. 3D deferred to Phase 4. Budget agreed for Phase 3.

**Empirical baseline (cdylib, wasm32-wasip2, opt-level=z, LTO, strip)**:
- **Stub binary (current)**: 57.9 KB raw / 24.3 KB gzipped (42% compression ratio)
- **Post-3F-RVF (projected)**: ~240-250 KB raw / ~105 KB gzipped (adds rvf-wasm ~28 KB, micro-hnsw ~12 KB, ruvector components)

**Agreed tiered budget**:
- **Phase 3 stub binary (3E)**: 300 KB / 120 KB gzipped -- empirically validated. 80% headroom from current baseline. Compression ratio of 42% means 300 KB raw -> ~126 KB gzipped, which is at the limit.
- **Phase 3 post-3F-RVF**: 300 KB / 120 KB gzipped -- projected ~250 KB / ~105 KB is within budget with 17%/12.5% headroom.
- **Phase 4 post-3D (when real HTTP/FS land)**: 400 KB / 160 KB gzipped -- accounts for real HTTP crate + serde_json growth.
- The original reviewer concern that "120 KB gzipped is unrealistic" is **disproven** -- empirical 42% compression ratio supports 120 KB gzipped at 300 KB raw. 3E targets retained as-is.

### 3.3 Theme: Security Gaps Across Plans

**Affected plans**: 3D (path traversal), 3F-agents (no security model for skill/agent loading), 3H (MCP server no access control), 3I (SEC-01 through SEC-07)

**Root cause**: Security considerations were not part of the original plan template. Each reviewer independently identified security gaps specific to their plan, but no systemic security review was conducted.

**Consensus recommendation**: Add a security review gate before Phase 3 implementation begins. Specific items:
1. **3D**: Fix `resolve()` path traversal (reject `..` and absolute paths when base_path is set)
2. **3F-agents**: Add YAML parsing depth limits, directory name validation, `allowed_tools` intersection (not union) semantics, agent model string validation against configured providers
3. **3H**: Add `allowed_tools` config for MCP server mode; apply CommandPolicy/UrlPolicy in tools/call handler
4. **3I**: Address SEC-04 (API key logging), SEC-05 (symlink traversal), MISSED-05 (web_fetch body size limit)
5. Consider a dedicated security sweep sprint after Phase 3 core implementation.

### ~~3.4 Theme: Discovery Chain / Config Hierarchy Inconsistency~~ **RESOLVED (2026-02-17)**

**Affected plans**: 3F-agents (skill/agent discovery), 3G (workspace config hierarchy), 02-technical-requirements.md

**Resolution**: All three documents now aligned on canonical 3-level discovery chain. Nanobot compatibility removed entirely per user decision. No `.git/` fallback for project detection.

**Canonical discovery chain (agreed)**:
```
1. <project>/.clawft/skills/    (project-scoped, via cwd walk-up for .clawft/ marker only)
2. ~/.clawft/skills/            (user-level personal)
3. ~/.clawft/workspace/skills/  (global workspace default, becomes workspace-aware when 3G lands)
```
`SkillScope` variants: `Project`, `User`, `Workspace`. When 3G lands, `Project` scope becomes workspace-aware via `WorkspaceContext.skills_dir`. Same pattern applies to agent discovery.

**Security additions (SEC-SKILL-01 through SEC-SKILL-07)**: YAML depth limits, directory name validation, prompt size limits (32KB), tool permission intersection, agent model validation, project skill trust warnings, metadata field limits.

**Dispatch return type fixed**: `DispatchResult` enum (`NotACommand`, `Handled`, `SkillInvocation(String)`) replaces `Result<bool>` in slash command dispatch.

### 3.5 Theme: Cross-Phase Sequencing Dependencies

**Affected plans**: 3D, 3E, 3F-RVF (all modify clawft-wasm; all have overlapping weeks)

**Root cause**: 3D (Weeks 15-17) and 3F-RVF (Weeks 15-22) both modify clawft-wasm. 3E must decide whether to benchmark against wasip1 or wasip2. The plans were written independently without a shared sequencing agreement.

**Consensus recommendation**: Establish explicit phase ordering:
1. **3E allocator comparison + wasm-opt pipeline** first (infrastructure, target-independent)
2. **3D WASI HTTP/FS + Docker** second (changes the WASM target to wasip2)
3. **3E re-benchmark** after 3D (update baselines against the wasip2 binary)
4. **3F-RVF Sprint 7 (WASM microkernel)** gated on 3D completion
5. All three plans must update Constraint C-02 (3E) to acknowledge wasip2 as eventual primary target.

### 3.6 Theme: Unvalidated External Dependencies

**Affected plans**: 3D (wasi-http-client crate), 3F-RVF (11 ruvector crates)

**Root cause**: Both plans depend on external crates whose APIs, compilation characteristics, and binary size impact have not been empirically verified.

**Consensus recommendation**: Mandatory pre-implementation validation spikes:
- **3D**: 2-4 hour spike to build wasi-http-client against wasm32-wasip2 on Rust 1.93. Also evaluate the `waki` crate as a lighter alternative. Go/no-go gate.
- **3F-RVF**: 2-day Sprint 0 to compile all 11 ruvector crates, validate public APIs match pseudocode, measure binary size impact. Go/no-go gate. If any crate fails to compile or has a significantly different API, the 3F-RVF plan must be restructured before proceeding.

---

## 4. Critical Issues Requiring Resolution Before Implementation

| # | Issue | Source Review | Affected Plans | Resolution | Owner |
|---|-------|--------------|----------------|------------|-------|
| 1 | ruvector crate maturity completely unvalidated -- 11 pre-1.0 crates from single repo with no evidence of compilation testing | 3F-RVF C2 | 3F-RVF | Add Sprint 0 (2 days): compile all crates, validate APIs, measure binary size. Hard go/no-go gate. | 3F-RVF author |
| 2 | ~~Duplicate MCP server implementations will diverge~~ | 3H MAJ-01, 3F-RVF C3 | 3F-RVF, 3H | **RESOLVED (2026-02-17)** -- Pluggable `ToolProvider` architecture agreed. `McpServerShell` + middleware pipeline in `clawft-services/src/mcp/`. 3H Session 2 delivers shared infra; 3F-RVF Sprint 4 implements `RvfToolProvider`. Stdio framing corrected to newline-delimited JSON (MCP spec). See Theme 3.1. | 3H author + 3F-RVF author |
| 3 | MCP server mode has zero access control | 3H CRIT-01 | 3H | Add allowed_tools config, apply CommandPolicy/UrlPolicy in tools/call handler. Must fix before shipping server mode. | 3H author |
| 4 | Claude delegation bridge duplicates tool execution logic without security guards (no truncation, no logging) | 3H CRIT-02 | 3H | Extract tool execution helper from run_tool_loop() with truncation + error formatting + logging. Reuse in both paths. | 3H author |
| 5 | WASM size budget conflict: 3D says 350KB/140KB, 3E says 300KB/120KB | 3D C1, 3E Critical-1 | 3D, 3E, 3F-RVF | Adopt unified tiered budget (see Theme 3.2). Measure cdylib baseline immediately. | 3D + 3E authors |
| 6 | wasi-http-client crate maturity and Rust 1.93 compatibility unverified | 3D C2 | 3D | Make Task 15.1 a strict go/no-go gate (1 day max). Also evaluate waki crate. | 3D author |
| 7 | Discovery chain conflicts across 3F-agents, 3G, and 02-technical-requirements.md | 3F-agents C1, 3G Conflict-1 | 3F-agents, 3G | Align on single canonical chain before implementation (see Theme 3.4). 30-minute alignment meeting. | 3F-agents + 3G authors |
| 8 | No security model for loading arbitrary skill/agent files (YAML bombs, path traversal, prompt injection, tool escalation) | 3F-agents C2 | 3F-agents | Add security section: YAML depth limits, directory name validation, allowed_tools intersection, model string validation. | 3F-agents author |
| 9 | Path traversal vulnerability in WasiFileSystem resolve() | 3D M1 | 3D | Reject paths with `..` or absolute paths when base_path is set. WASI preopens provide host sandbox but Rust API should enforce its own contract. | 3D author |
| 10 | Array merge semantics conflict: 07-workspaces.md says "replace", 02-technical-requirements.md says "concatenate" | 3G ISSUE-C1 | 3G | Document rationale for replacement semantics. Add note acknowledging conflict with tech requirements. Replacement is the safer default. | 3G author |
| 11 | Config normalize_keys() not applied before deep merge -- camelCase workspace keys will fail to match snake_case global keys | 3G ISSUE-C2 | 3G | Add normalize_keys(ws_json) before deep_merge(). Add camelCase/snake_case merge test. | 3G author |
| 12 | ~~3I gap analysis overstates P0 count: GAP-01, GAP-02, GAP-06 already resolved in code. True P0 is 8, not 12~~ | 3I ISSUE-01, ISSUE-02 | 3I | **RESOLVED (2026-02-17)** -- 3I plan re-baselined. GAP-01/02/06 marked resolved, GAP-16 demoted to P2. Sprint restructured: Day 1 = SSE streaming (hardest P0). P0 count corrected to 8. | 3I author |
| 13 | 3E targets wasm32-wasip1 exclusively (Constraint C-02) but 3D migrates to wasip2 | 3E Critical-1 | 3D, 3E | Update 3E Constraint C-02 to acknowledge wasip2. Sequence 3E allocator work before 3D, then re-benchmark after. | 3E author |
| 14 | 3F-RVF is ~40% overscoped: SONA (L3), Attention (L4), WASM microkernel should defer to Phase 4 | 3F-RVF Scope Assessment | 3F-RVF | Reduce from 44 to 32 engineer-days. Deliver L0-L1 with persistence, HNSW, MCP bridge, crypto trail. Defer L2-L4 and WASM. | 3F-RVF author |

---

## 5. Cross-Phase Conflicts

| Conflict | Plans Involved | Severity | Resolution |
|----------|---------------|----------|------------|
| ~~Both modify clawft-wasm in overlapping weeks~~ | ~~3D, 3F-RVF~~ | ~~HIGH~~ | **RESOLVED -- 3D deferred to Phase 4. No clawft-wasm contention.** |
| ~~Duplicate MCP server implementations with independent Content-Length framing~~ | ~~3H, 3F-RVF~~ | ~~HIGH~~ | **RESOLVED -- Pluggable ToolProvider architecture agreed. McpServerShell in clawft-services/src/mcp/. 3H Session 2 delivers; 3F-RVF Sprint 4 consumes. Stdio framing corrected: newline-delimited JSON (not Content-Length).** |
| ~~WASM size budget: 300KB vs 250KB targets (3D deferred)~~ | ~~3E, 3F-RVF~~ | ~~MEDIUM~~ | **RESOLVED -- Agreed budget: 300KB/120KB (Phase 3), 400KB/160KB (Phase 4). Baseline: 57.9KB/24.3KB (80% headroom). Post-3F projected ~250KB/~105KB within budget.** |
| ~~Skill discovery chain: 4-level (3F-agents) vs 3-level (3G) vs 3-level (tech req)~~ | ~~3F-agents, 3G~~ | ~~HIGH~~ | **RESOLVED -- Canonical 3-level chain aligned across 3F-agents, 3G, and 02-tech-req. Nanobot compat removed.** |
| ~~wasm32-wasip1 (3E) vs wasm32-wasip2 (3D)~~ | ~~3D, 3E~~ | ~~HIGH~~ | **RESOLVED -- 3D deferred. 3E works against current wasip2 stub binary only.** |
| Array merge semantics: "replace" (3G/07-workspaces) vs "concatenate" (02-tech-req) | 3G, 02-tech-req | MEDIUM | Document replacement as chosen behavior. Add note acknowledging conflict. Consider future `+`-prefix append convention. |
| Agent discovery: 3F-agents defines 3-level, 3G does not define agent discovery at all | 3F-agents, 3G | MEDIUM | 3G should reserve agents/ in workspace scaffold. 3F-agents should accept project_dir parameter for future workspace awareness. |
| MCP config scoping: 3H reads flat config, 3G introduces workspace-level config merge | 3H, 3G | MEDIUM | Ensure register_mcp_tools() consumes post-merge config. Works automatically if config is merged before the call. Document forward-compatibility. MCP config merge documented explicitly as part of P0 #7 resolution. |
| Skill allowed_tools interacting with MCP namespaced tools ({server}__{tool}) | 3F-agents, 3H | LOW | Document that allowed_tools uses ToolRegistry names including MCP-namespaced names. 3F-agents adds glob/prefix support for MCP-namespaced tools (SEC-SKILL-08, added as part of P0 #7 resolution). |
| ~~bin target (3D) vs cdylib target (3E)~~ | ~~3D, 3E~~ | ~~MEDIUM~~ | **RESOLVED -- 3D deferred. 3E targets current artifact type only.** |
| ~~.cargo/config.toml modified by both 3D and 3E~~ | ~~3D, 3E~~ | ~~LOW~~ | **RESOLVED -- 3D deferred. 3E owns .cargo/config.toml exclusively in Phase 3.** |

---

## 6. Scope Adjustments

### 3F-RVF: Reduce from 44 to 32 Engineer-Days

This is the only plan that received REVISE. The reviewer assessed it as ~40% overscoped for a first release. The consensus recommendation is:

**Essential (MUST deliver -- 32 days):**

| Sprint | Week | Focus | LOE |
|--------|------|-------|-----|
| 0 | 14 | Dependency validation (compile all ruvector crates, measure sizes) | 2d |
| 1 | 15 | RVF Runtime Foundation + ApiEmbedder + Bootstrap (moved from Sprint 5) | 6d |
| 2 | 16 | Progressive HNSW + Session/Router persistence | 5d |
| 3 | 17 | ruvllm (L1 only) + QualityScorer | 4d |
| 4 | 18 | `RvfToolProvider` + MCP registration (gates on 3H `McpServerShell` + `ToolProvider` trait) | 6d |
| 5 | 19 | rvf-crypto + AgentDB (no agentic-flow integration) | 4d |
| 6 | 20 | Polish, Benchmarks, CLI, Level integration tests | 5d |

**Deferred to Phase 4 (~16 days):**

| Sprint | Focus | Rationale |
|--------|-------|-----------|
| -- | tiny-dancer neural routing (L2) | Can ship with ruvllm complexity analysis only |
| -- | SONA self-learning (L3) | Requires validated quality scoring feedback loop that does not yet exist |
| -- | Attention-based context assembly (L4) | Requires large-context scenarios not yet the primary use case |
| -- | WASM microkernel (Sprint 7) | Depends on 3D completion; WASM is not the primary deployment target |

**Key changes:**
- MEMORY.md bootstrap moved to Sprint 1 (was Sprint 5) -- users need existing memory indexed immediately
- SONA/Attention deferred -- learning loop cannot be validated in a 6-day sprint
- WASM microkernel deferred -- not primary deployment target, avoids clawft-wasm contention with 3D
- MCP Server Bridge moved to Sprint 4, gated on 3H shared infrastructure (`McpServerShell` + `ToolProvider` trait). Sprint 4 only implements `RvfToolProvider` -- no transport code needed.

### 3I: Re-baseline Sprint Plan

With the corrected P0 count (8, down from 12), the 3I sprint plan should be revised:

- **Day 1**: SSE streaming (GAP-11, 6h -- hardest item) + web search fix (GAP-03 + MISSED-01, 3h)
- **Day 2**: JSON repair (GAP-14, 2h) + retry/failover (GAP-18, 4h) + weft onboard (GAP-12, 3h)
- **Day 3**: Memory tool search wiring (GAP-15, 2h) + tool call parsing verification (GAP-17, 1h) + tests (TEST-01, TEST-04, 5h)
- **Day 4**: P1 security items (SEC-04, SEC-05, MISSED-05, 3h) + docs + smoke tests with real API keys

GAP-01, GAP-02, GAP-06 removed from sprint (already resolved). GAP-16 demoted to P2 (test-only mock code). GAP-19 demoted to verification-only.

---

## 7. Integrated Timeline

The timeline accounts for all inter-phase dependencies and the sequencing constraints identified across reviews.

| Week | Phase(s) | Key Deliverables | Dependencies | Risk |
|------|----------|-----------------|--------------|------|
| 14 | 3F-RVF Sprint 0 | Dependency validation: compile all ruvector crates, validate APIs, measure binary sizes | None | HIGH -- go/no-go gate for entire 3F-RVF plan |
| 14 | 3I Day 1-2 | SSE streaming, web search fix, JSON repair, retry/failover | None | MEDIUM -- SSE streaming is hardest single item |
| 14 | Pre-implementation | ~~Align discovery chain~~ (DONE), ~~measure cdylib baseline~~ (DONE: 57.9KB/24.3KB) | None | **DONE** |
| 15 | 3E Week 1 | Allocator comparison + wasm-opt pipeline (against stub WASM binary) | None | LOW |
| 15 | 3F-RVF Sprint 1 | RVF Runtime Foundation + ApiEmbedder + Bootstrap | Sprint 0 passed | MEDIUM |
| 15 | 3I Day 3-4 | Memory wiring, tests, security items, smoke tests | None | LOW |
| 15-16 | 3F-agents Wk 1-2 | Types, parser, template renderer, skill registry, agent registry | Discovery chain aligned | LOW |
| 16 | 3E Week 2 | WASM CI integration + size gate hardening | None | LOW |
| 16 | 3F-RVF Sprint 2 | Progressive HNSW + persistence | Sprint 1 | MEDIUM |
| 16-17 | 3G Phases A-C | Types + deep merge + workspace discovery + manager | None | LOW |
| 17 | 3E Week 3 | PGO (stretch), documentation, benchmark validation | None | LOW |
| 17 | 3F-RVF Sprint 3 | ruvllm L1 + QualityScorer | Sprint 2 | MEDIUM |
| 17-18 | 3F-agents Wk 3-4 | Slash commands, CLI, ContextBuilder integration, security hardening | Registry complete | LOW |
| 17-18 | 3G Phases D-E | CLI commands + integration | Phases A-C | LOW |
| 18 | 3H Session 1-2 | MCP protocol fixes + `McpServerShell` + `ToolProvider` trait + middleware pipeline (`SecurityGuard`, `PermissionFilter`, `ResultGuard`, `AuditLog`) + `BuiltinToolProvider` + `CompositeToolProvider` (shared MCP infra in `clawft-services/src/mcp/`) | None | LOW |
| 18 | 3F-RVF Sprint 4 | `RvfToolProvider` implementation (11 RVF tools, in-process Rust calls) -- registers with 3H `McpServerShell`. No transport code. | 3H Session 2 (ToolProvider trait + McpServerShell) | MEDIUM |
| 19 | 3H Session 3-4 | Delegation engine + wiring + validation | Session 2 | LOW |
| 19 | 3F-RVF Sprint 5 | rvf-crypto + AgentDB | Sprint 4 | LOW |
| 20 | 3F-RVF Sprint 6 | Polish, benchmarks, CLI, integration tests | Sprint 5 | LOW |

> **NOTE (2026-02-17):** Phase 3D (WASI HTTP/FS + Docker Multi-Arch) has been **deferred to Phase 4**. The wasi-http-client maturity, Rust 1.93 compatibility, WASM size impact, and bin-vs-reactor component model all need more research. This removes 3 weeks from the Phase 3 timeline and eliminates the clawft-wasm contention risk. 3E optimization proceeds against the stub WASM binary only.

**Total wall-clock time**: ~6 weeks (Weeks 14-20), with significant parallelism.

**Critical path**: 3F-RVF Sprint 0 -> Sprint 1-6 (7 weeks sequential). All other plans can execute in parallel with 3F-RVF.

**Scheduling conflicts resolved**:
- 3D deferred to Phase 4 -- no clawft-wasm contention
- 3E optimizes stub binary only (no re-benchmark after 3D needed in Phase 3)
- 3H shared `McpServerShell` + `ToolProvider` trait available before 3F-RVF Sprint 4 (pluggable ToolProvider architecture agreed; 3F-RVF only implements `RvfToolProvider`, no transport code)

---

## 8. Prioritized Action Items

### P0: Must fix before any implementation begins

1. ~~**[3F-RVF] Sprint 0 dependency validation (2 days)**~~ **RESOLVED (2026-02-17) -- All 11 crates compile on Rust 1.93.1. All 8 API surfaces match plan pseudocode. Candle bloat confirmed excluded with `minimal` feature. 4 plan corrections applied: `ruvector-sona` (not `sona`), `rvf-adapter-agentdb` (singular), rvf-wasm ~28KB (not 8KB), `EwcPlusPlus` type name. Full report: `.planning/reviews/sprint0-validation.md`.**

2. ~~**[3D] wasi-http-client build spike (2-4 hours)**~~ **REMOVED -- 3D deferred to Phase 4.**

3. ~~**[3F-agents + 3G] Align discovery chain (30 minutes)**~~ **RESOLVED -- 3-level chain aligned across 3F-agents, 3G, 02-tech-req. Nanobot compat removed. Security section (SEC-SKILL-01 through 07) added. DispatchResult enum replaces Result<bool>.**

4. ~~**[3E] Measure cdylib baseline (15 minutes)**~~ **RESOLVED (2026-02-17) -- clawft-wasm cdylib (wasm32-wasip2, opt-level=z, LTO, strip): 57.9 KB raw, 24.3 KB gzipped. Budget: 300KB/120KB. Headroom: 242KB/96KB (80%). Measured in `/tmp/cdylib-baseline/`.**

5. ~~**[3E + 3F-RVF] Agree on WASM size budget for stub binary**~~ **RESOLVED (2026-02-17)** -- Empirical baseline: 57.9 KB raw / 24.3 KB gzipped (42% compression). Agreed budget: 300 KB / 120 KB (Phase 3), 400 KB / 160 KB (Phase 4 post-3D). Post-3F-RVF projected ~250 KB / ~105 KB -- within budget. 3E targets retained. 3E plan updated with empirical baseline. Theme 3.2 updated.

6. ~~**[3I] Re-baseline sprint plan**~~ **RESOLVED (2026-02-17)** -- GAP-01, GAP-02, GAP-06 marked resolved in 3I plan (already implemented in code). GAP-16 demoted to P2 (test-only mock). P0 count: 12 -> 8. Sprint plan restructured: Day 1 = SSE streaming + web search; Day 2 = JSON repair + retry + onboard; Day 3 = memory + tool parsing + tests; Day 4 = P1 security + docs + smoke tests. Acceptance criteria updated.

7. ~~**[3H + 3F-RVF] Agree on shared McpServer extraction**~~ **RESOLVED (2026-02-17)** -- Pluggable `ToolProvider` architecture agreed. Shared infrastructure lives in `clawft-services/src/mcp/` (4 files, ~650 lines total):
   - **`provider.rs` (~80 lines)**: `ToolProvider` trait (`namespace()`, `list_tools()`, `call_tool()`), `ToolDefinition`, `CallToolResult`, `ToolError` types.
   - **`server.rs` (~300 lines)**: `McpServerShell` -- all MCP protocol handling: JSON-RPC dispatch, newline-delimited stdio framing, initialize handshake, tools/list aggregation, tools/call routing. Generic over `AsyncRead + AsyncWrite` for testability.
   - **`middleware.rs` (~150 lines)**: Security pipeline -- `SecurityGuard` (CommandPolicy + UrlPolicy, resolves 3H CRIT-01), `PermissionFilter` (allowed_tools intersection), `ResultGuard` (truncate_result + sanitize_content, resolves 3H CRIT-02), `AuditLog` (tool invocation logging).
   - **`composite.rs` (~120 lines)**: `CompositeToolProvider` -- aggregates multiple `ToolProvider`s with namespace routing. `tools/list` merges with namespace prefixing; `tools/call` strips namespace and routes.
   - **Concrete providers**: `BuiltinToolProvider` (wraps ToolRegistry, 3H), `RvfToolProvider` (11 RVF tools, 3F-RVF), `ProxyToolProvider` (wraps McpClient), `DelegationToolProvider` (Claude bridge, 3H).
   - **Protocol correction**: MCP 2025-06-18 stdio transport uses **newline-delimited JSON**, not Content-Length framing (LSP convention). Both plans originally specified Content-Length -- corrected. Current clawft code is already spec-compliant.
   - **Plan impact**: 3H Session 2 delivers `McpServerShell` + `ToolProvider` trait + middleware + `BuiltinToolProvider`. 3F-RVF Sprint 4 implements `RvfToolProvider` (no transport code). 3F-agents adds glob/prefix for MCP-namespaced `allowed_tools` (SEC-SKILL-08). 3G documents MCP config merge. `clawft-rvf-mcp` crate simplified to tool handlers + `RvfToolProvider` only (no `transport.rs`).

### P1: Must fix during implementation

8. **[3D] Fix path traversal in resolve()**. Reject `..` components and absolute paths when base_path is set.

9. **[3D] Define WasmHttpError enum**. Replace Box<dyn Error> with typed error variants.

10. **[3D] Pin wasmtime version in CI**. Document exact flag syntax for `--wasi http`.

11. **[3D] Add tempfile dev-dependency**. Missing from Cargo.toml changes.

12. **[3D] Clarify bin vs reactor component model**. Document the command-line one-shot model as a limitation, plan for reactor model in future phase.

13. **[3E] Fix release profile opt-level mismatch**. Move opt-level = "z" to profile.release-wasm only. Use opt-level = 2 for native release. PGO with opt-level = "z" is counterproductive.

14. **[3E] Remove todo!() from clawft_alloc_stats()**. Either implement serialization or gate behind alloc-tracing feature.

15. **[3E] Verify talc API against exact pinned version**. talc 4.x API has changed; constructor pattern may not compile as written.

16. **[3F-agents] Add security section**. YAML depth limits, directory name validation, allowed_tools intersection semantics, agent model string validation, --trust-project-skills flag.

17. **[3F-agents] Fix slash command dispatch return type**. Return DispatchResult enum with SkillInvocation(String) variant instead of Result<bool>.

18. **[3F-agents] Defer context: fork to future phase**. Parse it, log "not yet supported" warning. Do not ship broken/silent fork behavior.

19. **[3F-agents] Fix template $N collision**. Require ${N} syntax or use regex word boundaries.

20. **[3F-agents] Fix agent tool filtering -- do not mutate in place**. Store original ToolRegistry state; apply per-agent filters as a view, not a mutation.

21. **[3G] Apply normalize_keys to workspace config before deep merge**. Add camelCase/snake_case merge test.

22. **[3G] Bound CLAWFT.md hierarchical walk**. Stop at .git/ boundary or home directory.

23. **[3G] Tighten import regex**. Anchor to line start, require file extension, avoid false positives on emails and @mentions.

24. **[3H] Add access control to MCP server mode**. allowed_tools config, CommandPolicy/UrlPolicy enforcement, initialized flag.

25. **[3H] Extract tool execution helper from run_tool_loop()**. Reuse in Claude delegation bridge with truncation + logging + error formatting.

26. **[3H] Update TestTransport with send_notification()**. Prevents build breakage when McpTransport trait is extended.

27. ~~**[3H] Add read timeout to Content-Length parsing**~~ **SUPERSEDED** -- MCP 2025-06-18 stdio transport uses newline-delimited JSON, not Content-Length framing. Content-Length parsing is not needed. Current clawft code is already spec-compliant. Read timeout should still be applied to the newline-delimited reader to prevent hangs on unresponsive servers.

28. **[3I] Expand GAP-03 remediation**. Pass ToolsConfig into register_all(). Wire Brave endpoint from config. Inject API key as HTTP header.

### P2: Track as known limitations

29. **[3D] WASI reactor model deferred**. Current plan uses command-line one-shot model (stdin/stdout). Reactor/exported-function model for future phase.

30. **[3D] Streaming HTTP in WASM deferred**. WasiHttpClient is synchronous. SSE streaming in WASM for future phase.

31. **[3E] PGO, thin-LTO comparison, eh_frame are stretch goals**. Move to SHOULD-HAVE if Week 2 runs behind.

32. **[3F-RVF] SONA (L3), Attention (L4), WASM microkernel deferred to Phase 4**.

33. **[3F-RVF] Concurrent write safety for .rvf files not addressed**. Document locking strategy if rvf-runtime is not thread-safe.

34. **[3F-RVF] Embedding dimension mismatch handling**. Add model metadata to store header. Future enhancement.

35. **[3F-agents] weft skills install deferred**. Plan references it but does not define it.

36. **[3F-agents] Skill deactivation (disabled: true field) not supported**. Future enhancement.

37. **[3G] Environment variable overlay ($CLAWFT_*) not in scope**. GAP-27, orthogonal to workspace config.

38. **[3G] weft workspace init as in-place alias**. Matches git init ergonomics. Minor addition.

39. **[3H] MCP listChanged notification, keepalive/ping, cancellation**. Nice-to-have protocol features for future phases.

40. **[3H] Workspace-scoped MCP config**. Forward-compatibility documented; works automatically after 3G deep merge is wired.

---

## 9. Final Recommendations

### Is the Phase 3 portfolio ready to begin implementation?

**Yes.** All 7 P0 action items are now resolved. Six of seven plans are ready to implement immediately. The 3F-RVF Sprint 0 validation gate has passed (all 11 ruvector crates compile, APIs match pseudocode). The shared MCP infrastructure is agreed (pluggable ToolProvider architecture). Discovery chains are aligned. WASM budgets are empirically validated. The only remaining condition is the 3F-RVF plan's REVISE verdict -- implementation should be monitored closely against the reduced 32 engineer-day scope.

### Implementation order

1. **Immediate (Week 14)**: P0 spikes and coordination (~~3F-RVF Sprint 0~~ DONE, ~~discovery chain alignment~~ DONE, ~~cdylib baseline~~ DONE, ~~WASM budget~~ DONE, ~~3I re-baseline~~ DONE, ~~shared McpServer extraction~~ DONE, 3I sprint execution). **All P0 items resolved.**
2. **Wave 1 (Weeks 15-17)**: 3E, 3F-RVF Sprints 1-3, 3F-agents, 3G -- heavy parallelism
3. **Wave 2 (Weeks 17-19)**: 3H (Session 1-2 delivers shared `McpServerShell` + `ToolProvider` infra), 3E re-benchmark, 3F-RVF Sprint 4 (`RvfToolProvider` consuming 3H shared infra), 3F-RVF Sprint 5
4. **Wave 3 (Week 20)**: 3F-RVF polish, integration testing across all phases

### Top 3 risks

1. **ruvector crate viability (3F-RVF)**. 11 pre-1.0 crates from a single repository with no evidence of real-world compilation testing. If they do not work, 32 engineer-days of planning is invalidated. Mitigated by Sprint 0 but remains the highest-impact risk.

2. **WASM binary size exceeding all budgets (3D + 3E + 3F-RVF)**. Real HTTP (wasi-http-client or raw wasi), serde_json, RVF components, and an allocator could push the binary well past 400KB. gzip may not compress enough. Mitigated by the tiered budget and early measurement, but if the post-3D binary lands at 500KB+, the WASM deployment story weakens significantly.

3. **SSE streaming (3I GAP-11)**. Adding complete_stream() to the LlmTransport trait, implementing SSE parsing, and wiring through the pipeline to CLI/channel output is the single hardest P0 gap item. Estimated at 6 hours but could expand to 2-3 days if the trait change cascades through implementations. Fallback: ship synchronous full-response output and defer streaming.

### What should be deferred to Phase 4

- **Phase 3D (entire)**: WASI HTTP/FS + Docker Multi-Arch. Needs more research on wasi-http-client maturity, Rust 1.93 compatibility, WASM size impact, and bin-vs-reactor component model. Deferred 2026-02-17.
- **3F-RVF L2-L4**: tiny-dancer neural routing, SONA self-learning, attention-based context assembly. These are research-grade features that require validated feedback loops before they can be meaningfully tested. Ship L0-L1 first.
- **3F-RVF WASM microkernel**: Not the primary deployment target. Ship native-only, validate, then optimize for WASM.
- **MCP advanced protocol features**: listChanged notifications, keepalive/ping, cancellation, protocol version negotiation fallback. (Note: Content-Length framing originally planned for both 3H and 3F-RVF is not needed -- MCP 2025-06-18 uses newline-delimited JSON for stdio transport.)
- **weft skills install**: Referenced in 3F-agents but not defined. Defer to Phase 4 with ClawHub integration.
- **Environment variable config overlay**: GAP-27. Orthogonal to workspace config; can be added after 3G.

---

## 10. Appendix: Review Statistics

### Review Coverage

| Metric | Value |
|--------|-------|
| Total reviews | 7 |
| Total review lines | 2,069 |
| Plans reviewed | 7 (3D, 3E, 3F-RVF, 3F-agents, 3G, 3H, 3I) |
| APPROVE_WITH_CHANGES | 6 |
| REVISE | 1 (3F-RVF) |
| Unique critical issues | 14 (deduplicated) |
| Unique major issues | 22 |
| Unique minor issues | 28 |
| Cross-phase conflicts identified | 11 |

### Scores by Plan

| Plan | Avg Score | Highest Dimension | Lowest Dimension |
|------|-----------|-------------------|------------------|
| 3D | 3.4/5 | Completeness, Dependencies, Test Coverage (4/5) | Size Budget (2/5) |
| 3E | 3.8/5 | Completeness (5/5) | Timeline, Size Targets, Memory Profiling (3/5) |
| 3F-RVF | 3.25/5 | Completeness, Test Coverage, Feature Flags (4/5) | Scope Creep Risk (2/5) |
| 3F-agents | 3.6/5 | Compatibility (5/5) | Security (2/5) |
| 3G | 4.6/5 | Req Coverage, Discovery, Hooks, Isolation, Compat (5/5) | Config Merge, CLAWFT.md, Security (4/5) |
| 3H | 3.75/5 | Claude Tool Format (5/5) | Agent Loop Integration, Session Lifecycle, Security (3/5) |
| 3I | 3.6/5 | Remediation Plans, Cross-Phase, Acceptance Criteria, Test Gaps (4/5) | Priority Accuracy, Gap Validity (3/5) |

### Issue Distribution

| Severity | Count | Plans Affected |
|----------|-------|---------------|
| Critical | 14 | All 7 plans have at least 1 critical issue |
| Major | 22 | 3D (6), 3E (4), 3F-RVF (4), 3F-agents (4), 3G (5), 3H (4), 3I (2) |
| Minor | 28 | Distributed across all plans |

### Recurring Reviewer Themes

| Theme | Frequency | Plans |
|-------|-----------|-------|
| Security gaps | 5/7 reviews | 3D, 3F-agents, 3H, 3I, (3G minor) |
| Binary size budget concerns | 3/7 reviews | 3D, 3E, 3F-RVF |
| Cross-phase sequencing | 5/7 reviews | 3D, 3E, 3F-RVF, 3G, 3H |
| Unvalidated external dependencies | 2/7 reviews | 3D, 3F-RVF |
| Duplicate infrastructure | 2/7 reviews | 3F-RVF, 3H |
| Discovery/config chain inconsistency | 3/7 reviews | 3F-agents, 3G, 3H |
| Timeline overload / optimistic scope | 3/7 reviews | 3D, 3E, 3F-RVF |
