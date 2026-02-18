# Expert Review: Phase 3I -- Gap Analysis

## Reviewer: QA Architect
## Date: 2026-02-17
## Verdict: APPROVE_WITH_CHANGES

The gap analysis is thorough and structurally sound. It covers the requirement space methodically and the 37 gaps are almost all legitimate. However, it contains several factual errors against the current codebase that inflate the P0 count and distort the sprint plan. Three gaps are already resolved in code, one gap misquotes the trait definition, and the web search gap is underspecified. After corrections, the P0 count drops from 12 to 8, making the 4-day sprint more realistic.

---

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| Coverage | 4 | Strong overall. Caught most significant gaps. Missed 5 items (see below). |
| Priority Accuracy | 3 | Three P0s are factually wrong (already fixed in code). Net P0 count is 8, not 12. |
| Gap Validity | 3 | 34 of 37 gaps are real. GAP-01/02 as described are invalid. GAP-06 is partially invalid. GAP-11 misquotes the trait. |
| Remediation Plans | 4 | Most remediation plans are concrete and actionable with reasonable LOE estimates. |
| Cross-Phase Coverage | 4 | P2 deferrals are well-reasoned. All gaps map to SPARC phases or explicit Phase 4 deferrals. |
| Sprint Plan | 3 | Day 1 is inflated due to already-resolved items. Days 2-3 are realistic. Day 4 is achievable. |
| Acceptance Criteria | 4 | Gate criteria are comprehensive. Two criteria need rewording (see below). |
| Test Gaps | 4 | The 8 test gaps are correctly identified. TEST-01 and TEST-04 as P0 are justified. |

**Weighted Average: 3.6 / 5**

---

## Strengths

1. **Systematic requirements tracing**. The audit walks through all 5 planning documents (01-05) and maps every specified feature to implementation status. This is exactly the right approach for a phase gate review.

2. **Honest deferred scope**. The RVF/ruvector analysis (sections 1.4, 1.5) correctly identifies that all integration is NOT STARTED but correctly notes that Level 0 fallback implementations are present and functional. The decision to defer to Phase 4 is well-justified with documented acceptance.

3. **Security gap identification**. SEC-01 through SEC-07 are well-scoped. The identification of API key logging risk (SEC-04), symlink traversal (SEC-05), and JSONL injection (SEC-06) shows mature security thinking.

4. **Crate-by-crate appendix**. Appendix B gives a quick status dashboard that is useful for sprint planning. The test counts match the codebase map cross-reference.

5. **Config completeness audit** (section 6.3). Flagging `failover_strategy` as parsed-but-unused and `mcp_servers` as not-acted-upon is the right kind of detail for a gap analysis. The former is real; the latter is now outdated (see Issues below).

---

## Issues Found

### Critical

**ISSUE-01: GAP-01 and GAP-02 are factually incorrect -- LLM bootstrap is already fixed.**

The gap analysis states:

> GAP-01: `enable_live_llm()` exists but is never called from CLI unless `--intelligent-routing` is passed.
> GAP-02: `AppContext::new()` always creates a stub transport. The `enable_live_llm()` method is only called when `--intelligent-routing` flag is passed.

This is **wrong**. The current code in both CLI commands calls `enable_live_llm()` unconditionally:

- `/home/aepod/dev/clawft/crates/clawft-cli/src/commands/agent.rs`, line 112:
  ```rust
  ctx.enable_live_llm();
  ```
- `/home/aepod/dev/clawft/crates/clawft-cli/src/commands/gateway.rs`, line 147:
  ```rust
  ctx.enable_live_llm();
  ```

The `--intelligent-routing` flag only controls the `IntelligentRouter` (vector-memory feature), NOT the live LLM pipeline. The `build_live_pipeline()` function in `bootstrap.rs` delegates to `llm_adapter::build_live_pipeline(config)` which wires a real `ClawftLlmAdapter` backed by `clawft-llm`.

**Impact**: Inflates P0 count by 2. Day 1 sprint plan allocates 2 hours to a non-existent problem.

**Recommendation**: Remove GAP-01 and GAP-02 from P0. Downgrade to P1 validation item: "verify `weft agent -m 'hello'` works end-to-end with a configured provider" (which is TEST-04 already).

---

**ISSUE-02: GAP-06 (MCP integration) is already implemented in clawft-cli.**

The gap analysis states:

> GAP-06: MCP integration incomplete: McpClient and transports exist in clawft-services but are NOT wired into the agent loop. Config has `mcp_servers` field but bootstrap ignores it.

