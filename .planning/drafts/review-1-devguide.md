# Review Report: Development Guide Drafts (03-dev-phases.md & 03-dev-contracts.md)

> Reviewer: Code Review Agent (Consensus Review Pass 1)
> Date: 2026-02-19
> Source of truth: `.planning/improvements.md`
> Existing doc: `.planning/03-development-guide.md`
> Draft 7: `.planning/drafts/03-dev-phases.md`
> Draft 8: `.planning/drafts/03-dev-contracts.md`
> Cross-refs: `01-biz-stability.md`, `01-biz-architecture.md`, `01-biz-plugins-channels.md`, `01-biz-features-deploy.md`

---

## 1. Stream-by-Stream Assessment (5A through 5K)

### Stream 5A: Critical Fixes & Type Safety (Week 1-2) -- PASS

**Coverage**: All 9 items from Workstream A (A1-A9) are present in Week 1, and all 8 items from Workstream I (I1-I8) are present in Week 2. Complete 1:1 mapping against improvements.md.

**Timeline**: Correctly maps to improvements.md "Week 1-2" for Workstream A and "Week 2-6" for Workstream I. Note: improvements.md says Workstream I spans Week 2-6, but 03-dev-phases.md compresses all of I into Week 2. This is aggressive but defensible since these are individually small changes and can land early.

**Issues**:
- None. This is a clean, faithful translation.

**Cross-doc alignment**: Matches biz-stability.md user stories SF-1 through SF-9 and TS-1 through TS-8 exactly. Success criteria in biz-stability.md align with validation section in dev-phases.md. No gaps.

---

### Stream 5B: Architecture Cleanup (Week 2-4) -- PASS WITH NOTES

**Coverage**: All items B1-B9 present. Week 2 covers type unification (B1, B2, B7, B8, B9). Week 3-4 covers structural splits (B3) and deduplication (B4, B5, B6).

**Timeline**: Improvements.md says "Week 2-4" -- matches exactly.

**Issues**:
1. MINOR: B3 involves splitting 9 files. The draft allocates 5 splits to Week 3 and 4 splits to Week 4. This is aggressive -- each split touches a large file (950-1646 lines) and requires careful re-export management. A more realistic estimate would be 2-3 splits per week, potentially stretching to Week 5. However, since the draft stays within improvements.md's stated timeline, this is acceptable as aspirational.
2. OBSERVATION: The draft correctly sequences B3 before C1 in the dependency chain (contracts draft identifies "B3 must precede C1"), which is not explicitly stated in improvements.md but is a sound architectural judgment. This is a GOOD addition.

**Cross-doc alignment**: Matches biz-architecture.md stories AC-1 through AC-9. Success criteria align.

---

### Stream 5C: Plugin & Skill System (Week 3-8) -- PASS WITH NOTES

**Coverage**: All items C1-C7, C4a present. Three sub-phases (5C.1, 5C.2, 5C.3) provide good internal structure.

**Timeline**: Improvements.md says "Week 3-8" -- matches exactly.

**Issues**:
1. MINOR: Sub-Phase 5C.1 places C3 prerequisite fix (replacing hand-rolled YAML parser) in Week 4, same as C2 (WASM host). Improvements.md positions the YAML parser fix as a prerequisite for C3, which itself starts in the C3 task. The draft handles this correctly by putting it in 5C.1.
2. OBSERVATION: C4a (autonomous skill creation) is placed in Week 8, the last item. This aligns with improvements.md's dependency chain (C4a depends on C4). Correct.
3. GOOD: The validation section includes "cargo build -p clawft-cli --no-default-features still compiles (plugin is feature-gated)" -- this cross-references 5A/A9 and ensures no regression, which is not in improvements.md but is a valuable addition.

**Cross-doc alignment**: Matches biz-plugins-channels.md stories PS-1 through PS-15. The WASM size budget (< 300KB uncompressed, < 120KB gzipped) is consistent across all documents. Dependencies table in biz-plugins-channels.md matches the dependency diagram in dev-phases.md.

---

### Stream 5D: Pipeline & LLM Reliability (Week 2-5) -- PASS

**Coverage**: All items D1-D11 present. Split into "Week 2-3: Core Pipeline Fixes" and "Week 4-5: Performance & Transport".

**Timeline**: Improvements.md says "Week 2-5" -- matches exactly.

**Issues**:
- None. Clean translation.

**Cross-doc alignment**: Matches biz-architecture.md stories PR-1 through PR-11 and success criteria.

---

### Stream 5E: Channel Enhancements (Week 4-8) -- PASS WITH NOTES

**Coverage**: All items E1-E6 (including E5a, E5b) present.

**Timeline**: Improvements.md says "Week 4-8" -- matches exactly.

**Issues**:
1. MINOR: E1 (Discord Resume) is placed in Week 4, but it has no dependency on the plugin system (it modifies an existing channel). This could start as early as Week 2. The draft is conservative here, which is fine.
2. OBSERVATION: The dependency note correctly identifies both C1 (plugin traits) and A4 (credential redaction for E2 OAuth). This matches improvements.md.

