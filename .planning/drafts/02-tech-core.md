# Technical Requirements Addendum: Workstreams A, B, C, I

> Draft additions for `02-technical-requirements.md`. Each section specifies
> code-level changes, new types, crate boundaries, and testing requirements
> at the same depth as the existing document.

---

## 12. Workstream A: Critical Fixes

### 12.1 Session Key Round-Trip Encoding (A1)

**Crate:** `clawft-core` | **File:** `src/session.rs`

The current `session_path()` uses a 1:1 character replacement that is not
reversible when the original key contains underscores:

```rust
// CURRENT (broken)
fn session_path(&self, key: &str) -> PathBuf {
    let filename = format!("{}.jsonl", key.replace(':', "_"));
    self.sessions_dir.join(filename)
}
```

**Fix:** Percent-encode all characters outside `[A-Za-z0-9._-]` so the
mapping is bijective. Use `percent-encoding` crate (already available
transitively through `url 2.5` in workspace deps).

```rust
use percent_encoding::{utf8_percent_encode, percent_decode_str, AsciiSet, CONTROLS};

/// Characters that require encoding in session filenames.
/// Everything outside `[A-Za-z0-9._-]` is encoded.
const SESSION_KEY_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b':')
    .add(b'/')
    .add(b'\\')
    .add(b' ')
    .add(b'_')   // encode underscores to avoid ambiguity
    .add(b'%');   // encode percent itself

/// Encode a session key for use as a filename component.
fn encode_session_key(key: &str) -> String {
    utf8_percent_encode(key, SESSION_KEY_ENCODE_SET).to_string()
}

/// Decode a filename component back to the original session key.
fn decode_session_key(filename: &str) -> String {
    percent_decode_str(filename)
        .decode_utf8_lossy()
        .into_owned()
}

fn session_path(&self, key: &str) -> PathBuf {
    let filename = format!("{}.jsonl", encode_session_key(key));
    self.sessions_dir.join(filename)
}

pub async fn list_sessions(&self) -> clawft_types::Result<Vec<String>> {
    let entries = self.platform.fs().list_dir(&self.sessions_dir).await?;
    Ok(entries
        .iter()
        .filter_map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".jsonl"))
                .map(decode_session_key)
        })
        .collect())
}
```

**Migration:** On startup, `SessionManager::new()` scans for existing files
using the old `_` convention and renames them to percent-encoded form. Log
each rename at `info` level. This is a one-time forward migration.

```rust
/// Migrate legacy session files from underscore encoding to percent encoding.
async fn migrate_legacy_filenames(&self) -> clawft_types::Result<u32> {
    let entries = self.platform.fs().list_dir(&self.sessions_dir).await?;
    let mut migrated = 0u32;
    for path in &entries {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Skip files that already contain percent-encoded sequences
            if stem.contains('%') {
                continue;
            }
            // Old convention: colons replaced with underscores
            if stem.contains('_') {
                let old_key = stem.replace('_', ":");
                let new_name = format!("{}.jsonl", encode_session_key(&old_key));
                let new_path = self.sessions_dir.join(&new_name);
                if !self.platform.fs().exists(&new_path).await {
                    self.platform.fs().rename(path, &new_path).await?;
                    tracing::info!(old = %stem, new = %new_name, "migrated session filename");
                    migrated += 1;
                }
            }
        }
    }
    Ok(migrated)
}
```

**New dependency:** None (uses `percent-encoding` re-exported from `url` crate).

**Testing requirements:**
- Unit test: `encode_session_key("telegram:user_123")` encodes both `:` and `_`
- Unit test: `decode_session_key(encode_session_key(key)) == key` for all test keys
- Unit test: `list_sessions` round-trips keys containing underscores, colons, and unicode
- Integration test: migration renames `telegram_user_123.jsonl` to `telegram%3Auser%5F123.jsonl`
- Property test: arbitrary ASCII strings survive encode/decode round-trip

---

### 12.2 Stable Hash Function in Embeddings (A2)

**Crate:** `clawft-core` | **File:** `src/embeddings/hash_embedder.rs`

**Current:** Uses `std::collections::hash_map::DefaultHasher` whose output is
explicitly unstable across Rust versions and program invocations (when
`RUSTFLAGS` includes randomized SipHash).

```rust
// CURRENT (unstable)
use std::hash::{DefaultHasher, Hash, Hasher};
```

**Fix:** Replace with `fnv` crate (FNV-1a, 64-bit). Deterministic, stable
across all platforms and Rust versions. Zero dependencies.

```rust
// NEW (stable)
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

pub fn compute_embedding(&self, text: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; self.dimension];
    for word in text.split_whitespace() {
        let mut hasher = FnvHasher::default();
        word.hash(&mut hasher);
        let hash = hasher.finish();
        for (i, slot) in vector.iter_mut().enumerate() {
            let bit = (hash ^ (i as u64)) >> (i % 64) & 1;
            *slot += if bit == 1 { 1.0 } else { -1.0 };
        }
    }
    // L2 normalize
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut vector {
            *v /= norm;
        }
    }
    vector
}
```

**New dependency:**
```toml
# workspace Cargo.toml [workspace.dependencies]
fnv = "1.0"

# clawft-core/Cargo.toml
[dependencies]
fnv = { workspace = true }
```

**Migration:** Add a version marker to persisted embedding metadata. On load,
if the version is `<2`, log a warning that reindexing is needed. Provide a
`weft memory reindex` CLI command (future, Workstream H2) that re-computes
all stored embeddings.

**Testing requirements:**
- Unit test: same input produces identical embedding across 1000 invocations
- Unit test: verify specific hash output for known input (golden test)
- Unit test: verify embedding dimension matches configuration
- Integration test: embeddings persisted with v1 (DefaultHasher) trigger warning on load

---

### 12.3 Safe JSON Error Formatting (A3)

**Crate:** `clawft-core` | **File:** `src/agent/loop_core.rs`

**Current (broken):**
```rust
format!("{{\"error\": \"{}\"}}", e)
```

If `e` contains `"` or `\`, the result is malformed JSON.

**Fix:**
```rust
serde_json::json!({"error": e.to_string()}).to_string()
```

**Apply to all occurrences.** Search pattern:
```
rg 'format!\(".*\\{\\{.*error.*\\}\\}"' crates/
```

**Testing requirements:**
- Unit test: error message containing `"`, `\`, `\n` produces valid JSON
- Unit test: `serde_json::from_str` round-trip succeeds for all error variants

---

### 12.4 Credential Redaction in Config Structs (A4)

**Crate:** `clawft-types` | **File:** `src/config.rs`

**Problem:** The following fields store plaintext credentials and appear in
`Debug` output and serialized JSON:

| Struct | Field | Current Type |
|--------|-------|-------------|
| `FeishuConfig` | `app_secret` | `String` |
| `FeishuConfig` | `encrypt_key` | `String` |
| `DingTalkConfig` | `client_secret` | `String` |
| `MochatConfig` | `claw_token` | `String` |
| `EmailConfig` | `imap_password` | `String` |
| `EmailConfig` | `smtp_password` | `String` |
| `QQConfig` | `secret` | `String` |
| `QQConfig` | `api_key` | `String` |

**Fix (two-part):**

**Part 1: Introduce `SecretRef` wrapper type**

Add to `clawft-types/src/secret.rs`:

