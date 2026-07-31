# WeftOS 1.0 Security Gate Audit

**Date:** 2026-04-04
**Auditor:** Security Auditor Agent (V3)
**Scope:** All 25 crates in the WeftOS workspace
**Commit:** dd8d95b (master)

---

## Executive Summary

The WeftOS codebase demonstrates strong security architecture overall.
Path traversal defenses, WASM sandbox isolation, ExoChain integrity
verification, and shell command policy enforcement are all well-implemented
with comprehensive test coverage. Two high-severity issues were fixed
during this audit. Several medium and low findings require follow-up
before production hardening is complete.

| Severity | Count | Fixed | Remaining |
|----------|-------|-------|-----------|
| Critical | 0     | --    | 0         |
| High     | 2     | 2     | 0         |
| Medium   | 5     | 0     | 5         |
| Low      | 4     | 0     | 4         |

---

## HIGH Findings (Fixed)

### H-1: Ed25519 signing key written with default umask (world-readable)

**File:** `crates/clawft-kernel/src/chain.rs` lines 563-570
**Severity:** HIGH
**OWASP:** A02:2021 -- Cryptographic Failures

`ChainManager::load_or_create_key()` called `std::fs::write()` without
restricting file permissions. On a multi-user system the raw 32-byte
Ed25519 private key would be readable by any local user, allowing
chain event forgery and signature spoofing.

**Fix applied:** Added `chmod 0600` after key file creation on Unix, and
a permission audit + auto-fix on key load. See lines 547-575 in the
updated `chain.rs`.

---

### H-2: TokenStore panics on poisoned RwLock

**File:** `crates/clawft-services/src/api/auth.rs` lines 33, 39
**Severity:** HIGH
**OWASP:** A05:2021 -- Security Misconfiguration

`generate_token()` and `validate()` both called `.unwrap()` on
`RwLock::read()` / `RwLock::write()`. If any thread panics while
holding the lock (e.g., during OOM), subsequent calls crash the
entire API server, creating a denial-of-service condition.

**Fix applied:** `generate_token()` now returns `Option<String>` and
uses `.ok()?`; `validate()` returns `false` on a poisoned lock.
The handler in `handlers.rs` returns HTTP 500 if token generation
fails. Tests updated.

---

## MEDIUM Findings (Document for Follow-up)

### M-1: API auth middleware disabled in production router

**File:** `crates/clawft-services/src/api/mod.rs` lines 309-315
**Severity:** MEDIUM
**OWASP:** A01:2021 -- Broken Access Control

The `auth_middleware` layer is commented out with the note "Enable once
the UI has a login flow." All `/api/*` endpoints (agent management,
session browsing, tool execution, config, delegation) are currently
unauthenticated. The WebSocket endpoint at `/ws` is also unprotected.

**Recommendation:** Before any network-exposed deployment:
1. Enable the auth middleware layer on the `/api` nest.
2. Add token validation to the WebSocket upgrade handler.
3. Consider binding to `127.0.0.1` only for local-only deployments.

---

### M-2: CORS set to permissive when no origins configured

**File:** `crates/clawft-services/src/api/mod.rs` line 295
**Severity:** MEDIUM
**OWASP:** A05:2021 -- Security Misconfiguration

When `cors_origins` is empty, `CorsLayer::permissive()` is used,
which sets `Access-Control-Allow-Origin: *` with credentials allowed.
This allows any website to make authenticated API requests if the
server is exposed to the network.

**Recommendation:** Default to denying cross-origin requests when no
explicit origins are configured, or at minimum require the `--cors`
flag for non-localhost binds.

---

### M-3: No rate limiting on API endpoints — **CLOSED (WEFT-546)**

**File:** `crates/clawft-services/src/api/middleware.rs` (`rate_limit_middleware`)
**Severity:** MEDIUM (historical)
**OWASP:** A04:2021 -- Insecure Design

