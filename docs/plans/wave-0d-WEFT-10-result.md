# WEFT-10 result — bootstrap split workspace/global for PermissionResolver ceiling

**Ticket:** WEFT-10  
**Branch:** `wave0d/weft-10-workspace-ceiling`  
**Base:** `release/0.8-staging`  
**Commit:** `49236f19`  
**Date:** 2026-07-30  
**Agent:** coder-10 (wave-0d)

## Problem

`load_config_raw` deep-merged cwd `.clawft/config.json` into a single
`Config`, so bootstrap called `PermissionResolver::new(routing, None)`.
`enforce_workspace_ceiling` never ran. Workspace overlays could elevate
level / tools / budgets past system-wide bounds — fine for single-user
kernels, blocks multi-tenant.

## What shipped

### Loader split (`clawft-platform`)

| API | Role |
|-----|------|
| `ConfigLayers { global, workspace }` | Split layers from discovery |
| `load_config_layers(fs, env)` | Primary: weave.toml + home JSON in `global`; cwd overlay in `workspace` |
| `ConfigLayers::merged()` | Legacy deep-merge for non-security fields |
| `load_config_raw` | Thin wrapper → `layers.merged()` (back-compat) |

### Typed split (`clawft-core` workspace)

| API | Role |
|-----|------|
| `SplitConfig { global, workspace, merged }` | Typed layers |
| `load_split_config` / `load_split_config_from` | Defaults + global file + workspace path |
| `load_merged_config*` | Now delegates to split, returns `.merged` |

### PermissionResolver

- Workspace user/channel overlays apply at resolve time
- Ceiling baseline uses **global-only** user/channel overrides
- `has_workspace` gates `enforce_workspace_ceiling`
- Clamp produces audit reasons; `tracing::warn!` on silent clamp
- Public helper `PermissionResolver::clamp_to_ceiling` for tests

### Bootstrap / daemon wiring

- `AppContext::with_routing_layers(global, workspace)`
- `into_agent_loop` → `PermissionResolver::new(global, workspace)`
- `build_daemon_agent_loop(..., routing, workspace_routing, graft)`
- Daemon `run` takes split layers from `load_config_layered`
- CLI agent / gateway / ui use layered load + `with_routing_layers`

### CLI / weave load helpers

`LoadedConfig { config, global_routing, workspace_routing }` via
`load_config_layered`; `load_config` remains merged-only back-compat.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Loader returns `(global, Option<workspace>)` or equivalent | **Done** — `ConfigLayers` / `SplitConfig` / `LoadedConfig` |
| bootstrap calls `PermissionResolver::new(global, Some(workspace))` | **Done** |
| `enforce_workspace_ceiling` clamps per FIX-04 (level, tools, budget, rate) | **Done** |
| Test: elevated workspace perms silently clamped + audit warning | **Done** — warn! + `clamp_to_ceiling` reasons |
| Multi-tenant scenario integration test | **Done** — `test_multi_tenant_workspace_ceiling_isolation` + split-config e2e |

## Tests

```bash
scripts/build.sh check
cargo test -p clawft-platform --lib config_loader
cargo test -p clawft-core --lib pipeline::permissions::tests
cargo test -p clawft-core --lib load_split
cargo test -p clawft-core --lib with_routing_layers
```

- **check:** pass  
- **config_loader:** 22 passed (incl. split layer tests)  
- **permissions:** 34 passed (incl. clamp, multi-tenant, rate-limit)  
- **split config e2e:** pass  
- **bootstrap wiring:** pass  

## Files

- `crates/clawft-platform/src/config_loader.rs`
- `crates/clawft-core/src/workspace/config.rs`
- `crates/clawft-core/src/workspace/mod.rs`
- `crates/clawft-core/src/pipeline/permissions.rs`
- `crates/clawft-core/src/bootstrap.rs`
- `crates/clawft-cli/src/commands/mod.rs`
- `crates/clawft-cli/src/commands/agent.rs`
- `crates/clawft-cli/src/commands/gateway.rs`
- `crates/clawft-cli/src/commands/ui_cmd.rs`
- `crates/clawft-weave/src/commands/mod.rs`
- `crates/clawft-weave/src/commands/kernel_cmd.rs`
- `crates/clawft-weave/src/daemon.rs`
- `docs/plans/wave-0d-WEFT-10-result.md`

## Residual / follow-ups

1. Cross-listed **ws06 MW-2** is the same root cause — close/link when
   integrating; no second implementation needed.
2. Explicit `--config` override is treated as pure global (no workspace
   split) — intentional; operators can still place workspace overlay
   at cwd.
3. `TODO(v1.1)` removed from `bootstrap.rs` daemon path.
