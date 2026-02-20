# Workstream A: Critical Fixes -- Security & Data Integrity

## Overview

- **Priority**: P0/P1 (MUST complete Week 1-2)
- **Branch**: `sprint/phase-5-5A`
- **Crates**: clawft-core, clawft-types, clawft-cli, clawft-services, clawft-llm
- **Dependencies**: None (foundation layer)
- **Blocks**: Everything else -- Workstream B, I, J, and all feature elements (04-10)

## Concurrency Plan

### Parallel Group 1 (No interdependencies -- can start simultaneously)
- **A1** (session key encoding) -- clawft-core/session.rs
- **A2** (hash embedder stability) -- clawft-core/embeddings/hash_embedder.rs
- **A3** (error JSON formatting) -- clawft-core/agent/loop_core.rs
- **A5** (API key echo) -- clawft-cli/commands/onboard.rs
- **A6** (SSRF IP ranges) -- clawft-services/mcp/middleware.rs
- **A7** (HTTP timeout) -- clawft-llm/openai_compat.rs
- **A8** (unsafe set_var) -- clawft-core/workspace.rs
- **A9** (no-default-features) -- clawft-cli/mcp_tools.rs

### Sequential Dependency
- **A4** (SecretRef) MUST complete before **B3** (file splits) begins.
  A4 touches `clawft-types/src/config.rs` extensively; the B3 split of config.rs should include the new `secret.rs` module from A4.

All 9 items can technically start in parallel since they touch different files. A4 is the only one with a downstream ordering constraint (blocks B3).

---

## Task Specifications

### A1: Session Key Round-Trip Corruption

**Priority**: P0 | **Crate**: clawft-core | **File**: `crates/clawft-core/src/session.rs`

#### Current Code (Bug)

```rust
// session.rs:343-346
fn session_path(&self, key: &str) -> PathBuf {
    let filename = format!("{}.jsonl", key.replace(':', "_"));
    self.sessions_dir.join(filename)
}
```

```rust
// session.rs:301-303 (inside list_sessions)
if let Some(stem) = name.strip_suffix(".jsonl") {
    // Reverse the filename sanitization: `_` back to `:`.
    keys.push(stem.replace('_', ":"));
}
```

**Reproduction**: A key like `"telegram:user_123"` is saved as `telegram_user_123.jsonl`. When `list_sessions()` reads it back, it decodes as `"telegram:user:123"` -- wrong. Any key containing an underscore will be corrupted on round-trip.

#### Required Changes

1. **Add dependency**: Add `percent-encoding = "2"` to `crates/clawft-core/Cargo.toml`.

2. **Rewrite `session_path()`** (line 343):
   ```rust
   use percent_encoding::{percent_encode, NON_ALPHANUMERIC};

   fn session_path(&self, key: &str) -> PathBuf {
       let encoded = percent_encode(key.as_bytes(), NON_ALPHANUMERIC).to_string();
       let filename = format!("{encoded}.jsonl");
       self.sessions_dir.join(filename)
   }
   ```

3. **Rewrite `list_sessions()`** (line 301-303):
   ```rust
   use percent_encoding::percent_decode_str;

   if let Some(stem) = name.strip_suffix(".jsonl") {
       match percent_decode_str(stem).decode_utf8() {
           Ok(decoded) => keys.push(decoded.into_owned()),
           Err(e) => {
               warn!(filename = %name, error = %e, "skipping undecodable session filename");
           }
       }
   }
   ```

4. **Add migration in `load_session()`**: Before loading, check if the percent-encoded path exists. If not, check for the old underscore-encoded path. If the old path exists, rename it to the new path and log a migration message:
   ```rust
   pub async fn load_session(&self, key: &str) -> clawft_types::Result<Session> {
       crate::security::validate_session_id(key)?;
       let path = self.session_path(key);

       // Migration: try old-format filename if new-format doesn't exist
       if !self.platform.fs().exists(&path).await {
           let old_filename = format!("{}.jsonl", key.replace(':', "_"));
           let old_path = self.sessions_dir.join(&old_filename);
           if self.platform.fs().exists(&old_path).await {
               warn!(
                   key = key,
                   old = %old_path.display(),
                   new = %path.display(),
                   "migrating session file from old encoding format"
               );
               // Read from old, write to new, keep old for safety
               let content = self.platform.fs().read_to_string(&old_path).await?;
               self.platform.fs().write_string(&path, &content).await?;
           }
       }

       let content = self.platform.fs().read_to_string(&path).await?;
       // ... rest of existing load logic unchanged
   }
   ```

#### New Dependencies

Add to `crates/clawft-core/Cargo.toml`:
```toml
percent-encoding = "2"
```

#### Test Specification

1. **Round-trip with underscores**: Key `"telegram:user_123"` saves and reloads correctly, producing `"telegram:user_123"` (not `"telegram:user:123"`).
2. **Round-trip with colons**: Key `"slack:channel:thread"` survives round-trip.
3. **Round-trip with special chars**: Key `"discord:guild/channel#123"` survives round-trip.
4. **list_sessions with mixed formats**: Both old-format and new-format files appear correctly in listing.
5. **Migration**: Save a file with old underscore encoding, verify `load_session` reads it via migration path.
6. **Property test** (if proptest available): Any valid session key (passing `validate_session_id`) survives `session_path` -> `list_sessions` round-trip.

