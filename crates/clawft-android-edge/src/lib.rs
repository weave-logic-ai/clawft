//! `clawft-android-edge` — WeftOS edge core for the Android splat-capture app.
//!
//! ## Surface (UniFFI + plain Rust)
//!
//! | Concern | Module |
//! |---------|--------|
//! | Ed25519 node identity (persisted) | [`identity`] |
//! | Host URL + capability token | [`pair_store`] |
//! | Session ZIP → splatd job | [`upload`] |
//!
//! Kotlin bindings: enable the default `uniffi` feature and generate with
//! `cargo run -p uniffi-bindgen -- generate …` or the steps in
//! `docs/guides/android-splat-capture-edge.md`.
//!
//! Plane: WEFT-707 (A3), ADR-077.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod hex_util;
pub mod identity;
pub mod pair_store;
pub mod upload;

pub use error::EdgeError;
pub use identity::{NodeIdentity, load_or_create as load_or_create_identity};
pub use pair_store::{HostPair, clear_pair, load_pair, parse_pair_input, save_pair};
pub use upload::{JobStatus, UploadResult, get_job_status, health_check, upload_session_zip};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

// ─── UniFFI free-function façade (string paths for Kotlin) ───────────────────

/// Load or create the Ed25519 node identity under `data_dir`.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_load_or_create_identity(data_dir: String) -> Result<NodeIdentity, EdgeError> {
    identity::load_or_create(std::path::Path::new(&data_dir))
}

/// Parse pairing QR / URL and return a [`HostPair`] (does not persist).
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_parse_pair(
    input: String,
    token_override: Option<String>,
) -> Result<HostPair, EdgeError> {
    pair_store::parse_pair_input(&input, token_override.as_deref())
}

/// Persist host pair to `data_dir/host_pair.json`.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_save_pair(data_dir: String, pair: HostPair) -> Result<(), EdgeError> {
    pair_store::save_pair(std::path::Path::new(&data_dir), &pair)
}

/// Load host pair if present.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_load_pair(data_dir: String) -> Result<Option<HostPair>, EdgeError> {
    pair_store::load_pair(std::path::Path::new(&data_dir))
}

/// Remove stored host pair.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_clear_pair(data_dir: String) -> Result<(), EdgeError> {
    pair_store::clear_pair(std::path::Path::new(&data_dir))
}

/// Upload session ZIP; returns `job_id`.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_upload_session_zip(pair: HostPair, zip_path: String) -> Result<UploadResult, EdgeError> {
    upload::upload_session_zip(&pair, std::path::Path::new(&zip_path))
}

/// Poll splatd job status.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_get_job_status(pair: HostPair, job_id: String) -> Result<JobStatus, EdgeError> {
    upload::get_job_status(&pair, &job_id)
}

/// `GET /healthz` body.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_health_check(pair: HostPair) -> Result<String, EdgeError> {
    upload::health_check(&pair)
}

/// Sign `message` with the node secret (hex signature bytes as raw Vec).
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn edge_sign(data_dir: String, message: Vec<u8>) -> Result<Vec<u8>, EdgeError> {
    identity::sign_message(std::path::Path::new(&data_dir), &message)
}

#[cfg(test)]
mod facade_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn facade_identity_and_pair() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_string_lossy().to_string();
        let id = edge_load_or_create_identity(path.clone()).unwrap();
        let id2 = edge_load_or_create_identity(path.clone()).unwrap();
        assert_eq!(id.node_id_hex, id2.node_id_hex);

        let pair = edge_parse_pair("http://127.0.0.1:7860".into(), Some("cap-1".into())).unwrap();
        edge_save_pair(path.clone(), pair.clone()).unwrap();
        let loaded = edge_load_pair(path.clone()).unwrap().unwrap();
        assert_eq!(loaded.token, "cap-1");
        edge_clear_pair(path).unwrap();
    }
}
