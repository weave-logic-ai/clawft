# WeftOS × MetaHarness

Foundation for **agentic development**, **fusion policy**, and **all data-surface
governance** (ADR-096, ADR-097). Not required to run the `weft` daemon.

**Plane:** WEFT-724 (epic), WEFT-725 tasks, WEFT-726 patterns, WEFT-727 ViewSpecs,
WEFT-728 universal surfaces.

## Commands

```bash
scripts/metaharness/score.sh                 # upstream ADR-041 scorecard
scripts/metaharness/weftos-score.sh          # WeftOS foundation asset score
scripts/metaharness/run-task.sh gate         # scripts/build.sh gate
scripts/metaharness/run-task.sh plane-dag    # plane-dag ready
scripts/metaharness/run-task.sh fusion-view  # validate ViewSpecs + anchors
scripts/metaharness/validate-views.sh
scripts/metaharness/seed-patterns.sh         # AgentDB patterns
scripts/metaharness/flywheel-status.sh
```

## Layout

| Path | Role |
|------|------|
| `tasks/` | Harness task specs (gate, plane-dag, fusion-view) |
| `commands/` | Human/agent command cards |
| `eval/` | Flywheel anchors (evaluate_only) |
| `weftos/surfaces.yaml` | ADR-097 surface inventory |
| `weftos/views/` | Candidate ViewSpec staging |
| `patterns-manifest.md` | AgentDB pattern keys |
| `config/views/` (repo root) | Champion ViewSpec fixtures |

## Rules

- Evaluate → receipt → promote (confirm + keys). Never silent champion swap.
- **State** (new sensor samples, files) flows freely under ACL.
- **Policy** (ViewSpec, retention, substrate denylist, soft-edge thresholds)
  is promote-gated (ADR-097).
- Grok: `.grok/rules/metaharness.md`

## Do not

- Commit private promote keys.
- Make Rust crates depend on this directory.
