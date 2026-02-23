# SPARC Implementation Plan: Phase I - Testing & Documentation

**Phase**: I (of A-I) -- Final phase
**Dependencies**: All previous phases (A-H) complete
**Scope**: Unit tests, integration tests, security tests, config fixtures, docs, dev notes
**Design Source**: `.planning/08-tiered-router.md` (Section 13: Success Criteria)

---

## 1. Specification

### 1.1 Overview

Phase I is the capstone that ensures the entire tiered router feature is
production-ready. It covers:

1. Per-phase unit test verification and gap filling
2. Integration tests for the full classify-route-transport pipeline
3. Security-specific tests for permission enforcement
4. Config fixture tests for all migration levels
5. Documentation updates for users and operators
6. Development notes capturing design decisions and implementation details

### 1.2 Unit Test Plan by Phase

Each phase (A-H) should have already written its core unit tests. Phase I
reviews coverage, fills gaps, and adds cross-cutting tests.

#### Phase A: RoutingConfig Types (`clawft-types`)

**Minimum tests**: 12
**Location**: `crates/clawft-types/src/config.rs` (inline `#[cfg(test)]`)

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `routing_config_default` | `RoutingConfig::default()` has mode="static", empty tiers, default permissions |
| 2 | `model_tier_config_serde` | `ModelTierConfig` serializes/deserializes with all fields |
| 3 | `model_tier_config_default_context` | Missing `max_context_tokens` defaults to 200000 |
| 4 | `permissions_config_default` | `PermissionsConfig::default()` has correct zero_trust/user/admin |
| 5 | `user_permissions_default` | `UserPermissions::default()` matches zero_trust values |
| 6 | `escalation_config_default` | enabled=true, threshold=0.6, max_escalation_tiers=1 |
| 7 | `cost_budget_config_default` | All zeros (unlimited), persistence=true |
| 8 | `rate_limit_config_default` | window_seconds=60, strategy=sliding_window |
| 9 | `auth_context_default` | sender_id="", channel="", permissions=zero_trust |
| 10 | `routing_config_snake_case` | `complexity_range`, `cost_per_1k_tokens` parse from JSON |
| 11 | `routing_config_camel_case` | `complexityRange`, `costPer1kTokens` aliases parse from JSON |
| 12 | `routing_config_unknown_fields_ignored` | Extra JSON fields do not cause parse errors |

#### Phase B: UserPermissions & Resolution (`clawft-core`)

**Minimum tests**: 10
**Location**: `crates/clawft-core/src/pipeline/permissions.rs`

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `resolve_zero_trust_defaults` | Unknown sender gets zero_trust permissions |
| 2 | `resolve_user_level` | Sender in allow_from gets user-level permissions |
| 3 | `resolve_admin_level` | CLI channel gets admin-level permissions |
| 4 | `resolve_per_user_override` | User in permissions.users gets explicit config |
| 5 | `resolve_per_channel_override` | Channel in permissions.channels gets channel-level |
| 6 | `resolve_user_override_beats_channel` | Per-user > per-channel in priority |
| 7 | `resolve_workspace_override_applied` | Workspace permissions merged into resolution |
| 8 | `resolve_partial_override_inherits_defaults` | Override with only `level` inherits other fields from level defaults |
| 9 | `merge_permissions_field_by_field` | Only specified fields override, rest preserved |
| 10 | `resolve_custom_permissions_preserved` | `custom_permissions` HashMap flows through |

#### Phase C: TieredRouter Core (`clawft-core`)

**Minimum tests**: 15
**Location**: `crates/clawft-core/src/pipeline/tiered_router.rs`

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `route_simple_to_free_tier` | Complexity 0.1 routes to "free" tier |
| 2 | `route_medium_to_standard_tier` | Complexity 0.5 routes to "standard" tier |
| 3 | `route_complex_to_premium_tier` | Complexity 0.8 routes to "premium" tier |
| 4 | `route_very_complex_to_elite` | Complexity 0.95 routes to "elite" tier |
| 5 | `route_picks_highest_allowed_tier` | Among matching tiers, highest quality selected |
| 6 | `route_respects_max_tier` | User with max_tier=standard cannot get premium |
| 7 | `route_escalation_promotes_tier` | User-level with escalation gets premium for high complexity |
| 8 | `route_escalation_disabled` | zero_trust user never escalates even at complexity 1.0 |
| 9 | `route_escalation_max_tiers_limit` | Escalation limited to max_escalation_tiers |
| 10 | `route_fallback_when_no_tier_matches` | Uses fallback_model when no tier covers complexity |
| 11 | `route_preference_order_strategy` | First model in tier selected |
| 12 | `route_round_robin_strategy` | Models rotate across requests |
| 13 | `route_no_auth_defaults_zero_trust` | Missing auth_context treated as zero_trust |
| 14 | `route_cli_defaults_admin` | CLI channel gets admin permissions |
| 15 | `route_model_access_filter` | User model_access patterns filter tier models |

#### Phase D: CostTracker & Budget Enforcement (`clawft-core`)

**Minimum tests**: 10
**Location**: `crates/clawft-core/src/pipeline/cost_tracker.rs`

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `record_cost_accumulates` | Multiple records sum correctly per user |
| 2 | `check_budget_under_limit` | User under budget returns Ok |
| 3 | `check_budget_over_daily` | User over daily limit returns BudgetExceeded |
| 4 | `check_budget_over_monthly` | User over monthly limit returns BudgetExceeded |
| 5 | `check_budget_unlimited` | Budget 0.0 means unlimited (always Ok) |
| 6 | `global_budget_enforcement` | Global daily limit applies across all users |
| 7 | `daily_reset` | After reset, user spend returns to 0 |
| 8 | `monthly_reset` | Monthly reset clears monthly accumulation |
| 9 | `budget_fallback_to_cheaper_tier` | Route falls back when preferred tier over budget |
| 10 | `concurrent_cost_recording` | Multiple threads recording costs are safe (DashMap) |

