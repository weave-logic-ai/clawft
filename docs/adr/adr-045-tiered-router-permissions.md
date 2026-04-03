# ADR-045: Tiered Router with Permission-Based Model Selection

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: Architecture Review (`.planning/08-tiered-router.md`)

## Context

The existing `StaticRouter` (Level 0) always returns the same provider/model pair from `config.json`, regardless of task complexity, user identity, or cost. A user asking "what time is it?" routes to the same `anthropic/claude-opus-4-5` as a user asking "design a distributed database with CRDT conflict resolution." This wastes money on trivial tasks and provides no mechanism to restrict model access by user or channel.

The `allow_from` fields on channel configs (Telegram, Slack, Discord, IRC) provide coarse binary access control -- either a user can talk to the bot or they cannot. There is no middle ground for cost-aware routing, per-user budgets, or tier-restricted access.

## Decision

Replace `StaticRouter` with `TieredRouter` that combines three inputs to select the optimal model:

1. **Task complexity** (from `TaskClassifier`): a 0.0-1.0 score indicating request difficulty
2. **User permissions** (from `AuthContext`): resolved from channel auth + config hierarchy
3. **Cost constraints** (from `CostTracker`): daily/monthly per-user budget enforcement

**Three permission levels** with a capability matrix (`UserPermissions` struct):

| Level | Name | Max Tier | Tool Access | Rate Limit | Daily Budget |
|-------|------|----------|-------------|------------|-------------|
| 0 | `zero_trust` | `free` | None | 10 req/min | $0.10 |
| 1 | `user` | `standard` | Basic (read, write, edit, search) | 60 req/min | $5.00 |
| 2 | `admin` | `elite` | All (`["*"]`) | Unlimited | Unlimited |

**Four model tiers** with overlapping complexity ranges:

| Tier | Models | Complexity Range | Cost/1K tokens |
|------|--------|-----------------|----------------|
| `free` | llama-3.1-8b (OpenRouter, Groq) | [0.0, 0.3] | $0.000 |
| `standard` | claude-haiku-3.5, gpt-4o-mini, llama-3.3-70b | [0.0, 0.7] | $0.001 |
| `premium` | claude-sonnet-4, gpt-4o | [0.3, 1.0] | $0.010 |
| `elite` | claude-opus-4-5, o1 | [0.7, 1.0] | $0.050 |

The `TieredRouter` implements the existing `ModelRouter` trait. The routing algorithm: (1) extract `AuthContext` from `ChatRequest` (default to `zero_trust`), (2) check rate limit via `RateLimiter`, (3) filter tiers by permission level, (4) select tier by complexity score with optional escalation (user-level can escalate to premium if complexity exceeds `escalation_threshold`), (5) apply budget constraints via `CostTracker`, (6) select model within tier by `TierSelectionStrategy` (PreferenceOrder, RoundRobin, LowestCost, or Random).

Permission resolution follows a layered hierarchy: built-in level defaults < global config < project config < per-user overrides < per-channel overrides. The `UserPermissions` struct includes glob-pattern model allowlists/denylists, tool allowlists/denylists, context/output token limits, streaming controls, and extensible `custom_permissions: HashMap<String, Value>`.

`StaticRouter` remains the Level 0 default. `TieredRouter` is opt-in via `"routing": { "mode": "tiered" }` in config.

## Consequences

### Positive
- Cost savings: trivial requests use free-tier models instead of premium ones
- Multi-tenant support: different users and channels get appropriate model access without separate deployments
- Budget guardrails: per-user daily/monthly caps prevent runaway costs
- Backward compatible: existing configs without a `routing` section continue using `StaticRouter` unchanged
- Extensible: the capability matrix supports glob patterns, custom dimensions, and per-channel overrides

### Negative
- Complexity scoring accuracy directly affects routing quality; a misclassified complex request on the free tier produces poor results
- Budget state (`CostTracker`) must persist across restarts, adding storage requirements
- The permission resolution hierarchy (5 layers) can be difficult to debug when unexpected routing occurs
- Every channel adapter (Telegram, Slack, Discord, IRC) must thread `AuthContext` through to the router, requiring changes to all channel plugins

### Neutral
- The `TierSelectionStrategy` enum (PreferenceOrder, RoundRobin, LowestCost, Random) provides deployment-time flexibility without code changes
- Rate limiting is per-user, not per-model or per-provider, which simplifies implementation but does not prevent provider-level rate limit exhaustion
- The overlapping complexity ranges are intentional: the router picks the highest-quality tier the user is allowed and can afford
