# ADR-003: WSP Transport Bindings — UDS / Mesh CBOR / WebSocket

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Session 6 (RQ6, Protocol Design) drafted the WeftOS Surface Protocol
(WSP) and committed to binding it against *existing* kernel rails
rather than inventing new transports. Three real deployment shapes
exist and cannot be collapsed into one: (a) a local composer and
renderer on the same node talking to the kernel over its Unix-domain
JSON-RPC socket; (b) a composer on node A and a renderer on node B
over the mesh, where envelope overhead matters and CBOR already
carries `weftos-leaf-types`; (c) a browser / webview / third-party
renderer whose only pipe is WebSocket. LSP proved JSON-RPC + capability
negotiation ages well; MCP confirmed it for AI workloads; the
leaf-push protocol (v0.6.17) proved CBOR-in-`MeshIpcEnvelope` is a
viable rail. Session 7's hybrid dev-panel (webview + capture sidecar)
forces the browser case to be first-class, not a footnote.

## Decision

WSP ships on three transports that share one CDDL schema in two
encodings. Never invent a fourth rail; never split the schema.

**Local (composer ↔ kernel ↔ renderer, same node)**: line-delimited
JSON-RPC 2.0 over the existing `clawft-rpc` Unix-domain socket.
Reuse `Request` / `Response`; add a new `Notification` envelope
(no correlation id) for observation-stream and substrate-update
traffic. This is the path the desktop egui shell uses today.

**Mesh (composer on node A, renderer on node B)**: CBOR-encoded WSP
frames carried as a new `MessagePayload::Wsp { frame: Vec<u8> }`
variant on `clawft-kernel::ipc::MessagePayload`. The outer mesh
envelope (`MeshIpcEnvelope`) is unchanged — no new routing layer.
CBOR is the correct encoding here because the observation stream is
high-frequency, small-per-record, and benefits from integer-tagged
map keys.

**Browser / WASM / third-party (editor sidecar, remote renderer)**:
JSON-RPC 2.0 over WebSocket, one logical session per connection,
identical frame shapes to the UDS path. CBOR-over-WebSocket is a
future optimisation but not a schema fork; if adopted, it follows
the same CDDL.

**Schema rule**: `§5` of Session 6's CDDL is authoritative. JSON and
CBOR encodings are mechanically equivalent — JSON uses string map
keys, CBOR may use integer map keys on hot paths. A renderer that
speaks only JSON is fully conformant; performance degrades, semantics
do not. This follows RFC 8949 CBOR-as-binary-JSON practice.

**Envelope rule**: LSP's request / response / notification trichotomy
is load-bearing. Today `clawft-rpc` has only request/response. WSP
adds `Notification` with `{ method, params }` and no id. Observation
streams and substrate updates use notifications, never requests, so
they do not pollute the correlation table.

## Consequences

### Positive
- No new socket, no new daemon, no new routing layer. The
  implementation cost is bounded by "extend existing rails".
- One CDDL, two encodings: a JSON-only renderer (toy, browser, third-
  party) is always conformant, which satisfies the AGENDA success
  criterion that a second renderer must be writable without touching
  composition logic.
- CBOR on mesh halves bytes on the observation tapestry (topology /
  doppler / range / bearing at high rate) and removes JSON number-
  parsing from the hot path.
- The browser / WebSocket path unblocks Session 7's dev-panel
  milestone 1 (egui-wasm in VSCode webview) without requiring the
  webview to open a Unix socket (which it cannot).
- `MessagePayload::Wsp` is backward-compatible: existing mesh peers
  ignore unknown payload variants via `#[non_exhaustive]`.

### Negative
- Three transports to test; a single canonical conformance suite per
  encoding is mandatory, not optional.
- Dual-encoding discipline adds review burden — every schema change
  must be verified to round-trip in both JSON and CBOR.
- `Notification` is a visible addition to `clawft-rpc::protocol`;
  extension authors must handle a new envelope shape.

### Neutral
- Versioning (ADR open) is handled at the capability-handshake level
  (`session.initialize`), not by forking the transport.

## Alternatives considered

1. **Invent a custom binary protocol** — rejected: Cap'n Proto /
   FlatBuffers would win on raw speed but lose LSP/MCP/leaf-push
   symmetry, which is the single biggest lever for renderer and
   client adoption. The bytes-saved delta is not worth the schema
   divergence.
2. **JSON everywhere, no CBOR** — rejected: the observation stream
   is high-frequency; parsing cost and wire size matter on mesh
   links. Session 6 §2 rationale 2 rules this out.
3. **CBOR everywhere, no JSON** — rejected: a browser renderer
   or a third-party tooling consumer would need a CBOR parser just
   to inspect a frame; dev-tools would regress. The single-schema
   rule keeps JSON a first-class reader without making it the
   only encoding.
4. **Separate sockets for control vs observation** — rejected:
   correlation-table pollution is solved by the `Notification`
   envelope; a second socket doubles the reconnect / capability
   surface and buys nothing.
5. **Roll WSP into RVF with a new segment type** — considered but
   deferred (Session 6 §12 open question). RVF alignment is still
   in flight; WSP should not block on it.

## Related

- Sessions: `session-6-protocol-design.md` (§2 transport + framing,
  §10 versioning), `session-7-dev-panel-embedding.md` (hybrid
  webview + sidecar, bridge design), `session-1-historical-canon.md`
  (X11 request/reply/event/error quadrant, Adaptive Cards).
- Foundation elements: predicate 3 (streaming-native), predicate 1
  (ontology-addressable — typed verb namespaces).
- ADRs: ADR-004 (schema shape), ADR-005 (verb set), ADR-006
  (primitive head), ADR-011 (dev-panel embedding).
