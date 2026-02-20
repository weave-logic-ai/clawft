# Specification Review: SPARC Elements 03 & 04 (Iteration 1)

> **Reviewer**: SPARC Specification Agent
> **Date**: 2026-02-19
> **Scope**: Element 03 (Critical Fixes & Cleanup) and Element 04 (Plugin & Skill System)
> **Source Documents Reviewed**:
> - `.planning/sparc/03-critical-fixes-cleanup/00-orchestrator.md`
> - `.planning/sparc/04-plugin-skill-system/00-orchestrator.md`
> - `.planning/drafts/01-biz-stability.md` (Workstreams A, I)
> - `.planning/drafts/01-biz-plugins-channels.md` (Workstreams C, E)
> - `.planning/drafts/02-tech-core.md` (Workstreams A, B, C, I)
> - `.planning/drafts/review-1-biz-tech.md` (Consensus review)
> - `.planning/drafts/review-1-devguide.md` (Dev guide review)
> - `.planning/improvements.md` (Source of truth)

---

## 1. Element 03: Critical Fixes & Cleanup

### 1.1 Overall Assessment: 7/10

Element 03 is well-structured with four logical phases (A: Security, B: Architecture, I: Type Safety, J: Documentation). The item coverage against `improvements.md` is complete, and the dependency declaration ("None -- this is the foundation layer") is correct. However, there are significant gaps in exit criteria specificity, several missing acceptance criteria, and some issues flagged in `review-1-biz-tech.md` that need resolution before development can start.

### 1.2 Coverage Check Against improvements.md

| improvements.md Item | In 03 Orchestrator? | Notes |
|---------------------|---------------------|-------|
| A1 (Session key round-trip) | YES | Correctly assigned to clawft-core, P0 |
| A2 (Unstable hash embeddings) | YES | Correctly assigned to clawft-core, P1 |
| A3 (Invalid JSON error formatting) | YES | Correctly assigned to clawft-core, P1 |
| A4 (Plaintext credentials) | YES | Correctly assigned to clawft-types, P0 |
| A5 (API key echo onboarding) | YES | Correctly assigned to clawft-cli, P0 |
| A6 (Incomplete SSRF IP range) | YES | Correctly assigned to clawft-services, P0 |
| A7 (No HTTP timeout) | YES | Correctly assigned to clawft-llm, P1 |
| A8 (Unsafe set_var in tests) | YES | Correctly assigned to clawft-core, P1 |
| A9 (--no-default-features failure) | YES | Correctly assigned to clawft-cli, P1 |
| B1 (Unify Usage type) | YES | Correctly assigned to clawft-types + clawft-llm, P1 |
| B2 (Unify LlmMessage types) | YES | Correctly assigned to clawft-core, P1 |
| B3 (Split oversized files) | YES | Correctly marked as Multiple crates, P1 |
| B4 (Unify cron storage) | YES | Correctly assigned to clawft-cli + clawft-services, P1 |
| B5 (Extract tool registry builder) | YES | Correctly assigned to clawft-cli, P2 |
| B6 (Extract shared policy types) | YES | Correctly assigned to clawft-types, P2 |
| B7 (Deduplicate ProviderConfig) | YES | Correctly assigned to clawft-llm + clawft-types, P2 |
| B8 (Consolidate build_messages) | YES | Correctly assigned to clawft-core, P2 |
| B9 (MCP protocol version constant) | YES | Correctly assigned to clawft-services, P2 |
| I1 (DelegationTarget serde) | YES | Correctly assigned to clawft-types, P2 |
| I2 (String policy modes to enums) | YES | Correctly assigned to clawft-types, P2 |
| I3 (ChatMessage content serialization) | YES | Correctly assigned to clawft-llm, P2 |
| I4 (Job ID collision) | YES | Correctly assigned to clawft-cli, P2 |
| I5 (camelCase acronym handling) | YES | Correctly assigned to clawft-platform, P2 |
| I6 (Dead code removal) | YES | Correctly marked as Multiple crates, P2 |
| I7 (Always-true test assertion) | YES | Correctly assigned to clawft-core, P2 |
| I8 (Share MockTransport) | YES | Correctly assigned to clawft-services, P2 |
| J1 (Fix provider counts) | YES | P1 |
| J2 (Fix assembler truncation) | YES | P1 |
| J3 (Fix token budget source) | YES | P1 |
| J4 (Document identity bootstrap) | YES | P2 |
| J5 (Document rate-limit retry) | YES | P2 |
| J6 (Document CLI log level) | YES | P2 |
| J7 (Plugin system docs) | YES | P2, correctly noted as dependent on C1-C6 |

**Coverage verdict**: 33/33 items present. COMPLETE.

### 1.3 Exit Criteria Assessment

The current exit criteria (Section 3) have eight items. Each is evaluated:

