# Business Requirements: Stability Fixes & Type Safety

> Draft sections for integration into [01-business-requirements.md](../01-business-requirements.md).
> Covers Workstream A (Critical Fixes) and Workstream I (Type Safety & Cleanup) from the
> [Unified Sprint Plan](../improvements.md).

---

## 5d. Stability & Security Fixes

These requirements address critical bugs, security vulnerabilities, and reliability gaps that
undermine the core guarantees in Goals G1-G5. Every item here is a prerequisite for the feature
work in Workstreams C-M; shipping new capabilities on top of a broken foundation wastes effort
and erodes user trust.

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| SF-1 | As a multi-channel user, I want my session history to persist correctly when my channel or chat ID contains underscores so that I do not lose conversation context or get crossed with another user's sessions | P0 |
| SF-2 | As an operator upgrading Rust toolchains, I want my persisted vector embeddings to remain valid after the upgrade so that semantic search continues to return correct results without manual re-indexing | P0 |
| SF-3 | As a developer integrating clawft with an LLM, I want error messages returned as tool results to always be valid JSON so that the model can parse them reliably instead of receiving malformed data | P0 |
| SF-4 | As an admin, I want API keys, passwords, and tokens never to appear in serialized config, debug output, or log files so that credentials are not leaked through routine operations | P0 |
| SF-5 | As a new user running onboarding, I want my API key input to be masked in the terminal so that shoulder-surfers or screen-share viewers cannot read it | P0 |
| SF-6 | As an admin operating clawft in a cloud environment, I want SSRF protection to block all RFC 1918 private IP ranges so that internal services cannot be probed through clawft's HTTP tools | P0 |
| SF-7 | As a user, I want LLM requests to time out after a reasonable period so that a non-responsive provider does not hang my agent indefinitely | P1 |
| SF-8 | As a contributor running the test suite, I want tests to be safe under parallel execution so that CI results are deterministic and free of undefined behavior | P1 |
| SF-9 | As an IoT/edge developer, I want to compile clawft with `--no-default-features` so that I can produce a minimal binary without MCP/services dependencies, and that binary must actually compile | P1 |

### Feature Summary

| Feature | Description | Affected Crate(s) |
|---------|-------------|-------------------|
| Session key encoding | Percent-encode or two-char-escape session filenames so that round-trip key reconstruction is lossless for all valid channel/chat IDs | `clawft-core` |
| Stable hash embeddings | Replace `DefaultHasher` with a deterministic, cross-version-stable hash; include a one-time re-index migration path | `clawft-core` |
| Safe JSON error formatting | Use `serde_json` for all tool-result error payloads so that special characters in error messages do not produce malformed JSON | `clawft-core` |
| Credential redaction | Store only environment variable names (not raw secrets) in config structs; add `#[serde(skip_serializing)]` and custom `Debug` impls that redact sensitive fields | `clawft-types` |
| Masked API key input | Use `rpassword` (or equivalent) during onboarding prompts to suppress terminal echo of secret values | `clawft-cli` |
| Complete SSRF IP blocking | Block the full RFC 1918 `172.16.0.0/12` range (`172.16.*` through `172.31.*`), not just `172.16.*` | `clawft-services` |
| HTTP request timeout | Apply a configurable timeout (default 120s) to all outbound LLM provider HTTP calls | `clawft-llm` |
| Thread-safe test environment | Replace `unsafe std::env::set_var` in tests with `temp_env` or mutex guards so tests are safe under Rust 2024 edition parallel execution | `clawft-core` |
| Conditional `services` compilation | Gate all `clawft_services` imports behind `#[cfg(feature = "services")]` with a no-op stub when disabled, matching the existing `delegate` pattern | `clawft-cli` |

### Non-Goals (Stability Fixes)

- Rewriting the session storage engine (the encoding fix is surgical, not a redesign)
- Introducing a secrets manager or vault integration (env-var-name pattern is sufficient for now)
- Supporting custom timeout values per-provider (global default is adequate for this phase; per-provider config is Workstream D)
- Backward compatibility with embeddings persisted using the unstable hash (a one-time migration is acceptable)

