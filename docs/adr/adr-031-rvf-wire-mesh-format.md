# ADR-031: rvf-wire Zero-Copy Segments as Mesh Wire Format

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K5 Symposium Mesh Architecture Panel (D8, M7), Ruvector Inventory Panel
**Depends-On**: ADR-024 (Noise Protocol Encryption), ADR-026 (QUIC Primary Transport)

## Context

The WeftOS mesh network needs a binary serialization format for inter-node protocol messages (chain sync, tree sync, heartbeat, governance distribution, and `KernelMessage` forwarding). The format must support zero-copy deserialization for performance (avoiding allocation during message routing), work within the Noise-encrypted byte stream (ADR-024), and integrate with the length-prefixed framing protocol (ADR-026). Four options were evaluated:

1. **Protocol Buffers (protobuf)**: Widely used, schema-driven, good cross-language support. However, introduces a new dependency (`prost` or `protobuf`), requires `.proto` schema files and a build-time code generator, and does not provide zero-copy deserialization.

2. **FlatBuffers**: Zero-copy deserialization, schema-driven. However, the Rust ecosystem support (`flatbuffers` crate) is less mature, the schema compiler adds build complexity, and the format is less compact than alternatives for small messages.

3. **bincode**: Fast, compact, already familiar in the Rust ecosystem. However, not self-describing, not zero-copy (requires full deserialization), and has no cross-language support.

4. **rvf-wire**: Already in the workspace (used for ExoChain persistence), provides zero-copy segment serialization via memory-mapped byte slices, requires no external schema files, and has no additional dependencies beyond what the workspace already carries.

## Decision

Use `weftos-rvf-wire 0.2` (the WeftOS fork, ADR-029) zero-copy segment serialization as the mesh wire format for binary payloads. JSON framing remains available as an alternative encoding for `KernelMessage` transport (type byte `0x01`) for debugging and development.

Within the mesh framing protocol, the TYPE byte in the length-prefixed frame header selects the encoding:

| Type Byte | Payload Format | Use |
|-----------|---------------|-----|
| `0x01` | JSON | `KernelMessage` (human-readable, debugging) |
| `0x02` | RVF wire segment | `KernelMessage` (binary, production) |
| `0x03` | RVF wire segment | Chain sync request/response |
| `0x04` | RVF wire segment | Tree sync request/response |
| `0x05` | RVF wire segment | Heartbeat (SWIM protocol) |
| `0x06` | RVF wire segment | Governance rule distribution |
| `0xFF` | -- | Protocol error / close |

RVF wire segments provide zero-copy access to payload fields via memory-mapped byte slices. A receiving node can route a mesh message by reading the segment header without deserializing the full payload. This is critical for high-throughput chain replication and cognitive sync streams where the routing node may forward messages without inspecting the body.

The `SyncStreamType` discriminator within RVF segments enables QUIC stream prioritization (D15):

```
Chain > Tree > IPC > Cognitive > Impulse
```

The `SyncStateDigest` (~140 bytes) exchanged on QUIC stream open for delta computation is also encoded as an RVF wire segment.

RVF wire segments are framed inside the Noise-encrypted channel. The serialization boundary is:

```
Application -> RVF wire segment -> Length-prefix frame -> Noise encrypt -> Transport (QUIC/WebSocket)
```

## Consequences

### Positive
- Zero-copy deserialization: message routing can inspect headers without allocating or copying the full payload, reducing latency and memory pressure for forwarding nodes
- No new dependencies: `rvf-wire` (via `weftos-rvf-wire`) is already in the workspace and used for ExoChain persistence; reusing it for mesh framing avoids adding protobuf, flatbuffers, or other serialization frameworks
- No build-time code generation: unlike protobuf or flatbuffers, rvf-wire does not require a schema compiler or `.proto`/`.fbs` files in the build pipeline
- Consistent serialization layer: both chain persistence and mesh transport use the same format, simplifying the codebase and reducing format conversion at boundaries
- The JSON fallback (type byte `0x01`) enables debugging and development without specialized tooling

### Negative
- Cross-language support is limited: `rvf-wire` currently has only a Rust implementation. Non-Rust mesh participants (e.g., a JavaScript browser node, a Python monitoring tool) must implement rvf-wire deserialization or use the JSON fallback
- The rvf-wire segment format constrains message structure to what the format supports (typed segments with byte-slice payloads); complex nested structures may require multiple segments or embedded CBOR/JSON within a segment
- The `weftos-rvf-wire` fork (ADR-029) adds maintenance overhead: mesh wire format changes must be coordinated with the fork's release cycle
- rvf-wire is not self-describing in the way CBOR or protobuf are; the receiver must know the expected segment layout for a given type byte

### Neutral
- The coexistence of JSON (`0x01`) and RVF (`0x02`) for `KernelMessage` encoding allows nodes to negotiate encoding preference; production deployments default to RVF for performance, development builds can use JSON for debuggability
- rvf-wire's zero-copy model relies on the backing buffer remaining valid for the lifetime of the deserialized view; this is naturally satisfied by the Noise decryption buffer, which is allocated per-message
- Future transport implementations (WebRTC, BLE, LoRa) use the same RVF wire segments inside their Noise-encrypted streams -- the wire format is transport-agnostic by design
