# Wave 0i — WEFT-391 result

**Ticket:** WEFT-391 — wire `set_env` to `BrowserEnvironment` via `OnceLock<BrowserRuntime>`  
**Branch:** `wave0i/weft-391-browser-env`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4c3-9a5b-7181-b33b-0595ed43d144`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-391 (wave-0i)

## Problem

After the browser pipeline wire-through, `BrowserPlatform` is moved into
`AgentLoop`, so `BrowserRuntime` no longer retained a handle to the live
`BrowserEnvironment`. `set_env` was a documented no-op stub; JS callers
could not mutate live env state. `BrowserPlatform::with_env` could only
pre-populate at construction, and `init()` had no JS-facing seed path.

## What shipped

### `clawft-platform` — shared env handle

| Item | Detail |
|------|--------|
| `BrowserPlatform.env` | Now `Arc<BrowserEnvironment>` (was owned value) |
| `BrowserPlatform::with_env_arc` | Construct from an existing `Arc` |
| `BrowserPlatform::env_arc` | Clone the live handle for `BrowserRuntime` |
| `Platform::env()` | Returns `self.env.as_ref()` — same map as the Arc |

### `clawft-wasm` — runtime wiring (WEFT-391)

| Item | Detail |
|------|--------|
| `BrowserRuntime.env` | `Arc<BrowserEnvironment>` stored next to `agent` / `tools` |
| `init(config_json, env_json?)` | Optional second arg: JSON object of string→string pre-seeds |
| `set_env(key, value)` | Mutates through `RUNTIME.env` after init; safe no-op before |
| `get_env(key)` | Reads the live map (JS `undefined` when unset / pre-init) |
| `parse_env_seed` | Validates object-of-strings; clear init errors on bad shape |

### Docs / TS surface

| Item | Detail |
|------|--------|
| `docs/browser/api-reference.md` | Lifecycle, `init` second arg, live `set_env`, new `get_env` |
| `clawft-ui/.../wasm-adapter.ts` | `ClawftWasm` interface matches new ABI |

### Test hygiene (browser feature)

| Item | Detail |
|------|--------|
| Native unit tests | Gated `#[cfg(all(test, feature = "native"))]` so `--features browser --no-default-features` can compile |
| `config_loader` MockFs | Same `async_trait` / `?Send` cfg as `FileSystem` |
| `test_discover_home_but_no_files` | Gated native-only (WASM path returns preferred path without FS probe) |

## Acceptance

| Criterion | Status |
|-----------|--------|
| `BrowserRuntime` stores `Arc<BrowserEnvironment>` alongside the agent | **Done** |
| `set_env(key, value)` mutates through it | **Done** |
| `init()` accepts a pre-seeded env map (optional) | **Done** — `env_json: Option<String>` |
| Test: set_env then read via `Platform.env()` returns the new value | **Done** — `browser::tests::set_env_via_arc_visible_through_platform_env` |
| Doc updated | **Done** — `docs/browser/api-reference.md` |

## Tests

**`clawft-platform` (`--features browser --no-default-features`)**

- `browser::tests::set_env_via_arc_visible_through_platform_env` — Arc set_var visible via `Platform::env()`
- `browser::tests::with_env_pre_seeds_vars`
- `browser::env::tests::arc_clone_sees_set_var_mutations`
- `browser::env::tests::set_get_remove_round_trip` / `with_vars_pre_populates`

**`clawft-platform` (default native)** — 52 passed (regression)

**`clawft-wasm` browser suite (`tests/browser_pipeline.rs`)**

- `init` calls updated for optional `env_json`
- `set_env_pre_init_is_safe_noop` + `get_env` smoke
- `init_rejects_malformed_env_map` / `init_env_map_non_string_value_errors`

**Compile**

- `cargo check -p clawft-wasm --features browser --target wasm32-unknown-unknown` — ok
- `scripts/build.sh check` — ok

## How to test

```bash
# Platform Arc / Platform.env round-trip (WEFT-391 AC)
cargo test -p clawft-platform --features browser --no-default-features --lib browser

# Native platform regression
cargo test -p clawft-platform --lib

# Browser WASM compile
cargo check -p clawft-wasm --features browser --target wasm32-unknown-unknown

# Full headless browser suite (needs wasm-pack + chromedriver)
scripts/build.sh test-browser
```

## Files changed

- `crates/clawft-platform/src/browser/mod.rs`
- `crates/clawft-platform/src/browser/env.rs`
- `crates/clawft-platform/src/lib.rs` (native test cfg)
- `crates/clawft-platform/src/env.rs` / `fs.rs` / `http.rs` / `process.rs` (native test cfg)
- `crates/clawft-platform/src/config_loader.rs` (MockFs ?Send + native-only discover test)
- `crates/clawft-wasm/src/lib.rs`
- `crates/clawft-wasm/tests/browser_pipeline.rs`
- `docs/browser/api-reference.md`
- `clawft-ui/src/lib/adapters/wasm-adapter.ts`
- `docs/plans/wave-0i-WEFT-391-result.md` (this file)

## Notes / follow-ups

- Pre-init `set_env` remains a no-op (does not queue); pre-seed via `init`'s `env_json`.
- Env store is still in-memory only (OPFS persistence is WEFT-14 / WEFT-392).
- Full `set_env` → agent turn → tool sees env still requires a valid API key path; covered by the Arc/`Platform::env` unit test plus HTML harness.