| Exit Criterion | Specific? | Testable? | Issues |
|----------------|-----------|-----------|--------|
| "All P0 items resolved and tested" | Partially | Yes | Does not define what "tested" means -- unit tests? integration tests? manual verification? |
| "All P1 items resolved and tested" | Partially | Yes | Same ambiguity as above |
| "All P2 items resolved or documented as deferred" | Yes | Yes | Good -- "documented as deferred" provides an escape valve |
| "No files > 500 lines in modified crates (B3)" | Yes | Yes | Automatable via CI. Should clarify: does this include test files? `02-tech-core.md` Section 13.3 says "Rule: Each resulting file must be under 500 lines" but does not exclude tests. |
| "Zero clippy warnings" | Yes | Yes | Strong and automatable |
| "All 1,383+ existing tests still pass" | Yes | Yes | Good baseline, though the number will grow. Consider "all existing + new tests pass" |
| "Documentation matches code behavior for all J items" | Partially | Partially | "Matches code behavior" is subjective. Each J item needs a specific check. For example, J1 should say "docs/guides/providers.md lists exactly N providers, matching the count in PROVIDERS constant." |
| "No plaintext credentials in Debug output or serialized JSON" | Yes | Yes | Automatable with a grep/audit command. Should specify the verification method. |

**Gap 1**: Exit criteria do not include acceptance criteria from the business requirements. `01-biz-stability.md` specifies 9 testable success criteria for Stability and 8 for Type Safety. The orchestrator should reference or incorporate these.

**Gap 2**: No migration-specific exit criteria. A1 (session key migration), A2 (hash migration), and A4 (SecretRef migration) all require backward-compatible migration paths. The exit criteria should include: "Legacy session files are auto-migrated on first access (A1)", "Embeddings with old hash trigger warning on load (A2)", "Config files with plaintext credential strings still deserialize via backward compat (A4)".

**Gap 3**: No performance-related exit criteria for A7. The business requirement SF-7 specifies "LLM HTTP requests time out after the configured duration (default 120s) and return a descriptive error." The exit criteria should quantify this.

### 1.4 Dependency Analysis Within Element

The orchestrator states "Dependencies: None (this is the foundation layer)" for external dependencies, which is correct. However, internal dependencies between items within the element are not documented.

**Missing internal dependencies**:

| Item | Depends On | Reason |
|------|-----------|--------|
| B3 (file splits) | A4 (SecretRef) | The split of `clawft-types/src/config.rs` should include the new `secret.rs` module from A4. Doing A4 first avoids splitting then re-splitting. |
| B6 (policy types) | A6 (SSRF fix) | The canonical `UrlPolicy` in `clawft-types` should include the complete SSRF IP check from A6. A6 should land first or concurrently. |
| B7 (ProviderConfig naming) | B1 (Usage unification) | Both touch `clawft-llm` types. Should be coordinated. |
| I2 (policy mode enums) | B3 (file splits) | If `config.rs` is split into `config/policies.rs`, I2 should target the post-split file. |
| J7 (plugin docs) | Element 04 (C1-C6) | Correctly noted, but this means J7 cannot complete within Element 03's timeline. The orchestrator should explicitly mark J7 as "started in 03, completed after 04." |

### 1.5 Priority Assessment

| Item | Assigned Priority | Assessment | Issue? |
|------|-------------------|------------|--------|
| A1 (Session key corruption) | P0 | Correct -- data integrity bug affecting all multi-channel deployments | No |
| A2 (Unstable hash) | P1 | **SHOULD BE P0** -- `improvements.md` calls this a Bug; persisted embeddings silently produce incorrect results after toolchain upgrade. The business requirement SF-2 is marked P0. This is a data integrity issue. | YES |
| A3 (Invalid JSON) | P1 | Correct -- affects LLM reliability but has workarounds | No |
| A4 (Plaintext credentials) | P0 | Correct -- security vulnerability | No |
| A5 (API key echo) | P0 | Correct -- security vulnerability | No |
| A6 (SSRF incomplete) | P0 | Correct -- security vulnerability | No |
| A7 (No HTTP timeout) | P1 | Correct -- reliability issue, not security | No |
| A8 (Unsafe set_var) | P1 | Correct -- UB under Rust 2024 edition, but only in tests | No |
| A9 (--no-default-features) | P1 | Correct -- compile error for optional configuration | No |

**Priority fix needed**: A2 should be elevated from P1 to P0 to match the business requirement priority.

### 1.6 Crate Assignment Verification

All crate assignments are correct. Cross-checked against `improvements.md` and `02-tech-core.md`:

- A1 -> `clawft-core` (session.rs): Correct
- A4 -> `clawft-types` (config.rs): Correct
- A5 -> `clawft-cli` (onboard.rs): Correct
- A6 -> `clawft-services` (middleware.rs): Correct, with note that `02-tech-core.md` also mentions `clawft-tools` as a secondary target after B6 consolidation
- I5 -> `clawft-platform` (config_loader.rs): Correct
- B4 -> `clawft-cli` + `clawft-services`: Correct -- both crates have cron implementations that need unification

### 1.7 Timeline Assessment

**Weeks 1-5** for 33 items across 4 phases.

