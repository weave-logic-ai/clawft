# Workstream B: Architecture Cleanup

**Status**: Complete (9/9 items done)
**Priority**: P1-P2
**Timeline**: Weeks 2-4 (overlaps with Workstream A tail)
**Blocks**: Feature elements 04-10 (plugin system, pipeline reliability, channel enhancements)
**Crates touched**: clawft-types, clawft-llm, clawft-core, clawft-cli, clawft-services, clawft-tools

---

## 1. Overview

Workstream B resolves structural issues that increase maintenance burden and block feature work. The 9 items range from type unification (B1, B2, B7) and code deduplication (B5, B6, B8, B9) to the large-scale file split effort (B3) and storage format unification (B4).

B3 is the largest single item in the entire Element 03 scope, covering 9 files totaling 11,558 lines that must be split to stay under the 500-line target.

### Concurrency Plan

Items can be worked in parallel with the following constraints:

```
B1 (Usage unification)  ─┬─> B7 (ProviderConfig naming) -- both touch clawft-llm types
                          │
A4 (SecretRef) ──────────> B3 (file splits) -- config.rs split needs secret.rs module
A6 (SSRF fix) ──────────> B6 (policy extraction) -- canonical UrlPolicy needs complete IP check
B1 (Usage unification) ──> B7 (ProviderConfig naming) -- coordinate to avoid churn

Parallel-safe groups:
  Group 1: B1, B2, B4, B5, B8, B9 (no interdependencies)
  Group 2: B3 (depends on A4)
  Group 3: B6 (depends on A6)
  Group 4: B7 (depends on B1)
```

---

## 2. Item Specifications

---

### B1: Unify `Usage` type across crates (P1)

**Problem**: Token usage is represented with incompatible types in two crates.

| Location | Type | Fields | Signedness |
|----------|------|--------|------------|
| `clawft-types/src/provider.rs:69` | `pub struct Usage` | `input_tokens`, `output_tokens` | `u32` |
| `clawft-llm/src/types.rs:151` | `pub struct Usage` | `prompt_tokens`, `completion_tokens`, `total_tokens` | `i32` |

Token counts are never negative, so `u32` is correct. However, the OpenAI API names (`prompt_tokens`, `completion_tokens`) are the industry standard that all 19+ providers use.

**Current code -- clawft-types/src/provider.rs:68-74**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

**Current code -- clawft-llm/src/types.rs:149-160**:
```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}
```

**Fix**:

1. **Canonical type in `clawft-types/src/provider.rs`**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Tokens consumed by the prompt / input.
    #[serde(alias = "input_tokens")]
    pub prompt_tokens: u32,

    /// Tokens generated in the response.
    #[serde(alias = "output_tokens")]
    pub completion_tokens: u32,

    /// Total tokens used (prompt + completion).
    /// Computed on deserialization; defaults to sum if absent.
    #[serde(default)]
    pub total_tokens: u32,
}
```

2. **Remove `Usage` from `clawft-llm/src/types.rs`**; import from `clawft_types::provider::Usage`.

3. **Update all references**: `input_tokens` -> `prompt_tokens`, `output_tokens` -> `completion_tokens` across both crates. The `#[serde(alias)]` ensures existing serialized data still deserializes correctly.

4. **Update `StreamChunk::Done` in `clawft-llm/src/types.rs:189-194`** to use the shared `Usage` type.

**Files to modify**:
- `crates/clawft-types/src/provider.rs` -- Update `Usage` struct
- `crates/clawft-llm/src/types.rs` -- Remove local `Usage`, import from clawft-types
- `crates/clawft-llm/Cargo.toml` -- Add/verify `clawft-types` dependency
- `crates/clawft-core/src/pipeline/transport.rs` -- Update any field references
- `crates/clawft-core/src/pipeline/llm_adapter.rs` -- Update conversion functions

**Acceptance criteria**:
- [ ] Single `Usage` type in `clawft-types`
- [ ] All fields are `u32` (never negative)
- [ ] Field names use OpenAI convention (`prompt_tokens`, `completion_tokens`)
- [ ] `#[serde(alias)]` supports both naming conventions
- [ ] `total_tokens` field included with `#[serde(default)]`
- [ ] `clawft-llm` imports from `clawft-types` (no local `Usage`)
- [ ] All tests pass, `cargo build` succeeds

---

### B2: Unify duplicate `LlmMessage` types (P1)

**Problem**: Two separate `LlmMessage` structs exist in `clawft-core` with identical purpose.

| Location | Fields | Notes |
|----------|--------|-------|
| `clawft-core/src/agent/context.rs:44` | `role`, `content`, `tool_call_id` | Has TODO comment acknowledging duplication (lines 42-43) |
| `clawft-core/src/pipeline/traits.rs:57` | `role`, `content`, `tool_call_id`, `tool_calls` | Has Serde derives, `tool_calls` for OpenAI round-trip |

