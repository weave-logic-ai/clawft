# 01-tiered-router -- Sprint Planning Summary

**Generated**: 2026-02-18
**Source documents**: 08-tiered-router.md, 00-initial-sprint/planning-summary.md
**Sprint**: Tiered Router & Permission System
**Estimated effort**: 8.5 developer-days

---

## 1. Sprint Overview

### What Is Being Built

The `StaticRouter` (Level 0) always returns the same provider/model pair from
`config.json` regardless of task complexity, user identity, or cost. This sprint
replaces it with a **`TieredRouter`** (Level 1) that combines three inputs to
select the best model:

1. **Task complexity** (from `TaskClassifier`) -- what the request needs
2. **User permissions** (from channel auth context) -- what the user is allowed
3. **Cost constraints** (from budget tracking) -- what the budget allows

The router maps complexity ranges to named model tiers (`free`, `standard`,
`premium`, `elite`), enforces permission boundaries, applies per-user rate
limits and cost budgets, and supports fallback chains when preferred models are
unavailable or budget-exhausted.

### Why

- A "what time is it?" request should not route to `anthropic/claude-opus-4-5`
- Anonymous/untrusted users should be restricted to free-tier models
- Per-user daily cost budgets prevent runaway spend on shared deployments
- Channel operators need model access control without code changes
- The existing `allow_from` fields provide binary access control; this adds
  graduated capability levels

### Design Principles

- **Additive, not breaking**: `StaticRouter` remains the default. `TieredRouter`
  is opt-in via `"routing": { "mode": "tiered" }`.
- **Config-driven**: All tiers, permissions, and budgets live in `config.json`.
- **Extensible**: Custom permission dimensions via `HashMap<String, Value>`.
- **Backward compatible**: Existing configs without `routing` work unchanged.

---

## 2. Phase Breakdown

### Phase A: RoutingConfig Types (0.5 days)

**Scope**: Add `RoutingConfig`, `ModelTierConfig`, `PermissionsConfig`,
`EscalationConfig`, `CostBudgetConfig`, `RateLimitConfig` structs to
`clawft-types/src/config.rs`. Wire `RoutingConfig` into root `Config` struct
with `#[serde(default)]`.

**Dependencies**: None
**Deliverables**: All new config types with serde, Default, Clone. Unit tests
for serialization roundtrips and default values.
**Crate**: `clawft-types`

### Phase B: UserPermissions + Resolution Logic (1 day)

**Scope**: Define `UserPermissions` and `AuthContext` structs. Implement
permission resolution: built-in defaults -> global config -> channel overrides
-> user overrides. Three built-in levels: `zero_trust` (0), `user` (1),
`admin` (2).

**Dependencies**: Phase A (RoutingConfig types must exist)
**Deliverables**: `UserPermissions`, `AuthContext` types. `PermissionResolver`
with layered merge logic. Unit tests for resolution precedence.
**Crate**: `clawft-types` (types), `clawft-core` (resolver)

### Phase C: TieredRouter Core (1.5 days)

**Scope**: Implement `TieredRouter` struct implementing `ModelRouter` trait.
Tier filtering by permission `max_tier`. Tier selection by complexity score.
Escalation logic. Model selection strategies (`PreferenceOrder`, `RoundRobin`,
`LowestCost`, `Random`). Fallback chain. Extended `RoutingDecision` with tier
metadata.

**Dependencies**: Phases A, B
**Deliverables**: `TieredRouter` struct, `TierSelectionStrategy` enum,
`RoutingDecision` extension. Unit tests for each selection path.
**Crate**: `clawft-core` (`src/pipeline/tiered_router.rs`)

### Phase D: CostTracker + Budget Enforcement (1 day)

**Scope**: Implement `CostTracker` with per-user daily and monthly spend
tracking. In-memory `DashMap` with periodic disk persistence. Budget check
integration in `TieredRouter::route()`. Cost recording in
`TieredRouter::update()`. Integration with `clawft-llm::UsageTracker`.

