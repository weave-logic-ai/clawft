---
description: Plane DAG ready list for agent claims — MetaHarness task weft-plane-dag
---

Run the MetaHarness harness task **weft-plane-dag** (WEFT-725).

```bash
scripts/metaharness/run-task.sh plane-dag
```

Requires `PLANE_API_KEY`. Prefer `scripts/plane-dag.sh ready --cycle 0.8.x`.
See `.metaharness/tasks/plane-dag.md`.
