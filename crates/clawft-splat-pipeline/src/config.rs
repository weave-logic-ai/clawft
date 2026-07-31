//! Pipeline configuration — tool paths and per-stage parameters.
//!
//! Everything a stage shells out to is named here so swapping tools
//! (colmap → glomap, brush → another trainer) is config, not code.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level config, loaded from `splatd.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineConfig {
    /// Root directory for job workspaces.
    pub data_dir: PathBuf,
    pub tools: Tools,
    pub frames: FramesConfig,
    pub sfm: SfmConfig,
    pub train: TrainConfig,
    pub compress: CompressConfig,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            tools: Tools::default(),
            frames: FramesConfig::default(),
            sfm: SfmConfig::default(),
            train: TrainConfig::default(),
            compress: CompressConfig::default(),
        }
    }
}

impl PipelineConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    /// Resolve `data_dir` to an absolute path. Stages shell out with
    /// `current_dir = <job root>`, so a *relative* `data_dir` makes every
    /// derived path (the input video, `dataset/images/…`, colmap dirs) resolve
    /// against the wrong cwd — e.g. ffprobe got `<jobroot>/data/jobs/<id>/input.mp4`
    /// and failed "No such file". Absolutizing here makes cwd irrelevant for all
    /// stages. Call once at startup.
    pub fn absolutize(&mut self) {
        if self.data_dir.is_relative()
            && let Ok(cwd) = std::env::current_dir()
        {
            self.data_dir = cwd.join(&self.data_dir);
        }
    }
}

/// External tool binaries. Absolute paths or names resolved via PATH.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Tools {
    pub ffmpeg: String,
    pub ffprobe: String,
    pub colmap: String,
    /// Optional: if set, `glomap mapper` replaces `colmap mapper` (much faster
    /// global SfM; feature extraction/matching stay on colmap).
    pub glomap: Option<String>,
    /// Brush headless trainer (`brush-cli`, or the `brush` app binary).
    pub brush: String,
    /// PlayCanvas splat-transform (npm i -g @playcanvas/splat-transform).
    /// None disables the compress stage.
    pub splat_transform: Option<String>,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            ffmpeg: "ffmpeg".into(),
            ffprobe: "ffprobe".into(),
            colmap: "colmap".into(),
            glomap: None,
            brush: "brush-cli".into(),
            splat_transform: Some("splat-transform".into()),
        }
    }
}

/// Frame extraction (stage 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FramesConfig {
    /// Frames per second to sample from the video.
    pub fps: f32,
    /// Hard cap on extracted frames (long videos get uniformly thinned).
    pub max_frames: u32,
    /// Longest-edge pixel size frames are downscaled to (0 = keep original).
    pub max_edge: u32,
    /// JPEG quality for ffmpeg (2 = best, 31 = worst).
    pub jpeg_q: u8,
}

impl Default for FramesConfig {
    fn default() -> Self {
        Self { fps: 2.0, max_frames: 300, max_edge: 1600, jpeg_q: 2 }
    }
}

/// COLMAP structure-from-motion (stage 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SfmConfig {
    /// COLMAP camera model. SIMPLE_RADIAL is robust for phone video.
    pub camera_model: String,
    /// Single camera shared across frames (true for one-phone video).
    pub single_camera: bool,
    /// Use sequential matching (right for video; exhaustive for photo sets).
    pub sequential: bool,
    /// Loop detection for sequential matching (helps closed walkarounds).
    pub loop_detection: bool,
}

impl Default for SfmConfig {
    fn default() -> Self {
        Self {
            camera_model: "SIMPLE_RADIAL".into(),
            single_camera: true,
            sequential: true,
            loop_detection: false,
        }
    }
}

/// Brush training (stage 4). Flag names match brush-cli's clap definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrainConfig {
    pub total_iters: u32,
    /// Training-time image resolution cap (brush --max-resolution).
    pub max_resolution: u32,
    /// Cap on splat count (brush --max-splats).
    pub max_splats: u32,
    /// Spherical-harmonics degree (brush --sh-degree).
    pub sh_degree: u32,
    /// Extra raw flags appended to the brush invocation.
    pub extra_args: Vec<String>,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            total_iters: 30_000,
            max_resolution: 1600,
            max_splats: 3_000_000,
            sh_degree: 3,
            extra_args: vec![],
        }
    }
}

/// SOG compression (stage 5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompressConfig {
    /// Output extension: "sog" (bundled) or "compressed.ply".
    pub format: String,
}

impl Default for CompressConfig {
    fn default() -> Self {
        Self { format: "sog".into() }
    }
}
