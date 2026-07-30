# Wave 0j — WEFT-392 result

**Ticket:** WEFT-392 — implement OPFS-backed BrowserFileSystem behind `browser-opfs` feature  
**Branch:** `wave0j/weft-392-browser-opfs`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4cf-e146-7d52-a3e6-24ac30bd138b`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-392 (wave-0j)

## Problem

`BrowserFileSystem` was an in-memory `Mutex<HashMap<PathBuf, String>>`. Page
reload wiped workspace files. Real OPFS was deferred because web-sys
`FileSystem*Handle` bindings were believed to require unstable flags; the
audit also wanted `ListDirectoryTool` metadata and a clear home-dir layout
for browser vs native.

## What shipped

### Feature flag

| Crate | Feature | Notes |
|-------|---------|--------|
| `clawft-platform` | `browser-opfs` | Depends on `browser` |
| `clawft-tools` | `browser-opfs` | Propagates platform feature |
| `clawft-wasm` | `browser-opfs` | Propagates platform + tools; `init()` opens OPFS-capable FS |

### `BrowserFileSystem` backends

| Backend | When | Persistence |
|---------|------|-------------|
| **Memory** | Default / always available | Session-only |
| **OPFS** | `browser-opfs` + `BrowserFileSystem::open()` / `BrowserPlatform::with_env_arc_open()` when `navigator.storage.getDirectory()` succeeds | Survives reload |

Runtime fallback: if OPFS is missing (non-secure context, no API), `open()`
logs a warning and uses memory.

### web-sys version / features (AC)

| Item | Value |
|------|--------|
| Minimum | **web-sys 0.3.70** (stable `StorageManager::getDirectory`, `FileSystemDirectoryHandle`) |
| Verified | **0.3.85** (workspace lockfile) |
| Unstable cfg | **Not required** — OPFS is not behind `web_sys_unstable_apis` |
| Features enabled | `Navigator`, `WorkerNavigator`, `StorageManager`, `FileSystemHandle`, `FileSystemHandleKind`, `FileSystemDirectoryHandle`, `FileSystemFileHandle`, `FileSystemGetDirectoryOptions`, `FileSystemGetFileOptions`, `FileSystemWritableFileStream`, `FileSystemCreateWritableOptions`, `File`, `Blob`, `WritableStream` |

### Path layout (home_dir collision)

| Constant | Value |
|----------|--------|
| `BROWSER_HOME_DIR` | `/clawft` |
| `BROWSER_WORKSPACE_DIR` | `/clawft/workspace` |
| Config (via `home_dir()`) | `/clawft/.clawft/config.json` |

Browser never uses relative cwd `.clawft/`; all agent FS roots are absolute
under `/clawft`. `clawft-wasm` `init()` registers tools with
`BROWSER_WORKSPACE_DIR`.

### `FileSystem::metadata` + ListDirectoryTool

- New `FsMetadata { is_dir, len }` on the platform trait.
- Native: `tokio::fs::metadata`.
- Browser memory/OPFS: real is_dir / size.
- `ListDirectoryTool` uses `platform.fs().metadata()` for both targets
  (no more browser stub of `(false, 0)`).

### file_tools OPFS-no-symlink audit

- `resolve_sandbox_path` (non-native): lexical `..` / `.` only — documented as
  OPFS has no symlinks.
- `path_exists` (non-native): still optimistic `true` (no sync OPFS stat);
  containment uses normalized paths; missing files fail on async FS ops.

### Send/Sync

OPFS directory handles are `!Send`/`!Sync`. Wrapped in `SendDirHandle` with
`unsafe impl Send/Sync` justified by single-threaded WASM.

## Acceptance

| Criterion | Status |
|-----------|--------|
| OPFS impl behind `browser-opfs` | **Done** |
| In-memory retained as fallback | **Done** |
| web-sys flags + version documented | **Done** (Cargo.toml + architecture.md) |
| file_tools resolve_sandbox_path / path_exists audited | **Done** |
| OPFS-aware ListDirectoryTool metadata | **Done** (via `FileSystem::metadata`) |
| Persistence test write → reopen → read | **Done** (`crates/clawft-wasm/tests/browser_opfs.rs`; requires wasm-pack + Chrome) |
| Browser home_dir collision resolved | **Done** (`/clawft` + constants) |

## Tests

**Host (memory path + metadata)**

```bash
cargo test -p clawft-platform --no-default-features --features browser --lib browser
# 11 passed (incl. memory RW, list_dir metadata, home_dir layout)

cargo test -p clawft-platform --lib
# 52 passed (native regression)

cargo test -p clawft-tools --lib file_tools
# 21 passed
```

**Compile**

```bash
cargo check -p clawft-platform --no-default-features --features browser-opfs --target wasm32-unknown-unknown
cargo check -p clawft-wasm --no-default-features --features browser-opfs --target wasm32-unknown-unknown
```

**Browser OPFS (when wasm-pack + chromedriver available)**

```bash
wasm-pack test --headless --chrome crates/clawft-wasm \
  --no-default-features --features browser-opfs \
  --test browser_opfs

# or
FEATURES=browser-opfs scripts/build.sh test-browser
```

## How to enable in production browser builds

```bash
cargo build -p clawft-wasm --target wasm32-unknown-unknown \
  --no-default-features --features browser-opfs

# or append via build.sh
FEATURES=browser-opfs scripts/build.sh browser
```

Without `browser-opfs`, behaviour remains the previous in-memory FS
(session-only). With the feature, `init()` calls
`BrowserPlatform::with_env_arc_open` and prefers OPFS.

## Files changed

- `crates/clawft-platform/Cargo.toml` — `browser-opfs` + web-sys OPFS features
- `crates/clawft-platform/src/fs.rs` — `FsMetadata` + `metadata()` trait method
- `crates/clawft-platform/src/browser/fs.rs` — dual backend (memory + OPFS)
- `crates/clawft-platform/src/browser/mod.rs` — open helpers, constants re-export
- `crates/clawft-platform/src/config_loader.rs` — MockFs `metadata`
- `crates/clawft-core/src/session.rs` — MockFs `metadata`
- `crates/clawft-core/tests/prompt_identity_golden.rs` — GoldenFs `metadata`
- `crates/clawft-tools/Cargo.toml` — `browser-opfs`
- `crates/clawft-tools/src/file_tools.rs` — metadata listing + OPFS audit comments
- `crates/clawft-wasm/Cargo.toml` — `browser-opfs`
- `crates/clawft-wasm/src/lib.rs` — `with_env_arc_open` + workspace constant
- `crates/clawft-wasm/tests/browser_opfs.rs` — persistence + metadata suite
- `docs/browser/architecture.md` — OPFS feature + web-sys matrix
- `scripts/build.sh` — `test-browser` FEATURES + browser_opfs suite
- `docs/plans/wave-0j-WEFT-392-result.md` — this file

## Notes / follow-ups

- Env persistence remains in-memory (WEFT-14).
- Conversation history persistence is WEFT-399.
- `wasm-pack` was not installed in this worktree; OPFS browser suite is
  written and compile-checked but not executed here.
- Enabling `browser-opfs` on the default browser CI build is optional; keep
  `browser` as the minimal matrix and add a job with `browser-opfs` when
  chrome OPFS coverage is desired.
