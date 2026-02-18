# Gap Analysis: Type Consistency & Interface Alignment

**Scope**: Phases A through I of the Tiered Router SPARC plans
**Source Design Doc**: `.planning/08-tiered-router.md`
**Existing Code**: `crates/clawft-core/src/pipeline/traits.rs`
**Date**: 2026-02-18

---

## 1. Type Name Consistency Gaps

### [GAP-T01] CRITICAL -- `TierSelectionStrategy`: String vs Enum inconsistency across plans

**Affected Plans**: A, C, H
**Issue**: Phase A defines `selection_strategy` as `Option<String>` on `RoutingConfig`. Phase C defines a separate `TierSelectionStrategy` enum locally in `tiered_router.rs` and parses the string at construction time via `TierSelectionStrategy::from_config_str()`. The design doc (Section 4.2) shows `TierSelectionStrategy` as an enum with `Serialize, Deserialize`. Phase H validates against known string values `["preference_order", "round_robin", "lowest_cost", "random"]`.

The inconsistency is: Phase A stores a raw `Option<String>`, but the design doc implies a proper enum. Phase C re-creates the enum in a different module. There is no single canonical enum definition in `clawft-types`.

**Recommended Fix**: Define `TierSelectionStrategy` as a proper `#[derive(Serialize, Deserialize)]` enum in `clawft-types/src/routing.rs` (Phase A). Use `#[serde(rename_all = "snake_case")]` for JSON mapping. Change `RoutingConfig.selection_strategy` from `Option<String>` to `Option<TierSelectionStrategy>`. Phase C should import it rather than redefine it. Phase H validates the enum variant at parse time via serde automatically.

---

### [GAP-T02] HIGH -- `PermissionOverrides` vs `PermissionLevelConfig`: Duplicate type for the same concept

**Affected Plans**: A, B, C, F, H
**Issue**: Phase A defines `PermissionLevelConfig` (all `Option<T>` fields) for partial permission overrides in config. Phase B defines a separate `PermissionOverrides` struct with identical fields and identical `Option<T>` semantics. These are the same type with two different names.

Phase C's `parse_permission_defaults()` expects `PermissionsConfig` to contain `Option<UserPermissions>` for each level (not `PermissionLevelConfig`). Phase F's `PermissionResolver::config_for_level()` also expects `Option<&UserPermissions>` from `PermissionsConfig`. Phase H's validation pseudocode accesses `config.permissions.zero_trust` as a `UserPermissions` (concrete type) rather than `PermissionLevelConfig` (option type).

**Recommended Fix**: Use Phase A's `PermissionLevelConfig` as the single canonical partial-override type. Remove Phase B's `PermissionOverrides`. Phase B's `PermissionResolver` should accept `&PermissionLevelConfig` in its merge method. All plans should reference `PermissionLevelConfig` when talking about config-layer overrides and `UserPermissions` when talking about resolved runtime values.

---

### [GAP-T03] HIGH -- `PermissionsConfig` field types: `PermissionLevelConfig` vs `UserPermissions` vs `Option<UserPermissions>`

**Affected Plans**: A, B, C, F, H, I
**Issue**: The type stored in `PermissionsConfig` for `zero_trust`, `user`, and `admin` fields varies across plans:

- Phase A: `PermissionLevelConfig` (correct -- partial override config type)
- Phase B: References the fields from Phase A but `from_config()` treats them as `Option<PermissionOverrides>` (which does not exist in Phase A)
- Phase C: `parse_permission_defaults()` calls `permissions.zero_trust` as `Option<UserPermissions>`, and stores results as `HashMap<String, UserPermissions>`
- Phase F: `config_for_level()` returns `Option<&UserPermissions>`, and `PermissionsConfig` is defined with `pub zero_trust: Option<UserPermissions>`
- Phase H: Validation pseudocode accesses `config.permissions.zero_trust` directly as `UserPermissions` (not `Option`, not `PermissionLevelConfig`)
- Phase I: Test helpers construct `PermissionsConfig` with `UserPermissions` values directly (no `Option`, no `PermissionLevelConfig`)

Phase A's design is the most correct: level configs are `PermissionLevelConfig` (partial overrides). But Phases C, F, and I treat them as `UserPermissions` or `Option<UserPermissions>`.

**Recommended Fix**: Standardize `PermissionsConfig` to use `PermissionLevelConfig` for `zero_trust`, `user`, and `admin` (matching Phase A). The `users` and `channels` maps should be `HashMap<String, PermissionLevelConfig>` (matching Phase A). All downstream phases must convert from `PermissionLevelConfig` to `UserPermissions` through the resolver. Phase C should not attempt its own resolution -- it should receive pre-resolved `UserPermissions` from the `AuthContext`.

