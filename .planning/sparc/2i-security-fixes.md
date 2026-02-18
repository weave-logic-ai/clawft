# SPARC Implementation Plan: Stream 2I - Security Fixes (P0)

**Timeline**: Must complete before Phase 3
**Owned Crates**: `clawft-tools` (security_policy, shell_tool, spawn_tool, web_fetch), `clawft-types` (config)
**Dependencies**: Stream 2F (security primitives), Stream 2H (tool implementations)
**Review**: `.planning/development_notes/review_security.md`
**Research**: `.planning/development_notes/research-ssrf-protection-crates.md`

---

## 1. Agent Instructions

### Source Files to Read (Current State)
```
clawft/crates/clawft-tools/src/shell_tool.rs       # SEC-1: ShellExecTool with bypassable denylist
clawft/crates/clawft-tools/src/spawn_tool.rs        # SEC-2: SpawnTool with zero validation
clawft/crates/clawft-tools/src/web_fetch.rs         # SEC-3: WebFetchTool with scheme-only check
clawft/crates/clawft-tools/src/lib.rs               # register_all — tool construction + registration
clawft/crates/clawft-types/src/config.rs            # ToolsConfig, ExecToolConfig — existing config types
clawft/crates/clawft-core/src/security.rs           # Existing security primitives (validate_session_id, etc.)
clawft/crates/clawft-core/src/bootstrap.rs          # AppContext — tool registration path
clawft/crates/clawft-cli/src/commands/agent.rs      # Tool registration in agent command
clawft/crates/clawft-cli/src/commands/gateway.rs    # Tool registration in gateway command
```

### Planning Documents (MUST READ)
```
.planning/development_notes/review_security.md      # Security findings SEC-1/2/3
.planning/development_notes/research-ssrf-protection-crates.md  # SSRF library research
.planning/sparc/2f-security-testing.md               # Prior security stream (security.rs)
```

### Documentation to Update
```
clawft/docs/guides/configuration.md                 # Add security policy config section
clawft/docs/reference/tools.md                      # Update shell/spawn/web_fetch security notes
clawft/docs/guides/tool-calls.md                    # Update security model section
clawft/docs/architecture/overview.md                # Update security boundaries section
```

---

## 2. Specification

### SEC-1: Shell Tool — Command Allowlist

**Current state**: `shell_tool.rs` uses a denylist of 11 hardcoded patterns (lines 22-34). Trivially bypassable — does not cover `curl`, `wget`, `python3 -c`, `base64 | sh`, `nc`, flag reordering, or tab-separated patterns.

**Required changes**:
- Create a shared `security_policy.rs` module in `clawft-tools/src/`
- Define `CommandPolicy` struct with:
  - `mode: PolicyMode` — enum `Allowlist` (default) | `Denylist`
  - `allowlist: Vec<String>` — permitted executable names (default safe set)
  - `denylist: Vec<String>` — blocked patterns (current 11 + expanded set)
- Default safe allowlist: `["echo", "cat", "ls", "pwd", "head", "tail", "wc", "grep", "find", "sort", "uniq", "diff", "date", "env", "true", "false", "test"]`
- `CommandPolicy::validate(command: &str) -> Result<(), PolicyError>`
  - In allowlist mode: extract the first token from the command, resolve through `PATH` if needed, check against allowlist
  - In denylist mode: run existing pattern matching (kept as fallback/defense-in-depth)
- Wire `CommandPolicy` into `ShellExecTool` constructor — pass via `register_all`
- Keep existing `DANGEROUS_PATTERNS` as defense-in-depth layer that runs regardless of mode

### SEC-2: Spawn Tool — Command Allowlist

**Current state**: `spawn_tool.rs` (line 78) accepts arbitrary `command` and `args` with zero filtering.

**Required changes**:
- Share the same `CommandPolicy` from SEC-1
- Wire `CommandPolicy` into `SpawnTool` constructor
- `SpawnTool::execute` calls `policy.validate(command)` before spawning
- Default allowlist for spawn: same as shell, plus `["node", "python3", "cargo", "rustc"]` (operator-configurable)
- When in allowlist mode, the `command` field must match an entry exactly (no path traversal, no absolute paths unless the absolute path itself is on the allowlist)

### SEC-3: SSRF Protection for WebFetchTool

