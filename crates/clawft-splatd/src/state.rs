//! Shared server state: config, job index, serial work queue, health counters.

use clawft_splat_pipeline::{JobDirs, JobRecord, PipelineConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

pub struct AppState {
    pub cfg: PipelineConfig,
    /// In-memory index of job records (source of truth is job.json on disk;
    /// this mirror avoids re-reading on every GET).
    pub jobs: Mutex<HashMap<String, JobRecord>>,
    /// Serial work queue — one worker, whisper-style one-in-flight.
    pub tx: mpsc::UnboundedSender<String>,
    pub started: Instant,
    pub jobs_done: AtomicU64,
    pub jobs_failed: AtomicU64,
    pub queue_depth: AtomicU64,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn job_dirs(&self, job_id: &str) -> JobDirs {
        JobDirs::new(&self.cfg.data_dir, job_id)
    }

    /// Rehydrate the index from disk at startup so restarts don't forget
    /// completed jobs (interrupted `running` jobs are marked failed —
    /// honest failure; the durable-workflow consumer will re-dispatch).
    pub async fn rehydrate(&self) {
        let jobs_root = self.cfg.data_dir.join("jobs");
        let Ok(entries) = std::fs::read_dir(&jobs_root) else { return };
        let mut jobs = self.jobs.lock().await;
        for entry in entries.flatten() {
            if let Ok(mut rec) = JobRecord::load(&entry.path()) {
                use clawft_splat_pipeline::JobStatus;
                if matches!(rec.status, JobStatus::Running | JobStatus::Queued) {
                    rec.status = JobStatus::Failed;
                    rec.error = Some("splatd restarted mid-job".into());
                    let _ = rec.save(&entry.path());
                    self.jobs_failed.fetch_add(1, Ordering::Relaxed);
                }
                jobs.insert(rec.job_id.clone(), rec);
            }
        }
        tracing::info!(n = jobs.len(), "rehydrated jobs from disk");
    }
}
