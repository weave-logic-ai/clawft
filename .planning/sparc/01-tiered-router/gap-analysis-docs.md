# Gap Analysis: Documentation Completeness & Development Notes Quality
# Sprint: 01-tiered-router (Tiered Router & Permission System)

**Analyst**: Code Review Agent
**Date**: 2026-02-18
**Scope**: Documentation deliverables, development notes completeness, plan-to-docs consistency, consensus log quality
**Reference Format**: Sprint 0 (`00-initial-sprint/`)

---

## 1. Documentation Deliverables Assessment

### 1.1 Phase I Documentation Plan Review

Phase I (`I-testing-documentation.md`) defines the following documentation outputs:

| Deliverable | Planned Location | Status in Plans | Assessment |
|-------------|-----------------|-----------------|------------|
| `docs/reference/config.md` routing section update | Section 5.2, items 1-4 | Explicitly listed | Adequate |
| `docs/guides/providers.md` tiered routing section | Section 1.6 | Planned as "Tiered Model Routing" section | See GAP-D01 |
| `docs/examples/config-static.json` | Section 1.6 | Explicitly listed | Adequate |
| `docs/examples/config-tiered-basic.json` | Section 1.6 | Explicitly listed | Adequate |
| `docs/examples/config-tiered-channels.json` | Section 1.6 | Explicitly listed | Adequate |
| `docs/examples/config-tiered-full.json` | Section 1.6 | Explicitly listed | Adequate |
| Inline rustdoc on all public types and methods | Orchestrator Section 8 | Required per phase | See GAP-D02 |
| `docs/guides/tiered-routing.md` standalone guide | Orchestrator Section 8, Phase I | Listed in orchestrator | See GAP-D03 |
| `docs/reference/tools.md` permission annotations | Not found | NOT planned | See GAP-D04 |
| Migration examples (Level 0-3) | Phase I Section 1.6 | Inline JSON in guide | Adequate |

---

## 2. Gap Findings

### [GAP-D01] MEDIUM -- Tiered routing guide location ambiguity between providers.md and standalone file

**Description**: The Phase I plan (Section 1.6) adds a "Tiered Model Routing" section to `docs/guides/providers.md`. However, the orchestrator plan (Section 8, Phase I row) says "Create `docs/guides/tiered-routing.md`". These are two different deliverables at two different paths. The planning-summary.md does not mention either location.

**Impact**: The Phase I agent will not know whether to create a standalone guide, add a section to providers.md, or do both. Risk of partial documentation or duplicate effort.

**Recommended Fix**: Resolve the ambiguity in the Phase I plan. Recommended approach: create `docs/guides/tiered-routing.md` as the standalone guide (matching the orchestrator), and add a brief cross-reference in `docs/guides/providers.md` pointing to it. Update Phase I Section 1.6 to reflect this decision.

---

### [GAP-D02] LOW -- No per-phase rustdoc verification criteria

**Description**: The orchestrator Section 8 ("Documentation Strategy") states that all phases A-H should produce "Rustdoc on all new [types/methods]" and that `cargo doc --no-deps` must build without warnings after each phase. However, the individual phase plans (A, B, C) do not include `cargo doc --no-deps` in their exit criteria verification commands. Phase A's exit criteria (Section 5.1) list `cargo build`, `cargo clippy`, `cargo test` but not `cargo doc`. Phases B and C follow the same pattern.

**Impact**: Agents may ship code without doc comments and the oversight will not be caught until Phase I. Retrofitting rustdoc at Phase I is more expensive than writing it inline.

**Recommended Fix**: Add `cargo doc --no-deps -p <crate>` to the verification commands in each phase plan (Sections 5.4 or equivalent). The orchestrator already requires it in Gate I, but per-phase enforcement ensures incremental compliance.

---

### [GAP-D03] MEDIUM -- Standalone tiered-routing guide has no content outline

**Description**: The orchestrator mentions `docs/guides/tiered-routing.md` in the Phase I deliverables and in Section 8, but only says "migration examples". Phase I Section 1.6 provides a content outline but under the wrong file path (providers.md). The standalone guide has no explicit content spec.

