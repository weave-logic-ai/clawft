# Graph ViewSpec fixtures

Versioned **fusion / multi-source View** policies for WeftOS.

| File | Purpose |
|------|---------|
| `room-12-identity.yaml` | Sample room identity fusion View (Graph Views F1–F10) |

## Rules (ADR-096 / ADR-097)

1. Fixtures are evaluate baselines; production champions need flywheel **promote**.
2. Do not auto-promote from CI.
3. Validate: `scripts/metaharness/validate-views.sh`
4. Anchors: `.metaharness/eval/`