```rust
use std::fmt;
use serde::{Deserialize, Serialize};

/// A reference to a secret value stored in an environment variable.
///
/// Never stores the actual secret. Only stores the name of the env var
/// that holds the secret. Custom `Debug` and `Display` impls redact the
/// value. `Serialize` writes the env var name only.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SecretRef {
    /// Environment variable name (e.g. `"FEISHU_APP_SECRET"`).
    #[serde(default)]
    pub env: String,
}

impl SecretRef {
    /// Create a new secret reference from an env var name.
    pub fn new(env_var: impl Into<String>) -> Self {
        Self { env: env_var.into() }
    }

    /// Resolve the secret value from the environment.
    ///
    /// Returns `None` if the env var is unset or the `env` field is empty.
    pub fn resolve(&self) -> Option<String> {
        if self.env.is_empty() {
            return None;
        }
        std::env::var(&self.env).ok()
    }

    /// Whether this reference has been configured (env var name is set).
    pub fn is_configured(&self) -> bool {
        !self.env.is_empty()
    }
}

impl fmt::Debug for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.env.is_empty() {
            write!(f, "SecretRef(<unconfigured>)")
        } else {
            write!(f, "SecretRef(env={})", self.env)
        }
    }
}

impl fmt::Display for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${{{}}}", self.env)
    }
}
```

**Part 2: Migrate config structs** (breaking change, major version bump of types)

Replace each plaintext credential field with a `SecretRef` + deprecation alias:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailConfig {
    #[serde(default)]
    pub enabled: bool,

    // ... non-secret fields unchanged ...

    /// IMAP password -- references an environment variable.
    #[serde(default, alias = "imapPassword")]
    pub imap_password: SecretRef,

    /// SMTP password -- references an environment variable.
    #[serde(default, alias = "smtpPassword")]
    pub smtp_password: SecretRef,
}
```

**Backward compatibility:** Add a custom deserializer that accepts both
`"imap_password": "literal"` (old format, logs a deprecation warning) and
`"imap_password": {"env": "IMAP_PASSWORD"}` (new format):

```rust
/// Deserializes either a plain string (legacy) or a SecretRef object.
/// If a plain string is received, it is treated as the env var name
/// and a deprecation warning is logged.
fn deserialize_secret_compat<'de, D>(deserializer: D) -> Result<SecretRef, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SecretOrString {
        Ref(SecretRef),
        Plain(String),
    }

    match SecretOrString::deserialize(deserializer)? {
        SecretOrString::Ref(r) => Ok(r),
        SecretOrString::Plain(s) if s.is_empty() => Ok(SecretRef::default()),
        SecretOrString::Plain(s) => {
            tracing::warn!(
                field = %s,
                "plaintext credential in config is deprecated; use {{\"env\": \"VAR_NAME\"}} instead"
            );
            // Treat the plain string as the env var name for forward compat
            Ok(SecretRef::new(s))
        }
    }
}
```

**New dependency:** None.

**Testing requirements:**
- Unit test: `Debug` output of `SecretRef` never contains the resolved value
- Unit test: serialized JSON contains env var name, not secret value
- Unit test: `resolve()` reads from env correctly
- Unit test: backward-compatible deserialization from plain string
- Unit test: backward-compatible deserialization from `{"env": "..."}` object

---

### 12.5 API Key Echo Suppression (A5)

**Crate:** `clawft-cli` | **File:** `src/commands/onboard.rs`

**Fix:** Replace `reader.next_line()` with `rpassword::read_password()` for
API key input.

```rust
use rpassword::read_password;

fn prompt_api_key(prompt: &str) -> std::io::Result<String> {
    eprint!("{prompt}: ");
    read_password()
}
```

**New dependency:**
```toml
# workspace Cargo.toml [workspace.dependencies]
rpassword = "5.0"

# clawft-cli/Cargo.toml
[dependencies]
rpassword = { workspace = true }
```

**Testing requirements:**
- Manual test: verify API key is not visible during input
- Unit test: mock stdin approach not feasible; skip in CI, test in integration

---

### 12.6 Complete SSRF Private IP Blocking (A6)

**Crate:** `clawft-services` | **File:** `src/mcp/middleware.rs`

**Current (incomplete):**
```rust
// Only checks 172.16.* but RFC 1918 covers 172.16.0.0/12
```

**Fix:** Full RFC 1918 + RFC 6598 + loopback + link-local check:

```rust
/// Check whether an IP address is in a private/reserved range.
fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            // 10.0.0.0/8
            octets[0] == 10
            // 172.16.0.0/12 (172.16.* through 172.31.*)
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            // 192.168.0.0/16
            || (octets[0] == 192 && octets[1] == 168)
            // 127.0.0.0/8 (loopback)
            || octets[0] == 127
            // 169.254.0.0/16 (link-local)
            || (octets[0] == 169 && octets[1] == 254)
            // 100.64.0.0/10 (CGN, RFC 6598)
            || (octets[0] == 100 && (64..=127).contains(&octets[1]))
            // 0.0.0.0/8
            || octets[0] == 0
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
            || v6.segments()[0] == 0xfe80  // link-local
            || v6.segments()[0] & 0xfe00 == 0xfc00  // unique local (ULA)
        }
    }
}
```

**Apply to:** Both `clawft-services/src/mcp/middleware.rs::UrlPolicy` and
`clawft-tools` (after B6 consolidation, the canonical copy in `clawft-types`).

**Testing requirements:**
- Unit test: `172.16.0.1` through `172.31.255.255` are all blocked
- Unit test: `172.15.255.255` and `172.32.0.0` are allowed
- Unit test: `100.64.0.1` (CGN) is blocked
- Unit test: loopback, link-local IPv4 and IPv6 are blocked
- Unit test: public IPs like `8.8.8.8` are allowed

---

### 12.7 HTTP Request Timeout (A7)

**Crate:** `clawft-llm` | **File:** `src/openai_compat.rs`

**Fix:**
```rust
use std::time::Duration;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

fn build_http_client(timeout: Option<Duration>) -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .timeout(timeout.unwrap_or(DEFAULT_REQUEST_TIMEOUT))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client")
}
```

Make timeout configurable via `ProviderConfig`:

```rust
// clawft-llm/src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    // ... existing fields ...

    /// Request timeout in seconds. Default: 120.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}
```

**Testing requirements:**
- Unit test: client with 1-second timeout actually times out
- Integration test: provider call with unreachable host fails within timeout

---

### 12.8 Safe Environment Variable Mutation in Tests (A8)

**Crate:** `clawft-core` | **File:** `src/workspace.rs` (and any other test modules)

**Fix:** Replace `unsafe { std::env::set_var(...) }` with `temp_env` crate.

```rust
#[cfg(test)]
mod tests {
    use temp_env::with_vars;

    #[test]
    fn workspace_from_env() {
        with_vars(
            [("CLAWFT_WORKSPACE", Some("/tmp/test-workspace"))],
            || {
                let ws = discover_workspace();
                assert_eq!(ws, PathBuf::from("/tmp/test-workspace"));
            },
        );
    }
}
```

**New dependency:**
```toml
# workspace Cargo.toml [workspace.dependencies]
temp_env = "0.3"

# clawft-core/Cargo.toml
[dev-dependencies]
temp_env = { workspace = true }
```

**Testing requirements:**
- Verify all tests pass under `cargo test --workspace` with parallel test runner
- Verify no `unsafe` blocks remain in test code for env var manipulation
- Search command: `rg 'set_var|remove_var' --type rust crates/ -g '!target/'`

---

### 12.9 Feature-Gated MCP Imports (A9)

**Crate:** `clawft-cli` | **File:** `src/mcp_tools.rs`

**Fix:** Gate all `clawft_services` imports behind `#[cfg(feature = "services")]`:

```rust
#[cfg(feature = "services")]
use clawft_services::mcp::{
    session::McpSession,
    transport::{HttpTransport, StdioTransport},
    types::{McpRequest, McpResponse},
};

#[cfg(feature = "services")]
pub fn register_mcp_tools(registry: &mut ToolRegistry, config: &Config) -> Result<()> {
    // ... full implementation ...
}

#[cfg(not(feature = "services"))]
pub fn register_mcp_tools(_registry: &mut ToolRegistry, _config: &Config) -> Result<()> {
    tracing::debug!("MCP tools disabled (services feature not enabled)");
    Ok(())
}
```

