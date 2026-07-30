//! App lifecycle types — the data shapes the desktop compositor and
//! sibling crates consume when launching an app.
//!
//! Implementation status:
//!
//! * [`SessionConfig`], [`AppLaunchRequest`], [`AppLaunchResult`],
//!   [`LaunchError`] — stable for M1.5.
//! * [`governance::Gate`] — ADR-012-backed. [`governance::NoopGate`] is
//!   always-grant (tests / dev). [`governance::StrictGate`] /
//!   [`governance::CapturePrivacyGate`] enforce capture-channel
//!   privacy (ADR-012 invariant 2) for both app launch and ontology
//!   adapter open (wired through
//!   `clawft_substrate::Substrate::subscribe_adapter`).
//!
//! None of this actually *launches* anything — that's the
//! `clawft-gui-egui::shell` compositor's job. The types here are what
//! gets handed to it.

use serde::{Deserialize, Serialize};

use crate::manifest::{AppManifest, Input, Mode};

/// Session-level config: the `(mode, input)` pair a caller wants.
///
/// Fixed at startup per Session 10 §2. The compositor intersects this
/// with the manifest's `supported_modes` / `supported_inputs`; a
/// mismatch is a hard refusal (no best-effort downgrade, ADR-015
/// §Launch step 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionConfig {
    pub mode: Mode,
    pub input: Input,
}

/// A request to launch an installed app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppLaunchRequest {
    /// `app://...` IRI, resolved through the registry.
    pub app_id: String,
    pub session: SessionConfig,
}

/// The verdict from a [`governance::Gate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AppLaunchResult {
    /// Launch approved; caller may instantiate these surfaces.
    Granted {
        /// The surface ids the compositor should instantiate. For
        /// `single-app` sessions this is always one element; for
        /// desktop / ide sessions it's the manifest's full list.
        surface_ids: Vec<String>,
    },
    /// Launch refused with a human-readable reason.
    Denied { reason: String },
}

/// Hard failures before governance gets a say. These are structural —
/// the manifest is known but doesn't admit this launch at all.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LaunchError {
    #[error("no installed app with id `{0}`")]
    UnknownApp(String),
    #[error("app `{app_id}` does not declare mode `{mode:?}` in supported_modes")]
    ModeUnsupported { app_id: String, mode: Mode },
    #[error("app `{app_id}` does not declare input `{input:?}` in supported_inputs")]
    InputUnsupported { app_id: String, input: Input },
    #[error("governance denied launch of `{app_id}`: {reason}")]
    GovernanceDenied { app_id: String, reason: String },
}

/// Pluggable governance for launch-time and adapter-open authorisation.
///
/// ADR-012 (Capture Privacy Invariants) gates capture channels
/// (`camera` / `mic` / `screen`) at grant time — not per-tick. A
/// caller must hold an explicit [`Permission`] grant (session /
/// per-goal `CapabilityGrant`) before:
///
/// * launching an app whose manifest declares capture permissions, or
/// * opening an ontology adapter topic that requires capture
///   (`Substrate::subscribe_adapter` → [`Gate::authorize_adapter_open`]).
///
/// Non-capture permissions (`fs:*`, `net:*`) are granted when the
/// requested permission is covered by an equal grant entry.
///
/// Per-goal consent expiry (ADR-008) and tray-chip composition
/// (ADR-012 invariant 3) remain compositor / kernel concerns; this
/// gate is the structural deny-closed checkpoint those layers call.
pub mod governance {
    use crate::manifest::{AppManifest, Permission};

    use super::{AppLaunchRequest, AppLaunchResult};

