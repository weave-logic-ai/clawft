# SPARC Implementation Plan: Phase C - TieredRouter Core

**Phase**: Tiered Router (01)
**Owner**: Pipeline architect
**Branch**: `weft/tiered-router`
**Status**: BLOCKED BY Phases A, B
**Effort**: 1.5 days

---

## Agent Instructions

### Mission

You are implementing the `TieredRouter` -- the Level 1 replacement for `StaticRouter` in the
clawft pipeline system. This router selects models based on three inputs: task complexity (from
`TaskClassifier`), user permissions (from `AuthContext`), and cost budgets (from `CostTracker`).

The router implements the existing `ModelRouter` trait (defined in `traits.rs`) as a drop-in
replacement for `StaticRouter`. When `config.routing.mode == "tiered"`, the pipeline uses
`TieredRouter` instead of `StaticRouter`. All existing `StaticRouter` behavior is unchanged.

The router's tier configuration has intentionally overlapping complexity ranges. A request at
complexity 0.5 could match `standard` (0.0-0.7) or `premium` (0.3-1.0). The router picks the
highest-quality tier the user is **allowed** (by permission level) and can **afford** (by cost
budget). This is the central design invariant.

### Files to Read

| File | Purpose |
|------|---------|
| `crates/clawft-core/src/pipeline/traits.rs` | `ModelRouter` trait, `RoutingDecision`, `TaskProfile`, `ChatRequest`, `ResponseOutcome` -- the interface this module implements |
| `crates/clawft-core/src/pipeline/router.rs` | `StaticRouter` -- Level 0 reference implementation to stay compatible with |
| `crates/clawft-core/src/pipeline/mod.rs` | Module declarations -- add `pub mod tiered_router;` here |
| `crates/clawft-types/src/routing.rs` | Phase A types: `RoutingConfig`, `ModelTierConfig`, `EscalationConfig`, `PermissionsConfig`, `TierSelectionStrategy` (config enum) |
| `crates/clawft-types/src/routing.rs` | Phase A types: `UserPermissions`, `AuthContext` structs |
| `crates/clawft-core/src/pipeline/permissions.rs` | Phase B: `PermissionResolver`, `UserPermissions::merge()`, built-in defaults |
| `.planning/08-tiered-router.md` Section 4 | TieredRouter struct design, `ModelTier`, `TierSelectionStrategy` |
| `.planning/08-tiered-router.md` Section 5 | Routing algorithm (8-step decision flow), escalation logic, fallback chain |
| `.planning/sparc/01-tiered-router/A-routing-config-types.md` | Phase A plan -- types this module consumes |
| `.planning/sparc/01-tiered-router/B-permissions-resolution.md` | Phase B plan -- permission resolution this module delegates to |

### Critical Success Criteria

- [ ] `TieredRouter` compiles and implements `ModelRouter` trait (`Send + Sync`)
- [ ] `StaticRouter` continues to work unchanged (all existing tests pass)
- [ ] `RoutingDecision` extended with `tier`, `cost_estimate_usd`, `escalated`, `budget_constrained` without breaking existing code
- [ ] `ChatRequest` extended with optional `auth_context` without breaking existing code
- [ ] `from_config()` correctly parses `RoutingConfig` into a working `TieredRouter`
- [ ] Tier selection picks the highest-quality tier the user is allowed and can afford
- [ ] Escalation promotes to next tier only when `escalation_allowed`, `escalation_config.enabled`, and `complexity > threshold`
- [ ] Budget constraints downgrade to cheaper tier when daily/monthly caps would be exceeded
- [ ] Fallback chain fires correctly: same tier -> lower tier -> fallback_model -> empty decision
- [ ] All 4 selection strategies (PreferenceOrder, RoundRobin, LowestCost, Random) work
- [ ] `cargo test -p clawft-core` passes with all new and existing tests
- [ ] `cargo clippy -p clawft-core` has zero warnings

---

## 1. Specification

### 1.1 TieredRouter Overview

The `TieredRouter` implements the existing `ModelRouter` trait (defined in `traits.rs`) and
provides complexity-aware, permission-gated, budget-constrained model routing. It replaces
`StaticRouter` when `config.routing.mode == "tiered"`.

**Core responsibilities:**

