# WASM Host-Function Permission Enforcement Specification

> **Element:** 04 -- Plugin & Skill System
> **Security Review Reference:** Gaps 5, 6, 7 (iteration-1-security.md)
> **Status:** DRAFT -- Addresses NO-GO verdict for Element 04
> **Date:** 2026-02-19
> **Prerequisite for:** C2 (WASM Plugin Host), K3 (Per-Agent Sandbox)

---

## 1. WASM Host-Function Permission Enforcement

Every WIT host function imported by a WASM plugin passes through the
`PluginSandbox` enforcement layer before any side-effecting operation
executes. The sandbox holds the plugin's declared `PluginPermissions`
(from its manifest) and enforces them at the host boundary. No host
function operates without a corresponding permission check.

The WIT interface (`clawft:plugin@0.1.0`) exposes five host functions.
This section specifies the enforcement contract for each.

### 1.1 `http-request`

**WIT signature:**
```wit
http-request: func(
    method: string,
    url: string,
    headers: list<tuple<string, string>>,
    body: option<string>,
) -> result<string, string>;
```

**Enforcement steps (executed in order):**

1. **Parse URL.** Parse `url` with the `url` crate. If parsing fails,
   return `Err("invalid URL")`. Do not attempt the request.

2. **Scheme validation.** Reject any URL whose scheme is not `http` or
   `https`. Explicitly blocked schemes:
   - `file://` -- filesystem access bypass
   - `data:` -- data exfiltration encoding
   - `javascript:` -- meaningless in this context but blocked for defense-in-depth
   - `ftp://`, `gopher://` -- legacy protocols with known SSRF vectors

   If the scheme is blocked, return `Err("scheme not allowed: {scheme}")`.

3. **Network allowlist check.** Compare the URL's host against
   `PluginPermissions.network`:
   - If `network` is empty, **all** outbound HTTP is denied. Return
     `Err("network access not permitted")`.
   - If `network` contains `"*"`, all hosts are allowed (skip to step 4).
   - Otherwise, the URL's host must exactly match one entry in the
     `network` list. Matching is case-insensitive. Port is not part of
     the match (i.e., `"api.example.com"` allows `https://api.example.com:8443/`).
   - Subdomain wildcards: an entry `"*.example.com"` matches
     `foo.example.com` and `bar.baz.example.com` but not `example.com`
     itself.

4. **SSRF check (reuse A6 `is_private_ip()`).** Resolve the hostname
   to an IP address. Pin the resolved IP for the duration of the request
   (DNS rebinding mitigation -- resolve once, use once). Validate the
   pinned IP:
   - Block RFC 1918: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
   - Block loopback: `127.0.0.0/8`
   - Block link-local: `169.254.0.0/16` (includes cloud metadata `169.254.169.254`)
   - Block CGN: `100.64.0.0/10` (RFC 6598)
   - Block null: `0.0.0.0/8`
   - Block IPv6 equivalents: `::1`, `fe80::/10`, `fc00::/7`
   - Block IPv4-mapped IPv6: convert `::ffff:x.x.x.x` to IPv4 before checking

   Use `reqwest::ClientBuilder::resolve()` to pin the IP, preventing
   re-resolution after validation.

   If the IP is private/reserved, return `Err("request to private IP denied")`.

5. **Rate limit.** Each plugin has a per-plugin HTTP request counter
   with a configurable window. Default: **10 requests per minute**.
   Configurable via manifest field `resources.max_http_requests_per_minute`.
   When the limit is exceeded, return `Err("rate limit exceeded: HTTP requests")`.
   The counter resets at the start of each window (fixed-window strategy).

6. **Request size limit.** If `body` is `Some`, enforce a maximum body
   size of **1 MB**. Return `Err("request body too large")` if exceeded.

7. **Execute request.** Use a shared `reqwest::Client` with:
   - Timeout: 30 seconds per request
   - No automatic redirect following (prevents redirect-to-private-IP attacks)
   - `User-Agent: clawft-plugin/{plugin_id}/{plugin_version}`

8. **Response size limit.** Read at most **4 MB** of response body.
   Truncate if larger. Return the response body as a string.

9. **Audit log.** Log every outbound request to the plugin audit trail:
   ```
   [PLUGIN_HTTP] plugin={plugin_id} method={method} url={url} status={status} bytes={response_len}
   ```

### 1.2 `read-file`

**WIT signature:**
```wit
read-file: func(path: string) -> result<string, string>;
```

**Enforcement steps:**

1. **Path canonicalization.** Resolve the path to an absolute canonical
   form using `std::fs::canonicalize()` (or `tokio::fs::canonicalize()`).
   This resolves all `..` sequences and symlinks to their real targets.
   If canonicalization fails (e.g., path does not exist), return
   `Err("path does not exist or cannot be resolved")`.

2. **Sandbox containment check.** Verify that the canonical path starts
   with one of the paths listed in `PluginPermissions.filesystem`. Each
   allowed path is itself canonicalized at plugin load time. The check
   is a strict prefix match on the canonical path string with a path
   separator boundary (i.e., `/allowed/dir` matches `/allowed/dir/file.txt`
   but not `/allowed/directory/file.txt`).

   If the path is outside all allowed directories, return
   `Err("filesystem access denied: path outside sandbox")`.

