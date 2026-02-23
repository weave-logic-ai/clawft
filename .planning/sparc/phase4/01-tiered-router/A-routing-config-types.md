# SPARC Plan: Phase A - Routing Config Types

**Phase**: Tiered Router (01)
**Owner**: Types architect
**Branch**: `weft/tiered-router`
**Status**: BLOCKS Phases B-I
**Effort**: 0.5 days

---

## Agent Instructions

### Mission

You are implementing the configuration types for the TieredRouter and Permission System
described in `.planning/08-tiered-router.md`. These are **pure data types** -- no business
logic, no runtime behavior, just serde-serializable structs with sensible defaults.

All types go into `crates/clawft-types/`. The root `Config` struct gains a new
`routing: RoutingConfig` field. Existing configs without a `routing` key must continue
to deserialize unchanged (backward compatibility is mandatory).

### Files to Read

| File | Purpose |
|------|---------|
| `crates/clawft-types/src/config.rs` | Existing config types -- match this style exactly |
| `crates/clawft-types/src/delegation.rs` | Example of a config subsection in its own file |
| `crates/clawft-types/src/lib.rs` | Module declarations and re-exports |
| `.planning/08-tiered-router.md` Section 7 | Canonical config JSON format |
| `.planning/08-tiered-router.md` Section 7.2 | Type definitions to implement |
| `.planning/08-tiered-router.md` Section 2-3 | Permission model and capability schema |

### Critical Success Criteria

- [ ] All new types compile with zero warnings (`cargo clippy`)
- [ ] `Config` struct now has `routing: RoutingConfig` field with `#[serde(default)]`
- [ ] Empty JSON `{}` still deserializes to `Config` with all defaults (backward compat)
- [ ] Full tiered config JSON from Section 7.1 of design doc deserializes correctly
- [ ] Serde roundtrip: serialize -> deserialize -> values match
- [ ] Unknown fields in `routing` section are silently ignored
- [ ] Both `snake_case` and `camelCase` field names work (via `alias`)
- [ ] All tests pass (`cargo test --workspace`)

---

## 1. Specification

### 1.1 New Types Overview

Ten new types are introduced, organized into two files:

| Type | File | Purpose |
|------|------|---------|
| `RoutingConfig` | `src/routing.rs` | Top-level routing configuration (mode, tiers, permissions, etc.) |
| `TierSelectionStrategy` | `src/routing.rs` | Enum for model selection strategy within a tier |
| `ModelTierConfig` | `src/routing.rs` | A named tier of models with complexity range and cost info |
| `PermissionsConfig` | `src/routing.rs` | Container for level defaults + per-user/per-channel overrides |
| `PermissionLevelConfig` | `src/routing.rs` | Configuration for a single permission level (zero_trust, user, admin) |
| `UserPermissions` | `src/routing.rs` | Resolved user permission capabilities (capability matrix) |
| `EscalationConfig` | `src/routing.rs` | Escalation behavior settings |
| `CostBudgetConfig` | `src/routing.rs` | Global cost budget limits |
| `RateLimitConfig` | `src/routing.rs` | Rate limiting strategy settings |
| `AuthContext` | `src/routing.rs` | Authentication context threaded through the pipeline |

### 1.2 Integration Point

The root `Config` struct in `config.rs` gains one new field:

```rust
#[serde(default)]
pub routing: RoutingConfig,
```

This is imported from the new `routing` module, similar to how `DelegationConfig` is
imported from `delegation.rs`.

### 1.3 Serde Requirements

All types must follow the existing pattern in `config.rs`:

- `#[derive(Debug, Clone, Serialize, Deserialize)]` on all structs
- `Default` derive or manual `impl Default` with named default functions
- `#[serde(default)]` on every field that has a sensible default
- `#[serde(alias = "camelCase")]` for any snake_case field that has a camelCase JSON counterpart
- Unknown fields silently ignored (serde's default behavior for structs without `deny_unknown_fields`)

### 1.4 Default Values

When `routing` is absent from JSON, `RoutingConfig::default()` produces a configuration
equivalent to the existing `StaticRouter` behavior:

- `mode`: `"static"` (not `"tiered"`)
- `tiers`: empty vec (no tiers defined)
- `selection_strategy`: `None` (typed as `Option<TierSelectionStrategy>`)
- `fallback_model`: `None`
- `permissions`: `PermissionsConfig::default()` (empty level configs, no user/channel overrides)
- `escalation`: disabled
- `cost_budgets`: no limits
- `rate_limiting`: 60-second window, sliding_window strategy, global_rate_limit_rpm = 0 (unlimited)

### 1.5 Forward Compatibility

- All structs accept unknown fields (no `#[serde(deny_unknown_fields)]`)
- `UserPermissions` has a `custom_permissions: HashMap<String, serde_json::Value>` escape hatch
- `PermissionsConfig` uses `HashMap<String, PermissionLevelConfig>` for user/channel overrides
  so new per-user/per-channel keys are automatically supported

### 1.6 Backward Compatibility

- `Config` with no `routing` key deserializes to `RoutingConfig::default()` (mode = "static")
- Existing test fixtures (`tests/fixtures/config.json`) must still pass all existing tests
- No changes to any existing type signatures

---

## 2. Pseudocode

### 2.1 File: `crates/clawft-types/src/routing.rs`

```rust
//! Routing and permission configuration types.
//!
//! Defines the config schema for the TieredRouter (Level 1) and its
//! permission system. All types support both `snake_case` and `camelCase`
//! field names in JSON. Unknown fields are silently ignored for forward
//! compatibility.
//!
//! When the `routing` section is absent from config, `RoutingConfig::default()`
//! produces settings equivalent to the existing `StaticRouter` (Level 0).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── TierSelectionStrategy ────────────────────────────────────────────────

/// Strategy for selecting a model within a tier.
///
/// Controls how the router picks among multiple models in a single tier.
/// Serializes/deserializes as snake_case strings (e.g., `"preference_order"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TierSelectionStrategy {
    /// Use models in the order listed (first available wins).
    PreferenceOrder,
    /// Rotate through models in round-robin fashion.
    RoundRobin,
    /// Pick the cheapest model in the tier.
    LowestCost,
    /// Pick a random model from the tier.
    Random,
}

// ── RoutingConfig ────────────────────────────────────────────────────────

/// Top-level routing configuration.
///
/// Added to the root `Config` struct alongside `agents`, `channels`, `providers`, etc.
/// When absent from JSON, defaults to `mode = "static"` (Level 0 StaticRouter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Routing mode: `"static"` (default, Level 0) or `"tiered"` (Level 1).
    #[serde(default = "default_routing_mode")]
    pub mode: String,

    /// Model tier definitions, ordered cheapest to most expensive.
    /// Only used when `mode = "tiered"`.
    #[serde(default)]
    pub tiers: Vec<ModelTierConfig>,

    /// Model selection strategy within a tier.
    /// Uses the `TierSelectionStrategy` enum for type safety.
    #[serde(default, alias = "selectionStrategy")]
    pub selection_strategy: Option<TierSelectionStrategy>,

    /// Fallback model when all tiers/budgets are exhausted.
    /// Format: `"provider/model"` (e.g., `"groq/llama-3.1-8b"`).
    #[serde(default, alias = "fallbackModel")]
    pub fallback_model: Option<String>,

    /// Permission level definitions and per-user/channel overrides.
    #[serde(default)]
    pub permissions: PermissionsConfig,

    /// Escalation behavior settings.
    #[serde(default)]
    pub escalation: EscalationConfig,

    /// Global cost budget settings.
    #[serde(default, alias = "costBudgets")]
    pub cost_budgets: CostBudgetConfig,

    /// Rate limiting settings.
    #[serde(default, alias = "rateLimiting")]
    pub rate_limiting: RateLimitConfig,
}

