# Vector Store Hardening (Cognitum Seed WS1)

**Date**: 2026-04-04
**Sprint**: 16
**Scope**: Gaps #9, #10, #12, #13 from Cognitum Seed gap analysis

## Changes

### 1. Epoch-based vector versioning (Gap #9)

- Added `epoch: AtomicU64` to `HnswService`, `HnswBackend`, `DiskAnnBackend`
- Every mutating operation (insert, remove, soft_delete, compact, clear) bumps the epoch
- `fn current_epoch(&self) -> u64` added to `VectorBackend` trait (default: 0)
- `HybridBackend` delegates epoch to cold tier (authoritative)
- Epoch uses `Ordering::SeqCst` for cross-thread visibility

### 2. Optimistic concurrency control (Gap #10)

- Added `fn insert_with_epoch(id, key, vector, metadata, parent_epoch)` to `VectorBackend` trait
- If `parent_epoch < current_epoch`, returns `VectorError::EpochConflict { expected, actual }`
- Added `EpochConflict` variant to `VectorError`
- Normal `insert()` remains unconditional (backward compatible)
- Implemented natively in `HnswBackend`, `DiskAnnBackend` (stub), `HybridBackend`
- Default trait impl covers backends that only override `current_epoch()`

### 3. Soft-delete + compaction (Gap #12)

- Added `fn soft_delete(&self, id: u64) -> bool` -- marks vector as tombstoned
- Added `fn compact(&self, older_than_epoch: u64) -> usize` -- purges old tombstones
- Added `fn tombstone_count(&self) -> usize`
- Tombstoned vectors excluded from `search()`, `contains()`, and `len()`
- Re-inserting a tombstoned ID clears the tombstone
- `HybridBackend` mirrors soft-deletes to both hot and cold tiers
- `Tombstone` struct stores `deleted_at_epoch` for age-based compaction

### 4. Vector capacity limits (Gap #13)

- Added `VectorError::StoreFull { max, current }` variant
- Added `fn max_vectors(&self) -> Option<usize>` and `fn set_max_vectors(&self, limit)`
- `HnswBackend`: default `None` (unbounded), configurable via `with_max_vectors()` or runtime `set_max_vectors()`
- `DiskAnnBackend`: defaults to `config.max_points`, overridable at runtime
- `HybridBackend`: delegates to cold tier
- Soft-deleted vectors do NOT count against capacity (live count = total - tombstones)
- Upserts (same ID) do not trigger capacity rejection

## Files Modified

| File | Changes |
|------|---------|
| `crates/clawft-kernel/src/vector_backend.rs` | `StoreFull`, `EpochConflict` error variants; 7 new trait methods with defaults |
| `crates/clawft-kernel/src/vector_hnsw.rs` | Full implementation of all 4 features; `Tombstone` struct; `with_max_vectors()` |
| `crates/clawft-kernel/src/vector_diskann.rs` | Full implementation for stub backend; epoch + tombstones + capacity |
| `crates/clawft-kernel/src/vector_hybrid.rs` | Delegation to cold tier; mirrored soft-delete to hot tier |
| `crates/clawft-kernel/src/hnsw_service.rs` | Added `epoch: AtomicU64`, `current_epoch()` method |

## Test Coverage

70 tests total across the vector modules:
- `vector_backend::tests`: 5 tests (error display, serialization)
- `vector_hnsw::tests`: 22 tests (epoch, concurrency, soft-delete, compaction, capacity)
- `vector_diskann::tests`: 14 tests (epoch, concurrency, soft-delete, compaction, capacity)
- `vector_hybrid::tests`: 18 tests (delegation, merge, promotion, epoch, soft-delete, capacity)
- `hnsw_service::tests`: 11 tests (existing + epoch)

## Design Decisions

- **SeqCst ordering for epochs**: Ensures all threads see epoch mutations in a consistent order, critical for optimistic concurrency correctness.
- **Tombstones are in-memory only**: For the HNSW backend, tombstones live in a `HashMap<u64, Tombstone>` protected by `Mutex`. Persistence of tombstones would require extending the save/load format (deferred to vector sync work in WS4/Gap #11).
- **Capacity checks use live count**: `live = total - tombstones`, so soft-deleted vectors free slots without requiring compaction first.
- **Default trait impls**: All new methods have sensible defaults so existing `VectorBackend` implementations continue to compile without changes.
- **DiskAnnBackend uses `StoreFull` instead of `CapacityExceeded`**: The old `CapacityExceeded` variant is preserved for backward compatibility but new capacity enforcement uses `StoreFull` which includes both `max` and `current` counts.
