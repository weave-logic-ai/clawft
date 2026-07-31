//! Install pipeline — ADR-015 §Install (M1.5+).
//!
//! Hosts seed and install apps through this module instead of parsing a
//! fixture and calling [`AppRegistry::install`] ad hoc. That ad-hoc path
//! was the M1.5.1α Desktop boot workaround: it stamped `installed_at`
//! via wall-clock (`web-time` on wasm) during first paint, which is what
//! WEFT-440 retires.
//!
//! Out-of-box (OOB) system apps use a fixed [`OOB_INSTALLED_AT`] stamp so
//! first-boot seeding never needs a live clock. User-facing installs
//! still go through [`AppRegistry::install`] (wall-clock timestamp).

use crate::manifest::{AppManifest, ManifestParseError};
use crate::registry::{AppRegistry, InstalledApp, RegistryError};

/// Fixed `installed_at` for out-of-box system apps.
///
/// Not a wall-clock instant — it marks the row as *seeded by the install
/// pipeline*, not user-installed at a particular moment. Keeps OOB
/// ensure paths free of `SystemTime` / `web-time` (WEFT-440).
pub const OOB_INSTALLED_AT: u64 = 0;

/// How an [`ensure_manifest`] / [`ensure_from_toml`] call resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureKind {
    /// The app id was already present; registry was not mutated.
    AlreadyInstalled,
    /// The app was newly registered (in-memory at minimum).
    NewlyInstalled,
}

/// Result of an idempotent ensure step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureOutcome {
    pub kind: EnsureKind,
    /// `app://...` id that is now present in the registry.
    pub app_id: String,
    /// `true` when the registry file was written successfully.
    /// `false` when the row exists only in memory (e.g. wasm without a
    /// writable FS, read-only path) — the Apps tab still has content.
    pub persisted: bool,
}

impl EnsureOutcome {
    /// Convenience: was this a first-time seed?
    pub fn is_new(&self) -> bool {
        self.kind == EnsureKind::NewlyInstalled
    }
}

/// Errors from the install pipeline (parse or registry).
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("manifest parse failed: {0}")]
    Parse(#[from] ManifestParseError),
    #[error("registry error: {0}")]
    Registry(#[from] RegistryError),
}

/// Parse a TOML manifest string, validate, and install (non-idempotent).
///
/// Fails with [`RegistryError::AlreadyInstalled`] if the id is present.
/// Stamps `installed_at` with the wall clock (user install path).
pub fn install_from_toml<'a>(
    registry: &'a mut AppRegistry,
    toml: &str,
) -> Result<&'a InstalledApp, InstallError> {
    let manifest = AppManifest::from_toml_str(toml)?;
    Ok(registry.install(manifest)?)
}

/// Idempotent install of an already-parsed manifest.
///
/// * If the id is already registered → [`EnsureKind::AlreadyInstalled`].
/// * Otherwise installs with wall-clock `installed_at`.
/// * If `save()` fails after the in-memory push, still returns
///   [`EnsureKind::NewlyInstalled`] with `persisted: false` so hosts
///   (Desktop on wasm) keep an Apps-tab entry without panicking.
pub fn ensure_manifest(
    registry: &mut AppRegistry,
    manifest: AppManifest,
) -> Result<EnsureOutcome, InstallError> {
    ensure_manifest_at(registry, manifest, None)
}

/// Idempotent install with an explicit `installed_at` stamp.
///
/// Pass `Some(secs)` to override the wall clock (OOB seeding uses
/// [`OOB_INSTALLED_AT`]). Pass `None` for a normal user install stamp.
pub fn ensure_manifest_at(
    registry: &mut AppRegistry,
    manifest: AppManifest,
    installed_at: Option<u64>,
) -> Result<EnsureOutcome, InstallError> {
    let id = manifest.id.clone();
    if registry.get(&id).is_some() {
        return Ok(EnsureOutcome {
            kind: EnsureKind::AlreadyInstalled,
            app_id: id,
            persisted: true,
        });
    }

    let stamp = installed_at.unwrap_or_else(crate::registry::now_unix_secs);
    match registry.install_at(manifest, stamp) {
        Ok(_) => Ok(EnsureOutcome {
            kind: EnsureKind::NewlyInstalled,
            app_id: id,
            persisted: true,
        }),
        Err(RegistryError::AlreadyInstalled { id }) => Ok(EnsureOutcome {
            kind: EnsureKind::AlreadyInstalled,
            app_id: id,
            persisted: true,
        }),
        Err(e) => {
            // install_at pushes before save — an Io error leaves the
            // in-memory row so hosts can still present the app (wasm /
            // RO fs). Report NewlyInstalled + persisted=false.
            if matches!(e, RegistryError::Io(_)) && registry.get(&id).is_some() {
                Ok(EnsureOutcome {
                    kind: EnsureKind::NewlyInstalled,
                    app_id: id,
                    persisted: false,
                })
            } else {
                Err(InstallError::Registry(e))
            }
        }
    }
}

