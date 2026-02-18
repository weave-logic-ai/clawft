# Security Gap Analysis: Tiered Router SPARC Plans

**Reviewer**: Security Architect
**Date**: 2026-02-18
**Scope**: Cross-referencing security-review.md findings against implementation plans (B through H), design doc (08-tiered-router.md), and consensus log.

---

## 1. Security Review Finding -> Plan Mapping

For each finding in security-review.md, this section verifies whether the corresponding SPARC plan addresses it.

### Finding 2.1: Zero-Trust Escalation via Workspace Config (High, 98%)

**Phases**: B, H

- **Phase B status**: NOT ADDRESSED. The `PermissionResolver` in Phase B applies workspace overrides (Step 4, line 629) without any ceiling enforcement. A workspace config that sets `zero_trust.escalation_allowed: true` would be merged directly. The merge function (Section 2.4) blindly applies `Some(escalation_allowed)` without checking whether the override expands permissions beyond the global config.
- **Phase H status**: PARTIALLY ADDRESSED. Phase H validates individual field values (escalation_threshold in [0.0, 1.0], budget non-negative, etc.) but does NOT implement workspace ceiling enforcement. The `deep_merge_routing()` function (Section 2.2) performs key-level merging without security boundary checks. There is no comparison of workspace values against global values to enforce "workspace can restrict but not expand."

**[GAP-S01]** CRITICAL. Workspace config ceiling enforcement is completely absent from all implementation plans. A workspace `.clawft/config.json` can set `zero_trust.escalation_allowed: true`, `zero_trust.tool_access: ["exec_shell"]`, or `zero_trust.cost_budget_daily_usd: 1000.0` with no guard. This was the #1 prioritized action item in the security review.
- **Affected Plans**: B (permission resolution), H (config validation)
- **Recommended Fix**: Add a `validate_workspace_ceiling()` function in Phase H that compares every security-sensitive dimension of the workspace config against the global config. The resolver in Phase B must accept both global and workspace overrides separately, applying ceiling enforcement before merging. Security-sensitive fields requiring ceiling enforcement: `escalation_allowed`, `tool_access`, `rate_limit`, `cost_budget_daily_usd`, `cost_budget_monthly_usd`, `max_tier`, `level`.
- **Confidence**: 99%

---

### Finding 2.2: Tool Access Glob Pattern Risk (Medium, 95%)

**Phases**: G, H

- **Phase G status**: ADDRESSED. The `ToolPermissionChecker::check()` (Section 2.1 / 5.5) uses exact string matching via `.iter().any(|s| s == tool_name)` for `tool_access`, and only the literal `"*"` triggers wildcard behavior. Glob patterns like `"exec*"` would NOT match `exec_shell` because the check is exact match, not glob match.
- **Phase H status**: NOT ADDRESSED. Phase H validates individual fields but does not explicitly reject glob-like patterns in `tool_access`. A config with `tool_access: ["exec*"]` would parse successfully but fail to match anything at runtime (silent misconfiguration).

**[GAP-S02]** LOW. The implementation is safe by default (exact match only), but there is no config validation warning for glob-like patterns in `tool_access` that would silently do nothing.
- **Affected Plans**: H (config validation)
- **Recommended Fix**: Add a validation warning in Phase H for any `tool_access` entry containing `*` that is not exactly `"*"`, informing the user that glob patterns are not supported.
- **Confidence**: 95%

---

### Finding 2.3: sender_id Spoofing (High, 97%)

**Phases**: F

- **Phase F status**: PARTIALLY ADDRESSED. Phase F establishes the convention that channel plugins set `metadata["sender_id"]` from platform-verified sources (e.g., Telegram API, Discord gateway). The plan documents that "Channel plugins MUST NOT accept sender_id from message content" (Section 4, Security Considerations). However, Phase F does NOT implement the recommended `trusted_source` flag on `AuthContext`, does NOT prevent the gateway HTTP API from accepting `auth_context` in the request body, and does NOT enumerate trusted channel plugin sources.

**[GAP-S03]** HIGH. The `AuthContext` struct in Phase F (Section 2) has no `trusted_source: bool` field. The `ChatRequest` in Phase C (Section 3.4) has `auth_context: Option<AuthContext>` as a public serde-deserializable field. This means a caller submitting a JSON `ChatRequest` to the gateway HTTP API can craft an arbitrary `auth_context` with `level: 2` (admin) permissions. There is no mechanism to distinguish a plugin-constructed `AuthContext` from a user-supplied one.
- **Affected Plans**: F (auth context threading), C (ChatRequest extension)
- **Recommended Fix**: (1) Add `trusted_source: bool` to `AuthContext`, defaulting to `false`. Channel plugins set it to `true`. (2) Mark `auth_context` on `ChatRequest` as `#[serde(skip_deserializing)]` so it cannot be injected via JSON. The gateway must construct `AuthContext` server-side from its own authentication (API key, JWT). (3) The `TieredRouter` should reject or ignore `AuthContext` where `trusted_source == false` and fall back to zero_trust.
- **Confidence**: 98%

---

### Finding 2.4: CLI Admin Default Unsafe for Shared/CI Contexts (High, 99%)

**Phases**: F, H

- **Phase B status**: PARTIALLY ADDRESSED. The `PermissionResolver` has `cli_default_level: u8` configurable via constructor (Section 2.5, line 571), defaulting to `2` (admin). This is configurable but defaults to admin.
- **Phase F status**: NOT ADDRESSED. Phase F hardcodes `channel == "cli"` -> `return 2` (line 351-352) without any safety checks, startup warnings, or environment variable override (`CLAWFT_CLI_PERMISSION_LEVEL`).
- **Phase H status**: NOT ADDRESSED. No validation or warning for `cli_default_level == 2` when `gateway.host == "0.0.0.0"`.