#### Phase E: RateLimiter (`clawft-core`)

**Minimum tests**: 8
**Location**: `crates/clawft-core/src/pipeline/rate_limiter.rs`

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `allow_under_limit` | Requests under rate_limit pass |
| 2 | `reject_over_limit` | Request exceeding rate_limit rejected |
| 3 | `window_expiry_allows_new` | After window expires, new requests allowed |
| 4 | `unlimited_rate_always_allows` | rate_limit=0 means unlimited |
| 5 | `per_user_isolation` | User A's limit does not affect User B |
| 6 | `sliding_window_accuracy` | Sliding window correctly counts within window |
| 7 | `rate_limited_decision_reason` | Returned decision has "rate_limited" in reason |
| 8 | `lru_eviction_bounded_memory` | Rate limiter does not grow unbounded |

#### Phase F: AuthContext Threading (`clawft-core`, `clawft-channels`)

**Minimum tests**: 8
**Location**: `crates/clawft-core/src/agent/loop_core.rs`, channel integration tests

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `auth_context_attached_from_telegram` | Telegram message includes sender_id in auth_context |
| 2 | `auth_context_attached_from_discord` | Discord message includes sender_id |
| 3 | `auth_context_attached_from_slack` | Slack message includes sender_id |
| 4 | `auth_context_cli_is_admin` | CLI requests default to admin auth_context |
| 5 | `auth_context_unknown_sender_zero_trust` | Unknown sender gets zero_trust level |
| 6 | `auth_context_flows_to_router` | Router receives auth_context from ChatRequest |
| 7 | `auth_context_flows_to_tool_registry` | Tool dispatch receives permissions |
| 8 | `auth_context_serialization` | AuthContext survives JSON roundtrip |

#### Phase G: Tool Permission Enforcement (`clawft-core`)

**Minimum tests**: 10
**Location**: `crates/clawft-core/src/tools/registry.rs`

| # | Test | What It Verifies |
|---|------|------------------|
| 1 | `allow_tool_in_access_list` | Tool in tool_access is allowed |
| 2 | `deny_tool_not_in_access_list` | Tool not in tool_access is rejected |
| 3 | `wildcard_allows_all_tools` | tool_access=["*"] allows everything |
| 4 | `denylist_overrides_wildcard` | tool_denylist rejects even if wildcard allows |
| 5 | `denylist_overrides_explicit_allow` | tool in both access and denylist is rejected |
| 6 | `zero_trust_no_tools` | Level 0 with empty tool_access rejects all tools |
| 7 | `user_level_basic_tools` | Level 1 allows read_file, web_search etc |
| 8 | `admin_all_tools` | Level 2 with wildcard allows exec, spawn etc |
| 9 | `permission_error_includes_details` | Error includes tool name, user level, required level |
| 10 | `mcp_tool_permission_check` | MCP tools follow same permission rules |

#### Phase H: Config Parsing & Validation

**Minimum tests**: 28 (see Phase H SPARC document for full list)

### 1.3 Integration Tests

Integration tests verify the full pipeline working together.

**Location**: `tests/integration/tiered_router.rs` (new file)

| # | Test | Description |
|---|------|-------------|
| 1 | `full_pipeline_classify_route_free` | Simple message -> KeywordClassifier(0.1) -> TieredRouter -> free tier model |
| 2 | `full_pipeline_classify_route_premium` | Complex message -> KeywordClassifier(0.8) -> TieredRouter -> premium model |
| 3 | `full_pipeline_permission_downgrade` | Complex message + zero_trust user -> limited to free tier |
| 4 | `full_pipeline_escalation` | Complex message + user-level + escalation -> premium allowed |
| 5 | `full_pipeline_budget_fallback` | Premium selected but budget exceeded -> falls back to standard |
| 6 | `full_pipeline_rate_limited` | Rapid requests -> rate limit hit -> appropriate response |
| 7 | `full_pipeline_tool_blocked` | zero_trust user calls exec -> PermissionDenied |
| 8 | `full_pipeline_tool_allowed` | admin user calls exec -> success |
| 9 | `full_pipeline_static_mode_unchanged` | Config with mode=static -> StaticRouter used -> always same model |
| 10 | `full_pipeline_config_roundtrip` | Load config_tiered.json -> build pipeline -> route -> verify |

### 1.4 Config Fixture Tests

**Location**: `tests/integration/config_fixtures.rs` (new file)

| # | Test | Fixture | Description |
|---|------|---------|-------------|
| 1 | `load_original_config` | `config.json` | Existing config still loads (no routing = static) |
| 2 | `load_tiered_config` | `config_tiered.json` | Full tiered config loads and validates |
| 3 | `load_minimal_tiered` | inline JSON | Minimal tiered: mode + 2 tiers only |
| 4 | `load_level1_migration` | inline JSON | Migration Level 1: tiered + no auth (Section 10) |
| 5 | `load_level2_migration` | inline JSON | Migration Level 2: channel-based permissions |
| 6 | `load_level3_migration` | inline JSON | Migration Level 3: full RBAC with per-user |
| 7 | `workspace_merge_overrides` | two inline JSONs | Global + workspace merged correctly |

### 1.5 Security Tests

**Location**: `tests/integration/tiered_router_security.rs` (new file)

These tests verify the security boundaries of the permission system.

