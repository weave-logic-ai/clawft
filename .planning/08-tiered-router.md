# Tiered Router & Permission System

## 1. Executive Summary

### Problem

clawft's `StaticRouter` (Level 0) always returns the same provider/model pair from `config.json`, regardless of task complexity, user identity, or cost. A user asking "what time is it?" routes to the same `anthropic/claude-opus-4-5` as a user asking "design a distributed database with CRDT conflict resolution." This wastes money on trivial tasks and provides no mechanism to restrict model access by user or channel.

The existing `allow_from` fields on channel configs (Telegram, Slack, Discord) provide coarse binary access control -- either a user can talk to the bot or they cannot. There is no middle ground: no way to say "anonymous users get free models only" or "this Discord channel is limited to mid-tier models" or "user X has a $5/day budget."

### Solution

Replace `StaticRouter` with a **`TieredRouter`** that combines three inputs to select the best model:

1. **Task complexity** (from `TaskClassifier`) -- what the request needs
2. **User permissions** (from channel auth context) -- what the user is allowed
3. **Cost constraints** (from budget tracking) -- what the budget allows

The router maps complexity ranges to model tiers, enforces permission boundaries, applies rate limits and cost budgets, and supports fallback chains when preferred models are unavailable or budget-exhausted.

### Design Principles

- **Additive, not breaking**: `StaticRouter` remains the Level 0 default. `TieredRouter` is opt-in via `"routing": { "mode": "tiered" }` in config.
- **Config-driven**: All tiers, permissions, and budgets are defined in `config.json`. No code changes needed to add models or adjust thresholds.
- **Extensible permissions**: The permission system uses a capability matrix, not just levels. Custom dimensions are supported via `HashMap<String, Value>`.
- **Backward compatible**: Existing configs without a `routing` section continue to use `StaticRouter` unchanged.

---

## 2. Permission Model

### 2.1 Permission Levels

Three built-in permission levels map to progressively broader capabilities. The level determines the ceiling; individual capability dimensions can further restrict within that ceiling.

| Level | Name | Intended Audience | Default Model Tier Access |
|-------|------|-------------------|---------------------------|
| 0 | `zero_trust` | Anonymous, unknown users, untrusted channels | `free` only |
| 1 | `user` | Authenticated users via `allow_from` or channel auth | `free`, `standard` |
| 2 | `admin` | Operators, owners, explicitly granted | `free`, `standard`, `premium`, `elite` |

### 2.2 Level 0: Zero-Trust

The default for any request where the sender cannot be identified or is not listed in any `allow_from` configuration.

| Dimension | Value | Rationale |
|-----------|-------|-----------|
| `max_tier` | `free` | Minimize cost for untrusted users |
| `model_access` | Free-tier models only | e.g., `openrouter/meta-llama/llama-3.1-8b-instruct:free`, `groq/llama-3.1-8b` |
| `tool_access` | `[]` (none) | No tool calls allowed |
| `max_context_tokens` | 4096 | Short context prevents abuse |
| `max_output_tokens` | 1024 | Short responses limit cost |
| `rate_limit` | 10 req/min | Aggressive throttling |
| `streaming_allowed` | `false` | Reduces server resource usage |
| `escalation_allowed` | `false` | Cannot promote to higher tier regardless of complexity |
| `cost_budget_daily_usd` | 0.10 | Hard daily cap |

### 2.3 Level 1: User

For authenticated users identified by channel-specific identifiers (Telegram user ID in `allow_from`, Slack user ID, Discord user ID).

| Dimension | Value | Rationale |
|-----------|-------|-----------|
| `max_tier` | `standard` | Mid-tier models for normal usage |
| `model_access` | Free + standard tier models | e.g., `anthropic/claude-haiku-3.5`, `openai/gpt-4o-mini`, `groq/llama-3.3-70b` |
| `tool_access` | `["read_file", "write_file", "edit_file", "list_dir", "web_search", "web_fetch", "message"]` | Basic tools, no exec/spawn |
| `max_context_tokens` | 16384 | Standard context window |
| `max_output_tokens` | 4096 | Standard response length |
| `rate_limit` | 60 req/min | Reasonable throughput |
| `streaming_allowed` | `true` | Better UX |
| `escalation_allowed` | `true` | Can escalate to `premium` if complexity exceeds threshold |
| `escalation_threshold` | 0.6 | Complexity score at which escalation triggers |
| `cost_budget_daily_usd` | 5.00 | Per-user daily cap |

### 2.4 Level 2: Admin

For operators and explicitly designated admin users. Full access to all models, tools, and features.

