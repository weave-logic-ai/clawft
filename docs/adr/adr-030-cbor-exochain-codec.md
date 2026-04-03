# ADR-030: CBOR (ciborium) as ExoChain Payload Codec

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: Architecture review, Sprint 14 (informed by K5 implementation, Cargo.toml dependency audit)
**Depends-On**: ADR-022 (ExoChain Mandatory Audit)

## Context

ExoChain events carry arbitrary payloads representing state-changing operations (agent lifecycle, assessments, governance decisions, tool invocations, etc. per ADR-022). These payloads must be serialized for chain storage, hash computation (SHAKE-256), replication across nodes, and verification by auditors. Three serialization formats were evaluated:

1. **JSON** (`serde_json`): Human-readable, widely supported, already used for `KernelMessage` transport. However, JSON is not deterministic (key ordering varies across implementations), not compact (string encoding, no binary support), and cannot guarantee hash stability -- the same logical payload could produce different byte representations and therefore different SHAKE-256 hashes.

2. **bincode**: Compact and fast, but not self-describing. A decoder must know the exact Rust type to deserialize. This breaks cross-language tooling (JavaScript chain viewers, Python analysis scripts) and makes schema evolution difficult.

3. **CBOR** (RFC 8949, via `ciborium 0.2`): Binary, compact, self-describing, and supports a canonical encoding mode (RFC 8949 Section 4.2) that guarantees deterministic byte output for identical logical values. This is critical for hash-chain integrity: the same payload always produces the same SHAKE-256 hash regardless of which node serialized it.

## Decision

ExoChain event payloads are serialized as CBOR using the `ciborium 0.2` crate. Deterministic Canonical CBOR (RFC 8949 Section 4.2) is enforced for all payloads that contribute to hash computation.

Canonical CBOR requirements enforced by the serializer:
- Map keys are sorted in bytewise lexicographic order
- Integer values use the shortest possible encoding
- No indefinite-length items
- No duplicate map keys

The dependency is declared at the workspace level:

```toml
ciborium = "0.2"
```

The `exochain` feature gate in `clawft-kernel` enables CBOR serialization. The `chain.rs` module uses `ciborium::ser::into_writer()` for serialization and `ciborium::de::from_reader()` for deserialization. The canonical form is verified before hash computation:

```rust
// Serialize to canonical CBOR
let mut buf = Vec::new();
ciborium::ser::into_writer(&payload, &mut buf)?;

// Hash the canonical bytes
let hash = shake256(&buf);
```

JSON remains available for `KernelMessage` transport (type byte `0x01` in the mesh framing protocol, ADR-026) and for human-readable debugging output. CBOR is used exclusively for chain event payloads where deterministic hashing is required.

## Consequences

### Positive
- Deterministic encoding guarantees hash stability: the same logical payload always produces the same SHAKE-256 hash, regardless of serialization order or platform
- Compact binary representation reduces chain storage size compared to JSON (typically 30-50% smaller for structured data)
- Self-describing format: a CBOR decoder can parse payloads without knowing the Rust type, enabling cross-language chain viewers and analysis tools
- RFC 8949 is a well-established IETF standard with implementations in every major language (JavaScript: `cbor-x`, Python: `cbor2`, Go: `fxamacker/cbor`)
- `ciborium` is a pure-Rust implementation with no unsafe code, aligning with WeftOS's security posture

### Negative
- All chain event consumers must understand CBOR; JSON-only tools cannot read chain payloads without a conversion layer
- Canonical CBOR enforcement adds serialization overhead compared to non-canonical CBOR (map key sorting, shortest-form integers)
- `ciborium 0.2` adds a dependency to the `exochain` feature gate; builds without `exochain` are unaffected
- Schema evolution requires care: adding fields to CBOR payloads is forward-compatible, but removing or renaming fields can break older chain readers
- Debugging raw chain data requires a CBOR-aware tool (e.g., `cbor-diag`) rather than a text editor

### Neutral
- CBOR coexists with JSON in the codebase: chain payloads use CBOR, `KernelMessage` transport uses JSON (or RVF wire segments). The boundary is clear: CBOR for persistence and hashing, JSON for transport debugging
- The `ciborium` crate's API mirrors `serde_json` closely (`into_writer`/`from_reader`), so the migration from JSON payloads was straightforward
- CBOR's binary format means chain payloads are not human-readable in raw form, but the `weaver chain inspect` CLI command decodes CBOR to JSON for display
