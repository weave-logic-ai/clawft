# Phase C2: WASM Plugin Host

> **Element:** 04 -- Plugin & Skill System
> **Phase:** C2
> **Timeline:** Week 4-5
> **Priority:** P1
> **Crates:** `clawft-wasm`, `clawft-core`
> **Dependencies IN:** C1 (plugin traits crate)
> **Blocks:** C4, C4a, C6 (MCP exposure), Elements 06-10 (WASM-based plugins)
> **Status:** Planning
> **Security Spec:** `01-wasm-security-spec.md` (this directory)

---

## 1. Overview

Phase C2 implements the WASM plugin host using wasmtime with WIT component model bindings and a complete security sandbox. This phase replaces the existing stub implementations in `clawft-wasm` with a functional plugin execution environment that enforces the permission model defined in `01-wasm-security-spec.md`.

The WASM host is the runtime counterpart to C1's trait definitions. Where C1 defines *what* a plugin can be, C2 defines *how* a plugin runs -- including sandboxing, fuel metering, memory limits, and host-function permission enforcement.

---

## 2. Current Code (All Stubs)

### `clawft-wasm/Cargo.toml`

Current dependencies: only `clawft-types`, `serde`, `serde_json`. NO wasmtime.

```toml
[dependencies]
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
dlmalloc = { version = "0.2", features = ["global"] }
talc = { version = "4.4", optional = true }
lol_alloc = { version = "0.4", optional = true }
```

### `clawft-wasm/src/fs.rs` -- 8 methods, all return Unsupported

```rust
pub struct WasiFileSystem;
impl WasiFileSystem {
    pub fn read_to_string(&self, _path: &Path) -> std::io::Result<String> {
        Err(unsupported("read_to_string"))
    }
    pub fn write_string(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
        Err(unsupported("write_string"))
    }
    pub fn exists(&self, _path: &Path) -> bool { false }
    pub fn append_string(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
        Err(unsupported("append_string"))
    }
    pub fn list_dir(&self, _path: &Path) -> std::io::Result<Vec<PathBuf>> {
        Err(unsupported("list_dir"))
    }
    pub fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
        Err(unsupported("create_dir_all"))
    }
    pub fn remove_file(&self, _path: &Path) -> std::io::Result<()> {
        Err(unsupported("remove_file"))
    }
    pub fn home_dir(&self) -> Option<PathBuf> { None }
}
```

### `clawft-wasm/src/http.rs` -- stub

```rust
pub struct WasiHttpClient;
impl WasiHttpClient {
    pub fn request(
        &self, _method: &str, _url: &str,
        _headers: &HashMap<String, String>, _body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn Error>> {
        Err("WASI HTTP not yet implemented".into())
    }
}
```

### `clawft-wasm/src/env.rs` -- functional in-memory WasiEnvironment

```rust
pub struct WasiEnvironment {
    vars: Mutex<HashMap<String, String>>,
}
// Fully functional: get_var, set_var, remove_var all work with in-memory HashMap.
```

### `clawft-wasm/src/lib.rs` -- stubs

```rust
pub fn init() -> i32 { 0 }
pub fn process_message(input: &str) -> String {
    format!("clawft-wasm v{}: received '{}' (pipeline not yet wired)", VERSION, input)
}
```

---

## 3. Implementation Tasks

### Task C2.1: Add `wasmtime` dependency behind feature flag

Update `clawft-wasm/Cargo.toml`:

```toml
[features]
default = []
wasm-plugins = [
    "dep:wasmtime",
    "dep:wasmtime-wasi",
    "dep:url",
    "dep:reqwest",
    "dep:shellexpand",
    "dep:regex",
    "dep:tokio",
    "dep:tracing",
]
alloc-talc = ["dep:talc"]
alloc-lol = ["dep:lol_alloc"]
alloc-tracing = []

[dependencies]
clawft-types = { workspace = true }
clawft-plugin = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

# Behind wasm-plugins feature
wasmtime = { version = "27", optional = true }
wasmtime-wasi = { version = "27", optional = true }
url = { version = "2", optional = true }
reqwest = { version = "0.12", features = ["json"], optional = true, default-features = false, features = ["rustls-tls"] }
shellexpand = { version = "3", optional = true }
regex = { version = "1", optional = true }
tokio = { workspace = true, optional = true }
tracing = { workspace = true, optional = true }
thiserror = { workspace = true }
```