**Current code -- context.rs:39-52**:
```rust
/// An LLM message with role and content.
///
/// Compatible with the `LlmMessage` type defined in
/// `crate::pipeline::traits`. Once that module is available, this
/// definition should be replaced with a re-export.
#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
}
```

**Current code -- traits.rs:56-73**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}
```

**Fix**:

1. **Keep** the `pipeline::traits::LlmMessage` as the canonical type (it has Serde derives and `tool_calls`).

2. **In `context.rs`**, replace the local struct with a re-export:
```rust
pub use crate::pipeline::traits::LlmMessage;
```

3. **Update `ContextBuilder::build_messages`** and `build_messages_for_agent` to construct `LlmMessage` with `tool_calls: None` (already compatible).

4. **Update `loop_core.rs:204-213`** where `context::LlmMessage` is manually converted to `pipeline::traits::LlmMessage` -- this mapping becomes a no-op since they are now the same type.

**Files to modify**:
- `crates/clawft-core/src/agent/context.rs` -- Replace struct with re-export
- `crates/clawft-core/src/agent/loop_core.rs` -- Remove manual conversion at lines 204-213
- Any other files importing `agent::context::LlmMessage`

**Acceptance criteria**:
- [ ] Single `LlmMessage` definition in `pipeline/traits.rs`
- [ ] `context.rs` re-exports from `pipeline::traits`
- [ ] Manual conversion in `loop_core.rs` removed
- [ ] All context builder tests still pass
- [ ] `cargo build` succeeds

---

### B3: Split oversized files (P1) -- LARGEST ITEM

**Dependency**: A4 (SecretRef) must land first for the `config.rs` split.

Nine files totaling 11,558 lines must be split. Target: no file exceeds 500 lines after split.

#### B3-1: `clawft-core/src/agent/loop_core.rs` (1,668 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| Module header + imports | 1-48 | Imports, constants |
| `AgentLoop` struct + constructor | 49-131 | Struct definition, `new()`, `with_cancel()`, accessors |
| `run()` loop | 132-185 | Main consume loop with cancellation |
| `process_message()` | 186-261 | Session lookup, context building, pipeline invocation |
| `resolve_auth_context()` | 262-284 | Auth resolution |
| `run_tool_loop()` | 286-409 | Tool execution loop, message building |
| Tests | 414-1668 | ~1,254 lines of tests |

**Proposed split**:
```
agent/
  loop_core.rs     -> agent/mod.rs (re-exports)
  loop_core/
    mod.rs           (~200 lines) -- AgentLoop struct, new(), run(), process_message()
    tool_loop.rs     (~130 lines) -- run_tool_loop(), MAX_TOOL_RESULT_BYTES
    message_builder.rs (~50 lines) -- assistant_tool_calls construction, text extraction
    auth.rs          (~30 lines) -- resolve_auth_context()
    tests/
      mod.rs         (~1,254 lines, further split by test group if needed)
```

**Key extraction -- `run_tool_loop` (lines 286-409)**: The tool execution loop is self-contained. It takes a `ChatRequest`, calls pipeline, extracts tool calls, executes them, and appends results. This can be extracted as a method on `AgentLoop` in a separate file.

**Key extraction -- message building (lines 340-366)**: The `assistant_tool_calls` construction and text extraction from `ContentBlock` are repeated patterns. Extract as helper functions.

#### B3-2: `clawft-core/src/pipeline/tiered_router.rs` (1,645 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| `ModelTier` struct + impls | 23-53 | Runtime tier representation |
| No-op implementations | 55-83 | `NoopCostTracker`, `NoopRateLimiter` |
| Internal helper types | 85-97 | `TierSelection`, `TierBudgetResult` |
| `TieredRouter` struct + core methods | 99-582 | Constructor, tier filtering/selection, budget, model selection, fallback, decision helpers |
| `ModelRouter` trait impl | 583-712 | `route()` and `update()` implementations |
| Debug impl | 713-728 | `fmt::Debug` |
| Helper functions | 729-754 | `model_matches_pattern`, `split_provider_model` |
| Tests | 756-1645 | ~889 lines of tests |

**Proposed split**:
```
pipeline/
  tiered_router.rs  -> pipeline/tiered_router/
    mod.rs           (~180 lines) -- TieredRouter struct, new(), route() trait impl
    tier_selection.rs (~150 lines) -- select_tier(), filter_tiers_by_permissions()
    budget.rs        (~100 lines) -- apply_budget_constraints(), NoopCostTracker
    model_selection.rs (~80 lines) -- select_model(), TierSelectionStrategy logic
    fallback.rs      (~80 lines) -- fallback chain logic
    types.rs         (~60 lines) -- ModelTier, TierSelection, TierBudgetResult, NoopRateLimiter
    helpers.rs       (~30 lines) -- model_matches_pattern, split_provider_model
    tests/
      mod.rs         (~889 lines, split by test category)