| # | Test | Attack Vector | Expected |
|---|------|---------------|----------|
| 1 | `zero_trust_cannot_access_premium` | Set max_tier override to premium in request | Router ignores; uses config-derived permissions |
| 2 | `zero_trust_cannot_use_exec` | zero_trust user calls exec_shell | PermissionDenied |
| 3 | `zero_trust_cannot_use_spawn` | zero_trust user calls spawn | PermissionDenied |
| 4 | `user_cannot_use_exec` | Level 1 user calls exec_shell (not in tool_access) | PermissionDenied |
| 5 | `user_cannot_override_model` | Level 1 user with model_override=false tries override | Override ignored |
| 6 | `budget_cannot_be_bypassed` | User at budget limit sends request | BudgetExceeded or fallback |
| 7 | `rate_limit_cannot_be_bypassed` | User at rate limit sends request | RateLimited response |
| 8 | `escalation_respects_max_tiers` | Level 0 user with escalation disabled | No escalation occurs |
| 9 | `max_tier_ceiling_enforced` | Level 1 user, complexity 1.0, escalation disabled | Standard tier max |
| 10 | `tool_denylist_overrides_wildcard` | Admin with tool_denylist=["spawn"] | spawn rejected despite wildcard |
| 11 | `model_denylist_enforced` | User with model_denylist=["anthropic/*"] | Anthropic models excluded |
| 12 | `token_limits_clamped` | zero_trust user request with max_tokens=200000 | Clamped to 1024 |
| 13 | `cost_tracking_persists_across_requests` | Multiple requests accumulate cost | Budget correctly tracked |
| 14 | `channel_permission_isolation` | Same user different channels | Gets channel-specific permissions |
| 15 | `permission_escalation_from_request_blocked` | Request metadata tries to set level=2 | Ignored; server-side resolution only |
| 16 | `fallback_model_respects_max_tier` (FIX-12) | User with max_tier=standard, no tier matches, fallback_model is in premium tier | Fallback denied; error decision returned instead of unauthorized model access |
| 17 | `rate_limited_fallback_respects_max_tier` (FIX-12) | User rate-limited, rate_limited_decision tries fallback to premium model | Fallback denied if user lacks permission for that tier |

### 1.6 Documentation Updates

#### docs/reference/config.md

Already updated in Phase H. Phase I verifies completeness:

- [ ] `routing` section fully documented with all sub-fields
- [ ] All field types, defaults, and descriptions match implementation
- [ ] All serde aliases documented
- [ ] Permission level table matches code
- [ ] Example JSON matches fixture `config_tiered.json`
- [ ] Migration levels documented with example configs for each

#### docs/guides/tiered-routing.md (FIX-11)

**Standalone file** (not a section in providers.md). Location: `docs/guides/tiered-routing.md`.

This is the primary user-facing guide for tiered model routing.

```markdown
# Tiered Model Routing Guide

clawft supports tiered model routing, where requests are automatically
directed to the most appropriate model based on task complexity, user
permissions, and cost budgets.

## Enabling Tiered Routing

Add a `routing` section to your config.json:

[example config for minimal tiered setup]

## Configuring Tiers

[tier definition format, complexity ranges, cost info]

## Permission Levels

[zero_trust / user / admin explanation with table]

## Tool Permission Matrix (FIX-11)

[tool name -> required level -> description table]

| Tool | Required Level | Description |
|------|---------------|-------------|
| `read_file` | 1 (user) | Read file contents |
| `write_file` | 1 (user) | Write file contents |
| `edit_file` | 1 (user) | Edit file in place |
| `list_dir` | 1 (user) | List directory contents |
| `web_search` | 1 (user) | Search the web |
| `web_fetch` | 1 (user) | Fetch URL content |
| `message` | 1 (user) | Send messages |
| `exec_shell` | 2 (admin) | Execute shell commands |
| `spawn` | 2 (admin) | Spawn subprocesses |
| MCP tools | Varies | Per-tool `required_permission_level` from server declaration |

## Cost Budgets

[per-user and global budget configuration]

## Migration from Static Routing

[step-by-step: static -> tiered -> channel permissions -> full RBAC]
```

#### docs/examples/

Create example config files:

| File | Description |
|------|-------------|
| `docs/examples/config-static.json` | Minimal config, static routing (Level 0) |
| `docs/examples/config-tiered-basic.json` | Tiered with 2 tiers, no permissions (Level 1) |
| `docs/examples/config-tiered-channels.json` | Tiered with channel permissions (Level 2) |
| `docs/examples/config-tiered-full.json` | Full RBAC with per-user overrides (Level 3) |

#### Development Notes

Create: `.planning/development_notes/01-tiered-router/`

| File | Content |
|------|---------|
| `phase-A-types.md` | Config type decisions, serde patterns, alias conventions |
| `phase-B-permissions.md` | Permission resolution algorithm, layering order, merge semantics |
| `phase-C-router.md` | Tier selection logic, escalation algorithm, fallback chain |
| `phase-D-cost-tracker.md` | Budget enforcement, persistence format, reset logic |
| `phase-E-rate-limiter.md` | Sliding window implementation, LRU eviction, DashMap usage |
| `phase-F-auth-context.md` | Auth context flow, channel-to-router threading, CLI defaults |
| `phase-G-tool-permissions.md` | Tool access enforcement, wildcard semantics, denylist priority |
| `phase-H-config-validation.md` | Validation rules, error reporting, deep merge semantics |
| `phase-I-testing.md` | Test strategy, coverage analysis, fixture design |
| `implementation-summary.md` | Overall summary: files changed, tests added, decisions made |

---

## 2. Pseudocode

### 2.1 Test Helper Functions

