# WEFT-68 result — WASM per-plugin fuel/memory observability

**Ticket:** WEFT-68  
**Branch:** `wave0f/weft-68-plugin-observability`  
**Base:** `release/0.8-staging`  
**Plane id:** `9220592f-93e6-492b-a1cb-87eb4626b805`  
**Date:** 2026-07-30  
**Status:** implemented

## Summary

Fuel and memory limits were already enforced by `clawft-wasm::engine`, but there
was no observability surface for plugin authors to tune limits empirically.
WEFT-68 adds:

1. **Per-plugin aggregate counters** (`PluginMetrics`) with a **pinned JSON shape**
2. **Chain-event emission** on each invocation (`plugin.wasm.invoke`)
3. **`weft plugins inspect <name>`** showing fuel + memory + invocation count
4. **Tests** that pin the metric shape end-to-end

## Acceptance criteria

| AC | Status | How |
|----|--------|-----|
| Per-plugin fuel-consumption counter via `chain_event!` or admin RPC | **Done** | `PluginMetricsRegistry::record` emits `tracing` on `chain_event` target with kind `plugin.wasm.invoke`; aggregates live in process + disk |
| `weft plugins inspect <name>` shows aggregate fuel + memory + invocation count | **Done** | New CLI subcommand; table + `--json` |
| Test pins the metric shape | **Done** | `metrics::tests::metric_shape_is_pinned` + CLI + engine integration tests |

## Metric shape (pinned)

```json
{
  "plugin_name": "string",
  "fuel_consumed_total": 0,
  "memory_peak_bytes": 0,
  "invocation_count": 0,
  "last_fuel_consumed": 0,
  "last_memory_peak_bytes": 0,
  "last_duration_ms": 0,
  "total_duration_ms": 0
}
```

Changing these keys is a breaking change for CLI / admin consumers.

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-plugin/src/metrics.rs` | **New** — `PluginMetrics`, registry, disk I/O, chain-event emit, shape-pin tests |
| `crates/clawft-plugin/src/lib.rs` | Export metrics module |
| `crates/clawft-wasm/src/engine.rs` | `memory_peak` on result; `finish_execution` → `record_plugin_invocation`; WEFT-68 test |
| `crates/clawft-cli/src/commands/plugins_cmd.rs` | `plugins inspect <name> [--json]` |
| `crates/clawft-core/src/chain_event.rs` | `EVENT_KIND_PLUGIN_WASM_INVOKE` constant |

## Persistence

Snapshots written to `~/.clawft/plugins/metrics/<sanitized-name>.json` after each
invocation (best-effort; warn on I/O failure). CLI reads that path first, then
the in-process global registry, then a zeroed shape.

## How to test

```bash
# Shape pin + aggregation unit tests
scripts/build.sh test clawft-plugin
# or: cargo test -p clawft-plugin metrics

# Engine records metrics on execute_tool
cargo test -p clawft-wasm --features wasm-plugins weft68

# CLI inspect shape
scripts/build.sh test clawft-cli
# or: cargo test -p clawft-cli inspect_renders

# Manual smoke
cargo run -p clawft-cli -- plugins inspect <name>
cargo run -p clawft-cli -- plugins inspect <name> --json
```

## Notes

- Pre-existing `clawft-wasm` FS sandbox tests (`dispatcher_read_file_within_sandbox`,
  `t37_audit_all_host_functions`, etc.) still fail under `wasm-plugins` in this
  tree; unrelated to WEFT-68 (fuel/metrics path tests pass).
- No push (per wave instructions). Lead merges from worktree branch.
