# Element 03: Critical Fixes & Cleanup -- Sprint Tracker

## Summary

- **Total items**: 33 (9 P0, 12 P1, 12 P2)
- **Workstreams**: A (Security & Data Integrity), B (Architecture Cleanup), I (Type Safety), J (Documentation Sync)
- **Timeline**: Weeks 1-5
- **Status**: Planning -> Development
- **Dependencies**: None (foundation layer)
- **Blocks**: Element 04 (Plugin System), all downstream feature work

---

## Execution Schedule

### Week 1 (P0 Security -- 5 items)

- [ ] A1 -- Session key round-trip corruption (percent-encoding) -- P0, clawft-core
- [ ] A2 -- Unstable hash in embeddings (fnv/xxhash) -- P0, clawft-core
- [ ] A4 -- Plaintext credentials in config structs (SecretRef) -- P0, clawft-types
- [ ] A5 -- API key echoed during onboarding (rpassword) -- P0, clawft-cli
- [ ] A6 -- Incomplete SSRF private IP range (172.16-31.*) -- P0, clawft-services

### Week 2 (P0 Remaining + P1 Start -- 8 items)

- [ ] A3 -- Invalid JSON from error formatting (serde_json) -- P1, clawft-core
- [ ] A7 -- No HTTP timeout on LLM client (120s default) -- P1, clawft-llm
- [ ] A8 -- `unsafe set_var` in parallel tests (temp_env) -- P1, clawft-core
- [ ] A9 -- `--no-default-features` compilation failure -- P1, clawft-cli
- [ ] B1 -- Unify `Usage` type across crates (u32) -- P1, clawft-types + clawft-llm
- [ ] B2 -- Unify duplicate `LlmMessage` types -- P1, clawft-core
- [ ] B3 -- Split oversized files (9 worst offenders) -- P1, multiple crates
- [ ] B4 -- Unify cron storage formats (JSONL) -- P1, clawft-cli + clawft-services

### Week 3 (P1 Doc Sync + P2 Architecture -- 7 items)

- [ ] J1 -- Fix provider counts (9 actual, docs say 7-8) -- P1, docs
- [ ] J2 -- Fix assembler truncation description -- P1, docs/architecture
- [ ] J3 -- Fix token budget source reference -- P1, docs/guides
- [ ] B5 -- Extract shared tool registry builder -- P2, clawft-cli
- [ ] B6 -- Extract shared policy types -- P2, clawft-types
- [ ] B7 -- Deduplicate ProviderConfig naming -- P2, clawft-llm + clawft-types
- [ ] B8 -- Consolidate build_messages duplication -- P2, clawft-core

### Week 4 (P2 Type Safety + Architecture -- 8 items)

- [ ] B9 -- MCP protocol version constant -- P2, clawft-services
- [ ] I1 -- DelegationTarget serde consistency (snake_case) -- P2, clawft-types
- [ ] I2 -- String-typed policy modes to enums -- P2, clawft-types
- [ ] I3 -- ChatMessage::content serialization (skip_if_none) -- P2, clawft-llm
- [ ] I4 -- Job ID collision fix (uuid) -- P2, clawft-cli
- [ ] I5 -- camelCase normalizer acronym handling -- P2, clawft-platform
- [ ] I6 -- Dead code removal -- P2, multiple crates
- [ ] I7 -- Fix always-true test assertion -- P2, clawft-core

### Week 5 (P2 Remaining -- 5 items)

- [ ] I8 -- Share MockTransport across crates (test-utils) -- P2, clawft-services
- [ ] J4 -- Document identity bootstrap behavior -- P2, docs/guides
- [ ] J5 -- Document rate-limit retry behavior -- P2, docs/guides
- [ ] J6 -- Document CLI log level change -- P2, docs/reference
- [ ] J7 -- Plugin system documentation (framework only; final after Element 04 C6) -- P2, docs

---

## Per-Item Status

