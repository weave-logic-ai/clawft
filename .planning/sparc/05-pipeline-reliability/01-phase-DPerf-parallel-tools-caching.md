# Phase D-Perf: Parallel Tool Execution, Bootstrap Caching & Async Skills I/O

> **Element:** 05 -- Pipeline & LLM Reliability
> **Phase:** D-Perf (Performance)
> **Timeline:** Week 2-3
> **Priority:** P1
> **Crates:** `clawft-core` (`src/agent/loop_core.rs`, `src/agent/context.rs`, `src/agent/skills_v2.rs`)
> **Dependencies:** None (all three items are independent; D11 overlaps with Element 04 C3)
> **Blocks:** D3 (Retry/back-off -- benefits from faster tool round-trips)
> **Status:** Planning

---

## 1. Overview

Phase D-Perf addresses three independent performance bottlenecks in the agent pipeline:

| Item | File | Problem | Fix |
|------|------|---------|-----|
| **D1** | `loop_core.rs:376-403` | Tool calls execute sequentially -- N tools take N x latency | Parallel execution via `futures::future::join_all` |
| **D10** | `context.rs:105-145` | Bootstrap files (SOUL.md, IDENTITY.md, etc.) re-read from disk every `build_system_prompt()` call | In-memory mtime-based cache |
| **D11** | `skills_v2.rs:434-590` | Blocking `std::fs` calls in `load_dir()` and `load_legacy_skill()` stall the Tokio executor | Replace with `tokio::fs` async equivalents |

All three items are independent and can be implemented in parallel. D11 overlaps with Element 04 Phase C3 Task 3.8 -- if C3 completes the `tokio::fs` migration first, D11 is satisfied and should be marked as done.

**Key deliverables:**
- Replace sequential tool execution loop with `futures::future::join_all`
- Add mtime-aware bootstrap file cache to `ContextBuilder`
- Replace all `std::fs` calls in `skills_v2.rs` with `tokio::fs`

---

## 2. Current Code

### 2.1 Sequential Tool Execution (D1)

**File:** `crates/clawft-core/src/agent/loop_core.rs`, lines 376-403

The tool execution loop processes each tool call serially. When the LLM requests N tool calls in a single turn, total latency is the sum of all individual tool latencies rather than the maximum.

```rust
// loop_core.rs:376-403
for (id, name, input) in tool_calls {
    let permissions = request
        .auth_context
        .as_ref()
        .map(|ctx| &ctx.permissions);
    let result = self
        .tools
        .execute(&name, input.clone(), permissions)
        .await;
    let result_json = match result {
        Ok(val) => {
            let truncated =
                crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
            serde_json::to_string(&truncated).unwrap_or_default()
        }
        Err(e) => {
            error!(tool = %name, error = %e, "tool execution failed");
            format!("{{\"error\": \"{}\"}}", e)
        }
    };

    request.messages.push(LlmMessage {
        role: "tool".into(),
        content: result_json,
        tool_call_id: Some(id),
        tool_calls: None,
    });
}
```

**Observations:**
- `self.tools.execute()` is an async call that may involve network I/O, subprocess spawning, or file operations
- The `request.messages.push()` at the end requires results in the same order as the original `tool_calls` vec
- `permissions` is shared read-only across all calls -- no mutation hazard
- `MAX_TOOL_RESULT_BYTES` truncation and error formatting are pure transformations -- safe to parallelize

### 2.2 Bootstrap File Disk Reads (D10)

**File:** `crates/clawft-core/src/agent/context.rs`, lines 105-145

Every call to `build_system_prompt()` re-reads up to 5 bootstrap files from disk. In a typical agent session, `build_system_prompt()` is called once per LLM turn, meaning these files are read repeatedly even though they rarely change.

