# npm audit residual risk (WEFT-598)

Last triage: 2026-07-31 on `release/0.8-staging` (branch `fix/weft-598-npm-audit`).

## Gate policy

| Surface | Command | Fail threshold |
|---------|---------|----------------|
| Local / CI | `scripts/build.sh npm-audit` | critical + high |
| Phase gate | check #13 (`gate`) | same |
| CI job | `.github/workflows/pr-gates.yml` → `npm-audit` | same |

Override with `NPM_AUDIT_LEVEL` (`critical` \| `high` \| `moderate` \| …) or
`NPM_AUDIT_SOFT=1` (report only).

Audited lockfiles (when present): `clawft-ui/`, repo root, `docs/src/`, `gui/`.

## Post-triage scores

| Lockfile | Critical | High | Moderate | Low | Notes |
|----------|----------|------|----------|-----|-------|
| `clawft-ui/` | 0 | 0 | 0 | 0 | Clean after vite/playwright bumps + overrides |
| root | 0 | 0 | ~29 | 0 | Residual under ruflo pin (see below) |
| `docs/src/` | 0 | 0 | 0 | 0 | sharp/postcss overrides |
| `gui/` | 0 | 0 | 0 | 0 | esbuild + minimatch overrides |

### Approximate fix counts (critical + high)

| Surface | Before (crit/high) | After | Fixed |
|---------|--------------------|-------|-------|
| clawft-ui | 1 / 6 | 0 / 0 | **7** |
| root | 1 / 13 | 0 / 0 | **14** |
| docs/src | 0 / 4 | 0 / 0 | **4** |
| gui | 0 / 5 | 0 / 0 | **5** |
| **Total crit+high cleared** | **~30** | **0** | **~30** |

(Exact Dependabot “142” total mixed severities/surfaces; this triage focused
critical + high on the product npm trees.)

## What was fixed (non-breaking)

### clawft-ui

- `vite` → `^7.3.6` (path traversal / dev-server highs)
- `@playwright/test` → `^1.62.1` (browser download cert verify)
- `npm audit fix` for `seroval` (critical), `postcss`, `js-yaml`
- Overrides: `brace-expansion@5.0.9`, `minimatch@^9.0.5` (eslint chain DoS)

### root (`package.json`)

Overrides (keep `ruflo` / `@claude-flow/cli` pin **3.32.38**):

| Package | Pin | Reason |
|---------|-----|--------|
| `protobufjs` | `7.6.5` | Critical RCE / nested onnx-proto 6.x |
| `undici` | `7.29.0` | High WebSocket / header issues |
| `adm-zip` | `0.6.0` | High memory allocation |
| `sharp` | `0.35.3` | High libvips CVEs |
| `@opentelemetry/propagator-jaeger` | `2.9.0` | High DoS on malformed header |

### docs/src

- `postcss` → `^8.5.25`
- Override `sharp@0.35.3`

### gui

- `esbuild` → `^0.28.1`
- Overrides: `brace-expansion`, `minimatch`, `esbuild`

## Accepted residual risk (root moderates)

~29 **moderate** findings remain on the root lockfile, almost entirely the
OpenTelemetry resources/SDK chain pulled by:

- `agentdb` → `@opentelemetry/*`
- `@claude-flow/cli@3.32.38` / `ruflo@3.32.38` (schema pin — **WEFT-684 / WEFT-669**)
- `agentic-flow@2.x`

### Why not force-fixed

1. **Ruflo pin is load-bearing** for `.swarm/agentdb-memory.db` schema ownership.
   Bumping `@claude-flow/cli` / `ruflo` outside the deliberate pin process risks
   silent AgentDB corruption.
2. npm `audit fix --force` proposes **major downgrades** of `agentic-flow` to
   `1.10.2`, which is wrong-direction and breaks the 2.x integration.
3. Moderates are DoS / resource issues in OTEL exporters used by **dev-time
   agent tooling**, not the shipped Rust daemon or clawft-ui production bundle.

### When to clear

- Next deliberate ruflo pin bump (see `package.json` → `weftos.rufloPinNote`).
- Track as follow-up if OTEL moderates are reclassified high, or if agent
  tooling is exposed on a network boundary.

## How to re-run

```bash
# All product npm trees, fail on ≥high
scripts/build.sh npm-audit

# Soft report
NPM_AUDIT_SOFT=1 scripts/build.sh npm-audit

# Single tree
(cd clawft-ui && npm audit --audit-level=high)
(cd . && npm audit --audit-level=high)
```

## UI build sanity

After bumps, `clawft-ui` must still build:

```bash
scripts/build.sh ui
# or: (cd clawft-ui && npm run build)
```