```

#### B3-3: `clawft-types/src/config.rs` (1,382 lines)

**CRITICAL**: This split must accommodate the `secret.rs` module from A4 (SecretRef). If A4 has not landed, note the placeholder and plan for integration.

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| Root config + agents | 1-133 | `Config`, `AgentsConfig`, `AgentDefaults` |
| Channels | 134-772 | `ChannelsConfig`, `TelegramConfig`, `SlackConfig`, `DiscordConfig`, `WhatsAppConfig`, `FeishuConfig`, `DingTalkConfig`, `MochatConfig`, `EmailConfig`, `QQConfig` |
| Providers | 773-854 | `ProviderConfig`, `ProvidersConfig` |
| Gateway | 855-897 | `GatewayConfig` |
| Tools | 898-1070+ | `ToolsConfig`, `WebSearchConfig`, `ExecToolConfig`, `CommandPolicyConfig`, `UrlPolicyConfig`, `MCPServerConfig` |
| Tests | remaining | Config parsing tests |

**Proposed split**:
```
config/
  mod.rs             (~100 lines) -- Config root struct, re-exports
  agents.rs          (~80 lines) -- AgentsConfig, AgentDefaults, defaults fns
  channels.rs        (~400 lines) -- ChannelsConfig + all 10 channel configs
  providers.rs       (~90 lines) -- ProviderConfig, ProvidersConfig
  gateway.rs         (~50 lines) -- GatewayConfig
  tools.rs           (~180 lines) -- ToolsConfig, WebSearchConfig, ExecToolConfig, policy configs
  secret.rs          (from A4) -- SecretRef type (placeholder if A4 not landed)
  tests.rs           (remaining test lines)
```

**Note**: The `channels.rs` file will be ~400 lines due to 10 channel configs. Each is small individually (20-40 lines) but they total a lot. If desired, a further `channels/` module split is possible, but ~400 lines is within the 500-line target.

#### B3-4: `clawft-core/src/pipeline/transport.rs` (1,281 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| `LlmProvider` trait | 37-79 | The trait abstraction |
| `OpenAiCompatTransport` struct + impl | 86-310 | Transport implementation with request building and response parsing |
| `convert_response()` function | 311-417 | JSON -> `LlmResponse` conversion (complex parsing) |
| Tests | 418-1281 | ~863 lines of tests |

**Proposed split**:
```
pipeline/
  transport/
    mod.rs           (~120 lines) -- LlmProvider trait, OpenAiCompatTransport struct
    request.rs       (~100 lines) -- Request building (messages -> JSON, tool schemas)
    response.rs      (~120 lines) -- convert_response(), JSON -> LlmResponse parsing
    tests/
      mod.rs         (~863 lines, split by test category)
```

#### B3-5: `clawft-core/src/tools/registry.rs` (1,241 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| `ToolError` enum | 24-56 | Error types |
| `ToolMetadata` struct | 62-78 | Permission metadata |
| Glob matching | 84-133 | `glob_matches()` function |
| Permission check | 133-241 | `matches_any_pattern()`, `check_tool_permission()` |
| MCP metadata extraction | 242-303 | `extract_mcp_metadata()` |
| `Tool` trait | 304-336 | The tool interface |
| `ToolRegistry` struct + impl | 337-489 | Registry with registration and execution |
| Tests | 490-1241 | ~751 lines of tests |

**Proposed split**:
```
tools/
  registry/
    mod.rs           (~200 lines) -- Tool trait, ToolRegistry struct, ToolError
    permissions.rs   (~120 lines) -- glob_matches, matches_any_pattern, check_tool_permission
    metadata.rs      (~70 lines) -- ToolMetadata, extract_mcp_metadata
    tests/
      mod.rs         (~751 lines)
```

#### B3-6: `clawft-core/src/agent/skills_v2.rs` (1,158 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| SKILL.md parser | 47-318 | `parse_skill_md()`, `extract_frontmatter()`, `parse_yaml_frontmatter()`, `parse_scalar()`, `extract_string_list()` |
| SkillRegistry | 319-564 | Registry with three-level discovery, loading, lookup |
| Legacy loader | 567-597 | `load_legacy_skill()` |
| Tests | 599-1158 | ~559 lines of tests |

**Proposed split**:
```
agent/
  skills_v2/
    mod.rs           (~60 lines) -- Re-exports
    parser.rs        (~280 lines) -- parse_skill_md(), extract_frontmatter(), parse_yaml_frontmatter(), helpers
    registry.rs      (~260 lines) -- SkillRegistry with discovery, loading, lookup, legacy loader
    tests/
      mod.rs         (~559 lines)