```rust
// context.rs:105-145
pub async fn build_system_prompt(&self) -> String {
    let bootstrap_files = ["SOUL.md", "IDENTITY.md", "AGENTS.md", "USER.md", "TOOLS.md"];
    let mut loaded_files = std::collections::HashMap::new();
    for filename in &bootstrap_files {
        let home = self.platform.fs().home_dir();
        if let Some(home) = home {
            let ws_path = expand_workspace(workspace, &home);
            let candidates = [
                ws_path.join(filename),
                ws_path.join(".clawft").join(filename),
            ];
            for file_path in &candidates {
                if self.platform.fs().exists(file_path).await {
                    match self.platform.fs().read_to_string(file_path).await {
                        Ok(content) if !content.trim().is_empty() => {
                            loaded_files.insert(*filename, content);
                        }
                        Ok(_) => {
                            debug!(file = %filename, "bootstrap file is empty, skipping");
                        }
                        Err(e) => {
                            warn!(file = %filename, error = %e, "failed to read bootstrap file");
                        }
                    }
                    break;
                }
            }
        }
    }
```

**Observations:**
- `ContextBuilder` struct fields: `config`, `memory`, `skills`, `platform` -- no cache field exists
- Platform abstraction (`self.platform.fs()`) is used for all file I/O -- cache must work through the same abstraction or sit above it
- `home_dir()` and `expand_workspace()` are deterministic for a given config -- path computation can also be cached

### 2.3 Blocking `std::fs` in Skills Loader (D11)

**File:** `crates/clawft-core/src/agent/skills_v2.rs`, lines 434-590

The `load_dir()` function and `load_legacy_skill()` helper use blocking `std::fs` calls that stall the Tokio executor thread when called from async context.

```rust
// skills_v2.rs:434 -- load_dir() is sync
fn load_dir(dir: &Path) -> Result<Vec<SkillDefinition>> {
    // ...
    let entries = std::fs::read_dir(dir).map_err(ClawftError::Io)?;      // :439 BLOCKING
    // ...
    match std::fs::metadata(&skill_md_path) {                              // :473 BLOCKING
    // ...
    match std::fs::read_to_string(&skill_md_path) {                        // :496 BLOCKING
    // ...
}

// skills_v2.rs:567 -- load_legacy_skill() is also sync
fn load_legacy_skill(json_path: &Path, skill_dir: &Path) -> Result<SkillDefinition> {
    let json_content = std::fs::read_to_string(json_path).map_err(ClawftError::Io)?;  // :568 BLOCKING
    // ...
    match std::fs::read_to_string(&prompt_path) {                          // :581 BLOCKING
    // ...
}
```

**Observations:**
- `load_dir()` is called from `discover()` / `discover_with_trust()` which are currently sync
- Converting `load_dir()` to async requires `discover()` and `discover_with_trust()` to become async
- All call sites must be updated (agent startup in `loop_core.rs`, CLI commands)
- Security validations (SEC-SKILL-02, SEC-SKILL-07) must remain intact after migration

---

## 3. Implementation Tasks

### Task 3.1: Add `futures` Dependency (D1)

**File:** `crates/clawft-core/Cargo.toml`

The workspace already defines `futures-util = "0.3"` in the root `Cargo.toml`. Add it to `clawft-core`:

```toml
[dependencies]
# ... existing deps ...
futures-util = { workspace = true }
```

If the workspace does not define `futures` (full crate), `futures-util` provides `future::join_all` which is all that is needed.

### Task 3.2: Parallel Tool Execution (D1)

**File:** `crates/clawft-core/src/agent/loop_core.rs`, lines 375-403

Replace the sequential `for` loop with parallel execution using `futures::future::join_all`.

**Before (sequential):**
```rust
for (id, name, input) in tool_calls {
    // ... execute sequentially, push result ...
}
```

