# Remediation Plan: Tiered Router SPARC Plans

**Date**: 2026-02-18
**Sources**: gap-analysis-types.md, gap-analysis-coverage.md, gap-analysis-security.md, gap-analysis-docs.md
**Status**: ALL FIXES APPLIED AND VALIDATED

---

## Summary

4 gap analyses identified 60+ findings. This remediation groups them into 12 actionable fix batches, ordered by severity and phase dependency.

---

## FIX-01: Canonical Type Ownership (CRITICAL)

**Gaps**: GAP-T01, T02, T03, T04, T05, T22, C01, C02
**Affects**: Plans A, B, C, F, H, I

**Decision**: All routing types live in `clawft-types/src/routing.rs` (Phase A owns definitions). No re-definitions elsewhere.

| Type | Canonical Location | Notes |
|------|--------------------|-------|
| `RoutingConfig` | `clawft-types/src/routing.rs` | Phase A |
| `ModelTierConfig` | `clawft-types/src/routing.rs` | Phase A |
| `PermissionsConfig` | `clawft-types/src/routing.rs` | Phase A -- uses `PermissionLevelConfig` for level defaults |
| `PermissionLevelConfig` | `clawft-types/src/routing.rs` | Phase A -- all-Option fields for partial overrides |
| `UserPermissions` | `clawft-types/src/routing.rs` | Phase A defines struct, Phase B adds `impl` methods in clawft-core |
| `AuthContext` | `clawft-types/src/routing.rs` | Phase A defines struct. `Default` = zero_trust. |
| `TierSelectionStrategy` | `clawft-types/src/routing.rs` | Phase A -- proper enum, not `Option<String>` |
| `EscalationConfig` | `clawft-types/src/routing.rs` | Phase A |
| `CostBudgetConfig` | `clawft-types/src/routing.rs` | Phase A |
| `RateLimitConfig` | `clawft-types/src/routing.rs` | Phase A |
| `BudgetResult` | `clawft-core/src/pipeline/cost_tracker.rs` | Phase D |
| `CostTracker` | `clawft-core/src/pipeline/cost_tracker.rs` | Phase D |
| `RateLimiter` | `clawft-core/src/pipeline/rate_limiter.rs` | Phase E |
| `TieredRouter` | `clawft-core/src/pipeline/tiered_router.rs` | Phase C |
| `PermissionResolver` | `clawft-core/src/pipeline/permissions.rs` | Phase B |

**Changes required**:
- Phase A: Change `selection_strategy` from `Option<String>` to `Option<TierSelectionStrategy>` enum
- Phase A: `AuthContext::default()` = zero_trust (not admin). Add `AuthContext::cli_default()` constructor.
- Phase A: `UserPermissions::default()` = zero_trust with correct budgets ($0.10 daily, $2.00 monthly)
- Phase B: Remove `PermissionOverrides` struct -- use Phase A's `PermissionLevelConfig` instead
- Phase B: Named constructors: `zero_trust_defaults()`, `user_defaults()`, `admin_defaults()` (with `_defaults` suffix)
- Phase C: Import from `clawft_types::routing`, NOT `clawft_types::auth`
- Phase C: Remove `resolve_permissions()` and `merge_permissions()` from TieredRouter (use AuthContext.permissions directly)
- Phase F: Do NOT create `auth.rs`. Import from `clawft_types::routing`.
- Phase F: Do NOT re-implement merge logic. Use Phase B's `PermissionResolver`.
- Phase F: `defaults_for_level()` unknown levels -> zero_trust (not admin)
- Phase H: Validate `TierSelectionStrategy` enum via serde (automatic with typed enum)
- Phase I: Test helpers use `PermissionLevelConfig` for config, `UserPermissions` for resolved values

---

## FIX-02: Single Permission Resolution (CRITICAL)

**Gaps**: GAP-C04, T15, T16, T18
**Affects**: Plans B, C, F

**Decision**: Phase B's `PermissionResolver` is the ONLY permission resolution implementation.

**Changes required**:
- Phase B: Constructor takes `&RoutingConfig` (not 6 HashMaps). `resolve()` takes 3 params: `(sender_id, channel, allow_from_match: bool)`
- Phase C: Remove `resolve_permissions()`, `merge_permissions()`, permission_defaults/user_overrides/channel_overrides fields from TieredRouter. Route() reads `request.auth_context.as_ref().map(|a| &a.permissions).unwrap_or(&UserPermissions::default())` directly.
- Phase F: Reference Phase B's `PermissionResolver`, do NOT re-implement resolution logic. AgentLoop calls `resolver.resolve(sender_id, channel, allow_from_match)` and attaches result to AuthContext.