| Dimension | Value | Rationale |
|-----------|-------|-----------|
| `max_tier` | `elite` | All model tiers available |
| `model_access` | All configured models | Including `anthropic/claude-opus-4-5`, `openai/o1` |
| `tool_access` | `["*"]` (all) | Including `exec`, `spawn`, MCP tools |
| `max_context_tokens` | 200000 | Full provider maximum |
| `max_output_tokens` | 16384 | Full output length |
| `rate_limit` | 0 (unlimited) | No throttling |
| `streaming_allowed` | `true` | All features enabled |
| `escalation_allowed` | `true` | Always allowed |
| `escalation_threshold` | 0.0 | Escalation at any complexity if needed |
| `cost_budget_daily_usd` | 0.0 (unlimited) | No budget cap |
| `model_override` | `true` | Can manually specify a model via `!model anthropic/claude-opus-4-5` |

---

## 3. Granular Permission Dimensions

Permissions are expressed as a capability matrix, not merely a level integer. Each dimension is independently configurable. The level provides sensible defaults; explicit dimension values override.

### 3.1 Capability Schema

```rust
/// User permission capabilities, resolved from config + auth context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// Permission level (0 = zero_trust, 1 = user, 2 = admin).
    pub level: u8,

    /// Maximum model tier this user can access.
    /// Must match a tier name from the routing config.
    pub max_tier: String,

    /// Explicit model allowlist. Empty = all models in allowed tiers.
    /// Supports glob patterns: "anthropic/*", "openrouter/meta-llama/*".
    #[serde(default)]
    pub model_access: Vec<String>,

    /// Explicit model denylist. Checked after allowlist.
    #[serde(default)]
    pub model_denylist: Vec<String>,

    /// Tool names this user can invoke. ["*"] = all tools.
    #[serde(default)]
    pub tool_access: Vec<String>,

    /// Tool names explicitly denied even if tool_access allows.
    #[serde(default)]
    pub tool_denylist: Vec<String>,

    /// Maximum input context tokens.
    pub max_context_tokens: usize,

    /// Maximum output tokens per response.
    pub max_output_tokens: usize,

    /// Rate limit in requests per minute. 0 = unlimited.
    pub rate_limit: u32,

    /// Whether SSE streaming responses are allowed.
    pub streaming_allowed: bool,

    /// Whether complexity-based escalation to a higher tier is allowed.
    pub escalation_allowed: bool,

    /// Complexity threshold (0.0-1.0) above which escalation triggers.
    /// Only meaningful when escalation_allowed is true.
    pub escalation_threshold: f32,

    /// Whether the user can manually override the model selection.
    #[serde(default)]
    pub model_override: bool,

    /// Daily cost budget in USD. 0.0 = unlimited.
    #[serde(default)]
    pub cost_budget_daily_usd: f64,

    /// Monthly cost budget in USD. 0.0 = unlimited.
    #[serde(default)]
    pub cost_budget_monthly_usd: f64,

    /// Extensible custom permission dimensions.
    /// Used by plugins, MCP servers, and future permission features.
    #[serde(default)]
    pub custom_permissions: HashMap<String, serde_json::Value>,
}
```

### 3.2 Permission Resolution

Permissions are resolved by layering, similar to the config hierarchy:

```
Priority (lowest to highest):
  1. Built-in defaults for the resolved level
  2. Global config:    routing.permissions.<level_name>
  3. Project config:   routing.permissions.<level_name> (workspace override)
  4. Per-user config:  routing.permissions.users.<user_id> (specific overrides)
  5. Per-channel:      routing.permissions.channels.<channel_name> (channel-wide override)
```

This allows configurations like:
- "All users get standard permissions, but Discord users in channel #general are limited to free tier"
- "User `alice` has admin permissions globally, but `bob` has user-level with a $2/day budget"
- "The CLI channel always gets admin permissions (local user = trusted)"

### 3.3 Auth Context Threading

The auth context flows from channel authentication through to the router:

```
Channel plugin (Telegram/Slack/Discord)
  |  identifies sender via platform-specific auth
  |  checks allow_from config
  v
InboundMessage { sender_id, channel, metadata }
  |
  v
AgentLoop
  |  resolves UserPermissions from config + sender_id + channel
  v
PipelineRegistry::complete(request, permissions)
  |
  v
TieredRouter::route(request, profile, permissions)
  |  applies permission constraints to tier selection
  v
RoutingDecision { provider, model, reason, tier, cost_estimate }
```

The `ChatRequest` type gains an optional `auth_context` field:

```rust
/// Authentication context threaded through the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthContext {
    /// Unique sender identifier (platform-specific).
    /// Telegram: user ID, Slack: user ID, Discord: user ID, CLI: "local".
    pub sender_id: String,

    /// Channel name the request originated from.
    pub channel: String,

    /// Resolved permissions for this sender.
    pub permissions: UserPermissions,
}
```

---

## 4. TieredRouter Design

### 4.1 Model Tiers

Model tiers define named groups of models with associated complexity ranges and cost information. Tiers are ordered from cheapest to most expensive.

