# Security Review & Hardening Analysis: Tiered Router Sprint

**Reviewer**: Security Architect
**Date**: 2026-02-18
**Sprint**: 01-tiered-router (Phases A through I)
**Design Document**: `.planning/08-tiered-router.md`
**Scope**: Adversarial security analysis of the TieredRouter, permission model, CostTracker, RateLimiter, AuthContext threading, tool permission enforcement, and config parsing/validation.
**Classification**: Internal -- contains attack descriptions and exploit paths.

---

## 1. Threat Model

### 1.1 Attacker Profiles

| Profile | Description | Capabilities | Motivation |
|---------|-------------|--------------|------------|
| **Anonymous External** | Unidentified sender on a public-facing channel (Discord with `allowFrom: []`, open Telegram bot) | Send arbitrary messages; no authenticated identity; assigned `zero_trust` (Level 0) | Abuse free-tier models, exhaust global budgets, DoS via rate limit flooding, probe for tool access |
| **Compromised User** | Valid `allowFrom` user whose platform account was taken over (stolen Telegram session, compromised Discord token) | Has Level 1 (`user`) permissions; can invoke allowed tools; has daily budget | Escalate to premium models, exhaust the legitimate user's budget, exfiltrate workspace files via `read_file`/`web_fetch`, pivot to admin |
| **Malicious Workspace Admin** | Someone who controls a workspace `.clawft/config.json` (project contributor with write access, CI with repo checkout) | Craft workspace config files; influence channel-level and per-user permission overrides | Escalate permissions for all users in the workspace, override global security settings, bypass budget caps, grant themselves admin |
| **Malicious MCP Server Operator** | Operator of an MCP server configured in `tools.mcpServers` | Register tools with arbitrary names and metadata; receive tool call arguments | Namespace spoofing, tool name collision with built-in tools, exfiltrate data passed as tool arguments |
| **Local Attacker (shared CLI)** | Someone with filesystem access but not intended to be an admin (CI runner, shared dev server, Docker container) | Invoke CLI which defaults to `admin` (Level 2); read/modify local config and persistence files | Full admin access by default; read/write all files; exec arbitrary commands; tamper with cost tracking |

### 1.2 Threat Enumeration

#### T-01: Privilege Escalation -- User Manipulating Auth Context to Gain Higher Permissions

**Description**: An attacker forges, spoofs, or manipulates the `AuthContext` to obtain a higher permission level than they are entitled to.

**Attack vectors**:
- Spoofing `sender_id` in `InboundMessage.metadata` via a custom or compromised channel plugin.
- Gateway HTTP API accepting `auth_context` directly in the request body, allowing callers to self-assign admin permissions.
- Workspace config override setting `routing.permissions.users.<attacker_id>.level: 2` to grant admin.
- Unknown numeric levels (e.g., `level: 255`) that map to something other than zero_trust.

**Likelihood**: High. The `sender_id` is a string set by channel plugins with no cryptographic binding.
**Impact**: Critical. Admin permissions grant `exec_shell`, `spawn`, unlimited budget, all model tiers.

#### T-02: Permission Bypass -- Accessing Tools/Models Beyond Assigned Level

**Description**: A user circumvents the permission resolution logic to access tools or models that their level should not permit.

**Attack vectors**:
- Complexity score manipulation to trigger escalation from `free` to `premium` tier (craft messages with "complex" keywords).
- `model_override` flag allowing direct model selection that bypasses tier filtering.
- Glob pattern abuse in `tool_access` if glob matching is ever introduced (currently exact-match only).
- MCP tool namespace collision: MCP server `exec` registers tool `shell`, creating `exec__shell` accessible via wildcard `["*"]`.
- Fallback model configured as an expensive/premium model that zero_trust users reach when their tier is unavailable.

**Likelihood**: Medium. Requires understanding of the routing algorithm or config structure.
**Impact**: High. Access to premium models incurs real API costs; tool access can lead to code execution.

#### T-03: Budget Manipulation -- Evading Cost Tracking to Exceed Budget

**Description**: An attacker exceeds their configured budget cap through race conditions, estimation errors, or persistence tampering.

**Attack vectors**:
- **TOCTOU race**: Two concurrent requests both pass `check_budget()` before either calls `record_estimated()`, causing double-spend.
- **Persistence file tampering**: Attacker with filesystem access deletes or modifies `~/.clawft/cost_tracking.json` to reset their spend to zero.
- **Estimation divergence**: Actual token usage significantly exceeds the pre-flight estimate, and the delta is only reconciled after the expensive call completes.
- **Clock manipulation**: Forward-setting the system clock to trigger a daily reset, clearing all budget counters.
- **Global budget exhaustion**: A single user with a high per-user limit sends enough requests to exhaust the global daily limit, blocking all other users.

**Likelihood**: Medium. The TOCTOU race requires concurrent requests, which is common in chat channels. File tampering requires filesystem access.
**Impact**: High. Unbounded API spend is the primary financial risk.

#### T-04: Rate Limit Evasion -- Circumventing Per-User Rate Limits

**Description**: An attacker bypasses the per-sender_id rate limiter to send more requests than allowed.

**Attack vectors**:
- **sender_id rotation**: On open channels (`allowFrom: []`), create multiple platform accounts. Each gets its own rate limit window and zero_trust budget.
- **Empty sender_id pooling**: Anonymous users with no `sender_id` all share the empty-string key, but if they can provide unique identifiers, they split into separate rate limit buckets.
- **No global rate limit**: The design has per-user limits but no aggregate cap across all users. 1000 unique senders at 10 req/min each = 10,000 req/min aggregate.

**Likelihood**: High on open channels. Discord account creation is easy; Telegram bots can be messaged by anyone.
**Impact**: Medium. Denial of service to the LLM providers; potential for API key rate-limiting or suspension.

#### T-05: Config Injection -- Malicious Config Values Exploiting Deserialization

**Description**: Crafted config values in `routing` section exploit serde deserialization or downstream logic.

**Attack vectors**:
- **Negative cost values**: `cost_per_1k_tokens: -0.01` could cause negative cost recording, effectively granting "credit" to the budget.
- **Escalation threshold out of range**: `escalation_threshold: -1.0` or `escalation_threshold: 2.0` causing unexpected comparison results.
- **Integer overflow**: `max_context_tokens: 18446744073709551615` (u64 max) or `rate_limit: 4294967295` (u32 max) causing unexpected behavior.
- **NaN/Infinity in float fields**: `cost_per_1k_tokens: NaN` or `escalation_threshold: Infinity` bypassing comparison logic.
- **Excessively long strings**: Tier name or model name of 10 MB causing memory pressure during routing.
- **custom_permissions injection**: Workspace config setting `custom_permissions.exec_enabled: true` which a plugin checks for gating exec access.