---

### [GAP-T04] HIGH -- `AuthContext::default()` returns different values across plans

**Affected Plans**: A, B, F
**Issue**: Phase A defines `AuthContext` with no `Default` derive or impl, and its serde defaults produce empty `sender_id`, empty `channel`, and `UserPermissions::default()` (zero-trust). Phase B defines `AuthContext::default()` with `sender_id = "local"`, `channel = "cli"`, and `admin_defaults()`. Phase F says `AuthContext` should default to zero-trust when `None` on the request. The design doc (Section 3.3) derives `Default` on `AuthContext` which would give empty strings and `UserPermissions::default()`.

Phase B disagrees with both Phase A and Phase F: it makes the `Default` impl produce CLI admin-level permissions.

**Recommended Fix**: `AuthContext::default()` should use zero-trust (matching the principle of least privilege). The CLI-admin behavior should be handled by the `PermissionResolver` when `channel == "cli"`, not by the `Default` impl. Phase B should change its `AuthContext::default()` to match Phase A. Add a separate `AuthContext::cli_default()` constructor for the CLI case.

---

### [GAP-T05] MEDIUM -- `UserPermissions::default()` vs `UserPermissions::zero_trust_defaults()` naming

**Affected Plans**: A, B, F
**Issue**: Phase A uses `UserPermissions::default()` which returns zero-trust values. Phase B defines `UserPermissions::zero_trust_defaults()`, `user_defaults()`, `admin_defaults()` as named constructors. Phase F defines `UserPermissions::zero_trust()`, `user()`, `admin()` (shorter names, no `_defaults` suffix). The `Default` trait impl in Phase A matches zero-trust.

Three different naming conventions for the same constructors:
- Phase A: `Default::default()` (zero-trust), no named constructors
- Phase B: `zero_trust_defaults()`, `user_defaults()`, `admin_defaults()`
- Phase F: `zero_trust()`, `user()`, `admin()`

**Recommended Fix**: Standardize on Phase B's naming (`zero_trust_defaults()`, `user_defaults()`, `admin_defaults()`) because the `_defaults` suffix clarifies these are built-in defaults, not fully resolved permissions. Keep `Default::default()` mapping to `zero_trust_defaults()`. Phase F should use the Phase B names.

---

### [GAP-T06] MEDIUM -- `defaults_for_level()` handles unknown levels differently

**Affected Plans**: B, F
**Issue**: Phase B's `defaults_for_level()` maps unknown levels (`_ =>`) to `zero_trust_defaults()`. Phase F's `defaults_for_level()` maps unknown levels (`_ =>`) to `admin()`. This is a security-critical difference: one defaults to most restrictive, the other to least restrictive.

**Recommended Fix**: Unknown levels must always fall back to `zero_trust_defaults()` (most restrictive). Phase F must be corrected. This is the principle of least privilege.

---

### [GAP-T07] MEDIUM -- `BudgetResult` vs `BudgetStatus` naming

**Affected Plans**: Orchestrator, D
**Issue**: The orchestrator plan (Section 4, Phase D deliverables item 3) specifies `check_budget()` returns `BudgetStatus` with variants `Ok`, `NearLimit`, `Exceeded`. Phase D defines `BudgetResult` with variants `Ok`, `OverDailyUser`, `OverMonthlyUser`, `OverDailyGlobal`, `OverMonthlyGlobal`. The orchestrator's `NearLimit` variant does not appear in Phase D's definition.

**Recommended Fix**: Use Phase D's `BudgetResult` (more granular). Remove the `NearLimit` concept from the orchestrator description, or add a `NearLimit` variant to `BudgetResult` if soft-warning behavior is desired. Update the orchestrator to match Phase D's actual definition.

---

### [GAP-T08] MEDIUM -- `CostTrackable` trait signatures differ from `CostTracker` method signatures

**Affected Plans**: C, D
**Issue**: Phase C defines the `CostTrackable` trait with three methods:
- `check_budget(&self, sender_id: &str) -> Option<f64>`
- `record_estimated_cost(&self, sender_id: &str, cost_usd: f64)`
- `record_actual_cost(&self, sender_id: &str, cost_usd: f64)`

Phase D defines `CostTracker` public methods with different signatures:
- `check_budget(&self, user_id, estimated_cost_usd, user_daily_limit, user_monthly_limit) -> BudgetResult`
- `record_estimated(&self, user_id, estimated_cost_usd)`
- `record_actual(&self, user_id, estimated_cost_usd, actual_cost_usd)`