```

#### B3-7: `clawft-core/src/pipeline/llm_adapter.rs` (1,129 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| `ClawftLlmAdapter` struct + trait impl | 59-237 | Adapter wrapping clawft_llm::Provider |
| Conversion functions | 238-323 | `convert_value_to_message`, `convert_response_to_value` |
| Factory functions | 324-584 | `create_adapter_from_config`, `apply_config_overrides`, `create_adapter_for_provider`, `resolve_app_api_key`, `create_adapters_for_tiers`, `build_live_pipeline`, `build_router` |
| Tests | 585-1129 | ~544 lines of tests |

**Proposed split**:
```
pipeline/
  llm_adapter/
    mod.rs           (~120 lines) -- ClawftLlmAdapter struct + LlmProvider trait impl
    conversions.rs   (~100 lines) -- convert_value_to_message, convert_response_to_value
    factory.rs       (~270 lines) -- create_adapter_from_config, build_live_pipeline, build_router, helpers
    tests/
      mod.rs         (~544 lines)
```

#### B3-8: `clawft-core/src/pipeline/traits.rs` (1,106 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| Request/message types | 24-73 | `ChatRequest`, `LlmMessage` |
| Classification types | 75-109 | `TaskProfile`, `TaskType` |
| Routing types | 111-149 | `RoutingDecision`, `ResponseOutcome` |
| Quality types | 151-164 | `QualityScore` |
| Context/transport types | 166-201 | `AssembledContext`, `TransportRequest` |
| Learning types | 203-229 | `Trajectory`, `LearningSignal` |
| Pipeline traits | 231-360 | 6 stage traits + `BudgetResult`, `CostTrackable`, `RateLimitable` |
| Pipeline & Registry | 362-509 | `Pipeline`, `PipelineRegistry` with orchestration logic |
| Tests | 510-1106 | ~596 lines of tests |

**Proposed split**:
```
pipeline/
  traits/
    mod.rs           (~80 lines) -- Re-exports of all public items
    types.rs         (~200 lines) -- ChatRequest, LlmMessage, TaskProfile, TaskType, RoutingDecision, ResponseOutcome, QualityScore, AssembledContext, TransportRequest, Trajectory, LearningSignal
    stages.rs        (~130 lines) -- 6 stage traits (TaskClassifier, ModelRouter, etc.)
    budget.rs        (~60 lines) -- BudgetResult, CostTrackable, RateLimitable
    registry.rs      (~160 lines) -- Pipeline, PipelineRegistry with orchestration
    tests/
      mod.rs         (~596 lines)
```

#### B3-9: `clawft-types/src/routing.rs` (948 lines)

**Current structure**:
| Section | Lines | Description |
|---------|-------|-------------|
| `TierSelectionStrategy` enum | 15-32 | Enum for model selection |
| `RoutingConfig` | 34-94 | Top-level routing config |
| `ModelTierConfig` | 96-144 | Tier definition |
| `PermissionsConfig` | 146-174 | Permission levels |
| `PermissionLevelConfig` | 176-251 | Detailed level config |
| `UserPermissions` | 252-378 | Runtime permissions with defaults |
| `AuthContext` | 379-444 | Authentication context |
| `EscalationConfig` | 445-480 | Escalation settings |
| `CostBudgetConfig` | 481-515 | Budget config |
| `RateLimitConfig` | 516-556 | Rate limit config |
| Tests | 557-948 | ~391 lines of tests |

**Proposed split**:
```
routing/
  mod.rs             (~80 lines) -- Re-exports
  config.rs          (~200 lines) -- RoutingConfig, ModelTierConfig, EscalationConfig, CostBudgetConfig, RateLimitConfig
  permissions.rs     (~200 lines) -- PermissionsConfig, PermissionLevelConfig, UserPermissions, TierSelectionStrategy
  auth.rs            (~70 lines) -- AuthContext
  tests.rs           (~391 lines)