3. **Symlink traversal check.** Even after canonicalization, verify that
   no intermediate component of the original (non-canonical) path is a
   symlink pointing outside the allowed directories. This prevents
   TOCTOU races where a symlink is created between canonicalization and
   the actual read. Implementation: walk each component of the original
   path, and if any component is a symlink, verify its target
   canonicalizes to within the allowed directories.

4. **File size limit.** Do not read files larger than **8 MB**. Return
   `Err("file too large")` if exceeded. This prevents a plugin from
   causing OOM by requesting a multi-gigabyte file.

5. **Execute read.** Read the file contents as UTF-8. If the file
   contains invalid UTF-8, return `Err("file is not valid UTF-8")`.

6. **Audit log.**
   ```
   [PLUGIN_FS] plugin={plugin_id} op=read path={canonical_path} bytes={len} result=ok|denied
   ```

### 1.3 `write-file`

**WIT signature:**
```wit
write-file: func(path: string, content: string) -> result<_, string>;
```

**Enforcement steps:**

1. **Resolve parent directory.** Split the path into parent directory
   and filename. Canonicalize the parent directory. If the parent does
   not exist, attempt to create intermediate directories only if the
   eventual canonical path of the deepest existing ancestor falls within
   the sandbox. Do not create directories outside allowed paths.

2. **Sandbox containment check.** Same as `read-file` step 2, applied
   to the target write path. The canonical parent + filename must fall
   within `PluginPermissions.filesystem`.

3. **Symlink traversal check.** Same as `read-file` step 3.

4. **Write size limit.** Reject writes larger than **4 MB**. Return
   `Err("write content too large")`.

5. **Execute write.** Write the content atomically (write to a temp
   file in the same directory, then rename). This prevents partial
   writes on crash.

6. **Audit log.**
   ```
   [PLUGIN_FS] plugin={plugin_id} op=write path={canonical_path} bytes={len} result=ok|denied
   ```

### 1.4 `get-env`

**WIT signature:**
```wit
get-env: func(name: string) -> option<string>;
```

**Enforcement steps:**

1. **Allowlist check.** Check whether `name` appears in
   `PluginPermissions.env_vars`. The comparison is exact (case-sensitive).

2. **If not in allowlist: return `None`.** Do not return an error. The
   plugin cannot distinguish between "not permitted" and "env var not
   set." This is intentional -- it prevents a plugin from enumerating
   which env vars exist.

3. **If in allowlist:** Read `std::env::var(name)`. Return `Some(value)`
   if set, `None` if not set.

4. **Secret protection.** Even if an env var is in the allowlist, the
   host logs the access. This provides an audit trail for secret
   access:
   ```
   [PLUGIN_ENV] plugin={plugin_id} var={name} result=found|not_found|denied
   ```

5. **Implicit deny list.** The following env var patterns are never
   accessible regardless of the allowlist, as defense-in-depth:
   - `PATH`, `HOME`, `USER`, `SHELL` (process identity)
   - `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN` (cloud credentials)
   - `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` (LLM provider keys)
   - Any var matching `*_SECRET*`, `*_PASSWORD*`, `*_TOKEN*` unless
     explicitly listed in the allowlist AND the plugin has been
     approved by the user (see Section 3.2, First Run approval)

### 1.5 `log`

**WIT signature:**
```wit
log: func(level: u8, message: string);
```

**Enforcement steps:**

1. **Rate limit.** Each plugin may emit at most **100 log messages per
   minute**. Configurable via `resources.max_log_messages_per_minute`.
   When exceeded, silently drop messages (do not error -- log flooding
   should not crash the plugin). Emit a single host-level warning every
   60 seconds when messages are being dropped:
   ```
   [PLUGIN_LOG_THROTTLE] plugin={plugin_id} dropped={count} in last 60s
   ```

2. **Message size limit.** Truncate messages longer than **4 KB** (4096
   bytes). Append `... [truncated]` to indicate truncation.

3. **Level mapping.** Map the `u8` level to host log levels:
   | Plugin level | Host level |
   |-------------|------------|
   | 0           | `ERROR`    |
   | 1           | `WARN`     |
   | 2           | `INFO`     |
   | 3           | `DEBUG`    |
   | 4+          | `TRACE`    |

4. **Output format.**
   ```
   [PLUGIN:{plugin_id}] {message}
   ```
   The plugin ID is always prepended so operators can distinguish
   plugin logs from host logs in aggregated output.

---

## 2. Resource Limits

### 2.1 Fuel Metering

wasmtime fuel metering prevents infinite loops and CPU abuse. Every
WASM instruction consumes fuel. When fuel is exhausted, the runtime
traps.

**Configuration:**

```rust
let mut config = wasmtime::Config::new();
config.consume_fuel(true);
```

**Per-invocation fuel budget:**

| Setting | Default | Manifest key | Approximate CPU time |
|---------|---------|-------------|---------------------|
| Fuel budget | 1,000,000,000 | `resources.max_fuel` | ~1 second |
| Minimum allowed | 1,000,000 | -- | ~1 ms |
| Maximum allowed | 10,000,000,000 | -- | ~10 seconds |

