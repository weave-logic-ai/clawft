//! Ed25519 node identity for the Android edge peer (ADR-077 / WEFT-707).
//!
//! On first use, a signing key is generated and written under
//! `<data_dir>/node_identity.json` (seed hex + public key hex). Subsequent
//! loads return the **same** `node_id` so the phone stays a stable WeftOS peer.

use crate::error::EdgeError;
use crate::hex_util::{bytes_to_hex, hex_to_bytes};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

const IDENTITY_FILE: &str = "node_identity.json";

/// Public identity view (safe to log / ship in session.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NodeIdentity {
    /// 32-byte verifying key as lowercase hex (canonical node id).
    pub node_id_hex: String,
    /// Same as `node_id_hex` today; kept for future multi-key schemes.
    pub public_key_hex: String,
}

#[derive(Serialize, Deserialize)]
struct IdentityFile {
    /// 32-byte seed (signing secret) as lowercase hex.
    seed_hex: String,
    public_key_hex: String,
    /// Schema marker for forward compatibility.
    version: u32,
}

/// Load an existing identity or create + persist a new Ed25519 keypair.
pub fn load_or_create(data_dir: &Path) -> Result<NodeIdentity, EdgeError> {
    fs::create_dir_all(data_dir).map_err(EdgeError::io)?;
    let path = identity_path(data_dir);
    if path.exists() {
        return load(&path);
    }
    create_and_store(&path)
}

/// Load identity from an explicit path (tests).
pub fn load(path: &Path) -> Result<NodeIdentity, EdgeError> {
    let raw = fs::read_to_string(path).map_err(EdgeError::io)?;
    let mut file: IdentityFile =
        serde_json::from_str(&raw).map_err(|e| EdgeError::identity(format!("parse: {e}")))?;
    let seed = hex_to_bytes(&file.seed_hex)
        .map_err(|e| EdgeError::identity(format!("seed hex: {e}")))?;
    if seed.len() != 32 {
        return Err(EdgeError::identity("seed must be 32 bytes"));
    }
    let mut seed_arr = [0u8; 32];
    seed_arr.copy_from_slice(&seed);
    let signing = SigningKey::from_bytes(&seed_arr);
    seed_arr.zeroize();
    file.seed_hex.zeroize();

    let verifying = signing.verifying_key();
    let public_hex = bytes_to_hex(verifying.as_bytes());
    if public_hex != file.public_key_hex {
        return Err(EdgeError::identity(
            "stored public key does not match seed (corrupt identity file)",
        ));
    }
    Ok(NodeIdentity {
        node_id_hex: public_hex.clone(),
        public_key_hex: public_hex,
    })
}

fn create_and_store(path: &Path) -> Result<NodeIdentity, EdgeError> {
    let signing = SigningKey::generate(&mut OsRng);
    let verifying = signing.verifying_key();
    let seed_hex = bytes_to_hex(signing.to_bytes().as_ref());
    let public_key_hex = bytes_to_hex(verifying.as_bytes());
    let file = IdentityFile {
        seed_hex: seed_hex.clone(),
        public_key_hex: public_key_hex.clone(),
        version: 1,
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| EdgeError::identity(format!("serialize: {e}")))?;
    // Restrictive mode on Unix; Android app-private dirs already isolate the file.
    write_private(path, json.as_bytes())?;
    let _ = seed_hex; // dropped (string); seed already in file
    Ok(NodeIdentity {
        node_id_hex: public_key_hex.clone(),
        public_key_hex,
    })
}

/// Sign an arbitrary message with the node identity (capability / pair proofs later).
pub fn sign_message(data_dir: &Path, message: &[u8]) -> Result<Vec<u8>, EdgeError> {
    let path = identity_path(data_dir);
    let raw = fs::read_to_string(&path).map_err(EdgeError::io)?;
    let mut file: IdentityFile =
        serde_json::from_str(&raw).map_err(|e| EdgeError::identity(format!("parse: {e}")))?;
    let seed = hex_to_bytes(&file.seed_hex)
        .map_err(|e| EdgeError::identity(format!("seed hex: {e}")))?;
    if seed.len() != 32 {
        return Err(EdgeError::identity("seed must be 32 bytes"));
    }
    let mut seed_arr = [0u8; 32];
    seed_arr.copy_from_slice(&seed);
    let signing = SigningKey::from_bytes(&seed_arr);
    seed_arr.zeroize();
    file.seed_hex.zeroize();
    let sig = signing.sign(message);
    Ok(sig.to_bytes().to_vec())
}

/// Verify a message signature against a hex public key (host / peer).
pub fn verify_message(
    public_key_hex: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, EdgeError> {
    let pk = hex_to_bytes(public_key_hex)
        .map_err(|e| EdgeError::identity(format!("pubkey hex: {e}")))?;
    if pk.len() != 32 {
        return Err(EdgeError::identity("public key must be 32 bytes"));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&pk);
    let vk = VerifyingKey::from_bytes(&arr)
        .map_err(|e| EdgeError::identity(format!("verifying key: {e}")))?;
    if signature.len() != 64 {
        return Ok(false);
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(signature);
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
    Ok(vk.verify_strict(message, &sig).is_ok())
}

pub fn identity_path(data_dir: &Path) -> PathBuf {
    data_dir.join(IDENTITY_FILE)
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), EdgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(EdgeError::io)?;
    }
    fs::write(path, bytes).map_err(EdgeError::io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_or_create_is_stable_across_restarts() {
        let dir = tempdir().unwrap();
        let a = load_or_create(dir.path()).unwrap();
        let b = load_or_create(dir.path()).unwrap();
        assert_eq!(a.node_id_hex, b.node_id_hex);
        assert_eq!(a.node_id_hex.len(), 64);
        assert_eq!(a.public_key_hex, a.node_id_hex);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let dir = tempdir().unwrap();
        let id = load_or_create(dir.path()).unwrap();
        let msg = b"weft.pair.v1:hello";
        let sig = sign_message(dir.path(), msg).unwrap();
        assert!(verify_message(&id.public_key_hex, msg, &sig).unwrap());
        assert!(!verify_message(&id.public_key_hex, b"tampered", &sig).unwrap());
    }
}