Phase D does mention implementing `CostTrackable` (line 30, Section 2.9) and shows a `CostTrackable for CostTracker` impl block. However, the trait methods (from Phase C) and the concrete methods (from Phase D) have incompatible signatures. The `impl CostTrackable` block would need to adapt between Phase C's simplified trait and Phase D's richer concrete API.

Additionally, the design doc's `update()` impl calls `self.cost_tracker.record(decision, outcome)`, but the `CostTrackable` trait has no `record(decision, outcome)` method.

**Recommended Fix**: Expand `CostTrackable` to match Phase D's full signatures:
- `check_budget(&self, user_id: &str, estimated_cost: f64, daily_limit: f64, monthly_limit: f64) -> BudgetResult`
- `record_estimated(&self, user_id: &str, estimated_cost_usd: f64)`
- `record_actual(&self, user_id: &str, estimated_cost_usd: f64, actual_cost_usd: f64)`

Remove the `record(decision, outcome)` call from the design doc example; the `update()` method should extract `sender_id` and cost values from the decision/outcome and call the individual methods.

---

### [GAP-T09] MEDIUM -- `RateLimitable` trait vs `RateLimiter` concrete type

**Affected Plans**: C, E
**Issue**: Phase C defines `RateLimitable` as a trait: `fn check(&self, sender_id: &str, limit: u32) -> bool`. Phase E defines a concrete `RateLimiter` struct with a `check(&self, sender_id: &str, limit: u32) -> bool` method. The signatures match, but Phase E does not mention implementing the `RateLimitable` trait.

Phase C's `TieredRouter` holds `Arc<dyn RateLimitable + Send + Sync>` and requires a trait impl.

**Recommended Fix**: Phase E must add `impl RateLimitable for RateLimiter { ... }` that delegates to the concrete `check()` method. Since the signatures already match, this is a trivial addition. Phase E's plan should explicitly note this trait impl requirement.

---

### [GAP-T10] MEDIUM -- `CostTrackable::record_estimated_cost()` vs `CostTracker::record_estimated()` method naming

**Affected Plans**: C, D
**Issue**: Phase C defines `CostTrackable::record_estimated_cost(&self, sender_id: &str, cost_usd: f64)`. Phase D defines `CostTracker::record_estimated(&self, user_id: &str, estimated_cost_usd: f64)`. Similarly, Phase C has `record_actual_cost` while Phase D has `record_actual`. The method names differ.

**Recommended Fix**: Standardize the method names. `record_estimated` and `record_actual` (Phase D) are cleaner. Update the `CostTrackable` trait in Phase C to match Phase D's naming.

---

### [GAP-T11] LOW -- `EscalationConfig.max_escalation_tiers` type inconsistency

**Affected Plans**: A, C
**Issue**: Phase A defines `max_escalation_tiers: u32` (non-optional). Phase C's tier selection code accesses `self.escalation_config.max_escalation_tiers.unwrap_or(1)`, treating it as `Option<u32>`. Phase A's definition has no `Option` wrapper.

**Recommended Fix**: Phase C should access `self.escalation_config.max_escalation_tiers` directly without `unwrap_or()`, since Phase A defines it as a concrete `u32` with a default of 1.

---

### [GAP-T12] LOW -- `ModelTierConfig.max_context_tokens` type inconsistency

**Affected Plans**: A, C
**Issue**: Phase A defines `max_context_tokens: usize` (concrete, non-optional) on `ModelTierConfig`. Phase C's `from_config()` accesses `t.max_context_tokens.unwrap_or(8192)`, treating it as `Option<usize>`.

**Recommended Fix**: Phase C should access `t.max_context_tokens` directly (not `unwrap_or`), since Phase A defines it as a concrete `usize` with `default_tier_max_context()` returning 8192.

---

## 2. Interface Alignment Gaps

### [GAP-T13] CRITICAL -- Phase C's `route()` does NOT match the existing `ModelRouter` trait signature for `RoutingDecision`

**Affected Plans**: C, all consuming phases
**Issue**: The existing `ModelRouter::route()` in `traits.rs` returns `RoutingDecision` with 3 fields: `provider`, `model`, `reason`. Phase C proposes extending `RoutingDecision` with 4 additional fields: `tier: Option<String>`, `cost_estimate_usd: Option<f64>`, `escalated: bool`, `budget_constrained: bool`.

This extension requires modifying every existing construction of `RoutingDecision` (in `StaticRouter`, in test code in `traits.rs`). Phase C acknowledges this but the existing test code in `traits.rs` constructs `RoutingDecision` with only 3 fields (lines 489-493, 689-692). All of these will fail to compile after the extension.

