# Phase 3D-3I Execution Overview Plan

**Date**: 2026-02-17
**Scope**: Phases 3D (deferred), 3E, 3F-RVF, 3F-agents, 3G, 3H, 3I
**Hive Size**: 8-13 concurrent expert agents
**Topology**: Hierarchical mesh (queen coordinator + specialist workers)
**Timeline**: 6 weeks (Weeks 14-20), ~20-26 engineer-days active work
**Critical Path**: 3H Session 2 -> 3F-RVF Sprint 4 (MCP infra dependency)

---

## 0. Prerequisites

Before launching any wave, verify:

- [x] Rust 1.93 installed (`rustc 1.93.0 (254b59607 2026-01-19)`) -- plan said 1.93.1, actual is 1.93.0; acceptable
- [x] `cargo test --workspace` passes (1,058 tests, 0 failures, 8 ignored)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] All SPARC plans reviewed and consensus reached (`.planning/reviews/consensus.md`)
- [x] All P0 action items resolved (all resolved per consensus §8.1)
- [x] Sprint 0 validation passed -- CONDITIONAL PASS (`.planning/reviews/sprint0-validation.md`)

---

## 1. Execution Strategy

### 1.1 Wave-Based Parallel Execution

Work is organized into 5 waves. Each wave launches 8-13 agents concurrently.
Agents work on small, well-scoped tasks from a specific phase/session.
Every agent writes development notes to its stream directory before completing.

### 1.2 Agent Roles

| Role | Count | Responsibility |
|------|-------|----------------|
| Queen Coordinator | 1 | Orchestrates waves, resolves conflicts, gates transitions |
| Rust Coder | 4-6 | Implements features per SPARC plan pseudocode/architecture |
| Test Engineer | 2-3 | Writes tests first (TDD), validates exit criteria |
| Security Auditor | 1 | Reviews security-sensitive code (middleware, policies, paths) |
| Doc Writer | 1 | Updates dev notes, verifies doc consistency |
| Reviewer | 1-2 | Cross-checks code against plan, catches drift |

### 1.3 Development Guidelines (All Agents)

Every agent MUST:

1. **Read the SPARC plan section** before writing any code
2. **Read existing code** (`Read` tool) before editing
3. **Write tests first** (TDD London School -- mock external deps)
4. **Run `cargo test -p <crate>` after every file change**
5. **Run `cargo clippy -p <crate> -- -D warnings` before completing**
6. **Write development notes** to `/.planning/development_notes/phase3/stream-3?/<task>.md`
7. **Keep files under 500 lines** -- split if approaching limit
8. **Never hardcode secrets** -- use env vars or config
9. **Never commit to master** -- all work on feature branches

### 1.4 Development Notes Format

Each agent writes a note in its stream directory:

```markdown
# <Task Title>

**Agent**: <role>
**Stream**: 3?
**Wave**: N
**Status**: complete | blocked | partial

## What Was Done
- Bullet list of files created/modified

## Decisions Made
- Any design choices not in the SPARC plan

## Test Results
- `cargo test -p <crate>` output summary
- New tests added: N

## Issues Found
- Anything blocking or unexpected

## Next Steps
- What the next agent/wave should know
```

---

## 2. Phase 3D: WASI HTTP/FS + Docker Multi-Arch

**Status**: DEFERRED TO PHASE 4
**Reason**: wasi-http-client maturity, Rust 1.93 compat, WASM size impact, bin-vs-reactor model
**Action**: No agents assigned. P1 items (path traversal, error types, wasmtime pinning) tracked in consensus for Phase 4.

Dev notes directory: `.planning/development_notes/phase3/stream-3d/`
- Write a `deferred.md` noting the deferral decision and Phase 4 prerequisites.

---

## 3. Wave 1: Foundation (3I + 3E + 3H Session 1)

**Target**: Fix critical gaps, establish optimization infra, fix MCP client

### 3.1 Stream 3I: Gap Analysis Sprint (Days 1-4)

Reference: `.planning/sparc/3i-gap-analysis.md` Section 7

