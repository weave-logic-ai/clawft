# Workstream A: Security & Data Integrity -- Notes

## Completion Status: ALL 9 ITEMS DONE (2026-02-20)

### A1: Session Key Percent-Encoding
- **File**: `crates/clawft-core/src/session.rs`
- **Fix**: Replaced `replace(':', "_")` with percent-encoding (`%3A` for `:`, `%25` for `%`)
- **Note**: `list_sessions()` now decodes `%XX` sequences correctly. Round-trip safe for keys like `telegram:user_123`.

### A2: FNV-1a Deterministic Hashing
- **File**: `crates/clawft-core/src/embeddings/hash_embedder.rs`
- **Fix**: Replaced `DefaultHasher` (non-deterministic across runs/platforms) with FNV-1a via `fnv` crate
- **Note**: FNV-1a produces identical output across all platforms. Golden test added.

### A4: SecretString Credential Wrapper
- **File**: `crates/clawft-types/src/secret.rs` (new), `config.rs`
- **Fix**: Created `SecretString` type that redacts in `Debug`/`Display`. Uses `expose()` method for access.
- **Note**: All provider configs, channel passwords, and OAuth secrets now use `SecretString`.

### A5: Masked Terminal Input
- **File**: `crates/clawft-cli/src/commands/onboard.rs`
- **Fix**: Terminal echo suppression for API key input during onboarding
- **Note**: Non-interactive `--yes` mode still works without prompting.

### A6: SSRF Protection Complete
- **File**: `crates/clawft-services/src/mcp/middleware.rs`
- **Fix**: Full RFC 1918 range (172.16.0.0/12), IPv4-mapped IPv6 (`::ffff:x.x.x.x`), cloud metadata (169.254.169.254)
- **Note**: Second octet parsed and checked for 16-31 range. IPv4-mapped addresses are unwrapped before checking.

### A3: JSON Error Formatting
- **File**: `crates/clawft-core/src/agent/loop_core.rs`
- **Fix**: Replaced manual `format!("{{\"error\": \"{}\"}}",...)` with `serde_json::json!({"error": ...}).to_string()`
- **Note**: Error messages with quotes/backslashes now produce valid JSON.

### A7: HTTP Request Timeout
- **Files**: `crates/clawft-llm/src/openai_compat.rs`, `config.rs`, `router.rs`
- **Fix**: `reqwest::ClientBuilder::new().timeout(Duration::from_secs(120))` on all Client instances
- **Note**: `timeout_secs` field added to `ProviderConfig` for per-provider override.

### A8: Remove unsafe set_var
- **Files**: `crates/clawft-core/src/workspace.rs`, `crates/clawft-llm/src/openai_compat.rs`
- **Fix**: Replaced `unsafe { std::env::set_var(...) }` with `temp_env` crate in tests
- **Note**: No unsafe env var manipulation remains in test code.

### A9: Feature Gate for Services
- **Files**: `crates/clawft-cli/src/mcp_tools.rs`, `commands/mod.rs`, `main.rs`
- **Fix**: Gated all `clawft_services` imports behind `#[cfg(feature = "services")]`
- **Note**: `cargo check -p clawft-cli --no-default-features` now compiles cleanly.