**Testing requirements:**
- CI job: `cargo build -p clawft-cli --no-default-features` compiles without errors
- CI job: `cargo build -p clawft-cli --no-default-features --features channel-telegram` compiles
- Unit test: no-op stub returns `Ok(())` when feature is off

---

## 13. Workstream B: Architecture Cleanup

### 13.1 Unified `Usage` Type (B1)

**Crate:** `clawft-types` | **File:** `src/provider.rs` (canonical)

The canonical `Usage` type already exists in `clawft-types` with `u32` fields:

```rust
// clawft-types/src/provider.rs (EXISTING -- keep as canonical)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

The duplicate in `clawft-llm/src/types.rs` uses `i32` and different field names:

```rust
// clawft-llm/src/types.rs (CURRENT -- to be replaced)
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}
```

**Fix:** Add a conversion layer in `clawft-llm` that maps OpenAI-format JSON
(`prompt_tokens`/`completion_tokens`) to the canonical type:

```rust
// clawft-llm/src/types.rs (NEW)
use clawft_types::provider::Usage;

// Re-export canonical type
pub use clawft_types::provider::Usage;

/// Raw OpenAI-format usage response (for deserialization only).
#[derive(Debug, Deserialize)]
pub(crate) struct RawOpenAiUsage {
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

impl From<RawOpenAiUsage> for Usage {
    fn from(raw: RawOpenAiUsage) -> Self {
        Usage {
            input_tokens: raw.prompt_tokens.unwrap_or(0).max(0) as u32,
            output_tokens: raw.completion_tokens.unwrap_or(0).max(0) as u32,
        }
    }
}
```

Add convenience methods to canonical `Usage`:

```rust
// clawft-types/src/provider.rs (additions)
impl Usage {
    /// Total tokens consumed (input + output).
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// Merge usage from multiple calls (accumulate).
    pub fn merge(&mut self, other: &Usage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
    }
}
```

**Testing requirements:**
- Unit test: `RawOpenAiUsage` with negative values clamps to 0
- Unit test: `Usage::total_tokens()` returns sum
- Unit test: all crates compile against the unified type
- Grep verification: no remaining `i32` token count fields outside `RawOpenAiUsage`

---

### 13.2 Deduplicate `LlmMessage` (B2)

**Crate:** `clawft-core`

**Current state:** Two identical `LlmMessage` structs:
- `clawft-core/src/pipeline/traits.rs` (has `tool_calls` field)
- `clawft-core/src/agent/context.rs` (missing `tool_calls` field)

**Fix:** Keep the `pipeline::traits::LlmMessage` as canonical (it is the
superset). Update `context.rs`:

```rust
// clawft-core/src/agent/context.rs (REPLACE local definition)
// Canonical message type from the pipeline module.
pub use crate::pipeline::traits::LlmMessage;
```

Remove the local `LlmMessage` struct from `context.rs` entirely.

**Testing requirements:**
- `cargo build --workspace` succeeds after removal
- All existing context tests pass with the re-exported type

---

### 13.3 File Split Plan (B3)

Target module structure for oversized files:

**`clawft-types/src/config.rs` (~1400 lines) -> `config/` module:**

```
crates/clawft-types/src/config/
    mod.rs              # Re-exports, Config root struct, AgentsConfig, AgentDefaults
    channels.rs         # TelegramConfig, SlackConfig, DiscordConfig, WhatsAppConfig,
                        #   FeishuConfig, DingTalkConfig, MochatConfig, EmailConfig, QQConfig
    providers.rs        # ProvidersConfig, ProviderConfig, ProviderEntry
    policies.rs         # CommandPolicyConfig, RateLimitConfig, ToolsConfig, WebSearchConfig
    gateway.rs          # GatewayConfig, McpServerConfig
    secret.rs           # SecretRef (new from A4)
```

**`clawft-core/src/agent/loop_core.rs` (1645 lines) -> split:**

```
crates/clawft-core/src/agent/
    loop_core.rs        # AgentLoop struct, run(), process_message() (~400 lines)
    tool_executor.rs    # Tool execution loop, result formatting (~400 lines)
    stream_handler.rs   # Streaming response accumulation (~300 lines)
    message_builder.rs  # Message construction helpers (~200 lines)
```

**`clawft-core/src/pipeline/tiered_router.rs` (1646 lines) -> split:**

```
crates/clawft-core/src/pipeline/
    tiered_router.rs    # TieredRouter impl of ModelRouter (~500 lines)
    cost_tracker.rs     # CostTracker, budget enforcement (~400 lines)
    tier_selector.rs    # Tier matching, escalation logic (~400 lines)
    classifier.rs       # KeywordClassifier + ComplexityClassifier (~300 lines)
```

**`clawft-core/src/tools/registry.rs` (1242 lines) -> split:**

```
crates/clawft-core/src/tools/
    registry.rs         # ToolRegistry, dispatch logic (~300 lines)
    builtins/
        mod.rs          # Re-exports
        file_tools.rs   # read_file, write_file, edit_file, list_dir
        exec_tool.rs    # shell exec
        web_tools.rs    # web_search, web_fetch
        message_tool.rs # message bus tool
```

**Rule:** Each resulting file must be under 500 lines. All splits preserve
public API via re-exports from the parent module.

---

### 13.4 Shared Tool Registry Builder (B5)

**Crate:** `clawft-cli` | **File:** `src/commands/mod.rs` (new function)

Extract the duplicated 6-step tool setup into a shared builder:

```rust
use clawft_core::tools::registry::ToolRegistry;
use clawft_platform::Platform;
use clawft_types::config::Config;

/// Build a fully-configured tool registry from config and platform.
///
/// This is the single source of truth for tool registration. Used by
/// `agent` command, `gateway`, and `mcp-server`.
pub async fn build_tool_registry<P: Platform>(
    config: &Config,
    platform: Arc<P>,
) -> anyhow::Result<ToolRegistry> {
    let mut registry = ToolRegistry::new();

    // Step 1: Register built-in file tools (always available)
    registry.register_file_tools(platform.clone());

    // Step 2: Register exec tool (feature-gated)
    #[cfg(feature = "tool-exec")]
    registry.register_exec_tool(platform.clone(), &config.tools);

    // Step 3: Register web tools (feature-gated)
    #[cfg(feature = "tool-web")]
    registry.register_web_tools(platform.clone(), &config.tools);

    // Step 4: Register message bus tool
    // (bus reference injected later by caller)

    // Step 5: Register MCP server tools
    #[cfg(feature = "services")]
    crate::mcp_tools::register_mcp_tools(&mut registry, config)?;

    // Step 6: Register delegation tools
    #[cfg(feature = "delegate")]
    crate::mcp_tools::register_delegation(&mut registry, config)?;

    Ok(registry)
}
```

**Testing requirements:**
- Unit test: builder with default config produces correct tool count
- Unit test: builder with `--no-default-features` produces minimal tool set

---

### 13.5 Policy Type Consolidation (B6)

**Crate:** `clawft-types` | **New file:** `src/policy.rs`

Move `CommandPolicy` and `UrlPolicy` from their duplicate definitions in
`clawft-services` and `clawft-tools` to a single canonical location:

```rust
// clawft-types/src/policy.rs
use std::collections::HashSet;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// Policy for validating shell commands before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicy {
    /// Allowed command basenames. If empty, all commands are rejected.
    pub allowed_commands: HashSet<String>,
    /// Denied argument patterns (regex strings).
    pub denied_patterns: Vec<String>,
}

/// Policy for validating URLs before HTTP requests (SSRF protection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlPolicy {
    /// Allowed URL schemes (e.g., "https", "http").
    pub allowed_schemes: HashSet<String>,
    /// Blocked domains (exact match).
    pub blocked_domains: HashSet<String>,
    /// Whether to block private/reserved IP ranges.
    #[serde(default = "default_true")]
    pub block_private_ips: bool,
}