- Accept a `ChatRequest` and `TaskProfile`, return a `RoutingDecision`
- Extract `AuthContext` from the `ChatRequest` (default to zero-trust if absent)
- Read the already-resolved `UserPermissions` from `AuthContext.permissions` (resolution is
  Phase B's responsibility; the router does NOT resolve or merge permissions)
- Filter model tiers by the user's `max_tier` permission
- Select the best tier based on task complexity score (0.0-1.0)
- Apply escalation logic when complexity exceeds available tiers (if permitted)
- Apply budget constraints, downgrading tier if daily/monthly caps would be exceeded
- Select a specific model within the chosen tier using one of 4 strategies
- Implement fallback chain: same tier -> lower tier -> fallback model -> error decision
- Record cost estimates via `update()` for budget tracking

### 1.2 TieredRouter Struct Fields

```rust
pub struct TieredRouter {
    /// Configured model tiers, ordered cheapest to most expensive.
    tiers: Vec<ModelTier>,

    /// Tier name -> ordinal index for O(1) lookup of max_tier boundaries.
    tier_index: HashMap<String, usize>,

    /// Model selection strategy within a tier.
    selection_strategy: TierSelectionStrategy,

    /// Round-robin counters per tier (tier ordinal -> atomic counter).
    round_robin_counters: Vec<AtomicUsize>,

    /// Escalation configuration from routing config.
    escalation_config: EscalationConfig,

    /// Fallback model when all tiers are exhausted or budget-blocked.
    /// Format: "provider/model".
    fallback_model: Option<String>,

    /// Cost tracker for budget enforcement (Phase D provides the real impl;
    /// this phase uses a trait object so the router is testable with mocks).
    cost_tracker: Arc<dyn CostTrackable + Send + Sync>,

    /// Rate limiter for per-user throttling (Phase E provides the real impl;
    /// this phase uses a trait object so the router is testable with mocks).
    rate_limiter: Arc<dyn RateLimitable + Send + Sync>,
}
```

### 1.3 ModelTier Runtime Struct

A runtime representation of a configured tier, enriched with an ordinal position for efficient
comparison. Built from `ModelTierConfig` (Phase A type) during `from_config()`.

```rust
pub struct ModelTier {
    pub name: String,
    pub models: Vec<String>,
    pub complexity_range: [f32; 2],
    pub cost_per_1k_tokens: f64,
    pub max_context_tokens: usize,
    /// Ordinal position in the tier list (0 = cheapest).
    pub ordinal: usize,
}
```

### 1.4 TierSelectionStrategy Enum

How to pick a model within a tier when multiple models are available. This is a runtime enum
parsed from the config string `routing.selection_strategy`.

```rust
pub enum TierSelectionStrategy {
    /// Use the first available model (preference order from config). Default.
    PreferenceOrder,
    /// Round-robin across models in the tier using AtomicUsize counters.
    RoundRobin,
    /// Pick the lowest-cost model in the tier (all models in a tier currently
    /// share the same cost_per_1k_tokens, so this behaves like PreferenceOrder;
    /// future per-model costs would change this).
    LowestCost,
    /// Random selection for load distribution across providers.
    Random,
}
```

### 1.5 Extended RoutingDecision

The existing `RoutingDecision` struct (3 fields: `provider`, `model`, `reason`) is extended
with optional metadata fields for tiered routing. Backward compatibility is maintained --
existing code that constructs `RoutingDecision` with only the 3 core fields must be updated
to include the new fields with default values (`None`, `false`).

New fields:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tier` | `Option<String>` | `None` | Name of the selected tier (None for StaticRouter) |
| `cost_estimate_usd` | `Option<f64>` | `None` | Estimated cost based on tier `cost_per_1k_tokens` |
| `escalated` | `bool` | `false` | Whether escalation was applied to promote beyond max_tier |
| `budget_constrained` | `bool` | `false` | Whether budget forced a tier downgrade |

### 1.6 CostTrackable and RateLimitable Traits

The `TieredRouter` depends on cost tracking (Phase D) and rate limiting (Phase E) via trait
objects. This phase defines the trait interfaces and provides no-op implementations for testing.

```rust
/// Trait for cost budget checking. Real impl in Phase D (CostTracker).
///
/// The trait signature matches Phase D's concrete API so that
/// `impl CostTrackable for CostTracker` is a direct delegation.
pub trait CostTrackable: Send + Sync {
    /// Check whether the estimated cost fits within the sender's daily and monthly limits.
    /// Returns a BudgetResult indicating allowed/denied and remaining budget.
    fn check_budget(&self, sender_id: &str, estimated_cost: f64, daily_limit: f64, monthly_limit: f64) -> BudgetResult;
    /// Record an estimated cost before the LLM call (pre-reservation).
    fn record_estimated(&self, sender_id: &str, estimated_cost: f64);
    /// Reconcile actual cost after response -- adjusts the reservation.
    fn record_actual(&self, sender_id: &str, estimated_cost: f64, actual_cost: f64);
}

/// Trait for rate limiting. Real impl in Phase E (RateLimiter).
///
/// The trait signature matches Phase E's concrete API so that
/// `impl RateLimitable for RateLimiter` is a direct delegation.
pub trait RateLimitable: Send + Sync {
    /// Returns true if the request is allowed, false if rate-limited.
    fn check(&self, sender_id: &str, limit: u32) -> bool;
}
```

No-op implementations (`NoopCostTracker`, `NoopRateLimiter`) are provided for testing and for
configurations that do not need budgets or rate limiting.

### 1.7 Routing Algorithm (8 Steps)

From Section 5.2 of the design doc:

1. **Extract auth context** -- Read `request.auth_context`, default to `AuthContext::default()` (zero-trust)
2. **Read permissions** -- Use the permissions already on the `AuthContext` (resolved upstream by Phase B's `PermissionResolver`). The router does NOT perform permission resolution itself; it reads `request.auth_context.as_ref().map(|a| &a.permissions).unwrap_or(&UserPermissions::default())` directly.
3. **Rate limit check** -- Delegate to `RateLimiter` (Phase E); if over limit, return rate-limited decision
4. **Filter tiers by permissions** -- Remove tiers above user's `max_tier`; maintain ordering cheapest-first
5. **Select tier by complexity** -- Find the highest-quality tier whose `complexity_range` covers the task complexity; apply escalation if needed and permitted
6. **Budget check** -- Query `CostTracker` (Phase D); if over budget, fall back to cheaper tier
7. **Model selection** -- Pick a model from the tier using the configured `TierSelectionStrategy`; apply `model_access` allowlist and `model_denylist` filters
8. **Apply token limits** -- Clamp context/output tokens to user's `max_context_tokens` / `max_output_tokens` (recorded in decision metadata; actual clamping happens downstream)

### 1.8 Tier Selection Logic

- Tiers are ordered cheapest to most expensive in config (ordinal 0 = cheapest)
- Each tier has a `complexity_range: [f32; 2]` (min, max inclusive)
- A task with complexity `c` matches a tier if `tier.complexity_range[0] <= c <= tier.complexity_range[1]`
- Among all matching **allowed** tiers, pick the **highest-quality** (highest ordinal) one
- Complexity ranges intentionally overlap -- e.g., standard covers [0.0, 0.7], premium covers [0.3, 1.0]. A complexity of 0.5 matches both. The router picks the more expensive one if the user is allowed and can afford it.

### 1.9 Escalation Logic

When no allowed tier covers the task complexity AND the user's permissions allow escalation:

1. Check `permissions.escalation_allowed == true`
2. Check `self.escalation_config.enabled == true`
3. Check `complexity > permissions.escalation_threshold`
4. If all three hold, look at tiers above `max_tier` up to `max_escalation_tiers` tiers higher
5. Among those escalation candidates, pick any that match the complexity range
6. If a match is found, return it with `escalated = true`
7. If no escalation candidate matches (or escalation is not permitted), fall back to the highest allowed tier

The `max_escalation_tiers` field (from `EscalationConfig`, default: 1) prevents unbounded
promotion. A Level 1 user with `max_tier = "standard"` can escalate at most to `premium`
(one tier above), never to `elite`.

### 1.10 Fallback Chain

```
Selected model in tier
  -> unavailable? Next model in same tier (preference order)
  -> all unavailable? Same-complexity model in next lower tier
  -> all unavailable? config.routing.fallback_model
  -> unavailable? Return decision with empty provider/model and explanatory reason
```

This chain integrates with the existing `FailoverController` in `clawft-llm`. The `TieredRouter`
selects the target; the transport layer handles provider-level failover within that selection.

### 1.11 Thread Safety

- `TieredRouter` is `Send + Sync` (required by `ModelRouter` trait bounds)
- All configuration fields are immutable after construction
- Round-robin counters use `AtomicUsize` with `Ordering::Relaxed` (occasional skips under concurrency are acceptable for load distribution)
- Shared state (`CostTracker`, `RateLimiter`) is behind `Arc<dyn Trait>`
- No `Mutex` in the hot path; all operations are lock-free

---

## 2. Pseudocode

### 2.1 Imports and Module Header

```rust
//! Tiered router (Level 1 implementation).
//!
//! Selects models based on task complexity, user permissions, and cost
//! budgets. Implements the `ModelRouter` trait as a drop-in replacement
//! for `StaticRouter` when `routing.mode == "tiered"`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;
use tracing;

// FIX-01: All routing types imported from clawft_types::routing (Phase A canonical location).
// NOT from clawft_types::auth -- that module does not exist.
use clawft_types::routing::{
    AuthContext, EscalationConfig, ModelTierConfig, PermissionsConfig,
    RoutingConfig, UserPermissions,
};

use super::traits::{
    ChatRequest, ModelRouter, ResponseOutcome, RoutingDecision, TaskProfile,
};
```

### 2.2 ModelTier Runtime Struct

```rust
/// Runtime representation of a model tier with routing state.
///
/// Built from `ModelTierConfig` during `TieredRouter::from_config()`.
/// The `ordinal` field tracks position in the cheapest-to-most-expensive
/// ordering for efficient comparison.
#[derive(Debug, Clone)]
pub struct ModelTier {
    /// Tier name (e.g., "free", "standard", "premium", "elite").
    pub name: String,
    /// Models available in this tier, in preference order.
    /// Format: "provider/model" (e.g., "anthropic/claude-haiku-3.5").
    pub models: Vec<String>,
    /// Complexity range this tier covers: [min, max] (both inclusive).
    pub complexity_range: [f32; 2],
    /// Approximate cost per 1K tokens (blended input/output) in USD.
    pub cost_per_1k_tokens: f64,
    /// Maximum context tokens supported by models in this tier.
    pub max_context_tokens: usize,
    /// Ordinal position in the tier list (0 = cheapest).
    pub ordinal: usize,
}

impl ModelTier {
    /// Returns true if the given complexity falls within this tier's range (inclusive).
    pub fn matches_complexity(&self, complexity: f32) -> bool {
        complexity >= self.complexity_range[0] && complexity <= self.complexity_range[1]
    }
}
```

### 2.3 TierSelectionStrategy Enum

```rust
/// How to pick a model within a tier when multiple are available.
#[derive(Debug, Clone, Default)]
pub enum TierSelectionStrategy {
    /// Use the first available model (preference order from config).
    #[default]
    PreferenceOrder,
    /// Round-robin across models in the tier.
    RoundRobin,
    /// Pick the lowest-cost model in the tier.
    LowestCost,
    /// Pick randomly (useful for load distribution across providers).
    Random,
}

impl TierSelectionStrategy {
    /// Parse from a config string. Unknown values default to PreferenceOrder.
    pub fn from_config_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "round_robin" | "roundrobin" => Self::RoundRobin,
            "lowest_cost" | "lowestcost" => Self::LowestCost,
            "random" => Self::Random,
            _ => Self::PreferenceOrder,
        }
    }
}
```

### 2.4 CostTrackable and RateLimitable Traits + No-op Impls

```rust
/// Trait for cost budget checking. Real impl in Phase D (CostTracker).
///
/// FIX-03: Signature matches Phase D's concrete API:
///   - check_budget() takes sender_id, estimated_cost, daily_limit, monthly_limit and returns BudgetResult
///   - record_estimated() takes sender_id and estimated_cost
///   - record_actual() takes sender_id, estimated_cost, and actual_cost for reconciliation
pub trait CostTrackable: Send + Sync {
    fn check_budget(&self, sender_id: &str, estimated_cost: f64, daily_limit: f64, monthly_limit: f64) -> BudgetResult;
    fn record_estimated(&self, sender_id: &str, estimated_cost: f64);
    fn record_actual(&self, sender_id: &str, estimated_cost: f64, actual_cost: f64);
}

/// Result of a budget check. Defined here as the trait return type;
/// Phase D re-exports or imports this.
#[derive(Debug, Clone)]
pub struct BudgetResult {
    /// Whether the request is allowed within budget.
    pub allowed: bool,
    /// Remaining daily budget in USD after this request (if allowed).
    pub remaining_daily_usd: f64,
    /// Remaining monthly budget in USD after this request (if allowed).
    pub remaining_monthly_usd: f64,
    /// Human-readable reason when denied.
    pub reason: Option<String>,
}

/// Trait for rate limiting. Real impl in Phase E (RateLimiter).
///
/// FIX-03: Signature matches Phase E's concrete API:
///   - check() takes sender_id and limit, returns bool
pub trait RateLimitable: Send + Sync {
    fn check(&self, sender_id: &str, limit: u32) -> bool;
}

