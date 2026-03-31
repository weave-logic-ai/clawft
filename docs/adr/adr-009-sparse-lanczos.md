# ADR-009: Sparse Lanczos for Spectral Analysis

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 7 (Algorithmic Optimization)

## Context

The ECC cognitive substrate performs spectral analysis on the causal graph to identify structural patterns. The current implementation uses a dense Laplacian with O(k*n^2) complexity. At 1,000 nodes, this is acceptable. At 10,000 nodes, the dense approach takes ~5 seconds wall time, which exceeds the DEMOCRITUS tick budget and blocks real-time causal analysis.

## Decision

Replace the dense Laplacian spectral analysis with sparse Lanczos iteration at O(k*m) complexity, where m is the number of edges (typically m << n^2 for real-world graphs). This provides approximately 200x speedup at 10K nodes. The existing community detection (Label Propagation Algorithm) is already near-optimal at O(k*m) and does not need replacement. Louvain/Leiden will be added as an offline alternative at v0.3 for higher-quality visualization.

## Consequences

### Positive
- 200x speedup at 10K nodes enables real-time spectral analysis within the tick budget
- Sparse representation matches the actual graph structure (real-world causal graphs are sparse)
- Enables per-tick spectral analysis feasibility for the GUI knowledge graph view

### Negative
- Sparse Lanczos implementation is more complex than dense eigendecomposition
- Numerical stability requires careful handling of convergence criteria
- Estimated 8 hours of implementation effort (P2 priority)

### Neutral
- LPA for community detection remains unchanged -- already optimal
- This is a v0.2 deliverable, not required for v0.1.0 tag
