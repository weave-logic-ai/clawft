//! Structural validation of [`AppManifest`] — ADR-015 §Validation
//! rules 1–9.
//!
//! A manifest that fails any of these rules is **malformed** and
//! never reaches the registry (mirrors ADR-006's structural-rejection
//! posture for primitive heads).
//!
//! Rule 6 (permission ↔ adapter consistency) uses the static
//! [`crate::adapter_catalog::AdapterCatalog`] until a live ADR-017
//! host registry / `clawft-adapter` crate replaces it (WEFT-413).
//!
//! Rule 10 (adapter dependency resolution) is deliberately out of
//! scope here — it's *environmental*, not structural; the host
//! flags missing adapters and installs the app disabled.

use crate::adapter_catalog::AdapterCatalog;
use crate::manifest::{AppManifest, EntryPoint, Input, Mode};

/// All structural failure modes ADR-015 §Validation can return.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ValidationError {
    /// ADR-015 rule 1 or 2 — `supported_modes` / `supported_inputs`
    /// is an empty set. (`field` is `"supported_modes"` or
    /// `"supported_inputs"`.)
    #[error("`{field}` must be a non-empty subset")]
    Empty { field: &'static str },

    /// ADR-015 rule 3 — `[narration]` entries declared but
    /// `supported_inputs` does not contain `voice`. Narration is a
    /// voice-mode concept; without voice it's incoherent.
    #[error("`[narration]` declared but `supported_inputs` does not include `voice`")]
    NarrationWithoutVoice,

    /// ADR-015 rule 4 — `supported_modes == ["single-app"]` but
    /// `surfaces.len() != 1`. Single-app is a locked kiosk: exactly
    /// one surface, no escape (Session 10 recommendation 2).
    #[error("single-app apps must declare exactly one surface (found {found})")]
    SingleAppSurfaceCount { found: usize },

    /// ADR-015 rule 5 — `influences` contains an `ide.*` verb but
    /// `ide` is not in `supported_modes`. ADR-018 only activates the
    /// IDE bridge in `ide` sessions.
    #[error("influence `{verb}` requires `ide` in supported_modes (ADR-018)")]
    IdeInfluenceWithoutIdeMode { verb: String },

    /// ADR-015 rule 6 — a `permissions` entry has no subscribed
    /// adapter that consumes that channel. Orphan capture / fs / net
    /// grants are either a privacy leak (channel live, app never
    /// reads) or a copy-paste error; either way, rejected.
    #[error(
        "permission `{permission}` has no subscribed adapter that consumes that channel (ADR-015 rule 6)"
    )]
    PermissionWithoutAdapterReader {
        /// Wire form of the orphan permission (`"mic"`, `"fs:/tmp"`, …).
        permission: String,
    },

    /// ADR-015 rule 7 — `wake-word` entry point declared but `voice`
    /// is not in `supported_inputs`.
    #[error("wake-word entry point declared but `voice` is not in supported_inputs")]
    WakeWordWithoutVoice,

    /// ADR-015 rule 8 — `version` is not valid semver. (Normally this
    /// is caught at TOML-parse time by the `semver` crate, but the
    /// rule is mirrored here so in-memory construction is also
    /// checked when manifests are built without going through TOML.)
    #[error("`version` must be valid semver (got {0})")]
    InvalidSemver(String),

    /// ADR-015 rule 9 — `id` is not a well-formed `app://` IRI. We
    /// require the `app://` scheme and a non-empty dotted path of
    /// ASCII identifier segments.
    #[error("`id` must be an `app://<dotted-path>` IRI (got `{0}`)")]
    InvalidIri(String),

    /// ADR-015 rule covering `[narration]` keys must appear in
    /// `subscriptions` (§Schema (8)).
    #[error("narration key `{key}` is not in `subscriptions`")]
    NarrationKeyNotSubscribed { key: String },

    /// ADR-015 rule 1 (value set) — a `supported_modes` entry used an
    /// unknown variant. Mirrors serde's own rejection and is here for
    /// callers constructing manifests in Rust who might bypass the
    /// deserialiser later.
    #[error("unknown mode variant `{0}`")]
    UnknownMode(String),
}

