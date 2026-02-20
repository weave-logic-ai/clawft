# Phase F-Core: Core Dev Tools

> **Element:** 07 -- Dev Tools & Applications
> **Phase:** F-Core
> **Timeline:** Week 5-7
> **Priority:** P1 (High -- F6 is P0 critical path for E5a Google Chat)
> **Crates:** `clawft-plugin-git`, `clawft-plugin-cargo`, `clawft-plugin-oauth2`, extension to `clawft-services`
> **Dependencies IN:** C1 (plugin traits -- `Tool`, `ToolContext`, `KeyValueStore`), D9 (MCP transport concurrency for F9a), A4 (SecretRef for F6)
> **Blocks:** E5a (Google Chat depends on F6), F9b (full MCP client depends on F9a), F5 (Calendar depends on F6)
> **Status:** Planning

---

## 1. Overview

Phase F-Core delivers four foundational dev tool capabilities:

1. **F1: Git Tool Plugin** -- Clone, commit, branch, diff, blame, log, status operations via `git2`.
2. **F2: Cargo/Build Integration** -- Build, test, clippy, check, publish via subprocess execution.
3. **F6: OAuth2 Helper** -- Authorization Code and Client Credentials flows with token persistence and automatic refresh. **Critical path** -- unblocks E5a (Google Chat) and F5 (Calendar).
4. **F9a: MCP Client MVP** -- Connect to a single external MCP server, list tools, invoke tools. Extends existing `McpSession` in `clawft-services`.

All tool plugins implement the `Tool` trait from `clawft-plugin` (C1). Each tool declares MCP-compatible JSON Schema for `input_schema`, enabling seamless exposure through the MCP server shell.

---

## 2. Crate Structure

Dev tool plugins are separate workspace crates, one per tool, behind CLI feature flags:

| Crate | Tool | Feature Flag | Key Dependency |
|-------|------|-------------|----------------|
| `crates/clawft-plugin-git` | F1: Git operations | `plugin-git` | `git2` |
| `crates/clawft-plugin-cargo` | F2: Cargo/build | `plugin-cargo` | `tokio::process` |
| `crates/clawft-plugin-oauth2` | F6: OAuth2 helper | `plugin-oauth2` | `oauth2`, `reqwest` |

F9a is **NOT** a separate crate. It extends the existing MCP infrastructure in `crates/clawft-services/src/mcp/`. The existing `McpSession` (mod.rs:182-258) already implements the initialize handshake, `list_tools`, and `call_tool`. F9a adds a `ToolProvider` adapter that bridges `McpSession` into the composite tool system.

### Crate Layout: clawft-plugin-git

```
crates/clawft-plugin-git/
  Cargo.toml
  src/
    lib.rs          -- Tool trait implementation, tool registration fn
    operations.rs   -- clone, commit, branch, diff, blame, log, status
    types.rs        -- GitConfig, GitResult, operation-specific types
```

### Crate Layout: clawft-plugin-cargo

```
crates/clawft-plugin-cargo/
  Cargo.toml
  src/
    lib.rs          -- Tool trait implementation, tool registration fn
    operations.rs   -- build, test, clippy, check, publish
    types.rs        -- CargoConfig, BuildResult, TestResult types
    parser.rs       -- JSON message-format output parser
```

### Crate Layout: clawft-plugin-oauth2

```
crates/clawft-plugin-oauth2/
  Cargo.toml
  src/
    lib.rs          -- Tool trait implementations, tool registration fn
    flows.rs        -- AuthorizationCode, ClientCredentials flow logic
    storage.rs      -- Token persistence to ~/.clawft/tokens/
    providers.rs    -- Google, Microsoft, Generic preset configs
    types.rs        -- OAuth2Config, TokenSet, ProviderPreset types
    rest_client.rs  -- Authenticated HTTP client using stored tokens
```

---

## 3. Tool Permission Model

Each plugin declares permissions in its manifest. The K3 sandbox enforces at runtime.

### F1: Git Plugin Permissions

```json
{
  "id": "com.clawft.plugin-git",
  "name": "Git Operations",
  "version": "0.1.0",
  "capabilities": ["tool"],
  "permissions": {
    "filesystem": ["$WORKSPACE"],
    "network": ["github.com", "gitlab.com", "bitbucket.org"],
    "env_vars": ["GIT_AUTHOR_NAME", "GIT_AUTHOR_EMAIL", "GIT_COMMITTER_NAME", "GIT_COMMITTER_EMAIL"],
    "shell": false
  }
}
```

### F2: Cargo Plugin Permissions

```json
{
  "id": "com.clawft.plugin-cargo",
  "name": "Cargo Build Integration",
  "version": "0.1.0",
  "capabilities": ["tool"],
  "permissions": {
    "filesystem": ["$WORKSPACE", "$CARGO_HOME"],
    "network": ["crates.io", "index.crates.io"],
    "env_vars": ["CARGO_HOME", "RUSTUP_HOME", "RUSTFLAGS"],
    "shell": true
  }
}
```

### F6: OAuth2 Plugin Permissions

```json
{
  "id": "com.clawft.plugin-oauth2",
  "name": "OAuth2 Helper",
  "version": "0.1.0",
  "capabilities": ["tool"],
  "permissions": {
    "filesystem": ["~/.clawft/tokens/"],
    "network": ["*"],
    "env_vars": [],
    "shell": false
  }
}
```

---

## 4. Implementation Tasks

### 4.1 F1: Git Tool Plugin

#### Task F1.1: Create crate scaffold

**File:** `crates/clawft-plugin-git/Cargo.toml`

```toml
[package]
name = "clawft-plugin-git"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Git operations tool plugin for clawft"

[dependencies]
clawft-plugin = { path = "../clawft-plugin" }
async-trait = { workspace = true }
git2 = "0.19"
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = "3"
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

Add to workspace `Cargo.toml` members list and define the `plugin-git` feature flag in the CLI crate.

#### Task F1.2: Define types

**File:** `crates/clawft-plugin-git/src/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// Configuration for git operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Path to the repository root.
    pub repo_path: String,
}

/// Result of a git clone operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneResult {
    pub path: String,
    pub head_commit: Option<String>,
}

/// Result of a git commit operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub oid: String,
    pub message: String,
}

/// A single diff entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub old_file: Option<String>,
    pub new_file: Option<String>,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
}

/// A single blame line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line_number: u32,
    pub commit_id: String,
    pub author: String,
    pub content: String,
}

/// A single log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub oid: String,
    pub author: String,
    pub message: String,
    pub timestamp: i64,
}