#### Agent 3I-A: SSE Streaming (GAP-11)
- **Crate**: `clawft-llm`, `clawft-core`
- **Task**: Implement `complete_stream()` on `OpenAiCompatProvider`. Add SSE parser, `StreamChunk` types, wire through pipeline to CLI/channel output.
- **LOE**: 6h
- **Tests**: Mock HTTP server returning SSE chunks; verify streaming callback invoked per chunk
- **Dev notes**: `stream-3i/sse-streaming.md`

#### Agent 3I-B: Web Search Fix + JSON Repair (GAP-03, GAP-14)
- **Crate**: `clawft-tools`, `clawft-types`, `clawft-core`
- **Task 1** (GAP-03, 1h): Fix web_search config field mismatch. Align `endpoint` vs `web_search_api_key`. Either construct Brave endpoint from key, or add `endpoint` field.
- **Task 2** (GAP-14, 2h): Implement JSON repair in `clawft-core`. Strip markdown fences, fix trailing commas, unquoted keys, truncated JSON.
- **Tests**: Unit tests for each repair case; integration test with malformed LLM JSON
- **Dev notes**: `stream-3i/web-search-json-repair.md`

#### Agent 3I-C: Retry/Failover + Onboard (GAP-18, GAP-12)
- **Crate**: `clawft-llm`, `clawft-cli`
- **Task 1** (GAP-18, 4h): Add exponential backoff retry + provider failover to `clawft-llm`. Try next provider on error.
- **Task 2** (GAP-12, 3h): Implement `weft onboard` command: create `~/.clawft/`, generate config template, prompt for API keys interactively.
- **Tests**: Mock provider that fails N times then succeeds; onboard creates expected directory structure
- **Dev notes**: `stream-3i/retry-onboard.md`

#### Agent 3I-D: Memory Wiring + Tool Parsing + Tests (GAP-15, GAP-17, GAP-19, TEST-01, TEST-04)
- **Crate**: `clawft-tools`, `clawft-core`, `clawft-llm`
- **Task 1** (GAP-15, 2h): Wire VectorStore-based search into `memory_read` tool.
- **Task 2** (GAP-17, 1h): Verify tool call parsing in `transport.rs`. Audit `ContentBlock::ToolUse` extraction.
- **Task 3** (GAP-19, 1h): Verify `MAX_TOOL_RESULT_BYTES` truncation enforced.
- **Task 4** (TEST-01, 3h): Add mock HTTP server tests for `OpenAiCompatProvider::complete()`.
- **Task 5** (TEST-04, 2h): Agent loop e2e test with mock LLM + tool calls.
- **Dev notes**: `stream-3i/memory-tools-tests.md`

#### Agent 3I-E: Security + Docs (SEC-04, SEC-05, MISSED-05, DOC-01, DOC-04)
- **Crate**: `clawft-llm`, `clawft-tools`, `clawft-core`
- **Task 1** (SEC-04, 1h): Audit all `debug!`/`info!` calls for API key leakage.
- **Task 2** (SEC-05, 1h): Add symlink path traversal test for file tools.
- **Task 3** (MISSED-05, 1h): Add web_fetch body size limit.
- **Task 4** (DOC-01, 1h): Create `docs/benchmarks/results.md` from existing data.
- **Task 5** (DOC-04, 0.5h): Fix routing.md readability reference.
- **Dev notes**: `stream-3i/security-docs.md`

### 3.2 Stream 3E: WASM Optimization (Weeks 1-2)

Reference: `.planning/sparc/3e-optimization.md` Sections 2-4

#### Agent 3E-A: Allocator Comparison + Feature Flags
- **Crate**: `clawft-wasm`
- **Task**: Implement feature-gated allocator selection (`alloc-talc`, `alloc-lol`, `alloc-tracing`). Follow pseudocode in 3E Section 2.1 exactly. Build comparison with all 3 allocators, record size deltas.
- **LOE**: 4h
- **Tests**: Build each allocator variant; verify size; run `capabilities()` with each
- **Dev notes**: `stream-3e/allocator-comparison.md`

