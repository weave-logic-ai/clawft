# Contributing to WeftOS

## Agent / MetaHarness

- Run `scripts/metaharness/run-task.sh gate` before claiming release readiness.
- Plane is the tracker: `scripts/plane-dag.sh ready --cycle 0.8.x`.
- Fusion ViewSpecs: `scripts/metaharness/validate-views.sh` (evaluate-only).
- Policy changes: MetaHarness flywheel / ADR-096 / ADR-097 — no silent promote.
- See `.metaharness/README.md` and `.grok/rules/metaharness.md`.

## Build

Use `scripts/build.sh` for build, test, check, clippy, and gate (not raw cargo).