**[GAP-S04]** HIGH. No startup warning, no environment variable override, and no detection of network-exposed gateway with admin CLI defaults. The security review recommended three mitigations; none are implemented.
- **Affected Plans**: F (auth context threading), H (config validation)
- **Recommended Fix**: (1) In Phase F, check `CLAWFT_CLI_PERMISSION_LEVEL` env var and use it if set. (2) In Phase H, add a startup warning when `cli_default_level == 2` AND (`gateway.host == "0.0.0.0"` OR `routing.mode == "tiered"`). (3) Document that CI/CD environments should set `CLAWFT_CLI_PERMISSION_LEVEL=0`.
- **Confidence**: 99%

---

### Finding 2.5: Per-User Override Allows Arbitrary Escalation (Medium, 96%)

**Phases**: B, H

- **Phase B status**: NOT ADDRESSED. The `PermissionResolver::determine_level()` (Section 2.5, line 656-658) directly returns the per-user override's `level` field without any ceiling check. A workspace config setting `users.attacker.level: 2` grants admin.
- **Phase H status**: NOT ADDRESSED. Validation checks that `level` is 0, 1, or 2 (valid values) but does NOT enforce that workspace-level user overrides cannot grant `level: 2` without global authorization.

**[GAP-S05]** MEDIUM. This is a subset of GAP-S01 (workspace ceiling enforcement). A workspace config can grant `level: 2` to any user. The fix from GAP-S01 should also cover this case: workspace user overrides MUST NOT exceed `max_grantable_level` from the global config.
- **Affected Plans**: B (permission resolution), H (config validation)
- **Recommended Fix**: Add `max_grantable_level` field to `RoutingConfig` (default: 1). Workspace-level `users` entries cannot set `level` above this value. Per-user `level: 2` must originate from the global config only.
- **Confidence**: 97%

---

### Finding 3.1: TOCTOU Race in Budget Checks (High, 95%)

**Phases**: D

- **Phase D status**: PARTIALLY ADDRESSED. The `CostTracker` design uses `DashMap::entry()` for `record_estimated()` (Section 2.6, line 511-514), which is atomic for individual map operations. However, `check_budget()` (Section 2.5) and `record_estimated()` are called as separate methods. The `TieredRouter` in Phase C calls `check_budget()` in `apply_budget_constraints()` and then `record_estimated_cost()` later (Section 2.14, line 969). Between these two calls, another concurrent request can pass the same budget check.

The security review recommended a "reserve then adjust" pattern using `DashMap::entry()` with an atomic compare-and-update. Phase D's `check_budget()` does NOT perform an atomic reservation -- it reads the value and returns a `BudgetResult`, leaving the caller to separately call `record_estimated()`.

**[GAP-S06]** HIGH. The budget check and reservation are not atomic. The security review's recommended `reserve_budget()` function (Section 3.1) combining check-and-reserve in a single `DashMap::entry()` call is not implemented. Phase D's `check_budget()` is a pure read; `record_estimated()` is a separate write. Two concurrent requests can both pass the check before either records.
- **Affected Plans**: D (CostTracker)
- **Recommended Fix**: Replace the separate `check_budget()` + `record_estimated()` pattern with a single `reserve_budget()` method that atomically checks and reserves within a `DashMap::entry()` lock. Return `BudgetResult` from this combined method. After the LLM response, call `reconcile_actual()` to adjust the reservation.
- **Confidence**: 96%

---

### Finding 3.2: Persistence File Tampering (Medium, 99%)

**Phases**: D

- **Phase D status**: PARTIALLY ADDRESSED. Phase D uses atomic file writes (temp + rename, Section 2.3 line 311-312) which prevents corruption. However, there is no HMAC/checksum on the persistence file, no restrictive file permissions (0600) set after writing, and no "prefer higher value" logic when loading from disk vs. in-memory state.

**[GAP-S07]** MEDIUM. The persistence file `~/.clawft/cost_tracking.json` can be modified by any user with filesystem access to reduce their recorded spend.
- **Affected Plans**: D (CostTracker)
- **Recommended Fix**: (1) Set file permissions to 0600 in the `save()` method after write. (2) On load, if the file shows LOWER spend than in-memory state, prefer in-memory (conservative). (3) HMAC/checksum is desirable but lower priority for V1; can be deferred.
- **Confidence**: 95%

---

### Finding 3.3: Cross-User Budget Exhaustion (Low, 90%)

**Phases**: D

- **Phase D status**: ADDRESSED. The `CostTracker` enforces global limits independently of per-user limits (Section 2.5, Steps 3-4). The check order is: user daily -> user monthly -> global daily -> global monthly. This matches the security review's recommendation that global budget is checked after per-user budgets.

No gap identified.

---

### Finding 3.4: Estimated vs Actual Cost Divergence (Medium, 93%)

**Phases**: D

- **Phase D status**: PARTIALLY ADDRESSED. Phase D provides `estimate_cost()` (Section 2.9) using `cost_per_1k_tokens * (input_tokens + max_output_tokens) / 1000.0`, which uses `max_output_tokens` (the maximum, not an estimate). The `record_actual()` method (Section 2.6) reconciles the delta. However, there is no safety margin, and Phase C's `apply_budget_constraints()` (Section 2.10, line 651) uses a rough estimate of just `tier.cost_per_1k_tokens` (approximately 1K tokens), not the full `input_tokens + max_output_tokens` formula.

**[GAP-S08]** MEDIUM. Phase C's budget check uses an underestimate (just `cost_per_1k_tokens` for ~1K tokens), while Phase D provides a proper estimation function that Phase C does not call.
- **Affected Plans**: C (TieredRouter), D (CostTracker)
- **Recommended Fix**: Phase C's `apply_budget_constraints()` should call `estimate_cost(tier.cost_per_1k_tokens, token_estimate, max_output_tokens)` from Phase D instead of using the raw `cost_per_1k_tokens` as the estimate. Add a 10% safety margin to the estimation.
- **Confidence**: 93%

---

