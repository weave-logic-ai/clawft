# WEFT-159 result — Matrix channel `/sync` long-poll, room auto-join, `m.room.message`

**Status:** implemented  
**Branch:** `wave0d/weft-159-matrix-channel`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb45e-9f1f-7ed2-91ad-4190c5f198f1`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30

## Problem

The Matrix `ChannelAdapter` was a planning stub: `start()` never long-polled
`/sync`, never joined rooms, and never parsed events; `send()` returned a
synthetic `$timestamp` event id without calling the CS API. Enabling
`--features matrix` silently dropped every message.

## Solution

Real Matrix Client-Server API (v3) path via `reqwest` (no `matrix-sdk` dep):

| Surface | Behavior |
|---------|----------|
| `GET /_matrix/client/v3/sync` | Long-poll loop with `timeout` + `since`; exponential backoff on errors |
| Since token | Persisted under `{state_dir}/since_token` (default: platform data dir `clawft/matrix`) |
| Auto-join | `POST /_matrix/client/v3/join/{room}` for `auto_join_rooms` at start |
| Invites | When `auto_accept_invites`, join rooms listed under `rooms.invite` in `/sync` |
| Inbound | `m.room.message` (`m.text` / `m.notice` / `m.emote`) → `MessagePayload::text` via host; own messages + allow-list filtered |
| Outbound | `PUT .../rooms/{room}/send/m.room.message/{txn}` with UUID txn id; returns real `event_id` |
| Cancel | `CancellationToken` honored on sync wait and backoff sleep |

## Files

| Path | Change |
|------|--------|
| `crates/clawft-channels/src/matrix/client.rs` | **New** — CS API client (`sync`, `join_room`, `send_text`) + path encoding |
| `crates/clawft-channels/src/matrix/channel.rs` | Real `start`/`send` loop, since-token I/O, wiremock tests |
| `crates/clawft-channels/src/matrix/types.rs` | `state_dir`, `sync_timeout_ms`; proper `Default` |
| `crates/clawft-channels/src/matrix/mod.rs` | Export `client` + `MatrixClient` / `MatrixAdapterConfig` |
| `scripts/build.sh` | `check` / `test` honor `--features` (so matrix tests actually run) |
| `.planning/reviews/0.7.0-release-gate/05-channels.md` | Stub table + task #5 marked done (WEFT-159) |
| `.planning/sparc/phase4/06-channel-enhancements/04-element-06-tracker.md` | E5 → Done |

## Acceptance criteria

- [x] `/sync` long-poll with since-token persistence and reconnect-with-backoff
- [x] Auto-join configured rooms; parse `m.room.message` into `MessagePayload`
- [x] `send()` issues `PUT .../send/m.room.message/{txn}` and returns real event id
- [x] Cancellation respected; since token on disk for resume
- [x] `scripts/build.sh test clawft-channels --features matrix` — **222 passed**
- [x] `scripts/build.sh check --features matrix` — clean
- [x] Tracker entries updated

## How to test

```bash
scripts/build.sh check --features matrix
scripts/build.sh test clawft-channels --features matrix
# or focused:
cargo test -p clawft-channels --features matrix --lib matrix::
```

Wiremock cases cover: send event id, auto-join POST, sync deliver + since
persist, invite accept, allow-list drop, cancel during backoff.

## Notes

- Implemented with raw CS API rather than `matrix-sdk` (heavy; planning doc
  mentioned SDK, but acceptance criteria name REST endpoints and Cargo already
  had `wiremock`/`tempfile` notes for Matrix).
- E2E encryption / media upload / threads out of scope for this ticket.