fn default_routing_mode() -> String {
    "static".into()
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            mode: default_routing_mode(),
            tiers: Vec::new(),
            selection_strategy: None,
            fallback_model: None,
            permissions: PermissionsConfig::default(),
            escalation: EscalationConfig::default(),
            cost_budgets: CostBudgetConfig::default(),
            rate_limiting: RateLimitConfig::default(),
        }
    }
}

// ── ModelTierConfig ──────────────────────────────────────────────────────

/// A named group of models at a similar cost/capability level.
///
/// Tiers are ordered from cheapest to most expensive in the `tiers` array.
/// Complexity ranges may overlap intentionally -- the router picks the
/// highest-quality tier the user is allowed and can afford.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTierConfig {
    /// Tier name (e.g., `"free"`, `"standard"`, `"premium"`, `"elite"`).
    pub name: String,

    /// Models available in this tier, in preference order.
    /// Format: `"provider/model"` (e.g., `"anthropic/claude-haiku-3.5"`).
    #[serde(default)]
    pub models: Vec<String>,

    /// Complexity range this tier covers: `[min, max]` where each is 0.0-1.0.
    #[serde(default = "default_complexity_range", alias = "complexityRange")]
    pub complexity_range: [f32; 2],

    /// Approximate cost per 1K tokens (blended input/output) in USD.
    /// Used for budget estimation, not billing.
    #[serde(default, alias = "costPer1kTokens")]
    pub cost_per_1k_tokens: f64,

    /// Maximum context tokens supported by models in this tier.
    #[serde(default = "default_tier_max_context", alias = "maxContextTokens")]
    pub max_context_tokens: usize,
}

fn default_complexity_range() -> [f32; 2] {
    [0.0, 1.0]
}

fn default_tier_max_context() -> usize {
    8192
}

impl Default for ModelTierConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            models: Vec::new(),
            complexity_range: default_complexity_range(),
            cost_per_1k_tokens: 0.0,
            max_context_tokens: default_tier_max_context(),
        }
    }
}

// ── PermissionsConfig ────────────────────────────────────────────────────

/// Container for permission level defaults and per-user/channel overrides.
///
/// The three built-in levels (`zero_trust`, `user`, `admin`) are stored as
/// named fields. Per-user and per-channel overrides are stored in HashMaps
/// keyed by sender ID or channel name.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    /// Level 0 (zero-trust) permission defaults.
    #[serde(default)]
    pub zero_trust: PermissionLevelConfig,

    /// Level 1 (user) permission defaults.
    #[serde(default)]
    pub user: PermissionLevelConfig,

    /// Level 2 (admin) permission defaults.
    #[serde(default)]
    pub admin: PermissionLevelConfig,

    /// Per-user permission overrides, keyed by sender ID.
    /// Values override the level defaults for that specific user.
    #[serde(default)]
    pub users: HashMap<String, PermissionLevelConfig>,

    /// Per-channel permission overrides, keyed by channel name.
    /// Values override the level defaults for all users in that channel.
    #[serde(default)]
    pub channels: HashMap<String, PermissionLevelConfig>,
}

// ── PermissionLevelConfig ────────────────────────────────────────────────

/// Configuration for a single permission level or override.
///
/// When used as a level default (e.g., `zero_trust`, `user`, `admin`), all
/// fields are meaningful. When used as a per-user or per-channel override,
/// typically only a subset of fields is specified -- the rest inherit from
/// the user's resolved level defaults.
///
/// All fields are `Option` to support partial overrides (only set the fields
/// you want to change). The router resolves the effective permissions by
/// layering: built-in defaults < level config < channel override < user override.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionLevelConfig {
    /// Permission level (0 = zero_trust, 1 = user, 2 = admin).
    #[serde(default)]
    pub level: Option<u8>,

    /// Maximum model tier this user can access.
    /// Must match a tier name from the routing config.
    #[serde(default, alias = "maxTier")]
    pub max_tier: Option<String>,

    /// Explicit model allowlist. Empty = all models in allowed tiers.
    /// Supports glob patterns: `"anthropic/*"`, `"openrouter/meta-llama/*"`.
    #[serde(default, alias = "modelAccess")]
    pub model_access: Option<Vec<String>>,

    /// Explicit model denylist. Checked after allowlist.
    #[serde(default, alias = "modelDenylist")]
    pub model_denylist: Option<Vec<String>>,

    /// Tool names this user can invoke. `["*"]` = all tools.
    #[serde(default, alias = "toolAccess")]
    pub tool_access: Option<Vec<String>>,

    /// Tool names explicitly denied even if tool_access allows.
    #[serde(default, alias = "toolDenylist")]
    pub tool_denylist: Option<Vec<String>>,

    /// Maximum input context tokens.
    #[serde(default, alias = "maxContextTokens")]
    pub max_context_tokens: Option<usize>,

    /// Maximum output tokens per response.
    #[serde(default, alias = "maxOutputTokens")]
    pub max_output_tokens: Option<usize>,

    /// Rate limit in requests per minute. 0 = unlimited.
    #[serde(default, alias = "rateLimit")]
    pub rate_limit: Option<u32>,

    /// Whether SSE streaming responses are allowed.
    #[serde(default, alias = "streamingAllowed")]
    pub streaming_allowed: Option<bool>,

    /// Whether complexity-based escalation to a higher tier is allowed.
    #[serde(default, alias = "escalationAllowed")]
    pub escalation_allowed: Option<bool>,

    /// Complexity threshold (0.0-1.0) above which escalation triggers.
    #[serde(default, alias = "escalationThreshold")]
    pub escalation_threshold: Option<f32>,

    /// Whether the user can manually override model selection.
    #[serde(default, alias = "modelOverride")]
    pub model_override: Option<bool>,

    /// Daily cost budget in USD. 0.0 = unlimited.
    #[serde(default, alias = "costBudgetDailyUsd")]
    pub cost_budget_daily_usd: Option<f64>,

    /// Monthly cost budget in USD. 0.0 = unlimited.
    #[serde(default, alias = "costBudgetMonthlyUsd")]
    pub cost_budget_monthly_usd: Option<f64>,

    /// Extensible custom permission dimensions.
    #[serde(default, alias = "customPermissions")]
    pub custom_permissions: Option<HashMap<String, serde_json::Value>>,
}

// ── UserPermissions ──────────────────────────────────────────────────────

