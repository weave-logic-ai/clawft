# Phase C4: Dynamic Loading & Hot-Reload / C4a: Autonomous Skill Creation

> **Element:** 04 -- Plugin & Skill System
> **Phase:** C4 (Week 6-7, P1) / C4a (Week 8+, P2 stretch)
> **Crates:** `clawft-core/src/agent/skill_watcher.rs` (new), `clawft-cli`
> **Depends on:** C2 (WASM Plugin Host), C3 (Skill Loader)
> **Blocks:** C4a blocks on C4; C6 (MCP Exposure) benefits from hot-reload integration
> **Status:** Planning

---

## 1. Overview

### C4: Hot-Reload (P1)

Phase C4 implements runtime skill loading via file-system watching. When a skill file is added, modified, or removed in any watched directory, the `SkillRegistry` updates automatically without requiring a process restart. C4 also introduces the `weft skill install/list/remove` CLI commands for skill management.

### C4a: Autonomous Skill Creation (P2, Stretch)

Phase C4a adds agent-driven skill creation: when the agent loop detects repeated task patterns (configurable threshold, default: 3 repetitions), it can auto-generate a SKILL.md, compile to WASM, and install into managed skills in a "pending" state requiring user approval. **Disabled by default, opt-in via config.**

---

## 2. Current Code

### 2.1 SkillRegistry (after C3)

After C3 completes, `SkillRegistry` will:
- Use `serde_yaml` for SKILL.md parsing
- Support four priority levels: workspace > managed > bundled > built-in
- Use `tokio::fs` for all I/O
- Be an owned, non-shared struct

C4 must make the registry shared and mutable at runtime.

### 2.2 Agent Loop Integration Point

**File:** `crates/clawft-core/src/agent/loop_core.rs`

The agent loop currently constructs `SkillRegistry` once at startup. C4 must replace this with a shared handle that the watcher can update.

### 2.3 CLI Commands Entry Point

**File:** `crates/clawft-cli/src/commands/`

New subcommands will be added under `weft skill`:
- `weft skill install <path>`
- `weft skill list`
- `weft skill remove <name>`

---

## 3. Implementation Tasks -- C4

### Task 4.1: Shared Registry with Arc<RwLock>

Wrap `SkillRegistry` for concurrent read access and exclusive write access:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared handle to the skill registry.
///
/// Multiple agent loop iterations can read concurrently.
/// The file-system watcher acquires a write lock to update.
pub type SharedSkillRegistry = Arc<RwLock<SkillRegistry>>;
```

Update `SkillRegistry` to support incremental updates:

```rust
impl SkillRegistry {
    /// Insert or replace a skill. Returns the previous skill if overwritten.
    pub fn upsert(&mut self, skill: SkillDefinition) -> Option<SkillDefinition> {
        self.skills.insert(skill.name.clone(), skill)
    }

    /// Remove a skill by name. Returns the removed skill if it existed.
    pub fn remove(&mut self, name: &str) -> Option<SkillDefinition> {
        self.skills.remove(name)
    }

    /// Rebuild the registry from all sources.
    /// Called after a file-system change to re-apply priority ordering.
    pub async fn rebuild(
        &mut self,
        workspace_dir: Option<&Path>,
        user_dir: Option<&Path>,
        bundled_dirs: Vec<&Path>,
        builtin_skills: Vec<SkillDefinition>,
        trust_workspace: bool,
    ) -> Result<()> {
        let fresh = Self::discover_with_trust(
            workspace_dir, user_dir, bundled_dirs, builtin_skills, trust_workspace
        ).await?;
        self.skills = fresh.skills;
        Ok(())
    }
}
```

### Task 4.2: File-System Watcher (`skill_watcher.rs`)

**New file:** `crates/clawft-core/src/agent/skill_watcher.rs`

```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

/// Configuration for the skill file-system watcher.
pub struct SkillWatcherConfig {
    /// Directories to watch (workspace, user, managed, bundled).
    pub watch_dirs: Vec<PathBuf>,
    /// Debounce duration for rapid file changes.
    pub debounce: Duration,
}

impl Default for SkillWatcherConfig {
    fn default() -> Self {
        Self {
            watch_dirs: Vec::new(),
            debounce: Duration::from_millis(500),
        }
    }
}