/// No-op cost tracker for when budget enforcement is disabled.
pub struct NoopCostTracker;
impl CostTrackable for NoopCostTracker {
    fn check_budget(&self, _sender_id: &str, _estimated_cost: f64, _daily_limit: f64, _monthly_limit: f64) -> BudgetResult {
        BudgetResult { allowed: true, remaining_daily_usd: f64::MAX, remaining_monthly_usd: f64::MAX, reason: None }
    }
    fn record_estimated(&self, _sender_id: &str, _estimated_cost: f64) {}
    fn record_actual(&self, _sender_id: &str, _estimated_cost: f64, _actual_cost: f64) {}
}

/// No-op rate limiter for when rate limiting is disabled.
pub struct NoopRateLimiter;
impl RateLimitable for NoopRateLimiter {
    fn check(&self, _sender_id: &str, _limit: u32) -> bool { true }
}
```

### 2.5 TieredRouter Constructor: `from_config()`

```rust
impl TieredRouter {
    /// Create a TieredRouter from routing configuration.
    ///
    /// The `cost_tracker` and `rate_limiter` are injected as trait objects
    /// so the router is testable independently of Phases D and E.
    pub fn from_config(
        config: &RoutingConfig,
        cost_tracker: Arc<dyn CostTrackable + Send + Sync>,
        rate_limiter: Arc<dyn RateLimitable + Send + Sync>,
    ) -> Self {
        // Build runtime tier list from config, preserving order (cheapest first)
        // FIX-12: Access t.max_context_tokens directly (not unwrap_or).
        // ModelTierConfig.max_context_tokens is a required field with a serde default,
        // so it is always present.
        let tiers: Vec<ModelTier> = config.tiers.iter().enumerate().map(|(i, t)| {
            ModelTier {
                name: t.name.clone(),
                models: t.models.clone(),
                complexity_range: t.complexity_range,
                cost_per_1k_tokens: t.cost_per_1k_tokens,
                max_context_tokens: t.max_context_tokens,
                ordinal: i,
            }
        }).collect();

        // Build tier name -> ordinal index for O(1) lookup
        let tier_index: HashMap<String, usize> = tiers.iter()
            .map(|t| (t.name.clone(), t.ordinal))
            .collect();

        // Build round-robin counters (one per tier)
        let round_robin_counters: Vec<AtomicUsize> = (0..tiers.len())
            .map(|_| AtomicUsize::new(0))
            .collect();

        // Parse selection strategy from config string
        let selection_strategy = config.selection_strategy.as_deref()
            .map(TierSelectionStrategy::from_config_str)
            .unwrap_or_default();

        // FIX-02: No permission_defaults, user_overrides, or channel_overrides fields.
        // Permission resolution is entirely Phase B's responsibility. The router reads
        // pre-resolved permissions from request.auth_context.permissions directly.

        Self {
            tiers,
            tier_index,
            selection_strategy,
            round_robin_counters,
            escalation_config: config.escalation.clone(),
            fallback_model: config.fallback_model.clone(),
            cost_tracker,
            rate_limiter,
        }
    }

    /// Create a TieredRouter with no-op cost tracker and rate limiter.
    /// Useful for testing and for configurations that do not need budgets.
    pub fn from_config_simple(config: &RoutingConfig) -> Self {
        Self::from_config(
            config,
            Arc::new(NoopCostTracker),
            Arc::new(NoopRateLimiter),
        )
    }

    // FIX-02: build_permission_defaults(), build_user_overrides(), and
    // build_channel_overrides() have been REMOVED. Permission resolution is
    // entirely Phase B's responsibility. The router reads pre-resolved
    // permissions from request.auth_context.permissions directly.
}
```

### 2.6 Permission Reading (no resolution in the router)

```rust
// FIX-02: resolve_permissions() and merge_permissions() have been REMOVED from TieredRouter.
// All permission resolution happens in Phase B's PermissionResolver, upstream of the router.
// The router reads pre-resolved permissions directly from the AuthContext.
//
// Usage in route():
//   let permissions = request.auth_context
//       .as_ref()
//       .map(|a| &a.permissions)
//       .unwrap_or(&UserPermissions::default());
//
// This is a simple read, not a resolution step. If auth_context is None (no upstream
// resolver ran), the router falls back to UserPermissions::default() which is zero-trust.
```

### 2.7 Tier Filtering by Permissions

```rust
impl TieredRouter {
    /// Filter tiers to only those the user is allowed to access.
    ///
    /// Returns tiers at or below the user's `max_tier`, preserving
    /// the cheapest-first ordering from config. The `tier_index` HashMap
    /// provides O(1) lookup of the max_tier boundary ordinal.
    fn filter_tiers_by_permissions<'a>(
        &'a self,
        permissions: &UserPermissions,
    ) -> Vec<&'a ModelTier> {
        let max_ordinal = self.tier_index
            .get(&permissions.max_tier)
            .copied()
            .unwrap_or(0); // Unknown tier name -> cheapest only

        self.tiers.iter()
            .filter(|t| t.ordinal <= max_ordinal)
            .collect()
    }
}
```

### 2.8 Tier Selection with Escalation

```rust
impl TieredRouter {
    /// Select the best tier for the given complexity, respecting permissions.
    ///
    /// Algorithm:
    /// 1. Find all allowed tiers whose complexity_range covers the task complexity.
    /// 2. Among matches, pick the highest-quality (highest ordinal) tier.
    /// 3. If no match, check escalation eligibility.
    /// 4. If escalation allowed, try tiers above max_tier (up to max_escalation_tiers).
    /// 5. If still no match, fall back to the highest allowed tier.
    fn select_tier<'a>(
        &'a self,
        complexity: f32,
        allowed_tiers: &[&'a ModelTier],
        permissions: &UserPermissions,
    ) -> TierSelection<'a> {
        // Step 1: Find matching tiers within permission boundary
        let matching: Vec<&&ModelTier> = allowed_tiers.iter()
            .filter(|t| t.matches_complexity(complexity))
            .collect();

        // Step 2: Pick the highest-quality match
        if let Some(best) = matching.iter().max_by_key(|t| t.ordinal) {
            return TierSelection { tier: best, escalated: false };
        }

        // Step 3: No allowed tier matches -- try escalation
        if permissions.escalation_allowed
            && self.escalation_config.enabled
            && complexity > permissions.escalation_threshold
        {
            let max_ordinal = self.tier_index
                .get(&permissions.max_tier)
                .copied()
                .unwrap_or(0);

            // FIX-12: Access max_escalation_tiers directly (not unwrap_or).
            // EscalationConfig.max_escalation_tiers is a required field with a
            // serde default of 1, so it is always present.
            let max_escalation = self.escalation_config.max_escalation_tiers;

            // Look at tiers above the user's max_tier, up to max_escalation_tiers
            let escalation_candidates: Vec<&ModelTier> = self.tiers.iter()
                .filter(|t| {
                    t.ordinal > max_ordinal
                        && t.ordinal <= max_ordinal + max_escalation as usize
                        && t.matches_complexity(complexity)
                })
                .collect();

            if let Some(best) = escalation_candidates.iter().max_by_key(|t| t.ordinal) {
                // FIX-12: Audit log at escalation decision point.
                tracing::info!(
                    user_max_tier = %permissions.max_tier,
                    escalated_to = %best.name,
                    complexity = %complexity,
                    "escalation applied: promoting beyond max_tier"
                );
                return TierSelection { tier: best, escalated: true };
            }
        }

        // Step 4: No match even with escalation -- fall back to highest allowed tier
        if let Some(highest) = allowed_tiers.iter().max_by_key(|t| t.ordinal) {
            return TierSelection { tier: highest, escalated: false };
        }

        // Should not reach here if tiers list is non-empty, but handle gracefully
        TierSelection { tier: &self.tiers[0], escalated: false }
    }
}

/// Internal result of tier selection.
struct TierSelection<'a> {
    tier: &'a ModelTier,
    escalated: bool,
}
```

### 2.9 Budget Constraints

```rust
impl TieredRouter {
    /// Apply budget constraints to the selected tier.
    ///
    /// If the estimated cost of the selected tier would push the user over
    /// their daily budget, fall back to cheaper tiers until one fits or all
    /// tiers are exhausted.
    /// FIX-03: Uses Phase D's estimate_cost() for cost estimation and passes
    /// permissions.cost_budget_daily_usd and permissions.cost_budget_monthly_usd
    /// to check_budget(). FIX-12: Uses Phase D's estimate_cost() instead of raw
    /// cost_per_1k_tokens; adds tracing audit log at budget decision points.
    fn apply_budget_constraints<'a>(
        &'a self,
        selected: &'a ModelTier,
        allowed_tiers: &[&'a ModelTier],
        auth: &AuthContext,
        permissions: &UserPermissions,
    ) -> TierBudgetResult<'a> {
        // If budget is unlimited (0.0), skip checking
        if permissions.cost_budget_daily_usd <= 0.0 && permissions.cost_budget_monthly_usd <= 0.0 {
            return TierBudgetResult { tier: selected, constrained: false };
        }

