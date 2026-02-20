# Security Review: Iteration 1 -- All SPARC Feature Elements

> **Reviewer**: Security Expert Agent
> **Date**: 2026-02-19
> **Scope**: SPARC elements 03-10, improvements.md, business/technical requirement drafts
> **Classification**: Internal -- Engineering Review

---

## Executive Summary

The ClawFT improvements sprint introduces substantial new attack surface through plugin execution (WASM + native), multi-channel OAuth2 integrations, subprocess spawning (Claude Flow, signal-cli), browser automation, and multi-agent routing. The existing codebase has known security vulnerabilities (A4/A5/A6) that are correctly identified and scheduled for remediation in element 03.

**Overall Security Posture**: The plans demonstrate security awareness -- credential redaction (A4), SSRF hardening (A6), sandbox architecture (K3), and a dedicated security plugin (K3a) are all positive. However, several areas have insufficient security specification, missing threat models, or gaps that could allow data exfiltration, privilege escalation, or cross-agent contamination if not addressed before implementation.

**Critical gaps identified**: 7
**High-priority gaps identified**: 11
**Medium-priority gaps identified**: 9

---

## 1. Element-by-Element Security Assessment

### Element 03: Critical Fixes & Cleanup

**Risk Level: HIGH (currently) -> LOW (after completion)**

#### A4: Credential Redaction -- Assessment: ADEQUATE with caveats

The `SecretRef` wrapper type design in `02-tech-core.md` Section 12.4 is well-designed:
- Stores env var names, not values
- Custom `Debug` impl redacts secrets
- `#[serde(skip_serializing)]` prevents serialization leaks
- Backward-compatible deserialization handles plain strings with deprecation warning

**Gap 1 -- Incomplete field audit**: The tech spec lists 8 fields across 5 config structs, but the grep pattern `*password*`, `*secret*`, `*token*`, `*key*` may miss fields with non-standard names. The `MochatConfig.claw_token` was identified, but the audit should also search for fields containing `auth`, `credential`, `bearer`, `apikey` (no underscore), and `passphrase`.

**Recommendation**: Add a CI lint rule (clippy custom lint or `cargo deny` policy) that rejects any `pub` `String` field in config structs whose name matches a secret-pattern regex unless it has the `_env` suffix or is of type `SecretRef`.

#### A5: API Key Echo Suppression -- Assessment: ADEQUATE

`rpassword::read_password()` is the standard approach. No issues.

**Recommendation**: Verify that the `rpassword` version (5.0) does not have known vulnerabilities. The latest is 5.0.1 as of early 2025 -- confirm current status.

#### A6: SSRF Protection -- Assessment: PARTIALLY ADEQUATE

The `is_private_ip()` implementation in `02-tech-core.md` Section 12.6 covers:
- RFC 1918: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
- Loopback: 127.0.0.0/8
- Link-local: 169.254.0.0/16
- CGN: 100.64.0.0/10 (RFC 6598)
- Null: 0.0.0.0/8
- IPv6: loopback, link-local, ULA

**Gap 2 -- DNS rebinding not addressed**: The SSRF check validates the resolved IP, but does not protect against DNS rebinding attacks. An attacker could register a domain that initially resolves to a public IP (passes validation), then rebinds to 127.0.0.1 on a subsequent resolution. The tech spec does not mention re-resolving or pinning the resolved IP.

**Gap 3 -- Missing IPv4-mapped IPv6**: The check does not handle IPv4-mapped IPv6 addresses (e.g., `::ffff:127.0.0.1`). A request to `http://[::ffff:10.0.0.1]/` could bypass the IPv4-only check path.

**Gap 4 -- Cloud metadata endpoints not blocked**: The check does not block cloud metadata endpoints (`169.254.169.254` on AWS/GCP/Azure). While this falls under the link-local range, it should be explicitly called out as a test case. Additionally, Azure metadata uses `169.254.169.254:80` with a required `Metadata: true` header, but the IP alone should be blocked.

**Recommendations**:
1. Add DNS rebinding mitigation: resolve hostname to IP once, pin the IP for the request, then validate the pinned IP before connecting. Use `reqwest`'s `resolve()` method.
2. Add IPv4-mapped IPv6 handling: convert `::ffff:x.x.x.x` to IPv4 before checking.
3. Add explicit test for `169.254.169.254` (cloud metadata).
4. Block `[::]` and `0.0.0.0` explicitly as binding addresses.
5. The tech spec says "Apply to both `clawft-services` and `clawft-tools`" -- after B6 consolidation, ensure there is exactly ONE canonical `is_private_ip()` implementation.