/// File status in the working directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEntry {
    pub path: String,
    pub status: String,
}
```

#### Task F1.3: Implement operations

**File:** `crates/clawft-plugin-git/src/operations.rs`

Each operation opens a `git2::Repository` and performs the requested action. All operations are synchronous (git2 is not async) and should be called via `tokio::task::spawn_blocking` in the tool's `execute()` method.

```rust
use git2::{Repository, Signature, StatusOptions};
use std::path::Path;

use crate::types::*;

/// Clone a repository from a URL to a local path.
pub fn git_clone(url: &str, target_path: &Path) -> Result<CloneResult, git2::Error> {
    let repo = Repository::clone(url, target_path)?;
    let head_commit = repo
        .head()
        .ok()
        .and_then(|r| r.peel_to_commit().ok())
        .map(|c| c.id().to_string());
    Ok(CloneResult {
        path: target_path.display().to_string(),
        head_commit,
    })
}

/// Create a commit with the given message on the current branch.
/// Stages all modified/new files first (equivalent to `git add -A`).
pub fn git_commit(
    repo_path: &Path,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<CommitResult, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let sig = Signature::now(author_name, author_email)?;
    let parent = repo.head().ok().and_then(|r| r.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
    Ok(CommitResult {
        oid: oid.to_string(),
        message: message.to_string(),
    })
}

/// List branches in the repository.
pub fn git_branch(repo_path: &Path, pattern: Option<&str>) -> Result<Vec<String>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let branches = repo.branches(None)?;
    let mut names = Vec::new();
    for branch in branches {
        let (branch, _) = branch?;
        if let Some(name) = branch.name()? {
            if let Some(pat) = pattern {
                if name.contains(pat) {
                    names.push(name.to_string());
                }
            } else {
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
}

/// Compute diff between HEAD and working directory.
pub fn git_diff(repo_path: &Path) -> Result<Vec<DiffEntry>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let head_tree = repo
        .head()
        .ok()
        .and_then(|r| r.peel_to_tree().ok());
    let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), None)?;
    let mut entries = Vec::new();
    diff.foreach(
        &mut |delta, _progress| {
            let entry = DiffEntry {
                old_file: delta.old_file().path().map(|p| p.display().to_string()),
                new_file: delta.new_file().path().map(|p| p.display().to_string()),
                status: format!("{:?}", delta.status()),
                additions: 0,
                deletions: 0,
            };
            entries.push(entry);
            true
        },
        None,
        None,
        None,
    )?;
    Ok(entries)
}

/// Blame a single file.
pub fn git_blame(repo_path: &Path, file_path: &str) -> Result<Vec<BlameLine>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let blame = repo.blame_file(Path::new(file_path), None)?;
    let mut lines = Vec::new();
    for (i, hunk) in blame.iter().enumerate() {
        let sig = hunk.final_signature();
        let author = sig.name().unwrap_or("unknown").to_string();
        lines.push(BlameLine {
            line_number: (i + 1) as u32,
            commit_id: hunk.final_commit_id().to_string(),
            author,
            content: String::new(), // Content filled from file read
        });
    }
    Ok(lines)
}

/// Get commit log for the current branch.
pub fn git_log(repo_path: &Path, max_count: usize) -> Result<Vec<LogEntry>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    let mut entries = Vec::new();
    for (i, oid) in revwalk.enumerate() {
        if i >= max_count {
            break;
        }
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();
        entries.push(LogEntry {
            oid: oid.to_string(),
            author: author.name().unwrap_or("unknown").to_string(),
            message: commit.message().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
        });
    }
    Ok(entries)
}

/// Get working directory status.
pub fn git_status(repo_path: &Path) -> Result<Vec<StatusEntry>, git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut entries = Vec::new();
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            entries.push(StatusEntry {
                path: path.to_string(),
                status: format!("{:?}", entry.status()),
            });
        }
    }
    Ok(entries)
}
```

#### Task F1.4: Implement Tool trait

**File:** `crates/clawft-plugin-git/src/lib.rs`

Each git operation is exposed as a separate `Tool` implementation. Tools use `tokio::task::spawn_blocking` to run synchronous `git2` calls without blocking the async runtime.

```rust
pub mod operations;
pub mod types;

use std::path::PathBuf;
use async_trait::async_trait;
use clawft_plugin::{PluginError, Tool, ToolContext};

/// Git status tool -- reports working directory status.
pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str { "git_status" }

    fn description(&self) -> &str {
        "Show the working tree status of a git repository"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "repo_path": {
                    "type": "string",
                    "description": "Path to the git repository"
                }
            },
            "required": ["repo_path"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &dyn ToolContext,
    ) -> Result<serde_json::Value, PluginError> {
        let repo_path: PathBuf = params
            .get("repo_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PluginError::ExecutionFailed("missing 'repo_path'".into()))?
            .into();

        let result = tokio::task::spawn_blocking(move || {
            operations::git_status(&repo_path)
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("join error: {e}")))?
        .map_err(|e| PluginError::ExecutionFailed(format!("git error: {e}")))?;

        serde_json::to_value(&result).map_err(PluginError::from)
    }
}

// GitCloneTool, GitCommitTool, GitBranchTool, GitDiffTool, GitBlameTool, GitLogTool
// follow the same pattern. Each tool:
//   1. Extracts parameters from the JSON input
//   2. Validates required fields
//   3. Calls the corresponding operations::git_* function via spawn_blocking
//   4. Serializes the result to JSON

/// Register all git tools. Returns a Vec of boxed Tool trait objects.
pub fn register_git_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(GitStatusTool),
        // Box::new(GitCloneTool),
        // Box::new(GitCommitTool),
        // Box::new(GitBranchTool),
        // Box::new(GitDiffTool),
        // Box::new(GitBlameTool),
        // Box::new(GitLogTool),
    ]
}
```

**Pattern:** Each git operation (clone, commit, branch, diff, blame, log, status) follows the same structure -- extract params, `spawn_blocking`, call `operations::git_*`, serialize result. The 7 tool structs are identical in shape, differing only in parameter schemas and the operation function called.

---

### 4.2 F2: Cargo/Build Integration

#### Task F2.1: Create crate scaffold

**File:** `crates/clawft-plugin-cargo/Cargo.toml`

```toml
[package]
name = "clawft-plugin-cargo"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Cargo/build tool plugin for clawft"

[dependencies]
clawft-plugin = { path = "../clawft-plugin" }
async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["process"] }
tracing = { workspace = true }

[dev-dependencies]
tempfile = "3"
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

#### Task F2.2: Define types

