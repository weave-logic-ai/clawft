# Element 07: Dev Tools & Applications -- Execution Tracker

## Summary

- **Total items**: 10 (F1-F9b)
- **Workstream**: F (Software Dev & App Tooling)
- **Timeline**: Weeks 5-10
- **Status**: Planning -> Development
- **Dependencies**: 04/C1 (plugin traits), 05/D9 (MCP transport concurrency), 04/C6 (MCP server for skills)
- **Blocks**: 06/E5a (Google Chat needs F6 OAuth2), 09/M4 (FlowDelegator needs F9a MCP client)

---

## Execution Schedule

Element 07 has 10 items (F1-F9b) across 3 phases spanning Weeks 5-10.

### Week 5-7 (F-Core -- 4 items, P0/P1)

- [x] F1 -- Git tool plugin (git2): clone, commit, branch, diff, blame, log, status -- crates/clawft-plugin-git
- [x] F2 -- Cargo/build integration (subprocess): build, test, clippy, check, publish -- crates/clawft-plugin-cargo
- [x] F6 -- Generic REST + OAuth2 helper (reusable flow): auth code flow, PKCE, token refresh, REST client -- crates/clawft-plugin-oauth2 (**P0 -- unblocks E5a**)
- [x] F9a -- Core MCP client library (MVP): connect, list_tools, call_tool, session handshake -- clawft-services/src/mcp/mod.rs (pre-existing, **P0 -- unblocks M4**)

### Week 6-9 (F-Advanced -- 4 items, P2)

- [ ] F3 -- Code analysis tree-sitter: AST parsing, complexity metrics, symbol listing -- crates/clawft-plugin-treesitter (new crate)
- [ ] F4 -- Browser CDP automation (chromiumoxide): screenshot, form fill, scrape, sandboxed -- crates/clawft-plugin-browser (new crate)
- [ ] F5 -- Calendar integration (REST+OAuth2): Google Cal, Outlook, iCal -- crates/clawft-plugin-calendar (new crate)
- [ ] F7 -- Docker/Podman orchestration (subprocess): build, run, stop, logs, exec -- crates/clawft-plugin-containers (new crate)

### Week 8-10 (F-MCP -- 2 items, P2)

- [ ] F8 -- MCP deep IDE integration: VS Code extension backend via MCP tools -- clawft-services/src/mcp/ide.rs (new file)
- [ ] F9b -- Full MCP client features: auto-discovery, connection pooling, schema caching, health checks -- clawft-services/src/mcp/client.rs + discovery.rs

---

## Per-Item Status Table

| Item | Description | Priority | Week | Crate / Location | Status | Owner | Branch | Key Deliverable |
|------|-------------|----------|------|-------------------|--------|-------|--------|-----------------|
| F1 | Git tool plugin (git2) | P1 | 5-7 | crates/clawft-plugin-git | **Done** | Agent-07 | sprint/phase-5 | 7 tools: clone, commit, branch, diff, blame, log, status. 15 tests. |
| F2 | Cargo/build integration (subprocess) | P1 | 5-7 | crates/clawft-plugin-cargo | **Done** | Agent-07 | sprint/phase-5 | 5 tools: build, test, clippy, check, publish. Input validation. 18 tests. |
| F3 | Code analysis tree-sitter (AST, complexity) | P2 | 6-8 | crates/clawft-plugin-treesitter (new) | Pending | -- | -- | Parse Rust/TS/Python, AST query, complexity metrics |
| F4 | Browser CDP automation (chromiumoxide) | P2 | 6-8 | crates/clawft-plugin-browser (new) | Pending | -- | -- | Screenshot + form fill with BrowserSandboxConfig enforcement |
| F5 | Calendar integration (REST+OAuth2) | P2 | 7-9 | crates/clawft-plugin-calendar (new) | Pending | -- | -- | List/create/update/delete events for Google Calendar |
| F6 | Generic REST + OAuth2 helper | **P0** | 5-7 | crates/clawft-plugin-oauth2 | **Done** | Agent-07 | sprint/phase-5 | 4 tools: authorize, callback, refresh, rest_request. PKCE+state CSRF. 0600 token perms. 19 tests. |
| F7 | Docker/Podman orchestration (subprocess) | P2 | 7-9 | crates/clawft-plugin-containers (new) | Pending | -- | -- | Build, run, stop, logs, exec with concurrency limits |
| F8 | MCP deep IDE integration | P2 | 8-10 | clawft-services/src/mcp/ide.rs (new) | Pending | -- | -- | VS Code extension connects to MCP server, IDE tools in tools/list |
| F9a | Core MCP client library (MVP) | **P0** | 5-7 | clawft-services/src/mcp/mod.rs (existing) | **Done** (pre-existing) | -- | sprint/phase-5 | McpClient + McpSession already implemented: connect, list_tools, call_tool, send_raw. 20+ tests. |
| F9b | Full MCP client features | P2 | 8-10 | clawft-services/src/mcp/client.rs + discovery.rs | Pending | -- | -- | Auto-discovery, connection pooling, schema caching, health checks |