#### Exit Criteria Additions Needed for Element 03:

- [ ] No credential field of type `String` exists in any config struct without `_env` suffix (CI lint)
- [ ] SSRF check blocks `::ffff:10.0.0.1` (IPv4-mapped IPv6)
- [ ] SSRF check blocks `169.254.169.254` (cloud metadata endpoint)
- [ ] DNS rebinding mitigation documented (even if deferred, the risk must be acknowledged)

---

### Element 04: Plugin & Skill System

**Risk Level: HIGH**

This element introduces the most significant new attack surface in the entire sprint. Plugins execute arbitrary code (native or WASM), can request network access, filesystem access, shell execution, and environment variable access.

#### C2: WASM Sandbox -- Assessment: PARTIALLY ADEQUATE

**Positive aspects**:
- wasmtime is well-audited and actively maintained by the Bytecode Alliance
- WIT component model provides typed FFI boundaries
- Size enforcement (300KB/120KB) limits complexity
- WASI capabilities model restricts filesystem/network access

**Gap 5 -- WASM sandbox escape via host functions**: The WIT interface definition in `02-tech-core.md` Section 14.3 exposes `http-request`, `read-file`, `write-file`, and `get-env` as host functions. These are the primary escape paths from the WASM sandbox:

1. `http-request`: A malicious plugin can exfiltrate data to any URL if the host does not enforce the `PluginPermissions.network` allowlist on this function. The tech spec does not show the host-side implementation that validates the URL against the permission manifest.

2. `read-file` / `write-file`: Must be restricted to the plugin's declared `PluginPermissions.filesystem` paths. The tech spec does not show path canonicalization or symlink resolution on these host functions.

3. `get-env`: Exposes environment variables to plugins. Since secrets are stored as env var values (resolved from `SecretRef`), a plugin requesting `get-env("OPENAI_API_KEY")` gets the raw secret. The `PluginPermissions.env_vars` allowlist exists in the manifest schema but the enforcement is not shown.

**Gap 6 -- No host-side permission enforcement implementation**: The `PluginPermissions` struct is defined, and the `PluginManifest` carries it, but there is no code or specification for how the host validates permissions when a plugin calls a host function. This is the single most critical security gap in the entire plan.

**Gap 7 -- Fuel metering not specified**: The tech spec mentions "CPU time limits via fuel metering" in the K3 sandbox architecture (Section 16.2) but does not show wasmtime fuel configuration in the C2 WASM host loader (Section 14.3). Without fuel metering, a malicious WASM plugin can perform infinite loops and DoS the host.

**Recommendations**:
1. Add explicit host-function permission checks for every WIT import:
   - `http-request`: Validate URL against `permissions.network` allowlist + SSRF check
   - `read-file` / `write-file`: Canonicalize path, verify it falls within `permissions.filesystem` paths, reject symlinks pointing outside
   - `get-env`: Only return values for env vars listed in `permissions.env_vars`
   - `log`: Rate-limit to prevent log flooding
2. Add wasmtime fuel metering: `config.consume_fuel(true)` with a per-invocation fuel budget (e.g., 1_000_000_000 fuel units ~ 1 second of CPU).
3. Add wasmtime memory limits: `store.limiter(|_| StoreLimits::new().memory_size(16 * 1024 * 1024))` (16MB per plugin).
4. Add a permission prompt UX: When a plugin requests permissions not previously approved, prompt the user for consent (like browser permission dialogs).

#### C3: Skill Loader -- Assessment: MEDIUM RISK

**Gap 8 -- Skill injection via untrusted SKILL.md**: Skills are loaded from multiple directories with precedence layering. A malicious skill in `~/.clawft/skills/` (managed directory) could override a bundled skill with a version that exfiltrates data via prompt injection.

**Recommendations**:
1. Skills installed from ClawHub should have signature verification (noted in `02-tech-pipeline.md` Section 16.4: "Verify signature (if present)" -- this should be REQUIRED, not optional).
2. Skills with `execution: shell` are particularly dangerous -- require explicit user approval for shell-execution skills.
3. Add integrity checking: hash the SKILL.md + implementation files and verify on load.

#### C4a: Autonomous Skill Creation -- Assessment: HIGH RISK

The agent autonomously generates skills including WASM compilation. This is a self-modifying code capability.

