# SPARC Task: Per-Agent Workspace Isolation + Timestamp Standardization

| Field | Value |
|-------|-------|
| **Element** | 08 -- Memory & Workspace |
| **Phase** | H1 (Per-Agent Workspace Isolation) + H3 (Timestamp Standardization) |
| **Timeline** | Weeks 4-6 |
| **Priority** | High (H1 blocks L2 routing in Element 09; H3 is a low-risk type cleanup) |
| **Crates** | `clawft-core` (workspace.rs), `clawft-types` (workspace.rs, cron.rs) |
| **Dependencies** | None -- H1/H3 have no upstream blockers |
| **Blocks** | L2 (Element 09 multi-agent routing calls `ensure_agent_workspace`), Contract 3.1 (plugin `tool_state/`) |
| **Status** | Planning |

---

## Overview

This phase delivers two tightly-related changes:

1. **H1 -- Per-Agent Workspace Isolation**: Extend `WorkspaceManager` to create, manage, and enumerate per-agent workspace directories under `~/.clawft/agents/<agentId>/`. Each agent gets its own `SOUL.md`, `AGENTS.md`, `USER.md`, `config.toml`, and subdirectories (`sessions/`, `memory/`, `skills/`, `tool_state/`). A symlink-based cross-agent shared memory protocol provides controlled read/write access between agents.

2. **H3 -- Timestamp Standardization**: Replace all `Option<String>` and `i64` millisecond timestamp fields with `DateTime<Utc>` from `chrono`. This affects `WorkspaceEntry` in `clawft-types` and `CronJob`/`CronJobState` in `clawft-types`. A backward-compatible serde deserializer handles existing JSON files.

---

## Current Code

### WorkspaceManager (clawft-core/src/workspace.rs)

**Subdirectory constants** (workspace.rs:73):
```rust
const WORKSPACE_SUBDIRS: &[&str] = &["sessions", "memory", "skills", "agents", "hooks"];
```

**WorkspaceManager struct** (workspace.rs:76-82):
```rust
pub struct WorkspaceManager {
    /// Path to the global registry file (`~/.clawft/workspaces.json`).
    registry_path: PathBuf,

    /// In-memory copy of the registry.
    registry: WorkspaceRegistry,
}
```

**create() method** (workspace.rs:129-166):
```rust
pub fn create(&mut self, name: &str, parent_dir: &Path) -> Result<PathBuf> {
    let ws_root = parent_dir.join(name);
    let dot_clawft = ws_root.join(".clawft");

    // Create .clawft/ and subdirectories
    for subdir in WORKSPACE_SUBDIRS {
        std::fs::create_dir_all(dot_clawft.join(subdir))?;
    }

    // Create config.json
    std::fs::write(dot_clawft.join("config.json"), "{}\n")?;

    // Create MEMORY.md and HISTORY.md (empty)
    std::fs::write(dot_clawft.join("MEMORY.md"), "")?;
    std::fs::write(dot_clawft.join("HISTORY.md"), "")?;

    // Create CLAWFT.md
    let clawft_md = format!(
        "# {name}\n\n\
         Workspace created by clawft.\n\n\
         ## Configuration\n\n\
         Edit `.clawft/config.json` to customize this workspace.\n"
    );
    std::fs::write(ws_root.join("CLAWFT.md"), clawft_md)?;

    // Register in global registry
    let now = chrono::Utc::now().to_rfc3339();
    let entry = WorkspaceEntry {
        name: name.into(),
        path: ws_root.clone(),
        last_accessed: Some(now.clone()),
        created_at: Some(now),
    };
    self.registry.register(entry);
    self.save_registry()?;

    Ok(ws_root)
}
```

### WorkspaceEntry (clawft-types/src/workspace.rs:11-26)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// Human-readable workspace name (unique within registry).
    pub name: String,

    /// Absolute path to the workspace root directory.
    pub path: PathBuf,

    /// ISO 8601 timestamp of the last time this workspace was accessed.
    #[serde(default)]
    pub last_accessed: Option<String>,

    /// ISO 8601 timestamp of when the workspace was first created.
    #[serde(default)]
    pub created_at: Option<String>,
}
```

### CronJob (clawft-types/src/cron.rs:139-174)

```rust
pub struct CronJob {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub schedule: CronSchedule,
    #[serde(default)]
    pub payload: CronPayload,
    #[serde(default)]
    pub state: CronJobState,