---

## Dependency Graph

### External Dependencies (must land before Element 07 work begins)

| Prerequisite | Source Element | Description | Blocks |
|-------------|---------------|-------------|--------|
| C1 Plugin Traits | 04 | `Tool` trait in `clawft-plugin` crate | F1-F7 (all tool plugin crates) |
| D9 MCP Transport | 05 | MCP transport concurrency (non-serialized concurrent calls) | F9a, F9b (MCP client) |
| C6 MCP Server Skills | 04 | MCP server for loaded skills (CompositeToolProvider) | F8 (IDE integration extends server) |

### Internal Dependencies (within Element 07)

```
F6 (OAuth2) ──────> F5 (Calendar needs OAuth2 for Google Cal, Outlook)
  F6 MUST complete before F5 can authenticate with any provider.

F9a (MCP Client MVP) ──────> F9b (Full MCP Client)
  F9a MUST be stable before F9b adds auto-discovery, pooling, caching.
```

### Cross-Element Dependencies (Element 07 unblocks other elements)

| Source (Element 07) | Target (Other Element) | Type | Impact |
|---------------------|------------------------|------|--------|
| F6 (OAuth2) | E5a (Element 06, Google Chat) | **CRITICAL** | Google Chat Workspace API requires OAuth2 for authentication |
| F6 (OAuth2) | E2 (Element 06, Email) | Blocks | Gmail OAuth2 depends on F6 |
| F9a (MCP Client) | M4 (Element 09, FlowDelegator) | **CRITICAL** | FlowDelegator needs MCP client to invoke external tools |

### Cross-Element Integration Tests

| Test | Elements | Week | Priority |
|------|----------|------|----------|
| Email Channel -> OAuth2 Helper | 06, 07 | 7 | P0 |
| Plugin -> Hot-reload -> MCP | 04, 07 | 8 | P0 |

---

## Exit Criteria

### Functional

- [ ] `weft` can create git branches and commits via tool call (F1)
- [ ] Browser CDP takes screenshots and fills forms (F4)
- [ ] OAuth2 flow works for at least one provider (Google) (F6)
- [ ] F9a: MCP client connects to a single external MCP server and exposes its tools
- [ ] F9b: MCP client supports auto-discovery, connection pooling, and schema caching
- [ ] All tools implement `Tool` plugin trait (F1-F7)
- [ ] All existing tests pass (regression gate)

### Security Exit Criteria

- [ ] Browser tool blocks `file://`, `data://`, and `javascript://` URL schemes (F4)
- [ ] Browser tool clears state (cookies, storage, sessions) between sessions (F4)
- [ ] MCP stdio child processes do not inherit secret environment variables (minimal env constructed explicitly) (F9a)
- [ ] External MCP server tools are tagged as "untrusted" in the tool registry (F9a)

---

## Security Checklist

| Check | Items Affected | Requirement |
|-------|---------------|-------------|
| Browser URL scheme blocking | F4 | Block `file://`, `data://`, `javascript://` schemes; reject navigation to unlisted domains |
| Browser state clearing | F4 | Clear cookies, storage, sessions between sessions when `clear_state_between_sessions` is true |
| MCP stdio env isolation | F9a, F9b | Construct minimal env explicitly for stdio child processes; no secret env vars inherited |
| MCP untrusted tagging | F9a, F9b | External MCP server tools tagged as "untrusted" in tool registry; require user approval for first invocation |
| Command argument validation | F2, F7 | All subprocess commands built programmatically; no shell interpolation; sanitize package names, flags, container names |
| OAuth2 CSRF protection | F6 | `state` parameter included in all authorization requests |
| Token persistence security | F6 | Tokens stored at `~/.clawft/tokens/` with 0600 file permissions; rotated refresh tokens persisted immediately |
| OAuth2 client_secret handling | F6 | `client_secret` via `SecretRef` (env var name, not plaintext) |
| Tree-sitter FFI safety | F3 | Native-only (no WASM variant); fuzz test C grammar bindings |
| Stdio command path validation | F9a | Validate stdio command paths; require full paths for untrusted commands |