```rust
/// A named group of models at a similar cost/capability level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTier {
    /// Tier name (e.g., "free", "standard", "premium", "elite").
    pub name: String,

    /// Models available in this tier, in preference order.
    /// Format: "provider/model" (e.g., "anthropic/claude-haiku-3.5").
    pub models: Vec<String>,

    /// Complexity range this tier is suitable for: [min, max].
    /// A request with complexity 0.5 matches tiers where 0.5 is in range.
    pub complexity_range: [f32; 2],

    /// Approximate cost per 1K tokens (blended input/output).
    /// Used for budget estimation, not billing.
    pub cost_per_1k_tokens: f64,

    /// Maximum context tokens supported by models in this tier.
    #[serde(default = "default_tier_max_context")]
    pub max_context_tokens: usize,
}
```

Default tier configuration:

| Tier | Models | Complexity Range | Cost/1K tokens | Use Cases |
|------|--------|-----------------|----------------|-----------|
| `free` | `openrouter/meta-llama/llama-3.1-8b-instruct:free`, `groq/llama-3.1-8b` | [0.0, 0.3] | $0.000 | Chitchat, simple Q&A, greetings |
| `standard` | `anthropic/claude-haiku-3.5`, `openai/gpt-4o-mini`, `groq/llama-3.3-70b` | [0.0, 0.7] | $0.001 | Summaries, basic code, research |
| `premium` | `anthropic/claude-sonnet-4-20250514`, `openai/gpt-4o` | [0.3, 1.0] | $0.010 | Complex code, analysis, multi-step reasoning |
| `elite` | `anthropic/claude-opus-4-5`, `openai/o1` | [0.7, 1.0] | $0.050 | Architecture design, novel algorithms, long-form creative |

Note: Complexity ranges overlap intentionally. A request with complexity 0.5 could route to `standard` or `premium` depending on user permissions and budget. The router picks the highest-quality tier the user is allowed and can afford.

### 4.2 TieredRouter Implementation

```rust
/// Level 1 tiered router that selects models based on
/// task complexity, user permissions, and cost budgets.
pub struct TieredRouter {
    /// Configured model tiers, ordered cheapest to most expensive.
    tiers: Vec<ModelTier>,

    /// Permission level defaults.
    permission_defaults: HashMap<String, UserPermissions>,

    /// Per-user permission overrides.
    user_overrides: HashMap<String, UserPermissions>,

    /// Per-channel permission overrides.
    channel_overrides: HashMap<String, UserPermissions>,

    /// Cost tracker for budget enforcement.
    cost_tracker: Arc<CostTracker>,

    /// Rate limiter for per-user throttling.
    rate_limiter: Arc<RateLimiter>,

    /// Model selection strategy within a tier.
    selection_strategy: TierSelectionStrategy,

    /// Fallback model when all tiers are exhausted or budget-blocked.
    fallback_model: Option<String>,
}

/// How to pick a model within a tier when multiple are available.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
```

The `TieredRouter` implements the existing `ModelRouter` trait. The `route` method signature does not change; the `AuthContext` is carried on the `ChatRequest`:

```rust
#[async_trait]
impl ModelRouter for TieredRouter {
    async fn route(
        &self,
        request: &ChatRequest,
        profile: &TaskProfile,
    ) -> RoutingDecision {
        // 1. Extract auth context (default to zero_trust if absent)
        let auth = request.auth_context.as_ref()
            .cloned()
            .unwrap_or_default();
        let permissions = self.resolve_permissions(&auth);

        // 2. Check rate limit
        if !self.rate_limiter.check(&auth.sender_id, permissions.rate_limit) {
            return self.rate_limited_decision(&permissions);
        }

        // 3. Filter tiers by permission level
        let allowed_tiers = self.filter_tiers_by_permissions(&permissions);

        // 4. Select tier by complexity (with escalation)
        let tier = self.select_tier(
            profile.complexity,
            &allowed_tiers,
            &permissions,
        );

        // 5. Check cost budget
        let tier = self.apply_budget_constraints(
            tier,
            &allowed_tiers,
            &auth,
            &permissions,
        );

        // 6. Select model from tier
        let (provider, model) = self.select_model(&tier);

        // 7. Build decision with audit trail
        RoutingDecision {
            provider,
            model,
            reason: format!(
                "tiered routing: complexity={:.2}, tier={}, level={}, user={}",
                profile.complexity, tier.name, permissions.level, auth.sender_id
            ),
        }
    }

    fn update(&self, decision: &RoutingDecision, outcome: &ResponseOutcome) {
        // Update cost tracker with actual usage
        self.cost_tracker.record(decision, outcome);
    }
}
```

### 4.3 Extended RoutingDecision

The `RoutingDecision` struct is extended with optional metadata for observability. The existing fields remain unchanged for backward compatibility.