The `wasm-plugins` feature flag ensures the host-side wasmtime code does not compile into the WASM target itself (which builds for `wasm32-wasip1`).

### Task C2.2: Define WIT interface `clawft:plugin@0.1.0`

Create `crates/clawft-wasm/wit/plugin.wit`:

```wit
package clawft:plugin@0.1.0;

world plugin {
    /// Host functions imported by plugins.
    import host: interface {
        /// Make an HTTP request. Returns the response body or an error.
        http-request: func(
            method: string,
            url: string,
            headers: list<tuple<string, string>>,
            body: option<string>,
        ) -> result<string, string>;

        /// Read a file from the sandboxed filesystem.
        read-file: func(path: string) -> result<string, string>;

        /// Write content to a file in the sandboxed filesystem.
        write-file: func(path: string, content: string) -> result<_, string>;

        /// Get an environment variable value.
        get-env: func(name: string) -> option<string>;

        /// Log a message at the given level (0=error, 1=warn, 2=info, 3=debug, 4+=trace).
        log: func(level: u8, message: string);
    }

    /// Plugin-exported functions.
    export plugin: interface {
        /// Initialize the plugin. Called once after instantiation.
        init: func() -> result<_, string>;

        /// Execute a tool with the given name and JSON parameters.
        /// Returns the result as a JSON string.
        execute-tool: func(
            tool-name: string,
            params-json: string,
        ) -> result<string, string>;

        /// Get the plugin's metadata as a JSON string.
        describe: func() -> string;
    }
}
```

### Task C2.3: Implement `PluginSandbox`

Create `crates/clawft-wasm/src/sandbox.rs` (behind `#[cfg(feature = "wasm-plugins")]`):

```rust
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use clawft_plugin::{PluginPermissions, PluginResourceConfig};

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
```

Include `RateCounter`, `NetworkAllowlist`, and `PluginSandbox::from_manifest()` as specified in `01-wasm-security-spec.md` Section 4.1.

### Task C2.4: Implement `validate_http_request()`

Create `crates/clawft-wasm/src/validation.rs` (behind `#[cfg(feature = "wasm-plugins")]`).

Full implementation from `01-wasm-security-spec.md` Section 4.2:

1. Parse URL with `url::Url::parse()`
2. Scheme validation -- reject `file://`, `data:`, `javascript:`, `ftp://`, `gopher://`
3. Network allowlist check against `PluginPermissions.network`
4. SSRF check with `is_private_ip()` -- block RFC 1918, loopback, link-local, CGN, null, IPv6 equivalents, IPv4-mapped IPv6
5. Rate limit via `RateCounter::try_increment()`
6. Body size limit (1 MB max)

The `is_private_ip()` function reuses A6 SSRF logic. After B6 consolidation it will live in `clawft_types::policy`. For now, define it locally with a TODO pointing to the consolidation target.

### Task C2.5: Implement `validate_file_access()`

Full implementation from `01-wasm-security-spec.md` Section 4.3:

1. Path canonicalization (for writes: canonicalize parent, join filename)
2. Sandbox containment check -- canonical path must start_with an allowed path
3. Symlink traversal check -- walk each component of original path, verify symlink targets resolve within sandbox
4. File size limit -- 8 MB for reads, 4 MB for writes

### Task C2.6: Implement `validate_env_access()`

Full implementation from `01-wasm-security-spec.md` Section 4.4:

