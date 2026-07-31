//! `clawft-splat-pipeline` — phone video **or** image-set session → Gaussian splat.
//!
//! ```text
//!  video ─ probe ─ frames ─ sfm ─ train ─ compress ─ package ─▶ artifacts/
//!  image-set ──┘ (skip ffmpeg; copy frames) colmap brush splat-transform
//!                                                          + world_model.json
//! ```
//!
//! Package (ADR-078 W0 / WEFT-708 + W1 / WEFT-709) writes `world_model.json`
//! with `SPLAT_SCENE` metadata, optional scene AABB, and W1 geometric
//! partition (surfaces / objects / free-space volumes) when points exist.
//! Image-set sessions follow `docs/weftos/capture-protocol-v1.md` (WEFT-704).
//!
//! Library only — no HTTP here. `clawft-splatd` (binary `splatd`) composes
//! this behind the v1 job API. See `docs/weftos/splat-pipeline-design.md`.

#![deny(rust_2018_idioms)]

pub mod config;
pub mod job;
pub mod multi_cam;
pub mod partition;
pub mod pipeline;
pub mod run;
pub mod session;
pub mod stages;
pub mod world_model;

pub use config::PipelineConfig;
pub use job::{JobDirs, JobRecord, JobStatus, Metrics, Stage};
pub use multi_cam::{
    CAMERAS_JSON, CameraEntry, CamerasDocument, MULTICAM_PROTOCOL_V1, MultiCamSession,
    flatten_frames, is_multi_cam_session, parse_cameras_json, parse_multi_cam_session,
};
pub use partition::{
    ObjectHit, PartitionParams, PartitionResult, Point3, SurfaceHit, VolumeHit, partition_points,
    partition_to_json_leaves,
};
pub use pipeline::{now_ms, run_job};
pub use session::{
    CAPTURE_PROTOCOL_V1, InputKind, JobInput, PoseLine, SESSION_DIR_NAME, SessionLayout,
    detect_job_input, is_session_dir, normalize_session_root, parse_session,
};
pub use world_model::{
    SceneAabb, build_world_model, build_world_model_stub, compute_scene_aabb, load_scene_points,
    write_world_model,
};
