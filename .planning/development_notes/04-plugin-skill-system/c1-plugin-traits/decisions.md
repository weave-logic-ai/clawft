# Phase C1: Plugin Traits -- Decisions

## Separate crate vs module
- **Decision**: Created `clawft-plugin` as a standalone workspace crate, not a module in clawft-core
- **Rationale**: Plugin traits are depended on by nearly every other crate (clawft-wasm, clawft-channels, clawft-core, clawft-services). A separate crate avoids circular dependencies and keeps compilation parallel.

## async_trait vs native async
- **Decision**: Used `#[async_trait]` from the `async-trait` crate
- **Rationale**: Rust stable doesn't yet support `async fn` in traits with `dyn` dispatch. `async_trait` is the standard approach and matches the rest of the codebase.

## CancellationToken for ChannelAdapter
- **Decision**: `ChannelAdapter::run()` takes a `CancellationToken` for graceful shutdown
- **Rationale**: Channel adapters are long-running tasks. CancellationToken from tokio-util provides cooperative cancellation that integrates cleanly with tokio's async runtime.

## MessagePayload as enum
- **Decision**: `MessagePayload` is an enum (Text/Structured/Binary) rather than a trait
- **Rationale**: The payload variants are known at design time and need serde support. An enum is simpler, exhaustive-matchable, and doesn't require dynamic dispatch.

## VoiceHandler as placeholder
- **Decision**: `VoiceHandler` trait defined with method signatures but no implementations
- **Rationale**: Forward-compat hook for Workstream G (deferred). Ensures trait remains compilable as the codebase evolves. Feature-gated behind `voice` flag.