1. Allowlist check (exact match)
2. Not in allowlist -> return `None` (silent denial)
3. Implicit deny patterns: `_SECRET`, `_PASSWORD`, `_TOKEN` (warn if accessed even when in allowlist)
4. Hardcoded deny list: `PATH`, `HOME`, `USER`, `SHELL`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`
5. Audit log every access

### Task C2.7: Complete `WasiFileSystem` (replace stubs)

Replace the stub implementations in `clawft-wasm/src/fs.rs`. When the `wasm-plugins` feature is enabled, `WasiFileSystem` takes a `PluginSandbox` reference and validates all operations through `validate_file_access()`:

```rust
#[cfg(feature = "wasm-plugins")]
impl WasiFileSystem {
    pub fn new_sandboxed(sandbox: Arc<PluginSandbox>) -> Self { ... }

    pub fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let canonical = validate_file_access(&self.sandbox, path, false)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
        std::fs::read_to_string(canonical)
    }
    // ... similar for write_string, append_string, exists, list_dir, create_dir_all, remove_file
}
```

When the feature is NOT enabled, the existing stubs remain unchanged.

### Task C2.8: Implement WASM HTTP client

Replace the stub in `clawft-wasm/src/http.rs`. When `wasm-plugins` is enabled, `WasiHttpClient` takes a `PluginSandbox` and validates all requests:

```rust
#[cfg(feature = "wasm-plugins")]
impl WasiHttpClient {
    pub fn new_sandboxed(sandbox: Arc<PluginSandbox>) -> Self { ... }

    pub async fn request(...) -> Result<HttpResponse, ...> {
        let url = validate_http_request(&self.sandbox, method, url_str, body)?;
        // Use reqwest with DNS pinning, no redirects, 30s timeout
        // Response size limit: 4 MB
        // Audit log
    }
}
```

### Task C2.9: Wire `init()` and `process_message()`

Update `clawft-wasm/src/lib.rs`:

- When `wasm-plugins` feature is enabled: `init()` loads the wasmtime engine, creates a `Store`, compiles the WIT bindings, and prepares the plugin execution environment
- `process_message()` creates a pipeline context and routes through loaded plugins
- When the feature is disabled: existing stubs remain unchanged

### Task C2.10: Fuel metering

Configure wasmtime fuel consumption:

```rust
let mut config = wasmtime::Config::new();
config.consume_fuel(true);

// Before each execute-tool call:
store.set_fuel(sandbox.fuel_budget)?;
```

| Setting | Default | Hard Max |
|---------|---------|----------|
| Fuel budget | 1,000,000,000 (~1s) | 10,000,000,000 (~10s) |

Fuel is NOT cumulative. Each invocation gets a fresh budget. On `Trap::OutOfFuel`, convert to `PluginError::ResourceExhausted`.

### Task C2.11: Memory limits via `StoreLimitsBuilder`

```rust
use wasmtime::StoreLimitsBuilder;

let limits = StoreLimitsBuilder::new()
    .memory_size(sandbox.memory_limit)       // default: 16 MB
    .table_elements(resources.max_table_elements as usize) // default: 10,000
    .instances(1)                             // one instance per plugin
    .tables(4)                               // max 4 tables
    .memories(1)                             // max 1 linear memory
    .build();

store.limiter(|_| &limits);
```

| Resource | Default | Hard Max |
|----------|---------|----------|
| Memory | 16 MB | 256 MB |
| Table elements | 10,000 | 100,000 |
| Instance count | 1 | 1 |

### Task C2.12: Plugin binary size enforcement

At install/load time, verify:
- Uncompressed WASM module: reject if > 300 KB
- Gzipped WASM module: reject if > 120 KB
- Total plugin directory: reject if > 10 MB

### Task C2.13: Execution timeout

Wrap `execute-tool` calls in `tokio::time::timeout`:

```rust
let result = tokio::time::timeout(
    Duration::from_secs(sandbox.resources.max_execution_seconds),
    execute_plugin_tool(&mut store, &instance, tool_name, params),
).await;