/// Watches skill directories for changes and triggers registry rebuilds.
pub struct SkillWatcher {
    config: SkillWatcherConfig,
    registry: SharedSkillRegistry,
    /// Channel to signal shutdown.
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl SkillWatcher {
    pub fn new(config: SkillWatcherConfig, registry: SharedSkillRegistry) -> Self {
        Self {
            config,
            registry,
            shutdown_tx: None,
        }
    }

    /// Start watching. Returns a handle to stop the watcher.
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(100);

        // Create OS file watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = event_tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )?;

        // Watch all configured directories
        for dir in &self.config.watch_dirs {
            if dir.exists() {
                watcher.watch(dir, RecursiveMode::Recursive)?;
            }
        }

        let registry = self.registry.clone();
        let debounce = self.config.debounce;

        // Spawn the event processing loop
        tokio::spawn(async move {
            let mut debounce_timer: Option<tokio::time::Instant> = None;

            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                                // Reset debounce timer on each change
                                debounce_timer = Some(tokio::time::Instant::now());
                            }
                            _ => {}
                        }
                    }
                    _ = async {
                        if let Some(timer) = debounce_timer {
                            tokio::time::sleep_until(timer + debounce).await;
                        } else {
                            // No pending changes, wait indefinitely
                            std::future::pending::<()>().await;
                        }
                    } => {
                        // Debounce period elapsed, rebuild registry
                        debounce_timer = None;
                        let mut reg = registry.write().await;
                        if let Err(e) = reg.rebuild(/* ... */).await {
                            tracing::warn!(error = %e, "skill registry rebuild failed");
                        } else {
                            tracing::info!("skill registry reloaded after file change");
                        }
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the watcher gracefully.
    pub fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
```

### Task 4.3: Skill Precedence Layering

The precedence rules from C3 are enforced during `rebuild()`:

| Priority | Source | Directory | Override Behavior |
|----------|--------|-----------|-------------------|
| 1 (highest) | Workspace | `.clawft/skills/` | Overrides all lower levels |
| 2 | Managed/Local | `~/.clawft/skills/` | Overrides bundled + built-in |
| 3 | Bundled | Plugin manifest `"skills"` dirs | Overrides built-in |
| 4 (lowest) | Built-in | Compiled into binary | Base layer |

**Override semantics:**
- Same-name skill at a higher level completely replaces the lower-level version
- Removing a skill at a higher level reveals the next-lower-level version (rebuild reconstructs from scratch)
- No partial merge -- entire `SkillDefinition` is replaced

### Task 4.4: Plugin-Shipped Skills

Plugins declare skill directories in their manifest:

```json
{
    "name": "my-plugin",
    "version": "1.0.0",
    "skills": ["skills/"]
}
```

When the plugin host loads a plugin, it reads the `skills` field and adds those directories to the watcher's bundled skill paths. Skills from plugin manifests participate in normal precedence (below workspace and user, above built-in).

**Integration point:** `PluginHost` (from C2/C7) passes plugin skill directories to `SkillWatcher` at startup.

### Task 4.5: Atomic Skill Swap

In-flight tool calls must not be interrupted by a skill reload. The atomic swap strategy:

1. New skill version loads alongside old version
2. New calls route to the new version
3. In-flight calls on the old version complete naturally
4. Old version is dropped when all references are released

Implementation via `Arc<SkillDefinition>`:

```rust
pub struct SkillRegistry {
    skills: HashMap<String, Arc<SkillDefinition>>,
}
```

When a tool call starts, it clones the `Arc<SkillDefinition>`. The registry can replace its entry without invalidating the Arc held by the in-flight call.

### Task 4.6: `weft skill install <path>` CLI Command

**File:** `crates/clawft-cli/src/commands/skill.rs` (new)

```
weft skill install <path>
```

Behavior:
1. Validate the source path exists and contains a valid `SKILL.md`
2. Parse the SKILL.md to extract the skill name
3. Validate the SKILL.md passes all security checks (SEC-SKILL-01 through SEC-SKILL-07)
4. Copy the entire skill directory to `~/.clawft/skills/<name>/`
5. If a skill with the same name already exists at user level, prompt for overwrite confirmation
6. The file-system watcher detects the new directory and hot-reloads automatically

```rust
/// Install a skill from a local directory path.
pub async fn skill_install(path: &Path) -> Result<()> {
    // 1. Validate source
    let skill_md_path = path.join("SKILL.md");
    if !skill_md_path.exists() {
        return Err(anyhow!("No SKILL.md found at {}", path.display()));
    }

    let content = tokio::fs::read_to_string(&skill_md_path).await?;
    let skill = parse_skill_md(&content, Some(&skill_md_path))?;

    // 2. Determine target
    let user_skills_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("cannot determine home directory"))?
        .join(".clawft/skills")
        .join(&skill.name);

    // 3. Copy
    tokio::fs::create_dir_all(&user_skills_dir).await?;
    copy_dir_recursive(path, &user_skills_dir).await?;

    println!("Installed skill '{}' to {}", skill.name, user_skills_dir.display());
    Ok(())
}
```

### Task 4.7: `weft skill list` CLI Command

```
weft skill list [--format json|table]
```

Lists all discovered skills with their source, priority level, version, and format (SKILL.md / Legacy / WASM).

### Task 4.8: `weft skill remove <name>` CLI Command

```
weft skill remove <name>
```

Removes a user-installed skill from `~/.clawft/skills/<name>/`. Does not remove workspace or built-in skills. Prompts for confirmation.

---

## 4. Implementation Tasks -- C4a (Stretch)

### Task 4a.1: Pattern Detection

**File:** `crates/clawft-core/src/agent/pattern_detector.rs` (new)

Track repeated task patterns in the agent loop:

```rust
pub struct PatternDetector {
    /// Minimum repetitions before suggesting skill creation.
    threshold: usize,
    /// Observed patterns: hash(task_signature) -> count.
    patterns: HashMap<u64, PatternEntry>,
}

struct PatternEntry {
    count: usize,
    first_seen: Instant,
    last_seen: Instant,
    /// Representative task descriptions for skill generation.
    exemplars: Vec<String>,
}

impl PatternDetector {
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            patterns: HashMap::new(),
        }
    }

    /// Record a task execution. Returns Some(exemplars) if threshold reached.
    pub fn record(&mut self, task_signature: &str) -> Option<Vec<String>> {
        let hash = hash_signature(task_signature);
        let entry = self.patterns.entry(hash).or_insert_with(|| PatternEntry {
            count: 0,
            first_seen: Instant::now(),
            last_seen: Instant::now(),
            exemplars: Vec::new(),
        });
        entry.count += 1;
        entry.last_seen = Instant::now();
        if entry.exemplars.len() < 5 {
            entry.exemplars.push(task_signature.to_string());
        }
        if entry.count >= self.threshold {
            Some(entry.exemplars.clone())
        } else {
            None
        }
    }
}
```

**Configuration:**
- Threshold: configurable, default: 3 repetitions
- Controlled by `clawft.toml` or environment variable
- **CRITICAL:** Disabled by default. Must be explicitly enabled:

```toml
[agent.autonomous_skills]
enabled = false  # Default: false. Must be explicitly set to true.
threshold = 3
```

### Task 4a.2: Skill Generation

When the pattern detector fires, generate a SKILL.md from the exemplar tasks:

1. Extract common structure from exemplar task descriptions
2. Identify variable parts (template parameters)
3. Generate SKILL.md frontmatter with:
   - Extracted name (from pattern)
   - Generated description
   - Variable list
   - **Minimal allowed-tools** (no shell, no network, workspace-only filesystem)
4. Generate instruction body from exemplar patterns

The generated SKILL.md must pass the same `parse_skill_md()` validation as manually authored skills.

### Task 4a.3: WASM Compilation (Optional)

If the generated skill includes executable logic (not just LLM instructions), compile to WASM:

1. Generate Rust source implementing the `Skill` trait
2. Compile to WASM target
3. Verify size limits (< 300KB uncompressed)
4. Package alongside SKILL.md

This step is optional for C4a -- many auto-generated skills will be LLM-instruction-only.

### Task 4a.4: Managed Install with Approval

Auto-generated skills install into `~/.clawft/skills/` in a "pending" state:

```rust
pub enum SkillInstallState {
    /// Active and available for use.
    Active,
    /// Installed but awaiting user approval.
    Pending,
    /// Explicitly disabled by user.
    Disabled,
}
```

The registry does NOT load `Pending` skills into the active set. The user must approve:

```
[Auto-Skill] Detected repeated pattern: "summarize document"
  Generated skill: summarize-doc (v0.1.0)
  Permissions: read-only filesystem (workspace only)
  Approve? [y/N]