| Phase | Week Range | Item Count | Assessment |
|-------|-----------|------------|------------|
| Phase A (Security) | Week 1-2 | 9 items | Realistic. P0 items are surgical fixes. A4 (SecretRef) is the largest piece but has detailed tech spec. |
| Phase B (Architecture) | Week 2-4 | 9 items | Aggressive. B3 alone involves splitting 9 files (950-1646 lines each). `review-1-devguide.md` notes this is "aspirational" but defensible. B4 (cron unification) lacks a tech spec, which adds risk. |
| Phase I (Type Safety) | Week 2-4 | 8 items | Realistic. All items are individually small changes with clear fixes documented in `02-tech-core.md`. |
| Phase J (Documentation) | Week 3-5 | 7 items | Realistic for J1-J6. J7 cannot complete here (depends on C1-C6 from Element 04). |

**Overall timeline verdict**: Achievable but tight. B3 is the riskiest item for schedule slip. The overlapping timelines (Phase A and B overlap at Week 2, Phase B and I overlap at Week 2-4) assume parallel execution by different developers.

### 1.8 Issues from review-1-biz-tech.md Requiring Resolution

| Review Finding | Status in 03 Orchestrator | Action Needed |
|---------------|--------------------------|---------------|
| Missing tech spec for B4 | B4 listed but no tech spec exists in `02-tech-core.md` | **MUST FIX**: Add tech spec for B4 before development starts. Needs migration plan, API specification, JSONL format definition. |
| Missing tech spec for B8 | B8 listed but no tech spec exists | LOW: Straightforward refactor. Can be spec'd during development. |
| Missing tech spec for B9 | B9 listed but no tech spec exists | LOW: Trivial change (const extraction). |
| A7 non-goal wording issue | Not addressed | **SHOULD FIX**: `01-biz-stability.md` non-goal says "Supporting custom timeout values per-provider" but `02-tech-core.md` Section 12.7 adds `timeout_secs` to `ProviderConfig`, making per-provider timeout part of A7. The non-goal should say "Advanced timeout strategies (circuit breaker, adaptive timeouts)." |
| Section numbering (all should be Phase 5) | Not relevant to orchestrator directly | FIX IN DRAFTS: Business requirement sections should all use Phase 5, not mixed Phase 1.5/2/3G+/4. |
| Verify Chinese platform config structs exist | Not addressed | **SHOULD VERIFY**: Before A4 migration, confirm `FeishuConfig`, `DingTalkConfig`, `MochatConfig`, `QQConfig` exist in `clawft-types/src/config.rs`. |

### 1.9 Missing Acceptance Criteria

The following items lack acceptance criteria beyond the generic exit criteria:

**Phase A - Missing specific acceptance criteria**:

| Item | Missing Criteria | Recommended Addition |
|------|-----------------|---------------------|
| A1 | Migration behavior | "Existing session files using underscore encoding are automatically renamed to percent-encoded form on first startup. Both old and new format files are readable during migration." |
| A2 | Hash golden test | "A golden test asserts that `compute_embedding("hello world")` produces a specific known output vector, and this output is identical across x86_64-linux, aarch64-linux, and x86_64-darwin." |
| A4 | Backward compat | "Config files using the old `"imap_password": "literal_string"` format deserialize without error, logging a deprecation warning." |
| A6 | IPv6 coverage | "SSRF protection blocks IPv6 loopback (::1), link-local (fe80::), and unique local (fc00::/7) addresses." |
| A8 | Scope verification | "Zero occurrences of `set_var` or `remove_var` in test code outside of `temp_env` guards. Verified by `rg 'set_var|remove_var' --type rust crates/ -g '!target/'`." |

**Phase B - Missing specific acceptance criteria**:

| Item | Missing Criteria | Recommended Addition |
|------|-----------------|---------------------|
| B3 | Post-split verification | "Each resulting file is under 500 lines (excluding blank lines and comments). All public APIs are re-exported from the parent module. `cargo doc --workspace` produces no broken links." |
| B4 | Migration behavior | "Jobs created via the CLI are visible in `CronService` and vice versa. A one-time migration converts existing flat-JSON jobs to JSONL format." |

**Phase I - Missing specific acceptance criteria**:

| Item | Missing Criteria | Recommended Addition |
|------|-----------------|---------------------|
| I1 | Backward compat | "Old PascalCase values (`'Local'`, `'Claude'`) still deserialize correctly via serde aliases." |
| I6 | Completeness check | "Every `#[allow(dead_code)]` annotation has either been removed (code deleted) or has a `// TODO(workstream-X)` comment referencing the future workstream that will activate it." |

### 1.10 Specific Gaps and Fix Recommendations

**GAP-03-1: No internal dependency diagram**
- The orchestrator lists phases with week ranges but does not show which items within a phase depend on each other.
- **FIX**: Add a "Dependency Graph (Internal)" section showing: A4 -> B3 (config split), A6 -> B6 (policy consolidation), B1 -> B7 (both touch clawft-llm types).

**GAP-03-2: B4 has no tech spec**
- `review-1-biz-tech.md` identifies this as Medium severity. The orchestrator assigns it to Phase B but there is no implementation guidance in `02-tech-core.md`.
- **FIX**: Write tech spec section 13.7 in `02-tech-core.md` covering: JSONL event sourcing format, `CronService` API changes, CLI migration logic.