```

**Note**: The tests file at 391 lines is within the 500-line target.

#### B3 Summary

| File | Current | Post-split largest file | Modules created |
|------|---------|------------------------|-----------------|
| `loop_core.rs` | 1,668 | ~200 (mod.rs) | 4 + tests |
| `tiered_router.rs` | 1,645 | ~180 (mod.rs) | 7 + tests |
| `config.rs` | 1,382 | ~400 (channels.rs) | 6-7 + tests |
| `transport.rs` | 1,281 | ~120 (mod.rs) | 3 + tests |
| `registry.rs` | 1,241 | ~200 (mod.rs) | 3 + tests |
| `skills_v2.rs` | 1,158 | ~280 (parser.rs) | 2 + tests |
| `llm_adapter.rs` | 1,129 | ~270 (factory.rs) | 3 + tests |
| `traits.rs` | 1,106 | ~200 (types.rs) | 4 + tests |
| `routing.rs` | 948 | ~200 (config.rs or permissions.rs) | 3 + tests |

**Execution strategy**: The 9 file splits are independent of each other (except config.rs depending on A4). They can be parallelized across workers. Each split should:

1. Create the new module directory
2. Move code to submodules
3. Add `mod.rs` with re-exports to maintain the same public API
4. Run `cargo build` to verify no breakage
5. Run `cargo test` on the affected crate

**Acceptance criteria**:
- [ ] No file exceeds 500 lines after split
- [ ] All public APIs unchanged (re-exports preserve existing import paths)
- [ ] `cargo build` succeeds for all crates
- [ ] `cargo test` passes for all crates
- [ ] `config.rs` split includes `secret.rs` slot for A4

---

### B4: Unify cron storage formats (P1)

**Problem**: Two incompatible storage formats for cron jobs.

| Location | Format | Description |
|----------|--------|-------------|
| `clawft-cli/src/commands/cron.rs` | Flat JSON (`CronStore`) | CLI reads/writes `~/.clawft/cron.json` as a single JSON object |
| `clawft-services/src/cron_service/storage.rs` | JSONL event sourcing | Gateway uses `StorageEvent` (Create/Update/Delete) appended as newline-delimited JSON |

**CLI code -- cron.rs:54-73** (flat JSON):
```rust
fn load_store(path: &Path) -> anyhow::Result<CronStore> {
    if !path.exists() {
        return Ok(CronStore::default());
    }
    let content = std::fs::read_to_string(path)?;
    let store: CronStore = serde_json::from_str(&content)?;
    Ok(store)
}

