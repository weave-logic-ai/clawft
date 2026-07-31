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
//! ## SOUL.journal (WEFT-330 write + WEFT-96 read-on-every-turn)
//!
//! Agent-side drift observations during chat turns are handled by
//! [`super::soul_journal`] — hooked from `loop_core` after a
//! successful turn when the documented drift signal fires. F1 seeds
//! the empty journal file and stamps the `soul_journal` derived-write
//! grant; F2's `weaver soul promote` reads substrate entries, diffs,
//! and applies on confirmation.
//!
//! **WEFT-96 / WS-D1:** every-turn identity loads may consult pending
//! journal entries via [`JournalAwareIdentityProvider`] +
//! [`SoulJournalReader`](super::soul_journal::SoulJournalReader). The
//! base persona remains `SOUL.md` + `IDENTITY.md` (human promote still
//! owns merges); pending observations surface as
//! [`Identity::pending_journal`] with
//! [`IdentitySource::Journal`] when non-empty. Production substrate
//! reader lives in `clawft-service-agent`.
//!
//! Plan reference: `docs/plans/agent-core-v1.md` Phase D1, F1;
//! WEFT-96 / WEFT-97 (WS-D1 / WS-D4 / WS-D5).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clawft_platform::Platform;

use crate::agent::soul_journal::{PendingJournalEntry, SoulJournalReader};
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

/// Provenance of a loaded [`Identity`] (WEFT-97 / WS-D4).
///
/// Wire form is the stable [`Self::as_str`] label so
/// [`AgentChatResult::identity_source`](clawft_types::agent_chat::AgentChatResult)
/// stays a free-form string without forcing callers onto the enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentitySource {
    /// `<workspace>/.clawft/{SOUL,IDENTITY}.md` via [`Platform::fs`].
    Clawft,
    /// [`FileIdentityProvider`] served a previously cached load after
    /// the disk re-read failed.
    Cache,
    /// Substrate-backed identity load (mesh-canonical path).
    Substrate,
    /// Base identity plus pending soul-journal observations consulted
    /// on this turn (WEFT-96 / WS-D1).
    Journal,
    /// In-test / stub provider.
    Stub,
}

impl IdentitySource {
    /// Stable wire / log label (e.g. `"clawft"`, `"journal"`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clawft => "clawft",
            Self::Cache => "cache",
            Self::Substrate => "substrate",
            Self::Journal => "journal",
            Self::Stub => "stub",
        }
    }
}

impl fmt::Display for IdentitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

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
    /// placeholder. Pending journal entries do **not** change the hash
    /// (human promote still owns SOUL merges).
    pub hash: String,
    /// Provenance of this load (WEFT-97 / WS-D4).
    pub source: IdentitySource,
    /// Pending soul-journal observations consulted this turn
    /// (WEFT-96 / WS-D1). Empty when no journal reader is attached or
    /// the journal has no pending rows. Never auto-applied to `soul`.
    pub pending_journal: Vec<PendingJournalEntry>,
}

impl Identity {
    /// Build a file-backed identity with no pending journal rows.
    pub fn from_files(
        soul: impl Into<String>,
        identity: impl Into<String>,
        source: IdentitySource,
    ) -> Self {
        let soul = soul.into();
        let identity = identity.into();
        let hash = sha256_identity_hash(&soul, &identity);
        Self {
            soul,
            identity,
            hash,
            source,
            pending_journal: Vec::new(),
        }
    }
}

/// Errors emitted by the identity load path.
///
/// Distinguishes missing seed files, binding-thread integrity failures,
/// and substrate / IO / deserialize failures so the chat path can
/// surface distinct RPC messages (WEFT-97 / WS-D5).
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

    /// Platform filesystem or other IO failure while loading identity
    /// or the journal (WEFT-97 / WS-D5).
    #[error("identity load failed: IO error: {0}")]
    Io(String),

    /// Failed to deserialize a substrate journal / identity payload
    /// (WEFT-97 / WS-D5).
    #[error("identity load failed: deserialize error: {0}")]
    Deserialize(String),

    /// Substrate list/read path failed or was denied (WEFT-97 / WS-D5).
    #[error("identity load failed: substrate error: {0}")]
    Substrate(String),
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
/// loader so they can be exercised against in-memory fixtures.
/// Substrate-backed and journal-aware providers (WEFT-96 / WEFT-97)
/// plug in here without caller-site changes.
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
/// first. The cache is only consulted as a fallback when the disk
/// path fails to resolve; that fallback is tagged
/// [`IdentitySource::Cache`].
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
                if let Some(mut cached) = self.cached.read().await.clone() {
                    warn!(
                        "identity provider: disk re-read failed; \
                         serving cached load (hash={})",
                        cached.hash
                    );
                    cached.source = IdentitySource::Cache;
                    return Ok(cached);
                }
                Err(IdentityError::NotFound)
            }
        }
    }
}