**GAP-03-3: A2 priority mismatch**
- Orchestrator says P1; business requirement SF-2 says P0.
- **FIX**: Elevate A2 to P0 in the orchestrator.

**GAP-03-4: Exit criteria lack migration acceptance tests**
- Three items (A1, A2, A4) have non-trivial migration paths that are not covered in exit criteria.
- **FIX**: Add three migration-specific exit criteria as listed in Section 1.9 above.

**GAP-03-5: J7 timeline is misleading**
- J7 is listed under Phase J (Week 3-5) but depends on C1-C6 from Element 04 (Week 3-8). J7 cannot complete within Element 03.
- **FIX**: Add note: "J7 starts during Phase J (framework documentation for C1). Full completion blocked until Element 04 C6 lands. Final J7 deliverable is tracked in Element 04's exit criteria."

**GAP-03-6: A7 non-goal contradiction with tech spec**
- Business draft non-goal says "Supporting custom timeout values per-provider" but the tech spec adds per-provider `timeout_secs` to `ProviderConfig`.
- **FIX**: Update the business requirement non-goal wording to: "Advanced timeout strategies (circuit breaker patterns, adaptive timeouts) -- simple per-provider timeout is included in A7."

---

## 2. Element 04: Plugin & Skill System

### 2.1 Overall Assessment: 6/10

Element 04 covers an ambitious scope -- a new crate, WASM host, skill loader, hot-reload, autonomous skill creation, slash-command wiring, MCP integration, and PluginHost unification. The orchestrator provides a good high-level phase breakdown but lacks critical detail in phases C4a, C5, C6, and C7. The tech specs for these items are identified as MISSING in `review-1-biz-tech.md`. The exit criteria are partially testable but several are too vague for development to start on the later phases.

### 2.2 Coverage Check Against improvements.md

| improvements.md Item | In 04 Orchestrator? | Notes |
|---------------------|---------------------|-------|
| C1 (Plugin trait crate) | YES | Phase C1, Week 3-4. Well specified in `02-tech-core.md` Section 14.1. |
| C2 (WASM plugin host) | YES | Phase C2, Week 4-5. Specified in `02-tech-core.md` Section 14.3. |
| C3 (Skill Loader) | YES | Phase C3, Week 5-6. Specified in `02-tech-core.md` Section 14.4. |
| C4 (Dynamic loading & hot-reload) | YES | Phase C4, Week 6-7. Specified in `02-tech-core.md` Sections 14.5-14.6. |
| C4a (Autonomous skill creation) | YES | Phase C4a, Week 7-8. **NO TECH SPEC** in `02-tech-core.md`. |
| C5 (Slash-command framework) | YES | Phase C5-C7, Week 7-8. **NO TECH SPEC** in `02-tech-core.md`. |
| C6 (MCP server skill exposure) | YES | Phase C5-C7, Week 7-8. **NO TECH SPEC** in `02-tech-core.md`. |
| C7 (PluginHost unification) | YES | Phase C5-C7, Week 7-8. **NO TECH SPEC** in `02-tech-core.md`. |

**Coverage verdict**: 8/8 items present. COMPLETE at the item level. However, C5-C7 are collapsed into a single row in the orchestrator with minimal description.

### 2.3 Exit Criteria Assessment

| Exit Criterion | Specific? | Testable? | Issues |
|----------------|-----------|-----------|--------|
| "`clawft-plugin` crate compiles with all trait definitions" | Yes | Yes | Automatable: `cargo build -p clawft-plugin` |
| "At least one plugin implements each trait" | Partially | Partially | Which traits? All six? "Implements" means a test plugin or a production plugin? This needs clarification. |
| "WASM plugin host loads and runs a test plugin" | Yes | Yes | Good. Should specify the test plugin's expected behavior. |
| "`weft skill install <path>` works for local skills" | Yes | Yes | Good. Specific CLI command with testable behavior. |
| "Hot-reload detects file changes within 1 second" | Yes | Yes | Excellent -- specific timing requirement. However, `01-biz-plugins-channels.md` says "within 2 seconds." Inconsistency. |
| "Skill precedence (workspace > managed > bundled) verified" | Yes | Yes | Good, but should specify the test scenario. |
| "VoiceHandler trait placeholder exists (forward-compat)" | Yes | Yes | Trivially verifiable by compilation. |
| "All existing tests pass" | Yes | Yes | Good baseline check. |

**Gap 1**: No exit criteria for C4a (autonomous skill creation). This is a complex feature that needs:
- "Agent detects repeated pattern after N occurrences (configurable)"
- "Generated SKILL.md validates against the SKILL.md schema"
- "Generated skill compiles to WASM and loads successfully"
- "Generated skill installs into `~/.clawft/skills/`"

**Gap 2**: No exit criteria for C5 (slash-command framework). Needs:
- "Agent commands routed through the registry, not inline match blocks"
- "Skills can contribute commands that appear in `/help`"
- "Existing built-in commands continue to work after wiring"