**After (parallel):**
```rust
use futures_util::future::join_all;

// Spawn all tool executions concurrently
let tool_futures: Vec<_> = tool_calls
    .into_iter()
    .map(|(id, name, input)| {
        let permissions = request
            .auth_context
            .as_ref()
            .map(|ctx| ctx.permissions.clone());
        let tools = self.tools.clone();
        async move {
            let result = tools
                .execute(&name, input.clone(), permissions.as_ref())
                .await;
            let result_json = match result {
                Ok(val) => {
                    let truncated =
                        crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
                    serde_json::to_string(&truncated).unwrap_or_default()
                }
                Err(e) => {
                    error!(tool = %name, error = %e, "tool execution failed");
                    format!("{{\"error\": \"{}\"}}", e)
                }
            };
            (id, result_json)
        }
    })
    .collect();

let results = join_all(tool_futures).await;

// Append results in original order (join_all preserves order)
for (id, result_json) in results {
    request.messages.push(LlmMessage {
        role: "tool".into(),
        content: result_json,
        tool_call_id: Some(id),
        tool_calls: None,
    });
}
```

**Key design decisions:**
- `join_all` preserves the order of the input iterator, so `tool_call_id` ordering is maintained without explicit sorting
- `self.tools.clone()` requires `ToolRegistry` to implement `Clone` (it uses `Arc` internally, so this is a shallow clone)
- `permissions.clone()` avoids lifetime issues with the borrow of `request.auth_context`
- No `tokio::spawn` -- we stay on the current task so cancellation propagates correctly

**Race condition mitigation:** When multiple tools write to the same file (e.g., two `write_file` calls), results are non-deterministic under parallel execution. This is the same behavior as concurrent processes -- the LLM should not issue conflicting writes. For a future hardening pass, per-path advisory locks (using `tokio::sync::Mutex<HashSet<PathBuf>>`) can be added to the `ToolRegistry`. This is out of scope for D1 but documented in Risk Notes.

### Task 3.3: Bootstrap File Cache Struct (D10)

**File:** `crates/clawft-core/src/agent/context.rs`

Add a cache struct and integrate it into `ContextBuilder`:

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::sync::Mutex;

/// Cached bootstrap file entry with modification-time tracking.
#[derive(Debug, Clone)]
struct CachedFile {
    content: String,
    mtime: SystemTime,
}