/// Wraps a base [`IdentityProvider`] and consults a
/// [`SoulJournalReader`] on every `current()` call (WEFT-96 / WS-D1).
///
/// Base `SOUL.md` / `IDENTITY.md` content is unchanged; pending
/// observations are attached to [`Identity::pending_journal`]. When
/// the journal is non-empty, [`Identity::source`] is
/// [`IdentitySource::Journal`]. Journal read failures degrade to the
/// base identity with a `warn!` by default so a transient substrate
/// outage never blocks chat; set [`Self::fail_closed`] to surface
/// [`IdentityError::Substrate`] / [`IdentityError::Io`] instead.
pub struct JournalAwareIdentityProvider {
    base: Arc<dyn IdentityProvider>,
    journal: Arc<dyn SoulJournalReader>,
    /// When `true`, journal read errors abort the turn via
    /// [`IdentityError`]; when `false` (default) they are logged and
    /// the base identity is returned unchanged.
    fail_closed: bool,
}

impl JournalAwareIdentityProvider {
    /// Degrade-on-journal-error (default production posture).
    pub fn new(base: Arc<dyn IdentityProvider>, journal: Arc<dyn SoulJournalReader>) -> Self {
        Self {
            base,
            journal,
            fail_closed: false,
        }
    }

    /// Fail the turn when the journal reader errors (strict mode).
    pub fn fail_closed(mut self, fail_closed: bool) -> Self {
        self.fail_closed = fail_closed;
        self
    }
}