### Finding 4.1: sender_id Rotation for Rate Limit Evasion (High, 97%)

**Phases**: E

- **Phase E status**: NOT ADDRESSED. Phase E implements only per-sender_id rate limiting. The "Non-Goals" section (Section 1) explicitly states: "Global aggregate rate limiting (that is a CostTracker concern)" and "Per-IP rate limiting (only per-sender_id)." There is no global rate limit, no per-channel rate limit, and no IP-based rate limiting for zero_trust users.

**[GAP-S09]** HIGH. An attacker with 1000 unique sender_ids (via multiple Discord accounts on a channel with `allowFrom: []`) gets 1000 * $0.10/day = $100/day of free-tier usage with no aggregate throttle.
- **Affected Plans**: E (RateLimiter)
- **Recommended Fix**: (1) Add a global rate limit parameter (e.g., `global_rate_limit_rpm` in `RateLimitConfig`) checked before per-user limits. (2) Add a per-channel rate limit parameter. (3) For zero_trust users, consider the `sender_id` count within a window -- if more than N new sender_ids appear in M seconds, start rejecting new unknown senders.
- **Confidence**: 97%

---

### Finding 4.2: Rate Limiter Memory Exhaustion (Medium, 96%)

**Phases**: E

- **Phase E status**: ADDRESSED. Phase E implements LRU eviction with `max_entries` (default 10,000). The `maybe_evict()` method (Section 2) triggers when entries exceed the cap. Memory analysis (Section 3) shows ~10 MB worst case at 10K entries, which is manageable.

**[GAP-S10]** LOW. The LRU eviction is implemented but the `maybe_evict()` scans all entries O(n) to find the minimum -- this could be slow under attack with many entries. Consider using a more efficient eviction structure, but this is a performance concern, not a security gap per se.
- **Affected Plans**: E (RateLimiter)
- **Recommended Fix**: For V1, the current O(n) eviction is acceptable. Note for V2: replace with a proper LRU cache (e.g., `lru` crate) for O(1) eviction.
- **Confidence**: 90%

---

### Finding 4.3: Clock Manipulation (Low, 85%)

**Phases**: D, E

- **Phase E status**: ADDRESSED. Phase E uses `std::time::Instant` (monotonic clock) for rate limiting windows (Section 1, Requirements table). No wall clock dependency.
- **Phase D status**: PARTIALLY ADDRESSED. Phase D uses `SystemTime::now()` for budget reset timestamps (Section 2.4), which is wall clock. The `should_reset_daily()` method checks that `day_now > day_last` (forward-only), which prevents backward clock manipulation from triggering spurious resets. However, forward clock manipulation could trigger early resets.

**[GAP-S11]** LOW. Forward clock manipulation could reset daily budgets early. This is a low-probability attack vector for typical deployments.
- **Affected Plans**: D (CostTracker)
- **Recommended Fix**: Accept for V1. Document that budget resets use wall clock time and are vulnerable to forward clock manipulation on shared servers.
- **Confidence**: 85%

---

### Finding 5.1: tool_access vs CommandPolicy/UrlPolicy Layering (High, 99%)

**Phases**: G

- **Phase G status**: ADDRESSED. Phase G's architecture (Section 3) explicitly documents the layered security model: Layer 1 (ToolPermissionChecker) -> Layer 2 (CommandPolicy) -> Layer 3 (UrlPolicy) -> Layer 4 (Workspace sandbox). The text states "Each layer is independent. A request must pass all applicable layers. The new layer does not replace or modify any existing layer." Phase G's test plan includes `admin_can_call_all_tools()` which verifies admin with `["*"]` passes the permission layer, but does NOT include a test verifying that CommandPolicy still applies after the permission layer passes.

**[GAP-S12]** MEDIUM. While the architecture documents defense-in-depth, there is no explicit test case in Phase G or Phase I that verifies: "admin user with tool_access: ['*'] has exec_shell calls still validated against CommandPolicy." The security review specifically recommended this test (Section 9, Phase G bullet 7).
- **Affected Plans**: G (tool permission enforcement), I (tests)
- **Recommended Fix**: Add an integration test that: (1) Creates an admin user with `tool_access: ["*"]`. (2) Configures CommandPolicy with an allowlist that excludes a command. (3) Invokes `exec_shell` with the excluded command. (4) Verifies the request is denied by CommandPolicy, not by the permission layer.
- **Confidence**: 99%

---

### Finding 5.2: MCP Tool Namespace Collision (Medium, 94%)

**Phases**: G

- **Phase G status**: PARTIALLY ADDRESSED. Phase G handles MCP tools with namespaced names (`server__tool`) and uses exact string matching. A user with `tool_access: ["myserver__search"]` can only invoke that specific tool (Section 4, Edge Case 4). However, Phase G does NOT implement a default `tool_denylist` for MCP-namespaced versions of sensitive tools (`*__exec_shell`, `*__spawn`, `*__exec`). The wildcard `["*"]` grants access to ALL MCP tools, including any that claim to execute commands.

**[GAP-S13]** MEDIUM. MCP tools are not subject to `CommandPolicy` because `CommandPolicy` only applies to the built-in `exec_shell` tool's implementation. An MCP server registering a tool called `my_server__run_command` that executes arbitrary commands would bypass `CommandPolicy` entirely, and any admin user with `tool_access: ["*"]` would have access.
- **Affected Plans**: G (tool permission enforcement)
- **Recommended Fix**: (1) Add a default `tool_denylist` for all permission levels that includes patterns for MCP-namespaced sensitive operations (exact matches for known dangerous tool suffixes). (2) Consider adding `CommandPolicy`-like validation for MCP tools that declare they execute commands. (3) Document that MCP tools from untrusted servers should be treated with suspicion.
- **Confidence**: 94%

---

### Finding 5.3: tool_access Wildcard Too Broad (Medium, 92%)

**Phases**: G

