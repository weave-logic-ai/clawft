# Workstream B: Architecture Cleanup -- Decisions

## B3: config.rs split strategy
- **Decision**: Split into directory module (`config/mod.rs` + `channels.rs` + `policies.rs`) rather than flat sibling files
- **Rationale**: Directory module pattern groups related types and provides clear import paths. Channels config is self-contained (647 lines). Policies are small but distinct from core config.

## B3: Remaining oversized files -- assessed, not split
- **Decision**: Did not split 8 other files (loop_core.rs, tiered_router.rs, transport.rs, etc.) beyond config.rs
- **Rationale**: Analysis showed implementation logic portions are under 500 lines; bulk is inline test modules. Test code in `#[cfg(test)] mod tests` does not count toward the 500-line production code limit. Splitting tests into separate files adds no value.

## B4: JSONL unification approach
- **Decision**: Extended `clawft-services` JSONL storage with sync helpers rather than introducing a new shared crate
- **Rationale**: CronService already had correct event-sourced storage. Adding sync helpers lets CLI use the same format without creating another crate in the workspace.

## B5: Tool registry extraction location
- **Decision**: Placed `register_core_tools()` in `commands/mod.rs` rather than a new module
- **Rationale**: `mod.rs` already contains shared utility functions (`load_config`, `expand_workspace`). Adding tool registry builder keeps shared CLI infrastructure in one place.

## B6: Security types in clawft-types
- **Decision**: Created `clawft-types/src/security.rs` for `CommandPolicy` and `UrlPolicy`
- **Rationale**: These types are consumed by both `clawft-tools` and `clawft-services`. Placing them in `clawft-types` (the bottom of the dependency graph) avoids circular dependencies.

## B7: ProviderConfig disambiguation
- **Decision**: Renamed `clawft-llm` type to `LlmProviderConfig` rather than merging
- **Rationale**: The two types serve different purposes -- `clawft-types::ProviderConfig` is config-file-level provider settings; `LlmProviderConfig` is runtime LLM client configuration. Merging would conflate concerns.