**Gap 9 -- No review gate on auto-generated skills**: The plan says "Agent detects repeated task patterns" and auto-generates skills. There is no specification for human review before the generated skill is activated.

**Recommendations**:
1. Auto-generated skills MUST be installed in a "pending" state and require explicit user approval before activation.
2. Auto-generated skills MUST have the minimum permission set (no shell, no network, filesystem limited to workspace).
3. Auto-generated WASM modules must pass the security plugin audit (K3a) before activation.

#### Exit Criteria Additions Needed for Element 04:

- [ ] Every WIT host function validates against `PluginPermissions` before executing
- [ ] WASM plugins have fuel metering enabled (configurable, default: 1B units)
- [ ] WASM plugins have memory limits (configurable, default: 16MB)
- [ ] `read-file` / `write-file` host functions canonicalize paths and reject symlinks outside allowed directories
- [ ] `http-request` host function applies SSRF check + network allowlist
- [ ] `get-env` host function only returns values for explicitly permitted env vars
- [ ] Auto-generated skills require user approval before activation
- [ ] Shell-execution skills require explicit user approval on install

---

### Element 05: Pipeline & LLM Reliability

**Risk Level: LOW**

D1 (parallel tool execution) introduces a concurrency concern where multiple tools writing to the same file could race. The tech spec correctly identifies this and proposes per-path advisory locks.

D8 (bounded bus) is a positive security improvement -- prevents OOM from unbounded message queues.

**No significant security gaps identified.**

---

### Element 06: Channel Enhancements

**Risk Level: MEDIUM**

#### OAuth2 Token Storage -- Assessment: NEEDS SPECIFICATION

The tech spec shows `OAuth2TokenManager` storing tokens in a `RwLock<Option<CachedToken>>` (in-memory only). This is acceptable for short-lived access tokens, but:

**Gap 10 -- Refresh token storage**: Refresh tokens are long-lived and must survive process restarts. The spec says they come from env vars (`refresh_token_env`), but OAuth2 flows generate new refresh tokens on each use (token rotation). There is no specification for persisting rotated refresh tokens.

**Gap 11 -- OAuth2 state parameter validation**: The OAuth2 authorization code flow requires a `state` parameter to prevent CSRF. The tech spec does not show state generation/validation in the OAuth2 flow.

**Recommendations**:
1. Specify refresh token persistence: encrypted file in `~/.clawft/tokens/` with filesystem permissions `0600`, encrypted with a key derived from a user-provided passphrase or system keyring.
2. Add explicit CSRF protection via `state` parameter in OAuth2 flows.
3. Add token revocation on shutdown or credential change.

#### Channel-Specific Security Concerns:

| Channel | Risk | Notes |
|---------|------|-------|
| Email (E2) | Medium | IMAP credentials, email content PII, attachment handling (malicious files) |
| WhatsApp (E3) | Low | Access token via env var, webhook verify token must be secret |
| Signal (E4) | Medium | `signal-cli` subprocess spawning -- command injection if phone number is not validated |
| Teams (E5b) | Medium | Azure AD multi-tenant app could be exploited for token theft if tenant_id is "common" |

**Gap 12 -- WhatsApp webhook verify_token in plaintext**: The `WhatsAppConfig` struct has `verify_token: String` as a plain string, not a `SecretRef`. This violates the A4 credential redaction pattern.

**Recommendations**:
1. Change `verify_token` to `SecretRef` type.
2. For Signal/iMessage subprocess channels, validate all user-supplied strings passed to subprocess arguments against injection patterns.
3. For the Email channel, sanitize attachment filenames to prevent path traversal when saving.
4. Recommend against `tenant_id = "common"` for Teams -- document that single-tenant is more secure.

#### Exit Criteria Additions Needed for Element 06:

- [ ] All channel config credential fields use `SecretRef` type (no plaintext secrets)
- [ ] OAuth2 flows include `state` parameter for CSRF protection
- [ ] Refresh token rotation is persisted securely (encrypted, 0600 permissions)
- [ ] Subprocess-based channels (Signal, iMessage) sanitize all arguments against command injection

---

### Element 07: Dev Tools & Applications

**Risk Level: MEDIUM-HIGH**

#### F1: Git Tool -- Assessment: ADEQUATE

Git operations via `git2` are sandboxed by libgit2's own security model. Write operations require confirmation. Path validation reuses existing file tool sanitization.

