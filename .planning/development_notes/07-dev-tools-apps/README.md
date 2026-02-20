# Development Notes: Element 07 -- Dev Tools & Applications

## Structure

Three phase directories corresponding to the execution phases in Element 07:

### `f-core/` -- Core Dev Tools (Week 5-7)

Items: F1 (Git), F2 (Cargo), F6 (OAuth2, P0), F9a (MCP Client MVP, P0)

### `f-advanced/` -- Advanced Tools (Week 6-9)

Items: F3 (Tree-sitter), F4 (Browser CDP), F5 (Calendar), F7 (Docker/Podman)

### `f-mcp/` -- MCP Ecosystem (Week 8-10)

Items: F8 (MCP IDE Integration), F9b (Full MCP Client)

## Files Per Directory

Each phase directory contains four tracking files:

- `decisions.md` -- Key design decisions with rationale
- `blockers.md` -- Active blockers requiring escalation
- `difficult-tasks.md` -- Tasks needing extra attention or research
- `notes.md` -- General development notes, findings, tips

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

**Item**: [F1/F6/F9a/etc.]
**Severity**: [Critical/High/Medium]
**Description**: What is blocking
**Attempted**: What has been tried
**Needs**: What is required to unblock
**Status**: [Active/Resolved]
```

### Difficult Task Records

```
## [DATE] Difficult: [TITLE]

**Item**: [F4/F9b/etc.]
**Difficulty**: [High/Very High]
**Why**: What makes this hard
**Approach**: Strategy being used
**Findings**: What was discovered during implementation
```