**Gap 3**: No exit criteria for C6 (MCP skill exposure). Needs:
- "Loaded skills appear as callable tools in MCP server tool listing"
- "External clients (VS Code, Claude Desktop) can discover and invoke skill-provided tools via MCP"

**Gap 4**: No exit criteria for C7 (PluginHost unification). Needs:
- "`PluginHost.start_all()` and `stop_all()` execute concurrently"
- "Existing channels (Telegram, Discord, Slack) work through the unified plugin system"
- "SOUL.md personality injection active in pipeline stages"

**Gap 5**: Hot-reload timing inconsistency: Orchestrator says 1 second, biz requirements say 2 seconds. Should be aligned to 2 seconds (the less aggressive target).

**Gap 6**: Missing exit criteria from `01-biz-plugins-channels.md` success criteria. The biz doc lists 14 criteria. The orchestrator has 8. Notable missing ones:
- "Plugin manifest schema validates JSON and YAML manifests"
- "Loaded skills appear in `weft skill list` output"
- "Slash-command registry discovers commands from loaded skills"
- "`PluginHost.start_all()` and `stop_all()` execute concurrently, not sequentially"
- "WASM plugins enforced under size budget (< 300 KB uncompressed)"
- "Autonomous skill creation generates valid SKILL.md + implementation from repeated patterns"

### 2.4 Dependency Analysis

**External dependencies stated**: "03 (B3 file splits must precede C1)"

This is correct and matches `review-1-devguide.md`'s finding that B3 -> C1 is an important dependency. However, the dependency is underspecified.

**Missing external dependencies**:

| Dependency | Reason | Source |
|-----------|--------|--------|
| A4 (credential redaction) | The `PluginHost` trait's `resolve_secret()` method (14.1.1) relies on the `SecretRef` pattern established in A4. | `02-tech-core.md` |
| A9 (feature-gated MCP) | C6 extends the MCP server. The feature-gating pattern from A9 must be established first. | Architectural inference |

**Internal dependencies stated**: The orchestrator has an implicit linear chain: C1 -> C2 -> C3 -> C4 -> C4a, C5-C7.

**Missing internal dependency detail from `01-biz-plugins-channels.md`**:

| Item | Depends On | Reason |
|------|-----------|--------|
| C2 (WASM host) | C1 (trait crate) | WASM host implements the plugin traits |
| C3 (Skill Loader) | C1 (trait crate) | Skills register as trait implementations |
| C3 (Skill Loader) | B3 (file splits) | `skills_v2.rs` YAML parser must be replaced first |
| C4 (hot-reload) | C2 + C3 | Hot-reload requires both WASM host and skill loader |
| C4a (autonomous creation) | C4 | Self-improving skills require the full hot-reload pipeline |
| C5 (slash-commands) | C3 | Dynamic commands come from loaded skills |
| C6 (MCP exposure) | C3 | MCP exposes loaded skills |
| C7 (unified PluginHost) | C1 | Unification targets the plugin trait system |

The orchestrator does not include this dependency table. It should.

### 2.5 Priority Assessment

The orchestrator does not assign explicit priorities (P0/P1/P2) to deliverables within Element 04. This is a significant gap compared to Element 03.

**Recommended priority assignment based on business requirements**:

| Phase | Recommended Priority | Rationale |
|-------|---------------------|-----------|
| C1 (Plugin trait crate) | P0 | Gates all plugin work. PS-1 and PS-2 are P0 in biz requirements. |
| C2 (WASM plugin host) | P1 | PS-3 and PS-4 are P1. |
| C3 (Skill Loader) | P1 | PS-5 and PS-6 are P1. |
| C4 (Hot-reload) | P1 | PS-7, PS-8, PS-9 are P1. |
| C4a (Autonomous skill creation) | P2 | PS-10 is P2. Most experimental feature in the element. |
| C5 (Slash-command wiring) | P2 | PS-11 is P2. Currently dead code that needs wiring. |
| C6 (MCP skill exposure) | P1 | PS-12 is P1. Critical for IDE integration. |
| C7 (PluginHost unification) | P2 | PS-14 is P2. Complex refactor. |

### 2.6 Crate Assignment Verification

| Item | Stated Crate(s) | Correct? | Issue |
|------|-----------------|----------|-------|
| C1 | `clawft-plugin` (new) | YES | New crate, well-specified in `02-tech-core.md` Section 14.1 |
| C2 | Not explicitly stated | UNCLEAR | `02-tech-core.md` shows code in both `clawft-wasm` and `clawft-core`. The orchestrator says "WASM Plugin Host" but does not state which crate. Should be `clawft-wasm` with glue in `clawft-core`. |
| C3 | Not explicitly stated | UNCLEAR | `02-tech-core.md` places skill loader in `clawft-core/src/agent/skill_loader.rs`. Orchestrator should state this. |
| C4 | Not explicitly stated | UNCLEAR | `02-tech-core.md` places hot-reload in `clawft-core/src/agent/skill_watcher.rs` and `skill_registry.rs`. |
| C4a | Not explicitly stated | UNCLEAR | No crate assignment. Likely `clawft-core` but needs confirmation. |
| C5 | Not explicitly stated | MISSING | `improvements.md` says `clawft-cli/src/interactive/`. |
| C6 | Not explicitly stated | MISSING | Likely `clawft-services/src/mcp/server.rs`. |
| C7 | Not explicitly stated | MISSING | `improvements.md` says `clawft-channels/src/host.rs`. |

