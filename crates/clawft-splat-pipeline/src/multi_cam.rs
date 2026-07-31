//! Multi-camera fixed-rig session schema (WEFT-711 / M0).
//!
//! Accepts a `cameras.json` describing known extrinsics/intrinsics and a
//! multi-folder layout under a session package:
//!
//! ```text
//! session-<uuid>/
//!   cameras.json
//!   session.json            # optional; protocol weft.multicam.v1
//!   cam_a/frames/...
//!   cam_b/frames/...
//! ```
//!
//! Single-cam `weft.capture.v1` (`frames/` + `poses.jsonl`) remains the
//! default path via [`crate::session`]. This module is additive.

use crate::session::{IMAGE_EXTENSIONS, SessionError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Protocol id for multi-cam session packages.
pub const MULTICAM_PROTOCOL_V1: &str = "weft.multicam.v1";

/// Well-known cameras file name.
pub const CAMERAS_JSON: &str = "cameras.json";

/// One camera in a fixed multi-cam rig.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraEntry {
    /// Stable camera id (matches folder name when present).
    pub id: String,
    /// Camera model label (e.g. `pinhole`, `opencv`, `simple_radial`).
    #[serde(default = "default_model")]
    pub model: String,
    /// Intrinsics 3×3 row-major (optional for M0 dry-run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<[f64; 9]>,
    /// Distortion coefficients (model-dependent; optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dist: Option<Vec<f64>>,
    /// World-from-camera transform 4×4 row-major (`T_world_cam`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t_world_cam: Option<[f64; 16]>,
    /// Optional image size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Relative folder of frames for this cam (default: `{id}/frames` or `{id}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames_dir: Option<String>,
}

fn default_model() -> String {
    "pinhole".into()
}

/// Root `cameras.json` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CamerasDocument {
    /// Schema / protocol version string.
    #[serde(default = "default_protocol")]
    pub protocol: String,
    /// Shared world frame name (default `scene_local`).
    #[serde(default = "default_frame")]
    pub frame: String,
    /// Camera entries (at least 2 for multi-cam).
    pub cameras: Vec<CameraEntry>,
    /// Optional free-form notes / sync metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<serde_json::Value>,
}

fn default_protocol() -> String {
    MULTICAM_PROTOCOL_V1.into()
}

fn default_frame() -> String {
    "scene_local".into()
}

/// Parsed multi-cam session layout.
#[derive(Debug, Clone)]
pub struct MultiCamSession {
    pub root: PathBuf,
    pub cameras: CamerasDocument,
    /// Per-camera resolved frame paths (sorted).
    pub frames_by_cam: Vec<(String, Vec<PathBuf>)>,
}

/// True when `path` looks like a multi-cam session (has `cameras.json` with ≥2 cams).
pub fn is_multi_cam_session(path: &Path) -> bool {
    let cam_path = path.join(CAMERAS_JSON);
    match parse_cameras_json(&cam_path) {
        Ok(doc) => doc.cameras.len() >= 2,
        Err(_) => false,
    }
}

/// Parse `cameras.json` from a path.
pub fn parse_cameras_json(path: &Path) -> Result<CamerasDocument, SessionError> {
    let raw = std::fs::read_to_string(path).map_err(SessionError::Io)?;
    let doc: CamerasDocument = serde_json::from_str(&raw).map_err(SessionError::Json)?;
    if doc.cameras.is_empty() {
        return Err(SessionError::NoFrames(format!(
            "cameras.json has no cameras: {}",
            path.display()
        )));
    }
    Ok(doc)
}

/// Parse a multi-cam session package (cameras.json + per-cam frame folders).
pub fn parse_multi_cam_session(root: &Path) -> Result<MultiCamSession, SessionError> {
    if !root.is_dir() {
        return Err(SessionError::MissingDir(root.display().to_string()));
    }
    let doc = parse_cameras_json(&root.join(CAMERAS_JSON))?;
    if doc.cameras.len() < 2 {
        return Err(SessionError::NoFrames(format!(
            "multi-cam requires ≥2 cameras, got {}",
            doc.cameras.len()
        )));
    }

    let mut frames_by_cam = Vec::new();
    for cam in &doc.cameras {
        let frames_dir = resolve_cam_frames_dir(root, cam);
        let frames = list_images(&frames_dir).unwrap_or_default();
        frames_by_cam.push((cam.id.clone(), frames));
    }

    // At least one camera must have images for a dry-run path.
    if frames_by_cam.iter().all(|(_, f)| f.is_empty()) {
        return Err(SessionError::NoFrames(root.display().to_string()));
    }

    Ok(MultiCamSession {
        root: root.to_path_buf(),
        cameras: doc,
        frames_by_cam,
    })
}