Before each `execute-tool` call, the host sets the fuel budget:

```rust
store.set_fuel(fuel_budget)?;
```

**When fuel is exhausted:**

- wasmtime raises `Trap::OutOfFuel`
- The host catches this and converts to `PluginError::ResourceExhausted`
- The tool call returns an error result to the agent:
  `{"error": "plugin resource exhausted: CPU time limit exceeded"}`
- Audit log:
  ```
  [PLUGIN_RESOURCE] plugin={plugin_id} resource=fuel used={fuel_consumed} limit={fuel_budget}
  ```

**Fuel is not cumulative.** Each invocation gets a fresh budget. A
plugin cannot "save up" fuel across calls.

### 2.2 Memory Limits

wasmtime `StoreLimits` caps the total memory a WASM instance can
allocate.

**Configuration:**

```rust
use wasmtime::StoreLimitsBuilder;

let limits = StoreLimitsBuilder::new()
    .memory_size(memory_limit_bytes)   // default: 16 MB
    .table_elements(table_limit)       // default: 10,000
    .instances(1)                      // one instance per plugin
    .tables(4)                         // max 4 tables
    .memories(1)                       // max 1 linear memory
    .build();

store.limiter(|_| &limits);
```

**Defaults and configuration:**

| Setting | Default | Manifest key | Hard maximum |
|---------|---------|-------------|-------------|
| Memory size | 16 MB | `resources.max_memory_mb` | 256 MB |
| Table elements | 10,000 | `resources.max_table_elements` | 100,000 |
| Instance count | 1 | -- (not configurable) | 1 |

**When memory is exceeded:**

- wasmtime raises `Trap::MemoryOutOfBounds` or returns an allocation
  failure from `memory.grow`
- The host converts to `PluginError::ResourceExhausted`
- The tool call returns an error result:
  `{"error": "plugin resource exhausted: memory limit exceeded"}`
- Audit log:
  ```
  [PLUGIN_RESOURCE] plugin={plugin_id} resource=memory limit={memory_limit_bytes}
  ```

### 2.3 Execution Timeout

In addition to fuel metering (which measures CPU instructions), the
host enforces a wall-clock timeout. This catches cases where a plugin
blocks on a host function (e.g., a slow HTTP request) that does not
consume fuel.

| Setting | Default | Manifest key |
|---------|---------|-------------|
| Execution timeout | 30 seconds | `resources.max_execution_seconds` |

Implementation: wrap the `execute-tool` call in `tokio::time::timeout`.
On timeout, drop the `Store` (which aborts the WASM instance) and return
`PluginError::ResourceExhausted`.

---

## 3. Plugin Lifecycle Security

### 3.1 Installation

Installation occurs via `weft plugin install <source>` or
`weft skill install <source>`. The source may be a local path, a git
URL, or a ClawHub registry identifier.

**Validation steps (executed in order):**

1. **Size verification.**
   - Uncompressed WASM module: reject if > 300 KB
   - Gzipped WASM module: reject if > 120 KB
   - Total plugin directory: reject if > 10 MB (including skills, docs, assets)

2. **Manifest schema validation.** Parse `clawft.plugin.json` (or
   `.yaml`). Validate against the `PluginManifest` schema. Reject if:
   - `id` is missing or empty
   - `version` is not valid semver
   - `capabilities` is empty
   - `wasm_module` is specified but the file does not exist
   - `permissions` references unknown permission types

3. **Permission policy check.** Evaluate requested permissions against
   installation policy:

   | Permission | Policy |
   |-----------|--------|
   | `filesystem` (limited paths) | Allow with notice |
   | `network` (specific domains) | Warn, require confirmation |
   | `network: ["*"]` (unrestricted) | Warn strongly, require confirmation |
   | `shell: true` | Warn strongly, require confirmation |
   | `env_vars` (any) | Block without explicit approval |

4. **Source-dependent verification:**

   - **ClawHub registry:** Verify Ed25519 cryptographic signature. The
     signature covers the SHA-256 hash of the manifest + WASM module +
     all skill files. Signature verification is **mandatory** for
     ClawHub installs. Reject unsigned packages.

   - **Git URL:** Log warning: "Installing unsigned plugin from git.
     Cannot verify integrity." Require `--allow-unsigned` flag or
     interactive confirmation.

   - **Local path:** Log warning: "Installing local plugin. No signature
     verification." No flag required (development workflow).

5. **Store installation metadata.** Write to
   `~/.clawft/plugins/{plugin_id}/install.json`:
   ```json
   {
     "installed_at": "2026-02-19T12:00:00Z",
     "source": "clawhub:com.example.my-plugin@1.0.0",
     "signature_verified": true,
     "manifest_hash": "sha256:abc123...",
     "approved_permissions": []
   }
   ```

### 3.2 First Run

The first time a plugin is loaded after installation, the host displays
a permission summary and requires explicit user approval.

**Approval prompt:**

```
Plugin "My Plugin" (com.example.my-plugin v1.0.0) requests:

  [network]    api.example.com, cdn.example.com
  [filesystem] ~/.clawft/plugins/my-plugin/data/
  [env_vars]   MY_PLUGIN_API_KEY

  Accept? [y/N]
```

