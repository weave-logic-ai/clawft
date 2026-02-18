# Expert Review: Phase 3H -- Claude & Claude-Flow Tool Delegation

**Reviewer**: Senior Distributed Systems Engineer (MCP Protocol / Tool Orchestration / LLM Agent Architectures)
**Date**: 2026-02-17
**Document**: `.planning/sparc/3h-tool-delegation.md` (1270 lines)
**Verdict**: **APPROVE_WITH_CHANGES**

---

## Scores

| Dimension | Score (1-5) | Notes |
|-----------|:-----------:|-------|
| MCP Protocol Correctness | 4 | Content-Length framing, handshake, notifications all correct. Missing `listChanged` notification handling, keepalive/ping, and protocol version negotiation fallback. |
| Agent Loop Integration | 3 | MCP client wiring is clean. Claude delegation bridge (section 2.5) duplicates the tool loop logic in `loop_core.rs` rather than reusing it. |
| Delegation Engine | 4 | Rule-based with regex + complexity heuristic. Auto mode is reasonable. Missing rule validation at config parse time. |
| Claude Tool Format | 5 | OpenAI-to-Anthropic schema translation is correct and complete. Tool result block format matches Anthropic spec. |
| Session Lifecycle | 3 | McpSession tracks server capabilities but has no reconnection logic. No lifecycle management for long-lived MCP client sessions. |
| Error Handling | 4 | Graceful degradation (FR-008) is well-specified. Transport errors logged, servers skipped. Missing structured error propagation to callers in some paths. |
| Security | 3 | API key masking mentioned. MCP server mode does not define sandboxing or access control for incoming tool calls. CommandPolicy/UrlPolicy mentioned in NFRs but not reflected in MCP server pseudocode. |
| Performance | 4 | Handshake < 100ms target is reasonable. Single-threaded async MCP server avoids concurrency overhead. Missing read timeout on Content-Length parsing. |
| **Overall** | **3.75** | Solid plan with correct protocol understanding. Needs targeted fixes for cross-phase alignment, agent loop integration, MCP server security, and session lifecycle. |

---

## Strengths

1. **Correct problem diagnosis**: The plan accurately identifies all 7 gaps in the existing MCP implementation. The current `StdioTransport` at `clawft-services/src/mcp/transport.rs:73-74` uses newline-delimited JSON (`line.push('\n')` / `read_line()`), which is incompatible with real MCP servers. The plan correctly identifies this as the root cause of MCP integration failures.

2. **Clean feature flag strategy**: MCP protocol fixes (Content-Length framing, handshake, notifications) are NOT feature-gated -- they fix the existing client for all users. Only the delegation engine, Claude bridge, and DelegateTaskTool are behind `delegate` / `tool-delegate` flags. This avoids the trap of feature-gating bug fixes.

3. **Incremental session design**: The 4-session implementation order is logically sequenced. Session 1 (protocol fixes) and Session 2 (integration) can be shipped independently of Sessions 3-4 (delegation). Each session produces a commit that passes CI.

4. **Comprehensive TDD plan**: 60+ tests across 12 tasks, covering happy paths, error paths, edge cases, and format variations. Test fixtures include mock transports and pre-programmed responses. The plan correctly notes that the MCP server should take `Read + Write` trait objects (not stdin/stdout) for testability.

5. **Existing wiring leveraged**: The plan builds on the existing `McpToolWrapper` in `clawft-cli/src/mcp_tools.rs`, `register_mcp_tools()`, and the `{server}__{tool}` namespacing convention. It extends rather than replaces the existing infrastructure.

6. **Protocol references**: Appendix A provides exact JSON-RPC examples for Content-Length framing (A.1), initialize handshake (A.2), tools/call response format (A.3), Anthropic tool use (A.4), and schema translation (A.5). These are all correct per the MCP 2025-06-18 spec and Anthropic Messages API.

---

## Issues

### Critical

**CRIT-01: MCP Server Mode has no access control or sandboxing**

