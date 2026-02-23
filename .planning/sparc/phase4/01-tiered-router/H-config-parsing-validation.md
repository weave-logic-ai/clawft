# SPARC Implementation Plan: Phase H - Config Parsing & Validation

## Agent Instructions

### Context

This is Phase H of the Tiered Router sprint (01-tiered-router). The clawft project
is a Rust AI assistant framework. The CLI binary is named `weft`. Phase H is the
config validation and deep-merge phase -- it sits between the runtime components
(Phases A-G) and the integration tests (Phase I).

### Dependencies

**Depends on Phases A through G completing first.** Specifically:

- **Phase A** (`clawft-types/src/routing.rs`): `RoutingConfig`, `ModelTierConfig`,
  `PermissionsConfig`, `PermissionLevelConfig`, `EscalationConfig`, `CostBudgetConfig`,
  `RateLimitConfig` -- the raw config types being validated.
- **Phase B** (`clawft-core/src/pipeline/permissions.rs`): `UserPermissions`, `AuthContext`,
  `PermissionResolver`, `PermissionOverrides` -- defines the known permission level names
  (`zero_trust`=0, `user`=1, `admin`=2) that validation must enforce.
- **Phases C-G**: `TieredRouter`, `CostTracker`, `RateLimiter`, auth threading,
  tool permissions -- these are consumers of the validated config. They assume config
  has already been validated before construction.

### Planning Documents to Reference

- `.planning/08-tiered-router.md` Section 7: full config JSON format
- `.planning/08-tiered-router.md` Section 7.2: config type additions
- `.planning/08-tiered-router.md` Section 7.3: workspace override
- `.planning/08-tiered-router.md` Section 10: migration path
- `.planning/sparc/01-tiered-router/A-routing-config-types.md`: types being parsed
- `.planning/sparc/01-tiered-router/B-permissions-resolution.md`: permission resolution
- `crates/clawft-types/src/config.rs`: existing config parsing patterns, serde conventions

### Files to Create

- `crates/clawft-types/src/routing_validation.rs` -- validation logic (NEW)

### Files to Modify

- `crates/clawft-types/src/lib.rs` -- add `pub mod routing_validation;`
- `crates/clawft-platform/src/config_loader.rs` -- integrate validation into load path
  and add workspace deep-merge for the routing section

### Config Fixtures to Create

- `tests/fixtures/config_tiered.json` -- full valid tiered config (all fields populated)
- `tests/fixtures/config_tiered_minimal.json` -- minimal valid tiered config (mode + 2 tiers)
- `tests/fixtures/config_tiered_invalid.json` -- various invalid configs for error testing

### Branch

Work on branch: `weft/tiered-router`

---

## 1. Specification

### 1.1 Scope

Phase H adds post-deserialization validation for the `RoutingConfig` and its nested
types. Serde handles structural correctness (JSON types, array lengths, field names),
but it cannot enforce semantic constraints like "tier names must be unique" or
"complexity min must be less than max." This phase fills that gap.

Phase H also implements the workspace deep-merge for the `routing` section, default
tier construction when `mode="tiered"` but no tiers are specified, and backward
compatibility behavior where an empty or absent `routing` section means static mode.

### 1.2 Validation Requirements

All validation runs after deserialization, before `TieredRouter` construction. When
`routing.mode` is `"static"`, tier and permission validation is skipped entirely
(static mode ignores routing config). Validation only applies when `mode = "tiered"`.

#### 1.2.1 Tier Validation

| Check | Severity | Rule |
|-------|----------|------|
| Tier names unique | ERROR | No two tiers may share the same `name` value |
| complexity_range bounds | ERROR | Both `complexity_range[0]` and `complexity_range[1]` must be in `[0.0, 1.0]` |
| complexity_range ordering | ERROR | `complexity_range[0]` must be `<=` `complexity_range[1]` (min <= max) |
| cost_per_1k_tokens non-negative | ERROR | `cost_per_1k_tokens >= 0.0` |
| max_context_tokens positive | ERROR | `max_context_tokens > 0` |
| Empty models list | WARNING | Tier has no models -- it is defined but unusable |
| Model format | WARNING | Each model string should contain `/` (provider/model format) |
| Overlapping complexity ranges | WARNING | Two tiers whose ranges overlap. This is intentional by design but worth logging for operator awareness. |

#### 1.2.2 Permission Validation

| Check | Severity | Rule |
|-------|----------|------|
| Permission level known | ERROR | Level must be 0, 1, or 2. Matches `UserPermissions::level_from_name()`: `zero_trust`=0, `user`=1, `admin`=2. |
| `max_tier` references defined tier | ERROR | When `max_tier` is set, it must match a tier name from `routing.tiers` |
| `escalation_threshold` in range | ERROR | Must be in `[0.0, 1.0]` |
| `cost_budget_daily_usd` non-negative | ERROR | Must be `>= 0.0` |
| `cost_budget_monthly_usd` non-negative | ERROR | Must be `>= 0.0` |
| `rate_limit` non-negative | ERROR | Must be `>= 0` (u32, so this is always true, but validate if overridden from JSON as signed) |
| Per-user override fields valid | ERROR | Same field constraints apply to `routing.permissions.users.<id>` entries |
| Per-channel override fields valid | ERROR | Same field constraints apply to `routing.permissions.channels.<name>` entries. Only valid permission dimension fields should be present (level, max_tier, tool_access, etc.). |

#### 1.2.3 Selection Strategy Validation

**NOTE (FIX-01):** `TierSelectionStrategy` is now a typed enum (not `Option<String>`).
Serde automatically rejects unknown variants at deserialization time. No manual string
validation is needed in `validate_routing_config()`. The table below documents the
valid variants for reference only; the check is performed by serde, not by this module.

| Check | Severity | Rule |
|-------|----------|------|
| Known strategy value | HANDLED BY SERDE | `TierSelectionStrategy` enum variants: `PreferenceOrder`, `RoundRobin`, `LowestCost`, `Random`. Invalid values are rejected at parse time with a serde error, not a `ValidationError`. |

#### 1.2.4 Escalation Validation

| Check | Severity | Rule |
|-------|----------|------|
| `escalation.threshold` in range | ERROR | Must be in `[0.0, 1.0]` |
| `max_escalation_tiers` exceeds tier count | WARNING | Non-fatal but likely misconfiguration |

#### 1.2.5 Cost Budget Validation

| Check | Severity | Rule |
|-------|----------|------|
| `global_daily_limit_usd` non-negative | ERROR | Must be `>= 0.0` |
| `global_monthly_limit_usd` non-negative | ERROR | Must be `>= 0.0` |
| `reset_hour_utc` in range | ERROR | Must be `0..=23` |

#### 1.2.6 Rate Limiting Validation

| Check | Severity | Rule |
|-------|----------|------|
| `window_seconds` positive | ERROR | Must be `> 0` |
| `strategy` known | ERROR | Must be `"sliding_window"` or `"fixed_window"` |

#### 1.2.7 Fallback Model Validation

| Check | Severity | Rule |
|-------|----------|------|
| Model format | WARNING | When `fallback_model` is set, it should contain `/` (provider/model) |

#### 1.2.8 Tool Access Glob Pattern Warning (FIX-12)

| Check | Severity | Rule |
|-------|----------|------|
| Glob-like tool_access pattern | WARNING | When a `tool_access` entry contains `*` but is not exactly `"*"` (e.g., `"file_*"`, `"exec_*"`), emit a warning. Glob patterns in config are intentional but error-prone; operators should be aware they are using pattern matching, not exact tool names. |

#### 1.2.9 CLI Admin Safety Warning (FIX-12)

| Check | Severity | Rule |
|-------|----------|------|
| CLI admin with network exposure | WARNING | When the `cli` channel has admin-level permissions (level=2) and the gateway is configured for network listening (non-localhost bind address), emit a safety warning. CLI admin defaults are designed for local-only use; network-exposed gateways should use explicit per-user permissions rather than blanket admin for CLI. |

### 1.3 Workspace Ceiling Enforcement (FIX-04)

Workspace configs must not grant permissions exceeding the global config ceiling.
A new function `validate_workspace_ceiling(global, workspace)` enforces this
constraint after deep-merge but before the merged config is returned.

#### Security-Sensitive Ceiling Fields

| Field | Ceiling Rule |
|-------|-------------|
| `level` | Workspace `level` cannot exceed `global.max_grantable_level` (default: 1). A workspace cannot promote users to admin unless the global config explicitly allows it. |
| `escalation_allowed` | Workspace cannot enable escalation if global disables it. If `global.escalation_allowed == false`, workspace `escalation_allowed` must also be `false`. |
| `tool_access` | Workspace cannot add tools not present in the global `tool_access` list. Each workspace `tool_access` entry must match at least one global `tool_access` entry. Exception: if global `tool_access` is `["*"]`, any workspace entry is allowed. |
| `rate_limit` | Workspace cannot increase rate limits beyond the global value. If global sets `rate_limit: 60`, workspace cannot set `rate_limit: 120`. A value of 0 (unlimited) in the workspace is only allowed if the global is also 0. |
| `cost_budget_daily_usd` | Workspace cannot increase the daily budget. If global sets `5.00`, workspace cannot set `10.00`. A value of `0.0` (unlimited) in the workspace is only allowed if global is also `0.0`. |
| `cost_budget_monthly_usd` | Same ceiling logic as `cost_budget_daily_usd`. |
| `max_tier` | Workspace cannot set a `max_tier` that is higher in the tier ordering than the global `max_tier`. Tier ordering is determined by position in the `tiers` array (later = higher). |