**Current state**: `web_fetch.rs` (line 68) validates only `http://` or `https://` scheme prefix. No host/IP validation.

**Required changes**:
- Create `url_safety.rs` module in `clawft-tools/src/`
- Implement `validate_url(url: &str) -> Result<(), UrlSafetyError>`:
  1. Parse URL with `url::Url`
  2. Extract hostname
  3. Reject known metadata hostnames: `169.254.169.254`, `metadata.google.internal`, `metadata.internal`
  4. Resolve hostname to IP addresses using `hickory-resolver`
  5. Reject all resolved IPs in blocked ranges:
     - Private: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
     - Loopback: `127.0.0.0/8`, `::1`
     - Link-local: `169.254.0.0/16`, `fe80::/10`
     - IPv4-mapped IPv6: `::ffff:0:0/96` mapped to private ranges
  6. Support configurable domain allowlist (bypass all checks for trusted domains)
  7. Support configurable domain blocklist (additional blocked domains)
- Wire into `WebFetchTool::execute` before the HTTP call
- Re-validate redirect targets (if platform HTTP client follows redirects, validate final URL)
- Make validation configurable: `UrlPolicy` struct with enable/disable, custom allow/block lists

**Rust dependencies to add to `clawft-tools/Cargo.toml`**:
```toml
hickory-resolver = "0.24"
url = "2"
ipnet = "2"
```

**Rationale for build-from-scratch** (vs dedicated SSRF crate): The available SSRF-specific crates (`agent-fetch` at 49 downloads, `url-preview` at 7K downloads) are too new and untested for security-critical code. Building with `hickory-resolver` (34M downloads, 5K stars) + `ipnet` (311M downloads) + `std::net` provides battle-tested components with full control over the validation logic.

---

## 3. Pseudocode

### 3.1 security_policy.rs

```
enum PolicyMode { Allowlist, Denylist }

struct CommandPolicy {
    mode: PolicyMode,
    allowlist: HashSet<String>,
    denylist: Vec<String>,           // kept for defense-in-depth
    dangerous_patterns: Vec<String>, // always checked regardless of mode
}

impl CommandPolicy {
    fn safe_defaults() -> Self {
        // mode: Allowlist
        // allowlist: [echo, cat, ls, pwd, head, tail, wc, grep, find, sort, uniq, diff, date, env, true, false, test]
        // dangerous_patterns: existing 11 patterns from shell_tool.rs
    }

    fn from_config(config: &CommandPolicyConfig) -> Self {
        // merge defaults with config overrides
    }

    fn validate(&self, command: &str) -> Result<(), PolicyError> {
        // Step 1: Always check dangerous_patterns (defense-in-depth)
        extract_dangerous_check(command)?

        // Step 2: Mode-specific check
        match self.mode {
            Allowlist => {
                let executable = extract_first_token(command)
                    // strip path prefix: "/usr/bin/ls" -> "ls"
                    // reject if contains path separators and base is not on allowlist
                if !self.allowlist.contains(executable) {
                    return Err(PolicyError::NotAllowed(executable))
                }
            }
            Denylist => {
                for pattern in &self.denylist {
                    if command.to_lowercase().contains(pattern) {
                        return Err(PolicyError::Blocked(pattern))
                    }
                }
            }
        }
    }

    fn extract_first_token(command: &str) -> &str {
        // handle: "ls -la" -> "ls"
        // handle: "/usr/bin/ls" -> "ls"
        // handle: "  ls" -> "ls" (trim)
        // handle: pipes: "echo foo | cat" -> "echo" (first command)
    }
}
```

### 3.2 url_safety.rs

