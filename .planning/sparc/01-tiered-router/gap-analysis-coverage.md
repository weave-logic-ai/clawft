# Gap Analysis: Tiered Router SPARC Plans -- Dependency Coverage & Completeness

**Analyzer**: Code Review Agent
**Date**: 2026-02-18
**Source Design Doc**: `.planning/08-tiered-router.md` (1205 lines, 13 sections)
**SPARC Plans Reviewed**: Orchestrator (00) + Phases A through I (10 documents)
**Analysis Dimensions**: Design Doc Coverage, Dependency Chain Validation, File Ownership Conflicts, Feature Completeness, Test Coverage Mapping

---

## 1. Design Doc Coverage (Sections 1-13)

### Coverage Matrix

| Design Doc Section | Primary Phase(s) | Coverage | Notes |
|--------------------|-------------------|----------|-------|
| 1. Executive Summary | Orchestrator | Full | Faithfully reproduced |
| 2. Permission Model (levels, tables) | A, B | Full | All 3 level tables covered |
| 3. Granular Permission Dimensions | A, B | Full | 16 dimensions mapped to struct fields |
| 4. TieredRouter Design | C | Full | Struct, traits, constructor, strategies |
| 5. Routing Algorithm (8 steps) | C | Partial | Step 8 (token clamping) underspecified |
| 6. Cost Tracking | D | Full | Budget check, recording, persistence, resets |
| 7. Configuration Format | A, H | Full | JSON schema, serde, aliases, defaults |
| 8. Extensibility | A (custom_permissions), G (MCP) | Partial | Custom tiers covered; plugin-provided perms partially covered |
| 9. Integration Points | C, F, G | Partial | 9.2 metadata vs field conflict; 9.5 UsageTracker integration sparse |
| 10. Migration Path | H, I | Full | 3 migration levels documented |
| 11. Implementation Plan | Orchestrator | Full | Phase mapping matches |
| 12. Risk Register | Orchestrator | Full | All 7 risks mirrored with mitigations |
| 13. Success Criteria (14 items) | Orchestrator (Sec 11), I | Full | All 14 mapped to phases and tests |

---

## 2. Gap Findings

### CRITICAL -- Type Definition Conflicts

**[GAP-C01]** Severity: CRITICAL
Affected Plans: A, B, C, F
Affected Sections: Design Doc 3.1, 7.2

**Description**: `UserPermissions` and `AuthContext` are defined with concrete struct layouts in four different plans, with incompatible variations:

- **Phase A** (`clawft-types/src/routing.rs`): Defines `UserPermissions` as a concrete-valued struct with serde defaults. `Default` impl produces zero_trust values (level=0, max_tier="", max_context=4096, etc.). Defines `AuthContext` with `sender_id`, `channel`, `permissions` fields. No explicit `Default` impl shown in Phase A pseudocode.

- **Phase B** (`clawft-core/src/pipeline/permissions.rs`): Redefines `UserPermissions` with the same fields but adds `zero_trust_defaults()`, `user_defaults()`, `admin_defaults()`, `defaults_for_level()`, and `level_name()` constructors. Also redefines `AuthContext` with `Default` returning `sender_id="local"`, `channel="cli"`, `permissions=admin_defaults()`.