/// Resolved user permission capabilities.
///
/// This is the **runtime** permission object produced by layering:
/// built-in defaults + level config + channel override + user override.
/// Unlike `PermissionLevelConfig` (which uses `Option` for partial overrides),
/// all fields here are concrete values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// Permission level (0 = zero_trust, 1 = user, 2 = admin).
    #[serde(default)]
    pub level: u8,

    /// Maximum model tier this user can access.
    #[serde(default, alias = "maxTier")]
    pub max_tier: String,

    /// Explicit model allowlist. Empty = all models in allowed tiers.
    #[serde(default, alias = "modelAccess")]
    pub model_access: Vec<String>,

    /// Explicit model denylist.
    #[serde(default, alias = "modelDenylist")]
    pub model_denylist: Vec<String>,

    /// Tool names this user can invoke. `["*"]` = all tools.
    #[serde(default, alias = "toolAccess")]
    pub tool_access: Vec<String>,

    /// Tool names explicitly denied.
    #[serde(default, alias = "toolDenylist")]
    pub tool_denylist: Vec<String>,

    /// Maximum input context tokens.
    #[serde(default = "default_max_context_tokens", alias = "maxContextTokens")]
    pub max_context_tokens: usize,

    /// Maximum output tokens per response.
    #[serde(default = "default_max_output_tokens", alias = "maxOutputTokens")]
    pub max_output_tokens: usize,

    /// Rate limit in requests per minute. 0 = unlimited.
    #[serde(default = "default_rate_limit", alias = "rateLimit")]
    pub rate_limit: u32,

    /// Whether SSE streaming responses are allowed.
    #[serde(default, alias = "streamingAllowed")]
    pub streaming_allowed: bool,

    /// Whether complexity-based escalation is allowed.
    #[serde(default, alias = "escalationAllowed")]
    pub escalation_allowed: bool,

    /// Complexity threshold (0.0-1.0) above which escalation triggers.
    #[serde(default = "default_escalation_threshold", alias = "escalationThreshold")]
    pub escalation_threshold: f32,

    /// Whether the user can manually override model selection.
    #[serde(default, alias = "modelOverride")]
    pub model_override: bool,

    /// Daily cost budget in USD. 0.0 = unlimited.
    /// Zero-trust default: $0.10/day (see design doc Section 2.2).
    #[serde(default = "default_cost_budget_daily_usd", alias = "costBudgetDailyUsd")]
    pub cost_budget_daily_usd: f64,

    /// Monthly cost budget in USD. 0.0 = unlimited.
    /// Zero-trust default: $2.00/month (see design doc Section 2.2).
    #[serde(default = "default_cost_budget_monthly_usd", alias = "costBudgetMonthlyUsd")]
    pub cost_budget_monthly_usd: f64,

    /// Extensible custom permission dimensions.
    #[serde(default, alias = "customPermissions")]
    pub custom_permissions: HashMap<String, serde_json::Value>,
}

fn default_cost_budget_daily_usd() -> f64 {
    0.10
}

fn default_cost_budget_monthly_usd() -> f64 {
    2.00
}

fn default_max_context_tokens() -> usize {
    4096
}

fn default_max_output_tokens() -> usize {
    1024
}

fn default_rate_limit() -> u32 {
    10
}

fn default_escalation_threshold() -> f32 {
    1.0
}

impl Default for UserPermissions {
    /// Returns zero-trust defaults. All values are restrictive.
    /// `cost_budget_daily_usd` = $0.10, `cost_budget_monthly_usd` = $2.00
    /// per design doc Section 2.2 (NOT 0.0, which would mean unlimited).
    fn default() -> Self {
        Self {
            level: 0,
            max_tier: "free".into(),
            model_access: Vec::new(),
            model_denylist: Vec::new(),
            tool_access: Vec::new(),
            tool_denylist: Vec::new(),
            max_context_tokens: default_max_context_tokens(),
            max_output_tokens: default_max_output_tokens(),
            rate_limit: default_rate_limit(),
            streaming_allowed: false,
            escalation_allowed: false,
            escalation_threshold: default_escalation_threshold(),
            model_override: false,
            cost_budget_daily_usd: default_cost_budget_daily_usd(),
            cost_budget_monthly_usd: default_cost_budget_monthly_usd(),
            custom_permissions: HashMap::new(),
        }
    }
}

// ── AuthContext ───────────────────────────────────────────────────────────

/// Authentication context threaded through the request pipeline.
///
/// Attached to `ChatRequest` by the agent loop after resolving the sender's
/// identity from channel authentication. When absent, the router defaults
/// to zero-trust permissions.
///
/// `Default` returns zero-trust values (empty sender_id, empty channel,
/// zero-trust permissions). For CLI use, call `AuthContext::cli_default()`
/// which sets sender_id="local", channel="cli", and admin permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// Unique sender identifier (platform-specific).
    /// Telegram: user ID, Slack: user ID, Discord: user ID, CLI: `"local"`.
    #[serde(default, alias = "senderId")]
    pub sender_id: String,

    /// Channel name the request originated from.
    #[serde(default)]
    pub channel: String,

    /// Resolved permissions for this sender.
    #[serde(default)]
    pub permissions: UserPermissions,
}

impl Default for AuthContext {
    /// Returns zero-trust defaults: empty sender_id, empty channel,
    /// zero-trust permissions. This is intentionally restrictive --
    /// unauthenticated requests get minimal access.
    fn default() -> Self {
        Self {
            sender_id: String::new(),
            channel: String::new(),
            permissions: UserPermissions::default(),
        }
    }
}

impl AuthContext {
    /// Convenience constructor for CLI use. Sets `sender_id = "local"`,
    /// `channel = "cli"`, and admin-level permissions. This is NOT the
    /// Default -- callers must explicitly opt into CLI privileges.
    pub fn cli_default() -> Self {
        Self {
            sender_id: "local".into(),
            channel: "cli".into(),
            permissions: UserPermissions {
                level: 2,
                max_tier: "elite".into(),
                tool_access: vec!["*".into()],
                max_context_tokens: 200_000,
                max_output_tokens: 16_384,
                rate_limit: 0,
                streaming_allowed: true,
                escalation_allowed: true,
                escalation_threshold: 0.0,
                model_override: true,
                cost_budget_daily_usd: 0.0,   // unlimited for CLI admin
                cost_budget_monthly_usd: 0.0,  // unlimited for CLI admin
                ..UserPermissions::default()
            },
        }
    }
}

// ── EscalationConfig ─────────────────────────────────────────────────────

/// Controls complexity-based escalation to higher model tiers.
///
/// When a request's complexity exceeds the user's allowed tier range and
/// escalation is enabled, the router may promote the request to the next
/// higher tier (subject to `max_escalation_tiers` limit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// Whether escalation is enabled globally.
    #[serde(default)]
    pub enabled: bool,

    /// Default complexity threshold for escalation (0.0-1.0).
    /// Per-user thresholds in `PermissionLevelConfig` override this.
    #[serde(default = "default_global_escalation_threshold")]
    pub threshold: f32,

    /// Maximum number of tiers a request can escalate beyond the user's `max_tier`.
    /// 1 = one tier above max (e.g., `standard` -> `premium`).
    #[serde(default = "default_max_escalation_tiers", alias = "maxEscalationTiers")]
    pub max_escalation_tiers: u32,
}

fn default_global_escalation_threshold() -> f32 {
    0.6
}

fn default_max_escalation_tiers() -> u32 {
    1
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: default_global_escalation_threshold(),
            max_escalation_tiers: default_max_escalation_tiers(),
        }
    }
}