```

### Task 4a.5: Permission Enforcement

Auto-generated skills are restricted to minimal permissions:

| Permission | Value | Rationale |
|-----------|-------|-----------|
| Shell execution | **Denied** | Prevents arbitrary command execution |
| Network access | **Denied** | Prevents data exfiltration |
| Filesystem read | Workspace only | Limited scope |
| Filesystem write | **Denied** | Read-only by default |
| Allowed tools | Explicitly listed only | No wildcard tool access |

These permissions are enforced by the WASM sandbox (C2) and the skill's `allowed-tools` list.

---

## 5. Concurrency Plan

### 5.1 Watcher Event Loop

The `SkillWatcher` runs as a background `tokio::spawn` task. It:
- Receives events from the `notify` crate via `mpsc::channel`
- Debounces rapid changes (500ms default)
- Acquires a write lock on `SharedSkillRegistry` only during rebuild
- Releases the write lock immediately after rebuild completes

### 5.2 Read/Write Lock Contention

Expected pattern:
- Reads: Every agent loop iteration reads skills (high frequency)
- Writes: Only on file-system changes (low frequency)

`tokio::sync::RwLock` is appropriate: many concurrent readers, rare exclusive writers. Write starvation is not a concern given low write frequency.

### 5.3 Atomic Swap via Arc

`Arc<SkillDefinition>` ensures in-flight calls hold a reference to the old version while the registry swaps to the new version. No synchronization needed beyond the existing `RwLock` on the registry itself.

### 5.4 Watcher Shutdown

The watcher uses a `oneshot` channel for graceful shutdown. On agent shutdown:
1. Send shutdown signal via oneshot
2. Watcher event loop breaks
3. `notify::Watcher` is dropped, releasing OS resources

---

## 6. Dependencies

### Crate Dependencies

| Dependency | Version | Purpose | New? |
|-----------|---------|---------|------|
| `notify` | 6.x | Cross-platform filesystem event watching | Yes |
| `tokio` | existing | Async runtime, RwLock, spawn, channels | No |
| `dirs` | existing or new | Home directory resolution for `~/.clawft/` | Check |

### Internal Dependencies

| Dependency | Status | Notes |
|-----------|--------|-------|
| C2 (WASM Plugin Host) | Required | WASM skill execution, plugin manifest parsing |
| C3 (Skill Loader) | Required | `SkillRegistry`, `parse_skill_md()`, async discover |
| `clawft-types::SkillDefinition` | Existing | Wrapped in `Arc` for atomic swap |
| `clawft-core::security` | Existing | Validation for `weft skill install` |

---

## 7. Tests Required

### 7.1 C4 Tests

```rust
#[tokio::test]
async fn test_watcher_detects_new_skill() {
    // Create a watched directory
    // Start watcher
    // Add a SKILL.md to the directory
    // Verify registry contains the new skill within 2 seconds
}

