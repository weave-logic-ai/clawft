# ADR-031: rvf-wire Zero-Copy Segments as Mesh Wire Format

**Date**: 2026-04-03
**Updated**: 2026-07-30 (WEFT-683 — implementation status / RVF deferral)
**Status**: Accepted
**Deciders**: K5 Symposium Mesh Architecture Panel (D8, M7), Ruvector Inventory Panel
**Depends-On**: ADR-024 (Noise Protocol Encryption), ADR-026 (QUIC Primary Transport)
**Related**: WEFT-110 (JSON KernelMessage path — Done), WEFT-683 (this amendment)

## Context

The WeftOS mesh network needs a binary serialization format for inter-node protocol messages (chain sync, tree sync, heartbeat, governance distribution, and `KernelMessage` forwarding). The format must support zero-copy deserialization for performance (avoiding allocation during message routing), work within the Noise-encrypted byte stream (ADR-024), and integrate with the length-prefixed framing protocol (ADR-026). Four options were evaluated:

1. **Protocol Buffers (protobuf)**: Widely used, schema-driven, good cross-language support. However, introduces a new dependency (`prost` or `protobuf`), requires `.proto` schema files and a build-time code generator, and does not provide zero-copy deserialization.

2. **FlatBuffers**: Zero-copy deserialization, schema-driven. However, the Rust ecosystem support (`flatbuffers` crate) is less mature, the schema compiler adds build complexity, and the format is less compact than alternatives for small messages.

3. **bincode**: Fast, compact, already familiar in the Rust ecosystem. However, not self-describing, not zero-copy (requires full deserialization), and has no cross-language support.

4. **rvf-wire**: Already in the workspace (used for ExoChain persistence), provides zero-copy segment serialization via memory-mapped byte slices, requires no external schema files, and has no additional dependencies beyond what the workspace already carries.

## Decision