// ── CostBudgetConfig ─────────────────────────────────────────────────────

/// Global cost budget settings.
///
/// These are system-wide limits that apply regardless of individual user
/// budgets. When the global daily or monthly limit is reached, all requests
/// fall back to the cheapest available tier or are rejected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBudgetConfig {
    /// Global daily spending limit in USD. 0.0 = unlimited.
    #[serde(default, alias = "globalDailyLimitUsd")]
    pub global_daily_limit_usd: f64,

    /// Global monthly spending limit in USD. 0.0 = unlimited.
    #[serde(default, alias = "globalMonthlyLimitUsd")]
    pub global_monthly_limit_usd: f64,

    /// Whether to persist cost tracking data to disk.
    #[serde(default, alias = "trackingPersistence")]
    pub tracking_persistence: bool,

    /// Hour (UTC) at which daily budgets reset. 0 = midnight UTC.
    #[serde(default, alias = "resetHourUtc")]
    pub reset_hour_utc: u8,
}

impl Default for CostBudgetConfig {
    fn default() -> Self {
        Self {
            global_daily_limit_usd: 0.0,
            global_monthly_limit_usd: 0.0,
            tracking_persistence: false,
            reset_hour_utc: 0,
        }
    }
}

// ── RateLimitConfig ──────────────────────────────────────────────────────

/// Rate limiting configuration.
///
/// Controls the sliding-window rate limiter that enforces per-user request
/// limits. The window size and strategy are global; per-user limits are
/// defined in `PermissionLevelConfig.rate_limit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Window size in seconds for rate limit calculations.
    #[serde(default = "default_window_seconds", alias = "windowSeconds")]
    pub window_seconds: u32,

    /// Rate limiting strategy: `"sliding_window"` (default) or `"fixed_window"`.
    #[serde(default = "default_rate_limit_strategy")]
    pub strategy: String,

    /// Global rate limit in requests per minute across ALL users.
    /// 0 = unlimited (no global cap). Checked before per-user limits.
    #[serde(default, alias = "globalRateLimitRpm")]
    pub global_rate_limit_rpm: u32,
}

fn default_window_seconds() -> u32 {
    60
}

fn default_rate_limit_strategy() -> String {
    "sliding_window".into()
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            window_seconds: default_window_seconds(),
            strategy: default_rate_limit_strategy(),
            global_rate_limit_rpm: 0,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ... (see Section 4 - Refinement for full test plan)
}
```

### 2.2 Changes to `crates/clawft-types/src/config.rs`

Add the import and new field to the root `Config` struct:

```rust
// At the top of config.rs, add the import alongside the existing delegation import:
use crate::routing::RoutingConfig;

// In the Config struct, add this field after `delegation`:
    /// Tiered routing and permission configuration.
    #[serde(default)]
    pub routing: RoutingConfig,
```

This is a two-line change to `config.rs`. No other modifications to existing types.

### 2.3 Changes to `crates/clawft-types/src/lib.rs`

Add the module declaration:

```rust
pub mod routing;
```

This is a one-line addition alongside the existing module declarations.

### 2.4 Test Fixture: `tests/fixtures/config_tiered.json`

A complete config file exercising all routing fields (taken from design doc Section 7.1
with the existing config fixture fields included):

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-20250514",
      "maxTokens": 8192
    }
  },
  "providers": {
    "anthropic": { "apiKey": "sk-ant-test-key" },
    "openai": { "apiKey": "sk-test-key" },
    "openrouter": { "apiKey": "sk-or-test-key", "apiBase": "https://openrouter.ai/api/v1" },
    "groq": { "apiKey": "gsk-test-key" }
  },
  "routing": {
    "mode": "tiered",
    "tiers": [
      {
        "name": "free",
        "models": [
          "openrouter/meta-llama/llama-3.1-8b-instruct:free",
          "groq/llama-3.1-8b"
        ],
        "complexity_range": [0.0, 0.3],
        "cost_per_1k_tokens": 0.0,
        "max_context_tokens": 8192
      },
      {
        "name": "standard",
        "models": [
          "anthropic/claude-haiku-3.5",
          "openai/gpt-4o-mini",
          "groq/llama-3.3-70b"
        ],
        "complexity_range": [0.0, 0.7],
        "cost_per_1k_tokens": 0.001,
        "max_context_tokens": 16384
      },
      {
        "name": "premium",
        "models": [
          "anthropic/claude-sonnet-4-20250514",
          "openai/gpt-4o"
        ],
        "complexity_range": [0.3, 1.0],
        "cost_per_1k_tokens": 0.01,
        "max_context_tokens": 200000
      },
      {
        "name": "elite",
        "models": [
          "anthropic/claude-opus-4-5",
          "openai/o1"
        ],
        "complexity_range": [0.7, 1.0],
        "cost_per_1k_tokens": 0.05,
        "max_context_tokens": 200000
      }
    ],
    "selection_strategy": "preference_order",
    "fallback_model": "groq/llama-3.1-8b",
    "permissions": {
      "zero_trust": {
        "level": 0,
        "max_tier": "free",
        "tool_access": [],
        "max_context_tokens": 4096,
        "max_output_tokens": 1024,
        "rate_limit": 10,
        "streaming_allowed": false,
        "escalation_allowed": false,
        "escalation_threshold": 1.0,
        "model_override": false,
        "cost_budget_daily_usd": 0.10,
        "cost_budget_monthly_usd": 2.00
      },
      "user": {
        "level": 1,
        "max_tier": "standard",
        "tool_access": [
          "read_file", "write_file", "edit_file", "list_dir",
          "web_search", "web_fetch", "message"
        ],
        "max_context_tokens": 16384,
        "max_output_tokens": 4096,
        "rate_limit": 60,
        "streaming_allowed": true,
        "escalation_allowed": true,
        "escalation_threshold": 0.6,
        "model_override": false,
        "cost_budget_daily_usd": 5.00,
        "cost_budget_monthly_usd": 100.00
      },
      "admin": {
        "level": 2,
        "max_tier": "elite",
        "tool_access": ["*"],
        "max_context_tokens": 200000,
        "max_output_tokens": 16384,
        "rate_limit": 0,
        "streaming_allowed": true,
        "escalation_allowed": true,
        "escalation_threshold": 0.0,
        "model_override": true,
        "cost_budget_daily_usd": 0.0,
        "cost_budget_monthly_usd": 0.0
      },
      "users": {
        "alice_telegram_123": {
          "level": 2
        },
        "bob_discord_456": {
          "level": 1,
          "cost_budget_daily_usd": 2.00,
          "tool_access": ["read_file", "list_dir", "web_search"]
        }
      },
      "channels": {
        "cli": { "level": 2 },
        "telegram": { "level": 1 },
        "discord": { "level": 0 }
      }
    },
    "escalation": {
      "enabled": true,
      "threshold": 0.6,
      "max_escalation_tiers": 1
    },
    "cost_budgets": {
      "global_daily_limit_usd": 50.0,
      "global_monthly_limit_usd": 500.0,
      "tracking_persistence": true,
      "reset_hour_utc": 0
    },
    "rate_limiting": {
      "window_seconds": 60,
      "strategy": "sliding_window"
    }
  }
}
```

---

## 3. Architecture