**File:** `crates/clawft-plugin-cargo/src/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// Configuration for a cargo operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoConfig {
    /// Working directory (project root).
    pub working_dir: String,
    /// Build profile: "debug" or "release".
    #[serde(default = "default_profile")]
    pub profile: String,
    /// Target specific package (-p flag).
    pub package: Option<String>,
    /// Use --workspace flag.
    #[serde(default)]
    pub workspace: bool,
    /// Additional flags passed to cargo.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_profile() -> String { "debug".to_string() }

/// Result of a cargo subprocess execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// Parsed JSON messages from --message-format=json (if applicable).
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,
}
```

#### Task F2.3: Implement operations

**File:** `crates/clawft-plugin-cargo/src/operations.rs`

**Security:** Command arguments are built programmatically -- no string interpolation from user input. Package names and flags are validated against an allowlist of safe characters before being passed to `Command`.

```rust
use std::path::Path;
use tokio::process::Command;

use crate::types::{CargoConfig, CargoOutput};

/// Validate that a string is safe to use as a cargo argument.
/// Rejects shell metacharacters to prevent injection.
fn validate_arg(arg: &str) -> Result<(), String> {
    if arg.contains(|c: char| {
        matches!(c, ';' | '&' | '|' | '$' | '`' | '(' | ')' | '{' | '}' | '<' | '>' | '!' | '\n' | '\r')
    }) {
        return Err(format!("unsafe characters in argument: {arg}"));
    }
    Ok(())
}

/// Build a cargo Command with common flags.
fn build_command(subcommand: &str, config: &CargoConfig) -> Result<Command, String> {
    validate_arg(subcommand)?;

    let mut cmd = Command::new("cargo");
    cmd.arg(subcommand);
    cmd.current_dir(&config.working_dir);

    if config.profile == "release" {
        cmd.arg("--release");
    }

    if config.workspace {
        cmd.arg("--workspace");
    }

    if let Some(ref pkg) = config.package {
        validate_arg(pkg)?;
        cmd.args(["-p", pkg]);
    }

    for arg in &config.extra_args {
        validate_arg(arg)?;
        cmd.arg(arg);
    }

    Ok(cmd)
}

/// Run a cargo command and capture output.
async fn run_cargo(subcommand: &str, config: &CargoConfig, json_output: bool) -> Result<CargoOutput, String> {
    let mut cmd = build_command(subcommand, config)?;

    if json_output {
        cmd.arg("--message-format=json");
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("failed to execute cargo {subcommand}: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let messages = if json_output {
        stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    } else {
        Vec::new()
    };

    Ok(CargoOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout,
        stderr,
        messages,
    })
}

/// cargo build
pub async fn cargo_build(config: &CargoConfig) -> Result<CargoOutput, String> {
    run_cargo("build", config, true).await
}

/// cargo test
pub async fn cargo_test(config: &CargoConfig) -> Result<CargoOutput, String> {
    run_cargo("test", config, false).await
}

/// cargo clippy
pub async fn cargo_clippy(config: &CargoConfig) -> Result<CargoOutput, String> {
    run_cargo("clippy", config, true).await
}

/// cargo check
pub async fn cargo_check(config: &CargoConfig) -> Result<CargoOutput, String> {
    run_cargo("check", config, true).await
}

/// cargo publish (requires network)
pub async fn cargo_publish(config: &CargoConfig) -> Result<CargoOutput, String> {
    run_cargo("publish", config, false).await
}
```

#### Task F2.4: Implement Tool trait

**File:** `crates/clawft-plugin-cargo/src/lib.rs`

```rust
pub mod operations;
pub mod types;

use async_trait::async_trait;
use clawft_plugin::{PluginError, Tool, ToolContext};

use crate::types::CargoConfig;

/// Cargo build tool.
pub struct CargoBuildTool;

#[async_trait]
impl Tool for CargoBuildTool {
    fn name(&self) -> &str { "cargo_build" }

    fn description(&self) -> &str {
        "Build a Rust project using cargo"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "working_dir": {
                    "type": "string",
                    "description": "Project root directory"
                },
                "profile": {
                    "type": "string",
                    "enum": ["debug", "release"],
                    "default": "debug"
                },
                "package": {
                    "type": "string",
                    "description": "Specific package to build (-p flag)"
                },
                "workspace": {
                    "type": "boolean",
                    "description": "Build all workspace members",
                    "default": false
                }
            },
            "required": ["working_dir"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &dyn ToolContext,
    ) -> Result<serde_json::Value, PluginError> {
        let config: CargoConfig = serde_json::from_value(params)
            .map_err(|e| PluginError::ExecutionFailed(format!("invalid params: {e}")))?;

        let result = operations::cargo_build(&config)
            .await
            .map_err(|e| PluginError::ExecutionFailed(e))?;

        serde_json::to_value(&result).map_err(PluginError::from)
    }
}

// CargoTestTool, CargoClippyTool, CargoCheckTool, CargoPublishTool
// follow the same pattern.

/// Register all cargo tools.
pub fn register_cargo_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(CargoBuildTool),
        // Box::new(CargoTestTool),
        // Box::new(CargoClippyTool),
        // Box::new(CargoCheckTool),
        // Box::new(CargoPublishTool),
    ]
}
```

---

### 4.3 F6: Generic REST + OAuth2 Helper

**CRITICAL PRIORITY** -- F6 unblocks E5a (Google Chat) and F5 (Calendar).

#### Task F6.1: Create crate scaffold

**File:** `crates/clawft-plugin-oauth2/Cargo.toml`

```toml
[package]
name = "clawft-plugin-oauth2"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "OAuth2 and authenticated REST client plugin for clawft"

[dependencies]
clawft-plugin = { path = "../clawft-plugin" }
async-trait = { workspace = true }
oauth2 = "5"
reqwest = { workspace = true, features = ["json"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["fs"] }
tracing = { workspace = true }
url = "2"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3"
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
wiremock = "0.6"
```

#### Task F6.2: Define types

**File:** `crates/clawft-plugin-oauth2/src/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// OAuth2 provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// Provider name (used as key for token storage).
    pub provider_name: String,
    /// Client ID.
    pub client_id: String,
    /// Client secret -- stored via SecretRef, never logged.
    #[serde(skip_serializing)]
    pub client_secret: String,
    /// Authorization endpoint URL.
    pub auth_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// Redirect URI for authorization code flow.
    pub redirect_uri: Option<String>,
    /// Scopes to request.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Pre-configured provider presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderPreset {
    Google,
    Microsoft,
    Generic,
}

