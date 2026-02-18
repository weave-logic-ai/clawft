# Phase A Decisions: RoutingConfig Types

**Phase**: A -- RoutingConfig Types in `clawft-types`
**Status**: Planning
**Date**: 2026-02-18

---

## Decision 1: Where to Put New Types

### Question

Should the new routing configuration types (`RoutingConfig`, `ModelTierConfig`,
`PermissionsConfig`, etc.) be added to the existing `config.rs` module, or
placed in a new `routing.rs` module in `clawft-types`?

### Options Considered

**Option A: Extend `config.rs`**

- `RoutingConfig` is a top-level config field, peer to `AgentsConfig`,
  `ChannelsConfig`, and `ProvidersConfig`
- All other top-level config structs live in `config.rs`
- Consistent with existing pattern
- Single file to reference for all config types
- Risk: `config.rs` grows larger (currently ~250 lines, would become ~450)

**Option B: New `routing.rs` module (SELECTED -- resolved by consensus)**

- Cleaner separation of concerns
- `config.rs` stays focused on existing types
- Requires `pub mod routing;` in `lib.rs`
- The routing module contains both config types AND runtime types (`AuthContext`,
  `UserPermissions`) -- this is more than pure config
- Matches the Phase A SPARC plan (`A-routing-config-types.md`) which specifies
  `crates/clawft-types/src/routing.rs` as the target file
- Follows the precedent set by `delegation.rs` (domain-specific types in own module)

### Decision

**Option B: New `routing.rs` module**. All routing types go in
`crates/clawft-types/src/routing.rs`. This includes `RoutingConfig`,
`ModelTierConfig`, `PermissionsConfig`, `PermissionLevelConfig`,
`UserPermissions`, `AuthContext`, `TierSelectionStrategy`, `EscalationConfig`,
`CostBudgetConfig`, and `RateLimitConfig`.

The `Config` struct in `config.rs` gains a `pub routing: RoutingConfig` field
with `#[serde(default)]`, importing from the routing module.

This decision was resolved by consensus (CONS-001) and confirmed by the
remediation plan (FIX-01, FIX-11). The original Option A (extend `config.rs`)
was rejected because:
1. The routing types include runtime types (`AuthContext`) beyond pure config
2. The Phase A SPARC plan already specified `routing.rs` as the target
3. Co-locating `AuthContext` with `UserPermissions` avoids cross-module imports
   (resolves CONS-004)

### Confidence: 98% (resolved by consensus, confirmed by remediation plan)

---

## Decision 2: Serde Strategy

### Question

What serialization conventions should the new types follow?

### Decision

Match the existing codebase convention: **`snake_case` field names as primary,
with `camelCase` aliases** via `#[serde(alias = "camelCase")]`.

This is consistent with how `config.rs` already handles fields like:
- `api_key` / `apiKey`
- `allow_from` / `allowFrom`
- `max_tokens` / `maxTokens`

### Implementation Pattern

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    #[serde(default = "default_routing_mode")]
    pub mode: String,

    #[serde(default)]
    pub tiers: Vec<ModelTierConfig>,

    #[serde(default, alias = "selectionStrategy")]
    pub selection_strategy: Option<String>,

    #[serde(default, alias = "fallbackModel")]
    pub fallback_model: Option<String>,

    #[serde(default)]
    pub permissions: PermissionsConfig,

    #[serde(default)]
    pub escalation: EscalationConfig,

    #[serde(default, alias = "costBudgets")]
    pub cost_budgets: CostBudgetConfig,

    #[serde(default, alias = "rateLimiting")]
    pub rate_limiting: RateLimitConfig,
}
```

### Key Rules

1. All fields use `#[serde(default)]` for backward compatibility
2. Nested config structs implement `Default`
3. `String` fields with specific defaults use `#[serde(default = "fn_name")]`
4. Enums use `#[serde(rename_all = "snake_case")]`

### Confidence: 98%

---

## Decision 3: Default Routing Mode

### Question

What should the default value of `routing.mode` be when the field is absent
from config?

### Options Considered

**Option A: `"static"` (SELECTED)**

- Existing behavior: `StaticRouter` reads `agents.defaults.model`
- All existing configs continue to work without modification
- Users must explicitly opt in to tiered routing
- Zero risk of behavioral change on upgrade

**Option B: `"tiered"` with sensible tier defaults**

- Would activate tiered routing by default
- Could break existing deployments that expect exact model routing
- More "batteries included" but violates principle of least surprise

**Option C: `"auto"` (detect from config)**

- If `routing.tiers` is present, use tiered; otherwise use static
- Implicit behavior is harder to debug
- User might not realize they activated tiered routing by adding a tier

### Decision

**Option A: `"static"`**. Backward compatibility is a hard requirement per the
design doc. The `TieredRouter` is an opt-in upgrade. The default function:

```rust
fn default_routing_mode() -> String {
    "static".into()
}
```

### Confidence: 99%

---

## Decision 4: UserPermissions Level as Primary

### Question

Should user permissions be driven primarily by a numeric `level` field (with
dimension overrides), or by a flat capability matrix with no levels?

### Options Considered

**Option A: Level as primary with dimension overrides (SELECTED)**

- Three levels (0, 1, 2) provide sensible defaults
- Dimension overrides allow fine-grained control per user/channel
- Levels are easy to understand: "this user is Level 1"
- Override syntax: `{ "level": 1, "tool_access": ["read_file"], "cost_budget_daily_usd": 2.0 }`
- Matches common RBAC patterns (roles with permission overrides)