Use `weftos-rvf-wire 0.2` (the WeftOS fork, ADR-029) zero-copy segment serialization as the **target** mesh wire format for binary payloads. JSON remains the **shipped** encoding for `KernelMessage` IPC envelopes until the RVF path is implemented and measured (see [Implementation status](#implementation-status-weft-683)).

### Framing vs encoding (clarified 2026-07-30)

Two orthogonal discriminators exist on the wire:

1. **Frame type** (`mesh_framing::FrameType`) — *message kind* in the length-prefixed frame header (`[4-byte BE len][1-byte type][payload]`). This is what production code actually uses today:

| Type Byte | FrameType | Use |
|-----------|-----------|-----|
| `0x01` | Handshake | WeftOS handshake |
| `0x02` | IpcMessage | KernelMessage IPC envelope |
| `0x03` | ChainSync | Chain sync request/response |
| `0x04` | TreeSync | Tree sync request/response |
| `0x05` | ServiceAdvert | Service advertisement |
| `0x06` | ProcessAdvert | Process advertisement |
| `0x07` | Heartbeat | Heartbeat (SWIM) |
| `0x08`–`0x0E` | Join / SyncDigest / Artifact / Log / Assessment | As in `mesh_framing.rs` |
| `0xFF` | — | Protocol error / close (reserved) |

2. **IPC payload encoding** — how the bytes *inside* an `IpcMessage` (`0x02`) frame are serialized. This is the dual-encoding decision for `KernelMessage` / `MeshIpcEnvelope`:

| Encoding byte (conceptual) | Format | Status (2026-07-30) |
|----------------------------|--------|---------------------|
| `0x01` | JSON (`serde_json`) | **Shipped** — default for all builds |
| `0x02` | RVF wire segment | **Deferred** — API reserved behind Cargo feature `mesh-rvf` |

> **Historical note:** The original 2026-04-03 type-byte table mixed message kind with payload encoding (JSON vs RVF as top-level type bytes for KernelMessage). That table never matched the implemented `FrameType` enum (Handshake at `0x01`, IpcMessage at `0x02`, …). WEFT-683 separates the two layers so the ADR describes the code.

### Target architecture (when RVF lands)

RVF wire segments provide zero-copy access to payload fields via memory-mapped byte slices. A receiving node can route a mesh message by reading the segment header without deserializing the full payload. This is critical for high-throughput chain replication and cognitive sync streams where the routing node may forward messages without inspecting the body.

The `SyncStreamType` discriminator within RVF segments enables QUIC stream prioritization (D15):

```
Chain > Tree > IPC > Cognitive > Impulse
```

The `SyncStateDigest` (~140 bytes) exchanged on QUIC stream open for delta computation is also a candidate for RVF wire segment encoding.

RVF wire segments are framed inside the Noise-encrypted channel. The target serialization boundary is:

```
Application -> RVF wire segment -> Length-prefix frame -> Noise encrypt -> Transport (QUIC/WebSocket)
```

Today's shipped boundary for KernelMessage IPC is:

```
Application -> JSON MeshIpcEnvelope -> FrameType::IpcMessage frame -> Noise encrypt -> Transport
```

## Implementation status (WEFT-683)

| Path | Status | Location |
|------|--------|----------|
| Length-prefix framing + `FrameType` | Shipped | `crates/clawft-kernel/src/mesh_framing.rs` |
| `MeshIpcEnvelope` JSON encode/decode | Shipped (WEFT-110) | `crates/clawft-kernel/src/mesh_ipc.rs` — `to_bytes` / `from_bytes` |
| JSON as production IPC encoding | **Current default** | Unconditional `serde_json`; `MeshIpcEncoding::Json` |
| RVF encode/decode for `KernelMessage` | **Not built** | Reserved: `MeshIpcEncoding::Rvf` under `feature = "mesh-rvf"`; returns `UnsupportedEncoding` |
| Type-byte *encoding* switch on the wire | **Not built** | No encoding prefix is written today; frames carry bare JSON |

### Why RVF for IPC was deferred

- Mesh IPC volume and latency have not been a measured bottleneck; JSON round-trips (WEFT-110) meet current development and early-mesh needs.
- Building a correct zero-copy RVF path for nested `KernelMessage` / `MessagePayload` types is non-trivial (segment layout, versioning, forward compatibility) and was never scoped under WEFT-110.
- Shipping an Accepted ADR that claimed RVF as the production default without an implementation was **plan/code drift**. Per the living-ADR rule, the plan is updated to match reality rather than leaving a false default assertion.

### Triggers to build the RVF IPC path (revisit criteria)

Implement Option B (full RVF path) when **any** of the following holds:

1. **Measured encoding cost** — profiling shows `serde_json::to_vec` / `from_slice` on `MeshIpcEnvelope` exceeds **~50 µs p99** per message on a representative mesh workload, **or** JSON payload size is a dominant share of mesh bandwidth at sustained load.
2. **Throughput target** — a release goal requires **≥10k KernelMessage/s** per node hop (or equivalent relay-forward rate) that JSON cannot meet in bench.
3. **Zero-copy forwarding requirement** — multi-hop relays must inspect only headers without allocating full envelope trees (chain / cognitive sync streams already pressure this).
4. **Cross-crate alignment** — ExoChain / weave RVF codecs are extended such that reusing the same segment layout for mesh IPC is a small incremental cost.

Until then, default production and debug builds use JSON. The `mesh-rvf` Cargo feature is the opt-in gate for experimental API surface only — it does **not** flip the production default.

### When Option B is done, acceptance looks like

- [ ] Encoding dispatch for IPC payloads (`0x01` JSON / `0x02` RVF), orthogonal to `FrameType`
- [ ] RVF encode/decode for `MeshIpcEnvelope` / `KernelMessage`, round-trip tested
- [ ] Production default flips to RVF under default features (JSON retained for debug / mixed-version peers)
- [ ] Benchmark documenting the performance claim that motivated the original decision

## Consequences

### Positive
- Zero-copy deserialization (target): message routing can inspect headers without allocating or copying the full payload, reducing latency and memory pressure for forwarding nodes
- No new dependencies for the RVF path: `rvf-wire` (via `weftos-rvf-wire`) is already in the workspace for ExoChain; reusing it for mesh framing avoids protobuf/flatbuffers
- No build-time code generation: unlike protobuf or flatbuffers, rvf-wire does not require a schema compiler
- Consistent serialization layer (target): chain persistence and mesh transport share a format family
- JSON fallback enables debugging and development without specialized tooling — and is **currently the only shipped IPC encoding**
- WEFT-683 removes ADR/code drift: defaults in prose match defaults in binaries

### Negative
- Cross-language support for RVF is limited: only a Rust implementation today; non-Rust peers must use JSON
- The rvf-wire segment format constrains message structure; complex nested structures may need embedded CBOR/JSON inside a segment
- The `weftos-rvf-wire` fork (ADR-029) adds maintenance overhead when the RVF mesh path is built
- rvf-wire is not self-describing; the receiver must know the expected segment layout
- Until Option B ships, mesh IPC forgoes the zero-copy benefits that motivated this ADR

### Neutral
- Dual encoding remains the **architectural** goal; JSON is the **operational** default until triggers fire
- Future transports (WebRTC, BLE, LoRa) can carry the same framed payloads inside Noise — transport-agnostic by design
- `FrameType` growth for new message kinds does not require an encoding change