```
struct UrlPolicy {
    enabled: bool,              // default: true
    allow_private: bool,        // default: false
    allowed_domains: HashSet<String>,  // bypass validation for these
    blocked_domains: HashSet<String>,  // additional blocks
}

const METADATA_HOSTS: [&str] = [
    "169.254.169.254",
    "metadata.google.internal",
    "metadata.internal",
]

const BLOCKED_NETS_V4: [Ipv4Net] = [
    "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16",  // RFC 1918
    "127.0.0.0/8",                                       // loopback
    "169.254.0.0/16",                                     // link-local
    "0.0.0.0/8",                                          // "this" network
]

const BLOCKED_NETS_V6: [Ipv6Net] = [
    "::1/128",                                            // loopback
    "fe80::/10",                                          // link-local
    "fc00::/7",                                           // unique-local
]

async fn validate_url(url: &str, policy: &UrlPolicy) -> Result<(), UrlSafetyError> {
    if !policy.enabled { return Ok(()) }

    let parsed = Url::parse(url)?
    let host = parsed.host_str()?

    // Check domain lists
    if policy.allowed_domains.contains(host) { return Ok(()) }
    if policy.blocked_domains.contains(host) { return Err(BlockedDomain) }
    if METADATA_HOSTS.contains(host) { return Err(MetadataEndpoint) }

    // Resolve hostname to IP
    let resolver = TokioAsyncResolver::tokio_from_system_conf()?
    let ips = resolver.lookup_ip(host).await?

    // Check every resolved IP
    for ip in ips {
        if is_blocked_ip(ip) {
            return Err(PrivateIp(ip))
        }
    }

    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        V4(v4) => BLOCKED_NETS_V4.any(|net| net.contains(v4)),
        V6(v6) => {
            BLOCKED_NETS_V6.any(|net| net.contains(v6))
            || is_blocked_mapped_v4(v6)  // check ::ffff:x.x.x.x
        }
    }
}
```

### 3.3 Config Types (clawft-types)

```
// Add to ToolsConfig:
struct ToolsConfig {
    ... existing fields ...
    pub command_policy: CommandPolicyConfig,
    pub url_policy: UrlPolicyConfig,
}

struct CommandPolicyConfig {
    mode: String,                  // "allowlist" (default) | "denylist"
    allowlist: Option<Vec<String>>, // overrides defaults when present
    denylist: Option<Vec<String>>,  // overrides defaults when present
}

struct UrlPolicyConfig {
    enabled: bool,                  // default: true
    allow_private: bool,            // default: false
    allowed_domains: Vec<String>,   // empty by default
    blocked_domains: Vec<String>,   // empty by default
}
```

---

## 4. Architecture

### Module Structure
```
clawft-tools/
├── src/
│   ├── security_policy.rs    # NEW: CommandPolicy, PolicyMode, PolicyError
│   ├── url_safety.rs         # NEW: UrlPolicy, validate_url, is_blocked_ip
│   ├── shell_tool.rs         # MODIFIED: accept CommandPolicy, validate before sh -c
│   ├── spawn_tool.rs         # MODIFIED: accept CommandPolicy, validate before spawn
│   ├── web_fetch.rs          # MODIFIED: accept UrlPolicy, validate_url before fetch
│   ├── lib.rs                # MODIFIED: register_all accepts policy configs
│   └── ...

clawft-types/
├── src/
│   └── config.rs             # MODIFIED: add CommandPolicyConfig, UrlPolicyConfig to ToolsConfig

clawft-cli/
├── src/commands/
│   ├── agent.rs              # MODIFIED: pass policies from config to register_all
│   └── gateway.rs            # MODIFIED: same
```

### Data Flow

```
Config -> ToolsConfig -> CommandPolicyConfig -> CommandPolicy
                      -> UrlPolicyConfig     -> UrlPolicy

register_all(registry, platform, workspace, command_policy, url_policy)
  -> ShellExecTool::new(workspace, command_policy.clone())
  -> SpawnTool::new(platform, workspace, command_policy.clone())
  -> WebFetchTool::new(platform, url_policy.clone())

ShellExecTool::execute(args)
  -> command_policy.validate(command)?   # NEW: before sh -c
  -> is_dangerous(command)?              # KEPT: defense-in-depth
  -> Command::new("sh").arg("-c").arg(command)

SpawnTool::execute(args)
  -> command_policy.validate(command)?   # NEW: before spawn
  -> spawner.run(command, args, ...)

WebFetchTool::execute(args)
  -> validate_url(url, &url_policy)?     # NEW: before HTTP call
  -> platform.http().request(...)
```

### Dependency Changes

```toml
# clawft-tools/Cargo.toml — add:
hickory-resolver = { version = "0.24", features = ["tokio-runtime"] }
url = "2"
ipnet = "2"
```

---

## 5. Refinement (Implementation Steps)