#### Acceptance Criteria

- [ ] `session_path("telegram:user_123")` produces a filename that decodes back to `"telegram:user_123"` in `list_sessions()`
- [ ] Old-format `.jsonl` files are automatically readable (migration path)
- [ ] No data loss for existing sessions
- [ ] All existing session tests pass
- [ ] New round-trip tests with underscore-containing keys pass

#### Migration Requirements

- Old-format files (`telegram_user_123.jsonl`) must be auto-migrated on first access via `load_session()`.
- Both old and new format files are readable during the migration period.
- The old file is preserved (not deleted) after migration for safety. A future cleanup task can remove old files.

---

### A2: Unstable Hash Function in Embeddings

**Priority**: P0 | **Crate**: clawft-core | **File**: `crates/clawft-core/src/embeddings/hash_embedder.rs`

#### Current Code (Bug)

```rust
// hash_embedder.rs:11
use std::hash::{DefaultHasher, Hash, Hasher};

// hash_embedder.rs:58-61
for word in &words {
    let mut hasher = DefaultHasher::new();
    word.to_lowercase().hash(&mut hasher);
    let hash = hasher.finish();
    // ...
}
```

**Problem**: `std::collections::hash_map::DefaultHasher` uses SipHash-1-3 internally, but the Rust standard library explicitly makes NO stability guarantees about its output across versions, platforms, or even different program runs (with randomization). Persisted embeddings become silently invalid after a toolchain upgrade.

#### Required Changes

1. **Add dependency**: Add `fnv = "1"` to `crates/clawft-core/Cargo.toml`.

2. **Replace import** (line 11):
   ```rust
   // Before:
   use std::hash::{DefaultHasher, Hash, Hasher};
   // After:
   use std::hash::{Hash, Hasher};
   use fnv::FnvHasher;
   ```

3. **Replace hasher construction** (line 59):
   ```rust
   // Before:
   let mut hasher = DefaultHasher::new();
   // After:
   let mut hasher = FnvHasher::default();
   ```

4. **Update module docs** (line 4): Change `DefaultHasher` reference to `FnvHasher`.

#### New Dependencies

Add to `crates/clawft-core/Cargo.toml`:
```toml
fnv = "1"
```

FNV-1a is deterministic, platform-independent, and produces the same output for the same input regardless of Rust version or target triple. The `fnv` crate is a single-file crate with zero dependencies.

#### Test Specification

1. **Golden test**: Assert that `HashEmbedder::default_dimension().compute_embedding("hello world")` produces a specific pinned output vector (first 8 and last 8 elements). Record the golden values after the FNV migration, then freeze them.
   ```rust
   #[test]
   fn golden_test_hello_world() {
       let embedder = HashEmbedder::default_dimension();
       let emb = embedder.compute_embedding("hello world");
       // Pin the first few values (recorded from FNV-1a output)
       assert_eq!(emb[0], EXPECTED_FIRST_VALUE);
       assert_eq!(emb[1], EXPECTED_SECOND_VALUE);
       // ... etc for at least 8 dimensions
   }
   ```

2. **Cross-platform note**: Add a comment in the golden test explaining that the same test should produce identical results on x86_64-linux, aarch64-linux, and x86_64-darwin. CI should verify this.

3. **Existing tests still pass**: The `deterministic_same_text_same_embedding` and all other existing tests must pass (they test properties, not specific values, so they are unaffected by the hash change).

#### Acceptance Criteria

- [ ] `DefaultHasher` is no longer used in hash_embedder.rs
- [ ] `FnvHasher` is used instead
- [ ] Golden test with pinned output passes
- [ ] All existing embedding tests pass
- [ ] Module docs updated

#### Migration Requirements

- Embeddings generated with the old `DefaultHasher` are silently incompatible with the new `FnvHasher`. Any persisted embeddings should be re-computed.
- If the embedding store supports versioning, add a version tag. If not, log a warning on load when embeddings appear stale (this is a best-effort -- the embeddings are still valid vectors, just not consistent with fresh computations).

---

### A3: Invalid JSON from Error Formatting

**Priority**: P1 | **Crate**: clawft-core | **File**: `crates/clawft-core/src/agent/loop_core.rs`

#### Current Code (Bug)

```rust
// loop_core.rs:393
format!("{{\"error\": \"{}\"}}", e)
```