- **Phase G status**: NOT ADDRESSED. Phase G's `ToolPermissionChecker` treats `["*"]` as "all tools including MCP tools." There is no `["builtin:*"]` vs `["mcp:*"]` distinction. The security review recommended namespace-aware wildcards.

**[GAP-S14]** LOW. This is a usability/documentation issue more than a security gap, because admins who set `["*"]` expect all tools. However, the implicit inclusion of third-party MCP tools is surprising.
- **Affected Plans**: G (tool permission enforcement)
- **Recommended Fix**: For V1, document clearly that `["*"]` includes MCP tools. For V2, consider adding `["builtin:*"]` and `["mcp:*"]` patterns. Low priority for initial implementation.
- **Confidence**: 88%

---

### Finding 6.1: Workspace Config Overrides Security Defaults (Critical, 99%)

Same as Finding 2.1. See [GAP-S01].

---

### Finding 6.2: Deep Merge Produces Unexpected Combinations (Medium, 90%)

**Phases**: B, H

- **Phase B status**: Phase B's merge semantics (Section 1.5) specify that Vec fields use REPLACE semantics when the override provides a non-empty vec. This matches the security review's recommendation.
- **Phase H status**: Phase H's `deep_merge_permissions()` (Section 2.2) does key-level merging on `users` and `channels` maps, and field-level merging on named levels. However, ceiling enforcement is still missing (see GAP-S01).

**[GAP-S15]** MEDIUM. The Vec replace semantics in Phase B are correct, but without ceiling enforcement (GAP-S01), a workspace can replace `zero_trust.tool_access: []` with `zero_trust.tool_access: ["exec_shell"]` via a complete replacement. The replace semantics actually make this easier to exploit -- a single workspace override replaces the entire safe default.
- **Affected Plans**: B (permission resolution), H (config validation)
- **Recommended Fix**: Ceiling enforcement (GAP-S01) must apply AFTER the merge, validating that the merged result for any permission level does not exceed the global config's grants for that level.
- **Confidence**: 92%

---

### Finding 6.3: custom_permissions Unvalidated (Medium, 88%)

**Phases**: B, H

- **Phase B status**: NOT ADDRESSED. Phase B's merge logic (Section 2.4, line 533-537) performs shallow key-level merge on `custom_permissions` without any validation or denylist.
- **Phase H status**: NOT ADDRESSED. Phase H's validation (Section 2.1) does not validate `custom_permissions` at all.

**[GAP-S16]** MEDIUM. The `custom_permissions` field accepts arbitrary JSON values from workspace configs. If any plugin or future code uses `custom_permissions["exec_enabled"]` as a security gate (as shown in the design doc Section 8.3), a workspace config can set this for zero_trust users.
- **Affected Plans**: B (permission resolution), H (config validation), G (tool permission enforcement)
- **Recommended Fix**: (1) Document that `custom_permissions` is NOT a security boundary. (2) In Phase G, ToolMetadata's `required_custom_permissions` should only be honored for built-in tools when the `custom_permissions` value originates from the global config (tracked via the ceiling enforcement in GAP-S01). (3) Consider a `custom_permissions_denylist` in global config.
- **Confidence**: 88%

---

### Finding 7.1: Complexity Score Manipulation for Escalation (Medium, 91%)

**Phases**: C

- **Phase C status**: PARTIALLY ADDRESSED. The escalation logic (Section 2.9) correctly enforces `max_escalation_tiers` (default 1) as a hard limit and requires both `permissions.escalation_allowed` and `escalation_config.enabled`. However, there is no escalation rate limiting (max N escalations per user per hour) and no `max_escalation_budget_multiplier` to cap the cost increase from escalation.

**[GAP-S17]** MEDIUM. A Level 1 user can craft every message to trigger escalation, effectively using premium-tier models for all requests while paying only user-level budget rates.
- **Affected Plans**: C (TieredRouter)
- **Recommended Fix**: (1) Add an escalation rate limit: track escalation count per user per hour using a counter in the cost tracker or rate limiter. (2) Add `max_escalation_budget_multiplier` to `EscalationConfig` that caps the cost increase (e.g., 3x base tier cost). (3) Log escalation events at `info` level for audit.
- **Confidence**: 91%

---

### Finding 7.2: model_override Bypasses All Routing Logic (Low, 99%)

**Phases**: C

- **Phase C status**: NOT ADDRESSED. Phase C's routing algorithm (Section 2.14) does not implement `model_override` handling. The `ChatRequest.model` field is not checked against `permissions.model_override` flag. This means model override functionality is not yet present, so the attack surface is not yet exposed, but it will need to be implemented with security controls.

**[GAP-S18]** LOW. Not yet implemented, but when `model_override` is added, it must still respect global budget limits. The security review recommended: even with `model_override: true`, enforce `global_daily_limit_usd`.
- **Affected Plans**: C (TieredRouter)
- **Recommended Fix**: When implementing model_override, (1) enforce global budget limits regardless, (2) log all model_override events at `warn` level, (3) validate the overridden model exists in the tier configuration.
- **Confidence**: 99%

---

### Finding 7.3: No Audit Trail for Permission Decisions (Medium, 99%)

**Phases**: C, G

- **Phase C status**: PARTIALLY ADDRESSED. Phase C's `RoutingDecision` includes a `reason` field (Section 2.14, line 977) that logs complexity, tier, level, and user. However, this is a decision reason, not a structured audit log. There is no dedicated audit logging for denied requests, escalation attempts, or budget limit hits.
- **Phase G status**: PARTIALLY ADDRESSED. Phase G logs `ToolError::PermissionDenied` with the tool name and reason, but this is an error response, not a structured audit event.