| ID | Description | Priority | Week | Crate(s) | Status | Owner | Branch | Notes |
|----|-------------|----------|------|----------|--------|-------|--------|-------|
| A1 | Session key round-trip corruption | P0 | 1 | clawft-core | Pending | -- | -- | Percent-encoding fix; migration for existing session files |
| A2 | Unstable hash in embeddings | P0 | 1 | clawft-core | Pending | -- | -- | fnv/xxhash; golden test across platforms |
| A3 | Invalid JSON from error formatting | P1 | 2 | clawft-core | Pending | -- | -- | serde_json for error strings |
| A4 | Plaintext credentials in config | P0 | 1 | clawft-types | Pending | -- | -- | SecretRef type; backward compat for old configs |
| A5 | API key echoed during onboarding | P0 | 1 | clawft-cli | Pending | -- | -- | rpassword for secure input |
| A6 | Incomplete SSRF private IP range | P0 | 1 | clawft-services | Pending | -- | -- | 172.16-31.*, IPv4-mapped IPv6 bypass |
| A7 | No HTTP timeout on LLM client | P1 | 2 | clawft-llm | Pending | -- | -- | 120s default timeout |
| A8 | unsafe set_var in parallel tests | P1 | 2 | clawft-core | Pending | -- | -- | temp_env crate |
| A9 | --no-default-features compilation | P1 | 2 | clawft-cli | Pending | -- | -- | Feature gate fix |
| B1 | Unify Usage type across crates | P1 | 2 | clawft-types, clawft-llm | Pending | -- | -- | u32 canonical type |
| B2 | Unify duplicate LlmMessage types | P1 | 2 | clawft-core | Pending | -- | -- | Single canonical type |
| B3 | Split oversized files | P1 | 2 | Multiple | Pending | -- | -- | 9 worst offenders >500 lines; notify all devs before merge |
| B4 | Unify cron storage formats | P1 | 2 | clawft-cli, clawft-services | Pending | -- | -- | JSONL standardization |
| B5 | Extract shared tool registry builder | P2 | 3 | clawft-cli | Pending | -- | -- | -- |
| B6 | Extract shared policy types | P2 | 3 | clawft-types | Pending | -- | -- | Depends on A6 landing |
| B7 | Deduplicate ProviderConfig naming | P2 | 3 | clawft-llm, clawft-types | Pending | -- | -- | Coordinate with B1 |
| B8 | Consolidate build_messages duplication | P2 | 3 | clawft-core | Pending | -- | -- | -- |
| B9 | MCP protocol version constant | P2 | 4 | clawft-services | Pending | -- | -- | -- |
| I1 | DelegationTarget serde consistency | P2 | 4 | clawft-types | Pending | -- | -- | snake_case serde |
| I2 | String-typed policy modes to enums | P2 | 4 | clawft-types | Pending | -- | -- | Target post-B3 file paths |
| I3 | ChatMessage::content serialization | P2 | 4 | clawft-llm | Pending | -- | -- | skip_if_none |
| I4 | Job ID collision fix | P2 | 4 | clawft-cli | Pending | -- | -- | uuid crate |
| I5 | camelCase normalizer acronym handling | P2 | 4 | clawft-platform | Pending | -- | -- | -- |
| I6 | Dead code removal | P2 | 4 | Multiple | Pending | -- | -- | -- |
| I7 | Fix always-true test assertion | P2 | 4 | clawft-core | Pending | -- | -- | -- |
| I8 | Share MockTransport across crates | P2 | 5 | clawft-services | Pending | -- | -- | test-utils module |
| J1 | Fix provider counts in docs | P1 | 3 | docs | Pending | -- | -- | 9 built-in (clawft-llm), 15 spec (clawft-types) |
| J2 | Fix assembler truncation description | P1 | 3 | docs/architecture | Pending | -- | -- | Assembler does truncate at Level 0 |
| J3 | Fix token budget source reference | P1 | 3 | docs/guides | Pending | -- | -- | max_context_tokens, not max_tokens |
| J4 | Document identity bootstrap | P2 | 5 | docs/guides | Pending | -- | -- | SOUL.md, IDENTITY.md override |
| J5 | Document rate-limit retry | P2 | 5 | docs/guides | Pending | -- | -- | 3 retries, exponential backoff |
| J6 | Document CLI log level change | P2 | 5 | docs/reference | Pending | -- | -- | Default changed to warn |
| J7 | Plugin system documentation | P2 | 5 | docs | Pending | -- | -- | Framework only; final after C1-C6 |