### 3.1 File Locations

| Component | File | Action |
|-----------|------|--------|
| `RoutingConfig`, `TierSelectionStrategy`, `ModelTierConfig`, `PermissionsConfig`, `PermissionLevelConfig`, `UserPermissions`, `AuthContext`, `EscalationConfig`, `CostBudgetConfig`, `RateLimitConfig` | `crates/clawft-types/src/routing.rs` | **New file** |
| `Config` struct (add `routing` field) | `crates/clawft-types/src/config.rs` | **Edit** (2 lines) |
| Module declaration | `crates/clawft-types/src/lib.rs` | **Edit** (1 line) |
| Test fixture | `tests/fixtures/config_tiered.json` | **New file** |

### 3.2 Module Structure

```
crates/clawft-types/src/
    lib.rs              # Add: pub mod routing;
    config.rs           # Add: use crate::routing::RoutingConfig; + field on Config
    routing.rs          # NEW: all routing/permission config types (~350 lines)
    delegation.rs       # Unchanged (pattern reference)
    error.rs            # Unchanged
    event.rs            # Unchanged
    provider.rs         # Unchanged
    session.rs          # Unchanged
    cron.rs             # Unchanged
    skill.rs            # Unchanged
    workspace.rs        # Unchanged
```

### 3.3 Integration with Root Config

The `Config` struct gains `routing: RoutingConfig` with `#[serde(default)]`. This follows
the exact same pattern as `delegation: DelegationConfig`:

```rust
// config.rs (current)
use crate::delegation::DelegationConfig;

pub struct Config {
    pub agents: AgentsConfig,
    pub channels: ChannelsConfig,
    pub providers: ProvidersConfig,
    pub gateway: GatewayConfig,
    pub tools: ToolsConfig,
    pub delegation: DelegationConfig,
}

// config.rs (after Phase A)
use crate::delegation::DelegationConfig;
use crate::routing::RoutingConfig;

pub struct Config {
    pub agents: AgentsConfig,
    pub channels: ChannelsConfig,
    pub providers: ProvidersConfig,
    pub gateway: GatewayConfig,
    pub tools: ToolsConfig,
    pub delegation: DelegationConfig,
    pub routing: RoutingConfig,         // <-- new
}
```

### 3.4 Type Relationship Diagram

```
Config
  |
  +-- routing: RoutingConfig
        |
        +-- mode: String
        +-- tiers: Vec<ModelTierConfig>
        |     |
        |     +-- name, models, complexity_range, cost_per_1k_tokens, max_context_tokens
        |
        +-- selection_strategy: Option<TierSelectionStrategy>
        +-- fallback_model: Option<String>
        +-- permissions: PermissionsConfig
        |     |
        |     +-- zero_trust: PermissionLevelConfig
        |     +-- user: PermissionLevelConfig
        |     +-- admin: PermissionLevelConfig
        |     +-- users: HashMap<String, PermissionLevelConfig>
        |     +-- channels: HashMap<String, PermissionLevelConfig>
        |
        +-- escalation: EscalationConfig
        |     |
        |     +-- enabled, threshold, max_escalation_tiers
        |
        +-- cost_budgets: CostBudgetConfig
        |     |
        |     +-- global_daily_limit_usd, global_monthly_limit_usd,
        |         tracking_persistence, reset_hour_utc
        |
        +-- rate_limiting: RateLimitConfig
              |
              +-- window_seconds, strategy, global_rate_limit_rpm

UserPermissions (resolved at runtime, not part of config tree)
  |
  +-- level, max_tier, model_access, model_denylist, tool_access, tool_denylist,
      max_context_tokens, max_output_tokens, rate_limit, streaming_allowed,
      escalation_allowed, escalation_threshold, model_override,
      cost_budget_daily_usd, cost_budget_monthly_usd, custom_permissions

AuthContext (attached to ChatRequest at runtime)
  |
  +-- sender_id, channel, permissions: UserPermissions
```

### 3.5 Design Decisions

1. **`PermissionLevelConfig` uses `Option<T>` for all fields** -- This enables partial
   overrides. A per-user config like `{"level": 2}` only sets the level; all other
   fields inherit from the level's defaults. The `UserPermissions` struct (with concrete
   non-Option fields) is produced by the resolver in Phase B.

2. **`PermissionsConfig` uses named fields for levels, not a HashMap** -- The three
   built-in levels (`zero_trust`, `user`, `admin`) are stable enough to be named fields.
   This provides better type safety and IDE support than `HashMap<String, PermissionLevelConfig>`.
   Per-user and per-channel overrides use HashMaps because the keys are dynamic.

3. **Separate `routing.rs` file** -- Following the `delegation.rs` pattern, routing
   types get their own module rather than being added to the already-long `config.rs`
   (~1350 lines). This keeps files under 500 lines per CLAUDE.md rules.

4. **`UserPermissions` is separate from `PermissionLevelConfig`** -- The config type
   uses `Option<T>` for partial overrides; the runtime type uses concrete values. This
   separation prevents confusion between "not specified" (inherit from level) and
   "explicitly set to default value."

5. **`AuthContext` is in routing.rs, not a separate file** -- It is small (3 fields)
   and tightly coupled to `UserPermissions`. No need for a separate module.

---

## 4. Refinement

### 4.1 Edge Cases

| Edge Case | Expected Behavior |
|-----------|-------------------|
| Empty `tiers` array | Valid config; `TieredRouter` has no tiers to select from, falls back to `fallback_model` |
| Overlapping complexity ranges across tiers | Valid and intentional; router picks highest-quality allowed tier |
| `complexity_range` where min > max | Should be caught at validation time (Phase H), not at parse time |
| Missing permission level (e.g., no `user` key in `permissions`) | `PermissionLevelConfig::default()` (all `None`) -- resolver in Phase B applies built-in defaults |
| `rate_limit: 0` | Interpreted as unlimited (no rate limiting) |
| `cost_budget_daily_usd: 0.0` | Interpreted as unlimited (no budget cap). Note: `UserPermissions::default()` uses $0.10/day (zero-trust), NOT 0.0 |
| `escalation_threshold: 0.0` in admin config | Escalation triggers at any complexity (correct for admin) |
| `escalation_threshold: 1.0` in zero_trust | Effectively disables escalation (complexity never reaches 1.0) |
| Per-user override with only `level` field | All other fields inherit from the specified level's defaults |
| `tool_access: ["*"]` | Grants access to all tools (wildcard) |
| Unknown tier names in `max_tier` | Valid at parse time; router in Phase C handles gracefully |
| `custom_permissions` with nested JSON | Supported via `HashMap<String, serde_json::Value>` |

### 4.2 Validation Rules

**Parse-time (Phase A -- what serde enforces automatically):**
- JSON type correctness (string for strings, number for numbers, etc.)
- Array structure for `complexity_range` (must be exactly 2 elements as `[f32; 2]`)
- Unknown fields silently ignored

**Runtime validation (Phase H -- NOT this phase):**
- `complexity_range[0] <= complexity_range[1]`
- `max_tier` references a tier name that exists in `tiers`
- `level` is 0, 1, or 2
- `escalation_threshold` is in [0.0, 1.0]
- `cost_budget_daily_usd >= 0.0`
- At least one model per tier when mode is `"tiered"`
- No duplicate tier names