        // FIX-12: Use Phase D's estimate_cost() for cost estimation instead of
        // raw cost_per_1k_tokens. Phase D provides estimate_cost(tier_name, estimated_tokens)
        // which accounts for per-model pricing. For Phase C standalone, we approximate
        // with cost_per_1k_tokens as the Phase D function is not yet available.
        let estimated_cost = selected.cost_per_1k_tokens; // TODO: replace with estimate_cost() when Phase D is integrated

        // FIX-03: Pass daily and monthly limits to check_budget()
        let budget_check = self.cost_tracker.check_budget(
            &auth.sender_id,
            estimated_cost,
            permissions.cost_budget_daily_usd,
            permissions.cost_budget_monthly_usd,
        );

        if budget_check.allowed {
            return TierBudgetResult { tier: selected, constrained: false };
        }

        // FIX-12: Audit log when budget constraint triggers a downgrade.
        tracing::info!(
            user = %auth.sender_id,
            selected_tier = %selected.name,
            estimated_cost = %estimated_cost,
            daily_limit = %permissions.cost_budget_daily_usd,
            monthly_limit = %permissions.cost_budget_monthly_usd,
            "budget constraint triggered: attempting tier downgrade"
        );

        // Budget would be exceeded -- fall back to cheaper tiers
        let mut candidates: Vec<&&ModelTier> = allowed_tiers.iter()
            .filter(|t| t.ordinal < selected.ordinal)
            .collect();
        candidates.sort_by(|a, b| b.ordinal.cmp(&a.ordinal)); // highest first

        for candidate in candidates {
            let candidate_cost = candidate.cost_per_1k_tokens; // TODO: replace with estimate_cost()
            let candidate_check = self.cost_tracker.check_budget(
                &auth.sender_id,
                candidate_cost,
                permissions.cost_budget_daily_usd,
                permissions.cost_budget_monthly_usd,
            );
            if candidate_check.allowed {
                return TierBudgetResult { tier: candidate, constrained: true };
            }
        }

        // No tier fits -- use the cheapest tier anyway (overage is recorded)
        if let Some(cheapest) = allowed_tiers.iter().min_by_key(|t| t.ordinal) {
            return TierBudgetResult { tier: cheapest, constrained: true };
        }

        TierBudgetResult { tier: selected, constrained: true }
    }
}

/// Internal result of budget constraint evaluation.
/// Named TierBudgetResult to avoid collision with the BudgetResult from CostTrackable.
struct TierBudgetResult<'a> {
    tier: &'a ModelTier,
    constrained: bool,
}
```

### 2.10 Model Selection Within a Tier

```rust
impl TieredRouter {
    /// Select a specific model from the tier's model list.
    ///
    /// Applies the configured selection strategy and filters by the user's
    /// model_access allowlist and model_denylist.
    fn select_model(
        &self,
        tier: &ModelTier,
        permissions: &UserPermissions,
    ) -> Option<(String, String)> {
        // Filter models by user's allowlist/denylist
        let available = self.filter_models_by_permissions(&tier.models, permissions);
        if available.is_empty() {
            return None;
        }

        let selected = match &self.selection_strategy {
            TierSelectionStrategy::PreferenceOrder => {
                available[0].clone()
            }
            TierSelectionStrategy::RoundRobin => {
                let counter = &self.round_robin_counters[tier.ordinal];
                let idx = counter.fetch_add(1, Ordering::Relaxed) % available.len();
                available[idx].clone()
            }
            TierSelectionStrategy::LowestCost => {
                // Within a tier all models share cost_per_1k_tokens;
                // just pick the first one. Per-model costs are a future extension.
                available[0].clone()
            }
            TierSelectionStrategy::Random => {
                let idx = rand::thread_rng().gen_range(0..available.len());
                available[idx].clone()
            }
        };

        Some(split_provider_model(&selected))
    }

    /// Filter a tier's model list against user permission allowlist/denylist.
    fn filter_models_by_permissions(
        &self,
        models: &[String],
        permissions: &UserPermissions,
    ) -> Vec<String> {
        models.iter().filter(|m| {
            // Allowlist check (empty = all allowed)
            let allowed = permissions.model_access.is_empty()
                || permissions.model_access.iter().any(|p| model_matches_pattern(m, p));
            // Denylist check
            let denied = permissions.model_denylist.iter().any(|p| model_matches_pattern(m, p));
            allowed && !denied
        }).cloned().collect()
    }
}

/// Check if a model name matches a glob-style pattern.
/// Supports exact match, prefix wildcard ("anthropic/*"), full wildcard ("*").
fn model_matches_pattern(model: &str, pattern: &str) -> bool {
    if pattern == "*" { return true; }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return model.starts_with(prefix);
    }
    model == pattern
}

/// Split a "provider/model" string into (provider, model).
/// If no slash, defaults provider to "openai".
fn split_provider_model(s: &str) -> (String, String) {
    if let Some(idx) = s.find('/') {
        (s[..idx].to_string(), s[idx + 1..].to_string())
    } else {
        ("openai".to_string(), s.to_string())
    }
}
```

### 2.11 Fallback Chain

```rust
impl TieredRouter {
    /// Execute the fallback chain when the primary tier's models are all unavailable.
    ///
    /// Chain: lower tiers (descending quality) -> fallback_model -> None
    ///
    /// FIX-06: The fallback chain verifies that any fallback model belongs to a tier
    /// at or below the user's max_tier. If the fallback_model would be in a tier above
    /// the user's permission level, it is NOT returned -- instead, return None (error
    /// decision) to prevent unauthorized model access.
    fn fallback_chain(
        &self,
        primary_tier: &ModelTier,
        allowed_tiers: &[&ModelTier],
        permissions: &UserPermissions,
    ) -> Option<(String, String, String)> {
        let max_ordinal = self.tier_index
            .get(&permissions.max_tier)
            .copied()
            .unwrap_or(0);

        // 1. Try lower tiers in descending quality order
        let mut lower_tiers: Vec<&&ModelTier> = allowed_tiers.iter()
            .filter(|t| t.ordinal < primary_tier.ordinal)
            .collect();
        lower_tiers.sort_by(|a, b| b.ordinal.cmp(&a.ordinal));

        for tier in lower_tiers {
            // FIX-06: Only consider tiers at or below user's max_tier
            if tier.ordinal > max_ordinal {
                continue;
            }
            if let Some((provider, model)) = self.select_model(tier, permissions) {
                let reason = format!(
                    "fallback from tier '{}' to tier '{}'",
                    primary_tier.name, tier.name
                );
                tracing::info!(
                    from_tier = %primary_tier.name,
                    to_tier = %tier.name,
                    "fallback chain: using lower tier"
                );
                return Some((provider, model, reason));
            }
        }

        // 2. Try the global fallback model -- but only if it belongs to a permitted tier
        if let Some(ref fallback) = self.fallback_model {
            // FIX-06: Check if the fallback model belongs to a tier at or below max_tier.
            // Search all tiers for the fallback model; if found in a tier above max_tier,
            // do not use it.
            let fallback_tier_ordinal = self.tiers.iter()
                .find(|t| t.models.iter().any(|m| m == fallback))
                .map(|t| t.ordinal);

            match fallback_tier_ordinal {
                Some(ordinal) if ordinal > max_ordinal => {
                    // Fallback model is in a tier above user's permission level.
                    tracing::info!(
                        fallback_model = %fallback,
                        fallback_tier_ordinal = %ordinal,
                        user_max_ordinal = %max_ordinal,
                        "fallback chain: fallback_model denied -- tier above user max_tier"
                    );
                    // Do NOT return the fallback -- fall through to None
                }
                _ => {
                    // Fallback model is in a permitted tier (or not in any tier, which
                    // we treat as allowed for backward compatibility with models that
                    // are configured only as fallback_model without a tier entry).
                    let (provider, model) = split_provider_model(fallback);
                    let reason = format!("fallback to configured fallback_model '{}'", fallback);
                    tracing::info!(fallback_model = %fallback, "fallback chain: using fallback_model");
                    return Some((provider, model, reason));
                }
            }
        }

        // 3. No models available
        None
    }
}
```

### 2.12 Rate-Limited and No-Tiers Decision Helpers

```rust
impl TieredRouter {
    /// Build a RoutingDecision for rate-limited requests.
    ///
    /// FIX-06: Verifies that the fallback model belongs to a tier at or below
    /// the user's max_tier before returning it. If the fallback model would be
    /// in a higher tier, returns an error decision instead.
    fn rate_limited_decision(&self, permissions: &UserPermissions) -> RoutingDecision {
        let max_ordinal = self.tier_index
            .get(&permissions.max_tier)
            .copied()
            .unwrap_or(0);

        if let Some(ref fallback) = self.fallback_model {
            // FIX-06: Check fallback model tier permission
            let fallback_tier_ordinal = self.tiers.iter()
                .find(|t| t.models.iter().any(|m| m == fallback))
                .map(|t| t.ordinal);

            if let Some(ordinal) = fallback_tier_ordinal {
                if ordinal > max_ordinal {
                    tracing::info!(
                        fallback_model = %fallback,
                        fallback_tier_ordinal = %ordinal,
                        user_max_ordinal = %max_ordinal,
                        "rate_limited_decision: fallback_model denied -- tier above user max_tier"
                    );
                    return RoutingDecision {
                        provider: String::new(), model: String::new(),
                        reason: "rate limited: fallback model not permitted for user tier".into(),
                        ..Default::default()
                    };
                }
            }

            let (provider, model) = split_provider_model(fallback);
            RoutingDecision {
                provider, model,
                reason: "rate limited: using fallback model".into(),
                ..Default::default()
            }
        } else {
            RoutingDecision {
                provider: String::new(), model: String::new(),
                reason: "rate limited: no fallback model configured".into(),
                budget_constrained: true,
                ..Default::default()
            }
        }
    }

