//! T39 lifecycle — **signed-install** (T37 / T38 family).
//!
//! Cryptographic signature verification for ClawHub packages lives in
//! the ClawHub client / CLI (K4). `clawft-plugin` has no signing API
//! today; these tests lock the **manifest install surface** that signed
//! install will wrap, plus an explicit scaffold for the future verify step.

use crate::harness::{install_manifest, plugin_json};
use serde_json::json;

/// T38 contract: local / unsigned install of a valid manifest proceeds.
///
/// Production CLI may log a warning for unsigned local plugins; the
/// trait crate only requires schema validity.
#[test]
fn unsigned_local_install_accepted_when_manifest_valid() {
    let body = plugin_json("com.example.unsigned-local", "1.0.0", false);
    // No signature field exists on PluginManifest today.
    assert!(body.get("signature").is_none());
    assert!(body.get("signed_by").is_none());
    let m = install_manifest(&body).expect("unsigned local install ok (T38)");
    assert_eq!(m.id, "com.example.unsigned-local");
}

/// Forward-compat: unknown signature-related fields on the wire are
/// ignored by serde (same as schema_version). When K4 lands a real
/// verify step, it will read these before or after parse.
#[test]
fn signature_metadata_fields_ignored_by_manifest_parse() {
    let mut body = plugin_json("com.example.signed-meta", "1.0.0", false);
    body["signature"] = json!({
        "alg": "ed25519",
        "key_id": "clawhub-root-1",
        "sig": "00".repeat(64)
    });
    body["signed_by"] = json!("clawhub-official");
    // Parse still succeeds — signature is not a required schema field.
    let m = install_manifest(&body).expect("extra signature fields ignored by schema");
    assert_eq!(m.id, "com.example.signed-meta");
}

/// Scaffold: install gate that *would* reject a ClawHub package without
/// a valid signature. Real crypto is out of crate scope (K4 / ClawHub).
///
/// // TODO(WEFT-73/K4): replace this scaffold with `clawft_services::clawhub`
/// // signature verification once the trust-root registry is available in-tree.
#[test]
fn signed_install_rejects_missing_signature_scaffold() {
    let package = SignedPackageFixture {
        plugin_id: "com.clawhub.example".into(),
        manifest: install_manifest(&plugin_json("com.clawhub.example", "1.2.3", false))
            .expect("manifest"),
        signature: None, // ClawHub package without signature
        allow_unsigned: false,
    };
    let err = package
        .verify_for_install()
        .expect_err("ClawHub install without signature must fail (T37 scaffold)");
    assert!(err.contains("signature"), "err was: {err}");
    assert!(err.contains("com.clawhub.example"));
}

/// Scaffold: explicit `--allow-unsigned` opt-in permits missing signature
/// (local/dev path; mirrors T38 warning + proceed).
#[test]
fn signed_install_allow_unsigned_opt_in_scaffold() {
    let package = SignedPackageFixture {
        plugin_id: "com.local.dev".into(),
        manifest: install_manifest(&plugin_json("com.local.dev", "0.0.1", false)).expect("m"),
        signature: None,
        allow_unsigned: true,
    };
    package
        .verify_for_install()
        .expect("allow_unsigned permits local install");
}

/// Scaffold: present signature blob is accepted by the placeholder verifier.
///
/// // TODO(WEFT-73/K4): assert Ed25519 verify against trust root, not mere presence.
#[test]
fn signed_install_accepts_present_signature_scaffold() {
    let package = SignedPackageFixture {
        plugin_id: "com.clawhub.signed".into(),
        manifest: install_manifest(&plugin_json("com.clawhub.signed", "2.0.0", false)).expect("m"),
        signature: Some(SignatureBlob {
            alg: "ed25519".into(),
            key_id: "clawhub-root-1".into(),
            // Placeholder — not a real signature.
            bytes: vec![0u8; 64],
        }),
        allow_unsigned: false,
    };
    package
        .verify_for_install()
        .expect("present signature passes scaffold verifier");
}

// ── Scaffold types (test-only; production will live under clawhub client) ─

struct SignatureBlob {
    #[allow(dead_code)]
    alg: String,
    #[allow(dead_code)]
    key_id: String,
    bytes: Vec<u8>,
}

struct SignedPackageFixture {
    plugin_id: String,
    #[allow(dead_code)]
    manifest: clawft_plugin::PluginManifest,
    signature: Option<SignatureBlob>,
    allow_unsigned: bool,
}

impl SignedPackageFixture {
    /// Placeholder install-time signature gate (T37/T38).
    ///
    /// // TODO(WEFT-73/K4): implement real verify:
    /// //   1. resolve key_id against ~/.clawft/trust-roots/
    /// //   2. ed25519 verify(manifest_canonical_bytes, sig, pubkey)
    /// //   3. reject key not in trust root
    fn verify_for_install(&self) -> Result<(), String> {
        match &self.signature {
            Some(sig) if !sig.bytes.is_empty() => Ok(()),
            Some(_) => Err(format!(
                "empty signature for ClawHub package '{}'",
                self.plugin_id
            )),
            None if self.allow_unsigned => Ok(()),
            None => Err(format!(
                "ClawHub install of '{}' rejected: missing signature (T37)",
                self.plugin_id
            )),
        }
    }
}