```rust
/// Create a minimal RoutingConfig for testing with 4 default tiers.
fn test_routing_config() -> RoutingConfig {
    RoutingConfig {
        mode: "tiered".into(),
        tiers: vec![
            ModelTierConfig {
                name: "free".into(),
                models: vec!["groq/llama-3.1-8b".into()],
                complexity_range: [0.0, 0.3],
                cost_per_1k_tokens: 0.0,
                max_context_tokens: 8192,
            },
            ModelTierConfig {
                name: "standard".into(),
                models: vec!["anthropic/claude-haiku-3.5".into()],
                complexity_range: [0.0, 0.7],
                cost_per_1k_tokens: 0.001,
                max_context_tokens: 16384,
            },
            ModelTierConfig {
                name: "premium".into(),
                models: vec!["anthropic/claude-sonnet-4-20250514".into()],
                complexity_range: [0.3, 1.0],
                cost_per_1k_tokens: 0.01,
                max_context_tokens: 200000,
            },
            ModelTierConfig {
                name: "elite".into(),
                models: vec!["anthropic/claude-opus-4-5".into()],
                complexity_range: [0.7, 1.0],
                cost_per_1k_tokens: 0.05,
                max_context_tokens: 200000,
            },
        ],
        selection_strategy: Some("preference_order".into()),
        fallback_model: Some("groq/llama-3.1-8b".into()),
        permissions: test_permissions_config(),
        escalation: EscalationConfig::default(),
        cost_budgets: CostBudgetConfig::default(),
        rate_limiting: RateLimitConfig::default(),
    }
}

/// Create test permissions config using PermissionLevelConfig (FIX-01).
///
/// IMPORTANT: PermissionsConfig contains PermissionLevelConfig instances (all-Option
/// fields for partial overrides in config). These represent raw config values.
/// Use test_resolved_permissions() when you need resolved UserPermissions values.
fn test_permissions_config() -> PermissionsConfig {
    PermissionsConfig {
        zero_trust: PermissionLevelConfig {
            level: Some(0),
            max_tier: Some("free".into()),
            tool_access: Some(vec![]),
            max_context_tokens: Some(4096),
            max_output_tokens: Some(1024),
            rate_limit: Some(10),
            streaming_allowed: Some(false),
            escalation_allowed: Some(false),
            escalation_threshold: Some(1.0),
            model_override: Some(false),
            cost_budget_daily_usd: Some(0.10),
            cost_budget_monthly_usd: Some(2.00),
            ..Default::default()
        },
        user: PermissionLevelConfig {
            level: Some(1),
            max_tier: Some("standard".into()),
            tool_access: Some(vec!["read_file".into(), "web_search".into()]),
            max_context_tokens: Some(16384),
            max_output_tokens: Some(4096),
            rate_limit: Some(60),
            streaming_allowed: Some(true),
            escalation_allowed: Some(true),
            escalation_threshold: Some(0.6),
            cost_budget_daily_usd: Some(5.00),
            cost_budget_monthly_usd: Some(100.00),
            ..Default::default()
        },
        admin: PermissionLevelConfig {
            level: Some(2),
            max_tier: Some("elite".into()),
            tool_access: Some(vec!["*".into()]),
            max_context_tokens: Some(200000),
            max_output_tokens: Some(16384),
            rate_limit: Some(0),
            streaming_allowed: Some(true),
            escalation_allowed: Some(true),
            escalation_threshold: Some(0.0),
            model_override: Some(true),
            cost_budget_daily_usd: Some(0.0),
            cost_budget_monthly_usd: Some(0.0),
            ..Default::default()
        },
        users: HashMap::new(),
        channels: {
            let mut m = HashMap::new();
            m.insert("cli".into(), PermissionLevelConfig {
                level: Some(2),
                ..Default::default()
            });
            m
        },
    }
}

/// Create resolved UserPermissions for a given level (FIX-01).
///
/// Use this when tests need concrete resolved permission values (not config).
/// PermissionLevelConfig is for config construction; UserPermissions is for
/// resolved values that the router and tool registry consume.
fn test_resolved_permissions(level: u8) -> UserPermissions {
    match level {
        0 => UserPermissions {
            level: 0,
            max_tier: "free".into(),
            tool_access: vec![],
            max_context_tokens: 4096,
            max_output_tokens: 1024,
            rate_limit: 10,
            streaming_allowed: false,
            escalation_allowed: false,
            escalation_threshold: 1.0,
            model_override: false,
            cost_budget_daily_usd: 0.10,
            cost_budget_monthly_usd: 2.00,
            ..Default::default()
        },
        1 => UserPermissions {
            level: 1,
            max_tier: "standard".into(),
            tool_access: vec!["read_file".into(), "web_search".into()],
            max_context_tokens: 16384,
            max_output_tokens: 4096,
            rate_limit: 60,
            streaming_allowed: true,
            escalation_allowed: true,
            escalation_threshold: 0.6,
            cost_budget_daily_usd: 5.00,
            cost_budget_monthly_usd: 100.00,
            ..Default::default()
        },
        2 => UserPermissions {
            level: 2,
            max_tier: "elite".into(),
            tool_access: vec!["*".into()],
            max_context_tokens: 200000,
            max_output_tokens: 16384,
            rate_limit: 0,
            streaming_allowed: true,
            escalation_allowed: true,
            escalation_threshold: 0.0,
            model_override: true,
            cost_budget_daily_usd: 0.0,
            cost_budget_monthly_usd: 0.0,
            ..Default::default()
        },
        _ => UserPermissions::default(), // zero_trust
    }
}

/// Create an AuthContext for a specific user and channel (FIX-01).
///
/// Uses test_resolved_permissions() for the permissions field, which returns
/// UserPermissions (resolved values), not PermissionLevelConfig (config values).
fn test_auth_context(sender_id: &str, channel: &str, level: u8) -> AuthContext {
    AuthContext {
        sender_id: sender_id.into(),
        channel: channel.into(),
        permissions: test_resolved_permissions(level),
    }
}

/// Create a ChatRequest with auth context.
fn test_chat_request(message: &str, auth: Option<AuthContext>) -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage::user(message)],
        auth_context: auth,
        ..Default::default()
    }
}

/// Create a TaskProfile with specified complexity.
fn test_task_profile(complexity: f32) -> TaskProfile {
    TaskProfile {
        complexity,
        task_type: TaskType::Chat,
        ..Default::default()
    }
}
```