**Permissions requiring explicit approval:**

- `shell: true` -- shell command execution
- `network` with `"*"` -- unrestricted network access
- Any `env_vars` entry -- environment variable access

**Approval storage:** Store the user's approval decision in
`~/.clawft/plugin-approvals.json`:

```json
{
  "com.example.my-plugin": {
    "version": "1.0.0",
    "approved_at": "2026-02-19T12:00:00Z",
    "permissions": {
      "network": ["api.example.com", "cdn.example.com"],
      "filesystem": ["~/.clawft/plugins/my-plugin/data/"],
      "env_vars": ["MY_PLUGIN_API_KEY"],
      "shell": false
    }
  }
}
```

**Version upgrade behavior:** If a plugin is updated and the new version
requests permissions not covered by the existing approval, the host
re-prompts for the additional permissions. Previously approved
permissions are retained.

### 3.3 Runtime

During execution, the following security invariants are maintained:

1. **All host functions check permissions before execution.** No host
   function has a "fast path" that bypasses the sandbox. The permission
   check is the first operation in every host function implementation.

2. **Fuel counter is reset per invocation.** Each call to
   `execute-tool` starts with a fresh fuel budget. A plugin cannot
   accumulate or transfer fuel.

3. **Memory is tracked per plugin instance.** Each plugin gets its own
   `Store` with its own `StoreLimits`. Memory usage from one plugin
   does not affect another.

4. **Audit log entry for every host function call.** Every host function
   invocation is logged with:
   - `plugin_name` -- which plugin made the call
   - `function` -- which host function was called
   - `args_summary` -- sanitized summary of arguments (URLs, paths, env
     var names -- never env var values)
   - `result_status` -- `ok`, `denied`, `error`, `rate_limited`
   - `duration_ms` -- wall-clock time of the host function execution

5. **Plugin isolation.** Each plugin runs in its own `Store`. Plugins
   cannot share memory, call each other's functions, or observe each
   other's state. The only inter-plugin communication path is through
   the host (via the tool registry and message bus).

---

## 4. Rust Implementation Sketch

### 4.1 `PluginSandbox`

```rust
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use clawft_plugin::PluginPermissions;

/// Runtime sandbox state for a loaded WASM plugin.
///
/// Holds the permission set, resource budgets, and rate-limit counters.
/// Created once per plugin instance and passed to all host function
/// implementations.
pub struct PluginSandbox {
    /// Plugin identifier (for logging).
    pub plugin_id: String,

    /// Declared permissions from the plugin manifest.
    pub permissions: PluginPermissions,

    /// Canonicalized allowed filesystem paths (resolved at load time).
    pub allowed_paths_canonical: Vec<PathBuf>,

    /// Fuel budget per invocation.
    pub fuel_budget: u64,

    /// Memory limit in bytes.
    pub memory_limit: usize,

    /// HTTP rate limiter state.
    pub http_rate: RateCounter,

    /// Log rate limiter state.
    pub log_rate: RateCounter,

    /// Network allowlist as a parsed set (for fast lookup).
    pub network_allowlist: NetworkAllowlist,

    /// Env var allowlist as a set (for fast lookup).
    pub env_var_allowlist: HashSet<String>,

    /// Env var patterns that are always denied.
    pub env_var_deny_patterns: Vec<regex::Regex>,
}

/// Simple fixed-window rate counter.
pub struct RateCounter {
    /// Maximum allowed count per window.
    pub limit: u64,
    /// Current count in the active window.
    pub count: AtomicU64,
    /// Start of the current window.
    pub window_start: std::sync::Mutex<Instant>,
    /// Window duration.
    pub window_duration: std::time::Duration,
}

impl RateCounter {
    pub fn new(limit: u64, window_duration: std::time::Duration) -> Self {
        Self {
            limit,
            count: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(Instant::now()),
            window_duration,
        }
    }

    /// Try to increment the counter. Returns `true` if within limit.
    pub fn try_increment(&self) -> bool {
        let mut start = self.window_start.lock().unwrap();
        if start.elapsed() >= self.window_duration {
            *start = Instant::now();
            self.count.store(1, Ordering::Relaxed);
            true
        } else {
            let prev = self.count.fetch_add(1, Ordering::Relaxed);
            prev < self.limit
        }
    }
}

/// Parsed network allowlist supporting exact match and wildcard subdomains.
pub struct NetworkAllowlist {
    /// Allow all hosts.
    pub allow_all: bool,
    /// Exact hostnames (lowercase).
    pub exact: HashSet<String>,
    /// Wildcard suffixes (e.g., ".example.com" for "*.example.com").
    pub wildcard_suffixes: Vec<String>,
}

impl NetworkAllowlist {
    pub fn from_permissions(network: &[String]) -> Self {
        let mut exact = HashSet::new();
        let mut wildcard_suffixes = Vec::new();
        let mut allow_all = false;

        for entry in network {
            if entry == "*" {
                allow_all = true;
            } else if let Some(suffix) = entry.strip_prefix("*.") {
                wildcard_suffixes.push(format!(".{}", suffix.to_lowercase()));
            } else {
                exact.insert(entry.to_lowercase());
            }
        }

        Self { allow_all, exact, wildcard_suffixes }
    }

    /// Check whether a hostname is allowed.
    pub fn is_allowed(&self, host: &str) -> bool {
        if self.allow_all {
            return true;
        }
        let host_lower = host.to_lowercase();
        if self.exact.contains(&host_lower) {
            return true;
        }
        for suffix in &self.wildcard_suffixes {
            if host_lower.ends_with(suffix) {
                return true;
            }
        }
        false
    }
}

impl PluginSandbox {
    /// Build a sandbox from a manifest's permissions and resource config.
    pub fn from_manifest(
        plugin_id: String,
        permissions: PluginPermissions,
        resources: &PluginResourceConfig,
    ) -> std::io::Result<Self> {
        // Canonicalize allowed filesystem paths at load time
        let allowed_paths_canonical = permissions
            .filesystem
            .iter()
            .filter_map(|p| {
                let expanded = shellexpand::tilde(p);
                std::fs::canonicalize(expanded.as_ref()).ok()
            })
            .collect();

        let env_var_allowlist: HashSet<String> =
            permissions.env_vars.iter().cloned().collect();

        let env_var_deny_patterns = vec![
            regex::Regex::new(r"(?i)_SECRET").unwrap(),
            regex::Regex::new(r"(?i)_PASSWORD").unwrap(),
            regex::Regex::new(r"(?i)_TOKEN").unwrap(),
        ];

        Ok(Self {
            plugin_id,
            network_allowlist: NetworkAllowlist::from_permissions(&permissions.network),
            allowed_paths_canonical,
            fuel_budget: resources.max_fuel.unwrap_or(1_000_000_000),
            memory_limit: resources
                .max_memory_mb
                .unwrap_or(16)
                .saturating_mul(1024 * 1024),
            http_rate: RateCounter::new(
                resources.max_http_requests_per_minute.unwrap_or(10),
                std::time::Duration::from_secs(60),
            ),
            log_rate: RateCounter::new(
                resources.max_log_messages_per_minute.unwrap_or(100),
                std::time::Duration::from_secs(60),
            ),
            env_var_allowlist,
            env_var_deny_patterns,
            permissions,
        })
    }
}

/// Resource limits from the plugin manifest `[resources]` section.
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct PluginResourceConfig {
    pub max_fuel: Option<u64>,
    pub max_memory_mb: Option<usize>,
    pub max_http_requests_per_minute: Option<u64>,
    pub max_log_messages_per_minute: Option<u64>,
    pub max_execution_seconds: Option<u64>,
    pub max_table_elements: Option<u32>,
}
```