**Likelihood**: Medium. Requires ability to edit config files (workspace admin, local access).
**Impact**: Medium-High. Depends on which value is crafted; negative costs and NaN are the most dangerous.

#### T-06: Information Disclosure -- Error Messages Leaking Permission Config Details

**Description**: Error messages or routing decision `reason` fields reveal internal permission configuration, budget amounts, or tier structure to end users.

**Attack vectors**:
- The `RoutingDecision.reason` field includes `format!("tiered routing: complexity={:.2}, tier={}, level={}, user={}", ...)` which reveals the user's permission level, selected tier, and sender_id.
- Rate limit error messages like `"sender exceeded 10 requests per 60 seconds"` reveal the exact rate limit configuration.
- Budget exhaustion messages like `"daily budget of $5.00 exceeded"` reveal the user's budget cap.
- `ToolError::PermissionDenied` messages include `"allowed: {:?}"` which dumps the full tool access list.

**Likelihood**: High. These strings are in the design pseudocode.
**Impact**: Low-Medium. Configuration details help attackers calibrate attacks (know exactly how many requests they can send, which tools are restricted).

#### T-07: Denial of Service -- Rate Limiter Memory Exhaustion via Many Unique sender_ids

**Description**: An attacker creates a flood of requests from unique sender_ids to exhaust the rate limiter's memory.

**Attack vectors**:
- Each unique `sender_id` creates a `SlidingWindowEntry` containing a `VecDeque<Instant>` plus a key string. At the default 10 req/min limit, each entry stores up to 10 timestamps.
- With LRU eviction at 10,000 entries: ~5 MB peak memory. But if the attacker sends requests faster than eviction runs, or if `maybe_evict()` is defeated by timing, memory could spike.
- The eviction scan is O(n) over all entries, which at 10,000 entries takes measurable time under the DashMap iteration lock.

**Likelihood**: Medium. Requires high-volume request generation on an open channel.
**Impact**: Medium. Memory exhaustion of the rate limiter degrades service for all users. The LRU cap mitigates this to a bounded ~10 MB.

---

## 2. Attack Surface Analysis

### 2.1 RoutingConfig Types (Phase A)

**Input vectors**:
- JSON config files (global `config.json`, workspace `.clawft/config.json`).
- Serde deserialization of all `routing` section fields.

**Trust boundaries**:
- Global config is operator-controlled (trusted).
- Workspace config is project-contributor-controlled (semi-trusted to untrusted).

**Potential vulnerabilities**:
- Negative values in `cost_per_1k_tokens`, `cost_budget_daily_usd`, `cost_budget_monthly_usd`.
- `escalation_threshold` outside [0.0, 1.0] range.
- `complexity_range` where `min > max`.
- `level` values outside 0-2 range.
- NaN or Infinity in f32/f64 fields (serde_json deserializes these from JSON).
- Duplicate tier names causing HashMap key collisions in `tier_index`.
- `max_context_tokens: 0` causing division-by-zero or zero-length context.
- Excessively large string values for tier names or model identifiers.

**Recommended mitigations**:
- Validate all numeric ranges in Phase H before any runtime use.
- Reject NaN and Infinity values for all float fields.
- Reject negative values for cost and budget fields.
- Enforce `escalation_threshold` in [0.0, 1.0].
- Enforce `complexity_range[0] <= complexity_range[1]` and both in [0.0, 1.0].
- Cap string lengths for tier names (64 chars), model identifiers (256 chars).
- Reject duplicate tier names.
- Enforce `level` in {0, 1, 2}; unknown values default to 0 at the type level.

### 2.2 Permission Resolution (Phase B)

**Input vectors**:
- `sender_id` string from channel plugin metadata.
- `channel` string from channel plugin metadata.
- `PermissionsConfig` with level defaults, per-user overrides, per-channel overrides.
- `allow_from` lists from channel configs.

**Trust boundaries**:
- `sender_id` and `channel` are set by channel plugins. Built-in plugins (Telegram, Discord, Slack) derive these from platform APIs (authenticated by the platform). Custom plugins or the gateway API may not have this guarantee.
- Per-user overrides in workspace config are semi-trusted.

**Potential vulnerabilities**:
- **Level escalation via config layering**: Workspace config sets `users.attacker_id.level: 2`, granting admin. The 5-layer merge applies per-channel overrides at highest priority (priority 5), with per-user at priority 4. **Mitigated**: FIX-04 adds `max_grantable_level` ceiling enforcement (default: 1) preventing workspace configs from granting admin.
- **Unknown level fallback ambiguity**: Phase B spec says `defaults_for_level(99)` returns zero_trust. Phase F spec says `defaults_for_level(255)` returns admin (the `_ => Self::admin()` match arm). This is a specification conflict that must be resolved to zero_trust.
- **Empty sender_id**: The empty string is a valid HashMap key. All anonymous users share the same per-user override entry if one exists for `""`.
- **allow_from semantics confusion**: Empty `allow_from` means "allow all" at the channel access level but does NOT grant Level 1 permissions in the resolver (the `!allow_list.is_empty()` check returns false). This is correct but subtle.

**Recommended mitigations**:
- Resolve the `defaults_for_level` match arm conflict: unknown levels MUST map to zero_trust (level 0), never admin.
- Workspace configs MUST NOT be able to set `level: 2` for any user. Implement a `max_grantable_level` ceiling (default: 1) that only the global config can raise.
- The `PermissionResolver` should treat empty `sender_id` as an explicit anonymous marker and never look up per-user overrides for it.
- Add a `validate()` method to `UserPermissions` that verifies internal consistency after resolution (e.g., `escalation_threshold` in range, `max_tier` is a known tier name).

### 2.3 TieredRouter (Phase C)

**Input vectors**:
- `ChatRequest` with optional `auth_context`.
- `TaskProfile` with `complexity` score from the classifier.
- Tier configuration (models, complexity ranges, costs).

**Trust boundaries**:
- `ChatRequest.auth_context` should only be set by trusted code (AgentLoop). If the ChatRequest is deserialized from an external source (gateway API), the `auth_context` field must be stripped or ignored.
- The `complexity` score comes from `TaskClassifier`. The Level 0 `KeywordClassifier` is trivially gameable.

**Potential vulnerabilities**:
- **Complexity manipulation for escalation**: Users craft messages to maximize keyword complexity scores, triggering escalation from `standard` to `premium` tier.
- **Fallback model permission bypass**: The fallback model is not subject to tier/permission checks. A fallback model set to `anthropic/claude-opus-4-5` allows zero_trust users to reach elite-tier models.
- **model_override bypasses all routing**: Admin users with `model_override: true` skip tier selection, complexity checks, and budget checks. If the admin account is compromised, the attacker gets unrestricted model access.
- **Round-robin counter overflow**: `AtomicUsize` wraps on overflow. With `Ordering::Relaxed`, two threads could briefly see the same counter value, selecting the same model. This is a correctness issue, not a security issue.
- **Empty tiers list panic**: If `self.tiers` is empty and escalation/fallback paths reach `&self.tiers[0]`, this panics. The code must guard against empty tier lists.

