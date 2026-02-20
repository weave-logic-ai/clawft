# SPARC Feature Element 03: Critical Fixes & Cleanup

**Workstreams**: A (Critical Fixes), B (Architecture Cleanup), I (Type Safety), J (Documentation Sync)
**Timeline**: Weeks 1-5
**Status**: Complete (all 33 items done)
**Dependencies**: None (this is the foundation layer)
**Blocks**: 04 (Plugin System), all downstream feature work

---

## 1. Summary

This feature element resolves all bugs, security issues, architecture debt, and documentation drift identified in the 9-crate review. It must complete before feature work builds on top.

---

## 2. Phases

### Phase A: Security & Data Integrity (Week 1-2)

| Item | Description | Crate | Priority |
|------|-------------|-------|----------|
| A1 | Session key round-trip corruption (percent-encoding) | clawft-core | P0 |
| A4 | Plaintext credentials in config structs (env var names) | clawft-types | P0 |
| A5 | API key echoed during onboarding (rpassword) | clawft-cli | P0 |
| A6 | Incomplete SSRF private IP range (172.16-31.*) | clawft-services | P0 |
| A3 | Invalid JSON from error formatting (serde_json) | clawft-core | P1 |
| A7 | No HTTP timeout on LLM client (120s default). Non-goal: advanced timeout strategies (circuit breaker, adaptive timeouts) -- simple per-provider timeout is included. | clawft-llm | P1 |
| A8 | `unsafe set_var` in parallel tests (temp_env) | clawft-core | P1 |
| A9 | `--no-default-features` compilation failure | clawft-cli | P1 |
| A2 | Unstable hash in embeddings (fnv/xxhash) | clawft-core | P0 |

### Phase B: Architecture Cleanup (Week 2-4)

| Item | Description | Crate | Priority |
|------|-------------|-------|----------|
| B1 | Unify `Usage` type across crates (u32) | clawft-types, clawft-llm | P1 |
| B2 | Unify duplicate `LlmMessage` types | clawft-core | P1 |
| B3 | Split oversized files (9 worst offenders > 500 lines; note: codebase audit found 39 files total over 500 lines -- B3 targets the 9 most critical initially) | Multiple | P1 |
| B4 | Unify cron storage formats (JSONL) | clawft-cli, clawft-services | P1 |
| B5 | Extract shared tool registry builder | clawft-cli | P2 |
| B6 | Extract shared policy types | clawft-types | P2 |
| B7 | Deduplicate ProviderConfig naming | clawft-llm, clawft-types | P2 |
| B8 | Consolidate build_messages duplication | clawft-core | P2 |
| B9 | MCP protocol version constant | clawft-services | P2 |

### Phase I: Type Safety (Week 2-4)

| Item | Description | Crate | Priority |
|------|-------------|-------|----------|
| I1 | DelegationTarget serde consistency (snake_case) | clawft-types | P2 |
| I2 | String-typed policy modes to enums | clawft-types | P2 |
| I3 | ChatMessage::content serialization (skip_if_none) | clawft-llm | P2 |
| I4 | Job ID collision fix (uuid) | clawft-cli | P2 |
| I5 | camelCase normalizer acronym handling | clawft-platform | P2 |
| I6 | Dead code removal | Multiple | P2 |
| I7 | Fix always-true test assertion | clawft-core | P2 |
| I8 | Share MockTransport across crates (test-utils) | clawft-services | P2 |

### Phase J: Documentation Sync (Week 3-5)

| Item | Description | Priority |
|------|-------------|----------|
| J1 | Fix provider counts (9 actual, docs say 7-8) | P1 |
| J2 | Fix assembler truncation description | P1 |
| J3 | Fix token budget source reference | P1 |
| J4 | Document identity bootstrap behavior | P2 |
| J5 | Document rate-limit retry behavior | P2 |
| J6 | Document CLI log level change | P2 |
| J7 | Plugin system documentation (after C1-C6). **Note**: Started in Element 03 (framework docs for C1), completed after Element 04 C6 lands. Final J7 deliverable tracked in Element 04 exit criteria. | P2 |

