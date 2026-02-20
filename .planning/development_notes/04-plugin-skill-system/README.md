# Development Notes: Element 04 -- Plugin & Skill System

## Structure

Each phase (C1-C7 + security) has four tracking files:

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

**Item**: [C1/C2/C3/etc.]
**Severity**: [Critical/High/Medium]
**Description**: What is blocking
**Attempted**: What has been tried
**Needs**: What is required to unblock
**Status**: [Active/Resolved]
```

### Difficult Task Records

```
## [DATE] Difficult: [TITLE]

**Item**: [C1/C2/C3/etc.]
**Difficulty**: [High/Very High]
**Why**: What makes this hard
**Approach**: Strategy being used
**Findings**: What was discovered during implementation
```

## Phase Directories

- `c1-plugin-traits/` -- C1: Plugin trait crate (clawft-plugin)
- `c2-wasm-host/` -- C2: WASM plugin host (wasmtime, WIT, security sandbox)
- `c3-skill-loader/` -- C3: Skill loader (serde_yaml, discovery, auto-registration)
- `c4-hot-reload/` -- C4/C4a: Hot-reload, dynamic loading, autonomous creation
- `c5c6-commands-mcp/` -- C5+C6: Slash-command framework and MCP skill exposure
- `c7-pluginhost/` -- C7: PluginHost unification and channel migration
- `security/` -- Cross-cutting security concerns (WASM sandbox, permissions, audit)
