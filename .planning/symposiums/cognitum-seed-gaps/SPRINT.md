# Cognitum Seed Gap Sprint

**Date**: 2026-04-11
**Goal**: Close the 14 gaps between WeftOS and Cognitum Seed
**Priority**: Items that affect the Pi 5 appliance UX and vector store reliability

---

## Triage — 3 Tiers

### Tier 1: Must Have (blocks appliance deployment)

| # | Gap | Effort | Why Critical |
|---|-----|--------|-------------|
| 9 | Epoch-based vector versioning | M | Without it, concurrent writes silently corrupt. Pi 5 runs multiple agents writing vectors |
| 10 | Optimistic concurrency control | M | Depends on #9. Prevents stale overwrites |
| 12 | Soft-delete + compaction | M | Hard deletes break sync. Tombstones needed for mesh vector sync |
| 13 | Vector capacity limits | S | Pi 5 has 8GB RAM. Unbounded growth = OOM crash |
| 5 | Custody attestation document | S | Single proof of system state. All ingredients exist, just need assembly |
| 3 | Persistent host revocation | S | Security: banned hosts must stay banned across reboots |

### Tier 2: Important (appliance UX + integration)

| # | Gap | Effort | Why Important |
|---|-----|--------|-------------|
| 6 | HTTP SSE delta stream | M | External clients need real-time events without joining mesh |
| 7 | External witness injection | S | HTTP clients need to write chain events |
| 8 | REST API facade | L | Cognitum is HTTP-first. Need HTTP wrapper over kernel IPC |
| 14 | Per-user profile namespaces | M | Multi-tenant vector isolation on shared device |
| 2 | Time-windowed pairing | M | Physical security for device enrollment |

### Tier 3: Nice to Have (appliance-specific)

| # | Gap | Effort | Why |
|---|-----|--------|-----|
| 1 | USB composite gadget mode | L | Pi-as-USB-drive UX. Very Pi-specific |
| 4 | Delivery endpoint | S | Tied to USB gadget mode |
| 11 | Vector sync pull/push | L | SyncStreamType::Hnsw = 4 stub exists. Full delta sync |

---

## Sprint Plan — Expert Agents

### WS1: Vector Store Hardening (Tier 1, items 9-10-12-13)

**Agent**: kernel-architect / coder
**Scope**: 4 items, all in the vector/HNSW layer

- **Epoch tracking**: Add monotonic `epoch: u64` to vector store. Every insert/delete bumps epoch. Each vector tagged with creation epoch.
- **Optimistic concurrency**: `insert_with_epoch(vector, parent_epoch)` rejects if parent_epoch < current_epoch (409 Conflict equivalent)
- **Soft-delete + compaction**: Tombstone records (deleted_at, epoch). `compact()` method removes tombstones older than N epochs. Stats tracking.
- **Capacity limits**: Configurable `max_vectors` (default 50K on constrained, 1M on server). Reject inserts when full.

Files: `crates/clawft-kernel/src/hnsw_service.rs`, `crates/clawft-kernel/src/vector_backend.rs`

### WS2: Chain + Attestation (Tier 1, items 5, 3)

**Agent**: security-architect / coder
**Scope**: Custody attestation + host revocation

- **Custody attestation**: New `CustodyAttestation` struct assembling: device_id + epoch + chain_head + chain_depth + vector_count + content_hash + timestamp. Signed with Ed25519. Exposed via `weaver custody attest`.
- **Host revocation**: Persistent ban list at `.weftos/revoked_hosts.json`. `weaver mesh revoke <host_id>`. Checked on Noise XX handshake — revoked hosts rejected.

Files: `crates/clawft-kernel/src/chain.rs`, `crates/clawft-kernel/src/cluster.rs`

### WS3: HTTP Facade (Tier 2, items 6-7-8)

**Agent**: backend-dev / coder
**Scope**: HTTP REST API wrapping kernel IPC

- **SSE endpoint**: `GET /events` streaming init/ingest/delete/compact events + 15s heartbeat. Uses existing kernel event log.
- **Witness injection**: `POST /custody/witness` accepts external chain events. Validates signature, appends to chain.
- **REST facade**: Map the 33 Cognitum endpoints to kernel IPC calls. Start with the critical ones: `/status`, `/vectors`, `/chain`, `/pair`, `/custody`.

Files: New `crates/clawft-kernel/src/http_facade.rs` or extend existing `http_api.rs`

### WS4: Profile Namespaces + Pairing (Tier 2, items 14, 2)

**Agent**: coder
**Scope**: Multi-tenant vectors + physical pairing

- **Profile namespaces**: `profiles/{id}` directory with isolated vector storage per user. Config: `[kernel.profiles]` section. Each profile gets its own HNSW/DiskANN instance.
- **Time-windowed pairing**: 30-second enrollment window triggered by `weaver mesh pair --window 30`. During window, mesh accepts new handshakes. After window closes, new peers rejected until next pairing window.

Files: `crates/clawft-kernel/src/hnsw_service.rs`, `crates/clawft-kernel/src/cluster.rs`

---

## Estimated Timeline

| Workstream | Agents | Est. Time | Parallel? |
|-----------|--------|-----------|-----------|
| WS1: Vector hardening | 1 | 2-3 hours | Yes |
| WS2: Chain + attestation | 1 | 1-2 hours | Yes |
| WS3: HTTP facade | 1 | 3-4 hours | Yes |
| WS4: Namespaces + pairing | 1 | 2-3 hours | Yes |

All 4 workstreams can run in parallel. Total wall-clock: ~4 hours with 4 agents.

---

## Success Criteria

- [ ] Vector inserts reject when epoch stale (optimistic concurrency)
- [ ] Vector store enforces capacity limit (configurable)
- [ ] Soft-delete tombstones survive restart, compaction works
- [ ] `weaver custody attest` produces signed attestation document
- [ ] Revoked hosts cannot reconnect after kernel restart
- [ ] `GET /events` streams SSE events to curl
- [ ] `POST /custody/witness` injects external chain events
- [ ] Per-user vector namespaces isolate data
- [ ] Time-windowed pairing accepts peers only during window
