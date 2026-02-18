# Sprint 3: ruvllm Quality Scoring and Complexity Analysis Stubs

## Summary

Implemented two modules gated behind the `rvf` feature flag:

1. **`scoring`** (`crates/clawft-core/src/scoring.rs`) -- Quality scoring trait
   and implementations for evaluating LLM response quality.
2. **`complexity`** (`crates/clawft-core/src/complexity.rs`) -- Task complexity
   analyzer for driving 3-tier model routing decisions.

These are standalone RVF-layer modules, separate from the existing pipeline
scorer in `crate::pipeline::scorer` which operates on typed request/response
pairs.

## Scoring Module

### QualityScorer Trait

```rust
pub trait QualityScorer: Send + Sync {
    fn score(&self, request: &str, response: &str) -> f32;
    fn record(&mut self, request: &str, response: &str, score: f32);
}
```

### Implementations

**NoopScorer** -- Always returns 0.5. Default when no real model is loaded.

**BasicScorer** -- Heuristic scorer using three signals:
- Length component (0.0-0.4): longer responses score higher up to 500 words
- Error penalty (-0.3): detects refusal phrases ("I can't", "I don't know", etc.)
- Tool-use bonus (+0.2): detects tool invocation patterns in responses
- Base score (0.3): any non-empty response gets a baseline

Records scored observations to a history vector for trend analysis.

### Tests (10 total)

- NoopScorer always returns 0.5
- NoopScorer record does not panic
- BasicScorer: long > short, error phrases detected, tool-use bonus
- Empty response scores zero
- Score clamped to [0.0, 1.0]
- History recording

## Complexity Module

### TaskComplexityAnalyzer

```rust
pub struct TaskComplexityAnalyzer;

impl TaskComplexityAnalyzer {
    pub fn new() -> Self;
    pub fn analyze(&self, task: &str) -> f32; // 0.0..=1.0
}
```

Five heuristic components:
1. **Length** (0.0-0.25): word count / 200, capped
2. **Sentence count** (0.0-0.15): proxy for requirement count
3. **Technical keywords** (0.0-0.25): 22 domain terms (api, database, kubernetes, etc.)
4. **Multi-step indicators** (0.0-0.20): "first", "then", "finally", "step 1", etc.
5. **Code fences** (0.15): presence of triple backticks

### Tests (10 total)

- Simple question < 0.3
- Complex multi-part > simple
- Technical keywords raise score
- Empty task returns 0.0
- Score clamped to [0.0, 1.0]
- Code fences, step indicators, longer text, multiple sentences

## Relationship to Existing Code

- `crate::intelligent_router::compute_complexity` -- Similar heuristic, used for
  the existing 3-tier router. The new `TaskComplexityAnalyzer` is a richer,
  self-contained version intended for the RVF layer.
- `crate::pipeline::scorer::NoopScorer` -- Pipeline-level scorer using typed
  `ChatRequest`/`LlmResponse`. The new `scoring::NoopScorer` works with raw
  strings for the RVF integration layer.

## Files Changed

- `crates/clawft-core/src/lib.rs` -- Added `scoring` and `complexity` modules under `rvf` gate
- `crates/clawft-core/src/scoring.rs` -- New file
- `crates/clawft-core/src/complexity.rs` -- New file

## Future Work

- Integrate BasicScorer with the intelligent router's cost/quality feedback loop
- Add ML-based scoring when ruvllm model bindings are available
- Connect TaskComplexityAnalyzer output to the ProgressiveSearch tier selection