#### Agent 3E-B: wasm-opt Pipeline + Twiggy Profiling + CI Gate
- **Crate**: Build scripts, CI workflows
- **Task 1**: Create `scripts/build/wasm-opt.sh` -- runs `wasm-opt -Oz` on WASM binary, validates with `wasmtime compile`.
- **Task 2**: Create `scripts/bench/wasm-twiggy.sh` -- runs `twiggy top/dominators/monos`, generates report.
- **Task 3**: Create `scripts/bench/wasm-size-gate.sh` -- checks 300KB/120KB limits.
- **Task 4**: Update WASM CI workflow to run wasm-opt + size gate.
- **LOE**: 4h
- **Tests**: Scripts run successfully against current WASM binary (57.9KB baseline)
- **Dev notes**: `stream-3e/wasm-opt-ci.md`

#### Agent 3E-C: Allocation Tracing + Startup Benchmark
- **Crate**: `clawft-wasm`
- **Task 1**: Implement `TracingAllocator` wrapper per 3E Section 2.2 pseudocode.
- **Task 2**: Create `scripts/bench/wasm-startup.sh` -- measures cold/warm instantiation time.
- **Task 3**: Verify feature exclusion -- no `channels`/`services`/`vector-memory` symbols in WASM binary.
- **LOE**: 3h
- **Tests**: alloc-tracing feature builds; stats_json() returns valid JSON; feature exclusion verified
- **Dev notes**: `stream-3e/tracing-startup.md`

### 3.3 Stream 3H: MCP Client Fixes (Session 1)

Reference: `.planning/sparc/3h-tool-delegation.md` Sections 3.4-3.6, 4.1-4.3

#### Agent 3H-A: MCP Initialize Handshake + Notifications + Content Extraction
- **Crate**: `clawft-services`
- **Task 1** (FR-001): Add `send_notification()` to `McpTransport` trait. Implement for Stdio/Http/Mock.
- **Task 2** (FR-001): Add `McpSession::connect()` with initialize handshake + `notifications/initialized`.
- **Task 3** (FR-004): Update `McpToolWrapper.execute()` to parse MCP content blocks (isError, content array). Per 3H Section 3.6.
- **LOE**: 4h
- **Tests**: Mock MCP server verifies handshake sequence; content extraction tests per Section 4.3
- **Dev notes**: `stream-3h/mcp-client-fixes.md`

---

## 4. Wave 2: Core Infrastructure (3H Session 2 + 3F-agents Wk 1-2 + 3G Phases A-C)

**Dependency**: Wave 1 complete (3I gaps fixed, 3H Session 1 done)

### 4.1 Stream 3H: Pluggable MCP Server (Session 2) -- CRITICAL PATH

Reference: `.planning/sparc/3h-tool-delegation.md` Sections 3.7, 4.4-4.8

#### Agent 3H-B: ToolProvider Trait + BuiltinToolProvider
- **Crate**: `clawft-services`
- **File**: `src/mcp/provider.rs` (~80 lines)
- **Task**: Implement `ToolProvider` trait (`namespace()`, `list_tools()`, `call_tool()`), `ToolDefinition`, `CallToolResult`, `ToolError` types. Implement `BuiltinToolProvider` wrapping existing `ToolRegistry`.
- **LOE**: 2h
- **Tests**: BuiltinToolProvider lists all tools; call_tool dispatches correctly; namespace returns "builtin"
- **Dev notes**: `stream-3h/tool-provider.md`

#### Agent 3H-C: CompositeToolProvider + McpServerShell
- **Crate**: `clawft-services`
- **Files**: `src/mcp/composite.rs` (~120 lines), `src/mcp/server.rs` (~300 lines)
- **Task 1**: Implement `CompositeToolProvider` with namespace prefixing and routing.
- **Task 2**: Implement `McpServerShell` -- newline-delimited JSON, initialize handshake, tools/list aggregation, tools/call routing. Generic over `AsyncBufRead + AsyncWrite`.
- **LOE**: 5h
- **Tests**: Two mock providers registered; tools/list merges both; tools/call routes by namespace; uninitialized requests rejected
- **Dev notes**: `stream-3h/composite-server.md`