This phase does NOT add validation logic. Types are pure data containers.

### 4.3 Backward Compatibility

| Scenario | Verification |
|----------|-------------|
| Existing `config.json` without `routing` key | `RoutingConfig::default()` is used; `mode = "static"` |
| Existing test fixture `tests/fixtures/config.json` | All existing tests in `config.rs` continue to pass unchanged |
| JSON with `routing: {}` (empty object) | `RoutingConfig::default()` -- all fields get defaults |
| JSON with partial `routing` (e.g., only `mode`) | Specified fields used, rest get defaults |

### 4.4 Migration Path

No migration needed. Adding `#[serde(default)]` means:
- Old configs (no `routing`) work unchanged
- New configs can add `routing` section incrementally
- Partial `routing` sections are valid

### 4.5 Potential Pitfalls

1. **`PermissionLevelConfig` vs `UserPermissions` confusion** -- Document clearly that
   `PermissionLevelConfig` is the config-file type (Options for partial overrides) and
   `UserPermissions` is the resolved-at-runtime type (concrete values). Phase B implements
   the resolver.

2. **`serde(alias)` only works for deserialization** -- Serialization always uses the
   Rust field name (snake_case). This matches existing behavior in `config.rs`.

3. **`[f32; 2]` serde behavior** -- Deserializes from JSON array `[0.0, 0.3]`. If the
   array has wrong length, serde returns an error. This is correct behavior.

4. **HashMap ordering** -- `users` and `channels` HashMaps have no guaranteed order.
   This is fine since order doesn't matter for permission resolution.

---

## 5. Completion

### 5.1 Exit Criteria

- [ ] New file `crates/clawft-types/src/routing.rs` compiles with zero warnings
- [ ] `crates/clawft-types/src/lib.rs` declares `pub mod routing;`
- [ ] `Config` struct in `config.rs` has `routing: RoutingConfig` with `#[serde(default)]`
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes (all existing tests + new tests)
- [ ] Test fixture `tests/fixtures/config_tiered.json` exists and is used by tests

### 5.2 Test Plan

At least 18 unit tests covering the following scenarios:

```
Test #  | Name                                     | What it verifies
--------|------------------------------------------|------------------------------------------
  1     | routing_config_defaults                  | RoutingConfig::default() has mode="static", empty tiers, etc.
  2     | model_tier_config_defaults               | ModelTierConfig::default() has range [0,1], 8192 context
  3     | permission_level_config_defaults          | PermissionLevelConfig::default() has all None fields
  4     | user_permissions_defaults                 | UserPermissions::default() matches zero-trust built-in ($0.10/day, $2.00/month)
  5     | auth_context_defaults                     | AuthContext::default() has empty strings + zero-trust perms
  6     | auth_context_cli_default                  | AuthContext::cli_default() has "local"/"cli" + admin perms
  7     | escalation_config_defaults               | threshold=0.6, max_tiers=1, enabled=false
  8     | cost_budget_config_defaults              | All limits 0.0, persistence false, reset_hour 0
  9     | rate_limit_config_defaults               | 60s window, sliding_window strategy, global_rate_limit_rpm=0
 10     | tier_selection_strategy_serde            | TierSelectionStrategy round-trips as snake_case strings
 11     | deserialize_full_tiered_config           | Full JSON from Section 7.1 deserializes correctly
 12     | serde_roundtrip_routing_config           | Serialize -> deserialize -> values match
 13     | camel_case_aliases                       | JSON with camelCase keys deserializes correctly
 14     | unknown_fields_ignored                   | JSON with unknown fields in routing section doesn't error
 15     | empty_routing_section                    | `"routing": {}` deserializes to defaults
 16     | backward_compat_no_routing               | Existing config.json without routing still works
 17     | per_user_partial_override                | Per-user config with only `level` deserializes correctly
 18     | complexity_range_array                   | `[0.3, 0.7]` deserializes to [f32; 2]
```