The `weft mcp-server` pseudocode (section 2.4) dispatches `tools/call` requests directly to `tool_registry.execute(name, args)` with no access control. When clawft runs as an MCP server, external clients can call `exec_shell`, `write_file`, `spawn`, or any other tool without restriction.

The technical requirements (section 10, line 1023-1027) specify "session isolation" for delegated calls but the 3H plan's MCP server pseudocode does not implement any form of:
- Caller authentication or identification
- Tool allowlist/denylist per caller
- CommandPolicy/UrlPolicy enforcement for incoming MCP requests
- Session isolation (all calls share the global tool registry)

NFR-003 states "Delegated tool execution respects CommandPolicy and UrlPolicy" but this is only implemented for the Claude delegation bridge (section 2.5 via `excluded_tools`), NOT for the MCP server mode.

**Impact**: Any process that can connect to `weft mcp-server` via stdin/stdout gains unrestricted access to all registered tools, including shell execution and file write.

**Recommendation**: Add an `allowed_tools` config field for MCP server mode (similar to `excluded_tools` in `DelegationConfig`). Apply `CommandPolicy` and `UrlPolicy` checks in the MCP server's `tools/call` handler. Document in the security model that MCP server mode inherits the host's security context.

---

**CRIT-02: Claude delegation bridge duplicates the agent loop tool execution logic**

The pseudocode in section 2.5 (`delegate_to_claude`) implements its own tool execution loop:

```
LOOP
    response = http_post(anthropic_api, {...})
    FOR block IN response["content"]
        IF block["type"] == "tool_use" THEN
            result = registry.execute(block["name"], block["input"])
            ...
```

This is a separate tool execution loop from the one in `clawft-core/src/agent/loop_core.rs:231-306` (`run_tool_loop()`). The existing loop handles:
- Tool result truncation via `crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES)` (line 283)
- Error formatting with JSON escaping (line 288)
- Max iterations enforcement (line 237)
- Debug logging per iteration (line 272-276)

The delegation bridge's loop duplicates all of this without the truncation, logging, or the security guards.

**Impact**: Tool results in the delegation path are not truncated, potentially blowing the Anthropic context window. Error formatting is inconsistent. Security logging is bypassed.

**Recommendation**: Extract the tool execution logic from `run_tool_loop()` into a reusable helper (e.g., `execute_tool_with_guards(registry, name, input) -> String`) that applies truncation, error formatting, and logging. Use this helper in both the agent loop and the Claude delegation bridge.

---

### Major

**MAJ-01: Cross-phase conflict with 3F-RVF MCP server bridge**

Phase 3F-RVF (`.planning/sparc/3f-rvf-integration.md`) creates a separate `clawft-rvf-mcp` crate that implements its own MCP server bridge with Content-Length framing (Sprint 6, Week 20). This bridge exposes 11 RVF tools (rvf_open, rvf_ingest, rvf_query, rvf_status, etc.) via JSON-RPC over stdio.

Phase 3H creates a `weft mcp-server` command in `clawft-cli` that also implements an MCP server over stdio with Content-Length framing.

These are two independent MCP server implementations with duplicated:
- Content-Length framing read/write logic
- `initialize` handshake handling
- `tools/list` and `tools/call` dispatch
- JSON-RPC request parsing

**Impact**: Code duplication, divergent protocol implementations, maintenance burden. Two separate MCP servers for different tool sets, with no mechanism to compose them.

**Recommendation**: Extract shared MCP server infrastructure into `clawft-services/src/mcp/server.rs` (a generic `McpServer` struct that takes a `ToolRegistry` and handles the protocol). Both `weft mcp-server` (3H) and `clawft-rvf-mcp` (3F) should use this shared server. The plan should mention this shared extraction and coordinate with 3F's timeline.

---

**MAJ-02: No workspace-scoped MCP server config support**

Phase 3G (`.planning/sparc/3g-workspaces.md`) introduces workspace-scoped configuration with a 3-layer config hierarchy:
```
Layer 1: Built-in defaults
Layer 2: Global config (~/.clawft/config.json)
Layer 3: Workspace config (<workspace>/.clawft/config.json)
```