**Status 2026-07-31:** Fixed. Per-IP token-bucket middleware is wired on the
HTTP façade (`RateLimitState` + `rate_limit_middleware` in `api/mod.rs`).
Classes: Auth (e.g. `/api/auth/token`) tighter quota; general `/api/*`; WS.
Unit tests: `rate_limit_blocks_after_quota`, `rate_limit_separate_ips`,
`rate_limit_general_api_allows_60`, `rate_class_classifies_paths`.

---

### M-4: No token revocation mechanism

**File:** `crates/clawft-services/src/api/auth.rs`
**Severity:** MEDIUM
**OWASP:** A07:2021 -- Authentication Failures

`TokenStore` supports generation and validation but has no `revoke()`
method. Tokens are valid for 24 hours with no way to invalidate them
early. A compromised token cannot be revoked.

**Recommendation:** Add `revoke_token(&self, token: &str) -> bool`
and a periodic expired-token cleanup task.

---

### M-5: ExoChain has no replay protection

**File:** `crates/clawft-kernel/src/chain.rs` lines 343-394
**Severity:** MEDIUM
**OWASP:** A08:2021 -- Software and Data Integrity Failures

The chain append operation uses monotonic sequence numbers and hash
linking, which prevents tampering with existing events. However, there
is no nonce or idempotency key in the append path. If the same event
payload is submitted twice (e.g., due to a retry), it will be
appended as two distinct events. In a distributed deployment (K1+),
this could allow event replay.

**Recommendation:** Add an optional `idempotency_key` field to
`ChainEvent` and check for duplicates before appending.

---

## LOW Findings (Document for Follow-up)

### L-1: `unsafe impl Send/Sync` for WasmToolAdapter and ToolRegistry

**File:** `crates/clawft-kernel/src/wasm_runner/registry.rs` lines 318-326
**Severity:** LOW

The `unsafe impl Send` and `unsafe impl Sync` blocks have safety
comments explaining the rationale. The fields are indeed `Arc`-wrapped
and contain `Send+Sync` data. However, using `#[derive]` or marker
traits where possible would be preferable to manual unsafe impls.

**Status:** Acceptable for 1.0 with existing safety comments. Consider
refactoring WasmToolAdapter fields to auto-derive Send+Sync.

---

### L-2: `unsafe` raw pointer in `ensure_section`

**File:** `crates/clawft-kernel/src/tools_extended.rs` lines 616-629
**Severity:** LOW

The function uses a raw pointer (`*mut serde_json::Map`) to walk a
JSON tree. The safety comment states "we control the mutable references
here and never alias." This is technically sound since the function
owns the mutable borrow and does not create aliasing references, but
the pattern is fragile under refactoring.

**Recommendation:** Refactor to use safe recursive or iterative
approaches with `entry()` API. Not a blocking issue.

---

### L-3: `unsafe transmute_copy` for ML-DSA-65 key extraction

**File:** `crates/clawft-kernel/src/chain.rs` lines 1617-1624
**Severity:** LOW

Uses `std::mem::transmute_copy` to extract raw bytes from
`MlDsa65VerifyKey`. Protected by a `size_of` assertion. This is
sound assuming the struct layout contains only the key bytes, but
fragile if the upstream crate changes its representation.

**Recommendation:** Request an `as_bytes()` method from the upstream
crate, or add a CI test that verifies the struct layout assertion.

---

### L-4: Test fixtures contain fake API keys

**Files:**
- `tests/fixtures/config.json` (lines 31, 35)
- `tests/fixtures/config_tiered.json` (lines 9-12)
- Various `#[cfg(test)]` blocks in `clawft-llm` and `clawft-cli`

**Severity:** LOW (informational)

Test keys like `sk-ant-test-key` and `sk-or-test-key` are clearly
synthetic and do not represent real credentials. The `clawft-security`
crate's own scanner correctly identifies real key patterns. No action
needed, but ensure `.gitignore` excludes any real config files.

---

## Passed Checks

### Path Traversal Protection -- PASS