```rust
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Provider name (e.g. "openai", "anthropic").
    pub provider: String,

    /// Model identifier (e.g. "gpt-4o", "claude-opus-4-5").
    pub model: String,

    /// Human-readable reason for the routing choice.
    pub reason: String,

    // ── New fields (optional, for tiered routing) ──

    /// The tier that was selected.
    pub tier: Option<String>,

    /// Estimated cost for this request (USD, based on tier cost_per_1k_tokens).
    pub cost_estimate_usd: Option<f64>,

    /// Whether escalation was applied.
    pub escalated: bool,

    /// Whether the decision was constrained by budget.
    pub budget_constrained: bool,
}
```

---

## 5. Routing Algorithm

### 5.1 Decision Flow

```
                         +------------------+
                         |  ChatRequest     |
                         |  (with optional  |
                         |   AuthContext)    |
                         +--------+---------+
                                  |
                    +-------------v--------------+
                    | 1. TaskClassifier::classify |
                    |    -> TaskProfile           |
                    |    (complexity, task_type)  |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 2. Resolve UserPermissions  |
                    |    sender_id + channel      |
                    |    -> level defaults        |
                    |    -> user overrides        |
                    |    -> channel overrides     |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 3. Rate limit check         |
                    |    sender_id, rate_limit    |
                    |    FAIL -> 429 / fallback   |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 4. Filter tiers by          |
                    |    permission max_tier      |
                    |    -> allowed_tiers[]       |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 5. Select tier by           |
                    |    complexity score         |
                    |    (with escalation logic)  |
                    |    -> candidate_tier        |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 6. Budget check             |
                    |    daily + monthly caps     |
                    |    OVER -> fall back to     |
                    |    lower tier               |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 7. Model selection          |
                    |    from tier (preference /  |
                    |    round-robin / lowest)    |
                    +-------------+--------------+
                                  |
                    +-------------v--------------+
                    | 8. Apply token limits       |
                    |    max_context_tokens       |
                    |    max_output_tokens        |
                    |    -> clamp on request      |
                    +-------------+--------------+
                                  |
                         +--------v---------+
                         | RoutingDecision   |
                         | (provider, model, |
                         |  reason, tier,    |
                         |  cost_estimate)   |
                         +------------------+
```

### 5.2 Step Details

**Step 1: Classify task.** The existing `TaskClassifier` (Level 0: `KeywordClassifier`) produces a `TaskProfile` with `complexity` in [0.0, 1.0] and `task_type`. No changes needed to the classifier.

**Step 2: Resolve permissions.** The router resolves the effective `UserPermissions` for this request:

```
effective = defaults_for_level(level)
effective = merge(effective, global_config.routing.permissions.<level_name>)
effective = merge(effective, project_config.routing.permissions.<level_name>)
effective = merge(effective, channel_overrides.<channel_name>)
effective = merge(effective, user_overrides.<sender_id>)
```

If no `auth_context` is present (e.g., CLI mode), the router checks config for a `cli_default_level` (defaults to `admin` for local CLI, since the local user owns the machine).

**Step 3: Rate limit check.** A sliding-window rate limiter keyed by `sender_id`. If the user exceeds their `rate_limit` (requests per minute), the router returns a special `RoutingDecision` with a `rate_limited` reason. The agent loop can return a "please slow down" message or a 429-equivalent.

**Step 4: Filter tiers.** Remove tiers that exceed the user's `max_tier`. Tier ordering is defined in config (cheapest first). If `max_tier` is `"standard"`, only `free` and `standard` tiers are available.

**Step 5: Select tier by complexity.** For each allowed tier, check if `complexity` falls within the tier's `complexity_range`. Among matching tiers, prefer the highest-quality (most expensive) tier, but only if permitted.

Escalation logic: If no allowed tier covers the request's complexity, and `escalation_allowed` is true and `complexity > escalation_threshold`, the router promotes to the next higher tier (if any) beyond `max_tier`. Example: a Level 1 user with `max_tier = "standard"` and `escalation_threshold = 0.6` sends a request classified at complexity 0.8. The `standard` tier covers [0.0, 0.7], so it does not match. With escalation, the router promotes to `premium` for this request.

If no tier matches even with escalation, fall back to the highest allowed tier (it will still handle the request, just potentially at lower quality).

**Step 6: Budget check.** Query the `CostTracker` for the user's spend today and this month. If the selected tier's `cost_per_1k_tokens` would push the estimated cost over budget, fall back to a cheaper tier. Continue falling back until a tier fits the budget or no tiers remain (in which case, use the fallback model or return a budget-exhausted decision).

**Step 7: Model selection.** From the selected tier's model list, pick a model according to the `selection_strategy`:
- `PreferenceOrder`: Use the first model whose provider is configured and has a valid API key.
- `RoundRobin`: Rotate through models in the tier across requests.
- `LowestCost`: Pick the cheapest model (useful when tier has models at different price points).
- `Random`: Random selection for load distribution.