#### F3: tree-sitter -- Assessment: LOW RISK

Read-only code parsing. No execution. Minimal attack surface.

#### F4: Browser CDP -- Assessment: HIGH RISK

**Gap 13 -- JavaScript evaluation is dangerous**: The `BrowserOperation::Evaluate { script: String }` operation allows arbitrary JavaScript execution in the browser context. While the browser is a separate process, this enables:
- Exfiltration of page content (including credentials from login pages)
- Navigation to phishing pages
- Cookie theft from authenticated sessions

The `allowed_domains` restriction helps but is easily bypassed if the page includes iframes or redirects.

**Gap 14 -- Browser tool can access local file URLs**: The spec does not mention blocking `file://` URLs, which would give the browser (and thus the agent) access to the local filesystem via the browser's file reader.

**Recommendations**:
1. Block `file://`, `data:`, and `javascript:` URL schemes in the browser tool.
2. Add a Content Security Policy (CSP) header injection to prevent iframes loading from disallowed domains.
3. Consider making `Evaluate` a restricted operation that requires explicit user approval per invocation.
4. Log all browser navigation events for audit purposes.
5. Clear browser cookies/storage between sessions to prevent cross-session leakage.

#### F6: OAuth2 Helper -- Assessment: See Element 06 analysis

Same concerns apply. The OAuth2 helper is shared infrastructure.

#### F9: MCP Client -- Assessment: MEDIUM RISK

**Gap 15 -- Untrusted MCP servers**: Connecting to external MCP servers means clawft executes tool calls defined by those servers. A malicious MCP server could:
- Define tools with misleading names that trick the agent into calling them
- Return malicious content in tool results that influence the LLM's behavior (indirect prompt injection)
- Exploit the MCP protocol to probe clawft's internal state

**Gap 16 -- MCP stdio transport spawns arbitrary commands**: The `McpTransportConfig::Stdio` variant accepts a `command: String` and `args: Vec<String>`. If this config comes from user input (`weft mcp add`), there is no validation that the command is a legitimate MCP server binary.

**Recommendations**:
1. Add a trust model for MCP servers: new servers are "untrusted" by default. Tool calls from untrusted servers require user confirmation.
2. Validate stdio command paths against a configurable allowlist or require them to be full paths (no PATH resolution for untrusted commands).
3. Add MCP server sandboxing: stdio child processes should inherit minimal environment variables (not the full parent env with all secrets).

#### Exit Criteria Additions Needed for Element 07:

- [ ] Browser tool blocks `file://`, `data://`, `javascript:` URL schemes
- [ ] Browser tool clears state between sessions
- [ ] MCP stdio child processes do not inherit secret environment variables
- [ ] External MCP server tools are tagged as "untrusted" in the tool registry

---

### Element 08: Memory & Workspace

**Risk Level: LOW-MEDIUM**

#### H1: Per-Agent Workspace -- Assessment: ADEQUATE

Agent isolation via separate directories is a solid approach. The `cross_agent_access: bool` flag defaults to `false`, which is the secure default.

**Minor concern**: The specification says "No cross-talk unless explicitly enabled via shared memory namespace." The implementation must ensure that agent A cannot read agent B's files even if it knows the path (i.e., path validation in file tools must respect agent workspace boundaries).

**Recommendation**: File tools operating in a multi-agent context must validate that the resolved absolute path falls within the current agent's workspace. This ties into the K3 sandbox filesystem policy.

#### H2: RVF Phase 3 -- Assessment: LOW RISK

Vector memory is data storage, not execution. The main risk is data integrity (addressed by A2 stable hash fix).

**Minor concern**: `weft memory import` could introduce tampered data. The RVF import should validate segment checksums.

---

### Element 09: Multi-Agent Routing

**Risk Level: MEDIUM-HIGH**

#### L1/L2: Agent Routing & Isolation -- Assessment: PARTIALLY ADEQUATE

The routing table design is reasonable. Per-agent session isolation is correctly specified.

**Gap 17 -- Cross-agent data access via message bus**: The `SwarmCoordinator` (L3) enables inter-agent communication via the `MessageBus`. If agents share a single bus, a compromised or malicious agent could read messages intended for other agents.

**Gap 18 -- Agent routing match criteria not validated**: The `match_criteria: HashMap<String, String>` accepts arbitrary key-value pairs. There is no validation that the match criteria are well-formed. A malicious config could create catch-all rules that route all messages to a single agent, bypassing intended isolation.