**Cross-doc alignment**: Matches biz-plugins-channels.md stories CE-1 through CE-14. The channel feature matrix in biz-plugins-channels.md is more detailed than anything in the dev guide drafts -- this is appropriate since the dev guide focuses on implementation plan, not feature specification. No misalignment.

---

### Stream 5F: Dev Tools & Applications (Week 5-10) -- PASS

**Coverage**: All items F1-F9 present.

**Timeline**: Improvements.md says "Week 5-10" -- matches exactly.

**Issues**:
1. MINOR: F6 (OAuth2 helper) is placed in Week 8 but E5a (Google Chat) is in Week 7 with a dependency on F6 per improvements.md. The biz-plugins-channels.md dependency table shows "E5a depends on F6 (OAuth2 helper)". The dev-phases draft has E5a in Week 7 and F6 in Week 8, which is an ordering problem. F6 should be moved to Week 6-7 or E5a should move to Week 8+.

**ISSUE SEVERITY: MEDIUM** -- dependency ordering violation between E5a and F6.

**Cross-doc alignment**: Matches biz-features-deploy.md stories DT-1 through DT-9.

---

### Stream 5G: Memory & Workspace (Week 4-8) -- PASS

**Coverage**: All items H1, H2 (8 sub-items), H3 present.

**Timeline**: Improvements.md says "Week 4-8" -- matches exactly.

**Issues**:
- None. Clean and well-structured with H2 sub-items broken out individually.

**Cross-doc alignment**: Matches biz-features-deploy.md stories MW-1 through MW-8. Success criteria align (e.g., "HNSW vector search returns results in < 10ms for 100K vectors" from biz doc, "Vector search returns semantically relevant results (not random)" from dev guide -- the biz doc is more specific on performance targets, which is appropriate).

---

### Stream 5H: Documentation Sync (Week 3-5) -- PASS

**Coverage**: All items J1-J7 present.

**Timeline**: Improvements.md says "Week 3-5" -- matches exactly.

**Issues**:
1. MINOR: J7 (Plugin system documentation) depends on C1-C6 per improvements.md. The draft places J7 in Week 5, but C6 completes in Week 8 (Sub-Phase 5C.3). This means J7 cannot be fully complete by Week 5. The draft acknowledges this with "(depends on C1-C6)" but the Week 5 placement is misleading. Recommendation: Note that J7 is "started" in Week 5 (covering C1 which lands Week 3-4) and "completed" after C6 lands.

**ISSUE SEVERITY: LOW** -- timeline note needed for J7.

**Cross-doc alignment**: Matches biz-features-deploy.md stories DS-1 through DS-7.

---

### Stream 5I: Multi-Agent Routing (Week 5-9) -- PASS

**Coverage**: All items L1-L4 present.

**Timeline**: Improvements.md says "Week 5-9" -- matches exactly.

**Issues**:
- None. Dependencies correctly identified (B5, C1, H1).

**Cross-doc alignment**: Matches biz-features-deploy.md stories MA-1 through MA-5.

---

### Stream 5J: Claude Flow Integration (Week 3-7) -- PASS WITH NOTES

**Coverage**: All items M1-M6 present. Two sub-phases ("Week 3-4: Core Integration" and "Week 5-7: Advanced Features").

**Timeline**: Improvements.md says "Week 3-7" -- matches exactly.

**Issues**:
1. OBSERVATION: M4 (Dynamic MCP Discovery) is listed as depending on F9 in improvements.md, but the dev-phases draft shows M4 starting in Week 5 while F9 is in Week 10. The contracts draft (03-dev-contracts.md) correctly notes this dependency: "M4 (Dynamic MCP Discovery) requires F9". This is a conflict -- if M4 truly requires F9, it cannot start in Week 5.

   Looking more carefully at improvements.md: "M4. Dynamic MCP server discovery... Deps: F9". But then the Timeline Summary shows M completing in Week 4-8. This suggests M4 can start prototyping its CLI interface independently while the full MCP client (F9) provides the underlying transport later. The dependency is on the transport, not on the CLI UX.

   Recommendation: Clarify in the dev guide that M4's CLI commands (`weft mcp add/list/remove`) can be implemented in Week 5-6 with basic connection logic, while full F9-grade connection pooling and auto-discovery integrates later.

**ISSUE SEVERITY: MEDIUM** -- M4/F9 dependency timeline needs clarification.

2. OBSERVATION: M2 and M3 are correctly identified as "standalone fixes that can land immediately" in the contracts draft. The dev-phases draft places them in Week 4, which is later than necessary. The contracts draft's critical path section says "M2 and M3 can land in Week 1 with zero dependencies." The dev-phases draft should align with this.

**ISSUE SEVERITY: LOW** -- M2/M3 could start earlier.

**Cross-doc alignment**: Matches biz-features-deploy.md stories CF-1 through CF-7.

---

