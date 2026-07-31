# clawft-wasm crate split (WEFT-398)

## Summary

Host-side wasmtime plugin code was moved out of `clawft-wasm` into a
dedicated crate:

| Crate | Role | Target |
|-------|------|--------|
| **`clawft-wasm`** | Browser (`wasm32-unknown-unknown` + wasm-bindgen) and WASI (`wasm32-wasip2`) **entrypoint** | guest / edge |
| **`clawft-wasm-host`** | Native **wasmtime host**: sandbox, engine, audit log, permission store | host (native) |

## Why

The `wasm-plugins` modules (`sandbox`, `engine`, `audit`,
`permission_store`, ~4 KLOC of host logic + WIT) ran on native / wasip2
*host* processes only. They sat in `clawft-wasm` for historical naming
reasons and were completely orthogonal to W-BROWSER. Co-locating them:

- blurred the browser vs host boundary,
- forced browser packaging docs to explain a feature that never ships
  in the `cdylib`,
- pulled host-only dependency declarations into the entry crate.

## Layout

```
crates/clawft-wasm/           # browser + wasip2 entry (cdylib + rlib)
  src/{lib,platform,env,fs,http,allocator,…}.rs
  www/                        # harness
  tests/browser_*.rs

crates/clawft-wasm-host/      # native wasmtime host (rlib only)
  src/{lib,sandbox,engine,audit,permission_store,
       sandboxed_fs,sandboxed_http}.rs
  wit/plugin.wit              # host↔guest contract
```

## API compatibility

Historical paths still compile with the compat feature:

```rust
// Preferred (new code):
use clawft_wasm_host::{PluginSandbox, WasmPluginEngine, AuditLog, PermissionStore};

// Legacy re-export (same types):
// cargo build -p clawft-wasm --features wasm-plugins
use clawft_wasm::sandbox::PluginSandbox;
use clawft_wasm::engine::WasmPluginEngine;
```

## Build / test

```bash
# Browser entry (unchanged)
scripts/build.sh browser
cargo check --target wasm32-unknown-unknown -p clawft-wasm \
  --no-default-features --features browser

# Host crate
cargo test -p clawft-wasm-host
cargo check -p clawft-wasm-host

# Compat re-export smoke
cargo test -p clawft-wasm --features wasm-plugins
```

## Related

- [`architecture.md`](architecture.md) — browser vs native platform
- [`building.md`](building.md) — browser build commands
- ADR-083 — browser WASM support / native ⊻ browser mutex
