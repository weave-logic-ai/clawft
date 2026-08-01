# Grok host reference (pathfinder for MetaHarness)

**Mode:** WIRE → UPSTREAM (S1)  
**String:** `SEE → WIRE → BUILD → UPSTREAM`

Upstream MetaHarness ships nine hosts (`claude-code`, `codex`, `opencode`,
`hermes`, `openclaw`, `rvm`, `copilot`, `github-actions`, …). **There is no
`@metaharness/host-grok` yet.** WeftOS runs production agent work as:

```
Grok Build          = executor (edit, shell, tests, subagents)
Ruflo / claude-flow = orchestrator (memory, swarm, team bus, MH tools)
MetaHarness         = score / genome / flywheel promote
WeftOS Rust         = product kernel (no Node required at runtime)
```

This directory is the **in-repo host adapter contract** agents and rUv can read.

## Layout (WeftOS)

| Path | Role |
|------|------|
| `.grok/rules/ruflo-grok.md` | Division of labor + tool map |
| `.grok/rules/metaharness.md` | Flywheel doctrine on Grok |
| `.grok/skills/agent-teams-grok/` | Named team bus |
| `.grok/agents/ruflo-*.md` | Team roles |
| `scripts/grok-team-bus.mjs` | Host-agnostic team bus |
| `docs/adr/adr-075-*.md` | Grok as WeftOS MCP client |
| `docs/adr/adr-076-*.md` | MCP capability catalog |

## Host contract (for future `@metaharness/host-grok`)

If MetaHarness added a Grok host adapter, it should emit:

1. **Rules** — Grok = executor, Ruflo = orchestrator (never wait for Ruflo to code).  
2. **MCP discovery** — `search_tool` then `use_tool` with `ruflo__*` / `claude-flow__*`.  
3. **Memory** — `memory_search` / `memory_store` namespace `patterns` (dual-host).  
4. **Team bus** — not Claude `SendMessage`; team_send / local bus.  
5. **MetaHarness** — score + **genome** + flywheel measure; not scorecard alone.  
6. **String** — `SEE → WIRE → BUILD → UPSTREAM` before inventing features.

## Doctor checklist

```bash
test -f .grok/rules/ruflo-grok.md
test -f .grok/rules/metaharness.md
test -f scripts/grok-team-bus.mjs
node scripts/metaharness/weftos-brain.mjs search "grok ruflo host"
npm run metaharness:loop
```

## Upstream contribution

Package this README + a minimal fixture tree for `ruvnet/metaharness` as
`packages/host-grok` proposal. Until then: **WIRE** is satisfied when agents
hit this path via brain/crosscut; **UPSTREAM** remains open for formal adapter.