### Step 1: Config Types
**File**: `clawft-types/src/config.rs`
- Add `CommandPolicyConfig` struct with `mode`, `allowlist`, `denylist` fields
- Add `UrlPolicyConfig` struct with `enabled`, `allow_private`, `allowed_domains`, `blocked_domains` fields
- Add both to `ToolsConfig` with `#[serde(default)]`
- All fields have safe defaults (allowlist mode, standard safe list, SSRF protection enabled)
- Tests: deserialize from JSON, verify defaults

### Step 2: CommandPolicy Module
**File**: `clawft-tools/src/security_policy.rs`
- `PolicyMode` enum: `Allowlist`, `Denylist`
- `PolicyError` enum: `NotAllowed(String)`, `Blocked(String)`, `DangerousPattern(String)`
- `CommandPolicy` struct with `safe_defaults()`, `from_config()`, `validate()`
- `extract_first_token()` helper — handles paths, pipes, leading whitespace
- Move `DANGEROUS_PATTERNS` from `shell_tool.rs` here (keep as defense-in-depth)
- Tests: 15+ tests covering:
  - Allowlist permits safe commands
  - Allowlist rejects unlisted commands (`curl`, `wget`, `nc`, `python3`)
  - Denylist mode works with existing patterns
  - `extract_first_token` handles edge cases (paths, pipes, tabs, quotes)
  - Dangerous patterns always checked regardless of mode
  - Bypass resistance: `sudo\t`, flag reordering, path traversal (`/usr/bin/curl`)
  - Config deserialization and merging

### Step 3: URL Safety Module
**File**: `clawft-tools/src/url_safety.rs`
- `UrlPolicy` struct with `from_config()`
- `UrlSafetyError` enum: `InvalidUrl`, `MetadataEndpoint`, `PrivateIp`, `BlockedDomain`, `ResolutionFailed`
- `validate_url()` async function
- `is_blocked_ip()` — checks IPv4 private/loopback/link-local, IPv6 loopback/link-local/unique-local, IPv4-mapped IPv6
- Constant arrays for blocked networks using `ipnet`
- Tests: 20+ tests covering:
  - Public URLs pass (`https://example.com`)
  - Private IPs blocked (10.x, 172.16.x, 192.168.x)
  - Loopback blocked (127.0.0.1, ::1)
  - Link-local blocked (169.254.x, fe80::)
  - Metadata endpoints blocked (169.254.169.254, metadata.google.internal)
  - IPv4-mapped IPv6 blocked (::ffff:10.0.0.1)
  - Allowed domains bypass checks
  - Blocked domains rejected
  - Policy disabled passes everything
  - Invalid URL format rejected
  - ftp:// scheme rejected

### Step 4: Wire into Shell Tool
**File**: `clawft-tools/src/shell_tool.rs`
- Add `policy: CommandPolicy` field to `ShellExecTool`
- Update constructors: `new(workspace, policy)`, `with_max_timeout(workspace, max_timeout, policy)`
- In `execute()`: call `self.policy.validate(command)?` before the `is_dangerous()` check
- Keep `is_dangerous()` as second layer of defense
- Convert `PolicyError` to `ToolError::PermissionDenied`
- Update existing tests, add new tests for policy integration

### Step 5: Wire into Spawn Tool
**File**: `clawft-tools/src/spawn_tool.rs`
- Add `policy: CommandPolicy` field to `SpawnTool`
- Update constructor: `new(platform, workspace, policy)`
- In `execute()`: call `self.policy.validate(command)?` before concurrency check
- Convert `PolicyError` to `ToolError::PermissionDenied`
- Update existing tests, add new tests for policy integration

### Step 6: Wire into Web Fetch Tool
**File**: `clawft-tools/src/web_fetch.rs`
- Add `url_policy: UrlPolicy` field to `WebFetchTool`
- Update constructor: `new(platform, url_policy)`
- In `execute()`: call `validate_url(url, &self.url_policy).await?` after scheme check, before HTTP call
- Convert `UrlSafetyError` to `ToolError::PermissionDenied`
- Update existing tests, add SSRF-specific tests

### Step 7: Update register_all and CLI
**File**: `clawft-tools/src/lib.rs`
- Update `register_all` signature to accept `CommandPolicy` and `UrlPolicy`
- Pass policies to tool constructors

