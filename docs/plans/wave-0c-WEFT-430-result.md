# WEFT-430 result — honest affordance ∩ permit in compose

**Ticket:** WEFT-430  
**Branch:** `wave0c/weft-430-affordance-permit`  
**Base:** `release/0.8-staging`  
**Commit:** `81306e0b`  
**Date:** 2026-07-30  
**Agent:** coder-430 (wave-0c)

## Problem

`compose::honest_affordances` was identity passthrough. Surfaces showed
every declared `AffordanceDecl` even when the app/session had no permit
to perform the verb (ADR-006 rule 2). WEFT-429 (Gate) and WEFT-596 (ACLs)
landed the permit/grant primitives; this ticket wires the **compose-time
intersection** so UX honesty matches governance.

## What composed

### `ComposePermits` (new)

| Field | Role |
|-------|------|
| `influences: Option<BTreeSet<String>>` | Manifest write-side verbs (`None` = open / legacy) |
| `grants: Vec<Permission>` | ADR-012 session capability grants |

Constructors: `open()`, `closed()`, `from_influences()`, `from_manifest()`,
`with_grants()`. Verbs are normalized (`rpc.` stripped) before matching.

### `honest_affordances(node, raw, permits)`

Filters `raw` to affordances where:

1. **Influences** — if `Some(set)`, bare verb must be in the set.
2. **Capture grants** — verbs that name mic/camera/screen capture also
   require a matching grant via `permission_covered` (WEFT-429 helper).

### Compose path

- `compose(...)` → open permits (tests / chip chrome).
- `compose_with_permits(..., permits)` → production path.
- `Frame` carries `permits`; every affordance render/dispatch site uses
  `frame.affordances(node)` (honest filter). Denied verbs are not drawn
  and cannot dispatch.

### Desktop admin

`render_selected_app` builds `ComposePermits::from_manifest` from the
selected app's registry entry and composes with permits.

### Fixture alignment

| Change | Why |
|--------|-----|
| `rpc.kernel.kill` → `rpc.kernel.kill-process` | Match daemon + influences |
| Admin influences + `kernel.restart` | Keep confirm-restart modal visible under honest ∩ |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Implement actual intersection of declared affordances and permission grants | **Done** |
| Surfaces hide affordances the user/app cannot perform | **Done** — UI + dispatch gated |
| Tests cover allow/deny intersections | **Done** — 8 unit tests |

## Tests

```bash
scripts/build.sh check
cargo test -p clawft-gui-egui --lib surface_host::compose::tests
cargo test -p clawft-gui-egui --test admin_app_e2e --test surface_headless_render --test compose_extra_iris --test chip_surfaces
cargo test -p clawft-app --lib manifest::
cargo test -p clawft-surface --test roundtrip --test weftos_admin_builder
```

- **check:** pass  
- **unit (honest_affordances):** 8 passed  
- **integration (gui-egui surface suite):** pass  
- **fixture / manifest:** pass  

## Residual / follow-ups

1. **WEFT-277** — GEPA / goal-aggregate (ADR-008) still deferred; this
   ticket is verb+grant honesty only.
2. **Session input modality** (ADR-019 invocation filter) not yet
   applied at compose; `invocations` remain declarative only.
3. **Existing on-disk registries** may still hold pre-0.8 admin
   influences without `kernel.restart` until re-install / upgrade.
4. **ACL path intersection** (WEFT-596) remains on substrate read egress;
   not mixed into affordance verbs here.

## Files

- `crates/clawft-gui-egui/src/surface_host/compose.rs`
- `crates/clawft-gui-egui/src/surface_host/mod.rs`
- `crates/clawft-gui-egui/src/surface_host/test_harness.rs`
- `crates/clawft-gui-egui/src/shell/desktop.rs`
- `crates/clawft-gui-egui/tests/surface_headless_render.rs`
- `crates/clawft-surface/fixtures/weftos-admin-desktop.toml`
- `crates/clawft-surface/src/tree.rs`
- `crates/clawft-surface/tests/roundtrip.rs`
- `crates/clawft-surface/tests/weftos_admin_builder.rs`
- `crates/clawft-app/fixtures/weftos-admin.toml`
- `crates/clawft-app/src/manifest.rs`
- `docs/plans/wave-0c-WEFT-430-result.md` (this file)