**[GAP-S19]** MEDIUM. There is no structured audit logging for permission decisions. The security review recommended structured JSON logs for every permission decision including `resolved_level`, `requested_tier`, `allowed_tier`, `escalation_applied`, `budget_remaining_usd`, `rate_limit_remaining`, `tool_requested`, and `tool_allowed`.
- **Affected Plans**: C (TieredRouter), G (tool permission enforcement)
- **Recommended Fix**: Add `tracing::info!` structured log events at key decision points: (1) Permission resolution result (level, sender_id, channel). (2) Tier selection result (requested vs allowed, escalation applied). (3) Budget constraint applied (remaining budget, downgrade tier). (4) Rate limit hit. (5) Tool permission denied. Use structured key-value pairs compatible with JSON log aggregation.
- **Confidence**: 99%

---

### Finding 7.4: Fallback Model Lacks Permission Check (Medium, 93%)

**Phases**: C

- **Phase C status**: NOT ADDRESSED. The `fallback_chain()` method (Section 2.12) and `no_tiers_available_decision()` (Section 2.14, line 1005) return the `fallback_model` WITHOUT checking whether it belongs to a tier the user is allowed to access. If `fallback_model` is `"anthropic/claude-opus-4-5"` (elite tier), a zero_trust user whose free-tier models are all unavailable would fall back to the most expensive model.

**[GAP-S20]** HIGH. The fallback model completely bypasses permission checks. A zero_trust user can receive elite-tier model access if their allowed models are unavailable (provider outage, API key missing).
- **Affected Plans**: C (TieredRouter)
- **Recommended Fix**: Before returning the fallback model, verify it belongs to a tier at or below the user's `max_tier`. If the fallback model exceeds the user's permission boundary, return an error decision instead. The `rate_limited_decision()` method (Section 2.13) has the same issue -- it also uses the fallback model without permission checking.
- **Confidence**: 97%

---

### Finding 7.5: CostTracker Reset Race Condition (Medium, 94%)

**Phases**: D

- **Phase D status**: PARTIALLY ADDRESSED. Phase D uses CAS (compare-and-swap) on `daily_reset_at` (Section 2.7, line 596-598) to prevent double-reset. However, the scenario described in the security review (Thread A reads pre-reset spend, reset happens, Thread A approves against pre-reset value) is not handled. The CAS prevents concurrent resets but does not prevent a budget check that straddles the reset boundary.

**[GAP-S21]** MEDIUM. A budget check that starts before a reset and completes after will allow a request against the old (higher) budget, but the cost is recorded against the new (fresh) budget. The net effect is a brief window of "extra" budget at reset time.
- **Affected Plans**: D (CostTracker)
- **Recommended Fix**: For V1, accept this as a known minor issue. The extra budget window is brief (milliseconds) and the maximum extra spend is one request's cost. For V2, implement an epoch counter: each budget check records the epoch at start; if the epoch changes during the check (reset occurred), re-run the check.
- **Confidence**: 90%

---

## 2. Security Invariant Verification

### INV-001: zero_trust MUST NOT access exec_shell or spawn

**Enforceable**: YES, with caveats.
- Phase G enforces `tool_access: []` for zero_trust by default, which blocks all tools.
- HOWEVER, GAP-S01 (workspace ceiling) means a workspace config CAN override `zero_trust.tool_access` to include `exec_shell`. Without ceiling enforcement, this invariant relies entirely on default config values, not code enforcement.
- **Verdict**: NOT GUARANTEED. Requires GAP-S01 fix.

### INV-002: CommandPolicy dangerous patterns ALWAYS enforced for ALL levels

**Enforceable**: YES.
- Phase G's architecture explicitly documents the layered model. CommandPolicy runs inside the tool implementation, after the permission layer.
- CAVEAT: MCP tools bypass CommandPolicy (GAP-S13). A malicious MCP tool claiming to execute commands is not gated by CommandPolicy.
- **Verdict**: GUARANTEED for built-in tools. NOT GUARANTEED for MCP tools.

### INV-003: UrlPolicy SSRF protection ALWAYS enforced for ALL levels

**Enforceable**: YES.
- Same architecture as INV-002. UrlPolicy applies inside `web_fetch` tool implementation.
- Same MCP caveat: an MCP tool making HTTP requests is not subject to UrlPolicy.
- **Verdict**: GUARANTEED for built-in tools. NOT GUARANTEED for MCP tools.

### INV-004: Rate limit MUST NOT be 0 for zero_trust

**Enforceable**: NO.
- The built-in default for zero_trust is `rate_limit: 10`. But without ceiling enforcement (GAP-S01), a workspace config can set `zero_trust.rate_limit: 0` (unlimited).
- Phase H validates that `rate_limit` values are non-negative but does not enforce a minimum for zero_trust.
- **Verdict**: NOT GUARANTEED. Requires GAP-S01 fix plus specific validation: `zero_trust.rate_limit` must be >= 1.

### INV-005: auth_context MUST only be set by trusted channel plugins

**Enforceable**: NO.
- GAP-S03 documents that `auth_context` is a public serde-deserializable field on `ChatRequest`. Any JSON caller can inject it.
- No `trusted_source` flag exists.
- **Verdict**: NOT GUARANTEED. Requires GAP-S03 fix.

### INV-006: Workspace configs MUST NOT escalate beyond global ceiling

**Enforceable**: NO.
- GAP-S01 documents the complete absence of ceiling enforcement.
- **Verdict**: NOT GUARANTEED. Requires GAP-S01 fix.

### INV-007: zero_trust MUST NOT have escalation_allowed: true unless global permits

**Enforceable**: NO.
- Subset of INV-006. No ceiling enforcement.
- **Verdict**: NOT GUARANTEED. Requires GAP-S01 fix.

### INV-008: Fallback model MUST be subject to permission checks

**Enforceable**: NO.
- GAP-S20 documents that the fallback model bypasses all permission checks.
- **Verdict**: NOT GUARANTEED. Requires GAP-S20 fix.

### INV-009: Budget checks MUST use atomic reserve-then-adjust