#[async_trait]
impl IdentityProvider for JournalAwareIdentityProvider {
    async fn current(&self) -> Result<Identity, IdentityError> {
        let mut id = self.base.current().await?;
        match self.journal.list_pending().await {
            Ok(pending) => {
                if !pending.is_empty() {
                    debug!(
                        count = pending.len(),
                        "identity: journal substrate read-on-every-turn \
                         attached pending observations"
                    );
                    id.pending_journal = pending;
                    id.source = IdentitySource::Journal;
                }
                Ok(id)
            }
            Err(e) => {
                if self.fail_closed {
                    // Classify common substrate / IO phrases so callers
                    // can distinguish error kinds (WEFT-97 / WS-D5).
                    let lower = e.to_ascii_lowercase();
                    if lower.contains("deserial") || lower.contains("json") {
                        return Err(IdentityError::Deserialize(e));
                    }
                    if lower.contains("substrate")
                        || lower.contains("grant")
                        || lower.contains("mesh")
                    {
                        return Err(IdentityError::Substrate(e));
                    }
                    return Err(IdentityError::Io(e));
                }
                warn!(
                    error = %e,
                    "identity: journal read-on-every-turn failed; \
                     serving base identity without pending rows"
                );
                Ok(id)
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
        self.try_load_from(&self.workspace.join(".clawft"), IdentitySource::Clawft)
            .await
    }

    async fn try_load_from(&self, dir: &Path, source: IdentitySource) -> Option<Identity> {
        let soul_path = dir.join("SOUL.md");
        let identity_path = dir.join("IDENTITY.md");
        let soul = self.platform.fs().read_to_string(&soul_path).await.ok()?;
        let identity = self
            .platform
            .fs()
            .read_to_string(&identity_path)
            .await
            .ok()?;
        debug!(
            ?soul_path,
            ?identity_path,
            source = source.as_str(),
            "identity loaded"
        );
        Some(Identity::from_files(soul, identity, source))
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
        assert_eq!(id.source, IdentitySource::Clawft);
        assert!(id.pending_journal.is_empty());
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
        assert_eq!(id.source, IdentitySource::Clawft);
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
        assert_eq!(
            cached.source,
            IdentitySource::Cache,
            "cache fallback must tag IdentitySource::Cache (WEFT-97)"
        );
    }

    #[tokio::test]
    async fn file_provider_returns_not_found_with_no_cache() {
        let provider = FileIdentityProvider::new(PathBuf::from("/missing"), mock_platform());
        let err = provider.current().await.unwrap_err();
        assert!(matches!(err, IdentityError::NotFound));
    }

    // ── WEFT-96 / WEFT-97 ───────────────────────────────────────────

    #[test]
    fn identity_source_wire_labels_are_stable() {
        assert_eq!(IdentitySource::Clawft.as_str(), "clawft");
        assert_eq!(IdentitySource::Cache.as_str(), "cache");
        assert_eq!(IdentitySource::Substrate.as_str(), "substrate");
        assert_eq!(IdentitySource::Journal.as_str(), "journal");
        assert_eq!(IdentitySource::Stub.as_str(), "stub");
        assert_eq!(IdentitySource::Journal.to_string(), "journal");
    }

    #[test]
    fn identity_error_variants_display_distinct_messages() {
        let msgs = [
            IdentityError::NotFound.to_string(),
            IdentityError::BindingThreadMismatch.to_string(),
            IdentityError::Io("disk full".into()).to_string(),
            IdentityError::Deserialize("bad json".into()).to_string(),
            IdentityError::Substrate("grant denied".into()).to_string(),
        ];
        for m in &msgs {
            assert!(m.starts_with("identity load failed:"), "{m}");
        }
        assert!(msgs[2].contains("IO error"));
        assert!(msgs[3].contains("deserialize"));
        assert!(msgs[4].contains("substrate"));
    }

    struct StubBaseProvider(Identity);

    #[async_trait]
    impl IdentityProvider for StubBaseProvider {
        async fn current(&self) -> Result<Identity, IdentityError> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn journal_aware_provider_attaches_pending_on_every_turn() {
        use crate::agent::soul_journal::{
            DriftObservation, InMemorySoulJournal, SoulJournal, SoulJournalReader,
        };

        let base = Arc::new(StubBaseProvider(Identity::from_files(
            "soul-base",
            "id-base",
            IdentitySource::Clawft,
        )));
        let journal = Arc::new(InMemorySoulJournal::new());
        journal
            .append(DriftObservation::synthetic(
                "prefer terse replies",
                "user wants shorter answers going forward",
            ))
            .await
            .unwrap();

        // Reader is the same in-memory journal (write+read path).
        let reader: Arc<dyn SoulJournalReader> = journal.clone();
        let provider = JournalAwareIdentityProvider::new(base, reader);

        let id = provider.current().await.expect("load");
        assert_eq!(id.source, IdentitySource::Journal);
        assert_eq!(id.soul, "soul-base");
        assert_eq!(id.pending_journal.len(), 1);
        assert_eq!(id.pending_journal[0].summary, "prefer terse replies");
        // Hash stays base-only (journal is not auto-promoted into SOUL).
        assert_eq!(id.hash, sha256_identity_hash("soul-base", "id-base"));

        // Second turn re-reads journal (read-on-every-turn).
        journal
            .append(DriftObservation::synthetic("second", "another note"))
            .await
            .unwrap();
        let id2 = provider.current().await.expect("second turn");
        assert_eq!(id2.pending_journal.len(), 2);
        assert_eq!(id2.source, IdentitySource::Journal);
    }

    #[tokio::test]
    async fn journal_aware_provider_keeps_base_source_when_empty() {
        use crate::agent::soul_journal::{InMemorySoulJournal, SoulJournalReader};

        let base = Arc::new(StubBaseProvider(Identity::from_files(
            "s",
            "i",
            IdentitySource::Clawft,
        )));
        let reader: Arc<dyn SoulJournalReader> = Arc::new(InMemorySoulJournal::new());
        let provider = JournalAwareIdentityProvider::new(base, reader);
        let id = provider.current().await.unwrap();
        assert_eq!(id.source, IdentitySource::Clawft);
        assert!(id.pending_journal.is_empty());
    }

    struct FailingJournalReader;

    #[async_trait]
    impl crate::agent::soul_journal::SoulJournalReader for FailingJournalReader {
        async fn list_pending(
            &self,
        ) -> Result<Vec<crate::agent::soul_journal::PendingJournalEntry>, String> {
            Err("substrate grant denied for soul_journal".into())
        }
    }

    #[tokio::test]
    async fn journal_aware_provider_degrades_on_reader_error_by_default() {
        let base = Arc::new(StubBaseProvider(Identity::from_files(
            "s",
            "i",
            IdentitySource::Clawft,
        )));
        let provider =
            JournalAwareIdentityProvider::new(base, Arc::new(FailingJournalReader));
        let id = provider.current().await.expect("degrade open");
        assert_eq!(id.source, IdentitySource::Clawft);
        assert!(id.pending_journal.is_empty());
    }

    #[tokio::test]
    async fn journal_aware_provider_fail_closed_maps_substrate_error() {
        let base = Arc::new(StubBaseProvider(Identity::from_files(
            "s",
            "i",
            IdentitySource::Clawft,
        )));
        let provider = JournalAwareIdentityProvider::new(base, Arc::new(FailingJournalReader))
            .fail_closed(true);
        let err = provider.current().await.unwrap_err();
        assert!(
            matches!(err, IdentityError::Substrate(_)),
            "expected Substrate variant, got {err:?}"
        );
    }
}