#[tokio::test]
async fn test_watcher_detects_skill_modification() {
    // Start with a skill in the watched directory
    // Modify the SKILL.md (change description)
    // Verify registry reflects the updated description
}

#[tokio::test]
async fn test_watcher_detects_skill_removal() {
    // Start with a skill in the watched directory
    // Remove the skill directory
    // Verify registry no longer contains the skill
}

#[tokio::test]
async fn test_watcher_debounce() {
    // Rapidly modify a skill 10 times within 200ms
    // Verify registry is rebuilt only once (after debounce window)
}

#[tokio::test]
async fn test_precedence_on_reload() {
    // Set up workspace and user skills with the same name
    // Remove the workspace skill
    // Verify the user skill is now active (lower-level revealed)
}

#[tokio::test]
async fn test_atomic_swap_inflight_call() {
    // Start a long-running skill call (hold Arc<SkillDefinition>)
    // Trigger a reload that replaces the skill
    // Verify the in-flight call still sees the old version
    // Verify new calls see the new version
}

#[tokio::test]
async fn test_skill_install_cli() {
    // Create a valid skill directory in a temp location
    // Run `skill_install(path)`
    // Verify skill copied to ~/.clawft/skills/<name>/
    // Verify SKILL.md is valid
}

#[tokio::test]
async fn test_skill_install_invalid_rejected() {
    // Create a directory without SKILL.md
    // Run `skill_install(path)`
    // Verify error returned
}

#[tokio::test]
async fn test_skill_list() {
    // Discover skills from multiple sources
    // Verify `skill_list()` returns all skills with correct metadata
}

#[tokio::test]
async fn test_skill_remove() {
    // Install a user skill
    // Run `skill_remove(name)`
    // Verify directory deleted
    // Verify skill no longer in registry after watcher fires
}

#[tokio::test]
async fn test_plugin_shipped_skills() {
    // Create a plugin manifest with skills directory
    // Load plugin
    // Verify plugin skills appear in registry at bundled priority
}
```

### 7.2 C4a Tests

```rust
#[test]
fn test_pattern_detector_threshold() {
    let mut detector = PatternDetector::new(3);
    assert!(detector.record("summarize document").is_none());
    assert!(detector.record("summarize document").is_none());
    // Third time triggers
    assert!(detector.record("summarize document").is_some());
}