Content that must be covered based on the design doc (Section 10):
- Migration Level 0 to 1 (add tiers, no auth)
- Migration Level 1 to 2 (add channel permissions)
- Migration Level 2 to 3 (add per-user RBAC + budgets)
- Complexity scoring explanation
- Escalation behavior
- Budget enforcement behavior
- Rate limiting behavior
- Tool permission enforcement

**Impact**: The Phase I agent lacks a clear spec for what the standalone guide must contain.

**Recommended Fix**: Move the content outline from Phase I Section 1.6 (providers.md section) to a new subsection explicitly labeled for `docs/guides/tiered-routing.md`. Enumerate the required sections matching the design doc's migration levels.

---

### [GAP-D04] HIGH -- No plan to document tool permission annotations in tools reference

**Description**: Phase G adds permission enforcement to the tool dispatch path. Tools will now have `required_permission_level` metadata (design doc Section 8.3). However, no phase plan or the Phase I documentation checklist mentions updating any tools reference documentation to show which tools require which permission level.

Users will need to know:
- Which tools are accessible at Level 0 (none by default)
- Which tools are accessible at Level 1 (read_file, write_file, etc.)
- Which tools require Level 2 (exec, spawn)
- How `tool_denylist` interacts with `tool_access`

The existing `docs/reference/config.md` documents tools in the `tools` section but has no permission annotations. There is no `docs/reference/tools.md` in the current codebase.

**Impact**: Operators configuring permissions will have no reference for which tools map to which permission levels. This is a user-facing documentation gap that will cause confusion during adoption.

**Recommended Fix**: Add a tools permission matrix to the Phase I documentation checklist. This could be either:
(a) A new section in `docs/guides/tiered-routing.md` under "Tool Permissions"
(b) A table in `docs/reference/config.md` under the routing.permissions section
Include: tool name, default required level, description of what it does.

---

### [GAP-D05] MEDIUM -- config.md routing section already partially exists but is incomplete

**Description**: The current `docs/reference/config.md` (lines 33-34) already lists `routing` in the root structure table with the description "Tiered model routing, permissions, budgets, rate limits". Lines 183-215 describe the `ModelRouter` trait and `StaticRouter`. However, there is no actual `routing` section with field-by-field documentation comparable to the `agents`, `providers`, or `channels` sections.

Phase H is tasked with updating config.md, and Phase I verifies completeness. But the existing partial content (root table entry, ModelRouter trait section) means the Phase I agent must understand what already exists and what needs to be added versus replaced.

**Impact**: Without explicit awareness of the existing partial content, the Phase H or I agent may create duplicate or inconsistent documentation.

**Recommended Fix**: Add a note to Phase H and Phase I plans acknowledging the existing config.md content at lines 33-34 and 183-215. Specify that Phase H/I should:
1. Expand the root structure table entry (already done)
2. Add a full `## routing` section after the `## tools` section
3. Update the "Pipeline Model Router" subsection (lines 183+) to reference TieredRouter alongside StaticRouter
4. Preserve backward-compatible content about StaticRouter

---

### [GAP-D06] HIGH -- Missing development notes for Phases B through I

**Description**: The Sprint 0 development notes (`00-initial-sprint/`) contain 50+ files organized by phase, stream, and wave. Each file documents decisions, implementation details, problems encountered, and solutions applied. Examples:
- `phase1/stream-1a/types-implementation.md`
- `phase2/stream-2a/channel-plugins.md`
- `phase3/stream-3b/cicd-setup.md`

The tiered-router sprint currently has only three development note files:
1. `planning-summary.md` -- Sprint-level overview (exists, adequate)
2. `phase-A/decisions.md` -- Phase A design decisions (exists, good quality)
3. `consensus-log.md` -- Cross-phase consensus items (exists, see GAP-D09)

Phase I (Section 5.3) specifies that development notes should be created for ALL phases:
- `phase-A-types.md`
- `phase-B-permissions.md`
- `phase-C-router.md`
- `phase-D-cost-tracker.md`
- `phase-E-rate-limiter.md`
- `phase-F-auth-context.md`
- `phase-G-tool-permissions.md`
- `phase-H-config-validation.md`
- `phase-I-testing.md`
- `implementation-summary.md`