**GAP**: The orchestrator omits crate assignments for all phases except C1 (which is a new crate). This is a significant omission -- Element 03 includes crate assignments for every item.

### 2.7 Timeline Assessment

**Weeks 3-8** for 8 items (C1-C7 + C4a).

| Phase | Week Range | Scope | Assessment |
|-------|-----------|-------|------------|
| C1 (Plugin trait crate) | Week 3-4 | New crate, 6 trait definitions, manifest schema | Realistic. Tech spec is detailed. The 6 traits are well-defined in `02-tech-core.md`. |
| C2 (WASM plugin host) | Week 4-5 | wasmtime integration, WIT definitions, WASI implementations | Tight. WASI filesystem and HTTP client are both significant. The WIT interface definition is detailed but the host-side implementation has incomplete code in `02-tech-core.md`. |
| C3 (Skill Loader) | Week 5-6 | SKILL.md parser, ClawHub discovery, auto-registration | Realistic for parser and registration. ClawHub discovery is underspecified (the `todo!()` stubs in tech spec). |
| C4 (Hot-reload) | Week 6-7 | File watcher, skill precedence, plugin-shipped skills | Realistic. The `notify` crate integration is straightforward. |
| C4a (Autonomous creation) | Week 7-8 | Pattern detection, skill generation, WASM compilation | HIGH RISK. No tech spec. Pattern detection heuristics are undefined. "Agent detects repeated task patterns" is hand-wavy without specifying what constitutes a pattern and what threshold triggers generation. |
| C5-C7 (Integration) | Week 7-8 | Slash-commands, MCP exposure, PluginHost unification | COMPRESSED. Three distinct features in 2 weeks. C7 (PluginHost unification) alone is a significant refactor touching `clawft-channels/src/host.rs`. |

**Timeline risk**: Weeks 7-8 are overloaded with C4a + C5 + C6 + C7. These four items, none of which have tech specs, are all scheduled for the same two weeks. This is the most significant timeline risk in the entire sprint.

**Recommendation**: Consider descoping C4a to post-MVP or at minimum sequencing it after C5-C7.

### 2.8 Issues from review-1-biz-tech.md Requiring Resolution

| Review Finding | Status in 04 Orchestrator | Action Needed |
|---------------|--------------------------|---------------|
| Missing tech spec for C4a | C4a listed with minimal description | **MUST FIX**: Write tech spec covering pattern detection heuristic, skill generation pipeline, WASM compilation step, installation to managed directory. Without this, development cannot start on C4a. |
| Missing tech spec for C5 | C5 listed in C5-C7 table | **MUST FIX**: Write tech spec covering how the existing dead code in `clawft-cli/src/interactive/` gets wired into the agent loop, how skills contribute commands. |
| Missing tech spec for C6 | C6 listed in C5-C7 table | **MUST FIX**: Write tech spec covering tool schema generation from loaded skills, MCP server changes, how skill metadata maps to MCP tool definitions. |
| Missing tech spec for C7 | C7 listed in C5-C7 table | **SHOULD FIX**: Write tech spec covering migration path from old `Channel` trait to new `ChannelAdapter` plugin trait, concurrent `start_all`/`stop_all`, SOUL.md injection. |

### 2.9 Missing Acceptance Criteria (Detailed)

**Phase C1 -- Missing**:
- "Plugin manifest validates against JSON Schema for both JSON and YAML formats"
- "`clawft-plugin` depends only on `clawft-types`, `serde`, `serde_json`, `async-trait`, `thiserror` (zero heavy deps)"
- "All six traits compile to WASM target (`wasm32-wasi`)"

**Phase C2 -- Missing**:
- "WASM module size enforcement rejects plugins over 300 KB uncompressed with a clear error message"
- "WASI filesystem read/write operations work within the sandbox directory"
- "WASM HTTP client can make outbound requests to allowed domains"
- "WASM plugin that attempts to access a non-permitted path receives a permission error"

**Phase C3 -- Missing**:
- "SKILL.md files with invalid YAML frontmatter produce a clear error message with line number"
- "Skills with nested YAML structures and multi-line values parse correctly (replacing the hand-rolled parser)"
- "`weft skill list` shows all loaded skills with their origin (workspace/managed/bundled)"

**Phase C4 -- Missing**:
- "Hot-reload does not interrupt in-flight tool calls (atomic swap with drain)"
- "File watcher debounces rapid changes (e.g., save-twice-quickly results in one reload, not two)"
- "Skill shadowing (workspace overriding managed) produces a log warning"

**Phase C4a -- ALL MISSING** (no acceptance criteria exist):
- "Pattern detection threshold is configurable (default: 3 repetitions)"
- "Generated SKILL.md passes the same validation as manually authored skills"
- "User is prompted for approval before auto-generated skills are installed"
- "Autonomous skill creation is disabled by default and must be opted into"

