# Workstream A: Security & Data Integrity -- Decisions

## A4: SecretString vs SecretRef (env-var indirection)
- **Decision**: Used `SecretString` wrapper (stores value, redacts on Debug/Display) instead of `SecretRef` (stores env var name, resolves at runtime)
- **Rationale**: SecretString is simpler -- no need to change config loading flow. The primary goal (preventing accidental logging/serialization of credentials) is achieved. `expose()` method provides explicit opt-in access.
- **Trade-off**: Secrets still exist in memory as strings. For a CLI tool this is acceptable. A service running long-term might prefer env-var-at-access-time pattern.

## A6: SSRF - Parse-then-check vs regex
- **Decision**: Parse the second octet as `u8` and check `(16..=31).contains()` for RFC 1918 172.16.0.0/12
- **Rationale**: More correct than string prefix matching. Handles edge cases like `172.160.0.1` (not private) that a naive `starts_with("172.1")` would false-positive on.

## A7: Per-provider timeout
- **Decision**: Added `timeout_secs: Option<u64>` to `ProviderConfig` rather than global-only timeout
- **Rationale**: Different providers have different latency characteristics. Gemini might need 180s for large context, while a local vLLM might timeout at 30s.

## A8: temp_env vs mutex guard
- **Decision**: Used `temp_env` crate over manual `Mutex` guard
- **Rationale**: `temp_env` is purpose-built for this exact problem. Less boilerplate than managing a static Mutex.
