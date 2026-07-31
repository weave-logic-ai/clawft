//! Concierge agent identity loader.
//!
//! Resolves the WeftOS Concierge's persona content (`SOUL.md`,
//! `IDENTITY.md`) for use as the system-prompt foundation.
//!
//! ## Resolution
//!
//! Per-instance only: `<workspace>/.clawft/SOUL.md` and
//! `<workspace>/.clawft/IDENTITY.md`. The directory is materialized
//! by `weaver init` (Phase F1), so every initialized workspace boots
//! with both files in place. If they're missing the loader returns
//! [`IdentityError::NotFound`] and the daemon's chat path surfaces
//! `agent: identity load failed: ...`.
//!
//! Phase F1 deleted the previous `docs/skills/clawft/` fallback the
//! spike used while `weaver init` did not yet seed `.clawft/`.
//!
//! ## What this module DOES (Phase D1 + F1, agent-core-v1)
//!
//! - SHA-256 (hex) hash of `SOUL.md + "\n" + IDENTITY.md` as the
//!   identity descriptor surfaced in logs and the system prompt.
//! - [`IdentityProvider`] async trait so [`AgentLoop`] is testable
//!   without filesystem IO. [`FileIdentityProvider`] caches the most
//!   recent successful load.
//! - [`BINDING_THREAD_EXCERPT`] compile-time constant that the
//!   `SystemPromptBuilder` / gate path checks against loaded SOUL.md.
//!   **WEFT-342 (v1.1 default):** mismatch hard-refuses the turn via
//!   governance rule `soul.binding_thread_intact` (`GateDecision::Deny`).
//!   Operators can set `agents.binding_thread_mode = "warn_only"` to
//!   restore the legacy annotate + `warn!` degraded path.
//!
//! ## What this is NOT (yet)
//!
//! - **No hot-reload watcher** — the cached `FileIdentityProvider`
//!   re-reads on every call (small files; cheap). A `notify`-driven
//!   watcher arrives when measurement says it earns its keep.
//!
//! ## SOUL.journal write path (WEFT-330)
//!
//! Agent-side drift observations during chat turns are handled by
//! [`super::soul_journal`] — hooked from `loop_core` after a
//! successful turn when the documented drift signal fires. F1 seeds
//! the empty journal file and stamps the `soul_journal` derived-write
//! grant; F2's `weaver soul promote` reads substrate entries, diffs,
//! and applies on confirmation. The journal is still not consulted on
//! every-turn identity loads (identity = `SOUL.md` + `IDENTITY.md`
//! only).
//!
//! Plan reference: `docs/plans/agent-core-v1.md` Phase D1, F1.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clawft_platform::Platform;

use crate::runtime::RwLock;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{debug, warn};

/// Distinctive paragraph from the canonical `SOUL.md` used as the
/// compile-time witness for the binding-thread integrity check.
///
/// The check is operated every turn by
/// [`SystemPromptBuilder`](crate::agent::system_prompt::SystemPromptBuilder)
/// and by `gate.check("soul.binding_thread_intact", …)` (WEFT-342):
///
/// - **Default (`agents.binding_thread_mode = "deny"`):** if loaded
///   `SOUL.md` does not contain this excerpt, the turn hard-refuses
///   with [`IdentityError::BindingThreadMismatch`] /
///   `GateDecision::Deny { reason: "binding-thread mismatch" }`.
/// - **Legacy (`warn_only`):** annotate the prompt
///   `binding-thread-status: mismatch` and emit a `warn!` log; the
///   agent continues in degraded mode.
///
/// Source: `docs/skills/clawft/SOUL.md` §"Core Personality Traits" /
/// "The Binding Thread" — quoted verbatim so the substring search is
/// stable across whitespace-only edits to the surrounding paragraph.
pub const BINDING_THREAD_EXCERPT: &str =
    "an agent must not diminish human capability, or by inaction allow it to be diminished";

/// Governance / gate action name for the binding-thread integrity rule
/// (WEFT-342). Evaluated by `EffectGate::check` on every chat turn.
pub const BINDING_THREAD_GATE_ACTION: &str = "soul.binding_thread_intact";