    /// Creation timestamp in milliseconds since epoch.
    #[serde(default)]
    pub created_at_ms: i64,

    /// Last update timestamp in milliseconds since epoch.
    #[serde(default)]
    pub updated_at_ms: i64,

    #[serde(default)]
    pub delete_after_run: bool,
}
```

### CronJobState (clawft-types/src/cron.rs:119-136)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronJobState {
    /// Next scheduled run time in milliseconds since epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_run_at_ms: Option<i64>,

    /// Last actual run time in milliseconds since epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at_ms: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_status: Option<JobStatus>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}
```

### InboundMessage (clawft-types/src/event.rs:17-41) -- Already Correct

```rust
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,

    /// When the message was received.
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,

    #[serde(default)]
    pub media: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Session (clawft-types/src/session.rs:19-42) -- Already Correct

```rust
pub struct Session {
    pub key: String,
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub last_consolidated: usize,
}
```

---

## Implementation Tasks

### H1: Per-Agent Workspace Isolation

#### Task H1.1: Add Agent Workspace Constants and Subdirectory List

**File**: `crates/clawft-core/src/workspace.rs`

Add a constant for per-agent subdirectories below the existing `WORKSPACE_SUBDIRS`:

```rust
/// Subdirectories created inside each agent workspace (`~/.clawft/agents/<agentId>/`).
const AGENT_WORKSPACE_SUBDIRS: &[&str] = &["sessions", "memory", "skills", "tool_state"];
```

#### Task H1.2: Add `agents_root()` Helper

**File**: `crates/clawft-core/src/workspace.rs`

Add a helper to `WorkspaceManager` that resolves the agents root directory:

```rust
impl WorkspaceManager {
    /// Returns the agents root directory: `~/.clawft/agents/`.
    fn agents_root(&self) -> Result<PathBuf> {
        // registry_path is ~/.clawft/workspaces.json, so parent is ~/.clawft/
        let dot_clawft = self.registry_path.parent().ok_or_else(|| ClawftError::ConfigInvalid {
            reason: "registry path has no parent directory".into(),
        })?;
        Ok(dot_clawft.join("agents"))
    }
}
```

#### Task H1.3: Implement `ensure_agent_workspace()`

**File**: `crates/clawft-core/src/workspace.rs`

This is the primary method called by L2 routing (Element 09) on agent dispatch. Idempotent: creates the workspace if it does not exist, returns the path if it does.

```rust
impl WorkspaceManager {
    /// Ensure an agent workspace exists at `~/.clawft/agents/<agent_id>/`.
    ///
    /// Creates the directory structure and default files if they do not exist.
    /// Idempotent: calling multiple times for the same agent_id is safe.
    ///
    /// Returns the absolute path to the agent workspace root.
    pub fn ensure_agent_workspace(&self, agent_id: &str) -> Result<PathBuf> {
        Self::validate_agent_id(agent_id)?;
        let agent_root = self.agents_root()?.join(agent_id);

        if agent_root.is_dir() {
            return Ok(agent_root);
        }

        self.create_agent_workspace_at(&agent_root, agent_id, None)
    }

    /// Validate that an agent_id is safe for use as a directory name.
    ///
    /// Rejects empty strings, path separators, `.`, `..`, and non-ASCII control chars.
    fn validate_agent_id(agent_id: &str) -> Result<()> {
        if agent_id.is_empty()
            || agent_id == "."
            || agent_id == ".."
            || agent_id.contains('/')
            || agent_id.contains('\\')
            || agent_id.contains('\0')
            || agent_id.chars().any(|c| c.is_control())
        {
            return Err(ClawftError::ConfigInvalid {
                reason: format!("invalid agent_id for workspace: {agent_id:?}"),
            });
        }
        Ok(())
    }

