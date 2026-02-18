# Tools Implementation - Development Notes

## Summary

Implemented 7 tools in `clawft-tools` crate, ported from Python nanobot:

| Tool | File | Source |
|------|------|--------|
| `read_file` | `file_tools.rs` | `nanobot/agent/tools/filesystem.py::ReadFileTool` |
| `write_file` | `file_tools.rs` | `nanobot/agent/tools/filesystem.py::WriteFileTool` |
| `edit_file` | `file_tools.rs` | `nanobot/agent/tools/filesystem.py::EditFileTool` |
| `list_directory` | `file_tools.rs` | `nanobot/agent/tools/filesystem.py::ListDirTool` |
| `exec_shell` | `shell_tool.rs` | `nanobot/agent/tools/shell.py::ExecTool` |
| `memory_read` | `memory_tool.rs` | New (no direct Python equivalent) |
| `memory_write` | `memory_tool.rs` | New (no direct Python equivalent) |

## Architecture Decisions

### Path Validation (Security)

Two validation functions handle different scenarios:

- `validate_path()`: For existing paths. Canonicalizes both the target and workspace, then checks `starts_with`. Used by read, edit, list operations.
- `validate_parent_path()`: For paths that may not exist yet (write). Walks up to the nearest existing ancestor, canonicalizes that, and validates containment. This allows creating new files in subdirectories that don't exist yet while still preventing traversal.

Both reject absolute paths outside the workspace. On Unix, `workspace.join("/etc/passwd")` resolves to `/etc/passwd`, which correctly fails the containment check.

### Shell Safety

Uses simple substring matching on lowercased command strings rather than regex. This is intentionally conservative -- false positives (blocking safe commands) are preferred over false negatives (allowing dangerous ones). Patterns block: `rm -rf /`, `sudo`, `mkfs`, `dd if=`, fork bombs, `shutdown`/`reboot`/`poweroff`.

Timeout is clamped to `max_timeout` (default 300s) regardless of what the caller requests.

The shell tool uses `child.wait()` + manual stdout/stderr reading (via `take()` on handles) instead of `child.wait_with_output()` because the latter consumes `self`, preventing `child.kill()` in the timeout branch.

### Memory Tools

Memory tools use the `Platform` trait's `FileSystem::home_dir()` to locate `~/.clawft/workspace/memory/MEMORY.md` with fallback to `~/.nanobot/workspace/memory/MEMORY.md`. This keeps the implementation platform-agnostic (no direct `dirs` dependency needed).

Search uses paragraph-level matching (split on `\n\n`, case-insensitive substring search). This is intentionally simple -- semantic search via embeddings belongs in a higher-level layer.

### Generic Platform Parameter

All file tools are generic over `P: Platform + 'static` to support both native and WASM platforms. The `'static` bound is required by the `Tool` trait (which requires `Send + Sync`).

## Test Coverage

44 tests total:

- **file_tools.rs**: 17 tests (path validation, read/write/edit/list, error cases, traversal rejection)
- **shell_tool.rs**: 14 tests (echo, exit codes, stderr, timeout, timeout clamping, dangerous command rejection for 5 patterns, working directory, safe commands allowed)
- **memory_tool.rs**: 9 tests (paragraph search, read/write roundtrip, append/overwrite modes, missing params, path resolution)
- **lib.rs**: 4 tests inherited from integration via the module tests

All tests use temp directories with atomic counters + PID for isolation. Cleanup is best-effort (`let _ = remove_dir_all`).

## Line Counts

| File | Total | Production | Tests |
|------|-------|------------|-------|
| `lib.rs` | 63 | 63 | 0 |
| `file_tools.rs` | 782 | 402 | 380 |
| `shell_tool.rs` | 433 | 176 | 257 |
| `memory_tool.rs` | 434 | 244 | 190 |

All production code sections are under 500 lines.

## Dependencies Used

From workspace Cargo.toml (no additional dependencies added):
- `clawft-core` (Tool trait, ToolRegistry)
- `clawft-platform` (Platform trait, FileSystem, NativePlatform)
- `async-trait`, `tokio`, `tracing`, `serde_json`

## Known Limitations

1. `list_directory` uses `tokio::fs::metadata` directly rather than a platform trait method (the FileSystem trait doesn't expose metadata). This is acceptable for now but breaks WASM purity if list_directory ever needs to run in WASM.
2. Memory tool search is substring-based, not semantic. Adequate for MVP.
3. Shell tool dangerous command detection uses simple string matching. A more sophisticated approach (AST parsing, regex) could be added later.
