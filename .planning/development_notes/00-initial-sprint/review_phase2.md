# Phase 2 Completion Review

**Date**: 2026-02-17
**Reviewers**: 4 expert agents (code-vs-plan, business-requirements, integration-wiring, test-quality)
**Scope**: Full audit of Phase 1 + Phase 2 deliverables against SPARC plans, business requirements, and technical requirements

## Executive Summary

Phase 1 and Phase 2 produced **~18,000 lines of Rust** across 9 crates with **777 unit tests**, zero clippy warnings, and zero trivial assertions. **44 of 64 planned deliverables are fully DONE** (69%), with 5 partial, 1 stub, 2 Level-0 no-ops, and 12 missing.

**The critical finding**: all components are well-built and tested in isolation, but the CLI commands contain placeholder stubs. The binary compiles and individual crates pass their tests, but `weft agent -m "hello"` prints a hardcoded placeholder string. ~600-800 lines of glue code would make the binary functional end-to-end.

## Codebase Statistics

| Metric | Value |
|---|---|
| Total Rust source files | ~55 |
| Total lines of Rust | ~18,000 |
| Unit tests | 777 |
| Doc tests | 3 |
| Integration test files | 0 |
| Security test files | 0 |
| Clippy warnings | 0 |
| Crates in workspace | 9 |

### Test Distribution

| Crate | Unit Tests | Quality |
|---|---|---|
| clawft-types | 55 | Strong |
| clawft-platform | 45 | Adequate |
| clawft-core | 263 (with vector-memory) | Strong |
| clawft-channels | 152 | Strong |
| clawft-llm | 56 | Adequate |
| clawft-tools | 44 | Strong |
| clawft-services | 59 | Adequate |
| clawft-cli | 103 | Adequate |
| clawft-wasm | 0 | N/A (Phase 3) |

## Gaps Identified

### P0 -- Blocks MVP Functionality

#### GAP-1: clawft-llm not bridged to pipeline
- `clawft_llm::Provider` and `clawft_core::pipeline::transport::LlmProvider` are separate trait systems
- No adapter exists to bridge them
- `bootstrap.rs:234` creates a stub transport that returns "transport not configured" error
- **Impact**: Every pipeline invocation fails. Binary cannot answer questions.

#### GAP-2: Agent CLI prints placeholder
- `commands/agent.rs:95` -- `run_single_message()` prints `"[Agent loop not yet wired]"`
- `commands/agent.rs:141-142` -- `run_interactive()` prints `"[Agent response placeholder]"`
- Tools are registered (line 74) but the ToolRegistry is dropped without being passed to an AgentLoop
- **Impact**: `weft agent -m "hello"` is non-functional

#### GAP-3: Gateway has no AgentLoop or outbound dispatch
- `commands/gateway.rs` creates a MessageBus and starts channels
- No `AgentLoop` consumes from the bus
- No outbound consumer routes responses back through channels
- **Impact**: Gateway receives messages but never responds

#### GAP-4: Security APIs missing
- Plan 1B section 4.8 requires `validate_session_id`, `truncate_result`, `sanitize_content`
- None of these functions exist
- Session key sanitization only replaces colons with underscores (no path traversal protection)
- Tool results passed directly to LLM without size checking
- **Impact**: Security gate cannot pass

### P1 -- Required for Phase 2 Completion

#### GAP-5: Slack/Discord not registered in gateway
- `gateway.rs:69` has comment: `"// Future channels would be registered here:"`
- Both channel implementations are complete with 96 tests
- **Fix**: Mirror the Telegram registration pattern (lines 55-66)

#### GAP-6: Missing tools (web_search, web_fetch, message, spawn)
- Only 7 of ~11 planned tools exist
- `web_search`, `web_fetch` were scheduled for Phase 1 Week 6
- `message` (bus integration) and `spawn` (subagent) also missing

#### GAP-7: Zero integration tests
- Orchestrator plan requires `tests/phase1_integration.rs` and `tests/phase2_integration.rs`
- Only `tests/fixtures/config.json` exists
- Missing fixtures: `session.jsonl`, `MEMORY.md`, `tools/*.json`

#### GAP-8: Zero security tests
- Plan requires `clawft-core/tests/security_tests.rs` with 5 specific tests
- Tests require the security APIs from GAP-4

#### GAP-9: Markdown converters not wired
- `MarkdownConverter` trait at `clawft-cli/src/markdown/mod.rs:18` says "Not yet wired"
- All 3 converters (Telegram, Slack, Discord) implemented and tested
- No outbound dispatch loop exists to insert them

#### GAP-10: Services not started
- CronService: fully implemented but CLI cron commands do raw file I/O, bypassing the service
- HeartbeatService: fully implemented, never instantiated
- McpClient: fully implemented with list_tools/call_tool, never used for tool discovery

#### GAP-11: Vector-memory not enabled in CLI
- Feature exists behind `vector-memory` flag with 63 tests
- Not enabled in clawft-cli's Cargo.toml dependency on clawft-core
- IntelligentRouter and SessionIndexer not wired into pipeline or agent loop

### P2 -- Missing CLI Commands & Polish

#### GAP-12: Missing CLI subcommands
- `weft sessions` -- list, inspect, delete sessions (not implemented)
- `weft memory` -- search, display memory (not implemented)
- `weft config` -- show resolved configuration (not implemented)
- `weft onboard` -- initial setup wizard (not implemented)

#### GAP-13: Shell completions not implemented
- Plan 2D requires bash/zsh/fish completions
- `clap_complete` not added as dependency

#### GAP-14: Codex OAuth flow missing
- Plan 2C lists this as deliverable
- Provider registry has `is_oauth: true` for openai_codex
- No OAuth implementation (device flow, token exchange, refresh)