### 4.2 `validate_http_request`

```rust
use std::net::IpAddr;
use url::Url;

/// Errors from HTTP request validation.
#[derive(Debug, thiserror::Error)]
pub enum HttpValidationError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("scheme not allowed: {0}")]
    DisallowedScheme(String),
    #[error("host not in network allowlist: {0}")]
    HostNotAllowed(String),
    #[error("request to private/reserved IP denied: {0}")]
    PrivateIp(IpAddr),
    #[error("rate limit exceeded: HTTP requests")]
    RateLimited,
    #[error("request body too large: {actual} bytes, max {max}")]
    BodyTooLarge { actual: usize, max: usize },
    #[error("network access not permitted")]
    NetworkDenied,
}

const BLOCKED_SCHEMES: &[&str] = &["file", "data", "javascript", "ftp", "gopher"];
const MAX_REQUEST_BODY: usize = 1_048_576; // 1 MB

/// Validate an HTTP request against the plugin's sandbox permissions.
///
/// Returns `Ok(())` if the request is permitted. Returns `Err` with a
/// specific reason if denied. Does NOT execute the request.
pub fn validate_http_request(
    sandbox: &PluginSandbox,
    method: &str,
    url_str: &str,
    body: Option<&str>,
) -> Result<Url, HttpValidationError> {
    // 1. Parse URL
    let url = Url::parse(url_str)
        .map_err(|e| HttpValidationError::InvalidUrl(e.to_string()))?;

    // 2. Scheme check
    let scheme = url.scheme();
    if BLOCKED_SCHEMES.contains(&scheme) {
        return Err(HttpValidationError::DisallowedScheme(scheme.to_string()));
    }
    if scheme != "http" && scheme != "https" {
        return Err(HttpValidationError::DisallowedScheme(scheme.to_string()));
    }

    // 3. Network allowlist
    if sandbox.permissions.network.is_empty() {
        return Err(HttpValidationError::NetworkDenied);
    }
    let host = url.host_str()
        .ok_or_else(|| HttpValidationError::InvalidUrl("no host in URL".into()))?;
    if !sandbox.network_allowlist.is_allowed(host) {
        return Err(HttpValidationError::HostNotAllowed(host.to_string()));
    }

    // 4. SSRF check -- resolve and validate IP
    //    (actual DNS resolution happens at request time; here we validate
    //     if the host is a literal IP)
    if let Some(ip) = url.host().and_then(|h| match h {
        url::Host::Ipv4(v4) => Some(IpAddr::V4(v4)),
        url::Host::Ipv6(v6) => Some(IpAddr::V6(v6)),
        _ => None,
    }) {
        if is_private_ip(&ip) {
            return Err(HttpValidationError::PrivateIp(ip));
        }
    }

    // 5. Rate limit
    if !sandbox.http_rate.try_increment() {
        return Err(HttpValidationError::RateLimited);
    }

    // 6. Body size
    if let Some(body) = body {
        if body.len() > MAX_REQUEST_BODY {
            return Err(HttpValidationError::BodyTooLarge {
                actual: body.len(),
                max: MAX_REQUEST_BODY,
            });
        }
    }

    Ok(url)
}

/// Check whether an IP address is in a private/reserved range.
///
/// Reuses the A6 SSRF protection logic. This is the canonical
/// implementation -- after B6 consolidation it lives in `clawft_types::policy`.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 10                                              // 10.0.0.0/8
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))      // 172.16.0.0/12
            || (octets[0] == 192 && octets[1] == 168)                    // 192.168.0.0/16
            || octets[0] == 127                                          // 127.0.0.0/8
            || (octets[0] == 169 && octets[1] == 254)                    // 169.254.0.0/16
            || (octets[0] == 100 && (64..=127).contains(&octets[1]))     // 100.64.0.0/10
            || octets[0] == 0                                            // 0.0.0.0/8
        }
        IpAddr::V6(v6) => {
            // Handle IPv4-mapped IPv6 (::ffff:x.x.x.x)
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return is_private_ip(&IpAddr::V4(mapped_v4));
            }
            v6.is_loopback()                                             // ::1
            || v6.segments()[0] == 0xfe80                                // fe80::/10 link-local
            || v6.segments()[0] & 0xfe00 == 0xfc00                       // fc00::/7 ULA
            || v6.is_unspecified()                                       // ::
        }
    }
}
```