**Recommendations**:
1. Multi-agent bus messages should be tagged with `(sender_agent_id, recipient_agent_id)` and the bus should enforce that agents can only read messages addressed to them.
2. Add validation for routing match criteria: reject empty matchers, require at least one specific field, and warn on overly broad matchers.
3. The `auto_create = true` option should be restricted -- auto-created agents must inherit a minimal permission profile, not the default agent's full permissions.

#### M1: FlowDelegator -- Assessment: MEDIUM RISK

**Gap 19 -- Subprocess environment exposure**: The `FlowDelegator` spawns `claude` as a subprocess. By default, `tokio::process::Command` inherits the parent's environment, which contains all secret env vars (API keys for every configured provider/channel). The spawned process should not have access to secrets it does not need.

**Gap 20 -- Temp file for MCP config**: The `build_mcp_config_json()` method writes a config file to `std::env::temp_dir()`. On shared systems, `/tmp` is world-readable. The temp file contains the path to the `weft` binary, which could be replaced with a malicious binary in a TOCTOU race.

**Recommendations**:
1. Explicitly set the child process environment: only pass env vars that Claude CLI needs (PATH, HOME, ANTHROPIC_API_KEY), not the full parent environment.
2. Use `tempfile::NamedTempFile` with secure permissions (0600) instead of writing to a predictable path in `/tmp`.
3. Add a timeout kill: if the subprocess exceeds the timeout, `child.kill()` must be called (the current code uses `timeout()` which only races against `wait_with_output()` -- if the child ignores stdin closure, it may run indefinitely).

#### M5: Bidirectional MCP Bridge -- Assessment: MEDIUM RISK

The bidirectional bridge means Claude Code can call back into clawft's tools. This creates a recursive delegation loop risk:
- clawft delegates to Claude Code
- Claude Code calls back into clawft's delegate tool
- clawft delegates back to Claude Code (infinite loop)

**Recommendation**: Add a delegation depth counter. Reject delegation requests that exceed a configurable maximum depth (default: 3).

#### Exit Criteria Additions Needed for Element 09:

- [ ] Bus messages are tagged with agent IDs; agents cannot read other agents' messages
- [ ] FlowDelegator child process receives a minimal, explicitly-constructed environment
- [ ] MCP config temp files use secure permissions (0600) via `tempfile` crate
- [ ] Delegation depth limit enforced (default: 3, configurable)
- [ ] Auto-created agents inherit minimal permission profile

---

### Element 10: Deployment & Community

**Risk Level: MEDIUM-HIGH**

#### K3: Per-Agent Sandbox -- Assessment: PARTIALLY ADEQUATE

The defense-in-depth layers (WASM + seccomp + landlock + network policy) are architecturally sound.

**Gap 21 -- Default sandbox is `None`**: The `SandboxType` default is `None` (no sandbox). This means agents are fully unsandboxed unless the operator explicitly configures sandboxing. For a security-critical feature, the default should be secure.

**Gap 22 -- seccomp/landlock Linux-only**: The sandbox relies on Linux kernel features (seccomp-bpf, landlock). On macOS (common development environment), there is no equivalent specified. The WASM sandbox layer works cross-platform, but native tools (shell, file, git) are unsandboxed on non-Linux systems.

**Recommendations**:
1. Change the default `SandboxType` to `Wasm` for WASM plugins and `OsSandbox` for native execution on Linux. The `None` option should require explicit opt-in with a security warning.
2. For macOS, use `sandbox-exec` (App Sandbox profiles) or at minimum `chroot`-equivalent directory restrictions.
3. Add a startup warning when sandboxing is unavailable on the current platform.

#### K3a: Security Plugin (50+ Checks) -- Assessment: PARTIALLY ADEQUATE

The `AuditCategory` enum covers:
- PromptInjection
- DataExfiltration
- UnsafeShellCommand
- CredentialLeak
- ExcessivePermission
- UnvalidatedInput

**Gap 23 -- Missing audit categories**: The following categories are not covered:
1. **Supply chain** -- dependency analysis of plugin manifests (do declared dependencies have known CVEs?)
2. **Denial of Service** -- resource consumption patterns (infinite loops, unbounded allocations, fork bombs)
3. **Information disclosure** -- skills that return excessive context in error messages
4. **Cross-agent contamination** -- skills that access paths outside their agent's workspace
5. **Indirect prompt injection** -- content from untrusted sources (web scraping, emails) that could manipulate the agent
6. **Token/cost abuse** -- skills that trigger excessive LLM calls