#### GAP-15: SSE streaming not implemented
- `clawft-llm` ChatRequest has `stream: Option<bool>`
- No SSE response parsing exists

## Checklist Accuracy Issues

Three items were marked `[x]` in planning docs but are not fully accurate:

| Checkbox | Issue |
|---|---|
| `[x] Telegram, Slack, Discord channels all work as plugins` | Slack/Discord not registered in gateway command |
| `[x] RVF vector memory search returns relevant context` | Feature-gated, not wired into agent loop |
| `[x] Clippy clean, no warnings` | True when checked, but no CI enforces it continuously |

## Component Wiring Matrix

| Component | Built | Tested | Wired | Gap |
|---|---|---|---|---|
| MessageBus | Yes | Yes | Partial | Gateway has no consumer |
| AgentLoop | Yes | Yes | No | Not used by any CLI command |
| AppContext/Bootstrap | Yes | Yes | No | CLI builds components manually |
| PipelineRegistry | Yes | Yes | Stub | Transport always returns error |
| OpenAiCompatTransport | Yes | Yes | Stub | No LlmProvider injected |
| clawft-llm Provider | Yes | Yes | No | Never imported outside its crate |
| TelegramChannel | Yes | Yes | Yes | Works in gateway |
| SlackChannel | Yes | Yes | No | Not registered in gateway |
| DiscordChannel | Yes | Yes | No | Not registered in gateway |
| CronService | Yes | Yes | No | CLI bypasses it |
| HeartbeatService | Yes | Yes | No | Never instantiated |
| McpClient | Yes | Yes | No | Never used for discovery |
| IntelligentRouter | Yes | Yes | No | Feature not enabled |
| SessionIndexer | Yes | Yes | No | Feature not enabled |
| VectorStore | Yes | Yes | No | Transitively unwired |
| MarkdownConverters | Yes | Yes | No | Annotated "not yet wired" |
| ToolRegistry | Yes | Yes | Partial | Tools registered then dropped |
| SessionManager | Yes | Yes | Partial | Created in agent cmd, unused |
| MemoryStore | Yes | Yes | Yes | Wired via ContextBuilder |
| SkillsLoader | Yes | Yes | Yes | Wired via ContextBuilder |

## Recommended Gap-Filling Streams

### Stream 2E: Integration Wiring (P0)
- LLM adapter bridge (clawft-llm → pipeline transport)
- Wire AppContext + AgentLoop into `weft agent`
- Wire AgentLoop + outbound dispatch into `weft gateway`
- Register Slack/Discord factories in gateway
- Wire markdown converters into outbound path
- Estimated: 400-700 lines

### Stream 2F: Security & Testing (P0-P1)
- Implement security APIs (validate_session_id, truncate_result, sanitize_content)
- Write security_tests.rs (5 planned tests)
- Write phase1_integration.rs (4 cross-crate E2E tests)
- Write phase2_integration.rs (5 cross-crate E2E tests)
- Add missing test fixtures
- Estimated: 600-900 lines

### Stream 2G: Service Wiring (P1)
- Start CronService in gateway with bus sender
- Start HeartbeatService in gateway
- Wire McpClient → ToolRegistry
- Enable vector-memory feature in CLI
- Wire IntelligentRouter into pipeline
- Estimated: 300-500 lines

### Stream 2H: CLI Gaps (P2)
- `weft sessions` subcommand
- `weft memory` subcommand
- `weft config` subcommand
- Shell completion generation (clap_complete)
- web_search + web_fetch tools
- message + spawn tools
- Estimated: 800-1200 lines

## Conclusion

The foundation is solid. The crate architecture, trait design, and test quality are all strong. The gap is almost entirely **integration glue** — connecting pieces that were built in parallel but never wired together. Streams 2E (wiring) and 2F (security/testing) are prerequisites before declaring Phase 2 complete. Streams 2G and 2H can be done in parallel with early Phase 3 work.




### COMPLETE - All streams finished

**Stream 2E (Integration Wiring)**: COMPLETE
**Stream 2F (Security & Testing)**: COMPLETE
**Stream 2G (Gateway & MCP)**: COMPLETE
**Stream 2H (CLI Gaps)**: COMPLETE

#### Stream 2H Agent Results (8 agents, all successful)

| Agent | Task | Status |
|-------|------|--------|
| a92cdc7 | `weft sessions` CLI subcommand | Done - sessions.rs, 15 tests |
| aa815ea | `web_search` tool | Done - web_search.rs, 10 tests |
| a25158c | `web_fetch` tool | Done - web_fetch.rs, 6 tests |
| a15fa0c | `message` tool | Done - message_tool.rs, 8 tests |
| aea8bf5 | `spawn` tool | Done - spawn_tool.rs, 8 tests |
| a61cf42 | MCP tools wiring | Done - mcp_tools.rs + agent.rs/gateway.rs |
| abfe3b2 | `weft memory` CLI subcommand | Done - memory_cmd.rs, 8 tests |
| ac8c1a6 | `weft config` + completions | Done - config_cmd.rs, completions.rs, 10 tests |

#### Integration Wiring (Queen coordinator)

All shared files updated to wire agent outputs together:

- `main.rs`: Added `mod completions`, SessionsCmd/MemoryCmd/ConfigCmd enums, Commands variants, match arms
- `commands/mod.rs`: Added module declarations for sessions, memory_cmd, config_cmd
- `clawft-tools/src/lib.rs`: Added module declarations + tool registration for web_search, web_fetch, spawn_tool
- `agent.rs` + `gateway.rs`: Registered message_tool (needs bus ref, separate from register_all)

#### Verification

- `cargo check --workspace`: Clean (0 warnings)
- `cargo clippy --workspace -- -D warnings`: Clean (0 warnings)
- `cargo test --workspace`: 892 tests pass, 0 failures