impl UrlPolicy {
    /// Check whether a URL passes this policy.
    pub fn validate(&self, url: &url::Url) -> Result<(), PolicyViolation> {
        // ... scheme check, domain check, private IP check (using A6 fix) ...
    }
}

/// A policy validation failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PolicyViolation {
    #[error("scheme '{0}' not allowed")]
    DisallowedScheme(String),
    #[error("domain '{0}' is blocked")]
    BlockedDomain(String),
    #[error("private IP address not allowed: {0}")]
    PrivateIp(IpAddr),
}

fn default_true() -> bool { true }
```

Both `clawft-services` and `clawft-tools` import from `clawft_types::policy`.
Remove the duplicate definitions.

**Testing requirements:**
- All existing policy tests migrate to `clawft-types`
- Verify `clawft-services` and `clawft-tools` compile against shared types

---

### 13.6 `ProviderConfig` Naming Resolution (B7)

**Fix:** Rename `clawft-llm`'s `ProviderConfig` to `LlmProviderConfig`:

```rust
// clawft-llm/src/config.rs
/// Configuration for connecting to a single LLM provider endpoint.
///
/// Not to be confused with `clawft_types::config::ProviderConfig` which
/// is the user-facing config format. This type is the resolved runtime
/// representation with env var names (never plaintext secrets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,
    #[serde(default)]
    pub model_prefix: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}
```

Update all references in `clawft-llm` (router, openai_compat, builtin_providers).

**Testing requirements:**
- `cargo build --workspace` succeeds
- All existing provider tests pass with the renamed type

---

## 14. Workstream C: Plugin & Skill System

### 14.1 `clawft-plugin` Crate Specification (C1)

**New crate:** `crates/clawft-plugin/`

**Purpose:** Unified trait definitions for all extension points. Zero runtime
dependencies beyond `serde` and `async-trait`. WASM-safe.

**Depends on:** `clawft-types` only.

```toml
# crates/clawft-plugin/Cargo.toml
[package]
name = "clawft-plugin"
version = "0.1.0"
edition = "2024"

[dependencies]
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
```

#### 14.1.1 Plugin Trait (unified lifecycle)

```rust
// clawft-plugin/src/lib.rs

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use clawft_types::error::ClawftError;

/// Metadata describing a plugin, loaded from its manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (reverse-domain, e.g. "com.clawft.email").
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Plugin author.
    #[serde(default)]
    pub author: Option<String>,
    /// Short description.
    #[serde(default)]
    pub description: Option<String>,
    /// Minimum clawft version required.
    #[serde(default)]
    pub min_clawft_version: Option<String>,
    /// Capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,
    /// Directories containing skills shipped with this plugin.
    #[serde(default)]
    pub skill_dirs: Vec<String>,
    /// Permissions this plugin requires.
    #[serde(default)]
    pub permissions: PluginPermissions,
    /// WASM module path (relative to manifest), if this is a WASM plugin.
    #[serde(default)]
    pub wasm_module: Option<String>,
}

/// The capabilities a plugin can declare.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Provides one or more tools.
    Tool,
    /// Provides a channel adapter (messaging integration).
    Channel,
    /// Provides a pipeline stage.
    PipelineStage,
    /// Provides skills (SKILL.md bundles).
    Skill,
    /// Provides a memory backend.
    Memory,
    /// Provides voice handling (reserved, not implemented).
    Voice,
}

/// Permissions a plugin requests from the host.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    /// Filesystem access (read/write paths).
    #[serde(default)]
    pub filesystem: Vec<String>,
    /// Network access (allowed domains or "*").
    #[serde(default)]
    pub network: Vec<String>,
    /// Environment variable access.
    #[serde(default)]
    pub env_vars: Vec<String>,
    /// Shell command execution.
    #[serde(default)]
    pub shell: bool,
}

/// Unified plugin lifecycle trait.
///
/// Every plugin (native or WASM) implements this trait. The host calls
/// methods in order: `init()` -> repeated `handle_message()` -> `shutdown()`.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin manifest (metadata, capabilities, permissions).
    fn manifest(&self) -> &PluginManifest;

    /// Initialize the plugin with host-provided services.
    ///
    /// Called once after loading. The plugin should validate its
    /// configuration and acquire any resources it needs.
    async fn init(&mut self, host: Arc<dyn PluginHost>) -> Result<(), ClawftError>;

    /// Shut down the plugin gracefully.
    ///
    /// Release resources, close connections, flush buffers.
    async fn shutdown(&mut self) -> Result<(), ClawftError>;

    /// Health check. Returns `true` if the plugin is operational.
    fn is_healthy(&self) -> bool;

    /// Downcast to concrete type for capability-specific interfaces.
    fn as_any(&self) -> &dyn Any;
}

/// Services the plugin host exposes to plugins during `init()`.
#[async_trait]
pub trait PluginHost: Send + Sync {
    /// Get a configuration value by key.
    fn config_value(&self, key: &str) -> Option<serde_json::Value>;

    /// Resolve a secret from an env var name.
    fn resolve_secret(&self, env_var: &str) -> Option<String>;

    /// Get the plugin's data directory (for persistent storage).
    fn data_dir(&self) -> std::path::PathBuf;

    /// Log a message at the given level.
    fn log(&self, level: tracing::Level, message: &str);
}
```

#### 14.1.2 ChannelAdapter Trait

Replaces the current `Channel` trait in `clawft-channels` with a
plugin-compatible version that supports binary payloads (forward-compat for
voice and media):

```rust
// clawft-plugin/src/channel.rs

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use clawft_types::error::ChannelError;
use clawft_types::event::{InboundMessage, OutboundMessage};

/// Payload type for channel messages.
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// UTF-8 text content.
    Text(String),
    /// Binary content (audio, images, files) with MIME type.
    Binary {
        mime_type: String,
        data: Vec<u8>,
    },
    /// Structured content (rich cards, buttons, embeds).
    Structured(serde_json::Value),
}

/// A channel adapter connects an external messaging platform to clawft.
///
/// This replaces `clawft_channels::traits::Channel`. The key differences:
/// - Supports binary payloads (for voice/media forward-compat)
/// - Integrates with the unified Plugin lifecycle
/// - Provides a `capabilities()` method for feature negotiation
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Unique channel identifier (e.g., "telegram", "email", "discord").
    fn name(&self) -> &str;

    /// Capabilities this channel supports.
    fn capabilities(&self) -> ChannelCapabilities;

    /// Start receiving messages. Runs until the cancellation token fires.
    ///
    /// Inbound messages are delivered via `host.deliver_inbound()`.
    async fn start(
        &self,
        host: std::sync::Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), ChannelError>;

    /// Send an outbound message to the channel.
    async fn send(&self, msg: &OutboundMessage) -> Result<(), ChannelError>;

    /// Send a binary/media payload to the channel.
    async fn send_payload(
        &self,
        channel_id: &str,
        payload: MessagePayload,
    ) -> Result<(), ChannelError> {
        // Default: unsupported. Channels that support media override this.
        Err(ChannelError::Unsupported("binary payloads not supported".into()))
    }

    /// Whether the channel is currently connected and running.
    fn is_running(&self) -> bool;
}

/// Capabilities a channel adapter declares.
#[derive(Debug, Clone, Default)]
pub struct ChannelCapabilities {
    /// Supports text messages.
    pub text: bool,
    /// Supports binary/media payloads (images, audio, files).
    pub binary: bool,
    /// Supports rich/structured messages (cards, buttons, embeds).
    pub structured: bool,
    /// Supports real-time streaming responses.
    pub streaming: bool,
    /// Supports message editing/updating.
    pub editable: bool,
    /// Supports message reactions.
    pub reactions: bool,
    /// Supports threads/reply chains.
    pub threads: bool,
}