---

## FIX-03: CostTrackable/RateLimitable Trait Alignment (CRITICAL)

**Gaps**: GAP-T08, T09, T10, C03
**Affects**: Plans C, D, E

**Decision**: Phase C's trait interfaces must match Phase D/E concrete APIs.

**Changes required**:
- Phase C: Update `CostTrackable` trait:
  ```
  fn check_budget(&self, user_id: &str, estimated_cost: f64, daily_limit: f64, monthly_limit: f64) -> BudgetResult;
  fn record_estimated(&self, user_id: &str, estimated_cost: f64);
  fn record_actual(&self, user_id: &str, estimated_cost: f64, actual_cost: f64);
  ```
- Phase C: Update `RateLimitable` trait to match Phase E: `fn check(&self, sender_id: &str, limit: u32) -> bool;`
- Phase D: Add `impl CostTrackable for CostTracker`
- Phase E: Add `impl RateLimitable for RateLimiter`
- Phase C: Route() passes `permissions.cost_budget_daily_usd` and `permissions.cost_budget_monthly_usd` to `check_budget()`

---

## FIX-04: Workspace Config Ceiling Enforcement (CRITICAL SECURITY)

**Gaps**: GAP-S01, S05, S15, S16
**Affects**: Plans B, H

**Changes required**:
- Phase H: Add `validate_workspace_ceiling(global: &RoutingConfig, workspace: &RoutingConfig) -> Vec<ValidationError>` function. Security-sensitive fields requiring ceiling enforcement: `level`, `escalation_allowed`, `tool_access` (cannot add tools not in global), `rate_limit` (cannot increase), `cost_budget_*` (cannot increase), `max_tier` (cannot upgrade).
- Phase H: Add `max_grantable_level` field to `RoutingConfig` (default: 1). Workspace user overrides cannot set `level` above this.
- Phase B: `PermissionResolver` accepts both global and workspace configs separately. Ceiling enforcement applied after merge, before returning resolved permissions.

---

## FIX-05: AuthContext Injection Prevention (HIGH SECURITY)

**Gaps**: GAP-S03
**Affects**: Plans F, C

**Changes required**:
- Phase F: Add `#[serde(skip_deserializing)]` to `ChatRequest.auth_context` field to prevent JSON injection via gateway API.
- Phase F: Document that AuthContext is set server-side ONLY by channel plugins and AgentLoop.
- Phase C: Add comment noting auth_context is trusted (populated by framework, not user input).

---

## FIX-06: Fallback Model Permission Check (HIGH SECURITY)

**Gaps**: GAP-S20
**Affects**: Plan C

**Changes required**:
- Phase C: `fallback_chain()` must verify fallback model belongs to a tier at or below user's `max_tier`. If not, return error decision instead of unauthorized model access.
- Phase C: `rate_limited_decision()` same fix -- do not return fallback model without permission check.

---

## FIX-07: Atomic Budget Reserve (HIGH SECURITY)

**Gaps**: GAP-S06
**Affects**: Plan D

**Changes required**:
- Phase D: Replace separate `check_budget()` + `record_estimated()` with single `reserve_budget()` method that atomically checks and reserves within a `DashMap::entry()` lock. Returns `BudgetResult`. After LLM response, `reconcile_actual()` adjusts the reservation.

---

## FIX-08: Global Rate Limit (HIGH SECURITY)

**Gaps**: GAP-S09
**Affects**: Plans E, A

**Changes required**:
- Phase A: Add `global_rate_limit_rpm: u32` (default 0 = unlimited) to `RateLimitConfig`
- Phase E: Check global rate limit before per-user limit. Track global request count in a separate `AtomicU64` counter.

---

## FIX-09: ChatRequest Extension Ownership (HIGH)

**Gaps**: GAP-T14
**Affects**: Plans C, F

**Decision**: Phase C owns the `ChatRequest` extension (adding `auth_context` field).

**Changes required**:
- Phase C: Add `auth_context: Option<AuthContext>` to ChatRequest with `#[serde(default, skip_deserializing)]`
- Phase C: Update ALL existing ChatRequest construction sites with `auth_context: None`
- Phase F: Do NOT re-add the field. Reference Phase C's change.

---

## FIX-10: RoutingDecision Extension (HIGH)

**Gaps**: GAP-T13
**Affects**: Plan C

**Changes required**:
- Phase C: Add `Default` for new RoutingDecision fields (`tier: None`, `cost_estimate_usd: None`, `escalated: false`, `budget_constrained: false`)
- Phase C: Use `..Default::default()` or explicit defaults in all existing RoutingDecision construction sites (StaticRouter, tests)
- Phase C: Document the breaking change clearly in the plan

