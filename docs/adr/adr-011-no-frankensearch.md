# ADR-011: Do Not Add FrankenSearch (Raw HNSW Sufficient)

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 9 (Optimization Plan)

## Context

FrankenSearch provides hybrid BM25 + HNSW search with Reciprocal Rank Fusion, solving the problem where pure semantic search misses exact keyword matches. The FrankenSearch Specialist evaluated adding it to WeftOS's search infrastructure. At current scale (<10K entries), pure HNSW semantic search is adequate. The keyword index would cost ~2x memory.

## Decision

Do not add FrankenSearch to v0.1 or v0.2. Raw HNSW search is sufficient at the current scale. Revisit at v0.3 only if user testing reveals missed exact-match queries (e.g., "I know the exact filename but search misses it").

The optimization priority is fixing the three HNSW algorithmic traps (O(n) upsert via retain, full rebuild on dirty, metadata cloning) which account for 60-70% of achievable tick latency reduction.

## Consequences

### Positive
- Simpler search stack -- one algorithm to maintain and optimize
- No additional memory overhead from keyword index
- Focus stays on fixing HNSW fundamentals (B1, B2 optimizations)

### Negative
- Pure semantic search will miss exact-match queries at scale
- If users need keyword search, it will be a v0.3+ feature with delay

### Neutral
- The HNSW fixes (HashMap index, deferred rebuild) are higher-impact and lower-effort (Score 12.5 and 8.3 vs FrankenSearch's Score 3.0)
