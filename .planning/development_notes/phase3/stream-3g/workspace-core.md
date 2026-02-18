# Stream 3G: Workspace Core Implementation

## Summary

Implemented workspace types, deep merge, discovery, and WorkspaceManager for
the clawft framework. This provides the foundation for multi-workspace support
with a 3-level config merge system (defaults < global < workspace).

## Files Created

- `crates/clawft-types/src/workspace.rs` -- WorkspaceEntry, WorkspaceRegistry
- `crates/clawft-core/src/config_merge.rs` -- deep_merge, normalize_keys, camel_to_snake
- `crates/clawft-core/src/workspace.rs` -- discover_workspace, WorkspaceManager, load_merged_config

## Files Modified

- `crates/clawft-types/src/lib.rs` -- added `pub mod workspace;`
- `crates/clawft-core/src/lib.rs` -- added `pub mod config_merge;` and `pub mod workspace;`
- `crates/clawft-core/Cargo.toml` -- added `dirs` dependency

## Design Decisions

### Deep Merge Rules (P1 #21)

- Objects: recursive merge, right keys override left
- Arrays: REPLACED (not concatenated) per consensus
- Scalars: right overrides left
- Null on right: deletes the key from base

### Workspace Discovery (4-step)

1. `$CLAWFT_WORKSPACE` env var (must have `.clawft/` subdir)
2. Walk from cwd upward looking for `.clawft/` directory
3. Fall back to `~/.clawft/` (global workspace)

### Config Merge (3-level)

1. `Config::default()` serialized to JSON (baseline)
2. `~/.clawft/config.json` merged on top (global overrides)
3. `<workspace>/.clawft/config.json` merged on top (workspace overrides)

The public `load_merged_config()` uses the real home directory. For testing,
`load_merged_config_from()` accepts explicit paths to avoid host contamination.

### Key Normalization

`normalize_keys()` converts camelCase JSON keys to snake_case recursively.
Handles consecutive uppercase (e.g. `HTMLParser` -> `html_parser`).

### WorkspaceManager

- `with_registry_path()` constructor enables deterministic testing
- `create()` builds the full `.clawft/` directory tree and registers
- `delete()` removes from registry only (caller handles disk cleanup)
- Directory structure: `.clawft/{sessions,memory,skills,agents,hooks}`

## Test Coverage

- clawft-types workspace: 11 tests (registry CRUD, serde, load/save)
- clawft-core config_merge: 16 tests (deep merge rules, camel_to_snake, normalize_keys)
- clawft-core workspace: 20 tests (discovery, WorkspaceManager lifecycle, config merge)
- Total new tests: 47

## Rust 2024 Edition Notes

- `std::env::set_var` and `std::env::remove_var` require `unsafe` blocks
- Tests that manipulate env vars use `unsafe { ... }` with safety comments
- Unique temp directories per test via atomic counter to avoid parallel test collisions