This is **wrong**. The file `/home/aepod/dev/clawft/crates/clawft-cli/src/mcp_tools.rs` contains a fully functional MCP tool bridge:

1. `register_mcp_tools()` iterates `config.tools.mcp_servers` (a `HashMap<String, MCPServerConfig>`)
2. For each server, it calls `create_mcp_client()` which supports both stdio and HTTP transports
3. It calls `client.list_tools()` to discover tools from the MCP server
4. It wraps each discovered tool as `McpToolWrapper` implementing the `Tool` trait
5. It registers each wrapper in the `ToolRegistry`

Both `agent.rs` (line 99) and `gateway.rs` (line 136) call `register_mcp_tools()`:
```rust
crate::mcp_tools::register_mcp_tools(&config, ctx.tools_mut()).await;
```

The MCP integration tests include: namespacing (`server__tool`), delegation, error propagation, transport selection (command vs URL), and object safety.

**Impact**: Inflates P0 count by 1. Day 3 sprint plan allocates 4 hours to an already-complete feature.

**Recommendation**: Remove GAP-06 from P0. Downgrade to P1: "end-to-end validation with a real MCP server (e.g., `npx -y @modelcontextprotocol/server-filesystem`)".

---

**ISSUE-03: GAP-11 misquotes the trait definition.**

The gap analysis states:

> GAP-11: `LlmTransport::complete_stream()` is defined in the trait but the `OpenAiCompatProvider` does NOT implement streaming.

The actual `LlmTransport` trait in `/home/aepod/dev/clawft/crates/clawft-core/src/pipeline/traits.rs` (line 233-236) contains only:

```rust
pub trait LlmTransport: Send + Sync {
    async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse>;
}
```

There is **no** `complete_stream()` method defined anywhere in the codebase. A grep for `complete_stream` across all crates returns zero results.

**Impact**: The gap is still real -- SSE streaming IS missing and IS important for UX. But the description is misleading about the current state. The issue is that streaming was never designed into the trait, not that the trait is unimplemented.

**Recommendation**: Rephrase GAP-11 to: "SSE streaming not implemented. The `LlmTransport` trait has no streaming method. Need to add `complete_stream()` to the trait, implement SSE parsing, and wire through the pipeline to CLI/channel output." Keep as P0 but note the LOE might be higher (adding a new trait method requires updating all implementations).

### Major

**ISSUE-04: GAP-03 underspecifies the web search config problem.**

The gap analysis says there's a "mismatch between config schema and tool implementation." The actual problem is worse: `register_all()` in `/home/aepod/dev/clawft/crates/clawft-tools/src/lib.rs` (line 91-93) creates the web search tool with `endpoint: None` unconditionally:

```rust
registry.register(Arc::new(web_search::WebSearchTool::new(
    platform.clone(),
    None,  // <-- endpoint is always None
)));
```

The config has `tools.web.search.api_key` and `tools.web.search.max_results`, but `register_all()` never reads these values. The `WebSearchTool` expects an `endpoint` URL, not an API key. The fix requires:
1. Adding an `endpoint` field to `WebSearchConfig` (or constructing the Brave endpoint from the API key)
2. Passing the endpoint from config into `register_all()` and through to the tool constructor
3. Adding the API key as a header

**Recommendation**: Update GAP-03 remediation to include all three steps. LOE should be 2h, not 1h.

---

**ISSUE-05: The `unimplemented!()` in session.rs (GAP-16) is in test code only, not production code.**