fn save_store(path: &Path, store: &CronStore) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(store)?;
    std::fs::write(path, content)?;
    Ok(())
}
```

**Service code -- storage.rs:16-29** (JSONL event sourcing):
```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StorageEvent {
    Create { job: CronJob },
    Update { job_id: String, field: String, value: serde_json::Value },
    Delete { job_id: String },
}
```

**Bug**: Jobs created via CLI are invisible to the gateway and vice versa because they use incompatible formats at potentially different file paths.

**Fix**:

1. **Unify on JSONL event sourcing** (the `CronStorage` approach from `clawft-services`).

2. **Make `CronStorage` available from a shared location** -- either promote it to `clawft-types` or make `clawft-cli` depend on `clawft-services`.

3. **Rewrite CLI cron commands** to use `CronStorage` instead of flat JSON:
   - `cron add` -> `storage.append_create(job)`
   - `cron remove` -> `storage.append_delete(job_id)`
   - `cron list` -> `storage.load_jobs()` (replay events)
   - `cron enable/disable` -> `storage.append_update(job_id, "enabled", true/false)`

4. **Migration**: On first CLI run, if `cron.json` exists in flat format, convert it to JSONL format (append a `Create` event for each job).

5. **Unify file path**: Both CLI and service should use the same file path (e.g., `~/.clawft/cron.jsonl`).

**Files to modify**:
- `crates/clawft-services/src/cron_service/storage.rs` -- Make `CronStorage` public, potentially move
- `crates/clawft-cli/src/commands/cron.rs` -- Rewrite to use `CronStorage`
- `crates/clawft-types/src/cron.rs` -- May need shared types

**Acceptance criteria**:
- [ ] CLI and service share the same JSONL storage
- [ ] Jobs created via CLI visible to gateway
- [ ] Jobs created via gateway visible to CLI
- [ ] Migration from old flat JSON format handled
- [ ] All cron tests pass

---

### B5: Extract shared tool registry builder (P2)

**Problem**: Identical 6-step tool setup block is copy-pasted in three CLI command files.

**Duplication locations**:

1. **`clawft-cli/src/commands/agent.rs:93-121`**:
```rust
let command_policy = build_command_policy(&config.tools.command_policy);
let url_policy = build_url_policy(&config.tools.url_policy);
let workspace = expand_workspace(&config.agents.defaults.workspace);
let web_search_config = build_web_search_config(&config.tools);
clawft_tools::register_all(ctx.tools_mut(), platform.clone(), workspace, command_policy, url_policy, web_search_config);
crate::mcp_tools::register_mcp_tools(&config, ctx.tools_mut()).await;
crate::mcp_tools::register_delegation(&config.delegation, ctx.tools_mut());
let bus_ref = ctx.bus().clone();
ctx.tools_mut().register(Arc::new(clawft_tools::message_tool::MessageTool::new(bus_ref)));
```

2. **`clawft-cli/src/commands/gateway.rs:121-148`** -- Nearly identical, but calls `super::agent::build_command_policy` etc.

3. **`clawft-cli/src/commands/mcp_server.rs:60-81`** -- Same pattern, without `MessageTool` (no bus).

**Fix**:

Extract a shared function in `commands/mod.rs` or a new `commands/tool_setup.rs`:

```rust
/// Build and populate a tool registry from config.
///
/// Steps:
/// 1. Build security policies from config
/// 2. Register built-in tools (file, shell, web, etc.)
/// 3. Register MCP server tools
/// 4. Register delegation tool
pub async fn build_tool_registry(
    registry: &mut ToolRegistry,
    config: &Config,
    platform: Arc<NativePlatform>,
) {
    let command_policy = build_command_policy(&config.tools.command_policy);
    let url_policy = build_url_policy(&config.tools.url_policy);
    let workspace = expand_workspace(&config.agents.defaults.workspace);
    let web_search_config = build_web_search_config(&config.tools);

    clawft_tools::register_all(
        registry, platform, workspace, command_policy, url_policy, web_search_config,
    );

    crate::mcp_tools::register_mcp_tools(config, registry).await;
    crate::mcp_tools::register_delegation(&config.delegation, registry);
}
```

The `MessageTool` registration stays in `agent.rs` and `gateway.rs` (it needs a bus reference that `mcp_server.rs` does not have).

**Files to modify**:
- `crates/clawft-cli/src/commands/mod.rs` (or new `tool_setup.rs`) -- Add shared function
- `crates/clawft-cli/src/commands/agent.rs` -- Replace duplicated block with call
- `crates/clawft-cli/src/commands/gateway.rs` -- Replace duplicated block with call
- `crates/clawft-cli/src/commands/mcp_server.rs` -- Replace duplicated block with call

**Acceptance criteria**:
- [ ] Single `build_tool_registry()` function used by all three commands
- [ ] `MessageTool` registration remains per-command (only where bus exists)
- [ ] All CLI commands produce the same tool set as before
- [ ] `cargo test` passes

---

### B6: Extract shared policy types (P2)

**Dependency**: A6 (SSRF fix) should land first so the canonical `UrlPolicy` includes the complete private IP range check.

**Problem**: `CommandPolicy` and `UrlPolicy` are defined in both `clawft-services` and `clawft-tools` because `clawft-services` does not depend on `clawft-tools`.

| Type | `clawft-tools` | `clawft-services/src/mcp/middleware.rs` |
|------|---------------|----------------------------------------|
| `CommandPolicy` | `src/security_policy.rs:42` -- Full implementation with `PolicyMode`, safe_defaults | `middleware.rs:70` -- Simplified mirror with `HashSet<String>` allowlist |
| `UrlPolicy` | `src/url_safety.rs:66` -- Full with `check_private_ips`, `blocked_domains`, `allowed_domains` | `middleware.rs:151` -- Simplified with `enabled` flag and `blocked_domains` |

**Current code -- middleware.rs:64-75** (simplified mirror):
```rust
/// Simplified command execution policy for middleware use.
///
/// Since `clawft-services` does not depend on `clawft-tools`, this is a
/// local mirror of the essential validation logic from
/// `clawft_tools::security_policy::CommandPolicy`.
pub struct CommandPolicy {
    pub allowed_commands: HashSet<String>,
    pub dangerous_patterns: Vec<String>,
}
```

**Fix**:

1. **Move canonical policy trait/types to `clawft-types`**:
   - Create `clawft-types/src/security.rs` with shared `CommandPolicyConfig` and `UrlPolicyConfig` types (the serializable config structs already exist in `config.rs`)
   - Add a `PolicyValidation` trait that both the full and simplified impls can satisfy

2. **Alternative (simpler)**: Add `clawft-tools` as a dependency of `clawft-services` and import directly. This is simpler but increases coupling.

3. **Recommended approach**: Since the middleware only needs the validation interface (not the full tool infrastructure), extract the policy structs and their `validate()` methods into `clawft-types/src/policy.rs`. Both crates import from there.

**After A6**: The canonical `UrlPolicy` in `clawft-types` must include the complete SSRF IP check (172.16.0.0/12, 10.0.0.0/8, 192.168.0.0/16, 127.0.0.0/8, etc.).

**Files to modify**:
- `crates/clawft-types/src/policy.rs` (new) -- Canonical `CommandPolicy`, `UrlPolicy`
- `crates/clawft-types/src/lib.rs` -- Add `pub mod policy`
- `crates/clawft-tools/src/security_policy.rs` -- Import from `clawft-types`
- `crates/clawft-tools/src/url_safety.rs` -- Import from `clawft-types`
- `crates/clawft-services/src/mcp/middleware.rs` -- Remove local types, import from `clawft-types`

**Acceptance criteria**:
- [ ] Single `CommandPolicy` and `UrlPolicy` definition in `clawft-types`
- [ ] `clawft-tools` imports from `clawft-types`
- [ ] `clawft-services/mcp/middleware.rs` imports from `clawft-types`
- [ ] SSRF IP check is complete (post-A6)
- [ ] All security tests pass

---

### B7: Deduplicate `ProviderConfig` naming (P2)

**Dependency**: B1 (Usage unification) should land first to avoid concurrent churn in `clawft-llm` types.

**Problem**: Both `clawft-llm` and `clawft-types` define a `ProviderConfig` struct with different semantics.

| Crate | Location | Purpose | Key field |
|-------|----------|---------|-----------|
| `clawft-llm` | `src/config.rs:12` | LLM endpoint config (base_url, headers) | `api_key_env: String` (env var name) |
| `clawft-types` | `src/config.rs:777` | Provider credentials from config file | `api_key: String` (plaintext key) |

**Current code -- clawft-llm/src/config.rs:12-35**:
```rust
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,        // <-- env var NAME, not the key itself
    pub model_prefix: Option<String>,
    pub default_model: Option<String>,
    pub headers: HashMap<String, String>,
}
```

**Current code -- clawft-types/src/config.rs:777-789**:
```rust
pub struct ProviderConfig {
    pub api_key: String,            // <-- plaintext API key
    pub api_base: Option<String>,
    pub extra_headers: Option<HashMap<String, String>>,
}
```

**Fix options**:

**Option A (rename)**: Rename `clawft-llm`'s to `LlmProviderConfig`:
```rust
pub struct LlmProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,
    // ...
}
```
This is the simplest fix. `clawft-llm` already imports it as `LlmProviderConfig` in some places (`llm_adapter.rs:29`).

**Option B (merge)**: Merge into a single type in `clawft-types` that covers both use cases. This is harder because the fields serve different purposes.

**Recommended**: Option A -- rename to `LlmProviderConfig` in `clawft-llm`.

**Note**: After A4 (SecretRef), the `clawft-types::ProviderConfig::api_key` field will become `SecretRef` instead of `String`, further differentiating the two types.

**Files to modify**:
- `crates/clawft-llm/src/config.rs` -- Rename struct to `LlmProviderConfig`
- `crates/clawft-llm/src/lib.rs` -- Update re-exports
- `crates/clawft-core/src/pipeline/llm_adapter.rs` -- Already uses `LlmProviderConfig` alias (line 29), just update the import source

**Acceptance criteria**:
- [ ] No naming collision between the two `ProviderConfig` types
- [ ] `clawft-llm` uses `LlmProviderConfig` consistently
- [ ] All callers updated
- [ ] `cargo build` succeeds

---

### B8: Consolidate `build_messages` duplication (P2)

**Problem**: `build_messages()` and `build_messages_for_agent()` in `context.rs` share ~80% of their code.

**Location**: `clawft-core/src/agent/context.rs`

**Shared logic** (duplicated in both methods):
1. Skills loading (lines 233-265 in `build_messages`, lines 370-401 in `build_messages_for_agent`)
2. Memory context (lines 269-281 in `build_messages`, lines 415-427 in `build_messages_for_agent`)
3. History truncation (lines 284-302 in `build_messages`, lines 430-448 in `build_messages_for_agent`)

**Unique to `build_messages_for_agent`**:
- Uses agent-specific system prompt (line 361)
- Takes agent's skill list (line 369)
- Has `extra_skill_instructions` injection (lines 404-412)

**Fix**: Extract a shared `build_messages_inner()` method:

```rust
/// Shared core of build_messages / build_messages_for_agent.
async fn build_messages_inner(
    &self,
    system_prompt: String,
    session: &Session,
    skill_names: &[String],
    extra_instructions: Option<&str>,
) -> Vec<LlmMessage> {
    let mut messages = Vec::new();

    // 1. System prompt
    messages.push(LlmMessage { role: "system".into(), content: system_prompt, tool_call_id: None });

    // 2. Skill prompts (shared logic)
    for skill_name in skill_names {
        // ... existing skill loading logic
    }

    // 3. Extra instructions (agent-only)
    if let Some(instructions) = extra_instructions && !instructions.trim().is_empty() {
        messages.push(LlmMessage {
            role: "system".into(),
            content: format!("# Skill Instructions\n\n{instructions}"),
            tool_call_id: None,
        });
    }

    // 4. Memory context (shared logic)
    // ...

    // 5. History (shared logic)
    // ...

    messages
}
```

Then both public methods become thin wrappers:

```rust
pub async fn build_messages(&self, session: &Session, active_skills: &[String]) -> Vec<LlmMessage> {
    let system_prompt = self.build_system_prompt().await;
    self.build_messages_inner(system_prompt, session, active_skills, None).await
}