    /// Internal: create agent workspace at a given path.
    fn create_agent_workspace_at(
        &self,
        agent_root: &Path,
        agent_id: &str,
        template: Option<&Path>,
    ) -> Result<PathBuf> {
        // Create subdirectories
        for subdir in AGENT_WORKSPACE_SUBDIRS {
            std::fs::create_dir_all(agent_root.join(subdir))?;
        }

        // Create default files (only if they don't already exist)
        let soul_md = agent_root.join("SOUL.md");
        if !soul_md.exists() {
            std::fs::write(&soul_md, format!("# Agent: {agent_id}\n"))?;
        }

        let agents_md = agent_root.join("AGENTS.md");
        if !agents_md.exists() {
            std::fs::write(&agents_md, "")?;
        }

        let user_md = agent_root.join("USER.md");
        if !user_md.exists() {
            std::fs::write(&user_md, "")?;
        }

        let config_toml = agent_root.join("config.toml");
        if !config_toml.exists() {
            let default_config = format!(
                "# Per-agent configuration overrides for {agent_id}\n\
                 # Shared memory namespaces (read-only exports)\n\
                 # shared_namespaces = []\n\
                 #\n\
                 # Import namespaces from other agents\n\
                 # [[import_namespaces]]\n\
                 # agent = \"other-agent\"\n\
                 # namespace = \"shared-context\"\n\
                 # read_write = false\n"
            );
            std::fs::write(&config_toml, default_config)?;
        }

        // If a template is provided, copy its contents over
        if let Some(template_path) = template {
            Self::apply_template(agent_root, template_path)?;
        }

        // Set directory permissions to 0700 (owner-only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(agent_root, perms)?;
        }