    /// Build a RoutingDecision when no tiers are available at all.
    /// FIX-10: Uses ..Default::default() for new RoutingDecision fields.
    fn no_tiers_available_decision(&self) -> RoutingDecision {
        if let Some(ref fallback) = self.fallback_model {
            let (provider, model) = split_provider_model(fallback);
            RoutingDecision {
                provider, model,
                reason: "no tiers available: using fallback model".into(),
                ..Default::default()
            }
        } else {
            RoutingDecision {
                provider: String::new(), model: String::new(),
                reason: "no tiers or fallback model available".into(),
                ..Default::default()
            }
        }
    }
}
```

### 2.13 ModelRouter Trait Implementation: `route()`

```rust
#[async_trait]
impl ModelRouter for TieredRouter {
    async fn route(
        &self,
        request: &ChatRequest,
        profile: &TaskProfile,
    ) -> RoutingDecision {
        // ── Step 1: Extract auth context ──────────────────────────────
        let auth = request.auth_context.as_ref()
            .cloned()
            .unwrap_or_default(); // zero-trust if absent

        // ── Step 2: Read pre-resolved permissions ─────────────────────
        // FIX-02: No resolution in the router. Permissions are resolved
        // upstream by Phase B's PermissionResolver and attached to AuthContext.
        // The router reads them directly. If auth_context is None, we get
        // zero-trust defaults.
        let default_permissions = UserPermissions::default();
        let permissions = request.auth_context
            .as_ref()
            .map(|a| &a.permissions)
            .unwrap_or(&default_permissions);

        // FIX-12: Audit log at route entry.
        tracing::info!(
            sender = %auth.sender_id,
            channel = %auth.channel,
            complexity = %profile.complexity,
            max_tier = %permissions.max_tier,
            level = %permissions.level,
            "route: starting tiered routing decision"
        );

        // ── Step 3: Check rate limit ──────────────────────────────────
        if permissions.rate_limit > 0
            && !self.rate_limiter.check(&auth.sender_id, permissions.rate_limit)
        {
            tracing::info!(sender = %auth.sender_id, "route: rate limited");
            return self.rate_limited_decision(permissions);
        }

        // ── Step 4: Filter tiers by permission level ──────────────────
        let allowed_tiers = self.filter_tiers_by_permissions(permissions);
        if allowed_tiers.is_empty() {
            tracing::info!(sender = %auth.sender_id, "route: no tiers available");
            return self.no_tiers_available_decision();
        }

        // ── Step 5: Select tier by complexity (with escalation) ───────
        let tier_selection = self.select_tier(
            profile.complexity,
            &allowed_tiers,
            permissions,
        );

        // ── Step 6: Apply budget constraints ──────────────────────────
        let budget_result = self.apply_budget_constraints(
            tier_selection.tier,
            &allowed_tiers,
            &auth,
            permissions,
        );

        let final_tier = budget_result.tier;
        let escalated = tier_selection.escalated;
        let budget_constrained = budget_result.constrained;

        // ── Step 7: Select model from tier ────────────────────────────
        let (provider, model) = match self.select_model(final_tier, permissions) {
            Some(pm) => pm,
            None => {
                // No models available in selected tier -- try fallback chain
                match self.fallback_chain(final_tier, &allowed_tiers, permissions) {
                    Some((p, m, reason)) => {
                        return RoutingDecision {
                            provider: p, model: m, reason,
                            tier: Some(final_tier.name.clone()),
                            cost_estimate_usd: Some(final_tier.cost_per_1k_tokens),
                            escalated, budget_constrained,
                            ..Default::default()
                        };
                    }
                    None => return self.no_tiers_available_decision(),
                }
            }
        };

        // ── Step 8: Record estimated cost + build decision ────────────
        // FIX-03: Use record_estimated() (renamed from record_estimated_cost)
        let cost_estimate = final_tier.cost_per_1k_tokens;
        self.cost_tracker.record_estimated(&auth.sender_id, cost_estimate);

        // FIX-12: Audit log at route completion.
        tracing::info!(
            sender = %auth.sender_id,
            provider = %provider,
            model = %model,
            tier = %final_tier.name,
            cost_estimate_usd = %cost_estimate,
            escalated = %escalated,
            budget_constrained = %budget_constrained,
            "route: decision made"
        );

        RoutingDecision {
            provider, model,
            reason: format!(
                "tiered routing: complexity={:.2}, tier={}, level={}, user={}",
                profile.complexity, final_tier.name, permissions.level, auth.sender_id,
            ),
            tier: Some(final_tier.name.clone()),
            cost_estimate_usd: Some(cost_estimate),
            escalated,
            budget_constrained,
        }
    }

    fn update(&self, decision: &RoutingDecision, _outcome: &ResponseOutcome) {
        // FIX-03: Use record_actual() (renamed from record_actual_cost).
        // Record actual cost after response is received.
        // Phase D (CostTracker) will provide real cost calculation
        // from clawft-llm's ModelCatalog. For Phase C, we use the
        // cost estimate as a proxy for both estimated and actual.
        if let Some(cost) = decision.cost_estimate_usd {
            // NOTE: sender_id is not directly available on RoutingDecision.
            // Phase F will thread sender_id through the pipeline properly.
            // For Phase C, the CostTracker's record_actual() is called with
            // the cost estimate as both estimated and actual.
            // self.cost_tracker.record_actual(sender_id, cost, cost);
            let _ = cost;
        }
    }
}
```

### 2.14 Debug Implementation

```rust
// FIX-02: Removed permission_levels, user_overrides_count, channel_overrides_count
// from Debug output -- those fields no longer exist on TieredRouter.
impl std::fmt::Debug for TieredRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TieredRouter")
            .field("tiers", &self.tiers.iter().map(|t| &t.name).collect::<Vec<_>>())
            .field("selection_strategy", &self.selection_strategy)
            .field("escalation_config", &self.escalation_config)
            .field("fallback_model", &self.fallback_model)
            .finish()
    }
}
```

---

## 3. Architecture

### 3.1 File Locations

| Component | File | Action |
|-----------|------|--------|
| `TieredRouter`, `ModelTier`, `TierSelectionStrategy`, `CostTrackable`, `BudgetResult`, `RateLimitable`, `NoopCostTracker`, `NoopRateLimiter` | `crates/clawft-core/src/pipeline/tiered_router.rs` | **New file** |
| Module declaration | `crates/clawft-core/src/pipeline/mod.rs` | **Edit**: add `pub mod tiered_router;` |
| `RoutingDecision` (extend with 4 fields) | `crates/clawft-core/src/pipeline/traits.rs` | **Edit**: add `tier`, `cost_estimate_usd`, `escalated`, `budget_constrained` |
| `ChatRequest` (add `auth_context`) | `crates/clawft-core/src/pipeline/traits.rs` | **Edit**: add `auth_context: Option<AuthContext>` |
| `StaticRouter` constructions (add new field defaults) | `crates/clawft-core/src/pipeline/router.rs` | **Edit**: update all `RoutingDecision` literals |
| `rand` dependency | `crates/clawft-core/Cargo.toml` | **Edit**: add `rand = "0.8"` |

### 3.2 Dependency Graph

```
clawft-types (Phase A)
  +-- RoutingConfig, ModelTierConfig, EscalationConfig, PermissionsConfig
  +-- UserPermissions, AuthContext

clawft-core/pipeline/permissions.rs (Phase B)
  +-- PermissionResolver (resolves AuthContext upstream)
  +-- UserPermissions::merge(), defaults_for_level()

clawft-core/pipeline/tiered_router.rs (THIS PHASE)
  +-- imports: traits.rs (ModelRouter, RoutingDecision, TaskProfile, ChatRequest)
  +-- imports: clawft_types::routing (FIX-01: canonical import path)
  +-- defines: TieredRouter, ModelTier, TierSelectionStrategy
  +-- defines: CostTrackable, RateLimitable traits (interfaces for Phase D, E)
  +-- defines: BudgetResult (FIX-03: return type for CostTrackable::check_budget)
  +-- defines: NoopCostTracker, NoopRateLimiter (stub impls for testing)
  +-- implements: ModelRouter for TieredRouter
  +-- NOTE: Does NOT resolve permissions (FIX-02: reads pre-resolved from AuthContext)
  v