#[test]
fn test_pattern_detector_different_patterns() {
    let mut detector = PatternDetector::new(3);
    detector.record("summarize document");
    detector.record("translate text");
    detector.record("summarize document");
    // Different pattern, no trigger
    assert!(detector.record("translate text").is_none());
    // Same pattern, third time triggers
    assert!(detector.record("summarize document").is_some());
}

#[test]
fn test_pattern_detector_configurable_threshold() {
    let mut detector = PatternDetector::new(5);
    for _ in 0..4 {
        assert!(detector.record("task").is_none());
    }
    assert!(detector.record("task").is_some());
}

#[test]
fn test_generated_skill_passes_validation() {
    // Generate a SKILL.md from exemplar patterns
    // Verify parse_skill_md() succeeds
    // Verify security checks pass
}

#[test]
fn test_generated_skill_minimal_permissions() {
    // Generate a skill
    // Verify allowed-tools does NOT include Bash, shell, or network tools
    // Verify metadata contains permission restrictions
}

#[test]
fn test_pending_skill_not_loaded() {
    // Install a skill in Pending state
    // Verify registry does NOT include it in active skills
}

#[test]
fn test_approved_skill_becomes_active() {
    // Install a skill in Pending state
    // Approve it
    // Verify registry now includes it
}

#[test]
fn test_autonomous_disabled_by_default() {
    // Create a default config
    // Verify autonomous_skills.enabled is false
}
```

---

## 8. Acceptance Criteria

### C4 (P1)

- [ ] `notify` crate dependency added
- [ ] `SkillWatcher` watches all skill directories (workspace, user, managed, bundled)
- [ ] File changes detected and debounced (default: 500ms, configurable)
- [ ] Registry rebuild triggered within 2 seconds of a file change
- [ ] `SharedSkillRegistry` (`Arc<RwLock<SkillRegistry>>`) used in agent loop
- [ ] Skill precedence maintained on reload: workspace > managed > bundled > built-in
- [ ] Removing a higher-level skill reveals the next-lower-level version
- [ ] Plugin-shipped skills loaded from plugin manifest `"skills"` field
- [ ] Atomic swap via `Arc<SkillDefinition>` -- in-flight calls uninterrupted
- [ ] `weft skill install <path>` -- validates, copies to `~/.clawft/skills/`, auto-detected by watcher
- [ ] `weft skill list` -- lists all skills with source, priority, version, format
- [ ] `weft skill remove <name>` -- removes user-installed skills, prompts for confirmation
- [ ] Watcher shuts down gracefully on agent exit

### C4a (P2, Stretch)

- [ ] Pattern detection threshold configurable (default: 3 repetitions)
- [ ] Autonomous skill creation disabled by default, opt-in via `clawft.toml`
- [ ] Generated SKILL.md passes same `parse_skill_md()` validation as manual skills
- [ ] Generated skills have minimal permissions (no shell, no network, workspace-only filesystem)
- [ ] Auto-generated skills install in "pending" state
- [ ] User prompted for approval before activation
- [ ] Approved skills become active; rejected skills are removed
- [ ] Pattern detector does not leak memory (bounded pattern history)

---

## 9. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `notify` crate platform differences (Linux inotify vs macOS FSEvents vs Windows) | Medium | Medium | Use `RecommendedWatcher` which selects the best backend per platform. Test on all CI platforms. |
| File watcher event storm on bulk operations (e.g., git checkout) | Medium | Low | Debounce at 500ms. Consider adding a "pause watcher" command for bulk operations. |
| RwLock write starvation under high read load | Low | Low | Tokio's `RwLock` is write-preferring by default. Rebuilds are fast (< 100ms for typical skill counts). |
| Atomic swap memory leak (old Arc<SkillDefinition> never dropped) | Low | Medium | Arc drops when last reference is released. In-flight calls are short-lived. Add monitoring for Arc strong_count. |
| C4a pattern detection false positives | Medium | Low | Require user approval for all auto-generated skills. High threshold (default: 3) reduces false positives. |
| C4a generated skills bypass security | Low | Critical | All generated skills go through the same `parse_skill_md()` + security validation pipeline. Minimal permissions enforced. User approval gate prevents activation of unsafe skills. |
| Week 6-7 schedule overlap with C5/C6 | Medium | Medium | C4 core (watcher + CLI) must complete before C6 starts (C6 needs hot-reload for MCP tool list updates). C4a deferred to Week 8+ as stretch. |
