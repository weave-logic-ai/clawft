# WEFT-136 result — persist AppManager state to disk

**Branch:** `wave0e/weft-136-appmanager-persist`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef5-7951-8346-377af905efcb`  
**Base:** `release/0.8-staging`  
**Ticket:** ws02: kernel — persist AppManager state to disk

## Gap

Apps installed during a daemon session were lost on restart. `AppManager`
kept state only in a `DashMap` with no on-disk manifest store.

## Fix

Mirrored the `ClusterMembership` / `cluster_peers.json` pattern:

| Piece | Detail |
|-------|--------|
| Store path | `.weftos/runtime/apps.json` (`DEFAULT_APPS_PERSIST_PATH`) |
| Format | `AppsFile { version, apps: Vec<InstalledApp> }` |
| Atomic write | write sibling `apps.json.tmp` → `rename` to `apps.json` |
| Mutations that persist | `install`, `remove`, `transition_to` (covers start/stop) |
| Rehydrate | `AppManager::with_persist_path` loads file at construction |
| Boot wiring | `Kernel::boot` builds `AppManager` with persist path; `Kernel::app_manager()` accessor |

On rehydrate:

- Agent PIDs and service names are cleared (processes do not survive restart).
- Transient states `Starting` / `Running` / `Stopping` normalize to `Stopped`
  so apps can be restarted cleanly.

### Files

- `crates/clawft-kernel/src/app.rs` — `AppsFile`, `with_persist_path`, `persist`, tests
- `crates/clawft-kernel/src/boot.rs` — boot rehydrate + `app_manager` field/accessor
- `crates/clawft-kernel/src/lib.rs` — re-exports `AppsFile`, `DEFAULT_APPS_PERSIST_PATH`

## Tests

```text
cargo nextest run -p clawft-kernel --lib app::tests
# 46 passed (includes 4 new persistence tests)

# Focused AC tests:
# - persisted_apps_rehydrate_on_restart  (install → drop → rehydrate → present)
# - persist_uses_atomic_tmp_rename
# - persist_reflects_remove_and_state_change
# - with_persist_path_missing_file_is_ok

scripts/build.sh check
# ok
```

## Acceptance

| Criterion | Status |
|-----------|--------|
| Manifest store persists installed apps | Yes (`apps.json` after install) |
| Boot rehydrates AppManager from disk | Yes (`with_persist_path` in `Kernel::boot`) |
| Atomic write (tmp + rename) | Yes |
| Test: install → restart/rehydrate → present | Yes (`persisted_apps_rehydrate_on_restart`) |

## How to verify

```bash
cargo nextest run -p clawft-kernel --lib \
  persisted_apps_rehydrate_on_restart \
  persist_uses_atomic_tmp_rename \
  persist_reflects_remove_and_state_change \
  with_persist_path_missing_file_is_ok

scripts/build.sh check
# or: scripts/build.sh test clawft-kernel
```
