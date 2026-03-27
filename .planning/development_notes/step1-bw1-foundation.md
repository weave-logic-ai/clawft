# BW1: Browser Foundation -- WASM Compilation for Core Crates

Phase BW1 of the Browser Workstream. Makes `clawft-types`, `clawft-platform`,
and `clawft-plugin` compile for `wasm32-unknown-unknown` by feature-gating
native-only dependencies.

## Files Changed

### Workspace Root

- **`Cargo.toml`** -- Added `getrandom = { version = "0.2", features = ["js"] }`
  to `[workspace.dependencies]` for WASM-compatible random number generation.

### clawft-types

- **`crates/clawft-types/Cargo.toml`**
  - Added `[features]` section: `default = ["native"]`, `native = ["dep:dirs"]`,
    `browser = ["uuid/js", "dep:getrandom"]`.
  - Changed `dirs` to optional (`dirs = { workspace = true, optional = true }`).
  - Added optional `getrandom` dependency for browser builds.

- **`crates/clawft-types/src/config/mod.rs`**
  - Gated `dirs::home_dir()` call behind `#[cfg(feature = "native")]` in
    `Config::workspace_path()`.
  - Added `Config::workspace_path_with_home(home: Option<&Path>)` method as a
    browser-friendly alternative that accepts an explicit home directory.

### clawft-platform

- **`crates/clawft-platform/Cargo.toml`**
  - Added `[features]` section: `default = ["native"]`,
    `native = ["dep:tokio", "dep:reqwest", "dep:dirs", "clawft-types/native"]`,
    `browser = ["dep:wasm-bindgen", ..., "clawft-types/browser"]`.
  - All native deps (tokio, reqwest, dirs) made optional.
  - Browser deps (wasm-bindgen, web-sys, js-sys, etc.) added as optional.
  - `clawft-types` set to `default-features = false` so feature propagation
    is explicit.

- **`crates/clawft-platform/src/lib.rs`**
  - `Platform` trait: Added `#[cfg_attr(not(feature = "browser"), async_trait)]`
    / `#[cfg_attr(feature = "browser", async_trait(?Send))]` for conditional
    Send bounds.
  - `NativePlatform` struct and impl: Gated behind `#[cfg(feature = "native")]`.

- **`crates/clawft-platform/src/http.rs`**
  - `HttpClient` trait: Added conditional `async_trait` attributes.
  - `NativeHttpClient` struct, impl, and Default: Gated behind
    `#[cfg(feature = "native")]`.

- **`crates/clawft-platform/src/fs.rs`**
  - `FileSystem` trait: Added conditional `async_trait` attributes.
  - `NativeFileSystem` struct and impl: Gated behind `#[cfg(feature = "native")]`.

- **`crates/clawft-platform/src/env.rs`**
  - `NativeEnvironment` struct and impl: Gated behind `#[cfg(feature = "native")]`.
  - `Environment` trait is synchronous, so no `async_trait` changes needed.

- **`crates/clawft-platform/src/process.rs`**
  - `ProcessSpawner` trait: Added conditional `async_trait` attributes.
  - `NativeProcessSpawner` struct and impl: Gated behind
    `#[cfg(feature = "native")]`.

- **`crates/clawft-platform/src/config_loader.rs`**
  - `discover_config_path()`: Gated synchronous `path.exists()` calls behind
    `#[cfg(feature = "native")]`. On non-native, returns the preferred config
    path without synchronous filesystem checks (the caller validates via
    async `fs.exists()`).

### clawft-plugin

- **`crates/clawft-plugin/Cargo.toml`**
  - Added `native = ["dep:tokio-util"]` feature (default enabled).
  - Changed `tokio-util` to optional.

- **`crates/clawft-plugin/src/traits.rs`**
  - Gated `use tokio_util::sync::CancellationToken` behind
    `#[cfg(feature = "native")]`.
  - Added a minimal `CancellationToken` polyfill for non-native targets
    using `Arc<AtomicBool>` with `cancel()`, `is_cancelled()`, and `Default`.

- **`crates/clawft-plugin/src/lib.rs`**
  - Added `CancellationToken` to the public re-exports from `traits`.

## Issues Encountered and Resolutions

### uuid requires randomness source on WASM

The `uuid` crate v1.21+ with `v4` feature requires specifying a randomness
source when targeting `wasm32-unknown-unknown`. Added the `browser` feature
to `clawft-types` that enables `uuid/js` and `getrandom` with the `js` feature.
WASM builds must use `--features browser` (or the platform crate's browser
feature which propagates it).

### Synchronous path.exists() in config_loader

The `discover_config_path()` function used `Path::exists()` which performs
synchronous I/O. This was gated behind `#[cfg(feature = "native")]`. On WASM,
the function returns the preferred candidate path and the caller validates
existence asynchronously via `fs.exists()`.

### async_trait Send bounds

The `async_trait` macro generates `Send`-bounded futures by default, which is
incompatible with single-threaded WASM runtimes. All platform traits use
conditional attributes: `#[cfg_attr(feature = "browser", async_trait(?Send))]`
to relax Send bounds in browser mode.

## WASM Compilation Status

| Crate | Command | Status |
|-------|---------|--------|
| clawft-types | `cargo check --target wasm32-unknown-unknown -p clawft-types --no-default-features --features browser` | PASS (clean) |
| clawft-platform | `cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser` | PASS (clean) |
| clawft-plugin | `cargo check --target wasm32-unknown-unknown -p clawft-plugin --no-default-features` | PASS (clean) |

Note: clawft-types and clawft-platform require `--features browser` on WASM
because `uuid` v4 needs the `js` randomness source. clawft-plugin does not
depend on uuid and compiles with `--no-default-features` alone.

## Test Results

| Crate | Tests | Status |
|-------|-------|--------|
| clawft-types | 175 | All passing |
| clawft-platform | 46 + 2 doctests | All passing |
| clawft-plugin | 73 | All passing |
| Full workspace (`cargo check --workspace`) | -- | Compiles clean |