fn resolve_cam_frames_dir(root: &Path, cam: &CameraEntry) -> PathBuf {
    if let Some(rel) = &cam.frames_dir {
        return root.join(rel);
    }
    let with_frames = root.join(&cam.id).join("frames");
    if with_frames.is_dir() {
        return with_frames;
    }
    root.join(&cam.id)
}

fn list_images(dir: &Path) -> Result<Vec<PathBuf>, SessionError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if IMAGE_EXTENSIONS.iter().any(|x| *x == ext) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Flatten all multi-cam frames into a single ordered list (cam order × frames).
///
/// M0 dry-run helper for feeding the existing single image-set train path.
pub fn flatten_frames(session: &MultiCamSession) -> Vec<PathBuf> {
    let mut all = Vec::new();
    for (_, frames) in &session.frames_by_cam {
        all.extend(frames.iter().cloned());
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_minimal_session(dir: &Path) {
        std::fs::create_dir_all(dir.join("cam_a").join("frames")).unwrap();
        std::fs::create_dir_all(dir.join("cam_b").join("frames")).unwrap();
        // Tiny fake "images"
        std::fs::write(dir.join("cam_a/frames/000.png"), b"png").unwrap();
        std::fs::write(dir.join("cam_b/frames/000.png"), b"png").unwrap();
        let doc = CamerasDocument {
            protocol: MULTICAM_PROTOCOL_V1.into(),
            frame: "scene_local".into(),
            cameras: vec![
                CameraEntry {
                    id: "cam_a".into(),
                    model: "pinhole".into(),
                    k: Some([500.0, 0.0, 320.0, 0.0, 500.0, 240.0, 0.0, 0.0, 1.0]),
                    dist: None,
                    t_world_cam: Some(identity4()),
                    width: Some(640),
                    height: Some(480),
                    frames_dir: None,
                },
                CameraEntry {
                    id: "cam_b".into(),
                    model: "pinhole".into(),
                    k: Some([500.0, 0.0, 320.0, 0.0, 500.0, 240.0, 0.0, 0.0, 1.0]),
                    dist: None,
                    t_world_cam: Some([
                        1.0, 0.0, 0.0, 0.5, // 0.5 m baseline on x
                        0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                    ]),
                    width: Some(640),
                    height: Some(480),
                    frames_dir: None,
                },
            ],
            sync: None,
        };
        let mut f = std::fs::File::create(dir.join(CAMERAS_JSON)).unwrap();
        f.write_all(serde_json::to_string_pretty(&doc).unwrap().as_bytes())
            .unwrap();
    }

    fn identity4() -> [f64; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    #[test]
    fn cameras_json_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_session(dir.path());
        let doc = parse_cameras_json(&dir.path().join(CAMERAS_JSON)).unwrap();
        assert_eq!(doc.cameras.len(), 2);
        assert_eq!(doc.protocol, MULTICAM_PROTOCOL_V1);
        assert!(is_multi_cam_session(dir.path()));
    }

    #[test]
    fn parse_multi_cam_collects_frames() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_session(dir.path());
        let session = parse_multi_cam_session(dir.path()).unwrap();
        assert_eq!(session.frames_by_cam.len(), 2);
        assert_eq!(session.frames_by_cam[0].1.len(), 1);
        assert_eq!(flatten_frames(&session).len(), 2);
    }

    #[test]
    fn single_cam_not_multi() {
        let dir = tempfile::tempdir().unwrap();
        let doc = CamerasDocument {
            protocol: MULTICAM_PROTOCOL_V1.into(),
            frame: "scene_local".into(),
            cameras: vec![CameraEntry {
                id: "only".into(),
                model: "pinhole".into(),
                k: None,
                dist: None,
                t_world_cam: None,
                width: None,
                height: None,
                frames_dir: None,
            }],
            sync: None,
        };
        std::fs::write(
            dir.path().join(CAMERAS_JSON),
            serde_json::to_string(&doc).unwrap(),
        )
        .unwrap();
        assert!(!is_multi_cam_session(dir.path()));
    }
}