**Option B: Flat capability matrix only**

- Every dimension explicitly specified per user
- No implicit defaults from levels
- More flexible but more verbose
- Higher cognitive load for operators

**Option C: Named roles (not just levels)**

- `"role": "researcher"` with custom role definitions
- More expressive but adds config complexity
- Over-engineered for a personal assistant framework

### Decision

**Option A: Level as primary**. The three levels cover the vast majority of
use cases:

| Level | Name | Who |
|-------|------|-----|
| 0 | `zero_trust` | Anonymous, unknown users |
| 1 | `user` | Authenticated via `allow_from` |
| 2 | `admin` | Operators, CLI, explicitly granted |

Each level provides complete default values for all dimensions. Individual
dimensions can be overridden per-user or per-channel without changing the
level. This balances simplicity with flexibility.

### Resolution Precedence

```
1. Built-in defaults for level
2. Global config: routing.permissions.<level_name>
3. Channel override: routing.permissions.channels.<channel>
4. User override: routing.permissions.users.<user_id>
```

Higher precedence wins at the leaf level (individual fields), not wholesale
replacement. Example: a user-level override of `cost_budget_daily_usd: 2.0`
changes only that field, not the entire permission set.

### Confidence: 95%

---

## Decision 5: ModelTierConfig Complexity Range Format

### Question

How should the complexity range be represented in the tier config?

### Options Considered

**Option A: Two-element array `[f32; 2]` (SELECTED)**

- Compact: `"complexity_range": [0.0, 0.3]`
- Clear: `[min, max]` semantics
- Easy to validate: `range[0] <= range[1]`, both in `0.0..=1.0`
- Matches the design doc specification

**Option B: Named struct `{ "min": 0.0, "max": 0.3 }`**

- More explicit field names
- More verbose in config
- Harder to scan visually in a list of tiers

**Option C: Single threshold (tier activates above this complexity)**

- Simpler but doesn't support overlapping ranges
- Overlapping ranges are a design requirement (complexity 0.5 can match
  both `standard` and `premium`)

### Decision

**Option A: `[f32; 2]`**. In Rust:

```rust
pub struct ModelTierConfig {
    pub name: String,
    pub models: Vec<String>,
    pub complexity_range: [f32; 2],
    pub cost_per_1k_tokens: f64,
    #[serde(default = "default_tier_max_context")]
    pub max_context_tokens: usize,
}
```

Validation: `complexity_range[0] >= 0.0`, `complexity_range[1] <= 1.0`,
`complexity_range[0] <= complexity_range[1]`.

### Confidence: 95%

---

## Decision 6: TierSelectionStrategy as Enum

### Question

Should the model selection strategy be a string or a typed enum?

### Decision

Typed enum with `#[serde(rename_all = "snake_case")]`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TierSelectionStrategy {
    #[default]
    PreferenceOrder,
    RoundRobin,
    LowestCost,
    Random,
}
```

Config uses lowercase snake_case strings: `"preference_order"`,
`"round_robin"`, `"lowest_cost"`, `"random"`.

### Confidence: 98%

---

## Open Questions

### Q1: Should `UserPermissions` implement `Merge` trait?

The permission resolution logic merges multiple layers of permissions. Should
this be a formal trait or just a function?

**Leaning**: Function (`merge_permissions(base, override)`) is simpler and
only used in one place. A trait is unnecessary abstraction.

### Q2: Should `RoutingConfig` validation be in `clawft-types` or `clawft-core`?

`clawft-types` is zero-dep beyond serde. Validation logic (checking tier name
uniqueness, complexity range validity) could add complexity.

**Leaning**: Basic validation (range checks) in `clawft-types` via a
`validate()` method. Business rule validation (tier name referenced in
permissions exists) in `clawft-core` during router construction.

### Q3: Should `AuthContext` go in `config.rs` or a new `auth.rs`?

`AuthContext` is not strictly configuration -- it's a runtime struct carried
on requests.

**RESOLVED**: `AuthContext` goes in `routing.rs` alongside `UserPermissions`
and all other routing types. No `auth.rs` module is created. See CONS-004
(resolved) and FIX-01 in the remediation plan. The rationale is that
`AuthContext` contains `UserPermissions`, so co-location avoids cross-module
imports, and a single-struct `auth.rs` module adds unnecessary file proliferation.

---

## Test Plan for Phase A

| Test | Description |
|------|-------------|
| `routing_config_defaults` | Default RoutingConfig has mode="static", empty tiers |
| `routing_config_serde_roundtrip` | Full config serializes and deserializes |
| `routing_config_from_empty_json` | Empty `{}` produces valid defaults |
| `routing_config_camel_case_aliases` | camelCase fields parse correctly |
| `model_tier_config_serde` | Tier with all fields roundtrips |
| `model_tier_config_defaults` | Missing optional fields get defaults |
| `permissions_config_defaults` | Default permissions are empty/sensible |
| `user_permissions_defaults` | Default UserPermissions matches zero_trust |
| `user_permissions_serde` | Full permissions struct roundtrips |
| `tier_selection_strategy_serde` | All 4 variants serialize to snake_case |
| `escalation_config_defaults` | Escalation disabled by default |
| `cost_budget_config_defaults` | Zero budgets (unlimited) by default |
| `rate_limit_config_defaults` | 60-second window by default |
| `config_with_routing_section` | Root Config accepts routing field |
| `config_without_routing_section` | Root Config without routing still works |