/// Canonical hard-refusal reason returned by the gate / identity path
/// when SOUL.md is missing the binding-thread excerpt (WEFT-342).
pub const BINDING_THREAD_MISMATCH_REASON: &str = "binding-thread mismatch";

/// Loaded identity content.
#[derive(Debug, Clone)]
pub struct Identity {
    /// `SOUL.md` content — persona, ethical constraints, values.
    pub soul: String,
    /// `IDENTITY.md` content — operational identity, skills, tone.
    pub identity: String,
    /// SHA-256 (hex, lowercase) of `soul + "\n" + identity`. Surfaced
    /// in logs and as the trailing `[hash]` line of the system prompt.
    /// Phase D1 replaced the spike's `len(soul)+len(identity)`
    /// placeholder.
    pub hash: String,
    /// Source of the loaded files. Always `"clawft"` after F1 (the
    /// `docs/skills/clawft/` fallback was removed). The field is
    /// retained as a `&'static str` so a future substrate-backed
    /// provider can introduce new variants without touching callers.
    pub source: &'static str,
}

/// Errors emitted by the identity load path.
///
/// Distinguishes missing seed files from binding-thread integrity
/// failures so the chat path can surface distinct RPC messages.
/// Variants stay shaped for forward compatibility (substrate-backed
/// loaders may add IO / deserialization variants later).
#[derive(Debug, Error)]
pub enum IdentityError {
    /// `<workspace>/.clawft/SOUL.md` or `IDENTITY.md` (or both) are
    /// missing. Callers treat this as a hard failure for the chat
    /// path. Run `weaver init` to materialize the seed files.
    #[error(
        "identity load failed: <workspace>/.clawft/{{SOUL.md,IDENTITY.md}} missing — run `weaver init`"
    )]
    NotFound,

    /// Loaded `SOUL.md` does not contain [`BINDING_THREAD_EXCERPT`] and
    /// the operator policy is hard-refuse (WEFT-342 default).
    ///
    /// Surfaced by `SystemPromptBuilder` when
    /// `agents.binding_thread_mode = "deny"`. The agent loop maps this
    /// to a turn-level error after `gate.check("soul.binding_thread_intact")`
    /// returns `Deny { reason: "binding-thread mismatch" }`.
    #[error("identity load failed: binding-thread mismatch")]
    BindingThreadMismatch,
}

/// Return `true` when `soul` contains the compile-time
/// [`BINDING_THREAD_EXCERPT`] substring (case-sensitive).
///
/// Shared by the system-prompt builder, gate evaluation helpers, and
/// unit tests so the check stays a single source of truth.
pub fn soul_contains_binding_thread(soul: &str) -> bool {
    soul.contains(BINDING_THREAD_EXCERPT)
}

/// Async interface for retrieving the agent's current identity.
///
/// Decouples `loop_core` and `SystemPromptBuilder` from the on-disk
/// loader so they can be exercised against in-memory fixtures. The
/// substrate-backed identity provider (Phase F1) will plug in here
/// without any caller-site changes.
#[async_trait]
pub trait IdentityProvider: Send + Sync + 'static {
    /// Return the current identity. Called once per turn; impls
    /// should be cheap (cached IO).
    async fn current(&self) -> Result<Identity, IdentityError>;
}

/// Filesystem-backed [`IdentityProvider`] that re-reads on every call
/// and caches the most recent successful load.
///
/// All IO goes through [`Platform::fs`] (WEFT-95 / MW-17) so the path
/// is native/WASM portable — the same mock / browser backends used by
/// the rest of the agent loop exercise identity loads without
/// `std::fs`.
///
/// The cache lets repeated calls within a turn skip the disk hit;
/// cross-turn changes (the user editing `SOUL.md` between turns) are
/// picked up on the next call because the loader still tries the disk
/// first. The cache is only consulted as a fallback when both the
/// per-instance and fallback paths fail to resolve.
pub struct FileIdentityProvider<P: Platform> {
    workspace: PathBuf,
    platform: Arc<P>,
    cached: RwLock<Option<Identity>>,
}

impl<P: Platform> FileIdentityProvider<P> {
    /// Build a provider rooted at the given workspace directory.
    pub fn new(workspace: impl Into<PathBuf>, platform: Arc<P>) -> Self {
        Self {
            workspace: workspace.into(),
            platform,
            cached: RwLock::new(None),
        }
    }