**Dependencies**: Phase C
**Deliverables**: `CostTracker` struct, budget enforcement in router,
persistence to `~/.clawft/cost_tracking.json`. Unit tests for budget scenarios.
**Crate**: `clawft-core` (`src/pipeline/cost_tracker.rs`)

### Phase E: RateLimiter (0.5 days)

**Scope**: Sliding-window rate limiter keyed by `sender_id`. Configurable
window size and per-user limits. LRU eviction to bound memory (max 10K tracked
users). Returns rate-limited routing decision when exceeded.

**Dependencies**: Phase B (needs UserPermissions for rate_limit field)
**Deliverables**: `RateLimiter` struct, integration with TieredRouter. Unit
tests for window behavior, eviction, and edge cases.
**Crate**: `clawft-core` (`src/pipeline/rate_limiter.rs`)

### Phase F: AuthContext Threading (1 day)

**Scope**: Thread `AuthContext` from channel plugins through `InboundMessage`
to `AgentLoop` to `PipelineRegistry::complete()` to `TieredRouter::route()`.
Add optional `auth_context` field to `ChatRequest`. Channel plugins extract
`sender_id` and resolve initial permission level from `allow_from`.

**Dependencies**: Phases B, C
**Deliverables**: Modified `InboundMessage`, `ChatRequest`, `AgentLoop`. CLI
defaults to admin. Tests for auth flow from each channel type.
**Crate**: `clawft-types` (message types), `clawft-core` (agent loop),
`clawft-channels` (auth extraction)

### Phase G: Tool Permission Enforcement (0.5 days)

**Scope**: Add permission checking to `ToolRegistry::execute()`. Check
`tool_access` allowlist and `tool_denylist`. Return `ToolError::PermissionDenied`
when denied. Zero-trust users get no tool access by default.

**Dependencies**: Phase B (UserPermissions)
**Deliverables**: Modified `ToolRegistry`, new `PermissionDenied` error variant.
Tests for allow/deny/wildcard scenarios.
**Crate**: `clawft-core` (`src/tools/mod.rs`, `src/tools/registry.rs`)

### Phase H: Config Parsing + Validation + Defaults (1 day)

**Scope**: Full config parsing with serde aliases (`camelCase` + `snake_case`).
Validation: tier names unique, complexity ranges valid (0.0-1.0), permission
levels in range, budget values non-negative. Sensible defaults for all fields.
Deep merge between global and project-level routing configs.

**Dependencies**: Phases A-G (all types must be stable)
**Deliverables**: Validation logic, error messages, default configs. Config
fixture files. Tests for parsing, validation, and merge behavior.
**Crate**: `clawft-types` (validation), `clawft-platform` (config loader)

### Phase I: Tests (1.5 days)

**Scope**: Comprehensive test suite covering all phases. Unit tests per module.
Integration tests for full routing flow (message -> auth -> classify -> route
-> model selection). Config fixture tests with example JSON files. Edge case
tests for budget exhaustion, rate limiting, fallback chains, escalation.

**Dependencies**: Phases A-H
**Deliverables**: Test files in `tests/`, fixture files in `tests/fixtures/`.
Target: 80+ new tests. All existing 1,058 tests continue passing.
**Crate**: All modified crates

---

## 3. Risk Assessment

From the design doc Section 12:

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Complexity score too coarse for good tier selection | Medium | Medium | 4 | Level 1+ classifiers improve accuracy; overlapping tier ranges absorb imprecision |
| Budget tracking lost on crash (in-memory) | Medium | Low | 3 | Periodic disk persistence; conservative pre-flight budget checks |
| Rate limiter memory grows unbounded | Low | Medium | 2 | LRU eviction; max 10K tracked users |
| Permission config too complex for users | Medium | Medium | 4 | Sensible defaults; basic use needs only `routing.mode` + `routing.tiers` |
| Escalation causes unexpected cost spikes | Low | Medium | 2 | `max_escalation_tiers: 1` caps escalation to one tier above max |
| Model availability changes break configs | Low | Low | 1 | Fallback chain + `fallback_model` config |
| Config merge conflicts (global vs project) | Low | Medium | 2 | Same deep-merge rules as existing config system |

