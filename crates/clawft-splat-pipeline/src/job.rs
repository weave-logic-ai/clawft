//! Job model — the record persisted to `job.json` and served over HTTP.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Probe,
    Frames,
    Sfm,
    Train,
    Compress,
    Package,
}

impl Stage {
    pub const ALL: [Stage; 6] = [
        Stage::Probe,
        Stage::Frames,
        Stage::Sfm,
        Stage::Train,
        Stage::Compress,
        Stage::Package,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Probe => "probe",
            Stage::Frames => "frames",
            Stage::Sfm => "sfm",
            Stage::Train => "train",
            Stage::Compress => "compress",
            Stage::Package => "package",
        }
    }
}

/// Metrics accumulated across stages; all optional so partial failures
/// still report what they learned.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub video_duration_s: Option<f64>,
    pub video_width: Option<u32>,
    pub video_height: Option<u32>,
    pub n_frames: Option<u32>,
    /// Images COLMAP registered into the reconstruction.
    pub n_registered: Option<u32>,
    /// Sparse points in the COLMAP model.
    pub n_points: Option<u64>,
    pub splat_ply_bytes: Option<u64>,
    pub splat_sog_bytes: Option<u64>,
}

/// The persisted job record. This is also the JSON body POSTed to
/// `callback_url` on terminal state (the shasta-os-020 `awaitArtifact` hook).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub status: JobStatus,
    /// Stage currently running (or the one that failed).
    pub stage: Option<Stage>,
    pub created_ts_ms: u64,
    pub finished_ts_ms: Option<u64>,
    /// Where the input came from (path, url, or "upload").
    pub source: String,
    /// Optional webhook invoked with this record on done/failed.
    pub callback_url: Option<String>,
    /// Stage name → wall-clock milliseconds.
    pub timings_ms: BTreeMap<String, u64>,
    pub metrics: Metrics,
    /// Artifact name → file name under `artifacts/`.
    pub artifacts: BTreeMap<String, String>,
    pub error: Option<String>,
}

impl JobRecord {
    pub fn new(job_id: String, source: String, callback_url: Option<String>, now_ms: u64) -> Self {
        Self {
            job_id,
            status: JobStatus::Queued,
            stage: None,
            created_ts_ms: now_ms,
            finished_ts_ms: None,
            source,
            callback_url,
            timings_ms: BTreeMap::new(),
            metrics: Metrics::default(),
            artifacts: BTreeMap::new(),
            error: None,
        }
    }

    pub fn save(&self, job_dir: &Path) -> anyhow::Result<()> {
        let tmp = job_dir.join("job.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, job_dir.join("job.json"))?;
        Ok(())
    }

    pub fn load(job_dir: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(job_dir.join("job.json"))?;
        Ok(serde_json::from_str(&text)?)
    }
}

/// Well-known paths inside a job workspace (see DESIGN.md §4).
#[derive(Debug, Clone)]
pub struct JobDirs {
    pub root: PathBuf,
}

impl JobDirs {
    pub fn new(data_dir: &Path, job_id: &str) -> Self {
        Self { root: data_dir.join("jobs").join(job_id) }
    }

    pub fn dataset(&self) -> PathBuf {
        self.root.join("dataset")
    }
    pub fn images(&self) -> PathBuf {
        self.dataset().join("images")
    }
    pub fn sparse(&self) -> PathBuf {
        self.dataset().join("sparse")
    }
    pub fn artifacts(&self) -> PathBuf {
        self.root.join("artifacts")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn colmap_db(&self) -> PathBuf {
        self.root.join("colmap.db")
    }

    pub fn create_all(&self) -> std::io::Result<()> {
        for d in [self.images(), self.sparse(), self.artifacts(), self.logs()] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }
}