/// Validate a manifest against ADR-015 §Validation rules 1–9 using
/// the builtin adapter capability catalogue for rule 6.
///
/// Returns on the **first** failure — callers that want all errors
/// should iterate by re-running after fixing. This matches how TOML
/// parse failures surface too (and keeps the error type an enum
/// rather than a list).
pub fn validate(manifest: &AppManifest) -> Result<(), ValidationError> {
    validate_with_catalog(manifest, &AdapterCatalog::builtin())
}

/// Same as [`validate`], but rule 6 consults `catalog` instead of the
/// builtin set. Hosts with a live ADR-017 registry (and unit tests
/// that inject fixtures) pass a custom catalogue here.
pub fn validate_with_catalog(
    manifest: &AppManifest,
    catalog: &AdapterCatalog,
) -> Result<(), ValidationError> {
    // Rule 9 — IRI shape. Cheapest check, runs first so malformed
    // identity fails before anything else.
    if !is_well_formed_app_iri(&manifest.id) {
        return Err(ValidationError::InvalidIri(manifest.id.clone()));
    }

    // Rule 8 — semver. Because `AppManifest::version` is already a
    // parsed `semver::Version`, this is a structural no-op for
    // TOML-parsed manifests, but we keep the case here because in-
    // memory constructors could (in theory) hand us a pre-release
    // string via `Version::new`. `Version::new(_, _, _)` is always
    // valid, so today this branch is unreachable; it's documented as
    // a forward-compat hook.
    //
    // (We don't `return Err(...)` here — there's no reachable failure
    // for a parsed `Version`.)

    // Rule 1 — supported_modes non-empty.
    if manifest.supported_modes.is_empty() {
        return Err(ValidationError::Empty {
            field: "supported_modes",
        });
    }

    // Rule 2 — supported_inputs non-empty.
    if manifest.supported_inputs.is_empty() {
        return Err(ValidationError::Empty {
            field: "supported_inputs",
        });
    }

    let has_voice = manifest.supported_inputs.contains(&Input::Voice);
    let has_ide = manifest.supported_modes.contains(&Mode::Ide);

    // Rule 3 — narration requires voice.
    if let Some(narration) = manifest.narration.as_ref()
        && !narration.is_empty()
        && !has_voice
    {
        return Err(ValidationError::NarrationWithoutVoice);
    }

    // Rule 3 (addendum from §Schema (8)) — narration keys must be
    // subscribed topics.
    if let Some(narration) = manifest.narration.as_ref() {
        for key in narration.keys() {
            if !manifest.subscriptions.iter().any(|s| s == key) {
                return Err(ValidationError::NarrationKeyNotSubscribed { key: key.clone() });
            }
        }
    }

    // Rule 4 — single-app kiosk has exactly one surface.
    if manifest.supported_modes == [Mode::SingleApp] && manifest.surfaces.len() != 1 {
        return Err(ValidationError::SingleAppSurfaceCount {
            found: manifest.surfaces.len(),
        });
    }

    // Rule 5 — ide.* influences require `ide` mode.
    if !has_ide {
        for verb in &manifest.influences {
            if verb.starts_with("ide.") {
                return Err(ValidationError::IdeInfluenceWithoutIdeMode { verb: verb.clone() });
            }
        }
    }

    // Rule 6 — permission-requires-adapter-reader (WEFT-413).
    // Every declared permission must be consumed by at least one
    // adapter resolved from `subscriptions` via the capability
    // catalogue. See [`AdapterCatalog::permission_has_reader`].
    for perm in &manifest.permissions {
        if !catalog.permission_has_reader(perm, &manifest.subscriptions) {
            return Err(ValidationError::PermissionWithoutAdapterReader {
                permission: perm.to_token(),
            });
        }
    }

    // Rule 7 — wake-word entry point requires voice input.
    if !has_voice {
        for ep in &manifest.entry_points {
            if let EntryPoint::WakeWord { .. } = ep {
                return Err(ValidationError::WakeWordWithoutVoice);
            }
        }
    }

    Ok(())
}

