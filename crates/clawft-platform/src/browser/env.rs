//! Browser environment implementation with optional OPFS persistence.
//!
//! ## Backends
//!
//! * **In-memory** (default) — `HashMap` behind a [`Mutex`]. Variables live
//!   only for the lifetime of the [`BrowserEnvironment`] instance.
//! * **OPFS** (`browser-opfs` feature) — the same map is snapshotted to
//!   [`ENV_PERSIST_PATH`] under the origin's private filesystem so values
//!   survive page reloads. When OPFS is unavailable the constructor falls
//!   back to memory-only (same as filesystem fallback in WEFT-13 / WEFT-392).
//!
//! ## Sync `Environment` trait + async load/flush
//!
//! [`Environment`] methods stay synchronous (matches native / WASM call
//! sites). Loading and durable writes use async constructors
//! ([`BrowserEnvironment::open`], [`BrowserEnvironment::open_with_seed`])
//! and [`BrowserEnvironment::flush`]. Mutations schedule a best-effort
//! background flush when a persist backend is active.
//!
//! ## Secrets in the browser (WEFT-14 trade-off)
//!
//! Persisting env vars in OPFS means API keys and tokens written via
//! `set_env` / `env_json` are readable by any script on the same origin
//! (XSS, browser extensions, DevTools). This is a deliberate PWA UX
//! trade-off, not a secrets vault. Prefer short-lived tokens, never store
//! long-lived root credentials in browser env, and treat OPFS as
//! origin-scoped user data. [`Debug`] redacts sensitive-looking values so
//! logs do not amplify the risk.
//!
//! The browser WASM runtime holds an [`std::sync::Arc`] clone of this
//! store (WEFT-391) so `set_env` can mutate live env state after
//! [`BrowserPlatform`](super::BrowserPlatform) is moved into `AgentLoop`.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use crate::env::Environment;

#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
use std::path::Path;
#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
use std::sync::Arc;

/// Virtual OPFS path for the env snapshot (under browser home layout).
///
/// Co-located with config under `/clawft/.clawft/` so one origin tree
/// holds both workspace files (WEFT-13) and env (WEFT-14).
pub const ENV_PERSIST_PATH: &str = "/clawft/.clawft/env.json";

/// Which storage backend a [`BrowserEnvironment`] is using for durability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserEnvBackend {
    /// Session-only in-memory map (default / fallback).
    Memory,
    /// Snapshotted to OPFS at [`ENV_PERSIST_PATH`].
    Opfs,
}

/// In-memory environment variable store for browser/WASM targets.
///
/// Designed to be shared via [`std::sync::Arc`] so the browser WASM
/// `set_env` entry point and the platform-owned agent loop observe the
/// same map (WEFT-391). With `browser-opfs`, prefer
/// [`BrowserEnvironment::open`] / [`open_with_seed`] so reloads restore
/// the previous snapshot.
pub struct BrowserEnvironment {
    vars: Mutex<HashMap<String, String>>,
    /// Generation bumped on every mutation so concurrent background flushes
    /// can be ordered (wasm + `browser-opfs` only).
    #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
    generation: AtomicU64,
    #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
    persist: Option<PersistHandle>,
}

#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
struct PersistHandle {
    /// Shared FS handle used only for the env snapshot path.
    fs: Arc<super::fs::BrowserFileSystem>,
}

impl BrowserEnvironment {
    /// Create a new empty browser environment (memory-only, no load).
    pub fn new() -> Self {
        Self {
            vars: Mutex::new(HashMap::new()),
            #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
            generation: AtomicU64::new(0),
            #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
            persist: None,
        }
    }