### Additional Sprint Risks

| Risk | Mitigation |
|------|------------|
| `DashMap` dependency adds binary size | Evaluate `std::sync::RwLock<HashMap>` first; DashMap only if contention is measurable |
| Breaking `ModelRouter` trait signature | Carry `AuthContext` on `ChatRequest`, not as a new trait parameter |
| Circular dependency between core and types | All new types go in `clawft-types`; logic stays in `clawft-core` |

---

## 4. Development Approach

### Test-Driven Development

Each phase follows Red-Green-Refactor:
1. Write failing tests that define the expected behavior
2. Implement minimal code to pass
3. Refactor for clarity and performance
4. Verify all existing tests still pass

### Parallel Execution

Phases with independent dependencies can overlap:
- **Phase A** (types) must complete first
- **Phases B, E** can start after A (both depend on types only)
- **Phases C, G** can start after B
- **Phase D** starts after C
- **Phase F** starts after B + C
- **Phase H** starts after all features are stable
- **Phase I** runs continuously (tests written alongside each phase)

Critical path: A -> B -> C -> D -> H -> I (5.5 days)
With parallelism: estimated 6-7 days wall clock

### Review Cycles

- Types (Phase A) reviewed before implementation begins (consensus item)
- Router algorithm (Phase C) reviewed for correctness
- Permission resolution (Phase B) reviewed for security
- Config format (Phase H) reviewed for usability

---

## 5. Key Decisions Made During Planning

| Decision | Rationale |
|----------|-----------|
| Default routing mode is `"static"` | Backward compatibility: existing configs work unchanged |
| `UserPermissions` uses `level` as primary with dimension overrides | Levels provide sensible defaults; dimensions allow fine-grained control |
| Private-by-default workspace permissions | New workspaces inherit global permissions; project config can only restrict, not escalate |
| CLI channel defaults to admin (Level 2) | Local user owns the machine; trusted by default |
| `AuthContext` on `ChatRequest`, not as trait parameter | Avoids breaking the `ModelRouter` trait signature |
| `TieredRouter` in `clawft-core`, not `clawft-llm` | Separation of concerns: routing intelligence is core logic, not transport |
| Types in `clawft-types`, logic in `clawft-core` | Prevents circular dependencies; types crate stays zero-dep |
| serde aliases match existing convention | `snake_case` primary with `camelCase` aliases (e.g., `cost_per_1k_tokens` / `costPer1kTokens`) |

---

## 6. Sprint Statistics

### New Types

| Type | Crate | Location |
|------|-------|----------|
| `RoutingConfig` | clawft-types | config.rs |
| `ModelTierConfig` | clawft-types | config.rs |
| `PermissionsConfig` | clawft-types | config.rs |
| `UserPermissions` | clawft-types | config.rs |
| `AuthContext` | clawft-types | config.rs or auth.rs |
| `EscalationConfig` | clawft-types | config.rs |
| `CostBudgetConfig` | clawft-types | config.rs |
| `RateLimitConfig` | clawft-types | config.rs |
| `TierSelectionStrategy` | clawft-types | config.rs |
| `TieredRouter` | clawft-core | pipeline/tiered_router.rs |
| `CostTracker` | clawft-core | pipeline/cost_tracker.rs |
| `RateLimiter` | clawft-core | pipeline/rate_limiter.rs |
| `PermissionResolver` | clawft-core | pipeline/permissions.rs |

**Total new types**: ~13 structs/enums

### New Files

| File | Purpose |
|------|---------|
| `crates/clawft-core/src/pipeline/tiered_router.rs` | TieredRouter implementation |
| `crates/clawft-core/src/pipeline/cost_tracker.rs` | Budget tracking and enforcement |
| `crates/clawft-core/src/pipeline/rate_limiter.rs` | Sliding-window rate limiter |
| `crates/clawft-core/src/pipeline/permissions.rs` | Permission resolution logic |
| `tests/fixtures/config_tiered.json` | Config fixture for integration tests |