---

## FIX-11: Documentation Gaps (HIGH)

**Gaps**: GAP-D04, D06, D01, D03, D11
**Affects**: Plans I, orchestrator, dev notes

**Changes required**:
- Phase I: Add tool permission matrix to documentation checklist (tool name -> required level)
- Phase I: Clarify guide location as `docs/guides/tiered-routing.md` (standalone file)
- Orchestrator: Add per-phase dev notes requirement
- Phase A decisions.md: Resolve contradiction -- types go in `routing.rs` (matching SPARC plan, not config.rs)
- Consensus log: Mark CONS-001 (routing.rs), CONS-004 (clawft-types), CONS-005 (extend RoutingDecision) as RESOLVED

---

## FIX-12: Minor Fixes (MEDIUM/LOW)

**Gaps**: GAP-T06, T07, T11, T12, T17, T19, T20, T21, S02, S04, S07, S08, S12, S17, S19
**Affects**: Various plans

**Changes required**:
- Phase B/F: Unknown level in `defaults_for_level()` always returns zero_trust
- Orchestrator: Fix "15 dimensions" to "16 dimensions"
- Phase C: Access `escalation_config.max_escalation_tiers` directly (not `unwrap_or()`)
- Phase C: Access `tier.max_context_tokens` directly (not `unwrap_or()`)
- Phase C: Use Phase D's `estimate_cost()` function in `apply_budget_constraints()` instead of raw `cost_per_1k_tokens`
- Phase C: Add fallback model permission check in `fallback_chain()` and `rate_limited_decision()`
- Phase D: Set file permissions to 0600 on persistence file
- Phase G: Inventory all `execute()` call sites for signature change
- Phase G/I: Add integration test for admin + CommandPolicy enforcement
- Phase H: Add validation warning for glob-like patterns in `tool_access`
- Phase F/H: Add CLI admin safety warning when gateway is network-exposed
- Phase C/G: Add `tracing::info!` structured audit log events at key decision points

---

## Application Order

1. FIX-01 (types) + FIX-02 (resolution) + FIX-03 (traits) -- foundational, must be first
2. FIX-04 (security ceiling) + FIX-05 (injection) + FIX-06 (fallback) + FIX-07 (budget) + FIX-08 (global rate) -- security hardening
3. FIX-09 (ownership) + FIX-10 (RoutingDecision) -- interface cleanup
4. FIX-11 (docs) + FIX-12 (minor) -- polish

---

## Follow-up Fixes (Applied 2026-02-18)

After initial 12-batch remediation, 4 additional gaps were identified and resolved:

| ID | Gap | Severity | Resolution |
|----|-----|----------|------------|
| GAP-T14 | Permission resolution priority ordering contradicted design doc Section 3.2 | CRITICAL | Updated Phase B (7 edits), Phase F (3 edits). Per-channel (priority 5) > per-user (priority 4). Added CONS-007 to consensus log. |
| GAP-C08 | `weft status` routing info missing from Phase I | HIGH | Added Section 3.4 to Phase I with implementation guidance, file ownership, and 2 smoke tests. |
| GAP-T34 | `sender_id` vs `user_id` naming inconsistency at API boundaries | MEDIUM | Standardized Phase D (14 changes) and Phase C (3 locations). `sender_id` at API boundary, `user_id` as internal DashMap key alias. |
| GAP-T26 | ToolError::PermissionDenied migration inventory missing | MEDIUM | Added 38-site inventory across 6 files to Phase G Section 4.6.1 with 13-item migration checklist. |

---

## Validation Results (2026-02-18)

### Security Validation -- PASSED
All security gaps verified as addressed:
- S01 (workspace ceiling): FIX-04 applied to Phase H
- S03 (AuthContext injection): FIX-05 applied to Phases F, C
- S05/S15/S16 (escalation): FIX-04 max_grantable_level in Phase H
- S06 (TOCTOU budget): FIX-07 atomic reserve_budget() in Phase D
- S09 (global rate limit): FIX-08 AtomicU64 counter in Phase E
- S19 (audit logging): FIX-12 tracing::info! events in Phases C, G
- S20 (fallback model): FIX-06 tier check in Phase C

### Consistency Validation -- PASSED
Manual targeted checks confirmed:
- No `clawft_types::auth` imports in normative plan sections
- Unknown permission levels resolve to zero_trust (Phase B)
- RoutingDecision Default implemented (Phase C)
- Global rate limit with AtomicU64 counter (Phase E)
- Per-channel > per-user priority ordering (Phases B, F, consensus log)