clawft-core/pipeline/cost_tracker.rs (Phase D -- future)
  +-- implements: CostTrackable for CostTracker (DashMap-based)
  v
clawft-core/pipeline/rate_limiter.rs (Phase E -- future)
  +-- implements: RateLimitable for RateLimiter (sliding-window)

clawft-core/pipeline/router.rs (existing, updated)
  +-- StaticRouter remains as Level 0 default (unchanged logic)
  +-- RoutingDecision literals updated with new field defaults
```

### 3.3 Integration with PipelineRegistry

The `PipelineRegistry` (in `traits.rs`) holds `Pipeline` structs containing
`router: Arc<dyn ModelRouter>`. The `TieredRouter` plugs in as a drop-in replacement:

```rust
// In AgentLoop initialization (Phase F handles the actual wiring)
let router: Arc<dyn ModelRouter> = match config.routing.mode.as_str() {
    "tiered" => Arc::new(TieredRouter::from_config(
        &config.routing,
        cost_tracker.clone(),
        rate_limiter.clone(),
    )),
    _ => Arc::new(StaticRouter::from_config(&config.agents)),
};
```

The `PipelineRegistry::complete()` method does not change. It calls
`pipeline.router.route(request, &profile)` regardless of which router implementation
is behind the trait.

### 3.4 ChatRequest Extension

The `ChatRequest` struct in `traits.rs` gains an optional `auth_context` field:

```rust
pub struct ChatRequest {
    pub messages: Vec<LlmMessage>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub temperature: Option<f64>,

    /// Authentication context for permission-gated routing.
    /// None = zero-trust defaults.
    ///
    /// FIX-05: skip_deserializing prevents JSON injection via gateway API.
    /// AuthContext is set server-side ONLY by channel plugins and AgentLoop.
    /// It is a trusted field populated by the framework, not user input.
    ///
    /// FIX-09: Phase C owns this ChatRequest extension. All existing
    /// construction sites must be updated with `auth_context: None`.
    #[serde(default, skip_deserializing)]
    pub auth_context: Option<AuthContext>,
}
```

This requires importing `AuthContext` from `clawft_types::routing` in `traits.rs` (FIX-01:
`clawft_types::routing`, NOT `clawft_types::auth`). Existing code that constructs `ChatRequest`
without `auth_context` must add the field (set to `None`). The `#[serde(skip_deserializing)]`
attribute prevents clients from injecting an `AuthContext` via JSON -- only server-side code
(channel plugins, AgentLoop) may set this field programmatically.

### 3.5 RoutingDecision Extension

```rust
/// FIX-10: Derives Default so existing construction sites can use
/// `..Default::default()` instead of listing all new fields explicitly.
/// Default values: tier=None, cost_estimate_usd=None, escalated=false,
/// budget_constrained=false, provider="", model="", reason="".
#[derive(Debug, Clone, Default)]
pub struct RoutingDecision {
    pub provider: String,
    pub model: String,
    pub reason: String,

    /// Tier that was selected (None for StaticRouter).
    pub tier: Option<String>,
    /// Estimated cost for this request in USD.
    pub cost_estimate_usd: Option<f64>,
    /// Whether escalation was applied.
    pub escalated: bool,
    /// Whether the decision was constrained by budget.
    pub budget_constrained: bool,
}
```

FIX-10: All existing `RoutingDecision` constructions in `router.rs` (StaticRouter) and test code
should use `..Default::default()` to fill the new fields. This is less error-prone than listing
all four new fields explicitly, and will automatically pick up any future field additions.

Example migration for StaticRouter:
```rust
// Before:
RoutingDecision { provider, model, reason }
// After:
RoutingDecision { provider, model, reason, ..Default::default() }
```

### 3.6 Thread Safety Design

| Component | Mechanism | Rationale |
|-----------|-----------|-----------|
| `TieredRouter` config fields | Immutable after construction | Tiers, permissions, config never change at runtime |
| `round_robin_counters` | `Vec<AtomicUsize>` | Lock-free increment, `Ordering::Relaxed` sufficient |
| `cost_tracker` | `Arc<dyn CostTrackable>` | Shared across pipeline; Phase D impl uses DashMap |
| `rate_limiter` | `Arc<dyn RateLimitable>` | Shared across pipeline; Phase E impl uses DashMap |
| `ModelRouter` trait | `Send + Sync` required | Enforced at compile time by trait bounds |

### 3.7 Fallback Chain Integration

The fallback chain within `TieredRouter` handles model-level fallback (which model within the
routing tier to try). This is distinct from the provider-level failover in `clawft-llm`'s
`FailoverController`, which handles HTTP errors, timeouts, and API key failures after the
routing decision is made. The two layers compose:

```
TieredRouter::route()           -- selects tier + model
  |
  v  (if no models in tier)
TieredRouter::fallback_chain()  -- tries lower tiers, then fallback_model
  |
  v  RoutingDecision(provider, model)
PipelineRegistry::complete()
  |
  v
LlmTransport::complete()
  |
  v  (if HTTP error / timeout)
FailoverController              -- retries with alternate provider endpoints
```

### 3.8 Cargo.toml Dependency Addition

Add to `crates/clawft-core/Cargo.toml` under `[dependencies]`:

```toml
rand = "0.8"
tracing = "0.1"
```

The `async-trait`, `serde`, `serde_json`, and `clawft-types` dependencies are already present.
FIX-12: `tracing` added for structured audit logging at key decision points.

---

## 4. Refinement

### 4.1 Edge Cases

| Edge Case | Expected Behavior | Test Required |
|-----------|-------------------|---------------|
| Empty tiers list in config | `from_config` creates router with zero tiers; all routes go to `fallback_model`; if no fallback, returns empty decision | Yes |
| No tier matches complexity and no escalation | Highest allowed tier is used as fallback (it still handles the request, just at potentially lower quality) | Yes |
| All models in selected tier unavailable (filtered by denylist) | Fallback chain fires: lower tier -> fallback model -> empty decision | Yes |
| Unknown `max_tier` name in permissions | `tier_index.get()` returns `None`, `max_ordinal` defaults to 0 (cheapest tier only) | Yes |
| Complexity exactly at tier boundary (0.3 or 0.7) | `matches_complexity` uses `<=` for both ends -- complexity 0.3 matches both `free` [0.0, 0.3] and `premium` [0.3, 1.0] | Yes |
| Complexity = 0.0 | Matches any tier with `complexity_range[0] == 0.0` | Yes |
| Complexity = 1.0 | Matches any tier with `complexity_range[1] == 1.0` | Yes |
| Empty `model_access` allowlist | Interpreted as "all models allowed" (not "no models") | Yes |
| Rate limit = 0 in permissions | Interpreted as unlimited; skip rate check entirely | Yes |
| Budget = 0.0 in permissions | Interpreted as unlimited; skip budget check entirely | Yes |
| No `auth_context` on request | Default to `AuthContext::default()` (zero-trust) | Yes |
| Fallback model in tier above user's max_tier | FIX-06: fallback_chain() and rate_limited_decision() return error decision, not unauthorized model | Yes |
| Escalation would go beyond all configured tiers | `max_escalation_tiers` limits promotion; if no escalation tier matches, falls back to highest allowed | Yes |
| Single model in tier | All 4 selection strategies return the same model | Yes |
| `fallback_model` not configured | `no_tiers_available_decision` returns empty `provider`/`model` with explanatory `reason` | Yes |
| Budget exhausted for all tiers | Uses cheapest tier anyway (overage is recorded, not blocked); `budget_constrained = true` | Yes |
| Tier with empty models list | `select_model` returns `None`, triggers fallback chain | Yes |

### 4.2 Escalation Safety

- `max_escalation_tiers` (default: 1) prevents unbounded tier promotion
- Escalation requires ALL THREE conditions: `permissions.escalation_allowed`, `escalation_config.enabled`, `complexity > escalation_threshold`
- Level 0 (zero_trust) built-in defaults set `escalation_allowed = false` and `escalation_threshold = 1.0` (unreachable), so zero-trust users can never escalate
- The escalation threshold comparison is strict (`>`, not `>=`), so a threshold of 0.6 requires complexity strictly above 0.6

### 4.3 Concurrent Access Patterns

- **Round-robin counters**: `AtomicUsize::fetch_add` with `Ordering::Relaxed`. Under high
  concurrency, two threads may get the same counter value (ABA scenario), causing the same
  model to be picked twice in a row. This is acceptable for load distribution -- the goal
  is approximate fairness, not strict alternation.

- **Cost tracker race**: `check_budget()` and `record_estimated_cost()` are separate calls
  (not atomic). Two concurrent requests could both pass the budget check before either records
  its cost, causing a temporary budget overshoot. This is documented as expected behavior in
  the design doc -- budget checks are estimates, not billing. The next request sees the
  updated totals.