**Problem**: If the error message `e` contains a double-quote (`"`), backslash (`\`), or newline, the output is malformed JSON. For example, an error `file "foo" not found` produces `{"error": "file "foo" not found"}` which is unparseable.

#### Required Changes

Replace line 393 with:

```rust
// Before:
format!("{{\"error\": \"{}\"}}", e)

// After:
serde_json::json!({"error": e.to_string()}).to_string()
```

`serde_json::json!` properly escapes all special characters in the error string. `serde_json` is already a dependency of this crate.

#### New Dependencies

None (serde_json is already imported).

#### Test Specification

1. **Error with double quotes**: Error message `file "foo" not found` produces valid JSON.
2. **Error with backslashes**: Error message `path C:\Users\test` produces valid JSON.
3. **Error with newlines**: Error message `line 1\nline 2` produces valid JSON.
4. **Error with all special chars**: Error message containing `"`, `\`, `\n`, `\t`, and null byte produces valid, parseable JSON.
5. **Verify JSON structure**: Parsed result has a single `"error"` key with the full error message as value.

```rust
#[test]
fn error_json_escapes_quotes() {
    let error_msg = r#"file "foo" not found"#;
    let json_str = serde_json::json!({"error": error_msg}).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["error"].as_str().unwrap(), error_msg);
}
```

#### Acceptance Criteria

- [ ] `format!("{{\"error\"` pattern no longer exists in loop_core.rs
- [ ] `serde_json::json!` is used for error formatting
- [ ] Error messages containing `"`, `\`, and newlines produce valid JSON
- [ ] All existing agent loop tests pass

#### Migration Requirements

None.

---

### A4: Plaintext Credentials in Config Structs

**Priority**: P0 | **Crate**: clawft-types | **File**: `crates/clawft-types/src/config.rs`

#### Current Code (Bug)

The following fields store plaintext secrets that appear in serialized JSON, Debug output, and audit logs:

| Line | Struct | Field | Description |
|------|--------|-------|-------------|
| 191 | `TelegramConfig` | `token` | Bot token |
| 224 | `SlackConfig` | `bot_token` | Slack bot token |
| 233 | `SlackConfig` | `app_token` | Slack app token |
| 324 | `DiscordConfig` | `token` | Discord bot token |
| 377 | `WhatsAppConfig` | `bridge_token` | Bridge auth token |
| 412 | `FeishuConfig` | `app_secret` | Feishu app secret |
| 420 | `FeishuConfig` | `verification_token` | Event verification token |
| 440 | `DingTalkConfig` | `client_secret` | DingTalk app secret |
| 520 | `MochatConfig` | `claw_token` | Mochat auth token |
| 641 | `EmailConfig` | `imap_password` | IMAP password |
| 666 | `EmailConfig` | `smtp_password` | SMTP password |
| 766 | `QQConfig` | `secret` | QQ bot app secret |
| 780 | `ProviderConfig` | `api_key` | LLM provider API key |
| 940 | `WebSearchConfig` | `api_key` | Search API key |

Example: `#[derive(Debug)]` on `TelegramConfig` (line 183) causes the token to appear in any `format!("{:?}", config)` call, including log statements.

#### Required Changes

This is a large change with multiple steps:

**Step 1: Create `SecretString` type**

Create a new file `crates/clawft-types/src/secret.rs`:

```rust
//! Secret string wrapper that prevents accidental exposure.

use std::fmt;
use serde::{Deserialize, Serialize, Serializer, Deserializer};

/// A string value that should not appear in logs, Debug output, or serialized JSON.
///
/// - `Debug` prints `[REDACTED]`
/// - `Serialize` skips the field (serializes as empty string)
/// - `Deserialize` accepts a plain string (backward compatible)
/// - `Display` prints `[REDACTED]`
/// - `expose()` returns the inner value for actual use
#[derive(Clone, Default)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the actual secret value. Use sparingly and only where needed.
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "\"\"")
        } else {
            write!(f, "\"[REDACTED]\"")
        }
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "")
        } else {
            write!(f, "[REDACTED]")
        }
    }
}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Never serialize the actual secret value
        serializer.serialize_str("")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(SecretString(s))
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        SecretString(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        SecretString(s.to_string())
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
```

**Step 2: Register the module**

Add `pub mod secret;` to `crates/clawft-types/src/lib.rs`.

**Step 3: Replace secret fields**

Change each secret field from `pub [field]: String` to `pub [field]: SecretString` for all fields listed in the table above. Each field should use `#[serde(default)]` (already present on all of them).

Example for `TelegramConfig`:
```rust
// Before:
pub token: String,
// After:
pub token: SecretString,
```

**Step 4: Update all call sites**

Search for all uses of these fields across the codebase. Replace:
- `config.telegram.token` with `config.telegram.token.expose()` where the actual value is needed (HTTP headers, API calls)
- `config.telegram.token.is_empty()` works directly (implemented on SecretString)

**Step 5: Update existing tests**

Tests that compare secret field values (e.g., `assert_eq!(cfg.channels.telegram.token, "test-bot-token-123")`) need to use `.expose()`:
```rust
assert_eq!(cfg.channels.telegram.token.expose(), "test-bot-token-123");
```

Note: The `clawft-llm` crate already follows the env-var pattern (`api_key_env: String`) in `ProviderConfig`. The `ProviderConfig` in `clawft-types/src/config.rs` (line 777-789) is a different struct -- it stores the literal `api_key` for deserialization from config files. This one needs `SecretString`.

#### New Dependencies

None (pure Rust implementation using existing serde traits).

#### Test Specification

1. **Serialization excludes secrets**: `serde_json::to_string(&config)` does not contain any token/password values.
2. **Debug output shows REDACTED**: `format!("{:?}", config)` shows `[REDACTED]` for non-empty secrets.
3. **Debug output shows empty for empty**: `format!("{:?}", SecretString::default())` shows `""`.
4. **Backward compat deserialization**: Old config JSON with literal secret strings deserializes without error.
5. **expose() returns actual value**: `SecretString::new("secret").expose()` returns `"secret"`.
6. **is_empty() works**: `SecretString::default().is_empty()` is true.
7. **CI lint grep**: No `pub String` fields matching the pattern `(password|secret|token|api_key|auth_token|bridge_token|claw_token|verification_token|client_secret|app_secret|bot_token|app_token)` exist without being `SecretString`.

#### Acceptance Criteria

- [ ] `SecretString` type exists in `clawft-types/src/secret.rs`
- [ ] All 14 secret fields use `SecretString` instead of `String`
- [ ] `Debug` output never contains actual secret values
- [ ] `Serialize` output never contains actual secret values
- [ ] Old config format (literal secrets in JSON) still deserializes
- [ ] All existing config tests pass (with `.expose()` adjustments)
- [ ] All call sites updated to use `.expose()` where the value is needed
- [ ] CI lint rule rejects new plaintext secret fields

#### Migration Requirements

- **Backward compatibility**: `SecretString`'s `Deserialize` impl accepts plain strings, so existing config.json files with literal secrets continue to work without changes.
- **Forward change**: Serialized config will show empty strings for secret fields instead of the actual values. Users who relied on serialized config roundtrips containing secrets will need to re-enter them or use env vars.
- **Deprecation warning**: Consider logging a warning when a non-empty secret is deserialized from a literal string field, advising users to use `*_env` variants instead. (This can be a follow-up if the core change is large enough.)

---

### A5: API Key Echoed During Onboarding

**Priority**: P0 | **Crate**: clawft-cli | **File**: `crates/clawft-cli/src/commands/onboard.rs`

#### Current Code (Bug)

```rust
// onboard.rs:156-161
print!("API key (leave blank to skip): ");
io::stdout().flush()?;

let mut key = String::new();
io::stdin().lock().read_line(&mut key)?;
let key = key.trim().to_owned();
```

**Problem**: `read_line()` on stdin echoes input to the terminal. When a user types their API key, it is visible on screen and in terminal scrollback buffers. This is a security issue -- API keys should never be visible during input.

#### Required Changes

1. **Add dependency**: Add `rpassword = "5"` to `crates/clawft-cli/Cargo.toml`.

2. **Replace the key input block** (lines 156-161):
   ```rust
   // Before:
   print!("API key (leave blank to skip): ");
   io::stdout().flush()?;
   let mut key = String::new();
   io::stdin().lock().read_line(&mut key)?;
   let key = key.trim().to_owned();

   // After:
   let key = if atty::is(atty::Stream::Stdin) {
       // Interactive terminal: suppress echo
       rpassword::prompt_password("API key (leave blank to skip): ")
           .unwrap_or_default()
           .trim()
           .to_owned()
   } else {
       // Non-interactive (CI, piped input): fall back to read_line
       print!("API key (leave blank to skip): ");
       io::stdout().flush()?;
       let mut key = String::new();
       io::stdin().lock().read_line(&mut key)?;
       key.trim().to_owned()
   };
   ```

   Alternatively, if adding `atty` as a dependency is undesirable, just use `rpassword::prompt_password` directly -- it handles non-TTY gracefully by falling back to regular stdin reading:
   ```rust
   let key = rpassword::prompt_password("API key (leave blank to skip): ")
       .unwrap_or_default()
       .trim()
       .to_owned();
   ```

#### New Dependencies

Add to `crates/clawft-cli/Cargo.toml`:
```toml
rpassword = "5"
```

Optionally, if TTY detection is needed:
```toml
atty = "0.2"
```

#### Test Specification

1. **Compile check**: The `rpassword` import compiles without error.
2. **CI note**: CI environments do not have a TTY. `rpassword::prompt_password` will fail with an error in non-TTY contexts. The `--yes` flag skips the prompt entirely, so CI tests should use `--yes`. Add a comment in the code explaining this.
3. **Manual verification**: Run `weft onboard` interactively and verify the API key is not visible while typing.

#### Acceptance Criteria

- [ ] `io::stdin().lock().read_line()` is no longer used for API key input
- [ ] `rpassword` is used for API key prompts
- [ ] API key is not echoed to terminal during input
- [ ] `--yes` flag still works (skips the prompt)
- [ ] Non-TTY fallback is handled gracefully (no panic)

#### Migration Requirements

None.

---

### A6: Incomplete Private IP Range in SSRF Protection

**Priority**: P0 | **Crate**: clawft-services | **File**: `crates/clawft-services/src/mcp/middleware.rs`

#### Current Code (Bug)

```rust
// middleware.rs:197-205
if host.starts_with("10.")
    || host.starts_with("192.168.")
    || host.starts_with("172.16.")
    || host.starts_with("127.")
    || host == "0.0.0.0"
    || host == "localhost"
{
    return Err(format!("blocked private address: {host}"));
}
```

**Problem**: RFC 1918 defines `172.16.0.0/12` as private, covering `172.16.0.0` through `172.31.255.255`. The current check only blocks `172.16.*`. URLs like `http://172.30.0.1/` bypass the SSRF protection entirely.

Additional gaps:
- **IPv4-mapped IPv6**: `http://[::ffff:10.0.0.1]/` bypasses all checks because the host is `::ffff:10.0.0.1`, which doesn't match any `starts_with` pattern.
- **IPv6 loopback**: `http://[::1]/` is extracted as `::1` by `extract_host()`, which doesn't match `127.*` or `localhost`.
- **Cloud metadata via IPv6**: `http://[::ffff:169.254.169.254]/` bypasses the metadata check.

#### Required Changes

Replace the private IP check block (lines 196-205) with:

```rust
// Block obvious private IP literals.
if is_private_or_loopback(&host) {
    return Err(format!("blocked private address: {host}"));
}
```

Add a helper function:

```rust
/// Check if a host string represents a private, loopback, or link-local address.
///
/// Handles:
/// - IPv4: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 127.0.0.0/8
/// - IPv6 loopback: ::1
/// - IPv4-mapped IPv6: ::ffff:x.x.x.x (converts to IPv4 and re-checks)
/// - Link-local: 169.254.0.0/16
/// - Special: 0.0.0.0, localhost
fn is_private_or_loopback(host: &str) -> bool {
    // Direct matches
    if host == "0.0.0.0" || host == "localhost" || host == "::1" {
        return true;
    }

    // IPv4-mapped IPv6: ::ffff:x.x.x.x
    if let Some(ipv4_part) = host.strip_prefix("::ffff:") {
        return is_private_or_loopback(ipv4_part);
    }

    // Parse IPv4 octets
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 {
        if let (Ok(o1), Ok(o2)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
            // 10.0.0.0/8
            if o1 == 10 {
                return true;
            }
            // 172.16.0.0/12 (172.16.* through 172.31.*)
            if o1 == 172 && (16..=31).contains(&o2) {
                return true;
            }
            // 192.168.0.0/16
            if o1 == 192 && o2 == 168 {
                return true;
            }
            // 127.0.0.0/8
            if o1 == 127 {
                return true;
            }
            // 169.254.0.0/16 (link-local)
            if o1 == 169 && o2 == 254 {
                return true;
            }
        }
    }

    false
}
```

Also add `169.254.0.0/16` to the private check (link-local addresses used for cloud metadata). Note: `169.254.169.254` is already caught by `METADATA_HOSTS`, but other link-local addresses are not.

#### New Dependencies

None.

#### Test Specification

1. **RFC 1918 full range**: `172.16.0.1` through `172.31.255.255` are all blocked.
2. **172.16.0.1 blocked**: Existing behavior preserved.
3. **172.30.0.1 blocked**: Previously bypassed, now caught.
4. **172.31.255.1 blocked**: Edge of the /12 range.
5. **172.32.0.1 allowed**: Just outside the /12 range.
6. **172.15.255.255 allowed**: Just below the /12 range.
7. **IPv4-mapped IPv6**: `::ffff:10.0.0.1` is blocked.
8. **IPv4-mapped IPv6 192.168**: `::ffff:192.168.1.1` is blocked.
9. **IPv6 loopback**: `::1` is blocked (via extract_host from `http://[::1]/`).
10. **Cloud metadata**: `169.254.169.254` is blocked (already via METADATA_HOSTS, add redundant private check).
11. **Link-local**: `169.254.1.1` is blocked.
12. **Public IPs allowed**: `8.8.8.8`, `1.1.1.1`, `172.32.0.1` all pass.
13. **SecurityGuard integration**: Verify `SecurityGuard::before_call` rejects `web_fetch` with `http://172.30.0.1/`.

```rust
#[test]
fn url_policy_blocks_full_172_range() {
    let policy = UrlPolicy::default();
    // Full range: 172.16.0.0 - 172.31.255.255
    assert!(policy.validate("http://172.16.0.1").is_err());
    assert!(policy.validate("http://172.20.0.1").is_err());
    assert!(policy.validate("http://172.30.0.1").is_err());
    assert!(policy.validate("http://172.31.255.1").is_err());
    // Outside range
    assert!(policy.validate("http://172.15.255.255").is_ok());
    assert!(policy.validate("http://172.32.0.1").is_ok());
}

#[test]
fn url_policy_blocks_ipv4_mapped_ipv6() {
    let policy = UrlPolicy::default();
    assert!(policy.validate("http://[::ffff:10.0.0.1]/").is_err());
    assert!(policy.validate("http://[::ffff:192.168.1.1]/").is_err());
    assert!(policy.validate("http://[::ffff:172.30.0.1]/").is_err());
}

#[test]
fn url_policy_blocks_ipv6_loopback() {
    let policy = UrlPolicy::default();
    assert!(policy.validate("http://[::1]/").is_err());
    assert!(policy.validate("http://[::1]:8080/").is_err());
}

#[test]
fn url_policy_blocks_link_local() {
    let policy = UrlPolicy::default();
    assert!(policy.validate("http://169.254.1.1").is_err());
    assert!(policy.validate("http://169.254.169.254").is_err());
}
```

#### Acceptance Criteria

- [ ] `starts_with("172.16.")` replaced with proper /12 range check
- [ ] `172.30.0.1` is blocked
- [ ] `::ffff:10.0.0.1` is blocked
- [ ] `::1` is blocked
- [ ] `169.254.169.254` is blocked (via both METADATA_HOSTS and private range check)
- [ ] `172.32.0.1` is NOT blocked (just outside the /12 range)
- [ ] All existing SSRF tests pass

#### Migration Requirements

None. This is a pure security hardening with no backward compatibility concerns.

---

### A7: No HTTP Request Timeout on LLM Provider Client

**Priority**: P1 | **Crate**: clawft-llm | **File**: `crates/clawft-llm/src/openai_compat.rs`

#### Current Code (Bug)

```rust
// openai_compat.rs:52
http: reqwest::Client::new(),

// openai_compat.rs:64
http: reqwest::Client::new(),
```

**Problem**: `reqwest::Client::new()` creates a client with no timeout. If an LLM provider's server accepts the TCP connection but never sends a response (e.g., load balancer hang, provider outage), the request blocks the agent loop indefinitely. There is no way to recover without killing the process.

#### Required Changes

1. **Define default timeout constant**:
   ```rust
   /// Default timeout for LLM API requests (2 minutes).
   const DEFAULT_TIMEOUT_SECS: u64 = 120;
   ```

2. **Replace `Client::new()` with `ClientBuilder`** (lines 52 and 64):
   ```rust
   // Before:
   http: reqwest::Client::new(),

   // After:
   http: reqwest::ClientBuilder::new()
       .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
       .build()
       .expect("failed to build reqwest client"),
   ```

3. **Make timeout configurable via `ProviderConfig`**: Add an optional `timeout_secs` field to the `ProviderConfig` struct in `crates/clawft-llm/src/config.rs`:
   ```rust
   /// Request timeout in seconds. Defaults to 120.
   #[serde(default)]
   pub timeout_secs: Option<u64>,
   ```

   Then use it in client construction:
   ```rust
   let timeout_secs = config.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
   http: reqwest::ClientBuilder::new()
       .timeout(std::time::Duration::from_secs(timeout_secs))
       .build()
       .expect("failed to build reqwest client"),
   ```

#### New Dependencies

None (reqwest::ClientBuilder is part of the existing reqwest dependency).

#### Test Specification

1. **Client has timeout**: Verify that the constructed client has a non-default (non-infinite) timeout by checking that a request to a non-responding server times out.
2. **Custom timeout from config**: Verify that `timeout_secs: Some(30)` in ProviderConfig results in a 30-second timeout.
3. **Default timeout when absent**: Verify that `timeout_secs: None` uses the 120-second default.

Note: Actually testing timeout behavior requires a mock HTTP server that hangs, which may be too heavy for unit tests. The primary verification is code-level: `reqwest::Client::new()` no longer appears in the file.

```rust
#[test]
fn client_uses_timeout() {
    let config = test_config();
    let provider = OpenAiCompatProvider::new(config);
    // Verify Client was built (it would panic if ClientBuilder failed)
    assert_eq!(provider.name(), "test-provider");
}
```

#### Acceptance Criteria

- [ ] `reqwest::Client::new()` no longer appears in openai_compat.rs
- [ ] `reqwest::ClientBuilder::new().timeout(...)` is used instead
- [ ] Default timeout is 120 seconds
- [ ] Timeout is configurable via `ProviderConfig::timeout_secs`
- [ ] All existing provider tests pass

#### Migration Requirements

None. Existing configs without `timeout_secs` get the 120-second default.

---

### A8: `unsafe std::env::set_var` in Parallel Tests

**Priority**: P1 | **Crate**: clawft-core | **File**: `crates/clawft-core/src/workspace.rs`

#### Current Code (Bug)

```rust
// workspace.rs:328-330
unsafe { std::env::set_var("CLAWFT_WORKSPACE", dir.to_str().unwrap()) };
let result = discover_workspace();
unsafe { std::env::remove_var("CLAWFT_WORKSPACE") };

// workspace.rs:341-343
unsafe { std::env::set_var("CLAWFT_WORKSPACE", "/nonexistent/path/for/test") };
let result = discover_workspace();
unsafe { std::env::remove_var("CLAWFT_WORKSPACE") };
```

**Problem**: `std::env::set_var` is `unsafe` in Rust 2024 edition because environment variables are process-global state. Calling it from parallel test threads is undefined behavior (data race). Tokio's default test runner runs tests in parallel, and `cargo test` also uses parallel threads by default.

#### Required Changes

1. **Add dev-dependency**: Add `temp-env = "0.3"` to `crates/clawft-core/Cargo.toml` under `[dev-dependencies]`.

2. **Replace unsafe blocks** in `discover_workspace_env_var` test (lines 322-336):
   ```rust
   // Before:
   #[test]
   fn discover_workspace_env_var() {
       let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
       let dir = std::env::temp_dir().join(format!("clawft-test-discover-env-{n}"));
       let _ = std::fs::remove_dir_all(&dir);
       std::fs::create_dir_all(dir.join(".clawft")).unwrap();

       unsafe { std::env::set_var("CLAWFT_WORKSPACE", dir.to_str().unwrap()) };
       let result = discover_workspace();
       unsafe { std::env::remove_var("CLAWFT_WORKSPACE") };

       assert_eq!(result, Some(dir.clone()));
       let _ = std::fs::remove_dir_all(&dir);
   }

   // After:
   #[test]
   fn discover_workspace_env_var() {
       let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
       let dir = std::env::temp_dir().join(format!("clawft-test-discover-env-{n}"));
       let _ = std::fs::remove_dir_all(&dir);
       std::fs::create_dir_all(dir.join(".clawft")).unwrap();

       let result = temp_env::with_var("CLAWFT_WORKSPACE", Some(dir.to_str().unwrap()), || {
           discover_workspace()
       });

       assert_eq!(result, Some(dir.clone()));
       let _ = std::fs::remove_dir_all(&dir);
   }
   ```

3. **Replace unsafe blocks** in `discover_workspace_env_var_invalid_skipped` test (lines 339-348):
   ```rust
   // After:
   #[test]
   fn discover_workspace_env_var_invalid_skipped() {
       let result = temp_env::with_var(
           "CLAWFT_WORKSPACE",
           Some("/nonexistent/path/for/test"),
           || discover_workspace(),
       );
       assert!(result.is_some());
       assert_ne!(result.unwrap(), PathBuf::from("/nonexistent/path/for/test"));
   }
   ```

4. **Also fix** the same pattern in `crates/clawft-llm/src/openai_compat.rs` (lines 479-485):
   ```rust
   // openai_compat.rs:479-485 (test: resolve_api_key_from_env)
   // Before:
   unsafe { std::env::set_var(env_var, "sk-from-env") };
   // ...
   unsafe { std::env::remove_var(env_var) };

   // After (add temp-env to clawft-llm dev-dependencies):
   let key = temp_env::with_var(env_var, Some("sk-from-env"), || {
       provider.resolve_api_key().unwrap()
   });
   assert_eq!(key, "sk-from-env");
   ```

#### New Dependencies

Add to `crates/clawft-core/Cargo.toml` under `[dev-dependencies]`:
```toml
temp-env = "0.3"
```

Also add to `crates/clawft-llm/Cargo.toml` under `[dev-dependencies]`:
```toml
temp-env = "0.3"
```

#### Test Specification

1. **No unsafe blocks in test code**: `grep -r "unsafe" crates/clawft-core/src/workspace.rs` returns zero results.
2. **Tests still pass**: All three workspace discovery tests pass with `temp_env`.
3. **Parallel safety**: Running `cargo test -p clawft-core` with default parallelism does not produce intermittent failures.

#### Acceptance Criteria

- [ ] `unsafe { std::env::set_var(...) }` no longer appears in workspace.rs
- [ ] `unsafe { std::env::remove_var(...) }` no longer appears in workspace.rs
- [ ] `temp_env::with_var` is used instead
- [ ] All workspace discovery tests pass
- [ ] No `unsafe` blocks remain in test code across the affected crates
- [ ] `unsafe { std::env::set_var(...) }` in openai_compat.rs tests is also fixed

#### Migration Requirements

None.

---

### A9: `--no-default-features` Does Not Compile

**Priority**: P1 | **Crate**: clawft-cli | **File**: `crates/clawft-cli/src/mcp_tools.rs`

#### Current Code (Bug)

```rust
// mcp_tools.rs:13-14
use clawft_services::mcp::transport::{HttpTransport, StdioTransport};
use clawft_services::mcp::{McpSession, ToolDefinition};
```

These imports are unconditional, but `clawft-services` is an optional dependency gated behind the `services` feature:

```toml
# Cargo.toml:
[features]
default = ["channels", "services"]
services = ["dep:clawft-services"]
```

Running `cargo build -p clawft-cli --no-default-features` produces 11 compilation errors because `clawft_services` doesn't exist.

#### Required Changes

1. **Gate all `clawft_services` imports and functions** in `mcp_tools.rs` behind `#[cfg(feature = "services")]`:

```rust
// mcp_tools.rs -- full file restructure:

//! MCP tool wrapper for bridging MCP server tools into the tool registry.

#[cfg(feature = "services")]
mod inner {
    // ALL existing code from mcp_tools.rs goes here, unchanged
    use std::sync::Arc;
    use async_trait::async_trait;
    use tracing::warn;
    use clawft_core::tools::registry::{Tool, ToolError};
    use clawft_services::mcp::transport::{HttpTransport, StdioTransport};
    use clawft_services::mcp::{McpSession, ToolDefinition};
    use clawft_types::config::MCPServerConfig;

    // ... all existing structs and functions ...

    pub use super::*; // re-export everything
}

#[cfg(feature = "services")]
pub use inner::*;
```

Alternatively, a simpler approach -- wrap the entire module contents in `#[cfg(feature = "services")]` blocks:

```rust
#[cfg(feature = "services")]
use std::sync::Arc;

#[cfg(feature = "services")]
use async_trait::async_trait;
#[cfg(feature = "services")]
use tracing::warn;

#[cfg(feature = "services")]
use clawft_core::tools::registry::{Tool, ToolError};
#[cfg(feature = "services")]
use clawft_services::mcp::transport::{HttpTransport, StdioTransport};
#[cfg(feature = "services")]
use clawft_services::mcp::{McpSession, ToolDefinition};
#[cfg(feature = "services")]
use clawft_types::config::MCPServerConfig;

// ... gate every struct/fn/impl with #[cfg(feature = "services")]
```

2. **Provide no-op stubs** for the public API when the feature is off (same pattern as `register_delegation` at lines 275-281):

```rust
/// No-op: MCP tools require the `services` feature.
#[cfg(not(feature = "services"))]
pub async fn register_mcp_tools(
    _config: &clawft_types::config::Config,
    _registry: &mut clawft_core::tools::registry::ToolRegistry,
) {
    // MCP services feature not compiled in.
}
```

3. **Check all call sites** in `main.rs` or `commands/*.rs` that call `register_mcp_tools()` or `create_mcp_client()` -- ensure they are also gated with `#[cfg(feature = "services")]`.

#### New Dependencies

None.

#### Test Specification

1. **Primary test**: `cargo check -p clawft-cli --no-default-features` succeeds with 0 errors.
2. **Default features still work**: `cargo check -p clawft-cli` succeeds.
3. **All features**: `cargo check -p clawft-cli --all-features` succeeds.
4. **Run tests**: `cargo test -p clawft-cli` passes (default features).

```bash
# CI validation:
cargo check -p clawft-cli --no-default-features
cargo check -p clawft-cli
cargo test -p clawft-cli
```

#### Acceptance Criteria

- [ ] `cargo check -p clawft-cli --no-default-features` succeeds
- [ ] `cargo check -p clawft-cli` succeeds (default features)
- [ ] All `clawft_services` imports are gated behind `#[cfg(feature = "services")]`
- [ ] No-op stub for `register_mcp_tools` exists when feature is off
- [ ] All existing tests pass with default features

#### Migration Requirements

None. This is a build system fix only.

---

## Exit Criteria

From the orchestrator document:

- [ ] All P0 items (A1, A2, A4, A5, A6) resolved and tested
- [ ] All P1 items (A3, A7, A8, A9) resolved and tested
- [ ] Zero clippy warnings in modified crates
- [ ] All existing tests still pass
- [ ] No plaintext credentials in Debug output or serialized JSON

### Migration-Specific Exit Criteria

- [ ] **A1**: Existing session files using underscore encoding are auto-migrated to percent-encoded form on first access. Both old and new format files are readable during migration.
- [ ] **A2**: A golden test asserts that `compute_embedding("hello world")` produces a specific known output vector, identical across x86_64-linux, aarch64-linux, and x86_64-darwin. Embeddings with the old hash trigger a warning on load.
- [ ] **A4**: Config files using the old `"imap_password": "literal_string"` format deserialize without error. Backward compatibility is maintained during migration.

### Security Exit Criteria

- [ ] SSRF check blocks `::ffff:10.0.0.1` (IPv4-mapped IPv6 bypass)
- [ ] SSRF check blocks `169.254.169.254` (cloud metadata endpoint)
- [ ] SSRF check blocks `172.30.0.1` (full RFC 1918 172.16.0.0/12 range)
- [ ] SSRF check blocks `[::1]` (IPv6 loopback)
- [ ] No credential `String` fields exist in config structs without `SecretString` wrapper (verified by CI lint)
- [ ] API key input during onboarding is not echoed to terminal

## Development Notes

Record progress in: `.planning/development_notes/02-improvements-overview/element-03/`

### File Change Summary

| Item | Files Modified | Files Created |
|------|---------------|---------------|
| A1 | `clawft-core/src/session.rs`, `clawft-core/Cargo.toml` | -- |
| A2 | `clawft-core/src/embeddings/hash_embedder.rs`, `clawft-core/Cargo.toml` | -- |
| A3 | `clawft-core/src/agent/loop_core.rs` | -- |
| A4 | `clawft-types/src/config.rs`, `clawft-types/src/lib.rs`, all call sites | `clawft-types/src/secret.rs` |
| A5 | `clawft-cli/src/commands/onboard.rs`, `clawft-cli/Cargo.toml` | -- |
| A6 | `clawft-services/src/mcp/middleware.rs` | -- |
| A7 | `clawft-llm/src/openai_compat.rs`, `clawft-llm/src/config.rs` | -- |
| A8 | `clawft-core/src/workspace.rs`, `clawft-core/Cargo.toml`, `clawft-llm/src/openai_compat.rs`, `clawft-llm/Cargo.toml` | -- |
| A9 | `clawft-cli/src/mcp_tools.rs` | -- |
