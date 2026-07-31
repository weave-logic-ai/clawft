//! Production-wire probe for config load → PermissionResolver (WEFT-327).
//!
//! Was an `#[ignore]` diagnostic against a developer home path
//! (`/home/aepod/dev/clawft`). Now runs hermetically in default
//! `cargo test` / `scripts/build.sh test` using checked-in fixtures under
//! `tests/fixtures/` and a temp-dir workspace + temp home.
//!
//! Must not depend on the process starting cwd (non-clawft roots are fine):
//! fixtures are embedded via `include_str!` and written into a `TempDir`.

use std::env;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clawft_core::pipeline::permissions::PermissionResolver;
use clawft_platform::config_loader::load_config_layers;
use clawft_platform::fs::{FileSystem, FsMetadata};
use clawft_types::config::Config;
use clawft_types::routing::RoutingConfig;
use tokio::sync::Mutex;

/// Embedded fixtures — compile-time, cwd-independent.
const GLOBAL_FIXTURE: &str = include_str!("fixtures/resolver_global.json");
const WORKSPACE_FIXTURE: &str = include_str!("fixtures/resolver_workspace.json");

/// Process-global lock for `set_current_dir` (process-wide side effect).
/// Async mutex so the guard can be held across `load_config_layers(...).await`.
static CWD_LOCK: Mutex<()> = Mutex::const_new(());

/// RAII guard: restore prior cwd on drop (test failure must not leave
/// the runner in a temp dir about to be deleted).
struct CwdGuard {
    original: PathBuf,
}

impl CwdGuard {
    fn switch_to(target: &Path) -> Self {
        let original = env::current_dir().expect("read cwd");
        env::set_current_dir(target).expect("switch cwd");
        Self { original }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original);
    }
}

/// Environment that ignores the real process env so `CLAWFT_CONFIG`
/// cannot point at a developer machine path.
struct EmptyEnv;

impl clawft_platform::env::Environment for EmptyEnv {
    fn get_var(&self, _name: &str) -> Option<String> {
        None
    }
    fn set_var(&self, _name: &str, _value: &str) {}
    fn remove_var(&self, _name: &str) {}
}

/// Native FS I/O with a controlled `home_dir` so Layer-2 discovery
/// (`~/.clawft/config.json`) is hermetic and cannot leak host config.
struct HermeticFs {
    home: PathBuf,
}

#[async_trait]
impl FileSystem for HermeticFs {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        tokio::fs::read_to_string(path).await
    }

    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await
    }

    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            entries.push(entry.path());
        }
        Ok(entries)
    }

    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        tokio::fs::create_dir_all(path).await
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        tokio::fs::remove_file(path).await
    }

    fn home_dir(&self) -> Option<PathBuf> {
        Some(self.home.clone())
    }

    async fn metadata(&self, path: &Path) -> std::io::Result<FsMetadata> {
        let meta = tokio::fs::metadata(path).await?;
        Ok(FsMetadata {
            is_dir: meta.is_dir(),
            len: meta.len(),
        })
    }
}

/// Stage hermetic home + workspace dirs, switch cwd, load production layers.
///
/// Returns owned temp dirs (keep them alive), global routing, and optional
/// workspace routing — the same split the daemon hands to `PermissionResolver`.
async fn load_probe_layers() -> (tempfile::TempDir, tempfile::TempDir, RoutingConfig, Option<RoutingConfig>) {
    let home = tempfile::TempDir::new().expect("temp home");
    let workspace = tempfile::TempDir::new().expect("temp workspace");

    // Layer 2: home JSON (system-wide ceiling / base channel grants).
    let home_clawft = home.path().join(".clawft");
    std::fs::create_dir_all(&home_clawft).expect("mkdir home .clawft");
    std::fs::write(home_clawft.join("config.json"), GLOBAL_FIXTURE).expect("write global fixture");

    // Layer 3: cwd-relative workspace overlay.
    let ws_clawft = workspace.path().join(".clawft");
    std::fs::create_dir_all(&ws_clawft).expect("mkdir workspace .clawft");
    std::fs::write(ws_clawft.join("config.json"), WORKSPACE_FIXTURE)
        .expect("write workspace fixture");

    // Caller must hold CWD_LOCK across this await.
    let _cwd = CwdGuard::switch_to(workspace.path());
    let fs = HermeticFs {
        home: home.path().to_path_buf(),
    };
    let layers = load_config_layers(&fs, &EmptyEnv)
        .await
        .expect("load_config_layers");

    assert!(
        layers.has_workspace(),
        "workspace overlay must load from cwd .clawft/config.json; layers={layers:?}"
    );

    let global_cfg: Config =
        serde_json::from_value(layers.global.clone()).expect("deserialize global Config");
    let workspace_routing = layers.workspace.map(|ws| {
        let cfg: Config = serde_json::from_value(ws).expect("deserialize workspace Config");
        cfg.routing
    });

    (home, workspace, global_cfg.routing, workspace_routing)
}

