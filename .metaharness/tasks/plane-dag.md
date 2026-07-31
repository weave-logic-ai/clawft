---
id: weft-plane-dag
title: Plane DAG ready inventory (cycle-aware)
plane: WEFT-725
command: scripts/plane-dag.sh ready --cycle 0.8.x
timeout_min: 5
surfaces: [plane, governance]
---

# Task: weft-plane-dag

Refresh readiness of the Plane dependency DAG for agent claim selection.

## Steps

1. `export PLANE_API_KEY=…` (from env; never commit)
2. `scripts/plane-dag.sh refresh` (optional full rebuild)
3. `scripts/plane-dag.sh ready --cycle 0.8.x --limit 30`
4. Optionally `scripts/plane-dag.sh ready --cycle 1.0.x --limit 20`

## Success

- Exit 0; ready list printed
- Agents claim only wave-0 tickets with AC

## Governance

Cycle taxonomy and claim rules are governance surfaces (ADR-097). Changing
priority policy is promote-gated.