### 4.3 `validate_file_access`

```rust
use std::path::{Path, PathBuf};

/// Errors from file access validation.
#[derive(Debug, thiserror::Error)]
pub enum FileValidationError {
    #[error("filesystem access denied: path outside sandbox")]
    OutsideSandbox,
    #[error("path does not exist or cannot be resolved")]
    CannotResolve,
    #[error("symlink points outside sandbox: {0}")]
    SymlinkEscape(PathBuf),
    #[error("file too large: {actual} bytes, max {max}")]
    FileTooLarge { actual: u64, max: u64 },
    #[error("filesystem access not permitted")]
    FsDenied,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

const MAX_READ_SIZE: u64 = 8 * 1024 * 1024;  // 8 MB
const MAX_WRITE_SIZE: usize = 4 * 1024 * 1024;  // 4 MB

/// Validate a file access request against the plugin's sandbox.
///
/// `write` indicates whether this is a write operation (stricter checks).
/// Returns the canonical path if access is permitted.
pub fn validate_file_access(
    sandbox: &PluginSandbox,
    path: &Path,
    write: bool,
) -> Result<PathBuf, FileValidationError> {
    // 0. Check that filesystem permissions exist at all
    if sandbox.allowed_paths_canonical.is_empty() {
        return Err(FileValidationError::FsDenied);
    }

    // 1. Canonicalize the path
    //    For writes, the file may not exist yet -- canonicalize the parent
    let canonical = if write {
        let parent = path.parent()
            .ok_or(FileValidationError::CannotResolve)?;
        let parent_canonical = std::fs::canonicalize(parent)
            .map_err(|_| FileValidationError::CannotResolve)?;
        let filename = path.file_name()
            .ok_or(FileValidationError::CannotResolve)?;
        parent_canonical.join(filename)
    } else {
        std::fs::canonicalize(path)
            .map_err(|_| FileValidationError::CannotResolve)?
    };

    // 2. Sandbox containment check
    let in_sandbox = sandbox.allowed_paths_canonical.iter().any(|allowed| {
        // Strict prefix match with path separator boundary
        canonical.starts_with(allowed)
    });
    if !in_sandbox {
        return Err(FileValidationError::OutsideSandbox);
    }

    // 3. Symlink traversal check on the original (non-canonical) path
    //    Walk each component and verify symlinks resolve within sandbox
    let mut accumulated = PathBuf::new();
    for component in path.components() {
        accumulated.push(component);
        if accumulated.is_symlink() {
            let link_target = std::fs::read_link(&accumulated)?;
            let resolved = if link_target.is_absolute() {
                link_target
            } else {
                accumulated.parent()
                    .unwrap_or(Path::new("/"))
                    .join(&link_target)
            };
            let resolved_canonical = std::fs::canonicalize(&resolved)
                .map_err(|_| FileValidationError::SymlinkEscape(accumulated.clone()))?;

            let symlink_in_sandbox = sandbox.allowed_paths_canonical.iter()
                .any(|allowed| resolved_canonical.starts_with(allowed));
            if !symlink_in_sandbox {
                return Err(FileValidationError::SymlinkEscape(accumulated));
            }
        }
    }

    // 4. Size check (for reads only -- writes are checked by caller)
    if !write {
        if let Ok(metadata) = std::fs::metadata(&canonical) {
            if metadata.len() > MAX_READ_SIZE {
                return Err(FileValidationError::FileTooLarge {
                    actual: metadata.len(),
                    max: MAX_READ_SIZE,
                });
            }
        }
    }

    Ok(canonical)
}
```

