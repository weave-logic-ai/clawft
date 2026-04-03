# ADR-041: Chain-Agnostic Blockchain Anchoring via ChainAnchor Trait

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K2 Symposium (D12), K-phases K4 scope (C7)

## Context

WeftOS maintains a tamper-evident ExoChain audit trail with hash-linked, Ed25519-signed events. For external auditability -- proving to a third party that a chain existed at a certain time without trusting the WeftOS operator -- the chain's head hash needs to be anchored in an external, independently verifiable ledger (a public blockchain, a timestamping service, or a consortium chain).

The K2 Symposium (Q12) debated whether to commit to a specific blockchain (Ethereum, Solana) or to define an abstraction. The panel observed that blockchain ecosystems change rapidly: gas costs fluctuate, new L2s emerge, and different deployments have different regulatory requirements. Committing to a single chain would limit deployment flexibility.

The design follows the same trait-first pattern used throughout WeftOS (e.g., `MeshTransport`, `DiscoveryBackend`, `EncryptedChannel`): define the trait surface now, implement the simplest binding first to prove the interface, then let users choose their backend.

## Decision

Define a `ChainAnchor` trait with three async methods:

```rust
pub trait ChainAnchor: Send + Sync {
    fn anchor(&self, hash: &[u8; 32], metadata: &AnchorMeta)
        -> Result<AnchorReceipt, AnchorError>;
    fn verify(&self, receipt: &AnchorReceipt)
        -> Result<AnchorStatus, AnchorError>;
    fn status(&self, receipt: &AnchorReceipt)
        -> Result<AnchorStatus, AnchorError>;
}
```

The first implementation is either a local mock (for testing) or OpenTimestamps (for a real but low-cost anchoring backend). Ethereum, Solana, and consortium chain backends implement the same trait later. `AnchorReceipt` is an opaque proof blob that the anchoring backend can verify. `AnchorMeta` carries optional metadata (chain_id, event_count, timestamp range) for the anchor.

The trait is chain-agnostic by design: the WeftOS kernel never references a specific blockchain. Backend selection is a deployment configuration choice.

## Consequences

### Positive
- Users can choose their anchoring backend without kernel changes
- The trait shape is minimal (3 methods), reducing the implementation burden for new backends
- OpenTimestamps as the first backend avoids gas costs and blockchain dependencies during development
- The `anchor(hash, metadata)` signature accepts any 32-byte hash, making it agnostic to the hash algorithm (SHAKE-256 today, BLAKE3 after K6 migration)
- Multiple backends can be active simultaneously (e.g., anchor to both Ethereum and OpenTimestamps for defense in depth)

### Negative
- The trait shape is a contract that all future backends must satisfy; changing it after multiple implementations exist is costly
- `AnchorReceipt` must be serializable and storable as a chain event, adding storage overhead
- The `verify()` method may require network access to the external chain, introducing latency and availability dependencies
- OpenTimestamps verification requires access to the Bitcoin blockchain (or a calendar server), which may not be available in air-gapped deployments

### Neutral
- The trait is K4 scope and not yet implemented in the codebase; this ADR documents the accepted design that will guide implementation
- The `status()` method (separate from `verify()`) allows checking anchoring progress for backends with confirmation delays (e.g., Bitcoin block confirmations)