#### `max_grantable_level` Field

A new field `max_grantable_level: u8` is added to `RoutingConfig` (default: 1).
This caps the maximum permission level that any workspace user override can assign.
With the default of 1, workspaces can assign `zero_trust` (0) or `user` (1) but
not `admin` (2). Only the global config can grant admin.

#### `validate_workspace_ceiling` Function Signature

```rust
/// Validate that a workspace config does not exceed the global config ceiling.
///
/// Returns a Vec of validation errors for any ceiling violations found.
/// An empty Vec means the workspace is within bounds.
pub fn validate_workspace_ceiling(
    global: &RoutingConfig,
    workspace: &RoutingConfig,
) -> Vec<ValidationError>
```

### 1.5 Workspace Deep-Merge for Routing

The workspace `.clawft/config.json` can partially override the global routing config.
Merge rules follow the existing config deep-merge pattern (see `07-workspaces.md`
Section 3.3):

- **Scalar fields** (`mode`, `selection_strategy`, `fallback_model`): Workspace value
  replaces global when present (non-default).
- **Array fields** (`tiers`): Workspace replaces global entirely when non-empty.
  There is no element-level merge for arrays. If the workspace defines tiers, the
  global tiers are discarded.
- **Object fields** (`permissions`, `escalation`, `cost_budgets`, `rate_limiting`):
  Key-level merge. Workspace keys override matching global keys. Global keys not
  present in workspace are preserved.
- **Nested HashMap fields** (`permissions.users`, `permissions.channels`): Key-merge.
  Workspace entries override or add. Global entries not in workspace are preserved.
- **Partial tier overrides**: Workspace cannot override a single tier's cost without
  replacing the entire tiers array. This is a design trade-off for simplicity.
  If partial tier override is needed, the workspace must re-specify all tiers.

### 1.6 Default Tier Construction

When `mode = "tiered"` but `tiers` is empty, the system constructs the four standard
default tiers rather than erroring. This matches the migration path (Section 10 of
the design doc) where a user can write:

```json
{ "routing": { "mode": "tiered" } }
```

And get sensible defaults. The four standard tiers are:

| Tier | Models | Complexity Range | Cost/1K |
|------|--------|-----------------|---------|
| `free` | `openrouter/meta-llama/llama-3.1-8b-instruct:free`, `groq/llama-3.1-8b` | [0.0, 0.3] | $0.000 |
| `standard` | `anthropic/claude-haiku-3.5`, `openai/gpt-4o-mini`, `groq/llama-3.3-70b` | [0.0, 0.7] | $0.001 |
| `premium` | `anthropic/claude-sonnet-4-20250514`, `openai/gpt-4o` | [0.3, 1.0] | $0.010 |
| `elite` | `anthropic/claude-opus-4-5`, `openai/o1` | [0.7, 1.0] | $0.050 |

A `default_tiers()` function returns this list. It is called during the config
loading flow when `mode == "tiered"` and `tiers.is_empty()`.

### 1.7 Backward Compatibility

Existing configs without a `routing` section MUST continue to work unchanged.
`RoutingConfig` derives `Default` and the `Config` struct has `#[serde(default)]`
on the `routing` field. This means:

```json
{ "agents": { "defaults": { "model": "anthropic/claude-opus-4-5" } } }
```

Parses successfully with `routing.mode == "static"`. No validation runs on the
routing section. The `StaticRouter` is used as before.

When `routing` is present but empty (`"routing": {}`), the same behavior applies:
`mode` defaults to `"static"`, all other fields get their `Default` values.

### 1.8 Error vs Warning Distinction

Validation produces two categories of diagnostic:

- **Errors** (`ValidationError`): Fatal. The config is invalid and the system cannot
  start. All errors are collected before returning -- the validator does not stop at
  the first error. The startup sequence logs all errors and aborts.
- **Warnings** (`ValidationWarning`): Non-fatal. The config is technically valid but
  may indicate a misconfiguration. Warnings are logged at WARN level during startup
  but do not prevent the system from running.

---

## 2. Pseudocode

### 2.1 ValidationError and ValidationWarning Types

```rust
/// A fatal validation error in the routing config.
///
/// Errors are structured (not just strings) so callers can programmatically
/// inspect which field failed and why.
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Two tiers share the same name.
    DuplicateTierName {
        index: usize,
        name: String,
    },

    /// A complexity_range value is outside [0.0, 1.0].
    ComplexityRangeOutOfBounds {
        tier_index: usize,
        tier_name: String,
        /// Which element: 0 for min, 1 for max.
        element: usize,
        value: f32,
    },

    /// complexity_range min > max.
    ComplexityRangeInverted {
        tier_index: usize,
        tier_name: String,
        min: f32,
        max: f32,
    },

    /// cost_per_1k_tokens is negative.
    NegativeCost {
        tier_index: usize,
        tier_name: String,
        value: f64,
    },

    /// max_context_tokens is zero.
    ZeroMaxContextTokens {
        tier_index: usize,
        tier_name: String,
    },

    /// Permission level is not 0, 1, or 2.
    InvalidPermissionLevel {
        field_path: String,
        value: u8,
    },

    /// max_tier references a tier name not in the tiers list.
    UndefinedMaxTier {
        field_path: String,
        tier_name: String,
        defined_tiers: Vec<String>,
    },

    /// escalation_threshold is outside [0.0, 1.0].
    EscalationThresholdOutOfRange {
        field_path: String,
        value: f32,
    },

    /// A cost budget field is negative.
    NegativeBudget {
        field_path: String,
        value: f64,
    },

    // FIX-01: UnknownSelectionStrategy removed. TierSelectionStrategy is now
    // a typed enum; serde rejects unknown variants at parse time.

    /// Unknown routing mode (not "static" or "tiered").
    UnknownRoutingMode {
        value: String,
    },

    /// Workspace config exceeds global ceiling for a security-sensitive field (FIX-04).
    WorkspaceCeilingViolation {
        field_path: String,
        global_value: String,
        workspace_value: String,
        reason: String,
    },

    /// rate_limiting.window_seconds is zero.
    ZeroWindowSeconds,

    /// rate_limiting.strategy is not a known value.
    UnknownRateLimitStrategy {
        value: String,
    },

    /// cost_budgets.reset_hour_utc is > 23.
    ResetHourOutOfRange {
        value: u8,
    },
}

/// A non-fatal validation warning.
#[derive(Debug, Clone)]
pub enum ValidationWarning {
    /// Tiered mode is enabled but no tiers are defined.
    /// (After default_tiers() is applied, this means even defaults were empty.)
    NoTiersDefined,

    /// A tier has no models in its models list.
    EmptyTierModels {
        tier_index: usize,
        tier_name: String,
    },

    /// A model identifier does not contain '/' (provider/model format).
    ModelFormatSuspicious {
        tier_index: usize,
        tier_name: String,
        model_index: usize,
        model: String,
    },

    /// Two tiers have overlapping complexity ranges.
    OverlappingComplexityRanges {
        tier_a: String,
        tier_b: String,
        overlap_min: f32,
        overlap_max: f32,
    },

    /// Fallback model does not contain '/' (provider/model format).
    FallbackModelFormatSuspicious {
        model: String,
    },

    /// max_escalation_tiers exceeds the number of defined tiers.
    EscalationTiersExceedDefined {
        max_escalation_tiers: u32,
        defined_tier_count: usize,
    },

    /// A tool_access entry contains glob-like pattern that is not exactly "*" (FIX-12).
    GlobLikeToolAccessPattern {
        field_path: String,
        pattern: String,
    },

    /// CLI channel has admin-level permissions while gateway is network-exposed (FIX-12).
    CliAdminNetworkExposure {
        bind_address: String,
    },
}
```

### 2.2 validate_routing_config Function