### Stream 5K: Deployment & Community (Week 8-12) -- PASS

**Coverage**: K2-K5 and K3a present. K1 and K6 correctly excluded with note to `ui_development.md`.

**Timeline**: Improvements.md says "Week 8-12" -- matches exactly.

**Issues**:
1. OBSERVATION: Forward-compatibility requirements (structured/binary payloads, stable MCP schemas, direct config/session API access) are listed. These are correctly sourced from improvements.md's cross-cutting concerns for K1/K6.

**Cross-doc alignment**: Matches biz-features-deploy.md stories DC-1 through DC-11. Success criteria phases are labeled differently (biz doc says "Phase 6" for deployment, dev guide says "Phase 5" stream 5K) -- this is a labeling inconsistency across documents.

**ISSUE SEVERITY: LOW** -- phase numbering inconsistency in biz-features-deploy.md success criteria ("Phase 6" vs the dev guide's "Phase 5 stream 5K").

---

## 2. Timeline Validation

### Weekly Breakdown Cross-Check Against improvements.md

| Week | improvements.md Focus | 03-dev-phases.md Streams Active | Match? |
|------|----------------------|--------------------------------|--------|
| 1-2 | A (Critical Fixes), I (Type Safety) | 5A (A+I) | YES |
| 2-4 | B (Architecture), D (early Pipeline) | 5B (B), 5D (D early) | YES |
| 3-5 | J (Docs), C (start Plugin), M (start, M1-M3) | 5H (J), 5C starts, 5J starts | YES |
| 4-8 | C (Plugin full), E (Channels), H (Memory), M (complete, M4-M6) | 5C, 5E, 5G, 5J continues | YES |
| 5-9 | L (Multi-Agent), D (complete), F (Dev Tools) | 5I, 5D completes, 5F | YES |
| 8-12 | K (Deployment) | 5K | YES |

**Verdict**: The weekly breakdown is a faithful mapping of improvements.md's timeline summary. No weeks are misaligned.

### MVP Milestone (Week 8) Cross-Check

The dev-phases.md MVP checklist (Section 13) lists 22 items. Cross-checking against improvements.md's MVP milestone:

| improvements.md MVP Item | In dev-phases MVP? | Notes |
|--------------------------|-------------------|-------|
| Plugin system working with skill precedence + hot-reload | YES (C1-C4) | |
| Email channel | YES (E2) | |
| Multi-agent routing | YES (L1) | |
| 3 ported OpenClaw skills | YES | |
| MCP client for external servers | YES (implied via M4) | improvements.md says F9 at MVP, dev-phases says M4 |
| Claude Flow integration functional | YES (M1, M3, M4) | |
| All critical/high fixes resolved | YES (A1-A9) | |

**ISSUE**: improvements.md MVP milestone explicitly includes "MCP client for external servers" (F9), but dev-phases.md places F9 in Week 10, well past the Week 8 MVP. The dev-phases MVP checklist does not list F9. This is a discrepancy.

**ISSUE SEVERITY: MEDIUM** -- F9 in MVP timeline is inconsistent between source doc and dev guide draft.

---

## 3. Dependency Diagram Correctness Check

### Main Diagram (Section 14) Assessment

The ASCII dependency diagram in 03-dev-phases.md shows:

```
5A -> 5B -> 5C -> 5E -> 5F
5A -> 5G
5D -> 5J (Claude Flow)
5D -> 5F (F9)
5C -> 5I -> 5K
5H (docs) follows feature completion
```

**Correct relationships captured**:
- A2 (hash fix) -> H2 (vector memory): YES
- A4 (cred redaction) -> E2 (email OAuth): YES (in "Key Dependency Chains")
- B4 (cron unify) -> E6 (heartbeat): YES
- B5 (tool registry) -> L1 (agent routing): YES
- C1 -> everything: YES (correctly identified as hub)
- C2 -> C4 -> C4a: YES
- C3 -> C4, C5, C6: YES
- C6 -> F8: YES
- D6 -> L4: YES
- D9 -> M1, F9: YES
- H1 -> L2: YES
- H2 -> K4: YES
- K3 -> K3a: YES
- L1 -> L2 -> L3: YES
- M1 -> M5: YES
- M4 -> M5: YES

**Missing relationships**:
1. B3 -> C1 dependency is captured in the contracts draft but NOT in the phases draft's dependency diagram. The contracts draft correctly notes "B3 (File Splits) must precede C1 -- large files must split before trait extraction." This should be added to the phases draft diagram.
2. F6 (OAuth2 helper) -> E5a (Google Chat) is in improvements.md but not in the dev-phases dependency diagram. It IS in the biz-plugins-channels.md dependency table.
3. C3 prereq fix (YAML parser replacement) dependency is implicit but not shown.

**ISSUE SEVERITY: MEDIUM** -- B3 -> C1 dependency missing from phases draft diagram.
**ISSUE SEVERITY: LOW** -- F6 -> E5a dependency missing from diagram.

**Incorrect relationships**: None found. All depicted relationships are accurate.

---

## 4. Overlap Analysis Between the Two Dev Guide Drafts

### 03-dev-phases.md (Draft 7) Scope
- Phase overview table (11 streams, parallel groups)
- Detailed weekly task tables for each stream (Sections 2-12)
- Phase 5 milestone checklist (Section 13)
- Stream dependency diagram (Section 14)
- Cross-cutting concerns (Section 15)
- Recommended dependencies table (Section 15)
- Risk mitigation table (Section 16)
- File ownership table (Section 17)
- Branching strategy (Section 17)
- Definition of Done (Section 18)

### 03-dev-contracts.md (Draft 8) Scope
- Dependency management between streams (Section 5)
- Interface contracts #13-#19 (Section 5)
- File ownership table (Section 5)
- Critical path analysis (Section 6)
- Risk mitigation table (Section 7)
- Definition of Done (Section 8)
- Testing contracts (Section 8)

### Overlap Areas

| Topic | In Draft 7? | In Draft 8? | Conflict? |
|-------|------------|------------|-----------|
| Dependency diagram/management | YES (Section 14) | YES (Section 5) | PARTIAL -- see below |
| File ownership | YES (Section 17) | YES (Section 5) | MINOR differences |
| Risk mitigation | YES (Section 16) | YES (Section 7) | COMPLEMENTARY -- different risks listed |
| Definition of Done | YES (Section 18) | YES (Section 8) | OVERLAPPING -- draft 8 extends draft 7 |
| Critical path | NO | YES (Section 6) | N/A |
| Interface contracts | NO | YES (Section 5) | N/A |
| Testing contracts | NO | YES (Section 8) | N/A |
| Branching strategy | YES (Section 17) | NO | N/A |
| Cross-cutting concerns | YES (Section 15) | NO | N/A |
| Weekly task tables | YES (Sections 2-12) | NO | N/A |
| Milestone checklists | YES (Section 13) | YES (in Section 6 MVP/Full) | PARTIAL -- different level of detail |

### Dependency Management Conflict Detail

Draft 7 uses an ASCII art diagram showing stream-level dependencies.
Draft 8 uses a textual dependency tree organized by Workstream (A, B, C, D, E, F, H, I, J, K, L, M).

**Key differences**:
- Draft 8 includes the B3 -> C1 dependency; Draft 7 does not.
- Draft 8 provides "Key dependency observations" commentary; Draft 7 provides a "Key Dependency Chains" section with itemized arrows.
- Draft 8's format is more detailed and granular (per-item dependencies). Draft 7's format is more visual.
- Both are accurate where they overlap, but Draft 8 is more complete.

### File Ownership Conflict Detail

Both drafts have file ownership tables. Draft 7 organizes by stream (5A-5K). Draft 8 organizes by workstream letter (A-M) with more granular per-item breakdown (e.g., separate rows for C1, C2, C3, C4, C4a, C5, C6, C7).

Draft 8 additionally identifies "Conflict zones" requiring coordination:
- `clawft-types/src/config.rs` (touched by A4, B3, B7, I2)
- `clawft-core/src/agent/loop_core.rs` (touched by A3, B3, D1)
- `clawft-services/src/mcp/` (touched by C6, F8, F9, M4, M5)
- `clawft-cli/Cargo.toml` (touched by M3 and plugin streams)

This is valuable information missing from Draft 7.

### Risk Mitigation Overlap

Draft 7 lists 7 risks. Draft 8 lists 12 risks. Only 2 risks appear in both:
- WASM plugin size exceeds budget (both drafts)
- Skill precedence conflicts (both drafts, worded differently)

The two risk tables are largely complementary and should be merged.

### Definition of Done Overlap

Draft 7: 10 items (items 1-10).
Draft 8: 13 items (items 1-13), where items 1-8 are identical to Draft 7 items 1-8, and items 9-13 are new additions.

Draft 7 also has two unique items:
- Item 9: "New channels register through `ChannelAdapter` trait"
- Item 10: "Plugin dependencies are behind feature flags"

These do not appear in Draft 8's list but are implicitly covered by Draft 8's items 9 (plugin trait compliance) and 13 (plugin isolation).

---

## 5. Interface Contracts Assessment (Contracts #13-#19)

### Contract #13: Plugin Traits

**Assessment**: Well-defined. Six traits with clear method signatures. The `VoiceHandler` placeholder is correctly marked. The `supports_binary() -> bool` on `ChannelAdapter` addresses the forward-compat requirement from improvements.md.

**Missing**: No lifecycle methods beyond `start()`/`stop()` on `ChannelAdapter`. Consider adding `on_config_change()` or `reconfigure()` for hot-reload scenarios. However, this may be over-engineering for the initial contract -- acceptable to defer.

**Cross-doc check**: biz-plugins-channels.md story PS-1 lists the same six traits. Consistent.

### Contract #14: Plugin Manifest Schema

**Assessment**: Good. Includes `capabilities`, `wasm`, `skills_dir`, `permissions`, and `config_schema`. The `max_size_kb: 300` aligns with improvements.md's size budget.

**Missing**: No `version` field for the manifest schema itself (distinct from the plugin version). If the schema evolves, consumers need to know which schema version to validate against. Recommendation: add `"manifest_version": 1`.

### Contract #15: SKILL.md Format Spec

**Assessment**: Clean YAML frontmatter spec. Includes `triggers`, `parameters`, `source` field for wasm/native/prompt.

**Missing**: No `dependencies` field for skills that require other skills or specific plugin capabilities. This may be needed for ClawHub (K4) where a published skill depends on another. However, this can be deferred to K4.

### Contract #16: Agent Routing Table Config

**Assessment**: Clear TOML format with wildcard support and first-match-wins semantics. Backward compatible (missing section = single shared agent).

**No issues**.

### Contract #17: FlowDelegator Protocol

**Assessment**: Well-specified. Spawn mechanism, I/O, MCP callback, fallback, health check with 5-minute cache.

**Missing**: No specification of what happens when the `claude` process exits with a non-zero status code. Recommendation: add error handling contract (e.g., "Non-zero exit code: extract stderr, return as delegation error, do not retry").

### Contract #18: Dynamic MCP Discovery Protocol

**Assessment**: Good. CLI commands, reconnection backoff (1s-60s), health check, hot-reload via notify.

**No issues**.

### Contract #19: Security Plugin Audit Check Interface

**Assessment**: Well-defined trait with severity levels and finding structure. 50+ built-in checks listed.

**Missing**: No specification of the check execution order or whether checks can be parallelized. For 50+ checks this matters for performance. However, this is an implementation detail, not a contract issue.

### Are 7 New Contracts Sufficient?

**Assessment**: Mostly yes. The contracts cover the major new subsystems (plugins, skills, routing, delegation, MCP discovery, security).

**Potentially missing contracts**:
1. **Hot-reload protocol**: How does the skill registry swap work? What guarantees are provided during reload? The contracts draft touches this in the testing section but not as a formal contract.
2. **Cross-agent communication protocol**: L3 (multi-agent swarming) involves agents communicating via the message bus. The message format and coordination semantics are not specified as a contract. This could be folded into an extension of contract #8 (MessageBus API).
3. **ClawHub registry API**: K4 involves an HTTP registry. The API format (REST endpoints, response schema, search semantics) is not specified. This can likely wait until K4 is closer.

**ISSUE SEVERITY: LOW** -- two potentially missing contracts (hot-reload, cross-agent), but neither is blocking for early phase work.

---

## 6. Critical Path Analysis Review

The contracts draft (Section 6) identifies the critical path as:

```
A (W1-2) -> B3 (W2-3) -> C1 (W3-4) -> C2+C3 (W4-6) -> C4 (W6-7) -> K (W8-12)
```

**Assessment**: This is correct. The critical path runs through the plugin infrastructure because it gates all feature streams. The B3 -> C1 gate is a good catch (not in improvements.md but architecturally sound).

**However**, the stated "Total critical path: 12 weeks to full vision" counts from Week 1 through Week 12, which is the entire sprint duration. This is technically true but not very informative -- a critical path that equals the total project duration means there is zero float on any item in the chain. This is realistic for K (deployment) which has no float, but the individual segments have some float:
- A can overlap with B3 (Week 2 overlap)
- B3 can overlap with C1 (Week 3 overlap)
- C2 and C3 are parallel

The "12 weeks" is the elapsed calendar time, not the sequential work time. This is correct usage.

**Parallel acceleration opportunities** are correctly identified:
- D is fully parallel with B and C. Correct.
- I is independent from Week 2. Correct.
- M2 and M3 can land in Week 1. Correct -- these are standalone config changes.
- E and F can prototype against C1 trait drafts. Correct.

**MVP milestone (Week 8)** analysis is accurate and aligns with improvements.md.

---

## 7. File Ownership Assessment

### Existing Phase Conflicts

The existing 03-development-guide.md defines ownership for Phases 1-4. Phase 5 introduces new streams that touch the same crates:

| Crate | Existing Owner(s) | Phase 5 Claimant(s) | Conflict? |
|-------|-------------------|---------------------|-----------|
| `clawft-types` | 1A, 3D, 4A | 5A (serde fixes), 5B (unification, splits), 5G (timestamps), 5I (routing) | YES -- 4 Phase 5 streams touch this crate |
| `clawft-core` | 1B, 2B, 3E | 5B (splits), 5C (skills), 5D (pipeline), 5G (memory), 5I (routing) | YES -- 5 Phase 5 streams |
| `clawft-cli` | 1C, 2D, 3D, 3F | 5A (onboard fix), 5C (interactive), 5J (Cargo.toml), 5K (security CLI) | YES -- 4 Phase 5 streams |
| `clawft-services` | 2C, 3F, 3G | 5B (cron), 5E (heartbeat), 5F (MCP), 5J (delegation), 5K (sandbox) | YES -- 5 Phase 5 streams |

The contracts draft (Draft 8) correctly identifies the "Conflict zones" for Phase 5 inter-stream conflicts. However, neither draft addresses the Phase 4/Phase 5 transition -- when Phase 4 streams (4A-4D) are complete, do their ownership claims release? The existing guide does not specify a Phase 4 completion gate before Phase 5 starts.

**Recommendation**: Add a note that Phase 5 assumes Phase 4 is complete and its ownership claims are released. If Phase 4 and Phase 5 overlap, additional coordination rules are needed for `clawft-types` (owned by both 4A and 5A/5B).

**ISSUE SEVERITY: MEDIUM** -- Phase 4/5 transition ownership not addressed.

---

## 8. Branching Strategy Assessment

Draft 7 (Section 17) proposes:
- `master` -- stable, do not commit directly
- `sprint/phase-5` -- integration branch
- `sprint/5a-fixes` through `sprint/5k-deploy` -- 11 stream branches

**Assessment**:
1. GOOD: Consistent with the existing strategy (Phase 1-3 used `weft/phase-X` and stream branches).
2. GOOD: The naming convention changed from `weft/` to `sprint/` to distinguish from the existing phase branches. This avoids confusion.
3. CONCERN: 11 stream branches is a lot of concurrent branches to maintain. The existing guide has at most 9 (Phase 3: 3A-3I) but those are sequential, not 11 concurrent. With 11 branches, merge coordination becomes complex. The "merge to sprint/phase-5 when stream milestones pass" rule is necessary but the order of merges matters (e.g., 5B must merge before 5C can merge cleanly).
4. ALIGNMENT: The `.claude/CLAUDE.md` says "Do not EVER commit to master, unless this rule is removed." The branching strategy respects this with "do not commit directly." Consistent.

**ISSUE SEVERITY: LOW** -- branch count is high but manageable with the merge-order constraint from the dependency diagram.

---

## 9. Milestone Checklists Assessment

### MVP Milestone (Week 8) -- 22 items

**Specificity**: Each item references the specific improvement item (e.g., "A1-A9", "C1", "M3"). This is good for traceability.

**Verifiability**: Most items are verifiable (e.g., "All files under 500 lines (B3 splits)", "Hot-reload watcher functional (C4)"). However, some are vague:
- "3+ ported OpenClaw skills working" -- which skills? What does "working" mean? Recommendation: name the specific skills targeted for porting.
- "Documentation sync complete (J1-J6)" -- is J7 excluded from MVP? Yes, correctly, since J7 depends on C1-C6 which finishes after Week 8. But this should be explicit: "J1-J6 complete; J7 started (depends on C6)."

### Full Sprint Milestone (Week 12) -- 25 items

**Specificity**: Good, with item references throughout.

**Alignment with biz-requirements success criteria**:
- biz-features-deploy.md says "Docker image published" and "weft security scan detects at least 5 known-bad patterns" -- these are more specific than the dev guide's "Multi-arch Docker images (K2)" and "Security plugin with 50+ audit checks (K3a)." The biz doc's criteria should be referenced from the dev guide.
- biz-features-deploy.md places Deployment success criteria under "Phase 6" -- this numbering does not match the dev guide's "Phase 5." This needs reconciliation.

**ISSUE SEVERITY: LOW** -- milestone items could be more specific in a few cases; phase numbering inconsistency with biz doc.

---

## 10. Cross-Document Alignment Issues

### Phase Numbering Inconsistency

| Document | How it refers to the sprint work |
|----------|----------------------------------|
| improvements.md | "Workstreams A-M" (no phase number) |
| 03-dev-phases.md | "Phase 5" with streams 5A-5K |
| 01-biz-stability.md | "Phase 1.5" (stability), "Phase 2" (type safety) |
| 01-biz-architecture.md | "Phase 3G+" (arch cleanup), "Phase 4" (pipeline) |
| 01-biz-plugins-channels.md | "Phase 5: Extend" |
| 01-biz-features-deploy.md | "Phase 5" (most), "Phase 6" (deployment), "Phase 4" (docs) |

**This is a significant inconsistency.** The dev guide calls everything "Phase 5." The business requirement drafts use different phase numbers depending on which agent wrote them. This needs to be standardized.

**Recommendation**: All drafts should align on "Phase 5" for the unified sprint, since that is what the dev guide uses and it follows sequentially from the existing Phase 4 in `03-development-guide.md`.

**ISSUE SEVERITY: HIGH** -- cross-document phase numbering is inconsistent and will confuse readers.

### Workstream G (Voice) and K1/K6 (UI) Exclusions

All documents consistently mark Voice and UI as out of scope. The forward-compatibility hooks are consistently specified across the dev guide and biz-features-deploy.md. No misalignment.

### Success Criteria Mapping

The business requirement drafts define success criteria that should map to the dev guide's milestone checklists. Spot-checking:

| Biz Success Criterion | Dev Guide Coverage |
|----------------------|-------------------|
| "Session keys with underscores round-trip correctly" (biz-stability) | "Session round-trip test: keys with underscores survive encode/decode (A1)" -- MATCH |
| "HNSW vector search returns results in < 10ms for 100K vectors" (biz-features-deploy) | "Vector search returns semantically relevant results" -- WEAKER, no performance target |
| "At least 3 dev tool plugins functional and MCP-exposed" (biz-features-deploy) | "Git, Cargo, tree-sitter, browser plugins (F1-F4)" -- MATCH (lists 4) |
| "Docker image published for linux/amd64 and linux/arm64" (biz-features-deploy) | "Multi-arch Docker images (K2)" -- MATCH |

The dev guide's validation sections are generally less specific than the biz success criteria. This is acceptable -- the dev guide focuses on implementation milestones while the biz doc defines acceptance criteria. However, there should be explicit cross-references between them.

### Story ID Coverage

The biz drafts define story ID prefixes (SF-, TS-, AC-, PR-, PS-, CE-, DT-, MW-, DC-, MA-, CF-, DS-). The dev guide does not reference these IDs. Adding story ID references to the dev guide task tables (e.g., "Task: Fix session key round-trip | Item: A1 | Story: SF-1") would improve traceability.

**ISSUE SEVERITY: LOW** -- traceability would improve with story ID cross-references, but not required.

---

## 11. Specific Suggested Fixes

### Critical Fixes (Must Address Before Merge)

1. **F6/E5a dependency ordering** (Draft 7, Section 6): Move F6 (OAuth2 helper) from Week 8 to Week 6-7 so it precedes E5a (Google Chat, Week 7). Alternative: move E5a to Week 8+.

2. **Phase numbering standardization** (All biz drafts): Standardize all biz requirement drafts to use "Phase 5" for the unified sprint work, matching the dev guide. Update biz-stability.md ("Phase 1.5"/"Phase 2"), biz-architecture.md ("Phase 3G+"/"Phase 4"), and biz-features-deploy.md ("Phase 4"/"Phase 5"/"Phase 6").

3. **B3 -> C1 dependency in phases draft** (Draft 7, Section 14): Add B3 -> C1 to the dependency diagram. Currently only in the contracts draft.

### Important Fixes (Should Address)

4. **F9 MVP inclusion** (Draft 7, Section 13): Either add F9 to the MVP milestone checklist (matching improvements.md's MVP definition) or note that F9 is a stretch goal for MVP. The current omission is a discrepancy.

5. **M4/F9 dependency clarification** (Draft 7, Section 11): Add a note that M4's CLI layer can be built independently in Week 5-6, with F9-grade transport integration coming later. This resolves the apparent timeline conflict.

6. **Phase 4/5 transition** (Draft 8, Section 5): Add a note that Phase 5 assumes Phase 4 is complete. If they overlap, specify coordination rules for `clawft-types` and `clawft-core/pipeline`.

7. **J7 timeline clarification** (Draft 7, Section 9): Note that J7 starts in Week 5 but completes after C6 lands (Week 8).

8. **M2/M3 early start** (Draft 7, Section 11): Move M2 and M3 to Week 1-2 (they have zero dependencies per both drafts' analysis).

### Minor Fixes (Nice to Have)

9. **Manifest schema version** (Draft 8, Contract #14): Add `"manifest_version": 1` to the plugin manifest schema.

10. **FlowDelegator error handling** (Draft 8, Contract #17): Add non-zero exit code handling contract.

11. **MVP skill names** (Draft 7, Section 13): Specify which 3 OpenClaw skills are targeted for porting.

12. **Performance targets in dev guide** (Draft 7): Reference biz doc performance targets (e.g., "< 10ms for 100K vectors" for H2) in the validation sections.

13. **Story ID cross-references** (Draft 7): Add story IDs (SF-*, TS-*, etc.) to the task tables for traceability.

---

## 12. Merge Strategy Recommendation

### How to Combine the Two Drafts into 03-development-guide.md

The two drafts have complementary scopes with limited overlap. The recommended merge strategy:

**Step 1: Take Draft 7 (03-dev-phases.md) as the primary structure.**
It contains the stream-by-stream task tables, milestone checklists, and dependency diagrams -- the core "what to build and when" content that parallels existing Phases 1-4 in the development guide.

**Step 2: Merge Draft 8 (03-dev-contracts.md) content into the appropriate sections of the combined document.**

Specific merge targets:

| Draft 8 Section | Merge Into |
|-----------------|-----------|
| Dependency management (Section 5, textual tree) | Replace Draft 7's dependency diagram (Section 14) with Draft 8's more complete version. Keep Draft 7's ASCII diagram as a visual summary but add B3->C1 and other missing edges. Incorporate "Key dependency observations" from Draft 8. |
| Interface contracts #13-#19 (Section 5) | Insert as a new subsection in the combined doc's "Concurrent Development Rules" section, extending existing contracts #1-#12. |
| File ownership (Section 5) | Merge with Draft 7's file ownership table. Use Draft 8's more granular per-item breakdown. Add "Conflict zones" from Draft 8. |
| Critical path (Section 6) | Add as new section in combined doc. Draft 7 has no critical path for Phase 5 -- this fills the gap. |
| Risk mitigation (Section 7) | Merge with Draft 7's risk table. De-duplicate the 2 overlapping risks. Combined table will have ~17 unique risks. |
| Definition of Done (Section 8) | Use Draft 8's 13-item list (which extends Draft 7's 10 items). Fold in Draft 7's items 9-10 (channel trait, feature flags) explicitly. |
| Testing contracts (Section 8) | Add as new subsection. This content does not exist in Draft 7. |

**Step 3: Structural placement within 03-development-guide.md.**

The existing guide has this structure:
1. Phase Overview (Section 1)
2. Phase 1: Warp (Section 2)
3. Phase 2: Weft (Section 3)
4. Phase 3: Finish (Section 4)
5. Phase 4: Tiered Router (Section 4b)
6. Concurrent Development Rules (Section 5)
7. Critical Path (Section 6)
8. Risk Mitigation (Section 7)
9. Definition of Done (Section 8)

Recommended additions:
- Add "Phase 5: Sprint" as **Section 4c** (after Phase 4, before Concurrent Development Rules)
- Update the Phase Overview table (Section 1) to include Phase 5
- Extend Section 5 (Concurrent Development Rules) with Phase 5 dependency management, contracts #13-#19, file ownership, and conflict zones
- Extend Section 6 (Critical Path) with Phase 5 critical path
- Extend Section 7 (Risk Mitigation) with merged risk table
- Extend Section 8 (Definition of Done) with items 9-13

**Step 4: De-duplication.**
- Remove duplicate dependency diagrams (keep the more complete one)
- Remove duplicate file ownership tables (keep the more granular one)
- Remove duplicate risk entries
- Remove duplicate DoD items

---

## 13. Overall Quality Assessment

### Draft 7: 03-dev-phases.md -- Score: 8/10

**Strengths**:
- Comprehensive coverage of all 11 streams with weekly task breakdowns
- Faithful mapping of improvements.md items to development tasks
- Clear validation criteria per stream
- Well-structured milestone checklists (MVP and Full Sprint)
- Consistent formatting and organization

**Weaknesses**:
- F6/E5a dependency ordering error
- Missing B3->C1 in dependency diagram
- F9 omitted from MVP despite being in source doc's MVP
- M2/M3 placed later than necessary
- No critical path analysis (deferred to contracts draft)
- Some validation criteria less specific than biz doc success criteria

### Draft 8: 03-dev-contracts.md -- Score: 8.5/10

**Strengths**:
- Excellent interface contracts (#13-#19) with concrete type signatures and wire formats
- Superior dependency analysis with key observations and conflict zone identification
- Strong critical path analysis with parallel acceleration opportunities
- Comprehensive risk mitigation with actionable response strategies
- Extended Definition of Done with plugin-specific criteria
- Testing contracts provide implementation-ready test specifications

**Weaknesses**:
- References existing sections ("extends existing contracts 1-12") without reproducing them, making it harder to read standalone
- Some contracts could be more complete (manifest schema version, FlowDelegator error handling)
- No weekly task breakdowns (by design -- complementary to Draft 7)
- Missing contracts for hot-reload protocol and cross-agent communication
- Phase 4/5 transition not addressed

### Combined Assessment

When merged, these two drafts will form a solid Phase 5 addition to the development guide. The complementary scope means minimal conflict during merge. The main work is:
1. Fix the dependency ordering issues (F6/E5a, B3/C1 in diagram)
2. Standardize phase numbering across all drafts
3. Reconcile F9 MVP inclusion
4. Merge overlapping sections (risks, DoD, file ownership)

The combined quality would be approximately **8.5/10** after the suggested fixes are applied.

---

## Summary of All Issues by Severity

### HIGH (1 issue)
1. Cross-document phase numbering inconsistency (biz drafts use Phase 1.5/2/3G+/4/5/6 vs dev guide's Phase 5)

### MEDIUM (4 issues)
2. F6/E5a dependency ordering violation (F6 Week 8, E5a Week 7)
3. F9 missing from MVP milestone checklist (in improvements.md MVP, not in dev guide MVP)
4. B3 -> C1 dependency missing from phases draft dependency diagram
5. Phase 4/5 ownership transition not addressed

### LOW (7 issues)
6. M4/F9 dependency timeline needs clarification
7. J7 timeline placement (Week 5 but depends on C6 at Week 8)
8. M2/M3 could start in Week 1-2 instead of Week 4
9. Phase numbering inconsistency in biz-features-deploy.md success criteria sections
10. Milestone specificity (which 3 OpenClaw skills? J7 MVP exclusion explicit?)
11. Missing contracts for hot-reload protocol and cross-agent communication
12. Story ID cross-references would improve traceability