    /// Return a reference to the workspace root.
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

#[async_trait]
impl<P: Platform + 'static> IdentityProvider for FileIdentityProvider<P> {
    async fn current(&self) -> Result<Identity, IdentityError> {
        let loader = IdentityLoader::new(self.workspace.clone(), Arc::clone(&self.platform));
        match loader.current().await {
            Some(id) => {
                let mut cache = self.cached.write().await;
                *cache = Some(id.clone());
                Ok(id)
            }
            None => {
                // Disk read failed — surface the cached value if we
                // ever loaded one, otherwise propagate the error so
                // the daemon's chat path returns the "identity load
                // failed" RPC error.
                if let Some(cached) = self.cached.read().await.clone() {
                    warn!(
                        "identity provider: disk re-read failed; \
                         serving cached load (hash={})",
                        cached.hash
                    );
                    return Ok(cached);
                }
                Err(IdentityError::NotFound)
            }
        }
    }
}

/// Resolves and reads identity content via [`Platform::fs`] (WEFT-95).
///
/// Async so WASM / browser backends (and in-memory mocks) can serve
/// the same path as native `tokio::fs` without a sync `std::fs` bypass.
pub struct IdentityLoader<P: Platform> {
    workspace: PathBuf,
    platform: Arc<P>,
}

impl<P: Platform> IdentityLoader<P> {
    /// Build a loader rooted at the given workspace directory.
    ///
    /// The workspace is typically the resolved
    /// [`clawft_types::config::AgentsConfig::workspace_root`]
    /// (WEFT-83 / plan §15.4). When that config key is unset, callers
    /// pass the daemon process CWD. `platform` supplies
    /// [`Platform::fs`] for all identity file reads (WEFT-95).
    pub fn new(workspace: impl Into<PathBuf>, platform: Arc<P>) -> Self {
        Self {
            workspace: workspace.into(),
            platform,
        }
    }

    /// Build a loader from [`clawft_types::config::AgentsConfig`].
    ///
    /// Uses `agents.workspace_root` when present; otherwise falls back
    /// to `std::env::current_dir()` (back-compat with pre-WEFT-83
    /// daemons).
    pub fn from_agents_config(
        agents: &clawft_types::config::AgentsConfig,
        platform: Arc<P>,
    ) -> std::io::Result<Self> {
        Ok(Self::new(agents.resolve_workspace_root()?, platform))
    }

    /// Return the workspace root this loader reads from.
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// Load the current identity from `<workspace>/.clawft/`.
    ///
    /// Returns `None` when either `SOUL.md` or `IDENTITY.md` is
    /// absent. Callers treat that as a hard failure for the chat path
    /// (the daemon's `agent.chat` handler returns
    /// `agent: identity load failed: ...`).
    ///
    /// F1 removed the `docs/skills/clawft/` fallback the spike used
    /// while `weaver init` did not yet seed `.clawft/`. Every
    /// initialized workspace now boots with the seed files in place.
    ///
    /// Reads go through [`Platform::fs`] (WEFT-95 / MW-17) — no
    /// `std::fs` on this path.
    pub async fn current(&self) -> Option<Identity> {
        self.try_load_from(&self.workspace.join(".clawft"), "clawft")
            .await
    }

    async fn try_load_from(&self, dir: &Path, source: &'static str) -> Option<Identity> {
        let soul_path = dir.join("SOUL.md");
        let identity_path = dir.join("IDENTITY.md");
        let soul = self.platform.fs().read_to_string(&soul_path).await.ok()?;
        let identity = self
            .platform
            .fs()
            .read_to_string(&identity_path)
            .await
            .ok()?;
        debug!(?soul_path, ?identity_path, source, "identity loaded");
        let hash = sha256_identity_hash(&soul, &identity);
        Some(Identity {
            soul,
            identity,
            hash,
            source,
        })
    }
}

/// Compute the SHA-256 (hex, lowercase) of `soul + "\n" + identity`.
///
/// Centralised so tests and the future substrate-backed identity
/// provider produce the exact same descriptor as the on-disk loader.
pub fn sha256_identity_hash(soul: &str, identity: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(soul.as_bytes());
    hasher.update(b"\n");
    hasher.update(identity.as_bytes());
    let digest = hasher.finalize();
    hex_lower(&digest)
}

