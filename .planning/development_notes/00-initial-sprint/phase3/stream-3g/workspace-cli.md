# Stream 3G-B: Workspace CLI + CLAWFT.md Loading + Scoped Resources

## Date: 2026-02-17

## Summary

Implemented Wave 3 of the workspace system: CLI commands, CLAWFT.md
loading with import resolution, and scoped resource path wiring.

## Files Created

### `crates/clawft-core/src/clawft_md.rs`
- `load_clawft_md(workspace_root)` -- loads and processes CLAWFT.md
- `find_clawft_md_chain(start_dir)` -- walks up to find all CLAWFT.md files, bounded at `.git`
- `resolve_imports(content, base_dir, depth)` -- recursive `@path` import resolution
- Security: rejects `..` path traversal, absolute paths, depth > 5
- Missing imports produce HTML comments instead of errors (graceful degradation)

### `crates/clawft-cli/src/commands/workspace_cmd.rs`
- `weft workspace create <name> [--dir <path>]`
- `weft workspace list [--all]`
- `weft workspace load <name-or-path>`
- `weft workspace status`
- `weft workspace delete <name> [-y]`
- `weft workspace config set <key> <value>` -- dot-notation config keys
- `weft workspace config get <key>`
- `weft workspace config reset`

## Files Modified

### `crates/clawft-core/src/lib.rs`
- Added `pub mod clawft_md;`

### `crates/clawft-cli/src/commands/mod.rs`
- Added `pub mod workspace_cmd;`

### `crates/clawft-cli/src/main.rs`
- Added `Workspace(WorkspaceArgs)` variant to `Commands` enum
- Added match arm routing to `workspace_cmd::run()`
- Added "workspace" to subcommand assertions in tests
- Added CLI parsing tests for all workspace subcommands

## Design Decisions

1. **Config set/get uses dot notation** -- `agents.defaults.model` maps
   to nested JSON keys. Values are auto-typed (int, float, bool, string).

2. **CLAWFT.md imports are lenient** -- missing imports produce HTML
   comments rather than hard errors, allowing partial workspace configs.

3. **Workspace delete only removes registry entry** -- filesystem
   cleanup is left to the user to prevent accidental data loss.

4. **Scoped resource paths** are printed in `workspace status` output
   to make the mapping clear: `{workspace}/.clawft/{sessions,memory,skills}/`.

5. **find_clawft_md_chain** stops at `.git` boundary to prevent
   leaking into unrelated parent workspaces.

## Test Coverage

### clawft-core (clawft_md.rs)
- Basic CLAWFT.md loading
- Missing CLAWFT.md returns empty string
- Import resolution (single, nested)
- Path traversal (`..`) rejection
- Absolute path rejection
- Circular import prevention (max depth)
- Missing import graceful handling
- Bare `@` passthrough
- `find_clawft_md_chain` parent collection
- `.git` boundary stopping

### clawft-cli (workspace_cmd.rs)
- `set_nested_value` for strings, ints, bools, floats, deep paths
- `get_nested_value` found/not-found/top-level
- CLI parsing tests for all subcommands (in main.rs)