/// Cache for bootstrap files, keyed by resolved path.
///
/// Uses `tokio::sync::Mutex` because the cache is checked/updated
/// inside async `build_system_prompt()`. The critical section is
/// short (one HashMap lookup + optional fs::metadata call).
type BootstrapCache = Arc<Mutex<HashMap<PathBuf, CachedFile>>>;
```

Update `ContextBuilder` struct:

```rust
pub struct ContextBuilder<P: Platform> {
    config: AgentsConfig,
    memory: Arc<MemoryStore<P>>,
    skills: Arc<SkillsLoader<P>>,
    platform: Arc<P>,
    bootstrap_cache: BootstrapCache,  // NEW
}
```

Update `ContextBuilder::new()` to initialize the cache:

```rust
pub fn new(
    config: AgentsConfig,
    memory: Arc<MemoryStore<P>>,
    skills: Arc<SkillsLoader<P>>,
    platform: Arc<P>,
) -> Self {
    Self {
        config,
        memory,
        skills,
        platform,
        bootstrap_cache: Arc::new(Mutex::new(HashMap::new())),
    }
}
```

### Task 3.4: Cache-Aware File Loading (D10)

**File:** `crates/clawft-core/src/agent/context.rs`, inside `build_system_prompt()`

Replace the direct `read_to_string` calls with a cache-check-then-read pattern:

```rust
/// Load a bootstrap file, using the mtime cache to skip disk reads
/// when the file has not been modified since last load.
async fn load_cached_file(
    platform: &Arc<P>,
    cache: &BootstrapCache,
    file_path: &Path,
) -> Option<String> {
    // Check current mtime via platform metadata
    let current_mtime = match platform.fs().metadata(file_path).await {
        Ok(meta) => meta.modified().ok(),
        Err(_) => return None, // file does not exist or is inaccessible
    };

    let mut cache_guard = cache.lock().await;

    // Cache hit: mtime matches
    if let Some(cached) = cache_guard.get(file_path) {
        if current_mtime == Some(cached.mtime) {
            return Some(cached.content.clone());
        }
    }

    // Cache miss or stale: re-read from disk
    drop(cache_guard); // release lock during I/O

    match platform.fs().read_to_string(file_path).await {
        Ok(content) if !content.trim().is_empty() => {
            if let Some(mtime) = current_mtime {
                let mut cache_guard = cache.lock().await;
                cache_guard.insert(
                    file_path.to_path_buf(),
                    CachedFile {
                        content: content.clone(),
                        mtime,
                    },
                );
            }
            Some(content)
        }
        _ => None,
    }
}
```

Then update `build_system_prompt()` to call `load_cached_file()` instead of the inline `exists()` + `read_to_string()` chain:

```rust
for file_path in &candidates {
    debug!(file = %file_path.display(), "checking for bootstrap file");
    if let Some(content) = Self::load_cached_file(
        &self.platform,
        &self.bootstrap_cache,
        file_path,
    ).await {
        debug!(file = %filename, bytes = content.len(), "loaded bootstrap file");
        loaded_files.insert(*filename, content);
        break;
    }
}
```

**Note:** The `Platform` trait must expose `metadata()` returning modification time. If it does not currently have this method, an alternative is to use `tokio::fs::metadata()` directly for the mtime check (bypassing the platform abstraction for this one stat call) while keeping `read_to_string` through the platform. Document this gap for future cleanup.

### Task 3.5: Async `load_dir()` Migration (D11)

**File:** `crates/clawft-core/src/agent/skills_v2.rs`

**OVERLAP NOTE:** This task is identical to Element 04 Phase C3 Task 3.8. If C3 completes the `tokio::fs` migration first, this task is satisfied. Implementors MUST check C3 status before starting D11.

Convert `load_dir()` from sync to async:

**Before (blocking):**
```rust
fn load_dir(dir: &Path) -> Result<Vec<SkillDefinition>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(dir).map_err(ClawftError::Io)?;
    // ...
    match std::fs::metadata(&skill_md_path) { ... }
    match std::fs::read_to_string(&skill_md_path) { ... }
}
```

**After (async):**
```rust
async fn load_dir(dir: &Path) -> Result<Vec<SkillDefinition>> {
    if !tokio::fs::try_exists(dir).await.unwrap_or(false) {
        return Ok(Vec::new());
    }

    let mut entries = tokio::fs::read_dir(dir).await.map_err(ClawftError::Io)?;
    let mut skills = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(ClawftError::Io)? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // SEC-SKILL-02: Validate directory name (unchanged).
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_string_lossy();
        if let Err(e) = validate_directory_name(&dir_name_str) {
            warn!(
                path = %path.display(),
                error = %e,
                "rejected skill directory with unsafe name"
            );
            continue;
        }

        let skill_md_path = path.join("SKILL.md");
        let skill_json_path = path.join("skill.json");

        if tokio::fs::try_exists(&skill_md_path).await.unwrap_or(false) {
            // SEC-SKILL-07: Check file size before reading.
            match tokio::fs::metadata(&skill_md_path).await {
                Ok(meta) => {
                    if let Err(e) =
                        validate_file_size(meta.len() as usize, MAX_SKILL_MD_SIZE, "SKILL.md")
                    {
                        warn!(
                            path = %skill_md_path.display(),
                            error = %e,
                            "SKILL.md too large, skipping"
                        );
                        continue;
                    }
                }
                Err(e) => {
                    warn!(
                        path = %skill_md_path.display(),
                        error = %e,
                        "failed to stat SKILL.md, skipping"
                    );
                    continue;
                }
            }

            match tokio::fs::read_to_string(&skill_md_path).await {
                Ok(content) => match parse_skill_md(&content, Some(&skill_md_path)) {
                    Ok(skill) => {
                        debug!(skill = %skill.name, "loaded SKILL.md");
                        skills.push(skill);
                    }
                    Err(e) => {
                        warn!(
                            path = %skill_md_path.display(),
                            error = %e,
                            "failed to parse SKILL.md, skipping"
                        );
                    }
                },
                Err(e) => {
                    warn!(
                        path = %skill_md_path.display(),
                        error = %e,
                        "failed to read SKILL.md, skipping"
                    );
                }
            }
        } else if tokio::fs::try_exists(&skill_json_path).await.unwrap_or(false) {
            match load_legacy_skill_async(&skill_json_path, &path).await {
                Ok(skill) => {
                    debug!(skill = %skill.name, "loaded legacy skill.json");
                    skills.push(skill);
                }
                Err(e) => {
                    warn!(
                        path = %skill_json_path.display(),
                        error = %e,
                        "failed to load legacy skill, skipping"
                    );
                }
            }
        }
    }

    Ok(skills)
}
```

### Task 3.6: Async `load_legacy_skill()` Migration (D11)

**File:** `crates/clawft-core/src/agent/skills_v2.rs`, lines 566-590

```rust
async fn load_legacy_skill_async(json_path: &Path, skill_dir: &Path) -> Result<SkillDefinition> {
    let json_content = tokio::fs::read_to_string(json_path)
        .await
        .map_err(ClawftError::Io)?;

    let mut skill: SkillDefinition =
        serde_json::from_str(&json_content).map_err(|e| ClawftError::PluginLoadFailed {
            plugin: format!("legacy skill.json: {e}"),
        })?;

    skill.format = SkillFormat::Legacy;
    skill.source_path = Some(json_path.to_path_buf());

    let prompt_path = skill_dir.join("prompt.md");
    if tokio::fs::try_exists(&prompt_path).await.unwrap_or(false) {
        match tokio::fs::read_to_string(&prompt_path).await {
            Ok(prompt) => {
                skill.instructions = prompt;
            }
            Err(e) => {
                warn!(
                    path = %prompt_path.display(),
                    error = %e,
                    "failed to read prompt.md"
                );
            }
        }
    }

    Ok(skill)
}
```

### Task 3.7: Update `discover()` / `discover_with_trust()` Signatures (D11)

**File:** `crates/clawft-core/src/agent/skills_v2.rs`

Both `discover()` and `discover_with_trust()` must become `async fn` since they call `load_dir()`:

```rust
pub async fn discover(/* ... */) -> Result<Self> { ... }
pub async fn discover_with_trust(/* ... */) -> Result<Self> { ... }
```

**Call sites to update:**
- `crates/clawft-core/src/agent/loop_core.rs` -- agent startup
- `crates/clawft-cli/src/commands/` -- CLI commands that invoke skill discovery
- Any test code calling `discover()` must become `#[tokio::test]`

