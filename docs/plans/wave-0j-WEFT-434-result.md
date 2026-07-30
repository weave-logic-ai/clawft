# WEFT-434 result — substrate-rpc streaming log endpoint

**Ticket:** WEFT-434  
**Branch:** `wave0j/weft-434-stream-log`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-434 (wave-0j)

## Problem

The kernel adapter’s `substrate/kernel/logs` topic declared
`RefreshHint::EventDriven` but runtime still polled `kernel.logs` every
1s and diffed tail windows (`diff_tail` option-2). No streaming log RPC
existed, so declared intent ≠ runtime.

## What shipped

### Daemon: `kernel.logs_stream`

| Piece | Detail |
|-------|--------|
| Method | `kernel.logs_stream` (streaming subscribe, same pattern as `ipc.subscribe_stream` / `substrate.subscribe`) |
| Capability | `Read` (alongside `kernel.logs`) |
| Params | `LogsParams` — `count` (initial tail; 0 = stream-only), optional `level` filter |
| Ack | `{ streaming: true, subscriber_id, initial_count }` |
| Frames | Line-delimited `LogEntry` JSON (not `Response` envelopes) |
| Cleanup | Unsubscribe on client disconnect |

### Kernel event log: seq + subscribe

| Piece | Detail |
|-------|--------|
| `BootEvent::seq` | Monotonic `u64` assigned on ingest (starts at 1) |
| `KernelEventLog::subscribe` | Live fan-out via bounded `std::sync::mpsc` (wasm-safe) |
| Wire `LogEntry.seq` | Present on `kernel.logs` and stream frames |

### RPC client

`DaemonClient::open_stream` + `StreamSession::next_line` — send request,
read ack, then pump frames. Consumes the client.

### Kernel adapter

| Before | After |
|--------|-------|
| 1s poll of `kernel.logs` + `diff_tail` | `kernel.logs_stream` pump |
| Value watermark | Monotonic `seq` de-dupe across reconnect |
| Periodic fallback | Reconnect backoff only (connection resilience) |

`diff_tail` kept under `#[cfg(test)]` for historical option-2 coverage.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Daemon exposes a streaming log RPC | **Done** — `kernel.logs_stream` |
| Kernel adapter drops the periodic poll fallback | **Done** — `stream_logs` |
| Optional: monotonic `seq: u64` per entry | **Done** — option-1 |
| Tests cover the streaming path | **Done** |

## Tests

```bash
scripts/build.sh check
cargo test -p clawft-kernel --lib console::
cargo test -p clawft-substrate --lib kernel::
cargo test -p clawft-weave --test kernel_logs_stream
```

- **check:** pass (workspace + kernel wasm no-default-features)
- **console (seq + subscribe):** 16 passed
- **kernel adapter:** 23 passed
- **kernel_logs_stream integration:** 1 passed

## Files

- `crates/clawft-kernel/src/console.rs`
- `crates/clawft-rpc/src/{client,lib}.rs`
- `crates/clawft-weave/src/{daemon,protocol,capability}.rs`
- `crates/clawft-weave/tests/kernel_logs_stream.rs`
- `crates/clawft-substrate/src/kernel.rs`
- `docs/plans/wave-0j-WEFT-434-result.md`