**Recommended mitigations**:
- Subject the fallback model to the same tier-based permission checks as any other model. If the fallback model belongs to a tier above the user's `max_tier`, reject it for that user.
- Even with `model_override: true`, enforce global budget limits (`global_daily_limit_usd`, `global_monthly_limit_usd`).
- Add bounds checking before any tier array indexing.
- Log all escalation events and model_override events at `warn` level for audit.
- Add an escalation rate limit: max N escalations per user per hour.

### 2.4 CostTracker (Phase D)

**Input vectors**:
- `user_id` string (from `AuthContext.sender_id`).
- `estimated_cost_usd` and `actual_cost_usd` (f64 values from tier cost calculations).
- Persistence file (`~/.clawft/cost_tracking.json`).

**Trust boundaries**:
- Cost values are computed internally from tier config and token estimates. Not directly user-controlled, but influenced by user's message length and model response length.
- The persistence file is on the local filesystem. Any user with filesystem access can read/modify it.

**Potential vulnerabilities**:
- **TOCTOU in check-then-record**: `check_budget()` and `record_estimated()` are separate operations. Concurrent requests can both pass the check before either records.
- **Persistence file tampering**: Delete file to reset budgets; edit to reduce recorded spend; edit to increase another user's spend (DoS).
- **Negative cost injection**: If `cost_per_1k_tokens` is negative (config validation failure), `record_estimated()` with a negative value would decrease the user's spend, granting effective credit.
- **Reset race condition**: A budget check starts before daily reset, reads pre-reset spend, then the reset clears spend, and the check approves based on stale data. The request executes against the post-reset budget.
- **f64 precision accumulation**: Over millions of small additions, floating-point accumulation error could cause budget checks to be off by a fraction of a cent. Not security-critical but could be surprising.
- **Global budget as shared resource**: A single user can consume the entire global daily limit, blocking all other users. No per-user fairness guarantee on the global limit.

**Recommended mitigations**:
- Use atomic reserve-then-adjust pattern: `DashMap::entry()` with check-and-increment in a single critical section.
- Add HMAC or checksum to the persistence file, keyed on a server-side secret. On load, verify integrity. If invalid, log a warning and start fresh (conservative: assume maximum spend).
- Set file permissions to 0600 on the persistence file.
- Validate that `estimated_cost_usd >= 0.0` before recording. Reject negative values.
- Use epoch-based reset: increment an epoch counter on reset; budget checks record the epoch at start and re-check if it changes.
- Reserve budget based on `max_output_tokens` (maximum possible cost), not an average estimate. Reconcile after actual response.

### 2.5 RateLimiter (Phase E)

**Input vectors**:
- `sender_id` string (arbitrary, attacker-controlled on open channels).
- `limit` value (from resolved `UserPermissions.rate_limit`).

**Trust boundaries**:
- `sender_id` is set by channel plugins. On open channels, each unique platform user gets a unique sender_id.
- `limit` is resolved from config; not directly user-controlled.

