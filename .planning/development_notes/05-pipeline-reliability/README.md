# Development Notes: Element 05 -- Pipeline & LLM Reliability

**Element**: 05 - Pipeline & LLM Reliability
**Workstream**: D
**Timeline**: Weeks 2-5
**SPARC Orchestrator**: `.planning/sparc/05-pipeline-reliability/00-orchestrator.md`
**Execution Tracker**: `.planning/sparc/05-pipeline-reliability/04-element-05-tracker.md`

---

## Directory Structure

Each phase directory contains four tracking files:

| File | Purpose |
|------|---------|
| `decisions.md` | Key design decisions with rationale |
| `blockers.md` | Active blockers requiring escalation |
| `difficult-tasks.md` | Tasks needing extra attention or research |
| `notes.md` | General development notes, findings, tips |

## Phase Directories

| Directory | Phase | Items | Week |
|-----------|-------|-------|------|
| `d-perf/` | D-Perf: Performance | D1 (parallel tools), D10 (bootstrap cache), D11 (async file I/O) | 2-3 |
| `d-reliability/` | D-Reliability: Correctness & Resilience | D3 (structured errors), D4 (retry policy), D7 (FnMut callbacks), D2 (streaming failover), D8 (bounded bus) | 3-4 |
| `d-observability/` | D-Observability: Metrics & Cost | D5 (latency recording), D6 (sender_id threading) | 4-5 |
| `d-transport/` | D-Transport: MCP Transport | D9 (MCP concurrency, request-ID multiplexing) | 4-5 |

## Conventions

### Decision Records

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

**Item**: [D1/D3/D9/etc.]
**Severity**: [Critical/High/Medium]
**Description**: What is blocking
**Attempted**: What has been tried
**Needs**: What is required to unblock
**Status**: [Active/Resolved]
```

### Difficult Task Records

```
## [DATE] Difficult: [TITLE]

**Item**: [D1/D2/D9/etc.]
**Difficulty**: [High/Very High]
**Why**: What makes this hard
**Approach**: Strategy being used
**Findings**: What was discovered during implementation
```

## Cross-References

- Master Orchestrator: `.planning/sparc/02-improvements-overview/00-orchestrator.md`
- Element Orchestrator: `.planning/sparc/05-pipeline-reliability/00-orchestrator.md`
- Execution Tracker: `.planning/sparc/05-pipeline-reliability/04-element-05-tracker.md`
- Legacy Notes: `.planning/development_notes/02-improvements-overview/element-05/`