### 2.2 Integration Test Structure

```rust
// tests/integration/tiered_router.rs

use clawft_types::config::*;
use clawft_core::pipeline::*;

/// Load the tiered config fixture and build a complete pipeline.
fn setup_tiered_pipeline() -> (PipelineRegistry, Config) {
    let config = load_fixture("config_tiered.json");
    let cost_tracker = Arc::new(CostTracker::new(
        config.routing.cost_budgets.clone(),
    ));
    let rate_limiter = Arc::new(RateLimiter::new(
        config.routing.rate_limiting.clone(),
    ));
    let classifier = Arc::new(KeywordClassifier::new());
    let router = Arc::new(TieredRouter::from_config(
        &config.routing,
        cost_tracker,
        rate_limiter,
    ));
    let assembler = Arc::new(TokenBudgetAssembler::new(4096));
    // Transport is mocked -- we only test classify+route
    let pipeline = PipelineRegistry::new(classifier, router, assembler);
    (pipeline, config)
}

#[tokio::test]
async fn full_pipeline_classify_route_free() {
    let (pipeline, _config) = setup_tiered_pipeline();
    let auth = test_auth_context("admin_user", "cli", 2);
    let request = test_chat_request("hello", Some(auth));

    let decision = pipeline.route(&request).await;

    // Simple greeting -> low complexity -> free tier
    assert_eq!(decision.tier.as_deref(), Some("free"));
    assert!(decision.model.contains("llama") || decision.model.contains("groq"));
}

#[tokio::test]
async fn full_pipeline_permission_downgrade() {
    let (pipeline, _config) = setup_tiered_pipeline();
    let auth = test_auth_context("anon", "discord", 0); // zero_trust
    let request = test_chat_request(
        "design a distributed database with CRDT conflict resolution",
        Some(auth),
    );

    let decision = pipeline.route(&request).await;

    // Complex task, but zero_trust -> limited to free tier
    assert_eq!(decision.tier.as_deref(), Some("free"));
}
```

### 2.3 Mock Types for Testing

```rust
/// Mock transport that records what it receives without making HTTP calls.
struct MockTransport {
    calls: Arc<Mutex<Vec<TransportCall>>>,
}

struct TransportCall {
    provider: String,
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: usize,
}

#[async_trait]
impl LlmTransport for MockTransport {
    async fn send(&self, request: &TransportRequest) -> Result<TransportResponse, TransportError> {
        self.calls.lock().push(TransportCall {
            provider: request.provider.clone(),
            model: request.model.clone(),
            messages: request.messages.clone(),
            max_tokens: request.max_tokens,
        });
        Ok(TransportResponse {
            content: "mock response".into(),
            usage: Usage { input_tokens: 100, output_tokens: 50 },
            ..Default::default()
        })
    }
}

/// Mock tool registry that tracks permission checks.
struct MockToolRegistry {
    permission_checks: Arc<Mutex<Vec<PermissionCheck>>>,
}

struct PermissionCheck {
    tool_name: String,
    user_level: u8,
    allowed: bool,
}
```

---

## 3. Architecture

### 3.1 Test File Locations

```
tests/
  fixtures/
    config.json                          # EXISTING -- backward compat
    config_tiered.json                   # NEW -- full tiered config
    config_tiered_invalid.json           # NEW -- config with validation errors
  integration/
    tiered_router.rs                     # NEW -- full pipeline integration tests
    tiered_router_security.rs            # NEW -- security boundary tests
    config_fixtures.rs                   # NEW -- config loading/migration tests

crates/
  clawft-types/src/
    config.rs                            # EXISTING -- add routing type tests
    routing_validation.rs                # NEW (Phase H) -- validation tests inline
  clawft-core/src/
    pipeline/
      tiered_router.rs                   # NEW (Phase C) -- router unit tests inline
      permissions.rs                     # NEW (Phase B) -- permission tests inline
      cost_tracker.rs                    # NEW (Phase D) -- cost tests inline
      rate_limiter.rs                    # NEW (Phase E) -- rate limiter tests inline
    tools/
      registry.rs                        # EXISTING -- add permission tests
    agent/
      loop_core.rs                       # EXISTING -- add auth context tests
  clawft-cli/src/
    commands/
      status.rs                          # EXISTING -- add routing info display + smoke tests

docs/
  reference/
    config.md                            # EXISTING -- update routing section
  guides/
    providers.md                         # EXISTING -- add tiered routing guide
  examples/
    config-static.json                   # NEW
    config-tiered-basic.json             # NEW
    config-tiered-channels.json          # NEW
    config-tiered-full.json              # NEW
```

### 3.2 Running Tests

```bash
# Run all tests
cargo test --workspace

# Run only tiered router tests
cargo test --workspace tiered_router

# Run only config validation tests
cargo test -p clawft-types routing

# Run only permission tests
cargo test -p clawft-core permissions

# Run integration tests
cargo test --test tiered_router
cargo test --test tiered_router_security
cargo test --test config_fixtures

# Run with output for debugging
cargo test --workspace tiered_router -- --nocapture
```

### 3.3 CI Integration

The existing CI workflow (`ci.yml`) runs `cargo test --workspace`. No changes
needed -- new tests are automatically included. The tiered router tests do not
require network access or external services (all providers are mocked).

Test runtime estimate: < 5 seconds for all tiered router tests combined.
No async sleep or real HTTP calls.

### 3.4 `weft status` Routing Info

**Success Criterion**: #14 -- "`weft status` reports the active routing mode,
tier count, and permission summary."

#### 3.4.1 Implementation Guidance

The `weft status` command handler is located at:

```
crates/clawft-cli/src/commands/status.rs
```