#### Agent 3H-D: Middleware Pipeline
- **Crate**: `clawft-services`
- **File**: `src/mcp/middleware.rs` (~150 lines)
- **Task**: Implement `Middleware` trait + `SecurityGuard` (CommandPolicy + UrlPolicy), `PermissionFilter` (allowed_tools intersection), `ResultGuard` (truncation + sanitization), `AuditLog`.
- **LOE**: 3h
- **Tests**: SecurityGuard rejects disallowed commands; PermissionFilter strips unauthorized tools; ResultGuard truncates at 64KB; AuditLog records invocations
- **Dev notes**: `stream-3h/middleware.md`

#### Agent 3H-E: `weft mcp-server` CLI Command
- **Crate**: `clawft-cli`
- **File**: `src/commands/mcp_server.rs` (~150 lines)
- **Task**: Wire `McpServerShell` into a new `weft mcp-server` subcommand. Build BuiltinToolProvider + CompositeToolProvider + middleware from config; run on stdin/stdout.
- **LOE**: 2h
- **Tests**: Integration test: spawn `weft mcp-server`, send initialize + tools/list + tools/call via stdin, verify responses
- **Dev notes**: `stream-3h/mcp-server-cmd.md`

### 4.2 Stream 3F-agents: Skills + Agents (Weeks 1-2)

Reference: `.planning/sparc/3f-agents-skills.md` Sections 2-4

#### Agent 3Fa-A: Shared Types + SKILL.md Parser
- **Crate**: `clawft-types`, `clawft-core`
- **Task 1**: Add `SkillDefinition`, `SkillContext`, `SkillFormat` types to `clawft-types/src/skill.rs`.
- **Task 2**: Implement `parse_skill_md()` -- YAML frontmatter + markdown body extraction. Per Section 2.1 FR-3F-004.
- **Task 3**: Implement `SkillRegistry` with 3-level discovery chain. Per Section 2.1 FR-3F-005.
- **LOE**: 4h
- **Tests**: Parse SKILL.md with frontmatter; legacy skill.json fallback; 3-level priority merge
- **Dev notes**: `stream-3f-agents/types-parser.md`

#### Agent 3Fa-B: Agent Definitions + Template Renderer
- **Crate**: `clawft-core`
- **Task 1**: Implement `AgentDefinition` + `AgentLoader` + `AgentRegistry`. Per Section 2.2.
- **Task 2**: Implement template variable rendering (`$ARGUMENTS`, `${N}` syntax). Per Section 2.3.
- **Task 3**: Integrate `SkillRegistry` + `AgentDefinition` into `ContextBuilder`. Per Section 2.4.
- **LOE**: 4h
- **Tests**: Agent YAML/JSON loading; template substitution; ContextBuilder injects skill prompt
- **Dev notes**: `stream-3f-agents/agents-templates.md`

### 4.3 Stream 3G: Workspaces (Phases A-C)

Reference: `.planning/sparc/3g-workspaces.md` Sections 1.2-1.3

#### Agent 3G-A: Workspace Types + Deep Merge + Discovery
- **Crate**: `clawft-types`, `clawft-core`
- **Task 1**: Add workspace types (`WorkspaceRegistry`, `WorkspaceEntry`).
- **Task 2**: Implement JSON deep merge with normalize_keys (camelCase/snake_case). Per consensus P1 #21.
- **Task 3**: Implement workspace discovery algorithm (4-step: env var -> cwd walk -> registry -> global). Per Section 1.3.
- **Task 4**: Implement `WorkspaceManager::create`, `list`, `load`, `status`, `delete`.
- **LOE**: 5h
- **Tests**: Deep merge with conflicting keys; discovery finds `.clawft/` directory; create scaffolds all dirs; MCP config merge (Section 1.4.1)
- **Dev notes**: `stream-3g/workspace-core.md`

---

## 5. Wave 3: Advanced Features (3H Session 3-4 + 3F-agents Wk 3-4 + 3G Phases D-E + 3F-RVF Sprint 1-3)

**Dependency**: Wave 2 complete (3H Session 2 delivers ToolProvider/McpServerShell)

### 5.1 Stream 3H: Delegation Engine (Sessions 3-4)

Reference: `.planning/sparc/3h-tool-delegation.md` Sections 2.5-2.6, 3.1 (delegation crate)