```rust
/// Validate a RoutingConfig after deserialization.
///
/// Returns Ok(warnings) when the config is valid (possibly with non-fatal warnings).
/// Returns Err(errors) when the config has fatal validation failures.
/// All errors are collected -- the function does not short-circuit on the first error.
///
/// When mode is "static", validation is skipped entirely (static mode ignores
/// routing config). Validation only runs when mode is "tiered".
pub fn validate_routing_config(
    config: &RoutingConfig,
) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
    let mut errors: Vec<ValidationError> = Vec::new();
    let mut warnings: Vec<ValidationWarning> = Vec::new();

    // Skip validation for static mode
    if config.mode == "static" {
        return Ok(warnings);
    }

    // Validate mode is a known value
    if config.mode != "tiered" && config.mode != "static" {
        errors.push(ValidationError::UnknownRoutingMode {
            value: config.mode.clone(),
        });
    }

    // ---- Tier validation ----

    // Warn if no tiers defined (after default_tiers would have been applied)
    if config.tiers.is_empty() {
        warnings.push(ValidationWarning::NoTiersDefined);
    }

    // Check tier name uniqueness
    let mut seen_names: HashSet<&str> = HashSet::new();
    for (i, tier) in config.tiers.iter().enumerate() {
        if !seen_names.insert(&tier.name) {
            errors.push(ValidationError::DuplicateTierName {
                index: i,
                name: tier.name.clone(),
            });
        }

        // Validate complexity_range bounds
        let [min, max] = tier.complexity_range;
        if min < 0.0 || min > 1.0 {
            errors.push(ValidationError::ComplexityRangeOutOfBounds {
                tier_index: i,
                tier_name: tier.name.clone(),
                element: 0,
                value: min,
            });
        }
        if max < 0.0 || max > 1.0 {
            errors.push(ValidationError::ComplexityRangeOutOfBounds {
                tier_index: i,
                tier_name: tier.name.clone(),
                element: 1,
                value: max,
            });
        }
        if min > max {
            errors.push(ValidationError::ComplexityRangeInverted {
                tier_index: i,
                tier_name: tier.name.clone(),
                min,
                max,
            });
        }

        // Validate cost_per_1k_tokens
        if tier.cost_per_1k_tokens < 0.0 {
            errors.push(ValidationError::NegativeCost {
                tier_index: i,
                tier_name: tier.name.clone(),
                value: tier.cost_per_1k_tokens,
            });
        }

        // Validate max_context_tokens
        if tier.max_context_tokens == 0 {
            errors.push(ValidationError::ZeroMaxContextTokens {
                tier_index: i,
                tier_name: tier.name.clone(),
            });
        }

        // Warn on empty models
        if tier.models.is_empty() {
            warnings.push(ValidationWarning::EmptyTierModels {
                tier_index: i,
                tier_name: tier.name.clone(),
            });
        }

        // Warn on model format
        for (j, model) in tier.models.iter().enumerate() {
            if !model.contains('/') {
                warnings.push(ValidationWarning::ModelFormatSuspicious {
                    tier_index: i,
                    tier_name: tier.name.clone(),
                    model_index: j,
                    model: model.clone(),
                });
            }
        }
    }

    // Check for overlapping complexity ranges (warning only)
    for i in 0..config.tiers.len() {
        for j in (i + 1)..config.tiers.len() {
            let a = &config.tiers[i];
            let b = &config.tiers[j];
            let overlap_min = a.complexity_range[0].max(b.complexity_range[0]);
            let overlap_max = a.complexity_range[1].min(b.complexity_range[1]);
            if overlap_min < overlap_max {
                warnings.push(ValidationWarning::OverlappingComplexityRanges {
                    tier_a: a.name.clone(),
                    tier_b: b.name.clone(),
                    overlap_min,
                    overlap_max,
                });
            }
        }
    }

    // ---- Permission validation ----

    let valid_tier_names: HashSet<&str> =
        config.tiers.iter().map(|t| t.name.as_str()).collect();

    // Helper closure: validate a PermissionLevelConfig at a given field path
    fn validate_permission_level_config(
        plc: &PermissionLevelConfig,
        field_prefix: &str,
        valid_tiers: &HashSet<&str>,
        errors: &mut Vec<ValidationError>,
    ) {
        if let Some(level) = plc.level {
            if level > 2 {
                errors.push(ValidationError::InvalidPermissionLevel {
                    field_path: format!("{}.level", field_prefix),
                    value: level,
                });
            }
        }
        if let Some(ref max_tier) = plc.max_tier {
            if !valid_tiers.is_empty() && !valid_tiers.contains(max_tier.as_str()) {
                errors.push(ValidationError::UndefinedMaxTier {
                    field_path: format!("{}.max_tier", field_prefix),
                    tier_name: max_tier.clone(),
                    defined_tiers: valid_tiers.iter().map(|s| s.to_string()).collect(),
                });
            }
        }
        if let Some(threshold) = plc.escalation_threshold {
            if threshold < 0.0 || threshold > 1.0 {
                errors.push(ValidationError::EscalationThresholdOutOfRange {
                    field_path: format!("{}.escalation_threshold", field_prefix),
                    value: threshold,
                });
            }
        }
        if let Some(daily) = plc.cost_budget_daily_usd {
            if daily < 0.0 {
                errors.push(ValidationError::NegativeBudget {
                    field_path: format!("{}.cost_budget_daily_usd", field_prefix),
                    value: daily,
                });
            }
        }
        if let Some(monthly) = plc.cost_budget_monthly_usd {
            if monthly < 0.0 {
                errors.push(ValidationError::NegativeBudget {
                    field_path: format!("{}.cost_budget_monthly_usd", field_prefix),
                    value: monthly,
                });
            }
        }
    }

    // Validate the three named permission levels
    validate_permission_level_config(
        &config.permissions.zero_trust,
        "routing.permissions.zero_trust",
        &valid_tier_names,
        &mut errors,
    );
    validate_permission_level_config(
        &config.permissions.user,
        "routing.permissions.user",
        &valid_tier_names,
        &mut errors,
    );
    validate_permission_level_config(
        &config.permissions.admin,
        "routing.permissions.admin",
        &valid_tier_names,
        &mut errors,
    );

    // Validate per-user overrides
    for (user_id, plc) in &config.permissions.users {
        validate_permission_level_config(
            plc,
            &format!("routing.permissions.users.{}", user_id),
            &valid_tier_names,
            &mut errors,
        );
    }

    // Validate per-channel overrides
    for (channel, plc) in &config.permissions.channels {
        validate_permission_level_config(
            plc,
            &format!("routing.permissions.channels.{}", channel),
            &valid_tier_names,
            &mut errors,
        );
    }

    // ---- Selection strategy validation ----
    // FIX-01: TierSelectionStrategy is now a typed enum validated by serde.
    // No manual string validation needed here. If deserialization succeeded,
    // the strategy is guaranteed to be a valid variant.

    // ---- Tool access glob pattern warnings (FIX-12) ----
    // Warn on glob-like patterns in tool_access that are not exactly "*".
    // This helps operators notice they are using pattern matching.
    fn warn_glob_patterns(
        plc: &PermissionLevelConfig,
        field_prefix: &str,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        if let Some(ref tool_access) = plc.tool_access {
            for entry in tool_access {
                if entry.contains('*') && entry != "*" {
                    warnings.push(ValidationWarning::GlobLikeToolAccessPattern {
                        field_path: format!("{}.tool_access", field_prefix),
                        pattern: entry.clone(),
                    });
                }
            }
        }
    }

    warn_glob_patterns(
        &config.permissions.zero_trust,
        "routing.permissions.zero_trust",
        &mut warnings,
    );
    warn_glob_patterns(
        &config.permissions.user,
        "routing.permissions.user",
        &mut warnings,
    );
    warn_glob_patterns(
        &config.permissions.admin,
        "routing.permissions.admin",
        &mut warnings,
    );
    for (user_id, plc) in &config.permissions.users {
        warn_glob_patterns(
            plc,
            &format!("routing.permissions.users.{}", user_id),
            &mut warnings,
        );
    }
    for (channel, plc) in &config.permissions.channels {
        warn_glob_patterns(
            plc,
            &format!("routing.permissions.channels.{}", channel),
            &mut warnings,
        );
    }

    // ---- Escalation validation ----

    if config.escalation.threshold < 0.0 || config.escalation.threshold > 1.0 {
        errors.push(ValidationError::EscalationThresholdOutOfRange {
            field_path: "routing.escalation.threshold".to_string(),
            value: config.escalation.threshold,
        });
    }
    if config.escalation.max_escalation_tiers > config.tiers.len() as u32 {
        warnings.push(ValidationWarning::EscalationTiersExceedDefined {
            max_escalation_tiers: config.escalation.max_escalation_tiers,
            defined_tier_count: config.tiers.len(),
        });
    }

    // ---- Cost budget validation ----

    if config.cost_budgets.global_daily_limit_usd < 0.0 {
        errors.push(ValidationError::NegativeBudget {
            field_path: "routing.cost_budgets.global_daily_limit_usd".to_string(),
            value: config.cost_budgets.global_daily_limit_usd,
        });
    }
    if config.cost_budgets.global_monthly_limit_usd < 0.0 {
        errors.push(ValidationError::NegativeBudget {
            field_path: "routing.cost_budgets.global_monthly_limit_usd".to_string(),
            value: config.cost_budgets.global_monthly_limit_usd,
        });
    }
    if config.cost_budgets.reset_hour_utc > 23 {
        errors.push(ValidationError::ResetHourOutOfRange {
            value: config.cost_budgets.reset_hour_utc,
        });
    }

    // ---- Rate limiting validation ----

    if config.rate_limiting.window_seconds == 0 {
        errors.push(ValidationError::ZeroWindowSeconds);
    }
    let valid_strategies = ["sliding_window", "fixed_window"];
    if !valid_strategies.contains(&config.rate_limiting.strategy.as_str()) {
        errors.push(ValidationError::UnknownRateLimitStrategy {
            value: config.rate_limiting.strategy.clone(),
        });
    }

    // ---- Fallback model format ----

    if let Some(ref model) = config.fallback_model {
        if !model.contains('/') {
            warnings.push(ValidationWarning::FallbackModelFormatSuspicious {
                model: model.clone(),
            });
        }
    }

    // ---- CLI admin safety warning (FIX-12) ----
    // Warn when CLI channel has admin permissions and gateway binds to non-localhost.
    if let Some(ref cli_plc) = config.permissions.channels.get("cli") {
        if cli_plc.level == Some(2) {
            // This check requires gateway bind address context, which is passed
            // as an optional parameter or checked separately. Here we note the
            // pattern; the actual bind address check is done in the config_loader
            // integration where gateway config is available.
            // See: validate_cli_admin_network_safety() below.
        }
    }

    // Return result
    if errors.is_empty() {
        Ok(warnings)
    } else {
        Err(errors)
    }
}

/// Validate that CLI admin permissions are safe given the gateway bind address (FIX-12).
///
/// Called by the config_loader after routing validation, when gateway config is available.
/// Emits a warning if CLI channel has admin-level permissions while the gateway is
/// bound to a non-localhost address, since this could allow remote users to get admin
/// privileges through the CLI channel.
pub fn validate_cli_admin_network_safety(
    config: &RoutingConfig,
    gateway_bind_address: Option<&str>,
) -> Vec<ValidationWarning> {
    let mut warnings = Vec::new();

    if let Some(bind_addr) = gateway_bind_address {
        let is_localhost = bind_addr.starts_with("127.")
            || bind_addr.starts_with("localhost")
            || bind_addr.starts_with("[::1]");

        if !is_localhost {
            if let Some(ref cli_plc) = config.permissions.channels.get("cli") {
                if cli_plc.level == Some(2) {
                    warnings.push(ValidationWarning::CliAdminNetworkExposure {
                        bind_address: bind_addr.to_string(),
                    });
                }
            }
        }
    }

    warnings
}

/// Validate that a workspace config does not exceed the global config ceiling (FIX-04).
///
/// Returns a Vec of validation errors for any ceiling violations.
/// An empty Vec means the workspace is within bounds.
///
/// Security-sensitive ceiling fields:
/// - `level`: cannot exceed `global.max_grantable_level` (default: 1)
/// - `escalation_allowed`: cannot enable if global disables
/// - `tool_access`: cannot add tools not in global
/// - `rate_limit`: workspace cannot increase beyond global
/// - `cost_budget_daily_usd`: workspace cannot increase
/// - `cost_budget_monthly_usd`: workspace cannot increase
/// - `max_tier`: cannot upgrade beyond global tier ordering
pub fn validate_workspace_ceiling(
    global: &RoutingConfig,
    workspace: &RoutingConfig,
) -> Vec<ValidationError> {
    let mut errors: Vec<ValidationError> = Vec::new();

    let max_grantable = global.max_grantable_level.unwrap_or(1);

    // Check all permission level configs in workspace
    fn check_level_ceiling(
        plc: &PermissionLevelConfig,
        field_prefix: &str,
        global_plc: &PermissionLevelConfig,
        max_grantable: u8,
        global_tool_access: &Option<Vec<String>>,
        errors: &mut Vec<ValidationError>,
    ) {
        // Level ceiling
        if let Some(ws_level) = plc.level {
            if ws_level > max_grantable {
                errors.push(ValidationError::WorkspaceCeilingViolation {
                    field_path: format!("{}.level", field_prefix),
                    global_value: max_grantable.to_string(),
                    workspace_value: ws_level.to_string(),
                    reason: "workspace cannot grant level above max_grantable_level".into(),
                });
            }
        }

        // Escalation ceiling
        if let (Some(ws_esc), Some(global_esc)) = (plc.escalation_allowed, global_plc.escalation_allowed) {
            if ws_esc && !global_esc {
                errors.push(ValidationError::WorkspaceCeilingViolation {
                    field_path: format!("{}.escalation_allowed", field_prefix),
                    global_value: "false".into(),
                    workspace_value: "true".into(),
                    reason: "workspace cannot enable escalation when global disables it".into(),
                });
            }
        }

        // Tool access ceiling
        if let Some(ref ws_tools) = plc.tool_access {
            if let Some(ref global_tools) = global_tool_access {
                // If global is ["*"], anything goes
                if !global_tools.iter().any(|s| s == "*") {
                    for ws_tool in ws_tools {
                        if ws_tool == "*" {
                            errors.push(ValidationError::WorkspaceCeilingViolation {
                                field_path: format!("{}.tool_access", field_prefix),
                                global_value: format!("{:?}", global_tools),
                                workspace_value: "*".into(),
                                reason: "workspace cannot grant wildcard when global does not".into(),
                            });
                        } else if !global_tools.contains(ws_tool) {
                            errors.push(ValidationError::WorkspaceCeilingViolation {
                                field_path: format!("{}.tool_access", field_prefix),
                                global_value: format!("{:?}", global_tools),
                                workspace_value: ws_tool.clone(),
                                reason: "workspace cannot add tool not in global tool_access".into(),
                            });
                        }
                    }
                }
            }
        }

        // Rate limit ceiling (workspace cannot increase)
        if let (Some(ws_rate), Some(global_rate)) = (plc.rate_limit, global_plc.rate_limit) {
            if global_rate > 0 && (ws_rate > global_rate || ws_rate == 0) {
                errors.push(ValidationError::WorkspaceCeilingViolation {
                    field_path: format!("{}.rate_limit", field_prefix),
                    global_value: global_rate.to_string(),
                    workspace_value: ws_rate.to_string(),
                    reason: "workspace cannot increase rate limit beyond global".into(),
                });
            }
        }

        // Cost budget daily ceiling
        if let (Some(ws_daily), Some(global_daily)) = (plc.cost_budget_daily_usd, global_plc.cost_budget_daily_usd) {
            if global_daily > 0.0 && (ws_daily > global_daily || ws_daily == 0.0) {
                errors.push(ValidationError::WorkspaceCeilingViolation {
                    field_path: format!("{}.cost_budget_daily_usd", field_prefix),
                    global_value: global_daily.to_string(),
                    workspace_value: ws_daily.to_string(),
                    reason: "workspace cannot increase daily budget beyond global".into(),
                });
            }
        }

        // Cost budget monthly ceiling
        if let (Some(ws_monthly), Some(global_monthly)) = (plc.cost_budget_monthly_usd, global_plc.cost_budget_monthly_usd) {
            if global_monthly > 0.0 && (ws_monthly > global_monthly || ws_monthly == 0.0) {
                errors.push(ValidationError::WorkspaceCeilingViolation {
                    field_path: format!("{}.cost_budget_monthly_usd", field_prefix),
                    global_value: global_monthly.to_string(),
                    workspace_value: ws_monthly.to_string(),
                    reason: "workspace cannot increase monthly budget beyond global".into(),
                });
            }
        }
    }

    // Check named levels
    check_level_ceiling(
        &workspace.permissions.zero_trust,
        "routing.permissions.zero_trust",
        &global.permissions.zero_trust,
        max_grantable,
        &global.permissions.zero_trust.tool_access,
        &mut errors,
    );
    check_level_ceiling(
        &workspace.permissions.user,
        "routing.permissions.user",
        &global.permissions.user,
        max_grantable,
        &global.permissions.user.tool_access,
        &mut errors,
    );
    check_level_ceiling(
        &workspace.permissions.admin,
        "routing.permissions.admin",
        &global.permissions.admin,
        max_grantable,
        &global.permissions.admin.tool_access,
        &mut errors,
    );

    // Check per-user overrides
    for (user_id, ws_plc) in &workspace.permissions.users {
        let global_plc = global.permissions.users.get(user_id)
            .unwrap_or(&global.permissions.user); // fallback to user-level global
        check_level_ceiling(
            ws_plc,
            &format!("routing.permissions.users.{}", user_id),
            global_plc,
            max_grantable,
            &global_plc.tool_access,
            &mut errors,
        );
    }

    // Check per-channel overrides
    for (channel, ws_plc) in &workspace.permissions.channels {
        let global_plc = global.permissions.channels.get(channel)
            .unwrap_or(&global.permissions.user);
        check_level_ceiling(
            ws_plc,
            &format!("routing.permissions.channels.{}", channel),
            global_plc,
            max_grantable,
            &global_plc.tool_access,
            &mut errors,
        );
    }

    errors
}
```

