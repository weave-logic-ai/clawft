# WEFT-13 + WEFT-14 result — browser OPFS FS + env persistence

**Tickets:**
- WEFT-13 — OPFS-backed `BrowserFileSystem` persistence
- WEFT-14 — OPFS-backed `BrowserEnvironment` persistence

**Branch / worktree:**
- Branch: `feat/weft-13-14-browser-persist`
- Worktree: `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/weft-13-14-browser-persist`
- Base: `release/0.8-staging` @ `4d8fa5d2`

## Summary

### WEFT-13 (FS)

Most of the OPFS filesystem work already landed as **WEFT-392**
(`BrowserFileSystem` dual backend, `browser-opfs` feature, wasm smoke tests).
This ticket closes the remaining acceptance gaps:

- Documented persistence in `docs/guides/browser.md` and refreshed
  `docs/browser/architecture.md`.
- Hardened host builds: OPFS `web_sys` path is `target_arch = "wasm32"` only
  so host unit tests with `browser-opfs` do not panic on imported statics.
- `scripts/build.sh test-browser` still runs `browser_opfs` under
  `FEATURES=browser-opfs`.

### WEFT-14 (Env)

Implemented durable env store behind the same `browser-opfs` feature:

- `BrowserEnvironment::open` / `open_with_seed` load `/clawft/.clawft/env.json`
  from OPFS when available; seed overlays persisted keys.
- Sync `Environment` trait methods keep working; mutations schedule a
  background OPFS flush on wasm; `flush().await` for deterministic tests.
- Graceful memory fallback when OPFS is missing / non-wasm host.
- `Debug` redacts sensitive-looking keys (`*API_KEY*`, `*TOKEN*`,
  `*SECRET*`, `*PASSWORD*`, `*AUTH*`, …).
- Secrets-in-browser trade-off documented in guides + module docs.
- `Platform::env()` remains a **sync** accessor (async load/flush only).

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-platform/src/browser/env.rs` | OPFS snapshot persistence, Debug redaction, open/flush |
| `crates/clawft-platform/src/browser/fs.rs` | wasm32-only OPFS open path; WEFT-13 docs tags |
| `crates/clawft-platform/src/browser/mod.rs` | `open()` loads env+fs; export `BrowserEnvBackend` |
| `crates/clawft-wasm/src/lib.rs` | `init` uses `BrowserEnvironment::open_with_seed` |
| `crates/clawft-wasm/tests/browser_env_persist.rs` | **new** wasm reload round-trip suite |
| `crates/clawft-wasm/tests/browser_opfs.rs` | Ticket tag WEFT-13 |
| `crates/clawft-wasm/Cargo.toml` | Feature comment |
| `crates/clawft-core/src/pipeline/mod.rs` | cfg native-only `llm_adapter` re-export (unblocks browser wasm check) |
| `scripts/build.sh` | `test-browser` runs `browser_env_persist` with OPFS |
| `docs/guides/browser.md` | Persistence + secrets trade-off section |
| `docs/browser/architecture.md` | Env OPFS row + feature diagram |

## How to test

```bash
# Host unit tests (memory + redaction + open fallback)
cargo test -p clawft-platform --no-default-features --features browser --lib browser::
cargo test -p clawft-platform --no-default-features --features browser-opfs --lib browser::

# wasm32 compile
cargo check -p clawft-platform --no-default-features --features browser-opfs --target wasm32-unknown-unknown
cargo check -p clawft-wasm --no-default-features --features browser-opfs --target wasm32-unknown-unknown

# Browser smoke (needs wasm-pack + Chrome/chromedriver)
FEATURES=browser-opfs scripts/build.sh test-browser
# → browser_pipeline + browser_opfs (WEFT-13) + browser_env_persist (WEFT-14)
```

## Verification run (this worktree)

| Check | Result |
|-------|--------|
| `cargo test -p clawft-platform … browser` | 16/16 browser tests pass |
| `cargo test -p clawft-platform … browser-opfs` (host) | 16/16 pass (memory fallback) |
| `cargo check … browser-opfs --target wasm32-unknown-unknown` (platform + wasm) | OK |
| `FEATURES=browser-opfs scripts/build.sh test-browser` | **skipped** — `wasm-pack` not installed in this environment |

## Acceptance criteria map

### WEFT-13

| AC | Status |
|----|--------|
| Reads/writes through OPFS | Done (WEFT-392 + this branch) |
| Falls back when OPFS unavailable | Done |
| State survives reload (smoke) | `browser_opfs.rs` |
| Existing clawft-wasm tests pass | Compile-checked; pipeline suite unchanged |
| Documented | `docs/guides/browser.md` + architecture |

### WEFT-14

| AC | Status |
|----|--------|
| Persist to OPFS or IndexedDB | OPFS at `ENV_PERSIST_PATH` |
| Round-trip survives reload | `browser_env_persist.rs` |
| `Platform::env()` signature retained | Sync accessor kept; open/flush async |
| Sensitive Debug redaction | `Debug` + unit test |
| Document secrets trade-off | guides + module docs |

## Plane close notes

Recommend closing both with:

- Commit SHA on merge of `feat/weft-13-14-browser-persist`
- Test commands above
- Note: full Chrome OPFS suite requires CI/`wasm-pack` job with `FEATURES=browser-opfs`
