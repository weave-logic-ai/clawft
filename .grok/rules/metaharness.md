# MetaHarness on Grok (WeftOS)

MetaHarness is **foundational for agentic development and fusion-policy
evolution** (ADR-096 draft). The `weft` runtime must still run without it.

## Doctrine

> Freeze the model. Evolve the harness. Promote only what proves lift.

- **Grok** executes (edit, shell, tests, subagents).  
- **Ruflo / claude-flow MCP** coordinates memory, score, flywheel when available.  
- **MetaHarness** scores readiness, routes with receipts, evaluates candidates,
  promotes with explicit confirm — never silent champion swaps.

## When to invoke

| Situation | Action |
|-----------|--------|
| Starting multi-file / fusion / harness work | `metaharness score` or `claude-flow__metaharness_score` if MCP up |
| Changing Graph View / fusion attach / promote gates | Prefer flywheel evaluate → receipt → promote; or document manual waiver in PR |
| Cost / model routing questions | Use savings / routing receipts skill patterns — never invent $ numbers |
| After a successful fusion or release pattern | `memory_store` namespace `patterns` so next host (Claude/Grok) can recall |
| Kernel / ECC / LeWM authority | **Do not** evolve policies that violate ADR-090 R1–R5 |

## Constraints (ADR-150 / ADR-096)

1. Removable — core `scripts/build.sh` green without MH.  
2. Optional — never add MH as a link requirement for Rust crates.  
3. Graceful degrade — if MCP/CLI missing, say so and continue.  
4. No auto-promote — `confirm=true` + keys or human merge only.

## Related docs

- `docs/adr/adr-096-metaharness-foundation.md`  
- `docs/adr/adr-098-environment-process-compose.md`  
- `docs/guides/agent-harness-triple-loop.md`  
- `docs/research/metaharness-foundation.md`  
- `docs/research/graph-views.md` (fusion operational model)  
- `docs/research/ruv-worldgraph-vs-weftos.md`  
- `.grok/rules/ruflo-grok.md`  