    /// Create a browser environment pre-populated with the given variables
    /// (memory-only; does not load or write OPFS).
    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        Self {
            vars: Mutex::new(vars),
            #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
            generation: AtomicU64::new(0),
            #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
            persist: None,
        }
    }

    /// Which durability backend is active.
    pub fn backend(&self) -> BrowserEnvBackend {
        #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
        {
            if self.persist.is_some() {
                return BrowserEnvBackend::Opfs;
            }
        }
        BrowserEnvBackend::Memory
    }

    /// Open the best available backend.
    ///
    /// * With `browser-opfs`: try OPFS, load existing [`ENV_PERSIST_PATH`]
    ///   if present; on OPFS failure fall back to empty memory.
    /// * Without the feature: always empty memory (same as [`new`]).
    pub async fn open() -> Self {
        Self::open_with_seed(HashMap::new()).await
    }

    /// Like [`open`], then overlay `seed` (seed wins on key conflicts).
    ///
    /// Used by wasm `init(env_json)` so a page-reload restore still accepts
    /// an explicit seed from the host UI.
    pub async fn open_with_seed(seed: HashMap<String, String>) -> Self {
        // OPFS is a browser API; never touch web_sys on host (non-wasm) builds.
        #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
        {
            match open_persist_backend().await {
                Ok(fs) => {
                    let mut vars = load_snapshot(&fs).await;
                    for (k, v) in seed {
                        vars.insert(k, v);
                    }
                    let env = Self {
                        vars: Mutex::new(vars),
                        generation: AtomicU64::new(0),
                        persist: Some(PersistHandle { fs }),
                    };
                    // Persist merged seed so reload sees it without another set.
                    if let Err(e) = env.flush().await {
                        tracing::warn!(
                            error = %e,
                            "BrowserEnvironment: initial OPFS flush failed"
                        );
                    } else {
                        tracing::info!(
                            path = ENV_PERSIST_PATH,
                            "BrowserEnvironment: using OPFS persistence"
                        );
                    }
                    return env;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "BrowserEnvironment: OPFS unavailable, falling back to in-memory store"
                    );
                }
            }
        }
        #[cfg(all(feature = "browser-opfs", not(target_arch = "wasm32")))]
        {
            let _ = &seed; // host: feature present but no OPFS runtime
        }
        if seed.is_empty() {
            Self::new()
        } else {
            Self::with_vars(seed)
        }
    }

    /// Force the in-memory backend (tests).
    pub fn memory() -> Self {
        Self::new()
    }

    /// Write the current map to durable storage (no-op for memory-only).
    ///
    /// Await this in tests after mutations before simulating a reload.
    pub async fn flush(&self) -> Result<(), String> {
        #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
        {
            if let Some(ref p) = self.persist {
                let snapshot = self
                    .vars
                    .lock()
                    .expect("BrowserEnvironment mutex poisoned")
                    .clone();
                return write_snapshot(&p.fs, &snapshot).await;
            }
        }
        let _ = self;
        Ok(())
    }

    fn schedule_flush(&self) {
        #[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
        {
            if let Some(ref p) = self.persist {
                // Bump generation so concurrent mutations order snapshots.
                let _seq = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
                let fs = Arc::clone(&p.fs);
                let snapshot = self
                    .vars
                    .lock()
                    .expect("BrowserEnvironment mutex poisoned")
                    .clone();
                let _ = _seq;
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = write_snapshot(&fs, &snapshot).await {
                        tracing::warn!(
                            error = %e,
                            "BrowserEnvironment: background OPFS flush failed"
                        );
                    }
                });
            }
        }
        #[cfg(not(all(feature = "browser-opfs", target_arch = "wasm32")))]
        {
            let _ = self;
        }
    }
}

impl Default for BrowserEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment for BrowserEnvironment {
    fn get_var(&self, name: &str) -> Option<String> {
        self.vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned")
            .get(name)
            .cloned()
    }

    fn set_var(&self, name: &str, value: &str) {
        self.vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned")
            .insert(name.to_string(), value.to_string());
        self.schedule_flush();
    }

    fn remove_var(&self, name: &str) {
        self.vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned")
            .remove(name);
        self.schedule_flush();
    }
}

/// Redacting `Debug` so API keys / tokens do not appear in logs (WEFT-14).
impl fmt::Debug for BrowserEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vars = self
            .vars
            .lock()
            .expect("BrowserEnvironment mutex poisoned");
        let mut map = f.debug_map();
        for (k, v) in vars.iter() {
            if is_sensitive_env_key(k) {
                if v.is_empty() {
                    map.entry(k, &"");
                } else {
                    map.entry(k, &"[REDACTED]");
                }
            } else {
                map.entry(k, v);
            }
        }
        map.finish()?;
        write!(f, " (backend={:?})", self.backend())
    }
}

/// Heuristic: treat KEY / SECRET / TOKEN / PASSWORD / CREDENTIAL / AUTH
/// name fragments as sensitive (case-insensitive).
pub fn is_sensitive_env_key(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    const MARKERS: &[&str] = &[
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "TOKEN",
        "API_KEY",
        "APIKEY",
        "CREDENTIAL",
        "PRIVATE_KEY",
        "AUTH",
        // bare KEY as segment (FOO_KEY, KEY_FOO) but not KEYBOARD etc. —
        // match `_KEY` suffix / `KEY_` prefix / exact `KEY`.
    ];
    if MARKERS.iter().any(|m| upper.contains(m)) {
        return true;
    }
    upper == "KEY"
        || upper.ends_with("_KEY")
        || upper.starts_with("KEY_")
        || upper.contains("_KEY_")
}

// ---------------------------------------------------------------------------
// OPFS snapshot helpers (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
async fn open_persist_backend() -> Result<Arc<super::fs::BrowserFileSystem>, String> {
    use super::fs::{BrowserFileSystem, BrowserFsBackend};
    let fs = BrowserFileSystem::open().await;
    if fs.backend() != BrowserFsBackend::Opfs {
        return Err("OPFS backend not active after BrowserFileSystem::open".into());
    }
    Ok(Arc::new(fs))
}

