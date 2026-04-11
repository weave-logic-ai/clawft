# HTTP REST Facade with SSE Streaming

**Sprint**: 16 (WS3: HTTP Facade)
**Date**: 2026-04-04
**Gaps Closed**: #6, #7, #8 from Cognitum Seed comparison

## Files Changed

- `crates/clawft-kernel/src/http_facade.rs` -- new module (feature-gated `http-api`)
- `crates/clawft-kernel/src/lib.rs` -- added `pub mod http_facade` declaration

## Architecture

Transport-agnostic facade layer. Defines typed request/response structs, SSE
formatting, route matching, and RPC parameter building. Does NOT embed an HTTP
server -- the actual binding (axum in `clawft-services`, hyper in a standalone
binary, etc.) calls into these types.

### Gap #6: SSE Delta Stream

- `SseEventType` enum: `init`, `ingest`, `delete`, `compact`, `chain_append`, `agent_spawn`, `agent_stop`, `heartbeat`
- `SseMessage::to_sse_frame()` produces spec-compliant `event: ...\ndata: ...\n\n`
- `SSE_HEARTBEAT_COMMENT` = `:heartbeat\n\n` (15-second interval constant)
- `classify_boot_event()` maps kernel `BootEvent` messages to SSE types
- `poll_events(event_log, cursor)` returns delta messages since last poll

Usage: axum handler creates a `Stream` that polls every 500ms and emits
SSE frames; sends `SSE_HEARTBEAT_COMMENT` every 15 seconds.

### Gap #7: External Witness Injection

- `WitnessRequest` / `WitnessResponse` types for `POST /custody/witness`
- `verify_witness_signature()` validates Ed25519 sig over `event_type || canonical_json(payload)`
- Caller appends to ExoChain via `ChainManager::append("witness.external", ...)`

### Gap #8: REST API Facade

Route table (11 endpoints + SSE + witness):

| HTTP | Path | RPC Method |
|------|------|------------|
| GET | `/events` | SSE stream |
| POST | `/custody/witness` | witness injection |
| GET | `/api/status` | `kernel.status` |
| GET | `/api/processes` | `kernel.ps` |
| GET | `/api/services` | `kernel.services` |
| GET | `/api/chain/status` | `chain.status` |
| GET | `/api/chain/events?count=N` | `kernel.logs` |
| GET | `/api/vectors/status` | `ecc.status` |
| POST | `/api/vectors/search` | `ecc.search` |
| GET | `/api/ecc/calibration` | `ecc.calibrate` |
| GET | `/api/custody/attest` | `custody.attest` |
| POST | `/api/agents/spawn` | `agent.spawn` |
| DELETE | `/api/agents/:pid` | `agent.stop` |

`match_facade_route()` + `build_rpc_params()` produce the RPC method name and
JSON params. The server layer forwards to the daemon's existing `dispatch()`
function (or makes a Unix socket RPC call).

## Tests

42 unit tests covering:
- SSE frame formatting and event classification
- Event polling with cursor-based delta
- Witness request/response serialization
- Hex decode edge cases
- Route matching for all 13 routes (including trailing slash, query strings, wrong method, invalid PID)
- RPC param building with query extraction and body pass-through
- HTTP method parsing
- FacadeResponse status codes
- SSE header constants

## Known Pre-existing Issues

- `ProfilesConfig` / `PairingConfig` re-exports in `lib.rs:243` reference types
  that do not yet exist in `clawft-types`. This blocks `cargo test` for the
  entire kernel crate (not specific to this module). `cargo check` succeeds.
- `clawft-weave` has a pre-existing `blake3` dependency resolution error in
  workspace check mode.

## Next Steps

1. Wire axum handlers in `clawft-services/src/api/` that call the facade types
2. Add `custody.attest` RPC method to daemon dispatch (WS2 dependency)
3. Implement actual SSE streaming loop in the axum handler using `poll_events()`
4. Add integration tests once `ProfilesConfig`/`PairingConfig` types land
