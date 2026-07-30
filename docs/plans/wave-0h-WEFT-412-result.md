# Wave 0h — WEFT-412 result

**Ticket:** WEFT-412 — clawft-app emit lifecycle teardown tombstone on uninstall-while-enabled (ADR-015)  
**Branch:** `wave0h/weft-412-uninstall-tombstone`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bef-7d73-b086-57786eb505fe`  
**Status:** implemented  
**Pushed:** no (lead merge)

## Acceptance criteria

| AC | Status |
|----|--------|
| Uninstall on an enabled app records a tombstone event/result | Done — `UninstallResult.teardown_tombstone` + bus log |
| Compositor consumers can subscribe to the tombstone | Done — `TeardownTombstoneBus::subscribe` → `mpsc::Receiver` |
| Test exercises uninstall-while-enabled and confirms tombstone emission | Done — registry + lifecycle unit tests |

## What shipped

### `LifecycleTeardownTombstone` (`lifecycle.rs`)

Serializable payload capturing ADR-015 §Lifecycle terminate work that the
compositor still owes when an **enabled** app is removed before M1.6+
hooks exist:

- `app_id` / `app_name`
- `surface_ids` (manifest surfaces)
- `subscription_topics` (manifest subscriptions)
- `influences` (affordance / WSP verbs to revoke)
- `reason: TeardownReason::UninstallWhileEnabled`
- `recorded_at` (unix secs)

### `TeardownTombstoneBus` (`lifecycle.rs`)

In-process Arc-backed bus:

- **`subscribe()`** — live `mpsc::Receiver` for future tombstones
- **`emit` / `recorded` / `drain` / `len` / `is_empty`** — log + fan-out
- Clones of the registry share the same bus (compositor can hold a
  clone without a second source of truth)
- **Not** persisted to `apps.json`

### `AppRegistry::uninstall` (`registry.rs`)

- Return type: `Result<UninstallResult, RegistryError>`
- **Enabled** → build tombstone, `teardown_bus.emit`, return `Some`
- **Disabled** → no tombstone (`None`), bus untouched
- `teardown_bus()` accessor for consumers

### Public re-exports (`lib.rs`)

`LifecycleTeardownTombstone`, `TeardownReason`, `TeardownTombstoneBus`,
`UninstallResult`.

## Compositor integration note (M1.6+)

```rust
let rx = desk.app_registry.teardown_bus().subscribe();
// … later, on uninstall of an enabled app …
// rx receives LifecycleTeardownTombstone; tear down surfaces,
// unsubscribe topics, revoke influences per ADR-015 §Lifecycle.
```

Or poll `registry.teardown_bus().drain()` on a shell tick.

## How to test

```bash
scripts/build.sh test clawft-app --verbose
# Key tests:
#   registry::tests::uninstall_while_enabled_emits_teardown_tombstone
#   registry::tests::uninstall_while_disabled_skips_teardown_tombstone
#   lifecycle::tests::teardown_tombstone_from_manifest_captures_lifecycle_fields
#   lifecycle::tests::teardown_bus_subscribe_and_drain
```

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-app/src/lifecycle.rs` | Tombstone + bus types; unit tests |
| `crates/clawft-app/src/registry.rs` | Uninstall emits tombstone; `UninstallResult`; bus field; AC tests |
| `crates/clawft-app/src/lib.rs` | Re-exports + module docs |
| `docs/plans/wave-0h-WEFT-412-result.md` | This report |

## Tests run

```text
scripts/build.sh test clawft-app --verbose
→ 35 passed, 0 failed
```

## API note

`AppRegistry::uninstall` no longer returns bare `InstalledApp`; callers
should use `result.removed` (and optionally `result.teardown_tombstone`).
In-tree call sites were tests only; no other crate used the old return
type at implementation time.