pub async fn build_messages_for_agent(
    &self, session: &Session, agent: &AgentDefinition, args: &str,
    extra_skill_instructions: Option<&str>,
) -> Vec<LlmMessage> {
    let system_prompt = self.build_system_prompt_for_agent(agent, args).await;
    let skill_names: Vec<String> = agent.skills.clone();
    self.build_messages_inner(system_prompt, session, &skill_names, extra_skill_instructions).await
}
```

**Files to modify**:
- `crates/clawft-core/src/agent/context.rs` -- Refactor into shared inner method

**Acceptance criteria**:
- [ ] No duplicated skill/memory/history logic
- [ ] Both public methods produce identical output to before
- [ ] All 12+ existing context builder tests pass unchanged
- [ ] No behavior change

---

### B9: MCP protocol version constant (P2)

**Problem**: The string `"2025-06-18"` is hardcoded in multiple places across two files.

**Occurrences found**:

| File | Line | Context |
|------|------|---------|
| `clawft-services/src/mcp/server.rs` | 17 | `const PROTOCOL_VERSION: &str = "2025-06-18";` (good, but local) |
| `clawft-services/src/mcp/server.rs` | 313 | Hardcoded in test: `"protocolVersion": "2025-06-18"` |
| `clawft-services/src/mcp/mod.rs` | 202 | `"protocolVersion": "2025-06-18"` in handshake |
| `clawft-services/src/mcp/mod.rs` | 224 | Fallback: `.unwrap_or("2025-06-18")` |
| `clawft-services/src/mcp/mod.rs` | 449, 482 | Test assertions using literal `"2025-06-18"` |

**Fix**:

1. **Create a shared constant** in `clawft-services/src/mcp/types.rs` (or `mod.rs` if no `types.rs` exists):
```rust
/// MCP protocol version for server identification and client handshake.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
```

2. **Replace all occurrences** in both `server.rs` and `mod.rs` with `MCP_PROTOCOL_VERSION`.

3. **Update tests** to reference the constant instead of hardcoded strings.

**Files to modify**:
- `crates/clawft-services/src/mcp/server.rs` -- Remove local `PROTOCOL_VERSION`, use shared const
- `crates/clawft-services/src/mcp/mod.rs` -- Replace all `"2025-06-18"` with const
- Create `crates/clawft-services/src/mcp/types.rs` if needed, or add to existing module

**Acceptance criteria**:
- [ ] Single `MCP_PROTOCOL_VERSION` constant
- [ ] Zero hardcoded `"2025-06-18"` strings (except in the constant definition)
- [ ] All MCP tests pass
- [ ] Updating the version requires changing exactly one line

---

## 3. Execution Order

```
Week 2 (can start immediately):
  B1 (Usage)      -- parallel
  B2 (LlmMessage) -- parallel
  B4 (Cron)       -- parallel
  B5 (ToolReg)    -- parallel
  B8 (Messages)   -- parallel
  B9 (MCP const)  -- parallel

