---
id: weft-gate
title: Full phase gate (build/check/clippy/test)
plane: WEFT-725
command: scripts/build.sh gate
timeout_min: 45
surfaces: [workspace, ci]
---

# Task: weft-gate

Run the authoritative WeftOS phase gate before commit/release claims.

## Steps

1. `scripts/build.sh gate`
2. Capture exit code; on failure, do not promote harness or ViewSpec changes.
3. Optionally store outcome: `scripts/metaharness/seed-patterns.sh --only gate-result`

## Success

- Exit 0 from `scripts/build.sh gate`
- No new secrets in tree

## Governance

Policy changes that relax gate steps require MetaHarness flywheel receipt + promote (ADR-096/097).