- **Phase C** (`clawft-core/src/pipeline/tiered_router.rs`): Imports `AuthContext` and `UserPermissions` from `clawft_types::auth` (a module that does not match Phase A's `clawft_types::routing`). Has its own `resolve_permissions()` and `merge_permissions()` methods on `TieredRouter`.

- **Phase F** (`clawft-core/src/agent/loop_core.rs`): References `clawft_types::routing::AuthContext`. Notes `AuthContext::default()` returns `sender_id=""`, `channel=""`, `permissions=zero_trust` -- contradicting Phase B's default of `"local"/"cli"/admin`.

**Conflict**: The `AuthContext::default()` semantics are the most dangerous disagreement. Phase B says default = admin (for CLI safety), while Phase A/F say default = zero_trust (for pipeline safety when auth is absent). These cannot both be correct. The design doc Section 3.3 says "default to zero-trust when absent" but Section 2.4 says "CLI: admin."

**Recommended Fix**: Adopt a single canonical definition:
1. `UserPermissions` and `AuthContext` live ONLY in `clawft-types/src/routing.rs` (Phase A's file). No redefinition elsewhere.
2. `AuthContext::default()` = zero_trust (empty sender, empty channel, zero_trust permissions). This is the safe pipeline default when auth_context is `None`.
3. Phase B adds `impl UserPermissions` methods (`zero_trust_defaults`, `user_defaults`, `admin_defaults`, `merge`, `defaults_for_level`, `level_name`) but does NOT redefine the struct. These impls should be in `clawft-core` or on the type in `clawft-types`.
4. CLI-as-admin is handled by `PermissionResolver` returning admin-level `AuthContext`, NOT by `AuthContext::default()`.
5. Phase C imports from `clawft_types::routing`, not `clawft_types::auth`.
6. Add this decision to the orchestrator's consensus decisions log.

---

**[GAP-C02]** Severity: CRITICAL
Affected Plans: A, B, C
Affected Sections: Design Doc 3.2 (Permission Resolution)

**Description**: `PermissionsConfig` type has incompatible layouts between Phase A and Phases B/C:

- **Phase A**: `PermissionsConfig` has named fields `zero_trust: PermissionLevelConfig`, `user: PermissionLevelConfig`, `admin: PermissionLevelConfig` (all concrete, non-Optional types via `#[derive(Default)]`), plus `users: HashMap<String, PermissionLevelConfig>` and `channels: HashMap<String, PermissionLevelConfig>`.

- **Phase B**: References `PermissionsConfig` with `zero_trust: Option<PermissionOverrides>`, `user: Option<PermissionOverrides>`, `admin: Option<PermissionOverrides>`, `users: Option<HashMap<String, PermissionOverrides>>`, `channels: Option<HashMap<String, PermissionOverrides>>`.

- **Phase C**: References `permissions.zero_trust` as `Option<UserPermissions>`, `permissions.user` as `Option<UserPermissions>`, etc. Uses `.clone()` on Option contents, not `PermissionOverrides`.

**Conflict**: Three different types for the same config fields:
1. Phase A: `PermissionLevelConfig` (Option-wrapped fields for partial overrides in config)
2. Phase B: `PermissionOverrides` (Option-wrapped fields, separate struct)
3. Phase C: `Option<UserPermissions>` (concrete-valued, implies the config contains fully-resolved permissions)

**Recommended Fix**: Standardize on Phase A's design, which correctly separates config representation from runtime representation:
1. Config layer: `PermissionsConfig` uses `PermissionLevelConfig` (all Option fields) for level defaults, users, and channels.
2. Runtime layer: `UserPermissions` (all concrete fields) is the resolved output.
3. Phase B's `PermissionOverrides` is conceptually identical to Phase A's `PermissionLevelConfig` -- use `PermissionLevelConfig` everywhere and remove `PermissionOverrides`.
4. Phase C must not treat config-layer permissions as `Option<UserPermissions>`. It should delegate resolution to Phase B's `PermissionResolver`.

---

### CRITICAL -- Trait Interface Mismatch

**[GAP-C03]** Severity: CRITICAL
Affected Plans: C, D
Affected Sections: Design Doc 5.2 Step 6, 6.1

**Description**: Phase C defines a `CostTrackable` trait that Phase D's `CostTracker` must implement, but their interfaces are incompatible:

- **Phase C's trait**:
  ```
  fn check_budget(&self, sender_id: &str) -> Option<f64>;
  fn record_estimated_cost(&self, sender_id: &str, cost_usd: f64);
  fn record_actual_cost(&self, sender_id: &str, cost_usd: f64);
  ```

- **Phase D's concrete API**:
  ```
  fn check_budget(&self, user_id: &str, estimated_cost_usd: f64, user_daily_limit: f64, user_monthly_limit: f64) -> BudgetResult;
  fn record_estimated(&self, user_id: &str, estimated_cost_usd: f64);
  fn record_actual(&self, user_id: &str, estimated_cost_usd: f64, actual_cost_usd: f64);
  ```

**Conflicts**:
1. `check_budget()`: Phase C takes only `sender_id` and returns `Option<f64>`. Phase D takes 4 parameters and returns `BudgetResult` enum.
2. `record_actual_cost()`: Phase C takes 2 params (sender_id, cost). Phase D's `record_actual()` takes 3 params (user_id, estimated, actual) for delta reconciliation.
3. Method names differ: `record_estimated_cost` vs `record_estimated`, `record_actual_cost` vs `record_actual`.

**Recommended Fix**: Update Phase C's `CostTrackable` trait to match Phase D's concrete API:
```rust
pub trait CostTrackable: Send + Sync {
    fn check_budget(
        &self,
        user_id: &str,
        estimated_cost_usd: f64,
        user_daily_limit: f64,
        user_monthly_limit: f64,
    ) -> BudgetResult;
    fn record_estimated(&self, user_id: &str, estimated_cost_usd: f64);
    fn record_actual(&self, user_id: &str, estimated_cost_usd: f64, actual_cost_usd: f64);
}
```
Update `NoopCostTracker` accordingly. The `TieredRouter.route()` method must extract `user_daily_limit` and `user_monthly_limit` from the resolved `UserPermissions` and pass them to `check_budget()`. Import or re-export `BudgetResult` in the appropriate module.

---

### CRITICAL -- Duplicated Permission Resolution Logic

**[GAP-C04]** Severity: CRITICAL
Affected Plans: B, C, F
Affected Sections: Design Doc 3.2

**Description**: Permission resolution is implemented three times:

1. **Phase B**: `PermissionResolver` struct with 5-layer `resolve()` method using `PermissionOverrides` and `merge()`. This is the canonical implementation with full coverage of built-in defaults, global config, workspace config, channel overrides, and user overrides.

2. **Phase C**: `TieredRouter` has its own `resolve_permissions()` and `merge_permissions()` as private methods. This implementation uses `UserPermissions` directly (not `PermissionOverrides`), only implements 4 layers (missing workspace layer), and uses a different merge strategy (empty-string/empty-vec checks instead of Option checks).

3. **Phase F**: References Phase B's `PermissionResolver` but also describes the resolution algorithm independently in Section 2.3.

**Conflict**: Phase C's 4-layer resolution diverges from the design doc's 5-layer resolution. It also has a different merge semantic: Phase C's merge uses "empty string means no change" whereas Phase B's merge uses `Option::None` means no change. These produce different results for edge cases (e.g., deliberately setting `max_tier` to `""` or `model_access` to `[]`).

**Recommended Fix**:
1. Phase C's `TieredRouter` should NOT contain permission resolution logic. It should receive a pre-resolved `UserPermissions` from Phase B's `PermissionResolver`.
2. The `TieredRouter::from_config()` should accept a `PermissionResolver` (or `Arc<PermissionResolver>`) and delegate all permission work to it.
3. Delete `resolve_permissions()` and `merge_permissions()` from `TieredRouter`.
4. In `TieredRouter::route()`, call `self.permission_resolver.resolve(sender_id, channel)` instead of `self.resolve_permissions(auth)`.

---

### HIGH -- Missing Routing Algorithm Step

**[GAP-C05]** Severity: HIGH
Affected Plans: C, F
Affected Sections: Design Doc 5.1 Step 8

**Description**: The routing algorithm's Step 8 ("Apply token limits -- Clamp context/output tokens to user's `max_context_tokens` / `max_output_tokens`") is mentioned in Phase C's specification (Section 1.2, Step 8) but no pseudocode or implementation guidance is provided for it. Phase C's `route()` method pseudocode ends after model selection (Step 7). The `RoutingDecision` struct is extended with `tier`, `cost_estimate_usd`, `escalated`, `budget_constrained` but does not include `max_context_tokens` or `max_output_tokens` fields for downstream enforcement.

**Recommended Fix**: Add to Phase C's `RoutingDecision` extension:
```rust
pub max_context_tokens: Option<usize>,
pub max_output_tokens: Option<usize>,
```
Add Step 8 pseudocode to the route() method:
```rust
// Step 8: Apply token limits from user permissions
let max_ctx = permissions.max_context_tokens.min(tier.max_context_tokens);
let max_out = permissions.max_output_tokens;
```
The `AgentLoop` or `PipelineRegistry` can then read these from the `RoutingDecision` to clamp the actual `ChatRequest` before sending to the LLM provider.

---

### HIGH -- AuthContext Default Semantics Disagreement

**[GAP-C06]** Severity: HIGH
Affected Plans: A, B, F
Affected Sections: Design Doc 2.4, 3.3, 9.2

**Description**: Phase B defines `AuthContext::default()` as `{sender_id: "local", channel: "cli", permissions: admin_defaults()}` while Phase F Section 2.5 states `AuthContext::default()` returns `{sender_id: "", channel: "", permissions: zero_trust}`. Phase A's I-testing-documentation (Phase I) Section 1.2 test #9 expects `AuthContext::default()` to be `{sender_id: "", channel: "", permissions: zero_trust}`.

The design doc is clear: when `auth_context` is `None` (absent), zero-trust is the safe default (Section 5.1 Step 1). CLI-as-admin is a property of the `PermissionResolver`, not the default constructor.

**Recommended Fix**: `AuthContext::default()` must return zero-trust values (`sender_id: ""`, `channel: ""`, `permissions: zero_trust`). Update Phase B's `AuthContext` impl to match. The CLI admin behavior is achieved by the `PermissionResolver` returning admin-level auth when `channel == "cli"`, which is Phase F's responsibility.

---

### HIGH -- Design Doc Section 9.2 Metadata vs Field Conflict

**[GAP-C07]** Severity: HIGH
Affected Plans: F
Affected Sections: Design Doc 9.2

**Description**: The design doc Section 9.2 states `InboundMessage.metadata["sender_id"]` is how sender identity flows from channels. However, Phase F Section 1.3 correctly identifies that `InboundMessage` already has dedicated `pub sender_id: String` and `pub channel: String` fields (not metadata). Phase F's implementation reads from these fields directly, which is correct.

However, the design doc's Section 9.2 example code says:
```
sets InboundMessage.metadata["sender_id"] = user_id
```
This is outdated/incorrect per the actual codebase.

**Recommended Fix**: Flag this as a design doc errata. Phase F's approach (reading `InboundMessage.sender_id` directly) is correct. Update Section 9.2 of the design doc to reflect the actual `InboundMessage` struct fields. No SPARC plan change needed -- Phase F already handles this correctly.

---

### HIGH -- weft status Integration Lacks Implementation Plan

**[GAP-C08]** Severity: HIGH
Affected Plans: I
Affected Sections: Design Doc 13, Success Criterion #14

**Description**: Success criterion #14 states: "`weft status` reports the active routing mode, tier count, and permission summary." The orchestrator (Section 11) maps this to Phase I with note "Integration test or manual verification." Phase I's test plan lists this as "manual verification" only. No SPARC phase provides implementation guidance for:
1. Where in the CLI codebase `weft status` reads routing config
2. What format the routing status output takes
3. How tier count and permission summary are surfaced

**Recommended Fix**: Add a sub-task to Phase I (or create a small Phase I addendum) that:
1. Identifies the `weft status` command handler (likely `crates/clawft-cli/src/commands/status.rs` or similar)
2. Adds routing mode, tier count, and permission level summary to the status output
3. Provides at minimum a smoke test that verifies the status output contains routing info when `routing.mode == "tiered"`

---

### MEDIUM -- File Ownership Conflicts on Shared Files

**[GAP-C09]** Severity: MEDIUM
Affected Plans: B, C, D, E, F
Affected Sections: Orchestrator Section 7

**Description**: The orchestrator's file ownership matrix (Section 7) identifies shared files but several plans modify the same files in overlapping ways:

1. **`src/pipeline/mod.rs`**: Modified by B, C, D, E, F (adding module declarations). The orchestrator acknowledges this but relies on "Phase X makes the modification with explicit backward compatibility." In practice, 5 phases adding `pub mod X;` lines to the same file creates merge ordering risk.

2. **`src/pipeline/traits.rs`**: Modified by both C (extending `RoutingDecision`) and F (adding `auth_context` to `ChatRequest`). These are different changes to the same file, and execution order (C before F per dependency graph) should be safe, but neither plan mentions the other's change.

3. **`src/pipeline/tiered_router.rs`**: Created by C, then modified by D (integrating CostTracker) and F (no direct modification per F's plan, but F's integration affects how the router reads auth_context).

**Recommended Fix**: The orchestrator's conflict resolution protocol (Section 7) is adequate in principle, but the phase plans should explicitly acknowledge what other phases have already modified. Add a "Pre-conditions" section to each phase plan listing expected state of shared files. Specifically:
- Phase D: Note that `tiered_router.rs` already exists from Phase C with `CostTrackable` trait object
- Phase F: Note that `traits.rs` already has `RoutingDecision` extensions from Phase C
- All phases modifying `mod.rs`: Include the full expected module list at time of modification

---

### MEDIUM -- Phase C's PermissionsConfig Usage Assumes Wrong Types

**[GAP-C10]** Severity: MEDIUM
Affected Plans: C
Affected Sections: Design Doc 7.2

**Description**: Phase C's `TieredRouter::parse_permission_defaults()` expects `permissions.zero_trust` to be `Option<UserPermissions>`, but Phase A defines it as `PermissionLevelConfig` (a non-Option struct with all-Option fields). This means Phase C's code:
```rust
if let Some(ref zt) = permissions.zero_trust {
    defaults.insert("zero_trust".into(), zt.clone());
}
```
will not compile because `permissions.zero_trust` is `PermissionLevelConfig`, not `Option<UserPermissions>`.

Similarly, `parse_user_overrides()` and `parse_channel_overrides()` expect `HashMap<String, UserPermissions>` but Phase A defines them as `HashMap<String, PermissionLevelConfig>`.

**Recommended Fix**: This is resolved by GAP-C04's recommendation. If `TieredRouter` delegates all permission work to `PermissionResolver`, it does not need `parse_permission_defaults()`, `parse_user_overrides()`, or `parse_channel_overrides()` at all. If those methods are retained for some reason, they must be updated to work with `PermissionLevelConfig` and convert to `UserPermissions` via the merge pipeline.

---

### MEDIUM -- Missing Workspace Layer in Phase C's Resolution

**[GAP-C11]** Severity: MEDIUM
Affected Plans: C
Affected Sections: Design Doc 3.2

**Description**: Phase C's `resolve_permissions()` implements only 4 layers:
1. Built-in defaults
2. Global config level defaults
3. Channel overrides
4. Per-user overrides

The design doc specifies 5 layers (Phase B correctly implements all 5):
1. Built-in defaults
2. Global config level overrides
3. **Workspace config level overrides** (MISSING in Phase C)
4. Channel overrides
5. Per-user overrides

**Recommended Fix**: Resolved by GAP-C04 -- Phase C should delegate to Phase B's `PermissionResolver` which has the workspace layer. If Phase C retains its own resolution (not recommended), add workspace_level_overrides to `TieredRouter` struct and merge them between steps 2 and 3.

---

### MEDIUM -- RateLimiter Dependency Mismatch

**[GAP-C12]** Severity: MEDIUM
Affected Plans: E, Orchestrator
Affected Sections: Orchestrator Section 2 (Dependency Graph)

**Description**: The orchestrator's dependency table says Phase E depends on "Phase A" (`A -> E`), but the Phase E plan header says "Depends On: Phase B (UserPermissions)". Phase E uses `UserPermissions.rate_limit` as its limit parameter, which comes from Phase B's resolution. However, Phase E's actual `check()` method takes `limit: u32` as a parameter, not a `UserPermissions` reference, so it technically only needs the `u32` type (which is primitive, not from Phase A or B).

The dependency graph shows `E` parallel with `B` after Gate A. This is correct from a compilation standpoint (E does not import Phase B types), but conceptually E cannot be integration-tested without B providing resolved permissions.

**Recommended Fix**: The orchestrator's dependency graph is correct for compilation ordering. Add a note to Phase E that integration testing requires Phase B's `UserPermissions` for meaningful limit values. The Gate B+E check already covers this by requiring both to pass before Window 3.

---

### MEDIUM -- Cost Tracking Integration with UsageTracker

**[GAP-C13]** Severity: MEDIUM
Affected Plans: D
Affected Sections: Design Doc 9.5

**Description**: Design doc Section 9.5 describes integration between `CostTracker` and `clawft-llm::UsageTracker`:
```
ModelRouter::update(decision, outcome)
  +-- TieredRouter.cost_tracker.record(...)
  +-- clawft_llm::UsageTracker.record(...)
```

Phase D's plan focuses entirely on `CostTracker` internals and does not address how `record_actual()` receives actual token usage from the LLM response. The `ModelRouter::update()` method is referenced but Phase D does not show how `actual_cost_usd` is computed from the response's `usage` field.

**Recommended Fix**: Add a section to Phase D (or Phase C's update() implementation) that:
1. Shows how `ResponseOutcome` provides token usage
2. Computes `actual_cost_usd` using `tier.cost_per_1k_tokens * (input_tokens + output_tokens) / 1000.0`
3. Calls `cost_tracker.record_actual(user_id, estimated_cost, actual_cost)` in the `update()` method
4. Notes that `clawft-llm::UsageTracker` integration is out of scope for this sprint but should be wired in a future phase

---

### MEDIUM -- Phase A File Location Disagreement with Design Doc

**[GAP-C14]** Severity: MEDIUM
Affected Plans: A
Affected Sections: Design Doc 11.2

**Description**: The design doc Section 11.2 says routing types go in `src/config.rs` (extend existing). Phase A creates a new file `src/routing.rs` instead. Phase C imports from `clawft_types::auth` (yet another location). Phase F imports from `clawft_types::routing`.

**Recommended Fix**: Phase A's choice of `src/routing.rs` (new file) is arguably better than cramming types into `config.rs` (separation of concerns). However, all downstream phases must agree on the import path. Standardize on `clawft_types::routing` as the module path. Update Phase C's imports from `clawft_types::auth` to `clawft_types::routing`. Update the design doc Section 11.2 to reflect the new file.

---

### LOW -- Phase B's PermissionOverrides vs Phase A's PermissionLevelConfig

**[GAP-C15]** Severity: LOW
Affected Plans: A, B
Affected Sections: Design Doc 3.2

**Description**: Phase A defines `PermissionLevelConfig` (all `Option<T>` fields for partial config overrides). Phase B defines `PermissionOverrides` (all `Option<T>` fields for partial overrides). These are structurally identical -- same fields, same `Option` wrapping, same purpose.

**Recommended Fix**: Use Phase A's `PermissionLevelConfig` as the single config-layer type. Phase B's `PermissionOverrides` is redundant and should be replaced with a type alias or direct usage of `PermissionLevelConfig`. Phase B's `merge()` method can be implemented as `impl UserPermissions { fn merge(&mut self, overrides: &PermissionLevelConfig) }`.

---

### LOW -- Design Doc Section 8.4 Future Extensions Not Flagged as Out of Scope

**[GAP-C16]** Severity: LOW
Affected Plans: None
Affected Sections: Design Doc 8.4

**Description**: Section 8.4 lists 6 future extensions (per-channel permissions, per-skill permissions, dynamic grants, delegation, audit logging, usage dashboards). These are correctly marked as "planned extension points, not current scope" in the design doc, and no SPARC phase attempts to implement them. This is correct behavior but worth noting for completeness.

**Recommended Fix**: No action needed. The SPARC plans correctly exclude future extensions.

---

### LOW -- Phase G Tool Permission Check Order Differs from Design Doc

**[GAP-C17]** Severity: LOW
Affected Plans: G
Affected Sections: Design Doc 9.4

**Description**: The design doc Section 9.4 checks allowlist before denylist:
```rust
if !permissions.tool_access.contains("*") && !permissions.tool_access.contains(tool_name) {
    // deny
}
if permissions.tool_denylist.contains(tool_name) {
    // deny
}
```

Phase G Section 1.2, Requirement 8 explicitly specifies: "Denylist first -- if the tool is in tool_denylist, deny immediately. Allowlist second."

This means Phase G's order is: denylist -> allowlist -> MCP level -> MCP custom. The design doc's order is: allowlist -> denylist.

**Recommended Fix**: Phase G's order (denylist first) is the more secure approach -- it ensures a denied tool is always blocked regardless of allowlist state. This is a deliberate improvement over the design doc's simpler example. Document this as a design decision in the decisions log.

---

### LOW -- Escalation Config Field Naming

**[GAP-C18]** Severity: LOW
Affected Plans: A, C
Affected Sections: Design Doc 7.1

**Description**: The design doc's JSON uses `escalation.threshold` (single field controlling the global escalation threshold). Phase A's `EscalationConfig` has `threshold: f32` (global) but `UserPermissions` also has `escalation_threshold: f32` (per-user). Phase C's escalation logic uses `permissions.escalation_threshold` (from UserPermissions), not the global `EscalationConfig.threshold`.

This means the global `EscalationConfig.threshold` is effectively unused at runtime because the per-user `escalation_threshold` from resolved permissions always takes precedence.

**Recommended Fix**: Document that `EscalationConfig.threshold` is the default value used when building permission-level defaults (i.e., it populates `UserPermissions.escalation_threshold` during resolution). The `EscalationConfig.enabled` field controls whether escalation is available system-wide, and `EscalationConfig.max_escalation_tiers` limits the escalation distance. Phase B's resolver should use `EscalationConfig.threshold` as the default `escalation_threshold` for level defaults if not overridden.

---

## 3. Feature Completeness Checklist

| Feature | Design Doc Reference | Phase(s) | Status | Gap ID |
|---------|---------------------|----------|--------|--------|
| 4 model tiers (free/standard/premium/elite) | Sec 4.1 | A, C | Complete | -- |
| 3 permission levels (zero_trust/user/admin) | Sec 2 | A, B | Complete | -- |
| 16 permission dimensions | Sec 3.1 | A, B | Complete | -- |
| 4 selection strategies | Sec 4.3 | C | Complete | -- |
| Escalation logic + max_escalation_tiers | Sec 4.4 | C | Complete | -- |
| Fallback chain (same tier -> lower -> fallback -> error) | Sec 4.5 | C | Complete | -- |
| Cost tracking (daily/monthly/global) | Sec 6 | D | Complete | -- |
| Rate limiting (sliding window, LRU eviction) | Sec 5.2 Step 3 | E | Complete | -- |
| Workspace config overrides (deep merge) | Sec 7.3, 10 | H | Complete | -- |
| CLI default admin | Sec 2.4, 9.3 | B, F | Complete | GAP-C06 (default semantics) |
| Per-user config overrides | Sec 3.2 | B | Complete | -- |
| Per-channel config overrides | Sec 3.2 | B | Complete | -- |
| Tool permission enforcement | Sec 8.3, 9.4 | G | Complete | GAP-C17 (order differs) |
| Token limit clamping (Step 8) | Sec 5.1 | C | Incomplete | GAP-C05 |
| 5-layer permission resolution | Sec 3.2 | B | Complete (B), Incomplete (C) | GAP-C04, GAP-C11 |
| CostTrackable trait <-> CostTracker API | Sec 6 | C, D | Incompatible | GAP-C03 |
| AuthContext default behavior | Sec 3.3, 5.1 | A, B, F | Conflicting | GAP-C01, GAP-C06 |
| weft status routing info | Sec 13 (#14) | I | No implementation plan | GAP-C08 |
| Backward compat (StaticRouter unchanged) | Sec 1, 10 | A, C, H | Complete | -- |
| serde aliases (camelCase + snake_case) | Sec 7 | A | Complete | -- |
| Config validation (17 rules) | Sec 7 | H | Complete | -- |
| Custom permission dimensions | Sec 8.1 | A | Complete | -- |
| MCP tool permission metadata | Sec 8.3 | G | Complete | -- |
| Persistence (cost tracking to disk) | Sec 6 | D | Complete | -- |
| UsageTracker integration | Sec 9.5 | D | Partial | GAP-C13 |

---

## 4. Test Coverage Mapping (14 Success Criteria)

| # | Success Criterion | Phase(s) | Unit Tests | Integration Tests | Coverage |
|---|-------------------|----------|------------|-------------------|----------|
| 1 | StaticRouter works when routing absent | A, C | A: config_default | I: backward compat | Full |
| 2 | TieredRouter routes by complexity | C | C: route_simple_to_free, route_complex_to_premium | I: full pipeline | Full |
| 3 | Level 0 blocked from premium (no escalation) | B, C | C: route_escalation_disabled | I: zero_trust blocked | Full |
| 4 | Level 1 escalation to premium | C | C: route_escalation_promotes | I: user escalation | Full |
| 5 | Level 2 access all + override | C | C: (not explicitly listed) | I: admin override | Partial -- Phase C missing explicit model_override test |
| 6 | Rate limiting rejects excess | E, F | E: test_over_limit | I: rate limit integration | Full |
| 7 | Budget fallback to cheaper tier | D | D: check_budget_over_daily | I: budget fallback | Full |
| 8 | Tool access enforced (Level 0 no exec) | G | G: zero_trust denied | I: tool permission | Full |
| 9 | CLI defaults to admin | B, F | B: resolve_admin_level | I: CLI admin | Full |
| 10 | Per-user overrides work | B, H | B: resolve_per_user_override | I: per-user config | Full |
| 11 | Config deep merge global+workspace | H | H: deep merge tests (6) | I: config merge | Full |
| 12 | allow_from unchanged | F | F: check_allow_from tests | I: allow_from compat | Full |
| 13 | Backward compat (unknown fields ignored) | A, H | A: unknown_fields_ignored | I: config fixtures | Full |
| 14 | weft status reports routing | I | None | Manual verification only | Incomplete (GAP-C08) |

**Test Count Summary**:
- Phase A: 16 unit tests planned
- Phase B: 19 unit tests planned (plan says 19), Phase I inventory says minimum 10
- Phase C: 39 unit tests planned (plan says 39), Phase I inventory says minimum 15
- Phase D: 18 unit tests planned
- Phase E: 11 unit tests planned
- Phase F: 14 unit tests + 3 integration sketches
- Phase G: 20 unit tests planned
- Phase H: 29 tests (7 parsing + 16 validation + 6 deep merge)
- Phase I: 10 integration + 15 security + 7 config fixture
- **Total**: 166+ unit tests, 32+ integration/security/fixture tests

---

## 5. Dependency Chain Validation

### Orchestrator Dependency Graph

```
A -> (B, E) -> (C, G) -> (D, F) -> H -> I
```

### Validation Results

| Edge | Valid? | Issue |
|------|--------|-------|
| A -> B | Yes | B uses types from A (RoutingConfig, PermissionsConfig) |
| A -> E | Yes (weak) | E only needs primitive types. Could start after A but has no compile dependency. |
| B -> C | Yes | C uses permission types from A, resolution from B |
| B -> G | Yes | G uses UserPermissions (tool_access, tool_denylist) from A/B |
| B -> F | Yes | F uses PermissionResolver from B |
| C -> D | Yes | D implements CostTrackable trait defined in C |
| C -> F | Yes | F modifies ChatRequest which C's router reads |
| E -> F | Yes (weak) | F needs RateLimiter available but does not modify it |
| D, F, G -> H | Yes | H validates all types and config structures from prior phases |
| H -> I | Yes | I tests everything; needs H's validation in place |

**Hidden Dependencies Not in Graph**:
1. **Phase D depends on Phase B** (not shown): `check_budget()` needs `UserPermissions.cost_budget_daily_usd` and `cost_budget_monthly_usd` at the call site. Phase D does not import these types directly (they are passed as `f64`), but the integration requires B's resolution to be complete.
2. **Phase G depends on Phase F** (noted in plan but not strict): G says "Phase F should also be complete or in progress so that ChatRequest carries auth_context." This is a soft dependency -- G can implement and test without F, but integration requires F.

### Critical Path

```
A (0.5d) -> B (1d) -> C (1.5d) -> D (1d) -> H (1d) -> I (1.5d) = 6.5 days
```

The parallel paths (E alongside B, G alongside C) save approximately 1 day, bringing the parallel estimate to ~5.5-6 days of elapsed time vs the 8.5-day total effort.

---

## 6. Summary

### Gap Statistics

| Severity | Count | IDs |
|----------|-------|-----|
| CRITICAL | 4 | GAP-C01, GAP-C02, GAP-C03, GAP-C04 |
| HIGH | 4 | GAP-C05, GAP-C06, GAP-C07, GAP-C08 |
| MEDIUM | 6 | GAP-C09, GAP-C10, GAP-C11, GAP-C12, GAP-C13, GAP-C14 |
| LOW | 4 | GAP-C15, GAP-C16, GAP-C17, GAP-C18 |
| **Total** | **18** | |

### Key Themes

1. **Type Definition Fragmentation** (GAP-C01, C02, C10, C14, C15): The most systemic issue. UserPermissions, AuthContext, PermissionsConfig, and PermissionOverrides are defined or redefined across 4 plans with incompatible representations. This will cause compile errors when phases are integrated unless the canonical definition in Phase A is strictly respected as the single source of truth.

2. **Permission Resolution Duplication** (GAP-C04, C11): Three independent implementations of permission resolution logic will cause behavioral divergence. The `PermissionResolver` from Phase B should be the single authority, with Phase C delegating to it rather than reimplementing.

3. **Trait-Implementation Mismatch** (GAP-C03): The `CostTrackable` trait defined in Phase C does not match the `CostTracker` API built in Phase D. This is a compile-time blocker that must be resolved before Phase D can implement the trait.

4. **Default Semantics Conflict** (GAP-C06): `AuthContext::default()` must be zero-trust for pipeline safety. CLI-as-admin is a resolution-time behavior, not a default-constructor behavior.

### Recommended Action Plan

**Before execution begins** (pre-Window 1):
1. Resolve GAP-C01: Add `AuthContext::default()` = zero_trust decision to decisions.md
2. Resolve GAP-C02: Confirm `PermissionLevelConfig` is the single config-layer type
3. Resolve GAP-C15: Remove `PermissionOverrides` from Phase B, use `PermissionLevelConfig`

**Between Windows 2 and 3** (after Gate B+E, before C/G launch):
4. Resolve GAP-C03: Update `CostTrackable` trait in Phase C's plan to match Phase D's API
5. Resolve GAP-C04: Remove permission resolution from Phase C; add `PermissionResolver` dependency
6. Resolve GAP-C05: Add Step 8 (token clamping) pseudocode to Phase C

**Between Windows 4 and 5** (after Gate D+F, before H launch):
7. Resolve GAP-C08: Add `weft status` sub-task to Phase I with implementation guidance
8. Resolve GAP-C13: Add actual-cost computation section to Phase D or C's update() method

**During execution** (continuous):
9. Resolve GAP-C09: Each phase verifies shared file state before modifying
10. Resolve GAP-C14: All phases use `clawft_types::routing` as the canonical import path