- **Rate limiter race**: Similar to cost tracker. Two requests could slip through a rate limit
  boundary simultaneously. The sliding-window implementation in Phase E handles this with
  atomic operations, but slight overages (1-2 extra requests) are acceptable.

### 4.4 Performance Considerations

The routing decision must complete in under 1ms with no I/O in the hot path:

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| read permissions from AuthContext | O(1) | Direct field access, no HashMap lookups (FIX-02) |
| `filter_tiers_by_permissions` | O(n) | Linear scan over tiers; n is typically 2-5 |
| `select_tier` | O(n) | Linear scan over allowed tiers |
| `apply_budget_constraints` | O(n) | Worst case: scan all allowed tiers |
| `select_model` | O(m) | Linear scan over tier models; m is typically 2-4 |
| `filter_models_by_permissions` | O(m * p) | m models, p patterns; both are small |
| `split_provider_model` | O(k) | Single string scan, k = model name length |
| `model_matches_pattern` | O(k) | String prefix/equality check |

Total: O(n * m * p) where n <= 5, m <= 4, p <= 5. Well under 1ms on any hardware.

No heap allocations in the hot path except:
- The returned `RoutingDecision` (unavoidable)
- `Vec` for `allowed_tiers` and `available_models` (small, stack-friendly with SmallVec if needed)
- String clones for provider/model names (could be `Arc<str>` if profiling shows it matters)

### 4.5 Error Handling Strategy

The `ModelRouter::route` method returns `RoutingDecision` (not `Result`). This is an infallible
interface -- the router always returns a decision, even if that decision is "no models available."
Errors are expressed as:

- Empty `provider`/`model` strings with an explanatory `reason`
- The `budget_constrained` flag signals budget issues
- The `tier: None` indicates no tier was selected (fallback or error)
- The agent loop is responsible for interpreting these signals and returning appropriate
  user-facing messages (e.g., "I'm temporarily rate limited" or "budget exhausted")

This matches the existing `StaticRouter` contract where `route()` always succeeds.

---

## 5. Completion

### 5.1 Exit Criteria

- [ ] `TieredRouter` compiles and implements `ModelRouter` trait
- [ ] `StaticRouter` continues to work unchanged (its tests still pass)
- [ ] `RoutingDecision` extended with 4 optional fields without breaking existing code
- [ ] `ChatRequest` extended with optional `auth_context` without breaking existing code
- [ ] All 4 selection strategies work correctly
- [ ] Tier selection respects complexity ranges and permission boundaries
- [ ] Tier selection picks the highest-quality tier the user is allowed and can afford
- [ ] Escalation logic promotes correctly and respects `max_escalation_tiers`
- [ ] Budget constraints downgrade tiers when budget is near exhaustion
- [ ] Fallback chain works: same tier -> lower tier -> fallback model -> empty decision
- [ ] `from_config()` correctly parses `RoutingConfig` into `TieredRouter`
- [ ] `from_config_simple()` creates a usable router without cost/rate dependencies
- [ ] No-op impls (`NoopCostTracker`, `NoopRateLimiter`) allow testing without Phase D/E
- [ ] All new code is `Send + Sync` (enforced by `ModelRouter` trait bounds)
- [ ] `cargo test -p clawft-core` passes with all new and existing tests
- [ ] `cargo clippy -p clawft-core -- -D warnings` has zero warnings
- [ ] File is under 500 lines (split if necessary)

### 5.2 Test Plan (35+ Unit Tests)

All tests go in a `#[cfg(test)] mod tests` block at the bottom of `tiered_router.rs`.

#### Tier Matching Tests (5)

| # | Name | Verifies |
|---|------|----------|
| 1 | `tier_matches_complexity_in_range` | complexity 0.5 matches tier [0.0, 0.7] |
| 2 | `tier_matches_complexity_at_lower_bound` | complexity 0.0 matches tier [0.0, 0.3] |
| 3 | `tier_matches_complexity_at_upper_bound` | complexity 0.7 matches tier [0.0, 0.7] |
| 4 | `tier_does_not_match_below_range` | complexity 0.0 does not match tier [0.3, 1.0] |
| 5 | `tier_does_not_match_above_range` | complexity 1.0 does not match tier [0.0, 0.3] |

#### Tier Selection Tests (3)

| # | Name | Verifies |
|---|------|----------|
| 6 | `select_tier_picks_highest_quality_match` | complexity 0.5 with free+standard tiers picks standard (highest ordinal) |
| 7 | `select_tier_with_single_matching_tier` | only one tier matches, picks it |
| 8 | `select_tier_falls_back_to_highest_allowed` | no tier matches complexity, uses highest allowed tier |

#### Escalation Tests (5)

| # | Name | Verifies |
|---|------|----------|
| 9 | `escalation_promotes_to_next_tier` | max_tier=standard, complexity=0.8, escalation_allowed=true -> premium |
| 10 | `escalation_respects_max_escalation_tiers` | max_escalation_tiers=1 limits to one tier above |
| 11 | `escalation_denied_when_not_allowed` | escalation_allowed=false prevents promotion |
| 12 | `escalation_denied_below_threshold` | complexity below escalation_threshold prevents promotion |
| 13 | `escalation_denied_when_config_disabled` | escalation.enabled=false prevents promotion |

#### Budget Constraint Tests (4)

| # | Name | Verifies |
|---|------|----------|
| 14 | `budget_unlimited_skips_check` | cost_budget_daily_usd=0.0 means no constraint |
| 15 | `budget_sufficient_allows_tier` | remaining > estimated cost allows selected tier |
| 16 | `budget_insufficient_downgrades_tier` | remaining < premium cost, downgrades to standard |
| 17 | `budget_exhausted_uses_cheapest_tier` | no budget left, falls back to free tier, `budget_constrained = true` |

#### Selection Strategy Tests (4)

| # | Name | Verifies |
|---|------|----------|
| 18 | `preference_order_picks_first_model` | PreferenceOrder returns models[0] |
| 19 | `round_robin_rotates_models` | RoundRobin cycles through models across multiple calls |
| 20 | `random_returns_valid_model` | Random returns a model from the tier's list |
| 21 | `lowest_cost_picks_first_model` | LowestCost returns models[0] (same cost within tier) |

#### Permission Filtering Tests (5)

| # | Name | Verifies |
|---|------|----------|
| 22 | `filter_tiers_by_max_tier` | max_tier=standard filters out premium and elite |
| 23 | `filter_tiers_unknown_max_tier` | unknown tier name defaults to cheapest only |
| 24 | `model_access_allowlist_filters` | model_access=["anthropic/*"] allows only anthropic models |
| 25 | `model_denylist_filters` | model_denylist=["openai/gpt-4o"] excludes that specific model |
| 26 | `empty_model_access_allows_all` | empty allowlist means all models allowed |

#### Fallback Chain Tests (5)

| # | Name | Verifies |
|---|------|----------|
| 27 | `fallback_to_lower_tier` | primary tier has no available models, falls back to lower tier |
| 28 | `fallback_to_fallback_model` | no tiers have available models, uses fallback_model config |
| 29 | `fallback_returns_none_when_no_fallback` | no tiers and no fallback_model, returns None |
| 40 | `fallback_model_denied_above_max_tier` | FIX-06: fallback_model in premium tier denied for user with max_tier=standard |
| 41 | `rate_limited_fallback_denied_above_max_tier` | FIX-06: rate_limited_decision denies fallback_model in tier above user's max_tier |

#### Full route() Integration Tests (5)

| # | Name | Verifies |
|---|------|----------|
| 30 | `route_low_complexity_to_free_tier` | complexity=0.1 routes to free tier model |
| 31 | `route_high_complexity_to_premium` | complexity=0.9 with admin perms routes to premium/elite |
| 32 | `route_no_auth_context_uses_zero_trust` | absent auth_context defaults to zero_trust permissions |
| 33 | `route_rate_limited_returns_fallback` | rate limiter returns false, decision uses fallback |
| 34 | `route_update_does_not_panic` | update() with any input does not panic |

#### Edge Case Tests (5)

| # | Name | Verifies |
|---|------|----------|
| 35 | `empty_tiers_config_uses_fallback` | zero tiers configured, routes to fallback model |
| 36 | `single_tier_config_always_uses_it` | one tier, always selected regardless of complexity |
| 37 | `model_matches_pattern_exact` | "anthropic/claude-haiku-3.5" matches "anthropic/claude-haiku-3.5" |
| 38 | `model_matches_pattern_wildcard` | "anthropic/claude-haiku-3.5" matches "anthropic/*" |
| 39 | `model_matches_pattern_star` | any model matches "*" |