The technical requirements (section 10, lines 1002-1004) explicitly state that MCP server configs are merged from both global and project-level sources:
```
Config sources (merged):
- Global: ~/.clawft/config.json -> tools.mcp_servers[]
- Project: .clawft/mcp/servers.json or .clawft/config.json -> tools.mcp_servers[]
```

The 3H plan's `register_mcp_tools()` function reads only `config.tools.mcp_servers` (a single flat map). It does not account for:
- Workspace-scoped MCP server configs that should overlay global configs
- The 3G deep merge semantics (objects merged recursively, arrays replaced)
- The `.clawft/mcp/servers.json` alternative config file mentioned in the tech requirements

**Impact**: MCP servers configured at the workspace level will not be discovered. Workspace-specific tool integrations will not work.

**Recommendation**: Add a note that `register_mcp_tools()` should accept the merged config (after 3G's deep merge has been applied). If 3H ships before 3G, document the forward-compatibility requirement: the function should work with the flat config now and automatically gain workspace support when 3G's config merge is wired in. No code changes needed if the config is already merged before `register_mcp_tools()` is called, but the plan should explicitly state this dependency.

---

**MAJ-03: No reconnection or lifecycle management for MCP client sessions**

The `McpSession::connect()` handshake is a one-shot operation. Once the session is established, there is no mechanism for:
- Detecting that the MCP server process has crashed (e.g., child process exited)
- Reconnecting to a restarted server
- Keepalive/ping to detect stale connections
- Handling `listChanged` notifications (MCP spec allows servers to notify when their tool list changes)
- Graceful shutdown (sending `notifications/cancelled` when the agent stops)

The existing `StdioTransport` holds an `Arc<Mutex<Child>>` but never checks if the child process is still alive.

**Impact**: If an MCP server crashes during an agent session, all subsequent tool calls to that server will hang or error. The agent loop will log errors but cannot recover without restarting. For long-running `weft gateway` sessions, this is a significant reliability concern.

**Recommendation**: Add at minimum:
1. A `is_alive()` check on `StdioTransport` (check `child.try_wait()`)
2. An exponential backoff reconnection in `McpSession` with a max retry count
3. A configurable `timeout` parameter on `StdioTransport::send_request()` (the risk assessment mentions this in row 3 but no implementation is specified)

---

**MAJ-04: `McpTransport` trait change is breaking for downstream consumers**

Adding `send_notification()` to the `McpTransport` trait (section 3.4) is a breaking change. Every existing implementor of `McpTransport` must add this method:
- `StdioTransport` (updated in plan)
- `HttpTransport` (updated in plan)
- `MockTransport` (updated in plan, but it is `#[cfg(test)]`)
- The `TestTransport` in `clawft-cli/src/mcp_tools.rs:160-187` (NOT mentioned in the plan)

The `TestTransport` in `mcp_tools.rs` is a test-only mock that implements `McpTransport`. Adding a new method to the trait will break compilation of `clawft-cli` tests unless `TestTransport` is also updated.

**Impact**: Build failure on Session 1 commit if `TestTransport` is not updated.

**Recommendation**: Add `TestTransport::send_notification()` implementation to the Task 2 scope. Also, consider adding a default implementation to the trait method to avoid breaking any external implementors:
```rust
async fn send_notification(&self, _notification: JsonRpcNotification) -> Result<()> {
    Ok(()) // Default: silently ignore
}
```

---

### Minor

**MIN-01: `JsonRpcRequest.id` is `u64`, but JSON-RPC 2.0 allows `String | Number | null`**

The current `JsonRpcRequest` in `types.rs:10` has `pub id: u64`. JSON-RPC 2.0 spec allows the `id` to be a string, number, or null. Some MCP servers may use string IDs. The `JsonRpcNotification` type will have no `id` field (correct), but `JsonRpcResponse.id` is also `u64`, which means responses from servers using string IDs will fail to deserialize.

**Impact**: Low -- most MCP servers use numeric IDs. But the 3H plan's MCP server mode (section 2.4) receives requests from external clients who may use string IDs.

**Recommendation**: Consider changing `id` to `serde_json::Value` or `Option<serde_json::Value>` in a follow-up. For 3H, at minimum, the MCP server mode should handle non-numeric IDs gracefully (return an error rather than crashing on deserialization).

---

**MIN-02: Delegation complexity estimator is simplistic**

The `estimate_complexity()` function in section 2.6 uses keyword matching ("orchestrate", "research") and word count. This is fragile:
- Short messages with complex intent ("refactor the auth module") score low
- Long messages with simple intent (pasted error logs) score high
- No consideration of tool count, context size, or conversation history

**Impact**: The `Auto` delegation target may make suboptimal decisions, routing complex tasks locally or simple tasks to Claude.

**Recommendation**: Document this as a known limitation. Consider adding a `tool_count_hint` parameter (if the message references multiple tools, bump complexity). The plan's edge case section (5.5) does not mention this.

---

**MIN-03: MCP server mode does not validate method ordering**

The MCP spec requires that `initialize` must be the first method called. The server pseudocode (section 2.4) handles each method independently in a match statement without tracking whether `initialize` has been received. A client that sends `tools/list` before `initialize` should receive an error per the spec.

**Impact**: Non-compliant MCP server behavior. Well-behaved clients will not trigger this, but a malformed client could interact with tools before the handshake completes.

**Recommendation**: Add an `initialized: bool` flag to the server state. Return JSON-RPC error -32002 ("Server not initialized") for any method other than `initialize` when `initialized == false`.

---

**MIN-04: No `params` field on `JsonRpcNotification` default**

Section 3.5 shows `McpClient::send_notification()` creating a `JsonRpcNotification` with a `params` field. The MCP spec's `notifications/initialized` notification has no params (just `jsonrpc` and `method`). If `params` is serialized as `{}` (empty object), this is technically fine per JSON-RPC 2.0, but some servers may be strict about unexpected fields.

**Recommendation**: Use `#[serde(skip_serializing_if = "Value::is_null")]` or `Option<Value>` for the `params` field on `JsonRpcNotification`, defaulting to `None` when no params are provided. Send `notifications/initialized` without a `params` field.

---

**MIN-05: `DelegationConfig.claude_model` default may become stale**

The default model `"claude-sonnet-4-20250514"` is hardcoded. Model IDs change over time as Anthropic releases new versions.

**Recommendation**: Use an alias like `"claude-sonnet-4-latest"` as the default if the Anthropic API supports it, or document that this default should be updated periodically. Not a blocking issue.

---

## Cross-Phase Conflicts

| Conflict | Phases | Severity | Resolution |
|----------|--------|----------|------------|
| Duplicate MCP server implementations | 3H (weft mcp-server) vs 3F (clawft-rvf-mcp) | Major | Extract shared `McpServer` into clawft-services. See MAJ-01. |
| Workspace-scoped MCP configs not addressed | 3H vs 3G | Major | Ensure `register_mcp_tools()` consumes the post-merge config. See MAJ-02. |
| MCP Content-Length framing implemented twice | 3H (StdioTransport rewrite) vs 3F (rvf-mcp Sprint 6) | Medium | 3H ships first; 3F should reuse the shared framing helpers from 3H's `transport.rs`. |
| Skill permission system not consulted | 3H (delegation) vs 3F (agents-skills) | Low | The agents-skills plan (3F-agents-skills.md) introduces `allowed_tools` per agent. The delegation engine should respect these when an agent delegates. Add a TODO for post-3F integration. |
| Tool registry snapshot stale for delegation | 3H vs 3F (skill-based tool registration) | Low | Risk assessment row 7 mentions this. The snapshot at startup means dynamically registered tools (from skill activation) are not visible to the delegation bridge. Document as known limitation. |

---

## Missing Requirements

| ID | Source Document | Requirement | Gap in 3H |
|----|----------------|-------------|-----------|
| MR-01 | 02-technical-requirements.md line 1024 | "Tool advertisement: All registered tools exposed via MCP `tools/list`" with `ToolRegistry::to_mcp_tools()` | Plan uses `tool_registry.list()` + manual JSON construction. The tech requirements specify a `to_mcp_tools()` method on `ToolRegistry`. Not functionally different, but naming/API mismatch. |
| MR-02 | 02-technical-requirements.md line 1025 | "Project-aware: MCP server inherits project workspace context from cwd" | Plan does not mention workspace context in MCP server mode. The server should respect 3G workspace discovery when resolving tool paths and config. |
| MR-03 | 02-technical-requirements.md line 1026 | "Skill passthrough: Skills loaded from project + global are available to delegated calls" | Plan does not mention skills in MCP server mode. If skills register tools in the ToolRegistry, they should be visible via `tools/list`. |
| MR-04 | 02-technical-requirements.md line 1027 | "Session isolation: Delegated calls use a dedicated session or caller-specified session ID" | Plan does not implement session isolation for MCP server mode. All tool calls share the global context. |
| MR-05 | 3i-gap-analysis.md GAP-06 | "Need: (1) parse mcp_servers from config, (2) connect transports, (3) list_tools, (4) register as dynamic tools in ToolRegistry, (5) handle tool execution via McpClient" | Plan addresses (1)-(5) for the client side. However, the gap analysis notes that `register_mcp_tools()` is not called from `bootstrap.rs`. The plan should verify the call site exists (it does: both `agent.rs` and `gateway.rs` call it). |
| MR-06 | 3i-gap-analysis.md GAP-19 | "Verify MAX_TOOL_RESULT_BYTES truncation is enforced" | The Claude delegation bridge (section 2.5) does NOT apply truncation to tool results before sending them back to the Anthropic API. See CRIT-02. |

---

## MCP Protocol Compliance

### Correct

| Aspect | Assessment |
|--------|-----------|
| Content-Length header framing | Correct. `Content-Length: N\r\n\r\n{json}` matches MCP 2025-06-18 spec. Write helper and read helper both specified. |
| Initialize handshake sequence | Correct. Send `initialize` -> receive result -> send `notifications/initialized`. Params include `protocolVersion`, `capabilities`, `clientInfo`. |
| Notification format | Correct. No `id` field, no response expected. `JsonRpcNotification` struct correctly omits `id`. |
| tools/list request | Correct. Empty params `{}`, returns `{tools: [...]}`. |
| tools/call request | Correct. Params `{name, arguments}`, returns `{content: [...], isError: bool}`. |
| Content block extraction | Correct. Filters for `type: "text"` blocks, concatenates, handles `isError: true`. |
| Server capability reporting | Correct. Reports `tools: {listChanged: false}` in initialize response. |
| Protocol version | Uses `2025-06-18` which is the current MCP spec version. Correct. |

### Issues

| Aspect | Issue | Severity |
|--------|-------|----------|
| `listChanged` notifications | Plan advertises `listChanged: false` in server mode but does not handle incoming `notifications/tools/list_changed` from external servers in client mode. If a server's tool set changes, clawft will not update its registry. | Low (most servers do not change tools at runtime) |
| Protocol version negotiation | Plan sends `protocolVersion: "2025-06-18"` but does not handle the case where the server responds with a different version. MCP spec says client SHOULD downgrade or reject. Plan says "log warning, proceed" which is acceptable but not spec-compliant. | Low |
| Cancellation | MCP 2025-06-18 supports `notifications/cancelled` for aborting in-flight operations. Not mentioned in the plan. | Low (nice-to-have) |
| Ping/keepalive | MCP spec defines `ping` method for connection liveness. Not mentioned. | Medium (relevant for long-lived gateway sessions) |
| Batch requests | MCP 2025-06-18 removed batch request support. Plan's single-request processing is correct. | Compliant (no issue) |
| Error codes | Server mode returns `-32601` for unknown methods. Should also handle `-32700` (parse error), `-32600` (invalid request), `-32602` (invalid params). | Low |

---

## Recommendations

### Before Implementation (Pre-Session 1)

1. **Coordinate with 3F-RVF**: Agree on a shared `McpServer` struct in `clawft-services` that both `weft mcp-server` and `clawft-rvf-mcp` will use. This avoids duplicate Content-Length framing code.

2. **Add `send_notification()` default impl to `McpTransport` trait**: Use a default implementation that returns `Ok(())` to avoid breaking the `TestTransport` in `clawft-cli/src/mcp_tools.rs:160-187`.

3. **Define MCP server security model**: Add an `allowed_tools` field to the MCP server config. Apply `CommandPolicy` checks in the `tools/call` handler.

### During Implementation (Sessions 1-4)

4. **Extract tool execution helper**: Factor the truncation + error formatting + logging from `run_tool_loop()` into a shared helper. Use it in both the agent loop and the Claude delegation bridge.

5. **Add read timeout to Content-Length parsing**: The `stdio_read()` function can hang indefinitely if the server sends a `Content-Length` header but then crashes before sending the body. Use `tokio::time::timeout()` with a configurable duration (suggest 30s default).

6. **Track initialization state in MCP server mode**: Add an `initialized: bool` flag. Reject pre-handshake requests with JSON-RPC error -32002.

7. **Update `TestTransport` in `mcp_tools.rs`**: Add `send_notification()` implementation to avoid build breakage.

### After Implementation (Post-Session 4)

8. **Add `is_alive()` check to `StdioTransport`**: Check `child.try_wait()` before sending requests. If the child has exited, return an error immediately rather than writing to a dead stdin pipe.

9. **Document workspace integration requirements**: Create a follow-up ticket for 3G integration: ensure `register_mcp_tools()` works with workspace-merged configs, and MCP server mode inherits workspace context.

10. **Plan `listChanged` notification handling**: When 3F-RVF or other dynamic tool sources are integrated, the MCP client should be able to re-list tools when it receives a `notifications/tools/list_changed` notification from the server.

---

## Timeline Assessment

| Session | Scope | Estimated Hours | Assessment |
|---------|-------|:---------------:|------------|
| 1 | MCP Protocol Fixes (FR-001, FR-002, FR-003) | 2-3h | **Realistic**. Content-Length framing rewrite is well-scoped. Tests are straightforward with mock stdin/stdout pairs. The notification type and trait extension are small additions. |
| 2 | MCP Integration (FR-004, FR-005) | 2-3h | **Realistic but tight**. Content extraction is simple. The MCP server mode (~250 lines) is the largest deliverable. Need to account for `TestTransport` update and MCP server security (if CRIT-01 is addressed here). |
| 3 | Delegation Engine (FR-006, FR-007) | 2-3h | **Realistic**. DelegationConfig types are straightforward serde structs. Schema translation is mechanical. The delegation engine regex matching is well-defined. ClaudeDelegator requires a mock HTTP server (mockito or similar). |
| 4 | Wiring + Validation (FR-008) | 2-3h | **Realistic**. Primarily CLI wiring and end-to-end testing. The DelegateTaskTool is a thin wrapper. Full validation suite should catch integration issues. |

**Overall**: 4 sessions / 8-12 hours is a reasonable estimate. The primary risk is Session 2 if MCP server security is added (CRIT-01), which could push it to 3-4 hours. The estimate of ~1150 lines new code and ~200 lines modifications is consistent with the file manifest.

**Dependency risk**: The plan depends on `2c-services` and `2e-integration-wiring`, both of which are complete. No blocking dependencies. The cross-phase concerns with 3F and 3G are forward-looking -- 3H can ship independently with documented TODOs.

---

## Summary

Phase 3H is a well-structured plan that correctly addresses the MCP protocol gaps in the existing codebase. The Content-Length framing fix, initialize handshake, and notification support are essential for real-world MCP server interoperability. The delegation engine provides a clean extensibility point for Claude and claude-flow integration.

The two critical issues are:
1. **MCP server mode security** (CRIT-01): No access control on incoming tool calls. Must be addressed before shipping server mode.
2. **Duplicated tool execution logic** (CRIT-02): The Claude delegation bridge should reuse the agent loop's tool execution guards (truncation, logging, security).

The major cross-phase conflict with 3F-RVF's MCP server (MAJ-01) should be coordinated before implementation to avoid duplicate infrastructure.

With these changes addressed, the plan is ready for implementation.