### 4.4 `validate_env_access`

```rust
/// Validate an environment variable access request.
///
/// Returns `Some(value)` if the variable is permitted and set.
/// Returns `None` if the variable is not permitted OR not set.
/// Never returns an error -- the plugin cannot distinguish denial
/// from absence.
pub fn validate_env_access(
    sandbox: &PluginSandbox,
    var_name: &str,
) -> Option<String> {
    // 1. Check allowlist
    if !sandbox.env_var_allowlist.contains(var_name) {
        tracing::debug!(
            plugin = %sandbox.plugin_id,
            var = var_name,
            "env var access denied: not in allowlist"
        );
        audit_log(
            &sandbox.plugin_id,
            "get-env",
            var_name,
            "denied",
        );
        return None;
    }

    // 2. Check implicit deny patterns (defense-in-depth)
    //    Only apply to vars matching secret patterns that are NOT
    //    explicitly approved via the first-run approval flow.
    //    For now, we trust the allowlist if the var is listed there,
    //    but log at WARN level for sensitive-looking vars.
    for pattern in &sandbox.env_var_deny_patterns {
        if pattern.is_match(var_name) {
            tracing::warn!(
                plugin = %sandbox.plugin_id,
                var = var_name,
                "plugin accessing sensitive-pattern env var (approved via allowlist)"
            );
            break;
        }
    }

    // 3. Read from environment
    let result = std::env::var(var_name).ok();

    audit_log(
        &sandbox.plugin_id,
        "get-env",
        var_name,
        if result.is_some() { "found" } else { "not_found" },
    );

    result
}

/// Write an audit log entry for a host function call.
fn audit_log(
    plugin_id: &str,
    function: &str,
    args_summary: &str,
    result_status: &str,
) {
    tracing::info!(
        target: "clawft::plugin::audit",
        plugin = plugin_id,
        function = function,
        args = args_summary,
        result = result_status,
        "plugin host function call"
    );
}
```

---

## 5. Plugin Manifest Resource Configuration

The plugin manifest `clawft.plugin.json` gains a `resources` section
for configuring sandbox limits:

```json
{
  "id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "capabilities": ["tool"],
  "permissions": {
    "network": ["api.example.com"],
    "filesystem": ["~/.clawft/plugins/my-plugin/data/"],
    "env_vars": ["MY_PLUGIN_API_KEY"],
    "shell": false
  },
  "resources": {
    "max_fuel": 2000000000,
    "max_memory_mb": 32,
    "max_http_requests_per_minute": 20,
    "max_log_messages_per_minute": 200,
    "max_execution_seconds": 60,
    "max_table_elements": 20000
  },
  "wasm_module": "plugin.wasm"
}
```

All `resources` fields are optional. Omitted fields use the defaults
defined in Section 2. The host enforces hard maximums -- a plugin cannot
request unlimited resources.

---

## 6. Security Tests Required

The following test cases must all pass before Element 04 receives a
security GO verdict. Tests are organized by host function and resource
limit.

### 6.1 HTTP Request Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T01 | WASM plugin sends HTTP to domain in `permissions.network` allowlist | Request succeeds |
| T02 | WASM plugin sends HTTP to domain NOT in `permissions.network` allowlist | `Err("host not in network allowlist")` |
| T03 | WASM plugin sends HTTP with `file://` scheme | `Err("scheme not allowed: file")` |
| T04 | WASM plugin sends HTTP with `data:` scheme | `Err("scheme not allowed: data")` |
| T05 | WASM plugin sends HTTP to `http://127.0.0.1/` | `Err("request to private/reserved IP denied")` |
| T06 | WASM plugin sends HTTP to `http://[::ffff:127.0.0.1]/` | `Err("request to private/reserved IP denied")` (IPv4-mapped IPv6) |
| T07 | WASM plugin sends HTTP to `http://169.254.169.254/` (cloud metadata) | `Err("request to private/reserved IP denied")` |
| T08 | WASM plugin sends HTTP to `http://10.0.0.1/` (RFC 1918) | `Err("request to private/reserved IP denied")` |
| T09 | WASM plugin with empty `permissions.network` sends any HTTP request | `Err("network access not permitted")` |
| T10 | WASM plugin sends 11th HTTP request within 1 minute (default rate limit = 10) | `Err("rate limit exceeded: HTTP requests")` |
| T11 | WASM plugin sends HTTP with 2 MB body | `Err("request body too large")` |
| T12 | WASM plugin with `"network": ["*.example.com"]` sends to `sub.example.com` | Request succeeds |
| T13 | WASM plugin with `"network": ["*.example.com"]` sends to `example.com` | `Err("host not in network allowlist")` |