**Recommendations**: Expand the `AuditCategory` enum to include:
```
SupplyChainRisk
DenialOfService
InformationDisclosure
CrossAgentAccess
IndirectPromptInjection
CostAbuse
```

#### K4: ClawHub Registry -- Assessment: MEDIUM RISK

**Gap 24 -- No mandatory code signing**: The spec says "Verify signature (if present)" -- signatures should be mandatory for all published skills, not optional.

**Recommendations**:
1. Require skill authors to sign their packages. ClawHub should reject unsigned uploads.
2. Add a `--allow-unsigned` flag for local development, but log a warning.
3. Implement a content hash manifest (like `npm`'s `integrity` field or `cargo`'s `Cargo.lock` checksums) for every installed skill.

#### Docker Image (K2) -- Assessment: ADEQUATE

The Dockerfile uses multi-stage builds, non-root user, minimal base image. This is well-designed.

**Minor recommendation**: Add `--no-install-recommends` (already present) and consider using distroless base image for even smaller attack surface.

#### Exit Criteria Additions Needed for Element 10:

- [ ] Default sandbox type is not `None` (secure by default)
- [ ] Security plugin covers at least 8 audit categories (add supply chain, DoS, prompt injection)
- [ ] ClawHub requires skill signatures for publication
- [ ] Platform-specific sandbox fallback documented for non-Linux systems

---

## 2. Supply Chain Analysis

### New Dependencies Assessment

| Dependency | Version | Purpose | Security Assessment | Risk |
|-----------|---------|---------|-------------------|------|
| `wasmtime` | 27 | WASM runtime | Bytecode Alliance, actively audited, WASM Alliance security model | LOW |
| `wit-bindgen` | 0.36 | WASM type generation | Same team as wasmtime | LOW |
| `tree-sitter` | 0.24 | AST parsing | Widely used, GitHub-maintained, C FFI (audit C code) | LOW-MEDIUM |
| `chromiumoxide` | 0.7 | Browser CDP | Smaller community, depends on Chrome binary availability | MEDIUM |
| `git2` | 0.19 | Git operations | libgit2 FFI, well-audited | LOW |
| `oauth2` | 4.4 | OAuth2 flows | Actively maintained, standard implementation | LOW |
| `lettre` | 0.11 | SMTP email | Rust-native, well-maintained | LOW |
| `imap` | 3.0 | IMAP email | Smaller project, review TLS handling | MEDIUM |
| `notify` | 7.0 | FS watching | Widely used, maintained | LOW |
| `serde_yaml` | 0.9 | YAML parsing | Note: `serde_yaml` was archived; confirm status or use `serde_yml` | MEDIUM |
| `fnv` | 1.0 | Stable hash | Minimal, zero-dep, stable | LOW |
| `rpassword` | 5.0 | Password input | Minimal, established | LOW |
| `temp_env` | 0.3 | Test env vars | Dev dependency only | LOW |
| `instant-distance` | 0.6 | HNSW search | Smaller project, verify correctness | LOW-MEDIUM |
| `matrix-sdk` | N/A | Matrix protocol | Mozilla-maintained, well-audited | LOW |
| `which` | N/A | Binary detection | Minimal utility | LOW |

**Action items**:
1. Verify `serde_yaml` 0.9 is actively maintained. If archived, switch to `serde_yml` or `yaml-rust2`.
2. Run `cargo audit` on all new dependencies before integration.
3. `chromiumoxide` depends on a locally-installed Chrome/Chromium binary -- this binary must be kept updated for security patches.
4. `tree-sitter` includes C language parsers via FFI -- run these through `cargo-fuzz` for memory safety.

---

## 3. Risk Register Security Analysis

### Existing Risk Scores Assessment

| Risk | Current Score | My Assessment | Reasoning |
|------|--------------|---------------|-----------|
| Session key migration breaks existing sessions | 6 | 6 -- Appropriate | Data integrity risk, not security |
| Credential redaction misses a field | 6 | **8** -- Should be higher | Impact is credential exposure; likelihood is medium given the manual grep approach |
| WASM plugin sandbox escape | 5 vs 8 | **8** -- Use higher score | Low likelihood due to wasmtime, but critical impact |
| Skill hot-reload race condition | 4 | 4 -- Appropriate | Availability concern, not confidentiality |