/// A stored token set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// Access token.
    pub access_token: String,
    /// Token type (usually "Bearer").
    pub token_type: String,
    /// Refresh token (if granted).
    pub refresh_token: Option<String>,
    /// Expiration timestamp (Unix seconds).
    pub expires_at: Option<i64>,
    /// Scopes the token was granted for.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Provider name this token belongs to.
    pub provider_name: String,
}

impl TokenSet {
    /// Check if the access token has expired (with 60-second buffer).
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(exp) => chrono::Utc::now().timestamp() >= (exp - 60),
            None => false,
        }
    }
}

/// Result of starting an authorization flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeResult {
    /// URL the user should visit to authorize.
    pub authorize_url: String,
    /// CSRF state parameter (must be validated on callback).
    pub state: String,
    /// PKCE code verifier (stored for callback exchange).
    pub pkce_verifier: Option<String>,
}
```

#### Task F6.3: Implement token storage

**File:** `crates/clawft-plugin-oauth2/src/storage.rs`

Tokens are stored as JSON files in `~/.clawft/tokens/<provider_name>.json` with `0600` permissions. **Security:** File permissions are set immediately after creation. Tokens are never written to world-readable locations.

```rust
use std::path::PathBuf;
use tokio::fs;
use tracing::debug;

use crate::types::TokenSet;

/// Get the token storage directory, creating it if needed.
pub async fn token_dir() -> Result<PathBuf, std::io::Error> {
    let dir = dirs::home_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no home directory"))?
        .join(".clawft")
        .join("tokens");
    fs::create_dir_all(&dir).await?;

    // Set directory permissions to 0700 on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).await?;
    }

    Ok(dir)
}

/// Persist a token set to disk.
pub async fn store_token(token: &TokenSet) -> Result<(), std::io::Error> {
    let dir = token_dir().await?;
    let path = dir.join(format!("{}.json", token.provider_name));

    let json = serde_json::to_string_pretty(token)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    fs::write(&path, json.as_bytes()).await?;

    // Set file permissions to 0600 on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).await?;
    }

    debug!(provider = %token.provider_name, path = %path.display(), "stored token");
    Ok(())
}

/// Load a token set from disk.
pub async fn load_token(provider_name: &str) -> Result<Option<TokenSet>, std::io::Error> {
    let dir = token_dir().await?;
    let path = dir.join(format!("{provider_name}.json"));

    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path).await?;
    let token: TokenSet = serde_json::from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(Some(token))
}

/// Delete a stored token.
pub async fn delete_token(provider_name: &str) -> Result<(), std::io::Error> {
    let dir = token_dir().await?;
    let path = dir.join(format!("{provider_name}.json"));

    if path.exists() {
        fs::remove_file(&path).await?;
        debug!(provider = %provider_name, "deleted token");
    }

    Ok(())
}
```

#### Task F6.4: Implement OAuth2 flows

**File:** `crates/clawft-plugin-oauth2/src/flows.rs`

```rust
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use oauth2::reqwest::async_http_client;

use crate::types::{AuthorizeResult, OAuth2Config, TokenSet};

/// Build an OAuth2 client from config.
fn build_client(config: &OAuth2Config) -> Result<BasicClient, String> {
    let client = BasicClient::new(
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
        AuthUrl::new(config.auth_url.clone()).map_err(|e| format!("bad auth URL: {e}"))?,
        Some(TokenUrl::new(config.token_url.clone()).map_err(|e| format!("bad token URL: {e}"))?),
    );

    let client = if let Some(ref redirect) = config.redirect_uri {
        client.set_redirect_uri(
            RedirectUrl::new(redirect.clone()).map_err(|e| format!("bad redirect URI: {e}"))?,
        )
    } else {
        client
    };

    Ok(client)
}

/// Start an Authorization Code + PKCE flow.
/// Returns the authorization URL the user should visit.
pub fn start_auth_code_flow(config: &OAuth2Config) -> Result<AuthorizeResult, String> {
    let client = build_client(config)?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut request = client.authorize_url(CsrfToken::new_random);

    for scope in &config.scopes {
        request = request.add_scope(Scope::new(scope.clone()));
    }

    let (authorize_url, csrf_state) = request.set_pkce_challenge(pkce_challenge).url();

    Ok(AuthorizeResult {
        authorize_url: authorize_url.to_string(),
        state: csrf_state.secret().clone(),
        pkce_verifier: Some(pkce_verifier.secret().clone()),
    })
}

/// Exchange an authorization code for tokens.
pub async fn exchange_auth_code(
    config: &OAuth2Config,
    code: &str,
    pkce_verifier: Option<&str>,
) -> Result<TokenSet, String> {
    let client = build_client(config)?;

    let mut request = client
        .exchange_code(AuthorizationCode::new(code.to_string()));

    if let Some(verifier) = pkce_verifier {
        request = request.set_pkce_verifier(PkceCodeVerifier::new(verifier.to_string()));
    }

    let token_result = request
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("token exchange failed: {e}"))?;

    let expires_at = token_result.expires_in().map(|d| {
        chrono::Utc::now().timestamp() + d.as_secs() as i64
    });

    Ok(TokenSet {
        access_token: token_result.access_token().secret().clone(),
        token_type: "Bearer".to_string(),
        refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
        expires_at,
        scopes: config.scopes.clone(),
        provider_name: config.provider_name.clone(),
    })
}

/// Refresh an expired access token using a refresh token.
pub async fn refresh_token(
    config: &OAuth2Config,
    refresh_token_value: &str,
) -> Result<TokenSet, String> {
    let client = build_client(config)?;

    let token_result = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token_value.to_string()))
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("token refresh failed: {e}"))?;

    let expires_at = token_result.expires_in().map(|d| {
        chrono::Utc::now().timestamp() + d.as_secs() as i64
    });

    // If the server rotated the refresh token, use the new one.
    let new_refresh = token_result
        .refresh_token()
        .map(|t| t.secret().clone())
        .or_else(|| Some(refresh_token_value.to_string()));

    Ok(TokenSet {
        access_token: token_result.access_token().secret().clone(),
        token_type: "Bearer".to_string(),
        refresh_token: new_refresh,
        expires_at,
        scopes: config.scopes.clone(),
        provider_name: config.provider_name.clone(),
    })
}