**Phase C5 -- ALL MISSING**:
- "All existing inline match-block commands in agent.rs work identically through the registry"
- "New commands contributed by skills appear in `/help` output"
- "Command name collisions between skills produce a clear error"

**Phase C6 -- ALL MISSING**:
- "MCP `tools/list` response includes all loaded skill tools with JSON Schema parameter definitions"
- "MCP `tools/call` for a skill-provided tool routes through the skill's `execute_tool()` method"
- "Adding/removing a skill via hot-reload updates the MCP tool listing without server restart"

**Phase C7 -- ALL MISSING**:
- "Existing Telegram, Discord, and Slack channels work through the unified PluginHost without behavior changes"
- "Channel startup is concurrent: 3 channels start in parallel, not sequentially"
- "SOUL.md content is injected into the Assembler pipeline stage system prompt"

### 2.10 Specific Gaps and Fix Recommendations

**GAP-04-1: No tech specs for C4a, C5, C6, C7**
- Four of eight phases lack implementation-level specifications. This is the single largest gap in Element 04.
- **FIX**: Write tech spec sections in `02-tech-core.md` (or a new `02-tech-plugin-integration.md`) for each. Minimum needed before development: function signatures, crate locations, data flow diagrams, and test requirements.

**GAP-04-2: No priority assignments**
- Element 03 assigns P0/P1/P2 to every item. Element 04 assigns none.
- **FIX**: Add priority column to all phase tables using the mapping in Section 2.5 above.

**GAP-04-3: No crate assignments for C2-C7**
- Only C1 has an explicit crate assignment (new `clawft-plugin`).
- **FIX**: Add crate column to all phase tables. Reference `improvements.md` and `02-tech-core.md` for the correct assignments.

**GAP-04-4: C5-C7 collapsed into a single phase**
- Three distinct features with different scopes and dependencies are presented as one table row. This makes it impossible to track progress or assess individual readiness.
- **FIX**: Split C5, C6, and C7 into separate phases with individual deliverable tables, exit criteria, and week assignments.

**GAP-04-5: Hot-reload timing inconsistency**
- Orchestrator: "within 1 second". Business requirements: "within 2 seconds".
- **FIX**: Align to 2 seconds (the more achievable target). If 1 second is the stretch goal, state both.

**GAP-04-6: Missing dependency on A4 (SecretRef)**
- The `PluginHost` trait exposes `resolve_secret()` which depends on the A4 `SecretRef` pattern.
- **FIX**: Add "A4 (credential redaction pattern)" to the dependencies section.

**GAP-04-7: No risk assessment**
- Element 03 does not have one either, but Element 04 carries significantly more risk due to C4a (autonomous creation) and C2 (WASM host).
- **FIX**: Add a risk section covering at minimum: WASM plugin sandbox escape (Score 5-8), skill hot-reload race conditions (Score 4), autonomous skill creation safety (Score 6), wasmtime version compatibility (Score 3).

**GAP-04-8: ClawHub discovery is underspecified**
- C3 mentions "ClawHub discovery (HTTP index + git clone)" but the tech spec has `todo!()` stubs.
- **FIX**: Either write the ClawHub client spec or explicitly defer ClawHub to K4 and note that C3 covers only local skill installation. The current state is ambiguous.

---

## 3. Cross-Element Analysis

### 3.1 Element 03 -> Element 04 Interface

The 03 -> 04 handoff is well-defined at the high level ("B3 file splits must precede C1") but needs more detail:

**Handoff checklist that should be documented**:
1. B3 completes the split of `clawft-types/src/config.rs` -- C1's `clawft-plugin` depends on types from this crate
2. B3 completes the split of `clawft-core/src/agent/skills_v2.rs` -- C3's skill loader refactors this file
3. A4's `SecretRef` pattern is established -- C1's `PluginHost::resolve_secret()` uses it
4. A6's complete SSRF check is in `clawft-types` (via B6) -- plugin HTTP access uses this for sandboxing
5. All P0/P1 items from Element 03 are resolved -- no feature work should build on known bugs

### 3.2 Consistency Between Elements

| Aspect | Element 03 | Element 04 | Consistent? |
|--------|-----------|-----------|-------------|
| Phase table format | Item/Description/Crate/Priority columns | Deliverable/Description columns only | NO -- 04 lacks Crate and Priority columns |
| Exit criteria granularity | 8 criteria, mix of specific and general | 8 criteria, mostly specific but missing C4a-C7 | NO -- 04 has larger gaps |
| Internal dependencies | Not documented | Not documented | Consistent (both missing) |
| Timeline format | Week ranges per phase | Week ranges per phase | YES |
| Risk assessment | Not present | Not present | Consistent (both missing, but more critical for 04) |

### 3.3 Items That Span Both Elements