/// Services the channel adapter host exposes to channel plugins.
#[async_trait]
pub trait ChannelAdapterHost: Send + Sync {
    /// Deliver an inbound message to the agent pipeline.
    async fn deliver_inbound(&self, msg: InboundMessage) -> Result<(), ChannelError>;

    /// Deliver an inbound binary/media payload.
    async fn deliver_payload(
        &self,
        sender_id: &str,
        channel_id: &str,
        payload: MessagePayload,
    ) -> Result<(), ChannelError>;

    /// Get configuration for this channel.
    fn config(&self) -> &serde_json::Value;
}
```

#### 14.1.3 PipelineStage Trait

```rust
// clawft-plugin/src/pipeline.rs

use async_trait::async_trait;
use serde_json::Value;

use clawft_types::error::ClawftError;

/// A pluggable pipeline stage that can be inserted into the 6-stage pipeline.
///
/// Pipeline stages declared by plugins are inserted between the standard
/// stages based on their declared `phase()`.
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Human-readable name of this stage.
    fn name(&self) -> &str;

    /// Which pipeline phase this stage runs in.
    fn phase(&self) -> PipelinePhase;

    /// Process a request/response through this stage.
    ///
    /// The `context` contains the current pipeline state. The stage can
    /// read and modify it. Return `Err` to abort the pipeline.
    async fn process(&self, context: &mut PipelineContext) -> Result<(), ClawftError>;
}

/// Where in the pipeline this stage executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelinePhase {
    /// After classification, before routing.
    PreRoute,
    /// After routing, before context assembly.
    PreAssemble,
    /// After context assembly, before transport.
    PreTransport,
    /// After transport, before scoring.
    PostTransport,
    /// After scoring, before learning.
    PostScore,
}

/// Mutable pipeline state passed through stages.
#[derive(Debug)]
pub struct PipelineContext {
    /// The current request (may be modified by stages).
    pub request: Value,
    /// The current response (populated after transport).
    pub response: Option<Value>,
    /// Metadata accumulated across stages.
    pub metadata: std::collections::HashMap<String, Value>,
}
```

#### 14.1.4 Skill Trait

```rust
// clawft-plugin/src/skill.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use clawft_types::error::ClawftError;

/// Metadata parsed from a SKILL.md YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Skill name (unique identifier).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Skill version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Tags for discovery/search.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Tool names this skill provides.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Required permissions.
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Execution mode.
    #[serde(default)]
    pub execution: SkillExecution,
}

fn default_version() -> String { "0.1.0".into() }

/// How a skill executes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillExecution {
    /// Prompt injection only (add instructions to system prompt).
    #[default]
    Prompt,
    /// WASM module execution.
    Wasm,
    /// Native Rust function.
    Native,
    /// Shell command wrapper.
    Shell,
}

/// A loadable skill that can provide tools and prompt augmentation.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Skill metadata.
    fn metadata(&self) -> &SkillMetadata;

    /// System prompt text to inject when this skill is active.
    fn prompt_text(&self) -> &str;

    /// Tool definitions this skill provides (JSON Schema format).
    fn tool_schemas(&self) -> Vec<serde_json::Value>;

    /// Execute a tool call from this skill.
    async fn execute_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ClawftError>;
}
```

#### 14.1.5 MemoryBackend Trait

```rust
// clawft-plugin/src/memory.rs

use async_trait::async_trait;
use serde_json::Value;

use clawft_types::error::ClawftError;

/// A pluggable memory backend for agent long-term storage.
///
/// The default implementation is the file-based MEMORY.md + HISTORY.md
/// system. Plugins can provide vector databases, SQL stores, cloud
/// backends, etc.
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Backend name (e.g., "file", "qdrant", "pinecone").
    fn name(&self) -> &str;

    /// Store a memory entry.
    async fn store(&self, key: &str, value: Value, metadata: Option<Value>)
        -> Result<(), ClawftError>;

    /// Retrieve a memory entry by exact key.
    async fn retrieve(&self, key: &str) -> Result<Option<Value>, ClawftError>;

    /// Search memories by semantic similarity.
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemorySearchResult>, ClawftError>;

    /// Delete a memory entry.
    async fn delete(&self, key: &str) -> Result<bool, ClawftError>;

    /// List all memory keys (optionally filtered by prefix).
    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ClawftError>;
}

/// A single result from a memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    /// The memory key.
    pub key: String,
    /// The stored value.
    pub value: Value,
    /// Similarity score (0.0 - 1.0).
    pub score: f32,
    /// Optional metadata.
    #[serde(default)]
    pub metadata: Option<Value>,
}
```

#### 14.1.6 VoiceHandler Trait (placeholder)

```rust
// clawft-plugin/src/voice.rs

use async_trait::async_trait;

use clawft_types::error::ClawftError;

/// Placeholder trait for voice processing plugins.
///
/// NOT IMPLEMENTED in the current sprint. This trait exists solely for
/// forward compatibility so that the plugin manifest schema can reserve
/// the `voice` capability type without breaking changes later.
///
/// See `voice_development.md` for the full voice roadmap.
#[async_trait]
pub trait VoiceHandler: Send + Sync {
    /// Process an audio buffer and return a transcription.
    async fn transcribe(&self, audio: &[u8], format: &str) -> Result<String, ClawftError>;

    /// Synthesize speech from text.
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>, ClawftError>;

    /// Whether this handler supports real-time streaming.
    fn supports_streaming(&self) -> bool;
}
```

#### 14.1.7 Module Structure

```
crates/clawft-plugin/src/
    lib.rs           # Plugin, PluginManifest, PluginHost, PluginCapability,
                     #   PluginPermissions, re-exports
    channel.rs       # ChannelAdapter, ChannelAdapterHost, MessagePayload,
                     #   ChannelCapabilities
    pipeline.rs      # PipelineStage, PipelinePhase, PipelineContext
    skill.rs         # Skill, SkillMetadata, SkillExecution
    memory.rs        # MemoryBackend, MemorySearchResult
    voice.rs         # VoiceHandler (placeholder)
```

---

### 14.2 Plugin Manifest Schema (C1)

The manifest file is named `clawft.plugin.json` and lives at the root of the
plugin directory:

```json
{
  "id": "com.clawft.email",
  "name": "Email Channel",
  "version": "0.1.0",
  "author": "ClawFT Contributors",
  "description": "IMAP/SMTP email channel adapter with Gmail OAuth2 support",
  "min_clawft_version": "0.2.0",
  "capabilities": ["channel", "tool"],
  "skill_dirs": ["skills/email-triage", "skills/email-compose"],
  "permissions": {
    "network": ["imap.gmail.com", "smtp.gmail.com", "oauth2.googleapis.com"],
    "env_vars": ["GMAIL_CLIENT_ID", "GMAIL_CLIENT_SECRET"],
    "filesystem": ["~/.clawft/plugins/email/"],
    "shell": false
  },
  "wasm_module": null,
  "config_schema": {
    "type": "object",
    "properties": {
      "imap_host": { "type": "string", "default": "imap.gmail.com" },
      "imap_port": { "type": "integer", "default": 993 },
      "smtp_host": { "type": "string", "default": "smtp.gmail.com" },
      "smtp_port": { "type": "integer", "default": 587 }
    }
  }
}
```

YAML alternative (`clawft.plugin.yaml`):

```yaml
id: com.clawft.email
name: Email Channel
version: 0.1.0
capabilities:
  - channel
  - tool
skill_dirs:
  - skills/email-triage
  - skills/email-compose
permissions:
  network:
    - imap.gmail.com
    - smtp.gmail.com
  env_vars:
    - GMAIL_CLIENT_ID
    - GMAIL_CLIENT_SECRET