match result {
    Ok(inner) => inner,
    Err(_) => {
        // Drop the Store to abort the WASM instance
        Err(PluginError::ResourceExhausted("execution timeout".into()))
    }
}
```

Default: 30 seconds. Configurable via `resources.max_execution_seconds`.

---

## 4. Resource Limits Summary

| Resource | Default | Hard Max | Manifest Key |
|----------|---------|----------|-------------|
| Fuel | 1,000,000,000 (~1s) | 10,000,000,000 | `resources.max_fuel` |
| Memory | 16 MB | 256 MB | `resources.max_memory_mb` |
| Table elements | 10,000 | 100,000 | `resources.max_table_elements` |
| HTTP requests | 10/min | -- | `resources.max_http_requests_per_minute` |
| Log messages | 100/min | -- | `resources.max_log_messages_per_minute` |
| Execution timeout | 30s | -- | `resources.max_execution_seconds` |
| HTTP body | 1 MB | -- | -- |
| HTTP response | 4 MB | -- | -- |
| File read | 8 MB | -- | -- |
| File write | 4 MB | -- | -- |
| Log message | 4 KB | -- | -- |
| Plugin WASM size | 300 KB uncompressed | -- | -- |
| Plugin WASM gzip | 120 KB compressed | -- | -- |

---

## 5. Concurrency Plan

### Parallel Tasks (can start simultaneously once C1 is complete)

- C2.1-C2.2 (deps + WIT): Must be done first as all other tasks depend on them
- C2.3-C2.6 (sandbox + validation): Can be developed in parallel -- they are independent modules
- C2.7-C2.8 (fs + http replacement): Can be done in parallel once validation functions exist
- C2.9-C2.13 (wiring + limits): Sequential, as they integrate everything

### Suggested Execution Order

```
C2.1 (deps) + C2.2 (WIT)
    |
    +-- C2.3 (sandbox) -- needed by everything below
    |     |
    |     +-- C2.4 (http validation) --|
    |     +-- C2.5 (file validation)  --|-- parallel
    |     +-- C2.6 (env validation)   --|
    |           |
    |           +-- C2.7 (fs impl)  --|
    |           +-- C2.8 (http impl) --|-- parallel
    |                    |
    |                    +-- C2.9 (wiring)
    |                    +-- C2.10 (fuel)
    |                    +-- C2.11 (memory)
    |                    +-- C2.12 (size checks)
    |                    +-- C2.13 (timeout)
```

---

## 6. Implementation Sketches

### 6.1 PluginSandbox (from `01-wasm-security-spec.md`)

```rust
pub struct PluginSandbox {
    pub plugin_id: String,
    pub permissions: PluginPermissions,
    pub allowed_paths_canonical: Vec<PathBuf>,
    pub fuel_budget: u64,
    pub memory_limit: usize,
    pub http_rate: RateCounter,
    pub log_rate: RateCounter,
    pub network_allowlist: NetworkAllowlist,
    pub env_var_allowlist: HashSet<String>,
    pub env_var_deny_patterns: Vec<regex::Regex>,
}

impl PluginSandbox {
    pub fn from_manifest(
        plugin_id: String,
        permissions: PluginPermissions,
        resources: &PluginResourceConfig,
    ) -> std::io::Result<Self> {
        let allowed_paths_canonical = permissions.filesystem.iter()
            .filter_map(|p| {
                let expanded = shellexpand::tilde(p);
                std::fs::canonicalize(expanded.as_ref()).ok()
            })
            .collect();

        let env_var_allowlist: HashSet<String> = permissions.env_vars.iter().cloned().collect();

        let env_var_deny_patterns = vec![
            regex::Regex::new(r"(?i)_SECRET").unwrap(),
            regex::Regex::new(r"(?i)_PASSWORD").unwrap(),
            regex::Regex::new(r"(?i)_TOKEN").unwrap(),
        ];

        Ok(Self {
            plugin_id,
            network_allowlist: NetworkAllowlist::from_permissions(&permissions.network),
            allowed_paths_canonical,
            fuel_budget: resources.max_fuel,
            memory_limit: resources.max_memory_mb.saturating_mul(1024 * 1024),
            http_rate: RateCounter::new(resources.max_http_requests_per_minute, Duration::from_secs(60)),
            log_rate: RateCounter::new(resources.max_log_messages_per_minute, Duration::from_secs(60)),
            env_var_allowlist,
            env_var_deny_patterns,
            permissions,
        })
    }
}
```

### 6.2 NetworkAllowlist

```rust
pub struct NetworkAllowlist {
    pub allow_all: bool,
    pub exact: HashSet<String>,
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