        Ok(agent_root.to_path_buf())
    }

    /// Copy template files into an agent workspace (non-recursive, top-level only).
    fn apply_template(agent_root: &Path, template_path: &Path) -> Result<()> {
        if !template_path.is_dir() {
            return Err(ClawftError::ConfigInvalid {
                reason: format!("template path is not a directory: {}", template_path.display()),
            });
        }
        for entry in std::fs::read_dir(template_path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                let dest = agent_root.join(entry.file_name());
                std::fs::copy(entry.path(), dest)?;
            }
        }
        Ok(())
    }
}
```

#### Task H1.4: Implement `create_agent_workspace()`

**File**: `crates/clawft-core/src/workspace.rs`

Explicit creation with optional template, fails if workspace already exists:

```rust
impl WorkspaceManager {
    /// Create a new agent workspace. Fails if it already exists.
    ///
    /// Optionally applies a template directory whose files are copied
    /// into the new workspace root.
    pub fn create_agent_workspace(
        &self,
        agent_id: &str,
        template: Option<&Path>,
    ) -> Result<PathBuf> {
        Self::validate_agent_id(agent_id)?;
        let agent_root = self.agents_root()?.join(agent_id);

        if agent_root.exists() {
            return Err(ClawftError::ConfigInvalid {
                reason: format!("agent workspace already exists: {agent_id}"),
            });
        }

        self.create_agent_workspace_at(&agent_root, agent_id, template)
    }
}
```

#### Task H1.5: Implement `delete_agent_workspace()`

**File**: `crates/clawft-core/src/workspace.rs`

```rust
impl WorkspaceManager {
    /// Delete an agent workspace and all its contents.
    ///
    /// Returns an error if the workspace does not exist.
    pub fn delete_agent_workspace(&self, agent_id: &str) -> Result<()> {
        Self::validate_agent_id(agent_id)?;
        let agent_root = self.agents_root()?.join(agent_id);

        if !agent_root.is_dir() {
            return Err(ClawftError::ConfigInvalid {
                reason: format!("agent workspace not found: {agent_id}"),
            });
        }

        // Validate the path is actually under agents_root (prevent path traversal)
        let canonical_agents_root = self.agents_root()?.canonicalize().map_err(|e| {
            ClawftError::ConfigInvalid {
                reason: format!("cannot canonicalize agents root: {e}"),
            }
        })?;
        let canonical_agent = agent_root.canonicalize().map_err(|e| {
            ClawftError::ConfigInvalid {
                reason: format!("cannot canonicalize agent workspace: {e}"),
            }
        })?;
        if !canonical_agent.starts_with(&canonical_agents_root) {
            return Err(ClawftError::ConfigInvalid {
                reason: format!("agent workspace path escapes agents root: {agent_id}"),
            });
        }

        std::fs::remove_dir_all(&agent_root)?;
        Ok(())
    }
}
```

#### Task H1.6: Implement `list_agent_workspaces()`

**File**: `crates/clawft-core/src/workspace.rs`

```rust
impl WorkspaceManager {
    /// List all agent workspace IDs.
    ///
    /// Returns directory names under `~/.clawft/agents/`.
    pub fn list_agent_workspaces(&self) -> Result<Vec<String>> {
        let agents_root = self.agents_root()?;
        if !agents_root.is_dir() {
            return Ok(Vec::new());
        }

        let mut agent_ids = Vec::new();
        for entry in std::fs::read_dir(&agents_root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    agent_ids.push(name.to_string());
                }
            }
        }
        agent_ids.sort();
        Ok(agent_ids)
    }
}
```

---

### Cross-Agent Shared Memory Protocol

The shared memory protocol enables controlled cross-agent memory access via the agent's `config.toml`.

#### Protocol Definition

**Exporting agent** (Agent A's `config.toml`):
```toml
# Namespaces this agent exports for other agents to read
shared_namespaces = ["project-context", "research-notes"]
```

**Importing agent** (Agent B's `config.toml`):
```toml
[[import_namespaces]]
agent = "agent-a"
namespace = "project-context"
read_write = false   # default: false (read-only)
```

#### Implementation Details

**File**: `crates/clawft-core/src/workspace.rs` (new function)

```rust
impl WorkspaceManager {
    /// Set up cross-agent shared memory symlinks for an agent.
    ///
    /// Reads the agent's `config.toml`, resolves `import_namespaces`,
    /// and creates symlinks from the agent's `memory/imports/<agent>/<namespace>`
    /// to the source agent's `memory/<namespace>` directory.
    ///
    /// **Security invariants**:
    /// - Symlink targets are validated to be under `~/.clawft/agents/`
    /// - Source agent must have the namespace listed in `shared_namespaces`
    /// - Write access requires explicit `read_write = true` in both the
    ///   importer's config AND the exporter's config
    pub fn setup_shared_memory(&self, agent_id: &str) -> Result<()> {
        Self::validate_agent_id(agent_id)?;
        let agent_root = self.agents_root()?.join(agent_id);
        let config_path = agent_root.join("config.toml");

        if !config_path.exists() {
            return Ok(()); // No config = no imports
        }

        let config_str = std::fs::read_to_string(&config_path)?;
        // Parse import_namespaces from TOML
        // For each import:
        //   1. Validate source agent_id
        //   2. Check source agent's shared_namespaces includes the namespace
        //   3. Create memory/imports/<source_agent>/ directory
        //   4. Create symlink: memory/imports/<source_agent>/<namespace> -> <source>/memory/<namespace>
        //   5. If read_write = false, mark symlink directory as read-only

        // Implementation deferred to coding phase -- this documents the contract.
        let _ = config_str; // placeholder
        Ok(())
    }
}
```

#### Filesystem Layout After Import

```
~/.clawft/agents/agent-b/
  memory/
    local-namespace/          # Agent B's own memory
    imports/
      agent-a/
        project-context/      # Symlink -> ~/.clawft/agents/agent-a/memory/project-context/
```

#### Consistency Model

| Access Pattern | Guarantee |
|---------------|-----------|
| Owner reads own memory | Read-your-writes (immediate) |
| Importer reads shared namespace | Eventual consistency (symlink target updates are atomic at filesystem level) |
| Writer with `read_write = true` | Requires fs-level locking (`flock` / `fcntl`) to prevent concurrent corruption |

#### Security Constraints

- Workspace directories: `0700` (owner-only on Unix)
- Symlink targets validated via `canonicalize()` -- must resolve under `~/.clawft/agents/`
- No path escape: agent_id validated against `/`, `\`, `..`, null bytes
- Write access is double opt-in: exporter must list namespace in `shared_namespaces`, importer must set `read_write = true`

---

### H3: Timestamp Standardization

#### Task H3.1: Update `WorkspaceEntry` Timestamps

**File**: `crates/clawft-types/src/workspace.rs`

**Before** (lines 11-26):
```rust
pub struct WorkspaceEntry {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub last_accessed: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}
```

**After**:
```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: PathBuf,

    /// When this workspace was last accessed.
    #[serde(default, deserialize_with = "crate::serde_compat::deserialize_optional_datetime")]
    pub last_accessed: Option<DateTime<Utc>>,

    /// When this workspace was first created.
    #[serde(default, deserialize_with = "crate::serde_compat::deserialize_optional_datetime")]
    pub created_at: Option<DateTime<Utc>>,
}
```

#### Task H3.2: Add Backward-Compatible Serde Deserializer

**File**: `crates/clawft-types/src/serde_compat.rs` (NEW)

Handles migration from existing JSON files that store timestamps as ISO 8601 strings, and from `i64` millisecond epoch values. Must support both old and new formats transparently.

```rust
//! Backward-compatible serde helpers for timestamp migration.

