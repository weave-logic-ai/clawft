# Development Notes: Element 08 -- Memory & Workspace

## Structure

Three phase directories, each with four tracking files:

- `decisions.md` -- Key design decisions with rationale
- `blockers.md` -- Active blockers requiring escalation
- `difficult-tasks.md` -- Tasks needing extra attention or research
- `notes.md` -- General development notes, findings, tips

## Conventions

### Decision Records

Use this format:

```
## [DATE] Decision: [TITLE]

**Context**: Why this decision is needed
**Options**: What alternatives were considered
**Decision**: What was chosen
**Rationale**: Why
**Consequences**: What this means going forward
```

### Blocker Records

```
## [DATE] Blocker: [TITLE]

**Item**: [H1/H2.1/H2.3/etc.]
**Severity**: [Critical/High/Medium]
**Description**: What is blocking
**Attempted**: What has been tried
**Needs**: What is required to unblock
**Status**: [Active/Resolved]
```

### Difficult Task Records

```
## [DATE] Difficult: [TITLE]

**Item**: [H1/H2.1/H2.3/etc.]
**Difficulty**: [High/Very High]
**Why**: What makes this hard
**Approach**: Strategy being used
**Findings**: What was discovered during implementation
```

## Phase Directories

### `h1-workspace/` -- H1: Per-Agent Workspace Isolation (Week 4-6)

- Per-agent workspace dirs (`~/.clawft/agents/<agentId>/`)
- Session isolation and per-agent skill overrides
- Cross-agent shared memory protocol (symlink-based, read-only default)

### `h2-vector-memory/` -- H2: RVF Phase 3 Vector Memory (Week 5-8)

- H2.1: HNSW-backed VectorStore (instant-distance)
- H2.2: Production Embedder trait (HashEmbedder + ApiEmbedder)
- H2.3: RVF segment I/O (depends on RVF 0.2 audit)
- H2.4: `weft memory export/import` CLI
- H2.5: POLICY_KERNEL persistence
- H2.6: WITNESS segments (SHA-256 hash chain) -- post-MVP
- H2.7: Temperature quantization (fp16/PQ storage) -- post-MVP
- H2.8: WASM micro-HNSW (8KB budget) -- post-MVP

### `h3-timestamps/` -- H3: Timestamp Standardization (Week 4-5)

- Replace all i64 ms and Option<String> timestamps with DateTime<Utc>
- Affects clawft-types and all downstream crates