### 2.3 deep_merge_routing Function

```rust
/// Deep-merge a workspace routing config on top of a global routing config.
///
/// Merge rules:
/// - Scalar fields: workspace wins if explicitly set (non-default).
/// - Array fields (tiers): workspace replaces entirely if non-empty.
/// - Optional scalar fields: workspace wins if Some.
/// - Nested objects (permissions, escalation, cost_budgets, rate_limiting):
///   field-by-field merge at the leaf level.
/// - HashMap fields (permissions.users, permissions.channels): key-merge.
///   Workspace entries override or add to global entries. Global entries not
///   in workspace are preserved.
pub fn deep_merge_routing(
    global: &RoutingConfig,
    workspace: Option<&RoutingConfig>,
) -> RoutingConfig {
    let ws = match workspace {
        Some(ws) => ws,
        None => return global.clone(),
    };

    RoutingConfig {
        // FIX-04: max_grantable_level -- global always wins (workspace cannot override)
        max_grantable_level: global.max_grantable_level,

        // Scalar: workspace wins if not default
        mode: if ws.mode != "static" {
            ws.mode.clone()
        } else {
            global.mode.clone()
        },

        // Array: workspace replaces entirely if non-empty
        tiers: if ws.tiers.is_empty() {
            global.tiers.clone()
        } else {
            ws.tiers.clone()
        },

        // Optional scalar: workspace wins if Some
        selection_strategy: ws
            .selection_strategy
            .clone()
            .or_else(|| global.selection_strategy.clone()),
        fallback_model: ws
            .fallback_model
            .clone()
            .or_else(|| global.fallback_model.clone()),

        // Nested object: deep merge
        permissions: deep_merge_permissions(&global.permissions, &ws.permissions),

        // Nested objects: field-by-field merge
        escalation: merge_escalation(&global.escalation, &ws.escalation),
        cost_budgets: merge_cost_budgets(&global.cost_budgets, &ws.cost_budgets),
        rate_limiting: merge_rate_limiting(&global.rate_limiting, &ws.rate_limiting),
    }
}

fn deep_merge_permissions(
    global: &PermissionsConfig,
    ws: &PermissionsConfig,
) -> PermissionsConfig {
    PermissionsConfig {
        // Named levels: merge field-by-field (ws overrides populated fields)
        zero_trust: merge_permission_level(&global.zero_trust, &ws.zero_trust),
        user: merge_permission_level(&global.user, &ws.user),
        admin: merge_permission_level(&global.admin, &ws.admin),

        // Per-user: key-merge
        users: {
            let mut merged = global.users.clone();
            for (key, val) in &ws.users {
                merged.insert(key.clone(), val.clone());
            }
            merged
        },

        // Per-channel: key-merge
        channels: {
            let mut merged = global.channels.clone();
            for (key, val) in &ws.channels {
                merged.insert(key.clone(), val.clone());
            }
            merged
        },
    }
}

/// Merge two PermissionLevelConfig instances. The workspace (ws) values
/// take precedence over the global values for any field that is Some.
fn merge_permission_level(
    global: &PermissionLevelConfig,
    ws: &PermissionLevelConfig,
) -> PermissionLevelConfig {
    PermissionLevelConfig {
        level: ws.level.or(global.level),
        max_tier: ws.max_tier.clone().or_else(|| global.max_tier.clone()),
        model_access: ws.model_access.clone().or_else(|| global.model_access.clone()),
        model_denylist: ws.model_denylist.clone().or_else(|| global.model_denylist.clone()),
        tool_access: ws.tool_access.clone().or_else(|| global.tool_access.clone()),
        tool_denylist: ws.tool_denylist.clone().or_else(|| global.tool_denylist.clone()),
        max_context_tokens: ws.max_context_tokens.or(global.max_context_tokens),
        max_output_tokens: ws.max_output_tokens.or(global.max_output_tokens),
        rate_limit: ws.rate_limit.or(global.rate_limit),
        streaming_allowed: ws.streaming_allowed.or(global.streaming_allowed),
        escalation_allowed: ws.escalation_allowed.or(global.escalation_allowed),
        escalation_threshold: ws.escalation_threshold.or(global.escalation_threshold),
        model_override: ws.model_override.or(global.model_override),
        cost_budget_daily_usd: ws.cost_budget_daily_usd.or(global.cost_budget_daily_usd),
        cost_budget_monthly_usd: ws.cost_budget_monthly_usd.or(global.cost_budget_monthly_usd),
        custom_permissions: match (&global.custom_permissions, &ws.custom_permissions) {
            (Some(g), Some(w)) => {
                let mut merged = g.clone();
                merged.extend(w.clone());
                Some(merged)
            }
            (None, Some(w)) => Some(w.clone()),
            (Some(g), None) => Some(g.clone()),
            (None, None) => None,
        },
    }
}

fn merge_escalation(
    global: &EscalationConfig,
    ws: &EscalationConfig,
) -> EscalationConfig {
    // For escalation, workspace wins for each field if it differs from default
    EscalationConfig {
        enabled: ws.enabled || global.enabled,
        threshold: if (ws.threshold - 0.6).abs() > f32::EPSILON {
            ws.threshold
        } else {
            global.threshold
        },
        max_escalation_tiers: if ws.max_escalation_tiers != 1 {
            ws.max_escalation_tiers
        } else {
            global.max_escalation_tiers
        },
    }
}

fn merge_cost_budgets(
    global: &CostBudgetConfig,
    ws: &CostBudgetConfig,
) -> CostBudgetConfig {
    CostBudgetConfig {
        global_daily_limit_usd: if ws.global_daily_limit_usd != 0.0 {
            ws.global_daily_limit_usd
        } else {
            global.global_daily_limit_usd
        },
        global_monthly_limit_usd: if ws.global_monthly_limit_usd != 0.0 {
            ws.global_monthly_limit_usd
        } else {
            global.global_monthly_limit_usd
        },
        tracking_persistence: ws.tracking_persistence || global.tracking_persistence,
        reset_hour_utc: if ws.reset_hour_utc != 0 {
            ws.reset_hour_utc
        } else {
            global.reset_hour_utc
        },
    }
}

fn merge_rate_limiting(
    global: &RateLimitConfig,
    ws: &RateLimitConfig,
) -> RateLimitConfig {
    RateLimitConfig {
        window_seconds: if ws.window_seconds != 60 {
            ws.window_seconds
        } else {
            global.window_seconds
        },
        strategy: if ws.strategy != "sliding_window" {
            ws.strategy.clone()
        } else {
            global.strategy.clone()
        },
    }
}
```

