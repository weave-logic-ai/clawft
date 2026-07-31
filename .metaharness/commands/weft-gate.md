---
description: Run WeftOS full phase gate (scripts/build.sh gate) — MetaHarness task weft-gate
---

Run the MetaHarness harness task **weft-gate** (WEFT-725).

```bash
scripts/metaharness/run-task.sh gate
```

Or directly: `scripts/build.sh gate`. Do not claim release readiness if this fails.
See `.metaharness/tasks/gate.md`.