**Enforceable**: NO.
- GAP-S06 documents that check and reserve are separate non-atomic calls.
- **Verdict**: NOT GUARANTEED. Requires GAP-S06 fix.

### INV-010: All permission decisions MUST be logged

**Enforceable**: NO.
- GAP-S19 documents the absence of structured audit logging.
- **Verdict**: NOT GUARANTEED. Requires GAP-S19 fix.

---

## 3. Privilege Escalation Path Analysis

### Path 1: zero_trust -> higher permissions via workspace config

**Attack**: Write a `.clawft/config.json` in a project directory that overrides `zero_trust` permissions.

```
zero_trust (Level 0)
  -> workspace config sets zero_trust.escalation_allowed: true
  -> workspace config sets zero_trust.escalation_threshold: 0.3
  -> user crafts complex message (complexity > 0.3)
  -> escalation promotes from free -> standard tier
  -> user now accesses paid models
```

**Guarded**: NO. GAP-S01 applies. No ceiling enforcement exists.

### Path 2: zero_trust -> admin via per-user workspace override

**Attack**: Write a workspace config with `users.<attacker_id>.level: 2`.

```
zero_trust (Level 0)
  -> workspace config adds attacker's sender_id with level: 2
  -> attacker sends message from their known sender_id
  -> PermissionResolver grants admin
```

**Guarded**: NO. GAP-S05 applies. No `max_grantable_level` ceiling.

### Path 3: sender_id spoofing to steal identity

**Attack**: Submit a `ChatRequest` JSON to the gateway with a crafted `auth_context`.

```
anonymous (no auth)
  -> submit JSON to gateway HTTP API
  -> include auth_context: { sender_id: "admin_user", level: 2 }
  -> TieredRouter reads auth_context from request
  -> grants admin permissions
```

**Guarded**: NO. GAP-S03 applies. `auth_context` is deserializable from JSON.

### Path 4: auth_context mutation after creation

**Attack**: In Rust code, modify `auth_context` after channel plugin creates it.

**Guarded**: YES, partially. `AuthContext` is `Clone` but immutable in practice. It flows through the pipeline as part of `ChatRequest` which is passed by reference (`&ChatRequest`). However, it could be cloned and modified by a malicious plugin or middleware. The `trusted_source` flag (GAP-S03) would help detect this.

### Path 5: Escalation beyond max_tier

**Attack**: Exploit escalation to access tiers beyond `max_tier`.

**Guarded**: YES. Phase C's `select_tier()` limits escalation candidates to `t.ordinal <= max_ordinal + max_escalation` (Section 2.9, line 580-581). With `max_escalation_tiers: 1`, a user with `max_tier: "standard"` (ordinal 1) can only reach `premium` (ordinal 2), never `elite` (ordinal 3).

### Path 6: Fallback model bypass

**Attack**: Cause all models in allowed tiers to be "unavailable" (provider outage), forcing fallback to an expensive model.

```
zero_trust (max_tier: free)
  -> free tier models unavailable (groq outage)
  -> fallback_chain returns fallback_model
  -> fallback_model = "anthropic/claude-opus-4-5" (elite)
  -> zero_trust user gets elite model
```

**Guarded**: NO. GAP-S20 applies. Fallback model has no permission check.

---

## 4. Consensus Item Security Assessment

### CONS-001: Type Location (config.rs vs routing.rs) -- Confidence 90%

**Security Impact**: LOW. This is a code organization decision with no security implications. Either location is acceptable. The recommendation to keep in `config.rs` (450 lines, within the 500-line budget) is fine.

**Assessment**: No security concern. Proceed with either approach.

### CONS-002: DashMap vs RwLock<HashMap> -- Confidence 80%

**Security Impact**: MEDIUM. The choice affects the TOCTOU race condition (Finding 3.1 / GAP-S06).

- **DashMap**: Provides `entry()` API that enables atomic check-and-update within a single shard lock. This is critical for the `reserve_budget()` pattern recommended in GAP-S06.
- **RwLock<HashMap>**: Requires holding the write lock for the entire check-and-reserve operation, which serializes ALL budget operations globally. This is simpler but creates a contention bottleneck.

**Assessment**: DashMap is the more secure choice because it enables per-user atomic budget reservation without global serialization. RwLock would also work but creates a performance cliff under load that could itself become a DoS vector (all requests serialize on budget checks).

**Recommendation**: Resolve as DashMap. The `entry()` API is essential for GAP-S06 remediation. If the team proceeds with Phase D as planned (using DashMap), the `check_budget()` and `record_estimated()` methods should be consolidated into a single `reserve_budget()` that uses `entry()`.

### CONS-003: Permission Escalation Security Model -- Confidence 85%

**Security Impact**: HIGH. This directly relates to Findings 2.1 and 7.1.

**Assessment**: The escalation model IS secure under the following conditions:
1. `zero_trust.escalation_allowed` is `false` (hardcoded, not just defaulted) -- requires GAP-S01 fix
2. `max_escalation_tiers` is enforced as a hard limit -- Phase C addresses this
3. Budget caps limit the financial damage of escalation abuse -- Phase D addresses this
4. Escalation rate limiting prevents sustained abuse -- GAP-S17 is open

Without conditions 1 and 4, the model has exploitable weaknesses. The classifier gaming concern (keyword stuffing to trigger high complexity) is real but bounded by budget caps.

**Recommendation**: Resolve as "Accept with mitigations." The mitigations are: (a) Ceiling enforcement (GAP-S01) to prevent workspace override of escalation settings for zero_trust, (b) Escalation rate limiting (GAP-S17), and (c) Logging of escalation events (GAP-S19).

### CONS-004: AuthContext Location -- Confidence 85%

**Security Impact**: LOW. Code organization decision. The recommendation for `auth.rs` is slightly better because it separates runtime auth types from config parsing, reducing the attack surface of the config parser.

**Recommendation**: Resolve as `auth.rs`. The separation improves code review ergonomics for security-sensitive types.