### 2.4 default_tiers Function

```rust
/// Construct the four standard default tiers.
///
/// Called when mode = "tiered" but no tiers are specified in config.
/// This enables the minimal config migration path:
///   { "routing": { "mode": "tiered" } }
///
/// Returns tiers matching the table in Section 4.1 of the design doc.
pub fn default_tiers() -> Vec<ModelTierConfig> {
    vec![
        ModelTierConfig {
            name: "free".into(),
            models: vec![
                "openrouter/meta-llama/llama-3.1-8b-instruct:free".into(),
                "groq/llama-3.1-8b".into(),
            ],
            complexity_range: [0.0, 0.3],
            cost_per_1k_tokens: 0.0,
            max_context_tokens: 8192,
        },
        ModelTierConfig {
            name: "standard".into(),
            models: vec![
                "anthropic/claude-haiku-3.5".into(),
                "openai/gpt-4o-mini".into(),
                "groq/llama-3.3-70b".into(),
            ],
            complexity_range: [0.0, 0.7],
            cost_per_1k_tokens: 0.001,
            max_context_tokens: 16384,
        },
        ModelTierConfig {
            name: "premium".into(),
            models: vec![
                "anthropic/claude-sonnet-4-20250514".into(),
                "openai/gpt-4o".into(),
            ],
            complexity_range: [0.3, 1.0],
            cost_per_1k_tokens: 0.01,
            max_context_tokens: 200_000,
        },
        ModelTierConfig {
            name: "elite".into(),
            models: vec![
                "anthropic/claude-opus-4-5".into(),
                "openai/o1".into(),
            ],
            complexity_range: [0.7, 1.0],
            cost_per_1k_tokens: 0.05,
            max_context_tokens: 200_000,
        },
    ]
}
```

### 2.5 Config Loading Flow

