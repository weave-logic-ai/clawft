# rvf-runtime 0.2 — three upstream bugs (WEFT-662)

**Status:** reported upstream (see issue links below)  
**Discovered:** WEFT-616 Phases 0/1 (`adc5f9bc`, 2026-07-14)  
**Tracked:** WEFT-662  
**Affected crate:** [`rvf-runtime`](https://crates.io/crates/rvf-runtime) — discovered on **0.2.0**  
**Updated:** 2026-08-26 — workspace moved to **0.3.2**; **Bug 1 is FIXED upstream**, Bugs 2 and 3 remain. See §"0.3.2 re-verification" below.  
**Upstream repository:** https://github.com/ruvnet/ruvector  
**Consumers in WeftOS:** `clawft-cow-memory`, `clawft-core` (`rvf` feature), `clawft-kernel` (`cluster` / related)

This document is the durable, repo-local record of three portability / semantics
bugs found while integrating `RvfStore` for branchable (agenticow-style) memory.
Local workarounds already ship; this ticket’s acceptance criterion was **filing
upstream with repro snippets** and documenting the shims so they can be removed
when fixes land.

---

## Bug index

| # | Short title | Severity | Upstream issue | Local workaround |
|---|-------------|----------|----------------|------------------|
| 1 | macOS link failure: hard-coded `__errno_location` | ~~**P0**~~ **FIXED in 0.3.2** | [ruvnet/RuVector#746](https://github.com/ruvnet/RuVector/issues/746) | `clawft-cow-memory` `macos_errno_shim` — now **dead code**, safe to remove |
| 2 | `RvfStore::open()` resets `metric` to L2 (and zeroes witness hash) | **P1** (silent correctness) | [ruvnet/RuVector#747](https://github.com/ruvnet/RuVector/issues/747) | always create/query as L2 + L2-normalize vectors |
| 3 | `delete()` bitmap never cleared by re-ingest; `compact()` drops re-ingested data | **P1** (data loss / query holes) | [ruvnet/RuVector#748](https://github.com/ruvnet/RuVector/issues/748) | crate-level tombstones; never call `RvfStore::delete` on working tips |

Related residual (not a separate Plane AC item, but called out in voice handoff):
there is **no public vector-by-id read** on `RvfStore` in 0.2 — see
`clawft-cow-memory/src/manifest.rs` comments. Tracked as a product gap, not
one of the three AC bugs below. **0.3.2 closes this** — see below.

---

## 0.3.2 re-verification (2026-08-26)

The workspace moved `rvf-runtime` 0.2.0 → **0.3.2** (`Cargo.toml`). Note there
is no 0.2.1: upstream published 0.2.0 (2026-02-16) then 0.3.0 (2026-06-11), so
0.2.0 was the terminal release of that line and this bump was the only way
forward.

Verified by diffing the crates.io sources of `rvf-runtime-0.2.0` and
`rvf-runtime-0.3.2`:

| Bug | State in 0.3.2 | Evidence |
|-----|----------------|----------|
| **1** macOS `__errno_location` | **FIXED** | `src/locking.rs` now declares `__errno_location` under `cfg(any(target_os = "linux", target_os = "android"))` and `__error` under `cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))`, with a matching `libc_errno()` per platform. |
| **2** `open()` resets `metric` | **STILL PRESENT** | `RvfStore::open()` still builds `RvfOptions { domain_profile, ..Default::default() }`, discarding the persisted metric. **Keep the L2-only discipline.** |
| **3** deletion bitmap / `compact()` | **ASSUMED STILL PRESENT** | Both versions carry a single `deletion_bitmap.clear()`; no behavioural fix identified. Not re-proven by test — **keep the crate-level tombstones.** |

### API compatibility

The 0.2 → 0.3 jump is **source-compatible for our usage** despite the major-ish
version change. Diffing the public `fn` surface of `RvfStore`:

- **Removed: nothing.**
- **Added:** `read_all_vectors`, `iter_vectors`, `metric`, `options`, `epoch`,
  `index_ready`, `embed_dashboard`, `extract_dashboard`.
- `RvfOptions` fields: **unchanged** (no additions, no removals).

`cargo check --workspace` and the nextest workspace suite both pass with **zero
source changes** to WeftOS.

New upstream modules in 0.3.2: `rabitq_path.rs` (RaBitQ quantization),
`vector_slab.rs`, `index_path.rs`, `hashing.rs`. New transitive deps:
`rvf-index` 0.2.0, `rvf-quant` 0.2.0.

### Follow-ups this unlocks

1. **Delete `crates/clawft-cow-memory/src/macos_errno_shim.rs`** and its `mod`
   declaration. The shim defines `__errno_location` via `#[no_mangle]` on macOS;
   0.3.2 no longer references that symbol on macOS, so the shim is dead code.
   Not done in this bump — it is a source change, kept separate from the
   dependency move.
2. **`read_all_vectors` / `iter_vectors` close the "no public vector-by-id read"
   residual** noted above. `clawft-cow-memory/src/manifest.rs` works around its
   absence; that workaround can now be revisited.
3. **`metric()` and `options()` accessors** make Bug 2 *observable* for the first
   time — we can now assert the metric a store came back with rather than
   assuming L2. They do not fix the reset.

---

## Bug 1 — macOS undefined symbol `__errno_location`

### Summary

`rvf-runtime`’s file-locking path declares a **glibc-only** `extern "C"` symbol
`__errno_location` and calls it for every Unix target. On macOS (and other
non-glibc Unix), the libc errno accessor is named `__error`, so any binary or
test that **links** `rvf-runtime` fails at link time with an undefined symbol.

`cargo check` / `cargo build --lib` succeed (rlib does not resolve externs);
failure appears only when producing an executable.

### Evidence (0.2.0)

`src/locking.rs` (verified in crates.io source `rvf-runtime-0.2.0`):

```rust
#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn __errno_location() -> *mut i32;
}

#[cfg(unix)]
fn libc_errno() -> *mut i32 {
    unsafe { __errno_location() }
}
```

(Same pattern still present in `rvf-runtime-0.3.0` at `locking.rs` ~304 / ~324.)

### Reproduction

```bash
# On macOS host
cargo new --bin rvf_errno_repro && cd rvf_errno_repro
# Cargo.toml:
#   [dependencies]
#   rvf-runtime = { version = "0.2", default-features = false, features = ["std"] }
#   tempfile = "3"

cat > src/main.rs <<'RS'
use std::path::Path;
use rvf_runtime::{DistanceMetric, RvfOptions, RvfStore};

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.rvf");
    let opts = RvfOptions {
        dimension: 4,
        metric: DistanceMetric::L2,
        ..Default::default()
    };
    let _s = RvfStore::create(&path, opts).expect("create");
    let _s2 = RvfStore::open(&path).expect("open");
    println!("ok");
}
RS

cargo build
# Expected on macOS without a shim:
#   ld: symbol(s) not found for architecture arm64
#   Undefined symbols: ___errno_location
```

### Expected fix (upstream)

Prefer portable errno access:

```rust
// Option A — libc crate (recommended)
fn libc_errno() -> *mut i32 {
    unsafe { libc::__errno_location() } // libc crate re-exports platform symbol
}

// Option B — cfg per platform
#[cfg(target_os = "linux")]
extern "C" { fn __errno_location() -> *mut i32; }
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
extern "C" { fn __error() -> *mut i32; }

#[cfg(unix)]
fn libc_errno() -> *mut i32 {
    #[cfg(target_os = "linux")]
    unsafe { __errno_location() }
    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    unsafe { __error() }
}
```

Or use the `errno` / `libc` crate APIs and drop the raw extern entirely.

### Local workaround (WeftOS)

| Location | Role |
|----------|------|
| [`crates/clawft-cow-memory/src/macos_errno_shim.rs`](../../crates/clawft-cow-memory/src/macos_errno_shim.rs) | **Single** `#[no_mangle] extern "C" fn __errno_location()` on `cfg(target_os = "macos")` forwarding to `libc::__error()` |
| [`crates/clawft-core/src/embeddings/mod.rs`](../../crates/clawft-core/src/embeddings/mod.rs) | Explicitly **does not** define a second shim (fat-LTO would multiply-define; WEFT-615) |

`clawft-cow-memory` is the sole link-time home for the symbol. Any future binary
that links `rvf-runtime` **without** depending on `clawft-cow-memory` will need
the same shim or an upstream fix. Consolidation of dual shims → one home is
**done**; removal waits on upstream.

---

## Bug 2 — `RvfStore::open()` resets `metric` to default L2

### Summary

`RvfStore::create(path, options)` honors `options.metric` (e.g. `Cosine`).
`RvfStore::open(path)` builds a **fresh** `RvfOptions { domain_profile, ..Default::default() }`,
so `metric` becomes `DistanceMetric::L2` regardless of create-time choice.
`boot()` restores `dimension` and `profile` from the on-disk manifest, but **not**
`metric` — metric is never persisted in the manifest format.

**Same path zeroes `last_witness_hash` on open** (`[0u8; 32]`), so a reopened
store’s witness chain restarts rather than resuming. Compact also resets the
hash by design.

### Evidence (0.2.0)

`store.rs` `open()`:

```rust
let opts = RvfOptions {
    domain_profile,
    ..Default::default()   // metric: DistanceMetric::L2
};
// ...
last_witness_hash: [0u8; 32],
```

`boot()` only:

```rust
self.options.dimension = manifest.dimension;
self.options.profile = manifest.profile_id;
// metric never restored
```

Still present in 0.3.x (`open` still `..Default::default()`; no metric field on
parsed manifest).

### Reproduction

```rust
use std::path::Path;
use rvf_runtime::{DistanceMetric, QueryOptions, RvfOptions, RvfStore};

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metric.rvf");

    let opts = RvfOptions {
        dimension: 3,
        metric: DistanceMetric::Cosine,
        ..Default::default()
    };
    {
        let mut s = RvfStore::create(&path, opts).unwrap();
        // unit-ish vectors: ranking under cosine vs L2 can diverge for non-unit data
        let a = [1.0f32, 0.0, 0.0];
        let b = [10.0f32, 10.0, 0.0];
        s.ingest_batch(&[&a, &b], &[1, 2], None).unwrap();
        // query while open with Cosine
        let q = [1.0f32, 1.0, 0.0];
        let _ = s.query(&q, 2, &QueryOptions::default()).unwrap();
        // If API exposes metric(): assert Cosine here
    }

    let s2 = RvfStore::open(&path).unwrap();
    // After open, distance path uses L2 (Default). Ranking of non-unit vectors
    // vs Cosine-created intent silently changes.
    // last_witness_hash() == [0; 32] even if create path had witnessed ops.
    let _ = s2;
}
```

### Expected fix (upstream)

1. Persist `metric` (and other durable options) in the MANIFEST segment.
2. On `open` / `boot`, restore `options.metric` from the manifest.
3. Restore or recompute `last_witness_hash` from the latest WITNESS segment
   so reopen continues the chain.

API options: `open_with_options(path, overrides)` for non-persisted knobs only.

### Local workaround (WeftOS)

[`crates/clawft-cow-memory/src/normalize.rs`](../../crates/clawft-cow-memory/src/normalize.rs)
implements the agenticow pattern:

- Never use `DistanceMetric::Cosine` at create time — always **L2**.
- **L2-normalize every vector** (ingest *and* query) to unit length.
- On unit vectors, L2 order ≡ cosine order, so top-K is stable across reopen.

Witness-hash restart on reopen is documented on
`PromoteReport.parent_hash` / `types.rs` as an upstream residual of WEFT-662.

---

## Bug 3 — permanent delete bitmap (re-ingest + compact)

### Summary

`RvfStore::delete(ids)` sets bits in an in-memory / manifest-persisted
`deletion_bitmap`. Those bits:

1. **Survive re-ingest of the same id** — `ingest_batch` inserts vector data but
   never clears the bit for that id.
2. Cause `query()` / index paths to **skip** the id forever while the bit is set.
3. Cause `compact()` to treat the id as dead: it **physically removes** the
   vector (including freshly re-ingested data), then clears the bitmap.

So “delete then re-insert under the same id” is broken, and “delete, re-insert,
compact” **destroys the re-inserted data**.

### Evidence (0.2.0)

- `delete`: `self.deletion_bitmap.delete(id)` when the vector exists and is not
  already deleted.
- `ingest_batch`: inserts into `self.vectors` only; **no**
  `deletion_bitmap.clear_ids(...)`.
- `query`: `if self.deletion_bitmap.is_deleted(vec_id) { continue; }`
- `compact`: `deleted_ids = deletion_bitmap.to_sorted_ids();` →
  `vectors.remove(id)` for each → `deletion_bitmap.clear()`.

`DeletionBitmap::clear_ids` exists (`deletion.rs`) but is not used by the
re-ingest path.

### Reproduction

```rust
use rvf_runtime::{DistanceMetric, QueryOptions, RvfOptions, RvfStore};

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("del.rvf");
    let opts = RvfOptions {
        dimension: 2,
        metric: DistanceMetric::L2,
        ..Default::default()
    };
    let mut s = RvfStore::create(&path, opts).unwrap();
    let v = [1.0f32, 0.0];
    s.ingest_batch(&[&v], &[42], None).unwrap();
    s.delete(&[42]).unwrap();

    // Re-ingest same id with new payload
    let v2 = [0.0f32, 1.0];
    s.ingest_batch(&[&v2], &[42], None).unwrap();

    let hits = s.query(&[0.0, 1.0], 1, &QueryOptions::default()).unwrap();
    // Actual: id 42 absent (still deleted) — BUG
    // Expected: id 42 present with v2

    s.compact().unwrap();
    let hits2 = s.query(&[0.0, 1.0], 1, &QueryOptions::default()).unwrap();
    // Actual: still absent — re-ingested payload was reclaimed as "dead" — BUG
    let _ = (hits, hits2);
}
```

### Expected fix (upstream)

On `ingest_batch` (or dedicated `upsert`), for each accepted id:

```rust
self.deletion_bitmap.clear_ids(&[vec_id]); // or per-id clear
```

Document that `delete` is soft-delete, re-ingest is undelete+replace, and
`compact` only drops ids still soft-deleted at compact time.

### Local workaround (WeftOS)

[`BranchableMemory::delete`](../../crates/clawft-cow-memory/src/branchable_memory.rs)
**deliberately does not** call `RvfStore::delete` on the working tip. Instead:

- Maintain a **crate-level tombstone set** per node.
- Chain-walk query masks results against tombstones (including same-node).
- Re-ingest of a tombstoned id removes the id from the tombstone set.
- `promote` to `base` is the narrow path that *does* call `RvfStore::delete`
  (base has no ancestor tombstone chain); documented limitation in `ops.rs`.

See also `chain_walk.rs` module docs and `tests/tombstone.rs`.

---

## Upstream issue URLs

Filed 2026-07-30 against `ruvnet/RuVector` (crates.io `repository` for `rvf-runtime`):

| Bug | GitHub issue |
|-----|----------------|
| 1 `__errno_location` | https://github.com/ruvnet/RuVector/issues/746 |
| 2 `open()` metric / witness | https://github.com/ruvnet/RuVector/issues/747 |
| 3 delete bitmap | https://github.com/ruvnet/RuVector/issues/748 |

Mirrored in `docs/plans/wave-0k-WEFT-662-result.md`. Plane WEFT-662 closes when these
URLs are recorded on the work item.

---

## Version matrix

| Version | Bug 1 | Bug 2 | Bug 3 | Notes |
|---------|-------|-------|-------|-------|
| 0.2.0 (WeftOS pin) | present | present | present | workspace `rvf-runtime = "0.2"` |
| 0.3.0 | present | present | present (same patterns) | INDEX_SEG etc. added; errno/open/delete paths unchanged in spirit |
| 0.3.2 (crates.io max as of report) | not re-diffed line-by-line; assume same unless release notes say otherwise | | | re-check on upgrade |

---

## Removal plan (when upstream fixes land)

1. Bump `rvf-runtime` in workspace `Cargo.toml` to the fixed version.
2. Delete `crates/clawft-cow-memory/src/macos_errno_shim.rs` and its `mod` /
   `libc` dep **if** Bug 1 is fixed for macOS links without the symbol.
3. Optionally switch to native Cosine metric if Bug 2 is fixed *and* metric is
   durable; keep L2-normalize discipline if chain-walk merge still requires
   uniform metric space.
4. Optionally call `RvfStore::delete` from `BranchableMemory::delete` once Bug 3
   is fixed; keep tombstones for multi-node lineage masking either way.
5. Close WEFT-662 follow-up / comment on Plane with the upgrade commit SHA.