---

## 4. Concurrency Plan

### 4.1 D1 -- Tool Execution Parallelism

```
Before:  Tool_A(100ms) -> Tool_B(100ms) -> Tool_C(100ms) = 300ms total
After:   join_all(Tool_A, Tool_B, Tool_C) = ~100ms total
```

- Use `futures_util::future::join_all` -- no `tokio::spawn` needed
- Results collected in original order (join_all preserves iterator order)
- Each tool future is independent -- no shared mutable state between them
- `ToolRegistry` is `Arc`-wrapped so cloning for each future is cheap

### 4.2 D10 -- Cache Concurrency

- `BootstrapCache` is `Arc<Mutex<HashMap<PathBuf, CachedFile>>>`
- Lock held briefly for lookup (microseconds), released before I/O
- Re-acquired after I/O to insert updated entry
- TOCTOU race on cache insert is benign -- worst case, two concurrent calls both read and cache the same file; last write wins with identical content

### 4.3 D11 -- Async I/O Non-Blocking

- All `std::fs` calls replaced with `tokio::fs` equivalents
- `tokio::fs` operations run on the Tokio blocking thread pool, avoiding executor stalls
- `load_dir()` processes entries sequentially within a single directory (directory listing is inherently sequential on most filesystems)
- Cross-directory parallelism is handled at the `discover()` level (future optimization, not in D11 scope)