**Potential vulnerabilities**:
- **sender_id rotation**: Attacker creates N unique identities, each getting its own rate limit window. N * limit = effective aggregate throughput.
- **Memory exhaustion**: Unique sender_ids create new `SlidingWindowEntry` instances. LRU eviction bounds this, but the eviction scan is O(n) and holds DashMap iteration locks.
- **Empty sender_id sharing**: All anonymous users with empty `sender_id` share one rate limit bucket. This could either cause unfair throttling (legitimate anonymous user blocked by attacker's requests) or be exploited (attacker fills the bucket, then legitimate anonymous users are blocked).
- **window_seconds = 0**: All timestamps expire immediately; every request is allowed. Functionally unlimited, undocumented.
- **DashMap lock contention during eviction**: The `maybe_evict()` method iterates all entries to find the LRU entry. Under high concurrency, this holds read locks on multiple shards.

**Recommended mitigations**:
- Implement a **global aggregate rate limit** (total requests per second across all senders) independent of per-sender limits.
- Implement a **per-channel rate limit** to cap total throughput from each channel.
- Validate `window_seconds > 0` in config parsing (Phase H).
- Consider switching from sliding window (VecDeque of timestamps) to token bucket (two fields per sender: token count + last refill time) for O(1) memory per entry.
- Hard-code the LRU cap maximum and validate the config value.
- Use monotonic time (`Instant::now()`) for all window calculations, never wall clock.

### 2.6 Auth Context Threading (Phase F)

**Input vectors**:
- `InboundMessage.metadata["sender_id"]` -- set by channel plugins.
- `InboundMessage.metadata["channel"]` -- set by channel plugins.
- `ChatRequest.auth_context` -- attached by `AgentLoop`.

**Trust boundaries**:
- Built-in channel plugins (Telegram, Discord, Slack) derive `sender_id` from platform-authenticated APIs. These are trusted.
- The gateway HTTP API and custom channel plugins may not authenticate senders. These are untrusted.
- The `ChatRequest.auth_context` field is `Option<AuthContext>` and is serializable. If a `ChatRequest` is deserialized from an external source (gateway POST body), the caller could inject an arbitrary `AuthContext`.

**Potential vulnerabilities**:
- **Auth context injection via gateway**: If the gateway API deserializes `ChatRequest` from the HTTP body and the caller includes `auth_context: { sender_id: "admin_user", permissions: { level: 2 } }`, the router would treat the request as admin.
- **sender_id forgery via custom plugin**: A custom channel plugin (registered via `channels.extra` or similar) that accepts `sender_id` from an HTTP header without verification.
- **CLI admin default on shared systems**: `sender_id = "local"`, `channel = "cli"` automatically resolves to admin (Level 2). On shared servers, CI runners, or Docker containers, this is a privilege escalation vector.
- **AuthContext is mutable after creation**: If the `AuthContext` is cloned and modified between resolution and consumption, permissions could be altered in the pipeline.

**Recommended mitigations**:
- The gateway API MUST strip `auth_context` from incoming `ChatRequest` bodies. The gateway resolves its own auth (API key, JWT) and constructs `AuthContext` server-side.
- Add a `trusted_source: bool` field to `AuthContext`. Only built-in channel plugins set this to `true`. The router should reject or downgrade untrusted auth contexts.
- When `routing.mode == "tiered"`, emit a startup warning if CLI defaults to admin. Provide `CLAWFT_CLI_PERMISSION_LEVEL` environment variable for CI/shared environments.
- Make `AuthContext` fields `pub(crate)` or provide only a builder pattern, preventing external code from constructing arbitrary contexts.

### 2.7 Tool Permissions (Phase G)

**Input vectors**:
- `tool_name` string (from LLM tool_call response).
- `UserPermissions.tool_access` and `UserPermissions.tool_denylist` lists.
- `ToolMetadata.required_permission_level` and `required_custom_permissions` from tool declarations.

**Trust boundaries**:
- Tool names in `tool_access` lists come from config (operator-controlled).
- Tool names in LLM responses come from the model. The model generates tool calls based on the schemas provided to it. A malicious or confused model could request tools not in the provided schemas.
- MCP tool metadata (`required_permission_level`) comes from the MCP server operator. A malicious MCP server could set `required_permission_level: 0` to make a dangerous tool accessible to everyone.

**Potential vulnerabilities**:
- **Wildcard grants too broad**: `tool_access: ["*"]` matches ALL tools including MCP-namespaced tools. An admin user's wildcard grants access to any MCP tool from any configured server.
- **MCP namespace collision**: MCP server `exec` registers tool `shell`, creating `exec__shell`. Accessible via wildcard. The tool's behavior is controlled by the MCP server, not by clawft's `CommandPolicy`.
- **Denylist bypass via model trickery**: The LLM could be prompted to call a tool using an alias or variation that bypasses exact string matching.
- **tool_access glob risk**: Currently exact-match only. If globs are ever added (as in `model_access`), `"exec*"` would match `exec_shell`. This must never be implemented for `tool_access`.
- **Permission check ordering**: The design spec says `NotFound` is returned before `PermissionDenied`. This is correct (prevents information disclosure about whether a tool exists based on permission errors). But the Phase G pseudocode checks `tools.get(name)` first, revealing that the tool exists before checking permissions. An attacker could enumerate tools by checking which return NotFound vs PermissionDenied.

**Recommended mitigations**:
- `tool_access` MUST support only exact string matching and the literal `"*"` wildcard. No glob patterns.
- Add a default `tool_denylist` for all MCP-namespaced patterns containing `exec`, `shell`, `spawn`, `eval`, `run` keywords. This can be overridden by admin config.
- Consider separate `mcp_tool_access` permission dimension, or require `tool_access` entries for MCP tools to be fully qualified (`server__tool`).
- Return `PermissionDenied` for both "tool not found" and "tool not allowed" when permissions are active, to prevent tool enumeration. Only return `NotFound` when no permissions are provided (backward compat mode).
- Ensure `CommandPolicy` and `UrlPolicy` always execute regardless of permission level. Add regression tests.

---

## 3. Security Requirements

### SR-01: AuthContext Must Be Immutable After Creation

Once the `AgentLoop` constructs an `AuthContext` and attaches it to a `ChatRequest`, no code in the pipeline may mutate the permissions. The `AuthContext` should be treated as a frozen snapshot.

**Implementation**: Make `AuthContext` fields non-public (use getter methods) or wrap in an `Arc` to prevent mutation. At minimum, add a code comment and test that verifies the `AuthContext` on the `ChatRequest` entering the router is identical to the one produced by the resolver.

**Phase**: F (Auth Context Threading)

### SR-02: Permission Resolution Must Fail-Closed

Any error during permission resolution -- config parse failure, missing level defaults, unexpected level value, HashMap lookup failure -- MUST result in `zero_trust` permissions. The system must never grant elevated permissions due to an error in the resolution path.

**Implementation**:
- `defaults_for_level()`: Unknown levels return `zero_trust`, not `admin`. Resolve the specification conflict between Phase B (`_ => Self::zero_trust_defaults()`) and Phase F (`_ => Self::admin()`). The correct behavior is `_ => Self::zero_trust()`.
- `PermissionResolver::resolve()`: Wrap the entire resolution in a `catch_unwind` or use safe combinators. On any unexpected state, return `UserPermissions::zero_trust()`.
- `TieredRouter::route()`: If `request.auth_context` is `None`, default to `AuthContext::default()` which must be zero_trust (not admin).

**Phase**: B (Permissions Resolution), F (Auth Context Threading)

### SR-03: CostTracker Must Use Atomic Operations

The budget check and budget reservation must be a single atomic operation. Separate `check_budget()` + `record_estimated()` calls create a TOCTOU window.

**Implementation**: Replace the two-step check-then-record with a single `reserve_budget()` method:

```rust
fn reserve_budget(&self, user_id: &str, cost: f64, limit: f64) -> bool {
    let mut entry = self.daily.entry(user_id.to_string()).or_insert(0.0);
    if *entry + cost <= limit {
        *entry += cost;
        true
    } else {
        false
    }
}
```

The `DashMap::entry()` API holds a shard-level lock for the duration, making the check-and-increment atomic for that key.

**Phase**: D (CostTracker)

### SR-04: Rate Limiter Must Bound Memory

The rate limiter must have a hard upper bound on memory consumption that cannot be exceeded regardless of attacker behavior.

**Implementation**:
- LRU eviction at `max_entries` (default: 10,000).
- `max_entries` must be validated at config time: minimum 100, maximum 1,000,000.
- Each entry's `VecDeque<Instant>` must be capped at the sender's rate limit value. If the entry has more timestamps than the limit, something is wrong.
- Consider using a fixed-size ring buffer instead of a growable `VecDeque`.

**Phase**: E (RateLimiter)

### SR-05: Config Validation Must Reject Malicious Values

The following values must be rejected during config validation (Phase H):

| Field | Constraint | Reason |
|-------|-----------|--------|
| `cost_per_1k_tokens` | `>= 0.0` and not NaN/Infinity | Negative costs create budget credit |
| `cost_budget_daily_usd` | `>= 0.0` and not NaN/Infinity | Negative budgets have undefined semantics |
| `cost_budget_monthly_usd` | `>= 0.0` and not NaN/Infinity | Same |
| `escalation_threshold` | `>= 0.0 && <= 1.0` and not NaN | Out-of-range values bypass escalation logic |
| `complexity_range[0]` | `>= 0.0 && <= 1.0` | Invalid range |
| `complexity_range[1]` | `>= complexity_range[0] && <= 1.0` | Inverted range |
| `level` | `0, 1, or 2` | Unknown levels must not map to admin |
| `rate_limit` (for zero_trust) | `>= 1` | Zero means unlimited; zero_trust must never be unlimited |
| `window_seconds` | `> 0` | Zero window makes rate limiting meaningless |
| `max_context_tokens` | `> 0` | Zero context is unusable |
| `max_output_tokens` | `> 0` | Zero output is unusable |
| `global_daily_limit_usd` | `>= 0.0` | Negative global limit has undefined semantics |
| `max_escalation_tiers` | `<= total_tiers - 1` | Cannot escalate beyond the tier list |

**Phase**: H (Config Parsing & Validation)

### SR-06: Tool Permission Checks Must Not Be Bypassable via tool_calls in Messages

The LLM can generate `tool_calls` in its response messages. These tool calls must be checked against the user's `tool_access` and `tool_denylist` before execution. The permission check must happen in the `ToolRegistry::execute()` dispatch path, not in the LLM response parsing path, to ensure all tool invocations go through the gate.

**Implementation**:
- `ToolRegistry::execute(name, args, permissions)` checks permissions before dispatch.
- The `AgentLoop` must pass `Some(&permissions)` for every tool call, not just some.
- There must be no code path that calls `tool.execute(args)` directly, bypassing the registry.
- Schema filtering (`schemas_for_user()`) should exclude tools the user cannot access, preventing the LLM from ever seeing restricted tools in its prompt.

**Phase**: G (Tool Permissions)

---

## 4. OWASP Top 10 Mapping

| OWASP Category | Applicable Component | Relevance | Risk | Mitigations in Design |
|----------------|---------------------|-----------|------|----------------------|
| **A01: Broken Access Control** | Permission Resolution, Tool Permissions, Tier Filtering, Escalation Logic | **Critical** | Users accessing tools/models beyond their permission level; workspace config overriding security defaults; fallback model bypassing tier checks | Fail-closed permission resolution; ceiling enforcement on workspace configs; tool allowlist/denylist with denylist-wins semantics |
| **A02: Cryptographic Failures** | CostTracker persistence, AuthContext | Medium | Persistence file has no integrity protection; auth context has no cryptographic binding to the originating channel | Add HMAC to persistence file; restrict file permissions; ensure auth_context is only constructed by trusted code |
| **A03: Injection** | Config deserialization, custom_permissions map | Medium | Malicious config values (negative costs, NaN, crafted custom_permissions) affecting routing logic | Input validation in Phase H; reject NaN/Infinity; range checks on all numeric fields |
| **A04: Insecure Design** | CLI admin default, global budget as shared resource, complexity-based escalation | **High** | CLI defaults to admin on shared systems; single user can exhaust global budget; keyword classifier is trivially gameable | Configurable CLI level; per-user budget enforcement before global; escalation rate limiting |
| **A05: Security Misconfiguration** | Workspace config overrides, open channels with allow_from: [] | **High** | Workspace config can escalate zero_trust permissions; open channels allow unlimited anonymous access | Config ceiling enforcement; startup security audit warnings; documentation of safe defaults |
| **A06: Vulnerable Components** | DashMap, serde_json | Low | Known vulnerabilities in dependencies | Regular dependency audits; `cargo audit` in CI |
| **A07: Auth Failures** | sender_id verification, gateway auth | **High** | sender_id spoofing; gateway accepting self-assigned auth contexts | Platform-verified sender_ids; gateway strips auth_context from request body; trusted_source flag |
| **A08: Software/Data Integrity** | Config deep merge, persistence file | Medium | Workspace config replacing security-critical arrays; persistence file modification | Replace semantics for security arrays; HMAC on persistence; atomic file writes (temp + rename) |
| **A09: Logging/Monitoring Failures** | All components | Medium | No audit trail for permission decisions, escalation events, budget violations, tool access denials | Structured logging for all security-relevant events (Finding 7.3) |
| **A10: SSRF** | web_fetch tool, model provider URLs | Low (existing mitigation) | Tool could fetch internal URLs; model provider URLs could be manipulated | Existing `UrlPolicy` enforcement; tool_access gating on web_fetch; UrlPolicy enforced at all permission levels |

---

## 5. Hardening Recommendations

### Phase A: RoutingConfig Types

1. Add `validate()` method to `RoutingConfig` that checks all numeric ranges, rejects NaN/Infinity, and validates tier name uniqueness.
2. Add string length limits: tier names max 64 chars, model identifiers max 256 chars, sender_id max 256 chars.
3. Do NOT add `#[serde(deny_unknown_fields)]` globally (breaks forward compat), but DO add it to `EscalationConfig` and `CostBudgetConfig` where the field set is stable and security-sensitive.
4. Add a `const MAX_TIERS: usize = 32` limit to prevent config bloat attacks.

### Phase B: UserPermissions and Resolution

1. Resolve the `defaults_for_level()` match arm conflict: ALL unknown levels MUST return `zero_trust()`, never `admin()`.
2. Implement `max_grantable_level` in global config (default: 1). Workspace configs cannot set any user/channel level above this ceiling.
3. The resolver must not look up per-user overrides for empty `sender_id`. Treat empty sender_id as a hard anonymous marker.
4. Add `UserPermissions::validate()` that asserts: `escalation_threshold` in [0.0, 1.0], `level` in {0, 1, 2}, `cost_budget_daily_usd >= 0.0`, `rate_limit > 0 || level > 0` (zero_trust cannot have unlimited rate).
5. Implement ceiling enforcement: when merging workspace overrides, for each security-sensitive field, clamp to the global config ceiling.

### Phase C: TieredRouter Core

1. Fallback model must be permission-checked. If `fallback_model` is in a tier above the user's `max_tier`, the router must reject it for that user and return an empty-provider error decision.
2. Guard against empty `self.tiers` in all code paths. Never index into `self.tiers[0]` without checking `!self.tiers.is_empty()`.
3. Even with `model_override: true`, enforce `global_daily_limit_usd` and `global_monthly_limit_usd`.
4. Add structured audit logging for every routing decision: `{ event: "routing_decision", sender_id, channel, level, complexity, selected_tier, escalated, budget_constrained, model }`.
5. Add an `escalation_count` per user tracked by the router (or CostTracker). If a user triggers more than N escalations per hour, temporarily disable escalation for that user.

### Phase D: CostTracker

1. Replace separate `check_budget()` + `record_estimated()` with atomic `reserve_budget()` using `DashMap::entry()`.
2. Add HMAC-SHA256 to the persistence file using a key derived from the config file path or a server secret. On load, verify HMAC. If invalid, log `warn!` and start fresh.
3. Set file permissions to 0600 after writing the persistence file.
4. Validate `estimated_cost_usd >= 0.0` in `record_estimated()`. Silently reject negative values.
5. Reserve based on `max_output_tokens * cost_per_1k_tokens / 1000.0` (maximum possible cost), not a heuristic estimate.
6. Add a 10% safety margin to budget checks: `if current + estimated * 1.1 > limit { ... }`.
7. Use epoch-based resets: increment an epoch counter on daily/monthly reset. Budget checks compare epochs to detect stale reads.
8. Clamp global totals to 0.0 after reconciliation (already in the design, verify implementation).

### Phase E: RateLimiter

1. Add a **global aggregate rate limit** (total requests per second across all senders). Default to 1000 req/min. Configurable via `routing.rate_limiting.global_limit`.
2. Validate `window_seconds > 0` and `max_entries >= 100` at config time.
3. Use `Instant::now()` (monotonic) for all timestamp operations. Never use `SystemTime`.
4. Consider switching from `VecDeque<Instant>` to a two-field token bucket (token_count: u32, last_refill: Instant) for O(1) memory per entry (~50 bytes vs ~500 bytes).
5. The LRU eviction scan is O(n). For the default 10,000 cap this is acceptable. For higher caps, consider maintaining a separate LRU list or using an `lru` crate.
6. Validate that `rate_limit >= 1` for zero_trust users in config validation. Zero means unlimited, which must never apply to zero_trust.

### Phase F: Auth Context Threading

1. The gateway API MUST strip `auth_context` from deserialized `ChatRequest` bodies. Add `#[serde(skip_deserializing)]` or explicitly overwrite the field after deserialization.
2. Add a `trusted_source: bool` field to `AuthContext` (default: `false`). Only built-in channel plugins set this to `true`. The router should downgrade untrusted contexts to zero_trust.
3. When `routing.mode == "tiered"`, emit a startup warning if the CLI channel resolves to admin and the gateway is bound to a non-loopback address.
4. Support `CLAWFT_CLI_PERMISSION_LEVEL` environment variable (values: "0", "1", "2") that overrides the CLI default level.
5. `AuthContext::default()` MUST return zero_trust permissions, not admin. The CLI admin behavior should be achieved through the resolver (`channel == "cli"` path), not through the default constructor.

### Phase G: Tool Permission Enforcement

1. `tool_access` supports ONLY exact string matching and the literal `"*"` wildcard. Document this. Add a config validation check that rejects entries containing `*` anywhere other than as the sole element.
2. Add a default `tool_denylist` for zero_trust and user levels that includes common dangerous tool patterns. At minimum: `["exec_shell", "spawn"]`.
3. When permissions are active, return the same error type for "tool not found" and "permission denied" to prevent tool enumeration. The error message can differ in server logs but not in user-facing output.
4. Add regression tests that verify `CommandPolicy` is enforced for Level 2 admin users. Even `tool_access: ["*"]` does not bypass `CommandPolicy`.
5. Add `schemas_for_user()` method that filters tool schemas by the user's permissions before sending to the LLM. This prevents the model from generating calls to tools it cannot access.

### Phase H: Config Parsing & Validation

1. Implement **workspace config ceiling enforcement**: workspace configs can restrict permissions (make them more restrictive) but CANNOT expand them beyond global defaults. Specifically:
   - `level` in workspace cannot exceed global config's level for any role.
   - `escalation_allowed` in workspace cannot be `true` if global is `false` for that level.
   - `tool_access` in workspace for any level cannot contain tools not in the global `tool_access` for that level, unless the global uses `["*"]`.
   - `rate_limit` in workspace cannot be lower (less restrictive) than global for that level.
   - `cost_budget_*` in workspace cannot be higher than global for that level.
2. Add a startup security audit that checks for and warns about:
   - `zero_trust.escalation_allowed: true`
   - `zero_trust.tool_access` containing any entries
   - `zero_trust.rate_limit: 0`
   - `zero_trust.cost_budget_daily_usd > 1.0`
   - CLI level = 2 with gateway on non-loopback
   - Any workspace config that attempts to exceed ceilings
3. Validate all float fields for NaN and Infinity. Serde_json does not produce these from standard JSON, but custom deserialization or programmatic construction could.
4. Log a `warn!` for any ceiling violation in workspace config, with the specific field path and the attempted value.

### Phase I: Testing

1. Add a dedicated `tests/security/` directory with adversarial test scenarios.
2. Test every permission escalation path (zero_trust -> user -> admin) through both config overrides and runtime escalation.
3. Test budget race conditions with concurrent requests (spawn multiple tokio tasks).
4. Test rate limiter with sender_id rotation (many unique senders).
5. Test config ceiling enforcement with workspace configs that attempt to grant admin to zero_trust.
6. Test fallback model permission checks.
7. Test `CommandPolicy`/`UrlPolicy` enforcement at all three permission levels.
8. Test that the gateway strips `auth_context` from incoming requests.

---

## 6. Security Testing Plan

### 6.1 Fuzzing Targets

| Target | Input | Property to Verify | Tool |
|--------|-------|--------------------|------|
| `RoutingConfig` deserialization | Random JSON blobs | Never panics; always produces `RoutingConfig` or a serde error | `cargo-fuzz` with `arbitrary` |
| `PermissionsConfig` deserialization | Random JSON blobs | Never panics; unknown levels always resolve to zero_trust | `cargo-fuzz` |
| `UserPermissions::merge()` | Random `PermissionOverrides` | Merged result never has higher privileges than max(base, override) for any security dimension | `proptest` |
| `PermissionResolver::resolve()` | Random (sender_id, channel, config) | Result level never exceeds 2; result always has valid field values | `proptest` |
| `RateLimiter::check()` | Random (sender_id, limit) sequences | Never panics; tracked_senders never exceeds max_entries + 1; always returns bool | `cargo-fuzz` |
| `CostTracker::check_budget()` | Random (user_id, cost, limits) | Never panics; BudgetResult is always a valid variant | `cargo-fuzz` |
| `model_matches_pattern()` | Random (model, pattern) pairs | Never panics; `"*"` always matches; exact strings match themselves | `proptest` |
| `split_provider_model()` | Random strings | Never panics; always returns a tuple of non-empty strings (with default provider) | `proptest` |

### 6.2 Penetration Test Scenarios

#### PEN-01: Privilege Escalation via Workspace Config

**Setup**: Tiered routing enabled with default permissions. Attacker has write access to workspace `.clawft/config.json`.

**Steps**:
1. Write workspace config granting `level: 2` to attacker's sender_id.
2. Send a message via the channel.
3. Verify that the request is routed with admin permissions.

**Expected**: With ceiling enforcement, the workspace config is clamped to `max_grantable_level` (default: 1). The attacker gets user-level, not admin.

#### PEN-02: Budget Double-Spend via Concurrent Requests

**Setup**: User with $0.05 remaining daily budget. Standard tier costs $0.04 per request.

**Steps**:
1. Send 10 concurrent requests from the same sender_id.
2. Measure total recorded spend.

**Expected**: With atomic reserve_budget(), only 1 request passes (reserving $0.04, leaving $0.01 which is insufficient for a second). Without atomic reservations, multiple could pass.

#### PEN-03: Rate Limit Evasion via sender_id Rotation

**Setup**: Open Discord channel (`allowFrom: []`). zero_trust rate limit = 10 req/min.

**Steps**:
1. Send 10 requests from sender_id "user_1" (all pass).
2. Send 10 more from "user_1" (all rejected -- rate limited).
3. Send 10 requests from "user_2" (all pass -- separate rate limit window).
4. Repeat with 100 unique sender_ids.

**Expected**: Each unique sender_id gets its own rate limit window. 100 unique senders = 1000 requests in the window. Mitigation: global aggregate rate limit catches this.

#### PEN-04: Tool Access via MCP Namespace Collision

**Setup**: Admin-configured MCP server named `system`. MCP server registers tool `exec`.

**Steps**:
1. A Level 1 user with `tool_access: ["read_file", "system__search"]` attempts to call `system__exec`.
2. Verify the call is denied (not in allowlist).
3. A Level 2 admin with `tool_access: ["*"]` calls `system__exec`.
4. Verify the call is allowed (wildcard) but `CommandPolicy` still applies.

**Expected**: Exact-match on `tool_access` prevents Level 1 from calling unlisted MCP tools. Admin with wildcard can call it, but `CommandPolicy` gates what the tool can do.

#### PEN-05: Fallback Model Permission Bypass

**Setup**: `fallback_model: "anthropic/claude-opus-4-5"` (elite tier). zero_trust user with `max_tier: "free"`.

**Steps**:
1. All free-tier models are unavailable (provider down).
2. Router falls through to fallback model.
3. Verify that the fallback is subject to tier permission checks.

**Expected**: With the hardening recommendation, the fallback model is checked against `max_tier`. Since `claude-opus-4-5` is in the `elite` tier and the user's max_tier is `free`, the fallback is rejected. An empty-provider error decision is returned.

#### PEN-06: Auth Context Injection via Gateway

**Setup**: Gateway API enabled on `0.0.0.0:3000`.

**Steps**:
1. POST to `/chat` with body including `auth_context: { sender_id: "admin", permissions: { level: 2, tool_access: ["*"] } }`.
2. Verify the injected auth_context is stripped.

**Expected**: The gateway ignores the `auth_context` in the request body. It resolves permissions based on its own authentication mechanism (API key, bearer token). The request is routed with the gateway-resolved permissions.

### 6.3 Property-Based Test Invariants

These invariants must hold for all possible inputs and should be verified with property-based testing (`proptest` or `quickcheck`):

| ID | Invariant | Components |
|----|-----------|------------|
| **PROP-01** | Permissions can only restrict, never expand beyond level defaults. `merge(base, override)` for any override produces a result where no security dimension is more permissive than `max(base_default, override_explicit)`. | Phase B |
| **PROP-02** | `defaults_for_level(n)` for any `n` outside {0, 1, 2} returns a result with `level == 0`. | Phase B |
| **PROP-03** | After `resolve()`, the result's `level` is always in {0, 1, 2}. | Phase B |
| **PROP-04** | For any `UserPermissions` with `escalation_allowed: false`, the `TieredRouter` never selects a tier above `max_tier`. | Phase C |
| **PROP-05** | For any user with `rate_limit: N > 0`, after N calls to `check()` within the window, the (N+1)th call returns `false`. | Phase E |
| **PROP-06** | `RateLimiter::tracked_senders()` never exceeds `max_entries + 1` (one slot for the most recent insertion before eviction runs). | Phase E |
| **PROP-07** | `CostTracker::daily_spend(user)` is always `>= 0.0`. | Phase D |
| **PROP-08** | After `CostTracker::force_daily_reset()`, `daily_spend(user) == 0.0` for all users. | Phase D |
| **PROP-09** | `ToolPermissionChecker::check(name, perms, meta)` returns `Err` for any `name` when `perms.tool_access` is empty. | Phase G |
| **PROP-10** | `ToolPermissionChecker::check(name, perms, meta)` returns `Err` for any `name` in `perms.tool_denylist`, regardless of `tool_access`. | Phase G |
| **PROP-11** | If `RoutingConfig` passes validation (Phase H), then `TieredRouter::from_config()` does not panic. | Phase A, C, H |
| **PROP-12** | For any workspace config, after ceiling enforcement, no security dimension exceeds the global config ceiling. | Phase H |

---

## 7. Confidence Assessment

### Security Claims and Confidence Ratings

| Claim ID | Claim | Confidence | Rationale | Additional Review Needed |
|----------|-------|------------|-----------|--------------------------|
| **SC-01** | zero_trust users cannot access models above the `free` tier without escalation | 98% | Enforced by `filter_tiers_by_permissions()` with ordinal comparison. The `tier_index` lookup defaults to 0 for unknown tier names. | Verify the ordinal default is 0, not the tier list length. |
| **SC-02** | zero_trust users cannot invoke `exec_shell` or `spawn` | 99% | `zero_trust` default has `tool_access: []`. Empty allowlist denies all tools. Denylist is irrelevant when allowlist is empty. | Verify no code path bypasses `ToolRegistry::execute()` permission check. |
| **SC-03** | Budget checks prevent overspend | 85% | TOCTOU race allows double-spend under concurrency. The atomic reserve pattern (SR-03) raises this to ~98% for per-user budgets. Global budget still uses separate atomics that can race. | Implement atomic reserve pattern. Stress-test with concurrent requests. Consider whether global budget needs a Mutex instead of CAS loop. |
| **SC-04** | Rate limiter bounds request throughput | 80% | Per-sender rate limits work correctly. But sender_id rotation on open channels bypasses per-sender limits entirely. No global aggregate limit exists. | Implement global aggregate rate limit. Test with sender_id rotation. |
| **SC-05** | Workspace configs cannot escalate permissions | 70% | The design currently has NO ceiling enforcement. Workspace configs can set `zero_trust.tool_access: ["exec_shell"]` or `users.attacker.level: 2`. This is the highest-severity gap in the design. | Implement ceiling enforcement in Phase H. This is the single most important security fix. |
| **SC-06** | AuthContext cannot be forged | 90% | Built-in channel plugins use platform-authenticated sender IDs. But the gateway API and custom plugins are not verified. The `ChatRequest.auth_context` field is serializable and could be injected. | Implement gateway auth_context stripping. Add trusted_source flag. |
| **SC-07** | CostTracker persistence is tamper-resistant | 60% | No integrity protection on the file. Any user with filesystem access can reset budgets. | Add HMAC to persistence file. Set file permissions to 0600. |
| **SC-08** | Fallback model respects permission boundaries | 50% | The current design does NOT check fallback model permissions. Any user can reach the fallback model regardless of their tier. | Implement fallback model permission check (high priority). |
| **SC-09** | CommandPolicy/UrlPolicy always enforced | 95% | These are checked at the tool execution layer, which runs after the permission check. Defense-in-depth design is correct. Small risk that a future refactor moves tool execution before CommandPolicy. | Add regression tests that verify CommandPolicy fires for admin users. |
| **SC-10** | Config validation rejects all malicious values | 75% | Phase H spec lists many validation rules but NaN/Infinity and negative float validation are not explicitly listed. String length limits are not specified. | Add explicit NaN/Infinity rejection. Add string length limits. Add comprehensive config fuzzing. |
| **SC-11** | Permission resolution determinism | 99% | Resolution is pure computation over immutable config data. HashMap iteration order does not affect results because each layer overwrites (not appends). | Verify with property-based tests. |
| **SC-12** | No information disclosure in error messages | 70% | Current design pseudocode includes detailed permission info in error messages and routing reasons. Rate limit exact values, budget amounts, and tool access lists are exposed. | Sanitize user-facing error messages. Keep detailed info in server logs only. |

### Areas Requiring Additional Review (Confidence < 95%)

| Area | Confidence | What Is Needed |
|------|-----------|----------------|
| **Workspace config ceiling enforcement** | 70% | Implementation in Phase H. Currently no enforcement exists. This is the critical gap. |
| **Budget TOCTOU prevention** | 85% | Atomic reserve pattern implementation and concurrent stress testing. |
| **Rate limit aggregate protection** | 80% | Global rate limit implementation and sender_id rotation testing. |
| **Config validation completeness** | 75% | Comprehensive fuzzing of config deserialization. NaN/Infinity handling. |
| **Information disclosure in errors** | 70% | Audit all error message strings. Separate user-facing from server-log messages. |
| **CostTracker persistence integrity** | 60% | HMAC implementation. File permission enforcement. |
| **Fallback model permission check** | 50% | Implementation required. Currently the fallback bypasses all tier checks. |

---

## 8. Security Invariants Summary

The following invariants MUST hold at all times. They are the minimum security contract of the tiered router system.

### Absolute Invariants (MUST NEVER be violated, regardless of configuration)

| ID | Invariant | Enforcement Point |
|----|-----------|-------------------|
| **INV-001** | `zero_trust` users MUST NOT invoke `exec_shell` or `spawn`, regardless of any config override. | Phase G: ToolPermissionChecker. Phase H: config validation rejects `exec_shell`/`spawn` in zero_trust tool_access. |
| **INV-002** | `CommandPolicy` dangerous command patterns are ALWAYS enforced for ALL permission levels. | Phase G: CommandPolicy runs inside tool.execute(), after ToolPermissionChecker passes. |
| **INV-003** | `UrlPolicy` SSRF protection is ALWAYS enforced for ALL permission levels (when enabled). | Phase G: UrlPolicy runs inside web_fetch tool.execute(). |
| **INV-004** | The `AuthContext` on a `ChatRequest` MUST only be set by trusted channel plugin code, never from user-supplied request data. | Phase F: Gateway strips auth_context from incoming requests. |
| **INV-005** | Unknown permission levels (outside 0-2) MUST resolve to `zero_trust`, never `admin`. | Phase B: `defaults_for_level()` match arm. |

### Configurable Invariants (enforced by default, relaxable only by global admin config)

| ID | Invariant | Default State | Override Mechanism |
|----|-----------|---------------|-------------------|
| **INV-006** | Workspace configs MUST NOT escalate permissions beyond the global config ceiling. | Enforced (max_grantable_level: 1) | Global config sets `max_grantable_level: 2` |
| **INV-007** | `zero_trust` users MUST NOT have `escalation_allowed: true`. | Enforced (zero_trust default: false) | Global config can set `zero_trust.escalation_allowed: true` |
| **INV-008** | The fallback model MUST be subject to tier/permission checks. | Enforced | No override (always enforced) |
| **INV-009** | Budget checks MUST use atomic reserve-then-adjust, not check-then-act. | Enforced | No override (always enforced) |
| **INV-010** | All permission decisions MUST be logged for audit trail. | Enforced at `debug!` level minimum | Configurable log level |

---

## 9. Prioritized Action Items

Ranked by severity and implementation urgency:

| Priority | Finding | Severity | Phase | Action |
|----------|---------|----------|-------|--------|
| **P0** | Workspace config can override security defaults (6.1) | Critical | H | Implement ceiling enforcement |
| **P0** | TOCTOU race in budget checks (3.1) | High | D | Atomic reserve_budget() |
| **P0** | sender_id spoofing via gateway (2.3) | High | F | Strip auth_context from gateway requests |
| **P0** | Fallback model lacks permission check (7.4) | Medium | C | Permission-check fallback model |
| **P1** | sender_id rotation for rate limit evasion (4.1) | High | E | Global aggregate rate limit |
| **P1** | CommandPolicy/UrlPolicy layering verification (5.1) | High | G | Regression tests |
| **P1** | CLI admin default unsafe for shared contexts (2.4) | High | F, H | Env var override, startup warning |
| **P1** | defaults_for_level spec conflict (SR-02) | High | B | Resolve to zero_trust |
| **P2** | Persistence file tampering (3.2) | Medium | D | HMAC + file permissions |
| **P2** | MCP tool namespace collision (5.2) | Medium | G | Default denylist for MCP exec patterns |
| **P2** | Config validation completeness (SR-05) | Medium | H | NaN/Infinity rejection, string length limits |
| **P2** | Information disclosure in errors (T-06) | Low-Medium | C, G | Sanitize user-facing error strings |
| **P3** | Rate limiter memory exhaustion (4.2) | Medium | E | Token bucket alternative, validated max_entries |
| **P3** | Complexity score manipulation (7.1) | Medium | C | Escalation rate limit per user |
| **P3** | Audit logging (7.3) | Medium | C, G | Structured logging for all permission decisions |
| **P3** | custom_permissions unvalidated (6.3) | Medium | H | Document as non-security-boundary; denylist |

---

## 10. Conclusion

The Tiered Router design is architecturally sound, with good separation between authentication (channel plugins), authorization (permission resolution), enforcement (TieredRouter + ToolPermissionChecker), and accounting (CostTracker + RateLimiter). The primary risks are:

1. **No workspace config ceiling enforcement** (Critical). This is the single highest-severity gap. Without it, any project contributor with write access to `.clawft/config.json` can grant themselves admin permissions.

2. **TOCTOU in budget checks** (High). The separate check-then-record pattern allows concurrent double-spend. The fix (atomic reserve) is straightforward.

3. **No global rate limit** (High). Per-sender rate limits are ineffective against sender_id rotation attacks on open channels. A global aggregate limit is required.

4. **Fallback model bypasses permissions** (Medium). The fallback chain must respect the same tier constraints as primary routing.

These four items should be addressed before the sprint is considered security-complete. The remaining findings are important hardening measures that reduce attack surface but do not represent fundamental design flaws.

Overall security posture after implementing all P0 and P1 items: **Adequate for production use on private/semi-private channels. Not recommended for fully public deployment (open Discord/Telegram) without the global rate limit and sender_id rotation mitigations.**
