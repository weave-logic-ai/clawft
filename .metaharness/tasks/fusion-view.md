---
id: weft-fusion-view
title: Fusion Graph View smoke (fixtures + validate)
plane: WEFT-725
command: scripts/metaharness/validate-views.sh
timeout_min: 5
surfaces: [sensors, bvh, graph-views, fusion]
---

# Task: weft-fusion-view

Smoke-check purpose-scoped fusion ViewSpecs (Graph Views operational model).

## Steps

1. `scripts/metaharness/validate-views.sh`
2. Confirm fixtures under `config/views/` parse and pass schema checks
3. Confirm flywheel anchors exist under `.metaharness/eval/` (evaluate-only)

## Success

- Validate script exit 0
- At least one room/region identity ViewSpec present
- No auto-promote of ViewSpec policy

## Governance

ViewSpec attach/window/promote-gate changes → flywheel evaluate → receipt →
explicit promote (ADR-096 §3, ADR-097).