```

---

### 14.3 WASM Plugin Host Architecture (C2)

**Crates:** `clawft-wasm` (existing) + `clawft-plugin`

**Runtime:** `wasmtime` with WIT (WebAssembly Interface Types) component model
for typed FFI boundaries.

```toml
# New dependencies for WASM plugin host
# workspace Cargo.toml [workspace.dependencies]
wasmtime = { version = "27", features = ["component-model"] }
wit-bindgen = "0.36"
```

**WIT Interface Definition:**

```wit
// wit/clawft-plugin.wit

package clawft:plugin@0.1.0;

interface types {
    record tool-call {
        name: string,
        arguments: string,  // JSON
    }

    record tool-result {
        content: string,    // JSON
        is-error: bool,
    }

    record message {
        role: string,
        content: string,
    }

    record plugin-info {
        id: string,
        name: string,
        version: string,
    }
}

interface host {
    use types.{message, tool-result};

    /// Read a config value.
    get-config: func(key: string) -> option<string>;

    /// Resolve an environment variable.
    get-env: func(name: string) -> option<string>;

    /// Log a message.
    log: func(level: u8, message: string);

    /// Read a file from the sandbox.
    read-file: func(path: string) -> result<string, string>;

    /// Write a file to the sandbox.
    write-file: func(path: string, content: string) -> result<_, string>;

    /// Make an HTTP request.
    http-request: func(
        method: string,
        url: string,
        headers: list<tuple<string, string>>,
        body: option<string>,
    ) -> result<string, string>;
}

interface plugin {
    use types.{plugin-info, tool-call, tool-result};

    /// Return plugin metadata.
    info: func() -> plugin-info;

    /// Initialize the plugin.
    init: func() -> result<_, string>;

    /// Process a tool call.
    execute-tool: func(call: tool-call) -> tool-result;

    /// Shutdown the plugin.
    shutdown: func();
}

world clawft-plugin {
    import host;
    export plugin;
}
```

**WASM Plugin Loader:**

```rust
// clawft-core/src/plugin/wasm_host.rs (or clawft-wasm/src/plugin_host.rs)

use std::path::Path;
use std::sync::Arc;

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};

use clawft_plugin::PluginManifest;

/// Maximum WASM module size (uncompressed).
const MAX_WASM_SIZE_BYTES: usize = 300 * 1024; // 300 KB

/// Maximum WASM module size (gzipped).
const MAX_WASM_GZIP_SIZE_BYTES: usize = 120 * 1024; // 120 KB

/// Host-side WASM plugin instance.
pub struct WasmPluginInstance {
    engine: Engine,
    manifest: PluginManifest,
    // Store and component instance created on init()
}