If the user has explicit `model_access` patterns, filter the tier's models against the allowlist before selection. If `model_denylist` is set, remove matching models.

**Step 8: Apply token limits.** Clamp `max_tokens` and context window on the `TransportRequest` to the user's `max_output_tokens` and `max_context_tokens`. This prevents a zero-trust user from requesting 200K context on a free model.

### 5.3 Fallback Chain

When the preferred model is unavailable (provider down, API key missing, rate limited by provider):

```
Selected model in tier
  |  unavailable
  v
Next model in same tier (preference order)
  |  all unavailable
  v
Same-complexity model in next lower tier
  |  all unavailable
  v
Fallback model from config (routing.fallback_model)
  |  unavailable
  v
Error: no models available
```

This integrates with the existing `FailoverController` in `clawft-llm`. The `TieredRouter` selects the target; the transport layer handles provider-level failover within that selection.

---

## 6. Cost Tracking

### 6.1 CostTracker

The `CostTracker` maintains per-user spend records in memory, with periodic persistence to disk.

```rust
/// Tracks cost accumulation per user for budget enforcement.
pub struct CostTracker {
    /// Daily spend per user (sender_id -> accumulated USD).
    daily: DashMap<String, f64>,

    /// Monthly spend per user.
    monthly: DashMap<String, f64>,

    /// Last reset timestamps.
    daily_reset: AtomicU64,
    monthly_reset: AtomicU64,

    /// Persistence path (e.g., ~/.clawft/cost_tracking.json).
    persistence_path: Option<PathBuf>,
}
```

Cost estimation uses the tier's `cost_per_1k_tokens` multiplied by estimated token count (from `AssembledContext::token_estimate` + `max_output_tokens`). Actual cost is updated after the response via `ModelRouter::update()`.

### 6.2 Integration with clawft-llm Cost System

`clawft-llm` already has a `UsageTracker` with real pricing data per model. The `TieredRouter`'s `CostTracker` delegates actual per-model cost calculation to `clawft-llm`'s `ModelCatalog` when available, falling back to the tier's `cost_per_1k_tokens` estimate when the model is not in the catalog.

```
TieredRouter::update(decision, outcome)
  |
  +-- cost_tracker.record_estimated(decision.tier, estimated_tokens)  // budget check
  +-- clawft_llm::UsageTracker.record(model, actual_usage)           // actual cost
```

---

## 7. Configuration Format

### 7.1 Full Configuration Example