The `unimplemented!()` at `/home/aepod/dev/clawft/crates/clawft-core/src/session.rs` line 476 is inside `MockHttp`, a test-only `HttpClient` implementation inside `#[cfg(test)] mod tests`. This is NOT production code. The mock is never called by actual tests (session tests don't make HTTP calls).

**Impact**: GAP-16 is a false positive P0. The sprint plan allocates 1 hour to replacing an `unimplemented!()` that only exists in test mocks.

**Recommendation**: Demote GAP-16 to P2 (minor quality cleanup). If the mock is truly unused, removing it is fine but not blocking.

### Minor

**ISSUE-06: Section 6.3 Config Completeness table incorrectly reports `pipelines` as `NOT IMPLEMENTED`.**

The table says `PipelineConfig` is not implemented. The `Config` struct in `clawft-types` does have `pub pipelines: Option<HashMap<String, PipelineConfig>>`. The *Config struct* parses it; the *PipelineRegistry* does not read it. The gap description (GAP-38) is accurate but the table status should say "parsed but unused" not "NOT IMPLEMENTED".

---

**ISSUE-07: Acceptance criteria bullet "LLM provider bootstrap wires automatically from config" is already passing.**

Since `enable_live_llm()` is called unconditionally, this criterion is met. The gate criterion should be reworded to: "`weft agent -m 'hello'` returns an LLM response (with configured provider)" -- which is already the first bullet.

---

**ISSUE-08: Acceptance criteria bullet "MCP servers from config are connected and tools are registered" is already passing.**

Same as ISSUE-02. The MCP wiring is complete. The gate criterion should be reworded to: "MCP server integration verified end-to-end with a real MCP server."

---

## Gaps MISSED by the Gap Analysis

### MISSED-01: `register_all()` does not accept config, so tool-specific config (web search endpoint, exec timeout) is not wired

The `register_all()` function in `clawft-tools/src/lib.rs` takes `workspace_dir`, `command_policy`, and `url_policy`, but does NOT take the `Config` or `ToolsConfig` struct. This means:
- `WebSearchTool` always gets `endpoint: None`
- `web.search.api_key` is never used
- `web.search.max_results` is never used
- `exec_tool.timeout` in config may not be propagated (needs audit)

**Priority**: P0 (same as GAP-03 but broader in scope). The fix is to pass `&ToolsConfig` into `register_all()` and wire all tool-specific config fields.

### MISSED-02: No graceful degradation when LLM provider config is missing

`build_live_pipeline()` delegates to `llm_adapter::build_live_pipeline(config)`. If no providers are configured in `config.providers.providers`, the adapter still creates a provider but with empty API key/base URL. This will produce cryptic HTTP errors at runtime instead of a clear "no LLM provider configured" message.

**Priority**: P1. The `enable_live_llm()` path should check `config.providers.providers.is_empty()` and either fall back to stub with a clear message, or log a prominent warning.

### MISSED-03: `Mutex::unwrap()` in production paths (session.rs)

The file `/home/aepod/dev/clawft/crates/clawft-core/src/session.rs` has 10+ occurrences of `self.files.lock().unwrap()` and `self.dirs.lock().unwrap()` in the `MockFs` test helper. While these are in test code (not production), the pattern should be audited in all production `Mutex` uses across the codebase to ensure poisoned lock panics cannot occur.

**Priority**: P2 (audit item).

### MISSED-04: `web_search` tool does not pass authentication headers

Even if the endpoint is wired (fixing GAP-03), the tool sends requests with an empty `headers` HashMap (line 93 of `web_search.rs`). Most search APIs (Brave, SerpAPI, etc.) require an `Authorization` or `X-Subscription-Token` header. The API key from config needs to be injected as a header.

**Priority**: P0 (part of GAP-03 fix, currently not mentioned in the remediation plan).

### MISSED-05: `web_fetch` tool has no content-length limit

The `web_fetch` tool fetches arbitrary URLs but there is no response body size limit. A malicious or large URL could consume unbounded memory. The `MAX_TOOL_RESULT_BYTES` truncation happens AFTER the full response is loaded into memory. A defense-in-depth approach would limit the HTTP response body read.

**Priority**: P1 (security hardening).

---

## Priority Reassessments

| Gap ID | Original | Reassessed | Rationale |
|--------|----------|------------|-----------|
| GAP-01 | P0 | RESOLVED (demote to P1 validation) | `enable_live_llm()` already called unconditionally in both agent.rs and gateway.rs |
| GAP-02 | P0 | RESOLVED (demote to P1 validation) | Same as GAP-01; the live pipeline IS wired |
| GAP-06 | P0 | RESOLVED (demote to P1 validation) | Full MCP tool bridge exists in `mcp_tools.rs`, wired in both commands |
| GAP-11 | P0 | P0 (keep, but fix description) | Streaming IS missing, but the trait does NOT have `complete_stream()` -- it needs to be added |
| GAP-16 | P0 | P2 | The `unimplemented!()` is in test-only mock code, not production |
| GAP-03 | P0 | P0 (keep, expand scope) | Worse than described: `register_all()` never reads config; auth headers not passed |
| GAP-19 | P0 | PASS (demote to verification) | `truncate_result()` IS called at line 283 of `loop_core.rs` with `MAX_TOOL_RESULT_BYTES`. Implementation has thorough tests. Just needs manual verification, not implementation work. |

**Revised P0 count: 8** (was 12)

| Remaining P0 | Description |
|---|---|
| GAP-03 (expanded) | Web search tool: endpoint not wired, auth headers missing, config not passed to `register_all()` |
| GAP-11 (corrected) | SSE streaming: trait needs new method, implementation, pipeline wiring |
| GAP-12 | `weft onboard` missing |
| GAP-14 | JSON repair not implemented |
| GAP-15 | VectorStore not wired into memory tool search |
| GAP-17 | Tool call parsing in transport.rs (verify correctness) |
| GAP-18 | No circuit breaker / retry logic in clawft-llm |
| MISSED-01 | `register_all()` does not accept ToolsConfig |

Note: GAP-17 should be verified before allocating implementation time -- the `convert_response()` function in transport.rs appears to handle tool calls correctly based on the test suite.

---

## Recommendations

1. **Re-baseline the sprint plan** after removing resolved GAPs (01, 02, 06, 16, 19). Day 1 loses ~6 hours of unnecessary work. Reallocate that time to Day 2 (SSE streaming is the hardest P0).

2. **Expand GAP-03 remediation** to include passing `ToolsConfig` into `register_all()`, constructing the Brave API endpoint from config, and injecting the API key as an HTTP header.

3. **Add a "smoke test" acceptance criterion**: Run `weft agent -m "hello"` with a real provider API key (OpenAI or Anthropic) and verify a response. This replaces the incorrect GAP-01/02 work with actual validation.

4. **Verify GAP-17 before allocating 2 hours**: The `convert_response()` function in transport.rs already has tests for text responses, tool calls, mixed text+tools, invalid JSON arguments, missing usage, and error propagation. Review the tests before assuming this needs implementation work.

5. **Treat MISSED-01 and GAP-03 as a single work item**: Refactoring `register_all()` to accept `ToolsConfig` and wiring all config fields is one coherent change. Combined LOE: 3h.

6. **Add MISSED-05 (web_fetch body size limit) to the P1 security items** alongside SEC-04 and SEC-05.

---

## Sprint Plan Assessment

### Original Plan (4 days, 12 P0s)

The original plan allocates 30+ hours across 4 days for 12 P0 items. With the corrected P0 count of 8 and 3 items being verification-only (not implementation), the actual implementation work is approximately 22 hours.

### Revised Plan Recommendation

**Day 1: SSE Streaming + Web Search Fix (Critical UX)**

| Task | Gap IDs | LOE | Notes |
|------|---------|-----|-------|
| SSE streaming: add `complete_stream()` to LlmTransport trait | GAP-11 | 6h | Hardest single item; start early |
| Fix web search: wire ToolsConfig into register_all, add endpoint+auth | GAP-03 + MISSED-01 | 3h | Config plumbing + tool fix |

**Day 2: JSON Repair + Retry + Onboard**

| Task | Gap IDs | LOE | Notes |
|------|---------|-----|-------|
| JSON repair for malformed LLM output | GAP-14 | 2h | |
| Retry/failover in clawft-llm | GAP-18 | 4h | Exponential backoff + provider failover |
| `weft onboard` command | GAP-12 | 3h | Non-interactive first (`--yes` flag) |

**Day 3: Memory + Verification + Tests**

| Task | Gap IDs | LOE | Notes |
|------|---------|-----|-------|
| Wire VectorStore into memory tool search | GAP-15 | 2h | |
| Verify tool call parsing in transport.rs (audit, not implement) | GAP-17 | 1h | May already be correct |
| clawft-llm mock HTTP tests | TEST-01 | 3h | |
| Agent loop e2e test with tool calls | TEST-04 | 2h | |

**Day 4: P1 Security + Docs + Smoke Test**

| Task | Gap IDs | LOE | Notes |
|------|---------|-----|-------|
| API key logging audit | SEC-04 | 1h | |
| Symlink path traversal test | SEC-05 | 1h | |
| web_fetch body size limit | MISSED-05 | 1h | |
| Create docs/benchmarks/results.md | DOC-01 | 1h | |
| Fix routing.md readability reference | DOC-04 | 0.5h | |
| Smoke test: `weft agent -m "hello"` with real API key | Validation | 1h | |
| Smoke test: MCP integration with real server | Validation | 1h | |

This revised plan is more realistic: 4 days, ~33 hours of work with clear validation checkpoints.

### Risk Reassessment

The original risk table is accurate. The highest risk remains SSE streaming (6h LOE, Day 1). If streaming slips, the fallback is acceptable: print full response synchronously and defer streaming to a follow-up. This fallback should be explicitly documented in the acceptance criteria as an acceptable degradation.