| Item | Element 03 Role | Element 04 Role |
|------|----------------|-----------------|
| J7 (Plugin docs) | Listed in Phase J, noted as "after C1-C6" | Not listed | J7 should have a tracking entry in 04's exit criteria as well |
| B3 (File splits) | Phase B deliverable | Prerequisite | 03 must complete B3 before 04 can start C1 meaningfully |
| I6 (Dead code) | Lists `ResumePayload`, slash-command framework as dead code | C5 activates the slash-command framework | 03's I6 should annotate dead code with "// TODO(C5)" rather than removing it, and 04 should note this. |

---

## 4. Development Readiness

### Element 03: Critical Fixes & Cleanup

**Rating: NEEDS MINOR FIXES**

**What is ready**:
- All 33 items are identified and correctly assigned
- Phase A (Security) has excellent tech specs in `02-tech-core.md` for all 9 items
- Phase I (Type Safety) has complete tech specs for 7 of 8 items
- Phase B has tech specs for 6 of 9 items (B1, B2, B3, B5, B6, B7)
- Phase J is documentation-only and well-scoped
- Crate assignments are all correct

**What must be fixed before development starts**:
1. Add tech spec for B4 (cron storage unification) -- Medium effort
2. Elevate A2 priority from P1 to P0 to match business requirements
3. Add migration-specific exit criteria for A1, A2, A4
4. Add internal dependency diagram
5. Clarify J7 timeline (cannot complete within Element 03)
6. Resolve A7 non-goal wording contradiction

**Estimated effort to fix**: 2-4 hours of specification work.

---

### Element 04: Plugin & Skill System

**Rating: NEEDS MAJOR WORK**

**What is ready**:
- C1 (Plugin trait crate) has an excellent, implementation-ready tech spec
- C2 (WASM plugin host) has a good tech spec with WIT definitions and Rust code
- C3 (Skill Loader) has a good tech spec for the parser and registry
- C4 (Hot-reload) has a good tech spec for the file watcher mechanism
- The overall phase structure and dependency on Element 03 are sound

**What must be fixed before development starts**:
1. **Write tech specs for C4a, C5, C6, C7** -- These four items have zero implementation guidance. C4a (autonomous skill creation) in particular requires a specification for the pattern detection heuristic, the skill generation pipeline, and the WASM compilation step. C5-C7 need crate locations, function signatures, and migration paths. This is the blocking issue.
2. **Add priority assignments** to all phases (P0/P1/P2)
3. **Add crate assignments** to all phases (only C1 has one)
4. **Split C5-C7** from one collapsed row into three separate phases with individual deliverables
5. **Add acceptance criteria** for C4a, C5, C6, C7 (currently zero criteria for these)
6. **Resolve hot-reload timing inconsistency** (1s vs 2s)
7. **Add risk section** covering WASM sandbox, hot-reload races, autonomous creation safety
8. **Clarify ClawHub scope** -- is C3 local-only or does it include ClawHub discovery?
9. **Add dependency on A4** (SecretRef pattern used by PluginHost)
10. **Reduce Week 7-8 overload** -- four unspecified features in two weeks is not achievable

**Estimated effort to fix**: 8-16 hours of specification work, primarily writing the four missing tech specs.

---

## 5. Summary of All Findings

### By Severity

**CRITICAL (blocks development)**:
1. Element 04: No tech specs for C4a, C5, C6, C7 (4 missing specs)
2. Element 04: Week 7-8 overloaded with 4 unspecified features

**HIGH (should fix before development starts)**:
3. Element 03: Missing tech spec for B4 (cron storage unification)
4. Element 03: A2 priority mismatch (P1 in orchestrator, P0 in biz requirements)
5. Element 04: No priority assignments for any item
6. Element 04: No crate assignments for C2-C7
7. Element 04: C5-C7 collapsed into one indistinguishable phase

**MEDIUM (should fix early in development)**:
8. Element 03: Missing migration-specific exit criteria (A1, A2, A4)
9. Element 03: Missing internal dependency diagram
10. Element 04: Hot-reload timing inconsistency (1s vs 2s)
11. Element 04: Missing dependency on A4 (SecretRef)
12. Element 04: No risk assessment
13. Element 04: ClawHub scope ambiguous in C3
14. Element 03: A7 non-goal wording contradiction with tech spec

**LOW (can fix during development)**:
15. Element 03: B8 and B9 missing tech specs (trivial items)
16. Element 03: J7 timeline needs clarification
17. Element 03: Exit criteria should reference biz requirement success criteria
18. Element 04: Missing acceptance criteria for C1 (manifest validation, WASM target)
19. Element 04: Missing acceptance criteria for C2 (size enforcement, sandbox permissions)
20. Element 04: Missing acceptance criteria for C3 (error messages, skill listing)
21. Element 04: Missing acceptance criteria for C4 (atomic swap, debounce, shadowing warning)

### Scorecard

| Element | Score | Readiness | Primary Blocker |
|---------|-------|-----------|-----------------|
| 03 - Critical Fixes & Cleanup | **7/10** | NEEDS MINOR FIXES | B4 tech spec, A2 priority, migration exit criteria |
| 04 - Plugin & Skill System | **6/10** | NEEDS MAJOR WORK | C4a/C5/C6/C7 tech specs, priority/crate assignments, Week 7-8 overload |