Week 2-3 (after A4 lands):
  B3 (File splits) -- largest item, may span 1-2 weeks

Week 3 (after A6 lands):
  B6 (Policy types) -- needs SSRF fix

Week 3 (after B1 lands):
  B7 (ProviderConfig) -- needs Usage unification to avoid churn
```

---

## 4. Exit Criteria

- [x] All P1 items (B1-B4) resolved and tested -- DONE 2026-02-19
- [x] All P2 items (B5-B9) resolved and tested -- DONE 2026-02-19
- [x] No file over 500 lines (B3 target) -- impl under 500; test bulk acceptable
- [x] Zero duplicate type definitions across crate boundaries -- DONE
- [x] `cargo build` succeeds for all crates -- VERIFIED
- [x] `cargo test` passes for all crates -- VERIFIED
- [x] `cargo clippy` clean (no new warnings introduced) -- VERIFIED
- [x] All public APIs preserved (re-exports where needed) -- VERIFIED

---

## 5. Risk Notes

- **B3 is large**: 9 file splits with thousands of lines moved. Merge conflicts likely if other workstreams touch the same files during the split. Recommend completing B3 file splits in quick succession on a dedicated branch.
- **B1 field rename**: Changing `input_tokens` to `prompt_tokens` has wide ripple. The `#[serde(alias)]` approach ensures deserialization backward compat but callsite code must be updated.
- **B4 migration**: Existing flat JSON cron stores need one-time conversion. A migration function must be robust against partial writes.
- **B6 coupling**: Extracting policy types to `clawft-types` creates a new dependency edge. Ensure `clawft-types` stays infrastructure-only (no business logic in the policy validate methods if possible).