However, the Phase I plan places ALL dev notes creation into Phase I itself (Section 1.6, "Development Notes" subsection). This is a documentation-last approach that contradicts the Sprint 0 pattern where notes were written during implementation.

**Impact**: If dev notes are written only at Phase I (retrospectively), they will lack the granular decisions, problems, and "gotchas" that are captured during implementation. Sprint 0's notes are high quality precisely because they were written in-the-moment.

**Recommended Fix**: Add a dev notes deliverable to each phase plan (A through H). Each phase agent should create its dev notes file upon completion, following the template in Phase I Section 5.4. Phase I then reviews and adds cross-cutting observations. The directory structure should be:
```
.planning/development_notes/01-tiered-router/
    planning-summary.md          (exists)
    consensus-log.md             (exists)
    phase-A/decisions.md         (exists)
    phase-B/decisions.md         (MISSING -- add to Phase B plan)
    phase-C/decisions.md         (MISSING -- add to Phase C plan)
    phase-D/decisions.md         (MISSING -- add to Phase D plan)
    phase-E/decisions.md         (MISSING -- add to Phase E plan)
    phase-F/decisions.md         (MISSING -- add to Phase F plan)
    phase-G/decisions.md         (MISSING -- add to Phase G plan)
    phase-H/decisions.md         (MISSING -- add to Phase H plan)
    phase-I/decisions.md         (MISSING -- add to Phase I plan)
    implementation-summary.md    (MISSING -- Phase I deliverable)
```

---

### [GAP-D07] MEDIUM -- Phase A decisions.md has unresolved open questions

**Description**: The Phase A decisions file (`phase-A/decisions.md`) contains three open questions at the bottom:
- Q1: Should `UserPermissions` implement a `Merge` trait? (Leaning: function)
- Q2: Should `RoutingConfig` validation be in `clawft-types` or `clawft-core`? (Leaning: split)
- Q3: Should `AuthContext` go in `config.rs` or a new `auth.rs`? (Leaning: auth.rs)

These are marked as "leaning" but not resolved. Q3 appears to have been resolved in the Phase A plan itself (Section 3.5, item 5: "AuthContext is in routing.rs, not a separate file"), which contradicts the Phase A decisions.md leaning toward `auth.rs`.

**Impact**: Agents implementing Phases B and C may make conflicting assumptions about type locations.

**Recommended Fix**: Update `phase-A/decisions.md` to resolve Q1, Q2, and Q3 with final decisions. For Q3, note that the Phase A SPARC plan decided on `routing.rs`, overriding the initial leaning. Mark each question as RESOLVED with the rationale.

---

### [GAP-D08] LOW -- Planning-summary.md references design doc section numbers but not by title

**Description**: The planning-summary.md (Section 3) references "design doc Section 12" for risk assessment, but does not include the section title ("Risk Register"). When the design doc is reorganized, these cross-references break silently.

**Impact**: Low -- the design doc is unlikely to change during this sprint. But for long-term maintainability, named references are more robust.

**Recommended Fix**: Include section titles alongside numbers in cross-references. Example: "design doc Section 12 (Risk Register)" instead of "design doc Section 12".

---

### [GAP-D09] MEDIUM -- All 6 consensus items are OPEN with no resolution criteria or deadlines

