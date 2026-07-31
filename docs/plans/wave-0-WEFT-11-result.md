# WEFT-11 result — Windows named-pipe transport for DaemonClient

## Summary

Implemented **client-side** Windows named-pipe transport for
`DaemonClient` in `clawft-rpc`, plus **server listener helpers** that
compile on Windows. Full weave daemon accept-loop wiring remains
residual under **WEFT-559** (documented).

Plane: WEFT-11 (ws01-core / 0.8.x).

## Acceptance

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Named-pipe transport on Windows **or** clear non-support | **Done** — real `NamedPipeClient` impl under `cfg(windows)` |
| 2 | `DaemonClient::connect()` works on Windows (or clear Err) | **Done** — dials pipe / returns `None` if no server; `call` errors only on other platforms |
| 3 | CI matrix note / docs for Windows status | **Done** — `docs/guides/build.md`, cargo-dist comments in root `Cargo.toml` |
| 4 | Update `docs/guides/` | **Done** — kernel.md, build.md, weftos-deferred-requirements.md |

Pragmatic scope (per ticket): full Windows daemon is large → client +
server stubs + unit tests + residual docs, not full `daemon.rs` rewire.

## What shipped

### `crates/clawft-rpc`

| File | Change |
|------|--------|
| `src/protocol.rs` | `PIPE_NAME_PREFIX`, `pipe_name_for_path`, `default_pipe_name`; stable hash isolation for project-local runtimes |
| `src/client.rs` | `cfg(windows)` `DaemonClient` over `tokio::net::windows::named_pipe::ClientOptions`; PIPE_BUSY retry; non-unix/non-windows stub with clear errors |
| `src/named_pipe.rs` | **New** — `create_listener` / `create_listener_next` (+ off-Windows stubs that return `Unsupported`) |
| `src/lib.rs` | Re-exports; Windows-aware `connect_or_bail` message |
| `Cargo.toml` | tokio features `time` + `macros` for retries / tests |

### Docs / matrix

| File | Change |
|------|--------|
| `docs/guides/weftos-deferred-requirements.md` | Windows section → partially landed; residual checklist → WEFT-559 |
| `docs/guides/kernel.md` | New “Windows transport (WEFT-11)” operator table |
| `docs/guides/build.md` | Windows target notes + CI matrix note |
| `Cargo.toml` `[workspace.metadata.dist]` | Updated comment (client landed; dist still off until server) |
| `crates/clawft-weave/.../kernel_cmd.rs` | Clearer non-Unix bail pointing at WEFT-11 / WEFT-559 |

## Tests / build

```text
scripts/build.sh check                          # PASS
scripts/build.sh test clawft-rpc                # 18 passed (Unix host)
```

Notable tests:

- `pipe_name_for_path_*` — stable hash + `\\.\pipe\` namespace (all OS)
- `connect_returns_none_when_no_daemon` — still holds on Windows mapping
- `named_pipe::create_listener_errors_off_windows` — Unix host
- `windows_named_pipe_roundtrip` — `cfg(windows)` only (needs Windows host / future CI job)

## Residual (do not close WEFT-559)

1. Wire `clawft-weave::daemon` accept loop with
   `create_listener` / `create_listener_next` + existing
   `handle_json_connection` / `dispatch_json_line`.
2. Enable `kernel start|stop|restart` on Windows once listening works.
3. Re-enable `x86_64-pc-windows-msvc` in cargo-dist.
4. Add `windows-latest` CI job for `cargo test -p clawft-rpc`.

## How to verify

```bash
scripts/build.sh check
scripts/build.sh test clawft-rpc

# On a Windows host (or cross with msvc toolchain):
cargo test -p clawft-rpc --target x86_64-pc-windows-msvc
```

## Worktree

- Path: this Grok Build worktree under
  `~/.grok/worktrees/mathewbeane-weftos/…`
- Branch: `release/0.8-staging` (tip was ~`76e2cb80` at claim)
- Commit: *not committed by agent* — leave for lead to review/merge

## Plane close text (suggested)

> WEFT-11: Windows named-pipe **client** + server **helpers** in
> clawft-rpc. `DaemonClient::connect_path` maps logical
> `socket_path` → `\\.\pipe\clawft-kernel-<hash>`. Docs updated
> (kernel/build/deferred-requirements). Daemon accept loop residual
> → WEFT-559. `scripts/build.sh check` + `test clawft-rpc` green (18).
