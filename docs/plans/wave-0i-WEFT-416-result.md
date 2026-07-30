# WEFT-416 result — clawft-substrate per-id Replace/Remove deltas on processes/services

**Ticket:** WEFT-416  
**Branch:** `wave0i/weft-416-substrate-deltas`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-416 (wave-0i)

## Problem

`substrate/kernel/processes` and `substrate/kernel/services` emitted a
**whole-list `Replace` every poll tick** (1s / 2s). Steady-state traffic
re-sent the full array even when nothing changed. Per-pid / per-name
deltas were deferred pending a daemon-side row-id contract
(`.planning/reviews/0.7.0-release-gate/13-app-substrate-surface.md` task 7;
`kernel.rs` topic table).

## What shipped

### 1. Daemon row-id contract (`clawft-weave` protocol + handlers)

| Type | Field | Convention |
|------|-------|------------|
| `ProcessInfo` | `row_id: String` (`#[serde(default)]`) | process-table / agent.list → `pid:{pid}`; service-as-process rows in `kernel.ps` → `svc:{name}` |
| `ServiceInfo` | `row_id: String` (`#[serde(default)]`) | equals `name` (unique in registry) |

Helpers: `process_row_id_for_pid`, `process_row_id_for_service`,
`process_row_id_from_fields` (legacy composite fallback).

**Why not bare `pid`?** `kernel.ps` merges registered services as virtual
process rows that share the daemon OS pid — pid is not unique in the
merged list.

### 2. Kernel adapter per-id deltas (`clawft-substrate`)

`poll_processes` / `poll_services` no longer whole-list Replace every tick.
They call pure `diff_keyed_list`:

| Event | Deltas |
|-------|--------|
| New / changed row | `Replace` at `…/by-id/{row_id}` (processes) or `…/by-name/{row_id}` (services) |
| Removed row | `Remove` at the same path |
| Any per-row change | also root-list `Replace` at topic path (table-UI backward compat) |
| Steady state | **zero deltas** |

Paths:

- `substrate/kernel/processes/by-id/{row_id}`
- `substrate/kernel/services/by-name/{row_id}`
- Root arrays still updated on change so `native_live` / ProcessTableViewer /
  surface bindings keep working without a GUI migration.

Legacy daemons without `row_id` fall back to `{pid}:{agent_id}` (processes)
or `name` (services).

### 3. Tests

| Test | Asserts |
|------|---------|
| `diff_keyed_list_first_tick_emits_per_row_and_root` | seed = N by-id + 1 root |
| `diff_keyed_list_steady_state_emits_zero_deltas` | identical payload → `[]` |
| `diff_keyed_list_one_row_change_emits_single_replace_plus_root` | 1 Replace + root; unchanged rows silent |
| `diff_keyed_list_removal_emits_remove_plus_root` | `Remove` + root shrinks |
| `diff_keyed_list_new_row_emits_replace_plus_root` | insert only |
| `diff_keyed_list_steady_state_delta_size_strictly_less_than_full_replace` | 20-row steady → 0; one-row mutate → 2 not 21 |
| row_id / sanitize helpers | contract + path safety |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Daemon RPC grows a row-id contract for processes/services | **Done** — `row_id` on `ProcessInfo` / `ServiceInfo` |
| Adapter emits per-row Replace/Remove deltas | **Done** — `diff_keyed_list` + keyed pollers |
| Tests confirm reduced delta size on steady-state ticks | **Done** — steady state emits 0 deltas |

## Tests / build

```bash
scripts/build.sh test clawft-substrate
scripts/build.sh check
```

- **clawft-substrate:** 164 passed (nextest + mock_adapter)
- **check:** pass (workspace; pre-existing clawft-kernel warnings only)

## Residual / follow-ups

1. **Root list still replaced on any membership/content change** — necessary
   for table UIs that bind `$substrate/kernel/processes` as an array. A
   future surface migration can reassemble from `by-id/*` and drop the
   root Replace entirely (further bandwidth win on partial updates).
2. **Order-only changes** with identical row values do not re-emit the root
   list (BTreeMap-keyed compare). Daemon currently sorts by pid; unlikely
   to matter.
3. **GUI / surface** can later bind individual `by-id` / `by-name` paths
   for incremental widgets; not required for this ticket.

## Files

- `crates/clawft-weave/src/protocol.rs` — row_id fields + helpers
- `crates/clawft-weave/src/daemon.rs` — set row_id in kernel.ps / kernel.services / agent.list
- `crates/clawft-weave/src/commands/leaf_cmd.rs` — ProcessInfo test fixtures
- `crates/clawft-substrate/src/kernel.rs` — diff_keyed_list, pollers, tests
- `docs/plans/wave-0i-WEFT-416-result.md` — this file