/// Client Credentials flow (for service accounts).
pub async fn client_credentials_flow(config: &OAuth2Config) -> Result<TokenSet, String> {
    let client = build_client(config)?;

    let mut request = client.exchange_client_credentials();

    for scope in &config.scopes {
        request = request.add_scope(Scope::new(scope.clone()));
    }

    let token_result = request
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("client credentials flow failed: {e}"))?;

    let expires_at = token_result.expires_in().map(|d| {
        chrono::Utc::now().timestamp() + d.as_secs() as i64
    });

    Ok(TokenSet {
        access_token: token_result.access_token().secret().clone(),
        token_type: "Bearer".to_string(),
        refresh_token: None,
        expires_at,
        scopes: config.scopes.clone(),
        provider_name: config.provider_name.clone(),
    })
}
```

#### Task F6.5: Implement provider presets

**File:** `crates/clawft-plugin-oauth2/src/providers.rs`

```rust
use crate::types::{OAuth2Config, ProviderPreset};

/// Return the auth and token URLs for a known provider.
pub fn preset_urls(preset: ProviderPreset) -> (&'static str, &'static str) {
    match preset {
        ProviderPreset::Google => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
        ),
        ProviderPreset::Microsoft => (
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        ),
        ProviderPreset::Generic => ("", ""),
    }
}

/// Build an OAuth2Config from a preset.
pub fn from_preset(
    preset: ProviderPreset,
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    redirect_uri: Option<String>,
) -> OAuth2Config {
    let (auth_url, token_url) = preset_urls(preset);
    OAuth2Config {
        provider_name: format!("{:?}", preset).to_lowercase(),
        client_id,
        client_secret,
        auth_url: auth_url.to_string(),
        token_url: token_url.to_string(),
        redirect_uri,
        scopes,
    }
}
```

#### Task F6.6: Implement REST client

**File:** `crates/clawft-plugin-oauth2/src/rest_client.rs`

```rust
use reqwest::{Client, Method, Response};
use serde_json::Value;
use tracing::debug;

use crate::storage;
use crate::types::{OAuth2Config, TokenSet};
use crate::flows;

/// An authenticated HTTP client that uses stored OAuth2 tokens.
pub struct AuthenticatedClient {
    http: Client,
    config: OAuth2Config,
}

impl AuthenticatedClient {
    pub fn new(config: OAuth2Config) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    /// Get a valid access token, refreshing if expired.
    async fn get_token(&self) -> Result<String, String> {
        let token = storage::load_token(&self.config.provider_name)
            .await
            .map_err(|e| format!("failed to load token: {e}"))?
            .ok_or_else(|| format!("no stored token for provider '{}'", self.config.provider_name))?;

        if !token.is_expired() {
            return Ok(token.access_token);
        }

        // Token expired -- attempt refresh.
        let refresh = token.refresh_token
            .ok_or("token expired and no refresh token available")?;

        debug!(provider = %self.config.provider_name, "refreshing expired token");
        let new_token = flows::refresh_token(&self.config, &refresh).await?;

        // Persist the refreshed token immediately (rotated refresh tokens).
        storage::store_token(&new_token)
            .await
            .map_err(|e| format!("failed to persist refreshed token: {e}"))?;

        Ok(new_token.access_token)
    }

    /// Make an authenticated HTTP request.
    pub async fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<Value>,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<Value, String> {
        let token = self.get_token().await?;

        let method = method.to_uppercase().parse::<Method>()
            .map_err(|e| format!("invalid HTTP method: {e}"))?;

        let mut req = self.http
            .request(method, url)
            .bearer_auth(&token);

        if let Some(hdrs) = headers {
            for (k, v) in hdrs {
                req = req.header(&k, &v);
            }
        }

        if let Some(body) = body {
            req = req.json(&body);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .unwrap_or_else(|_| Value::Null);

        if !status.is_success() {
            return Err(format!("HTTP {status}: {body}"));
        }

        Ok(body)
    }
}
```

#### Task F6.7: Implement Tool trait for OAuth2

**File:** `crates/clawft-plugin-oauth2/src/lib.rs`

Four tools exposed:

| Tool Name | Description |
|-----------|-------------|
| `oauth2_authorize` | Start auth flow, return redirect URL |
| `oauth2_callback` | Handle callback with auth code, exchange for tokens |
| `oauth2_refresh` | Manually refresh tokens |
| `rest_request` | Make authenticated REST request using stored tokens |

```rust
pub mod flows;
pub mod providers;
pub mod rest_client;
pub mod storage;
pub mod types;

use async_trait::async_trait;
use clawft_plugin::{PluginError, Tool, ToolContext};

/// Start an OAuth2 authorization flow.
pub struct OAuth2AuthorizeTool;

#[async_trait]
impl Tool for OAuth2AuthorizeTool {
    fn name(&self) -> &str { "oauth2_authorize" }