### CONS-005: RoutingDecision Extension Strategy -- Confidence 92%

**Security Impact**: LOW. Flat `Option<T>` fields vs. wrapped `RoutingMetadata` has no security difference. The flat approach is simpler to audit.

**Recommendation**: Resolve as flat Option fields. Matches design doc and simplifies security review.

### CONS-006: Config Validation Boundary -- Confidence 88%

**Security Impact**: HIGH. The location of validation determines when security-sensitive values are checked.

**Assessment**: The proposed split ("basic in types, business in core") creates a risk: if validation is split across crates, a code path might construct a `RoutingConfig` in `clawft-core` without calling the `clawft-types` validation. All security-critical validation MUST run before ANY use of the config, which means it should run at config load time in a single pass.

**Recommendation**: Resolve as "all validation in a single `validate_routing_config()` function called at config load time." The function can live in `clawft-types` (it only depends on the config types) or in a shared validation module. The critical requirement is that it runs once, checks everything, and its result gates startup.

---

## 5. Missing Security Controls

### [GAP-S22] MEDIUM: No input validation on sender_id format

**Description**: The `sender_id` string is used as keys in DashMap (rate limiter, cost tracker), HashMap (permission overrides), and potentially logged. There is no validation of its format, length, or content. An attacker could submit extremely long sender_id values to cause memory exhaustion in the rate limiter and cost tracker DashMaps.

**Affected Plans**: E (RateLimiter), D (CostTracker), F (AuthContext)

**Recommended Fix**: Validate sender_id at the AuthContext construction point: max length (e.g., 256 bytes), ASCII-only or Unicode-normalized, no control characters. Reject or truncate invalid sender_ids.

**Confidence**: 93%

---

### [GAP-S23] MEDIUM: No bounds checking on config numeric values (DoS via resource exhaustion)

**Description**: Phase H validates ranges for some numeric values (complexity_range in [0.0, 1.0], reset_hour in 0-23) but does not validate upper bounds for others:
- `max_context_tokens`: Could be set to `usize::MAX`, causing allocation failures
- `max_output_tokens`: Could be set to `usize::MAX`
- `rate_limit`: u32 allows up to 4 billion req/min (effectively unlimited but wastes memory tracking)
- `save_interval_ops`: Could be 0, causing save on every operation (disk I/O flood)

**Affected Plans**: H (config validation)

**Recommended Fix**: Add upper bound validation for all numeric config fields. Suggested maximums: `max_context_tokens <= 2_000_000`, `max_output_tokens <= 100_000`, `rate_limit <= 10_000` (or 0 for unlimited), `save_interval_ops >= 1`.

**Confidence**: 95%

---

### [GAP-S24] MEDIUM: No fail-closed behavior on errors

**Description**: Several error paths in the plans fall back to permissive behavior:
- Phase C: When `auth_context` is `None`, default is `AuthContext::default()` which in Phase F defaults to zero_trust but in Phase B defaults to CLI admin (inconsistency).
- Phase C: When `tier_index.get()` fails for an unknown `max_tier`, defaults to ordinal 0 (cheapest only). This is fail-safe but could also be interpreted as fail-open if the config error means the user should have been blocked entirely.
- Phase D: When `persistence_path` is not writable, continues in-memory only without alerting the operator.

**Affected Plans**: C (TieredRouter), B (permissions), D (CostTracker)

**Recommended Fix**: (1) Resolve the `AuthContext::default()` inconsistency -- Phase B defaults to admin, Phase F defaults to zero_trust. The correct default must be zero_trust (fail-closed). (2) When config errors prevent proper operation, log at `error` level, not just `warn`. (3) If the persistence file cannot be written after multiple retries, consider throttling all users to zero_trust budget levels (fail-safe).

**[GAP-S24a]** HIGH -- Specific sub-issue: `AuthContext::default()` inconsistency. Phase B (line 291-296) defaults to `sender_id: "local"`, `channel: "cli"`, `permissions: admin_defaults()`. Phase F (line 87-96) defaults to `sender_id: ""`, `channel: ""`, `permissions: zero_trust()`. Phase C (Section 2.14, line 902) uses `.unwrap_or_default()` when `auth_context` is `None`. If Phase B's default is used, a missing auth_context silently grants admin. If Phase F's default is used, it correctly falls to zero_trust. These MUST be reconciled, and the correct answer is zero_trust.

**Confidence**: 98%

---

### [GAP-S25] LOW: Rate limiter has no memory bounds on individual entries

**Description**: Phase E's `SlidingWindowEntry` uses a `VecDeque<Instant>` that grows up to `limit` entries. For admin users with `rate_limit: 0`, the check short-circuits before creating an entry. But for users with very high limits (e.g., `rate_limit: 10000`), the VecDeque could hold 10,000 `Instant` values (~160 KB) per sender. With 10,000 senders, this is ~1.6 GB.

**Affected Plans**: E (RateLimiter)

**Recommended Fix**: Cap the VecDeque size to `min(limit, max_window_entries)` where `max_window_entries` is a hard-coded constant (e.g., 1000). If `rate_limit > max_window_entries`, use a token bucket algorithm instead of sliding window for that sender.

**Confidence**: 88%

---

### [GAP-S26] MEDIUM: No test for workspace ceiling enforcement (because it does not exist)

**Description**: The security review's Phase I recommendations (Section 9) call for "dedicated security test suite with adversarial scenarios" including "test config override ceiling enforcement." Since ceiling enforcement is not implemented (GAP-S01), these tests cannot exist. This gap tracks the testing requirement.

**Affected Plans**: I (tests)

**Recommended Fix**: When GAP-S01 is implemented, add the following adversarial tests:
1. Workspace sets `zero_trust.escalation_allowed: true` -> verify it is blocked
2. Workspace sets `zero_trust.tool_access: ["exec_shell"]` -> verify it is blocked
3. Workspace sets `users.attacker.level: 2` -> verify it is capped to `max_grantable_level`
4. Workspace sets `zero_trust.rate_limit: 0` -> verify minimum is enforced
5. Workspace sets `zero_trust.cost_budget_daily_usd: 1000` -> verify it is capped to global

