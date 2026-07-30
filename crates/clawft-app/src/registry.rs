//! Local app registry — JSON-backed for M1.5.
//!
//! ADR-015 §Install specifies a SQLite table; for M1.5 the ROADMAP
//! and session-10 §7 acceptance explicitly allow a JSON file as a
//! first cut (SQLite lands with migrations in M1.6+). The schema
//! matches ADR-015 §Install step 6 fields in spirit: manifest,
//! `installed_at`, enabled state. Manifest-hash / consent-id / cached
//! compiled-surfaces columns are left for the SQLite migration.
//!
//! Default path:
//!
//! * `$XDG_DATA_HOME/weftos/apps.json` if set
//! * otherwise `~/.weftos/apps.json`

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
// `std::time` panics under wasm32-unknown-unknown; `web-time` is a
// drop-in that uses std on native and `performance.now()` in the
// browser.
use web_time::{SystemTime, UNIX_EPOCH};

use crate::lifecycle::{
    LifecycleTeardownTombstone, TeardownTombstoneBus, TeardownReason,
};
use crate::manifest::AppManifest;
use crate::validation::{ValidationError, validate};

/// A row in the local app registry.
///
/// `Eq` is intentionally omitted — see [`AppManifest`] for the reason.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledApp {
    /// The full parsed manifest.
    pub manifest: AppManifest,
    /// Unix epoch seconds when `install()` was called.
    pub installed_at: u64,
    /// `false` means the app is registered but not launchable (e.g.
    /// missing-adapter dependency error per ADR-015 rule 10; owner
    /// turned it off manually).
    pub enabled: bool,
}

/// Result of a successful [`AppRegistry::uninstall`].
///
/// When the removed app was **enabled**, `teardown_tombstone` carries
/// the ADR-015 §Lifecycle teardown payload (surfaces / subscriptions /
/// influences) so the compositor can act once M1.6+ hooks land
/// (WEFT-412). Disabled uninstalls leave the field `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct UninstallResult {
    /// The registry row that was removed.
    pub removed: InstalledApp,
    /// Present when `removed.enabled` was `true` at uninstall time.
    pub teardown_tombstone: Option<LifecycleTeardownTombstone>,
}

/// The registry file's on-disk schema. Kept as a struct for forward
/// compat — adding fields is an additive change.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    apps: Vec<InstalledApp>,
}

/// JSON-backed persistent app registry.
///
/// Single-process, single-writer. Thread-safety is the caller's
/// problem (wrap in a `Mutex` if you need it). Every mutation calls
/// [`AppRegistry::save`] before returning, so a crash mid-call at
/// worst loses the most-recent op.
///
/// Lifecycle teardown tombstones (WEFT-412) are held on an in-process
/// [`TeardownTombstoneBus`] shared across clones of this registry.
/// The bus is **not** persisted to `apps.json`.
#[derive(Debug, Clone)]
pub struct AppRegistry {
    path: PathBuf,
    apps: Vec<InstalledApp>,
    /// In-process bus for uninstall-while-enabled tombstones.
    teardown_bus: TeardownTombstoneBus,
}