    fn description(&self) -> &str {
        "Start an OAuth2 authorization code flow. Returns a URL for the user to visit."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "provider": {
                    "type": "string",
                    "enum": ["google", "microsoft", "generic"],
                    "description": "OAuth2 provider preset"
                },
                "client_id": { "type": "string" },
                "client_secret": { "type": "string" },
                "scopes": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "redirect_uri": { "type": "string" },
                "auth_url": {
                    "type": "string",
                    "description": "Required if provider is 'generic'"
                },
                "token_url": {
                    "type": "string",
                    "description": "Required if provider is 'generic'"
                }
            },
            "required": ["client_id", "client_secret", "scopes"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &dyn ToolContext,
    ) -> Result<serde_json::Value, PluginError> {
        // Build config from preset or generic URLs
        let config = build_config_from_params(&params)?;

        let result = flows::start_auth_code_flow(&config)
            .map_err(|e| PluginError::ExecutionFailed(e))?;

        // Store PKCE verifier in tool state for later callback
        if let Some(ref verifier) = result.pkce_verifier {
            let key = format!("oauth2_pkce_{}", result.state);
            ctx.key_value_store()
                .set(&key, verifier)
                .await
                .map_err(|e| PluginError::ExecutionFailed(format!("store pkce: {e}")))?;
        }

        serde_json::to_value(&result).map_err(PluginError::from)
    }
}

/// Helper to build OAuth2Config from tool params.
fn build_config_from_params(params: &serde_json::Value) -> Result<types::OAuth2Config, PluginError> {
    let client_id = params.get("client_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PluginError::ExecutionFailed("missing client_id".into()))?
        .to_string();

    let client_secret = params.get("client_secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PluginError::ExecutionFailed("missing client_secret".into()))?
        .to_string();

    let scopes: Vec<String> = params.get("scopes")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let redirect_uri = params.get("redirect_uri")
        .and_then(|v| v.as_str())
        .map(String::from);

    let provider = params.get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("generic");

    let preset = match provider {
        "google" => types::ProviderPreset::Google,
        "microsoft" => types::ProviderPreset::Microsoft,
        _ => types::ProviderPreset::Generic,
    };

    if preset == types::ProviderPreset::Generic {
        let auth_url = params.get("auth_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PluginError::ExecutionFailed("generic provider requires auth_url".into()))?
            .to_string();
        let token_url = params.get("token_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PluginError::ExecutionFailed("generic provider requires token_url".into()))?
            .to_string();

        Ok(types::OAuth2Config {
            provider_name: "generic".to_string(),
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_uri,
            scopes,
        })
    } else {
        Ok(providers::from_preset(preset, client_id, client_secret, scopes, redirect_uri))
    }
}

// OAuth2CallbackTool, OAuth2RefreshTool, RestRequestTool
// follow similar patterns.

/// Register all OAuth2 tools.
pub fn register_oauth2_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(OAuth2AuthorizeTool),
        // Box::new(OAuth2CallbackTool),
        // Box::new(OAuth2RefreshTool),
        // Box::new(RestRequestTool),
    ]
}
```

**Security requirements for F6:**

1. OAuth2 `state` parameter MUST be validated on callback (CSRF protection)
2. PKCE verifier stored in `ToolContext::key_value_store`, keyed by state
3. Tokens stored with `0600` file permissions
4. Rotated refresh tokens persisted immediately after exchange
5. `client_secret` passed via `SecretRef` (A4) at runtime, never logged
6. Token file path sanitized -- provider_name validated to prevent directory traversal

---

### 4.4 F9a: Core MCP Client Library (MVP)

#### Task F9a.1: Assessment of existing code

The existing codebase already has a functional MCP client:

- **`McpClient`** (`crates/clawft-services/src/mcp/mod.rs:56-171`): Low-level JSON-RPC client with `list_tools()`, `call_tool()`, `send_raw()`.
- **`McpSession`** (`crates/clawft-services/src/mcp/mod.rs:182-258`): High-level session with `connect()` handshake (initialize + notifications/initialized), delegates to `McpClient`.
- **`McpTransport` trait** (`transport.rs:20-27`): Pluggable transport with `StdioTransport` and `HttpTransport` implementations.
- **`ToolProvider` trait** (`provider.rs:91-101`): Abstraction for tool sources with `namespace()`, `list_tools()`, `call_tool()`.
- **`ToolDefinition`** (`mod.rs:44-53`): Tool metadata with `name`, `description`, `input_schema`.
- **`CompositeToolProvider`** (`composite.rs`): Aggregates multiple `ToolProvider`s.

**F9a scope is therefore narrow:** Create an `McpToolProvider` that wraps `McpSession` and implements `ToolProvider`, bridging external MCP servers into the composite tool system.

#### Task F9a.2: Create McpToolProvider

**File:** `crates/clawft-services/src/mcp/mcp_tool_provider.rs` (new file)

```rust
//! ToolProvider implementation backed by a remote MCP server.
//!
//! Bridges an [`McpSession`] into the [`ToolProvider`] abstraction so
//! external MCP server tools appear alongside built-in tools.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::provider::{CallToolResult, ContentBlock, ToolError, ToolProvider};
use super::{McpSession, ToolDefinition};

/// A [`ToolProvider`] backed by a connected MCP server session.
///
/// External tools are namespaced as `mcp:<server-name>:<tool-name>`.
/// All tools from this provider are tagged as "untrusted" in metadata.
pub struct McpToolProvider {
    /// Server name used for namespacing.
    server_name: String,
    /// The active MCP session.
    session: Arc<McpSession>,
    /// Cached tool definitions (refreshed on demand).
    cached_tools: RwLock<Vec<ToolDefinition>>,
}

impl McpToolProvider {
    /// Create a new MCP tool provider wrapping an established session.
    pub fn new(server_name: String, session: Arc<McpSession>) -> Self {
        Self {
            server_name,
            session,
            cached_tools: RwLock::new(Vec::new()),
        }
    }

    /// Refresh the tool cache by querying the remote server.
    pub async fn refresh_tools(&self) -> Result<(), ToolError> {
        let tools = self.session.list_tools().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("failed to list tools from MCP server: {e}"))
        })?;
        debug!(
            server = %self.server_name,
            tool_count = tools.len(),
            "refreshed MCP tool cache"
        );
        *self.cached_tools.write().await = tools;
        Ok(())
    }

    /// Get the namespaced tool name.
    fn namespaced_name(&self, tool_name: &str) -> String {
        format!("mcp:{}:{}", self.server_name, tool_name)
    }

    /// Strip the namespace prefix from a tool name.
    fn strip_namespace<'a>(&self, full_name: &'a str) -> Option<&'a str> {
        let prefix = format!("mcp:{}:", self.server_name);
        full_name.strip_prefix(&prefix)
    }
}

#[async_trait]
impl ToolProvider for McpToolProvider {
    fn namespace(&self) -> &str {
        &self.server_name
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        // Return cached tools with namespaced names.
        // Caller should call refresh_tools() first if cache may be stale.
        let guard = self.cached_tools.try_read();
        match guard {
            Ok(tools) => tools
                .iter()
                .map(|t| ToolDefinition {
                    name: self.namespaced_name(&t.name),
                    description: t.description.clone(),
                    input_schema: t.input_schema.clone(),
                })
                .collect(),
            Err(_) => {
                warn!(server = %self.server_name, "tool cache locked, returning empty");
                Vec::new()
            }
        }
    }

    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<CallToolResult, ToolError> {
        let raw_name = self.strip_namespace(name).unwrap_or(name);

        debug!(
            server = %self.server_name,
            tool = %raw_name,
            "calling remote MCP tool"
        );

        let result = self
            .session
            .call_tool(raw_name, args)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("MCP call failed: {e}")))?;

        // Convert raw JSON result to CallToolResult.
        // MCP tools/call returns { content: [...], isError: bool }
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let content = result
            .get("content")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([{"type": "text", "text": result.to_string()}]));

        let content_blocks: Vec<ContentBlock> = if let Some(arr) = content.as_array() {
            arr.iter()
                .filter_map(|item| {
                    let text = item.get("text").and_then(|t| t.as_str())?;
                    Some(ContentBlock::Text(text.to_string()))
                })
                .collect()
        } else {
            vec![ContentBlock::Text(content.to_string())]
        };

        Ok(CallToolResult {
            content: content_blocks,
            is_error,
        })
    }
}
```

#### Task F9a.3: Wire into MCP module

**File:** `crates/clawft-services/src/mcp/mod.rs` -- add module declaration:

```rust
pub mod mcp_tool_provider;
```

Add re-export:

```rust
pub use mcp_tool_provider::McpToolProvider;
```

#### Task F9a.4: Connection factory for stdio/HTTP

**File:** `crates/clawft-services/src/mcp/mcp_tool_provider.rs` -- add factory functions:

```rust
use super::transport::{HttpTransport, StdioTransport};
use std::collections::HashMap;