**Confidence**: 99%

---

## 6. Gap Summary

| ID | Severity | Finding | Plans Affected | Status |
|----|----------|---------|----------------|--------|
| GAP-S01 | CRITICAL | Workspace ceiling enforcement missing | B, H | UNADDRESSED |
| GAP-S02 | LOW | No validation warning for glob-like tool_access patterns | H | UNADDRESSED |
| GAP-S03 | HIGH | auth_context injectable via JSON deserialization | F, C | UNADDRESSED |
| GAP-S04 | HIGH | CLI admin default with no env var override or warnings | F, H | UNADDRESSED |
| GAP-S05 | MEDIUM | Per-user workspace override can grant admin | B, H | UNADDRESSED |
| GAP-S06 | HIGH | Budget check and reserve not atomic (TOCTOU) | D | UNADDRESSED |
| GAP-S07 | MEDIUM | Persistence file has no integrity protection | D | PARTIALLY |
| GAP-S08 | MEDIUM | Phase C uses underestimate for budget checks | C, D | UNADDRESSED |
| GAP-S09 | HIGH | No global or per-channel rate limit | E | UNADDRESSED |
| GAP-S10 | LOW | LRU eviction is O(n) scan | E | DEFERRED |
| GAP-S11 | LOW | Clock manipulation can trigger early budget reset | D | ACCEPTED |
| GAP-S12 | MEDIUM | No test verifying CommandPolicy applies after permission check | G, I | UNADDRESSED |
| GAP-S13 | MEDIUM | MCP tools bypass CommandPolicy | G | UNADDRESSED |
| GAP-S14 | LOW | Wildcard ["*"] includes MCP tools implicitly | G | DEFERRED |
| GAP-S15 | MEDIUM | Vec replace semantics enable easier zero_trust override | B, H | UNADDRESSED |
| GAP-S16 | MEDIUM | custom_permissions unvalidated from workspace | B, H, G | UNADDRESSED |
| GAP-S17 | MEDIUM | No escalation rate limiting | C | UNADDRESSED |
| GAP-S18 | LOW | model_override not yet implemented (future risk) | C | DEFERRED |
| GAP-S19 | MEDIUM | No structured audit logging for permission decisions | C, G | UNADDRESSED |
| GAP-S20 | HIGH | Fallback model bypasses permission checks | C | UNADDRESSED |
| GAP-S21 | MEDIUM | Budget reset race condition | D | ACCEPTED (V1) |
| GAP-S22 | MEDIUM | No sender_id format validation | E, D, F | UNADDRESSED |
| GAP-S23 | MEDIUM | No upper bounds on config numeric values | H | UNADDRESSED |
| GAP-S24 | HIGH | AuthContext::default() inconsistency (admin vs zero_trust) | B, F, C | UNADDRESSED |
| GAP-S25 | LOW | Rate limiter VecDeque unbounded per entry | E | DEFERRED |
| GAP-S26 | MEDIUM | No adversarial security tests for ceiling enforcement | I | BLOCKED on S01 |

**Severity Distribution**:
- CRITICAL: 1 (GAP-S01)
- HIGH: 5 (GAP-S03, GAP-S04, GAP-S06, GAP-S09, GAP-S20, GAP-S24)
- MEDIUM: 13 (GAP-S05, GAP-S07, GAP-S08, GAP-S12, GAP-S13, GAP-S15, GAP-S16, GAP-S17, GAP-S19, GAP-S22, GAP-S23, GAP-S25, GAP-S26)
- LOW: 5 (GAP-S02, GAP-S10, GAP-S11, GAP-S14, GAP-S18)

---

## 7. Security Posture Summary

The Tiered Router SPARC plans demonstrate a solid architectural foundation with clear separation of concerns, defense-in-depth layering for tool access, and well-designed per-user budget and rate limiting systems. The security review was thorough and identified 22 findings with appropriate severity ratings.

However, the implementation plans have a significant security gap: **the single most dangerous attack vector identified in the security review (workspace config escalation, Finding 6.1 / GAP-S01) has NO implementation in any SPARC plan**. This is the highest-priority fix and affects the enforceability of 5 of the 10 security invariants (INV-001, INV-004, INV-006, INV-007, and partially INV-002/003 via MCP).

**Critical action items before implementation begins**:

1. **GAP-S01**: Add workspace ceiling enforcement to Phase B and Phase H plans. This is non-negotiable for security. Without it, any contributor who can modify a `.clawft/config.json` file in a project directory can escalate anonymous users to admin.

2. **GAP-S03**: Add `trusted_source` to AuthContext and prevent JSON injection of auth_context on ChatRequest. Without this, the gateway HTTP API is an open admin backdoor.

3. **GAP-S24**: Resolve the `AuthContext::default()` inconsistency between Phase B (admin) and Phase F (zero_trust). The correct default MUST be zero_trust.

4. **GAP-S20**: Add permission checks to the fallback model path. Currently a zero_trust user can receive elite-tier access via the fallback chain.

5. **GAP-S06**: Consolidate budget check and reservation into an atomic operation.

6. **GAP-S09**: Add global and per-channel rate limits to prevent sender_id rotation attacks.

**Overall security readiness**: The plans are approximately 60% aligned with the security review findings. The architecture is sound, but critical enforcement mechanisms (ceiling enforcement, auth context integrity, atomic budget operations) are missing from the implementation specifications. These gaps should be addressed in plan revisions before coding begins.

The 6 open consensus items should be resolved with security implications in mind, particularly CONS-002 (DashMap for atomic budget operations), CONS-003 (escalation security model acceptance with mandatory mitigations), and CONS-006 (validation boundary -- must be a single pass at config load time).