Additionally, the `Trajectory` struct in `traits.rs` (line 190) stores a `RoutingDecision`, so the clone on line 373 must still work. The design doc (Section 4.3) shows `RoutingDecision` with the extended fields.

**Recommended Fix**: Phase C must:
1. Add `Default` derive or a manual `Default` impl for `RoutingDecision` where new fields default to `None`/`false`.
2. Update all existing `RoutingDecision` constructions in `traits.rs` tests and `StaticRouter` to include the new fields (with defaults).
3. Consider using a builder pattern or `..Default::default()` syntax to future-proof construction sites.

---

### [GAP-T14] CRITICAL -- Permission resolution priority ordering contradicts between design doc and Phase B

**Affected Plans**: B, C, F, Design Doc
**Issue**: The design doc (Section 3.2) defines the merge priority as:

```
Priority (lowest to highest):
  1. Built-in defaults for the resolved level
  2. Global config:    routing.permissions.<level_name>
  3. Project config:   routing.permissions.<level_name> (workspace override)
  4. Per-user config:  routing.permissions.users.<user_id>
  5. Per-channel:      routing.permissions.channels.<channel_name>
```

This means per-channel overrides (priority 5) take precedence over per-user overrides (priority 4). A channel restriction would override a user-level permission.

Phase B (Section 1.3) defines the opposite ordering:

```
Priority (lowest to highest):
  1. Built-in defaults for the resolved level (hardcoded in Rust)
  2. Global config:    routing.permissions.<level_name>
  3. Workspace config: routing.permissions.<level_name>
  4. Per-channel:      routing.permissions.channels.<channel_name>
  5. Per-user:         routing.permissions.users.<sender_id>
```

Here per-user (priority 5) takes precedence over per-channel (priority 4). Phase B's Section 4.1 confirms this: "the per-channel restriction on max_tier still applies because channel overrides are applied before user overrides. If the user also has a max_tier override, it takes final precedence."

Phase F's Section 1.4 note says "user overrides are highest priority in the merge" consistent with Phase B.

The design doc's use-case examples ("All users get standard permissions, but Discord users in channel #general are limited to free tier") suggest that channel restrictions should be enforceable even for specific users, which aligns with the design doc ordering (channel > user). But the SPARC plans implement the inverse.

**Recommended Fix**: This is a design-level decision that must be resolved before implementation. The two orderings have fundamentally different security semantics:
- Design doc ordering (channel > user): Channels can enforce restrictions that override individual user privileges. More restrictive.
- Phase B ordering (user > channel): Individual user config always wins. More permissive per-user.

Recommend: Adopt Phase B's ordering (user > channel), which is more conventional. Individual user overrides should take precedence since they represent intentional per-user configuration. Update the design doc's Section 3.2 to swap priorities 4 and 5. Verify that the design doc's example use cases still work with the corrected ordering.

---

### [GAP-T15] HIGH -- Phase F's `ChatRequest` extension requires updating all existing construction sites

**Affected Plans**: C, F
**Issue**: Both Phase C and Phase F propose adding `auth_context: Option<AuthContext>` to `ChatRequest`. The existing `traits.rs` has multiple test construction sites for `ChatRequest` (lines 448-459, 567-586, 654, 810-819, 845-855) that do not include `auth_context`. After the extension, all of these must add `auth_context: None`.

Phases C and F both claim ownership of this change, creating a potential conflict.