### Dependencies & Sequencing

| ID | Blocks | Reason |
|----|--------|--------|
| SF-4 (credential redaction) | Workstream E: Email channel, Workstream F: OAuth2 | Email and OAuth2 introduce more credential types; the env-var-name pattern must be established first |
| SF-2 (stable hash) | Workstream H: Vector memory (RVF Phase 3) | Building production vector search on top of an unstable hash function invalidates all stored indices |
| SF-6 (SSRF protection) | Workstream F: Browser automation, REST helper | New HTTP-capable tools widen the SSRF surface; the protection must be complete before adding them |

---

## 5e. Type Safety & Cleanup

These requirements address type-system gaps, serialization inconsistencies, and dead code that
increase the cost of every future change. While individually small, they compound: an enum
serialized as PascalCase in one place and snake_case in another means every consumer must handle
both, a tautological test assertion means a broken code path goes undetected, and string-typed
fields that accept specific values are bugs waiting for a typo.

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| TS-1 | As a developer writing config files, I want all enum values in `clawft.toml` and `config.json` to follow a consistent naming convention so that I do not have to guess whether a value is `"snake_case"` or `"PascalCase"` | P1 |
| TS-2 | As a developer configuring command policies and rate limiting, I want the `mode` and `strategy` fields to be validated at deserialization time against a known set of values so that typos produce clear errors instead of silent misbehavior | P1 |
| TS-3 | As a user of a provider that rejects `null` content fields, I want chat messages with no content to omit the field entirely from the JSON payload so that my provider does not return a 400 error | P1 |
| TS-4 | As an operator running scheduled jobs, I want job IDs to be globally unique so that two jobs created in the same second do not collide and overwrite each other | P1 |
| TS-5 | As a developer working with config keys, I want acronyms in camelCase identifiers to be normalized correctly (e.g., `HTMLParser` becomes `html_parser`, not `h_t_m_l_parser`) so that config lookups succeed | P2 |
| TS-6 | As a contributor reading the codebase, I want dead code to be either removed or clearly annotated with the workstream that will activate it, so that I do not waste time understanding unused code paths or wonder if something is broken | P2 |
| TS-7 | As a contributor, I want every test assertion to verify a meaningful property so that test passes actually indicate correctness rather than a tautology that can never fail | P2 |
| TS-8 | As a developer writing integration tests across crates, I want a shared `MockTransport` available behind a `test-utils` feature flag so that I do not have to duplicate test infrastructure in every crate | P2 |

### Feature Summary

| Feature | Description | Affected File(s) |
|---------|-------------|-------------------|
| Consistent serde casing | Add `#[serde(rename_all = "snake_case")]` to `DelegationTarget` to match all other enums in the codebase | `clawft-types/src/routing.rs` |
| Typed policy modes | Replace `String` fields `CommandPolicyConfig::mode` and `RateLimitConfig::strategy` with proper enums that reject unknown values at parse time | `clawft-types/src/config.rs` |
| Optional content serialization | Add `#[serde(skip_serializing_if = "Option::is_none")]` to `ChatMessage::content` so that `None` is omitted rather than serialized as `null` | `clawft-llm/src/types.rs` |
| UUID-based job IDs | Replace seconds+PID job ID generation with `uuid::Uuid::new_v4()` (already in workspace deps) to eliminate same-second collisions | `clawft-cli/src/commands/cron.rs` |
| Acronym-aware camelCase normalizer | Add consecutive-uppercase detection to the `camelCase` normalizer so that `HTMLParser` normalizes to `html_parser` | `clawft-platform/src/config_loader.rs` |
| Dead code audit | Remove or annotate with `// TODO(workstream-X)` references: `evict_if_needed`, `ResumePayload`, interactive slash-command framework, no-op CLI flags | Multiple files |
| Meaningful test assertions | Replace `assert!(result.is_err() \|\| result.is_ok())` with assertions that test the expected specific outcome | `clawft-core/src/pipeline/transport.rs` |
| Shared `MockTransport` | Move `MockTransport` behind a `test-utils` feature flag instead of `#[cfg(test)]` so downstream crates can import it for integration testing | `clawft-services/src/mcp/transport.rs` |

