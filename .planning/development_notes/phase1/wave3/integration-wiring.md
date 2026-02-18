# Integration Wiring -- Development Note

**Date**: 2026-02-16
**Module**: clawft-core integration layer
**Files modified/created**:
- `crates/clawft-core/src/agent/loop_core.rs` (rewritten)
- `crates/clawft-core/src/pipeline/transport.rs` (rewritten)
- `crates/clawft-core/src/bootstrap.rs` (new)
- `crates/clawft-core/src/lib.rs` (added `pub mod bootstrap;`)

## Summary

Completed the integration wiring layer that connects all clawft-core components
into a functioning agent loop. This replaces the previous scaffolds in
`loop_core.rs` and `transport.rs` with full implementations, and adds a
bootstrap module for one-step initialization.

## Module 1: AgentLoop (`loop_core.rs`)

Full implementation of the consume-process-respond cycle:

- **`AgentLoop<P>`** struct holds all 7 dependencies: config, platform, bus,
  pipeline, tools, context, sessions.
- **`run()`** infinite loop consuming from the MessageBus, processing each
  message, logging errors without terminating.
- **`process_message()`** 10-step pipeline: session lookup, user message append,
  context building, type conversion between `context::LlmMessage` and
  `pipeline::traits::LlmMessage`, pipeline request construction, tool loop
  execution, session persistence, outbound dispatch.
- **`run_tool_loop()`** iterative tool execution with `max_tool_iterations`
  guard. Extracts `ContentBlock::ToolUse` from LLM responses, executes via
  `ToolRegistry`, appends tool results as `role=tool` messages, re-invokes
  pipeline. Returns `ClawftError::Provider` when limit exceeded.

### Design decisions

- Two LlmMessage types exist (`agent::context::LlmMessage` and
  `pipeline::traits::LlmMessage`). They have identical structure but are
  defined in different modules. `process_message()` maps between them. This
  will be unified in a future refactor.
- Tool error results are serialized as JSON strings with an `error` key,
  matching the OpenAI convention for tool result errors.

## Module 2: Transport (`transport.rs`)

Dual-mode transport with backward compatibility:

- **`LlmProvider` trait** bridges the pipeline to any HTTP LLM client using
  simple types (`serde_json::Value`). This avoids importing `clawft-llm` types
  directly since the crate's `lib.rs` does not yet export its modules.
- **`OpenAiCompatTransport::new()`** returns a stub (returns error on every
  call). Used during development or when no provider is configured.
- **`OpenAiCompatTransport::with_provider()`** wraps any `LlmProvider` for
  real HTTP calls.
- **`convert_response()`** translates raw OpenAI-format JSON to
  `clawft_types::provider::LlmResponse`. Handles text, tool calls, mixed
  content, missing usage, invalid tool arguments JSON, all stop reasons.

### Design decisions

- The `LlmProvider` trait uses `serde_json::Value` for both messages and
  responses, not `clawft-llm` types. This is because `clawft-llm/src/lib.rs`
  is currently a stub with no module declarations. Once it exports types, a
  thin adapter implementing `LlmProvider` that wraps `clawft_llm::Provider`
  will be trivial.

## Module 3: Bootstrap (`bootstrap.rs`)

One-step initialization from `Config` + `Platform`:

- **`AppContext<P>`** holds all initialized components: config, platform, bus,
  sessions, tools, pipeline, context, memory, skills.
- **`AppContext::new()`** initializes everything in order: bus, sessions,
  memory, skills, context builder, empty tool registry, default Level 0
  pipeline.
- **`into_agent_loop()`** consumes the context and produces an `AgentLoop`.
- **`tools_mut()`** allows registering tools before consuming.
- **`set_pipeline()`** allows replacing the default pipeline with a custom one
  (e.g. one with a real LLM provider transport).
- **`bus()`** returns a reference so channel adapters can clone the inbound
  sender before the context is consumed.

### Design decisions

- `MemoryStore::new(platform)` and `SkillsLoader::new(platform)` are used
  instead of `with_paths` / `with_dir` (which are `#[cfg(test)]` only). These
  resolve paths from the platform home directory.
- Default pipeline uses stub transport (no real LLM calls). The caller must
  either inject an `LlmProvider` via `set_pipeline()` or the agent will return
  errors on every LLM call.
- `build_default_pipeline()` is a standalone function (not a method) to allow
  reuse in tests.

## Module 4: lib.rs

Added `pub mod bootstrap;` to module declarations.

## Test Coverage

200 tests total in clawft-core, all passing:
- `loop_core`: 8 tests (construction, process_message, tool loop, max
  iterations, session persistence, send bound)
- `transport`: 17 tests (stub mode, provider mode, convert_response edge cases)
- `bootstrap`: 13 tests (construction, accessors, tool registration, pipeline
  replacement, agent loop production, bus clone survival, send bound)

## Known Limitations

1. **clawft-llm types not importable**: The `clawft-llm` crate compiles but
   exports nothing (empty `lib.rs`). The `LlmProvider` trait in transport.rs
   is a workaround. Once `clawft-llm` exports its `Provider` trait, an adapter
   should be written.

2. **Two LlmMessage types**: `context::LlmMessage` and
   `traits::LlmMessage` have the same shape but are separate types. A future
   refactor should unify them.

3. **Stub transport in bootstrap**: The default pipeline has no real LLM
   provider. Callers must configure one for production use.

## Next Steps

- Export modules from `clawft-llm/src/lib.rs` and write an adapter
  implementing `LlmProvider` that wraps `clawft_llm::Provider`.
- Register built-in tools from `clawft-tools` in the bootstrap.
- Wire channel adapters (Telegram, Slack) to the MessageBus.
- Unify the two LlmMessage types.