### 5.3 Test Implementation Sketch

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const TIERED_FIXTURE_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/config_tiered.json"
    );

    fn load_tiered_fixture() -> crate::config::Config {
        let content = std::fs::read_to_string(TIERED_FIXTURE_PATH)
            .expect("config_tiered.json fixture should exist");
        serde_json::from_str(&content).expect("tiered fixture should deserialize")
    }

    #[test]
    fn routing_config_defaults() {
        let cfg = RoutingConfig::default();
        assert_eq!(cfg.mode, "static");
        assert!(cfg.tiers.is_empty());
        assert!(cfg.selection_strategy.is_none());
        assert!(cfg.fallback_model.is_none());
        assert!(!cfg.escalation.enabled);
    }

    #[test]
    fn model_tier_config_defaults() {
        let cfg = ModelTierConfig::default();
        assert!(cfg.name.is_empty());
        assert!(cfg.models.is_empty());
        assert_eq!(cfg.complexity_range, [0.0, 1.0]);
        assert_eq!(cfg.cost_per_1k_tokens, 0.0);
        assert_eq!(cfg.max_context_tokens, 8192);
    }

    #[test]
    fn permission_level_config_defaults() {
        let cfg = PermissionLevelConfig::default();
        assert!(cfg.level.is_none());
        assert!(cfg.max_tier.is_none());
        assert!(cfg.tool_access.is_none());
        assert!(cfg.rate_limit.is_none());
        assert!(cfg.streaming_allowed.is_none());
    }

    #[test]
    fn user_permissions_defaults() {
        let perms = UserPermissions::default();
        assert_eq!(perms.level, 0);
        assert_eq!(perms.max_tier, "free");
        assert!(perms.tool_access.is_empty());
        assert_eq!(perms.max_context_tokens, 4096);
        assert_eq!(perms.max_output_tokens, 1024);
        assert_eq!(perms.rate_limit, 10);
        assert!(!perms.streaming_allowed);
        assert!(!perms.escalation_allowed);
        assert!((perms.escalation_threshold - 1.0).abs() < f32::EPSILON);
        assert!(!perms.model_override);
        // Zero-trust budgets: $0.10/day, $2.00/month (NOT 0.0 which means unlimited)
        assert!((perms.cost_budget_daily_usd - 0.10).abs() < f64::EPSILON);
        assert!((perms.cost_budget_monthly_usd - 2.00).abs() < f64::EPSILON);
        assert!(perms.custom_permissions.is_empty());
    }

    #[test]
    fn auth_context_defaults() {
        let ctx = AuthContext::default();
        assert!(ctx.sender_id.is_empty());
        assert!(ctx.channel.is_empty());
        assert_eq!(ctx.permissions.level, 0);
        // Verify zero-trust budgets propagate through AuthContext::default()
        assert!((ctx.permissions.cost_budget_daily_usd - 0.10).abs() < f64::EPSILON);
        assert!((ctx.permissions.cost_budget_monthly_usd - 2.00).abs() < f64::EPSILON);
    }

    #[test]
    fn auth_context_cli_default() {
        let ctx = AuthContext::cli_default();
        assert_eq!(ctx.sender_id, "local");
        assert_eq!(ctx.channel, "cli");
        assert_eq!(ctx.permissions.level, 2);
        assert_eq!(ctx.permissions.max_tier, "elite");
        assert_eq!(ctx.permissions.tool_access, vec!["*"]);
        assert_eq!(ctx.permissions.max_context_tokens, 200_000);
        assert_eq!(ctx.permissions.max_output_tokens, 16_384);
        assert_eq!(ctx.permissions.rate_limit, 0); // unlimited
        assert!(ctx.permissions.streaming_allowed);
        assert!(ctx.permissions.escalation_allowed);
        assert!((ctx.permissions.escalation_threshold - 0.0).abs() < f32::EPSILON);
        assert!(ctx.permissions.model_override);
        assert_eq!(ctx.permissions.cost_budget_daily_usd, 0.0);   // unlimited for CLI admin
        assert_eq!(ctx.permissions.cost_budget_monthly_usd, 0.0);  // unlimited for CLI admin
    }

    #[test]
    fn tier_selection_strategy_serde() {
        // Verify snake_case serialization
        let json = serde_json::to_string(&TierSelectionStrategy::PreferenceOrder).unwrap();
        assert_eq!(json, "\"preference_order\"");

        let json = serde_json::to_string(&TierSelectionStrategy::RoundRobin).unwrap();
        assert_eq!(json, "\"round_robin\"");

        let json = serde_json::to_string(&TierSelectionStrategy::LowestCost).unwrap();
        assert_eq!(json, "\"lowest_cost\"");

        let json = serde_json::to_string(&TierSelectionStrategy::Random).unwrap();
        assert_eq!(json, "\"random\"");

        // Verify deserialization
        let strategy: TierSelectionStrategy =
            serde_json::from_str("\"preference_order\"").unwrap();
        assert_eq!(strategy, TierSelectionStrategy::PreferenceOrder);

        let strategy: TierSelectionStrategy =
            serde_json::from_str("\"round_robin\"").unwrap();
        assert_eq!(strategy, TierSelectionStrategy::RoundRobin);

        // Verify invalid value is rejected
        let result = serde_json::from_str::<TierSelectionStrategy>("\"invalid_strategy\"");
        assert!(result.is_err());
    }

    #[test]
    fn escalation_config_defaults() {
        let cfg = EscalationConfig::default();
        assert!(!cfg.enabled);
        assert!((cfg.threshold - 0.6).abs() < f32::EPSILON);
        assert_eq!(cfg.max_escalation_tiers, 1);
    }

    #[test]
    fn cost_budget_config_defaults() {
        let cfg = CostBudgetConfig::default();
        assert_eq!(cfg.global_daily_limit_usd, 0.0);
        assert_eq!(cfg.global_monthly_limit_usd, 0.0);
        assert!(!cfg.tracking_persistence);
        assert_eq!(cfg.reset_hour_utc, 0);
    }

    #[test]
    fn rate_limit_config_defaults() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.window_seconds, 60);
        assert_eq!(cfg.strategy, "sliding_window");
        assert_eq!(cfg.global_rate_limit_rpm, 0); // 0 = unlimited
    }

    #[test]
    fn deserialize_full_tiered_config() {
        let cfg = load_tiered_fixture();
        let routing = &cfg.routing;

        // Mode
        assert_eq!(routing.mode, "tiered");

        // Tiers
        assert_eq!(routing.tiers.len(), 4);
        assert_eq!(routing.tiers[0].name, "free");
        assert_eq!(routing.tiers[0].models.len(), 2);
        assert_eq!(routing.tiers[0].complexity_range, [0.0, 0.3]);
        assert_eq!(routing.tiers[0].cost_per_1k_tokens, 0.0);
        assert_eq!(routing.tiers[0].max_context_tokens, 8192);

        assert_eq!(routing.tiers[1].name, "standard");
        assert_eq!(routing.tiers[2].name, "premium");
        assert_eq!(routing.tiers[3].name, "elite");
        assert_eq!(routing.tiers[3].cost_per_1k_tokens, 0.05);
        assert_eq!(routing.tiers[3].max_context_tokens, 200000);

        // Selection strategy and fallback
        assert_eq!(routing.selection_strategy, Some(TierSelectionStrategy::PreferenceOrder));
        assert_eq!(routing.fallback_model.as_deref(), Some("groq/llama-3.1-8b"));

        // Permissions - zero_trust
        let zt = &routing.permissions.zero_trust;
        assert_eq!(zt.level, Some(0));
        assert_eq!(zt.max_tier.as_deref(), Some("free"));
        assert_eq!(zt.tool_access.as_ref().map(|v| v.len()), Some(0));
        assert_eq!(zt.max_context_tokens, Some(4096));
        assert_eq!(zt.max_output_tokens, Some(1024));
        assert_eq!(zt.rate_limit, Some(10));
        assert_eq!(zt.streaming_allowed, Some(false));
        assert_eq!(zt.escalation_allowed, Some(false));

        // Permissions - user
        let u = &routing.permissions.user;
        assert_eq!(u.level, Some(1));
        assert_eq!(u.max_tier.as_deref(), Some("standard"));
        assert_eq!(u.tool_access.as_ref().map(|v| v.len()), Some(7));
        assert_eq!(u.streaming_allowed, Some(true));
        assert_eq!(u.escalation_allowed, Some(true));

        // Permissions - admin
        let a = &routing.permissions.admin;
        assert_eq!(a.level, Some(2));
        assert_eq!(a.max_tier.as_deref(), Some("elite"));
        assert_eq!(a.model_override, Some(true));
        assert_eq!(a.rate_limit, Some(0));

        // Per-user overrides
        assert!(routing.permissions.users.contains_key("alice_telegram_123"));
        assert_eq!(routing.permissions.users["alice_telegram_123"].level, Some(2));
        assert!(routing.permissions.users.contains_key("bob_discord_456"));
        assert_eq!(routing.permissions.users["bob_discord_456"].level, Some(1));
        assert_eq!(
            routing.permissions.users["bob_discord_456"].cost_budget_daily_usd,
            Some(2.00)
        );

        // Per-channel overrides
        assert_eq!(routing.permissions.channels["cli"].level, Some(2));
        assert_eq!(routing.permissions.channels["telegram"].level, Some(1));
        assert_eq!(routing.permissions.channels["discord"].level, Some(0));

        // Escalation
        assert!(routing.escalation.enabled);
        assert!((routing.escalation.threshold - 0.6).abs() < f32::EPSILON);
        assert_eq!(routing.escalation.max_escalation_tiers, 1);

        // Cost budgets
        assert_eq!(routing.cost_budgets.global_daily_limit_usd, 50.0);
        assert_eq!(routing.cost_budgets.global_monthly_limit_usd, 500.0);
        assert!(routing.cost_budgets.tracking_persistence);
        assert_eq!(routing.cost_budgets.reset_hour_utc, 0);

        // Rate limiting
        assert_eq!(routing.rate_limiting.window_seconds, 60);
        assert_eq!(routing.rate_limiting.strategy, "sliding_window");
        assert_eq!(routing.rate_limiting.global_rate_limit_rpm, 0); // absent from JSON = default 0
    }

    #[test]
    fn serde_roundtrip_routing_config() {
        let cfg = load_tiered_fixture();
        let json = serde_json::to_string(&cfg.routing).unwrap();
        let restored: RoutingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.mode, cfg.routing.mode);
        assert_eq!(restored.tiers.len(), cfg.routing.tiers.len());
        assert_eq!(restored.tiers[0].name, cfg.routing.tiers[0].name);
        assert_eq!(
            restored.fallback_model,
            cfg.routing.fallback_model
        );
        assert_eq!(
            restored.escalation.max_escalation_tiers,
            cfg.routing.escalation.max_escalation_tiers
        );
    }

    #[test]
    fn camel_case_aliases() {
        let json = r#"{
            "mode": "tiered",
            "selectionStrategy": "round_robin",
            "fallbackModel": "groq/llama-3.1-8b",
            "costBudgets": {
                "globalDailyLimitUsd": 25.0,
                "globalMonthlyLimitUsd": 250.0,
                "trackingPersistence": true,
                "resetHourUtc": 6
            },
            "rateLimiting": {
                "windowSeconds": 120
            },
            "escalation": {
                "maxEscalationTiers": 2
            }
        }"#;
        let cfg: RoutingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.selection_strategy, Some(TierSelectionStrategy::RoundRobin));
        assert_eq!(cfg.fallback_model.as_deref(), Some("groq/llama-3.1-8b"));
        assert_eq!(cfg.cost_budgets.global_daily_limit_usd, 25.0);
        assert_eq!(cfg.cost_budgets.global_monthly_limit_usd, 250.0);
        assert!(cfg.cost_budgets.tracking_persistence);
        assert_eq!(cfg.cost_budgets.reset_hour_utc, 6);
        assert_eq!(cfg.rate_limiting.window_seconds, 120);
        assert_eq!(cfg.escalation.max_escalation_tiers, 2);
    }

    #[test]
    fn unknown_fields_ignored() {
        let json = r#"{
            "mode": "tiered",
            "future_field": "should be ignored",
            "escalation": {
                "enabled": true,
                "unknown_nested": 42
            },
            "tiers": [{
                "name": "test",
                "models": [],
                "complexity_range": [0.0, 1.0],
                "cost_per_1k_tokens": 0.0,
                "some_future_field": true
            }]
        }"#;
        let cfg: RoutingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.mode, "tiered");
        assert!(cfg.escalation.enabled);
        assert_eq!(cfg.tiers.len(), 1);
        assert_eq!(cfg.tiers[0].name, "test");
    }

    #[test]
    fn empty_routing_section() {
        let json = r#"{}"#;
        let cfg: RoutingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.mode, "static");
        assert!(cfg.tiers.is_empty());
        assert!(cfg.selection_strategy.is_none());
        assert!(cfg.fallback_model.is_none());
        assert!(!cfg.escalation.enabled);
    }

    #[test]
    fn backward_compat_no_routing() {
        // Existing config without routing section should still work
        let json = r#"{
            "agents": { "defaults": { "model": "anthropic/claude-opus-4-5" } },
            "providers": { "anthropic": { "apiKey": "test" } }
        }"#;
        let cfg: crate::config::Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert_eq!(cfg.routing.mode, "static");
        assert!(cfg.routing.tiers.is_empty());
    }

    #[test]
    fn per_user_partial_override() {
        let json = r#"{
            "users": {
                "alice": { "level": 2 },
                "bob": { "level": 1, "cost_budget_daily_usd": 3.50 }
            }
        }"#;
        let cfg: PermissionsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.users["alice"].level, Some(2));
        assert!(cfg.users["alice"].max_tier.is_none());
        assert!(cfg.users["alice"].tool_access.is_none());
        assert_eq!(cfg.users["bob"].level, Some(1));
        assert_eq!(cfg.users["bob"].cost_budget_daily_usd, Some(3.50));
    }

    #[test]
    fn complexity_range_array() {
        let json = r#"{
            "name": "test",
            "models": ["provider/model"],
            "complexity_range": [0.3, 0.7],
            "cost_per_1k_tokens": 0.005
        }"#;
        let tier: ModelTierConfig = serde_json::from_str(json).unwrap();
        assert_eq!(tier.complexity_range, [0.3, 0.7]);
        assert_eq!(tier.name, "test");
        assert_eq!(tier.models, vec!["provider/model"]);
        assert_eq!(tier.cost_per_1k_tokens, 0.005);
    }
}
```

### 5.4 Validation Commands

```bash
# Build
cargo build --workspace

