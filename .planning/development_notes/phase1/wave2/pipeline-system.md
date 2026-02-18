# Pipeline System -- Development Notes

**Agent**: pipeline-engineer (coder)
**Date**: 2026-02-16
**Phase**: 1 / Wave 2
**Crate**: `clawft-core`
**Module**: `pipeline/`

## Summary

Implemented the complete 6-stage pluggable pipeline system in `clawft-core::pipeline`.
All 7 files have been written with Level 0 implementations and comprehensive tests.

## Files Written

| File | Lines | Purpose |
|------|-------|---------|
| `pipeline/traits.rs` | ~475 (prod) | All 6 traits + 12 supporting types + `Pipeline` + `PipelineRegistry` |
| `pipeline/classifier.rs` | ~260 (prod) | `KeywordClassifier` -- case-insensitive keyword matching |
| `pipeline/router.rs` | ~130 (prod) | `StaticRouter` -- config-driven provider/model selection |
| `pipeline/assembler.rs` | ~180 (prod) | `TokenBudgetAssembler` -- chars/4 token estimation + truncation |
| `pipeline/transport.rs` | ~60 (prod) | `OpenAiCompatTransport` -- stub returning `Provider` error |
| `pipeline/scorer.rs` | ~55 (prod) | `NoopScorer` -- returns perfect 1.0 scores |
| `pipeline/learner.rs` | ~55 (prod) | `NoopLearner` -- no-op record/adapt |

## Architecture Decisions

### No regex dependency
The spec suggested regex for the classifier, but `regex` is not in the workspace
dependencies. Rather than adding a new dependency for simple keyword matching,
the classifier uses `str::contains()` on lowercased input. This is sufficient
for Level 0 and avoids binary bloat. Level 1+ can introduce regex or ML-based
classification.

### Keyword priority ordering
Patterns are checked in a static priority order (code > review > research >
creative > analysis > tool_use). The first matching pattern group determines
the `TaskType`. All keyword hits from all patterns contribute to the complexity
score, but only the first-matched group sets the type.

### Complexity heuristic
Complexity = (matched keyword count) / (word count), clamped to [0.1, 0.9].
This is intentionally simple -- it gives higher complexity to messages that
mention many pipeline-relevant concepts (e.g. "implement debug fix code" is
denser than "Can you implement something for me in my big project today?").

### Token estimation
Using `chars / 4` + 4 overhead per message. This is a well-known rough
approximation for English text with typical tokenizers. The 4-token overhead
accounts for role, structural tokens, and separator tokens.

### Truncation strategy
The assembler always preserves the first message (system prompt) and fills
from the end of the conversation backwards. This matches the common pattern
where the system prompt provides essential instructions and the most recent
messages have the current context.

### Transport stub
The transport returns `ClawftError::Provider` rather than a custom error variant,
because the transport layer genuinely represents a provider-side concern. The
error message clearly states that `clawft-llm` is required, guiding the
integrator to wire up the real transport.

### `PipelineRegistry` orchestration
The registry uses the default pipeline's classifier for all incoming requests
(classification determines which specialized pipeline to use, so it must be
done before pipeline selection). All other stages use the selected pipeline.

## Test Coverage

Each file has a `#[cfg(test)]` module. Total test count across all files:

- `traits.rs`: 16 tests (type construction, serde roundtrips, registry orchestration)
- `classifier.rs`: 17 tests (each keyword group, priority, case insensitivity, complexity)
- `router.rs`: 9 tests (routing, config parsing, `from_config`, split logic)
- `assembler.rs`: 11 tests (empty, fits, truncation, preservation, estimation)
- `transport.rs`: 4 tests (error variant, error message, ignores fields)
- `scorer.rs`: 5 tests (perfect scores, ignores input/output, consistency)
- `learner.rs`: 6 tests (no-panic record/adapt, bulk calls, default)

**Total: 68 tests**

All tests are designed to pass without network access or external dependencies.
Async tests use `#[tokio::test]`.

## Dependencies Used

- `clawft-types` (workspace): `LlmResponse`, `ContentBlock`, `StopReason`, `Usage`, `ClawftError`, `Result`, `AgentsConfig`
- `async-trait` (workspace): `#[async_trait]` for async trait methods
- `serde` + `serde_json` (workspace): Serialization for `ChatRequest`, `LlmMessage`, `TaskType`
- `tokio` (workspace, dev): `#[tokio::test]` for async tests

No new dependencies were added.

## Integration Points

- **clawft-llm**: The `OpenAiCompatTransport` is the integration seam. When
  clawft-llm is ready, create a new transport that wraps its client and
  implements `LlmTransport`.
- **Agent loop**: The agent loop in `clawft-core::agent` should construct a
  `PipelineRegistry` and call `registry.complete(request)` for each turn.
- **Configuration**: `StaticRouter::from_config()` reads from `AgentsConfig`,
  bridging the config system to the pipeline.

## Next Steps

1. Wire `PipelineRegistry` into the agent loop
2. Implement real `LlmTransport` backed by `clawft-llm`
3. Level 1 classifier: TF-IDF or embedding-based classification
4. Level 1 router: EMA-based adaptive routing with latency/quality tracking
5. Level 1 scorer: Heuristic scoring (response length, format compliance)
6. Level 1 learner: EMA statistics + trajectory storage