`crates/clawft-tools/src/file_tools.rs` implements robust workspace
containment:
- `validate_path()` canonicalizes paths and checks `starts_with()`
  against the workspace root.
- `validate_parent_path()` handles new file creation by
  canonicalizing the deepest existing ancestor.
- Symlink escapes are caught by `canonicalize()` following links
  before the containment check.
- Tests cover `../../../etc/passwd`, symlink-to-outside, and
  directory traversal attempts.

### WASM Sandbox Isolation -- PASS

`crates/clawft-kernel/src/wasm_runner/`:
- Fuel metering enforced on every execution (`consume_fuel(true)`).
- Memory limits enforced via `ResourceLimiter`.
- File access validated through `PluginSandbox` with canonical path
  containment checks and size limits (8MB read, 4MB write).
- Network access gated through hostname allowlist with proper
  wildcard handling.
- Environment variable access restricted to explicit allowlists.
- Comprehensive audit logging via `AuditLog`.

### ExoChain Integrity -- PASS

`crates/clawft-kernel/src/chain.rs`:
- SHAKE-256 hash linking with domain-separated fields.
- Payload content commitment via separate `payload_hash`.
- `verify_integrity()` checks prev_hash linkage, payload hash
  recomputation, and event hash recomputation.
- Ed25519 and dual Ed25519+ML-DSA-65 signing for RVF segments.
- Witness chain creation and verification.
- Integrity check on load (JSON and RVF formats).

### Shell Command Policy -- PASS

`crates/clawft-tools/src/shell_tool.rs` + `security_policy.rs`:
- Allowlist-based command policy (default: safe commands only).
- Denylist with 11 dangerous patterns.
- First-token extraction rejects path traversal in command names.
- Timeout enforcement with process kill on timeout.
- Tests cover `../../../bin/sh` traversal, dangerous patterns, and
  allowlist enforcement.

### GovernanceGate -- PASS (with note)

`crates/clawft-kernel/src/gate.rs`:
- `GovernanceGate::open()` exists but is correctly documented as a
  development convenience. In production boot (`boot.rs` line 744),
  the gate is created with explicit risk threshold and rules.
- Effect vector evaluation, rule matching, and human escalation all
  function correctly.
- Note: Ensure `GovernanceGate::open()` is never used in production
  config. Consider adding a compile-time or runtime warning.

### Secret Handling -- PASS

- `clawft-types` uses a `SecretString` wrapper that redacts on
  `Debug` display.
- `clawft-security` crate has pattern-based secret scanning
  (OpenAI keys, AWS keys, PGP blocks, etc.).
- API keys in provider config are loaded from environment variables,
  not hardcoded.
- No `.env` files or private keys found in the repository.
- `TtsProviderInfo::api_key` is annotated `#[serde(skip_serializing)]`
  to prevent accidental exposure in API responses.

### Dependency Audit -- INCOMPLETE

`cargo audit` is not installed in the current environment. This check
should be run in CI.

**Recommendation:** Add `cargo install cargo-audit` to the CI pipeline
and run `cargo audit` as part of `scripts/build.sh gate`.

---

## Summary of Changes Made

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/chain.rs` | Set key file to 0600 on create; auto-fix permissions on load |
| `crates/clawft-services/src/api/auth.rs` | Replace `.unwrap()` with graceful error handling on poisoned locks |
| `crates/clawft-services/src/api/handlers.rs` | Return HTTP 500 instead of panicking when token generation fails |

---

## Recommended Pre-Deployment Checklist

1. [x] Fix signing key file permissions (H-1)
2. [x] Fix TokenStore panic on poisoned lock (H-2)
3. [ ] Enable auth middleware on `/api` routes (M-1)
4. [ ] Restrict CORS to explicit origins (M-2)
5. [x] Add rate limiting middleware (M-3) — WEFT-546
6. [ ] Add token revocation (M-4)
7. [ ] Add ExoChain idempotency keys (M-5)
8. [ ] Install and run `cargo audit` in CI
9. [ ] Verify `GovernanceGate::open()` is not used in production configs