### 5.3 Files Modified Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `crates/clawft-core/src/pipeline/tiered_router.rs` | **NEW** | Full TieredRouter implementation (~400 lines, reduced by FIX-02 permission removal) |
| `crates/clawft-core/src/pipeline/mod.rs` | **EDIT** | Add `pub mod tiered_router;` (1 line) |
| `crates/clawft-core/src/pipeline/traits.rs` | **EDIT** | Extend `RoutingDecision` (4 fields + `Default` derive per FIX-10), extend `ChatRequest` (1 field with `skip_deserializing` per FIX-05), add `AuthContext` import from `clawft_types::routing` (FIX-01) |
| `crates/clawft-core/src/pipeline/router.rs` | **EDIT** | Update `RoutingDecision` constructions with `..Default::default()` (FIX-10); update `ChatRequest` constructions with `auth_context: None` (FIX-09) |
| `crates/clawft-core/Cargo.toml` | **EDIT** | Add `rand = "0.8"` and `tracing = "0.1"` dependencies (FIX-12) |

### 5.4 Verification Commands

```bash
# Build just the affected crate
cargo build -p clawft-core

# Test all pipeline tests (StaticRouter + TieredRouter)
cargo test -p clawft-core

# Test only tiered_router module
cargo test -p clawft-core tiered_router

# Lint with warnings as errors
cargo clippy -p clawft-core -- -D warnings

# Full workspace regression check
cargo test --workspace
cargo clippy --workspace
```

### 5.5 Effort Estimate

**1.5 days**, broken down:

| Task | Hours |
|------|-------|
| `ModelTier`, `TierSelectionStrategy`, helper types | 1 |
| `CostTrackable` / `RateLimitable` traits + no-op impls | 1 |
| `TieredRouter` struct + `from_config()` constructor | 2 |
| `filter_tiers_by_permissions()` (FIX-02: no resolve_permissions) | 0.5 |
| Fallback permission checks + tracing audit logs (FIX-06, FIX-12) | 0.5 |
| `select_tier()` with escalation logic | 2 |
| `apply_budget_constraints()` | 1 |
| `select_model()` + `filter_models_by_permissions()` + `model_matches_pattern()` | 1 |
| `fallback_chain()` + helper decision builders | 1 |
| `ModelRouter` trait impl (`route()` + `update()`) | 1 |
| Update `traits.rs` (`RoutingDecision` + `ChatRequest` extensions) | 0.5 |
| Update `router.rs` (StaticRouter `RoutingDecision` literals) | 0.5 |
| Unit tests (35+) | 3 |
| **Total** | **~15 hours (1.5 days)** |

### 5.6 What This Phase Does NOT Include

- No permission resolution (Phase B) -- the router reads pre-resolved permissions from `AuthContext.permissions` directly (FIX-02)
- No `CostTracker` implementation (Phase D) -- uses `CostTrackable` trait with `NoopCostTracker`
- No `RateLimiter` implementation (Phase E) -- uses `RateLimitable` trait with `NoopRateLimiter`
- No `AuthContext` threading from channels through agent loop (Phase F) -- reads `auth_context` from `ChatRequest` if present
- No tool permission enforcement (Phase G)
- No config validation (Phase H)
- No integration tests with real providers (Phase I)

This phase delivers the core routing algorithm as a standalone, testable module with
dependency-injected interfaces for cost tracking and rate limiting.

### 5.7 Branch

All work on branch: `weft/tiered-router`

---

**End of SPARC Plan**

---

## Remediation Applied

**Date**: 2026-02-18
**Source**: `/home/aepod/dev/clawft/.planning/sparc/01-tiered-router/remediation-plan.md`

The following fixes from the remediation plan have been applied to this Phase C document:

### FIX-01: Canonical Type Ownership (CRITICAL)
- Confirmed all imports reference `clawft_types::routing` (not `clawft_types::auth`).
- Added explicit comment in section 2.1 imports noting the canonical import path.
- Updated dependency graph in section 3.2 to note FIX-01.

### FIX-02: Single Permission Resolution (CRITICAL)
- REMOVED `permission_defaults`, `user_overrides`, `channel_overrides` fields from TieredRouter struct (section 1.2).
- REMOVED `resolve_permissions()` method and all supporting code (`level_to_name()`, `merge_permissions()`).
- REMOVED `build_permission_defaults()`, `build_user_overrides()`, `build_channel_overrides()` helper methods from `from_config()`.
- REMOVED Debug impl references to the three removed fields.
- Updated `route()` Step 2 to read permissions directly: `request.auth_context.as_ref().map(|a| &a.permissions).unwrap_or(&UserPermissions::default())`.
- Section 2.6 rewritten as "Permission Reading" documenting the direct read pattern.
- Section 1.1 mission statement updated to reflect read-only permission access.
- Section 1.7 algorithm step 2 updated from "Resolve permissions" to "Read permissions".
- Performance table updated to replace O(1) HashMap lookups with O(1) direct field access.
- "What This Phase Does NOT Include" section updated to explicitly state no permission resolution.
- Edge case table updated: removed "User in both user_overrides and channel_overrides" (no longer applicable).
- Effort estimate updated to remove `resolve_permissions()` line item.

### FIX-03: CostTrackable/RateLimitable Trait Alignment (CRITICAL)
- Updated `CostTrackable` trait signature in sections 1.6 and 2.4:
  - `check_budget(&self, sender_id: &str, estimated_cost: f64, daily_limit: f64, monthly_limit: f64) -> BudgetResult`
  - `record_estimated(&self, sender_id: &str, estimated_cost: f64)` (renamed from `record_estimated_cost`)
  - `record_actual(&self, sender_id: &str, estimated_cost: f64, actual_cost: f64)` (renamed from `record_actual_cost`)
- Added `BudgetResult` struct definition in section 2.4 (with `allowed`, `remaining_daily_usd`, `remaining_monthly_usd`, `reason` fields).
- Updated `NoopCostTracker` to implement the new trait signatures.
- Updated `RateLimitable` trait confirmed matching Phase E: `fn check(&self, sender_id: &str, limit: u32) -> bool`.
- Updated `apply_budget_constraints()` (section 2.9) to call `check_budget()` with `permissions.cost_budget_daily_usd` and `permissions.cost_budget_monthly_usd`.
- Updated `route()` Step 8 to call `record_estimated()` (renamed).
- Updated `update()` comment to reference `record_actual()` (renamed).
- Updated file locations table to include `BudgetResult`.
- Renamed internal `BudgetResult<'a>` struct to `TierBudgetResult<'a>` to avoid name collision.

### FIX-05: AuthContext Injection Prevention (HIGH SECURITY)
- Updated `ChatRequest` extension (section 3.4) to use `#[serde(default, skip_deserializing)]` on the `auth_context` field (was `skip_serializing_if`).
- Added comment documenting that `auth_context` is a trusted field set server-side only.

### FIX-06: Fallback Model Permission Check (HIGH SECURITY)
- Updated `fallback_chain()` (section 2.11) to verify fallback model belongs to a tier at or below user's `max_tier`. If the fallback model is in a higher tier, it is not returned.
- Updated `rate_limited_decision()` (section 2.12) with the same permission check for the fallback model.
- Added two new test cases (tests 40-41) for fallback permission denial.
- Updated edge case table with "Fallback model in tier above user's max_tier" entry.
- Added `tracing::info!` audit logs at fallback decision points.

### FIX-09: ChatRequest Extension Ownership (HIGH)
- Confirmed Phase C owns the `ChatRequest` extension with `auth_context: Option<AuthContext>`.
- Updated section 3.4 to note `auth_context: None` requirement for all existing construction sites.
- Updated files modified summary to note `auth_context: None` updates in `router.rs`.

### FIX-10: RoutingDecision Default Trait (HIGH)
- Added `Default` derive to `RoutingDecision` struct (section 3.5).
- Documented default values: `tier: None`, `cost_estimate_usd: None`, `escalated: false`, `budget_constrained: false`, `provider: ""`, `model: ""`, `reason: ""`.
- Updated `no_tiers_available_decision()` to use `..Default::default()`.
- Updated `rate_limited_decision()` to use `..Default::default()`.
- Updated fallback chain return in `route()` to use `..Default::default()`.
- Added migration example in section 3.5.

### FIX-12: Minor Fixes (MEDIUM/LOW)
- Access `escalation_config.max_escalation_tiers` directly in `select_tier()` (section 2.8) -- it is a required field with serde default, not optional.
- Access `tier.max_context_tokens` directly in `from_config()` (section 2.5) -- it is a required field with serde default, not optional.
- Added TODO comment in `apply_budget_constraints()` to replace `cost_per_1k_tokens` with Phase D's `estimate_cost()` when integrated.
- Added `tracing::info!` structured audit log events at key decision points:
  - Route entry (sender, channel, complexity, max_tier, level)
  - Rate limit triggered
  - No tiers available
  - Escalation applied (user_max_tier, escalated_to, complexity)
  - Budget constraint triggered (user, tier, cost, limits)
  - Fallback chain tier switch
  - Fallback model used or denied
  - Route completion (sender, provider, model, tier, cost, escalated, budget_constrained)
- Added `tracing = "0.1"` to Cargo.toml dependencies.
- Added `use tracing;` to module imports.