#### Agent 3H-F: DelegationConfig Types + Engine
- **Crate**: `clawft-types`, `clawft-services`
- **Task 1**: Add `DelegationConfig`, `DelegationRule`, `DelegationTarget` types.
- **Task 2**: Implement `DelegationEngine` (regex-based rule matcher, fallback logic). Per Section 2.6.
- **Task 3**: Implement `openai_to_anthropic()` schema conversion. Per Section 2.5.
- **LOE**: 3h (feature-gated: `delegate`)
- **Tests**: Rule matching with regex patterns; fallback to local when target unavailable; schema conversion round-trip
- **Dev notes**: `stream-3h/delegation-engine.md`

#### Agent 3H-G: Claude Delegator + DelegateTaskTool + Wiring
- **Crate**: `clawft-services`, `clawft-tools`, `clawft-cli`
- **Task 1**: Implement `ClaudeDelegator` (Anthropic Messages API bridge with tool use loop). Per Section 2.5.
- **Task 2**: Implement `DelegateTaskTool` (`Tool` trait impl). Per Section 3.1.
- **Task 3**: Wire delegation into CLI startup -- `register_delegation()` in `mcp_tools.rs`.
- **LOE**: 4h (feature-gated: `delegate`)
- **Tests**: Mock HTTP test with Claude tool use round-trip; delegation disabled with missing API key
- **Dev notes**: `stream-3h/claude-delegator.md`

### 5.2 Stream 3F-agents: CLI + Security (Weeks 3-4)

Reference: `.planning/sparc/3f-agents-skills.md` Sections 5-6

#### Agent 3Fa-C: Slash Commands + Skills CLI
- **Crate**: `clawft-cli`
- **Task 1**: Implement `SlashCommandRegistry` + built-in slash commands. Per Section 5.1.
- **Task 2**: Implement `weft skills {list,show,run,install}` commands. Per Section 5.2.
- **Task 3**: Implement `weft agents {list,show,use}` commands. Per Section 5.3.
- **Task 4**: Implement `weft help <topic>` command. Per Section 5.4.
- **LOE**: 4h
- **Tests**: Slash command dispatch; skills list shows all sources; agent switch updates context
- **Dev notes**: `stream-3f-agents/cli-commands.md`

#### Agent 3Fa-D: Security Hardening (SEC-SKILL-01 through SEC-SKILL-08)
- **Crate**: `clawft-core`
- **Task**: Implement all skill/agent security controls:
  - SEC-SKILL-01: YAML depth limit (max 10 levels)
  - SEC-SKILL-02: Directory name validation (no `..`, no `/`)
  - SEC-SKILL-03: allowed_tools intersection semantics
  - SEC-SKILL-04: Agent model string validation
  - SEC-SKILL-05: `--trust-project-skills` flag
  - SEC-SKILL-06: Prompt injection guards
  - SEC-SKILL-07: File size limits for SKILL.md (50KB max)
  - SEC-SKILL-08: MCP tool namespace isolation (`{server}__{tool}` matching, glob support)
- **LOE**: 3h
- **Tests**: Each SEC-SKILL has a dedicated test; path traversal blocked; oversized SKILL.md rejected
- **Dev notes**: `stream-3f-agents/security-hardening.md`

### 5.3 Stream 3G: Workspace CLI + Integration (Phases D-E)

Reference: `.planning/sparc/3g-workspaces.md` Sections 1.2.4-1.2.11

#### Agent 3G-B: Workspace CLI + CLAWFT.md + Scoped Resources
- **Crate**: `clawft-cli`, `clawft-core`
- **Task 1**: Implement `weft workspace {create,list,load,status,delete,config}` commands.
- **Task 2**: Implement CLAWFT.md loading with `@path/to/file` import, hierarchical walk (bounded at `.git/` or home). Per FR-W07 + consensus P1 #22.
- **Task 3**: Wire workspace-scoped sessions, memory, skills into CLI commands.
- **LOE**: 4h
- **Tests**: workspace create scaffolds dirs; config deep-merge with workspace overrides; CLAWFT.md injection in system prompt; path traversal in imports blocked
- **Dev notes**: `stream-3g/workspace-cli.md`