    pub fn is_allowed(&self, host: &str) -> bool {
        if self.allow_all { return true; }
        let host_lower = host.to_lowercase();
        if self.exact.contains(&host_lower) { return true; }
        for suffix in &self.wildcard_suffixes {
            if host_lower.ends_with(suffix) { return true; }
        }
        false
    }
}
```

### 6.3 RateCounter

```rust
pub struct RateCounter {
    pub limit: u64,
    pub count: AtomicU64,
    pub window_start: Mutex<Instant>,
    pub window_duration: Duration,
}

impl RateCounter {
    pub fn new(limit: u64, window_duration: Duration) -> Self {
        Self {
            limit,
            count: AtomicU64::new(0),
            window_start: Mutex::new(Instant::now()),
            window_duration,
        }
    }

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
```

### 6.4 validate_http_request() with is_private_ip()

```rust
const BLOCKED_SCHEMES: &[&str] = &["file", "data", "javascript", "ftp", "gopher"];
const MAX_REQUEST_BODY: usize = 1_048_576; // 1 MB

pub fn validate_http_request(
    sandbox: &PluginSandbox, method: &str, url_str: &str, body: Option<&str>,
) -> Result<Url, HttpValidationError> {
    let url = Url::parse(url_str).map_err(|e| HttpValidationError::InvalidUrl(e.to_string()))?;

    // Scheme check
    let scheme = url.scheme();
    if BLOCKED_SCHEMES.contains(&scheme) || (scheme != "http" && scheme != "https") {
        return Err(HttpValidationError::DisallowedScheme(scheme.to_string()));
    }

    // Network allowlist
    if sandbox.permissions.network.is_empty() {
        return Err(HttpValidationError::NetworkDenied);
    }
    let host = url.host_str().ok_or_else(|| HttpValidationError::InvalidUrl("no host".into()))?;
    if !sandbox.network_allowlist.is_allowed(host) {
        return Err(HttpValidationError::HostNotAllowed(host.to_string()));
    }

    // SSRF check for literal IPs
    if let Some(ip) = url.host().and_then(|h| match h {
        url::Host::Ipv4(v4) => Some(IpAddr::V4(v4)),
        url::Host::Ipv6(v6) => Some(IpAddr::V6(v6)),
        _ => None,
    }) {
        if is_private_ip(&ip) {
            return Err(HttpValidationError::PrivateIp(ip));
        }
    }

    // Rate limit
    if !sandbox.http_rate.try_increment() {
        return Err(HttpValidationError::RateLimited);
    }

    // Body size
    if let Some(body) = body {
        if body.len() > MAX_REQUEST_BODY {
            return Err(HttpValidationError::BodyTooLarge { actual: body.len(), max: MAX_REQUEST_BODY });
        }
    }

    Ok(url)
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 10
            || (o[0] == 172 && (16..=31).contains(&o[1]))
            || (o[0] == 192 && o[1] == 168)
            || o[0] == 127
            || (o[0] == 169 && o[1] == 254)
            || (o[0] == 100 && (64..=127).contains(&o[1]))
            || o[0] == 0
        }
        IpAddr::V6(v6) => {
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return is_private_ip(&IpAddr::V4(mapped_v4));
            }
            v6.is_loopback()
            || v6.segments()[0] == 0xfe80
            || v6.segments()[0] & 0xfe00 == 0xfc00
            || v6.is_unspecified()
        }
    }
}
```

### 6.5 validate_file_access() with symlink traversal

```rust
const MAX_READ_SIZE: u64 = 8 * 1024 * 1024;   // 8 MB
const MAX_WRITE_SIZE: usize = 4 * 1024 * 1024; // 4 MB