### 4.4 Cross-Item Independence

All three items modify different files and have no code-level dependencies:

```
D1:  loop_core.rs  (tool execution)
D10: context.rs    (bootstrap cache)
D11: skills_v2.rs  (async I/O)
```

They can be implemented and tested by separate developers in parallel branches.

---

## 5. Tests Required

### 5.1 D1 -- Parallel Tool Execution Tests

| Test | Type | Description |
|------|------|-------------|
| `parallel_tools_faster_than_sequential` | Timing | 3 tools each sleeping 100ms complete in < 200ms total. Use `tokio::time::sleep` mock tools. |
| `parallel_tools_preserve_order` | Correctness | Given tools [A, B, C], results appear in messages as [A_result, B_result, C_result] regardless of completion order. |
| `parallel_tool_error_does_not_cancel_others` | Error handling | Tool B fails; Tool A and C results still appended. Tool B result is `{"error": "..."}`. |
| `parallel_tools_single_tool_unchanged` | Regression | Single tool call still works (degenerate case of join_all with one future). |
| `parallel_tools_empty_vec` | Edge case | Zero tool calls produces zero messages (no panic on empty join_all). |
| `parallel_tools_permissions_forwarded` | Security | Each parallel tool receives the correct permissions from `auth_context`. |

### 5.2 D10 -- Bootstrap Cache Tests

| Test | Type | Description |
|------|------|-------------|
| `cache_hit_skips_disk_read` | Performance | Second `build_system_prompt()` call does not invoke `read_to_string()`. Mock platform to track call count. |
| `cache_miss_reads_from_disk` | Correctness | First call reads from disk and returns correct content. |
| `cache_invalidated_on_mtime_change` | Correctness | After modifying a bootstrap file (new mtime), next call re-reads from disk. |
| `cache_handles_deleted_file` | Edge case | Cached file deleted between calls; next call returns `None` for that file, other files unaffected. |
| `cache_handles_empty_file` | Edge case | File exists but is empty; not cached, not included in prompt. |
| `cache_concurrent_access` | Thread safety | Multiple concurrent `build_system_prompt()` calls do not panic or deadlock. |

### 5.3 D11 -- Async Skills I/O Tests

| Test | Type | Description |
|------|------|-------------|
| `async_load_dir_loads_skills` | Correctness | `load_dir()` loads SKILL.md and skill.json skills correctly (existing behavior preserved). |
| `async_load_dir_missing_dir` | Edge case | Non-existent directory returns empty vec without error. |
| `async_load_dir_security_checks` | Security | SEC-SKILL-02 (directory name validation) and SEC-SKILL-07 (file size) still enforced. |
| `async_load_legacy_skill` | Correctness | Legacy skill.json + prompt.md loaded correctly via async path. |
| `async_discover_with_trust` | Integration | Full async discovery chain works: built-in -> user -> workspace with trust gate. |
| `no_blocking_fs_calls_remain` | Static | Grep `skills_v2.rs` for `std::fs::` -- zero matches (enforced in CI or as a `#[test]`). |

### 5.4 Regression Tests

All existing tests in `loop_core.rs` (lines 413+) and `skills_v2.rs` (lines 598-1158) must continue to pass. The D11 migration requires converting sync tests to `#[tokio::test]` for any test that calls `load_dir()`, `discover()`, or `discover_with_trust()`.

---

## 6. Acceptance Criteria

### D1: Parallel Tool Execution
- [ ] `futures-util` dependency added to `clawft-core/Cargo.toml`
- [ ] Sequential `for` loop in `loop_core.rs:376-403` replaced with `join_all`
- [ ] Tool results appended in original `tool_call_id` order
- [ ] Tool errors do not cancel sibling tool executions
- [ ] Timing test: 3 x 100ms tools complete in < 200ms
- [ ] All existing `loop_core.rs` tests pass