### 5.4 Stream 3F-RVF: Core RVF Integration (Sprints 1-3)

Reference: `.planning/sparc/3f-rvf-integration.md` Sections 2-4, consensus Section 6

#### Agent 3Fr-A: RVF Runtime Foundation + ApiEmbedder (Sprint 1)
- **Crate**: `clawft-core`
- **Task**: Integrate rvf-runtime, rvf-types, rvf-index. Implement `ApiEmbedder` using `clawft-llm` provider embeddings. Wire MEMORY.md bootstrap for first-startup indexing.
- **LOE**: 6 engineer-days
- **Tests**: rvf-runtime opens/creates store; ApiEmbedder returns embeddings; bootstrap indexes MEMORY.md
- **Dev notes**: `stream-3f-rvf/sprint1-foundation.md`

#### Agent 3Fr-B: Progressive HNSW + Persistence (Sprint 2)
- **Crate**: `clawft-core`
- **Task**: Replace brute-force VectorStore with 3-tier Progressive HNSW (Layer A/B/C). Add session/router persistence via `.rvf` files.
- **LOE**: 5 engineer-days
- **Tests**: HNSW search returns top-k correctly; persistence round-trip; degradation when cold
- **Dev notes**: `stream-3f-rvf/sprint2-hnsw.md`

#### Agent 3Fr-C: ruvllm L1 + QualityScorer (Sprint 3)
- **Crate**: `clawft-core`
- **Task**: Integrate ruvllm Level 1 (TaskComplexityAnalyzer). Replace `NoopScorer` with `QualityScoringEngine`.
- **LOE**: 4 engineer-days
- **Tests**: Complexity analysis produces scores; scorer records quality metrics; integration with pipeline
- **Dev notes**: `stream-3f-rvf/sprint3-ruvllm.md`

---

## 6. Wave 4: Integration (3F-RVF Sprint 4-6)

**Dependency**: Wave 3 complete (3H Session 2 ToolProvider available, 3F-RVF Sprint 3 done)

### 6.1 Stream 3F-RVF: MCP + Crypto + Polish (Sprints 4-6)

#### Agent 3Fr-D: RvfToolProvider + MCP Registration (Sprint 4)
- **Crate**: New `clawft-rvf-mcp`
- **Task**: Implement `RvfToolProvider` with 11 tools (see 3F-RVF Section Sprint 6 task list). Register with `McpServerShell` via `CompositeToolProvider`. No transport code.
- **Gates on**: 3H Session 2 (ToolProvider trait + McpServerShell)
- **LOE**: 5 engineer-days
- **Tests**: ToolProvider trait conformance; each of 11 tool handlers; error handling; registration with shell; >= 18 unit + 5 integration tests
- **Dev notes**: `stream-3f-rvf/sprint4-toolprovider.md`

#### Agent 3Fr-E: rvf-crypto + AgentDB (Sprint 5)
- **Crate**: `clawft-core`, `clawft-rvf-mcp`
- **Task**: Integrate rvf-crypto (WITNESS segments, Ed25519 chain). Connect AgentDB adapter (`rvf-adapter-agentdb`).
- **LOE**: 4 engineer-days
- **Tests**: WITNESS attestation round-trip; AgentDB store/query; crypto feature-gated
- **Dev notes**: `stream-3f-rvf/sprint5-crypto-agentdb.md`

#### Agent 3Fr-F: Polish + Benchmarks + Integration Tests (Sprint 6)
- **Crate**: All RVF-touched crates
- **Task**: End-to-end level integration tests (L0-L4). CLI RVF commands. Benchmark native vs WASM search. Verify WASM size under budget.
- **LOE**: 5 engineer-days
- **Tests**: All 5 levels tested end-to-end; WASM binary <= 300KB; level downgrade graceful
- **Dev notes**: `stream-3f-rvf/sprint6-polish.md`

---

## 7. Wave 5: Review + Validation

**Dependency**: All implementation waves complete

### 7.1 Cross-Stream Integration Testing