use chrono::{DateTime, Utc};
use serde::{self, Deserialize, Deserializer};

/// Deserialize an `Option<DateTime<Utc>>` from either:
/// - A JSON string (ISO 8601, e.g. "2026-01-01T00:00:00Z")
/// - A JSON number (milliseconds since epoch)
/// - A `DateTime<Utc>` (already in the correct format)
/// - null / missing (returns None)
pub fn deserialize_optional_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateTimeOrString {
        DateTime(DateTime<Utc>),
        String(String),
        MillisI64(i64),
        Null,
    }

    match Option::<DateTimeOrString>::deserialize(deserializer)? {
        Some(DateTimeOrString::DateTime(dt)) => Ok(Some(dt)),
        Some(DateTimeOrString::String(s)) => {
            if s.is_empty() {
                return Ok(None);
            }
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| Some(dt.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom)
        }
        Some(DateTimeOrString::MillisI64(ms)) => {
            DateTime::from_timestamp_millis(ms)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom(format!("invalid timestamp_millis: {ms}")))
        }
        Some(DateTimeOrString::Null) | None => Ok(None),
    }
}

/// Deserialize a `DateTime<Utc>` from either:
/// - A JSON string (ISO 8601)
/// - A JSON number (milliseconds since epoch)
/// - A `DateTime<Utc>` (already correct)
pub fn deserialize_datetime_from_millis_or_string<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateTimeOrMillis {
        DateTime(DateTime<Utc>),
        String(String),
        MillisI64(i64),
    }

    match DateTimeOrMillis::deserialize(deserializer)? {
        DateTimeOrMillis::DateTime(dt) => Ok(dt),
        DateTimeOrMillis::String(s) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom),
        DateTimeOrMillis::MillisI64(ms) => DateTime::from_timestamp_millis(ms)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid timestamp_millis: {ms}"))),
    }
}

/// Deserialize an `Option<DateTime<Utc>>` from an optional millisecond value.
pub fn deserialize_optional_datetime_from_millis<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptMillis {
        DateTime(DateTime<Utc>),
        MillisI64(i64),
        String(String),
        Null,
    }

    match Option::<OptMillis>::deserialize(deserializer)? {
        Some(OptMillis::DateTime(dt)) => Ok(Some(dt)),
        Some(OptMillis::MillisI64(ms)) => DateTime::from_timestamp_millis(ms)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid timestamp_millis: {ms}"))),
        Some(OptMillis::String(s)) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
        Some(OptMillis::Null) | None => Ok(None),
    }
}
```

**Register the module** in `crates/clawft-types/src/lib.rs`:
```rust
pub mod serde_compat;
```

#### Task H3.3: Update `CronJob` Timestamps

**File**: `crates/clawft-types/src/cron.rs`

**Before** (lines 163-169):
```rust
    /// Creation timestamp in milliseconds since epoch.
    #[serde(default)]
    pub created_at_ms: i64,

    /// Last update timestamp in milliseconds since epoch.
    #[serde(default)]
    pub updated_at_ms: i64,
```

**After**:
```rust
    /// When this job was created.
    #[serde(
        default = "Utc::now",
        alias = "created_at_ms",
        deserialize_with = "crate::serde_compat::deserialize_datetime_from_millis_or_string"
    )]
    pub created_at: DateTime<Utc>,

    /// When this job was last updated.
    #[serde(
        default = "Utc::now",
        alias = "updated_at_ms",
        deserialize_with = "crate::serde_compat::deserialize_datetime_from_millis_or_string"
    )]
    pub updated_at: DateTime<Utc>,