---

## Review Gates

| Gate | Scope | Requirement |
|------|-------|-------------|
| F-Core Security Review | F6, F9a | OAuth2 state parameter verified; MCP stdio env isolation confirmed; SecretRef usage for secrets |
| F-Core Functional Review | F1, F2, F6, F9a | Standard code review; tool call end-to-end tests; `mcp:<server>:<tool>` namespace verified |
| F-Advanced Security Review | F4 | BrowserSandboxConfig enforcement test; URL scheme blocking verified; state clearing between sessions |
| F-Advanced Functional Review | F3, F4, F5, F7 | Standard code review; tree-sitter parse + query tests; browser screenshot test; calendar CRUD test |
| F-MCP Review | F8, F9b | IDE tools in tools/list response; auto-discovery from config; connection pool reuse test |
| Trait Compliance Review | F1-F7 | All plugins implement `Tool` trait from `clawft-plugin`; MCP-compatible JSON Schema for `input_schema` |
| Cross-Element Integration Review | F6, F9a | Email->OAuth2 (Week 7), Plugin->Hot-reload->MCP (Week 8) tests pass |

---

## Risk Register

Scoring: Likelihood (Low=1, Medium=2, High=3) x Impact (Low=1, Medium=2, High=3, Critical=4)

| # | Risk | Likelihood | Impact | Score | Mitigation |
|---|------|-----------|--------|-------|------------|
| R1 | Browser CDP security exposure (local file access, JS eval exfiltration) | Medium | High | **6** | BrowserSandboxConfig with allowed_domains; block file/data/javascript URL schemes; clear state between sessions |
| R2 | Malicious MCP server tool injection via F9 | Medium | High | **6** | Tag external tools as "untrusted"; require user approval for first invocation; sandbox stdio child processes |
| R3 | MCP stdio transport spawns arbitrary commands | Medium | High | **6** | Validate stdio command paths; inherit minimal env vars; require full paths for untrusted commands |
| R4 | tree-sitter C grammar FFI memory safety | Low | Medium | **3** | Native-only (no WASM); fuzz test C grammar bindings; optional features per language |
| R5 | F9a/F9b split delays full MCP client | Low | Medium | **3** | F9a scoped to minimal viable client; M4 depends on F9a only; F9b is non-blocking |
| R6 | Browser + Docker resource exhaustion under multi-agent use | Medium | Medium | **4** | Per-tool concurrency limits in plugin manifest (default: 1 browser session/agent, 3 Docker ops/global) |
| R7 | Plugin crate proliferation increases CI build time | Low | Low | **2** | Feature-gated; CI builds only changed crates; parallel workspace compilation |

---

## Merge Coordination

### Merge Order

1. **F6** (OAuth2) -- first, unblocks E5a (Google Chat) in Element 06
2. **F1 + F2 + F9a** -- core dev tools + MVP MCP client (unblocks M4)
3. **F3 + F4** -- advanced tools (tree-sitter, browser)
4. **F5 + F7** -- calendar (needs F6) + Docker
5. **F8 + F9b** -- MCP ecosystem (post-MVP)

### File Ownership

Within `crates/clawft-services/src/mcp/`:
- F8 owns `ide.rs` (new)
- F9 owns `client.rs` (new) and `discovery.rs` (new)
- Shared files (`transport.rs`, `types.rs`, `middleware.rs`) require cross-stream PR review

### Conflict Zones

- Plugin crates are independent -- no cross-stream conflicts expected
- `clawft-services/src/mcp/` is shared with streams 5C (C6 server skills) and 5I (M4/M5 delegation)
- Changes to `transport.rs` or `types.rs` require cross-stream PR review

---

## Progress Summary

| Phase | Items | Pending | In Progress | Completed | % Done |
|-------|-------|---------|-------------|-----------|--------|
| F-Core (F1, F2, F6, F9a) | 4 | 0 | 0 | 4 | 100% |
| F-Advanced (F3, F4, F5, F7) | 4 | 4 | 0 | 0 | 0% |
| F-MCP (F8, F9b) | 2 | 2 | 0 | 0 | 0% |
| **Total** | **10** | **6** | **0** | **4** | **40%** |