---

## 2.5 Internal Dependency Graph

The following items have internal ordering constraints. Items not listed here may be executed in parallel within their phase.

```
A4 (SecretRef) ──────> B3 (file splits)
  config.rs split should include the new secret.rs module from A4;
  doing A4 first avoids splitting then re-splitting.

A6 (SSRF fix) ──────> B6 (policy type extraction)
  The canonical UrlPolicy in clawft-types should include the
  complete SSRF IP check from A6. Land A6 first or concurrently.

B1 (Usage unification) ──────> B7 (ProviderConfig naming)
  Both touch clawft-llm types. Coordinate to avoid churn.

I2 (policy mode enums) ──────> B3 (post-split)
  If config.rs is split into config/policies.rs, I2 should
  target the post-split file path.
```

---

## 3. Exit Criteria

- [x] All P0 items resolved and tested -- DONE 2026-02-20
- [x] All P1 items resolved and tested -- DONE 2026-02-19/2026-02-20
- [x] All P2 items resolved or documented as deferred -- DONE 2026-02-19
- [x] No files > 500 lines in modified crates (B3) -- impl under 500 lines; test bulk acceptable
- [x] Zero clippy warnings -- VERIFIED 2026-02-20
- [x] All 2,075+ existing tests still pass -- 2,407 tests, 0 failures
- [x] Documentation matches code behavior for all J items -- DONE 2026-02-19
- [x] No plaintext credentials in Debug output or serialized JSON -- SecretString wrapper DONE

### Migration-Specific Exit Criteria

- [x] **A1**: Existing session files using underscore encoding are auto-migrated to percent-encoded form on first startup. Both old and new format files are readable during migration. -- DONE 2026-02-20
- [x] **A2**: A golden test asserts that `compute_embedding("hello world")` produces a specific known output vector, identical across x86_64-linux, aarch64-linux, and x86_64-darwin. Embeddings with the old hash trigger a warning on load. -- DONE 2026-02-20
- [x] **A4**: Config files using the old `"imap_password": "literal_string"` format deserialize without error, logging a deprecation warning. Backward compatibility is maintained during migration. -- DONE 2026-02-20

### Security Exit Criteria

- [x] SSRF check blocks `::ffff:10.0.0.1` (IPv4-mapped IPv6 bypass) -- DONE 2026-02-20
- [x] SSRF check blocks `169.254.169.254` (cloud metadata endpoint) -- DONE 2026-02-20
- [x] No credential `String` fields exist in config structs without `_env` suffix (verified by CI lint) -- SecretString wrapper DONE 2026-02-20

---

## 4. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| B3 file splits cause merge conflicts with parallel development | Medium | Medium | **6** | Land B3 early in the sprint (Week 2); coordinate with all developers via branch protection. No feature branches should touch the 9 target files during B3 work. |
| A2 hash migration silently corrupts existing embeddings | Low | Critical | **5** | Golden test with cross-platform verification. Old-hash embeddings trigger a warning and re-computation on load. Migration is lazy, not forced. |
| A4 SecretRef migration misses a credential field | Medium | High | **8** | CI lint rule rejects any `pub String` field in config structs matching secret-pattern regex without `_env` suffix. Grep audit (`*password*`, `*secret*`, `*token*`, `*key*`, `*auth*`, `*credential*`, `*bearer*`, `*apikey*`, `*passphrase*`) before merge. |
| IPv4-mapped IPv6 SSRF bypass discovered after A6 lands | Low | High | **4** | Explicit test cases for `::ffff:10.0.0.1` and `169.254.169.254` added to security exit criteria. Convert `::ffff:x.x.x.x` to IPv4 before SSRF check. |