/// Parse TOML then [`ensure_manifest`].
pub fn ensure_from_toml(
    registry: &mut AppRegistry,
    toml: &str,
) -> Result<EnsureOutcome, InstallError> {
    let manifest = AppManifest::from_toml_str(toml)?;
    ensure_manifest(registry, manifest)
}

/// Parse TOML then [`ensure_manifest_at`] with a fixed stamp.
pub fn ensure_from_toml_at(
    registry: &mut AppRegistry,
    toml: &str,
    installed_at: u64,
) -> Result<EnsureOutcome, InstallError> {
    let manifest = AppManifest::from_toml_str(toml)?;
    ensure_manifest_at(registry, manifest, Some(installed_at))
}

/// Out-of-box system apps shipped with the host.
///
/// These are seeded through the install pipeline on first boot so the
/// Apps tab always has content. Seeding uses [`OOB_INSTALLED_AT`] and
/// never touches the wall clock (WEFT-440).
pub mod oob {
    use super::*;

    /// Bundled WeftOS Admin manifest (ADR-015 reference app).
    pub const WEFTOS_ADMIN_TOML: &str = include_str!("../fixtures/weftos-admin.toml");

    /// Stable id for the admin reference app.
    pub const WEFTOS_ADMIN_ID: &str = "app://weftos.admin";

    /// Ensure WeftOS Admin is registered (idempotent, OOB stamp).
    ///
    /// Replaces the Desktop `Default` ad-hoc
    /// `from_toml_str` + `install` seed path.
    pub fn ensure_weftos_admin(
        registry: &mut AppRegistry,
    ) -> Result<EnsureOutcome, InstallError> {
        ensure_from_toml_at(registry, WEFTOS_ADMIN_TOML, OOB_INSTALLED_AT)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::manifest::{EntryPoint, Input, Mode, SurfaceRef};
    use semver::Version;

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
    fn ensure_manifest_installs_once_then_is_idempotent() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();

        let first = ensure_manifest(&mut reg, mk_manifest("app://once")).unwrap();
        assert_eq!(first.kind, EnsureKind::NewlyInstalled);
        assert!(first.persisted);
        assert_eq!(first.app_id, "app://once");
        assert_eq!(reg.list().len(), 1);

        let second = ensure_manifest(&mut reg, mk_manifest("app://once")).unwrap();
        assert_eq!(second.kind, EnsureKind::AlreadyInstalled);
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn ensure_from_toml_at_uses_oob_stamp_not_wall_clock() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();

        let outcome =
            ensure_from_toml_at(&mut reg, oob::WEFTOS_ADMIN_TOML, OOB_INSTALLED_AT).unwrap();
        assert_eq!(outcome.kind, EnsureKind::NewlyInstalled);
        assert_eq!(outcome.app_id, oob::WEFTOS_ADMIN_ID);

        let row = reg.get(oob::WEFTOS_ADMIN_ID).expect("admin installed");
        assert_eq!(
            row.installed_at, OOB_INSTALLED_AT,
            "OOB seed must not stamp wall-clock time"
        );
        assert!(row.enabled);
        assert_eq!(row.manifest.name, "WeftOS Admin");
    }

    #[test]
    fn oob_ensure_weftos_admin_is_idempotent() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();

        let a = oob::ensure_weftos_admin(&mut reg).unwrap();
        assert!(a.is_new());
        let b = oob::ensure_weftos_admin(&mut reg).unwrap();
        assert!(!b.is_new());
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.get(oob::WEFTOS_ADMIN_ID).unwrap().installed_at, 0);
    }

    #[test]
    fn install_from_toml_rejects_duplicate() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();

        install_from_toml(&mut reg, oob::WEFTOS_ADMIN_TOML).unwrap();
        let err = install_from_toml(&mut reg, oob::WEFTOS_ADMIN_TOML).unwrap_err();
        assert!(matches!(
            err,
            InstallError::Registry(RegistryError::AlreadyInstalled { .. })
        ));
    }

    #[test]
    fn ensure_from_toml_rejects_malformed() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();
        let err = ensure_from_toml(&mut reg, "this is not toml [[[").unwrap_err();
        assert!(matches!(err, InstallError::Parse(_)));
    }

    #[test]
    fn ensure_manifest_at_explicit_stamp() {
        let dir = tempdir().unwrap();
        let mut reg = AppRegistry::new(dir.path().join("apps.json"));
        reg.load().unwrap();

        ensure_manifest_at(&mut reg, mk_manifest("app://stamped"), Some(1_700_000_000)).unwrap();
        assert_eq!(
            reg.get("app://stamped").unwrap().installed_at,
            1_700_000_000
        );
    }
}
