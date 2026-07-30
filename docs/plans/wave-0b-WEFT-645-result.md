# WEFT-645 result — hermetic clawft-rpc no-daemon client tests

**Status:** done  
**Branch:** `wave0b/weft-645-hermetic-rpc-tests`  
**Ticket:** make clawft-rpc no-daemon client tests hermetic (fail when a live daemon is up)

## Problem

`client::tests::connect_returns_none_when_no_daemon` and
`client::tests::is_daemon_running_false_when_no_daemon` probed the **default**
daemon socket (`protocol::socket_path()` → project `.weftos/runtime/` or
`~/.clawft/kernel.sock`). On any machine with a live weaver daemon those
assertions fail, fail-fasting full workspace test runs and masking the rest
of the suite.

## Approach

1. **`DaemonClient::connect_path(path)`** — connect to an explicit UDS path
   (Unix). Non-Unix stub still returns `None`.
2. **`is_daemon_running_at(path)`** — same probe, path-parameterized.
3. **`connect()` / `is_daemon_running()`** keep the default-path behavior for
   production callers; both now route through the path-based helpers.
4. **Tests** use `tempfile::tempdir()` + `SOCKET_NAME` so nothing listens and
   a live developer daemon cannot affect the result. No env-var mutation
   (`WEFTOS_RUNTIME_DIR`), so parallel tests stay race-free.

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-rpc/src/client.rs` | `connect_path`, `is_daemon_running_at`; hermetic tests |
| `crates/clawft-rpc/src/lib.rs` | re-export `is_daemon_running_at` |
| `crates/clawft-rpc/Cargo.toml` | `tempfile` dev-dependency |

## Verification

```text
cargo test -p clawft-rpc
# 13 passed; 0 failed

scripts/build.sh check
# (workspace compile check)
```

## Acceptance criteria

- [x] Both tests point at an isolated runtime dir / socket path (temp dir)
- [x] Live daemon on the shared default path cannot flip the result
- [x] `cargo test -p clawft-rpc` green
- [x] `scripts/build.sh check` green