/// Configuration for connecting to an external MCP server.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    /// Unique server name (used for tool namespacing).
    pub name: String,
    /// Transport type.
    pub transport: McpTransportConfig,
}

/// Transport configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpTransportConfig {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    Http {
        url: String,
    },
}

/// Connect to an external MCP server and return a ToolProvider.
///
/// # Security
///
/// For stdio transport: the child process is spawned with a minimal
/// environment. Secret env vars from the parent process are NOT inherited.
/// Only explicitly listed env vars in the config are passed.
pub async fn connect_mcp_server(
    config: McpServerConfig,
) -> Result<McpToolProvider, ToolError> {
    let session = match config.transport {
        McpTransportConfig::Stdio { command, args, env } => {
            // Validate command path -- must be absolute or a known binary.
            if command.contains("..") || command.contains(';') || command.contains('|') {
                return Err(ToolError::ExecutionFailed(
                    "unsafe stdio command path".into(),
                ));
            }

            let transport = StdioTransport::new(&command, &args, &env)
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("stdio transport failed: {e}"))
                })?;

            McpSession::connect(Box::new(transport))
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("MCP handshake failed: {e}"))
                })?
        }
        McpTransportConfig::Http { url } => {
            let transport = HttpTransport::new(url);
            McpSession::connect(Box::new(transport))
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("MCP handshake failed: {e}"))
                })?
        }
    };

    let provider = McpToolProvider::new(config.name, Arc::new(session));
    provider.refresh_tools().await?;
    Ok(provider)
}
```

**Security requirements for F9a:**

1. Stdio child processes MUST NOT inherit the parent's full environment. Only explicitly listed env vars are passed.
2. Command paths validated -- no shell metacharacters, no `..` traversal.
3. External tools tagged as "untrusted" via the `mcp:<server>:<tool>` namespace convention.
4. `McpSession` ownership is `Arc<McpSession>` -- safe for concurrent tool calls.

---

## 5. Tool Plugin <-> Memory Contract

All tool plugins use `ToolContext::key_value_store()` for state persistence. The contract from the cross-element integration spec:

```rust
// Defined in clawft-plugin (C1)
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, PluginError>;
    async fn set(&self, key: &str, value: &str) -> Result<(), PluginError>;
    async fn delete(&self, key: &str) -> Result<bool, PluginError>;
    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, PluginError>;
}
```

Plugin crates depend on `clawft-plugin` only, NEVER on `clawft-core`. The AgentLoop backs `KeyValueStore` with `~/.clawft/agents/<agentId>/tool_state/<plugin_name>/`.

**Usage per plugin:**

| Plugin | Keys Stored | Purpose |
|--------|-------------|---------|
| F1 Git | (none -- stateless) | -- |
| F2 Cargo | (none -- stateless) | -- |
| F6 OAuth2 | `oauth2_pkce_<state>` | PKCE verifier for pending auth flows |
| F9a MCP | (none -- state in `McpSession`) | -- |

---

## 6. Tests Required

### F1: Git Plugin Tests

| Test | Description |
|------|-------------|
| `test_git_clone_creates_repo` | Clone a local bare repo to a tempdir. Verify `.git/` exists and `CloneResult` has `head_commit`. |
| `test_git_commit_creates_oid` | Init repo in tempdir, create a file, commit. Verify `CommitResult.oid` is 40 hex chars. |
| `test_git_branch_lists_default` | Init repo with a commit. Verify `git_branch()` returns `["master"]` or `["main"]`. |
| `test_git_diff_detects_changes` | Init repo, commit a file, modify it. Verify `git_diff()` returns one `DiffEntry` with `status: "Modified"`. |
| `test_git_blame_returns_hunks` | Init repo, commit a file with 3 lines. Verify `git_blame()` returns 3+ `BlameLine` entries. |
| `test_git_log_returns_commits` | Init repo, make 3 commits. Verify `git_log(path, 10)` returns 3 `LogEntry` items in reverse chronological order. |
| `test_git_status_shows_untracked` | Init repo, create a file (don't stage). Verify `git_status()` shows the file with untracked status. |
| `test_git_status_tool_execute` | Create `GitStatusTool`, call `execute()` with valid params. Verify JSON result is an array. |
| `test_git_tool_missing_repo_path` | Call `GitStatusTool::execute()` without `repo_path`. Verify `PluginError::ExecutionFailed`. |

### F2: Cargo Plugin Tests

| Test | Description |
|------|-------------|
| `test_validate_arg_rejects_semicolons` | `validate_arg("foo;bar")` returns `Err`. |
| `test_validate_arg_rejects_pipes` | `validate_arg("foo|bar")` returns `Err`. |
| `test_validate_arg_accepts_clean` | `validate_arg("my-crate")` returns `Ok`. |
| `test_cargo_build_tool_schema` | Verify `CargoBuildTool.parameters_schema()` has `working_dir` in `required`. |
| `test_cargo_output_serde_roundtrip` | Serialize and deserialize a `CargoOutput`. Verify all fields preserved. |
| `test_cargo_build_integration` | Run `cargo_build` in a known Rust project. Verify `CargoOutput.success` is true. (integration test, requires cargo installed) |

### F6: OAuth2 Plugin Tests

| Test | Description |
|------|-------------|
| `test_token_set_not_expired` | Create `TokenSet` with `expires_at` 1 hour from now. Verify `is_expired()` is false. |
| `test_token_set_expired` | Create `TokenSet` with `expires_at` 1 minute ago. Verify `is_expired()` is true. |
| `test_token_set_no_expiry` | Create `TokenSet` with `expires_at: None`. Verify `is_expired()` is false. |
| `test_store_load_token_roundtrip` | Store a token, load it back. Verify all fields match. (tempdir for `~/.clawft/tokens/`) |
| `test_token_file_permissions_unix` | Store a token. Verify file has `0600` permissions. (Unix-only test) |
| `test_delete_token` | Store and then delete a token. Verify `load_token` returns `None`. |
| `test_preset_urls_google` | Verify `preset_urls(Google)` returns correct Google OAuth2 URLs. |
| `test_preset_urls_microsoft` | Verify `preset_urls(Microsoft)` returns correct Azure AD URLs. |
| `test_start_auth_code_flow` | Build config, call `start_auth_code_flow`. Verify result has non-empty `authorize_url` and `state`. |
| `test_csrf_state_is_random` | Call `start_auth_code_flow` twice. Verify the two `state` values differ. |
| `test_build_config_from_params_google` | Pass params with `provider: "google"`. Verify `auth_url` is Google's endpoint. |
| `test_build_config_from_params_generic_missing_url` | Pass `provider: "generic"` without `auth_url`. Verify `PluginError`. |
| `test_oauth2_authorize_tool_schema` | Verify `OAuth2AuthorizeTool.parameters_schema()` has `client_id` in `required`. |
| `test_provider_name_sanitized` | Verify that a provider_name with `../` is rejected or sanitized to prevent path traversal in token storage. |

### F9a: MCP Client Tests

| Test | Description |
|------|-------------|
| `test_mcp_tool_provider_namespace` | Create `McpToolProvider` with name "test". Verify `namespace()` returns "test". |
| `test_mcp_tool_provider_list_tools_namespaced` | Populate cache with tools "a", "b". Verify `list_tools()` returns `["mcp:test:a", "mcp:test:b"]`. |
| `test_mcp_tool_provider_call_tool_strips_namespace` | Call `call_tool("mcp:test:echo", ...)`. Verify the underlying session receives `"echo"`. |
| `test_mcp_tool_provider_call_tool_without_namespace` | Call `call_tool("echo", ...)`. Verify it still works (falls back to raw name). |
| `test_mcp_tool_provider_refresh_tools` | Create provider with mock session. Call `refresh_tools()`. Verify cache is populated. |
| `test_mcp_server_config_serde_stdio` | Serialize/deserialize an `McpServerConfig` with stdio transport. |
| `test_mcp_server_config_serde_http` | Serialize/deserialize an `McpServerConfig` with HTTP transport. |
| `test_connect_mcp_server_rejects_unsafe_command` | Pass command `"../evil;rm"`. Verify `ToolError::ExecutionFailed` with "unsafe". |
| `test_mcp_tool_provider_empty_cache` | Create provider, don't refresh. Verify `list_tools()` returns empty vec. |
| `test_mcp_tool_provider_is_error_handling` | Mock session returns `{ isError: true, content: [...] }`. Verify `CallToolResult.is_error` is true. |

---

## 7. Acceptance Criteria

### F1: Git Tool Plugin

- [ ] `cargo build -p clawft-plugin-git` compiles cleanly
- [ ] All 7 git operations implemented: clone, commit, branch, diff, blame, log, status
- [ ] Each operation exposed as a separate `Tool` trait implementation
- [ ] All tools return MCP-compatible JSON Schema from `parameters_schema()`
- [ ] `git2` calls wrapped in `tokio::task::spawn_blocking`
- [ ] Plugin manifest declares filesystem and network permissions
- [ ] `cargo test -p clawft-plugin-git` -- all tests pass
- [ ] `cargo clippy -p clawft-plugin-git -- -D warnings` is clean

### F2: Cargo/Build Integration

- [ ] `cargo build -p clawft-plugin-cargo` compiles cleanly
- [ ] All 5 operations implemented: build, test, clippy, check, publish
- [ ] Command arguments validated against injection (no shell metacharacters)
- [ ] JSON message-format output parsed for structured results
- [ ] `--release`, `--workspace`, `-p <crate>` flags supported
- [ ] `cargo test -p clawft-plugin-cargo` -- all tests pass
- [ ] `cargo clippy -p clawft-plugin-cargo -- -D warnings` is clean

### F6: OAuth2 Helper

- [ ] `cargo build -p clawft-plugin-oauth2` compiles cleanly
- [ ] Authorization Code + PKCE flow works end-to-end
- [ ] Client Credentials flow works end-to-end
- [ ] Token persistence in `~/.clawft/tokens/` with `0600` permissions
- [ ] Automatic token refresh before expiration
- [ ] Google and Microsoft presets return correct URLs
- [ ] OAuth2 `state` parameter validated on callback (CSRF protection)
- [ ] PKCE verifier stored via `ToolContext::key_value_store`
- [ ] Rotated refresh tokens persisted immediately
- [ ] `client_secret` never logged or serialized (skip_serializing)
- [ ] Provider name sanitized to prevent directory traversal
- [ ] `cargo test -p clawft-plugin-oauth2` -- all tests pass
- [ ] `cargo clippy -p clawft-plugin-oauth2 -- -D warnings` is clean

### F9a: MCP Client MVP

- [ ] `McpToolProvider` implements `ToolProvider` trait
- [ ] External tools namespaced as `mcp:<server-name>:<tool-name>`
- [ ] `connect_mcp_server` factory supports stdio and HTTP transports
- [ ] Stdio child processes spawned with minimal env (no secret inheritance)
- [ ] Command paths validated (no `..`, no shell metacharacters)
- [ ] Tool cache populated via `refresh_tools()`
- [ ] `CallToolResult` correctly maps `isError` from MCP response
- [ ] `McpServerConfig` serializable/deserializable for configuration files
- [ ] All existing MCP tests continue to pass
- [ ] `cargo test -p clawft-services` -- all tests pass
- [ ] `cargo clippy -p clawft-services -- -D warnings` is clean

---

## 8. Implementation Order

F6 is highest priority (blocks E5a Google Chat). Recommended order:

```
F6 (OAuth2) ──> F1 (Git) ──> F2 (Cargo) ──> F9a (MCP Client)
     │                                            │
     │                                            ▼
     └──────> E5a (Google Chat)            F9b (Full MCP Client)
```

F1 and F2 are independent of each other and can be parallelized. F9a depends on D9 (MCP transport concurrency from Element 05) landing first.

---

## 9. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| `git2` C library FFI causes build issues on some platforms | Medium | Medium | **4** | Pin `git2` version; document system deps (libgit2, libssh2, openssl); provide build guide |
| OAuth2 token storage file permission bypass on non-Unix | Low | High | **3** | On Windows, use `SetFileAttributes` for restricted access; document platform differences |
| Cargo subprocess hangs on large builds | Medium | Low | **2** | Add configurable timeout (default 300s) to all cargo operations; kill process on timeout |
| F6 delayed past Week 6 blocks E5a Google Chat | Low | High | **3** | F6 is P0 priority; assign dedicated developer; E5a can use mock OAuth2 for early testing |
| MCP stdio command path validation too restrictive | Low | Low | **1** | Allow configurable path allowlist; document required paths for common MCP servers |
| `oauth2` crate API changes between versions | Low | Low | **1** | Pin `oauth2 = "5"` major version; use only stable API surface |
| PKCE verifier storage race with concurrent auth flows | Low | Medium | **2** | Key by unique `state` parameter; TTL on stored verifiers (10 minutes) |