impl WasmPluginInstance {
    /// Load a WASM plugin from a manifest directory.
    pub async fn load(
        manifest_dir: &Path,
        manifest: PluginManifest,
    ) -> Result<Self, WasmPluginError> {
        let wasm_path = manifest
            .wasm_module
            .as_ref()
            .ok_or(WasmPluginError::NoWasmModule)?;

        let wasm_bytes = tokio::fs::read(manifest_dir.join(wasm_path)).await?;

        // Enforce size budget
        if wasm_bytes.len() > MAX_WASM_SIZE_BYTES {
            return Err(WasmPluginError::ModuleTooLarge {
                actual: wasm_bytes.len(),
                max: MAX_WASM_SIZE_BYTES,
            });
        }

        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;
        // Component compilation and linking happens in init()

        Ok(Self { engine, manifest })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WasmPluginError {
    #[error("manifest does not specify a wasm_module path")]
    NoWasmModule,
    #[error("WASM module is {actual} bytes, max is {max}")]
    ModuleTooLarge { actual: usize, max: usize },
    #[error("wasmtime error: {0}")]
    Runtime(#[from] wasmtime::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

### 14.4 Skill Loader Architecture (C3)

**Crate:** `clawft-core` | **File:** `src/agent/skills_v2.rs` (refactored)

**SKILL.md Format:**

```markdown
---
name: git-commit
description: Commit staged changes with a generated message
version: 0.2.0
tags: [git, vcs, developer]
tools:
  - name: git_commit
    description: Stage and commit files
    parameters:
      type: object
      properties:
        message:
          type: string
          description: Commit message
        files:
          type: array
          items:
            type: string
          description: Files to stage
      required: [message]
execution: shell
permissions: [filesystem, shell]
---

# Git Commit Skill

You have access to git operations. When the user asks you to commit changes,
use the `git_commit` tool to stage the specified files and create a commit
with the given message.

Always confirm with the user before committing.
```

**Skill Loader with serde_yaml:**

```rust
// clawft-core/src/agent/skill_loader.rs

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clawft_plugin::skill::{Skill, SkillMetadata};
use clawft_platform::Platform;

/// Parsed skill from a SKILL.md file.
pub struct FileSkill {
    /// Parsed YAML frontmatter.
    pub metadata: SkillMetadata,
    /// Markdown body (the prompt text).
    pub body: String,
    /// Source path for diagnostics.
    pub source: PathBuf,
}

/// Parse a SKILL.md file into metadata and prompt body.
pub fn parse_skill_md(content: &str) -> Result<(SkillMetadata, String), SkillLoadError> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err(SkillLoadError::MissingFrontmatter);
    }

    let rest = &content[3..];
    let end = rest
        .find("\n---")
        .ok_or(SkillLoadError::MissingFrontmatter)?;

    let yaml = &rest[..end];
    let body = rest[end + 4..].trim().to_string();

    let metadata: SkillMetadata =
        serde_yaml::from_str(yaml).map_err(SkillLoadError::YamlParse)?;

    Ok((metadata, body))
}

#[derive(Debug, thiserror::Error)]
pub enum SkillLoadError {
    #[error("SKILL.md is missing YAML frontmatter (---) delimiters")]
    MissingFrontmatter,
    #[error("failed to parse YAML frontmatter: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

**New dependency:**
```toml
# workspace Cargo.toml [workspace.dependencies]
serde_yaml = "0.9"

# clawft-core/Cargo.toml
[dependencies]
serde_yaml = { workspace = true }
```

**ClawHub Discovery:**

```rust
/// Discover skills from the ClawHub registry.
pub struct ClawHubClient {
    base_url: String,
    http: Arc<dyn clawft_platform::HttpClient>,
}

impl ClawHubClient {
    const DEFAULT_BASE_URL: &'static str = "https://hub.clawft.dev/api/v1";

    /// Search for skills by query.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SkillListing>, ClawftError> {
        // GET /skills/search?q={query}&limit={limit}
        todo!()
    }

    /// Download and install a skill by its registry ID.
    pub async fn install(&self, skill_id: &str, target_dir: &Path) -> Result<PathBuf, ClawftError> {
        // GET /skills/{id}/download -> tar.gz -> extract to target_dir
        todo!()
    }
}
```

---

### 14.5 Skill Precedence Layering (C4)

Three-layer resolution with highest-precedence-wins semantics:

```
Priority (highest to lowest):
  1. Workspace skills:  <project>/.clawft/skills/
  2. Managed skills:    ~/.clawft/skills/         (installed via `weft skill install`)
  3. Bundled skills:    compiled-in defaults       (shipped with binary)
```

Plugin-shipped skills participate as a fourth layer between managed and bundled:

```
Priority (highest to lowest):
  1. Workspace skills:      <project>/.clawft/skills/
  2. Managed skills:        ~/.clawft/skills/
  3. Plugin-shipped skills: <plugin-dir>/skills/     (declared in manifest.skill_dirs)
  4. Bundled skills:        compiled-in defaults
```

**Implementation:**

```rust
// clawft-core/src/agent/skill_registry.rs

use std::collections::HashMap;
use std::path::PathBuf;

use clawft_plugin::skill::SkillMetadata;

/// Origin of a loaded skill (for precedence and diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkillOrigin {
    /// Compiled into the binary.
    Bundled,
    /// Shipped with a plugin.
    Plugin { plugin_id: String },
    /// Installed in ~/.clawft/skills/ via `weft skill install`.
    Managed,
    /// Project-local in .clawft/skills/.
    Workspace,
}

/// A registered skill with its origin and source path.
#[derive(Debug, Clone)]
pub struct RegisteredSkill {
    pub metadata: SkillMetadata,
    pub prompt_text: String,
    pub origin: SkillOrigin,
    pub source_path: Option<PathBuf>,
}

/// Registry that merges skills from all layers with precedence.
pub struct SkillRegistry {
    /// Skills keyed by name. Higher-precedence overwrites lower.
    skills: HashMap<String, RegisteredSkill>,
}

impl SkillRegistry {
    /// Build the registry by scanning all skill directories in precedence order.
    ///
    /// Scans bundled first, then plugin, then managed, then workspace.
    /// Later entries overwrite earlier ones (higher precedence wins).
    pub async fn discover(
        workspace_dir: Option<&PathBuf>,
        managed_dir: &PathBuf,
        plugin_skill_dirs: &[(String, PathBuf)], // (plugin_id, dir)
    ) -> Result<Self, SkillLoadError> {
        let mut skills = HashMap::new();

        // Layer 1: Bundled (compiled-in)
        for skill in Self::bundled_skills() {
            skills.insert(skill.metadata.name.clone(), skill);
        }

        // Layer 2: Plugin-shipped
        for (plugin_id, dir) in plugin_skill_dirs {
            let discovered = Self::scan_directory(dir, SkillOrigin::Plugin {
                plugin_id: plugin_id.clone(),
            }).await?;
            for skill in discovered {
                skills.insert(skill.metadata.name.clone(), skill);
            }
        }

        // Layer 3: Managed
        let managed = Self::scan_directory(managed_dir, SkillOrigin::Managed).await?;
        for skill in managed {
            skills.insert(skill.metadata.name.clone(), skill);
        }

        // Layer 4: Workspace (highest precedence)
        if let Some(ws) = workspace_dir {
            let ws_skills = Self::scan_directory(ws, SkillOrigin::Workspace).await?;
            for skill in ws_skills {
                skills.insert(skill.metadata.name.clone(), skill);
            }
        }

        Ok(Self { skills })
    }

    async fn scan_directory(
        dir: &PathBuf,
        origin: SkillOrigin,
    ) -> Result<Vec<RegisteredSkill>, SkillLoadError> {
        // Scan dir for subdirectories containing SKILL.md
        // Parse each with parse_skill_md()
        todo!()
    }

    fn bundled_skills() -> Vec<RegisteredSkill> {
        // Return compiled-in default skills
        vec![]
    }
}
```

---

### 14.6 Hot-Reload Mechanism (C4)

**Dependency:**
```toml
# workspace Cargo.toml [workspace.dependencies]
notify = { version = "7.0", features = ["macos_fsevent"] }

# clawft-core/Cargo.toml
[dependencies]
notify = { workspace = true }
```

**File watcher:**

```rust
// clawft-core/src/agent/skill_watcher.rs

use std::path::PathBuf;
use std::sync::Arc;

use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use tokio::sync::mpsc;

use super::skill_registry::SkillRegistry;

/// Watches skill directories for changes and triggers reload.
pub struct SkillWatcher {
    watcher: RecommendedWatcher,
    /// Directories being watched.
    watched_dirs: Vec<PathBuf>,
}

impl SkillWatcher {
    /// Start watching the given directories for SKILL.md changes.
    ///
    /// Returns a receiver that emits `SkillReloadEvent` when changes
    /// are detected. The caller is responsible for rebuilding the
    /// `SkillRegistry` when an event arrives.
    pub fn start(
        dirs: Vec<PathBuf>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<SkillReloadEvent>), notify::Error> {
        let (tx, rx) = mpsc::unbounded_channel();

        let handler_tx = tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_)
                    | EventKind::Modify(_)
                    | EventKind::Remove(_) => {
                        for path in &event.paths {
                            if path.file_name().map(|n| n == "SKILL.md").unwrap_or(false)
                                || path.extension().map(|e| e == "wasm").unwrap_or(false)
                            {
                                let _ = handler_tx.send(SkillReloadEvent {
                                    path: path.clone(),
                                    kind: event.kind.clone(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        })?;

        for dir in &dirs {
            watcher.watch(dir, RecursiveMode::Recursive)?;
        }

        Ok((Self { watcher, watched_dirs: dirs }, rx))
    }
}

/// An event indicating a skill file has changed.
#[derive(Debug, Clone)]
pub struct SkillReloadEvent {
    pub path: PathBuf,
    pub kind: EventKind,
}
```

**Integration with AgentLoop:**

The `AgentLoop` spawns the watcher on startup and listens for events.
When a `SkillReloadEvent` arrives, it calls `SkillRegistry::discover()`
to rebuild the registry. The rebuild is atomic (swap the entire registry).

---

## 15. Workstream I: Type Safety & Cleanup

### 15.1 `DelegationTarget` Serde Consistency (I1)

**Crate:** `clawft-types` | **File:** `src/delegation.rs`

**Current:** Serializes as PascalCase (`"Local"`, `"Claude"`, `"Flow"`, `"Auto"`).

```rust
// CURRENT
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationTarget {
    Local,
    Claude,
    Flow,
    #[default]
    Auto,
}
```

**Fix:** Add `#[serde(rename_all = "snake_case")]` to match all other enums:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationTarget {
    Local,
    Claude,
    Flow,
    #[default]
    Auto,
}
```

**Migration:** This is a breaking change for existing config files. Add
backward-compatible aliases:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationTarget {
    #[serde(alias = "Local")]
    Local,
    #[serde(alias = "Claude")]
    Claude,
    #[serde(alias = "Flow")]
    Flow,
    #[serde(alias = "Auto")]
    #[default]
    Auto,
}
```

**Testing requirements:**
- Unit test: `"local"` deserializes correctly (new format)
- Unit test: `"Local"` deserializes correctly (backward compat)
- Unit test: serialization produces `"local"` (new format)
- Update existing test in `delegation.rs` that asserts PascalCase output

---

### 15.2 String-Typed Policy Modes to Enums (I2)

**Crate:** `clawft-types` | **File:** `src/config.rs` (after B3 split: `config/policies.rs`)

**Current:**
```rust
pub struct CommandPolicyConfig {
    pub mode: String,  // "allowlist" | "denylist" | "disabled"
}

pub struct RateLimitConfig {
    pub strategy: String,  // "fixed_window" | "sliding_window" | "token_bucket"
}
```

**Fix:**

```rust
/// Command policy enforcement mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandPolicyMode {
    /// Only commands in the allowlist may execute.
    #[default]
    Allowlist,
    /// Commands in the denylist are blocked; all others allowed.
    Denylist,
    /// No command policy enforcement.
    Disabled,
}

/// Rate limiting strategy.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStrategy {
    /// Fixed time window (resets at boundary).
    #[default]
    FixedWindow,
    /// Sliding time window (rolling average).
    SlidingWindow,
    /// Token bucket (burst-friendly).
    TokenBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicyConfig {
    #[serde(default)]
    pub mode: CommandPolicyMode,
    // ... other fields ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub strategy: RateLimitStrategy,
    // ... other fields ...
}
```

**Testing requirements:**
- Unit test: each enum variant round-trips through JSON
- Unit test: default values match the previous string defaults

---

### 15.3 `ChatMessage::content` Serialization Fix (I3)

**Crate:** `clawft-llm` | **File:** `src/types.rs`

**Current:**
```rust
pub struct ChatMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,  // None serializes as "content": null
    // ...
}
```

**Fix:**
```rust
pub struct ChatMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // ...
}
```

**Testing requirements:**
- Unit test: assistant message with `content: None` does not emit `"content"` key
- Unit test: assistant message with `content: Some("")` still emits `"content": ""`
- Unit test: user message with content present is unchanged

---

### 15.4 Job ID Collision Fix (I4)

**Crate:** `clawft-cli` | **File:** `src/commands/cron.rs`

**Current:**
```rust
fn generate_job_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("job-{}-{}", now, std::process::id())
}
```

**Fix:**
```rust
fn generate_job_id() -> String {
    format!("job-{}", uuid::Uuid::new_v4())
}
```

`uuid` with feature `v4` is already in workspace dependencies.

**Testing requirements:**
- Unit test: 1000 calls produce 1000 unique IDs

---

### 15.5 `camelCase` Normalizer Acronym Handling (I5)

**Crate:** `clawft-platform` | **File:** `src/config_loader.rs`

**Fix:** Add consecutive-uppercase detection:

```rust
/// Convert a string from camelCase/PascalCase to snake_case,
/// correctly handling consecutive uppercase (acronyms).
///
/// Examples:
///   "HTMLParser" -> "html_parser"
///   "getHTTPSUrl" -> "get_https_url"
///   "simpleCase" -> "simple_case"
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
                let next = chars.get(i + 1);

                // Insert underscore before:
                // - uppercase preceded by lowercase (e.g., "sC" in "simpleCase")
                // - uppercase preceded by uppercase followed by lowercase
                //   (e.g., the "P" in "HTMLParser")
                if prev.is_lowercase()
                    || (prev.is_uppercase()
                        && next.map_or(false, |n| n.is_lowercase()))
                {
                    result.push('_');
                }
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}
```

**Testing requirements:**
- `"HTMLParser"` -> `"html_parser"`
- `"getHTTPSUrl"` -> `"get_https_url"`
- `"simpleCase"` -> `"simple_case"`
- `"XMLHttpRequest"` -> `"xml_http_request"`
- `"already_snake"` -> `"already_snake"`
- `"A"` -> `"a"`
- `""` -> `""`

---

### 15.6 Always-True Test Assertion (I7)

**Crate:** `clawft-core` | **File:** `src/pipeline/transport.rs`

**Current:**
```rust
assert!(result.is_err() || result.is_ok());
```

**Fix:** Replace with specific assertion based on test intent. If the test
expects success:
```rust
assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
```

If the test expects failure:
```rust
assert!(result.is_err(), "expected Err, got: {:?}", result.unwrap());
```

**Testing requirements:**
- The test must actually verify expected behavior

---

### 15.7 Shared `MockTransport` via Feature Flag (I8)

**Crate:** `clawft-services` | **File:** `src/mcp/transport.rs`

**Current:** `MockTransport` is behind `#[cfg(test)]`, invisible to other crates.

**Fix:** Expose behind a `test-utils` feature:

```toml
# clawft-services/Cargo.toml
[features]
default = []
test-utils = []
```

```rust
// clawft-services/src/mcp/transport.rs

/// Mock transport for testing MCP communication.
///
/// Available to downstream crates via the `test-utils` feature.
#[cfg(any(test, feature = "test-utils"))]
pub mod mock {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    pub struct MockTransport {
        responses: Mutex<VecDeque<String>>,
        sent: Mutex<Vec<String>>,
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                responses: Mutex::new(VecDeque::new()),
                sent: Mutex::new(Vec::new()),
            }
        }

        pub fn enqueue_response(&self, response: impl Into<String>) {
            self.responses.lock().unwrap().push_back(response.into());
        }

        pub fn sent_messages(&self) -> Vec<String> {
            self.sent.lock().unwrap().clone()
        }
    }
}
```

**Testing requirements:**
- Downstream crate test: `clawft-cli` can use `MockTransport` with `test-utils` feature

---

## 16. Feature Flags Additions

New feature flags for the plugin system:

```toml
# clawft-core/Cargo.toml
[features]
default = []
# ... existing features ...

# Plugin system
plugin = ["dep:clawft-plugin"]
wasm-plugins = ["plugin", "dep:wasmtime"]

# clawft-cli/Cargo.toml
[features]
default = ["channel-telegram", "all-tools", "plugin"]

# ... existing features ...

# Plugin system
plugin = ["clawft-core/plugin", "clawft-channels/plugin"]
wasm-plugins = ["plugin", "clawft-core/wasm-plugins"]

# Voice (reserved, no-op)
voice = []

# Services
services = ["dep:clawft-services"]

# Delegation
delegate = ["services", "clawft-services/delegation"]
```

---

## 17. Updated Crate Dependency Graph

```
clawft-llm            (standalone library; depends on: reqwest, serde, async-trait)
                      ^ external dependency (git or crates.io)

clawft-types          (zero deps beyond serde)
    |
clawft-plugin         (depends on: types, serde, async-trait)  <- NEW
    |
clawft-platform       (depends on: types)
    |
clawft-core           (depends on: types, platform, plugin, clawft-llm)
    |
    +-- clawft-tools       (depends on: types, platform, core, plugin)
    +-- clawft-channels    (depends on: types, platform, core, plugin)  <- plugin host
    +-- clawft-services    (depends on: types, platform, core)
    |
clawft-cli            (depends on: all above) -> binary: `weft`
clawft-wasm           (depends on: types, platform, core, plugin, clawft-llm)
```

Key changes:
- `clawft-plugin` is a new crate between `clawft-types` and `clawft-platform`
- `clawft-core`, `clawft-tools`, and `clawft-channels` all depend on `clawft-plugin`
- `clawft-plugin` depends only on `clawft-types` (keeps it WASM-safe)
- `wasmtime` dependency is feature-gated behind `wasm-plugins` (not in default build)

---

## 18. Updated Workspace Cargo.toml Additions

```toml
# New workspace members
[workspace]
members = [
    # ... existing members ...
    "crates/clawft-plugin",
]

# New workspace dependencies
[workspace.dependencies]
# ... existing deps ...

# Plugin system
clawft-plugin = { path = "crates/clawft-plugin" }

# WASM plugin host (feature-gated)
wasmtime = { version = "27", features = ["component-model"], optional = true }
wit-bindgen = { version = "0.36", optional = true }

# Skill system
serde_yaml = "0.9"
notify = { version = "7.0", features = ["macos_fsevent"] }

# Critical fixes
fnv = "1.0"
rpassword = "5.0"
temp_env = "0.3"
```

---

## 19. Testing Strategy Additions

| Level | Tool | Scope |
|-------|------|-------|
| Session key encoding | Unit test | A1: encode/decode round-trip for all edge cases |
| Hash stability | Golden test | A2: fixed input produces known output across runs |
| JSON error safety | Unit test | A3: error messages with special chars produce valid JSON |
| Credential redaction | Unit test | A4: Debug output never contains secrets |
| SSRF IP ranges | Unit test | A6: all RFC 1918 + RFC 6598 ranges blocked |
| HTTP timeout | Integration test | A7: unreachable host fails within configured timeout |
| Feature gate compilation | CI job | A9: `--no-default-features` builds cleanly |
| Usage type unification | Cross-crate test | B1: canonical type used everywhere |
| File split | Build test | B3: all splits compile and re-export correctly |
| Policy consolidation | Cross-crate test | B6: single canonical policy type |
| Plugin trait loading | Unit test | C1: plugin manifest deserialization round-trip |
| WASM plugin sandbox | Integration test | C2: WASM module loads, size budget enforced |
| SKILL.md parsing | Unit test | C3: YAML frontmatter + markdown body parsed correctly |
| Skill precedence | Integration test | C4: workspace skills override managed/bundled |
| Hot-reload | Integration test | C4: file change triggers registry rebuild |
| Serde consistency | Unit test | I1: DelegationTarget snake_case + backward compat |
| Enum validation | Unit test | I2: CommandPolicyMode and RateLimitStrategy round-trip |
| Content null fix | Unit test | I3: None content omitted from serialization |
| Job ID uniqueness | Unit test | I4: 1000 UUIDs are distinct |
| Snake case acronyms | Unit test | I5: "HTMLParser" -> "html_parser" |