**Recommended Fix**: Assign the `ChatRequest` extension exclusively to Phase C (it is listed in Phase C's deliverables). Phase F should NOT re-add it. Phase C must update all existing `ChatRequest` construction sites. Use `#[serde(default)]` on the field (already specified in both plans) and add `auth_context: None` to all existing tests.

---

### [GAP-T16] HIGH -- Phase F's `PermissionResolver` has different constructor and method signatures than Phase B

**Affected Plans**: B, F
**Issue**: Phase B defines `PermissionResolver::new()` with 6 separate HashMap parameters and `resolve(&self, sender_id: &str, channel: &str) -> UserPermissions` (2 params). Phase F defines `PermissionResolver::new(config: RoutingConfig)` (1 param, stores entire config) and `resolve(&self, sender_id: &str, channel: &str, allow_from_match: bool) -> UserPermissions` (3 params).

The `resolve()` signatures differ: Phase B takes 2 args, Phase F takes 3 (adds `allow_from_match: bool`). Phase F's approach is arguably better (the resolver needs to know if the sender was in allow_from), but it conflicts with Phase B's published interface.

**Recommended Fix**: Use Phase F's 3-parameter `resolve()` signature since the `allow_from_match` information is needed for level resolution. Phase B's plan should be updated to include `allow_from_match` in `resolve()`. Phase B should also use Phase F's simpler constructor that takes `RoutingConfig` directly rather than pre-extracted HashMaps.

---

### [GAP-T17] HIGH -- Phase F redefines `PermissionResolver` merge logic differently than Phase B

**Affected Plans**: B, F
**Issue**: Phase B's merge uses `PermissionOverrides` (all `Option<T>` fields) and only overrides when `Some(...)`. Phase F's merge takes `&UserPermissions` (concrete fields) and uses heuristics like "override if non-empty string" and "override if non-zero" for numeric fields.

Phase B's approach is correct: `Option<T>` cleanly distinguishes "not set" from "set to default value". Phase F's heuristic approach cannot distinguish "rate_limit intentionally set to 0 (unlimited)" from "rate_limit not configured" because both are 0.

**Recommended Fix**: Use Phase B's `PermissionLevelConfig` (all `Option<T>`) approach for the merge. The resolver should convert `PermissionLevelConfig` to `UserPermissions` via the merge chain. Phase F should NOT reimplement merge logic.

---

### [GAP-T18] MEDIUM -- Phase G's `ToolRegistry::execute()` signature change

**Affected Plans**: G
**Issue**: Phase G changes `ToolRegistry::execute()` from `(name, args)` to `(name, args, permissions: Option<&UserPermissions>)`. Every call site in the codebase must be updated. Phase G lists the agent loop change but does not inventory all call sites.

The existing `traits.rs` does not show `ToolRegistry::execute()` directly (tool dispatch is in `clawft-core/src/tools/`), but the signature change is a breaking change that requires auditing all callers.

**Recommended Fix**: Phase G should inventory all `execute()` call sites and update each one to pass `None` for backward compatibility, then `Some(&permissions)` where auth context is available. The file ownership matrix should note that Phase G modifies tool dispatch call sites beyond just `registry.rs`.

---

### [GAP-T19] MEDIUM -- Phase C duplicates permission resolution that Phase B already provides

**Affected Plans**: B, C
**Issue**: Phase C implements `TieredRouter::resolve_permissions(&self, auth: &AuthContext) -> UserPermissions` with its own merge logic (Section 2.7). This duplicates Phase B's `PermissionResolver`. The `TieredRouter` should not contain permission resolution logic -- it should consume pre-resolved `UserPermissions` from `AuthContext`.

Phase F's design (where `AgentLoop` resolves permissions and attaches them to `AuthContext`) means the router should just read `auth.permissions` directly, not re-resolve.

**Recommended Fix**: Remove `resolve_permissions()` from `TieredRouter` (Phase C). The router should read `request.auth_context.permissions` directly. All permission resolution happens in Phase B's `PermissionResolver`, invoked by Phase F's `AgentLoop`. Phase C's `TieredRouter` should not store `permission_defaults`, `user_overrides`, or `channel_overrides`.

---

### [GAP-T20] MEDIUM -- Design doc `update()` calls `self.cost_tracker.record(decision, outcome)` but no such trait method exists

**Affected Plans**: C, D, Design Doc
**Issue**: The design doc (Section 4.2) shows the `ModelRouter::update()` impl calling `self.cost_tracker.record(decision, outcome)`. However, the `CostTrackable` trait defined in Phase C has no `record(decision, outcome)` method. Phase C's own `update()` implementation (Section 2.9) extracts `cost_estimate_usd` from the decision and calls the individual record methods. Phase D's `CostTracker` has `record_estimated()` and `record_actual()` methods that take individual parameters.

The design doc's `.record(decision, outcome)` signature implies a single method that takes both the routing decision and the response outcome, but no plan defines such a method.

**Recommended Fix**: The `update()` implementation should extract the relevant fields from `RoutingDecision` and `ResponseOutcome` and call the individual `record_estimated` / `record_actual` methods on the `CostTrackable` trait. Update the design doc example to show the correct method calls. Phase C's approach (extracting fields manually) is correct.

---

## 3. Field Consistency Gaps

### [GAP-T21] HIGH -- UserPermissions field count: "15 dimensions" vs "16 dimensions"

**Affected Plans**: Orchestrator, A, B
**Issue**: The orchestrator (Section 4, Phase A deliverables item 4) says "all 15 dimension fields from design doc Section 3.1". Phase B's specification (Section 1.1) lists 16 fields. The design doc (Section 3.1) shows 16 fields (level, max_tier, model_access, model_denylist, tool_access, tool_denylist, max_context_tokens, max_output_tokens, rate_limit, streaming_allowed, escalation_allowed, escalation_threshold, model_override, cost_budget_daily_usd, cost_budget_monthly_usd, custom_permissions).

**Recommended Fix**: The correct count is 16 fields. Update the orchestrator to say "16 dimensions" instead of "15 dimensions".

---

### [GAP-T22] MEDIUM -- `UserPermissions` field values differ for zero-trust `cost_budget_daily_usd` and `cost_budget_monthly_usd`

**Affected Plans**: A, B, Design Doc
**Issue**: Phase A's `UserPermissions::default()` sets `cost_budget_daily_usd: 0.0` and `cost_budget_monthly_usd: 0.0` (both zero = unlimited). The design doc Section 2.2 specifies zero-trust should have `cost_budget_daily_usd: 0.10` and `cost_budget_monthly_usd: 2.00`. Phase B correctly uses `0.10` and `2.00` for zero_trust_defaults().

Phase A's `Default` impl should match zero-trust built-in defaults, but it uses `0.0` (unlimited) instead of `0.10/$2.00`.

**Recommended Fix**: Phase A's `UserPermissions::default()` should set `cost_budget_daily_usd: 0.10` and `cost_budget_monthly_usd: 2.00` to match the design doc's zero-trust defaults. The current `0.0` values mean an anonymous user gets unlimited budget, which contradicts the zero-trust security model.

---

### [GAP-T23] MEDIUM -- Phase I test helpers use `PermissionsConfig` with `UserPermissions` directly in struct fields

**Affected Plans**: I, A
**Issue**: Phase I's `test_permissions_config()` constructs `PermissionsConfig` with `zero_trust: UserPermissions { ... }`, `user: UserPermissions { ... }`, `admin: UserPermissions { ... }`. Phase A defines these fields as `PermissionLevelConfig` (Option-wrapped), not `UserPermissions`.

**Recommended Fix**: Phase I test helpers must use `PermissionLevelConfig` for the `zero_trust`, `user`, and `admin` fields of `PermissionsConfig`, matching Phase A's type definition.

---

## 4. Import Path Consistency Gaps

### [GAP-T24] HIGH -- `UserPermissions` and `AuthContext` crate location conflicts

**Affected Plans**: A, B, C, F
**Issue**:
- Phase A: Defines both types in `clawft-types/src/routing.rs`
- Phase B: Says `UserPermissions` and `AuthContext` are in `clawft-types` (Section 3.7 shows `clawft-types/src/config.rs`) but then proposes implementing methods and `PermissionResolver` in `clawft-core/src/pipeline/permissions.rs`. The pseudocode re-declares `UserPermissions` and `AuthContext` structs.
- Phase C: Imports from `clawft_types::auth::{AuthContext, UserPermissions}` (a non-existent `auth` module)
- Phase F: Proposes a new `clawft-types/src/auth.rs` file for `AuthContext` and `UserPermissions`, and adds `pub mod auth;` to `lib.rs`. This conflicts with Phase A which puts them in `routing.rs`.

Four different locations proposed:
1. `clawft-types/src/routing.rs` (Phase A)
2. `clawft-types/src/config.rs` (Phase B reference)
3. `clawft_types::auth` (Phase C import)
4. `clawft-types/src/auth.rs` (Phase F)

**Recommended Fix**: Use Phase A's location: `clawft-types/src/routing.rs`. The types are defined there; all other phases should import from `clawft_types::routing::{AuthContext, UserPermissions, PermissionLevelConfig, RoutingConfig, ...}`. Phase F should NOT create `auth.rs`. Phase C should update its imports. Phase B should add methods and the resolver in `clawft-core` but import the type definitions from `clawft-types`.

---

### [GAP-T25] MEDIUM -- `PermissionResolver` crate location

**Affected Plans**: B, F
**Issue**: Both Phase B and Phase F define a `PermissionResolver` in `clawft-core/src/pipeline/permissions.rs`. They have different constructors, different method signatures, and different internal data models. Only one implementation can exist.

**Recommended Fix**: Use Phase B's `PermissionResolver` as the canonical implementation (it is the phase explicitly scoped for this). Phase F should use Phase B's resolver rather than redefining it. Phase F's contribution should be limited to: (1) wiring the resolver into `AgentLoop`, (2) extending `ChatRequest` with `auth_context`, and (3) modifying channel plugins. Phase F should NOT redefine `PermissionResolver`.

---

### [GAP-T26] MEDIUM -- `ToolError::PermissionDenied` variant structure: existing code vs Plan G

**Affected Plans**: G
**Issue**: The existing codebase defines `ToolError::PermissionDenied(String)` as a tuple variant (in `crates/clawft-core/src/tools/registry.rs:36`). Existing pattern matches in the codebase use `ToolError::PermissionDenied(msg)` (e.g., `clawft-tools/tests/security_integration.rs:84`).

Phase G proposes changing this to a struct variant: `ToolError::PermissionDenied { tool: String, reason: String }`. This is a breaking change that requires updating every existing pattern match on `PermissionDenied`.

Phase G's Section 5.6 acknowledges this migration and lists the pattern-match updates, but the file ownership matrix does not include all test files that match on `PermissionDenied`.

**Recommended Fix**: Phase G should provide a complete inventory of all existing `ToolError::PermissionDenied(...)` match sites. The `security_integration.rs` test file and any other external crate tests must be updated. The change from tuple variant to struct variant is correct (structured errors are better), but all match sites must be migrated atomically.

---

## 5. Missing Type Definitions

### [GAP-T27] HIGH -- `PermissionOverrides` defined in Phase B but not in Phase A

**Affected Plans**: A, B
**Issue**: Phase B defines `PermissionOverrides` as the config-file type for partial permission overrides. Phase A defines `PermissionLevelConfig` for the same purpose. These are the same concept with different names and neither references the other.

**Recommended Fix**: Per GAP-T02, eliminate `PermissionOverrides` and use `PermissionLevelConfig` everywhere. Phase B's `from_config()` should convert `PermissionLevelConfig` to the merge inputs.

---

### [GAP-T28] MEDIUM -- `ToolMetadata` not defined in `clawft-types`

**Affected Plans**: G
**Issue**: Phase G defines `ToolMetadata` locally in `clawft-core/src/tools/registry.rs`. This type is referenced by the `Tool` trait's `metadata()` method and by `ToolPermissionChecker::check()`. No other phase defines or references this type.

**Recommended Fix**: This is acceptable. `ToolMetadata` is an internal type to the tools subsystem and does not need to be in `clawft-types`. No action needed.

---

### [GAP-T29] MEDIUM -- `CostTrackerConfig` defined in Phase D but not in `clawft-types`

**Affected Plans**: D, A
**Issue**: Phase D defines `CostTrackerConfig` as a runtime config struct with fields: `global_daily_limit_usd`, `global_monthly_limit_usd`, `reset_hour_utc`, `persistence_enabled`, `save_interval_ops`. Phase A defines `CostBudgetConfig` with similar fields: `global_daily_limit_usd`, `global_monthly_limit_usd`, `tracking_persistence`, `reset_hour_utc`.

These overlap but are not identical:
- `CostBudgetConfig.tracking_persistence` vs `CostTrackerConfig.persistence_enabled` (same concept, different names)
- `CostTrackerConfig.save_interval_ops` has no counterpart in `CostBudgetConfig`

**Recommended Fix**: Phase D's `CostTracker::new()` should accept `&CostBudgetConfig` (from Phase A) directly, plus any additional runtime-only parameters. Remove the redundant `CostTrackerConfig` struct. Add `save_interval_ops` as a constant or add it to `CostBudgetConfig` if it needs to be configurable.

---

### [GAP-T30] LOW -- `ModelTier` (runtime) vs `ModelTierConfig` (config) naming

**Affected Plans**: C, A
**Issue**: Phase A defines `ModelTierConfig` (the serde config type). Phase C defines `ModelTier` (the runtime type with an additional `ordinal` field). This is intentional and well-designed -- config types and runtime types are separate. However, the design doc (Section 4.1) shows `ModelTier` as the type name, which matches Phase C but could be confused with Phase A's `ModelTierConfig`.

**Recommended Fix**: No action needed. The `Config` suffix convention (config types) vs bare name (runtime types) is clear and intentional.

---

### [GAP-T31] LOW -- Phase I default values in test helpers do not match Phase A

**Affected Plans**: I, A
**Issue**: Phase I's test helper `test_routing_config()` shows `EscalationConfig::default()` but Phase I's unit test table (Section 1.2, Phase A test #6) expects `enabled=true, threshold=0.6, max_escalation_tiers=1`. Phase A's actual `EscalationConfig::default()` has `enabled: false` (not `true`).

**Recommended Fix**: Phase I's test table has an error: `EscalationConfig::default()` should be `enabled=false`, not `enabled=true`. Update the Phase I test specification to match Phase A's implementation.

---

## 6. Cross-Cutting Concerns

### [GAP-T32] MEDIUM -- No `TierSelectionStrategy` enum in Phase A but design doc shows it

**Affected Plans**: A, Design Doc
**Issue**: The orchestrator (Section 4, Phase A deliverables item 6) explicitly requires a `TierSelectionStrategy` enum with variants `PreferenceOrder`, `RoundRobin`, `LowestCost`, `Random`. The design doc Section 4.2 shows it as a proper enum. However, Phase A's pseudocode stores it as `Option<String>` on `RoutingConfig` with no enum definition.

**Recommended Fix**: (Same as GAP-T01) Phase A should define the enum in `routing.rs`. This was called out in the deliverables but not implemented in the pseudocode.

---

### [GAP-T33] LOW -- Phase H validation accesses `PermissionsConfig` fields inconsistently

**Affected Plans**: H, A
**Issue**: Phase H's validation pseudocode iterates over `[("zero_trust", &config.permissions.zero_trust), ...]` as if the fields are concrete `UserPermissions` values. But Phase A defines them as `PermissionLevelConfig` (with all `Option<T>` fields). Validation would need to unwrap the Options or validate differently.

**Recommended Fix**: Phase H validation should handle `PermissionLevelConfig` fields (all Optional). Validate only the fields that are `Some(...)`. For example, `if let Some(level) = perms.level { validate_level(level); }`.

---

### [GAP-T34] LOW -- `sender_id` vs `user_id` parameter naming

**Affected Plans**: B, C, D, E
**Issue**: The naming of the user identity parameter is inconsistent across phases:
- Phase B: uses `sender_id` consistently
- Phase C: `CostTrackable` trait uses `sender_id`
- Phase D: `CostTracker` methods use `user_id`
- Phase E: `RateLimiter::check()` uses `sender_id`

The design doc uses `sender_id` for the AuthContext field and `user_id` in the CostTracker data model.

**Recommended Fix**: Standardize on `sender_id` at the trait/API boundary (matching `AuthContext.sender_id` and the codebase convention). Phase D's `CostTracker` can use `user_id` internally as a DashMap key alias, but the public API and trait methods should accept `sender_id: &str` for consistency with the rest of the pipeline.

---

## Summary

| Severity | Count | Gap IDs |
|----------|-------|---------|
| CRITICAL | 3 | GAP-T01, GAP-T13, GAP-T14 |
| HIGH | 8 | GAP-T02, GAP-T03, GAP-T04, GAP-T15, GAP-T16, GAP-T17, GAP-T21, GAP-T24, GAP-T27 |
| MEDIUM | 15 | GAP-T05, GAP-T06, GAP-T07, GAP-T08, GAP-T09, GAP-T10, GAP-T18, GAP-T19, GAP-T20, GAP-T22, GAP-T23, GAP-T25, GAP-T26, GAP-T29, GAP-T32 |
| LOW | 6 | GAP-T11, GAP-T12, GAP-T28, GAP-T30, GAP-T31, GAP-T33, GAP-T34 |
| **Total** | **34** | |

### Key Themes

1. **Permission resolution priority ordering**: The most critical new finding is that the design doc and Phase B disagree on whether per-user or per-channel overrides have higher priority (GAP-T14). This fundamentally changes the security semantics of the permission system.

2. **Dual type definitions**: The most pervasive issue is the same concept being defined differently across phases. `PermissionLevelConfig` vs `PermissionOverrides` (GAP-T02, T27), `CostBudgetConfig` vs `CostTrackerConfig` (GAP-T29), and multiple naming variants for `UserPermissions` constructors (GAP-T05).

3. **Permission resolution ownership**: Three phases (B, C, F) all implement some form of permission resolution. Only Phase B should own this. Phase C and F should consume resolved permissions, not re-resolve them (GAP-T03, T17, T19).

4. **Import path disagreement**: `UserPermissions` and `AuthContext` are proposed in 4 different module locations (GAP-T24). Phase A's `routing.rs` should be canonical.

5. **Interface signature mismatches**: The `CostTrackable` trait (Phase C) and `CostTracker` implementation (Phase D) have incompatible method signatures (GAP-T08, T10). The design doc's `update()` calls a non-existent `record()` method (GAP-T20). Similarly, `PermissionResolver` has incompatible constructors and signatures between Phases B and F (GAP-T16).

6. **Security-critical defaults**: `AuthContext::default()` and `defaults_for_level()` have inconsistent fallback behavior that could lead to privilege escalation (GAP-T04, T06, T22).

---

**End of Gap Analysis**