```rust
fn load_config(
    global_path: &Path,
    workspace_path: Option<&Path>,
) -> Result<Config, ConfigError> {
    // 1. Deserialize global config
    let global_json = read_file(global_path)?;
    let mut global: Config = serde_json::from_str(&global_json)
        .map_err(|e| ConfigError::ParseError {
            path: global_path.to_path_buf(),
            source: e,
        })?;

    // 2. Deserialize workspace config (if exists)
    let workspace: Option<Config> = workspace_path
        .filter(|p| p.exists())
        .map(|p| {
            let json = read_file(p)?;
            serde_json::from_str(&json).map_err(|e| ConfigError::ParseError {
                path: p.to_path_buf(),
                source: e,
            })
        })
        .transpose()?;

    // 3. Deep merge all sections (existing logic for agents/channels/providers/tools)
    let mut merged = deep_merge_config(&global, &workspace);

    // 4. Deep merge routing section specifically
    merged.routing = deep_merge_routing(
        &global.routing,
        workspace.as_ref().map(|w| &w.routing),
    );

    // 5. Apply default tiers if mode=tiered but tiers empty
    if merged.routing.mode == "tiered" && merged.routing.tiers.is_empty() {
        merged.routing.tiers = default_tiers();
    }

    // 6. Validate workspace ceiling (FIX-04)
    if let Some(ref ws_config) = workspace {
        let ceiling_errors = validate_workspace_ceiling(
            &global.routing,
            &ws_config.routing,
        );
        if !ceiling_errors.is_empty() {
            for e in &ceiling_errors {
                log::error!("workspace ceiling violation: {:?}", e);
            }
            return Err(ConfigError::RoutingValidationErrors(ceiling_errors));
        }
    }

    // 7. Validate routing config
    match validate_routing_config(&merged.routing) {
        Ok(warnings) => {
            for w in &warnings {
                log::warn!("config validation: {:?}", w);
            }
        }
        Err(errors) => {
            // Log all errors, then return failure
            for e in &errors {
                log::error!("config validation error: {:?}", e);
            }
            return Err(ConfigError::RoutingValidationErrors(errors));
        }
    }

    Ok(merged)
}
```

### 2.6 Display impl for ValidationError

```rust
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateTierName { index, name } => {
                write!(f, "routing.tiers[{}].name: duplicate tier name '{}'", index, name)
            }
            Self::ComplexityRangeOutOfBounds { tier_index, tier_name, element, value } => {
                write!(
                    f,
                    "routing.tiers[{}] ('{}'): complexity_range[{}] = {} is outside [0.0, 1.0]",
                    tier_index, tier_name, element, value
                )
            }
            Self::ComplexityRangeInverted { tier_index, tier_name, min, max } => {
                write!(
                    f,
                    "routing.tiers[{}] ('{}'): complexity_range min {} > max {}",
                    tier_index, tier_name, min, max
                )
            }
            Self::NegativeCost { tier_index, tier_name, value } => {
                write!(
                    f,
                    "routing.tiers[{}] ('{}'): cost_per_1k_tokens = {} must be non-negative",
                    tier_index, tier_name, value
                )
            }
            Self::ZeroMaxContextTokens { tier_index, tier_name } => {
                write!(
                    f,
                    "routing.tiers[{}] ('{}'): max_context_tokens must be > 0",
                    tier_index, tier_name
                )
            }
            Self::InvalidPermissionLevel { field_path, value } => {
                write!(f, "{}: permission level {} must be 0, 1, or 2", field_path, value)
            }
            Self::UndefinedMaxTier { field_path, tier_name, defined_tiers } => {
                write!(
                    f,
                    "{}: max_tier '{}' does not match any defined tier (available: {})",
                    field_path, tier_name, defined_tiers.join(", ")
                )
            }
            Self::EscalationThresholdOutOfRange { field_path, value } => {
                write!(f, "{}: escalation_threshold {} must be in [0.0, 1.0]", field_path, value)
            }
            Self::NegativeBudget { field_path, value } => {
                write!(f, "{}: budget {} must be non-negative", field_path, value)
            }
            // FIX-01: UnknownSelectionStrategy removed (serde handles typed enum validation)
            Self::WorkspaceCeilingViolation { field_path, global_value, workspace_value, reason } => {
                write!(
                    f,
                    "{}: workspace value '{}' exceeds global ceiling '{}' ({})",
                    field_path, workspace_value, global_value, reason
                )
            }
            Self::UnknownRoutingMode { value } => {
                write!(f, "routing.mode: unknown mode '{}', expected 'static' or 'tiered'", value)
            }
            Self::ZeroWindowSeconds => {
                write!(f, "routing.rate_limiting.window_seconds: must be > 0")
            }
            Self::UnknownRateLimitStrategy { value } => {
                write!(
                    f,
                    "routing.rate_limiting.strategy: unknown '{}', expected 'sliding_window' or 'fixed_window'",
                    value
                )
            }
            Self::ResetHourOutOfRange { value } => {
                write!(f, "routing.cost_budgets.reset_hour_utc: {} must be 0-23", value)
            }
        }
    }
}

impl std::error::Error for ValidationError {}
```

---

## 3. Architecture

### 3.1 File Locations

| Component | File | Action |
|-----------|------|--------|
| `ValidationError` enum | `crates/clawft-types/src/routing_validation.rs` | **NEW** |
| `ValidationWarning` enum | `crates/clawft-types/src/routing_validation.rs` | **NEW** |
| `validate_routing_config()` | `crates/clawft-types/src/routing_validation.rs` | **NEW** |
| `default_tiers()` | `crates/clawft-types/src/routing_validation.rs` | **NEW** |
| `deep_merge_routing()` | `crates/clawft-types/src/routing_validation.rs` | **NEW** |
| Module declaration | `crates/clawft-types/src/lib.rs` | **EDIT** (add `pub mod routing_validation;`) |
| Config loading integration | `crates/clawft-platform/src/config_loader.rs` | **EDIT** (call validate + merge + defaults) |
| Full valid config fixture | `tests/fixtures/config_tiered.json` | **NEW** |
| Minimal valid config fixture | `tests/fixtures/config_tiered_minimal.json` | **NEW** |
| Invalid config fixture | `tests/fixtures/config_tiered_invalid.json` | **NEW** |

### 3.2 Validation Runs After Deserialization, Before TieredRouter Construction

```
config.json
    |
    v
serde_json::from_str() --> Config { routing: RoutingConfig }
    |
    v
deep_merge_routing(global, workspace)  --> merged RoutingConfig
    |
    v
apply default_tiers() if mode=tiered and tiers empty
    |
    v
validate_routing_config(&merged.routing)
    |
    +-- Ok(warnings) --> log warnings, continue
    |
    +-- Err(errors)  --> log errors, abort startup
    |
    v
TieredRouter::from_config(&config.routing, ...)
    // Assumes config is already valid. No duplicate validation.
```

### 3.3 ValidationError Is a Structured Enum, Not Strings

The `ValidationError` enum has typed variants for each failure class. This enables:

- Programmatic error matching in tests (`matches!(err, ValidationError::DuplicateTierName { .. })`)
- Clear `Display` formatting for operator-facing messages
- Future extension (e.g., config editor integration, linting tool)

`ValidationWarning` is also a typed enum for the same reasons.

### 3.4 Config Fixtures

**`tests/fixtures/config_tiered.json`** -- A complete config exercising every
routing field. This is the "golden" fixture used by roundtrip and deserialization
tests. Includes all four tiers, all three permission levels fully populated,
per-user and per-channel overrides, escalation, cost budgets, and rate limiting.

**`tests/fixtures/config_tiered_minimal.json`** -- A minimal but valid tiered
config. Contains only `routing.mode = "tiered"` and two tiers with the bare
minimum fields (name, models, complexity_range, cost_per_1k_tokens). No
permissions section, no escalation, no budgets. Tests that defaults fill in
correctly.

**`tests/fixtures/config_tiered_invalid.json`** -- A config intentionally
containing multiple validation errors. Used by tests to verify that all
errors are collected (not just the first). Contains: duplicate tier name,
inverted complexity range, negative cost, invalid permission level, undefined
max_tier reference, out-of-range escalation threshold.

### 3.5 Module Structure After Phase H

```
crates/clawft-types/src/
    lib.rs                  # Add: pub mod routing_validation;
    config.rs               # Already has routing: RoutingConfig (Phase A)
    routing.rs              # Config types (Phase A)
    routing_validation.rs   # NEW: validation + merge + defaults (~400 lines)
    delegation.rs           # Unchanged
    ...

crates/clawft-platform/src/
    config_loader.rs        # EDIT: call validate_routing_config + deep_merge_routing

tests/fixtures/
    config.json             # Existing -- unchanged
    config_tiered.json      # NEW -- full tiered config
    config_tiered_minimal.json  # NEW -- minimal tiered config
    config_tiered_invalid.json  # NEW -- intentionally invalid config
```

---

## 4. Refinement

### 4.1 Error Severity Decision Table