```

#### Task H3.4: Update `CronJobState` Timestamps

**File**: `crates/clawft-types/src/cron.rs`

**Before** (lines 121-128):
```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_run_at_ms: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at_ms: Option<i64>,
```

**After**:
```rust
    /// Next scheduled run time.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "next_run_at_ms",
        deserialize_with = "crate::serde_compat::deserialize_optional_datetime_from_millis"
    )]
    pub next_run_at: Option<DateTime<Utc>>,

    /// Last actual run time.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "last_run_at_ms",
        deserialize_with = "crate::serde_compat::deserialize_optional_datetime_from_millis"
    )]
    pub last_run_at: Option<DateTime<Utc>>,
```

#### Task H3.5: Update All Call Sites

After changing the types, all code that constructs `WorkspaceEntry`, `CronJob`, or `CronJobState` must be updated.

**WorkspaceEntry construction** in `crates/clawft-core/src/workspace.rs` (line 155-161):

**Before**:
```rust
let now = chrono::Utc::now().to_rfc3339();
let entry = WorkspaceEntry {
    name: name.into(),
    path: ws_root.clone(),
    last_accessed: Some(now.clone()),
    created_at: Some(now),
};
```

**After**:
```rust
let now = chrono::Utc::now();
let entry = WorkspaceEntry {
    name: name.into(),
    path: ws_root.clone(),
    last_accessed: Some(now),
    created_at: Some(now),
};
```

**CronJob construction** -- find all sites creating `CronJob` and change `created_at_ms: <millis>` to `created_at: <DateTime<Utc>>`, similarly for `updated_at_ms` to `updated_at`.

**CronJobState construction** -- find all sites creating `CronJobState` and change `next_run_at_ms: Some(<millis>)` to `next_run_at: Some(<DateTime<Utc>>)`, similarly for `last_run_at_ms`.

#### Task H3.6: Update Test Fixtures

All tests that construct these types with raw strings or i64 millis must be updated:

**clawft-types/src/workspace.rs tests** -- `sample_entry()`:
```rust
// Before:
fn sample_entry(name: &str) -> WorkspaceEntry {
    WorkspaceEntry {
        name: name.into(),
        path: PathBuf::from(format!("/tmp/ws-{name}")),
        last_accessed: None,
        created_at: Some("2026-01-01T00:00:00Z".into()),
    }
}

// After:
fn sample_entry(name: &str) -> WorkspaceEntry {
    WorkspaceEntry {
        name: name.into(),
        path: PathBuf::from(format!("/tmp/ws-{name}")),
        last_accessed: None,
        created_at: Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
    }
}
```

**clawft-types/src/cron.rs tests** -- `cron_job_serde_roundtrip()`, `cron_store_serde_roundtrip()`:
```rust
// Before:
created_at_ms: 1_700_000_000_000,
updated_at_ms: 1_700_000_000_000,

// After:
created_at: DateTime::from_timestamp_millis(1_700_000_000_000).unwrap(),
updated_at: DateTime::from_timestamp_millis(1_700_000_000_000).unwrap(),
```

#### Task H3.7: Add Backward-Compatibility Tests

**File**: `crates/clawft-types/src/serde_compat.rs` (tests module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn workspace_entry_from_old_string_timestamps() {
        let json = r#"{
            "name": "ws",
            "path": "/tmp/ws",
            "last_accessed": "2026-01-15T10:30:00+00:00",
            "created_at": "2026-01-01T00:00:00Z"
        }"#;
        let entry: crate::workspace::WorkspaceEntry = serde_json::from_str(json).unwrap();
        assert!(entry.last_accessed.is_some());
        assert!(entry.created_at.is_some());
    }

    #[test]
    fn workspace_entry_from_new_datetime_timestamps() {
        let entry = crate::workspace::WorkspaceEntry {
            name: "ws".into(),
            path: std::path::PathBuf::from("/tmp/ws"),
            last_accessed: Some(Utc::now()),
            created_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let restored: crate::workspace::WorkspaceEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.last_accessed, entry.last_accessed);
    }

    #[test]
    fn workspace_entry_null_timestamps() {
        let json = r#"{"name": "ws", "path": "/tmp/ws"}"#;
        let entry: crate::workspace::WorkspaceEntry = serde_json::from_str(json).unwrap();
        assert!(entry.last_accessed.is_none());
        assert!(entry.created_at.is_none());
    }

    #[test]
    fn cron_job_from_old_millis_timestamps() {
        let json = r#"{
            "id": "j1",
            "name": "test",
            "created_at_ms": 1700000000000,
            "updated_at_ms": 1700000000000
        }"#;
        let job: crate::cron::CronJob = serde_json::from_str(json).unwrap();
        assert_eq!(
            job.created_at.timestamp_millis(),
            1_700_000_000_000
        );
    }

    #[test]
    fn cron_job_state_from_old_millis() {
        let json = r#"{
            "next_run_at_ms": 1700000100000,
            "last_run_at_ms": 1700000000000,
            "last_status": "ok"
        }"#;
        let state: crate::cron::CronJobState = serde_json::from_str(json).unwrap();
        assert!(state.next_run_at.is_some());
        assert!(state.last_run_at.is_some());
    }
}
```

