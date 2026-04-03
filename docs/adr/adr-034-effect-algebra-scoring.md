# ADR-034: Five-Dimensional Effect Algebra for Governance Scoring

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K2 Symposium (C9: N-dimensional EffectVector), K3 Symposium D2 (context-based gate actions), ECC Symposium D7 (per-tree scoring), kernel-governance.md specification

## Context

The governance engine needs to evaluate whether an agent action should be permitted, warned, deferred, or denied. Binary allow/deny is too coarse -- a filesystem read has different risk characteristics than a shell execution, and governance rules need to reason about multiple dimensions of impact simultaneously. The K2 Symposium discussed N-dimensional scoring (C9) but deferred the generalization, choosing to establish a fixed 5-dimensional baseline first.

Every tool in the 27-tool built-in catalog has an `EffectVector` embedded in its `BuiltinToolSpec`. When an agent executes a tool, the governance gate extracts this vector and computes a magnitude score to determine whether thresholds are exceeded.

## Decision

Actions are scored along 5 fixed dimensions using the `EffectVector` struct defined in `crates/clawft-kernel/src/governance.rs`:

```rust
pub struct EffectVector {
    pub risk: f64,       // Probability of negative outcome (0.0 - 1.0)
    pub fairness: f64,   // Impact on equitable treatment (0.0 - 1.0)
    pub privacy: f64,    // Impact on data privacy (0.0 - 1.0)
    pub novelty: f64,    // How unprecedented the action is (0.0 - 1.0)
    pub security: f64,   // Impact on system security (0.0 - 1.0)
}
```

**Magnitude calculation**: The L2 norm (Euclidean distance from origin) of the 5-dimensional vector:

```
magnitude = sqrt(risk^2 + fairness^2 + privacy^2 + novelty^2 + security^2)
```

The maximum possible magnitude is `sqrt(5) = 2.236` (all dimensions at 1.0). In practice, most actions score well below 1.0.

**Threshold comparison**: The governance engine compares `magnitude` against the configured `risk_threshold` (default `0.7`, set in `governance.genesis`). If `magnitude > risk_threshold`, the action triggers blocking/warning behavior depending on the severity of active rules.

**Helper methods on `EffectVector`**:
- `magnitude()` -- L2 norm of all 5 dimensions
- `any_exceeds(threshold)` -- true if any single dimension exceeds the threshold
- `max_dimension()` -- returns the highest individual dimension value

**Tool catalog integration**: Each `BuiltinToolSpec` in the `ToolRegistry` includes an `EffectVector`. For example, `fs.read_file` might have `{risk: 0.1, privacy: 0.2}` while `shell_exec` might have `{risk: 0.8, security: 0.7}`. When the agent loop processes an `exec` command, it calls `GovernanceRequest::with_tool_context()` to attach the tool name and its effect vector to the gate context.

**Gate integration (K3 D2)**: The gate action remains generic `"tool.exec"` with the tool name and effect vector passed as context. The governance engine evaluates context fields (`context["tool"]`, `context["effect"]`, `context["pid"]`) to make tool-specific decisions without changing the gate action string.

## Consequences

### Positive
- Multi-dimensional scoring enables nuanced governance -- a high-privacy-low-risk action is treated differently from a high-risk-low-privacy action
- L2-norm provides a single scalar magnitude for threshold comparison while preserving dimensional information for rule matching
- Every built-in tool has a pre-assigned effect vector, so governance scoring works out-of-the-box for the 27-tool catalog
- The `any_exceeds()` helper enables per-dimension threshold rules in addition to magnitude-based rules
- Effect vectors are part of the genesis rules, making the scoring model tamper-evident

### Negative
- The 5 fixed dimensions are embedded in genesis rules, tool catalogs, and governance evaluation logic -- adding or removing a dimension is a breaking change that cascades across the system and requires a `governance.root.supersede` event
- K2 Symposium C9 called for refactoring to N-dimensional configurable named dimensions; the current 5D model defers this generalization, creating a known migration obligation
- L2-norm treats all dimensions equally -- there is no weighting mechanism for environments where security matters more than novelty, for example

### Neutral
- The 5 chosen dimensions (risk, fairness, privacy, novelty, security) align with AI-SDLC SOP categories and cover the governance concerns identified across K2, K3, and ECC symposiums
- A future ADR superseding this one would implement the C9 N-dimensional refactor, replacing the fixed struct with a `Vec<(String, f64)>` or similar