**Description**: The consensus log contains 6 entries (CONS-001 through CONS-006), all with status `OPEN`. None have:
- A target resolution date or phase gate by which they must be resolved
- A clear resolution criterion (what would constitute a decision)
- More than one position recorded (only the sparc-implementer's position exists)

The consensus protocol (Section 3 of the log) states that 80-94% confidence items need 1 additional review and <80% items need 2 reviews. Current state:
- CONS-001 (90%): Needs 1 review -- none recorded
- CONS-002 (80%): Needs 2 reviews -- none recorded
- CONS-003 (85%): Security-related, needs 2+ reviews -- none recorded
- CONS-004 (85%): Needs 1 review -- none recorded
- CONS-005 (92%): Needs 1 review -- none recorded
- CONS-006 (88%): Needs 1 review -- none recorded

**Impact**: Without resolution, implementing agents will make ad-hoc decisions. CONS-002 (DashMap vs RwLock) and CONS-003 (escalation security) are particularly important because they affect multiple phases (D, E and B, C respectively).

**Recommended Fix**:
1. Assign resolution deadlines tied to phase gates:
   - CONS-001, CONS-004: Must be RESOLVED before Phase A begins
   - CONS-002: Must be RESOLVED before Phase D begins
   - CONS-003: Must be RESOLVED before Phase B begins (security-critical)
   - CONS-005: Must be RESOLVED before Phase C begins
   - CONS-006: Must be RESOLVED before Phase H begins
2. The orchestrator should spawn review agents for each item before launching the dependent phase
3. Record the second/third positions in the log

---

### [GAP-D10] LOW -- Planning-summary.md test count target differs from Phase I plan

**Description**: The planning-summary.md (Section 6, "Test Count Targets") projects approximately 100 new tests. Phase I (`I-testing-documentation.md`, Section 5.1) specifies 133+ new tests (101 unit + 32 integration). The orchestrator does not specify a test count.

**Impact**: Minor discrepancy. The Phase I plan is more detailed and should be authoritative.

**Recommended Fix**: Update the planning-summary.md test count target to reference Phase I as the authoritative source: "See Phase I plan for detailed test matrix (133+ new tests projected)."

---

### [GAP-D11] MEDIUM -- Phase A plan creates `routing.rs` but design doc Section 7.2 and orchestrator specify extending `config.rs`

**Description**: There is a three-way inconsistency in where routing types should live:

1. **Design doc** (Section 7.2): Says "These types are added to `clawft-types/src/config.rs`"
2. **Orchestrator** (Section 4, Phase A): Says `Crate: clawft-types`, deliverables include "Integration into existing Config struct as `pub routing: RoutingConfig`"
3. **Phase A SPARC plan** (Section 1.1): Says types go into `src/routing.rs` (a NEW file), with only the `Config` struct change in `config.rs`
4. **Phase A decisions.md** (Decision 1): Selected "Extend config.rs" but notes 450-line estimate
5. **Phase A SPARC plan** (Section 3.5, item 3): Says "Separate `routing.rs` file -- Following the `delegation.rs` pattern"

The Phase A decisions.md says "Extend config.rs" (Option A) at 90% confidence. But the Phase A SPARC plan implements Option B (separate routing.rs). This is a direct contradiction within the Phase A documentation.

**Impact**: The implementing agent will encounter conflicting instructions. The decisions.md says config.rs; the SPARC plan says routing.rs. CONS-001 in the consensus log tracks this exact question but is OPEN.

**Recommended Fix**: Resolve CONS-001 before execution. The Phase A SPARC plan's routing.rs approach is the more recent and detailed specification, so update decisions.md Decision 1 to reflect this final choice. Mark CONS-001 as RESOLVED.

---

### [GAP-D12] LOW -- Phase B plan duplicates UserPermissions and AuthContext struct definitions from Phase A

**Description**: Phase B (`B-permissions-resolution.md`) contains full struct definitions for `UserPermissions` (Section 2.1) and `AuthContext` (Section 2.2) that overlap with Phase A's definitions in `A-routing-config-types.md` (Section 2.1). The Phase A plan creates these types in `routing.rs`. Phase B then references them as if they need to be created.

Phase B also defines `PermissionOverrides` (Section 2.4), which maps closely to `PermissionLevelConfig` from Phase A. The relationship is unclear -- are these the same type with different names, or two different types?

**Impact**: The Phase B agent may attempt to redefine types that Phase A already created, causing compilation errors or confusion about which definition is authoritative.

**Recommended Fix**: Phase B's specification sections (2.1, 2.2) should be labeled as "Reference: defined in Phase A" rather than appearing as new definitions. Clarify that `PermissionOverrides` in Phase B is either:
(a) An alias for `PermissionLevelConfig` from Phase A, or
(b) A distinct type that wraps or extends `PermissionLevelConfig`
Update Phase B Section 2.4 accordingly.

---

### [GAP-D13] LOW -- No plan to update `weft status` command documentation

**Description**: Success criterion 14 from the design doc (Section 13) states: "`weft status` reports the active routing mode, tier count, and permission summary." Phase I maps this to "Integration test or manual verification." However, there is no documentation update planned for the CLI reference to reflect the new `weft status` output fields.

The existing `docs/reference/config.md` does not document CLI commands. CLI documentation may live elsewhere (e.g., `clawft-cli` crate docs or a separate CLI reference).

**Impact**: Low -- CLI output changes are self-documenting via `--help`. But the status output additions should be noted somewhere for operators.

**Recommended Fix**: Add a line item to Phase I's documentation checklist: "Update CLI status command documentation (if a CLI reference doc exists) to reflect routing mode, tier count, and permission summary in `weft status` output."

---

## 3. Consistency Analysis: Plans vs docs/reference/config.md

### 3.1 Existing config.md Content

The current `docs/reference/config.md` includes:
- Root structure table listing `routing` with correct description (line 33)
- StaticRouter documentation (lines 191-202)
- ModelRouter trait signature (lines 209-214)
- No `## routing` section with field-by-field documentation

### 3.2 Phase A Routing Config Schema vs config.md

**Phase A defines these types** (from `A-routing-config-types.md` Section 2.1):

| Type | Fields | Documented in config.md? |
|------|--------|--------------------------|
| `RoutingConfig` | mode, tiers, selection_strategy, fallback_model, permissions, escalation, cost_budgets, rate_limiting | Root table entry only; no field details |
| `ModelTierConfig` | name, models, complexity_range, cost_per_1k_tokens, max_context_tokens | Not documented |
| `PermissionsConfig` | zero_trust, user, admin, users, channels | Not documented |
| `PermissionLevelConfig` | 16 Option fields | Not documented |
| `UserPermissions` | 16 concrete fields | Not documented |
| `AuthContext` | sender_id, channel, permissions | Not documented |
| `EscalationConfig` | enabled, threshold, max_escalation_tiers | Not documented |
| `CostBudgetConfig` | global_daily_limit_usd, global_monthly_limit_usd, tracking_persistence, reset_hour_utc | Not documented |
| `RateLimitConfig` | window_seconds, strategy | Not documented |

### 3.3 Serde Alias Consistency

Phase A specifies camelCase aliases for these fields:
- `selectionStrategy` / `selection_strategy`
- `fallbackModel` / `fallback_model`
- `costBudgets` / `cost_budgets`
- `rateLimiting` / `rate_limiting`
- `complexityRange` / `complexity_range`
- `costPer1kTokens` / `cost_per_1k_tokens`
- `maxContextTokens` / `max_context_tokens`
- And 15+ more in PermissionLevelConfig/UserPermissions

The existing config.md documents that "All fields support both `snake_case` and `camelCase` names" (line 4). This is consistent with the Phase A approach.

### 3.4 Default Values Consistency

Phase A defaults:
- `routing.mode`: "static" -- consistent with config.md's StaticRouter description
- `escalation.enabled`: **false** -- differs from design doc Section 7.1 JSON which has `"enabled": true`
- `escalation.threshold`: 0.6 -- matches design doc
- `cost_budgets.tracking_persistence`: **false** -- design doc Section 7.1 JSON has `true`

The Phase A SPARC plan's `EscalationConfig::default()` sets `enabled: false`, which is correct (backward compat: existing configs should not have escalation enabled by default). The design doc Section 7.1 JSON is a full example, not a defaults specification. This is consistent but should be explicitly noted.

### [GAP-D14] LOW -- Design doc example JSON shows escalation.enabled=true but default is false

**Description**: The design doc Section 7.1 full config example has `"enabled": true` for escalation. The Phase A SPARC plan correctly sets the default to `false` for backward compatibility. However, this discrepancy could confuse the Phase H agent when creating test fixtures.

**Recommended Fix**: Add a comment in the Phase A SPARC plan test fixture (Section 2.4) noting that the fixture uses `"enabled": true` as a demonstration, while the `Default` impl uses `false`.

---

## 4. Missing Development Notes: Phase-by-Phase Assessment

### Comparison with Sprint 0 Format

Sprint 0 development notes follow this structure:
```
.planning/development_notes/00-initial-sprint/
    planning-summary.md              (sprint overview)
    codebase-map.md                  (codebase inventory)
    phase1/
        stream-1a/
            types-implementation.md  (per-stream implementation notes)
            platform-implementation.md
            channels-implementation.md
            initialization.md
        wave2/
            core-infrastructure.md   (per-wave implementation notes)
            ...
        wave3/
            ...
    phase2/
        stream-2a/
            channel-plugins.md
        ...
    phase3/
        phase3-status.md             (phase status tracker)
        round3-summary.md            (round summaries)
        ...
```

Each Sprint 0 note includes:
- Date and author
- Files changed
- Tests added
- Design decisions with context/options/chosen/consequences
- Implementation notes (key patterns, gotchas)
- Test coverage table
- Open questions for later phases

### Tiered Router Sprint: Current State vs Required

| Dev Note | Sprint 0 Equivalent | Exists? | Quality |
|----------|---------------------|---------|---------|
| `planning-summary.md` | `planning-summary.md` | YES | Good -- covers all phases, risks, decisions, stats |
| `consensus-log.md` | No equivalent (new practice) | YES | Incomplete -- all OPEN, see GAP-D09 |
| `phase-A/decisions.md` | `phase1/stream-1a/types-implementation.md` | YES | Good structure but has unresolved Qs (GAP-D07) |
| `phase-B/decisions.md` | `phase2/stream-2a/channel-plugins.md` | NO | MISSING |
| `phase-C/decisions.md` | `phase1/wave2/pipeline-system.md` | NO | MISSING |
| `phase-D/decisions.md` | No close equivalent | NO | MISSING |
| `phase-E/decisions.md` | No close equivalent | NO | MISSING |
| `phase-F/decisions.md` | `phase1/wave3/integration-wiring.md` | NO | MISSING |
| `phase-G/decisions.md` | `phase3/stream-2i/security-fixes.md` | NO | MISSING |
| `phase-H/decisions.md` | `phase3/stream-3f-agents/types-parser.md` | NO | MISSING |
| `phase-I/decisions.md` | `phase3/exit-criteria-review.md` | NO | MISSING |
| `implementation-summary.md` | `phase3/phase3-status.md` | NO | MISSING |

### Missing Dev Note Content that Should Be Captured

**Phase B** (Security-critical): Permission resolution algorithm decisions, merge semantics rationale, `PermissionOverrides` vs `PermissionLevelConfig` relationship, empty-vec-means-no-change decision.

**Phase C** (Architecture-critical): `CostTrackable`/`RateLimitable` trait design decisions, `ModelTier` runtime vs `ModelTierConfig` config separation, `TierSelection` internal type rationale, `model_matches_pattern` simplicity vs full glob support.

**Phase D**: `DashMap` vs `RwLock` final decision (blocked on CONS-002), persistence format (JSON), LRU eviction strategy, reset timing edge cases.

**Phase E**: Sliding window implementation approach, `VecDeque<Instant>` vs token bucket, memory bounds calculation.

**Phase F**: ChatRequest extension approach, InboundMessage metadata threading, channel-specific sender_id extraction patterns.

**Phase G**: Denylist-overrides-allowlist rationale, glob pattern support scope, `ToolError::PermissionDenied` error format.

**Phase H**: Validation split (types vs core), deep merge edge cases, camelCase alias verification approach.

---

## 5. Consensus Log Quality Assessment

### Structure Quality: GOOD
- Clear protocol definition with confidence thresholds
- Entry template with all required fields
- Status tracking (OPEN/RESOLVED/ESCALATED/DEFERRED)

### Content Quality: INCOMPLETE

| Criterion | Assessment |
|-----------|------------|
| All items have clear questions | YES -- each CONS entry has a well-formed question |
| Context provided for each item | YES -- adequate context in each entry |
| Multiple positions recorded | NO -- only sparc-implementer's position exists for all 6 items |
| Resolution criteria defined | NO -- no item specifies what constitutes resolution |
| Deadlines or gate dependencies | NO -- no item links to a phase gate deadline |
| Actionable decisions | NO -- all items are OPEN with no path to resolution |
| Security items prioritized | PARTIAL -- CONS-003 is identified as security-related but has no additional urgency markers |

### Specific Issues

**CONS-001** (Type location): Already resolved by the Phase A SPARC plan (routing.rs), but the consensus log still shows OPEN. Should be marked RESOLVED.

**CONS-002** (DashMap vs RwLock): The most impactful open item. Affects Phases D and E. The Phase A SPARC plan does not depend on this, but the planning-summary.md (Section 8, "Dependencies") lists `dashmap` as a potential workspace dependency "pending consensus item 3". This means the implementing agent for Phase D will be blocked.

**CONS-003** (Escalation security): Security-critical. The protocol requires 2+ reviews for security items. Zero reviews have been conducted. The design doc explicitly addresses this concern (Section 5.2, escalation is bounded), but no security reviewer has validated the design.

**CONS-004** (AuthContext location): Already resolved by the Phase A SPARC plan (routing.rs, decision 5). Should be marked RESOLVED.

**CONS-005** (RoutingDecision extension): Already resolved by the Phase C SPARC plan (flat Option fields, Section 3.5). Should be marked RESOLVED.

**CONS-006** (Config validation boundary): Already partially resolved by Phase A SPARC plan (Section 4.2: parse-time vs runtime validation split). The Phase H plan will finalize. Should be marked as partially resolved or deferred to Phase H.

---

## 6. Completeness Checklist

### Documentation Deliverables

- [ ] `docs/reference/config.md` -- Full `## routing` section with all sub-fields (Phase H/I)
- [ ] `docs/reference/config.md` -- Permission levels table (Phase I)
- [ ] `docs/reference/config.md` -- Migration path documented, Level 0-3 (Phase I)
- [ ] `docs/reference/config.md` -- Complete example JSON updated (Phase I)
- [ ] `docs/reference/config.md` -- TieredRouter added to Pipeline Model Router subsection (Phase I)
- [ ] `docs/guides/tiered-routing.md` -- Standalone guide created (Phase I) [GAP-D01/D03]
- [ ] `docs/guides/tiered-routing.md` -- Migration examples for all 4 levels (Phase I)
- [ ] `docs/guides/tiered-routing.md` -- Tool permission matrix (Phase I) [GAP-D04]
- [ ] `docs/guides/providers.md` -- Cross-reference to tiered-routing guide (Phase I)
- [ ] `docs/examples/config-static.json` created (Phase I)
- [ ] `docs/examples/config-tiered-basic.json` created (Phase I)
- [ ] `docs/examples/config-tiered-channels.json` created (Phase I)
- [ ] `docs/examples/config-tiered-full.json` created (Phase I)
- [ ] All example configs validated against schema (Phase I)
- [ ] Inline rustdoc on all public types -- verified per phase [GAP-D02]
- [ ] `cargo doc --no-deps` builds without warnings (Phase I)
- [ ] CLI status command documentation updated [GAP-D13]

### Development Notes

- [x] `planning-summary.md` -- Sprint overview
- [x] `consensus-log.md` -- Cross-phase consensus tracking
- [x] `phase-A/decisions.md` -- Phase A design decisions
- [ ] `phase-A/decisions.md` -- Open questions resolved [GAP-D07]
- [ ] `phase-B/decisions.md` -- Permission resolution decisions [GAP-D06]
- [ ] `phase-C/decisions.md` -- Router algorithm decisions [GAP-D06]
- [ ] `phase-D/decisions.md` -- Cost tracker decisions [GAP-D06]
- [ ] `phase-E/decisions.md` -- Rate limiter decisions [GAP-D06]
- [ ] `phase-F/decisions.md` -- Auth threading decisions [GAP-D06]
- [ ] `phase-G/decisions.md` -- Tool permission decisions [GAP-D06]
- [ ] `phase-H/decisions.md` -- Config validation decisions [GAP-D06]
- [ ] `phase-I/decisions.md` -- Testing strategy decisions [GAP-D06]
- [ ] `implementation-summary.md` -- Final sprint summary [GAP-D06]

### Consensus Log

- [ ] CONS-001 resolved (already decided: routing.rs) [GAP-D09]
- [ ] CONS-002 resolved with review (DashMap vs RwLock) [GAP-D09]
- [ ] CONS-003 resolved with 2+ security reviews (escalation model) [GAP-D09]
- [ ] CONS-004 resolved (already decided: routing.rs) [GAP-D09]
- [ ] CONS-005 resolved (already decided: flat Option fields) [GAP-D09]
- [ ] CONS-006 resolved or deferred to Phase H [GAP-D09]
- [ ] All items have deadline tied to dependent phase gates [GAP-D09]

### Plan-to-Doc Consistency

- [ ] Phase A type location: plans, decisions.md, and consensus log aligned [GAP-D11]
- [ ] Phase B type definitions: marked as references, not redefinitions [GAP-D12]
- [ ] Test count: planning-summary updated to reference Phase I [GAP-D10]
- [ ] Escalation default: discrepancy between design doc example and Default impl noted [GAP-D14]

---

## 7. Summary of Findings

| ID | Severity | Category | Description |
|----|----------|----------|-------------|
| GAP-D01 | MEDIUM | Docs | Guide location ambiguity: providers.md vs tiered-routing.md |
| GAP-D02 | LOW | Docs | No per-phase rustdoc verification in exit criteria |
| GAP-D03 | MEDIUM | Docs | Standalone tiered-routing guide has no content outline |
| GAP-D04 | HIGH | Docs | No plan to document tool permission annotations |
| GAP-D05 | MEDIUM | Docs | config.md has partial routing content; no merge instructions for agents |
| GAP-D06 | HIGH | Dev Notes | Missing decision logs for Phases B through I |
| GAP-D07 | MEDIUM | Dev Notes | Phase A decisions.md has unresolved open questions |
| GAP-D08 | LOW | Dev Notes | Planning-summary uses section numbers without titles |
| GAP-D09 | MEDIUM | Consensus | All 6 consensus items OPEN with no deadlines or additional reviews |
| GAP-D10 | LOW | Consistency | Test count discrepancy between planning-summary and Phase I |
| GAP-D11 | MEDIUM | Consistency | Type location contradicts between decisions.md and SPARC plan |
| GAP-D12 | LOW | Consistency | Phase B duplicates Phase A type definitions |
| GAP-D13 | LOW | Docs | No plan to update weft status CLI documentation |
| GAP-D14 | LOW | Consistency | Escalation enabled default mismatch between example and code |

### Severity Distribution

| Severity | Count |
|----------|-------|
| HIGH | 2 |
| MEDIUM | 6 |
| LOW | 6 |
| **Total** | **14** |

### Priority Actions Before Execution Begins

1. **Resolve CONS-001, CONS-004, CONS-005** -- These are already decided by the SPARC plans. Update the consensus log to RESOLVED status. (5 minutes)
2. **Resolve CONS-003** -- Spawn security review before Phase B. (Blocks Phase B)
3. **Resolve CONS-002** -- Spawn review before Phase D. (Blocks Phase D)
4. **Add per-phase dev notes requirement** -- Add a "Development Notes" deliverable to each phase plan's exit criteria. (30 minutes)
5. **Resolve Phase A open questions** -- Update `phase-A/decisions.md` Q1-Q3 with final decisions. (10 minutes)
6. **Clarify guide location** -- Choose tiered-routing.md as the canonical location. Update Phase I plan. (5 minutes)
7. **Add tool permission documentation** -- Add to Phase I documentation checklist. (5 minutes)

---

**End of Gap Analysis**
