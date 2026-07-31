# WeftOS × MetaHarness

Foundation layer for **agentic development** and **fusion-policy evolution**
(ADR-096). Not required to run the `weft` daemon.

## Commands

```bash
scripts/metaharness/score.sh              # readiness JSON → score-latest.json
scripts/metaharness/flywheel-status.sh    # receipts / MCP pointer
npx metaharness genome .                  # 7-section readiness
npx metaharness analyze .                 # plan only
```

MCP (Ruflo / claude-flow): `metaharness_score`, `metaharness_flywheel`.

## Rules

- Evaluate → receipt → promote (confirm + keys). Never silent champion swap.
- Graph View / fusion policy churn uses the same discipline
  (`docs/research/graph-views.md` §9b).
- Grok: `.grok/rules/metaharness.md`.

## Do not

- Commit private promote keys.
- Make Rust crates depend on this directory.