pub fn validate_file_access(
    sandbox: &PluginSandbox, path: &Path, write: bool,
) -> Result<PathBuf, FileValidationError> {
    if sandbox.allowed_paths_canonical.is_empty() {
        return Err(FileValidationError::FsDenied);
    }

    // Canonicalize (for writes: canonicalize parent, join filename)
    let canonical = if write {
        let parent = path.parent().ok_or(FileValidationError::CannotResolve)?;
        let parent_canonical = std::fs::canonicalize(parent)
            .map_err(|_| FileValidationError::CannotResolve)?;
        let filename = path.file_name().ok_or(FileValidationError::CannotResolve)?;
        parent_canonical.join(filename)
    } else {
        std::fs::canonicalize(path).map_err(|_| FileValidationError::CannotResolve)?
    };

    // Sandbox containment
    let in_sandbox = sandbox.allowed_paths_canonical.iter()
        .any(|allowed| canonical.starts_with(allowed));
    if !in_sandbox {
        return Err(FileValidationError::OutsideSandbox);
    }

    // Symlink traversal check on original path
    let mut accumulated = PathBuf::new();
    for component in path.components() {
        accumulated.push(component);
        if accumulated.is_symlink() {
            let link_target = std::fs::read_link(&accumulated)?;
            let resolved = if link_target.is_absolute() {
                link_target
            } else {
                accumulated.parent().unwrap_or(Path::new("/")).join(&link_target)
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

    // Size check (reads only)
    if !write {
        if let Ok(metadata) = std::fs::metadata(&canonical) {
            if metadata.len() > MAX_READ_SIZE {
                return Err(FileValidationError::FileTooLarge {
                    actual: metadata.len(), max: MAX_READ_SIZE,
                });
            }
        }
    }

    Ok(canonical)
}
```

### 6.6 validate_env_access() with implicit deny patterns

```rust
/// Hardcoded deny list -- never accessible regardless of allowlist.
const HARDCODED_DENY: &[&str] = &[
    "PATH", "HOME", "USER", "SHELL",
    "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN",
    "ANTHROPIC_API_KEY", "OPENAI_API_KEY",
];

pub fn validate_env_access(sandbox: &PluginSandbox, var_name: &str) -> Option<String> {
    // Check hardcoded deny list
    if HARDCODED_DENY.contains(&var_name) {
        audit_log(&sandbox.plugin_id, "get-env", var_name, "denied");
        return None;
    }

    // Check allowlist
    if !sandbox.env_var_allowlist.contains(var_name) {
        audit_log(&sandbox.plugin_id, "get-env", var_name, "denied");
        return None;
    }

    // Warn on sensitive-looking vars (even if in allowlist)
    for pattern in &sandbox.env_var_deny_patterns {
        if pattern.is_match(var_name) {
            tracing::warn!(
                plugin = %sandbox.plugin_id, var = var_name,
                "plugin accessing sensitive-pattern env var (approved via allowlist)"
            );
            break;
        }
    }

    let result = std::env::var(var_name).ok();
    audit_log(
        &sandbox.plugin_id, "get-env", var_name,
        if result.is_some() { "found" } else { "not_found" },
    );
    result
}

fn audit_log(plugin_id: &str, function: &str, args_summary: &str, result_status: &str) {
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

## 7. Tests Required

### 7.1 HTTP Request Tests (T01-T13)

| # | Test Case | Expected Result |
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

### 7.2 Filesystem Tests (T14-T22)

| # | Test Case | Expected Result |
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

### 7.3 Environment Variable Tests (T23-T27)

| # | Test Case | Expected Result |
|---|-----------|----------------|
| T23 | WASM plugin reads env var listed in `permissions.env_vars` (var is set) | `Some(value)` |
| T24 | WASM plugin reads env var listed in `permissions.env_vars` (var is not set) | `None` |
| T25 | WASM plugin reads env var NOT in `permissions.env_vars` | `None` (not an error) |
| T26 | WASM plugin reads `OPENAI_API_KEY` without it in allowlist | `None` |
| T27 | WASM plugin with empty `permissions.env_vars` reads any env var | `None` |

### 7.4 Resource Limit Tests (T28-T32)

| # | Test Case | Expected Result |
|---|-----------|----------------|
| T28 | WASM plugin exceeds fuel budget (infinite loop) | Trap -> `PluginError::ResourceExhausted` |
| T29 | WASM plugin exceeds memory limit (allocate > 16 MB) | Trap -> `PluginError::ResourceExhausted` |
| T30 | WASM plugin exceeds wall-clock timeout (30s default) | Timeout -> `PluginError::ResourceExhausted` |
| T31 | WASM plugin with custom `resources.max_fuel = 500_000_000` exhausts fuel faster | Trap at lower threshold |
| T32 | WASM plugin with custom `resources.max_memory_mb = 8` exceeds 8 MB | Trap at lower threshold |

### 7.5 Log Rate Limit Tests (T33-T35)

| # | Test Case | Expected Result |
|---|-----------|----------------|
| T33 | WASM plugin emits 100 log messages in < 1 minute | All logged |
| T34 | WASM plugin emits 101st log message within the same minute | Message silently dropped; host emits throttle warning |
| T35 | WASM plugin emits log message > 4 KB | Message truncated to 4 KB with `... [truncated]` suffix |

### 7.6 Lifecycle Security Tests (T36-T42)

| # | Test Case | Expected Result |
|---|-----------|----------------|
| T36 | Install WASM plugin > 300 KB uncompressed | Rejected at install |
| T37 | Install ClawHub plugin without valid signature | Rejected at install |
| T38 | Install local plugin without signature (no `--allow-unsigned`) | Warning logged, install proceeds |
| T39 | Plugin requests `shell: true` on first run | User prompted for approval; denied if not approved |
| T40 | Plugin requests env var access on first run | User prompted for approval |
| T41 | Plugin version upgrade adds new `network` permission | User re-prompted for the new permission only |
| T42 | All host function calls produce audit log entries | Audit log contains entries for every call |

### 7.7 Cross-Cutting Tests (T43-T45)

| # | Test Case | Expected Result |
|---|-----------|----------------|
| T43 | Two plugins running concurrently cannot read each other's memory | Each plugin isolated in its own Store |
| T44 | Plugin A's rate limit exhaustion does not affect Plugin B | Independent rate counters |
| T45 | Fuel reset between invocations: first call uses 900M fuel, second call gets full 1B | Second call succeeds with fresh budget |

### 7.8 Unit Tests (validation functions, not full WASM)

These tests validate the sandbox logic without loading actual WASM modules:

| Test | Description |
|------|-------------|
| `test_network_allowlist_exact_match` | Exact host match works |
| `test_network_allowlist_wildcard_match` | `*.example.com` matches `sub.example.com` |
| `test_network_allowlist_wildcard_no_bare` | `*.example.com` does NOT match `example.com` |
| `test_network_allowlist_allow_all` | `["*"]` allows everything |
| `test_network_allowlist_empty_denies_all` | `[]` denies everything |
| `test_is_private_ip_loopback` | 127.0.0.1 is private |
| `test_is_private_ip_rfc1918` | 10.x, 172.16-31.x, 192.168.x are private |
| `test_is_private_ip_link_local` | 169.254.x.x is private |
| `test_is_private_ip_cgn` | 100.64-127.x.x is private |
| `test_is_private_ip_null` | 0.x.x.x is private |
| `test_is_private_ip_ipv6_loopback` | ::1 is private |
| `test_is_private_ip_ipv6_mapped_v4` | ::ffff:127.0.0.1 is private |
| `test_is_private_ip_public` | 8.8.8.8 is NOT private |
| `test_rate_counter_within_limit` | 10 increments succeed with limit=10 |
| `test_rate_counter_exceeds_limit` | 11th increment fails with limit=10 |
| `test_rate_counter_resets_after_window` | Counter resets after window expires |
| `test_env_access_hardcoded_deny` | PATH, HOME, OPENAI_API_KEY always denied |
| `test_env_access_not_in_allowlist` | Returns None for vars not in allowlist |
| `test_env_access_in_allowlist` | Returns value for vars in allowlist |
| `test_sandbox_from_manifest_defaults` | Default PluginResourceConfig produces correct sandbox values |

---

## 8. Acceptance Criteria

- [ ] `cargo build -p clawft-wasm --features wasm-plugins` compiles cleanly
- [ ] `cargo build -p clawft-wasm` (without feature) compiles cleanly with existing stubs
- [ ] WIT interface `clawft:plugin@0.1.0` defined with 5 host functions
- [ ] `PluginSandbox` struct with all fields: permissions, paths, fuel, memory, rate counters, allowlists
- [ ] `NetworkAllowlist` supports exact match, wildcard subdomain match, and `"*"` allow-all
- [ ] `RateCounter` with fixed-window strategy, configurable limit and duration
- [ ] `validate_http_request()` enforces: URL parse, scheme check, allowlist, SSRF, rate limit, body size
- [ ] `is_private_ip()` blocks: RFC 1918, loopback, link-local, CGN, null, IPv6 equivalents, IPv4-mapped IPv6
- [ ] `validate_file_access()` enforces: canonicalization, sandbox containment, symlink traversal, size limits
- [ ] `validate_env_access()` enforces: allowlist, implicit deny patterns, hardcoded deny list, audit log
- [ ] `WasiFileSystem` stubs replaced with sandboxed implementations (when `wasm-plugins` enabled)
- [ ] `WasiHttpClient` stub replaced with sandboxed implementation (when `wasm-plugins` enabled)
- [ ] Fuel metering configured: 1B default, fresh budget per invocation
- [ ] Memory limits configured via `StoreLimitsBuilder`: 16 MB default
- [ ] Plugin binary size enforcement: <300 KB uncompressed, <120 KB gzipped
- [ ] Execution timeout: 30s default via `tokio::time::timeout`
- [ ] All 45 security tests (T01-T45) pass
- [ ] All unit tests for validation functions pass
- [ ] `cargo clippy -p clawft-wasm --features wasm-plugins -- -D warnings` is clean
- [ ] Audit logging present for all host function calls

---

## 9. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| wasmtime version churn | Medium | Medium | Pin to wasmtime 27.x. WIT interface is versioned (`@0.1.0`). Upgrade path documented. |
| WASM build time increase | Medium | Low | `wasmtime` is behind `wasm-plugins` feature flag. CI only builds with the feature on host targets, not on `wasm32-wasip1`. |
| DNS rebinding in HTTP validation | Low | High | Resolve DNS once, pin IP via `reqwest::ClientBuilder::resolve()`. No automatic redirect following. |
| Symlink TOCTOU in file validation | Low | Medium | Walk every path component for symlinks. Accept the small TOCTOU window as defense-in-depth (canonicalization is the primary guard). |
| Plugin isolation bypass via shared host state | Low | Critical | Each plugin gets its own `Store`, `PluginSandbox`, and rate counters. No shared mutable state between plugins. |
| `reqwest` dep size impact on WASM target | Low | Low | `reqwest` is only pulled when `wasm-plugins` is enabled, which is a host-only feature. The WASM target (`wasm32-wasip1`) never enables this feature. |

---

## 10. References

- Security Specification: `01-wasm-security-spec.md` (this directory)
- Plugin Traits (C1): `01-phase-C1-plugin-traits.md` (this directory)
- Orchestrator: `00-orchestrator.md` (this directory)
- wasmtime fuel metering: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel
- wasmtime StoreLimits: https://docs.rs/wasmtime/latest/wasmtime/struct.StoreLimitsBuilder.html
- WIT specification: https://component-model.bytecodealliance.org/design/wit.html
- SSRF prevention (A6): `.planning/drafts/02-tech-core.md` Section 12.6