The existing `run()` function in `status.rs` loads the config via
`load_config()` and prints agent defaults, gateway, channels, providers, and
tools. The routing info section should be added to the default (non-`--detailed`)
output, immediately after the "Agent defaults:" block and before the `if
args.detailed` branch.

Once Phase A adds `pub routing: RoutingConfig` to the `Config` struct (with
`#[serde(default)]`), the status command can access `config.routing` to display
routing information.

**Key code location**: `status.rs`, function `run()`, approximately after the
`memory_window` println (line ~76 in the current file). Insert a new block:

```rust
// --- Routing info ---
println!();
println!("Routing:");
println!("  Mode:               {}", config.routing.mode);
if config.routing.mode == "tiered" {
    let tier_names: Vec<&str> = config.routing.tiers.iter()
        .map(|t| t.name.as_str())
        .collect();
    println!("  Tiers:              {} ({})",
        config.routing.tiers.len(),
        tier_names.join(", "));
    if let Some(ref strategy) = config.routing.selection_strategy {
        println!("  Active strategy:    {}", strategy);
    }
    if let Some(ref fallback) = config.routing.fallback_model {
        println!("  Fallback model:     {}", fallback);
    }
    // Permission level summary
    println!("  Permission levels:  zero_trust (max_tier={}), user (max_tier={}), admin (max_tier={})",
        config.routing.permissions.zero_trust.max_tier.as_deref().unwrap_or("free"),
        config.routing.permissions.user.max_tier.as_deref().unwrap_or("standard"),
        config.routing.permissions.admin.max_tier.as_deref().unwrap_or("elite"));
} else {
    println!("  (static -- single model, no tiers)");
}
```

If no `routing` field exists in the config (absent section), `RoutingConfig::default()`
produces `mode = "static"` and the output will read:

```
Routing:
  Mode:               static
  (static -- single model, no tiers)
```

When `routing.mode == "tiered"`, the output will read:

```
Routing:
  Mode:               tiered
  Tiers:              3 (free, standard, premium)
  Active strategy:    preference_order
  Fallback model:     groq/llama-3.1-8b
  Permission levels:  zero_trust (max_tier=free), user (max_tier=standard), admin (max_tier=elite)
```

#### 3.4.2 File Ownership

| File | Change |
|------|--------|
| `crates/clawft-cli/src/commands/status.rs` | Add routing info display block in `run()` |

This file is not owned by any prior phase (A-H). Phase I owns it.

**Dependency**: Requires Phase A types (`RoutingConfig`, `ModelTierConfig`,
`PermissionsConfig` with `PermissionLevelConfig`) to be in place before the
code compiles.

#### 3.4.3 Smoke Test

**Location**: `crates/clawft-cli/src/commands/status.rs` (inline `#[cfg(test)]`)
or `tests/integration/tiered_router.rs`

A smoke test that verifies `weft status` displays routing info when tiered
routing is configured:

```rust
#[test]
fn status_routing_info_tiered() {
    // Build a Config with routing.mode = "tiered" and 3 tiers.
    let mut config = Config::default();
    config.routing.mode = "tiered".into();
    config.routing.tiers = vec![
        ModelTierConfig {
            name: "free".into(),
            models: vec!["groq/llama-3.1-8b".into()],
            complexity_range: [0.0, 0.3],
            cost_per_1k_tokens: 0.0,
            max_context_tokens: 8192,
        },
        ModelTierConfig {
            name: "standard".into(),
            models: vec!["anthropic/claude-haiku-3.5".into()],
            complexity_range: [0.0, 0.7],
            cost_per_1k_tokens: 0.001,
            max_context_tokens: 16384,
        },
        ModelTierConfig {
            name: "premium".into(),
            models: vec!["anthropic/claude-sonnet-4-20250514".into()],
            complexity_range: [0.3, 1.0],
            cost_per_1k_tokens: 0.01,
            max_context_tokens: 200000,
        },
    ];
    config.routing.selection_strategy = Some("preference_order".into());

    // Capture the routing display output.
    let output = format_routing_status(&config);

    assert!(output.contains("tiered"), "output must contain 'tiered'");
    assert!(output.contains("3"), "output must contain tier count '3'");
    assert!(output.contains("free"), "output must contain tier name 'free'");
    assert!(output.contains("standard"), "output must contain tier name 'standard'");
    assert!(output.contains("premium"), "output must contain tier name 'premium'");
    assert!(output.contains("preference_order"), "output must contain strategy");
    assert!(output.contains("zero_trust"), "output must contain permission level");
}

#[test]
fn status_routing_info_static() {
    // Default config has mode = "static".
    let config = Config::default();

    let output = format_routing_status(&config);

    assert!(output.contains("static"), "output must contain 'static'");
    assert!(!output.contains("Tiers:"), "static mode should not show tier details");
}
```

**Implementation note**: To make the routing display testable without running the
full async `run()` function, extract the routing display logic into a pure
function `format_routing_status(config: &Config) -> String` that builds the
output string. The `run()` function calls this helper and prints the result.
The tests exercise the helper directly.

---

## 4. Refinement

### 4.1 Coverage Targets

| Crate / Module | Target | Measurement |
|----------------|--------|-------------|
| `clawft-types/config.rs` (routing types) | 95%+ line coverage | All branches of serde deserialization |
| `routing_validation.rs` | 95%+ line coverage | Every error path has a test |
| `tiered_router.rs` | 90%+ line coverage | Core routing logic fully tested |
| `permissions.rs` | 90%+ line coverage | All resolution paths tested |
| `cost_tracker.rs` | 85%+ line coverage | Core budget logic, skip persistence edge cases |
| `rate_limiter.rs` | 85%+ line coverage | Core rate logic, skip LRU edge cases |
| `tools/registry.rs` (permission checks) | 90%+ for new code | All permission paths tested |
| Integration tests | N/A | 10 pipeline tests, 15 security tests, 7 config tests |

