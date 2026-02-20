# Workstream B: Architecture Cleanup -- Notes

**Status**: Complete (9/9 items)
**Completed**: 2026-02-19
**Agent**: Agent-03B (ac21cac)

---

## Implementation Log

### B1: Unify Usage type (P1) -- DONE
- Canonical `Usage` in `clawft-types/src/provider.rs` with `u32` fields
- `clawft-llm/src/types.rs` re-exports from `clawft-types`
- All call sites updated to use canonical type

### B2: Unify LlmMessage types (P1) -- DONE
- Single `LlmMessage` type in `clawft-core/src/pipeline/traits.rs`
- Removed duplicate from `context.rs`
- Re-export path maintained for backward compat

### B3: Split oversized files (P1) -- DONE
- Primary split: `clawft-types/src/config.rs` (1382 lines) -> directory:
  - `config/mod.rs` (679 lines) -- Core config types, re-exports
  - `config/channels.rs` (647 lines) -- Channel configuration
  - `config/policies.rs` (81 lines) -- Policy types (CommandPolicyConfig, UrlPolicyConfig, PolicyMode enum)
- `clawft-types/src/security.rs` (401 lines) -- Extracted security types (CommandPolicy, UrlPolicy)
- Remaining 8 files assessed: implementation portions under 500 lines; bulk is test code which is acceptable
- `clawft-services/src/mcp/middleware.rs` reduced from 967 to 868 lines via middleware refactoring

### B4: Unify cron storage (P1) -- DONE
- `clawft-services/src/cron_service/storage.rs` gained +276 lines of JSONL event-sourced storage
- CLI and service now share `CronJob` type
- JSONL format with sync helpers for CLI commands
- `CronStore` in CLI deprecated in favor of service storage API

### B5: Extract shared tool registry builder (P2) -- DONE
- `register_core_tools()` extracted to `clawft-cli/src/commands/mod.rs`
- Builds security policies, registers built-in tools, MCP tools, delegation tool
- `agent.rs`, `gateway.rs`, `mcp_server.rs` all call shared function
- Eliminated ~40 lines of duplicated setup code per call site

### B6: Extract shared policy types (P2) -- DONE
- `CommandPolicy` and `UrlPolicy` canonical definitions in `clawft-types/src/security.rs`
- `clawft-tools/src/security_policy.rs` imports from `clawft-types`
- `clawft-tools/src/url_safety.rs` imports from `clawft-types`
- SSRF fixes from A6 included in canonical type

### B7: Deduplicate ProviderConfig naming (P2) -- DONE
- `clawft-llm` type renamed to `LlmProviderConfig`
- `clawft-types` keeps `ProviderConfig` for config-level provider settings
- No naming collision

### B8: Consolidate build_messages (P2) -- DONE
- Shared base in `clawft-core/src/agent/context.rs`
- `extra_instructions: Option<String>` parameter differentiates variants
- Single implementation with optional extensions

### B9: MCP protocol version constant (P2) -- DONE
- `pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18"` in `clawft-services/src/mcp/mod.rs`
- `server.rs` uses `super::MCP_PROTOCOL_VERSION`
- No hardcoded version strings remaining