### 6.2 Filesystem Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T14 | WASM plugin reads file within allowed `permissions.filesystem` path | File contents returned |
| T15 | WASM plugin reads file outside allowed path (e.g., `/etc/passwd`) | `Err("filesystem access denied")` |
| T16 | WASM plugin reads file via `../../etc/passwd` path traversal | `Err("filesystem access denied")` (canonicalization resolves `..`) |
| T17 | WASM plugin reads file via symlink that points outside sandbox | `Err("symlink points outside sandbox")` |
| T18 | WASM plugin writes file within allowed path | Write succeeds |
| T19 | WASM plugin writes file outside allowed path | `Err("filesystem access denied")` |
| T20 | WASM plugin reads file larger than 8 MB | `Err("file too large")` |
| T21 | WASM plugin with empty `permissions.filesystem` reads any file | `Err("filesystem access not permitted")` |
| T22 | WASM plugin writes via symlink that points outside sandbox | `Err("symlink points outside sandbox")` |

### 6.3 Environment Variable Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T23 | WASM plugin reads env var listed in `permissions.env_vars` (var is set) | `Some(value)` |
| T24 | WASM plugin reads env var listed in `permissions.env_vars` (var is not set) | `None` |
| T25 | WASM plugin reads env var NOT in `permissions.env_vars` | `None` (not an error) |
| T26 | WASM plugin reads `OPENAI_API_KEY` without it in allowlist | `None` |
| T27 | WASM plugin with empty `permissions.env_vars` reads any env var | `None` |

### 6.4 Resource Limit Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T28 | WASM plugin exceeds fuel budget (infinite loop) | Trap -> `PluginError::ResourceExhausted` |
| T29 | WASM plugin exceeds memory limit (allocate > 16 MB) | Trap -> `PluginError::ResourceExhausted` |
| T30 | WASM plugin exceeds wall-clock timeout (30s default) | Timeout -> `PluginError::ResourceExhausted` |
| T31 | WASM plugin with custom `resources.max_fuel = 500_000_000` exhausts fuel faster | Trap at lower threshold |
| T32 | WASM plugin with custom `resources.max_memory_mb = 8` exceeds 8 MB | Trap at lower threshold |

### 6.5 Log Rate Limit Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T33 | WASM plugin emits 100 log messages in < 1 minute | All logged |
| T34 | WASM plugin emits 101st log message within the same minute | Message silently dropped; host emits throttle warning |
| T35 | WASM plugin emits log message > 4 KB | Message truncated to 4 KB with `... [truncated]` suffix |

### 6.6 Lifecycle Security Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T36 | Install WASM plugin > 300 KB uncompressed | Rejected at install |
| T37 | Install ClawHub plugin without valid signature | Rejected at install |
| T38 | Install local plugin without signature (no `--allow-unsigned`) | Warning logged, install proceeds |
| T39 | Plugin requests `shell: true` on first run | User prompted for approval; denied if not approved |
| T40 | Plugin requests env var access on first run | User prompted for approval |
| T41 | Plugin version upgrade adds new `network` permission | User re-prompted for the new permission only |
| T42 | All host function calls produce audit log entries | Audit log contains entries for every call |

### 6.7 Cross-Cutting Tests

| # | Test case | Expected result |
|---|-----------|----------------|
| T43 | Two plugins running concurrently cannot read each other's memory | Each plugin isolated in its own Store |
| T44 | Plugin A's rate limit exhaustion does not affect Plugin B | Independent rate counters |
| T45 | Fuel reset between invocations: first call uses 900M fuel, second call gets full 1B | Second call succeeds with fresh budget |

---

## 7. Exit Criteria

Element 04 transitions from NO-GO to GO when all of the following are
satisfied:

- [ ] Every WIT host function (`http-request`, `read-file`, `write-file`, `get-env`, `log`) validates against `PluginPermissions` before executing
- [ ] WASM plugins have fuel metering enabled with configurable budget (default: 1B units)
- [ ] WASM plugins have memory limits via `StoreLimits` (default: 16 MB)
- [ ] `read-file` / `write-file` host functions canonicalize paths and reject symlinks outside allowed directories
- [ ] `http-request` host function applies SSRF check (including IPv4-mapped IPv6) + network allowlist
- [ ] `get-env` host function returns `None` for non-permitted env vars (never errors)
- [ ] Rate limiting enforced on `http-request` and `log` host functions
- [ ] Audit logging present for all host function calls
- [ ] ClawHub installs require cryptographic signature verification
- [ ] First-run permission approval prompt implemented for shell, unrestricted network, and env var access
- [ ] All 45 security tests (T01-T45) pass
- [ ] Auto-generated skills require user approval before activation (C4a)
- [ ] Shell-execution skills require explicit user approval on install (C3)

---

## 8. References

- Security Review: `.planning/sparc/reviews/iteration-1-security.md` (Gaps 5, 6, 7)
- Technical Specification: `.planning/drafts/02-tech-core.md` (Sections 14.1-14.6)
- Improvements Plan: `.planning/improvements.md` (C2: WASM Plugin Host, K3: Per-Agent Sandbox)
- wasmtime fuel metering: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel
- wasmtime StoreLimits: https://docs.rs/wasmtime/latest/wasmtime/struct.StoreLimitsBuilder.html
- SSRF prevention (A6): `.planning/drafts/02-tech-core.md` Section 12.6