| Condition | Severity | Rationale |
|-----------|----------|-----------|
| Duplicate tier names | ERROR | Router cannot disambiguate tiers with same name |
| complexity_range min > max | ERROR | No complexity score can match an inverted range |
| complexity_range values outside [0.0, 1.0] | ERROR | Complexity scores are always in [0.0, 1.0] by contract |
| Negative cost_per_1k_tokens | ERROR | Negative cost is nonsensical for budget calculation |
| max_context_tokens == 0 | ERROR | Zero-token context window is unusable |
| Permission level > 2 | ERROR | Only 0, 1, 2 are defined; unknown levels would fallback to zero_trust silently, creating confusion |
| max_tier references undefined tier | ERROR | Router cannot find the tier to enforce the ceiling |
| escalation_threshold outside [0.0, 1.0] | ERROR | Complexity scores are [0.0, 1.0]; threshold outside this range is meaningless |
| Negative budget | ERROR | Negative budget is nonsensical |
| Unknown selection_strategy | HANDLED BY SERDE (FIX-01) | Now a typed enum; serde rejects unknown variants at parse time |
| window_seconds == 0 | ERROR | Division by zero in rate limiter |
| reset_hour_utc > 23 | ERROR | Not a valid hour |
| Overlapping tier complexity ranges | WARNING | Intentional by design (Section 4.1), but operator should be aware |
| Empty tier models list | WARNING | Tier is defined but unusable; not fatal because fallback can handle it |
| Model without '/' separator | WARNING | Likely misconfiguration but may be a local/custom model format |
| Fallback model without '/' separator | WARNING | Same as model format |
| max_escalation_tiers > tier count | WARNING | Will be clamped at runtime; likely misconfiguration |
| Tiered mode with no tiers (after defaults applied) | WARNING | Should not happen after default_tiers(), but defensive |
| Glob-like tool_access pattern (FIX-12) | WARNING | Pattern contains `*` but is not `"*"` -- operator may not intend glob matching |
| CLI admin + network-exposed gateway (FIX-12) | WARNING | CLI admin defaults are for local use; network exposure is a security concern |
| Workspace ceiling violation (FIX-04) | ERROR | Workspace config exceeds global security ceiling |

### 4.2 Builder Pattern Consideration

A builder pattern for `RoutingConfig` with validation on `build()` was considered
but rejected for Phase H. Reasons:

1. The config is loaded from JSON by serde, not constructed programmatically.
   A builder adds API surface that no current caller uses.
2. Validation after deserialization is the standard pattern in clawft (see
   `CommandPolicyConfig`, `UrlPolicyConfig` -- validated by their consumers,
   not by a builder).
3. A builder can be added in a future phase if programmatic config construction
   becomes a use case (e.g., config editor UI).

The validation function approach (`validate_routing_config()`) is preferred because
it composes cleanly with the existing config loading pipeline and collects all
errors in a single pass.

### 4.3 Workspace Merge: Partial Tier Override Limitation

Workspace merge replaces the entire `tiers` array if the workspace config provides
any tiers. This means a workspace cannot override just one tier's `cost_per_1k_tokens`
without re-specifying all tiers.

This is a deliberate simplification:
- Element-level array merge requires a key (tier name) to match elements, adding
  complexity to the merge logic.
- Tier definitions are relatively small and self-contained.
- If a workspace needs different tiers, it should define all of them.

If partial tier override becomes a strong use case, a future enhancement can add
name-based tier merging where workspace tiers with matching names override the
global tier and non-matching global tiers are preserved. This is out of scope
for Phase H.

### 4.4 Workspace Merge: Scalar Default Detection Problem

The merge logic uses "is this the default value?" to detect whether a workspace
field was explicitly set. This has a known limitation: if the workspace explicitly
sets a field to its default value (e.g., `"mode": "static"`), the merge treats it
as "not set" and uses the global value.

For most fields this is acceptable because setting a field to its default is a
no-op. The one edge case is `mode`: a workspace wanting to force `"static"` mode
while the global uses `"tiered"` would need a different sentinel. For Phase H,
this is documented as a known limitation. The workaround is to remove the `routing`
section from the global config entirely.

### 4.5 NaN and Infinity Handling

`serde_json` rejects `NaN` and `Infinity` values at parse time (they are not valid
JSON). No special handling is needed in the validation layer. If a future config
format (TOML, YAML) is supported, the validation function should add checks for
`f32::is_nan()` and `f32::is_infinite()`.

### 4.6 Validation Performance

Validation runs once at startup. For a config with N tiers, M users, and K channels:
- Tier uniqueness check: O(N) with HashSet
- Tier field validation: O(N)
- Overlap detection: O(N^2) -- acceptable because N is small (typically 2-6 tiers)
- Permission validation: O(M + K) with constant-time field checks
- Total: O(N^2 + M + K), which is sub-millisecond for any realistic config

---

## 5. Completion

### 5.1 Success Criteria

- [ ] All invalid configs produce clear, human-readable error messages with field paths
- [ ] Validation collects ALL errors, not just the first one
- [ ] Valid configs deserialize and serialize roundtrip correctly (no data loss)
- [ ] Workspace routing override merges correctly (leaf-level field override)
- [ ] Workspace tiers array replaces global tiers array entirely
- [ ] Workspace channel permissions merge with global channel permissions (key-merge)
- [ ] Workspace user overrides merge with global user overrides (key-merge)
- [ ] `mode: "tiered"` with empty tiers triggers default_tiers() construction
- [ ] `mode: "static"` skips all routing validation (backward compat)
- [ ] Empty `routing` section defaults to `mode: "static"` (backward compat)
- [ ] Absent `routing` section defaults to `mode: "static"` (backward compat)
- [ ] Existing `tests/fixtures/config.json` still loads and passes all existing tests
- [ ] Warnings are logged but do not prevent startup
- [ ] Errors abort startup with all failures listed

### 5.2 Test Plan

#### Validation Tests (16 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 1 | `validate_valid_full_config` | Load `config_tiered.json` fixture; validate returns Ok with zero errors |
| 2 | `validate_valid_minimal_config` | Load `config_tiered_minimal.json` fixture; validate returns Ok |
| 3 | `validate_static_mode_skips_validation` | Config with `mode: "static"` and invalid tier data still passes (skipped) |
| 4 | `validate_duplicate_tier_names` | Two tiers named `"free"` produces `DuplicateTierName` error |
| 5 | `validate_complexity_range_min_gt_max` | `[0.8, 0.3]` produces `ComplexityRangeInverted` error |
| 6 | `validate_complexity_range_out_of_bounds_low` | `[-0.1, 0.5]` produces `ComplexityRangeOutOfBounds` error |
| 7 | `validate_complexity_range_out_of_bounds_high` | `[0.0, 1.5]` produces `ComplexityRangeOutOfBounds` error |
| 8 | `validate_negative_cost_per_1k` | `cost_per_1k_tokens: -0.001` produces `NegativeCost` error |
| 9 | `validate_zero_max_context_tokens` | `max_context_tokens: 0` produces `ZeroMaxContextTokens` error |
| 10 | `validate_invalid_permission_level` | `level: 5` produces `InvalidPermissionLevel` error |
| 11 | `validate_undefined_max_tier` | `max_tier: "mythical"` produces `UndefinedMaxTier` error |
| 12 | `validate_escalation_threshold_out_of_range` | `escalation_threshold: 1.5` produces `EscalationThresholdOutOfRange` error |
| 13 | `validate_negative_daily_budget` | `cost_budget_daily_usd: -1.0` produces `NegativeBudget` error |
| 14 | REMOVED (FIX-01) | `TierSelectionStrategy` is now a typed enum; serde rejects unknown variants at parse time. No `ValidationError` test needed. |
| 15 | `validate_zero_window_seconds` | `window_seconds: 0` produces `ZeroWindowSeconds` error |
| 16 | `validate_reset_hour_out_of_range` | `reset_hour_utc: 25` produces `ResetHourOutOfRange` error |

#### Warning Tests (5 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 17 | `validate_empty_tiers_warning` | Tiered mode with empty tiers produces `NoTiersDefined` warning |
| 18 | `validate_empty_models_warning` | Tier with `models: []` produces `EmptyTierModels` warning |
| 19 | `validate_model_format_warning` | Model `"gpt-4o"` (no slash) produces `ModelFormatSuspicious` warning |
| 20 | `validate_overlapping_ranges_warning` | Two tiers overlapping produces `OverlappingComplexityRanges` warning |
| 21 | `validate_fallback_model_format_warning` | `fallback_model: "gpt-4o"` produces `FallbackModelFormatSuspicious` warning |

#### Error Collection Tests (2 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 22 | `validate_collects_all_errors` | Config with 3+ distinct errors returns all of them, not just the first |
| 23 | `validate_errors_and_warnings_separate` | Config with both errors and warnings returns errors (warnings are in the Err path metadata or tested separately with a valid config) |

#### Deep Merge Tests (6 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 24 | `merge_no_workspace` | No workspace config returns global unchanged |
| 25 | `merge_workspace_overrides_mode` | Workspace `mode: "tiered"` overrides global `mode: "static"` |
| 26 | `merge_workspace_replaces_tiers_array` | Workspace with 2 tiers replaces global 4 tiers entirely |
| 27 | `merge_workspace_adds_channel_permissions` | Workspace adds `telegram: { level: 0 }` alongside global `cli: { level: 2 }` |
| 28 | `merge_workspace_overrides_daily_budget` | Workspace `global_daily_limit_usd: 10.0` overrides global `50.0` |
| 29 | `merge_workspace_adds_user_override` | Workspace adds user override that global does not have |

#### Default Tiers Tests (2 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 30 | `default_tiers_returns_four_tiers` | `default_tiers()` returns exactly 4 tiers (free, standard, premium, elite) |
| 31 | `default_tiers_values_match_spec` | Each default tier's name, models, complexity_range, cost match the spec table |