#### Agent Review-A: Full Test Suite + Clippy
- **Task**: Run `cargo test --workspace` (all features). Run `cargo clippy --workspace -- -D warnings`. Run `cargo fmt --all -- --check`. Report any failures.
- **Dev notes**: `stream-3i/final-test-suite.md`

#### Agent Review-B: WASM Build Validation
- **Task**: Build clawft-wasm for wasm32-wasip2. Run wasm-opt. Verify size <= 300KB / 120KB gzipped. Run twiggy top-20 report. Verify no banned features in binary.
- **Dev notes**: `stream-3e/final-wasm-validation.md`

#### Agent Review-C: MCP Integration Validation
- **Task**: Spawn `weft mcp-server`, connect test client, verify initialize handshake + tools/list + tools/call. Test with BuiltinToolProvider. If RvfToolProvider is available, test that too.
- **Dev notes**: `stream-3h/final-mcp-validation.md`

#### Agent Review-D: Security Review
- **Task**: Run security review across all new code. Check:
  - CommandPolicy/UrlPolicy enforced via SecurityGuard middleware
  - Path traversal blocked in file tools and CLAWFT.md imports
  - API keys never logged
  - Tool result truncation enforced
  - MCP namespace isolation (SEC-SKILL-08)
- **Dev notes**: `stream-3i/final-security-review.md`

### 7.2 Documentation Consistency Review