### 4.2 Property-Based Testing for Permissions

Consider adding property-based tests (using `proptest` or `quickcheck`) for
permission resolution. These catch edge cases that hand-written tests miss.

```rust
// Example property: permission resolution is deterministic
proptest! {
    #[test]
    fn permission_resolution_deterministic(
        sender_id in "[a-z]{5,10}",
        channel in prop_oneof!["cli", "telegram", "discord", "slack"],
        level in 0u8..=2,
    ) {
        let config = test_routing_config();
        let result1 = resolve_permissions(&config, &sender_id, &channel);
        let result2 = resolve_permissions(&config, &sender_id, &channel);
        assert_eq!(result1.level, result2.level);
        assert_eq!(result1.max_tier, result2.max_tier);
    }
}

// Example property: higher level always >= lower level's tier access
proptest! {
    #[test]
    fn higher_level_has_more_access(
        complexity in 0.0f32..=1.0,
    ) {
        let config = test_routing_config();
        let zero_trust = resolve_for_level(&config, 0);
        let user = resolve_for_level(&config, 1);
        let admin = resolve_for_level(&config, 2);

        // Admin's max_tier index >= user's >= zero_trust's
        let tier_order = ["free", "standard", "premium", "elite"];
        let zt_idx = tier_order.iter().position(|t| *t == zero_trust.max_tier).unwrap_or(0);
        let u_idx = tier_order.iter().position(|t| *t == user.max_tier).unwrap_or(0);
        let a_idx = tier_order.iter().position(|t| *t == admin.max_tier).unwrap_or(0);

        assert!(a_idx >= u_idx);
        assert!(u_idx >= zt_idx);
    }
}
```

### 4.3 Test Isolation

- Each test creates its own config/router/tracker instances
- No shared mutable state between tests
- No filesystem I/O except fixture loading (deterministic paths via `CARGO_MANIFEST_DIR`)
- No network I/O (all providers mocked)
- No time dependencies (cost tracker and rate limiter accept injectable clocks)

### 4.4 Regression Tests for Success Criteria

Each success criterion from Section 13 of the spec maps to at least one test:

| Criterion | Test(s) |
|-----------|---------|
| StaticRouter works unchanged when routing absent | `load_original_config`, `full_pipeline_static_mode_unchanged` |
| TieredRouter routes simple to free, complex to premium | `route_simple_to_free_tier`, `route_complex_to_premium_tier` |
| Level 0 cannot access premium | `zero_trust_cannot_access_premium`, `route_respects_max_tier` |
| Level 1 can escalate to premium | `route_escalation_promotes_tier`, `full_pipeline_escalation` |
| Level 2 can access all models | `route_cli_defaults_admin`, tests using admin auth |
| Rate limiting rejects excess requests | `reject_over_limit`, `full_pipeline_rate_limited` |
| Budget enforcement falls back | `budget_fallback_to_cheaper_tier`, `full_pipeline_budget_fallback` |
| Tool access enforced | `zero_trust_no_tools`, `zero_trust_cannot_use_exec` |
| CLI defaults to admin | `resolve_admin_level`, `auth_context_cli_is_admin` |
| Per-user overrides work | `resolve_per_user_override` |
| Config deep merge works | `merge_workspace_*` tests |
| Existing allow_from unchanged | `load_original_config` (no routing = no change) |
| Backward compat (unknown fields ignored) | `routing_config_unknown_fields_ignored` |
| `weft status` reports routing mode, tier count, permission summary | `status_routing_info_tiered`, `status_routing_info_static` (Section 3.4.3) |

---

## 5. Completion

### 5.1 Testing Checklist

#### Unit Tests (per phase)

- [ ] Phase A: 12+ routing config type tests passing
- [ ] Phase B: 10+ permission resolution tests passing
- [ ] Phase C: 15+ tiered router core tests passing
- [ ] Phase D: 10+ cost tracker tests passing
- [ ] Phase E: 8+ rate limiter tests passing
- [ ] Phase F: 8+ auth context threading tests passing
- [ ] Phase G: 10+ tool permission tests passing
- [ ] Phase H: 28+ config parsing/validation tests passing

**Total unit tests: 101+ new tests**

#### `weft status` Routing Info (Section 3.4)

- [ ] `status_routing_info_tiered` -- tiered config displays mode, tier count, tier names, strategy, permissions
- [ ] `status_routing_info_static` -- static/default config displays mode, no tier details

**Total `weft status` tests: 2 new tests**

#### Integration Tests

- [ ] 10 full pipeline integration tests passing
- [ ] 17 security boundary tests passing (15 original + 2 fallback permission tests from FIX-12)
- [ ] 7 config fixture tests passing

**Total integration tests: 34+ new tests**

#### Aggregate

- [ ] All 137+ new tests passing (135 original + 2 `weft status` smoke tests)
- [ ] All existing 1,058 tests still passing (no regressions)
- [ ] `cargo test --workspace` reports 0 failures
- [ ] `cargo clippy --workspace -- -D warnings` reports 0 warnings

### 5.2 Documentation Checklist

- [ ] `docs/reference/config.md` -- routing section complete with all fields
- [ ] `docs/reference/config.md` -- permission levels table
- [ ] `docs/reference/config.md` -- migration path documented (Level 0-3)
- [ ] `docs/reference/config.md` -- complete example JSON updated
- [ ] `docs/guides/tiered-routing.md` -- standalone tiered routing guide (FIX-11)
- [ ] `docs/guides/tiered-routing.md` -- tool permission matrix (tool name -> required level -> description) (FIX-11)
- [ ] `docs/guides/tiered-routing.md` -- migration from static to tiered documented
- [ ] `docs/examples/config-static.json` created
- [ ] `docs/examples/config-tiered-basic.json` created
- [ ] `docs/examples/config-tiered-channels.json` created
- [ ] `docs/examples/config-tiered-full.json` created
- [ ] All example configs are valid JSON and pass validation

