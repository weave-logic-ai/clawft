# ADR-010 v0.3 — Mesh `select!` cancel-correctness audit (WEFT-18)

**Date**: 2026-07-30  
**ADR**: [ADR-010 Keep Tokio](../adr/adr-010-keep-tokio.md)  
**Plane**: WEFT-18  
**Scope**: Mesh networking / foundation runtime `tokio::select!` sites and the
framed I/O they race.

## Why this audit exists

ADR-010 rejected a runtime migration to Asupersync and deferred cancel-
correctness to a **targeted audit of mesh `select!` branches** at v0.3. The
ADR text named `mesh_runtime.rs` and `mesh_heartbeat.rs`; those modules no
longer contain `select!` (heartbeat is synchronous SWIM state; runtime is
primarily request/response + mpsc). The load-bearing production race is the
**bidirectional peer loop** wired in `boot.rs`, which races encrypted
`recv` against outbound mpsc.

## Catalog — every `tokio::select!` in mesh networking paths

| # | Location | Role | Racing futures | Verdict |
|---|----------|------|----------------|---------|
| M1 | `crates/clawft-kernel/src/boot.rs` (~L559) mesh accept peer loop | Inbound encrypted frame vs outbound queue | `channel.recv_encrypted()`, `out_rx.recv()` | **Fixed load-bearing path** — see Finding F1 |
| M2 | `crates/clawft-substrate/src/mesh.rs` (~L153) ontology poller | Cancel vs periodic RPC poll | `oneshot::Receiver`, `Interval::tick` | **OK** — both cancel-safe; lag-on-cancel only |
| — | `mesh_runtime.rs` | Peer map, envelope inject, assessment demux | *(no `select!`)* | N/A — async APIs use mpsc `send`/`recv` at call sites |
| — | `mesh_heartbeat.rs` | SWIM state machine | *(no `select!`, sync)* | N/A |
| — | `mesh_tcp.rs` / `mesh_ws.rs` / `mesh_noise.rs` | Transports | *(no `select!`; used **by** M1)* | Framing cancel-safety is the contract for M1 |
| — | `mesh_listener.rs`, `mesh_process.rs`, `mesh_ipc.rs`, … | Pool / framing helpers | *(no production `select!`)* | N/A |

### Adjacent (not mesh, catalogued for completeness)

Kernel has other `select!` sites (`agent_loop.rs`, `stream_anchor.rs`,
`talk_loop_service.rs`) outside mesh networking. They are **out of WEFT-18
scope** (ADR-010 mesh deliverable).

Substrate adapters (`network.rs`, `presence.rs`, `chain.rs`, `bluetooth.rs`,
`rfkill.rs`, `mic.rs`, `kernel.rs`) use the same cancel+tick pattern as M2;
same verdict (OK). Not mesh wire protocol.

## Cancel-safety checklist (applied per site)

For each racing future *F* in `select!`:

1. **Message integrity** — If *F* is dropped, is a complete application message
   lost, or does the source retain it?
2. **Stream alignment** — If *F* is multi-step I/O (length then body), does a
   drop mid-frame desync subsequent reads?
3. **Side effects** — Does the arm body after a win perform non-idempotent work
   that another concurrent path can observe half-done? (Arm bodies are not
   cancelled by the losing branch.)
4. **Shutdown** — Can the loop exit without leaking peers / half-open crypto?