/// `id` must match `app://<segment>(.<segment>)*` where each segment
/// is a non-empty run of `[A-Za-z0-9_-]`.
fn is_well_formed_app_iri(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("app://") else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    rest.split('.').all(|seg| {
        !seg.is_empty()
            && seg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use semver::Version;

    use super::*;
    use crate::manifest::{EntryPoint, Permission, SurfaceRef};

    fn baseline() -> AppManifest {
        AppManifest {
            id: "app://weftos.admin".to_string(),
            name: "WeftOS Admin".to_string(),
            version: Version::new(0, 1, 0),
            icon: None,
            supported_modes: vec![Mode::Desktop, Mode::Ide],
            supported_inputs: vec![Input::Pointer],
            entry_points: vec![EntryPoint::Cli {
                flag: "admin".to_string(),
            }],
            surfaces: vec![SurfaceRef::from("surfaces/admin-main.toml")],
            surface_states: None,
            subscriptions: vec!["substrate/kernel/status".to_string()],
            influences: vec!["kernel.restart-service".to_string()],
            permissions: vec![],
            narration: None,
        }
    }

    #[test]
    fn baseline_is_valid() {
        validate(&baseline()).expect("baseline must validate");
    }

    #[test]
    fn weftos_admin_fixture_is_valid() {
        let m = AppManifest::from_toml_str(include_str!("../fixtures/weftos-admin.toml"))
            .expect("fixture parses");
        validate(&m).expect("fixture must validate");
    }

    #[test]
    fn empty_supported_modes_rejected() {
        let mut m = baseline();
        m.supported_modes.clear();
        assert_eq!(
            validate(&m),
            Err(ValidationError::Empty {
                field: "supported_modes"
            })
        );
    }

    #[test]
    fn empty_supported_inputs_rejected() {
        let mut m = baseline();
        m.supported_inputs.clear();
        assert_eq!(
            validate(&m),
            Err(ValidationError::Empty {
                field: "supported_inputs"
            })
        );
    }

    #[test]
    fn narration_without_voice_rejected() {
        let mut m = baseline();
        let mut narration = BTreeMap::new();
        narration.insert("substrate/kernel/status".to_string(), "ok".to_string());
        m.narration = Some(narration);
        // supported_inputs is [Pointer] — no voice.
        assert_eq!(validate(&m), Err(ValidationError::NarrationWithoutVoice));
    }

    #[test]
    fn narration_key_must_be_subscribed() {
        let mut m = baseline();
        m.supported_inputs.push(Input::Voice);
        let mut narration = BTreeMap::new();
        narration.insert(
            "substrate/unsubscribed/topic".to_string(),
            "tmpl".to_string(),
        );
        m.narration = Some(narration);
        assert!(matches!(
            validate(&m),
            Err(ValidationError::NarrationKeyNotSubscribed { .. })
        ));
    }

    #[test]
    fn single_app_requires_exactly_one_surface() {
        let mut m = baseline();
        m.supported_modes = vec![Mode::SingleApp];
        m.surfaces = vec![SurfaceRef::from("a.toml"), SurfaceRef::from("b.toml")];
        assert_eq!(
            validate(&m),
            Err(ValidationError::SingleAppSurfaceCount { found: 2 })
        );

        m.surfaces.clear();
        assert_eq!(
            validate(&m),
            Err(ValidationError::SingleAppSurfaceCount { found: 0 })
        );
    }

    #[test]
    fn ide_influence_without_ide_mode_rejected() {
        let mut m = baseline();
        m.supported_modes = vec![Mode::Desktop];
        m.influences.push("ide.open-file".to_string());
        assert!(matches!(
            validate(&m),
            Err(ValidationError::IdeInfluenceWithoutIdeMode { .. })
        ));
    }

    #[test]
    fn wake_word_without_voice_rejected() {
        let mut m = baseline();
        m.entry_points.push(EntryPoint::WakeWord {
            phrase: "weft, hi".to_string(),
        });
        // supported_inputs is [Pointer].
        assert_eq!(validate(&m), Err(ValidationError::WakeWordWithoutVoice));
    }

    #[test]
    fn invalid_iri_rejected() {
        for bad in [
            "weftos.admin",
            "app:weftos.admin",
            "app://",
            "app://..foo",
            "app://bad segment",
        ] {
            let mut m = baseline();
            m.id = bad.to_string();
            assert!(
                matches!(validate(&m), Err(ValidationError::InvalidIri(_))),
                "expected InvalidIri for id=`{bad}`"
            );
        }
    }

    #[test]
    fn well_formed_iri_accepts_typical_ids() {
        for ok in [
            "app://weftos.admin",
            "app://weftos.admin.v2",
            "app://team-acme.my_app",
            "app://solo",
        ] {
            assert!(
                is_well_formed_app_iri(ok),
                "expected `{ok}` to be well-formed"
            );
        }
    }

    // ── ADR-015 rule 6 (WEFT-413) ────────────────────────────────────

    #[test]
    fn rule6_orphan_camera_rejected() {
        let mut m = baseline();
        m.permissions.push(Permission::Camera);
        // subscriptions are kernel-only — no camera reader.
        assert_eq!(
            validate(&m),
            Err(ValidationError::PermissionWithoutAdapterReader {
                permission: "camera".into(),
            })
        );
    }

    #[test]
    fn rule6_orphan_mic_rejected() {
        let mut m = baseline();
        m.permissions.push(Permission::Mic);
        assert!(matches!(
            validate(&m),
            Err(ValidationError::PermissionWithoutAdapterReader { .. })
        ));
    }

    #[test]
    fn rule6_mic_with_mic_subscription_accepted() {
        let mut m = baseline();
        m.permissions.push(Permission::Mic);
        m.subscriptions
            .push("substrate/sensor/mic".to_string());
        validate(&m).expect("mic + mic subscription must validate");
    }

    #[test]
    fn rule6_mic_with_node_scoped_subscription_accepted() {
        let mut m = baseline();
        m.permissions.push(Permission::Mic);
        m.subscriptions
            .push("substrate/host-local/sensor/mic/rms".to_string());
        validate(&m).expect("node-scoped mic topic must count as reader");
    }

    #[test]
    fn rule6_fs_without_fs_reader_rejected() {
        let mut m = baseline();
        m.permissions
            .push(Permission::FsPath("/var/log/weftos".into()));
        // kernel subscriptions do not consume fs.
        assert_eq!(
            validate(&m),
            Err(ValidationError::PermissionWithoutAdapterReader {
                permission: "fs:/var/log/weftos".into(),
            })
        );
    }

    #[test]
    fn rule6_fs_with_fs_subscription_accepted() {
        let mut m = baseline();
        m.permissions
            .push(Permission::FsPath("/var/log/weftos".into()));
        m.subscriptions.push("substrate/fs/watched".to_string());
        validate(&m).expect("fs adapter subscription covers any fs: path");
    }

    #[test]
    fn rule6_git_fs_mismatch_rejected() {
        let mut m = baseline();
        m.permissions
            .push(Permission::FsPath("/var/log/weftos".into()));
        m.subscriptions.push("substrate/git/status".to_string());
        assert!(matches!(
            validate(&m),
            Err(ValidationError::PermissionWithoutAdapterReader { .. })
        ));
    }

    #[test]
    fn rule6_git_fs_match_accepted() {
        let mut m = baseline();
        m.permissions
            .push(Permission::FsPath("/workspace/.git".into()));
        m.subscriptions.push("substrate/git/branch".to_string());
        validate(&m).expect("git adapter covers /workspace/.git");
    }

    #[test]
    fn rule6_net_with_gh_subscription_accepted() {
        let mut m = baseline();
        m.permissions
            .push(Permission::NetDomain("api.github.com".into()));
        m.subscriptions.push("substrate/gh/issues".to_string());
        validate(&m).expect("gh adapter covers api.github.com");
    }

    #[test]
    fn rule6_net_domain_mismatch_rejected() {
        let mut m = baseline();
        m.permissions
            .push(Permission::NetDomain("evil.example".into()));
        m.subscriptions.push("substrate/gh/issues".to_string());
        assert_eq!(
            validate(&m),
            Err(ValidationError::PermissionWithoutAdapterReader {
                permission: "net:evil.example".into(),
            })
        );
    }

    #[test]
    fn rule6_custom_catalog_injected() {
        use crate::adapter_catalog::{AdapterCapability, AdapterChannel, AdapterCatalog};

        let mut m = baseline();
        m.permissions.push(Permission::Screen);
        m.subscriptions.push("app/custom/screen".to_string());

        // Builtin: custom topic is unknown → reject.
        assert!(matches!(
            validate(&m),
            Err(ValidationError::PermissionWithoutAdapterReader { .. })
        ));

        let custom = AdapterCatalog::from_entries([AdapterCapability {
            id: "custom-screen",
            topic_prefixes: &["app/custom/screen"],
            path_contains: &[],
            channels: &[AdapterChannel::Screen],
        }]);
        validate_with_catalog(&m, &custom)
            .expect("custom catalogue must accept matching screen reader");
    }
}