    /// Verdict from [`Gate::authorize_adapter_open`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum AdapterOpenResult {
        /// Adapter may open the topic.
        Granted,
        /// Adapter open refused — `missing` lists the permissions the
        /// caller still needs; `reason` is human-readable for health
        /// events / logs.
        Denied {
            /// Permissions that were required but not covered by grants.
            missing: Vec<Permission>,
            /// Human-readable denial reason (ADR-012 wording).
            reason: String,
        },
    }

    impl AdapterOpenResult {
        /// `true` when the open is allowed.
        pub fn is_granted(&self) -> bool {
            matches!(self, Self::Granted)
        }

        /// `true` when the open is refused.
        pub fn is_denied(&self) -> bool {
            matches!(self, Self::Denied { .. })
        }
    }

    /// Runtime authorisation for app launch and ontology adapter open.
    pub trait Gate: std::fmt::Debug + Send + Sync {
        /// Return [`AppLaunchResult::Granted`] or
        /// [`AppLaunchResult::Denied`]. Implementations may consult
        /// the manifest and request together — *structural* launch
        /// constraints (mode / input intersection) are already
        /// validated by the caller before this is invoked.
        fn authorize_launch(
            &self,
            req: &AppLaunchRequest,
            manifest: &AppManifest,
        ) -> AppLaunchResult;

        /// Authorize opening an ontology adapter topic (ADR-017 + ADR-012).
        ///
        /// * `adapter_id` — stable adapter id (`"mic"`, `"kernel"`, …).
        /// * `topic` — absolute topic path being opened.
        /// * `required` — permissions the adapter declares for this
        ///   open (from [`clawft_app::manifest::Permission`] / adapter
        ///   `permissions()`). Capture-sensitive topics with an empty
        ///   `required` list still need a capture grant — the substrate
        ///   injects an inferred capture permission before calling.
        /// * `granted` — capability grants held by the caller for this
        ///   session / goal.
        ///
        /// Default behaviour (via blanket expectations on implementors):
        /// deny-closed for any required permission not covered by
        /// `granted`. Capture channels always require an exact match
        /// in `granted` (ADR-012 invariant 2 — no standing ambient
        /// capture without a grant).
        fn authorize_adapter_open(
            &self,
            adapter_id: &str,
            topic: &str,
            required: &[Permission],
            granted: &[Permission],
        ) -> AdapterOpenResult;
    }

    /// Always grants; useful for tests and open-dev loops.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct NoopGate;

    impl Gate for NoopGate {
        fn authorize_launch(
            &self,
            _req: &AppLaunchRequest,
            manifest: &AppManifest,
        ) -> AppLaunchResult {
            AppLaunchResult::Granted {
                surface_ids: manifest
                    .surfaces
                    .iter()
                    .map(|s| s.as_str().to_string())
                    .collect(),
            }
        }

        fn authorize_adapter_open(
            &self,
            _adapter_id: &str,
            _topic: &str,
            _required: &[Permission],
            _granted: &[Permission],
        ) -> AdapterOpenResult {
            AdapterOpenResult::Granted
        }
    }

    /// ADR-012 capture-privacy gate.
    ///
    /// # Launch
    ///
    /// A manifest that declares any capture channel (`camera` / `mic` /
    /// `screen`) is denied at launch unless that channel is present in
    /// the grant set passed via the adapter-open path. For pure launch
    /// (no separate grant list yet), capture declarations are denied —
    /// the host must open capture through a granted adapter subscribe
    /// rather than standing launch-time ambient access.
    ///
    /// # Adapter open
    ///
    /// Every entry in `required` must be covered by `granted`:
    ///
    /// * capture channels — exact match required
    /// * `fs:<path>` — exact match, or a grant whose path is a prefix
    ///   of the required path
    /// * `net:<domain>` — exact domain match
    ///
    /// Empty `required` → granted (public / meta adapters).
    ///
    /// [`StrictGate`] is a type alias of this type kept for callers that
    /// adopted the M1.5 name.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CapturePrivacyGate;

    /// M1.5 name for [`CapturePrivacyGate`] (backward-compatible alias).
    pub type StrictGate = CapturePrivacyGate;

    impl Gate for CapturePrivacyGate {
        fn authorize_launch(
            &self,
            _req: &AppLaunchRequest,
            manifest: &AppManifest,
        ) -> AppLaunchResult {
            for perm in &manifest.permissions {
                if is_capture(perm) {
                    return AppLaunchResult::Denied {
                        reason: format!(
                            "capture channel `{}` requires ADR-012 \
                             CapabilityGrant (denied at launch; open \
                             via granted adapter subscribe)",
                            perm.to_token()
                        ),
                    };
                }
            }
            AppLaunchResult::Granted {
                surface_ids: manifest
                    .surfaces
                    .iter()
                    .map(|s| s.as_str().to_string())
                    .collect(),
            }
        }

        fn authorize_adapter_open(
            &self,
            adapter_id: &str,
            topic: &str,
            required: &[Permission],
            granted: &[Permission],
        ) -> AdapterOpenResult {
            if required.is_empty() {
                return AdapterOpenResult::Granted;
            }

            let mut missing = Vec::new();
            for req in required {
                if !permission_covered(req, granted) {
                    missing.push(req.clone());
                }
            }

            if missing.is_empty() {
                AdapterOpenResult::Granted
            } else {
                let tokens: Vec<String> = missing.iter().map(Permission::to_token).collect();
                AdapterOpenResult::Denied {
                    reason: format!(
                        "ADR-012: adapter `{adapter_id}` topic `{topic}` \
                         denied — missing grant(s): {}",
                        tokens.join(", ")
                    ),
                    missing,
                }
            }
        }
    }

    /// Whether `perm` is a capture channel (ADR-012 / ADR-015 grammar).
    pub fn is_capture(perm: &Permission) -> bool {
        matches!(
            perm,
            Permission::Camera | Permission::Mic | Permission::Screen
        )
    }

    /// Whether `required` is covered by any entry in `granted`.
    ///
    /// Capture and net require exact equality. Filesystem grants cover
    /// the required path when the grant path is a prefix (including
    /// equality).
    pub fn permission_covered(required: &Permission, granted: &[Permission]) -> bool {
        match required {
            Permission::Camera | Permission::Mic | Permission::Screen => {
                granted.iter().any(|g| g == required)
            }
            Permission::NetDomain(need) => granted.iter().any(|g| match g {
                Permission::NetDomain(have) => have == need,
                _ => false,
            }),
            Permission::FsPath(need) => granted.iter().any(|g| match g {
                Permission::FsPath(have) => {
                    need == have
                        || need.starts_with(&format!(
                            "{have}{}",
                            if have.ends_with('/') { "" } else { "/" }
                        ))
                }
                _ => false,
            }),
        }
    }

    /// Infer a capture permission from adapter id / topic path when an
    /// adapter marks a topic `Capture` but left `permissions()` empty.
    ///
    /// Used by the substrate so ADR-012 cannot be bypassed by omitting
    /// the declaration. Returns `None` when no capture channel can be
    /// inferred (caller should still deny Capture-sensitivity opens
    /// without a grant).
    pub fn infer_capture_permission(adapter_id: &str, topic: &str) -> Option<Permission> {
        let hay = format!("{adapter_id}/{topic}").to_ascii_lowercase();
        if hay.contains("mic") || hay.contains("audio") {
            Some(Permission::Mic)
        } else if hay.contains("camera") || hay.contains("video") {
            Some(Permission::Camera)
        } else if hay.contains("screen") || hay.contains("display") {
            Some(Permission::Screen)
        } else {
            None
        }
    }
}

