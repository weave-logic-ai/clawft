# ADR-024: Noise Protocol (snow) for All Inter-Node Encryption

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K5 Symposium Security Panel (D3), Mesh Architecture Panel (M3)
**Depends-On**: ADR-025 (Ed25519 Node Identity)

## Context

WeftOS K6 introduces a transport-agnostic encrypted mesh network connecting Cloud, Edge, Browser, WASI, and Embedded nodes. The encryption layer must work identically over every transport (QUIC, WebSocket, WebRTC, BLE, LoRa) without depending on transport-native encryption (e.g., TLS). The options evaluated were TLS 1.3, DTLS, and the Noise Protocol Framework. TLS requires X.509 certificates, a PKI or CA, and certificate revocation lists -- infrastructure WeftOS deliberately avoids because the trust root is `governance.genesis`, not a certificate hierarchy. DTLS has the same PKI dependency. The Noise Protocol Framework provides authenticated key exchange with forward secrecy, mutual authentication, and no certificates.

## Decision

All inter-node traffic uses the Noise Protocol via the `snow 0.9` crate. Two handshake patterns are employed:

- **XX pattern** for first contact between nodes that have never communicated. Both sides transmit their static (Ed25519-derived Curve25519) public keys during the handshake. This provides mutual authentication, forward secrecy, and identity hiding (static keys are encrypted after the first message). Cost: 1.5 RTT.

- **IK pattern** for reconnection to known peers whose static public key was cached from a previous XX handshake or obtained via DHT lookup. The initiator encrypts to the responder's known key in the first message. Cost: 1 RTT.

The Noise parameter strings are:

```rust
// First contact (XX)
"Noise_XX_25519_ChaChaPoly_BLAKE2b"

// Known peer reconnection (IK)
"Noise_IK_25519_ChaChaPoly_BLAKE2b"
```

| Component | Choice | Rationale |
|-----------|--------|-----------|
| DH | X25519 (via `x25519-dalek 2.0`) | Fast, small keys (32 bytes), widely audited |
| Cipher | ChaChaPoly | AEAD, constant-time, no AES-NI dependency (critical for ARM/WASI) |
| Hash | BLAKE2b | Fast, 256-bit security, aligns with BLAKE3 migration path (ADR-043) |

The Noise layer wraps the raw `MeshStream` returned by any `MeshTransport` implementation. After the handshake completes, all subsequent bytes on the stream are authenticated and encrypted. The `mesh_noise.rs` module in `clawft-kernel` implements the handshake state machine and encrypted framing.

Browser nodes run WASM-compiled `snow` for Noise handshakes over WebSocket, since browsers cannot use transport-native QUIC encryption directly.

## Consequences

### Positive
- Transport-agnostic encryption: every transport adapter (QUIC, WebSocket, WebRTC, BLE) uses the identical Noise-based encryption layer, eliminating per-transport security audits
- No PKI, no X.509 certificates, no CRL infrastructure to operate
- Forward secrecy via ephemeral keys: compromising a node's long-term key does not expose past sessions
- Mutual authentication: both parties prove identity during the handshake, preventing MITM without a CA
- IK pattern provides 1-RTT reconnection for known peers, reducing latency for steady-state mesh traffic
- Proven in production: `snow` is the same library used by WireGuard and libp2p

### Negative
- Browser nodes cannot use native browser TLS; they must run WASM-compiled `snow` over WebSocket, adding ~100KB to the WASM bundle
- No integration with enterprise PKI or certificate-based infrastructure (intentional, but limits deployment in environments that mandate X.509)
- First-contact (XX) handshake is vulnerable to MITM if the attacker controls the discovery layer; mitigated by verifying `genesis_hash` in the WeftOS handshake layer above Noise
- Noise nonce-based encryption requires tracking message counters per session; counter reset requires a new handshake

### Neutral
- ChaChaPoly removes any dependency on AES-NI hardware, making performance consistent across x86, ARM, and WASM targets
- BLAKE2b as the Noise hash function is distinct from the SHAKE-256 used by ExoChain (coexistence documented in ADR-043)
- The hybrid ML-KEM-768 post-quantum key exchange (K6.4b) runs inside the Noise-encrypted channel, not alongside it