#[tokio::test]
async fn probe_chat_resolution_against_hermetic_config() {
    // Serializes cwd mutation; holds lock across the await that discovers
    // the workspace path relative to process cwd.
    let _guard = CWD_LOCK.lock().await;

    let (_home, _workspace, global_routing, workspace_routing) = load_probe_layers().await;

    // Sanity: fixtures reached the loader with camelCase → snake_case normalization.
    assert!(
        global_routing
            .permissions
            .channels
            .get("agent.chat")
            .and_then(|c| c.level)
            == Some(2),
        "global fixture must grant agent.chat level 2"
    );
    let ws = workspace_routing
        .as_ref()
        .expect("workspace routing present");
    let chat_ch = ws
        .permissions
        .channels
        .get("agent.chat")
        .expect("workspace agent.chat channel");
    assert_eq!(
        chat_ch.tool_access.as_ref().map(|v| v.len()),
        Some(3),
        "workspace toolAccess must normalize and deserialize"
    );

    // Production wire: split global + workspace so enforce_workspace_ceiling runs.
    let resolver = PermissionResolver::new(&global_routing, workspace_routing.as_ref());

    // Original diagnostic target: panel / agent.chat.
    let chat = resolver.resolve("panel", "agent.chat", false);
    assert_eq!(
        chat.level, 2,
        "agent.chat channel grant must resolve to level 2; got {chat:?}"
    );
    assert_eq!(
        chat.tool_access,
        vec![
            "read_file".to_string(),
            "message".to_string(),
            "list_dir".to_string()
        ],
        "workspace tool_access overlay must win on agent.chat"
    );
    assert_eq!(
        chat.tool_denylist,
        vec!["shell".to_string()],
        "workspace tool_denylist must apply to agent.chat"
    );

    // Second diagnostic target: local / cli.
    let cli = resolver.resolve("local", "cli", false);
    assert_eq!(
        cli.level, 2,
        "cli channel (global fixture) must resolve to admin level 2; got {cli:?}"
    );
    assert!(
        cli.tool_access.iter().any(|t| t == "*"),
        "cli should retain wildcard tool access from global fixture; got {:?}",
        cli.tool_access
    );
}

#[tokio::test]
async fn probe_loads_when_started_from_unrelated_cwd() {
    // Explicit AC: must not break when run from a non-clawft cwd.
    // Start in a decoy temp dir, then load from a separately staged
    // workspace — discovery is relative to the switched cwd, not the
    // cargo/test runner start directory.
    let _guard = CWD_LOCK.lock().await;

    let decoy = tempfile::TempDir::new().expect("decoy cwd");
    {
        let _decoy_cwd = CwdGuard::switch_to(decoy.path());
        // Still no .clawft here — just establish a non-repo cwd.
        assert!(
            !decoy.path().join(".clawft").exists(),
            "decoy must not look like a clawft workspace"
        );
    }

    let (_home, _workspace, global_routing, workspace_routing) = load_probe_layers().await;
    let resolver = PermissionResolver::new(&global_routing, workspace_routing.as_ref());
    let chat = resolver.resolve("panel", "agent.chat", false);
    assert_eq!(
        chat.level, 2,
        "hermetic probe must succeed after starting in an unrelated cwd"
    );
}