### D10: Bootstrap Cache
- [ ] `CachedFile` struct defined with `content: String` and `mtime: SystemTime`
- [ ] `BootstrapCache` type alias added (`Arc<Mutex<HashMap<PathBuf, CachedFile>>>`)
- [ ] `ContextBuilder` gains `bootstrap_cache` field, initialized in `new()`
- [ ] `build_system_prompt()` checks cache before reading from disk
- [ ] Cache invalidated when file mtime changes
- [ ] Cache miss (first call) reads from disk and stores result
- [ ] Deleted/empty files handled gracefully
- [ ] All existing `context.rs` tests pass

### D11: Async File I/O in Skills Loader
- [ ] All `std::fs::read_dir` replaced with `tokio::fs::read_dir` in `load_dir()`
- [ ] All `std::fs::metadata` replaced with `tokio::fs::metadata` in `load_dir()`
- [ ] All `std::fs::read_to_string` replaced with `tokio::fs::read_to_string` in `load_dir()` and `load_legacy_skill()`
- [ ] All `std::fs::exists` / `.exists()` replaced with `tokio::fs::try_exists`
- [ ] `load_dir()` signature changed to `async fn`
- [ ] `load_legacy_skill()` converted to `async fn load_legacy_skill_async()`
- [ ] `discover()` and `discover_with_trust()` changed to `async fn`
- [ ] All call sites updated (loop_core.rs, CLI commands)
- [ ] Security validations SEC-SKILL-02 and SEC-SKILL-07 preserved
- [ ] All existing skills_v2.rs tests pass (converted to `#[tokio::test]` where needed)
- [ ] Zero `std::fs::` calls remain in `skills_v2.rs` (verified by grep)

---

## 7. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **D1: Same-file race conditions** -- Two parallel tools write the same file, producing corrupted output | Low | Medium | Document as known limitation. LLMs rarely issue conflicting writes in a single turn. Future mitigation: per-path advisory locks in `ToolRegistry`. |
| **D1: `ToolRegistry` not `Clone`** -- `self.tools.clone()` fails if `ToolRegistry` is not `Clone` | Medium | Low | `ToolRegistry` uses `Arc` internally. If `Clone` is not derived, pass `Arc<ToolRegistry>` into closures instead. Alternatively, use `&self.tools` with `join_all` on borrowed futures if lifetimes permit. |
| **D1: Permissions lifetime** -- Borrowing `request.auth_context` across async boundary | Medium | Low | Clone `permissions` into each future. The `Vec<String>` clone cost is negligible compared to tool execution time. |
| **D10: Platform trait lacks `metadata()`** -- Cannot check mtime through the platform abstraction | Medium | Medium | Fallback: use `tokio::fs::metadata()` directly for the stat call. Document as tech debt for platform trait expansion. |
| **D10: Stale cache on rapid file changes** -- File modified twice within mtime granularity (1s on some filesystems) | Very Low | Low | Accept as known limitation. Bootstrap files change at human speed, not programmatic speed. |
| **D11: Overlap with Element 04 C3** -- Both C3 and D11 modify the same functions in `skills_v2.rs` | High | Medium | Check C3 status before starting D11. If C3 is in progress, coordinate to avoid merge conflicts. If C3 is complete, D11 is done -- mark as satisfied. |
| **D11: `discover()` async signature breaks callers** -- Changing to `async fn` is a breaking internal API change | High | Low | All callers are within the workspace and can be updated. No external API surface affected. |
| **D11: Sync test helpers** -- Existing tests use sync `discover()` calls | Medium | Low | Convert tests to `#[tokio::test]` and add `.await`. Alternatively, keep a thin sync wrapper using `tokio::runtime::Handle::current().block_on()` for test convenience (not recommended for production code). |