### Non-Goals (Type Safety)

- Migrating existing user config files to the new enum values (deserialization must accept both old and new forms via `#[serde(alias)]` where needed)
- Achieving 100% dead code elimination (code tied to planned workstreams is annotated, not removed)
- Enforcing strict types on all string fields project-wide (scoped to the fields identified in Workstream I only)

### Migration Notes

| Change | Backward Compatibility |
|--------|----------------------|
| `DelegationTarget` serde casing | Old PascalCase values (`"Local"`, `"Claude"`) must still deserialize via `#[serde(alias)]`; new serialization output will be `snake_case` |
| Policy mode enums | Existing string values in config files must map cleanly to enum variants; unrecognized values produce a clear deserialization error with the list of valid options |
| `ChatMessage::content` | No user-facing config change; affects only the wire format sent to LLM providers |
| Job ID format | Existing cron jobs retain their old IDs; only newly created jobs use UUIDs |

---

## Success Criteria Additions

### Stability & Security (Phase 1.5 -- before Phase 2 feature work)

- [ ] Session keys with underscores in channel/chat IDs round-trip correctly through save and load
- [ ] `cargo test` passes with no `unsafe` env mutations under parallel execution
- [ ] `cargo build -p clawft-cli --no-default-features` compiles without errors
- [ ] No secret values (API keys, passwords, tokens) appear in `Debug` output or serialized JSON of any config struct
- [ ] Onboarding flow masks API key input (no terminal echo)
- [ ] URLs targeting `172.16.0.0/12` (e.g., `http://172.30.0.1/`) are blocked by SSRF protection
- [ ] LLM HTTP requests time out after the configured duration (default 120s) and return a descriptive error
- [ ] Error messages containing special characters (quotes, backslashes, newlines) produce valid JSON tool results
- [ ] Vector embeddings produce identical output across Rust toolchain versions (deterministic hash)

### Type Safety & Cleanup (Phase 2 -- concurrent with Architecture Cleanup)

- [ ] All enum types in `clawft-types` serialize with consistent `snake_case` naming
- [ ] `CommandPolicyConfig::mode` and `RateLimitConfig::strategy` reject unknown string values at deserialization with a helpful error message
- [ ] Chat messages with `None` content omit the `content` field from serialized JSON (no `null` values)
- [ ] Job IDs are UUIDs; no collisions observed under concurrent cron job creation
- [ ] `HTMLParser` normalizes to `html_parser` (not `h_t_m_l_parser`)
- [ ] No `#[allow(dead_code)]` annotations remain without a `// TODO(workstream-X)` reference
- [ ] The tautological assertion in `transport.rs` is replaced with a meaningful check
- [ ] `MockTransport` is importable from `clawft-services` by downstream crates via the `test-utils` feature

---

## Risk Register Additions

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Session key migration breaks existing sessions | Medium | High | 6 | Implement fallback: try new encoding first, fall back to legacy decoding for existing files; one-time migration on first access |
| Credential redaction misses a field | Medium | High | 6 | Grep audit for all `String` fields named `*password*`, `*secret*`, `*token*`, `*key*` (excluding `_env` suffix); add a CI lint rule |
| `DelegationTarget` serde change breaks external integrations | Low | Medium | 2 | `#[serde(alias)]` ensures old PascalCase values still deserialize; only serialization output changes |
| Dead code removal breaks a feature that was partially wired | Low | Medium | 2 | Each removal is preceded by a codebase-wide reference search; annotate with workstream reference rather than delete when in doubt |
| Stable hash migration invalidates cached embeddings | Certain | Low | 3 | Acceptable cost; migration runs automatically on first access; document in upgrade notes |
