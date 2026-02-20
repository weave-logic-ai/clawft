# D-Perf: Performance -- Notes

**Items**: D1 (parallel tool execution), D10 (bootstrap cache), D11 (async file I/O)
**Week**: 2-3

---

## Completed: 2026-02-20

### D1: Parallel Tool Execution
- Used `futures::join_all` to execute tool calls concurrently
- Results collected and pushed to messages in original order (preserving tool_call_id association)
- Per-path advisory locks not implemented yet (future hardening)

### D10: Bootstrap File Cache
- `CachedFile` struct with `content: String` and `mtime: SystemTime`
- `BootstrapCache` type: `Arc<Mutex<HashMap<PathBuf, CachedFile>>>` using `tokio::sync::Mutex`
- `load_cached_file()` checks mtime via `tokio::fs::metadata()`, returns cached on hit
- Added to `ContextBuilder::new()` auto-initialization

### D11: Async File I/O
- All `std::fs` calls in `skills_v2.rs` replaced with `tokio::fs`
- `load_dir()` -> async using `tokio::fs::read_dir()`, `metadata()`, `read_to_string()`, `try_exists()`
- `load_legacy_skill()` renamed to `load_legacy_skill_async()`
- `discover()` and `discover_with_trust()` made async
- CLI caller `skills_cmd.rs:run()` made async
- 15 tests converted to `#[tokio::test]`