/// Render a byte slice as a lowercase hex string. Avoids pulling
/// `hex` as a new dep.
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use clawft_platform::fs::{FileSystem, FsMetadata};
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;

    // ── In-memory platform (WASM-equivalent path, WEFT-95) ─────────
    //
    // Exercises IdentityLoader via Platform::fs without std::fs reads —
    // the same shape browser / WASI backends use.

    struct MockFs {
        files: StdMutex<HashMap<PathBuf, String>>,
    }

    impl MockFs {
        fn new() -> Self {
            Self {
                files: StdMutex::new(HashMap::new()),
            }
        }

        fn seed(&self, path: impl Into<PathBuf>, content: impl Into<String>) {
            self.files
                .lock()
                .unwrap()
                .insert(path.into(), content.into());
        }
    }

    #[async_trait]
    impl FileSystem for MockFs {
        async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
            self.files.lock().unwrap().get(path).cloned().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("file not found: {}", path.display()),
                )
            })
        }

        async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
            self.files
                .lock()
                .unwrap()
                .insert(path.to_path_buf(), content.to_string());
            Ok(())
        }

        async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
            let mut files = self.files.lock().unwrap();
            files.entry(path.to_path_buf()).or_default().push_str(content);
            Ok(())
        }

        async fn exists(&self, path: &Path) -> bool {
            self.files.lock().unwrap().contains_key(path)
        }

        async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .lock()
                .unwrap()
                .keys()
                .filter(|p| p.parent() == Some(path))
                .cloned()
                .collect())
        }

        async fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }

        async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }

        fn home_dir(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/mock-home"))
        }

        async fn metadata(&self, path: &Path) -> std::io::Result<FsMetadata> {
            if let Some(content) = self.files.lock().unwrap().get(path) {
                return Ok(FsMetadata {
                    is_dir: false,
                    len: content.len() as u64,
                });
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            ))
        }
    }

    struct MockEnv;
    impl clawft_platform::env::Environment for MockEnv {
        fn get_var(&self, _name: &str) -> Option<String> {
            None
        }
        fn set_var(&self, _name: &str, _value: &str) {}
        fn remove_var(&self, _name: &str) {}
    }

    struct MockHttp;
    #[async_trait]
    impl clawft_platform::http::HttpClient for MockHttp {
        async fn request(
            &self,
            _method: &str,
            _url: &str,
            _headers: &HashMap<String, String>,
            _body: Option<&[u8]>,
        ) -> Result<clawft_platform::http::HttpResponse, Box<dyn std::error::Error + Send + Sync>>
        {
            Err("MockHttp unused in identity tests".into())
        }
    }

    struct MockPlatform {
        fs: MockFs,
        env: MockEnv,
        http: MockHttp,
    }

    impl MockPlatform {
        fn new() -> Self {
            Self {
                fs: MockFs::new(),
                env: MockEnv,
                http: MockHttp,
            }
        }
    }

    #[async_trait]
    impl Platform for MockPlatform {
        fn http(&self) -> &dyn clawft_platform::http::HttpClient {
            &self.http
        }
        fn fs(&self) -> &dyn FileSystem {
            &self.fs
        }
        fn env(&self) -> &dyn clawft_platform::env::Environment {
            &self.env
        }
        fn process(&self) -> Option<&dyn clawft_platform::process::ProcessSpawner> {
            None
        }
    }

    fn mock_platform() -> Arc<MockPlatform> {
        Arc::new(MockPlatform::new())
    }

    #[cfg(feature = "native")]
    fn native_platform() -> Arc<clawft_platform::NativePlatform> {
        Arc::new(clawft_platform::NativePlatform::new())
    }

    #[test]
    fn binding_thread_excerpt_is_non_empty() {
        assert!(!BINDING_THREAD_EXCERPT.is_empty());
        assert!(BINDING_THREAD_EXCERPT.len() > 16);
        assert_eq!(BINDING_THREAD_GATE_ACTION, "soul.binding_thread_intact");
        assert_eq!(BINDING_THREAD_MISMATCH_REASON, "binding-thread mismatch");
    }

    #[test]
    fn soul_contains_binding_thread_helper() {
        assert!(soul_contains_binding_thread(&format!(
            "prefix {BINDING_THREAD_EXCERPT} suffix"
        )));
        assert!(!soul_contains_binding_thread("nothing relevant here"));
    }

    #[test]
    fn sha256_hash_matches_known_vector() {
        // Reference: printf 'hello\nworld' | sha256sum
        //   26c60a61d01db5836ca70fefd44a6a016620413c8ef5f259a6c5612d4f79d3b8
        // Composition is `soul + "\n" + identity` so passing
        // soul="hello", identity="world" reproduces the canonical
        // "hello\nworld" digest.
        let h = sha256_identity_hash("hello", "world");
        assert_eq!(
            h,
            "26c60a61d01db5836ca70fefd44a6a016620413c8ef5f259a6c5612d4f79d3b8"
        );
        assert_eq!(h.len(), 64); // SHA-256 hex is 64 chars
        // Hash is deterministic — repeated calls return the same value.
        assert_eq!(h, sha256_identity_hash("hello", "world"));
        // Distinct inputs produce distinct hashes.
        assert_ne!(h, sha256_identity_hash("hello", "WORLD"));
    }

    /// WEFT-95: in-memory Platform::fs path (WASM-equivalent — no std::fs).
    #[tokio::test]
    async fn loads_via_platform_fs_mock_no_std_fs() {
        let platform = mock_platform();
        let workspace = PathBuf::from("/wasm-ws");
        let clawft = workspace.join(".clawft");
        platform
            .fs
            .seed(clawft.join("SOUL.md"), "soul content");
        platform
            .fs
            .seed(clawft.join("IDENTITY.md"), "identity content");

        let loader = IdentityLoader::new(&workspace, Arc::clone(&platform));
        let id = loader.current().await.expect("should load via mock fs");
        assert_eq!(id.soul, "soul content");
        assert_eq!(id.identity, "identity content");
        assert_eq!(id.source, "clawft");
        assert_eq!(
            id.hash,
            sha256_identity_hash("soul content", "identity content")
        );
        assert_eq!(id.hash.len(), 64);
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn loads_from_clawft_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let clawft = tmp.path().join(".clawft");
        std::fs::create_dir_all(&clawft).unwrap();
        std::fs::write(clawft.join("SOUL.md"), "soul content").unwrap();
        std::fs::write(clawft.join("IDENTITY.md"), "identity content").unwrap();

        let loader = IdentityLoader::new(tmp.path(), native_platform());
        let id = loader.current().await.expect("should load");
        assert_eq!(id.soul, "soul content");
        assert_eq!(id.identity, "identity content");
        assert_eq!(id.source, "clawft");
        // Hash must be SHA-256 hex of `"soul content" + "\n" + "identity content"`.
        assert_eq!(
            id.hash,
            sha256_identity_hash("soul content", "identity content")
        );
        assert_eq!(id.hash.len(), 64);
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn does_not_load_from_docs_skills_anymore() {
        // F1 removed the docs/skills/clawft/ fallback. Even when only
        // the docs path is populated, the loader returns None — the
        // chat path must emit `identity load failed: ... run `weaver
        // init`` rather than silently substituting bundled docs.
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("docs").join("skills").join("clawft");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("SOUL.md"), "doc soul").unwrap();
        std::fs::write(docs.join("IDENTITY.md"), "doc identity").unwrap();

        let loader = IdentityLoader::new(tmp.path(), native_platform());
        assert!(
            loader.current().await.is_none(),
            "post-F1: docs/skills/clawft/ must not satisfy the loader"
        );
    }

    #[tokio::test]
    async fn returns_none_when_clawft_dir_missing() {
        let loader = IdentityLoader::new(PathBuf::from("/empty-ws"), mock_platform());
        assert!(loader.current().await.is_none());
    }

    #[tokio::test]
    async fn returns_none_when_only_one_file_present() {
        // Half-populated .clawft/ should fail loud, not partial-load.
        let platform = mock_platform();
        let workspace = PathBuf::from("/half-ws");
        platform
            .fs
            .seed(workspace.join(".clawft").join("SOUL.md"), "soul only");

        let loader = IdentityLoader::new(&workspace, platform);
        assert!(loader.current().await.is_none());
    }

    #[tokio::test]
    async fn two_workspaces_load_distinct_identities() {
        // WEFT-83: configure two workspace roots with different identity
        // files; IdentityLoader (driven by agents.workspace_root) must
        // resolve each independently — not the process CWD.
        let platform = mock_platform();
        let alpha = PathBuf::from("/ws-alpha");
        let beta = PathBuf::from("/ws-beta");
        for (root, soul, id) in [
            (&alpha, "soul-alpha", "id-alpha"),
            (&beta, "soul-beta", "id-beta"),
        ] {
            let clawft = root.join(".clawft");
            platform.fs.seed(clawft.join("SOUL.md"), soul);
            platform.fs.seed(clawft.join("IDENTITY.md"), id);
        }

        let mut cfg_alpha = clawft_types::config::AgentsConfig::default();
        cfg_alpha.workspace_root = Some(alpha.clone());
        let mut cfg_beta = clawft_types::config::AgentsConfig::default();
        cfg_beta.workspace_root = Some(beta.clone());

        let loader_a =
            IdentityLoader::from_agents_config(&cfg_alpha, Arc::clone(&platform)).unwrap();
        let loader_b =
            IdentityLoader::from_agents_config(&cfg_beta, Arc::clone(&platform)).unwrap();

        let id_a = loader_a.current().await.expect("alpha identity");
        let id_b = loader_b.current().await.expect("beta identity");
        assert_eq!(id_a.soul, "soul-alpha");
        assert_eq!(id_b.soul, "soul-beta");
        assert_ne!(id_a.hash, id_b.hash);
        assert_eq!(loader_a.workspace(), alpha.as_path());
        assert_eq!(loader_b.workspace(), beta.as_path());
    }

    #[test]
    fn from_agents_config_falls_back_to_cwd_when_unset() {
        let cwd = std::env::current_dir().unwrap();
        let cfg = clawft_types::config::AgentsConfig::default();
        assert!(cfg.workspace_root.is_none());
        let loader = IdentityLoader::from_agents_config(&cfg, mock_platform()).unwrap();
        assert_eq!(loader.workspace(), cwd.as_path());
    }

    // ── FileIdentityProvider tests ────────────────────────────────

    #[tokio::test]
    async fn file_provider_loads_and_caches() {
        let platform = mock_platform();
        let workspace = PathBuf::from("/prov-ws");
        let clawft = workspace.join(".clawft");
        platform.fs.seed(clawft.join("SOUL.md"), "soul-1");
        platform.fs.seed(clawft.join("IDENTITY.md"), "id-1");

        let provider = FileIdentityProvider::new(&workspace, Arc::clone(&platform));
        let first = provider.current().await.expect("first load");
        assert_eq!(first.soul, "soul-1");

        // Mutate the files between calls — provider must observe the
        // change because every call re-reads from disk.
        platform.fs.seed(clawft.join("SOUL.md"), "soul-2");
        let second = provider.current().await.expect("second load");
        assert_eq!(second.soul, "soul-2");
        assert_ne!(first.hash, second.hash);
    }

    #[tokio::test]
    async fn file_provider_serves_cache_when_disk_disappears() {
        let platform = mock_platform();
        let workspace = PathBuf::from("/cache-ws");
        let clawft = workspace.join(".clawft");
        platform.fs.seed(clawft.join("SOUL.md"), "cached-soul");
        platform.fs.seed(clawft.join("IDENTITY.md"), "cached-id");

        let provider = FileIdentityProvider::new(&workspace, Arc::clone(&platform));
        let first = provider.current().await.expect("warm cache");

        // Remove the files; the cache should still resolve.
        platform
            .fs()
            .remove_file(&clawft.join("SOUL.md"))
            .await
            .unwrap();
        platform
            .fs()
            .remove_file(&clawft.join("IDENTITY.md"))
            .await
            .unwrap();

        let cached = provider.current().await.expect("cache fallback");
        assert_eq!(cached.soul, first.soul);
        assert_eq!(cached.hash, first.hash);
    }

    #[tokio::test]
    async fn file_provider_returns_not_found_with_no_cache() {
        let provider = FileIdentityProvider::new(PathBuf::from("/missing"), mock_platform());
        let err = provider.current().await.unwrap_err();
        assert!(matches!(err, IdentityError::NotFound));
    }
}