# Lint
cargo clippy --workspace

# Test all (existing + new)
cargo test --workspace

# Test only routing module
cargo test --package clawft-types routing

# Verify existing config tests still pass
cargo test --package clawft-types config::tests
```

### 5.5 Definition of Done

1. Code compiles without warnings (`cargo clippy --workspace`)
2. All tests pass (`cargo test --workspace`)
3. No new `unsafe` blocks
4. All public types have doc comments
5. No hardcoded secrets or paths
6. Existing `tests/fixtures/config.json` tests pass unchanged
7. New `tests/fixtures/config_tiered.json` fixture is tested
8. File `routing.rs` is under 500 lines

### 5.6 What This Phase Does NOT Include

- No validation logic (Phase H)
- No permission resolution/layering logic (Phase B)
- No `TieredRouter` implementation (Phase C)
- No `CostTracker` (Phase D)
- No `RateLimiter` (Phase E)
- No `AuthContext` threading through pipeline (Phase F)
- No tool permission enforcement (Phase G)

This phase is purely additive data types with serde support and defaults.

---

## Remediation Applied

The following fixes from `remediation-plan.md` were applied to this plan on 2026-02-18:

| Fix ID | Change | Details |
|--------|--------|---------|
| **FIX-01** | `selection_strategy` type changed | `Option<String>` replaced with `Option<TierSelectionStrategy>`. New `TierSelectionStrategy` enum added with `#[serde(rename_all = "snake_case")]` and variants: `PreferenceOrder`, `RoundRobin`, `LowestCost`, `Random`. |
| **FIX-01** | `AuthContext::default()` documented as zero-trust | Already returned zero-trust values. Added explicit doc comments confirming this is intentional. |
| **FIX-01** | `AuthContext::cli_default()` added | New constructor for CLI use: `sender_id="local"`, `channel="cli"`, admin-level permissions. Callers must explicitly opt in. |
| **FIX-01** | `UserPermissions::default()` budget values corrected | `cost_budget_daily_usd` changed from `0.0` to `0.10`, `cost_budget_monthly_usd` changed from `0.0` to `2.00`. These are zero-trust values from design doc Section 2.2. `0.0` means unlimited, which is not zero-trust. |
| **FIX-01** | Type ownership confirmed | All types confirmed in `clawft-types/src/routing.rs`, exported via `pub mod routing;` in `lib.rs`. |
| **FIX-08** | `global_rate_limit_rpm` added to `RateLimitConfig` | New `u32` field with `#[serde(default)]`, default `0` (unlimited). Checked before per-user limits by Phase E. |
| **FIX-12** | "15 dimensions" check | Not applicable -- Phase A does not mention "15 dimensions". No change needed. |

Tests updated: `user_permissions_defaults`, `auth_context_defaults`, `rate_limit_config_defaults`, `deserialize_full_tiered_config`, `camel_case_aliases`. New tests added: `auth_context_cli_default`, `tier_selection_strategy_serde`. Test plan count increased from 16 to 18.

---

**End of SPARC Plan**