Reference: [Tokio cancel safety](https://docs.rs/tokio/latest/tokio/macro.select.html#cancellation-safety).

## Findings

### F1 — CRITICAL (fixed): `TcpMeshStream::recv` was not cancel-safe

**Where**: `crates/clawft-kernel/src/mesh_tcp.rs` (pre-fix `read_exact` length
then `read_exact` body).

**How it breaks M1**: In the boot peer loop, when `out_rx.recv()` wins,
`recv_encrypted()` → `MeshStream::recv()` is cancelled. If the old `recv` had
already consumed the 4-byte length (or part of the body), the next loop
iteration re-entered `recv` from a cold start and interpreted payload bytes as
a length prefix → **permanent frame desync** on that TCP connection (silent
message loss / cascade errors until reconnect).

**Fix**:
- `TcpMeshStream` retains `len_buf` / `len_filled` / `body` / `body_filled` on
  `self` and advances with single-buffer `read`s so progress survives future
  drop.
- Documented cancel-safety on `MeshStream::recv` and
  `EncryptedChannel::recv_encrypted`.
- Comment on M1 `select!` stating the contract.
- Regression tests: `recv_survives_select_cancellation`,
  `recv_resumes_after_manual_partial_len_state`.

### F2 — OK: outbound mpsc arm (M1)

`tokio::sync::mpsc::Receiver::recv` is cancel-safe. Complete messages remain in
the channel if the future is dropped. After a win, `send_encrypted` runs in the
arm body (not raced); task drop mid-send closes the peer path, which is the
correct failure mode.

### F3 — OK: WebSocket `MeshStream::recv`

`WsMeshStream` / `WsClientStream` use `StreamExt::next()` over tungstenite
messages. Incomplete WebSocket frames stay inside the reader; a dropped `next`
future does not desync message boundaries. Length prefix is fully inside one
binary WS message (`decode_ws_payload`).

### F4 — OK: substrate mesh poller (M2)

`oneshot` cancel and `Interval::tick` are cancel-safe. RPC
(`DaemonClient::connect` / `simple_call`) and `tx.send` run only after a tick
wins; cancellation during an in-flight RPC is observed on the **next** loop
iteration (~3s poll). Acceptable for tray ontology; not a wire-framing risk.

### F5 — Observation (not fixed): seed-peer path is write-only

`boot.rs` seed connect spawns a drain loop on outbound only (no inbound
`select!`). That is a functional asymmetry, not a cancel bug. Tracked only as
audit note; out of AC for cancel-correctness.

### F6 — Observation: `mesh_runtime` / `mesh_heartbeat` have no `select!`

ADR-010’s original file names evolved: cancel risk moved to the boot peer loop
+ transport framing. Heartbeat miss/suspect transitions are synchronous and
driven by callers (e.g. system service ticks), not by raced async arms.

### F7 — Residual / non-blocking

| Item | Risk | Disposition |
|------|------|-------------|
| `TcpMeshStream::send` multi-`write_all` | Not raced in `select!` | Documented; fix if a future loop races `send` |
| `MessageTooLarge` after length read | Leaves unread body on wire | Pre-existing; reconnect recovers |
| Accept loop has no shutdown `select!` | Listener task lives until process stop | Separate lifecycle work (MeshService stop disconnects peers) |
| Noise handshake multi-step `recv` | Not inside `select!` (runs before the loop) | Handshake drop = connection drop; OK |

## Regression convention (prevents reintroduction)

1. **Trait contract** — `MeshStream::recv` and `EncryptedChannel::recv_encrypted`
   rustdoc **must** state cancel-safety. New transports implement the contract
   or must not be used inside `select!`.
2. **Peer-loop comment** — The mesh bidirectional `select!` in `boot.rs` must
   keep an explicit cancel-safety comment listing each arm.
3. **Tests** — `mesh_tcp` includes `recv_survives_select_cancellation` (and a
   resume helper). Do not delete without replacing equivalent coverage.
4. **Review checklist** — Any new `tokio::select!` in `crates/clawft-kernel/src/mesh*.rs`
   or the mesh section of `boot.rs` must answer the four checklist questions
   above in the PR description (or link this memo).
5. **No Asupersync** — Per ADR-010; fix local cancel bugs, do not migrate runtime.

There is no custom clippy lint for cancel-safety (ecosystem gap). The
doc-comment + test convention is the enforceable substitute.

## Acceptance criteria (WEFT-18)

| AC | Status |
|----|--------|
| Catalog every `tokio::select!` site in mesh code | Done — table above |
| Verify branches cancel-safe per checklist | Done — F1–F7 |
| Document findings or fixes in an audit memo | Done — this file |
| Add lint or doc-comment convention preventing regression | Done — trait + boot comments + tests + §Regression convention |

## Files touched

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/mesh_tcp.rs` | Cancel-safe framed `recv`; regression tests |
| `crates/clawft-kernel/src/mesh.rs` | `MeshStream` cancel-safety contract |
| `crates/clawft-kernel/src/mesh_noise.rs` | `EncryptedChannel` cancel-safety contract |
| `crates/clawft-kernel/src/boot.rs` | Peer-loop cancel-safety comment |
| `crates/clawft-substrate/src/mesh.rs` | Poller cancel-safety comment |
| `docs/adr/adr-010-keep-tokio.md` | Audit status + link |
| `docs/research/adr-010-mesh-select-cancel-audit-2026-07-30.md` | This memo |

## How to verify

```bash
# Focused TCP + framing tests
scripts/build.sh test -p clawft-kernel -- mesh_tcp

# Or full kernel suite when practical
scripts/build.sh test -p clawft-kernel
```