#### Config Loading Flow Tests (4 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 32 | `load_tiered_config_applies_defaults` | `{ "routing": { "mode": "tiered" } }` results in 4 default tiers after loading |
| 33 | `load_static_config_no_validation` | `{ "routing": { "mode": "static" } }` with invalid tier data loads successfully |
| 34 | `load_empty_json_backward_compat` | `{}` loads with `routing.mode == "static"` and no errors |
| 35 | `load_existing_fixture_still_works` | `config.json` (existing fixture) loads and passes (no routing section = static) |

#### Workspace Ceiling Tests (FIX-04, 6 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 36 | `ceiling_workspace_level_exceeds_max_grantable` | Workspace sets `level: 2` when `max_grantable_level: 1` produces `WorkspaceCeilingViolation` |
| 37 | `ceiling_workspace_enables_escalation_when_global_disables` | Workspace `escalation_allowed: true` when global is `false` produces error |
| 38 | `ceiling_workspace_adds_tool_not_in_global` | Workspace adds `"exec_shell"` to `tool_access` when global does not have it |
| 39 | `ceiling_workspace_increases_rate_limit` | Workspace `rate_limit: 120` when global is `60` produces error |
| 40 | `ceiling_workspace_increases_daily_budget` | Workspace `cost_budget_daily_usd: 10.0` when global is `5.0` produces error |
| 41 | `ceiling_workspace_within_bounds_passes` | Workspace that stays within all ceilings returns empty error Vec |

#### Glob-Like Pattern Warning Tests (FIX-12, 2 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 42 | `warn_glob_like_tool_access_pattern` | `tool_access: ["file_*"]` produces `GlobLikeToolAccessPattern` warning |
| 43 | `no_warn_exact_wildcard` | `tool_access: ["*"]` does NOT produce `GlobLikeToolAccessPattern` warning |

#### CLI Admin Safety Tests (FIX-12, 2 tests)

| # | Test Name | Description |
|---|-----------|-------------|
| 44 | `warn_cli_admin_network_exposed` | CLI channel with `level: 2` + gateway bind `0.0.0.0:8080` produces `CliAdminNetworkExposure` warning |
| 45 | `no_warn_cli_admin_localhost` | CLI channel with `level: 2` + gateway bind `127.0.0.1:8080` produces no warning |

### 5.3 Config Fixture: `tests/fixtures/config_tiered_minimal.json`

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-20250514"
    }
  },
  "routing": {
    "mode": "tiered",
    "tiers": [
      {
        "name": "fast",
        "models": ["groq/llama-3.3-70b"],
        "complexity_range": [0.0, 0.5],
        "cost_per_1k_tokens": 0.0003
      },
      {
        "name": "smart",
        "models": ["anthropic/claude-sonnet-4-20250514"],
        "complexity_range": [0.3, 1.0],
        "cost_per_1k_tokens": 0.01
      }
    ]
  }
}
```

### 5.4 Config Fixture: `tests/fixtures/config_tiered_invalid.json`

```json
{
  "routing": {
    "mode": "tiered",
    "tiers": [
      {
        "name": "free",
        "models": ["groq/llama-3.1-8b"],
        "complexity_range": [0.0, 0.3],
        "cost_per_1k_tokens": 0.0
      },
      {
        "name": "free",
        "models": [],
        "complexity_range": [0.8, 0.3],
        "cost_per_1k_tokens": -0.01,
        "max_context_tokens": 0
      }
    ],
    "selection_strategy": "magic_sort",
    "permissions": {
      "zero_trust": {
        "level": 5,
        "max_tier": "mythical",
        "escalation_threshold": 1.5,
        "cost_budget_daily_usd": -1.0
      }
    },
    "cost_budgets": {
      "reset_hour_utc": 25,
      "global_daily_limit_usd": -10.0
    },
    "rate_limiting": {
      "window_seconds": 0
    }
  }
}
```

This fixture should produce at least 10 validation errors when passed through
`validate_routing_config()`.

### 5.5 Effort Estimate

1.5 days. Breakdown:
- `routing_validation.rs` (validation + merge + defaults): 4 hours
- `validate_workspace_ceiling()` (FIX-04): 2 hours
- Glob-like pattern warnings + CLI admin safety (FIX-12): 1 hour
- Config fixture creation: 1 hour
- Test implementation (45 tests): 3 hours
- Integration with `config_loader.rs`: 1 hour

### 5.6 Branch

`weft/tiered-router`

### 5.7 Dependencies

Depends on Phases A-G completing first. Specifically:
- Phase A must have landed the `RoutingConfig`, `ModelTierConfig`, `PermissionsConfig`,
  `PermissionLevelConfig`, `EscalationConfig`, `CostBudgetConfig`, `RateLimitConfig`
  types in `crates/clawft-types/src/routing.rs`.
- Phase B must have landed `UserPermissions` with `level_from_name()` and
  `level_name()` for the known permission level name mapping.
- Phases C-G are runtime consumers and do not need to be complete for Phase H
  to start, but Phase H's tests may reference patterns from those phases.

### 5.8 Definition of Done

1. `routing_validation.rs` compiles with zero warnings (`cargo clippy -p clawft-types`)
2. All 45 tests pass (`cargo test -p clawft-types`)
3. Existing tests still pass (`cargo test --workspace`)
4. `routing_validation.rs` is under 500 lines
5. All public types and functions have doc comments
6. No `unwrap()` on user-provided data (validation is panic-free)
7. No hardcoded secrets or paths
8. `ValidationError` implements `Display` and `Error`
9. `ValidationWarning` implements `Debug` (Display optional)
10. Config fixtures are valid JSON and used by at least one test each

### 5.9 What This Phase Does NOT Include

- No `TieredRouter` construction logic (Phase C)
- No `CostTracker` or `RateLimiter` implementation (Phases D, E)
- No `AuthContext` threading through the pipeline (Phase F)
- No tool permission enforcement (Phase G)
- No integration tests across phases (Phase I)
- No builder pattern for `RoutingConfig` (future enhancement if needed)
- No partial tier-level merge (workspace tiers replace entire array)

---

## Remediation Applied

**Date**: 2026-02-18
**Source**: `/home/aepod/dev/clawft/.planning/sparc/01-tiered-router/remediation-plan.md`

The following fixes from the remediation plan have been applied to this document:

### FIX-01: TierSelectionStrategy Typed Enum
- **Section 1.2.3**: Updated to note that `TierSelectionStrategy` is now a typed enum
  validated automatically by serde. Manual string validation removed from
  `validate_routing_config()`. The `UnknownSelectionStrategy` error variant has been
  removed from the `ValidationError` enum.
- **Section 2.2**: Removed the selection strategy validation block from the pseudocode.
  Added a comment explaining serde handles this automatically.
- **Section 2.6 (Display impl)**: Removed the `UnknownSelectionStrategy` arm.
- **Section 4.1**: Updated severity from ERROR to HANDLED BY SERDE.
- **Section 5.2 test #14**: Marked as REMOVED (serde handles validation).

### FIX-04: Workspace Config Ceiling Enforcement
- **Section 1.3 (NEW)**: Added full specification for `validate_workspace_ceiling()`
  with security-sensitive ceiling fields table: `level`, `escalation_allowed`,
  `tool_access`, `rate_limit`, `cost_budget_daily_usd`, `cost_budget_monthly_usd`,
  `max_tier`.
- **Section 1.3**: Added `max_grantable_level` field specification (default: 1).
- **Section 2.1**: Added `WorkspaceCeilingViolation` error variant to `ValidationError`.
- **Section 2.2 (pseudocode)**: Added full `validate_workspace_ceiling()` implementation.
- **Section 2.3 (deep merge)**: Added `max_grantable_level` to merge (global always wins).
- **Section 2.5 (config loading)**: Added ceiling validation step before routing validation.
- **Section 4.1**: Added "Workspace ceiling violation" to error severity table.
- **Section 5.2**: Added 6 new ceiling tests (#36-41).

### FIX-12: Glob-Like Pattern Warning in tool_access
- **Section 1.2.8 (NEW)**: Added specification for glob-like tool_access pattern warning.
- **Section 2.1**: Added `GlobLikeToolAccessPattern` warning variant.
- **Section 2.2 (pseudocode)**: Added `warn_glob_patterns()` helper and calls for all
  permission level configs, user overrides, and channel overrides.
- **Section 4.1**: Added to warning severity table.
- **Section 5.2**: Added 2 new tests (#42-43).

### FIX-12: CLI Admin Safety Warning
- **Section 1.2.9 (NEW)**: Added specification for CLI admin network exposure warning.
- **Section 2.1**: Added `CliAdminNetworkExposure` warning variant.
- **Section 2.2 (pseudocode)**: Added `validate_cli_admin_network_safety()` function.
- **Section 4.1**: Added to warning severity table.
- **Section 5.2**: Added 2 new tests (#44-45).
- **Section 5.5**: Updated effort estimate from 1 day to 1.5 days.
- **Section 5.8**: Updated test count from 35 to 45.

---

**End of SPARC Plan**