### Missing Security Risks

The following risks should be added to the risk register:

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Plugin data exfiltration via host functions | Medium | Critical | **8** | Enforce permission allowlists on all WIT host function calls; rate-limit network access; audit log all outbound requests from plugins |
| DNS rebinding bypasses SSRF protection | Low | High | **4** | Pin resolved IPs before connecting; do not re-resolve hostnames during a request |
| Cross-agent message bus eavesdropping | Medium | High | **6** | Tag bus messages with agent IDs; enforce agent-scoped message delivery |
| FlowDelegator environment variable leakage | Medium | Critical | **8** | Construct minimal child process environment; never inherit full parent env |
| Malicious MCP server tool injection | Medium | High | **6** | Require user approval for untrusted MCP server tools; sandbox MCP stdio child processes |
| Auto-generated skill privilege escalation | Low | Critical | **5** | Require human approval for auto-generated skills; enforce minimal permissions |
| OAuth2 refresh token theft from disk | Medium | High | **6** | Encrypt token storage; use OS keyring where available; restrict file permissions to 0600 |
| Browser tool local file access via file:// URLs | Low | High | **4** | Block file://, data://, javascript:// URL schemes in browser navigation |
| ClawHub supply chain attack (malicious skill) | Medium | Critical | **8** | Mandatory code signing; content hash verification; security plugin audit on install |
| Recursive delegation loop (infinite subprocess spawn) | Low | Medium | **3** | Enforce delegation depth limit (default: 3) |
| IPv4-mapped IPv6 SSRF bypass | Low | High | **4** | Convert ::ffff:x.x.x.x to IPv4 before SSRF check |

---

## 4. Missing Security Requirements

### Element 03 (Critical Fixes)
1. CI lint for credential field detection (no new plaintext secret fields)
2. DNS rebinding mitigation specification
3. IPv4-mapped IPv6 handling in SSRF check
4. Cloud metadata endpoint (169.254.169.254) explicit test case

### Element 04 (Plugin System)
1. Host-function permission enforcement implementation (CRITICAL -- most important missing piece)
2. WASM fuel metering configuration
3. WASM memory limits per plugin
4. Path canonicalization and symlink rejection in file host functions
5. Skill signature verification on install (mandatory, not optional)
6. Human review gate for auto-generated skills

### Element 06 (Channels)
1. Refresh token rotation persistence with encryption
2. OAuth2 CSRF protection via state parameter
3. Subprocess argument sanitization for Signal/iMessage
4. `WhatsAppConfig.verify_token` must use `SecretRef`