The `routing` section is added to the existing `config.json` schema at the top level, alongside `agents`, `channels`, `providers`, etc.

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-20250514",
      "maxTokens": 8192
    }
  },
  "providers": {
    "anthropic": { "apiKey": "sk-ant-..." },
    "openai": { "apiKey": "sk-..." },
    "openrouter": { "apiKey": "sk-or-...", "apiBase": "https://openrouter.ai/api/v1" },
    "groq": { "apiKey": "gsk_..." }
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
        "cli": {
          "level": 2
        },
        "telegram": {
          "level": 1
        },
        "discord": {
          "level": 0
        }
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

### 7.2 Config Type Additions

These types are added to `clawft-types/src/config.rs`:

```rust
/// Routing configuration (top-level, alongside agents/channels/providers).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    /// Routing mode: "static" (default, Level 0) or "tiered" (Level 1).
    #[serde(default = "default_routing_mode")]
    pub mode: String,

    /// Model tier definitions. Only used when mode = "tiered".
    #[serde(default)]
    pub tiers: Vec<ModelTierConfig>,

    /// Model selection strategy within a tier.
    #[serde(default, alias = "selectionStrategy")]
    pub selection_strategy: Option<String>,

    /// Fallback model when all tiers/budgets exhausted.
    #[serde(default, alias = "fallbackModel")]
    pub fallback_model: Option<String>,

    /// Permission level definitions and per-user/channel overrides.
    #[serde(default)]
    pub permissions: PermissionsConfig,

    /// Escalation settings.
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
```

### 7.3 Workspace Override

Project-level `.clawft/config.json` can override routing config via the existing deep-merge system (see `07-workspaces.md` Section 3.3). This allows:

- A private project to restrict all users to `free` tier
- A team project to set higher budgets
- A development project to default all requests to `admin` level

```json
// .clawft/config.json (project-level override)
{
  "routing": {
    "permissions": {
      "channels": {
        "cli": { "level": 2 },
        "telegram": { "level": 0 }
      }
    },
    "cost_budgets": {
      "global_daily_limit_usd": 10.0
    }
  }
}
```

---

## 8. Extensibility

### 8.1 Custom Permission Dimensions

The `custom_permissions: HashMap<String, serde_json::Value>` field supports arbitrary permission data that plugins and future features can query.

Examples:
```json
{
  "custom_permissions": {
    "max_file_size_bytes": 10485760,
    "allowed_mcp_servers": ["filesystem", "web-search"],
    "vision_enabled": true,
    "audio_enabled": false,
    "max_concurrent_subagents": 3
  }
}
```

Plugin code accesses custom permissions via:
```rust
let max_file_size = permissions.custom_permissions
    .get("max_file_size_bytes")
    .and_then(|v| v.as_u64())
    .unwrap_or(1_048_576); // 1 MB default
```

### 8.2 Custom Tiers

The tier list is fully configurable. Users can define any number of tiers beyond the four defaults:

```json
{
  "routing": {
    "tiers": [
      { "name": "local", "models": ["ollama/llama-3.2-3b"], "complexity_range": [0.0, 0.5], "cost_per_1k_tokens": 0.0 },
      { "name": "cloud-fast", "models": ["groq/llama-3.3-70b"], "complexity_range": [0.0, 0.7], "cost_per_1k_tokens": 0.0003 },
      { "name": "cloud-smart", "models": ["anthropic/claude-sonnet-4-20250514"], "complexity_range": [0.3, 1.0], "cost_per_1k_tokens": 0.01 }
    ]
  }
}
```

### 8.3 Plugin-Provided Permissions

MCP servers and skill plugins can declare required permission levels in their metadata. The agent loop checks permissions before dispatching tool calls.

```json
// In an MCP server's tool declaration
{
  "name": "exec_shell",
  "description": "Execute a shell command",
  "required_permission_level": 2,
  "required_custom_permissions": {
    "exec_enabled": true
  }
}
```

The tool dispatch checks:
```rust
if permissions.level < tool.required_permission_level {
    return Err(ToolError::PermissionDenied {
        tool: tool.name.clone(),
        required_level: tool.required_permission_level,
        user_level: permissions.level,
    });
}
```

### 8.4 Future Extensions

These are documented as planned extension points, not current scope:

| Extension | Description | Blocked By |
|-----------|-------------|------------|
| Per-channel permissions | Different permission levels per Discord channel/guild | Channel plugin reporting channel context |
| Per-skill permissions | Skills declare required permission level | Skill metadata schema |
| Dynamic permission grants | Time-limited admin access (e.g., "admin for 1 hour") | Token-based auth system |
| Permission delegation | Admin grants specific permissions to users at runtime | Command interface for permission management |
| Audit logging | Log every permission decision with sender, tier, model, cost | Structured logging infrastructure |
| Usage dashboards | Per-user/channel cost and usage visualization | Metrics export (Prometheus/OpenTelemetry) |
| Team budgets | Shared budget pools across multiple users | User grouping mechanism |
| API key rotation per tier | Different API keys for different tiers (e.g., free tier uses a separate OpenRouter key) | Per-tier provider config |

---

## 9. Integration Points

### 9.1 PipelineRegistry Integration

The `TieredRouter` plugs into the existing `PipelineRegistry` as a drop-in replacement for `StaticRouter`. The pipeline construction in `AgentLoop` checks the routing mode:

```rust
// In AgentLoop initialization
let router: Arc<dyn ModelRouter> = match config.routing.mode.as_str() {
    "tiered" => Arc::new(TieredRouter::from_config(&config.routing, cost_tracker, rate_limiter)),
    _ => Arc::new(StaticRouter::from_config(&config.agents)),
};
```

The `PipelineRegistry::complete()` method does not change. It calls `pipeline.router.route(request, &profile)` regardless of which router implementation is behind the trait.

### 9.2 Channel Auth to Router

The auth context flows from channel plugins through the `InboundMessage` to the `AgentLoop`:

```
TelegramPlugin::start()
  |  receives update, extracts sender user_id
  |  checks config.channels.telegram.allow_from
  |  sets InboundMessage.metadata["sender_id"] = user_id
  |
  v
AgentLoop::process_message(inbound)
  |  extracts sender_id from metadata
  |  resolves permission level:
  |    - sender_id in routing.permissions.users -> use override
  |    - sender_id in channel.allow_from -> level 1 (user)
  |    - otherwise -> level 0 (zero_trust)
  |  attaches AuthContext to ChatRequest
  |
  v
PipelineRegistry::complete(request_with_auth)
  |
  v
TieredRouter::route(request, profile)
  |  reads request.auth_context.permissions
```

### 9.3 Existing allow_from Compatibility

The existing `allow_from` fields on channel configs continue to function as before for binary access control. The tiered router adds a _second_ layer:

| allow_from status | TieredRouter behavior |
|-------------------|-----------------------|
| `allow_from` is empty (allow all) | All users get channel-default permission level |
| User is in `allow_from` | User gets `user` level (Level 1) minimum |
| User is NOT in `allow_from` and list is non-empty | User is rejected by channel plugin (never reaches router) |
| User is in `routing.permissions.users` | User gets the explicitly configured level |
| CLI channel | Always `admin` (Level 2) unless overridden |

This means `allow_from` remains the first gate (channel-level access), and permissions are the second gate (capability-level access). No existing configs break.

### 9.4 Tool Access Enforcement

Tool permission checking happens in the `ToolRegistry` dispatch path, not in the router. The router sets the permission context; the tool registry enforces it:

```rust
// In ToolRegistry::execute()
pub async fn execute(
    &self,
    tool_name: &str,
    args: serde_json::Value,
    permissions: &UserPermissions,
) -> Result<serde_json::Value, ToolError> {
    // Check tool access
    if !permissions.tool_access.contains(&"*".to_string())
        && !permissions.tool_access.contains(&tool_name.to_string())
    {
        return Err(ToolError::PermissionDenied {
            tool: tool_name.into(),
            reason: format!("tool '{}' not in allowed tools for permission level {}", tool_name, permissions.level),
        });
    }
    if permissions.tool_denylist.contains(&tool_name.to_string()) {
        return Err(ToolError::PermissionDenied {
            tool: tool_name.into(),
            reason: format!("tool '{}' is explicitly denied", tool_name),
        });
    }

    // Dispatch to tool implementation
    let tool = self.get(tool_name)?;
    tool.execute(args).await
}
```

### 9.5 Cost Tracking Integration

The `CostTracker` integrates with the existing `clawft-llm::UsageTracker` and `ModelCatalog`:

```
Response arrives with usage: { input_tokens: 1500, output_tokens: 800 }
  |
  v
ModelRouter::update(decision, outcome)
  |
  +-- TieredRouter.cost_tracker.record(
  |       user_id: "alice",
  |       tier: "standard",
  |       estimated_cost: tier.cost_per_1k_tokens * (1500 + 800) / 1000
  |   )
  |
  +-- clawft_llm::UsageTracker.record(
          model: "anthropic/claude-haiku-3.5",
          usage: { input: 1500, output: 800 },
          cost: ModelCatalog::cost("claude-haiku-3.5", 1500, 800)
      )
```

The `CostTracker` uses estimated costs for budget decisions (fast, no external lookup). The `UsageTracker` records actual costs for reporting and analytics.

---

## 10. Migration Path

### Level 0 (Current State): StaticRouter

No changes needed. Existing configs without a `routing` section continue to use `StaticRouter`. The `routing.mode` defaults to `"static"`.

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-20250514"
    }
  }
}
```

### Level 1: TieredRouter with Static Tiers

Add `routing` section to config. No auth system needed -- all requests use the configured default permission level. This is useful for cost optimization even in single-user setups.

```json
{
  "routing": {
    "mode": "tiered",
    "tiers": [
      { "name": "fast", "models": ["groq/llama-3.3-70b"], "complexity_range": [0.0, 0.5], "cost_per_1k_tokens": 0.0003 },
      { "name": "smart", "models": ["anthropic/claude-sonnet-4-20250514"], "complexity_range": [0.3, 1.0], "cost_per_1k_tokens": 0.01 }
    ],
    "permissions": {
      "channels": {
        "cli": { "level": 2 }
      }
    }
  }
}
```

With this config, simple chat goes to Groq (fast, nearly free) while complex tasks route to Claude Sonnet (smart, costs more). All requests from the CLI get admin-level access (all tiers).

### Level 2: TieredRouter with Channel-Based Permissions

Add per-channel permission levels that map to `allow_from` semantics. Users in `allow_from` get Level 1; unknown users get Level 0.

```json
{
  "routing": {
    "mode": "tiered",
    "tiers": [ "..." ],
    "permissions": {
      "channels": {
        "cli": { "level": 2 },
        "telegram": { "level": 1 },
        "discord": { "level": 0 }
      }
    }
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "...",
      "allowFrom": ["user1", "user2"]
    },
    "discord": {
      "enabled": true,
      "token": "...",
      "allowFrom": []
    }
  }
}
```

### Level 3: Full RBAC with Per-User Permissions

Add per-user overrides, cost budgets, and audit logging.

```json
{
  "routing": {
    "mode": "tiered",
    "tiers": [ "..." ],
    "permissions": {
      "zero_trust": { "..." },
      "user": { "..." },
      "admin": { "..." },
      "users": {
        "alice_telegram_123": { "level": 2 },
        "bob_discord_456": { "level": 1, "cost_budget_daily_usd": 2.00 },
        "eve_slack_789": { "level": 1, "tool_access": ["read_file", "web_search"] }
      },
      "channels": {
        "cli": { "level": 2 },
        "telegram": { "level": 1 },
        "discord": { "level": 0 },
        "slack": { "level": 1 }
      }
    },
    "cost_budgets": {
      "global_daily_limit_usd": 50.0,
      "global_monthly_limit_usd": 500.0,
      "tracking_persistence": true
    }
  }
}
```

### Migration Summary

| Migration Level | Effort | Config Changes | Code Changes |
|----------------|--------|----------------|--------------|
| 0 -> 1 | Add `routing` section | Add tiers + mode | None (router is auto-selected) |
| 1 -> 2 | Add `permissions.channels` | Assign levels to channels | None (auth threading already works) |
| 2 -> 3 | Add `permissions.users` + `cost_budgets` | Per-user overrides + budgets | None (all config-driven) |

No migration step requires code changes after the initial `TieredRouter` implementation. All configuration is additive -- existing keys are never removed or renamed.

---

## 11. Implementation Plan

### 11.1 Phased Delivery

| Phase | Scope | Depends On | Effort |
|-------|-------|------------|--------|
| **A** | `RoutingConfig` types in `clawft-types` | None | 0.5 days |
| **B** | `UserPermissions` struct + resolution logic | Phase A | 1 day |
| **C** | `TieredRouter` core (tier selection, model picking) | Phases A, B | 1.5 days |
| **D** | `CostTracker` + budget enforcement | Phase C | 1 day |
| **E** | `RateLimiter` (sliding window) | Phase B | 0.5 days |
| **F** | `AuthContext` threading: channel -> agent loop -> router | Phases B, C | 1 day |
| **G** | Tool permission enforcement in `ToolRegistry` | Phase B | 0.5 days |
| **H** | Config parsing + validation + defaults | Phases A-G | 1 day |
| **I** | Tests: unit, integration, config fixtures | Phases A-H | 1.5 days |
| **Total** | | | **8.5 days** |

### 11.2 File Locations

| Component | Crate | File |
|-----------|-------|------|
| `RoutingConfig`, `ModelTierConfig`, `PermissionsConfig`, `UserPermissions` | `clawft-types` | `src/config.rs` (extend existing) |
| `AuthContext` | `clawft-types` | `src/config.rs` or `src/auth.rs` (new) |
| `TieredRouter` | `clawft-core` | `src/pipeline/tiered_router.rs` (new) |
| `CostTracker` | `clawft-core` | `src/pipeline/cost_tracker.rs` (new) |
| `RateLimiter` | `clawft-core` | `src/pipeline/rate_limiter.rs` (new) |
| Permission resolution | `clawft-core` | `src/pipeline/permissions.rs` (new) |
| Tool permission checks | `clawft-core` | `src/tools/mod.rs` (extend existing) |
| Auth context attachment | `clawft-core` | `src/agent/loop.rs` (extend existing) |
| Config fixture | tests | `tests/fixtures/config_tiered.json` (new) |

---

## 12. Risk Register

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Complexity score too coarse for good tier selection | Medium | Medium | 4 | Level 1+ classifiers (ruvllm) improve accuracy; tiers have overlapping ranges |
| Budget tracking lost on crash (in-memory) | Medium | Low | 3 | Periodic persistence to disk; conservative budget checks |
| Rate limiter memory grows unbounded with many users | Low | Medium | 2 | LRU eviction on rate limiter entries; max 10K tracked users |
| Permission config too complex for users | Medium | Medium | 4 | Sensible defaults; only `routing.mode` and `routing.tiers` needed for basic use |
| Escalation causes unexpected cost spikes | Low | Medium | 2 | `max_escalation_tiers: 1` limits escalation to one tier above max |
| Model availability changes break tier configs | Low | Low | 1 | Fallback chain + `fallback_model` config |
| Config merge conflicts between global and project routing | Low | Medium | 2 | Same deep-merge rules as all other config (project wins at leaf level) |

---

## 13. Success Criteria

- [ ] `StaticRouter` continues to work unchanged when `routing` section is absent
- [ ] `TieredRouter` routes simple requests to `free` tier and complex requests to `premium/elite`
- [ ] Permission Level 0 users cannot access premium models even with high-complexity requests (unless escalation is configured)
- [ ] Permission Level 1 users can escalate to premium when complexity exceeds threshold
- [ ] Permission Level 2 users can access all models and override routing manually
- [ ] Rate limiting rejects requests that exceed the per-user limit
- [ ] Cost budget enforcement falls back to cheaper tiers when budget is near exhaustion
- [ ] Tool access is enforced: Level 0 users cannot invoke `exec` or `spawn`
- [ ] CLI channel defaults to admin permissions
- [ ] Per-user config overrides work (user-specific budgets, tool lists)
- [ ] Config deep merge between global and project routing sections works correctly
- [ ] Existing `allow_from` channel configs continue to function unchanged
- [ ] All new config types serialize/deserialize with backward compatibility (unknown fields ignored)
- [ ] `weft status` reports the active routing mode, tier count, and permission summary
