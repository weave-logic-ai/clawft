# WEFT-122 result — wire axum handlers to http_facade types + SSE loop

**Ticket:** WEFT-122  
**Branch:** `wave0g/weft-122-http-facade-axum`  
**Base:** `release/0.8-staging`  
**Commit:** branch tip of `wave0g/weft-122-http-facade-axum` (`git log -1 --oneline`)  
**Date:** 2026-07-30  
**Agent:** coder-122 (wave-0g)  
**Status:** Shipped (smoke tests green)

## Problem

`clawft-kernel::http_facade` defined 13 routes + SSE + witness types, but no
axum handler in `clawft-services/src/api/` called into it. The facade was
orphaned; the SSE streaming loop on `poll_events()` was missing.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| axum handlers in `clawft-services/src/api/` call into `http_facade` types | **Done** |
| SSE handler loops on `poll_events()` | **Done** |
| Smoke test for each route + SSE client smoke test | **Done** (8 tests) |

## What shipped

### `crates/clawft-kernel/src/lib.rs`

- Exported `pub mod http_facade` behind feature `http-api` (module existed but
  was never declared — truly orphaned).

### `crates/clawft-services` (feature `api`)

| Piece | Role |
|-------|------|
| `src/api/http_facade_api.rs` | Axum binding: `KernelFacadeBackend` trait, `InMemoryKernelFacade`, route table, SSE loop, witness handler |
| `src/api/handlers.rs` | Merges nest-relative facade routes; `DELETE /agents/{name}` shares path with dashboard GET (axum param conflict) |
| `src/api/mod.rs` | `ApiState.kernel_facade: Arc<dyn KernelFacadeBackend>`; top-level `/events` + `/custody/witness` auth-gated |
| `Cargo.toml` | `api` enables `dep:clawft-kernel` + `clawft-kernel/http-api` |
| `tests/http_facade_smoke.rs` | Route smoke + SSE client + `poll_events` cursor |

### Gateway

`clawft-cli` `build_api_state` wires `InMemoryKernelFacade` until a daemon RPC
backend is plugged in (stub envelopes; SSE + routes work end-to-end).

### Route surface (uses `match_facade_route` / `build_rpc_params`)

| HTTP | Path | RPC / stream |
|------|------|--------------|
| GET | `/events` | SSE via `poll_events` (500ms poll, 15s heartbeat) |
| POST | `/custody/witness` | witness inject (`WitnessRequest`/`Response`) |
| GET | `/api/status` | `kernel.status` |
| GET | `/api/processes` | `kernel.ps` |
| GET | `/api/services` | `kernel.services` |
| GET | `/api/chain/status` | `chain.status` |
| GET | `/api/chain/events?count=N` | `kernel.logs` |
| GET | `/api/vectors/status` | `ecc.status` |
| POST | `/api/vectors/search` | `ecc.search` |
| GET | `/api/ecc/calibration` | `ecc.calibrate` |
| GET | `/api/ecc/coherence` | `ecc.coherence` |
| GET | `/api/custody/attest` | `custody.attest` |
| POST | `/api/agents/spawn` | `agent.spawn` |
| DELETE | `/api/agents/:pid` | `agent.stop` |

All facade routes require Bearer auth (same middleware as `/api/*`).

## Tests

```bash
cargo test -p clawft-services --features api --test http_facade_smoke
# → 8 passed

cargo test -p clawft-services --features api --test api_middleware
# → 11 passed

cargo test -p clawft-kernel --features http-api http_facade
# → 52 passed (kernel unit suite for the module)

cargo check -p clawft-cli --features api
# → ok
```

| Smoke test | Asserts |
|------------|---------|
| `facade_rpc_routes_smoke` | All 12 RPC routes 200 + method name |
| `facade_chain_events_passes_count_query` | `build_rpc_params` count=7 |
| `facade_agent_stop_injects_pid` | pid + graceful in params |
| `facade_witness_accepts_valid_request` | accepted + chain_hash |
| `facade_witness_rejects_empty_signature` | 400 rejected |
| `facade_routes_require_auth` | status/events/witness 401 |
| `facade_sse_poll_events_stream` | live TCP client gets `text/event-stream` frames |
| `facade_sse_uses_poll_events_for_new_log_entries` | cursor + AgentSpawn classify |

## Out of scope (intentionally)

- Production daemon RPC backend for `KernelFacadeBackend::call_rpc` (gateway
  uses in-memory stub; drop-in trait object ready).
- Ed25519 witness verification path (requires kernel `exochain`; stub accepts
  non-empty signature).
- WEFT-123 full integration suite gated on ProfilesConfig/PairingConfig.

## Follow-ups

- Implement a `DaemonKernelFacade` that forwards `call_rpc` over Unix socket.
- Optional: expose `KernelEventLog` from a live kernel for production SSE.
- Plane: close WEFT-122 with this commit SHA after lead merge.