### Element 07 (Dev Tools)
1. Browser URL scheme blocklist (file://, data://, javascript://)
2. Browser session state clearing
3. MCP stdio child process environment sanitization
4. MCP server trust model (untrusted by default)

### Element 09 (Multi-Agent)
1. Agent-scoped message bus delivery enforcement
2. Delegation depth limit
3. FlowDelegator minimal environment construction
4. Temp file secure creation (tempfile crate, 0600)
5. Routing rule validation (reject empty/catch-all matchers)

### Element 10 (Deployment)
1. Secure-by-default sandbox (not `None`)
2. Expanded security audit categories (supply chain, DoS, prompt injection)
3. Mandatory skill signing for ClawHub publication
4. macOS sandbox fallback specification

---

## 5. Missing Security Tests

| Test | Element | Priority |
|------|---------|----------|
| WASM plugin attempting to read file outside allowed path -> DENIED | 04 | P0 |
| WASM plugin attempting HTTP to non-allowed domain -> DENIED | 04 | P0 |
| WASM plugin attempting to read non-permitted env var -> returns None | 04 | P0 |
| WASM plugin exceeding fuel limit -> terminated | 04 | P0 |
| WASM plugin exceeding memory limit -> OOM error | 04 | P1 |
| SSRF check blocks ::ffff:127.0.0.1 | 03 | P0 |
| SSRF check blocks 169.254.169.254 | 03 | P0 |
| Agent A cannot read agent B's session files via file tool | 09 | P0 |
| Agent A cannot read agent B's bus messages | 09 | P0 |
| FlowDelegator child process env does not contain FEISHU_APP_SECRET | 09 | P0 |
| Browser tool rejects file:// URL navigation | 07 | P1 |
| MCP stdio child process env does not contain all parent secrets | 07 | P1 |
| OAuth2 flow rejects response without matching state parameter | 06 | P1 |
| Skill with execution: shell requires user confirmation | 04 | P1 |
| Auto-generated skill is not activated without user approval | 04 | P1 |
| Delegation depth exceeding limit returns error | 09 | P1 |
| Unsigned ClawHub skill triggers warning on install | 10 | P2 |
| Security plugin detects prompt injection patterns in skill text | 10 | P1 |

---

## 6. Security Readiness Assessment

| Element | Readiness | Rationale |
|---------|-----------|-----------|
| **03: Critical Fixes** | **CONDITIONAL GO** | Core security fixes are well-specified. Must add IPv4-mapped IPv6 SSRF test and CI lint for credential fields before merging. DNS rebinding can be documented as known limitation for now. |
| **04: Plugin System** | **NO-GO** | Cannot proceed without specifying host-function permission enforcement for WASM plugins. This is the primary security boundary for the plugin system. Fuel metering and memory limits must also be specified. After these are addressed, the element can be re-evaluated. |
| **05: Pipeline** | **GO** | No significant security concerns. Parallel tool execution with per-path locking is adequate. |
| **06: Channels** | **CONDITIONAL GO** | Must fix WhatsApp verify_token to use SecretRef. Must specify OAuth2 state parameter validation. Refresh token persistence can be deferred to a follow-up but must be documented as a known gap. |
| **07: Dev Tools** | **CONDITIONAL GO** | Must block file:// URLs in browser tool. Must sanitize MCP stdio child process environment. Tree-sitter and git tools are acceptable. |
| **08: Memory** | **GO** | Low security risk. Per-agent workspace isolation is well-designed. Minor recommendation: validate RVF import segment checksums. |
| **09: Multi-Agent** | **CONDITIONAL GO** | Must specify: (1) agent-scoped bus message delivery, (2) FlowDelegator minimal environment, (3) delegation depth limit. Without these, cross-agent contamination and secret leakage are possible. |
| **10: Deployment** | **CONDITIONAL GO** | Must change default sandbox type from `None` to a secure default. Must expand security audit categories. Must require skill signing for ClawHub. Docker image is well-designed. |

---

## 7. Priority Action Items

### P0 (Must fix before implementation begins)

1. **Specify WASM host-function permission enforcement** (Element 04, Gap 5/6) -- This is the single most important security deliverable. Without it, the plugin system has no meaningful security boundary.

2. **Add WASM fuel metering and memory limits** (Element 04, Gap 7) -- DoS prevention for the WASM runtime.

3. **Fix IPv4-mapped IPv6 SSRF bypass** (Element 03, Gap 3) -- Trivial fix, high impact if missed.

4. **Sanitize FlowDelegator child process environment** (Element 09, Gap 19) -- Prevents leaking all provider API keys to the Claude subprocess.

### P1 (Must fix before the element ships)

5. Block file:// URLs in browser tool (Element 07, Gap 14)
6. Add agent-scoped bus message delivery (Element 09, Gap 17)
7. Fix WhatsApp verify_token to SecretRef (Element 06, Gap 12)
8. Add delegation depth limit (Element 09, M5)
9. Change default sandbox to secure default (Element 10, Gap 21)
10. Add DNS rebinding documentation as known limitation (Element 03, Gap 2)

### P2 (Should fix during the sprint)

11. OAuth2 state parameter validation (Element 06, Gap 11)
12. Expand security audit categories (Element 10, Gap 23)
13. Require ClawHub skill signatures (Element 10, Gap 24)
14. Add CI lint for credential field detection (Element 03, Gap 1)
15. Secure temp file creation in FlowDelegator (Element 09, Gap 20)

---

## 8. Conclusion

The sprint plan demonstrates genuine security awareness, with dedicated workstream items for credential handling, SSRF protection, sandboxing, and a security audit plugin. The architectural decisions (env-var-name pattern for secrets, defense-in-depth sandboxing, plugin permission manifest) are sound in principle.

The primary concern is the gap between the permission *declaration* model (which is well-designed in the plugin manifest schema) and the permission *enforcement* model (which has no implementation specification). This is the critical path: the plugin system's security guarantees are only as strong as the host-side enforcement of declared permissions.

The secondary concern is the multi-agent routing element's potential for cross-agent data leakage via the shared message bus and the FlowDelegator's unconstrained environment inheritance.

Addressing the P0 action items before implementation begins will bring the overall security posture from "concerning" to "acceptable." Completing the P1 items before each element ships will bring it to "solid."