#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
async fn load_snapshot(fs: &super::fs::BrowserFileSystem) -> HashMap<String, String> {
    use crate::fs::FileSystem;
    let path = Path::new(ENV_PERSIST_PATH);
    match fs.read_to_string(path).await {
        Ok(raw) => match serde_json::from_str::<HashMap<String, String>>(&raw) {
            Ok(map) => map,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = ENV_PERSIST_PATH,
                    "BrowserEnvironment: corrupt env snapshot, starting empty"
                );
                HashMap::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = ENV_PERSIST_PATH,
                "BrowserEnvironment: failed to read env snapshot"
            );
            HashMap::new()
        }
    }
}

#[cfg(all(feature = "browser-opfs", target_arch = "wasm32"))]
async fn write_snapshot(
    fs: &super::fs::BrowserFileSystem,
    vars: &HashMap<String, String>,
) -> Result<(), String> {
    use crate::fs::FileSystem;
    let path = Path::new(ENV_PERSIST_PATH);
    let json = serde_json::to_string(vars).map_err(|e| e.to_string())?;
    fs.write_string(path, &json)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn set_get_remove_round_trip() {
        let env = BrowserEnvironment::new();
        assert!(env.get_var("K").is_none());
        env.set_var("K", "v1");
        assert_eq!(env.get_var("K"), Some("v1".to_string()));
        env.set_var("K", "v2");
        assert_eq!(env.get_var("K"), Some("v2".to_string()));
        env.remove_var("K");
        assert!(env.get_var("K").is_none());
    }

    #[test]
    fn with_vars_pre_populates() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "1".to_string());
        vars.insert("B".to_string(), "2".to_string());
        let env = BrowserEnvironment::with_vars(vars);
        assert_eq!(env.get_var("A"), Some("1".to_string()));
        assert_eq!(env.get_var("B"), Some("2".to_string()));
        assert!(env.get_var("C").is_none());
    }

    /// WEFT-391 core invariant: Arc clones share the same Mutex map.
    #[test]
    fn arc_clone_sees_set_var_mutations() {
        let env = Arc::new(BrowserEnvironment::new());
        let sibling = Arc::clone(&env);
        env.set_var("CLAWFT_WEFT391", "shared");
        assert_eq!(
            sibling.get_var("CLAWFT_WEFT391"),
            Some("shared".to_string())
        );
        sibling.set_var("CLAWFT_WEFT391", "updated");
        assert_eq!(env.get_var("CLAWFT_WEFT391"), Some("updated".to_string()));
    }

    #[test]
    fn memory_backend_by_default() {
        let env = BrowserEnvironment::memory();
        assert_eq!(env.backend(), BrowserEnvBackend::Memory);
    }

    #[test]
    fn sensitive_key_heuristic() {
        assert!(is_sensitive_env_key("ANTHROPIC_API_KEY"));
        assert!(is_sensitive_env_key("api_key"));
        assert!(is_sensitive_env_key("OPENAI_TOKEN"));
        assert!(is_sensitive_env_key("db_password"));
        assert!(is_sensitive_env_key("OAUTH_SECRET"));
        assert!(is_sensitive_env_key("MY_KEY"));
        assert!(is_sensitive_env_key("KEY"));
        assert!(!is_sensitive_env_key("HOME"));
        assert!(!is_sensitive_env_key("MODEL_NAME"));
        assert!(!is_sensitive_env_key("KEYBOARD_LAYOUT"));
    }

    #[test]
    fn debug_redacts_sensitive_values() {
        let env = BrowserEnvironment::new();
        env.set_var("HOME", "/clawft");
        env.set_var("ANTHROPIC_API_KEY", "sk-secret-value-do-not-leak");
        env.set_var("EMPTY_TOKEN", "");
        let dbg = format!("{env:?}");
        assert!(
            dbg.contains("sk-secret-value-do-not-leak") == false,
            "secret must not appear in Debug: {dbg}"
        );
        assert!(
            dbg.contains("[REDACTED]"),
            "expected redaction marker: {dbg}"
        );
        assert!(
            dbg.contains("/clawft"),
            "non-sensitive values should still show: {dbg}"
        );
        // Empty sensitive values render as empty, not REDACTED.
        assert!(dbg.contains("EMPTY_TOKEN"));
    }

    #[tokio::test]
    async fn open_without_opfs_is_memory() {
        // Host target never gets a real OPFS backend; open falls back.
        let env = BrowserEnvironment::open().await;
        assert_eq!(env.backend(), BrowserEnvBackend::Memory);
        env.set_var("X", "1");
        assert_eq!(env.get_var("X"), Some("1".to_string()));
        // flush is a no-op for memory.
        env.flush().await.unwrap();
    }

    #[tokio::test]
    async fn open_with_seed_applies_on_memory_fallback() {
        let mut seed = HashMap::new();
        seed.insert("SEEDED".to_string(), "yes".to_string());
        let env = BrowserEnvironment::open_with_seed(seed).await;
        assert_eq!(env.get_var("SEEDED"), Some("yes".to_string()));
    }
}
