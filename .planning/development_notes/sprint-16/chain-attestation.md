# Chain Attestation + Host Revocation (Cognitum Seed Gaps #5 and #3)

**Date**: 2026-04-11
**Sprint**: 16 (WS2)
**Status**: Implemented, tests passing

---

## Gap #5: Custody Attestation Document

### What was implemented

- `CustodyAttestation` struct in `crates/clawft-kernel/src/chain.rs`
  - Fields: `device_id`, `epoch`, `chain_head`, `chain_depth`, `vector_count`, `content_hash`, `timestamp`, `signature`
  - All fields signed with kernel's Ed25519 key using a canonical format

- `ChainManager::generate_attestation()` method
  - Reads chain state (head hash, depth)
  - Accepts vector count, epoch, and content hash from caller
  - Builds a deterministic canonical string and signs with Ed25519
  - Returns `None` if no signing key configured

- Daemon RPC: `"custody.attest"` method
  - Gathers vector count/epoch from ECC vector backend (when available)
  - Computes content hash via BLAKE3
  - Returns `CustodyAttestResult` as JSON

- CLI: `weaver custody attest` command
  - New `custody_cmd.rs` in `clawft-weave/src/commands/`
  - Prints human-readable summary + JSON

### Files modified
- `crates/clawft-kernel/src/chain.rs` (struct + method)
- `crates/clawft-kernel/src/lib.rs` (re-export)
- `crates/clawft-weave/src/daemon.rs` (RPC dispatch)
- `crates/clawft-weave/src/protocol.rs` (wire types)
- `crates/clawft-weave/src/commands/custody_cmd.rs` (new CLI)
- `crates/clawft-weave/src/commands/mod.rs` (module registration)
- `crates/clawft-weave/src/main.rs` (CLI enum + dispatch)
- `crates/clawft-weave/Cargo.toml` (blake3 dep for ecc feature)

### Tests
- `chain::tests::generate_attestation_with_key` -- verifies full attestation + signature
- `chain::tests::generate_attestation_without_key_returns_none`
- `chain::tests::custody_attestation_serde_roundtrip`
- `commands::custody_cmd::tests::custody_args_parses`

---

## Gap #3: Persistent Host Revocation

### What was implemented

- New module: `crates/clawft-kernel/src/revocation.rs`
  - `RevokedHost` struct (host_id, revoked_at, reason)
  - `RevocationList` with thread-safe internal Mutex
  - Methods: `revoke_host()`, `is_revoked()`, `list_revoked()`, `unrevoke_host()`
  - Persistence: reads/writes `.weftos/runtime/revoked_hosts.json`
  - `load()` constructor for boot-time initialization

- Kernel integration:
  - `RevocationList` field added to `Kernel` struct in `boot.rs`
  - Loaded during boot sequence (step 6b)
  - Accessor: `kernel.revocation_list()`

- Cluster integration:
  - `ClusterMembership::add_peer_checked()` method
  - Rejects peers whose ID is in the revocation list with `AuthFailed`

- Daemon RPC:
  - `"mesh.revoke"` -- adds host to ban list, logs to chain
  - `"mesh.unrevoke"` -- removes host from ban list
  - `"mesh.revoked"` -- lists all banned hosts

- CLI (via `weaver cluster` subcommands):
  - `weaver cluster revoke <host_id> --reason "..."`
  - `weaver cluster unrevoke <host_id>`
  - `weaver cluster revoked`

### Files modified
- `crates/clawft-kernel/src/revocation.rs` (new module)
- `crates/clawft-kernel/src/lib.rs` (module + re-exports)
- `crates/clawft-kernel/src/boot.rs` (field, init, accessor)
- `crates/clawft-kernel/src/cluster.rs` (add_peer_checked)
- `crates/clawft-weave/src/daemon.rs` (3 RPC methods)
- `crates/clawft-weave/src/protocol.rs` (wire types)
- `crates/clawft-weave/src/commands/cluster_cmd.rs` (3 subcommands)

### Tests
- `revocation::tests::revoke_and_check`
- `revocation::tests::unrevoke`
- `revocation::tests::persistence_roundtrip`
- `revocation::tests::load_missing_file`
- `revocation::tests::load_malformed_file`
- `revocation::tests::list_revoked`
- `revocation::tests::default_path`
- `cluster::tests::add_peer_checked_allows_clean_host`
- `cluster::tests::add_peer_checked_rejects_revoked_host`

---

## Design Decisions

1. **Attestation signature format**: Used a simple canonical text format rather than CBOR or JSON to avoid canonicalization ambiguity. Fields are ordered deterministically with newline separators and prefixed with version `v1`.

2. **Content hash proxy**: Since `VectorBackend` does not expose ID enumeration, the content hash is computed from `count + epoch` rather than actual vector IDs. This still provides a useful fingerprint that changes when the store changes.

3. **Revocation via cluster_cmd**: Rather than a separate `weaver mesh` command tree, revocation subcommands were added to `weaver cluster` since they operate on mesh peers which are managed through the cluster subsystem.

4. **Ban list persistence**: Simple JSON file at `.weftos/runtime/revoked_hosts.json`. No migration needed -- the list is self-contained.
