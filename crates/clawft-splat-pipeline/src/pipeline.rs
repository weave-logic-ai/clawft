//! Stage orchestration — runs a job's stages in order, persisting the
//! record after every transition so `job.json` survives a restart
//! mid-pipeline (the durable-workflow consumer re-polls; we never lose
//! what already ran).

use crate::config::PipelineConfig;
use crate::job::{JobDirs, JobRecord, JobStatus, Stage};
use crate::session::JobInput;
use crate::stages;
use std::time::Instant;

/// Drive all stages for `rec`. On stage failure the record is marked
/// Failed with the stage name + error retained (honest failure, per the
/// shasta-os-019 discipline — no fabricated artifacts).
///
/// `input` is either a video file (`JobInput::Video`) or a weft.capture.v1
/// session directory (`JobInput::ImageSet`).
pub async fn run_job(
    cfg: &PipelineConfig,
    dirs: &JobDirs,
    rec: &mut JobRecord,
    input: &JobInput,
) {
    rec.status = JobStatus::Running;
    let _ = rec.save(&dirs.root);

    for stage in Stage::ALL {
        rec.stage = Some(stage);
        let _ = rec.save(&dirs.root);
        let started = Instant::now();

        let result = match stage {
            Stage::Probe => stages::probe(cfg, dirs, rec, input).await,
            Stage::Frames => stages::frames(cfg, dirs, rec, input).await,
            Stage::Sfm => stages::sfm(cfg, dirs, rec).await,
            Stage::Train => stages::train(cfg, dirs, rec).await,
            Stage::Compress => stages::compress(cfg, dirs, rec).await,
            Stage::Package => stages::package(cfg, dirs, rec).await,
        };

        rec.timings_ms
            .insert(stage.name().to_string(), started.elapsed().as_millis() as u64);

        if let Err(e) = result {
            tracing::error!(job = %rec.job_id, stage = stage.name(), error = %e, "stage failed");
            rec.status = JobStatus::Failed;
            rec.error = Some(format!("{}: {e}", stage.name()));
            rec.finished_ts_ms = Some(now_ms());
            let _ = rec.save(&dirs.root);
            return;
        }
        let _ = rec.save(&dirs.root);
    }

    rec.status = JobStatus::Done;
    rec.stage = None;
    rec.finished_ts_ms = Some(now_ms());
    let _ = rec.save(&dirs.root);
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