**Total new files**: ~5 (types added to existing `config.rs`)

### Estimated LOC

| Component | Lines |
|-----------|-------|
| Config types (clawft-types) | ~200 |
| UserPermissions + AuthContext | ~100 |
| TieredRouter | ~350 |
| CostTracker | ~200 |
| RateLimiter | ~150 |
| PermissionResolver | ~150 |
| Tool permission checks | ~50 |
| Auth context threading | ~80 |
| Config validation | ~100 |
| Tests | ~600 |
| **Total** | **~1,980** |

### Test Count Targets

| Category | Count |
|----------|-------|
| Config serde roundtrips | ~15 |
| UserPermissions defaults and overrides | ~10 |
| Permission resolution precedence | ~10 |
| TieredRouter tier selection | ~15 |
| Escalation logic | ~5 |
| CostTracker budget enforcement | ~10 |
| RateLimiter window behavior | ~8 |
| Tool permission enforcement | ~8 |
| Auth context flow (integration) | ~5 |
| Config validation | ~8 |
| Full routing flow (integration) | ~6 |
| **Total new tests** | **~100** |
| **Cumulative (with existing 1,058)** | **~1,158** |

---

## 7. Consensus Items

Areas where multiple agents should review before implementation proceeds:

### 1. RoutingConfig Type Shape (Phase A)

Whether to extend `config.rs` vs create a new `routing.rs` module in
`clawft-types`. Current decision: extend `config.rs` since `RoutingConfig`
is a top-level config peer to `AgentsConfig`, `ChannelsConfig`, etc.

**Confidence**: 90% -- warrants review

### 2. ModelRouter Trait Backward Compatibility (Phase C)

The `route()` method signature does not change; `AuthContext` is carried on
`ChatRequest`. This avoids breaking all existing `ModelRouter` implementations
but requires `ChatRequest` to gain an optional field.

**Confidence**: 95% -- proceed unless objection raised

### 3. DashMap vs RwLock<HashMap> for CostTracker (Phase D)

`DashMap` provides better concurrent performance but adds a dependency.
`RwLock<HashMap>` is stdlib-only. For a single-user deployment the difference
is negligible.

**Confidence**: 80% -- needs measurement or team preference

### 4. Permission Escalation Security Model (Phase B)

Escalation allows a Level 1 user to temporarily access a higher tier model.
This is bounded by `max_escalation_tiers: 1` and only triggers above the
`escalation_threshold`. Still, it means a `user` can hit `premium` models.

**Confidence**: 85% -- security review recommended

### 5. Tool Denylist Precedence (Phase G)

Current design: denylist wins over allowlist. `tool_access: ["*"]` +
`tool_denylist: ["exec"]` means all tools except exec. This matches
standard RBAC patterns but should be documented clearly.

**Confidence**: 95% -- standard pattern

---

## 8. Dependencies and Workspace Changes

### New Workspace Dependencies (Potential)

| Dependency | Used By | Reason |
|------------|---------|--------|
| `dashmap` | CostTracker, RateLimiter | Concurrent HashMap (pending consensus item 3) |

### Modified Crates

| Crate | Changes |
|-------|---------|
| `clawft-types` | New config types, AuthContext, UserPermissions |
| `clawft-core` | TieredRouter, CostTracker, RateLimiter, permission enforcement, auth threading |
| `clawft-channels` | Auth context extraction in Telegram/Slack/Discord plugins |
| `clawft-cli` | Status command shows routing mode summary |

### Unchanged Crates

- `clawft-llm` (no changes; transport layer is separate from routing intelligence)
- `clawft-tools` (tool implementations unchanged; only registry dispatch changes)
- `clawft-services` (no routing logic here)
- `clawft-platform` (config loader may need minor update for validation)
- `clawft-wasm` (no routing in WASM build)
