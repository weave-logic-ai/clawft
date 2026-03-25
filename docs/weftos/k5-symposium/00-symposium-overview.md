# WeftOS K5 Symposium: Encrypted Mesh Network Architecture

**Date**: 2026-03-25
**Branch**: feature/weftos-kernel-sprint
**Status**: COMPLETE -- K6 readiness assessed, mesh architecture designed
**Predecessor**: [ECC Symposium](../ecc-symposium/00-symposium-overview.md)

---

## Purpose

This symposium evaluates K6 readiness and designs the transport-agnostic
encrypted mesh network that will connect WeftOS kernel instances across
Cloud, Edge, Browser, WASI, and embedded nodes. Three independent research
efforts were synthesized:

1. **K6 Readiness Audit** -- Source-level audit of all K0-K5 kernel code
   for networking prerequisites (41 GREEN, 22 YELLOW, 21 RED items)
2. **Mesh Networking Research** -- Survey of libp2p, snow, quinn, iroh,
   and DeFi-era mesh patterns for transport-agnostic networking
3. **Ruvector Crate Inventory** -- Audit of ruvector-cluster, ruvector-raft,
   ruvector-replication, ruvector-delta-consensus, ruvector-dag, and rvf-wire
   for reusable networking primitives

## Core Principle

**"The mesh IS the network."** Nodes authenticate via public key, discover
peers via DHT or mDNS, and communicate over whatever transport is available --
TCP, QUIC, WebSocket, WebRTC, BLE, or LoRa. The protocol layer does not care
about the underlying transport. Governance.genesis serves as the cluster-wide
trust root.

## Key Question

How to build a transport-agnostic encrypted mesh for WeftOS that:
- Works identically across Cloud, Edge, Browser, WASI, and embedded nodes
- Uses proven cryptographic primitives (Noise Protocol, Ed25519, ML-DSA-65)
- Reuses existing ruvector algorithms without their I/O assumptions
- Integrates cleanly with the existing K0-K5 kernel architecture
- Remains small (~1,150 lines of new Rust code)

## Key Finding

**WeftOS is 75% ready for K6 networking.** The cluster module, chain crypto,
IPC message types, and governance gate are all wire-ready. The critical gaps
are: no transport layer, no cluster-join authentication, no TLS/Noise
encryption, no `RemoteNode` message target, no chain merge protocol, and no
tree sync mechanism. All 6 gaps are addressable in ~1,150 lines across 6
sub-phases (K6.0-K6.5).

## Recommended Architecture

**snow (Noise Protocol) + quinn (QUIC) + selective libp2p (kad, mdns)**

- **snow** provides transport-agnostic authenticated encryption via the Noise
  Protocol Framework, proven in WireGuard and libp2p
- **quinn** provides QUIC as the primary native transport with built-in
  multiplexing, congestion control, and connection migration
- **libp2p-kad** and **libp2p-mdns** provide DHT discovery and local network
  discovery without importing the full libp2p framework
- Ed25519 public keys serve as node identity (replacing UUID-based NodeId)
- ML-DSA-65 dual signing for post-quantum chain events (already in chain.rs)

## Symposium Documents

| # | Document | Purpose |
|---|----------|---------|
| 00 | `00-symposium-overview.md` | This file -- scope, principle, and findings summary |
| 01 | `01-mesh-architecture.md` | Transport-agnostic layer design with ASCII diagrams |
| 02 | `02-k6-readiness-assessment.md` | GREEN/YELLOW/RED audit + ruvector crate inventory |
| 03 | `03-security-and-identity.md` | Noise handshakes, Ed25519 identity, threat model |
| 04 | `04-k6-implementation-plan.md` | Phased plan: K6.0-K6.5 with line estimates |
| 05 | `05-symposium-results.md` | Decisions D1-D10, commitments C1-C5, open questions |

## Panel Roster

| Panel | Focus | Source |
|-------|-------|--------|
| K6 Readiness | Source-level audit of K0-K5 code | k6-readiness-audit.md |
| Mesh Architecture | libp2p, snow, quinn, iroh survey | Agent research output |
| Ruvector Inventory | Crate-level audit of networking primitives | Agent research output |

## Recommended Reading Order

Start with **01-mesh-architecture** for the architectural vision, then
**02-k6-readiness-assessment** for current state, then
**04-k6-implementation-plan** for the phased build plan.
