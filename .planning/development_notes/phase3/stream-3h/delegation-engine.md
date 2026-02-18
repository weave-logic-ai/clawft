# Stream 3H-F: DelegationConfig Types + Engine

## Summary

Implemented the delegation configuration types in `clawft-types` and the
delegation engine in `clawft-services` (behind the `delegate` feature gate).
This provides the foundation for routing tasks between local execution,
Claude AI, and Claude Flow orchestration.

## Files Modified

### clawft-types
- `crates/clawft-types/src/delegation.rs` -- **NEW**: `DelegationConfig`,
  `DelegationRule`, `DelegationTarget` types with serde support
- `crates/clawft-types/src/lib.rs` -- Added `pub mod delegation;`
- `crates/clawft-types/src/config.rs` -- Added `delegation: DelegationConfig`
  field to root `Config` struct (with `#[serde(default)]`)

### clawft-services
- `crates/clawft-services/src/delegation/mod.rs` -- **REWRITTEN**:
  `DelegationEngine` with regex-based rule matching and complexity heuristics.
  Now uses types from `clawft_types::delegation` instead of local types.
- `crates/clawft-services/src/delegation/schema.rs` -- **NEW**:
  `openai_to_anthropic()` and `tool_result_block()` schema conversion utilities
- `crates/clawft-services/src/delegation/claude.rs` -- Fixed `mut` on mockito
  servers (pre-existing issue)
- `crates/clawft-services/src/lib.rs` -- Added feature-gated
  `#[cfg(feature = "delegate")] pub mod delegation;`
- `crates/clawft-services/Cargo.toml` -- Added `delegate` feature with
  `regex` dependency

### Workspace
- `Cargo.toml` -- Added `regex = "1"` to workspace dependencies

## Design Decisions

### Type Location
`DelegationConfig`, `DelegationRule`, and `DelegationTarget` live in
`clawft-types` so they can be referenced by any crate in the workspace
(including the CLI, core, and services). The engine implementation lives
in `clawft-services` because it has runtime dependencies (regex).

### Complexity Heuristic
The `complexity_estimate()` function uses a weighted average of:
- **Length factor** (30%): Longer tasks are assumed more complex, saturating
  at 500 characters.
- **Question mark density** (20%): Questions suggest research/investigation.
- **Keyword hits** (50%): A curated list of 20 complexity-indicating keywords.

Thresholds for auto-routing:
- < 0.3: Local
- 0.3..0.7: Claude (if available)
- >= 0.7: Flow (if available)

### Feature Gating
The `delegate` feature gates both the `delegation` module and the `regex`
dependency. Without this feature, the delegation code is not compiled,
keeping the binary size minimal for deployments that don't need delegation.

### Schema Conversion
`openai_to_anthropic()` handles the common case of tools defined in
OpenAI function-calling format that need to be sent to the Anthropic
Messages API. Invalid schemas are silently skipped rather than causing
errors, since tool definitions may come from external MCP servers.

## Test Coverage

### clawft-types (7 delegation tests)
- `delegation_config_defaults` -- Default values
- `delegation_config_serde_roundtrip` -- Full serialization cycle
- `delegation_config_from_empty_json` -- Empty JSON produces defaults
- `delegation_config_camel_case_aliases` -- camelCase field aliases
- `delegation_target_variants` -- All 4 enum variants serialize/deserialize
- `delegation_target_default_is_auto` -- Default variant is Auto

### clawft-services delegation::tests (12 tests)
- `rule_matching_dispatches_correctly` -- Regex rules route correctly
- `fallback_when_claude_unavailable` -- Claude -> Local fallback
- `fallback_when_flow_unavailable` -- Flow -> Claude -> Local fallback
- `auto_mode_low_complexity_is_local` -- Simple tasks stay local
- `auto_mode_high_complexity_routes_to_flow` -- Complex tasks go to Flow
- `auto_mode_medium_complexity_routes_to_claude` -- Medium tasks go to Claude
- `auto_mode_falls_back_to_local_when_services_disabled` -- Disabled = Local
- `complexity_estimate_empty_is_zero` -- Empty string = 0.0
- `complexity_estimate_scales_with_keywords` -- Keywords increase score
- `complexity_estimate_capped_at_one` -- Score never exceeds 1.0
- `invalid_regex_skipped` -- Bad regex patterns are silently skipped
- `first_rule_wins` -- First matching rule takes precedence

### clawft-services delegation::schema::tests (8 tests)
- `openai_to_anthropic_single_tool` -- Basic conversion
- `openai_to_anthropic_multiple_tools` -- Multiple tools
- `openai_to_anthropic_missing_description` -- Missing field defaults
- `openai_to_anthropic_missing_parameters` -- Missing params defaults
- `openai_to_anthropic_skips_invalid` -- Invalid schemas skipped
- `openai_to_anthropic_empty_input` -- Empty input = empty output
- `tool_result_block_format` -- Correct Anthropic format
- `tool_result_block_empty_result` -- Empty content
- `tool_result_block_json_content` -- JSON string content

## Verification

```
cargo test -p clawft-types         # 85 passed (7 new delegation tests)
cargo test -p clawft-services --features delegate  # 178 passed (20 new)
cargo clippy -p clawft-services --features delegate -- -D warnings  # 0 warnings
```