impl AppRegistry {
    /// Build a registry rooted at `path`. Does **not** read the file
    /// — call [`Self::load`] for that. (`new` is infallible so tests
    /// can construct registries against paths that don't exist yet.)
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            apps: Vec::new(),
            teardown_bus: TeardownTombstoneBus::new(),
        }
    }

    /// Resolve the default registry path:
    /// `$XDG_DATA_HOME/weftos/apps.json` or `~/.weftos/apps.json`.
    pub fn default_path() -> Result<PathBuf, RegistryError> {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            let p = PathBuf::from(xdg);
            if !p.as_os_str().is_empty() {
                return Ok(p.join("weftos").join("apps.json"));
            }
        }
        // `dirs` is native-only in this workspace; we fall back to
        // $HOME manually to stay minimal-deps on this new crate.
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home).join(".weftos").join("apps.json"));
        }
        Err(RegistryError::NoHomeDir)
    }

    /// Load the registry contents from disk. Missing file is treated
    /// as an empty registry (first-run is not an error).
    pub fn load(&mut self) -> Result<(), RegistryError> {
        match fs::read_to_string(&self.path) {
            Ok(text) => {
                let parsed: RegistryFile = serde_json::from_str(&text)?;
                self.apps = parsed.apps;
                Ok(())
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                self.apps.clear();
                Ok(())
            }
            Err(err) => Err(RegistryError::Io(err)),
        }
    }

    /// Persist to disk, creating parent directories as needed. Writes
    /// via a temp file + rename to keep the on-disk state coherent
    /// through process death.
    pub fn save(&self) -> Result<(), RegistryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = RegistryFile {
            apps: self.apps.clone(),
        };
        let body = serde_json::to_string_pretty(&file)?;
        let tmp = tmp_sibling(&self.path);
        fs::write(&tmp, body)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Install a manifest. Validates structurally (ADR-015 rules
    /// 1–9); rejects duplicates by `id`. Saves on success.
    pub fn install(&mut self, manifest: AppManifest) -> Result<&InstalledApp, RegistryError> {
        validate(&manifest)?;
        if self.apps.iter().any(|a| a.manifest.id == manifest.id) {
            return Err(RegistryError::AlreadyInstalled { id: manifest.id });
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.apps.push(InstalledApp {
            manifest,
            installed_at: now,
            enabled: true,
        });
        self.save()?;
        Ok(self.apps.last().expect("just-pushed app must exist"))
    }

    /// Remove the installed app with `id`.
    ///
    /// When the app was **enabled**, records a
    /// [`LifecycleTeardownTombstone`] on the registry's
    /// [`TeardownTombstoneBus`] (and returns it on
    /// [`UninstallResult::teardown_tombstone`]) so the compositor can
    /// run ADR-015 §Lifecycle teardown when M1.6+ hooks land (WEFT-412).
    /// Disabled apps uninstall without a tombstone.
    pub fn uninstall(&mut self, id: &str) -> Result<UninstallResult, RegistryError> {
        let idx = self
            .apps
            .iter()
            .position(|a| a.manifest.id == id)
            .ok_or_else(|| RegistryError::NotFound { id: id.to_string() })?;
        let removed = self.apps.remove(idx);
        let teardown_tombstone = if removed.enabled {
            let tombstone =
                LifecycleTeardownTombstone::uninstall_while_enabled(&removed.manifest);
            debug_assert_eq!(
                tombstone.reason,
                TeardownReason::UninstallWhileEnabled
            );
            self.teardown_bus.emit(tombstone.clone());
            Some(tombstone)
        } else {
            None
        };
        self.save()?;
        Ok(UninstallResult {
            removed,
            teardown_tombstone,
        })
    }

    /// Borrow the lifecycle teardown tombstone bus (WEFT-412).
    ///
    /// Compositor consumers call [`TeardownTombstoneBus::subscribe`]
    /// to receive future uninstall-while-enabled events, or
    /// [`TeardownTombstoneBus::recorded`] / [`TeardownTombstoneBus::drain`]
    /// for the in-memory log.
    pub fn teardown_bus(&self) -> &TeardownTombstoneBus {
        &self.teardown_bus
    }

    /// List all installed apps in install order.
    pub fn list(&self) -> &[InstalledApp] {
        &self.apps
    }

    /// Get one installed app by id.
    pub fn get(&self, id: &str) -> Option<&InstalledApp> {
        self.apps.iter().find(|a| a.manifest.id == id)
    }

    /// Enable an app (makes it launchable).
    pub fn enable(&mut self, id: &str) -> Result<(), RegistryError> {
        self.set_enabled(id, true)
    }

    /// Disable an app without uninstalling it.
    pub fn disable(&mut self, id: &str) -> Result<(), RegistryError> {
        self.set_enabled(id, false)
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), RegistryError> {
        let app = self
            .apps
            .iter_mut()
            .find(|a| a.manifest.id == id)
            .ok_or_else(|| RegistryError::NotFound { id: id.to_string() })?;
        app.enabled = enabled;
        self.save()?;
        Ok(())
    }

    /// Borrow the backing-file path (useful for diagnostics/tests).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn tmp_sibling(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

/// Errors surfaced by [`AppRegistry`] operations.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("app `{id}` is already installed")]
    AlreadyInstalled { id: String },
    #[error("app `{id}` is not installed")]
    NotFound { id: String },
    #[error("manifest failed validation: {0}")]
    Invalid(#[from] ValidationError),
    #[error("could not resolve a home directory for the default registry path")]
    NoHomeDir,
    #[error("registry I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("registry JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use semver::Version;
    use tempfile::tempdir;

    use super::*;
    use crate::lifecycle::TeardownReason;
    use crate::manifest::{AppManifest, EntryPoint, Input, Mode, SurfaceRef};

    fn mk_manifest(id: &str) -> AppManifest {
        AppManifest {
            id: id.to_string(),
            name: "Test App".to_string(),
            version: Version::new(0, 1, 0),
            icon: None,
            supported_modes: vec![Mode::Desktop],
            supported_inputs: vec![Input::Pointer],
            entry_points: vec![EntryPoint::Cli {
                flag: "test".to_string(),
            }],
            surfaces: vec![SurfaceRef::from("surfaces/main.toml")],
            surface_states: None,
            subscriptions: vec![],
            influences: vec![],
            permissions: vec![],
            narration: None,
        }
    }

    #[test]
    fn install_list_get_uninstall_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("apps.json");

        let mut reg = AppRegistry::new(&path);
        reg.load().expect("load empty is ok");
        assert!(reg.list().is_empty());

        let m = mk_manifest("app://weftos.admin");
        reg.install(m.clone()).expect("install");
        assert_eq!(reg.list().len(), 1);
        let got = reg.get("app://weftos.admin").expect("get");
        assert_eq!(got.manifest, m);
        assert!(got.enabled);

        // Persistence: load into a fresh registry from the same path.
        let mut reg2 = AppRegistry::new(&path);
        reg2.load().expect("load from disk");
        assert_eq!(reg2.list().len(), 1);
        assert_eq!(
            reg2.get("app://weftos.admin").map(|a| &a.manifest),
            Some(&m)
        );

        // Uninstall returns the row and removes it (enabled → tombstone).
        let result = reg2.uninstall("app://weftos.admin").expect("uninstall");
        assert_eq!(result.removed.manifest, m);
        assert!(result.teardown_tombstone.is_some());
        assert!(reg2.list().is_empty());

        // And persists again.
        let mut reg3 = AppRegistry::new(&path);
        reg3.load().expect("load after uninstall");
        assert!(reg3.list().is_empty());
    }

    #[test]
    fn install_rejects_duplicate_id() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();
        reg.install(mk_manifest("app://dup")).unwrap();
        let err = reg.install(mk_manifest("app://dup")).unwrap_err();
        assert!(matches!(err, RegistryError::AlreadyInstalled { .. }));
    }

    #[test]
    fn install_rejects_invalid_manifest() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();
        let mut bad = mk_manifest("app://bad");
        bad.supported_modes.clear();
        let err = reg.install(bad).unwrap_err();
        assert!(matches!(err, RegistryError::Invalid(_)));
    }

    #[test]
    fn enable_disable_toggle_persists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("apps.json");
        let mut reg = AppRegistry::new(&path);
        reg.load().unwrap();
        reg.install(mk_manifest("app://toggle")).unwrap();
        reg.disable("app://toggle").unwrap();
        assert!(!reg.get("app://toggle").unwrap().enabled);

        let mut reg2 = AppRegistry::new(&path);
        reg2.load().unwrap();
        assert!(!reg2.get("app://toggle").unwrap().enabled);

        reg2.enable("app://toggle").unwrap();
        assert!(reg2.get("app://toggle").unwrap().enabled);
    }

    #[test]
    fn uninstall_missing_returns_not_found() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();
        let err = reg.uninstall("app://nope").unwrap_err();
        assert!(matches!(err, RegistryError::NotFound { .. }));
    }

    /// WEFT-412: uninstall while enabled records a lifecycle teardown
    /// tombstone and delivers it to bus subscribers.
    #[test]
    fn uninstall_while_enabled_emits_teardown_tombstone() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();

        let mut m = mk_manifest("app://enabled.teardown");
        m.surfaces = vec![
            SurfaceRef::from("surfaces/main.toml"),
            SurfaceRef::from("surfaces/tray.toml"),
        ];
        m.subscriptions = vec![
            "substrate/kernel/status".to_string(),
            "substrate/sensor/mic".to_string(),
        ];
        m.influences = vec!["kernel.restart-service".to_string()];
        reg.install(m.clone()).unwrap();
        assert!(reg.get("app://enabled.teardown").unwrap().enabled);

        let rx = reg.teardown_bus().subscribe();
        assert!(reg.teardown_bus().is_empty());

        let result = reg
            .uninstall("app://enabled.teardown")
            .expect("uninstall enabled");

        assert_eq!(result.removed.manifest.id, m.id);
        let tomb = result
            .teardown_tombstone
            .expect("enabled uninstall must yield tombstone");
        assert_eq!(tomb.app_id, "app://enabled.teardown");
        assert_eq!(tomb.app_name, "Test App");
        assert_eq!(
            tomb.surface_ids,
            vec![
                "surfaces/main.toml".to_string(),
                "surfaces/tray.toml".to_string()
            ]
        );
        assert_eq!(
            tomb.subscription_topics,
            vec![
                "substrate/kernel/status".to_string(),
                "substrate/sensor/mic".to_string()
            ]
        );
        assert_eq!(tomb.influences, vec!["kernel.restart-service".to_string()]);
        assert_eq!(tomb.reason, TeardownReason::UninstallWhileEnabled);
        assert!(tomb.recorded_at > 0);

        // Bus log + live subscriber both see it.
        let recorded = reg.teardown_bus().recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].app_id, tomb.app_id);
        assert_eq!(recorded[0].reason, TeardownReason::UninstallWhileEnabled);

        let via_sub = rx.try_recv().expect("subscriber must receive tombstone");
        assert_eq!(via_sub.app_id, tomb.app_id);
        assert_eq!(via_sub.surface_ids, tomb.surface_ids);
        assert_eq!(via_sub.subscription_topics, tomb.subscription_topics);
        assert_eq!(via_sub.influences, tomb.influences);
    }

    /// WEFT-412: uninstall of a disabled app does not emit a tombstone
    /// (no live surfaces / subscriptions to tear down).
    #[test]
    fn uninstall_while_disabled_skips_teardown_tombstone() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();
        reg.install(mk_manifest("app://disabled.teardown")).unwrap();
        reg.disable("app://disabled.teardown").unwrap();

        let rx = reg.teardown_bus().subscribe();
        let result = reg
            .uninstall("app://disabled.teardown")
            .expect("uninstall disabled");

        assert!(!result.removed.enabled);
        assert!(result.teardown_tombstone.is_none());
        assert!(reg.teardown_bus().is_empty());
        assert!(rx.try_recv().is_err());
    }
}