### 5.3 Development Notes Checklist

**REQUIREMENT (FIX-11):** Each phase MUST produce dev notes upon completion.
Dev notes are a mandatory deliverable, not optional. The implementing agent
for each phase is responsible for writing its dev notes file before the phase
is considered done. Dev notes capture design decisions, implementation gotchas,
and test coverage that would otherwise be lost.

- [ ] `.planning/development_notes/01-tiered-router/` directory created
- [ ] `phase-A-types.md` -- config type decisions documented (Phase A agent writes this)
- [ ] `phase-B-permissions.md` -- resolution algorithm documented (Phase B agent writes this)
- [ ] `phase-C-router.md` -- routing algorithm documented (Phase C agent writes this)
- [ ] `phase-D-cost-tracker.md` -- budget enforcement documented (Phase D agent writes this)
- [ ] `phase-E-rate-limiter.md` -- rate limiter design documented (Phase E agent writes this)
- [ ] `phase-F-auth-context.md` -- auth flow documented (Phase F agent writes this)
- [ ] `phase-G-tool-permissions.md` -- tool access enforcement documented (Phase G agent writes this)
- [ ] `phase-H-config-validation.md` -- validation rules documented (Phase H agent writes this)
- [ ] `phase-I-testing.md` -- test strategy and results documented (Phase I agent writes this)
- [ ] `implementation-summary.md` -- overall summary with metrics (Phase I agent writes this)

### 5.4 Dev Notes Template

Each phase dev notes file should follow this structure:

```markdown
# Phase X: [Name] -- Development Notes

**Date**: YYYY-MM-DD
**Author**: [agent name]
**Files changed**: [list]
**Tests added**: [count]

## Design Decisions

### Decision 1: [Title]
- **Context**: Why this decision was needed
- **Options considered**: What alternatives existed
- **Chosen**: What was picked and why
- **Consequences**: What this means for the codebase

## Implementation Notes

### Key patterns
- [Pattern 1 with code reference]
- [Pattern 2 with code reference]

### Gotchas
- [Thing that was tricky or surprising]

## Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| [module] | [count] | [estimate] |

## Open Questions

- [Any unresolved questions for future phases]
```

### 5.5 Final Exit Criteria

The tiered router feature is considered complete when:

- [ ] All 137+ new tests pass (135 original + 2 `weft status` smoke tests)
- [ ] All 1,058 existing tests pass (zero regressions)
- [ ] `cargo check --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` succeeds
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds with 0 failures
- [ ] All 14 success criteria from spec Section 13 have corresponding tests
- [ ] Documentation covers all new config fields
- [ ] Example configs for all 4 migration levels exist
- [ ] Development notes for all 9 phases exist
- [ ] No hardcoded secrets or API keys in any new code
- [ ] All new files follow existing code conventions (serde patterns, test patterns)
- [ ] `config.json` (existing fixture) still loads unchanged
- [ ] Empty `{}` config still loads with all defaults

---

## Remediation Applied

**Date**: 2026-02-18
**Source**: `/home/aepod/dev/clawft/.planning/sparc/01-tiered-router/remediation-plan.md`

The following fixes from the remediation plan have been applied to this document:

### FIX-11: Tool Permission Matrix in Documentation Checklist
- **Section 1.6**: Added tool permission matrix table to the `docs/guides/tiered-routing.md`
  outline. Matrix maps tool name -> required level -> description for all built-in tools
  and notes MCP tool per-server `required_permission_level` declarations.
- **Section 5.2**: Added documentation checklist item for tool permission matrix.

### FIX-11: Clarify Guide Location as docs/guides/tiered-routing.md
- **Section 1.6**: Changed from "docs/guides/providers.md -- Add a new section" to
  "docs/guides/tiered-routing.md -- Standalone file". The tiered routing guide is its
  own file, not a section appended to the providers guide.
- **Section 5.2**: Updated checklist from `docs/guides/providers.md` references to
  `docs/guides/tiered-routing.md`.

### FIX-11: Per-Phase Dev Notes Requirement
- **Section 5.3**: Added explicit requirement statement: each phase MUST produce dev
  notes upon completion. Dev notes are a mandatory deliverable. Updated each checklist
  item to note which phase agent is responsible for writing it.

### FIX-01: Test Helpers Use PermissionLevelConfig for Config, UserPermissions for Resolved
- **Section 2.1**: Rewrote `test_permissions_config()` to return `PermissionsConfig`
  containing `PermissionLevelConfig` instances (all-Option fields) instead of
  `UserPermissions` instances. This matches the Phase A type architecture where config
  types use `PermissionLevelConfig` and runtime types use `UserPermissions`.
- **Section 2.1**: Added new `test_resolved_permissions(level)` helper that returns
  concrete `UserPermissions` values for a given level. Tests that need resolved
  permissions (router, tool registry) use this helper.
- **Section 2.1**: Updated `test_auth_context()` to use `test_resolved_permissions()`
  for the `permissions` field.
- Added doc comments explaining the distinction between config construction helpers
  (`PermissionLevelConfig`) and resolved value helpers (`UserPermissions`).

### FIX-12: Security Integration Test for Fallback Model Permission Checking
- **Section 1.5**: Added security tests #16 (`fallback_model_respects_max_tier`) and
  #17 (`rate_limited_fallback_respects_max_tier`). These verify that `fallback_chain()`
  and `rate_limited_decision()` do not return a fallback model from a tier above the
  user's `max_tier`. A user with `max_tier=standard` must not receive a premium-tier
  fallback model.
- **Section 5.1**: Updated security test count from 15 to 17 and total integration
  test count from 32 to 34.
- **Section 5.5**: Updated total new test count from 133 to 135.
