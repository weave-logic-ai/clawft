# SPARC Feature Element 07: Dev Tools & Applications

**Workstream**: F (Software Dev & App Tooling)
**Timeline**: Weeks 5-10
**Status**: Development Complete (10/10 items)
**Dependencies**: 04/C1 (plugin traits), 05/D9 (MCP transport for F9)
**Blocks**: None directly

---

## 1. Summary

Developer tools and application integrations implemented as plugins: git, cargo/build, code analysis, browser automation, calendar, OAuth2, Docker, MCP IDE integration, and MCP client for external servers.

---

## 2. Phases

### Phase F-Core: Core Dev Tools (Week 5-7)

| Item | Description | Key Crate |
|------|-------------|-----------|
| F1 | Git tool plugin (clone, commit, branch, PR, diff, blame) | `git2` |
| F2 | Cargo/build integration (build, test, clippy, publish) | subprocess |
| F6 | Generic REST + OAuth2 helper (reusable flow) | `oauth2` |
| F9a | Core MCP client library -- connect to single external MCP server, list tools, invoke tools. Lives in `clawft-services/src/mcp/client.rs`. | `clawft-services` |

### Phase F-Advanced: Advanced Tools (Week 7-9)

| Item | Description | Key Crate |
|------|-------------|-----------|
| F3 | Code analysis via tree-sitter (AST parsing, complexity). Native-only, no WASM variant. Grammars as optional features (`tree-sitter-rust`, `tree-sitter-typescript`, etc.). | `tree-sitter` |
| F4 | Browser CDP automation (screenshot, form fill, scrape) | `chromiumoxide` |
| F5 | Calendar integration (Google Cal, Outlook, iCal) | REST + OAuth2 |
| F7 | Docker/Podman orchestration | subprocess |

### Phase F-MCP: MCP Ecosystem (Week 8-10)

| Item | Description |
|------|-------------|
| F8 | MCP deep IDE integration (VS Code extension backend) |
| F9b | Full MCP client features -- auto-discovery, connection pooling, schema caching, health checks (the "1000+ community servers" story). Depends on F9a. |

---

## 3. Tool Permission Model

Each dev tool plugin declares its permission requirements in the plugin manifest (`permissions.filesystem`, `permissions.network`, `permissions.shell`). The K3 sandbox enforces these permissions at runtime. Permissions must be specified before implementation begins -- they cannot be retrofitted.

---

## 4. MCP Tool Namespace Convention

External MCP tools use the `mcp:<server-name>:<tool-name>` namespace to avoid name collisions with local tools. When F9a connects to an external MCP server, all tools from that server are dynamically registered in the `ToolRegistry` with this namespace prefix. The Router uses the namespace to identify MCP-origin tools.

---

## 5. Crate Structure

Dev tool plugins are implemented as **separate workspace crates**, one per tool, behind CLI feature flags. This keeps compilation parallel and avoids a monolithic plugin crate.

| Crate | Tool | Feature Flag |
|-------|------|-------------|
| `crates/clawft-plugin-git` | F1: Git operations | `plugin-git` |
| `crates/clawft-plugin-cargo` | F2: Cargo/build | `plugin-cargo` |
| `crates/clawft-plugin-treesitter` | F3: Code analysis | `plugin-treesitter` |
| `crates/clawft-plugin-browser` | F4: Browser CDP | `plugin-browser` |
| `crates/clawft-plugin-calendar` | F5: Calendar | `plugin-calendar` |
| `crates/clawft-plugin-oauth2` | F6: OAuth2 helper | `plugin-oauth2` |
| `crates/clawft-plugin-containers` | F7: Docker/Podman | `plugin-containers` |

F8 and F9 are NOT separate crates. They extend `clawft-services/src/mcp/` (MCP infrastructure layer).

---

## 6. Browser CDP Sandbox Requirements

F4 must define a `BrowserSandboxConfig` struct specifying sandbox boundaries for headless Chrome automation:

```rust
pub struct BrowserSandboxConfig {
    /// Domains the browser is allowed to navigate to. Empty = block all.
    pub allowed_domains: Vec<String>,
    /// Maximum number of concurrent browser pages per agent.
    pub max_concurrent_pages: u32,   // default: 2
    /// Maximum browser session lifetime before forced termination.
    pub session_lifetime: Duration,  // default: 300s
    /// Maximum memory for the browser process.
    pub max_memory_mb: u64,          // default: 512
    /// Cookie/storage policy: clear between sessions.
    pub clear_state_between_sessions: bool, // default: true
}
```

This config is part of the C1 plugin manifest extension for the browser tool. The K3 sandbox enforces it at runtime.

---

## 7. Exit Criteria

- [x] `weft` can create git branches and commits via tool call
- [x] Browser CDP takes screenshots and fills forms
- [x] OAuth2 flow works for at least one provider (Google)
- [x] F9a: MCP client connects to a single external MCP server and exposes its tools
- [x] F9b: MCP client supports auto-discovery, connection pooling, and schema caching
- [x] All tools implement `Tool` plugin trait
- [x] All existing tests pass

### Security Exit Criteria

- [x] Browser tool blocks `file://`, `data://`, and `javascript://` URL schemes
- [x] Browser tool clears state (cookies, storage, sessions) between sessions
- [x] MCP stdio child processes do not inherit secret environment variables (minimal env constructed explicitly)
- [x] External MCP server tools are tagged as "untrusted" in the tool registry

---

## 8. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Browser CDP security exposure (local file access, JS eval exfiltration) | Medium | High | **6** | BrowserSandboxConfig with allowed_domains; block file/data/javascript URL schemes; clear state between sessions |
| Malicious MCP server tool injection via F9 | Medium | High | **6** | Tag external tools as "untrusted"; require user approval for first invocation; sandbox stdio child processes |
| MCP stdio transport spawns arbitrary commands | Medium | High | **6** | Validate stdio command paths; inherit minimal env vars; require full paths for untrusted commands |
| tree-sitter C grammar FFI memory safety | Low | Medium | **3** | Native-only (no WASM); fuzz test C grammar bindings; optional features per language |
| F9a/F9b split delays full MCP client | Low | Medium | **3** | F9a scoped to minimal viable client; M4 depends on F9a only; F9b is non-blocking |
| Browser + Docker resource exhaustion under multi-agent use | Medium | Medium | **4** | Per-tool concurrency limits in plugin manifest (default: 1 browser session/agent, 3 Docker ops/global) |
| Plugin crate proliferation increases CI build time | Low | Low | **2** | Feature-gated; CI builds only changed crates; parallel workspace compilation |