**Files**: `clawft-cli/src/commands/agent.rs`, `gateway.rs`
- Construct `CommandPolicy::from_config(&config.tools.command_policy)`
- Construct `UrlPolicy::from_config(&config.tools.url_policy)`
- Pass to `register_all`

### Step 8: Update Documentation
**Files**:
- `clawft/docs/guides/configuration.md` — Add "Security Policy" section with:
  - Command policy config (allowlist/denylist mode, custom lists)
  - URL policy config (SSRF protection, domain lists)
  - Example JSON configurations for both
  - Safe defaults explanation
- `clawft/docs/reference/tools.md` — Update `exec_shell`, `spawn`, `web_fetch` entries:
  - Document allowlist behavior
  - Document SSRF protection
  - Note which commands are in the default allowlist
- `clawft/docs/guides/tool-calls.md` — Update security model section:
  - Add command policy description
  - Add URL validation description
- `clawft/docs/architecture/overview.md` — Update security boundaries section:
  - Add security policy layer diagram
  - Document allowlist-by-default philosophy

### Step 9: Verify
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace`
- Verify all new tests pass
- Verify no regressions in existing 892 tests

---

## 6. Completion Checklist

### SEC-1 (Shell)
- [ ] `security_policy.rs` created with `CommandPolicy`
- [ ] `PolicyMode::Allowlist` is the default
- [ ] Safe default allowlist defined (17 commands)
- [ ] `DANGEROUS_PATTERNS` moved here, always checked as defense-in-depth
- [ ] `extract_first_token` handles paths, pipes, tabs, whitespace
- [ ] `ShellExecTool` wired to use `CommandPolicy`
- [ ] Denylist mode still available via configuration
- [ ] 15+ tests for command policy
- [ ] Documentation updated (configuration, tools reference, tool-calls guide)

### SEC-2 (Spawn)
- [ ] `SpawnTool` wired to use same `CommandPolicy`
- [ ] Validation runs before process spawn
- [ ] Tests for spawn + policy integration

### SEC-3 (SSRF)
- [ ] `url_safety.rs` created with `UrlPolicy` and `validate_url`
- [ ] `hickory-resolver` + `url` + `ipnet` added as dependencies
- [ ] All private/loopback/link-local IPv4 ranges blocked
- [ ] All loopback/link-local/unique-local IPv6 ranges blocked
- [ ] IPv4-mapped IPv6 addresses checked
- [ ] Cloud metadata hostnames blocked
- [ ] Configurable domain allow/block lists
- [ ] `WebFetchTool` wired to validate before fetch
- [ ] 20+ tests for URL safety
- [ ] Documentation updated

### Config
- [ ] `CommandPolicyConfig` added to `ToolsConfig`
- [ ] `UrlPolicyConfig` added to `ToolsConfig`
- [ ] All defaults are safe (allowlist mode, SSRF protection enabled)
- [ ] Config deserialization tests

### Integration
- [ ] `register_all` updated to accept policies
- [ ] `agent.rs` and `gateway.rs` construct policies from config
- [ ] All existing tests pass (no regressions)
- [ ] `cargo check`, `clippy`, `test`, `build` all clean

---

## 7. Agent Allocation

This stream requires 5 agents working in parallel:

| Agent | Role | Files | Est. Lines |
|-------|------|-------|------------|
| A1 | Config types + CommandPolicy | `config.rs`, `security_policy.rs` | 300-400 |
| A2 | URL safety module | `url_safety.rs` | 250-350 |
| A3 | Wire shell + spawn tools | `shell_tool.rs`, `spawn_tool.rs`, `lib.rs` | 150-250 |
| A4 | Wire web_fetch + CLI | `web_fetch.rs`, `agent.rs`, `gateway.rs` | 100-200 |
| A5 | Documentation updates | 4 doc files | 200-300 |

**Dependencies**: A3 depends on A1 (needs `CommandPolicy`). A4 depends on A1+A2 (needs both policies). A5 can start immediately (knows the design).

**Recommended execution order**:
1. A1 + A2 + A5 in parallel (no dependencies)
2. A3 + A4 after A1 + A2 complete (need the policy types)
3. Queen coordinator wires `lib.rs` and runs final verification

**Estimated total**: 1,000-1,500 lines of Rust + 200-300 lines of documentation updates