/// Shape-check a launch request against a manifest. Callers compose
/// this with a [`governance::Gate`] to do the full authorisation
/// dance; it's factored out so the structural / policy concerns stay
/// separate.
pub fn check_launch_shape(
    req: &AppLaunchRequest,
    manifest: &AppManifest,
) -> Result<(), LaunchError> {
    if !manifest.supported_modes.contains(&req.session.mode) {
        return Err(LaunchError::ModeUnsupported {
            app_id: req.app_id.clone(),
            mode: req.session.mode,
        });
    }
    if !manifest.supported_inputs.contains(&req.session.input) {
        return Err(LaunchError::InputUnsupported {
            app_id: req.app_id.clone(),
            input: req.session.input,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::governance::{
        AdapterOpenResult, CapturePrivacyGate, Gate, NoopGate, StrictGate, infer_capture_permission,
        permission_covered,
    };
    use super::*;
    use crate::manifest::{EntryPoint, Permission, SurfaceRef};

    fn mk_manifest() -> AppManifest {
        AppManifest {
            id: "app://example.lifecycle".to_string(),
            name: "Lifecycle".to_string(),
            version: Version::new(0, 1, 0),
            icon: None,
            supported_modes: vec![Mode::Desktop, Mode::Ide],
            supported_inputs: vec![Input::Pointer],
            entry_points: vec![EntryPoint::Cli {
                flag: "ex".to_string(),
            }],
            surfaces: vec![SurfaceRef::from("s.toml")],
            surface_states: None,
            subscriptions: vec![],
            influences: vec![],
            permissions: vec![],
            narration: None,
        }
    }

    fn req(mode: Mode, input: Input) -> AppLaunchRequest {
        AppLaunchRequest {
            app_id: "app://example.lifecycle".to_string(),
            session: SessionConfig { mode, input },
        }
    }

    #[test]
    fn shape_check_rejects_unsupported_mode() {
        let m = mk_manifest();
        let err = check_launch_shape(&req(Mode::SingleApp, Input::Pointer), &m).unwrap_err();
        assert!(matches!(err, LaunchError::ModeUnsupported { .. }));
    }

    #[test]
    fn shape_check_rejects_unsupported_input() {
        let m = mk_manifest();
        let err = check_launch_shape(&req(Mode::Desktop, Input::Voice), &m).unwrap_err();
        assert!(matches!(err, LaunchError::InputUnsupported { .. }));
    }

    #[test]
    fn noop_gate_grants_and_passes_surface_ids() {
        let m = mk_manifest();
        let r = NoopGate.authorize_launch(&req(Mode::Desktop, Input::Pointer), &m);
        match r {
            AppLaunchResult::Granted { surface_ids } => {
                assert_eq!(surface_ids, vec!["s.toml".to_string()]);
            }
            AppLaunchResult::Denied { .. } => panic!("NoopGate must grant"),
        }
    }

    #[test]
    fn strict_gate_denies_camera_capture() {
        let mut m = mk_manifest();
        m.permissions.push(Permission::Camera);
        let r = CapturePrivacyGate.authorize_launch(&req(Mode::Desktop, Input::Pointer), &m);
        assert!(matches!(r, AppLaunchResult::Denied { .. }));
    }

    #[test]
    fn strict_gate_grants_fs_only_permissions() {
        let mut m = mk_manifest();
        m.permissions.push(Permission::FsPath("/tmp".to_string()));
        m.permissions
            .push(Permission::NetDomain("x.com".to_string()));
        let r = CapturePrivacyGate.authorize_launch(&req(Mode::Desktop, Input::Pointer), &m);
        assert!(matches!(r, AppLaunchResult::Granted { .. }));
    }

    #[test]
    fn capture_privacy_gate_is_strict_gate_alias() {
        // Type-level identity: StrictGate is CapturePrivacyGate.
        let gate: StrictGate = CapturePrivacyGate;
        let _: &dyn Gate = &gate;
    }

    #[test]
    fn noop_adapter_open_always_grants() {
        let r = NoopGate.authorize_adapter_open(
            "mic",
            "substrate/sensor/mic",
            &[Permission::Mic],
            &[],
        );
        assert!(r.is_granted());
    }

    #[test]
    fn adr012_adapter_open_denies_capture_without_grant() {
        let r = CapturePrivacyGate.authorize_adapter_open(
            "mic",
            "substrate/sensor/mic",
            &[Permission::Mic],
            &[],
        );
        match r {
            AdapterOpenResult::Denied { missing, reason } => {
                assert_eq!(missing, vec![Permission::Mic]);
                assert!(reason.contains("ADR-012"), "{reason}");
                assert!(reason.contains("mic"), "{reason}");
            }
            AdapterOpenResult::Granted => panic!("must deny without grant"),
        }
    }

    #[test]
    fn adr012_adapter_open_allows_capture_with_grant() {
        let r = CapturePrivacyGate.authorize_adapter_open(
            "mic",
            "substrate/sensor/mic",
            &[Permission::Mic],
            &[Permission::Mic],
        );
        assert!(r.is_granted());
    }

    #[test]
    fn adr012_adapter_open_public_empty_required_grants() {
        let r = CapturePrivacyGate.authorize_adapter_open(
            "kernel",
            "substrate/kernel/processes",
            &[],
            &[],
        );
        assert!(r.is_granted());
    }

    #[test]
    fn adr012_fs_prefix_grant_covers_child_path() {
        assert!(permission_covered(
            &Permission::FsPath("/tmp/weft/a".into()),
            &[Permission::FsPath("/tmp".into())],
        ));
        assert!(!permission_covered(
            &Permission::FsPath("/var".into()),
            &[Permission::FsPath("/tmp".into())],
        ));
    }

    #[test]
    fn infer_capture_from_adapter_id_and_topic() {
        assert_eq!(
            infer_capture_permission("mic", "substrate/sensor/mic"),
            Some(Permission::Mic)
        );
        assert_eq!(
            infer_capture_permission("cam", "substrate/sensor/camera"),
            Some(Permission::Camera)
        );
        assert_eq!(
            infer_capture_permission("kernel", "substrate/kernel/processes"),
            None
        );
    }
}
