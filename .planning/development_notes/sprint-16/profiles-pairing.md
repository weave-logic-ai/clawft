# Per-User Profile Namespaces + Time-Windowed Pairing

**Date**: 2026-04-04
**Gaps Closed**: Cognitum Seed #14 (profiles), #2 (pairing)
**Status**: Implemented, tested, not committed

---

## Item 1: Per-User Profile Namespaces (Gap #14)

### Files Changed

- `crates/clawft-types/src/config/kernel.rs` -- Added `ProfilesConfig` and `PairingConfig` structs to `KernelConfig`
- `crates/clawft-kernel/src/profile_store.rs` -- **New file**: `ProfileStore` with isolated vector backends per profile
- `crates/clawft-kernel/src/lib.rs` -- Module registration + re-exports

### Design Decisions

- Each profile gets a `DashMap<String, ProfileEntry>` entry with its own `HnswBackend` instance
- Active profile tracked via `RwLock<String>` for cheap reads
- Profile metadata persisted to `{storage_path}/{id}/profile.json`
- Vectors directory created at `{storage_path}/{id}/vectors/` (reserved for future DiskANN per-profile)
- Default profile auto-created on first `load_existing()` if no profiles found
- ID validation: `[a-zA-Z0-9_-]+` only (prevents path traversal)
- Cannot delete the currently active profile (switch first)

### Config

```toml
[kernel.profiles]
enabled = true
storage_path = ".weftos/profiles"
default_profile = "default"
```

### API Surface

- `ProfileStore::create_profile(id, name)` -> `ProfileMeta`
- `ProfileStore::delete_profile(id)`
- `ProfileStore::list_profiles()` -> `Vec<ProfileMeta>`
- `ProfileStore::switch_profile(id)` -- sets active profile
- `ProfileStore::insert(id, key, vector, metadata)` -- delegates to active backend
- `ProfileStore::search(query, k)` -- delegates to active backend
- `ProfileStore::get_profile_backend(id)` -- direct access to a profile's backend
- `ProfileStore::load_existing()` -- boot-time scan of profiles directory
- `ProfileStore::persist_meta(id)` -- flush metadata to disk

### Tests (10 passing)

- `create_and_list_profiles`, `duplicate_profile_rejected`, `invalid_id_rejected`
- `switch_and_active_profile`, `delete_profile`
- `insert_and_search_active_profile`, `profiles_are_isolated`
- `load_existing_creates_default`, `load_existing_reads_persisted`
- `persist_meta_updates_vector_count`

---

## Item 2: Time-Windowed Pairing (Gap #2)

### Files Changed

- `crates/clawft-kernel/src/cluster.rs` -- Added `PairingGate`, `PairingWindowResult`, `PairedHost`, `PairedHostsFile`
- `crates/clawft-types/src/config/kernel.rs` -- Added `PairingConfig`

### Design Decisions

- Default state: mesh rejects all new peer connections (`PairingGate` window closed)
- `open_pairing_window(duration)` opens a time-limited enrollment window
- During the window, `should_accept_peer(peer_id)` returns `true` and records the peer
- After window closes, only previously paired peers are accepted
- Paired hosts persisted to `.weftos/runtime/paired_hosts.json` (configurable)
- Best-effort persistence: save on every pair/unpair, warn on failure
- Uses `Instant` for window deadline (not wall-clock, immune to clock skew)
- Thread-safe: all state behind `std::sync::Mutex`

### Config

```toml
[kernel.pairing]
persist_path = ".weftos/runtime/paired_hosts.json"
default_window_secs = 30
```

### API Surface

- `PairingGate::open_pairing_window(duration)` -> `PairingWindowResult`
- `PairingGate::is_pairing_open()` -> `bool`
- `PairingGate::should_accept_peer(peer_id)` -> `bool` (accept if paired OR window open)
- `PairingGate::paired_hosts()` -> `Vec<String>`
- `PairingGate::unpair(peer_id)` -> `bool`
- `PairingGate::load()` / `PairingGate::save()` -- disk persistence

### Tests (7 passing)

- `pairing_gate_default_closed`
- `pairing_gate_open_window_accepts`
- `pairing_gate_already_paired_always_accepted`
- `pairing_gate_unpair`
- `pairing_gate_persist_and_load`
- `pairing_gate_load_creates_file_if_missing`
- `pairing_window_result_serde`

---

## Remaining Integration Work

- Wire `ProfileStore` into `Kernel::boot()` (create from `ProfilesConfig`)
- Wire `PairingGate` into `Kernel::boot()` (create from `PairingConfig`)
- Add RPC handlers: `profile.create`, `profile.delete`, `profile.list`, `profile.switch`
- Add RPC handlers: `mesh.pair`, `mesh.paired`, `mesh.unpair`
- Add CLI commands in `clawft-weave`: `weaver profile create/list/delete/switch`, `weaver mesh pair/paired/unpair`
- Integrate `PairingGate::should_accept_peer()` into mesh Noise XX handshake path