---

## Internal Dependencies

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

J7 (plugin docs) ──────> C1-C6 (Element 04)
  Framework docs started in Element 03; final completion
  after Element 04 C6 lands.
```

---

## Cross-Element Dependencies

| Source (Element 03) | Target (Other Element) | Dependency Type |
|---------------------|------------------------|-----------------|
| A4 (SecretRef) | Element 04 C1 (PluginHost uses SecretRef) | Data type dependency |
| A2 (stable hash) | Element 08 H2 (vector memory needs stable embeddings) | Correctness dependency |
| A6 (SSRF) | Element 04 C2 (WASM HTTP host function reuses SSRF check) | Security dependency |
| A9 (feature gates) | Element 04 C2 (WASM feature gate pattern) | Build system dependency |
| B3 (file splits) | Element 04 C3 (skills_v2.rs split needed before skill loader) | File structure dependency |
| B5 (tool registry) | Element 09 L1 (agent routing needs shared registry builder) | API dependency |

---

## Exit Criteria

### Functional

- [ ] All P0 items (9) resolved and tested
- [ ] All P1 items (12) resolved and tested
- [ ] All P2 items (12) resolved or documented as deferred
- [ ] No files > 500 lines in modified crates (B3)
- [ ] Zero clippy warnings
- [ ] All 2,075+ existing tests still pass
- [ ] Documentation matches code behavior for all J items

### Migration-Specific

- [ ] **A1**: Existing session files using underscore encoding are auto-migrated to percent-encoded form on first startup. Both old and new format files are readable during migration.
- [ ] **A2**: A golden test asserts that `compute_embedding("hello world")` produces a specific known output vector, identical across x86_64-linux, aarch64-linux, and x86_64-darwin. Embeddings with the old hash trigger a warning on load.
- [ ] **A4**: Config files using the old `"imap_password": "literal_string"` format deserialize without error, logging a deprecation warning. Backward compatibility is maintained during migration.

### Security

- [ ] SSRF check blocks `::ffff:10.0.0.1` (IPv4-mapped IPv6 bypass)
- [ ] SSRF check blocks `169.254.169.254` (cloud metadata endpoint)
- [ ] No credential `String` fields exist in config structs without `_env` suffix (verified by CI lint)
- [ ] No plaintext credentials in Debug output or serialized JSON

---

## Review Gates

| Gate | Scope | Requirement |
|------|-------|-------------|
| P0 Review | A1, A2, A4, A5, A6 | Code review required before P1 work begins |
| Security Review | A4, A5, A6 | Security-focused review required; SSRF test coverage verified |
| B3 Coordination | File splits | All developers notified before merge (conflict risk); branch protection enforced on 9 target files |
| P1 Review | A3, A7-A9, B1-B4, J1-J3 | Standard code review |
| P2 Review | B5-B9, I1-I8, J4-J7 | Standard code review; deferred items documented |

---

## Risk Register

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| B3 file splits cause merge conflicts | Medium | Medium | 6 | Land B3 early (Week 2); notify all devs; branch protection on target files |
| A2 hash migration corrupts embeddings | Low | Critical | 5 | Golden test with cross-platform verification; old-hash warning + lazy re-computation |
| A4 SecretRef migration misses a field | Medium | High | 8 | CI lint rejects credential fields without `_env` suffix; grep audit before merge |
| IPv4-mapped IPv6 SSRF bypass | Low | High | 4 | Explicit test cases for `::ffff:10.0.0.1` and `169.254.169.254` |
| J items introduce doc inconsistencies | Low | Low | 2 | Each J item verifies against source code; doc CI link checker |

---

## Progress Summary

| Workstream | Total | Pending | In Progress | Completed | % Done |
|------------|-------|---------|-------------|-----------|--------|
| A (Security) | 9 | 9 | 0 | 0 | 0% |
| B (Architecture) | 9 | 9 | 0 | 0 | 0% |
| I (Type Safety) | 8 | 8 | 0 | 0 | 0% |
| J (Doc Sync) | 7 | 7 | 0 | 0 | 0% |
| **Total** | **33** | **33** | **0** | **0** | **0%** |