---

## Tests Required

### H1: Per-Agent Workspace Isolation

| # | Test Name | Scope | Description |
|---|-----------|-------|-------------|
| 1 | `ensure_agent_workspace_creates_dirs` | unit | Verify all subdirs (`sessions/`, `memory/`, `skills/`, `tool_state/`) are created |
| 2 | `ensure_agent_workspace_creates_files` | unit | Verify `SOUL.md`, `AGENTS.md`, `USER.md`, `config.toml` are created |
| 3 | `ensure_agent_workspace_idempotent` | unit | Calling twice returns same path, does not overwrite existing files |
| 4 | `create_agent_workspace_fails_if_exists` | unit | Second call with same ID returns error |
| 5 | `create_agent_workspace_with_template` | unit | Template files are copied into the new workspace |
| 6 | `delete_agent_workspace_removes_all` | unit | Directory is fully removed |
| 7 | `delete_agent_workspace_not_found` | unit | Returns error for nonexistent agent |
| 8 | `list_agent_workspaces_empty` | unit | Returns empty vec when no agents exist |
| 9 | `list_agent_workspaces_multiple` | unit | Returns sorted list of agent IDs |
| 10 | `validate_agent_id_rejects_path_traversal` | unit | `..`, `/`, `\`, empty string, null bytes all rejected |
| 11 | `validate_agent_id_accepts_valid` | unit | Alphanumeric, hyphens, underscores accepted |
| 12 | `delete_agent_workspace_rejects_path_escape` | unit | Symlink-based path escape attempt is blocked by `canonicalize()` |
| 13 | `agents_root_resolves_correctly` | unit | Returns `<registry_parent>/agents` |
| 14 | `ensure_agent_workspace_sets_permissions` | unit (unix) | Directory permissions are 0700 |

### H3: Timestamp Standardization

| # | Test Name | Scope | Description |
|---|-----------|-------|-------------|
| 15 | `workspace_entry_old_string_compat` | unit | Deserialize old `Option<String>` ISO 8601 timestamps |
| 16 | `workspace_entry_new_datetime` | unit | Roundtrip with `DateTime<Utc>` |
| 17 | `workspace_entry_null_timestamps` | unit | Missing/null fields default to `None` |
| 18 | `cron_job_old_millis_compat` | unit | Deserialize old `i64` millisecond timestamps |
| 19 | `cron_job_new_datetime` | unit | Roundtrip with `DateTime<Utc>` |
| 20 | `cron_job_state_old_millis_compat` | unit | Deserialize old `Option<i64>` millisecond timestamps |
| 21 | `cron_job_state_new_datetime` | unit | Roundtrip with `Option<DateTime<Utc>>` |
| 22 | `deserialize_optional_datetime_empty_string` | unit | Empty string deserializes to `None` |
| 23 | `deserialize_optional_datetime_invalid_string` | unit | Non-ISO-8601 string returns error |
| 24 | `workspace_create_uses_datetime` | integration | `WorkspaceManager::create()` stores `DateTime<Utc>` in registry JSON |

---

## Acceptance Criteria

### H1: Per-Agent Workspace Isolation

- [ ] `WorkspaceManager::ensure_agent_workspace("my-agent")` creates `~/.clawft/agents/my-agent/` with all subdirs and default files
- [ ] Calling `ensure_agent_workspace` twice returns the same path without overwriting existing files
- [ ] `create_agent_workspace` fails with a clear error if workspace already exists
- [ ] `create_agent_workspace` with a `template` path copies template files into the new workspace
- [ ] `delete_agent_workspace` removes the entire directory tree
- [ ] `delete_agent_workspace` rejects agent IDs that resolve outside `~/.clawft/agents/`
- [ ] `list_agent_workspaces` returns a sorted `Vec<String>` of agent IDs
- [ ] Agent ID validation rejects `..`, `/`, `\`, empty, null bytes, and control characters
- [ ] Directory permissions are `0700` on Unix
- [ ] Per-agent directory layout matches spec: `SOUL.md`, `AGENTS.md`, `USER.md`, `config.toml`, `sessions/`, `memory/`, `skills/`, `tool_state/`
- [ ] Default `config.toml` includes commented-out `shared_namespaces` and `import_namespaces` sections
- [ ] Cross-agent shared memory protocol is documented in `config.toml` comments and orchestrator doc
- [ ] All 14 H1 tests pass

### H3: Timestamp Standardization

- [ ] `WorkspaceEntry.last_accessed` and `created_at` are `Option<DateTime<Utc>>`
- [ ] `CronJob.created_at` and `updated_at` are `DateTime<Utc>` (renamed from `created_at_ms`/`updated_at_ms`)
- [ ] `CronJobState.next_run_at` and `last_run_at` are `Option<DateTime<Utc>>` (renamed from `next_run_at_ms`/`last_run_at_ms`)
- [ ] Old JSON files with `Option<String>` ISO 8601 timestamps deserialize correctly
- [ ] Old JSON files with `i64` millisecond timestamps deserialize correctly
- [ ] New JSON files serialize `DateTime<Utc>` as ISO 8601 strings (chrono default)
- [ ] `InboundMessage.timestamp` (already `DateTime<Utc>`) -- no change needed, verified
- [ ] `Session.created_at` and `updated_at` (already `DateTime<Utc>`) -- no change needed, verified
- [ ] All existing tests updated and passing
- [ ] All 10 H3 tests pass
- [ ] `cargo test -p clawft-types` passes with zero failures
- [ ] `cargo test -p clawft-core` passes with zero failures

---

## Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| H3 serde backward-compat breaks existing `workspaces.json` | Medium | High | **6** | Custom deserializer handles both `String` and `DateTime<Utc>`; integration test verifies old JSON parses |
| H3 cron backward-compat breaks existing `cron_store.json` | Medium | High | **6** | Custom deserializer handles both `i64` millis and `DateTime<Utc>`; integration test verifies old JSON parses |
| H1 agent workspace path traversal via crafted agent_id | Low | Critical | **4** | `validate_agent_id()` rejects all dangerous chars; `delete` uses `canonicalize()` to verify path is under agents root |
| H1 cross-agent shared memory race conditions | Low | Medium | **3** | Read-only by default; write access requires double opt-in + fs-level `flock` locking; eventual consistency model |
| H1 symlink target validation bypass via TOCTOU | Low | Medium | **3** | `canonicalize()` at operation time; O_NOFOLLOW where applicable; documented as defense-in-depth, not bulletproof |
| H3 field rename (`created_at_ms` -> `created_at`) breaks external consumers | Medium | Medium | **4** | `serde(alias)` preserves old field names for deserialization; serialization uses new names only -- external writers should migrate |
| H1 `tool_state/` directory semantics undefined until Contract 3.1 | Low | Low | **2** | Directory is created empty; actual usage defined by plugin KeyValueStore contract in Element 04 |

---

## Cross-Element Dependencies

| Direction | Element | Item | Contract |
|-----------|---------|------|----------|
| **This blocks** | 09 (Multi-Agent Routing) | L2 calls `ensure_agent_workspace(agent_id)` on routing dispatch | H1 must ship before L2 |
| **This blocks** | 04 (Plugin/Skill System) | Contract 3.1: `tool_state/<plugin_name>/` per-agent plugin storage | `tool_state/` dir must exist |
| **No dependency** | 08 (Memory & Workspace) | H2 (Vector Memory) | H1 and H2 are independent; H2 uses `memory/` subdir created by H1 |
| **No dependency** | 03 (Critical Fixes) | A2 (stable hash) | H3 timestamp changes do not affect hash stability |