#### Agent Review-E: Doc Consistency
- **Task**: Verify all clawft/docs/* are consistent with implemented code:
  - ToolProvider trait signature matches actual code (fix overview.md vs contributing.md mismatch)
  - Config examples use JSON (not TOML)
  - `tools.mcp_servers` config key used consistently
  - MCP server/client mode distinction clear
  - Cross-references resolve (no broken links)
- **Dev notes**: `stream-3i/final-doc-review.md`

#### Agent Review-F: SPARC Plan Reconciliation
- **Task**: Verify consensus.md reflects actual implementation state:
  - All P0 items verified resolved
  - Sprint numbering consistent (consensus Sprint 4 = plan Sprint 6 for RvfToolProvider -- reconcile)
  - Middleware trait method names consistent (architecture vs pseudocode)
  - Timeline accurate
- **Dev notes**: `stream-3i/plan-reconciliation.md`

---

## 8. Dependency Graph

```
Wave 1 (parallel):
  3I-A: SSE streaming ──────────────────────────────────────┐
  3I-B: Web search + JSON repair ───────────────────────────┤
  3I-C: Retry/failover + onboard ───────────────────────────┤
  3I-D: Memory wiring + tests ──────────────────────────────┤
  3I-E: Security + docs ────────────────────────────────────┤
  3E-A: Allocator comparison ───────────────────────────────┤
  3E-B: wasm-opt + CI gate ────────────────────────────────┤
  3E-C: Tracing + startup ─────────────────────────────────┤
  3H-A: MCP client fixes (Session 1) ──────────────────────┘
                                                            │
                                                            v
Wave 2 (parallel, after Wave 1):
  3H-B: ToolProvider + Builtin ─────────────────────┐
  3H-C: Composite + McpServerShell (CRITICAL) ──────┤──── Gates Wave 4 (3F-RVF Sprint 4)
  3H-D: Middleware pipeline ────────────────────────┤
  3H-E: `weft mcp-server` cmd ─────────────────────┤
  3Fa-A: Types + SKILL.md parser ───────────────────┤
  3Fa-B: Agents + templates ────────────────────────┤
  3G-A: Workspace types + discovery ────────────────┘
                                                            │
                                                            v
Wave 3 (parallel, after Wave 2):
  3H-F: Delegation engine ──────────────────────────┐
  3H-G: Claude delegator + wiring ──────────────────┤
  3Fa-C: Slash commands + skills CLI ───────────────┤
  3Fa-D: Security hardening ────────────────────────┤
  3G-B: Workspace CLI + CLAWFT.md ──────────────────┤
  3Fr-A: RVF Foundation (Sprint 1) ─────────────────┤
  3Fr-B: HNSW + Persistence (Sprint 2) ────────────┤
  3Fr-C: ruvllm + Scorer (Sprint 3) ────────────────┘
                                                            │
                                                            v
Wave 4 (after Wave 3 + 3H Session 2):
  3Fr-D: RvfToolProvider (Sprint 4) ────────────────┐
  3Fr-E: rvf-crypto + AgentDB (Sprint 5) ───────────┤
  3Fr-F: Polish + benchmarks (Sprint 6) ────────────┘
                                                            │
                                                            v
Wave 5 (after all implementation):
  Review-A: Full test suite ────────────────────────┐
  Review-B: WASM validation ────────────────────────┤
  Review-C: MCP integration ────────────────────────┤
  Review-D: Security review ────────────────────────┤
  Review-E: Doc consistency ────────────────────────┤
  Review-F: Plan reconciliation ────────────────────┘
```

---

## 9. Agent Assignment Quick Reference

When an agent is spawned, it should receive:

1. **This file** as context (the overview plan)
2. **The specific SPARC plan** for its stream (e.g., `.planning/sparc/3h-tool-delegation.md`)
3. **The section reference** (e.g., "Section 3.7, Tasks 7a-7e")
4. **The consensus document** for cross-cutting decisions (`.planning/reviews/consensus.md`)

### Agent Spawn Template

```
You are Agent [ID] working on Stream [3?], Wave [N].

Your task: [description from this plan]

MUST READ before coding:
- .planning/sparc/[plan].md (sections [X-Y])
- .planning/reviews/consensus.md (for cross-cutting decisions)
- Existing source files you will modify (use Read tool)

MUST DO:
1. Write tests FIRST (TDD)
2. Run `cargo test -p <crate>` after each change
3. Run `cargo clippy -p <crate> -- -D warnings` before completing
4. Write development notes to .planning/development_notes/phase3/stream-3?/[task].md

MUST NOT:
- Commit to master
- Save files to root directory
- Skip reading existing code before editing
- Hardcode any secrets
```

---

## 10. Exit Criteria

Phase 3D-3I is complete when:

- [ ] All 8 P0 gaps from 3I resolved (GAP-03, GAP-11, GAP-12, GAP-14, GAP-15, GAP-17, GAP-18, GAP-19)
- [ ] `cargo test --workspace` passes with >= 1,150 tests (up from 1,054)
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] WASM binary <= 300 KB uncompressed, <= 120 KB gzipped (post wasm-opt)
- [ ] `weft mcp-server` command functional (initialize, tools/list, tools/call)
- [ ] ToolProvider trait implemented with BuiltinToolProvider + middleware pipeline
- [ ] Skill system supports SKILL.md + skill.json dual format
- [ ] Agent definitions loadable from YAML/JSON
- [ ] Workspace create/list/load/status/delete functional
- [ ] CLAWFT.md loading with import resolution
- [ ] Delegation engine + Claude bridge functional (feature-gated)
- [ ] RvfToolProvider registers with McpServerShell (if 3F-RVF Sprints 1-4 complete)
- [ ] All development notes written to `.planning/development_notes/phase3/stream-3?/`
- [ ] Wave 5 review agents confirm no regressions
- [ ] User docs in docs/ have been updated with all new functionality and examples of relevant guides/references/etc have been added.
---

## 11. Known Issues to Resolve During Implementation

From Wave 3 reviews:

1. **ToolProvider trait signature**: `overview.md` shows `async fn list_tools() -> Vec<ToolDefinition>` + `Result<CallToolResult, ToolError>`, while `contributing.md` shows `fn list_tools() -> Vec<ToolSchema>` + `Result<Value, ToolError>`. The actual Rust code (written in 3H Session 2) is authoritative. Update docs to match.

2. **Middleware trait method names**: 3H pseudocode (Section 2.4) uses `on_list_tools`/`on_call_tool`/`on_tool_result`, while architecture (Section 3.7) uses `filter_tools`/`before_call`/`after_call`. Use the Section 3.7 names (they are better API design).

3. **Sprint numbering**: Consensus shows RvfToolProvider as Sprint 4 (reduced plan), but 3F-RVF plan shows Sprint 6 (original plan). Implementation should follow the consensus reduced plan. The agent for 3Fr-D should treat this as Sprint 4 per consensus Section 6.
